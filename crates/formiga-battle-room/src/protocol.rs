//! What a client and a room say to each other: one JSON message a line, each way.
//!
//! The room is authoritative. Each client resolves every turn itself from the orders the room
//! sends back, so it can animate the turn, and checks its board's hash against the room's: a
//! difference is a desync, answered by sending the whole match record again.

use formiga_battle_contract::Contender;
use formiga_battle_core::{Arena, MatchRecord, Orders, Outcome, Ruleset, Side};
use formiga_travel::TravelRelationship;
use serde::{Deserialize, Serialize};

/// The version of these messages. A client and a room that differ refuse each other in words.
pub const PROTOCOL_VERSION: u32 = 1;
/// The longest line either side reads; anything longer is refused unread.
pub const MAX_LINE_BYTES: usize = 128 * 1024;
/// The port a room listens on unless told otherwise.
pub const DEFAULT_PORT: u16 = 7878;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "msg", rename_all = "snake_case")]
pub enum ClientMsg {
    /// The first message on every connection.
    Hello {
        protocol: u32,
        rules_version: u32,
        battle_version: String,
    },
    /// Open a room for a friend to join, with the three companions this Handler brings.
    Create {
        handler: String,
        team: Vec<Contender>,
        /// How the three get on with each other, as Desktop's roster has it.
        #[serde(default)]
        bonds: Vec<TravelRelationship>,
    },
    /// Join a friend's room by its code.
    Join {
        code: String,
        handler: String,
        team: Vec<Contender>,
        #[serde(default)]
        bonds: Vec<TravelRelationship>,
    },
    /// This turn's orders. Sent again, they replace the last, until the room locks the turn.
    Submit { turn: u8, orders: Orders },
    /// Back after losing the connection, with the seat's token.
    Rejoin { code: String, token: String },
    /// The board here does not match the room's: send the whole match again.
    Resync,
    /// Leave the room for good.
    Leave,
}

/// Why the room turned something away, in words a Handler can be shown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum Refusal {
    /// This Battle and the room's speak different versions.
    Version {
        room_protocol: u32,
        room_rules: u32,
    },
    NoSuchRoom,
    RoomFull,
    /// The team sent could not be used: not three, the same companion twice, or a look that
    /// cannot be drawn.
    Team {
        reason: String,
    },
    /// The orders are for a turn that is not the one being played.
    NotThisTurn,
    /// The connection said something out of order, or too much too fast.
    Unexpected,
    /// The room is not taking new rooms just now.
    Busy,
}

impl Refusal {
    pub fn words(&self) -> String {
        match self {
            Self::Version { .. } => {
                "Your Battle and your friend's room are different versions.".to_owned()
            }
            Self::NoSuchRoom => "There is no room with that code, or it has closed.".to_owned(),
            Self::RoomFull => "That room already has two Handlers.".to_owned(),
            Self::Team { reason } => format!("That team cannot play: {reason}."),
            Self::NotThisTurn => "Those orders were for another turn.".to_owned(),
            Self::Unexpected => "The room did not expect that.".to_owned(),
            Self::Busy => "The room service is busy; try again in a moment.".to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "msg", rename_all = "snake_case")]
pub enum ServerMsg {
    /// The room speaks this version.
    Welcome {
        protocol: u32,
        rules_version: u32,
    },
    Refused {
        refusal: Refusal,
    },
    /// A room is open: give the code to a friend. The token lets this Handler back in.
    Created {
        code: String,
        token: String,
    },
    /// Both Handlers are in: the match begins. `you` is the side this Handler plays.
    Started {
        code: String,
        token: String,
        you: Side,
        handlers: [String; 2],
        teams: [Vec<Contender>; 2],
        bonds: [Vec<TravelRelationship>; 2],
        arena: Arena,
        ruleset: Ruleset,
        seed: u64,
        /// Milliseconds until this turn locks.
        deadline_ms: u64,
    },
    /// The other Handler has given this turn's orders.
    OpponentReady {
        turn: u8,
    },
    /// The turn is locked: these are the orders both sides sent, `None` for any not given in
    /// time, which every client fills the same way the room did.
    Resolved {
        turn: u8,
        submitted: [Orders; 2],
        hash: u64,
        outcome: Option<Outcome>,
        /// Milliseconds until the next turn locks, if there is one.
        deadline_ms: u64,
    },
    /// The other Handler's connection dropped; the timer runs on and their orders are filled.
    OpponentAway,
    OpponentBack,
    /// The match as the room has it, for a client coming back or out of step.
    Resume {
        you: Side,
        handlers: [String; 2],
        teams: [Vec<Contender>; 2],
        bonds: [Vec<TravelRelationship>; 2],
        record: MatchRecord,
        deadline_ms: u64,
    },
    /// The match is over without a turn to show: a forfeit, or the room closing.
    Ended {
        outcome: Option<Outcome>,
        reason: String,
    },
}
