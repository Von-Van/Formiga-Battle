//! Playbooks compile to ordinary team plans: never an order the team could not carry out, never
//! more than three, the same plan from the same board, a record of every turn, and a match that
//! replays from what was sent like any other.

use formiga_battle_core::ai::Coach;
use formiga_battle_core::muster;
use formiga_battle_core::playbook::{
    self, Advance, Deviation, Fallback, Intent, Pilot, Playbook, PlaybookError, Runner, Stage,
    Timing,
};
use formiga_battle_core::tournament::{self, drawn_creature};
use formiga_battle_core::*;

fn rosters(seed: u64) -> [Vec<Profile>; 2] {
    [
        (0..3).map(|i| drawn_creature(seed, i)).collect(),
        (3..6).map(|i| drawn_creature(seed, i)).collect(),
    ]
}

/// Every sample, one stage at a time and all at once.
fn every_playbook() -> Vec<Playbook> {
    Playbook::samples()
        .into_iter()
        .flat_map(|p| [p.clone(), p.bundled()])
        .chain(Intent::ALL.map(|i| Playbook::new(i.label(), &[Stage::new(i)])))
        .collect()
}

/// A playbook on the left against `coach` on the right, checking every plan before it is sent.
fn play_checked(
    arena: &Arena,
    seed: u64,
    book: &Playbook,
    coach: Coach,
) -> (Match, Vec<TurnReport>, Runner) {
    let mut runner = Runner::new(book.clone(), Side::Left);
    let (m, reports) = tournament::play_by(arena, Ruleset::default(), seed, rosters(seed), {
        let runner = &mut runner;
        move |m, side| match side {
            Side::Left => {
                let plan = runner.plan(m);
                let tactical = plan.iter().filter(|d| d.tactical()).count();
                assert!(tactical <= PLAN_SIZE, "{}: {plan:?}", book.name);
                let (_, dropped) = muster::checked(m, side, &plan);
                assert!(
                    dropped.is_empty(),
                    "{} dropped {dropped:?}: {plan:?}",
                    book.name
                );
                let (_, unused) = muster::uses(m, side, &plan);
                assert!(unused.is_empty(), "{} use dropped {unused:?}", book.name);
                assert!(
                    plan.iter().any(Directive::tactical) || m.team(side).count() == 0,
                    "{} gave no orders on turn {}",
                    book.name,
                    m.board.turn + 1
                );
                plan
            }
            Side::Right => coach.plan(m, side),
        }
    });
    (m, reports, runner)
}

#[test]
fn every_sample_is_a_valid_playbook() {
    for p in every_playbook() {
        assert_eq!(p.validate(), Ok(()), "{}", p.name);
    }
    let stage = Stage::new(Intent::Secure);
    assert_eq!(
        Playbook::new("", &[stage]).validate(),
        Err(PlaybookError::Name)
    );
    assert_eq!(
        Playbook::new("Too many", &[stage; 4]).validate(),
        Err(PlaybookError::Stages)
    );
    assert_eq!(
        Playbook::new("None", &[]).validate(),
        Err(PlaybookError::Stages)
    );
    assert_eq!(
        Playbook::new("Long", &[stage, stage.turns(9)]).validate(),
        Err(PlaybookError::Turns(2))
    );
}

#[test]
fn a_playbook_only_ever_gives_orders_the_team_can_carry_out() {
    for arena in Arena::built_in() {
        for book in every_playbook() {
            for seed in 1..=3u64 {
                for coach in [Coach::Exhibition, Coach::Turtle] {
                    let (m, reports, runner) = play_checked(&arena, seed, &book, coach);
                    assert!(m.outcome().is_some());
                    assert_eq!(runner.log.len(), reports.len(), "a record for every turn");
                }
            }
        }
    }
}

#[test]
fn the_same_board_and_playbook_give_the_same_match() {
    let arena = Arena::backyard();
    for book in every_playbook() {
        let (a, ra, la) = play_checked(&arena, 7, &book, Coach::Exhibition);
        let (b, rb, lb) = play_checked(&arena, 7, &book, Coach::Exhibition);
        assert_eq!(a.hash(), b.hash(), "{}", book.name);
        assert_eq!(ra, rb, "{}", book.name);
        assert_eq!(la.log, lb.log, "{}", book.name);
    }
}

#[test]
fn a_playbook_match_replays_from_what_was_sent() {
    for arena in Arena::built_in() {
        let book = &Playbook::samples()[0];
        let (m, reports, _) = play_checked(&arena, 11, book, Coach::Familiar);
        let (again, replayed) = m.record().replay().expect("replays");
        assert_eq!(again.hash(), m.hash());
        assert_eq!(
            replayed.iter().map(|r| r.hash).collect::<Vec<_>>(),
            reports.iter().map(|r| r.hash).collect::<Vec<_>>()
        );
    }
}

