//! The map event and found things: the sprinkler and its taps, items turning up, being picked up,
//! dropped and used, and what they do to each other.

mod common;
use common::*;
use formiga_battle_core::*;

fn tap(x: i8, y: i8) -> Order {
    Order::Contest {
        target: Target::Lever {
            at: Tile::new(x, y),
        },
    }
}

fn use_on(x: i8, y: i8) -> Order {
    Order::Use {
        at: Tile::new(x, y),
    }
}

fn carry(m: &mut Match, slot: Slot, item: Item) {
    m.board.fighters[slot.side.index()][slot.index as usize].carrying = Some(item);
}

fn landed(report: &TurnReport) -> Option<(Vec<Slot>, Vec<Slot>)> {
    report.events.iter().find_map(|e| match &e.kind {
        EventKind::SceneryLanded { hit, sheltered, .. } => Some((hit.clone(), sheltered.clone())),
        _ => None,
    })
}

#[test]
fn the_sprinkler_is_warned_a_whole_turn_ahead_and_soaks_whoever_is_in_the_yard() {
    // Each team holds an arm of the yard, keeping it contested; L1 waits just outside.
    let placed = [(L0, (3, 3)), (L1, (6, 1)), (R0, (5, 3))];
    let mut m = board_on(Arena::backyard(), &placed);
    let given = orders(&[(L0, hold(3, 3)), (L1, hold(6, 1)), (R0, hold(5, 3))]);
    for _ in 0..2 {
        m.resolve_orders(given);
    }
    let third = m.resolve_orders(given);
    let warned = third.events.iter().find_map(|e| match &e.kind {
        EventKind::SceneryWarned {
            lands_turn,
            lands_tick,
            ..
        } => Some((e.turn, *lands_turn, *lands_tick)),
        _ => None,
    });
    assert_eq!(warned, Some((3, 4, 2)));
    assert!(interpret::warned(&m, Tile::new(3, 3)));
    assert!(!interpret::warned(&m, Tile::new(6, 1)));
    let fourth = m.resolve_orders(given);
    assert_eq!(landed(&fourth), Some((vec![L0, R0], vec![])));
    assert!(!m.fighter(L0).holding && !m.fighter(R0).holding);
    assert!(m.fighter(L1).holding);
    assert_eq!(composure(&m, L0), 3, "the sprinkler costs nobody composure");
}

#[test]
fn a_leaf_umbrella_keeps_the_sprinkler_off() {
    let mut m = board_on(
        Arena::backyard(),
        &[(L0, (3, 3)), (L1, (3, 4)), (R0, (8, 1))],
    );
    carry(&mut m, L1, Item::Leaf);
    m.board.turn = 3;
    // The sprinkler is coming at the start of turn 4's second tick; the leaf opens a tick after
    // it is used, in the first.
    m.board.event_at = Some(4 * 3 + 2);
    m.board.event_tiles = m.arena.scenery_tiles(&[]);
    let report = m.resolve_orders(orders(&[(L0, hold(3, 3)), (L1, use_on(3, 3))]));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::Sheltering { slots, .. } if slots == &vec![L0]))
    );
    assert_eq!(landed(&report), Some((vec![], vec![L0])));
    assert!(m.fighter(L0).holding);
}

#[test]
fn turning_a_tap_brings_the_sprinkler_a_tick_later_and_then_the_tap_rests() {
    // L0 stands beside the left tap; R0 holds an arm of the yard.
    let mut m = board_on(Arena::backyard(), &[(L0, (1, 5)), (R0, (5, 3))]);
    let report = m.resolve_orders(orders(&[(L0, tap(1, 6)), (R0, hold(5, 3))]));
    let turned = report
        .events
        .iter()
        .position(|e| matches!(e.kind, EventKind::SwitchTurned { slot: L0, .. }))
        .expect("the tap turns");
    let soaked = report
        .events
        .iter()
        .position(|e| matches!(e.kind, EventKind::SceneryLanded { .. }))
        .expect("the sprinkler comes");
    assert_eq!(
        report.events[soaked].tick,
        report.events[turned].tick + 1,
        "a tick after the tap"
    );
    assert_eq!(landed(&report), Some((vec![R0], vec![])));
    let again = m.resolve_orders(orders(&[(L0, tap(1, 6)), (R0, hold(5, 3))]));
    assert!(
        kinds(&again)
            .iter()
            .any(|k| matches!(k, EventKind::SwitchResting { .. }))
    );
}

