//! The rooms, as a state machine with no network and no clock of its own: it is told who said
//! what and when, and answers with who to tell what. The server wires it to sockets; tests drive
//! it directly.
//!
//! A room is opened by one Handler, who is given a short code to pass to a friend. When the
//! friend joins, the match begins: the opener plays the left team, the friend the right. Each
//! turn locks when both have sent orders, or at the deadline, whichever comes first; orders not
//! given in time are filled by the rules' own fallback, the same for both. A Handler who drops
//! out keeps their seat: the timer runs on, and they can come back with their seat's token. Three
//! turns in a row with no orders at all and their side forfeits, which is said in words.

use crate::code;
use crate::protocol::*;
use crate::roster::{check_team, room_match};
use formiga_battle_contract::Contender;
use formiga_battle_core::{Arena, Match, Orders, RULES_VERSION, Ruleset, Side, TEAM_SIZE};
use formiga_travel::TravelRelationship;
use std::collections::{BTreeMap, VecDeque};

/// One connection, as the server numbers them.
pub type ConnId = u64;
/// Milliseconds, on whatever clock the server keeps.
pub type Millis = u64;
/// Who to tell what.
pub type Out = Vec<(ConnId, ServerMsg)>;

/// Bounds the service holds itself to.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_rooms: usize,
    /// A room nobody has joined closes after this long.
    pub waiting_ms: Millis,
    /// Time added to every turn's deadline for the network.
    pub grace_ms: Millis,
    /// Turns in a row with no orders at all before a side forfeits.
    pub absent_turns: u8,
    /// A room with nobody connected closes after this long.
    pub empty_ms: Millis,
    /// A finished room is kept this long for anyone coming back to see the end.
    pub finished_ms: Millis,
    /// At most this many messages from one connection in any second.
    pub per_second: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_rooms: 500,
            waiting_ms: 15 * 60 * 1000,
            grace_ms: 3000,
            absent_turns: 3,
            empty_ms: 2 * 60 * 1000,
            finished_ms: 2 * 60 * 1000,
            per_second: 20,
        }
    }
}

struct Seat {
    handler: String,
    team: Vec<Contender>,
    bonds: Vec<TravelRelationship>,
    conn: Option<ConnId>,
    token: String,
    absent: u8,
}

enum Stage {
    Waiting,
    Playing {
        m: Box<Match>,
        deadline: Millis,
        submitted: [Orders; 2],
        given: [bool; 2],
    },
    Over {
        since: Millis,
    },
}

struct Room {
    created: Millis,
    seats: [Option<Seat>; 2],
    stage: Stage,
    /// When the room last had anyone connected.
    last_seen: Millis,
}

#[derive(Default)]
struct Conn {
    greeted: bool,
    room: Option<String>,
    side: Option<Side>,
    recent: VecDeque<Millis>,
}

pub struct Service {
    limits: Limits,
    rooms: BTreeMap<String, Room>,
    conns: BTreeMap<ConnId, Conn>,
    entropy: Box<dyn FnMut() -> u64 + Send>,
}

fn refuse(conn: ConnId, refusal: Refusal) -> Out {
    vec![(conn, ServerMsg::Refused { refusal })]
}

fn side_index(side: Side) -> usize {
    side.index()
}

impl Service {
    /// A service drawing its codes, tokens and match seeds from the system's randomness.
    pub fn new(limits: Limits) -> Self {
        Self::with_entropy(
            limits,
            Box::new(|| {
                let mut bytes = [0u8; 8];
                getrandom::fill(&mut bytes).expect("the system's randomness");
                u64::from_le_bytes(bytes)
            }),
        )
    }

    /// A service drawing from `entropy` instead: for tests.
    pub fn with_entropy(limits: Limits, entropy: Box<dyn FnMut() -> u64 + Send>) -> Self {
        Self {
            limits,
            rooms: BTreeMap::new(),
            conns: BTreeMap::new(),
            entropy,
        }
    }

