//! The beacon, the gate and its lever, the fallback, and how a creature's nature shows in the way
//! it walks.

mod common;
use common::*;
use formiga_battle_core::*;

#[test]
fn holding_the_beacon_alone_for_two_turns_wins_by_capture() {
    let mut m = board(&[(L0, (6, 0)), (R0, (0, 4))]);
    m.resolve_turn(orders(&[(L0, hold(6, 0)), (R0, hold(0, 4))]));
    assert_eq!(m.board.capture, [3, 0]);
    assert_eq!(m.outcome(), None);
    m.resolve_turn(orders(&[(L0, hold(6, 0)), (R0, hold(0, 4))]));
    assert_eq!(
        m.outcome(),
        Some(Outcome {
            winner: Some(Side::Left),
            victory: Some(Victory::Capture),
            turn: 2
        })
    );
}

#[test]
fn anyone_of_the_other_team_in_the_zone_pauses_the_beacon() {
    let arena = Arena::backyard();
    let mut m = board_on(arena, &[(L0, (4, 3)), (R0, (5, 3))]);
    let report = m.resolve_turn(orders(&[(L0, hold(4, 3)), (R0, walk(5, 3))]));
    assert_eq!(m.board.capture, [0, 0]);
    assert!(kinds(&report).contains(&&EventKind::CaptureContested));
}

#[test]
fn two_holding_against_one_in_the_zone_control_it() {
    let arena = Arena::backyard();
    let mut m = board_on(arena, &[(L0, (4, 3)), (L1, (4, 2)), (R0, (5, 3))]);
    let given = orders(&[(L0, hold(4, 3)), (L1, hold(4, 2)), (R0, hold(5, 3))]);
    m.resolve_turn(given);
    assert_eq!(m.board.capture, [3, 0]);

    // Read the handoff's way, anyone of the other team in the zone at all pauses it.
    let mut m = board_on(
        Arena::backyard(),
        &[(L0, (4, 3)), (L1, (4, 2)), (R0, (5, 3))],
    );
    m.ruleset.capture_rule = CaptureRule::Uncontested;
    m.resolve_turn(given);
    assert_eq!(m.board.capture, [0, 0]);
}

#[test]
fn walking_through_the_zone_is_not_holding_it() {
    let arena = Arena::backyard();
    let mut m = board_on(arena, &[(L0, (4, 3)), (R0, (8, 1))]);
    m.resolve_turn(orders(&[(L0, walk(4, 3)), (R0, hold(8, 1))]));
    assert_eq!(m.board.capture, [0, 0]);
}

#[test]
fn a_pulled_lever_swings_the_gate_the_tick_after_and_everyone_saw_it_coming() {
    let arena = Arena::backyard();
    let gate = arena.gates[0];
    let lever = arena.levers[0];
    let mut m = board_on(arena, &[(L0, (3, 6)), (R0, (8, 1))]);
    assert!(!m.walkable(gate));
    let report = m.resolve_turn(orders(&[
        (
            L0,
            Order::Contest {
                target: Target::Lever { at: lever },
            },
        ),
        (R0, hold(8, 1)),
    ]));
    let pulled = report
        .events
        .iter()
        .find(|e| matches!(e.kind, EventKind::LeverPulled { .. }))
        .expect("the lever was pulled");
    let swung = report
        .events
        .iter()
        .find(|e| matches!(e.kind, EventKind::GateSwung { open: true, .. }))
        .expect("the gate swung");
    assert_eq!(pulled.tick + 1, swung.tick);
    assert!(matches!(
        pulled.kind,
        EventKind::LeverPulled {
            swings_turn: 1,
            swings_tick: 2,
            ..
        }
    ));
    assert!(m.walkable(gate));
    // It rests for a whole turn after.
    let report = m.resolve_turn(orders(&[
        (
            L0,
            Order::Contest {
                target: Target::Lever { at: lever },
            },
        ),
        (R0, hold(8, 1)),
    ]));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::LeverResting { .. }))
    );
}

#[test]
fn someone_holding_beside_the_lever_turns_a_pull_away() {
    let arena = Arena::backyard();
    let lever = arena.levers[0];
    let mut m = board_on(arena, &[(L0, (3, 6)), (R0, (5, 6))]);
    let report = m.resolve_turn(orders(&[
        (
            L0,
            Order::Contest {
                target: Target::Lever { at: lever },
            },
        ),
        (R0, hold(5, 6)),
    ]));
    assert_eq!(composure(&m, L0), 2);
    assert!(!m.walkable(m.arena.gates[0]));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::LeverDefended { .. }))
    );
}

#[test]
fn a_gate_cannot_close_on_someone_standing_in_it() {
    let arena = Arena::backyard();
    let lever = arena.levers[0];
    let gate = arena.gates[0];
    let mut m = board_on(arena, &[(L0, (3, 6)), (R0, (gate.x, gate.y))]);
    m.board.gates_open = vec![true];
    let report = m.resolve_turn(orders(&[
        (
            L0,
            Order::Contest {
                target: Target::Lever { at: lever },
            },
        ),
        (R0, hold(gate.x, gate.y)),
    ]));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::GateJammed { by: R0, .. }))
    );
    assert_eq!(m.board.gates_open, vec![true]);
}

