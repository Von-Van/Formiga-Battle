//! Encirclement: two contesters from different sides break a hold, holders side by side turn
//! them away, and a team that only ever holds the beacon is beaten by one that walks round it.

mod common;
use common::*;
use formiga_battle_core::ai::{Coach, flank_threatened};
use formiga_battle_core::tournament::run;
use formiga_battle_core::*;

#[test]
fn a_flanked_holder_is_pushed_away_from_a_contester_and_out_of_the_zone_if_it_can_be() {
    // R0 holds the top arm of the cross; L0 comes from the middle below it, L1 from its left.
    let mut m = board_on(
        Arena::backyard(),
        &[(L0, (4, 3)), (L1, (3, 2)), (R0, (4, 2))],
    );
    let report = m.resolve_turn(orders(&[
        (L0, contest(R0)),
        (L1, contest(R0)),
        (R0, hold(4, 2)),
    ]));
    // Away from L0 is (4, 1), out of the zone; away from L1 is (5, 2), out of it too. The upper
    // of the two, as every tie between ways out is broken the same way.
    assert_eq!(at(&m, R0), (4, 1));
    assert!(!m.arena.in_zone(m.fighter(R0).at));
    assert_eq!(composure(&m, R0), 2);
    assert!(kinds(&report).contains(&&EventKind::Flanked {
        target: R0,
        by: vec![L0, L1],
        from: Tile::new(4, 2),
        pushed_to: Some(Tile::new(4, 1)),
    }));
}

#[test]
fn a_flanked_holder_stays_off_balance_for_the_rest_of_the_turn_and_holds_again_after() {
    let mut m = board(&[(L0, (0, 2)), (L1, (2, 2)), (R0, (1, 2))]);
    let report = m.resolve_turn(orders(&[
        (L0, contest(R0)),
        (L1, contest(R0)),
        (R0, hold(1, 2)),
    ]));
    assert!(m.fighter(R0).interrupted && !m.fighter(R0).holding);
    let broke = report
        .events
        .iter()
        .find(|e| matches!(e.kind, EventKind::Flanked { .. }))
        .expect("flanked");
    assert_eq!(
        broke.tick, 1,
        "they were in place, so it broke before anyone stepped"
    );
    let report = m.resolve_turn(orders(&[
        (L0, hold(0, 2)),
        (L1, hold(2, 2)),
        (R0, hold(1, 2)),
    ]));
    assert!(m.fighter(R0).holding);
    assert!(kinds(&report).contains(&&EventKind::Holding {
        slot: R0,
        at: Tile::new(1, 2)
    }));
}

#[test]
fn one_contester_alone_is_still_turned_away() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    m.resolve_turn(orders(&[(L0, contest(R0)), (R0, hold(1, 2))]));
    assert!(m.fighter(R0).holding);
    assert_eq!(composure(&m, L0), 2);
}

#[test]
fn a_holder_beside_the_one_flanked_turns_the_flank_away_with_it() {
    // R1 holds corner to corner with R0 and touches both contesters.
    let mut m = board(&[(L0, (1, 0)), (L1, (0, 1)), (R0, (1, 1)), (R1, (0, 0))]);
    assert!(!flank_threatened(&m, R0), "nobody is holding yet");
    m.resolve_turn(orders(&[
        (L0, hold(1, 0)),
        (L1, hold(0, 1)),
        (R0, hold(1, 1)),
        (R1, hold(0, 0)),
    ]));
    assert!(!flank_threatened(&m, R0), "guarded by R1");
    let report = m.resolve_turn(orders(&[
        (L0, contest(R0)),
        (L1, contest(R0)),
        (R0, hold(1, 1)),
        (R1, hold(0, 0)),
    ]));
    assert!(m.fighter(R0).holding);
    assert!(
        !kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::Flanked { .. }))
    );
}

#[test]
fn three_against_a_holder_and_its_neighbour_still_break_it() {
    let mut m = board(&[
        (L0, (1, 0)),
        (L1, (0, 1)),
        (L2, (2, 1)),
        (R0, (1, 1)),
        (R1, (0, 0)),
    ]);
    m.resolve_turn(orders(&[
        (L0, hold(1, 0)),
        (L1, hold(0, 1)),
        (L2, hold(2, 1)),
        (R0, hold(1, 1)),
        (R1, hold(0, 0)),
    ]));
    assert!(
        flank_threatened(&m, R0),
        "three on its sides, two to turn them away"
    );
    m.resolve_turn(orders(&[
        (L0, contest(R0)),
        (L1, contest(R0)),
        (L2, contest(R0)),
        (R0, hold(1, 1)),
        (R1, hold(0, 0)),
    ]));
    assert!(!m.fighter(R0).holding);
    assert!(m.fighter(R1).holding, "only the one contested breaks");
}

#[test]
fn a_flank_already_in_place_lands_before_the_holder_can_step_away() {
    let mut m = board(&[(L0, (0, 2)), (L1, (2, 2)), (R0, (1, 2))]);
    m.resolve_turn(orders(&[
        (L0, hold(0, 2)),
        (L1, hold(2, 2)),
        (R0, hold(1, 2)),
    ]));
    m.resolve_turn(orders(&[
        (L0, contest(R0)),
        (L1, contest(R0)),
        (R0, walk(1, 4)),
    ]));
    // Not holding when it tried to leave, it is simply caught by both.
    assert_eq!(composure(&m, R0), 1);
    assert_eq!(at(&m, R0), (1, 2));
}

#[test]
fn a_team_that_only_ever_holds_the_beacon_loses_to_one_that_walks_round_it() {
    let s = run(
        &Arena::backyard(),
        Ruleset::default(),
        5,
        60,
        [Coach::Exhibition, Coach::Turtle],
    );
    assert!(
        s.left.score() > 0.55,
        "exhibition scores {:.3}",
        s.left.score()
    );
    let s = run(
        &Arena::backyard(),
        Ruleset::default(),
        5,
        30,
        [Coach::Familiar, Coach::Turtle],
    );
    assert!(
        s.left.score() > 0.7,
        "familiar scores {:.3}",
        s.left.score()
    );
}
