//! Found things: small items that turn up on the board in mirrored pairs, that any creature can
//! pick up by stepping onto them, and that give whoever carries one a single USE. They belong to
//! the board, never to a creature: nothing is kept after the match, and which one turns up, and
//! where, comes from the match seed, so there is no kit to collect and no best one to bring.
//!
//! | Item | USE on a tile within reach | Lasts |
//! |---|---|---|
//! | Dandelion puff | Everyone on the tile and the two either side of it, across the way it is blown, is pushed one tile on and knocked off balance. Two holders side by side brace each other and are not moved. Twigs there blow over | Lands and is gone |
//! | Twig | Wedged into an empty tile: nobody can stand there, and a gate cannot close on it | Two turns |
//! | Leaf umbrella | Opened over a teammate, or itself: the map event cannot soak or wash it and a contest cannot catch it on the move. A puff carries it two tiles instead of one | The rest of this turn and the next |
//! | Pebble bell | Rung: everyone of the other team within two tiles who has not yet settled a contest this turn loses it. Rung beside a switch, it sets the map event off | Lands and is gone |
//! | Honey drop | The tile and the four round it go sticky: whoever steps in stops there for the turn, and can still hold. A pushed creature sticks on the first honey it is pushed onto, and a twig will not stand in it | Two turns |
//! | Springy seed | Its user hops onto the tile, up to two away, over anyone or a twig in between, never over a wall or water | Lands and is gone |
//!
//! Every effect is marked on its tiles the tick it is used and lands the tick after, and none
//! costs composure.

use crate::grid::Tile;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Item {
    /// Push: blows creatures one tile on.
    Puff,
    /// Obstacle: blocks a tile for a while.
    Twig,
    /// Shield: keeps a teammate dry and on its feet.
    Leaf,
    /// Interrupt: rung, it spoils the other team's contests nearby.
    Bell,
    /// Terrain: sticky ground that stops whoever steps in.
    Honey,
    /// Mobility, and rare: a hop of two tiles.
    Seed,
}

impl Item {
    pub const ALL: [Self; 6] = [
        Self::Puff,
        Self::Twig,
        Self::Leaf,
        Self::Bell,
        Self::Honey,
        Self::Seed,
    ];

    /// How far from the tile it is used on its user may stand, in steps.
    pub const fn reach(self) -> u8 {
        match self {
            Self::Puff | Self::Twig | Self::Bell | Self::Honey | Self::Seed => 2,
            Self::Leaf => 1,
        }
    }

    /// How likely it is to be the one that turns up, against the others.
    pub const fn weight(self) -> u64 {
        match self {
            Self::Puff => 26,
            Self::Twig => 22,
            Self::Leaf => 18,
            Self::Bell => 16,
            Self::Honey => 12,
            Self::Seed => 6,
        }
    }

    /// Its name, as the recap says it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Puff => "dandelion puff",
            Self::Twig => "twig",
            Self::Leaf => "leaf umbrella",
            Self::Bell => "pebble bell",
            Self::Honey => "honey drop",
            Self::Seed => "springy seed",
        }
    }
}

/// Turns, from the start of the match, the first items turn up in; and how many turns apart after
/// that. Each is glinted a whole turn before. A match variant can change both (`Finds`).
pub const FIRST_TURN: u8 = 2;
pub const EVERY: u8 = 3;

/// When found things turn up in a match, and how likely each is to be the one: the arena's own
/// rhythm, which a match variant may change. Every arena plays the standard one unless a variant
/// says otherwise, and a record of a match played before variants reads as it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finds {
    /// The first turn found things turn up in. Never the first turn, which has no turn before it
    /// to glint in.
    pub first: u8,
    /// Turns between finds after that; at least 1.
    pub every: u8,
    /// How likely each found thing is, against the others, in `Item::ALL`'s order. A zero never
    /// turns up.
    pub weights: [u8; 6],
}

impl Default for Finds {
    fn default() -> Self {
        Self {
            first: FIRST_TURN,
            every: EVERY,
            weights: Item::ALL.map(|i| i.weight() as u8),
        }
    }
}

impl Finds {
    /// Whether this is the rhythm every arena has without a variant.
    pub fn is_standard(&self) -> bool {
        *self == Self::default()
    }

    /// Whether found things turn up in this turn.
    pub fn due(&self, turn: u8) -> bool {
        self.every > 0 && turn >= self.first && (turn - self.first).is_multiple_of(self.every)
    }

    /// How likely this found thing is, against the others.
    pub fn weight(&self, item: Item) -> u64 {
        let at = Item::ALL
            .iter()
            .position(|i| *i == item)
            .expect("every item is listed");
        u64::from(self.weights[at])
    }

    /// Whether a match can be played by it: found things come round, from the second turn on, and
    /// at least one of them can turn up.
    pub fn playable(&self) -> bool {
        self.first >= 2 && self.every >= 1 && self.weights.iter().any(|w| *w > 0)
    }
}

/// Whole turns a twig stays wedged, or honey sticky.
pub const TWIG_TURNS: u8 = 2;
pub const HONEY_TURNS: u8 = 2;

/// How far a pebble bell is heard.
pub const BELL_REACH: u8 = 2;

/// The tiles a honey drop lands on: the tile and the four round it.
pub fn honey_tiles(at: Tile) -> Vec<Tile> {
    let mut tiles = vec![at];
    tiles.extend(at.steps());
    tiles
}

/// The way a puff blows, from where its user stands toward the tile it is used on: along
/// whichever of the two directions is further, across the board when they are level.
pub fn blowing(from: Tile, to: Tile) -> Option<(i8, i8)> {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    match (dx, dy) {
        (0, 0) => None,
        _ if dx.abs() >= dy.abs() => Some((dx.signum(), 0)),
        _ => Some((0, dy.signum())),
    }
}

/// The tiles a puff used from `from` on `to` blows across: the tile itself and the two either side
/// of it, across the way it blows.
pub fn puff_tiles(from: Tile, to: Tile) -> Vec<Tile> {
    match blowing(from, to) {
        None => Vec::new(),
        Some((dx, dy)) => {
            let (sx, sy) = (dy.abs(), dx.abs());
            vec![
                Tile::new(to.x - sx, to.y - sy),
                to,
                Tile::new(to.x + sx, to.y + sy),
            ]
        }
    }
}