#[test]
fn holding_beside_a_tap_shoos_off_whoever_turns_it() {
    let mut m = board_on(Arena::backyard(), &[(L0, (1, 5)), (R0, (2, 6))]);
    let report = m.resolve_orders(orders(&[(L0, tap(1, 6)), (R0, hold(2, 6))]));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::SwitchDefended { slot: L0, .. }))
    );
    assert_eq!(composure(&m, L0), 2);
    assert_eq!(m.board.event_at, None);
}

#[test]
fn found_things_glint_a_turn_ahead_then_turn_up_on_both_spots_of_a_mirrored_pair() {
    let mut m = board_on(Arena::backyard(), &[(L0, (0, 1)), (R0, (8, 1))]);
    let first = m.resolve_orders(orders(&[(L0, hold(0, 1)), (R0, hold(8, 1))]));
    let (spots, item) = first
        .events
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::ItemsGlint {
                spots,
                item,
                appears_turn: 2,
            } => Some((spots.clone(), *item)),
            _ => None,
        })
        .expect("something glints at the end of turn 1");
    assert_eq!(spots.len(), 2);
    assert_eq!(spots[1], m.arena.mirror(spots[0]));
    m.resolve_orders(orders(&[(L0, hold(0, 1)), (R0, hold(8, 1))]));
    assert_eq!(m.board.ground, vec![(spots[0], item), (spots[1], item)]);
}

#[test]
fn stepping_onto_something_picks_it_up_and_being_caught_on_the_move_drops_it() {
    // The only quickest way from (1, 2) to (3, 2) is over (2, 2).
    let mut m = board(&[(L0, (1, 2)), (R0, (6, 4))]);
    m.board.ground.push((Tile::new(2, 2), Item::Twig));
    let report = m.resolve_orders(orders(&[(L0, walk(3, 2)), (R0, hold(6, 4))]));
    assert!(kinds(&report).iter().any(|k| matches!(
        k,
        EventKind::PickedUp {
            slot: L0,
            item: Item::Twig,
            ..
        }
    )));
    assert_eq!(m.fighter(L0).carrying, Some(Item::Twig));
    assert!(m.board.ground.is_empty());

    // Caught before it can step, it lets go where it stands.
    let mut m = board(&[(L0, (1, 2)), (R0, (1, 3))]);
    carry(&mut m, L0, Item::Puff);
    let report = m.resolve_orders(orders(&[(L0, walk(5, 2)), (R0, contest(L0))]));
    assert!(kinds(&report).iter().any(|k| matches!(
        k,
        EventKind::Dropped {
            slot: L0,
            item: Item::Puff,
            at: Some(_)
        }
    )));
    assert_eq!(m.fighter(L0).carrying, None);
    assert_eq!(m.board.ground, vec![(Tile::new(1, 2), Item::Puff)]);
}

#[test]
fn a_puff_bowls_over_whoever_stands_across_its_way_but_not_two_holders_side_by_side() {
    // L0 blows rightwards at (3, 2): the tiles (3, 1), (3, 2) and (3, 3) are blown on to x = 4.
    let mut m = board(&[(L0, (1, 2)), (R0, (3, 2)), (R1, (3, 3))]);
    carry(&mut m, L0, Item::Puff);
    let report = m.resolve_orders(orders(&[
        (L0, use_on(3, 2)),
        (R0, walk(3, 2)),
        (R1, walk(3, 3)),
    ]));
    let blown = report
        .events
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::Puffed { blown, .. } => Some(blown.clone()),
            _ => None,
        })
        .expect("the puff lands");
    assert_eq!(blown.len(), 2);
    assert_eq!(at(&m, R0), (4, 2));
    assert_eq!(at(&m, R1), (4, 3));
    assert!(m.fighter(R0).interrupted && m.fighter(R1).interrupted);

    let mut m = board(&[(L0, (1, 2)), (R0, (3, 2)), (R1, (3, 3))]);
    carry(&mut m, L0, Item::Puff);
    let report = m.resolve_orders(orders(&[
        (L0, use_on(3, 2)),
        (R0, hold(3, 2)),
        (R1, hold(3, 3)),
    ]));
    assert!(kinds(&report).iter().any(
        |k| matches!(k, EventKind::Puffed { blown, braced, .. } if blown.is_empty() && braced.len() == 2)
    ));
    assert_eq!(at(&m, R0), (3, 2));
}

