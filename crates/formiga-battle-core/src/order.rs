//! The three universal orders. They say what a Handler intends; the creature works out how.

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
    /// A lever, from a tile next to it.
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
}

impl Order {
    pub const fn verb(&self) -> &'static str {
        match self {
            Self::Move { .. } => "MOVE",
            Self::Hold { .. } => "HOLD",
            Self::Contest { .. } => "CONTEST",
        }
    }
}

/// One team's orders for a turn, by roster place. `None` is an order not given before the timer
/// ran out; the fallback fills it.
pub type Orders = [Option<Order>; crate::arena::TEAM_SIZE];
