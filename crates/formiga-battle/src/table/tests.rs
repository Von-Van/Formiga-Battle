//! The table's tests: sittings driven without the window, through orders, whole matches and
//! seasons, what Desktop is written, and two tables playing each other through a local relay.

#[test]
fn a_handler_name_from_the_relay_is_cleaned_before_it_is_shown_or_kept() {
    assert_eq!(handler_name("  Sam\u{202e}\u{0007}  "), "Sam");
    assert_eq!(handler_name("\u{200b}"), "A Handler");
    assert_eq!(handler_name(&"x".repeat(80)).chars().count(), 24);
}

use super::*;
use formiga_battle_contract::sample;

fn table() -> Table {
    let mut t = Table::new(sample::snapshot(), sample::visitors(9), 5);
    t.start(0.0).unwrap();
    t
}

#[test]
fn a_trios_playbook_fills_in_its_orders_and_the_handler_can_change_them() {
    use formiga_battle_core::playbook::Playbook;
    let mut t = Table::new(sample::snapshot_with_record(), sample::visitors(9), 5);
    let trio = t.record.trios[0].clone();
    t.picked = trio
        .members
        .iter()
        .filter_map(|id| t.home.iter().position(|(c, _)| c.traveler.id.0 == *id))
        .collect();
    assert_eq!(
        t.picked.len(),
        TEAM_SIZE,
        "the sample's trio is in its colony"
    );
    let book = Playbook::samples()[0].clone();
    t.career.playbooks.insert(
        trio.id.clone(),
        Shelf {
            books: vec![book.clone()],
            playing: Some(0),
        },
    );
    t.start(0.0).unwrap();
    let proposed = t.playbook.as_ref().expect("plays by it").log[0]
        .plan
        .clone();
    assert_eq!(t.plan(), proposed, "its orders are filled in");
    assert_eq!(t.playbook_words().unwrap().0, book.name);
    // The Handler takes one out before locking in.
    t.clear_order(0);
    t.lock(0.0);
    let first = &t.playbook.as_ref().unwrap().log[0];
    assert!(first.sent.is_some(), "the change is recorded");
    assert!(
        t.log.last().unwrap().1[0].starts_with(&format!("Turn 1: {}", book.name)),
        "the whole match says what the playbook made of the turn"
    );
    // Once the turn has been shown, the next turn's orders are filled in again.
    t.frame(1.0e6);
    assert_eq!(t.playbook.as_ref().unwrap().log.len(), 2);
    assert!(t.orders.iter().any(Option::is_some));
}

#[test]
fn three_who_are_not_a_trio_play_by_no_playbook() {
    let t = table();
    assert!(t.playbook.is_none());
    assert!(t.playbook_words().is_none());
}

fn yard(t: &Table) -> Tile {
    t.m.as_ref().unwrap().arena.zone_centre()
}

#[test]
fn a_handler_builds_a_team_order_square_by_square() {
    let mut t = table();
    assert_eq!(t.selected, Some(0));
    t.choose_verb(Verb::Press);
    let centre = yard(&t);
    let next = Tile::new(centre.x - 1, centre.y);
    assert!(t.click(centre));
    assert!(t.click(next));
    assert_eq!(
        t.orders[0],
        Some(Directive::Press {
            squares: vec![centre, next]
        })
    );
    assert_eq!(t.selected, Some(0), "a PRESS stays chosen for more squares");
    assert!(t.click(next), "a square clicked again comes out");
    assert_eq!(
        t.orders[0],
        Some(Directive::Press {
            squares: vec![centre]
        })
    );
    assert!(
        t.plan_marks()
            .iter()
            .any(|m| m.tile == centre && m.kind == PlanKind::Press && m.order == 0)
    );
}

#[test]
fn dragging_adds_squares_to_the_order_in_hand_and_never_takes_one_out() {
    let mut t = table();
    let zone = t.arena.zone.clone();
    assert!(t.take_token(Verb::Hold));
    assert!(t.add_square(zone[0]));
    assert!(t.add_square(zone[1]));
    assert!(!t.add_square(zone[1]), "a square already named stays");
    assert_eq!(
        t.orders[0],
        Some(Directive::Hold {
            squares: vec![zone[0], zone[1]],
            facing: t.first_facing(zone[0]),
        })
    );
}

