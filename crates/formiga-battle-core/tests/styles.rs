//! The four coaching styles can be told apart by the jobs their plans give a team, and none of
//! them beats every other: a style is a way of playing, not a difficulty.

use formiga_battle_core::ai::Coach;
use formiga_battle_core::tournament::run;
use formiga_battle_core::*;
use std::collections::BTreeMap;

/// How often a style's plans give its creatures each kind of job, as shares of all the jobs its
/// team took on over a few matches against the Exhibition coach.
fn job_mix(archetype: Archetype) -> BTreeMap<String, f64> {
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let mut total = 0u32;
    for seed in 1..=8u64 {
        let rosters = [
            (0..3)
                .map(|i| tournament::drawn_creature(seed, i))
                .collect(),
            (3..6)
                .map(|i| tournament::drawn_creature(seed, i))
                .collect(),
        ];
        let (_, reports) = tournament::play(
            &Arena::backyard(),
            Ruleset::default(),
            seed,
            rosters,
            [Coach::Styled(archetype), Coach::Exhibition],
        );
        for report in &reports {
            for e in &report.events {
                if let EventKind::Assigned { slot, task, .. } = &e.kind
                    && slot.side == Side::Left
                {
                    *counts.entry(format!("{:?}", task.role)).or_default() += 1;
                    total += 1;
                }
            }
        }
    }
    counts
        .into_iter()
        .map(|(k, n)| (k, f64::from(n) / f64::from(total.max(1))))
        .collect()
}

/// The distance between two mixes: the shares that would have to change to turn one into the
/// other.
fn apart(a: &BTreeMap<String, f64>, b: &BTreeMap<String, f64>) -> f64 {
    let keys: std::collections::BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    keys.iter()
        .map(|k| (a.get(*k).unwrap_or(&0.0) - b.get(*k).unwrap_or(&0.0)).abs())
        .sum::<f64>()
        / 2.0
}

#[test]
fn every_style_gives_its_team_different_jobs_from_every_other() {
    let mixes: Vec<(Archetype, BTreeMap<String, f64>)> = Archetype::ALL
        .into_iter()
        .map(|a| (a, job_mix(a)))
        .collect();
    let mut report = String::new();
    let mut nearest = f64::MAX;
    for (i, (a, ma)) in mixes.iter().enumerate() {
        report.push_str(&format!("{:<12} {:?}\n", a.label(), ma));
        for (b, mb) in mixes.iter().skip(i + 1) {
            let d = apart(ma, mb);
            report.push_str(&format!("  {} to {}: {d:.3}\n", a.label(), b.label()));
            nearest = nearest.min(d);
        }
    }
    eprintln!("{report}");
    assert!(
        nearest >= 0.08,
        "two styles give nearly the same jobs ({nearest:.3} apart):\n{report}"
    );
}

/// Every style, the adaptive one too.
const EVERY: [Archetype; 5] = [
    Archetype::Aggressor,
    Archetype::Defender,
    Archetype::Tactician,
    Archetype::Opportunist,
    Archetype::Adaptive,
];

#[test]
fn on_no_arena_does_one_style_beat_every_other_style() {
    let mut wins_against_all = Vec::new();
    let mut report = String::new();
    for arena in Arena::built_in() {
        for a in EVERY {
            let mut beats_all = true;
            for b in EVERY {
                if a == b {
                    continue;
                }
                let s = run(
                    &arena,
                    Ruleset::default(),
                    5,
                    60,
                    [Coach::Styled(a), Coach::Styled(b)],
                );
                let score = s.left.score();
                report.push_str(&format!(
                    "{}: {} v {}: {score:.3}\n",
                    arena.name,
                    a.label(),
                    b.label()
                ));
                // 120 matches a pair: about 0.045 either way is chance.
                beats_all &= score > 0.6;
            }
            if beats_all {
                wins_against_all.push(format!("{} on {}", a.label(), arena.name));
            }
        }
    }
    eprintln!("{report}");
    assert!(
        wins_against_all.is_empty(),
        "{wins_against_all:?} beat every other style:\n{report}"
    );
}

#[test]
fn the_adaptive_coach_answers_what_the_other_team_orders_most() {
    use formiga_battle_core::ai::adapted;
    let rosters = || {
        [
            (0..3).map(|i| tournament::drawn_creature(4, i)).collect(),
            (3..6).map(|i| tournament::drawn_creature(4, i)).collect(),
        ]
    };
    // Against a team that only ever holds, after three turns of watching it goes round them.
    let mut m = Match::new(Arena::backyard(), Ruleset::default(), 4, rosters()).unwrap();
    assert_eq!(
        adapted(&m, Side::Left),
        Archetype::Adaptive,
        "nothing seen yet"
    );
    for turn in 0..3 {
        let plans = [
            Coach::Styled(Archetype::Adaptive).plan(&m, Side::Left),
            Coach::Turtle.plan(&m, Side::Right),
        ];
        m.resolve_turn(plans);
        if turn == 0 {
            assert_eq!(
                adapted(&m, Side::Left),
                Archetype::Adaptive,
                "one turn is not enough"
            );
        }
    }
    assert_eq!(adapted(&m, Side::Left), Archetype::Tactician);
    // It reads only what was sent: the same match gives the same answer.
    let again = m.clone();
    assert_eq!(adapted(&again, Side::Left), adapted(&m, Side::Left));
}
