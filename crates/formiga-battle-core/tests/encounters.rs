//! MOVE, HOLD, PRESS and TRICK against each other, one encounter at a time, including the group
//! edge cases the handoff asks to be explicit rules.

mod common;
use common::*;
use formiga_battle_core::*;

#[test]
fn a_press_next_to_a_mover_pushes_it_back_before_it_can_step_away() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[(L0, press(R0)), (R0, walk(4, 2))]));
    assert_eq!(
        at(&m, R0),
        (2, 2),
        "pushed one tile on, away from the presser"
    );
    assert!(
        m.fighter(R0).interrupted,
        "and stopped for the rest of the turn"
    );
    assert_eq!(
        (composure(&m, L0), composure(&m, R0)),
        (3, 3),
        "a press costs nobody composure"
    );
    assert!(kinds(&report).contains(&&EventKind::Pressed {
        target: R0,
        by: vec![L0],
        pushed_to: Some(Tile::new(2, 2)),
        tricking: false,
    }));
}

#[test]
fn a_presser_walks_up_to_a_mover_and_stops_it_on_the_way() {
    let mut m = board(&[(L0, (0, 2)), (R0, (5, 2))]);
    m.resolve_orders(orders(&[(L0, press(R0)), (R0, walk(0, 1))]));
    assert!(m.fighter(R0).interrupted);
    assert_eq!(composure(&m, R0), 3);
}

#[test]
fn a_press_takes_what_the_one_it_catches_carries_and_it_is_usable_next_turn() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    m.board.fighters[1][0].carrying = Some(Item::Twig);
    let report = m.resolve_orders_using(
        orders(&[(L0, press(R0)), (R0, walk(4, 2))]),
        using(Side::Left, &[(0, (0, 1))]),
    );
    assert_eq!(m.fighter(L0).carrying, Some(Item::Twig));
    assert_eq!(m.fighter(R0).carrying, None);
    assert!(kinds(&report).contains(&&EventKind::Stole {
        by: L0,
        from: R0,
        item: Item::Twig,
    }));
    assert!(
        kinds(&report).contains(&&EventKind::OrderDropped {
            side: Side::Left,
            index: 0
        }),
        "a USE ITEM for something not yet carried when the turn began is left out"
    );
    let report = m.resolve_orders_using(
        orders(&[(L0, hold(0, 2)), (R0, hold(2, 2))]),
        using(Side::Left, &[(0, (0, 1))]),
    );
    assert!(
        report
            .events
            .iter()
            .any(|e| e.tick == 1 && matches!(e.kind, EventKind::ItemUsed { slot: L0, .. })),
        "stolen last turn, it is used at the start of this one"
    );
}

#[test]
fn a_press_with_nowhere_to_push_knocks_the_mover_off_balance_where_it_stands() {
    let mut m = board(&[(L0, (5, 2)), (R0, (6, 2))]);
    let report = m.resolve_orders(orders(&[(L0, press(R0)), (R0, walk(6, 0))]));
    assert_eq!(at(&m, R0), (6, 2));
    assert!(m.fighter(R0).interrupted);
    assert!(kinds(&report).contains(&&EventKind::Pressed {
        target: R0,
        by: vec![L0],
        pushed_to: None,
        tricking: false,
    }));
}

#[test]
fn a_full_pair_of_paws_drops_what_it_catches_rather_than_take_it() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    m.board.fighters[0][0].carrying = Some(Item::Leaf);
    m.board.fighters[1][0].carrying = Some(Item::Twig);
    m.resolve_orders(orders(&[(L0, press(R0)), (R0, walk(4, 2))]));
    assert_eq!(m.fighter(L0).carrying, Some(Item::Leaf));
    assert_eq!(m.fighter(R0).carrying, None);
    assert!(m.board.ground.contains(&(Tile::new(2, 2), Item::Twig)));
}

#[test]
fn two_presses_meeting_cost_each_one_and_stop_both() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[(L0, press(R0)), (R0, press(L0))]));
    assert_eq!((composure(&m, L0), composure(&m, R0)), (2, 2));
    assert!(m.fighter(L0).interrupted && m.fighter(R0).interrupted);
    assert_eq!(
        (at(&m, L0), at(&m, R0)),
        ((0, 2), (1, 2)),
        "nobody is pushed"
    );
    let collisions = kinds(&report)
        .iter()
        .filter(|k| matches!(k, EventKind::Collided { .. }))
        .count();
    assert_eq!(collisions, 1, "two pressing each other collide once");
}

