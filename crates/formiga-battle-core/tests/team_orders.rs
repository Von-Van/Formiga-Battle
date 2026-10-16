//! Team orders: a Handler gives the whole team TAKE, DEFEND and INTERACT, and the team shares
//! them out by itself.

mod common;

use common::*;
use formiga_battle_core::*;

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

/// Each creature's job this turn, as the team shared out the plan.
fn jobs(report: &TurnReport) -> Vec<(Slot, Task, Order, Why)> {
    report
        .events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Assigned {
                slot,
                task,
                order,
                why,
            } => Some((slot, task, order, why)),
            _ => None,
        })
        .collect()
}

fn job_of(report: &TurnReport, slot: Slot) -> (Task, Order, Why) {
    jobs(report)
        .into_iter()
        .find(|(s, ..)| *s == slot)
        .map(|(_, t, o, w)| (t, o, w))
        .expect("every creature on the board takes a job")
}

/// A creature that leans hard toward meeting the other team.
fn bold(id: u64) -> Profile {
    let mut p = plain(id);
    p.nature.boldness = 1000;
    p.nature.feistiness = 1000;
    p.nature.suspicion = 0;
    p
}

/// A creature that would much rather stand and hold.
fn wary(id: u64) -> Profile {
    let mut p = plain(id);
    p.nature.boldness = 0;
    p.nature.suspicion = 1000;
    p.nature.feistiness = 0;
    p
}

#[test]
fn a_take_sends_whoever_is_nearest_to_hold_an_empty_square() {
    let mut m = board(&[(L0, (1, 0)), (L1, (0, 4)), (R0, (6, 4))]);
    let report = m.resolve_turn([vec![take(&[(3, 0)])], Vec::new()]);
    let (task, order, _) = job_of(&report, L0);
    assert_eq!(task.role, Role::Claim);
    assert_eq!(task.directive, Some(0));
    assert_eq!(order, hold(3, 0));
    assert_ne!(job_of(&report, L1).0.role, Role::Claim);
    assert!(m.fighter(L0).holding);
}

#[test]
fn naming_a_square_twice_brings_a_second_to_guard_it() {
    let mut m = board(&[(L0, (2, 2)), (L1, (1, 3)), (R0, (6, 4))]);
    let report = m.resolve_turn([vec![take(&[(3, 2)]), defend(&[(3, 2)])], Vec::new()]);
    let roles: Vec<Role> = [L0, L1].map(|s| job_of(&report, s).0.role).to_vec();
    assert!(roles.contains(&Role::Claim), "{roles:?}");
    assert!(roles.contains(&Role::Guard), "{roles:?}");
}

#[test]
fn taking_a_square_one_of_theirs_holds_sends_two_to_flank_it() {
    let mut m = board(&[(L0, (3, 1)), (L1, (3, 3)), (R0, (3, 2))]);
    m.board.fighters[1][0].holding = true;
    let report = m.resolve_turn([vec![take(&[(3, 2)])], vec![defend(&[(3, 2)])]]);
    for slot in [L0, L1] {
        let (task, order, _) = job_of(&report, slot);
        assert_eq!(task.role, Role::Flank);
        assert_eq!(order, contest(R0), "both in place: they contest together");
    }
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::Flanked { target: R0, .. }))
    );
}

#[test]
fn interacting_with_something_on_the_ground_fetches_it() {
    let mut m = board(&[(L0, (0, 2)), (L1, (6, 0)), (R0, (6, 4))]);
    m.board.ground.push((Tile::new(2, 2), Item::Puff));
    let report = m.resolve_turn([vec![interact(2, 2)], Vec::new()]);
    let (task, order, _) = job_of(&report, L0);
    assert_eq!(task.role, Role::Fetch);
    assert_eq!(order, walk(2, 2));
    assert_eq!(m.fighter(L0).carrying, Some(Item::Puff));
}

#[test]
fn interacting_with_one_of_ours_carrying_something_has_it_used() {
    let mut m = board(&[(L0, (2, 2)), (R0, (3, 2))]);
    m.board.fighters[0][0].carrying = Some(Item::Puff);
    let report = m.resolve_turn([vec![interact(2, 2)], Vec::new()]);
    let (task, order, _) = job_of(&report, L0);
    assert_eq!(task.role, Role::Use);
    assert_eq!(
        order,
        Order::Use {
            at: Tile::new(3, 2)
        }
    );
}

