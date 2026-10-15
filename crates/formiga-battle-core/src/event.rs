//! What happened in a turn, tick by tick, with the reason behind every choice a creature made. A
//! client animates these; the recap turns them into sentences; a replay compares them.

use crate::grid::Tile;
use crate::order::{Order, Side, Slot};
use serde::{Deserialize, Serialize};

/// Why a creature stepped where it did, when it had a choice. Every reason comes from its
/// profile weighing the board, and every one can be said in words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum Reason {
    /// The plain way there.
    Direct,
    /// Went round so as not to walk into reach of the other team.
    AvoidedExposure,
    /// Went round the capture zone while the other team was in it.
    AvoidedContestedCentre,
    /// Kept beside a teammate.
    KeptNearAlly { ally: Slot },
    /// Kept away from a teammate it does not get on with.
    KeptApartFrom { ally: Slot },
    /// Took an outside lane.
    SideRoute,
    /// Kept close to cover.
    TookCover,
    /// Took its second choice on a whim.
    OnImpulse,
    /// Walked right up past the other team.
    Bold,
    /// With a moment to spare after arriving, moved up beside a teammate.
    FollowedUpToAlly { ally: Slot },
    /// With a moment to spare after arriving, moved out of reach or into cover.
    FollowedUpToCover,
    /// Stepped back after a contest.
    SteppedBack,
}

/// Why the fallback chose an order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fill {
    /// No order arrived before the timer ran out.
    Missing,
    /// The order given could not be carried out at all (a tile off the board, a target on its
    /// own team), so it was treated as missing.
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Victory {
    /// Held the beacon long enough.
    Capture,
    /// Every creature on the other team was knocked out.
    Knockout,
    /// The turn cap came: more capture progress, then more composure left, then more creatures
    /// standing.
    TurnCap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    /// `None` is a draw.
    pub winner: Option<Side>,
    /// How it was won; `None` with a draw.
    pub victory: Option<Victory>,
    pub turn: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum EventKind {
    OrderFilled {
        slot: Slot,
        order: Order,
        fill: Fill,
    },
    Stepped {
        slot: Slot,
        from: Tile,
        to: Tile,
        reason: Reason,
    },
    /// Wanted to step and could not: two wanted the same tile, two wanted to swap, or the tile
    /// ahead was not cleared. Nobody is hurt by a bump.
    Bumped {
        slot: Slot,
        wanted: Tile,
    },
    /// Began holding where it stands.
    Holding {
        slot: Slot,
        at: Tile,
    },
    /// A contest stopped someone who was not holding. They lose one composure for each
    /// contester, capped.
    Interrupted {
        target: Slot,
        by: Vec<Slot>,
    },
    /// Someone holding turned a contest away. Each holder costs one contester a composure. When
    /// contesters outnumber the holders, the holder they pressed loses one too.
    Countered {
        target: Slot,
        holders: Vec<Slot>,
        contesters: Vec<Slot>,
        pressed: bool,
    },
    /// A contest met someone busy with a contest of their own. Nobody is hurt.
    Scuffle {
        contester: Slot,
        target: Slot,
        mutual: bool,
    },
    /// On the way to one contest, took another that walked into reach.
    TookOpportunity {
        slot: Slot,
        meant: Slot,
        took: Slot,
    },
    /// The lever was pulled; the gates swing at the start of the given turn and tick.
    LeverPulled {
        slot: Slot,
        lever: Tile,
        swings_turn: u8,
        swings_tick: u8,
    },
    LeverDefended {
        slot: Slot,
        lever: Tile,
        holders: Vec<Slot>,
    },
    /// The lever was still resting after its last swing.
    LeverResting {
        slot: Slot,
        lever: Tile,
    },
    GateSwung {
        gate: Tile,
        open: bool,
    },
    /// A gate could not close with someone standing in it, and stays open.
    GateJammed {
        gate: Tile,
        by: Slot,
    },
    Composure {
        slot: Slot,
        lost: u8,
        left: u8,
    },
    /// A cartoon knockout: dizzy, it sits out the rest of this match and nothing else.
    KnockedOut {
        slot: Slot,
    },
    CaptureProgress {
        side: Side,
        progress: u8,
    },
    CaptureContested,
    Finished {
        outcome: Outcome,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub turn: u8,
    /// 0 for what happens before the first tick (orders filled); then 1, 2, 3.
    pub tick: u8,
    #[serde(flatten)]
    pub kind: EventKind,
}