#[test]
fn every_turn_says_which_stage_ran_and_why_it_moved_on() {
    for arena in Arena::built_in() {
        for book in Playbook::samples() {
            for seed in 1..=4u64 {
                let (_, _, runner) = play_checked(&arena, seed, &book, Coach::Exhibition);
                let mut last: Option<u8> = Some(0);
                for (k, c) in runner.log.iter().enumerate() {
                    assert_eq!(c.turn as usize, k + 1);
                    assert_eq!(c.side, Side::Left);
                    assert_eq!(c.timing, Timing::Successive);
                    assert_eq!(c.ran.len(), 1, "one stage at a time");
                    let ran = c.ran[0];
                    match ran.stage {
                        Some(s) => {
                            assert!((s as usize) < book.stages.len());
                            assert_eq!(ran.intent, book.stages[s as usize].intent);
                            assert_ne!(ran.deviation, Some(Deviation::Finished));
                        }
                        None => assert_eq!(ran.deviation, Some(Deviation::Finished)),
                    }
                    // The stage changes only with a reason given, and a reason is given only when
                    // it moves on from the one it was on.
                    if ran.stage != last {
                        assert!(!c.advanced.is_empty(), "turn {}: {c:?}", c.turn);
                    }
                    for a in &c.advanced {
                        let from = match a {
                            Advance::Met { stage } | Advance::RanOut { stage, .. } => *stage,
                        };
                        assert_eq!(Some(from), last, "{c:?}");
                        if let Advance::RanOut { fallback, .. } = a {
                            assert_eq!(*fallback, book.stages[from as usize].fallback);
                        }
                    }
                    let line = c.line(&book);
                    assert!(line.starts_with(&format!("Turn {}: {}", c.turn, book.name)));
                    last = ran.stage;
                }
            }
        }
    }
}

#[test]
fn a_stage_holds_until_its_goal_or_its_turns_and_then_takes_its_fallback() {
    // Fetch on the Backyard before anything has turned up has nothing to aim at, so it never gets
    // its goal on the first turns: it runs out after its turns, then goes again.
    let book = Playbook::new(
        "Wait for it",
        &[Stage::new(Intent::Fetch).turns(1).fallback(Fallback::Again)],
    );
    let (_, _, runner) = play_checked(&Arena::backyard(), 3, &book, Coach::Turtle);
    let first = &runner.log[0];
    assert_eq!(first.ran[0].stage, Some(0));
    assert_eq!(first.ran[0].deviation, Some(Deviation::NoTarget));
    assert!(first.advanced.is_empty(), "the first turn always plays");
    let second = &runner.log[1];
    assert_eq!(
        second.advanced,
        vec![Advance::RanOut {
            stage: 0,
            fallback: Fallback::Again
        }]
    );
    assert_eq!(second.ran[0].stage, Some(0));
}

#[test]
fn bundled_stages_share_the_turn() {
    let book = Playbook::samples()[0].clone().bundled();
    let (_, _, runner) = play_checked(&Arena::backyard(), 5, &book, Coach::Exhibition);
    for c in &runner.log {
        assert_eq!(c.timing, Timing::Bundled);
        assert!(c.advanced.is_empty());
        assert!(!c.ran.is_empty() && c.ran.len() <= book.stages.len());
        assert_eq!(c.ran[0].stage, Some(0), "the first stage always has a hand");
    }
}

#[test]
fn a_comparison_counts_every_match_from_the_playbooks_side() {
    let book = Pilot::Playbook(Playbook::samples()[1].clone());
    let coach = Pilot::Coach(Coach::Exhibition);
    let f = playbook::face_off(
        &Arena::backyard(),
        Ruleset::default(),
        3,
        4,
        [&book, &coach],
    );
    assert_eq!(f.record.played, 8);
    assert_eq!(
        f.record.won,
        f.won_by_capture + f.won_by_knockout + f.won_at_cap
    );
    assert!(f.compiled >= f.record.played);
    assert!((0.0..=1.0).contains(&f.as_asked()));
    let mirror = playbook::face_off(
        &Arena::backyard(),
        Ruleset::default(),
        3,
        4,
        [&coach, &book],
    );
    assert_eq!(mirror.record.played, 8);
    assert_eq!(mirror.compiled, 0, "a coach keeps no playbook record");
    assert!(f.record.won + f.record.drawn <= f.record.played);
}