#[test]
fn between_two_jobs_just_as_near_a_bold_one_meets_the_other_team_and_a_wary_one_holds() {
    let rosters = [
        vec![bold(1), wary(2), plain(3)],
        vec![plain(4), plain(5), plain(6)],
    ];
    // Each is as far from the empty square as from the one of theirs walking about.
    for (b, w) in [(L0, L1), (L1, L0)] {
        let mut m = board_with(
            field(),
            rosters.clone(),
            &[(b, (2, 2)), (w, (4, 2)), (R0, (3, 0))],
        );
        let report = m.resolve_turn([vec![take(&[(3, 4)]), take(&[(3, 0)])], Vec::new()]);
        let bold_job = job_of(&report, L0);
        let wary_job = job_of(&report, L1);
        assert_eq!(bold_job.0.role, Role::Oust, "{bold_job:?}");
        assert_eq!(wary_job.0.role, Role::Claim, "{wary_job:?}");
    }
}

#[test]
fn who_a_creature_is_never_outweighs_a_job_only_it_is_near() {
    let rosters = [
        vec![bold(1), wary(2), plain(3)],
        vec![plain(4), plain(5), plain(6)],
    ];
    // The wary one is next to the one of theirs; the bold one is across the field.
    let mut m = board_with(
        field(),
        rosters,
        &[(L0, (0, 4)), (L1, (5, 1)), (R0, (6, 1))],
    );
    let report = m.resolve_turn([vec![take(&[(6, 1)])], Vec::new()]);
    assert_eq!(job_of(&report, L1).0.role, Role::Oust);
}

#[test]
fn a_creature_whose_find_is_taken_first_thinks_again() {
    let mut m = board(&[(L0, (0, 2)), (L1, (0, 4)), (R0, (3, 1))]);
    m.board.ground.push((Tile::new(3, 2), Item::Twig));
    let report = m.resolve_turn([vec![interact(3, 2)], vec![interact(3, 2)]]);
    assert!(
        kinds(&report)
            .iter()
            .any(|k| matches!(k, EventKind::PickedUp { slot: R0, .. })),
        "{:?}",
        kinds(&report)
    );
    assert!(
        kinds(&report).iter().any(|k| matches!(
            k,
            EventKind::Rethought {
                because: Because::ItemGone,
                ..
            }
        )),
        "{:?}",
        kinds(&report)
    );
}

#[test]
fn with_no_plan_every_creature_falls_back_on_its_own() {
    let mut m = board(&[(L0, (0, 2)), (L1, (0, 4)), (R0, (6, 4))]);
    let report = m.resolve_turn([Vec::new(), Vec::new()]);
    let filled = kinds(&report)
        .iter()
        .filter(|k| matches!(k, EventKind::OrderFilled { .. }))
        .count();
    assert_eq!(filled, 3);
    assert!(jobs(&report).is_empty());
}

#[test]
fn orders_past_three_or_with_nothing_to_aim_at_are_left_out() {
    let mut m = board(&[(L0, (0, 2)), (R0, (6, 4))]);
    let report = m.resolve_turn([
        vec![
            take(&[(1, 1)]),
            interact(2, 2),
            take(&[(1, 2)]),
            take(&[(1, 3)]),
        ],
        Vec::new(),
    ]);
    let dropped: Vec<u8> = kinds(&report)
        .iter()
        .filter_map(|k| match k {
            EventKind::OrderDropped { index, .. } => Some(*index),
            _ => None,
        })
        .collect();
    assert_eq!(dropped, vec![1, 3]);
}

#[test]
fn a_plan_replays_exactly_from_its_record() {
    let mut m = board(&[(L0, (0, 2)), (L1, (0, 4)), (R0, (6, 4)), (R1, (6, 0))]);
    m.resolve_turn([
        vec![take(&[(6, 0)]), defend(&[(3, 2)])],
        vec![take(&[(3, 2)])],
    ]);
    m.resolve_turn([vec![interact(3, 2)], vec![defend(&[(6, 0), (5, 0)])]]);
    let record = m.record();
    let json = serde_json::to_string(&record).unwrap();
    let back: MatchRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(back.turns, record.turns);
}