#[test]
fn a_press_that_meets_one_pressing_someone_else_collides_too() {
    let mut m = board(&[(L0, (0, 2)), (L1, (0, 0)), (R0, (1, 2))]);
    m.resolve_orders(orders(&[
        (L0, press(R0)),
        (L1, hold(0, 0)),
        (R0, press(L1)),
    ]));
    assert_eq!((composure(&m, L0), composure(&m, R0)), (2, 2));
    assert!(m.fighter(R0).interrupted, "R0 stops before it gets to L1");
    assert_eq!(composure(&m, L1), 3);
}

#[test]
fn a_holder_turns_a_lone_press_from_the_side_away_and_nobody_is_hurt() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[(L0, press(R0)), (R0, hold(1, 2))]));
    assert_eq!((composure(&m, L0), composure(&m, R0)), (3, 3));
    assert!(m.fighter(R0).holding);
    assert!(kinds(&report).contains(&&EventKind::Repelled {
        target: R0,
        holders: vec![R0],
        pressers: vec![L0],
        faced: vec![],
    }));
}

#[test]
fn a_holder_facing_the_presser_turns_it_away_and_costs_it_one() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[(L0, press(R0)), (R0, facing(1, 2, Facing::West))]));
    assert_eq!((composure(&m, L0), composure(&m, R0)), (2, 3));
    assert!(kinds(&report).contains(&&EventKind::Repelled {
        target: R0,
        holders: vec![R0],
        pressers: vec![L0],
        faced: vec![L0],
    }));
}

#[test]
fn two_presses_from_opposite_sides_overwhelm_a_lone_holder() {
    let mut m = board(&[(L0, (0, 2)), (L1, (2, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[
        (L0, press(R0)),
        (L1, press(R0)),
        (R0, facing(1, 2, Facing::West)),
    ]));
    assert_eq!(composure(&m, R0), 2, "flanked, the holder loses one");
    assert_eq!((composure(&m, L0), composure(&m, L1)), (3, 3));
    assert!(!m.fighter(R0).holding && m.fighter(R0).interrupted);
    // Both ways out are where a presser stands: boxed in, it stays where it is.
    assert_eq!(at(&m, R0), (1, 2));
    assert!(kinds(&report).contains(&&EventKind::Flanked {
        target: R0,
        by: vec![L0, L1],
        from: Tile::new(1, 2),
        pushed_to: None,
    }));
}

#[test]
fn two_presses_against_two_holders_side_by_side_are_turned_away_by_those_facing_them() {
    // R0 holds at (1,1) facing north, toward L0; R1 holds at (0,0) facing south, toward L1.
    let mut m = board(&[(L0, (1, 0)), (L1, (0, 1)), (R0, (1, 1)), (R1, (0, 0))]);
    m.resolve_orders(orders(&[
        (L0, press(R0)),
        (L1, press(R0)),
        (R0, facing(1, 1, Facing::North)),
        (R1, facing(0, 0, Facing::South)),
    ]));
    assert_eq!((composure(&m, R0), composure(&m, R1)), (3, 3));
    assert_eq!((composure(&m, L0), composure(&m, L1)), (2, 2));
}

#[test]
fn a_trick_from_the_side_breaks_a_hold_without_moving_the_holder() {
    let mut m = board(&[(L0, (1, 1)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[(L0, trick(R0)), (R0, facing(1, 2, Facing::West))]));
    assert_eq!((composure(&m, L0), composure(&m, R0)), (3, 2));
    assert!(!m.fighter(R0).holding, "its hold is broken for the turn");
    assert!(!m.fighter(R0).interrupted, "but it is still on its feet");
    assert_eq!(at(&m, R0), (1, 2), "a trick does not move a holder");
    assert!(kinds(&report).contains(&&EventKind::Tricked {
        target: R0,
        by: vec![L0],
        from: Tile::new(1, 2),
    }));
}

#[test]
fn a_trick_from_behind_breaks_a_hold() {
    let mut m = board(&[(L0, (2, 2)), (R0, (1, 2))]);
    m.resolve_orders(orders(&[(L0, trick(R0)), (R0, facing(1, 2, Facing::West))]));
    assert_eq!(composure(&m, R0), 2);
    assert!(!m.fighter(R0).holding);
}

#[test]
fn a_holder_tricked_in_the_zone_still_keeps_it_contested_until_pressed_off() {
    // R0 holds the zone's one tile, (6, 0), facing west; L0 comes round behind it from the south
    // and L1 presses it once its hold is broken.
    let mut m = board(&[(L0, (6, 1)), (L1, (5, 0)), (R0, (6, 0))]);
    let report = m.resolve_orders(orders(&[
        (L0, trick(R0)),
        (L1, hold(5, 0)),
        (R0, facing(6, 0, Facing::West)),
    ]));
    assert!(m.fighter(R0).broken);
    assert!(
        !kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::CaptureProgress { .. })),
        "nobody lights the beacon while it stands there"
    );
    let mut m = board(&[(L0, (6, 1)), (L1, (4, 0)), (R0, (6, 0))]);
    let report = m.resolve_orders(orders(&[
        (L0, trick(R0)),
        (L1, press(R0)),
        (R0, facing(6, 0, Facing::West)),
    ]));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::Tricked { target: R0, .. }))
    );
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::Pressed { target: R0, .. })),
        "its hold broken, a press pushes it on like anyone not holding"
    );
}

