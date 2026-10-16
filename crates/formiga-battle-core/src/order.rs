//! What a Handler says, and what each creature ends up doing about it.
//!
//! A Handler gives the whole team a plan: up to three orders a turn, each PRESS some squares,
//! HOLD some squares facing one way, TRICK whoever holds some squares, or INTERACT with something
//! on the board. Nobody is told who goes where; the team shares the plan out among itself
//! (`muster`). On top of the three, each creature carrying something can be told to USE ITEM.
//! What each creature then does is one of MOVE, HOLD, PRESS, TRICK and INTERACT, and those say
//! what it intends; it works out how.
//!
//! PRESS, HOLD and TRICK go round in a circle: a PRESS pushes a TRICK off its feet, a HOLD turns a
//! PRESS away, and a TRICK from the side or behind breaks a HOLD.

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

/// The way a holder faces: one of the four ways a creature can step. North is the far edge of
/// the board (row 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    North,
    East,
    South,
    West,
}

impl Facing {
    pub const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

    /// One step this way.
    pub const fn step(self) -> (i8, i8) {
        match self {
            Self::North => (0, -1),
            Self::East => (1, 0),
            Self::South => (0, 1),
            Self::West => (-1, 0),
        }
    }

    pub const fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
        }
    }

    /// Turned a quarter clockwise, as the planning table turns a HOLD.
    pub const fn turned(self) -> Self {
        match self {
            Self::North => Self::East,
            Self::East => Self::South,
            Self::South => Self::West,
            Self::West => Self::North,
        }
    }

    /// The tile in front of a creature standing at `at`.
    pub fn ahead(self, at: Tile) -> Tile {
        let (dx, dy) = self.step();
        Tile::new(at.x + dx, at.y + dy)
    }

    /// The way from `from` toward `to`: along whichever direction is further, east or west when
    /// they are level. `None` for the same tile.
    pub fn toward(from: Tile, to: Tile) -> Option<Self> {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        match (dx, dy) {
            (0, 0) => None,
            _ if dx.abs() >= dy.abs() => Some(if dx > 0 { Self::East } else { Self::West }),
            _ => Some(if dy > 0 { Self::South } else { Self::North }),
        }
    }

    /// Toward the other team's side of the board.
    pub const fn across(side: Side) -> Self {
        match side {
            Side::Left => Self::East,
            Side::Right => Self::West,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::North => "north",
            Self::East => "east",
            Self::South => "south",
            Self::West => "west",
        }
    }
}

/// Where someone touching a holder stands, as the holder faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quarter {
    Front,
    Side,
    Rear,
}

impl Quarter {
    /// Where `from` stands against a creature at `at` facing `facing`; `None` unless it touches.
    pub fn of(at: Tile, facing: Facing, from: Tile) -> Option<Self> {
        if !at.touches(from) {
            return None;
        }
        Some(if facing.ahead(at) == from {
            Self::Front
        } else if facing.opposite().ahead(at) == from {
            Self::Rear
        } else {
            Self::Side
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "order", rename_all = "snake_case")]
pub enum Order {
    /// Reach a tile. Walks past anyone holding when there is a way round; open to a PRESS the
    /// whole turn.
    Move { to: Tile },
    /// Defend a tile, facing one way: walk there, then stand. Turns a PRESS away, and costs a
    /// presser in front of it composure; a TRICK from the side or behind breaks it. Holds the
    /// beacon.
    Hold { at: Tile, facing: Facing },
    /// Displace and interrupt one of the other team: walk up next to it and push. Pushes anyone
    /// not holding a tile on, stops them for the turn and takes what they carry; turned away by
    /// someone holding unless two come at it from different sides.
    Press { target: Slot },
    /// Outmaneuver one of the other team holding: walk round to its side or back and break its
    /// hold. Fails from the front; caught and pushed by anyone pressing.
    Trick { target: Slot },
    /// Work a lever, or a switch that sets off the arena's map event, from a tile next to it.
    Interact { at: Tile },
}

impl Order {
    pub const fn verb(&self) -> &'static str {
        match self {
            Self::Move { .. } => "MOVE",
            Self::Hold { .. } => "HOLD",
            Self::Press { .. } => "PRESS",
            Self::Trick { .. } => "TRICK",
            Self::Interact { .. } => "INTERACT",
        }
    }

    /// The tile a HOLD is for.
    pub const fn hold_at(&self) -> Option<Tile> {
        match self {
            Self::Hold { at, .. } => Some(*at),
            _ => None,
        }
    }

    /// The one of the other team a PRESS or a TRICK is aimed at.
    pub const fn target(&self) -> Option<Slot> {
        match self {
            Self::Press { target } | Self::Trick { target } => Some(*target),
            _ => None,
        }
    }

    /// Settled once a turn: a PRESS, a TRICK or an INTERACT.
    pub const fn settles(&self) -> bool {
        matches!(
            self,
            Self::Press { .. } | Self::Trick { .. } | Self::Interact { .. }
        )
    }
}

/// One team's creatures' orders for a turn, by roster place, once its plan has been shared out.
/// `None` is an order nobody worked out; the fallback fills it.
pub type Orders = [Option<Order>; crate::arena::TEAM_SIZE];

/// The most orders a Handler gives a team in one turn.
pub const PLAN_SIZE: usize = 3;

/// The most squares one PRESS, HOLD or TRICK names.
pub const MAX_SQUARES: usize = 5;

/// One of a team's orders for the turn. Naming a square in more than one order makes it more
/// pressing, and brings more of the team to it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "order", rename_all = "snake_case")]
pub enum Directive {
    /// Get these squares: walk onto an empty one and hold it, push one of theirs off it, and
    /// flank one of theirs holding there from two sides.
    Press { squares: Vec<Tile> },
    /// Keep these squares: stand on them facing this way, and keep the other team off them.
    Hold { squares: Vec<Tile>, facing: Facing },
    /// Outfox whoever of theirs is on these squares: someone goes round to its side or back and,
    /// once it is holding, breaks its hold.
    Trick { squares: Vec<Tile> },
    /// Go and work something: a found thing on the ground or glinting is fetched, the lever is
    /// pulled, the tap is turned.
    Interact { at: Tile },
    /// A bonus beyond the three orders: the creature in roster place `by`, carrying something
    /// since the turn began, uses it on `at`, within the item's reach of where it stands. Using
    /// takes its first tick; then it carries on with its job.
    UseItem { by: u8, at: Tile },
}

impl Directive {
    pub const fn verb(&self) -> &'static str {
        match self {
            Self::Press { .. } => "PRESS",
            Self::Hold { .. } => "HOLD",
            Self::Trick { .. } => "TRICK",
            Self::Interact { .. } => "INTERACT",
            Self::UseItem { .. } => "USE ITEM",
        }
    }

    /// The squares it names.
    pub fn squares(&self) -> &[Tile] {
        match self {
            Self::Press { squares } | Self::Hold { squares, .. } | Self::Trick { squares } => {
                squares
            }
            Self::Interact { at } | Self::UseItem { at, .. } => std::slice::from_ref(at),
        }
    }

    /// One of the three orders a turn, rather than the bonus USE ITEM.
    pub const fn tactical(&self) -> bool {
        !matches!(self, Self::UseItem { .. })
    }
}

/// A team's orders for a turn, in the order the Handler gave them: up to `PLAN_SIZE` tactical
/// orders, and a USE ITEM for each creature carrying something. Empty is no orders given before
/// the timer ran out: every creature falls back on its own.
pub type TeamPlan = Vec<Directive>;
