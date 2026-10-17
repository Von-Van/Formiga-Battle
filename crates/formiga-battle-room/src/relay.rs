//! The relay friend matches go through, as a state machine: what it keeps, what each request
//! does to it, and what it answers. It never runs the rules. It holds each side's orders until
//! the turn locks (both in, or the deadline passed), hands both sides the same locked orders, and
//! compares the board hashes both sides report, so a mismatch shows at once.
//!
//! This is the reference: the relay on the Formiga site does exactly this, and
//! `docs/RELAY.md` gives the same rules as HTTP requests. A local copy (`formiga-battle relay`)
//! serves this one directly for development and tests.
//!
//! What it keeps, it keeps only while the room is open: the two Handlers' display names and
//! anonymous keys, the six companions as their windows need to draw and play them, the orders and
//! the hashes. A seat's
//! token is kept only as a digest. A room is deleted when its match ends and both sides have seen
//! the end, or when it has gone quiet; nothing outlives it.

use crate::build::{Build, Mismatch, Released};
use crate::code;
use formiga_battle_contract::{Contender, Pennant};
use formiga_battle_core::{Ruleset, Side, TEAM_SIZE, TeamPlan};
use formiga_travel::TravelRelationship;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// The version of the relay's requests and answers. 2: an entrant says which Battle and Desktop
/// it plays with ([`Build`]). 3: a side sends its team's plan, not an order per creature, and an
/// entrant may carry the Handler's [`Card`]. 4: an entrant may carry the Handler's anonymous key
/// ([`Entrant::handler_key`]), so a friend's Battle can know the same Handler again.
pub const RELAY_VERSION: u32 = 4;
/// The largest body the relay reads.
pub const MAX_BODY_BYTES: usize = 64 * 1024;
/// The header a seat's token travels in. Never in a URL, so it is never in a link or a log.
pub const SEAT_HEADER: &str = "X-Formiga-Seat";

/// Milliseconds, on the relay's own clock.
pub type Millis = u64;

/// 64-bit numbers travel as 16 hex digits: the relay on the site is JavaScript, whose numbers
/// lose anything past 53 bits.
pub mod hex_u64 {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(value: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{value:016x}"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let text = String::deserialize(d)?;
        if text.len() != 16 {
            return Err(D::Error::custom("expected 16 hex digits"));
        }
        u64::from_str_radix(&text, 16).map_err(D::Error::custom)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_rooms: usize,
    /// A room nobody joins is deleted after this long.
    pub waiting_ms: Millis,
    /// Added to every turn's deadline for the network and for polling.
    pub grace_ms: Millis,
    /// Locked turns in a row with no orders from a side before it forfeits.
    pub absent_turns: u8,
    /// A room nobody has asked about for this long is deleted.
    pub quiet_ms: Millis,
    /// No room lives longer than this.
    pub lifetime_ms: Millis,
    /// A finished room is kept this long for both sides to see the end.
    pub over_ms: Millis,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_rooms: 500,
            waiting_ms: 15 * 60 * 1000,
            grace_ms: 4000,
            absent_turns: 3,
            quiet_ms: 5 * 60 * 1000,
            lifetime_ms: 2 * 60 * 60 * 1000,
            over_ms: 2 * 60 * 1000,
        }
    }
}

/// Who a side is, as the other side's window needs it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entrant {
    pub protocol: u32,
    pub rules_version: u32,
    /// Which Battle and Desktop it plays with: a joiner's must match the opener's.
    pub build: Build,
    /// A display name for the Handler, as they typed it.
    pub handler: String,
    pub team: Vec<Contender>,
    #[serde(default)]
    pub bonds: Vec<TravelRelationship>,
    /// The arena to play on, by id, from the ones that come with Battle. The opener's is the
    /// room's; a joiner's says nothing.
    #[serde(default = "backyard")]
    pub arena: String,
    /// What the Handler shows their friend of themselves besides their name. Optional, and
    /// cleaned like the name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<Card>,
    /// The Handler's anonymous key: 32 lowercase hex digits Desktop made at random for the colony
    /// and keeps with its Battle record. It says nothing about the Handler, only that it is the
    /// same one as before. None in a rehearsal; a key that is not one is left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handler_key: Option<String>,
}

impl Entrant {
    /// The entrant's Handler key, if it is one: how a window reads a friend's key, so one that is
    /// not a key is never taken for one, whatever the relay passed on.
    pub fn key(&self) -> Option<&str> {
        self.handler_key
            .as_deref()
            .filter(|key| formiga_battle_contract::is_handler_key(key))
    }
}