#[test]
fn a_holder_facing_the_trickster_reads_a_trick_on_its_neighbour() {
    // R0 holds facing west; R1 holds beside it facing east, toward L0, which comes at R0's side.
    let mut m = board(&[(L0, (2, 1)), (R0, (2, 2)), (R1, (1, 1))]);
    let report = m.resolve_orders(orders(&[
        (L0, trick(R0)),
        (R0, facing(2, 2, Facing::West)),
        (R1, facing(1, 1, Facing::East)),
    ]));
    assert!(kinds(&report).contains(&&EventKind::TrickRead { target: R1, by: L0 }));
    assert!(m.fighter(R0).holding);
}

#[test]
fn a_trick_from_the_front_is_read_and_nothing_comes_of_it() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[(L0, trick(R0)), (R0, facing(1, 2, Facing::West))]));
    assert_eq!((composure(&m, L0), composure(&m, R0)), (3, 3));
    assert!(m.fighter(R0).holding);
    assert!(kinds(&report).contains(&&EventKind::TrickRead { target: R0, by: L0 }));
}

#[test]
fn a_trickster_goes_round_a_holder_rather_than_across_its_front() {
    let mut m = board(&[(L0, (0, 2)), (R0, (2, 2))]);
    m.board.fighters[1][0].holding = true;
    m.board.fighters[1][0].facing = Facing::West;
    let report = m.resolve_orders(orders(&[(L0, trick(R0)), (R0, facing(2, 2, Facing::West))]));
    assert!(
        !report.events.iter().any(|e| matches!(
            e.kind,
            EventKind::Stepped { slot: L0, to, .. } if to == Tile::new(1, 2)
        )),
        "it never steps in front of the holder"
    );
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::Tricked { target: R0, .. }))
    );
}

#[test]
fn a_press_catches_a_trickster_pushes_it_and_takes_what_it_carries() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    m.board.fighters[0][0].carrying = Some(Item::Bell);
    let report = m.resolve_orders(orders(&[(L0, trick(R0)), (R0, press(L0))]));
    assert!(m.fighter(L0).interrupted);
    assert_eq!(m.fighter(R0).carrying, Some(Item::Bell));
    assert_eq!(composure(&m, L0), 3);
    assert!(kinds(&report).contains(&&EventKind::Pressed {
        target: L0,
        by: vec![R0],
        pushed_to: None,
        tricking: true,
    }));
}

#[test]
fn a_trick_on_one_pressing_someone_else_is_caught_by_it() {
    let mut m = board(&[(L0, (1, 1)), (L1, (4, 2)), (R0, (1, 2))]);
    m.resolve_orders(orders(&[
        (L0, trick(R0)),
        (L1, walk(4, 2)),
        (R0, press(L1)),
    ]));
    assert!(m.fighter(L0).interrupted);
    assert_eq!(
        at(&m, L0),
        (1, 0),
        "pushed away from the one it tried to trick"
    );
}

