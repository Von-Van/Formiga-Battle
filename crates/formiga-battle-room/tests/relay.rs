//! The relay, driven directly and over HTTP: two friends, a code, a whole match worked out by
//! both windows, and nothing kept once it is over.

use formiga_battle_contract::{Contender, sample};
use formiga_battle_core::ai::Coach;
use formiga_battle_core::{Directive, Match, RULES_VERSION, Side, TEAM_SIZE, TeamPlan, Tile};
use formiga_battle_room::relay::{Answer, Limits, Refusal, Request, Stage};
use formiga_battle_room::roster::{bonds_among, room_match};
use formiga_battle_room::{
    Build, Card, Entrant, RELAY_VERSION, Relay, RelayClient, Released, View, serve_relay,
};

fn entrant(seed: u8, handler: &str) -> Entrant {
    let roster = if seed == 0 {
        sample::snapshot()
    } else {
        sample::visitors(seed)
    };
    let team: Vec<Contender> = roster.contenders[..TEAM_SIZE].to_vec();
    Entrant {
        protocol: RELAY_VERSION,
        rules_version: RULES_VERSION,
        build: Build {
            battle: "0.1.0".to_owned(),
            desktop: Some("0.67.7".to_owned()),
            digest: "a".repeat(64),
        },
        handler: handler.to_owned(),
        bonds: bonds_among(&roster, &team),
        team,
        arena: "backyard".to_owned(),
        card: None,
        handler_key: None,
    }
}

fn counting() -> Box<dyn FnMut() -> u64 + Send> {
    let mut n = 7u64;
    Box::new(move || {
        n = n
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        n
    })
}

fn seated(answer: Answer) -> (String, String) {
    match answer {
        Answer::Seat { code, seat } => (code, seat),
        other => panic!("expected a seat, got {other:?}"),
    }
}

fn viewed(answer: Answer) -> View {
    match answer {
        Answer::View(view) => *view,
        other => panic!("expected a view, got {other:?}"),
    }
}

fn refusal(answer: Answer) -> Refusal {
    match answer {
        Answer::Refused(r) => r,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// A room opened and joined: its code and both seats.
fn room(relay: &mut Relay) -> (String, String, String) {
    let (code, host) = seated(relay.handle(
        Request::Open {
            entrant: entrant(0, "Jake"),
        },
        0,
    ));
    let (_, guest) = seated(relay.handle(
        Request::Join {
            code: code.to_lowercase(),
            entrant: entrant(9, "Friend"),
        },
        10,
    ));
    (code, host, guest)
}

#[test]
fn another_relay_version_is_turned_away_before_anything_is_kept() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let mut old = entrant(0, "Jake");
    old.protocol += 1;
    let r = refusal(relay.handle(Request::Open { entrant: old }, 0));
    assert!(matches!(r, Refusal::Version { .. }));
    assert_eq!(relay.rooms(), 0);
}

#[test]
fn a_friend_playing_by_other_rules_cannot_join() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, _) = seated(relay.handle(
        Request::Open {
            entrant: entrant(0, "Jake"),
        },
        0,
    ));
    let mut other = entrant(9, "Friend");
    other.rules_version += 1;
    let r = refusal(relay.handle(
        Request::Join {
            code,
            entrant: other,
        },
        10,
    ));
    assert_eq!(
        r,
        Refusal::Version {
            relay: RELAY_VERSION,
            rules_version: RULES_VERSION
        }
    );
}

/// What a joiner is told by a relay knowing `released`, when it plays with `change`d from the
/// opener's build; `None` if it is seated.
fn joining_with(released: Released, change: impl FnOnce(&mut Build)) -> Option<Refusal> {
    let mut relay = Relay::with_entropy(Limits::default(), counting()).knowing(released);
    let (code, _) = seated(relay.handle(
        Request::Open {
            entrant: entrant(0, "Jake"),
        },
        0,
    ));
    let mut other = entrant(9, "Friend");
    change(&mut other.build);
    match relay.handle(
        Request::Join {
            code,
            entrant: other,
        },
        10,
    ) {
        Answer::Seat { .. } => None,
        Answer::Refused(r) => Some(r),
        answer => panic!("expected a seat or a refusal, got {answer:?}"),
    }
}

