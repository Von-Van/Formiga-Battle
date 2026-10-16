//! An arena: the tiles, what stands on them, and the props, all authored as data in a small text
//! format so a new arena is a new file rather than new code.
//!
//! ```text
//! id: backyard
//! name: Backyard
//! swing_ticks: 1
//! cooldown_turns: 1
//! . . . . . . . . .
//! 1 . # # G # # . 2
//! . . # Z Z Z # . .
//! ```
//!
//! | Symbol | Tile |
//! |---|---|
//! | `.` | Open ground |
//! | `#` | An obstacle: nobody walks through it |
//! | `Z` | Open ground inside the capture zone, where the beacon counts |
//! | `G` | A gate: closed at the start, walkable only while open |
//! | `g` | A gate that starts open |
//! | `L` | A lever: nobody stands on it; a CONTEST from next door swings every gate |
//! | `1`, `2` | Where the left and right teams start, in reading order; open ground |
//!
//! Gameplay lives only in these tiles. Whatever is drawn over them later is decoration and never
//! decides where anyone can walk.

use crate::grid::Tile;
use crate::order::Side;

/// The arenas that come with Battle, by id, Backyard first. A new arena is a file in `arenas/`
/// and a line here.
const BUILT_IN: &[(&str, &str)] = &[
    ("backyard", include_str!("../../../arenas/backyard.arena")),
    ("woodland", include_str!("../../../arenas/woodland.arena")),
    ("millpond", include_str!("../../../arenas/millpond.arena")),
    ("cove", include_str!("../../../arenas/cove.arena")),
];
use serde::{Deserialize, Serialize};

/// The largest board the rules accept on either side.
pub const MAX_SIDE: usize = 16;
/// How many creatures a team fields.
pub const TEAM_SIZE: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terrain {
    Open,
    Obstacle,
    Gate,
    Lever,
}

/// What a lever does, as the arena file sets it: how long the gates take to swing once it is
/// pulled (shown to everyone while they swing), and how long it rests after.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeverSpec {
    /// Ticks between the pull and the gates moving. At least 1, so a swing is always telegraphed
    /// before it happens.
    pub swing_ticks: u8,
    /// Whole turns after a swing before the lever can be pulled again.
    pub cooldown_turns: u8,
}

