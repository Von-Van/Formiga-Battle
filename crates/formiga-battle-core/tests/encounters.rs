//! MOVE, HOLD and CONTEST against each other, one encounter at a time, including the group edge
//! cases the handoff asks to be explicit rules.

mod common;
use common::*;
use formiga_battle_core::*;

#[test]
fn a_contest_next_to_a_mover_catches_it_before_it_can_step_away() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[(L0, contest(R0)), (R0, walk(4, 2))]));
    assert_eq!(composure(&m, R0), 2);
    assert_eq!(composure(&m, L0), 3);
    assert_eq!(
        at(&m, R0),
        (1, 2),
        "an interrupted mover stops where it was caught"
    );
    assert!(kinds(&report).contains(&&EventKind::Interrupted {
        target: R0,
        by: vec![L0]
    }));
}

#[test]
fn a_contester_walks_up_to_a_mover_and_interrupts_it_on_the_way() {
    // R0 walks toward L0's side; L0 walks out to meet it.
    let mut m = board(&[(L0, (0, 2)), (R0, (5, 2))]);
    m.resolve_orders(orders(&[(L0, contest(R0)), (R0, walk(0, 1))]));
    assert_eq!(composure(&m, R0), 2);
    assert!(m.fighter(R0).interrupted);
}

#[test]
fn two_contesters_on_one_mover_cost_it_two() {
    let mut m = board(&[(L0, (0, 2)), (L1, (1, 1)), (R0, (1, 2))]);
    m.resolve_orders(orders(&[
        (L0, contest(R0)),
        (L1, contest(R0)),
        (R0, walk(4, 2)),
    ]));
    assert_eq!(composure(&m, R0), 1);
}

#[test]
fn three_contesters_on_one_mover_are_capped_at_two() {
    let mut m = board(&[(L0, (0, 2)), (L1, (1, 1)), (L2, (2, 2)), (R0, (1, 2))]);
    m.resolve_orders(orders(&[
        (L0, contest(R0)),
        (L1, contest(R0)),
        (L2, contest(R0)),
        (R0, walk(1, 4)),
    ]));
    assert_eq!(composure(&m, R0), 1);
}

#[test]
fn a_holder_counters_a_lone_contester_and_is_unhurt() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[(L0, contest(R0)), (R0, hold(1, 2))]));
    assert_eq!(composure(&m, L0), 2);
    assert_eq!(composure(&m, R0), 3);
    assert!(kinds(&report).contains(&&EventKind::Countered {
        target: R0,
        holders: vec![R0],
        contesters: vec![L0],
    }));
}

#[test]
fn two_contesters_from_opposite_sides_break_a_hold_and_neither_is_turned_away() {
    let mut m = board(&[(L0, (0, 2)), (L1, (2, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[
        (L0, contest(R0)),
        (L1, contest(R0)),
        (R0, hold(1, 2)),
    ]));
    assert_eq!(composure(&m, R0), 2, "flanked, the holder loses one");
    assert_eq!((composure(&m, L0), composure(&m, L1)), (3, 3));
    assert!(!m.fighter(R0).holding && m.fighter(R0).interrupted);
    // Both ways out are where a contester stands: boxed in, it stays where it is.
    assert_eq!(at(&m, R0), (1, 2));
    assert!(kinds(&report).contains(&&EventKind::Flanked {
        target: R0,
        by: vec![L0, L1],
        from: Tile::new(1, 2),
        pushed_to: None,
    }));
}

#[test]
fn two_contesters_against_two_holders_side_by_side_are_both_turned_away() {
    // R0 holds at (1,1); R1 holds at (0,0), next to both contesters.
    let mut m = board(&[(L0, (1, 0)), (L1, (0, 1)), (R0, (1, 1)), (R1, (0, 0))]);
    m.resolve_orders(orders(&[
        (L0, contest(R0)),
        (L1, contest(R0)),
        (R0, hold(1, 1)),
        (R1, hold(0, 0)),
    ]));
    assert_eq!(composure(&m, R0), 3);
    assert_eq!(composure(&m, R1), 3);
    assert_eq!(composure(&m, L0), 2);
    assert_eq!(composure(&m, L1), 2);
}

#[test]
fn two_creatures_contesting_each_other_only_scuffle() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[(L0, contest(R0)), (R0, contest(L0))]));
    assert_eq!(composure(&m, L0), 3);
    assert_eq!(composure(&m, R0), 3);
    let scuffles = kinds(&report)
        .iter()
        .filter(|k| matches!(k, EventKind::Scuffle { mutual: true, .. }))
        .count();
    assert_eq!(scuffles, 1, "a mutual scuffle is told once");
}

#[test]
fn a_mover_walks_round_a_holder_and_neither_is_hurt() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    m.ruleset.ticks_per_turn = 4;
    m.resolve_orders(orders(&[(L0, walk(2, 2)), (R0, hold(1, 2))]));
    assert_eq!(at(&m, L0), (2, 2));
    assert_eq!(composure(&m, L0), 3);
    assert_eq!(composure(&m, R0), 3);
}