#[test]
fn a_friend_with_another_battle_or_desktop_cannot_join_and_is_told_which_to_have() {
    let told = Some(Refusal::Mismatch {
        battle: "0.1.0".to_owned(),
        desktop: Some("0.67.7".to_owned()),
    });
    assert_eq!(
        joining_with(Released::default(), |b| b.battle = "0.1.1".to_owned()),
        told
    );
    assert_eq!(
        joining_with(Released::default(), |b| b.desktop =
            Some("0.67.8".to_owned())),
        told
    );
    assert_eq!(
        joining_with(Released::default(), |b| b.desktop = None),
        told
    );
    assert_eq!(joining_with(Released::default(), |_| {}), None);
}

#[test]
fn another_copy_of_the_same_versions_cannot_join_unless_both_were_released() {
    let other = |b: &mut Build| b.digest = "b".repeat(64);
    assert_eq!(
        joining_with(Released::default(), other),
        Some(Refusal::Build)
    );
    let mut released = Released::default();
    released.0.insert(
        "0.1.0".to_owned(),
        ["a".repeat(64), "b".repeat(64)].into_iter().collect(),
    );
    assert_eq!(joining_with(released, other), None);
}

#[test]
fn a_copy_that_is_none_of_its_releases_builds_cannot_open_a_room() {
    let mut released = Released::default();
    released
        .0
        .insert("0.1.0".to_owned(), ["b".repeat(64)].into_iter().collect());
    let mut relay = Relay::with_entropy(Limits::default(), counting()).knowing(released);
    let r = refusal(relay.handle(
        Request::Open {
            entrant: entrant(0, "Jake"),
        },
        0,
    ));
    assert_eq!(r, Refusal::Build);
    assert_eq!(relay.rooms(), 0);
}

#[test]
fn a_build_of_another_shape_is_not_read() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let mut odd = entrant(0, "Jake");
    odd.build.digest = "not a digest".to_owned();
    assert_eq!(
        refusal(relay.handle(Request::Open { entrant: odd }, 0)),
        Refusal::Malformed
    );
}

#[test]
fn a_team_that_is_not_three_is_turned_away() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let mut two = entrant(0, "Jake");
    two.team.truncate(2);
    assert!(matches!(
        refusal(relay.handle(Request::Open { entrant: two }, 0)),
        Refusal::Team { .. }
    ));
}

#[test]
fn both_sides_see_the_same_match_begin_from_opposite_ends() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, host, guest) = room(&mut relay);
    let a = viewed(relay.handle(
        Request::Look {
            code: code.clone(),
            seat: host,
        },
        20,
    ));
    let b = viewed(relay.handle(Request::Look { code, seat: guest }, 20));
    assert_eq!((a.stage, b.stage), (Stage::Playing, Stage::Playing));
    assert_eq!((a.you, b.you), (Side::Left, Side::Right));
    assert_eq!(a.seed, b.seed);
    assert_eq!(a.entrants, b.entrants);
    assert_eq!(a.turn, 1);
}

#[test]
fn a_wrong_seat_opens_nothing() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, _, _) = room(&mut relay);
    assert_eq!(
        refusal(relay.handle(
            Request::Look {
                code,
                seat: "0".repeat(32)
            },
            20
        )),
        Refusal::NoSuchRoom
    );
}