#[test]
fn a_trick_on_someone_not_holding_waits_for_its_chance() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[(L0, trick(R0)), (R0, walk(1, 4))]));
    assert_eq!(composure(&m, R0), 3);
    assert!(!m.fighter(R0).interrupted);
    assert!(
        !kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::Tricked { .. }))
    );
}

#[test]
fn a_press_on_one_working_a_lever_stops_the_pull() {
    let arena = Arena::parse(
        "id: lever\nname: Lever\n\
         1 1 1 . . . Z\n\
         . . . G . . .\n\
         . . L . . . .\n\
         . . . . . . .\n\
         2 2 2 . . . .\n",
    )
    .unwrap();
    let mut m = board_on(arena, &[(L0, (1, 2)), (R0, (0, 2))]);
    let report = m.resolve_orders(orders(&[(L0, interact(2, 2)), (R0, press(L0))]));
    assert!(
        !kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::LeverPulled { .. }))
    );
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::Pressed { target: L0, .. }))
    );
}

#[test]
fn a_use_item_takes_the_first_tick_and_then_the_user_goes_about_its_job() {
    let mut m = board(&[(L0, (0, 2)), (R0, (6, 4))]);
    m.board.fighters[0][0].carrying = Some(Item::Twig);
    let report = m.resolve_orders_using(
        orders(&[(L0, walk(5, 2)), (R0, hold(6, 4))]),
        using(Side::Left, &[(0, (0, 1))]),
    );
    assert_eq!(
        at(&m, L0),
        (2, 2),
        "two steps, not three: using took a tick"
    );
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::TwigWedged { by: L0, .. }))
    );
}

#[test]
fn a_use_item_out_of_reach_is_left_out() {
    let mut m = board(&[(L0, (0, 2)), (R0, (6, 4))]);
    m.board.fighters[0][0].carrying = Some(Item::Twig);
    let report = m.resolve_orders_using(
        orders(&[(L0, walk(5, 2)), (R0, hold(6, 4))]),
        using(Side::Left, &[(0, (4, 2))]),
    );
    assert!(kinds(&report).contains(&&EventKind::OrderDropped {
        side: Side::Left,
        index: 0
    }));
    assert_eq!(at(&m, L0), (3, 2));
    assert_eq!(m.fighter(L0).carrying, Some(Item::Twig));
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
fn a_press_is_settled_once_a_turn() {
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[(L0, press(R0)), (R0, walk(1, 2))]));
    let presses = kinds(&report)
        .iter()
        .filter(|k| matches!(k, EventKind::Pressed { .. }))
        .count();
    assert_eq!(presses, 1, "one press, all turn");
}

#[test]
fn a_creature_with_a_nervous_streak_stays_put_after_a_press_that_landed() {
    let mut teams = rosters();
    teams[0][0] = with_tension(plain(1), formiga_core::Tension::BraveButNervous);
    let mut m = board_with(field(), teams, &[(L0, (0, 2)), (R0, (1, 2))]);
    m.resolve_orders(orders(&[(L0, press(R0)), (R0, walk(4, 2))]));
    assert!(m.fighter(R0).interrupted);
    assert_eq!(at(&m, L0), (0, 2));
}

#[test]
fn a_creature_with_a_nervous_streak_hops_back_when_its_press_is_turned_away() {
    let mut teams = rosters();
    teams[0][0] = with_tension(plain(1), formiga_core::Tension::BraveButNervous);
    let mut m = board_with(field(), teams, &[(L0, (0, 2)), (R0, (1, 2))]);
    let report = m.resolve_orders(orders(&[(L0, press(R0)), (R0, facing(1, 2, Facing::West))]));
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
    let mut m = board(&[(L0, (0, 2)), (R0, (1, 2))]);
    m.board.fighters[1][0].composure = 1;
    let report = m.resolve_orders(orders(&[(L0, press(R0)), (R0, press(L0))]));
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
        (L0, press(L1)),
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