impl Default for LeverSpec {
    fn default() -> Self {
        Self {
            swing_ticks: 1,
            cooldown_turns: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arena {
    pub id: String,
    pub name: String,
    pub width: u8,
    pub height: u8,
    /// Row by row, `width * height` of them.
    terrain: Vec<Terrain>,
    /// The beacon's capture zone.
    pub zone: Vec<Tile>,
    pub gates: Vec<Tile>,
    /// Which gates start open.
    pub gates_open: Vec<bool>,
    pub levers: Vec<Tile>,
    pub lever: LeverSpec,
    /// Where each team starts: left, then right.
    pub starts: [[Tile; TEAM_SIZE]; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ArenaError {
    #[error("line {line}: {message}")]
    Line { line: usize, message: String },
    #[error("the arena has no rows")]
    Empty,
    #[error("the arena is larger than {MAX_SIDE} tiles on a side")]
    TooLarge,
    #[error("the arena needs exactly {TEAM_SIZE} starting tiles for each team")]
    Starts,
    #[error("the arena has no capture zone")]
    NoZone,
    #[error("the arena has gates but no lever, or a lever but no gates")]
    Unpaired,
    #[error("the arena needs an id and a name")]
    Unnamed,
}

impl Arena {
    /// Reads an arena file. Anything it does not understand is refused, never guessed at.
    pub fn parse(text: &str) -> Result<Self, ArenaError> {
        let mut id = String::new();
        let mut name = String::new();
        let mut lever = LeverSpec::default();
        let mut rows: Vec<Vec<char>> = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let fail = |message: &str| ArenaError::Line {
                line,
                message: message.to_owned(),
            };
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }
            if let Some((key, value)) = trimmed.split_once(':') {
                let value = value.trim();
                let number = || value.parse::<u8>().map_err(|_| fail("expected a number"));
                match key.trim() {
                    "id" => id = value.to_owned(),
                    "name" => name = value.to_owned(),
                    "swing_ticks" => lever.swing_ticks = number()?.max(1),
                    "cooldown_turns" => lever.cooldown_turns = number()?,
                    _ => return Err(fail("unknown setting")),
                }
                continue;
            }
            let row: Vec<char> = trimmed
                .split_whitespace()
                .map(|cell| {
                    let mut chars = cell.chars();
                    match (chars.next(), chars.next()) {
                        (Some(symbol), None) if ".#ZGgL12".contains(symbol) => Ok(symbol),
                        _ => Err(fail("unknown tile symbol")),
                    }
                })
                .collect::<Result<_, _>>()?;
            if rows.first().is_some_and(|first| first.len() != row.len()) {
                return Err(fail("every row must be as wide as the first"));
            }
            rows.push(row);
        }
        if id.is_empty() || name.is_empty() {
            return Err(ArenaError::Unnamed);
        }
        let height = rows.len();
        let width = rows.first().map_or(0, Vec::len);
        if height == 0 || width == 0 {
            return Err(ArenaError::Empty);
        }
        if height > MAX_SIDE || width > MAX_SIDE {
            return Err(ArenaError::TooLarge);
        }
        let mut arena = Self {
            id,
            name,
            width: width as u8,
            height: height as u8,
            terrain: Vec::with_capacity(width * height),
            zone: Vec::new(),
            gates: Vec::new(),
            gates_open: Vec::new(),
            levers: Vec::new(),
            lever,
            starts: [[Tile::new(0, 0); TEAM_SIZE]; 2],
        };
        let mut starts: [Vec<Tile>; 2] = [Vec::new(), Vec::new()];
        for (y, row) in rows.iter().enumerate() {
            for (x, symbol) in row.iter().enumerate() {
                let tile = Tile::new(x as i8, y as i8);
                let terrain = match symbol {
                    '#' => Terrain::Obstacle,
                    'G' | 'g' => {
                        arena.gates.push(tile);
                        arena.gates_open.push(*symbol == 'g');
                        Terrain::Gate
                    }
                    'L' => {
                        arena.levers.push(tile);
                        Terrain::Lever
                    }
                    'Z' => {
                        arena.zone.push(tile);
                        Terrain::Open
                    }
                    '1' => {
                        starts[0].push(tile);
                        Terrain::Open
                    }
                    '2' => {
                        starts[1].push(tile);
                        Terrain::Open
                    }
                    _ => Terrain::Open,
                };
                arena.terrain.push(terrain);
            }
        }
        for (side, tiles) in starts.iter().enumerate() {
            arena.starts[side] = tiles
                .as_slice()
                .try_into()
                .map_err(|_| ArenaError::Starts)?;
        }
        if arena.zone.is_empty() {
            return Err(ArenaError::NoZone);
        }
        if arena.gates.is_empty() != arena.levers.is_empty() {
            return Err(ArenaError::Unpaired);
        }
        Ok(arena)
    }

    /// The first arena: a cross-shaped beacon yard in the middle, a gate in a short wall across
    /// the far lane, and the lever that swings it on the near side.
    pub fn backyard() -> Self {
        Self::parse(BUILT_IN[0].1).expect("Backyard parses")
    }

    /// Every arena that comes with Battle, Backyard first. Both windows of a friend match must
    /// play the same one, so an arena is named by its id and never sent as text.
    pub fn built_in() -> Vec<Self> {
        BUILT_IN
            .iter()
            .map(|(id, text)| Self::parse(text).unwrap_or_else(|e| panic!("{id} parses: {e}")))
            .collect()
    }

    /// The built-in arena with this id.
    pub fn by_id(id: &str) -> Option<Self> {
        BUILT_IN
            .iter()
            .find(|(known, _)| *known == id)
            .and_then(|(_, text)| Self::parse(text).ok())
    }

    pub fn contains(&self, tile: Tile) -> bool {
        tile.x >= 0 && tile.y >= 0 && (tile.x as u8) < self.width && (tile.y as u8) < self.height
    }

    pub fn terrain(&self, tile: Tile) -> Terrain {
        if !self.contains(tile) {
            return Terrain::Obstacle;
        }
        self.terrain[tile.y as usize * self.width as usize + tile.x as usize]
    }

    pub fn in_zone(&self, tile: Tile) -> bool {
        self.zone.contains(&tile)
    }

    /// How far a tile is across the board from `side`'s own edge: the column, counted from the
    /// left for the left team and from the right for the right. Every choice between tiles that
    /// are otherwise equal goes by it, so the board plays the same from both sides.
    pub fn across(&self, side: Side, t: Tile) -> i8 {
        match side {
            Side::Left => t.x,
            Side::Right => self.width as i8 - 1 - t.x,
        }
    }

    /// Every tile, row by row.
    pub fn tiles(&self) -> impl Iterator<Item = Tile> + '_ {
        (0..self.height as i8)
            .flat_map(move |y| (0..self.width as i8).map(move |x| Tile::new(x, y)))
    }

    /// The middle of the capture zone, rounded toward the first zone tile.
    pub fn zone_centre(&self) -> Tile {
        let n = self.zone.len() as i32;
        let (sx, sy) = self.zone.iter().fold((0i32, 0i32), |(sx, sy), t| {
            (sx + t.x as i32, sy + t.y as i32)
        });
        let centre = Tile::new((sx / n) as i8, (sy / n) as i8);
        if self.in_zone(centre) {
            centre
        } else {
            self.zone[0]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backyard_reads_with_a_cross_shaped_yard_three_starts_a_side_and_one_lever_for_its_gate() {
        let arena = Arena::backyard();
        assert_eq!(arena.id, "backyard");
        assert_eq!(arena.zone.len(), 5);
        assert_eq!(arena.gates.len(), 1);
        assert_eq!(arena.levers.len(), 1);
        assert_eq!(arena.zone_centre(), Tile::new(4, 3));
        for side in 0..2 {
            for tile in arena.starts[side] {
                assert_eq!(arena.terrain(tile), Terrain::Open);
            }
        }
    }

    #[test]
    fn every_built_in_arena_reads_under_its_own_id_and_is_the_same_for_both_teams() {
        for arena in Arena::built_in() {
            assert_eq!(
                Arena::by_id(&arena.id).map(|a| a.id),
                Some(arena.id.clone())
            );
            let mirror = |t: Tile| Tile::new(arena.width as i8 - 1 - t.x, t.y);
            for tile in arena.tiles() {
                assert_eq!(
                    arena.terrain(tile),
                    arena.terrain(mirror(tile)),
                    "{}",
                    arena.id
                );
                assert_eq!(
                    arena.in_zone(tile),
                    arena.in_zone(mirror(tile)),
                    "{}",
                    arena.id
                );
            }
        }
        assert!(Arena::by_id("nowhere").is_none());
    }

    #[test]
    fn backyard_is_the_same_for_both_teams() {
        let arena = Arena::backyard();
        let mirror = |t: Tile| Tile::new(arena.width as i8 - 1 - t.x, t.y);
        for tile in arena.tiles() {
            assert_eq!(arena.terrain(tile), arena.terrain(mirror(tile)), "{tile}");
            assert_eq!(arena.in_zone(tile), arena.in_zone(mirror(tile)), "{tile}");
        }
        for (left, right) in arena.starts[0].iter().zip(arena.starts[1]) {
            assert_eq!(mirror(*left), right);
        }
    }

    #[test]
    fn an_arena_with_a_stray_symbol_is_refused_with_its_line() {
        let text = "id: x\nname: X\n1 1 1 Z\n2 2 2 ?\n";
        assert!(matches!(
            Arena::parse(text),
            Err(ArenaError::Line { line: 4, .. })
        ));
    }

    #[test]
    fn a_gate_without_a_lever_is_refused() {
        let text = "id: x\nname: X\n1 1 1 Z\n2 2 2 G\n";
        assert_eq!(Arena::parse(text), Err(ArenaError::Unpaired));
    }
}