#[test]
fn a_turn_locks_when_both_are_in_and_neither_sees_the_other_s_orders_before() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, host, guest) = room(&mut relay);
    let orders: TeamPlan = Vec::new();
    let after_host = viewed(relay.handle(
        Request::Orders {
            code: code.clone(),
            seat: host.clone(),
            turn: 1,
            orders: orders.clone(),
        },
        100,
    ));
    assert!(after_host.locked.is_empty());
    let guest_sees = viewed(relay.handle(
        Request::Look {
            code: code.clone(),
            seat: guest.clone(),
        },
        110,
    ));
    assert!(
        guest_sees.opponent_ready,
        "told the other is ready, not what they ordered"
    );
    assert!(guest_sees.locked.is_empty());
    let after_guest = viewed(relay.handle(
        Request::Orders {
            code: code.clone(),
            seat: guest,
            turn: 1,
            orders: orders.clone(),
        },
        120,
    ));
    assert_eq!(after_guest.locked.len(), 1);
    assert_eq!(after_guest.turn, 2);
    assert_eq!(
        refusal(relay.handle(
            Request::Orders {
                code,
                seat: host,
                turn: 1,
                orders: orders.clone()
            },
            130
        )),
        Refusal::NotThisTurn
    );
}

#[test]
fn a_plan_bigger_than_any_battle_sends_is_refused() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, host, _) = room(&mut relay);
    let take = Directive::Press {
        squares: vec![Tile::new(4, 3)],
    };
    let wide = Directive::Press {
        squares: (0..9).map(|x| Tile::new(x, 0)).collect(),
    };
    let use_item = |by| Directive::UseItem {
        by,
        at: Tile::new(4, 3),
    };
    for orders in [
        vec![take.clone(); 4],
        vec![wide],
        vec![use_item(0); TEAM_SIZE + 1],
    ] {
        assert_eq!(
            refusal(relay.handle(
                Request::Orders {
                    code: code.clone(),
                    seat: host.clone(),
                    turn: 1,
                    orders,
                },
                100,
            )),
            Refusal::Malformed
        );
    }
}

#[test]
fn three_orders_and_a_use_item_for_each_creature_are_a_plan_battle_sends() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, host, _) = room(&mut relay);
    let mut orders: TeamPlan = vec![
        Directive::Hold {
            squares: vec![Tile::new(4, 3)],
            facing: formiga_battle_core::Facing::West,
        };
        3
    ];
    orders.extend((0..TEAM_SIZE as u8).map(|by| Directive::UseItem {
        by,
        at: Tile::new(4, 3),
    }));
    viewed(relay.handle(
        Request::Orders {
            code,
            seat: host,
            turn: 1,
            orders,
        },
        100,
    ));
}

#[test]
fn at_the_deadline_the_turn_locks_with_whatever_was_given_when_anyone_next_asks() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, host, _) = room(&mut relay);
    let v = viewed(relay.handle(
        Request::Look {
            code: code.clone(),
            seat: host.clone(),
        },
        5_000,
    ));
    assert!(v.locked.is_empty());
    let v = viewed(relay.handle(Request::Look { code, seat: host }, 50_000));
    assert_eq!(v.locked.len(), 1);
    assert_eq!(v.locked[0].submitted, [Vec::new(), Vec::new()]);
}

#[test]
fn a_side_with_no_orders_three_turns_running_forfeits_in_words() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, host, _) = room(&mut relay);
    let mut now = 10;
    for turn in 1..=3u8 {
        relay.handle(
            Request::Orders {
                code: code.clone(),
                seat: host.clone(),
                turn,
                orders: Vec::new(),
            },
            now,
        );
        now += 45_000;
    }
    let v = viewed(relay.handle(Request::Look { code, seat: host }, now));
    assert_eq!(v.stage, Stage::Over);
    let ending = v.ending.expect("an ending");
    assert_eq!(ending.forfeited, Some(Side::Right));
    assert!(ending.reason.contains("Friend"), "{}", ending.reason);
}

#[test]
fn different_boards_after_the_same_turn_are_flagged_to_both() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, host, guest) = room(&mut relay);
    let orders: TeamPlan = Vec::new();
    relay.handle(
        Request::Orders {
            code: code.clone(),
            seat: host.clone(),
            turn: 1,
            orders: orders.clone(),
        },
        10,
    );
    relay.handle(
        Request::Orders {
            code: code.clone(),
            seat: guest.clone(),
            turn: 1,
            orders: orders.clone(),
        },
        20,
    );
    relay.handle(
        Request::Board {
            code: code.clone(),
            seat: host.clone(),
            turn: 1,
            hash: 1,
            finished: false,
        },
        30,
    );
    let v = viewed(relay.handle(
        Request::Board {
            code,
            seat: guest,
            turn: 1,
            hash: 2,
            finished: false,
        },
        40,
    ));
    assert!(v.out_of_step);
}