/// What a Handler shows a friend of themselves besides their name: their club, pennant and
/// standing, and the trio their three are, if they are one. Every part is optional; text is
/// cleaned and bounded as the name is, and a pennant whose parts are not plain ids is left out.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub club: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pennant: Option<Pennant>,
    /// The Handler's standing, as their Battle words it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing: Option<String>,
    /// The name of the trio the three are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trio: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trio_pennant: Option<Pennant>,
}

/// The most characters each of a card's words may have.
pub const MAX_CLUB_CHARS: usize = 32;
pub const MAX_STANDING_CHARS: usize = 48;
pub const MAX_TRIO_CHARS: usize = 32;

impl Card {
    /// The card with every part cleaned: text plain and bounded, an empty part left out, and a
    /// pennant left out unless all four of its parts are plain ids. `None` if nothing is left.
    pub fn cleaned(&self) -> Option<Self> {
        let text = |t: &Option<String>, max: usize| {
            t.as_deref()
                .map(|t| formiga_expansion_rulebook::sanitize_text(t, max))
                .filter(|t| !t.is_empty())
        };
        let flag = |p: &Option<Pennant>| p.clone().filter(Pennant::holds);
        let card = Self {
            club: text(&self.club, MAX_CLUB_CHARS),
            pennant: flag(&self.pennant),
            standing: text(&self.standing, MAX_STANDING_CHARS),
            trio: text(&self.trio, MAX_TRIO_CHARS),
            trio_pennant: flag(&self.trio_pennant),
        };
        (card != Self::default()).then_some(card)
    }
}

fn backyard() -> String {
    "backyard".to_owned()
}

/// The most characters an arena id may have.
pub const MAX_ARENA_CHARS: usize = 32;

/// An arena id as Battle names one: lowercase letters, digits, `_` and `-`.
pub fn arena_id_well_formed(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ARENA_CHARS
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
}

/// One locked turn: the plan each side sent, empty for one not given in time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Locked {
    pub turn: u8,
    pub submitted: [TeamPlan; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Waiting,
    Playing,
    Over,
}

/// How a match with a friend ended, as the relay knows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ending {
    /// Set when a side forfeited or left; otherwise the windows know the winner themselves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forfeited: Option<Side>,
    pub reason: String,
}

/// Everything a side sees of its room.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub relay: u32,
    pub code: String,
    pub stage: Stage,
    pub you: Side,
    /// Both sides, left then right, once the match has begun.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entrants: Option<[Entrant; 2]>,
    #[serde(with = "hex_u64")]
    pub seed: u64,
    pub ruleset: Ruleset,
    /// The turn being played, from 1.
    pub turn: u8,
    /// Milliseconds until it locks.
    pub deadline_ms: u64,
    pub opponent_ready: bool,
    /// Milliseconds since the other side last asked about the room.
    pub opponent_quiet_ms: u64,
    /// Every locked turn so far, oldest first.
    pub locked: Vec<Locked>,
    /// The two sides reported different boards after the same turn.
    pub out_of_step: bool,
    /// The match is over, both sides have reported the board after its last turn, and every
    /// board both reported was the same. Until then a window keeps what the match earned to
    /// itself.
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending: Option<Ending>,
}

/// A request, as the HTTP layer hands it over.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// `POST /rooms`
    Open { entrant: Entrant },
    /// `POST /rooms/{code}/join`
    Join { code: String, entrant: Entrant },
    /// `GET /rooms/{code}`
    Look { code: String, seat: String },
    /// `POST /rooms/{code}/orders`
    Orders {
        code: String,
        seat: String,
        turn: u8,
        orders: TeamPlan,
    },
    /// `POST /rooms/{code}/board`: the board's hash after a turn, and whether the match ended.
    Board {
        code: String,
        seat: String,
        turn: u8,
        hash: u64,
        finished: bool,
    },
    /// `DELETE /rooms/{code}`
    Leave { code: String, seat: String },
}

