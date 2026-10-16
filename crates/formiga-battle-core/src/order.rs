//! What a Handler says, and what each creature ends up doing about it.
//!
//! A Handler gives the whole team a plan: up to three orders a turn, each TAKE some squares,
//! DEFEND some squares or INTERACT with something on the board. Nobody is told who goes where;
//! the team shares the plan out among itself (`muster`). What each creature then does is one of
//! MOVE, HOLD, CONTEST and USE, and those say what it intends; it works out how.

use crate::grid::Tile;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub const BOTH: [Self; 2] = [Self::Left, Self::Right];

    pub const fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }

    pub const fn other(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

/// One creature in a match: its team, and its place in that team's roster.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Slot {
    pub side: Side,
    pub index: u8,
}

impl Slot {
    pub const fn new(side: Side, index: u8) -> Self {
        Self { side, index }
    }

    /// A number unique to the slot, for rolls.
    pub const fn key(self) -> u64 {
        (self.side.index() as u64) * 8 + self.index as u64
    }

    /// Every slot, left team first.
    pub fn all() -> impl Iterator<Item = Self> {
        Side::BOTH
            .into_iter()
            .flat_map(|side| (0..crate::arena::TEAM_SIZE as u8).map(move |i| Self::new(side, i)))
    }
}

/// What a CONTEST is aimed at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    /// A creature on the other team.
    Creature { slot: Slot },
    /// A lever, or a switch that sets off the arena's map event, from a tile next to it.
    Lever { at: Tile },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "order", rename_all = "snake_case")]
pub enum Order {
    /// Reach a tile. Walks past anyone holding when there is a way round; open to a contest the
    /// whole turn.
    Move { to: Tile },
    /// Defend a tile: walk there, then stand. Counters a contest; holds the beacon.
    Hold { at: Tile },
    /// Disrupt a creature of the other team, or pull a lever. Interrupts someone moving; loses
    /// to someone holding.
    Contest { target: Target },
    /// Use what it is carrying on a tile: walks until it is in reach, and the item's effect lands
    /// on that tile a tick later. Only for a creature carrying something; the item belongs to
    /// the board, never to the creature, and is gone once used.
    Use { at: Tile },
}

impl Order {
    pub const fn verb(&self) -> &'static str {
        match self {
            Self::Move { .. } => "MOVE",
            Self::Hold { .. } => "HOLD",
            Self::Contest { .. } => "CONTEST",
            Self::Use { .. } => "USE",
        }
    }
}

/// One team's creatures' orders for a turn, by roster place, once its plan has been shared out.
/// `None` is an order nobody worked out; the fallback fills it.
pub type Orders = [Option<Order>; crate::arena::TEAM_SIZE];

/// The most orders a Handler gives a team in one turn.
pub const PLAN_SIZE: usize = 3;

/// The most squares one TAKE or DEFEND names.
pub const MAX_SQUARES: usize = 5;

/// One of a team's orders for the turn. Naming a square in more than one order makes it more
/// pressing, and brings more of the team to it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "order", rename_all = "snake_case")]
pub enum Directive {
    /// Get these squares and hold them: walk onto an empty one, move one of theirs off it.
    Take { squares: Vec<Tile> },
    /// Keep these squares: stand on them, and keep the other team off them.
    Defend { squares: Vec<Tile> },
    /// Go and get something: a found thing on the ground or glinting, the lever, the tap, or one
    /// of theirs carrying something. One of ours carrying something uses it.
    Interact { at: Tile },
}

impl Directive {
    pub const fn verb(&self) -> &'static str {
        match self {
            Self::Take { .. } => "TAKE",
            Self::Defend { .. } => "DEFEND",
            Self::Interact { .. } => "INTERACT",
        }
    }

    /// The squares it names.
    pub fn squares(&self) -> &[Tile] {
        match self {
            Self::Take { squares } | Self::Defend { squares } => squares,
            Self::Interact { at } => std::slice::from_ref(at),
        }
    }
}

/// A team's orders for a turn, in the order the Handler gave them. Empty is no orders given
/// before the timer ran out: every creature falls back on its own.
pub type TeamPlan = Vec<Directive>;