#[test]
fn a_finished_match_is_deleted_once_both_have_seen_its_end() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, host, guest) = room(&mut relay);
    let orders: TeamPlan = Vec::new();
    relay.handle(
        Request::Orders {
            code: code.clone(),
            seat: host.clone(),
            turn: 1,
            orders: orders.clone(),
        },
        10,
    );
    relay.handle(
        Request::Orders {
            code: code.clone(),
            seat: guest.clone(),
            turn: 1,
            orders: orders.clone(),
        },
        20,
    );
    relay.handle(
        Request::Board {
            code: code.clone(),
            seat: host.clone(),
            turn: 1,
            hash: 9,
            finished: true,
        },
        30,
    );
    let v = viewed(relay.handle(
        Request::Board {
            code: code.clone(),
            seat: guest,
            turn: 1,
            hash: 9,
            finished: true,
        },
        40,
    ));
    assert_eq!(v.stage, Stage::Over);
    viewed(relay.handle(
        Request::Look {
            code: code.clone(),
            seat: host.clone(),
        },
        50,
    ));
    // Both have seen the end: the next request finds nothing kept.
    assert_eq!(
        refusal(relay.handle(Request::Look { code, seat: host }, 60)),
        Refusal::NoSuchRoom
    );
    assert_eq!(relay.rooms(), 0);
}

/// Both sides give no orders for `turn` and report `hashes` after it.
fn play_turn(
    relay: &mut Relay,
    (code, host, guest): (&str, &str, &str),
    turn: u8,
    hashes: [u64; 2],
    now: u64,
) -> [View; 2] {
    for seat in [host, guest] {
        relay.handle(
            Request::Orders {
                code: code.to_owned(),
                seat: seat.to_owned(),
                turn,
                orders: Vec::new(),
            },
            now,
        );
    }
    let mut views = Vec::new();
    for (seat, hash) in [host, guest].into_iter().zip(hashes) {
        views.push(viewed(relay.handle(
            Request::Board {
                code: code.to_owned(),
                seat: seat.to_owned(),
                turn,
                hash,
                finished: false,
            },
            now + 1,
        )));
    }
    let [a, b]: [View; 2] = views.try_into().expect("two views");
    [a, b]
}

#[test]
fn a_match_is_confirmed_to_both_only_once_both_boards_after_its_last_turn_are_in() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, host, guest) = room(&mut relay);
    for (seat, at) in [(&host, 10), (&guest, 20)] {
        relay.handle(
            Request::Orders {
                code: code.clone(),
                seat: seat.clone(),
                turn: 1,
                orders: Vec::new(),
            },
            at,
        );
    }
    let v = viewed(relay.handle(
        Request::Board {
            code: code.clone(),
            seat: host.clone(),
            turn: 1,
            hash: 9,
            finished: true,
        },
        30,
    ));
    assert!(!v.confirmed, "the friend's board is not in yet");
    let v = viewed(relay.handle(
        Request::Board {
            code: code.clone(),
            seat: guest.clone(),
            turn: 1,
            hash: 9,
            finished: true,
        },
        40,
    ));
    assert!(v.confirmed && v.stage == Stage::Over);
    let v = viewed(relay.handle(
        Request::Look {
            code: code.clone(),
            seat: host.clone(),
        },
        50,
    ));
    assert!(v.confirmed, "the first to report hears it too");
    assert_eq!(relay.rooms(), 1, "kept until the next request");
    relay.handle(Request::Look { code, seat: host }, 60);
    assert_eq!(relay.rooms(), 0);
}