/// Why a request was turned away. Each has its HTTP status and words a Handler can be shown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "refused", rename_all = "snake_case")]
pub enum Refusal {
    /// 409: this Battle speaks another version of the relay, or plays by other rules than the
    /// friend who opened the room (whose rules version this names).
    Version { relay: u32, rules_version: u32 },
    /// 409: the friend who opened the room plays with another Battle or another Desktop (or in a
    /// rehearsal, with none), whose versions this names.
    Mismatch {
        battle: String,
        desktop: Option<String>,
    },
    /// 409: this Battle is not the same build as the friend's, or not one the relay knows as
    /// released.
    Build,
    /// 404
    NoSuchRoom,
    /// 409
    RoomFull,
    /// 422
    Team { reason: String },
    /// 409
    NotThisTurn,
    /// 400
    Malformed,
    /// 503
    Busy,
}

impl Refusal {
    pub const fn status(&self) -> u16 {
        match self {
            Self::Version { .. }
            | Self::Mismatch { .. }
            | Self::Build
            | Self::RoomFull
            | Self::NotThisTurn => 409,
            Self::NoSuchRoom => 404,
            Self::Team { .. } => 422,
            Self::Malformed => 400,
            Self::Busy => 503,
        }
    }

    pub fn words(&self) -> String {
        match self {
            Self::Version { .. } => {
                "Your Battle and your friend's are different versions; both need the newest."
                    .to_owned()
            }
            Self::Mismatch { battle, desktop } => match desktop {
                Some(desktop) => format!(
                    "Your friend plays with Battle {battle} and Formiga Desktop {desktop}; both of \
                     you need the same Battle and the same Desktop."
                ),
                None => format!(
                    "Your friend is rehearsing with Battle {battle}, without Formiga Desktop; both \
                     of you need the same Battle, opened the same way."
                ),
            },
            Self::Build => "Your Battle is not the same copy as your friend's; both of you need \
                            Battle exactly as it was released."
                .to_owned(),
            Self::NoSuchRoom => "There is no room with that code, or it has closed.".to_owned(),
            Self::RoomFull => "That room already has two Handlers.".to_owned(),
            Self::Team { reason } => format!("That team cannot play: {reason}."),
            Self::NotThisTurn => "Those orders were for a turn that is over.".to_owned(),
            Self::Malformed => "Battle and the relay did not understand each other.".to_owned(),
            Self::Busy => "Friend matches are busy just now; try again in a moment.".to_owned(),
        }
    }
}

/// What the relay answers: a room's code and the seat's token, a view, or a refusal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Answer {
    Seat {
        code: String,
        seat: String,
    },
    View(Box<View>),
    Refused(Refusal),
    /// The room is left; nothing more to say.
    Gone {},
}

impl Answer {
    pub fn status(&self) -> u16 {
        match self {
            Self::Seat { .. } => 201,
            Self::View(_) | Self::Gone {} => 200,
            Self::Refused(r) => r.status(),
        }
    }
}

struct Seat {
    entrant: Entrant,
    seat_digest: String,
    seen: Millis,
    absent: u8,
    /// It has seen the match end.
    saw_end: bool,
}

struct Room {
    created: Millis,
    seed: u64,
    sides: [Option<Seat>; 2],
    stage: Stage,
    turn: u8,
    deadline: Millis,
    pending: [Option<TeamPlan>; 2],
    locked: Vec<Locked>,
    /// Each side's report of the board after a turn: (turn, hash, finished).
    boards: [Option<(u8, u64, bool)>; 2],
    out_of_step: bool,
    ending: Option<Ending>,
    over_since: Option<Millis>,
}

impl Room {
    /// Both sides have reported the board after the last locked turn.
    fn both_reported(&self) -> bool {
        let last = self.locked.len();
        self.boards
            .iter()
            .all(|b| b.is_some_and(|(turn, ..)| usize::from(turn) == last))
    }

    /// The match is over and both windows came to the same board after every turn.
    fn confirmed(&self) -> bool {
        self.stage == Stage::Over
            && !self.out_of_step
            && self.both_reported()
            && self.boards[0].map(|b| b.1) == self.boards[1].map(|b| b.1)
    }
}

pub struct Relay {
    limits: Limits,
    rooms: BTreeMap<String, Room>,
    entropy: Box<dyn FnMut() -> u64 + Send>,
    released: Released,
}