    pub fn rooms(&self) -> usize {
        self.rooms.len()
    }

    pub fn connected(&mut self, conn: ConnId) {
        self.conns.insert(conn, Conn::default());
    }

    fn token(&mut self) -> String {
        format!("{:016x}{:016x}", (self.entropy)(), (self.entropy)())
    }

    /// Whatever a connection said.
    pub fn message(&mut self, conn: ConnId, msg: ClientMsg, now: Millis) -> Out {
        let Some(c) = self.conns.get_mut(&conn) else {
            return Vec::new();
        };
        c.recent.push_back(now);
        while c
            .recent
            .front()
            .is_some_and(|t| now.saturating_sub(*t) > 1000)
        {
            c.recent.pop_front();
        }
        if c.recent.len() > self.limits.per_second {
            return refuse(conn, Refusal::Busy);
        }
        let greeted = c.greeted;
        match msg {
            ClientMsg::Hello {
                protocol,
                rules_version,
                ..
            } => {
                if protocol != PROTOCOL_VERSION || rules_version != RULES_VERSION {
                    return refuse(
                        conn,
                        Refusal::Version {
                            room_protocol: PROTOCOL_VERSION,
                            room_rules: RULES_VERSION,
                        },
                    );
                }
                if let Some(c) = self.conns.get_mut(&conn) {
                    c.greeted = true;
                }
                vec![(
                    conn,
                    ServerMsg::Welcome {
                        protocol: PROTOCOL_VERSION,
                        rules_version: RULES_VERSION,
                    },
                )]
            }
            _ if !greeted => refuse(conn, Refusal::Unexpected),
            ClientMsg::Create {
                handler,
                team,
                bonds,
            } => self.create(conn, handler, team, bonds, now),
            ClientMsg::Join {
                code,
                handler,
                team,
                bonds,
            } => self.join(conn, &code, handler, team, bonds, now),
            ClientMsg::Submit { turn, orders } => self.submit(conn, turn, orders, now),
            ClientMsg::Rejoin { code, token } => self.rejoin(conn, &code, &token, now),
            ClientMsg::Resync => self.resume_for(conn, now),
            ClientMsg::Leave => self.disconnected(conn, now),
        }
    }

    fn create(
        &mut self,
        conn: ConnId,
        handler: String,
        team: Vec<Contender>,
        bonds: Vec<TravelRelationship>,
        now: Millis,
    ) -> Out {
        if self.conns.get(&conn).is_some_and(|c| c.room.is_some()) {
            return refuse(conn, Refusal::Unexpected);
        }
        if self.rooms.len() >= self.limits.max_rooms {
            return refuse(conn, Refusal::Busy);
        }
        if let Err(reason) = check_team(&team, &bonds) {
            return refuse(conn, Refusal::Team { reason });
        }
        let code = loop {
            let code = code::generate(&mut *self.entropy);
            if !self.rooms.contains_key(&code) {
                break code;
            }
        };
        let token = self.token();
        self.rooms.insert(
            code.clone(),
            Room {
                created: now,
                seats: [
                    Some(Seat {
                        handler: clean_name(&handler),
                        team,
                        bonds,
                        conn: Some(conn),
                        token: token.clone(),
                        absent: 0,
                    }),
                    None,
                ],
                stage: Stage::Waiting,
                last_seen: now,
            },
        );
        if let Some(c) = self.conns.get_mut(&conn) {
            c.room = Some(code.clone());
            c.side = Some(Side::Left);
        }
        vec![(conn, ServerMsg::Created { code, token })]
    }

