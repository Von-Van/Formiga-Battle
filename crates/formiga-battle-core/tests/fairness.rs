//! The handoff's fairness checks, small enough to run with every test: no side of the board, no
//! temperament kind, wins consistently more, and nothing about a creature changes the numbers
//! everyone plays by. `formiga-battle tournament` runs the same thing at scale.

use formiga_battle_core::ai::Coach;
use formiga_battle_core::tournament::{drawn_creature, run};
use formiga_battle_core::*;

#[test]
fn neither_side_of_the_board_and_no_kind_of_creature_wins_consistently_more() {
    let s = run(
        &Arena::backyard(),
        Ruleset::default(),
        3,
        150,
        [Coach::Exhibition; 2],
    );
    assert_eq!(s.matches, 300);
    assert!(
        (0.42..=0.58).contains(&s.left.score()),
        "left {:.3}",
        s.left.score()
    );
    for (kind, record) in &s.by_kind {
        assert!(
            (0.35..=0.65).contains(&record.score()),
            "{kind} scores {:.3} over {}",
            record.score(),
            record.played
        );
    }
}

#[test]
fn matches_end_inside_the_turn_cap_and_the_beacon_decides_some_outright() {
    let s = run(
        &Arena::backyard(),
        Ruleset::default(),
        4,
        100,
        [Coach::Exhibition; 2],
    );
    assert!(s.average_turns() <= f64::from(Ruleset::default().turn_cap));
    // One in eight: since rules version 5, creatures taking their own way round arrive later
    // and fewer matches are won at the beacon (about 13%, from 21% under version 4).
    assert!(
        s.by_capture * 8 > s.matches,
        "captures {} of {}",
        s.by_capture,
        s.matches
    );
}

#[test]
fn every_creature_starts_with_the_same_composure_and_takes_one_step_a_tick_whoever_it_is() {
    let rosters = [
        (0..3).map(|i| drawn_creature(11, i)).collect::<Vec<_>>(),
        (3..6).map(|i| drawn_creature(11, i)).collect::<Vec<_>>(),
    ];
    let mut m = Match::new(Arena::backyard(), Ruleset::default(), 2, rosters).unwrap();
    for slot in Slot::all() {
        assert_eq!(m.fighter(slot).composure, Ruleset::default().composure);
    }
    let orders = [
        Coach::Exhibition.orders(&m, Side::Left),
        Coach::Exhibition.orders(&m, Side::Right),
    ];
    let report = m.resolve_turn(orders);
    for e in &report.events {
        if let EventKind::Stepped { from, to, .. } = e.kind {
            assert_eq!(from.distance(to), 1);
        }
    }
    let mut per_tick = std::collections::BTreeMap::new();
    for e in &report.events {
        if let EventKind::Stepped { slot, .. } = e.kind {
            *per_tick.entry((e.tick, slot)).or_insert(0) += 1;
        }
    }
    assert!(per_tick.values().all(|n| *n == 1));
}