#[test]
fn two_movers_wanting_one_tile_are_settled_by_a_fair_draw_and_nobody_is_hurt() {
    let mut left_first = 0;
    for seed in 0..200 {
        let mut m = board(&[(L0, (0, 2)), (R0, (2, 2))]);
        m.seed = seed;
        m.ruleset.ticks_per_turn = 1;
        let report = m.resolve_orders(orders(&[(L0, walk(1, 2)), (R0, walk(1, 2))]));
        assert_eq!(composure(&m, L0) + composure(&m, R0), 6);
        let bumped = kinds(&report)
            .iter()
            .filter(|k| matches!(k, EventKind::Bumped { .. }))
            .count();
        assert_eq!(bumped, 1, "one steps in and the other is bumped");
        if at(&m, L0) == (1, 2) {
            left_first += 1;
        }
    }
    assert!((70..130).contains(&left_first), "{left_first} of 200");
}

#[test]
fn two_creatures_meeting_in_a_narrow_way_do_not_bump_for_ever() {
    // A corridor one tile wide; each walks toward the other's end.
    let arena = Arena::parse(
        "id: lane\nname: Lane\n\
         1 1 1 # # # Z\n\
         . . . . . . .\n\
         # # # # # # #\n\
         2 2 2 # # # #\n",
    )
    .unwrap();
    let mut m = board_on(arena, &[(L0, (1, 1)), (R0, (3, 1))]);
    m.resolve_orders(orders(&[(L0, walk(6, 1)), (R0, walk(0, 1))]));
    let report = m.resolve_orders(orders(&[(L0, walk(6, 1)), (R0, walk(0, 1))]));
    let bumps = kinds(&report)
        .iter()
        .filter(|k| matches!(k, EventKind::Bumped { .. }))
        .count();
    assert!(
        bumps <= 2,
        "they stop trying rather than bumping every tick: {bumps}"
    );
}

#[test]
fn two_movers_cannot_walk_through_each_other() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    m.ruleset.ticks_per_turn = 1;
    m.resolve_orders(orders(&[(L0, walk(1, 2)), (R0, walk(0, 2))]));
    assert_eq!(at(&m, L0), (0, 2));
    assert_eq!(at(&m, R0), (1, 2));
}

#[test]
fn a_creature_steps_into_a_tile_its_teammate_is_leaving_the_same_tick() {
    let mut m = board(&[(L0, (0, 2)), (L1, (1, 2)), (R0, (6, 4))]);
    m.ruleset.ticks_per_turn = 1;
    m.resolve_orders(orders(&[
        (L0, walk(1, 2)),
        (L1, walk(2, 2)),
        (R0, hold(6, 4)),
    ]));
    assert_eq!(at(&m, L0), (1, 2));
    assert_eq!(at(&m, L1), (2, 2));
}

#[test]
fn a_contest_is_settled_once_a_turn() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    m.resolve_orders(orders(&[(L0, contest(R0)), (R0, walk(1, 2))]));
    assert_eq!(composure(&m, R0), 2, "one contest, one composure, all turn");
}

#[test]
fn a_creature_with_a_nervous_streak_stays_put_after_a_contest_that_landed() {
    let mut teams = rosters();
    teams[0][0] = with_tension(plain(1), formiga_core::Tension::BraveButNervous);
    let mut m = board_with(field(), teams, &[(L0, (0, 2)), (R0, (1, 2))]);
    m.resolve_orders(orders(&[(L0, contest(R0)), (R0, walk(4, 2))]));
    assert_eq!(composure(&m, R0), 2);
    assert_eq!(at(&m, L0), (0, 2));
}

#[test]
fn a_creature_with_a_nervous_streak_hops_back_when_its_contest_is_turned_away() {
    let mut teams = rosters();
    teams[0][0] = with_tension(plain(1), formiga_core::Tension::BraveButNervous);
    let mut m = board_with(field(), teams, &[(L0, (0, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[(L0, contest(R0)), (R0, hold(1, 2))]));
    assert_eq!(composure(&m, L0), 2);
    assert_ne!(at(&m, L0), (0, 2));
    assert!(report.events.iter().any(|e| matches!(
        e.kind,
        EventKind::Stepped {
            reason: Reason::SteppedBack,
            ..
        }
    )));
}

#[test]
fn three_composure_lost_is_a_knockout_and_the_last_one_standing_wins() {
    let mut m = board(&[(L0, (0, 2)), (L1, (1, 1)), (R0, (1, 2))]);
    m.board.fighters[1][0].composure = 2;
    let report = m.resolve_orders(orders(&[
        (L0, contest(R0)),
        (L1, contest(R0)),
        (R0, walk(4, 2)),
    ]));
    assert!(m.fighter(R0).out);
    assert!(
        m.occupant(Tile::new(1, 2)).is_none(),
        "a knocked-out creature leaves the board"
    );
    assert_eq!(
        m.outcome(),
        Some(Outcome {
            winner: Some(Side::Left),
            victory: Some(Victory::Knockout),
            turn: 1
        })
    );
    assert!(kinds(&report).contains(&&EventKind::KnockedOut { slot: R0 }));
}

#[test]
fn an_order_aimed_at_a_teammate_is_treated_as_missing_and_filled() {
    let mut m = board(&[(L0, (0, 2)), (L1, (1, 1)), (R0, (6, 4))]);
    let report = m.resolve_orders(orders(&[
        (L0, contest(L1)),
        (L1, hold(1, 1)),
        (R0, hold(6, 4)),
    ]));
    assert!(report.events.iter().any(|e| matches!(
        e.kind,
        EventKind::OrderFilled {
            slot: L0,
            fill: event::Fill::Invalid,
            ..
        }
    )));
}
