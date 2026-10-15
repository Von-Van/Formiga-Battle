//! The flat logical grid every arena is laid out on. The isometric picture is drawn over it later;
//! the rules only ever see whole tiles.

use serde::{Deserialize, Serialize};

/// One tile, by column and row. Row 0 is the far edge of the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Tile {
    pub x: i8,
    pub y: i8,
}

impl Tile {
    pub const fn new(x: i8, y: i8) -> Self {
        Self { x, y }
    }

    /// Steps along the grid, never diagonally: how far a walk is.
    pub fn distance(self, other: Self) -> u8 {
        ((self.x - other.x).unsigned_abs()) + ((self.y - other.y).unsigned_abs())
    }

    /// Next to, in one of the four directions a creature can step. A contest reaches only this
    /// far.
    pub fn touches(self, other: Self) -> bool {
        self.distance(other) == 1
    }

    /// Within one tile in any direction, diagonals included: close enough to count as beside
    /// someone.
    pub fn beside(self, other: Self) -> bool {
        self != other && (self.x - other.x).abs() <= 1 && (self.y - other.y).abs() <= 1
    }

    /// The four tiles one step away, in a fixed order.
    pub fn steps(self) -> [Self; 4] {
        [
            Self::new(self.x, self.y - 1),
            Self::new(self.x + 1, self.y),
            Self::new(self.x, self.y + 1),
            Self::new(self.x - 1, self.y),
        ]
    }

    /// The eight tiles around it, in a fixed order.
    pub fn around(self) -> [Self; 8] {
        [
            Self::new(self.x - 1, self.y - 1),
            Self::new(self.x, self.y - 1),
            Self::new(self.x + 1, self.y - 1),
            Self::new(self.x + 1, self.y),
            Self::new(self.x + 1, self.y + 1),
            Self::new(self.x, self.y + 1),
            Self::new(self.x - 1, self.y + 1),
            Self::new(self.x - 1, self.y),
        ]
    }
}

impl std::fmt::Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_contest_reaches_only_the_four_tiles_next_door() {
        let here = Tile::new(3, 3);
        assert!(here.touches(Tile::new(3, 4)));
        assert!(!here.touches(Tile::new(4, 4)));
        assert!(here.beside(Tile::new(4, 4)));
        assert!(!here.beside(here));
        assert_eq!(here.distance(Tile::new(0, 5)), 5);
    }
}
