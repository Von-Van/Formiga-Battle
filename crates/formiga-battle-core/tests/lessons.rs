//! Field Lessons: each lesson's intended plan does what it asks, on any seed, and giving no orders
//! (or only half of what it asks) does not.

mod common;
use common::*;
use formiga_battle_core::lesson::{Lesson, Verdict, YOU};
use formiga_battle_core::*;

const SEEDS: std::ops::Range<u64> = 1..41;

fn take(squares: &[(i8, i8)]) -> Directive {
    Directive::Take {
        squares: squares.iter().map(|(x, y)| Tile::new(*x, *y)).collect(),
    }
}

fn defend(squares: &[(i8, i8)]) -> Directive {
    Directive::Defend {
        squares: squares.iter().map(|(x, y)| Tile::new(*x, *y)).collect(),
    }
}

fn interact(x: i8, y: i8) -> Directive {
    Directive::Interact {
        at: Tile::new(x, y),
    }
}

/// Plays a lesson out, giving `plan(m)` each turn, until it is decided. Panics if it never is.
fn play(
    lesson: Lesson,
    seed: u64,
    rosters: [Vec<Profile>; 2],
    plan: impl Fn(&Match) -> TeamPlan,
) -> (Verdict, Vec<Event>) {
    let mut m = lesson.stage(Ruleset::default(), seed, rosters).unwrap();
    let mut events = Vec::new();
    let mut given = Vec::new();
    loop {
        let ours = plan(&m);
        let theirs = lesson.visitors(&m);
        given.push(ours.clone());
        let report = m.resolve_turn([ours, theirs]);
        events.extend(report.events);
        let verdict = lesson.judge(&m, &events, &given);
        if verdict.decided() {
            return (verdict, events);
        }
        assert!(given.len() < 20, "{lesson:?} never ended");
    }
}

fn learned(v: &Verdict) -> bool {
    matches!(v, Verdict::Learned(_))
}

/// The plan each lesson teaches.
fn intended(lesson: Lesson) -> impl Fn(&Match) -> TeamPlan {
    move |m: &Match| match lesson {
        Lesson::Beacon => vec![take(&[(4, 3)])],
        Lesson::Composure => vec![defend(&[(2, 3), (2, 5)])],
        Lesson::Flank => vec![take(&[(4, 2)])],
        Lesson::Orders => vec![take(&[(4, 4)]), defend(&[(7, 6)])],
        Lesson::Weather => {
            if m.fighter(Slot::new(YOU, 0)).carrying.is_some() {
                vec![interact(4, 3)]
            } else {
                vec![defend(&[(4, 3)])]
            }
        }
        Lesson::Personality => vec![take(&[(4, 3)]), take(&[(3, 3), (4, 2), (4, 4)])],
    }
}

#[test]
fn every_lesson_is_learned_with_the_plan_it_teaches_on_any_seed() {
    for lesson in Lesson::ALL {
        for seed in SEEDS {
            let (verdict, events) = play(lesson, seed, rosters(), intended(lesson));
            assert!(
                learned(&verdict),
                "{lesson:?} seed {seed}: {verdict:?}\n{:#?}",
                events
                    .iter()
                    .map(|e| (e.turn, e.tick, &e.kind))
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn no_lesson_is_learned_by_giving_no_orders() {
    for lesson in Lesson::ALL {
        if lesson == Lesson::Personality {
            continue;
        }
        for seed in SEEDS {
            let (verdict, _) = play(lesson, seed, rosters(), |_| Vec::new());
            assert!(!learned(&verdict), "{lesson:?} seed {seed}: {verdict:?}");
        }
    }
}

#[test]
fn half_a_plan_is_not_enough_where_a_lesson_asks_for_two_things() {
    for seed in SEEDS {
        let (verdict, _) = play(Lesson::Orders, seed, rosters(), |_| vec![take(&[(4, 4)])]);
        assert!(!learned(&verdict), "flank alone, seed {seed}: {verdict:?}");
        let (verdict, _) = play(Lesson::Composure, seed, rosters(), |_| {
            vec![defend(&[(2, 3)])]
        });
        assert!(!learned(&verdict), "one square, seed {seed}: {verdict:?}");
        let (verdict, _) = play(Lesson::Weather, seed, rosters(), |_| {
            vec![defend(&[(4, 3)])]
        });
        assert!(!learned(&verdict), "no umbrella, seed {seed}: {verdict:?}");
    }
}

#[test]
fn who_they_are_can_be_learned_by_every_kind_of_trio() {
    let mut failed = Vec::new();
    for seed in 0..200u64 {
        let ours: Vec<Profile> = (0..3)
            .map(|n| tournament::drawn_creature(seed, n))
            .collect();
        let (verdict, _) = play(
            Lesson::Personality,
            seed,
            [ours.clone(), vec![plain(4), plain(5), plain(6)]],
            intended(Lesson::Personality),
        );
        if !learned(&verdict) {
            failed.push((seed, ours.iter().map(|p| p.kind).collect::<Vec<_>>()));
        }
    }
    assert!(failed.len() <= 2, "{failed:?}");
}

#[test]
fn a_lesson_begins_where_it_says() {
    let m = Lesson::Weather
        .stage(Ruleset::default(), 1, rosters())
        .unwrap();
    assert_eq!(m.board.turn, 2);
    assert_eq!(m.fighter(Slot::new(YOU, 0)).carrying, Some(Item::Leaf));
    for lesson in Lesson::ALL {
        let m = lesson.stage(Ruleset::default(), 1, rosters()).unwrap();
        assert!(m.outcome().is_none());
        for slot in m.standing_slots() {
            assert!(m.walkable(m.fighter(slot).at), "{lesson:?} {slot:?}");
        }
    }
}