#[test]
fn a_puff_catches_a_leaf_umbrella_like_a_sail() {
    let mut m = board(&[(L0, (1, 2)), (R0, (3, 2))]);
    carry(&mut m, L0, Item::Puff);
    m.board.fighters[1][0].shelter_until = 100;
    m.resolve_orders(orders(&[(L0, use_on(3, 2)), (R0, walk(3, 2))]));
    assert_eq!(at(&m, R0), (5, 2));
}

#[test]
fn a_twig_blocks_its_tile_for_two_turns_and_keeps_an_open_gate_from_closing() {
    let mut m = board(&[(L0, (1, 2)), (R0, (6, 4))]);
    carry(&mut m, L0, Item::Twig);
    m.resolve_orders(orders(&[(L0, use_on(3, 2)), (R0, hold(6, 4))]));
    assert!(!m.walkable(Tile::new(3, 2)));
    m.resolve_orders(orders(&[(L0, hold(1, 2)), (R0, hold(6, 4))]));
    m.resolve_orders(orders(&[(L0, hold(1, 2)), (R0, hold(6, 4))]));
    assert!(!m.walkable(Tile::new(3, 2)), "still there through turn 3");
    let report = m.resolve_orders(orders(&[(L0, hold(1, 2)), (R0, hold(6, 4))]));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::TwigFell { .. }))
    );
    assert!(m.walkable(Tile::new(3, 2)));

    // Backyard's gate, open, with a twig wedged in it, cannot swing shut.
    let mut m = board_on(Arena::backyard(), &[(L0, (4, 5)), (R0, (8, 1))]);
    m.board.gates_open = vec![true];
    m.board.twigs.push((Tile::new(4, 0), 100));
    let report = m.resolve_orders(orders(&[(L0, tap(4, 6)), (R0, hold(8, 1))]));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::GateWedged { .. }))
    );
    assert_eq!(m.board.gates_open, vec![true]);
}

#[test]
fn a_contest_bounces_off_a_leaf_umbrella_and_hurts_nobody() {
    let mut m = board(&[(L0, (1, 2)), (R0, (2, 2))]);
    m.board.fighters[0][0].shelter_until = 100;
    let report = m.resolve_orders(orders(&[(L0, walk(1, 4)), (R0, contest(L0))]));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::Sheltered { target: L0, .. }))
    );
    assert_eq!(composure(&m, L0), 3);
    assert_eq!(composure(&m, R0), 3);
}

#[test]
fn a_cautious_creature_goes_round_ground_marked_for_the_sprinkler_and_a_bold_one_walks_on() {
    let leaning = |boldness: u16, suspicion: u16| Profile {
        nature: Nature {
            boldness,
            suspicion,
            impulsiveness: 0,
            ..Nature::MIDDLING
        },
        ..plain(1)
    };
    // Walking along the yard's middle row, the arm (3, 3) is on the quickest way.
    let walk_past = |profile: Profile| {
        let rosters = [
            vec![profile, plain(2), plain(3)],
            vec![plain(4), plain(5), plain(6)],
        ];
        let mut m = board_with(Arena::backyard(), rosters, &[(L0, (1, 3)), (R0, (8, 6))]);
        m.board.event_at = Some(100);
        m.board.event_tiles = m.arena.scenery_tiles(&[]);
        m.resolve_orders(orders(&[(L0, walk(4, 3)), (R0, hold(8, 6))]))
    };
    let wary = walk_past(leaning(0, 1000));
    assert!(kinds(&wary).iter().any(|k| matches!(
        k,
        EventKind::Stepped {
            slot: L0,
            reason: Reason::SkirtedWarning,
            ..
        }
    )));
    let bold = walk_past(leaning(1000, 0));
    assert!(kinds(&bold).iter().any(|k| matches!(
        k,
        EventKind::Stepped {
            slot: L0,
            to: Tile { x: 3, y: 3 },
            ..
        }
    )));
}

