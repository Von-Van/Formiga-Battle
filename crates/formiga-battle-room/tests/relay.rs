//! The relay, driven directly and over HTTP: two friends, a code, a whole match worked out by
//! both windows, and nothing kept once it is over.

use formiga_battle_contract::{Contender, sample};
use formiga_battle_core::ai::Coach;
use formiga_battle_core::{Match, RULES_VERSION, Side, TEAM_SIZE};
use formiga_battle_room::relay::{Answer, Limits, Refusal, Request, Stage};
use formiga_battle_room::roster::{bonds_among, room_match};
use formiga_battle_room::{Entrant, RELAY_VERSION, Relay, RelayClient, View, serve_relay};

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
        handler: handler.to_owned(),
        bonds: bonds_among(&roster, &team),
        team,
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
    let orders = [None; TEAM_SIZE];
    let after_host = viewed(relay.handle(
        Request::Orders {
            code: code.clone(),
            seat: host.clone(),
            turn: 1,
            orders,
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
            orders,
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
                orders
            },
            130
        )),
        Refusal::NotThisTurn
    );
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
    assert_eq!(v.locked[0].submitted, [[None; TEAM_SIZE]; 2]);
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
                orders: [None; TEAM_SIZE],
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
    let orders = [None; TEAM_SIZE];
    relay.handle(
        Request::Orders {
            code: code.clone(),
            seat: host.clone(),
            turn: 1,
            orders,
        },
        10,
    );
    relay.handle(
        Request::Orders {
            code: code.clone(),
            seat: guest.clone(),
            turn: 1,
            orders,
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
    let orders = [None; TEAM_SIZE];
    relay.handle(
        Request::Orders {
            code: code.clone(),
            seat: host.clone(),
            turn: 1,
            orders,
        },
        10,
    );
    relay.handle(
        Request::Orders {
            code: code.clone(),
            seat: guest.clone(),
            turn: 1,
            orders,
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
            let report = m.resolve_turn(locked.submitted);
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

#[test]
fn two_windows_play_a_whole_match_through_a_local_relay_and_agree_on_every_board() {
    let relay = serve_relay("127.0.0.1:0", Limits::default()).unwrap();
    let client = RelayClient::new(&relay.base());
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
            let orders = Coach::Exhibition.orders(m, w.you);
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
    // Both reported the end; once both have looked, the relay keeps nothing.
    let _ = host.client.look(&host.code, &host.seat);
    let _ = guest.client.look(&guest.code, &guest.seat);
    assert!(host.client.look(&host.code, &host.seat).is_err());
    relay.stop();
}