#[test]
fn missing_orders_are_filled_for_both_teams_by_the_same_fallback() {
    let mut m = Match::new(Arena::backyard(), Ruleset::default(), 3, rosters()).unwrap();
    let report = m.resolve_turn([[None; TEAM_SIZE]; 2]);
    let filled = report
        .events
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                EventKind::OrderFilled {
                    fill: event::Fill::Missing,
                    ..
                }
            )
        })
        .count();
    assert_eq!(filled, 6);
    // From a mirrored start, the fallback sends both teams to mirrored tiles.
    let mirror = |t: Tile| Tile::new(m.arena.width as i8 - 1 - t.x, t.y);
    for i in 0..TEAM_SIZE {
        match (report.orders[0][i], report.orders[1][i]) {
            (Some(Order::Hold { at: left }), Some(Order::Hold { at: right })) => {
                assert_eq!(mirror(left), right)
            }
            other => panic!("expected two holds, got {other:?}"),
        }
    }
}

#[test]
fn the_fallback_holds_in_the_zone_and_a_bold_creature_contests_a_neighbour() {
    let mut bold = plain(1);
    bold.nature.boldness = 1000;
    let mut cautious = plain(2);
    cautious.nature.boldness = 0;
    cautious.nature.suspicion = 1000;
    let teams = [
        vec![bold, cautious, plain(3)],
        vec![plain(4), plain(5), plain(6)],
    ];
    let m = board_with(
        Arena::backyard(),
        teams,
        &[
            (L0, (1, 0)),
            (L1, (1, 6)),
            (L2, (4, 3)),
            (R0, (2, 0)),
            (R1, (2, 6)),
        ],
    );
    assert_eq!(ai::fallback(&m, L2), hold(4, 3));
    assert_eq!(ai::fallback(&m, L0), contest(R0));
    assert_eq!(ai::fallback(&m, L1), hold(1, 6));
}

/// The left creature walks from one side of the field to the other past a holder whose reach
/// covers the straight way.
fn walk_past_a_holder(profile: Profile) -> (Match, TurnReport) {
    let teams = [
        vec![profile, plain(2), plain(3)],
        vec![plain(4), plain(5), plain(6)],
    ];
    let mut m = board_with(field(), teams, &[(L0, (0, 2)), (R0, (2, 1))]);
    let report = m.resolve_turn(orders(&[(L0, walk(3, 2)), (R0, hold(2, 1))]));
    (m, report)
}

#[test]
fn a_cautious_creature_goes_round_out_of_reach_and_is_later_for_it() {
    let mut cautious = plain(1);
    cautious.nature.boldness = 0;
    cautious.nature.suspicion = 1000;
    let (m, report) = walk_past_a_holder(cautious);
    let passed: Vec<Tile> = report
        .events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Stepped { slot: L0, to, .. } => Some(to),
            _ => None,
        })
        .collect();
    assert!(
        !passed.contains(&Tile::new(2, 2)),
        "kept out of reach: {passed:?}"
    );
    assert_ne!(at(&m, L0), (3, 2), "and has not arrived yet");
    assert!(report.events.iter().any(|e| matches!(
        e.kind,
        EventKind::Stepped {
            reason: Reason::AvoidedExposure,
            ..
        }
    )));

    let (m, _) = walk_past_a_holder(plain(1));
    assert_eq!(
        at(&m, L0),
        (3, 2),
        "a creature that leans no way walks straight there"
    );
}

#[test]
fn with_no_detour_budget_even_the_most_cautious_creature_walks_the_quickest_way() {
    let mut cautious = plain(1);
    cautious.nature.boldness = 0;
    cautious.nature.suspicion = 1000;
    let teams = [
        vec![cautious, plain(2), plain(3)],
        vec![plain(4), plain(5), plain(6)],
    ];
    let mut m = board_with(field(), teams, &[(L0, (0, 2)), (R0, (2, 1))]);
    m.ruleset.detour_budget = 0;
    // A tick more than the walk, for the one it hangs back.
    m.ruleset.ticks_per_turn = 4;
    m.resolve_turn(orders(&[(L0, walk(3, 2)), (R0, hold(2, 1))]));
    assert_eq!(at(&m, L0), (3, 2));
}

fn wary() -> Profile {
    let mut p = plain(1);
    p.nature.boldness = 0;
    p.nature.suspicion = 1000;
    p
}

#[test]
fn a_wary_creature_hangs_back_one_tick_a_turn_before_walking_in_among_the_other_team() {
    let teams = [
        vec![wary(), plain(2), plain(3)],
        vec![plain(4), plain(5), plain(6)],
    ];
    let mut m = board_with(field(), teams, &[(L0, (1, 2)), (R0, (3, 2))]);
    m.ruleset.detour_budget = 0;
    let report = m.resolve_turn(orders(&[(L0, hold(2, 2)), (R0, hold(3, 2))]));
    let hung = report
        .events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::HungBack { slot: L0, .. }))
        .count();
    assert_eq!(hung, 1, "once a turn, and no more");
    assert_eq!(at(&m, L0), (2, 2), "and then it goes on");
}

#[test]
fn a_wary_creature_sent_to_contest_goes_without_hanging_back() {
    let teams = [
        vec![wary(), plain(2), plain(3)],
        vec![plain(4), plain(5), plain(6)],
    ];
    let mut m = board_with(field(), teams, &[(L0, (0, 2)), (R0, (3, 2))]);
    let report = m.resolve_turn(orders(&[(L0, contest(R0)), (R0, walk(3, 2))]));
    assert!(
        !report
            .events
            .iter()
            .any(|e| matches!(e.kind, EventKind::HungBack { .. }))
    );
    assert_eq!(composure(&m, R0), 2);
}

#[test]
fn a_creature_that_is_not_wary_never_hangs_back() {
    let mut m = board(&[(L0, (1, 2)), (R0, (3, 2))]);
    let report = m.resolve_turn(orders(&[(L0, hold(2, 3)), (R0, hold(3, 2))]));
    assert!(
        !report
            .events
            .iter()
            .any(|e| matches!(e.kind, EventKind::HungBack { .. }))
    );
}