    fn join(
        &mut self,
        conn: ConnId,
        code: &str,
        handler: String,
        team: Vec<Contender>,
        bonds: Vec<TravelRelationship>,
        now: Millis,
    ) -> Out {
        let Some(code) = code::normalize(code) else {
            return refuse(conn, Refusal::NoSuchRoom);
        };
        if self.conns.get(&conn).is_some_and(|c| c.room.is_some()) {
            return refuse(conn, Refusal::Unexpected);
        }
        if let Err(reason) = check_team(&team, &bonds) {
            return refuse(conn, Refusal::Team { reason });
        }
        let token = self.token();
        let seed = (self.entropy)();
        let Some(room) = self.rooms.get_mut(&code) else {
            return refuse(conn, Refusal::NoSuchRoom);
        };
        if room.seats[1].is_some() || !matches!(room.stage, Stage::Waiting) {
            return refuse(conn, Refusal::RoomFull);
        }
        room.seats[1] = Some(Seat {
            handler: clean_name(&handler),
            team,
            bonds,
            conn: Some(conn),
            token,
            absent: 0,
        });
        room.last_seen = now;
        let seats = [
            room.seats[0].as_ref().expect("the opener"),
            room.seats[1].as_ref().expect("just seated"),
        ];
        let ruleset = Ruleset::default();
        let m = match room_match(
            Arena::backyard(),
            ruleset,
            seed,
            [&seats[0].team, &seats[1].team],
            [&seats[0].bonds, &seats[1].bonds],
        ) {
            Ok(m) => m,
            Err(error) => {
                room.seats[1] = None;
                return refuse(
                    conn,
                    Refusal::Team {
                        reason: error.to_string(),
                    },
                );
            }
        };
        let deadline = now + u64::from(ruleset.turn_seconds) * 1000 + self.limits.grace_ms;
        let handlers = seats.map(|s| s.handler.clone());
        let teams = seats.map(|s| s.team.clone());
        let bonds = seats.map(|s| s.bonds.clone());
        let tokens = seats.map(|s| s.token.clone());
        let conns = seats.map(|s| s.conn);
        room.stage = Stage::Playing {
            m: Box::new(m),
            deadline,
            submitted: [[None; TEAM_SIZE]; 2],
            given: [false; 2],
        };
        if let Some(c) = self.conns.get_mut(&conn) {
            c.room = Some(code.clone());
            c.side = Some(Side::Right);
        }
        let mut out = Vec::new();
        for side in Side::BOTH {
            if let Some(to) = conns[side.index()] {
                out.push((
                    to,
                    ServerMsg::Started {
                        code: code.clone(),
                        token: tokens[side.index()].clone(),
                        you: side,
                        handlers: handlers.clone(),
                        teams: teams.clone(),
                        bonds: bonds.clone(),
                        arena: Arena::backyard(),
                        ruleset,
                        seed,
                        deadline_ms: deadline - now,
                    },
                ));
            }
        }
        out
    }

    fn submit(&mut self, conn: ConnId, turn: u8, orders: Orders, now: Millis) -> Out {
        let Some((code, side)) = self
            .conns
            .get(&conn)
            .and_then(|c| Some((c.room.clone()?, c.side?)))
        else {
            return refuse(conn, Refusal::Unexpected);
        };
        let Some(room) = self.rooms.get_mut(&code) else {
            return refuse(conn, Refusal::NoSuchRoom);
        };
        let Stage::Playing {
            m,
            submitted,
            given,
            ..
        } = &mut room.stage
        else {
            return refuse(conn, Refusal::Unexpected);
        };
        if turn != m.board.turn + 1 {
            return refuse(conn, Refusal::NotThisTurn);
        }
        // Orders for creatures it does not have, or that cannot be carried out, are left out:
        // the fallback fills them, as for an order not given.
        let mut kept = orders;
        for (i, order) in kept.iter_mut().enumerate() {
            let slot = formiga_battle_core::Slot::new(side, i as u8);
            if order.is_some_and(|o| !m.valid(slot, &o)) {
                *order = None;
            }
        }
        submitted[side_index(side)] = kept;
        let first = !given[side_index(side)];
        given[side_index(side)] = true;
        let mut out = Vec::new();
        if first
            && let Some(other) = room.seats[side.other().index()]
                .as_ref()
                .and_then(|s| s.conn)
        {
            out.push((other, ServerMsg::OpponentReady { turn }));
        }
        if given[0] && given[1] {
            out.extend(self.resolve(&code, now));
        }
        out
    }

