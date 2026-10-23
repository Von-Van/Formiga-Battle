//! A fixture between two Handlers nobody is playing, played out under the same rules as any other
//! match: the same `Match`, the same coaches a Handler could meet, the same seed rule. Its match id
//! comes from the bracket's seed and the fixture's place, so the same fixture played on another
//! machine, or played again after a crash, is the same match with the same id.

use crate::bracket::Decided;
use formiga_battle_core::ai::Coach;
use formiga_battle_core::rng::{self, Purpose};
use formiga_battle_core::tournament::play;
use formiga_battle_core::{Arena, MatchRecord, Profile, Ruleset, Side, Victory};

/// What a fixture came to.
#[derive(Clone, Debug)]
pub struct PlayedOut {
    pub match_id: String,
    /// 0 if the first side won, 1 if the second, `None` for a draw.
    pub winner: Option<u8>,
    pub decided: Decided,
    pub turns: u8,
    pub record: MatchRecord,
}

/// The match seed for fixture `index` of a bracket seeded `seed`.
pub fn match_seed(seed: u64, index: usize) -> u64 {
    rng::roll(seed, Purpose::Pairing, &[2000, index as u64])
}

/// The id of fixture `index`'s match, as 32 hex digits, like every match id.
pub fn match_id(seed: u64, index: usize) -> String {
    format!(
        "{:016x}{:016x}",
        rng::roll(seed, Purpose::Pairing, &[3000, index as u64]),
        rng::roll(seed, Purpose::Pairing, &[3001, index as u64])
    )
}

/// How a match's outcome settles a fixture.
pub fn decided(victory: Option<Victory>) -> Decided {
    match victory {
        Some(Victory::Capture) => Decided::Capture,
        Some(Victory::Knockout) => Decided::Knockout,
        Some(Victory::TurnCap) => Decided::Cap,
        Some(Victory::Forfeit) => Decided::Forfeit,
        None => Decided::Toss,
    }
}

/// Plays fixture `index` out between two teams, each with its own coach.
pub fn play_out(
    arena: &Arena,
    ruleset: Ruleset,
    seed: u64,
    index: usize,
    rosters: [Vec<Profile>; 2],
    coaches: [Coach; 2],
) -> PlayedOut {
    let (m, _) = play(arena, ruleset, match_seed(seed, index), rosters, coaches);
    let outcome = m.outcome().expect("played to the end");
    PlayedOut {
        match_id: match_id(seed, index),
        winner: outcome.winner.map(|w| u8::from(w == Side::Right)),
        decided: decided(outcome.victory),
        turns: outcome.turn,
        record: m.record(),
    }
}