#[test]
fn boards_that_differ_after_the_last_turn_reach_both_windows_before_the_room_goes() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, host, guest) = room(&mut relay);
    let seats = (code.as_str(), host.as_str(), guest.as_str());
    let cap = formiga_battle_core::Ruleset::default().turn_cap;
    for turn in 1..cap {
        let [a, b] = play_turn(&mut relay, seats, turn, [5, 5], u64::from(turn) * 100);
        assert!(!a.out_of_step && !b.out_of_step);
    }
    // The last turn locks the match over before either board is in; the boards then differ.
    let [a, b] = play_turn(&mut relay, seats, cap, [5, 6], u64::from(cap) * 100);
    assert_eq!(a.stage, Stage::Over);
    assert!(
        !a.confirmed && !a.out_of_step,
        "the friend's board was not in yet"
    );
    assert!(b.out_of_step && !b.confirmed);
    let v = viewed(relay.handle(
        Request::Look {
            code: code.clone(),
            seat: host.clone(),
        },
        u64::from(cap) * 100 + 10,
    ));
    assert!(
        v.out_of_step && !v.confirmed,
        "the first to report hears it"
    );
}

#[test]
fn a_room_its_opener_leaves_before_anyone_joins_closes_at_once() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, host) = seated(relay.handle(
        Request::Open {
            entrant: entrant(0, "Jake"),
        },
        0,
    ));
    assert!(matches!(
        relay.handle(
            Request::Leave {
                code: code.clone(),
                seat: host
            },
            5
        ),
        Answer::Gone {}
    ));
    assert_eq!(relay.rooms(), 0);
    assert_eq!(
        refusal(relay.handle(
            Request::Join {
                code,
                entrant: entrant(9, "Friend"),
            },
            6,
        )),
        Refusal::NoSuchRoom
    );
}

#[test]
fn a_body_too_big_to_read_is_answered_with_a_refusal_rather_than_a_reset() {
    use std::io::{Read, Write};
    let relay = serve_relay("127.0.0.1:0", Limits::default()).unwrap();
    let addr = relay.base();
    let host = addr
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap()
        .to_owned();
    let mut stream = std::net::TcpStream::connect(&host).unwrap();
    let body = vec![b'x'; 200 * 1024];
    write!(
        stream,
        "POST /api/battle/rooms HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .unwrap();
    stream.write_all(&body).unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    assert!(answer.starts_with("HTTP/1.1 400"), "{answer}");
    relay.stop();
}

#[test]
fn a_handlers_card_reaches_their_friend_cleaned_and_an_entrant_without_one_still_plays() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let pennant = |symbol: &str| formiga_battle_contract::Pennant {
        shape: "swallowtail".to_owned(),
        field: "forest".to_owned(),
        symbol: symbol.to_owned(),
        trim: "stitched".to_owned(),
    };
    let mut opener = entrant(0, "Jake");
    opener.card = Some(Card {
        club: Some("  Hedge\u{202E} End   Club ".to_owned()),
        pennant: Some(pennant("acorn")),
        standing: Some("x".repeat(200)),
        trio: Some("The Thistle Three".to_owned()),
        trio_pennant: Some(pennant("Not An Id!")),
    });
    let (code, host) = seated(relay.handle(Request::Open { entrant: opener }, 0));
    // A joiner from before cards, as the site's relay passes it on: no card at all.
    let mut joiner = serde_json::to_value(entrant(9, "Friend")).unwrap();
    assert!(joiner.get("card").is_none());
    joiner["card"] = serde_json::json!({ "club": "\u{200b}", "pennant": null });
    let joiner: Entrant = serde_json::from_value(joiner).unwrap();
    relay.handle(
        Request::Join {
            code: code.clone(),
            entrant: joiner,
        },
        10,
    );
    let v = viewed(relay.handle(Request::Look { code, seat: host }, 20));
    let [a, b] = v.entrants.expect("both are in");
    let card = a.card.expect("the opener's card");
    assert_eq!(card.club.as_deref(), Some("Hedge End Club"));
    assert_eq!(card.pennant, Some(pennant("acorn")));
    assert_eq!(card.standing.map(|s| s.chars().count()), Some(48));
    assert_eq!(card.trio.as_deref(), Some("The Thistle Three"));
    assert_eq!(
        card.trio_pennant, None,
        "a pennant of other shapes is left out"
    );
    assert_eq!(b.card, None, "a card with nothing on it is no card");
}