    /// Locks and resolves the turn being played.
    fn resolve(&mut self, code: &str, now: Millis) -> Out {
        let limits = self.limits;
        let Some(room) = self.rooms.get_mut(code) else {
            return Vec::new();
        };
        let Stage::Playing {
            m,
            deadline,
            submitted,
            given,
        } = &mut room.stage
        else {
            return Vec::new();
        };
        let sent = *submitted;
        let report = m.resolve_turn(sent);
        for side in Side::BOTH {
            if let Some(seat) = room.seats[side.index()].as_mut() {
                if given[side.index()] {
                    seat.absent = 0;
                } else {
                    seat.absent = seat.absent.saturating_add(1);
                }
            }
        }
        *submitted = [[None; TEAM_SIZE]; 2];
        *given = [false; 2];
        let ruleset = m.ruleset;
        *deadline = now + u64::from(ruleset.turn_seconds) * 1000 + limits.grace_ms;
        let mut outcome = m.outcome();
        let mut forfeit = None;
        if outcome.is_none() {
            for side in Side::BOTH {
                if room.seats[side.index()]
                    .as_ref()
                    .is_some_and(|s| s.absent >= limits.absent_turns)
                {
                    forfeit = Some(side);
                }
            }
        }
        let deadline_ms = *deadline - now;
        let hash = report.hash;
        let turn = report.turn;
        let conns: Vec<ConnId> = room.seats.iter().flatten().filter_map(|s| s.conn).collect();
        let mut out: Out = conns
            .iter()
            .map(|c| {
                (
                    *c,
                    ServerMsg::Resolved {
                        turn,
                        submitted: sent,
                        hash,
                        outcome,
                        deadline_ms,
                    },
                )
            })
            .collect();
        if let Some(side) = forfeit {
            m.forfeit(side);
            outcome = m.outcome();
            let who = room.seats[side.index()]
                .as_ref()
                .map_or("A Handler".to_owned(), |s| s.handler.clone());
            for c in &conns {
                out.push((
                    *c,
                    ServerMsg::Ended {
                        outcome,
                        reason: format!(
                            "{who} was away for {} turns in a row.",
                            limits.absent_turns
                        ),
                    },
                ));
            }
        }
        if outcome.is_some() {
            room.stage = Stage::Over { since: now };
        }
        out
    }

    fn rejoin(&mut self, conn: ConnId, code: &str, token: &str, now: Millis) -> Out {
        let Some(code) = code::normalize(code) else {
            return refuse(conn, Refusal::NoSuchRoom);
        };
        let Some(room) = self.rooms.get_mut(&code) else {
            return refuse(conn, Refusal::NoSuchRoom);
        };
        let Some(side) = Side::BOTH.into_iter().find(|side| {
            room.seats[side.index()]
                .as_ref()
                .is_some_and(|s| constant_time_eq(&s.token, token))
        }) else {
            return refuse(conn, Refusal::NoSuchRoom);
        };
        if let Some(seat) = room.seats[side.index()].as_mut() {
            seat.conn = Some(conn);
        }
        room.last_seen = now;
        let other = room.seats[side.other().index()]
            .as_ref()
            .and_then(|s| s.conn);
        if let Some(c) = self.conns.get_mut(&conn) {
            c.room = Some(code.clone());
            c.side = Some(side);
        }
        let mut out = self.resume_for(conn, now);
        if let Some(other) = other {
            out.push((other, ServerMsg::OpponentBack));
        }
        out
    }

