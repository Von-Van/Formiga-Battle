//! What happened in a turn, tick by tick, with the reason behind every choice a creature made. A
//! client animates these; the recap turns them into sentences; a replay compares them.

use crate::arena::Effect;
use crate::grid::Tile;
use crate::items::Item;
use crate::muster::{Because, Task, Why};
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
    /// Waited a tick rather than walk in among the other team.
    HungBack,
    /// Went round tiles that were marked for something about to land on them.
    SkirtedWarning,
    /// Went out of its way for something lying on the ground.
    WentForItem,
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
    /// The other Handler was away too long in a match with a friend.
    Forfeit,
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
    /// The job a creature took on when its team shared out the plan, the order it comes to, and
    /// why it was this creature.
    Assigned {
        slot: Slot,
        task: Task,
        order: Order,
        why: Why,
    },
    /// Partway through the turn its job came to nothing, and it took another from the plan.
    Rethought {
        slot: Slot,
        task: Task,
        order: Order,
        because: Because,
    },
    /// One of a team's orders could not be carried out at all (no square on the board, nothing
    /// there to interact with, or more than three orders), so it was left out.
    OrderDropped {
        side: Side,
        index: u8,
    },
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
    /// Waited a tick rather than walk in among two or more of the other team.
    HungBack {
        slot: Slot,
        at: Tile,
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
    /// Someone holding turned a contest away, with any holders of its team beside it. Each
    /// holder costs one contester a composure.
    Countered {
        target: Slot,
        holders: Vec<Slot>,
        contesters: Vec<Slot>,
    },
    /// Contested from more sides than it had holders to turn them away, a holder's hold broke:
    /// it was pushed off its tile away from a contester (`pushed_to`), or knocked off balance
    /// where it stood if every way out was blocked. Nobody loses composure; it holds nothing more
    /// this turn.
    Flanked {
        target: Slot,
        by: Vec<Slot>,
        from: Tile,
        pushed_to: Option<Tile>,
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
    /// The map event is coming: its tiles are marked until it lands at the start of the given
    /// turn and tick.
    SceneryWarned {
        tiles: Vec<Tile>,
        lands_turn: u8,
        lands_tick: u8,
    },
    /// A switch was turned, and the map event is coming a tick later.
    SwitchTurned {
        slot: Slot,
        switch: Tile,
    },
    /// Someone of the other team holding beside the switch shooed the one turning it off; each
    /// holder costs it one composure, as at the lever.
    SwitchDefended {
        slot: Slot,
        switch: Tile,
        holders: Vec<Slot>,
    },
    /// The switch was still resting, or the map event was already coming.
    SwitchResting {
        slot: Slot,
        switch: Tile,
    },
    /// The map event landed: everyone in `hit` got the effect; those in `sheltered` were under a
    /// leaf umbrella. A wave or the rising water washed each of `moved` toward the middle of the
    /// board (`to` is `None` for one with nowhere to go, knocked off its feet where it stood).
    SceneryLanded {
        effect: Effect,
        tiles: Vec<Tile>,
        hit: Vec<Slot>,
        sheltered: Vec<Slot>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        moved: Vec<Blown>,
    },
    /// Ground that went under is back above the water.
    GroundReopened {
        tiles: Vec<Tile>,
    },
    /// Something is glinting on these spots: this item turns up on each of them at the start of
    /// the given turn.
    ItemsGlint {
        spots: Vec<Tile>,
        item: Item,
        appears_turn: u8,
    },
    ItemAppeared {
        at: Tile,
        item: Item,
    },
    PickedUp {
        slot: Slot,
        item: Item,
        at: Tile,
    },
    /// Knocked off its feet, it let go of what it carried: it lies at `at`, or is lost if
    /// something already lies there.
    Dropped {
        slot: Slot,
        item: Item,
        at: Option<Tile>,
    },
    /// It used what it carried on `at`; the effect lands on `tiles` at the start of the next
    /// tick.
    ItemUsed {
        slot: Slot,
        item: Item,
        at: Tile,
        tiles: Vec<Tile>,
    },
    /// A puff blew across: who was blown where (`to` is `None` for one blown off its feet where
    /// it stood, with nowhere to go), who braced side by side and was not moved, and the twigs
    /// it blew over.
    Puffed {
        by: Slot,
        blown: Vec<Blown>,
        braced: Vec<Slot>,
        twigs: Vec<Tile>,
    },
    /// A twig was wedged into a tile; it falls over at the end of the given turn.
    TwigWedged {
        by: Slot,
        at: Tile,
        until_turn: u8,
    },
    /// A twig fell over, and its tile is open again.
    TwigFell {
        at: Tile,
    },
    /// A twig thrown into water floats there as a raft until the end of the given turn.
    RaftFloated {
        by: Slot,
        at: Tile,
        until_turn: u8,
    },
    RaftSank {
        at: Tile,
    },
    /// A pebble bell rang where its user stood: everyone in `hushed` lost the contest it had not
    /// settled yet.
    Rung {
        by: Slot,
        at: Tile,
        hushed: Vec<Slot>,
    },
    /// Honey spilled over these tiles, sticky until the end of the given turn.
    HoneySpilled {
        by: Slot,
        tiles: Vec<Tile>,
        until_turn: u8,
    },
    HoneyDried {
        tiles: Vec<Tile>,
    },
    /// It got into honey and is stuck where it stands for the rest of the turn; it can still
    /// hold.
    Stuck {
        slot: Slot,
        at: Tile,
    },
    /// A springy seed hopped it two tiles in one go.
    Hopped {
        slot: Slot,
        from: Tile,
        to: Tile,
    },
    /// A gate could not close with a twig wedged in it, and stays open.
    GateWedged {
        gate: Tile,
    },
    /// A leaf umbrella opened over these creatures, until the end of the given turn.
    Sheltering {
        by: Slot,
        slots: Vec<Slot>,
        until_turn: u8,
    },
    /// A contest bounced off a leaf umbrella: nobody is hurt and the one under it goes on.
    Sheltered {
        target: Slot,
        by: Vec<Slot>,
    },
    ShelterFolded {
        slot: Slot,
    },
    /// What was used came to nothing: the tile was taken, or nobody was there to shelter.
    ItemFizzled {
        slot: Slot,
        item: Item,
        at: Tile,
    },
    CaptureContested,
    Finished {
        outcome: Outcome,
    },
}

/// One creature blown by a puff.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blown {
    pub slot: Slot,
    pub from: Tile,
    /// Where it landed; `None` if it had nowhere to go and was knocked off its feet where it
    /// stood.
    pub to: Option<Tile>,
    /// It was under a leaf umbrella, which the puff caught like a sail.
    pub sailed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub turn: u8,
    /// 0 for what happens before the first tick (orders filled); then 1, 2, 3.
    pub tick: u8,
    #[serde(flatten)]
    pub kind: EventKind,
}