#[test]
fn a_handlers_key_reaches_their_friend_and_one_that_is_not_a_key_is_left_out() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let mut opener = entrant(0, "Jake");
    opener.handler_key = Some(sample::HANDLER_KEY.to_owned());
    let (code, host) = seated(relay.handle(Request::Open { entrant: opener }, 0));
    let mut joiner = entrant(9, "Friend");
    joiner.handler_key = Some("Not A Key, But Somebody's Email".to_owned());
    relay.handle(
        Request::Join {
            code: code.clone(),
            entrant: joiner,
        },
        10,
    );
    let v = viewed(relay.handle(Request::Look { code, seat: host }, 20));
    let [a, b] = v.entrants.expect("both are in");
    assert_eq!(a.handler_key.as_deref(), Some(sample::HANDLER_KEY));
    assert_eq!(a.key(), Some(sample::HANDLER_KEY));
    assert_eq!(b.handler_key, None, "a key that is not one is left out");

    // An entrant without a key, as a rehearsal sends it, plays as before and sends none on.
    let quiet = serde_json::to_value(entrant(0, "Jake")).unwrap();
    assert!(quiet.get("handler_key").is_none());
}

#[test]
fn a_battle_speaking_the_relay_before_handler_keys_is_told_to_update() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let mut older = entrant(0, "Jake");
    older.protocol = 3;
    assert_eq!(
        refusal(relay.handle(Request::Open { entrant: older }, 0)),
        Refusal::Version {
            relay: RELAY_VERSION,
            rules_version: RULES_VERSION,
        }
    );
}

#[test]
fn the_opener_s_arena_reaches_both_sides_and_a_badly_named_one_is_turned_away() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let mut opener = entrant(0, "Jake");
    opener.arena = "woodland".to_owned();
    let (code, host) = seated(relay.handle(Request::Open { entrant: opener }, 0));
    relay.handle(
        Request::Join {
            code: code.clone(),
            entrant: entrant(9, "Friend"),
        },
        10,
    );
    let v = viewed(relay.handle(Request::Look { code, seat: host }, 20));
    let [a, b] = v.entrants.expect("both are in");
    assert_eq!(
        (a.arena.as_str(), b.arena.as_str()),
        ("woodland", "backyard")
    );

    let mut odd = entrant(0, "Jake");
    odd.arena = "Back Yard!".to_owned();
    assert_eq!(
        refusal(relay.handle(Request::Open { entrant: odd }, 30)),
        Refusal::Malformed
    );
}

#[test]
fn a_room_gone_quiet_is_deleted() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, host, _) = room(&mut relay);
    assert_eq!(
        refusal(relay.handle(Request::Look { code, seat: host }, 10 * 60 * 1000)),
        Refusal::NoSuchRoom
    );
}

#[test]
fn leaving_a_match_forfeits_it() {
    let mut relay = Relay::with_entropy(Limits::default(), counting());
    let (code, host, guest) = room(&mut relay);
    assert!(matches!(
        relay.handle(
            Request::Leave {
                code: code.clone(),
                seat: guest
            },
            10
        ),
        Answer::Gone {}
    ));
    let v = viewed(relay.handle(Request::Look { code, seat: host }, 20));
    assert_eq!(v.ending.and_then(|e| e.forfeited), Some(Side::Right));
}

/// One window's side of a match over HTTP: it works out every turn itself from the locked
/// orders.
struct Window {
    client: RelayClient,
    code: String,
    seat: String,
    m: Option<Match>,
    you: Side,
}

impl Window {
    fn look(&self) -> View {
        self.client.look(&self.code, &self.seat).unwrap()
    }

    fn seat_match(&mut self, view: &View) {
        let [a, b] = view.entrants.as_ref().expect("both in");
        self.you = view.you;
        self.m = Some(
            room_match(
                formiga_battle_core::Arena::backyard(),
                view.ruleset,
                view.seed,
                [&a.team, &b.team],
                [&a.bonds, &b.bonds],
            )
            .unwrap(),
        );
    }