    /// The whole match so far, for one connection.
    fn resume_for(&mut self, conn: ConnId, now: Millis) -> Out {
        let Some((code, side)) = self
            .conns
            .get(&conn)
            .and_then(|c| Some((c.room.clone()?, c.side?)))
        else {
            return refuse(conn, Refusal::Unexpected);
        };
        let Some(room) = self.rooms.get(&code) else {
            return refuse(conn, Refusal::NoSuchRoom);
        };
        let (record, deadline) = match &room.stage {
            Stage::Playing { m, deadline, .. } => (m.record(), *deadline),
            _ => return refuse(conn, Refusal::Unexpected),
        };
        let seats: Vec<&Seat> = room.seats.iter().flatten().collect();
        if seats.len() < 2 {
            return refuse(conn, Refusal::Unexpected);
        }
        vec![(
            conn,
            ServerMsg::Resume {
                you: side,
                handlers: [seats[0].handler.clone(), seats[1].handler.clone()],
                teams: [seats[0].team.clone(), seats[1].team.clone()],
                bonds: [seats[0].bonds.clone(), seats[1].bonds.clone()],
                record,
                deadline_ms: deadline.saturating_sub(now),
            },
        )]
    }

    /// A connection closed or left. Its seat is kept for it to come back to.
    pub fn disconnected(&mut self, conn: ConnId, now: Millis) -> Out {
        let Some(c) = self.conns.remove(&conn) else {
            return Vec::new();
        };
        let (Some(code), Some(side)) = (c.room, c.side) else {
            return Vec::new();
        };
        let Some(room) = self.rooms.get_mut(&code) else {
            return Vec::new();
        };
        if let Some(seat) = room.seats[side.index()].as_mut()
            && seat.conn == Some(conn)
        {
            seat.conn = None;
        }
        room.last_seen = now;
        if matches!(room.stage, Stage::Waiting) {
            self.rooms.remove(&code);
            return Vec::new();
        }
        match room.seats[side.other().index()]
            .as_ref()
            .and_then(|s| s.conn)
        {
            Some(other) if matches!(room.stage, Stage::Playing { .. }) => {
                vec![(other, ServerMsg::OpponentAway)]
            }
            _ => Vec::new(),
        }
    }

    /// Time passing: deadlines locking turns, and rooms closing.
    pub fn tick(&mut self, now: Millis) -> Out {
        let mut out = Vec::new();
        let due: Vec<String> = self
            .rooms
            .iter()
            .filter(|(_, r)| matches!(r.stage, Stage::Playing { deadline, .. } if now >= deadline))
            .map(|(code, _)| code.clone())
            .collect();
        for code in due {
            out.extend(self.resolve(&code, now));
        }
        let limits = self.limits;
        let mut closing = Vec::new();
        for (code, room) in &mut self.rooms {
            if room.seats.iter().flatten().any(|s| s.conn.is_some()) {
                room.last_seen = now;
            }
            let close = match room.stage {
                Stage::Waiting => now.saturating_sub(room.created) > limits.waiting_ms,
                Stage::Playing { .. } => now.saturating_sub(room.last_seen) > limits.empty_ms,
                Stage::Over { since } => now.saturating_sub(since) > limits.finished_ms,
            };
            if close {
                closing.push(code.clone());
            }
        }
        for code in closing {
            if let Some(room) = self.rooms.remove(&code) {
                for seat in room.seats.iter().flatten() {
                    if let Some(c) = seat.conn {
                        if let Some(conn) = self.conns.get_mut(&c) {
                            conn.room = None;
                            conn.side = None;
                        }
                        if matches!(room.stage, Stage::Waiting | Stage::Playing { .. }) {
                            out.push((
                                c,
                                ServerMsg::Ended {
                                    outcome: None,
                                    reason: "The room closed.".to_owned(),
                                },
                            ));
                        }
                    }
                }
            }
        }
        out
    }
}

/// A Handler's name as shown to the other: plain text, at most 24 characters.
fn clean_name(name: &str) -> String {
    let name = formiga_expansion_rulebook::sanitize_text(name, 24);
    if name.is_empty() {
        "A Handler".to_owned()
    } else {
        name
    }
}

/// Compares two tokens in time that does not depend on where they differ.
fn constant_time_eq(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}