#[test]
fn a_trick_names_squares_as_a_press_does() {
    let mut t = table();
    let zone = t.arena.zone.clone();
    assert!(t.take_token(Verb::Trick));
    assert!(t.click(zone[0]));
    assert!(t.add_square(zone[1]));
    assert_eq!(
        t.orders[0],
        Some(Directive::Trick {
            squares: vec![zone[0], zone[1]]
        })
    );
    assert!(t.plan_marks().iter().all(|m| m.kind == PlanKind::Trick));
}

#[test]
fn a_hold_faces_their_nearest_and_turns_a_quarter_at_a_time() {
    let mut t = table();
    let centre = yard(&t);
    let first = t.first_facing(centre);
    let m = t.m.as_ref().unwrap();
    let nearest = m
        .team(Side::Right)
        .map(|s| m.fighter(s).at)
        .min_by_key(|at| (centre.distance(*at), at.y, at.x))
        .unwrap();
    assert_eq!(Some(first), Facing::toward(centre, nearest));
    assert!(t.take_token(Verb::Hold));
    assert!(t.click(centre));
    let facing = |t: &Table| match &t.orders[0] {
        Some(Directive::Hold { facing, .. }) => *facing,
        other => panic!("{other:?}"),
    };
    assert_eq!(facing(&t), first);
    assert!(t.turn_hold(false));
    assert_eq!(facing(&t), first.turned());
    // More squares keep the way it faces.
    assert!(t.add_square(Tile::new(centre.x - 1, centre.y)));
    assert_eq!(facing(&t), first.turned());
    assert!(t.turn_hold(true));
    assert_eq!(facing(&t), first);
    // The facing is on the board's marks.
    assert!(
        t.plan_marks()
            .iter()
            .all(|m| m.kind == PlanKind::Hold && m.facing == Some(first))
    );
    // Put down and given elsewhere, the R key still turns the one last given.
    t.put_down();
    t.selected = Some(1);
    assert!(t.turn_hold(false));
    assert_eq!(facing(&t), first.turned());
    t.clear_order(0);
    assert!(!t.turn_hold(false), "nothing left to turn");
}

#[test]
fn a_square_named_by_two_orders_is_marked_twice_stacked() {
    let mut t = table();
    let centre = yard(&t);
    t.take_token(Verb::Press);
    t.press(centre, false);
    t.verb = None;
    t.take_token(Verb::Hold);
    t.press(centre, false);
    let marks: Vec<_> = t
        .plan_marks()
        .into_iter()
        .filter(|m| m.tile == centre)
        .collect();
    assert_eq!(marks.len(), 2);
    assert_eq!(marks[1].stack, 1);
    assert_ne!(marks[0].order, marks[1].order);
}

#[test]
fn with_all_three_orders_given_no_more_can_be_picked() {
    let mut t = table();
    let zone = t.arena.zone.clone();
    for (i, tile) in zone.iter().take(3).enumerate() {
        assert!(t.take_token(Verb::Hold), "order {i}");
        t.press(*tile, false);
        t.verb = None;
    }
    t.selected = None;
    assert!(!t.take_token(Verb::Press));
}

#[test]
fn with_no_order_picked_a_click_presses_a_square_and_a_right_click_holds_it() {
    let mut t = table();
    let centre = yard(&t);
    assert_eq!(
        t.press(centre, false),
        Aim::Give(Directive::Press {
            squares: vec![centre]
        })
    );
    assert_eq!(t.selected, Some(1), "on to the next order still empty");
    assert_eq!(
        t.press(centre, true),
        Aim::Give(Directive::Hold {
            squares: vec![centre],
            facing: t.first_facing(centre),
        })
    );
    assert_eq!(t.selected, Some(2));
    assert_eq!(t.plan().len(), 2);
    assert_eq!(t.turnable(), Some(1), "the HOLD just given turns");
}