    /// Resolves every locked turn not yet resolved here, reporting each board.
    fn catch_up(&mut self, view: &View) {
        let m = self.m.as_mut().unwrap();
        for locked in &view.locked {
            if locked.turn <= m.board.turn || m.outcome().is_some() {
                continue;
            }
            let report = m.resolve_turn(locked.submitted.clone());
            self.client
                .board(
                    &self.code,
                    &self.seat,
                    report.turn,
                    report.hash,
                    m.outcome().is_some(),
                )
                .unwrap();
        }
    }
}

/// A whole match through the relay `FORMIGA_BATTLE_RELAY` names, for checking another relay
/// (the site's) against this one, or else through a local one.
#[test]
fn two_windows_play_a_whole_match_through_a_relay_and_agree_on_every_board() {
    let named = std::env::var(formiga_battle_room::RELAY_ENV)
        .ok()
        .filter(|v| !v.trim().is_empty());
    let local = match named {
        Some(_) => None,
        None => Some(serve_relay("127.0.0.1:0", Limits::default()).unwrap()),
    };
    let base = named.unwrap_or_else(|| local.as_ref().unwrap().base());
    let client = RelayClient::new(&base);
    let (code, host_seat) = client.open(&entrant(0, "Jake")).unwrap();
    let (_, guest_seat) = client.join(&code, &entrant(9, "Friend")).unwrap();
    let mut host = Window {
        client: client.clone(),
        code: code.clone(),
        seat: host_seat,
        m: None,
        you: Side::Left,
    };
    let mut guest = Window {
        client,
        code,
        seat: guest_seat,
        m: None,
        you: Side::Right,
    };
    let v = host.look();
    host.seat_match(&v);
    let v = guest.look();
    guest.seat_match(&v);
    for _ in 0..20 {
        for w in [&host, &guest] {
            let m = w.m.as_ref().unwrap();
            if m.outcome().is_some() {
                continue;
            }
            let orders = Coach::Exhibition.plan(m, w.you);
            w.client
                .orders(&w.code, &w.seat, m.board.turn + 1, orders)
                .unwrap();
        }
        let v = host.look();
        host.catch_up(&v);
        let v = guest.look();
        guest.catch_up(&v);
        // Once both have reported the end, the room is gone; until then, never out of step.
        if let Ok(v) = guest.client.look(&guest.code, &guest.seat) {
            assert!(!v.out_of_step);
        }
        if host.m.as_ref().unwrap().outcome().is_some() {
            break;
        }
    }
    let (a, b) = (host.m.as_ref().unwrap(), guest.m.as_ref().unwrap());
    assert!(a.outcome().is_some());
    assert_eq!(a.board, b.board);
    // Both reported the end, and both boards were the same; once both have looked, the relay
    // keeps nothing.
    let v = host.client.look(&host.code, &host.seat).unwrap();
    assert!(v.confirmed && !v.out_of_step);
    let _ = guest.client.look(&guest.code, &guest.seat);
    assert!(host.client.look(&host.code, &host.seat).is_err());
    if let Some(relay) = local {
        relay.stop();
    }
}

#[test]
fn an_older_battle_is_told_to_update_whatever_else_its_entrant_lacks() {
    use std::io::{Read, Write};
    let relay = serve_relay("127.0.0.1:0", Limits::default()).unwrap();
    let address = relay
        .base()
        .replace("http://", "")
        .replace("/api/battle", "");
    let body = r#"{"protocol":1,"rules_version":3,"handler":"Jake","team":[{},{},{}]}"#;
    let mut stream = std::net::TcpStream::connect(address).unwrap();
    write!(
        stream,
        "POST /api/battle/rooms HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    assert!(answer.starts_with("HTTP/1.1 409"), "{answer}");
    assert!(
        answer.ends_with(r#"{"refused":"version","relay":4,"rules_version":3}"#),
        "{answer}"
    );
}