#[test]
fn a_match_with_the_sprinkler_and_found_things_replays_the_same() {
    let mut m = Match::new(Arena::backyard(), Ruleset::default(), 11, rosters()).unwrap();
    let mut seen_items = false;
    while m.outcome().is_none() {
        let plans = [
            ai::Coach::Exhibition.plan(&m, Side::Left),
            ai::Coach::Exhibition.plan(&m, Side::Right),
        ];
        let report = m.resolve_turn(plans);
        seen_items |= report
            .events
            .iter()
            .any(|e| matches!(e.kind, EventKind::ItemAppeared { .. }));
    }
    assert!(seen_items);
    let (replayed, _) = m.record().replay().expect("replays");
    assert_eq!(replayed.hash(), m.hash());
}

/// The map event coming next tick on these tiles, as if warned or set off.
fn coming(m: &mut Match, tiles: Vec<Tile>, from: Vec<Tile>) {
    m.board.event_at = Some(u16::from(m.board.turn + 1) * 3 + 1);
    m.board.event_tiles = tiles;
    m.board.event_from = from;
}

fn moved(report: &TurnReport) -> Vec<Blown> {
    report
        .events
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::SceneryLanded { moved, .. } => Some(moved.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn a_wave_washes_whoever_stands_on_the_sandbars_ends_toward_the_middle_of_the_beach() {
    let arena = Arena::by_id("cove").unwrap();
    let tiles = arena.scenery_tiles(&[]);
    let mut m = board_on(arena, &[(L0, (8, 1)), (R0, (4, 1))]);
    coming(&mut m, tiles, Vec::new());
    let report = m.resolve_orders(orders(&[(L0, hold(8, 1)), (R0, hold(4, 1))]));
    assert_eq!(at(&m, R0), (4, 2));
    assert_eq!(moved(&report).len(), 1);
    assert!(m.fighter(R0).interrupted && !m.fighter(R0).holding);
    assert_eq!(composure(&m, R0), 3);
}

#[test]
fn the_pond_rises_over_the_jettys_sides_for_a_turn_washing_off_whoever_is_there() {
    let arena = Arena::by_id("millpond").unwrap();
    let tiles = arena.scenery_tiles(&[]);
    let mut m = board_on(arena, &[(L0, (3, 3)), (R0, (8, 5))]);
    coming(&mut m, tiles, Vec::new());
    m.resolve_orders(orders(&[(L0, hold(3, 3)), (R0, hold(8, 5))]));
    assert_eq!(at(&m, L0), (3, 2));
    assert!(!m.walkable(Tile::new(3, 3)) && !m.walkable(Tile::new(5, 3)));
    m.resolve_orders(orders(&[(L0, hold(3, 2)), (R0, hold(8, 5))]));
    assert!(!m.walkable(Tile::new(3, 3)), "under through the turn after");
    let report = m.resolve_orders(orders(&[(L0, hold(3, 2)), (R0, hold(8, 5))]));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::GroundReopened { .. }))
    );
    assert!(m.walkable(Tile::new(3, 3)));
}

#[test]
fn a_twig_thrown_in_the_millpond_floats_as_a_raft_anyone_can_stand_on() {
    let mut m = board_on(
        Arena::by_id("millpond").unwrap(),
        &[(L0, (2, 2)), (R0, (8, 5))],
    );
    carry(&mut m, L0, Item::Twig);
    assert!(!m.walkable(Tile::new(2, 1)));
    let report = m.resolve_orders(orders(&[(L0, use_on(2, 1)), (R0, hold(8, 5))]));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::RaftFloated { .. }))
    );
    assert!(m.walkable(Tile::new(2, 1)));
}