#[test]
fn use_item_is_a_bonus_for_whoever_carries_something() {
    let mut t = table();
    assert!(!t.take_token(Verb::UseItem), "nobody carries anything");
    let carrier = Slot::new(Side::Left, 1);
    let at = {
        let m = t.m.as_mut().unwrap();
        m.board.fighters[0][1].carrying = Some(formiga_battle_core::Item::Bell);
        m.fighter(carrier).at
    };
    // Three orders given: USE ITEM still goes beside them.
    let zone = t.arena.zone.clone();
    for tile in zone.iter().take(3) {
        t.take_token(Verb::Hold);
        t.press(*tile, false);
        t.verb = None;
    }
    assert!(t.take_token(Verb::UseItem));
    assert_eq!(t.user, Some(1), "the only one carrying something is chosen");
    assert!(
        t.choices()
            .iter()
            .any(|(tile, c)| *tile == at && *c == Choice::Reach)
    );
    let far = Tile::new(at.x + 4, at.y);
    assert_eq!(t.aim(far, false), Aim::Nothing, "out of the bell's reach");
    assert_eq!(
        t.press(at, false),
        Aim::Give(Directive::UseItem { by: 1, at })
    );
    assert_eq!(t.verb, None);
    let plan = t.plan();
    assert_eq!(plan.iter().filter(|d| d.tactical()).count(), PLAN_SIZE);
    assert!(plan.contains(&Directive::UseItem { by: 1, at }));
    assert!(
        t.plan_marks()
            .iter()
            .any(|m| m.kind == PlanKind::UseItem && m.tile == at && m.from == Some(at))
    );
    // One for each carrier: given again, it moves rather than adds.
    let near = Tile::new(at.x + 1, at.y);
    assert!(t.take_token(Verb::UseItem));
    t.press(near, false);
    assert_eq!(
        t.plan()
            .iter()
            .filter(|d| matches!(d, Directive::UseItem { .. }))
            .count(),
        1
    );
    t.clear_use(1);
    assert!(!t.plan().iter().any(|d| !d.tactical()));
}

#[test]
fn with_two_carrying_something_use_item_asks_which() {
    let mut t = table();
    let at = {
        let m = t.m.as_mut().unwrap();
        m.board.fighters[0][0].carrying = Some(formiga_battle_core::Item::Bell);
        m.board.fighters[0][2].carrying = Some(formiga_battle_core::Item::Bell);
        m.fighter(Slot::new(Side::Left, 2)).at
    };
    assert!(t.take_token(Verb::UseItem));
    assert_eq!(t.user, None);
    assert_eq!(t.press(at, false), Aim::Carrier(2));
    assert_eq!(t.user, Some(2));
    assert!(t.take_token(Verb::UseItem), "picked up again, the next");
    assert_eq!(t.user, Some(0));
}

#[test]
fn interact_needs_something_there_to_interact_with() {
    let mut t = table();
    t.choose_verb(Verb::Interact);
    let centre = yard(&t);
    assert_eq!(t.aim(centre, false), Aim::Nothing);
    let lever = t.m.as_ref().unwrap().arena.levers[0];
    assert_eq!(
        t.aim(lever, false),
        Aim::Give(Directive::Interact { at: lever })
    );
}

#[test]
fn a_cleared_order_comes_out_of_the_plan() {
    let mut t = table();
    let centre = yard(&t);
    t.press(centre, false);
    t.clear_order(0);
    assert!(t.plan().is_empty());
}

#[test]
fn nothing_can_be_aimed_while_a_turn_plays_out() {
    let mut t = table();
    let centre = yard(&t);
    t.lock(0.0);
    assert_eq!(t.aim(centre, false), Aim::Nothing);
}

#[test]
fn a_locked_turn_is_shown_through_and_the_next_turns_orders_begin() {
    let mut t = table();
    t.lock(1.0);
    assert!(t.shown.is_some());
    let length = t.shown.as_ref().unwrap().playback.duration();
    t.frame(1.0 + length / 2.0);
    assert!(t.shown.is_some());
    t.frame(1.0 + length + 0.1);
    assert!(t.shown.is_none());
    assert_eq!(t.orders, [None, None, None]);
    assert_eq!(t.log.len(), 1);
    assert_eq!(t.m.as_ref().unwrap().board.turn, 1);
}

