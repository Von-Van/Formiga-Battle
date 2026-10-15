//! Rooms driven directly and over real sockets: two friends, a code, a whole match, and every
//! board the clients resolve themselves matching the room's.

use formiga_battle_contract::{Contender, sample};
use formiga_battle_core::ai::Coach;
use formiga_battle_core::{Arena, Match, Side, TEAM_SIZE};
use formiga_battle_room::protocol::*;
use formiga_battle_room::roster::{bonds_among, room_match};
use formiga_battle_room::service::{Limits, Service};
use formiga_battle_room::{Client, Heard, serve};
use std::time::Duration;

fn team(seed: u8) -> (Vec<Contender>, Vec<formiga_travel::TravelRelationship>) {
    let roster = if seed == 0 {
        sample::snapshot()
    } else {
        sample::visitors(seed)
    };
    let team: Vec<Contender> = roster.contenders[..TEAM_SIZE].to_vec();
    let bonds = bonds_among(&roster, &team);
    (team, bonds)
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

fn hello() -> ClientMsg {
    ClientMsg::Hello {
        protocol: PROTOCOL_VERSION,
        rules_version: formiga_battle_core::RULES_VERSION,
        battle_version: "test".to_owned(),
    }
}

/// Opens a room on 1 and joins it from 2; gives back the code and the started match.
fn opened(service: &mut Service) -> String {
    service.connected(1);
    service.connected(2);
    service.message(1, hello(), 0);
    service.message(2, hello(), 0);
    let (t, b) = team(0);
    let out = service.message(
        1,
        ClientMsg::Create {
            handler: "Jake".into(),
            team: t,
            bonds: b,
        },
        0,
    );
    let ServerMsg::Created { code, .. } = &out[0].1 else {
        panic!("{out:?}")
    };
    let (t, b) = team(9);
    let out = service.message(
        2,
        ClientMsg::Join {
            code: code.to_lowercase(),
            handler: "Friend".into(),
            team: t,
            bonds: b,
        },
        10,
    );
    assert_eq!(out.len(), 2, "both are told the match has begun");
    assert!(
        out.iter()
            .all(|(_, m)| matches!(m, ServerMsg::Started { .. }))
    );
    code.clone()
}

#[test]
fn a_different_version_is_turned_away_in_words() {
    let mut service = Service::with_entropy(Limits::default(), counting());
    service.connected(1);
    let out = service.message(
        1,
        ClientMsg::Hello {
            protocol: PROTOCOL_VERSION + 1,
            rules_version: 0,
            battle_version: "x".into(),
        },
        0,
    );
    assert!(matches!(
        out[0].1,
        ServerMsg::Refused {
            refusal: Refusal::Version { .. }
        }
    ));
    let out = service.message(1, ClientMsg::Leave, 0);
    assert!(out.is_empty() || matches!(out[0].1, ServerMsg::Refused { .. }));
}

#[test]
fn nothing_is_heard_before_hello() {
    let mut service = Service::with_entropy(Limits::default(), counting());
    service.connected(1);
    let (t, b) = team(0);
    let out = service.message(
        1,
        ClientMsg::Create {
            handler: "Jake".into(),
            team: t,
            bonds: b,
        },
        0,
    );
    assert!(matches!(
        out[0].1,
        ServerMsg::Refused {
            refusal: Refusal::Unexpected
        }
    ));
}

#[test]
fn a_turn_locks_when_both_have_given_orders() {
    let mut service = Service::with_entropy(Limits::default(), counting());
    opened(&mut service);
    let out = service.message(
        1,
        ClientMsg::Submit {
            turn: 1,
            orders: [None; TEAM_SIZE],
        },
        100,
    );
    assert_eq!(out, vec![(2, ServerMsg::OpponentReady { turn: 1 })]);
    let out = service.message(
        2,
        ClientMsg::Submit {
            turn: 1,
            orders: [None; TEAM_SIZE],
        },
        200,
    );
    let resolved = out
        .iter()
        .filter(|(_, m)| matches!(m, ServerMsg::Resolved { turn: 1, .. }))
        .count();
    assert_eq!(resolved, 2, "both are sent the locked turn");
    let out = service.message(
        1,
        ClientMsg::Submit {
            turn: 1,
            orders: [None; TEAM_SIZE],
        },
        300,
    );
    assert!(matches!(
        out[0].1,
        ServerMsg::Refused {
            refusal: Refusal::NotThisTurn
        }
    ));
}

#[test]
fn at_the_deadline_the_turn_locks_with_whatever_was_given() {
    let mut service = Service::with_entropy(Limits::default(), counting());
    opened(&mut service);
    service.message(
        1,
        ClientMsg::Submit {
            turn: 1,
            orders: [None; TEAM_SIZE],
        },
        100,
    );
    assert!(service.tick(5_000).is_empty());
    let out = service.tick(60_000);
    assert!(
        out.iter()
            .any(|(_, m)| matches!(m, ServerMsg::Resolved { turn: 1, .. }))
    );
}

#[test]
fn a_handler_away_three_turns_in_a_row_forfeits_and_is_told_why() {
    let mut service = Service::with_entropy(Limits::default(), counting());
    opened(&mut service);
    let mut now = 100;
    let mut ended = None;
    for turn in 1..=3u8 {
        service.message(
            1,
            ClientMsg::Submit {
                turn,
                orders: [None; TEAM_SIZE],
            },
            now,
        );
        now += 60_000;
        for (_, msg) in service.tick(now) {
            if let ServerMsg::Ended { outcome, reason } = msg {
                ended = Some((outcome, reason));
            }
        }
    }
    let (outcome, reason) = ended.expect("a forfeit");
    assert_eq!(outcome.and_then(|o| o.winner), Some(Side::Left));
    assert!(reason.contains("Friend"), "{reason}");
}

#[test]
fn a_handler_who_drops_out_comes_back_with_the_seat_token_to_the_whole_match() {
    let mut service = Service::with_entropy(Limits::default(), counting());
    service.connected(1);
    service.connected(2);
    service.message(1, hello(), 0);
    service.message(2, hello(), 0);
    let (t, b) = team(0);
    let out = service.message(
        1,
        ClientMsg::Create {
            handler: "Jake".into(),
            team: t,
            bonds: b,
        },
        0,
    );
    let ServerMsg::Created { code, token } = out[0].1.clone() else {
        panic!()
    };
    let (t, b) = team(9);
    service.message(
        2,
        ClientMsg::Join {
            code: code.clone(),
            handler: "Friend".into(),
            team: t,
            bonds: b,
        },
        10,
    );
    let out = service.disconnected(1, 50);
    assert_eq!(out, vec![(2, ServerMsg::OpponentAway)]);
    service.connected(3);
    service.message(3, hello(), 60);
    let out = service.message(
        3,
        ClientMsg::Rejoin {
            code: code.clone(),
            token: "0".repeat(32),
        },
        70,
    );
    assert!(
        matches!(
            out[0].1,
            ServerMsg::Refused {
                refusal: Refusal::NoSuchRoom
            }
        ),
        "a wrong token opens nothing"
    );
    let out = service.message(3, ClientMsg::Rejoin { code, token }, 80);
    assert!(matches!(
        out[0],
        (
            3,
            ServerMsg::Resume {
                you: Side::Left,
                ..
            }
        )
    ));
    assert!(out.contains(&(2, ServerMsg::OpponentBack)));
}

#[test]
fn a_team_that_is_not_three_is_turned_away() {
    let mut service = Service::with_entropy(Limits::default(), counting());
    service.connected(1);
    service.message(1, hello(), 0);
    let (t, _) = team(0);
    let out = service.message(
        1,
        ClientMsg::Create {
            handler: "Jake".into(),
            team: t[..2].to_vec(),
            bonds: vec![],
        },
        0,
    );
    assert!(matches!(
        out[0].1,
        ServerMsg::Refused {
            refusal: Refusal::Team { .. }
        }
    ));
}

/// One client's side of a match: waits for each message it needs, resolving every turn itself.
struct Player {
    client: Client,
    m: Option<Match>,
    you: Side,
}

impl Player {
    fn hear(&self) -> ServerMsg {
        match self.client.hear(Duration::from_secs(10)) {
            Some(Heard::Msg(msg)) => *msg,
            other => panic!("expected a message, heard {other:?}"),
        }
    }

    fn start(&mut self, msg: ServerMsg) {
        let ServerMsg::Started {
            you,
            teams,
            bonds,
            arena,
            ruleset,
            seed,
            ..
        } = msg
        else {
            panic!("{msg:?}")
        };
        self.you = you;
        self.m = Some(
            room_match(
                arena,
                ruleset,
                seed,
                [&teams[0], &teams[1]],
                [&bonds[0], &bonds[1]],
            )
            .unwrap(),
        );
    }
}

#[test]
fn two_friends_play_a_whole_match_over_sockets_and_every_board_agrees_with_the_room() {
    let server = serve("127.0.0.1:0", Limits::default()).unwrap();
    let addr = server.addr().to_string();
    let mut host = Player {
        client: Client::connect(&addr, "test").unwrap(),
        m: None,
        you: Side::Left,
    };
    let mut guest = Player {
        client: Client::connect(&addr, "test").unwrap(),
        m: None,
        you: Side::Right,
    };
    assert!(matches!(host.hear(), ServerMsg::Welcome { .. }));
    assert!(matches!(guest.hear(), ServerMsg::Welcome { .. }));
    let (t, b) = team(0);
    host.client
        .send(&ClientMsg::Create {
            handler: "Jake".into(),
            team: t,
            bonds: b,
        })
        .unwrap();
    let ServerMsg::Created { code, .. } = host.hear() else {
        panic!()
    };
    let (t, b) = team(9);
    guest
        .client
        .send(&ClientMsg::Join {
            code,
            handler: "Friend".into(),
            team: t,
            bonds: b,
        })
        .unwrap();
    let started = guest.hear();
    guest.start(started);
    let started = host.hear();
    host.start(started);
    assert_eq!(host.you, Side::Left);
    assert_eq!(guest.you, Side::Right);
    let mut turns = 0;
    loop {
        for p in [&host, &guest] {
            let m = p.m.as_ref().unwrap();
            let orders = Coach::Exhibition.orders(m, p.you);
            p.client
                .send(&ClientMsg::Submit {
                    turn: m.board.turn + 1,
                    orders,
                })
                .unwrap();
        }
        let mut done = false;
        for p in [&mut host, &mut guest] {
            loop {
                match p.hear() {
                    ServerMsg::OpponentReady { .. } => continue,
                    ServerMsg::Resolved {
                        submitted,
                        hash,
                        outcome,
                        ..
                    } => {
                        let m = p.m.as_mut().unwrap();
                        let report = m.resolve_turn(submitted);
                        assert_eq!(report.hash, hash, "the board here matches the room's");
                        assert_eq!(m.outcome(), outcome);
                        done = outcome.is_some();
                        break;
                    }
                    other => panic!("{other:?}"),
                }
            }
        }
        turns += 1;
        if done || turns > 20 {
            break;
        }
    }
    assert!(host.m.as_ref().unwrap().outcome().is_some());
    assert_eq!(
        host.m.as_ref().unwrap().board,
        guest.m.as_ref().unwrap().board
    );
    let _ = Arena::backyard();
    server.stop();
}
