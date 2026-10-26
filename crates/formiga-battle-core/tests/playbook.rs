//! Playbooks compile to ordinary team plans: never an order the team could not carry out, never
//! more than three, the same plan from the same board, a record of every turn, and a match that
//! replays from what was sent like any other.

use formiga_battle_core::ai::Coach;
use formiga_battle_core::muster;
use formiga_battle_core::playbook::{
    self, Advance, Deviation, FacingHint, Fallback, Intent, Pilot, Playbook, PlaybookError, Runner,
    Stage, Timing, Trigger,
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
        .flat_map(|p| [p.clone(), p.clone().bundled(), p.untriggered()])
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
                            Advance::Met { stage } | Advance::RanOut { stage, .. } => Some(*stage),
                            Advance::Triggered { from, to, trigger } => {
                                let stage = book.stages[*to as usize];
                                assert_eq!(stage.when, Some(*trigger));
                                assert_ne!(Some(*to), *from, "a stage never brings itself in");
                                *from
                            }
                        };
                        assert_eq!(from, last, "{c:?}");
                        if let Advance::RanOut { stage, fallback } = a {
                            assert_eq!(*fallback, book.stages[*stage as usize].fallback);
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

#[test]
fn the_board_brings_a_stage_in_whatever_stage_the_team_is_on() {
    // Secure for four turns, with Counter brought in the moment they hold the beacon: against a
    // team that digs in on it, Counter comes in on the second turn, not after the fourth.
    let book = Playbook::new(
        "Answer them",
        &[
            Stage::new(Intent::Secure).turns(4),
            Stage::new(Intent::Counter).when(Trigger::TheyHoldBeacon),
        ],
    );
    let (_, _, runner) = play_checked(&Arena::backyard(), 2, &book, Coach::Turtle);
    let first = runner
        .log
        .iter()
        .find(|c| !c.advanced.is_empty())
        .expect("the stage changes");
    assert_eq!(
        first.advanced,
        vec![Advance::Triggered {
            from: Some(0),
            to: 1,
            trigger: Trigger::TheyHoldBeacon
        }]
    );
    assert!(
        first.turn < 5,
        "brought in before Secure ran out: {first:?}"
    );
    assert_eq!(first.ran[0].intent, Intent::Counter);
    assert!(
        first
            .line(&book)
            .contains("came in as they hold the beacon")
    );
    // The same playbook with no trigger stays on Secure until it gets its goal or its turns.
    let (_, _, plain) = play_checked(&Arena::backyard(), 2, &book.untriggered(), Coach::Turtle);
    assert!(plain.log.iter().take(4).all(|c| c.ran[0].stage == Some(0)));
}

#[test]
fn the_handler_can_change_any_of_a_playbooks_orders() {
    let book = Playbook::samples()[0].clone();
    let rosters = rosters(4);
    let mut m = Match::new(Arena::backyard(), Ruleset::default(), 4, rosters).unwrap();
    let mut runner = Runner::new(book.clone(), Side::Left);
    // Turn one as given; turn two with the Handler's own orders; turn three half kept.
    let mut sent_plans = Vec::new();
    for turn in 1..=3u8 {
        let proposed = runner.plan(&m);
        let sent = match turn {
            1 => proposed.clone(),
            2 => vec![Directive::Hold {
                squares: vec![m.arena.zone_centre()],
                facing: Facing::East,
            }],
            _ => {
                let mut p: TeamPlan = proposed
                    .iter()
                    .filter(|d| d.tactical())
                    .take(1)
                    .cloned()
                    .collect();
                // A HOLD the playbook did not give.
                let own = Facing::ALL
                    .into_iter()
                    .map(|facing| Directive::Hold {
                        squares: vec![m.arena.zone_centre()],
                        facing,
                    })
                    .find(|d| !proposed.contains(d))
                    .expect("one facing it did not give");
                p.push(own);
                p
            }
        };
        runner.sent(&sent);
        sent_plans.push(sent.clone());
        m.resolve_turn([sent, Coach::Exhibition.plan(&m, Side::Right)]);
    }
    let log = &runner.log;
    assert_eq!(log[0].sent, None, "kept as given");
    assert_eq!(log[0].sent_plan(), &log[0].plan);
    let two = log[1].sent.as_ref().expect("changed");
    assert_eq!(two.plan, sent_plans[1]);
    assert!(log[1].line(&book).contains("their own orders"));
    let three = log[2].sent.as_ref().expect("changed");
    assert_eq!((three.kept, three.added), (1, 1));
    assert_eq!(log[2].sent_plan(), &sent_plans[2]);
    // The match was played from what was sent, and replays from it.
    for (record, sent) in m.history.iter().zip(&sent_plans) {
        assert_eq!(&record.submitted[0], sent);
    }
    let (again, _) = m.record().replay().expect("replays");
    assert_eq!(again.hash(), m.hash());
}

#[test]
fn no_sample_playbook_beats_every_thinking_rival() {
    // A playbook is a way of saying what the team wants, not a stronger team: none should beat
    // every style of rival a Handler meets at a Regional, where rivals think each turn through.
    // (Against the book-playing rivals of the cups and the Opens, a good plan should win.)
    let styles = [
        Archetype::Aggressor,
        Archetype::Defender,
        Archetype::Tactician,
        Archetype::Opportunist,
        Archetype::Adaptive,
    ];
    let mut report = String::new();
    let mut dominant = Vec::new();
    for book in Playbook::samples() {
        let pilot = Pilot::Playbook(book.clone());
        let mut beats_all = true;
        for style in styles {
            let coach = Pilot::Coach(Coach::Thinking(style));
            let f = playbook::face_off(
                &Arena::backyard(),
                Ruleset::default(),
                6,
                8,
                [&pilot, &coach],
            );
            let score = f.record.score();
            report.push_str(&format!("{} v {}: {score:.3}\n", book.name, style.label()));
            beats_all &= score > 0.62;
        }
        if beats_all {
            dominant.push(book.name.clone());
        }
    }
    eprintln!("{report}");
    assert!(
        dominant.is_empty(),
        "{dominant:?} beat every style:\n{report}"
    );
}

#[test]
fn a_secure_stage_facing_across_keeps_that_facing_on_squares_already_ours() {
    // One of ours already stands on the beacon's middle. Asked to face across the board, the
    // stage holds that square facing across, never presses onto it (which would face it toward
    // the nearest of theirs instead).
    let mut m = Match::new(Arena::backyard(), Ruleset::default(), 3, rosters(3)).unwrap();
    let centre = m.arena.zone_centre();
    m.board.fighters[0][0].at = centre;
    let book = Playbook::new(
        "Across",
        &[Stage::new(Intent::Secure).facing(FacingHint::Across)],
    );
    let mut runner = Runner::new(book, Side::Left);
    let plan = runner.plan(&m);
    assert!(
        !plan
            .iter()
            .any(|d| matches!(d, Directive::Press { squares } if squares.contains(&centre))),
        "no PRESS onto a square of ours: {plan:?}"
    );
    assert!(
        plan.iter().any(|d| matches!(
            d,
            Directive::Hold { squares, facing } if squares.contains(&centre) && *facing == Facing::across(Side::Left)
        )),
        "the square of ours is held facing across: {plan:?}"
    );
}