#[test]
fn while_orders_are_given_the_team_says_who_will_probably_go_where_and_nothing_of_theirs() {
    let mut t = table();
    assert!(t.likely().is_empty(), "no plan, nothing to say");
    t.coach_orders();
    let likely = t.likely();
    assert!(!likely.is_empty());
    assert!(likely.iter().all(|(slot, _, _)| slot.side == t.you));
    // Locked, it is the turn's own business.
    t.lock(0.0);
    assert!(t.likely().is_empty());
    // And what came of each order is told once the turn has been played.
    assert!(!t.told.is_empty());
}

#[test]
fn a_whole_match_can_be_played_through_the_table() {
    let mut t = table();
    let mut now = 0.0;
    while !t.finished(now) {
        t.coach_orders();
        t.lock(now);
        now += 20.0;
        t.frame(now);
    }
    assert!(t.outcome().is_some());
}

/// Plays every fixture of the event under way that is the colony's, coached as a Handler
/// would be. How many it played.
fn play_event(t: &mut Table) -> u32 {
    let mut played = 0;
    while let Some((index, _, rival)) = t.next_event_fixture() {
        t.event_fixture = Some(index);
        t.start(0.0).unwrap();
        assert_eq!(
            t.rival.map(|r| r.id),
            Some(rival.id),
            "the bracket's rival comes"
        );
        let mut now = 0.0;
        while !t.finished(now) {
            t.coach_orders();
            t.lock(now);
            now += 20.0;
            t.frame(now);
        }
        played += 1;
        assert!(
            t.event_line.is_some(),
            "the result says how the event stands"
        );
        assert_eq!(t.event_fixture, None);
    }
    played
}

#[test]
fn a_season_runs_its_four_events_and_the_next_one_has_a_champion_to_defend() {
    let mut t = Table::new(sample::snapshot(), sample::visitors(9), 3);
    t.begin_next_event();
    let first = t.career.event.clone().expect("an event begun");
    assert_eq!(first.name, "The Backyard Open");
    assert_eq!(first.bracket.entrants.len(), 4);
    assert_eq!(first.bracket.entrants.last().map(String::as_str), Some(YOU));
    // Beginning again while it is under way changes nothing.
    t.begin_next_event();
    assert_eq!(t.career.events_entered, 1);
    let mut meetings = play_event(&mut t);
    assert!(t.career.event.as_ref().unwrap().bracket.finished());
    for _ in 0..3 {
        t.begin_next_event();
        meetings += play_event(&mut t);
        assert!(
            t.career.event.as_ref().unwrap().bracket.finished(),
            "the rivals' own fixtures are played out too"
        );
    }
    let season = t.career.season.clone().unwrap();
    assert!(season.finished());
    let champion = season.champion.clone().expect("a champion");
    assert_eq!(t.career.hall.len(), 1);
    assert_eq!(t.career.hall[0].who, champion);
    assert_eq!(t.career.meetings(), meetings);
    assert_eq!(t.career.events_entered, 4);
    // Season 2 begins with last season's champion defending.
    t.begin_next_event();
    let next = t.career.season.clone().unwrap();
    assert_eq!(next.number, 2);
    assert_eq!(next.defending, Some(champion));
    assert_eq!(next.played, 0);
    // What it noticed is backed by matches.
    for r in &t.career.recognition {
        assert!(!r.evidence.is_empty(), "{:?} has its evidence", r.noticed);
    }
    let (title, how) = career::standing(&t.career);
    assert!(!title.is_empty() && !how.is_empty());
}

#[test]
fn a_match_against_the_visitors_is_set_up_in_the_variant_the_where_page_shows() {
    let mut t = table();
    let shown = t.variant();
    assert_eq!(t.m.as_ref().unwrap().arena.variant.as_ref(), Some(&shown));
    // The place chosen stays the arena as its file has it; only the match's copy is varied.
    assert_eq!(t.arena.variant, None);
    t.start_lesson(Lesson::Flank, 0.0).unwrap();
    assert_eq!(
        t.m.as_ref().unwrap().arena.variant,
        None,
        "lessons are never varied"
    );
}

