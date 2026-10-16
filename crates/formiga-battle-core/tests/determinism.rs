//! The same match is the same match everywhere: replayed from its record, sent as JSON, or played
//! again from scratch. A changed order shows up as a desync on the turn it was changed.

use formiga_battle_core::ai::Coach;
use formiga_battle_core::tournament::{drawn_creature, play};
use formiga_battle_core::*;

fn rosters(seed: u64) -> [Vec<Profile>; 2] {
    [
        (0..3).map(|i| drawn_creature(seed, i)).collect(),
        (3..6).map(|i| drawn_creature(seed, i)).collect(),
    ]
}

#[test]
fn the_same_seed_rosters_and_orders_give_the_same_match_byte_for_byte() {
    for seed in 0..12 {
        let arena = Arena::backyard();
        let (a, _) = play(
            &arena,
            Ruleset::default(),
            seed,
            rosters(seed),
            [Coach::Exhibition; 2],
        );
        let (b, _) = play(
            &arena,
            Ruleset::default(),
            seed,
            rosters(seed),
            [Coach::Exhibition; 2],
        );
        assert_eq!(a.history, b.history, "seed {seed}");
        assert_eq!(a.board, b.board);
    }
}

#[test]
fn a_recorded_match_replays_from_its_json() {
    let arena = Arena::backyard();
    let (m, reports) = play(
        &arena,
        Ruleset::default(),
        5,
        rosters(5),
        [Coach::Exhibition, Coach::Absent],
    );
    let json = serde_json::to_string(&m.record()).unwrap();
    let record: MatchRecord = serde_json::from_str(&json).unwrap();
    let (replayed, replayed_reports) = record.replay().expect("replays");
    assert_eq!(replayed.board, m.board);
    assert_eq!(replayed_reports, reports);
}

#[test]
fn a_changed_order_is_found_as_a_desync_on_its_turn() {
    let arena = Arena::backyard();
    let (m, _) = play(
        &arena,
        Ruleset::default(),
        9,
        rosters(9),
        [Coach::Exhibition; 2],
    );
    let mut record = m.record();
    assert!(record.turns.len() >= 2);
    record.turns[1].submitted[0] = vec![Directive::Defend {
        squares: vec![Tile::new(0, 0)],
    }];
    match record.replay() {
        Err(replay::ReplayError::Desync { turn, .. }) => assert_eq!(turn, 2),
        other => panic!("expected a desync, got {other:?}"),
    }
}

#[test]
fn a_record_from_other_rules_is_refused_rather_than_replayed_differently() {
    let arena = Arena::backyard();
    let (m, _) = play(
        &arena,
        Ruleset::default(),
        1,
        rosters(1),
        [Coach::Absent; 2],
    );
    let mut record = m.record();
    record.rules_version += 1;
    assert!(matches!(
        record.replay(),
        Err(replay::ReplayError::RulesVersion(_))
    ));
}

#[test]
fn every_match_ends_by_the_turn_cap() {
    let arena = Arena::backyard();
    for seed in 0..20 {
        let (m, _) = play(
            &arena,
            Ruleset::default(),
            seed,
            rosters(seed),
            [Coach::Absent; 2],
        );
        assert!(m.outcome().unwrap().turn <= Ruleset::default().turn_cap);
    }
}