#[test]
fn shaking_a_tree_bonks_whoever_stands_round_it_and_drops_a_twig_beside_it() {
    let mut m = board_on(
        Arena::by_id("woodland").unwrap(),
        &[(L0, (2, 2)), (R0, (0, 1))],
    );
    let report = m.resolve_orders(orders(&[(L0, tap(1, 2)), (R0, hold(0, 1))]));
    let (hit, _) = landed(&report).expect("the acorns fall");
    assert_eq!(hit, vec![L0, R0]);
    assert!(m.board.ground.contains(&(Tile::new(1, 1), Item::Twig)));
    let mirrored = m.arena.scenery_tiles(&[Tile::new(7, 2)]);
    assert!(mirrored.contains(&Tile::new(8, 1)) && mirrored.contains(&Tile::new(6, 2)));
}

#[test]
fn a_pebble_bell_spoils_a_contest_not_yet_settled_nearby() {
    // R0 needs two steps to reach L1; the bell rings before it gets there.
    let mut m = board(&[(L0, (3, 3)), (L1, (2, 2)), (R0, (5, 2))]);
    carry(&mut m, L0, Item::Bell);
    let report = m.resolve_orders(orders(&[
        (L0, use_on(3, 3)),
        (L1, hold(2, 2)),
        (R0, contest(L1)),
    ]));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::Rung { hushed, .. } if hushed == &vec![R0]))
    );
    assert!(
        !kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::Countered { .. }))
    );
    assert_eq!(composure(&m, R0), 3);
}

#[test]
fn a_pebble_bell_rung_beside_a_tap_sets_the_sprinkler_off() {
    let mut m = board_on(Arena::backyard(), &[(L0, (1, 5)), (R0, (8, 1))]);
    carry(&mut m, L0, Item::Bell);
    let report = m.resolve_orders(orders(&[(L0, use_on(1, 5)), (R0, hold(8, 1))]));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::SwitchTurned { slot: L0, .. }))
    );
}

#[test]
fn honey_stops_whoever_steps_in_and_whoever_is_pushed_onto_it() {
    let mut m = board(&[(L0, (0, 2)), (R0, (6, 4))]);
    m.board.honey.push((Tile::new(2, 2), 100));
    let report = m.resolve_orders(orders(&[(L0, walk(5, 2)), (R0, hold(6, 4))]));
    assert_eq!(at(&m, L0), (2, 2));
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::Stuck { slot: L0, .. }))
    );

    // A sail's two tiles stop on the first honey.
    let mut m = board(&[(L0, (1, 2)), (R0, (3, 2))]);
    carry(&mut m, L0, Item::Puff);
    m.board.fighters[1][0].shelter_until = 100;
    m.board.honey.push((Tile::new(4, 2), 100));
    m.resolve_orders(orders(&[(L0, use_on(3, 2)), (R0, hold(3, 2))]));
    assert_eq!(at(&m, R0), (4, 2));
}

#[test]
fn a_springy_seed_hops_over_a_creature_but_never_over_a_wall() {
    let mut m = board(&[(L0, (1, 2)), (R0, (2, 2))]);
    carry(&mut m, L0, Item::Seed);
    m.resolve_orders(orders(&[(L0, use_on(3, 2)), (R0, hold(2, 2))]));
    assert_eq!(at(&m, L0), (3, 2));

    let walled = Arena::parse(
        "id: w\nname: W\n1 1 1 . . . Z\n. . . . . . .\n. . # . . . .\n. . . . . . .\n2 2 2 . . . .\n",
    )
    .unwrap();
    let mut m = board_on(walled, &[(L0, (1, 2)), (R0, (6, 4))]);
    carry(&mut m, L0, Item::Seed);
    let report = m.resolve_orders(orders(&[(L0, use_on(3, 2)), (R0, hold(6, 4))]));
    assert!(kinds(&report).iter().any(|k| matches!(
        k,
        EventKind::ItemFizzled {
            item: Item::Seed,
            ..
        }
    )));
    assert_eq!(at(&m, L0), (1, 2));
}