#[test]
fn no_way_of_playing_wins_most_of_the_rivals_own_events() {
    // Eight rivals, every way of playing among them, in event after event on every place:
    // whoever wins should not be settled by how they play.
    let t = Table::new(sample::snapshot(), sample::visitors(9), 3);
    let mut by_style: std::collections::BTreeMap<&str, u32> = Default::default();
    let mut events = 0;
    for (k, arena) in Arena::built_in().into_iter().enumerate() {
        for n in 0..4u64 {
            let entrants: Vec<String> = (0..8)
                .map(|i| {
                    rival::visiting(&arena.id, i + n as u32 + k as u32)
                        .id
                        .to_owned()
                })
                .collect();
            let mut b =
                formiga_battle_world::Bracket::new("rivals", 100 + n * 7 + k as u64, entrants)
                    .expect("eight rivals, each once");
            t.play_out_rivals(&mut b, &arena);
            let champion = b.champion().expect("played out");
            let style = rival::by_id(&b.entrants[champion])
                .unwrap()
                .archetype
                .label();
            *by_style.entry(style).or_default() += 1;
            events += 1;
        }
    }
    let most = by_style.values().copied().max().unwrap_or(0);
    assert!(
        most * 2 <= events,
        "one way of playing won {most} of {events}: {by_style:?}"
    );
}

#[test]
fn a_visitor_with_one_of_your_names_is_told_apart() {
    let mut t = table();
    let ours = t.m.as_ref().unwrap().rosters[0][0].name.clone();
    t.away[t.teams[1][0]].0.traveler.name = ours.clone();
    t.away[t.teams[1][0]].1.name = ours.clone();
    t.start(0.0).unwrap();
    let theirs: Vec<String> = t.m.as_ref().unwrap().rosters[1]
        .iter()
        .map(|p| p.name.clone())
        .collect();
    assert!(!theirs.contains(&ours), "{theirs:?}");
}

#[test]
fn a_friend_is_sent_desktops_handler_key_and_a_rehearsal_sends_none() {
    let session = || Session {
        directory: std::env::temp_dir(),
        result: BattleResult::new(
            formiga_battle_contract::SessionId::parse("00000000000000000000000000000000").unwrap(),
            "0".repeat(64),
            "0.1.0",
        ),
    };
    let mut t = Table::new(sample::snapshot_with_record(), sample::visitors(9), 5);
    assert_eq!(
        t.handler_key(),
        None,
        "a rehearsal has none, whatever it was given"
    );
    t.session = Some(session());
    assert_eq!(t.handler_key().as_deref(), Some(sample::HANDLER_KEY));
    t.record.handler_key = Some("Not A Key".to_owned());
    assert_eq!(
        t.handler_key(),
        None,
        "what is not a key is never sent as one"
    );

    let mut older = sample::snapshot_with_record();
    older.record.as_mut().unwrap().handler_key = None;
    let mut t = Table::new(older, sample::visitors(9), 5);
    t.session = Some(session());
    assert_eq!(t.handler_key(), None, "a Desktop that made none sends none");
}

