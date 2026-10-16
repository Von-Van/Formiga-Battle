//! Many matches at once, for the fairness checks: creatures drawn the way Desktop draws a new
//! companion, put in random trios, played against each other from both sides of the board, and
//! counted by temperament kind and by trio.
//!
//! What it looks for is the handoff's test of fairness: no kind of creature and no trio should
//! win consistently more than another. A spread here is a reason to change a rule, never a
//! reason to tune a kind.

use crate::ai::Coach;
use crate::arena::Arena;
use crate::event::Victory;
use crate::game::{Match, TurnReport};
use crate::order::Side;
use crate::profile::Profile;
use crate::rng::{self, Purpose};
use crate::ruleset::Ruleset;
use formiga_core::{LearnedTendencies, Temperament, TemperamentKind};
use std::collections::BTreeMap;

const NAMES: [&str; 16] = [
    "Pip", "Mallow", "Clover", "Mochi", "Pebble", "Noodle", "Sprig", "Biscuit", "Fig", "Tansy",
    "Button", "Puddle", "Maple", "Wren", "Moss", "Bean",
];

/// A creature as Desktop would make one from scratch, with leanings drifted a little either way
/// from where its temperament rests, as a while of living in a colony drifts them.
pub fn drawn_creature(seed: u64, n: u64) -> Profile {
    let mut bytes = [0u8; 32];
    for (i, chunk) in bytes.chunks_mut(8).enumerate() {
        chunk.copy_from_slice(&rng::roll(seed, Purpose::Roster, &[n, i as u64]).to_le_bytes());
    }
    let (temperament, _) = Temperament::generated(bytes, 0, None);
    let mut leanings = LearnedTendencies::baseline(&temperament);
    let drift = |k: u64| (rng::roll(seed, Purpose::Roster, &[n, 100 + k]) % 61) as f32 - 30.0;
    for (k, value) in [
        &mut leanings.cursor_trust,
        &mut leanings.sociability,
        &mut leanings.climbing,
        &mut leanings.sleep_security,
        &mut leanings.exploration,
        &mut leanings.play,
        &mut leanings.home_affinity,
        &mut leanings.routine,
    ]
    .into_iter()
    .enumerate()
    {
        LearnedTendencies::adjust(value, drift(k as u64));
    }
    let name = NAMES[(n as usize) % NAMES.len()];
    Profile::from_parts(n + 1, name, &temperament, &leanings)
}

/// Plays a match to its end with the given coaches. Gives the match and every turn's report.
pub fn play(
    arena: &Arena,
    ruleset: Ruleset,
    seed: u64,
    rosters: [Vec<Profile>; 2],
    coaches: [Coach; 2],
) -> (Match, Vec<TurnReport>) {
    let mut m = Match::new(arena.clone(), ruleset, seed, rosters).expect("rosters of three");
    let mut reports = Vec::new();
    while m.outcome().is_none() {
        let plans = [
            coaches[0].plan(&m, Side::Left),
            coaches[1].plan(&m, Side::Right),
        ];
        reports.push(m.resolve_turn(plans));
    }
    (m, reports)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Record {
    pub played: u32,
    pub won: u32,
    pub drawn: u32,
}

impl Record {
    /// Wins plus half the draws, over the matches played.
    pub fn score(&self) -> f64 {
        if self.played == 0 {
            return 0.5;
        }
        (f64::from(self.won) + f64::from(self.drawn) / 2.0) / f64::from(self.played)
    }

    fn add(&mut self, result: Option<bool>) {
        self.played += 1;
        match result {
            Some(true) => self.won += 1,
            None => self.drawn += 1,
            Some(false) => {}
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Summary {
    pub matches: u32,
    pub left: Record,
    pub by_capture: u32,
    pub by_knockout: u32,
    pub at_cap: u32,
    pub draws: u32,
    pub turns: u32,
    pub knockouts: u32,
    pub fallback_orders: u32,
    /// Each creature counts its team's result.
    pub by_kind: BTreeMap<String, Record>,
    /// By trio of kinds, sorted, so the same three in any order are one trio.
    pub by_trio: BTreeMap<String, Record>,
}

impl Summary {
    pub fn average_turns(&self) -> f64 {
        f64::from(self.turns) / f64::from(self.matches.max(1))
    }

    /// The widest gap between two kinds' scores.
    pub fn kind_spread(&self) -> f64 {
        let scores: Vec<f64> = self.by_kind.values().map(Record::score).collect();
        let max = scores.iter().copied().fold(f64::MIN, f64::max);
        let min = scores.iter().copied().fold(f64::MAX, f64::min);
        if scores.is_empty() { 0.0 } else { max - min }
    }
}

fn kind_name(kind: TemperamentKind) -> String {
    kind.label().to_owned()
}

/// `pairings` random pairs of random trios, each played twice with the sides swapped, so neither
/// side of the board is in it.
pub fn run(
    arena: &Arena,
    ruleset: Ruleset,
    seed: u64,
    pairings: u32,
    coaches: [Coach; 2],
) -> Summary {
    let mut summary = Summary::default();
    for pairing in 0..u64::from(pairings) {
        let base = pairing * 6;
        let a: Vec<Profile> = (0..3).map(|i| drawn_creature(seed, base + i)).collect();
        let b: Vec<Profile> = (3..6).map(|i| drawn_creature(seed, base + i)).collect();
        let match_seed = rng::roll(seed, Purpose::Roster, &[pairing, 999]);
        for swap in [false, true] {
            let rosters = if swap {
                [b.clone(), a.clone()]
            } else {
                [a.clone(), b.clone()]
            };
            let (m, reports) = play(arena, ruleset, match_seed, rosters.clone(), coaches);
            let outcome = m.outcome().expect("played to the end");
            summary.matches += 1;
            summary.turns += u32::from(outcome.turn);
            match outcome.victory {
                Some(Victory::Capture) => summary.by_capture += 1,
                Some(Victory::Knockout) => summary.by_knockout += 1,
                Some(Victory::TurnCap | Victory::Forfeit) => summary.at_cap += 1,
                None => summary.draws += 1,
            }
            for report in &reports {
                for e in &report.events {
                    match e.kind {
                        crate::event::EventKind::KnockedOut { .. } => summary.knockouts += 1,
                        crate::event::EventKind::OrderFilled { .. } => summary.fallback_orders += 1,
                        _ => {}
                    }
                }
            }
            let result = |side: Side| outcome.winner.map(|w| w == side);
            summary.left.add(result(Side::Left));
            for side in Side::BOTH {
                let team = &rosters[side.index()];
                for p in team {
                    summary
                        .by_kind
                        .entry(kind_name(p.kind))
                        .or_default()
                        .add(result(side));
                }
                let mut trio: Vec<String> = team.iter().map(|p| kind_name(p.kind)).collect();
                trio.sort();
                summary
                    .by_trio
                    .entry(trio.join(" + "))
                    .or_default()
                    .add(result(side));
            }
        }
    }
    summary
}