fn digest(seat: &str) -> String {
    Sha256::digest(seat.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn refused(r: Refusal) -> Answer {
    Answer::Refused(r)
}

impl Relay {
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

    pub fn with_entropy(limits: Limits, entropy: Box<dyn FnMut() -> u64 + Send>) -> Self {
        Self {
            limits,
            rooms: BTreeMap::new(),
            entropy,
            released: Released::default(),
        }
    }

    /// The same relay, knowing these released builds.
    pub fn knowing(self, released: Released) -> Self {
        Self { released, ..self }
    }

    pub fn rooms(&self) -> usize {
        self.rooms.len()
    }

    fn seat(&mut self) -> String {
        format!("{:016x}{:016x}", (self.entropy)(), (self.entropy)())
    }

    /// What the relay checks of an entrant: its shape, and that its build is one the relay may
    /// admit. It does not know the rules, so it cannot check a team any further; each window
    /// checks the other's team before it plays.
    fn check(&self, entrant: &Entrant) -> Result<(), Refusal> {
        if entrant.protocol != RELAY_VERSION {
            return Err(Refusal::Version {
                relay: RELAY_VERSION,
                rules_version: entrant.rules_version,
            });
        }
        if entrant.team.len() != TEAM_SIZE || entrant.bonds.len() > 3 {
            return Err(Refusal::Team {
                reason: "a team is three companions".to_owned(),
            });
        }
        if !entrant.build.well_formed() || !arena_id_well_formed(&entrant.arena) {
            return Err(Refusal::Malformed);
        }
        if !self.released.admits(&entrant.build) {
            return Err(Refusal::Build);
        }
        Ok(())
    }

    fn clean(mut entrant: Entrant) -> Entrant {
        let name = formiga_expansion_rulebook::sanitize_text(&entrant.handler, 24);
        entrant.handler = if name.is_empty() {
            "A Handler".to_owned()
        } else {
            name
        };
        entrant.card = entrant.card.as_ref().and_then(Card::cleaned);
        entrant.handler_key = entrant.key().map(str::to_owned);
        entrant
    }

    /// One request, at `now`.
    pub fn handle(&mut self, request: Request, now: Millis) -> Answer {
        self.sweep(now);
        match request {
            Request::Open { entrant } => {
                if let Err(r) = self.check(&entrant) {
                    return refused(r);
                }
                if self.rooms.len() >= self.limits.max_rooms {
                    return refused(Refusal::Busy);
                }
                let code = loop {
                    let code = code::generate(&mut *self.entropy);
                    if !self.rooms.contains_key(&code) {
                        break code;
                    }
                };
                let seat = self.seat();
                let seed = (self.entropy)();
                self.rooms.insert(
                    code.clone(),
                    Room {
                        created: now,
                        seed,
                        sides: [
                            Some(Seat {
                                entrant: Self::clean(entrant),
                                seat_digest: digest(&seat),
                                seen: now,
                                absent: 0,
                                saw_end: false,
                            }),
                            None,
                        ],
                        stage: Stage::Waiting,
                        turn: 0,
                        deadline: 0,
                        pending: [None, None],
                        locked: Vec::new(),
                        boards: [None, None],
                        out_of_step: false,
                        ending: None,
                        over_since: None,
                    },
                );
                Answer::Seat { code, seat }
            }
            Request::Join { code, entrant } => {
                let Some(code) = code::normalize(&code) else {
                    return refused(Refusal::NoSuchRoom);
                };
                if let Err(r) = self.check(&entrant) {
                    return refused(r);
                }
                let seat = self.seat();
                let grace = self.limits.grace_ms;
                let released = &self.released;
                let Some(room) = self.rooms.get_mut(&code) else {
                    return refused(Refusal::NoSuchRoom);
                };
                if room.stage != Stage::Waiting || room.sides[1].is_some() {
                    return refused(Refusal::RoomFull);
                }
                // Both Battles must play by the same rules; the relay only compares the numbers.
                let rules = room.sides[0]
                    .as_ref()
                    .map_or(0, |s| s.entrant.rules_version);
                if entrant.rules_version != rules {
                    return refused(Refusal::Version {
                        relay: RELAY_VERSION,
                        rules_version: rules,
                    });
                }
                // And with the same Battle and Desktop, as unaltered as the relay can tell.
                if let Some(opener) = room.sides[0].as_ref().map(|s| &s.entrant.build) {
                    match released.mismatch(opener, &entrant.build) {
                        Some(Mismatch::Versions) => {
                            return refused(Refusal::Mismatch {
                                battle: opener.battle.clone(),
                                desktop: opener.desktop.clone(),
                            });
                        }
                        Some(Mismatch::Build) => return refused(Refusal::Build),
                        None => {}
                    }
                }
                room.sides[1] = Some(Seat {
                    entrant: Self::clean(entrant),
                    seat_digest: digest(&seat),
                    seen: now,
                    absent: 0,
                    saw_end: false,
                });
                room.stage = Stage::Playing;
                room.turn = 1;
                room.deadline = now + turn_ms(grace);
                Answer::Seat { code, seat }
            }
            Request::Look { code, seat } => self.with_seat(&code, &seat, now, |_, _, _| Ok(())),
            Request::Orders {
                code,
                seat,
                turn,
                orders,
            } => {
                let grace = self.limits.grace_ms;
                self.with_seat(&code, &seat, now, |room, side, now| {
                    if room.stage != Stage::Playing || turn != room.turn {
                        return Err(Refusal::NotThisTurn);
                    }
                    if !plan_well_formed(&orders) {
                        return Err(Refusal::Malformed);
                    }
                    room.pending[side.index()] = Some(orders);
                    if room.pending.iter().all(Option::is_some) {
                        lock(room, now, grace);
                    }
                    Ok(())
                })
            }
            Request::Board {
                code,
                seat,
                turn,
                hash,
                finished,
            } => self.with_seat(&code, &seat, now, |room, side, now| {
                if turn == 0 || turn as usize > room.locked.len() {
                    return Err(Refusal::NotThisTurn);
                }
                room.boards[side.index()] = Some((turn, hash, finished));
                if let [Some(a), Some(b)] = room.boards
                    && a.0 == b.0
                {
                    if a.1 != b.1 {
                        room.out_of_step = true;
                    } else if a.2 && b.2 && room.stage == Stage::Playing {
                        room.stage = Stage::Over;
                        room.over_since = Some(now);
                        room.ending = Some(Ending {
                            forfeited: None,
                            reason: "The match is over.".to_owned(),
                        });
                    }
                }
                Ok(())
            }),
            Request::Leave { code, seat } => {
                let answer = self.with_seat(&code, &seat, now, |room, side, now| {
                    if room.stage == Stage::Waiting {
                        // Nobody else is in it: it closes at once.
                        room.stage = Stage::Over;
                        room.over_since = Some(now);
                    }
                    if room.stage == Stage::Playing {
                        let name = room.sides[side.index()]
                            .as_ref()
                            .map_or("A Handler".to_owned(), |s| s.entrant.handler.clone());
                        room.stage = Stage::Over;
                        room.over_since = Some(now);
                        room.ending = Some(Ending {
                            forfeited: Some(side),
                            reason: format!("{name} left the match."),
                        });
                    }
                    if let Some(s) = room.sides[side.index()].as_mut() {
                        s.saw_end = true;
                    }
                    Ok(())
                });
                self.sweep(now);
                match answer {
                    Answer::View(_) => Answer::Gone {},
                    other => other,
                }
            }
        }
    }

    /// Finds the room and the side a seat belongs to, brings the room up to `now`, applies `f`,
    /// and answers with the side's view.
    fn with_seat(
        &mut self,
        code: &str,
        seat: &str,
        now: Millis,
        f: impl FnOnce(&mut Room, Side, Millis) -> Result<(), Refusal>,
    ) -> Answer {
        let limits = self.limits;
        let Some(code) = code::normalize(code) else {
            return refused(Refusal::NoSuchRoom);
        };
        let Some(room) = self.rooms.get_mut(&code) else {
            return refused(Refusal::NoSuchRoom);
        };
        let wanted = digest(seat);
        let Some(side) = Side::BOTH.into_iter().find(|s| {
            room.sides[s.index()]
                .as_ref()
                .is_some_and(|x| constant_time_eq(&x.seat_digest, &wanted))
        }) else {
            return refused(Refusal::NoSuchRoom);
        };
        if let Some(s) = room.sides[side.index()].as_mut() {
            s.seen = now;
        }
        catch_up(room, now, limits);
        if let Err(r) = f(room, side, now) {
            return refused(r);
        }
        // A side has seen the end once the match is over and it has been told how the boards
        // came out: both sides have reported the board after the last turn. Or the match ended
        // by someone leaving or being away, when there is no board left to report.
        let walked_out = room.ending.as_ref().is_some_and(|e| e.forfeited.is_some());
        if room.stage == Stage::Over
            && (room.both_reported() || walked_out)
            && let Some(s) = room.sides[side.index()].as_mut()
        {
            s.saw_end = true;
        }
        Answer::View(Box::new(view(&code, room, side, now)))
    }

    /// Deletes rooms whose time is up.
    fn sweep(&mut self, now: Millis) {
        let limits = self.limits;
        self.rooms.retain(|_, room| {
            let quiet = room
                .sides
                .iter()
                .flatten()
                .map(|s| s.seen)
                .max()
                .unwrap_or(room.created);
            let expired = now.saturating_sub(room.created) > limits.lifetime_ms
                || now.saturating_sub(quiet) > limits.quiet_ms
                || (room.stage == Stage::Waiting
                    && now.saturating_sub(room.created) > limits.waiting_ms)
                || (room.stage == Stage::Over
                    && (room.sides.iter().flatten().all(|s| s.saw_end)
                        || room
                            .over_since
                            .is_some_and(|t| now.saturating_sub(t) > limits.over_ms)));
            !expired
        });
    }
}

/// Whether a plan is no bigger than any plan Battle sends: the relay never runs the rules, so it
/// checks only that a plan is small enough to keep. A plan is up to `PLAN_SIZE` orders, and a
/// USE ITEM for each creature besides.
pub fn plan_well_formed(plan: &TeamPlan) -> bool {
    let uses = plan.iter().filter(|d| !d.tactical()).count();
    plan.len() - uses <= formiga_battle_core::PLAN_SIZE
        && uses <= TEAM_SIZE
        && plan
            .iter()
            .all(|d| d.squares().len() <= formiga_battle_core::order::MAX_SQUARES)
}

fn turn_ms(grace: Millis) -> Millis {
    u64::from(Ruleset::default().turn_seconds) * 1000 + grace
}

/// Locks the turn being played with whatever each side has sent, and sets the next deadline
/// from `now`. At the turn cap the match is over, whatever the windows have said.
fn lock(room: &mut Room, now: Millis, grace: Millis) {
    let submitted = [
        room.pending[0].clone().unwrap_or_default(),
        room.pending[1].clone().unwrap_or_default(),
    ];
    for side in Side::BOTH {
        if let Some(s) = room.sides[side.index()].as_mut() {
            s.absent = if room.pending[side.index()].is_some() {
                0
            } else {
                s.absent.saturating_add(1)
            };
        }
    }
    room.locked.push(Locked {
        turn: room.turn,
        submitted,
    });
    room.pending = [None, None];
    room.turn += 1;
    room.deadline = now + turn_ms(grace);
    if room.locked.len() >= usize::from(Ruleset::default().turn_cap) {
        room.stage = Stage::Over;
        room.over_since = Some(now);
        room.ending.get_or_insert(Ending {
            forfeited: None,
            reason: "The match is over.".to_owned(),
        });
    }
}

/// Locks every turn whose deadline has passed, and ends the match for a side away too long, or
/// at the turn cap if the windows never said it ended sooner.
fn catch_up(room: &mut Room, now: Millis, limits: Limits) {
    while room.stage == Stage::Playing && now >= room.deadline {
        let due = room.deadline;
        lock(room, due, limits.grace_ms);
        for side in Side::BOTH {
            if room.sides[side.index()]
                .as_ref()
                .is_some_and(|s| s.absent >= limits.absent_turns)
            {
                let name = room.sides[side.index()]
                    .as_ref()
                    .map_or("A Handler".to_owned(), |s| s.entrant.handler.clone());
                room.stage = Stage::Over;
                room.over_since = Some(now);
                room.ending = Some(Ending {
                    forfeited: Some(side),
                    reason: format!(
                        "{name} gave no orders for {} turns in a row.",
                        limits.absent_turns
                    ),
                });
                return;
            }
        }
    }
}

fn view(code: &str, room: &Room, side: Side, now: Millis) -> View {
    let other = room.sides[side.other().index()].as_ref();
    let entrants = match (&room.sides[0], &room.sides[1]) {
        (Some(a), Some(b)) => Some([a.entrant.clone(), b.entrant.clone()]),
        _ => None,
    };
    View {
        relay: RELAY_VERSION,
        code: code.to_owned(),
        stage: room.stage,
        you: side,
        entrants,
        seed: room.seed,
        ruleset: Ruleset::default(),
        turn: room.turn,
        deadline_ms: room.deadline.saturating_sub(now),
        opponent_ready: room.pending[side.other().index()].is_some(),
        opponent_quiet_ms: other.map_or(0, |s| now.saturating_sub(s.seen)),
        locked: room.locked.clone(),
        out_of_step: room.out_of_step,
        confirmed: room.confirmed(),
        ending: room.ending.clone(),
    }
}

/// Compares two strings in time that does not depend on where they differ.
fn constant_time_eq(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}