#[test]
fn an_ai_match_earns_its_rewards_and_writes_them_into_desktops_session() {
    let dir = std::env::temp_dir().join(format!("formiga-battle-table-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut t = Table::new(sample::snapshot(), sample::visitors(9), 5);
    t.session = Some(Session {
        directory: dir.clone(),
        result: BattleResult::new(
            formiga_battle_contract::SessionId::parse("00000000000000000000000000000000").unwrap(),
            "0".repeat(64),
            "0.1.0",
        ),
    });
    t.start(0.0).unwrap();
    let mut now = 0.0;
    while !t.finished(now) {
        t.coach_orders();
        t.lock(now);
        now += 20.0;
        t.frame(now);
    }
    assert!(!t.earned.is_empty());
    assert_eq!(t.record.circuit_points, 0, "the AI gives no points");
    let written = BattleResult::decode(
        &std::fs::read(dir.join(formiga_battle_contract::RESULT_FILE)).unwrap(),
    )
    .unwrap();
    assert_eq!(written.matches.len(), 1);
    assert_eq!(written.matches[0].mode, formiga_battle_contract::Mode::Ai);
}

#[test]
fn a_session_from_a_version_2_desktop_is_written_so_it_can_keep_it() {
    let dir = std::env::temp_dir().join(format!("formiga-battle-v2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut roster = sample::snapshot();
    roster.version = 2;
    roster.desktop_version = "0.67.8".to_owned();
    let mut t = Table::new(roster, sample::visitors(9), 5);
    t.session = Some(Session {
        directory: dir.clone(),
        result: BattleResult::new(
            formiga_battle_contract::SessionId::parse("00000000000000000000000000000000").unwrap(),
            "0".repeat(64),
            "0.1.0",
        ),
    });
    t.start(0.0).unwrap();
    let mut now = 0.0;
    while !t.finished(now) {
        t.coach_orders();
        t.lock(now);
        now += 20.0;
        t.frame(now);
    }
    let written = BattleResult::decode(
        &std::fs::read(dir.join(formiga_battle_contract::RESULT_FILE)).unwrap(),
    )
    .unwrap();
    assert_eq!(written.matches.len(), 1);
    let r = &written.matches[0].rewards;
    assert_eq!(written.matches[0].mode, formiga_battle_contract::Mode::Ai);
    // What Desktop 0.67.8 refuses in a match against the visitors: anything a friend match
    // earns, the notebook included.
    assert!(r.notebook.is_empty() && r.participation.is_empty() && r.history.is_none());
    assert!(!r.journal.is_empty(), "and the rest is still earned");
    assert!(written.min_reader_version <= 2);
    assert!(
        t.record.notebook.is_empty(),
        "nor kept here as if Desktop would"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_trio_named_before_a_match_plays_it_and_desktop_is_told_both() {
    let dir = std::env::temp_dir().join(format!("formiga-battle-trio-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut t = Table::new(sample::snapshot(), sample::visitors(9), 5);
    t.session = Some(Session {
        directory: dir.clone(),
        result: BattleResult::new(
            formiga_battle_contract::SessionId::parse("00000000000000000000000000000000").unwrap(),
            "0".repeat(64),
            "0.1.0",
        ),
    });
    // Signing a name for a friend makes it the Handler's.
    assert!(t.record.handler.is_none());
    let card = crate::identity::card_with_name(&t.record, "Jo").unwrap();
    t.set_handler(card);
    let ids: Vec<u64> = t
        .picked
        .iter()
        .map(|&i| t.home[i].0.traveler.id.0)
        .collect();
    t.set_trio(crate::identity::named("The Thistle Three", &ids).unwrap());
    let read = |dir: &std::path::Path| {
        BattleResult::decode(
            &std::fs::read(dir.join(formiga_battle_contract::RESULT_FILE)).unwrap(),
        )
        .unwrap()
    };
    let named = read(&dir);
    assert!(
        named.matches.is_empty(),
        "nothing played yet, and still written"
    );
    let registry = named.registry.expect("the card and trio, for Desktop");
    assert_eq!(registry.handler.unwrap().name, "Jo");
    assert_eq!(registry.trios[0].name, "The Thistle Three");
    let card = t.shared_card().expect("a card for a friend");
    assert_eq!(card.trio.as_deref(), Some("The Thistle Three"));
    assert_eq!(card.standing.as_deref(), Some("New Handler"));

    t.start(0.0).unwrap();
    let mut now = 0.0;
    while !t.finished(now) {
        t.coach_orders();
        t.lock(now);
        now += 20.0;
        t.frame(now);
    }
    assert!(
        t.earned
            .iter()
            .any(|l| l.starts_with("The Thistle Three: 1 played"))
    );
    assert_eq!(t.record.trios[0].visitors.played, 1);
    let written = read(&dir);
    assert_eq!(
        written.matches[0].rewards.trio.as_ref().unwrap().id,
        t.record.trios[0].id
    );
    assert!(written.registry.is_some());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_trio_changed_after_it_played_tells_desktop_who_played_each_match() {
    let dir = std::env::temp_dir().join(format!("formiga-battle-lineup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut t = Table::new(sample::snapshot(), sample::visitors(9), 5);
    assert!(t.home.len() > TEAM_SIZE, "someone to swap in");
    t.session = Some(Session {
        directory: dir.clone(),
        result: BattleResult::new(
            formiga_battle_contract::SessionId::parse("00000000000000000000000000000000").unwrap(),
            "0".repeat(64),
            "0.1.0",
        ),
    });
    let ids = |t: &Table| -> Vec<u64> {
        t.picked
            .iter()
            .map(|&i| t.home[i].0.traveler.id.0)
            .collect()
    };
    let play = |t: &mut Table| {
        t.start(0.0).unwrap();
        let mut now = 0.0;
        while !t.finished(now) {
            t.coach_orders();
            t.lock(now);
            now += 20.0;
            t.frame(now);
        }
    };
    let first = formiga_battle_contract::lineup_of(&ids(&t));
    t.set_trio(crate::identity::named("The Thistle Three", &first).unwrap());
    play(&mut t);
    // The third gives their place to someone else, and the trio plays again.
    t.picked = vec![0, 1, TEAM_SIZE];
    let second = formiga_battle_contract::lineup_of(&ids(&t));
    let mut card = t.record.trios[0].card();
    card.members = second.clone();
    t.set_trio(card);
    play(&mut t);
    let written = BattleResult::decode(
        &std::fs::read(dir.join(formiga_battle_contract::RESULT_FILE)).unwrap(),
    )
    .unwrap();
    let who: Vec<_> = written
        .matches
        .iter()
        .map(|m| m.rewards.trio.as_ref().unwrap().members.clone())
        .collect();
    assert_eq!(who, [first.clone(), second.clone()]);
    // Desktop keeps the three it had first as an earlier lineup, by the contract's own rule.
    let mut kept = formiga_battle_contract::BattleRecord::default();
    kept.register_session(
        written.registry.as_ref().unwrap(),
        &written.matches,
        time::OffsetDateTime::UNIX_EPOCH,
    );
    for m in &written.matches {
        kept.apply(m);
    }
    assert_eq!(kept.trios[0].members, second);
    assert_eq!(kept.trios[0].lineups.len(), 1);
    assert_eq!(kept.trios[0].lineups[0].members, first);
    assert_eq!(kept.trios[0].visitors.played, 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn two_tables_play_a_whole_match_through_a_relay() {
    // A relay named in FORMIGA_BATTLE_RELAY, such as the site's, or one of our own.
    let named = std::env::var(formiga_battle_room::RELAY_ENV)
        .ok()
        .filter(|v| !v.trim().is_empty());
    let local = match named {
        Some(_) => None,
        None => Some(
            formiga_battle_room::serve_relay("127.0.0.1:0", formiga_battle_room::Limits::default())
                .unwrap(),
        ),
    };
    let base = named.unwrap_or_else(|| local.as_ref().unwrap().base());
    let mut host = Table::new(sample::snapshot(), sample::visitors(9), 1);
    let mut guest = Table::new(sample::visitors(11), sample::visitors(9), 2);
    host.relay = base.clone();
    guest.relay = base;
    host.open_room("Jake", None);
    let mut now = 0.0;
    // Opening a room first fingerprints this program, which takes a while in a debug build
    // with the other tests busy, so the wait is by the wall clock rather than by the table's.
    let opened = std::time::Instant::now();
    let code = loop {
        host.poll(now);
        if let Some(code) = host.friend.as_ref().and_then(|f| f.code.clone()) {
            break code;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        now += 0.005;
        assert!(
            opened.elapsed().as_secs() < 60,
            "the room opens: {:?}",
            host.notice
        );
    };
    guest.open_room("Friend", Some(&code.to_lowercase()));
    // Joining fingerprints the program too, so this wait is also by the wall clock.
    let joined = std::time::Instant::now();
    let mut began = [false, false];
    while !(began[0] && began[1]) {
        began[0] |= host.poll(now);
        began[1] |= guest.poll(now);
        std::thread::sleep(std::time::Duration::from_millis(20));
        now += 0.02;
        assert!(
            joined.elapsed().as_secs() < 60,
            "both see the match begin: {:?} {:?}",
            host.notice,
            guest.notice
        );
    }
    assert_eq!(host.you, Side::Left);
    assert_eq!(guest.you, Side::Right);
    assert_eq!(host.other_short(), "Friend");
    assert_eq!(host.other_handler.as_deref(), Some("Friend"));
    assert_eq!(guest.other_handler.as_deref(), Some("Jake"));
    let mut rounds = 0;
    while !(host.finished(now) && guest.finished(now)) {
        for t in [&mut host, &mut guest] {
            let turn = t.m.as_ref().unwrap().board.turn + 1;
            if t.shown.is_none() && t.outcome().is_none() && !t.friend.as_ref().unwrap().sent(turn)
            {
                t.coach_orders();
                t.lock(now);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        now += 2.0;
        host.poll(now);
        guest.poll(now);
        host.frame(now);
        guest.frame(now);
        rounds += 1;
        assert!(rounds < 600, "the match ends");
    }
    let (a, b) = (host.m.as_ref().unwrap(), guest.m.as_ref().unwrap());
    assert_eq!(a.board, b.board, "both see the same board");
    assert!(a.outcome().is_some());
    // What it earned is kept once the relay has confirmed both boards.
    let held = |t: &Table| t.friend.as_ref().is_some_and(|f| f.held.is_some());
    // While they are being checked, nothing that would drop them can begin.
    if held(&host) {
        assert!(host.checking_board());
        host.open_room("Jake", None);
        assert_eq!(host.notice.as_deref(), Some(STILL_CHECKING));
        assert!(host.start(now).is_err(), "the visitors wait too");
        assert!(held(&host), "and the earnings are still held");
    }
    while held(&host) || held(&guest) {
        std::thread::sleep(std::time::Duration::from_millis(50));
        now += 1.0;
        host.poll(now);
        guest.poll(now);
        rounds += 1;
        assert!(rounds < 700, "the boards are confirmed");
    }
    for t in [&mut host, &mut guest] {
        assert!(
            t.earned.iter().any(|l| l.contains("Open Circuit points")),
            "{:?}",
            t.earned
        );
        assert!(!t.checking_board());
        // Settled, the room is left behind so the next one can be opened.
        t.leave_when_settled();
        assert!(t.friend.is_none());
    }
    if let Some(relay) = local {
        relay.stop();
    }
}

#[test]
fn a_room_that_closes_mid_match_ends_it_here_earning_nothing() {
    let mut t = table();
    let (line, asked, tell) = crate::friend::Line::fake();
    t.friend = Some(Friend {
        line,
        code: Some("ABCDEF".to_owned()),
        seated: true,
        gone: false,
        handlers: ["Jake".to_owned(), "Friend".to_owned()],
        cards: [None, None],
        view: None,
        sent_turn: None,
        opponent_ready: false,
        opponent_away: false,
        deadline: 30.0,
        held: None,
    });
    tell.send(Told::Gone("The room has closed.".to_owned()))
        .unwrap();
    assert!(!t.poll(1.0));
    assert!(t.room_gone(), "the match stays on the board, ended");
    assert_eq!(t.notice.as_deref(), Some("The room has closed."));
    // Nothing more goes to the relay: not orders given, nor the last-moment send at the lock.
    t.coach_orders();
    t.lock(2.0);
    t.poll(29.5);
    assert!(asked.try_recv().is_err());
    assert!(t.outcome().is_none());
    assert!(t.earned.is_empty());
    // Leaving it lets the line go.
    t.leave_room();
    assert!(t.friend.is_none());
    assert!(!t.room_gone());
}

#[test]
fn a_colony_of_one_is_joined_by_friends_from_next_door() {
    let mut roster = sample::snapshot();
    roster.contenders.truncate(1);
    roster.relationships.clear();
    let t = Table::new(roster, sample::visitors(9), 5);
    assert_eq!(t.home.len(), 3);
    assert!(t.away.len() >= 3);
}
