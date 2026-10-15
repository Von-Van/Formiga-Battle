//! The board's geometry on screen: Formiga Home's 2:1 isometric grid, seen from the near corner.
//!
//! A tile is 48 pixels across and 24 down: half again Home's, so a creature fits its tile and six
//! of them in the beacon's yard can all be told apart. Arena columns (`x`) run down to the right on screen and
//! rows (`y`) down to the left, so the left team starts along the upper-left edge and the right
//! team along the lower-right one. Creatures are drawn upright over the grid, anchored by their
//! feet at the middle of their tile.

use formiga_battle_core::Tile;

pub const TILE_W: i32 = 48;
pub const TILE_H: i32 = 24;
const HALF_W: f32 = (TILE_W / 2) as f32;
const HALF_H: f32 = (TILE_H / 2) as f32;
/// How deep the ground is drawn at the board's near edges.
pub const SLAB: i32 = 8;
/// Room above the far tiles for a creature standing on them, and its pips.
const HEADROOM: f32 = 50.0;
const MARGIN: f32 = 14.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub width: u8,
    pub depth: u8,
    /// The far corner of the board, on screen.
    pub origin: (f32, f32),
    /// The picture's size, in pixels.
    pub size: (u32, u32),
}

impl View {
    /// A board centred in a picture just big enough for it, every creature's head inside.
    pub fn new(width: u8, depth: u8) -> Self {
        let (w, d) = (f32::from(width), f32::from(depth));
        let across = (w + d) * HALF_W + 2.0 * MARGIN;
        let down = HEADROOM + (w + d) * HALF_H + SLAB as f32 + 2.0 * MARGIN;
        let size = (across.ceil() as u32, down.ceil() as u32);
        let origin = ((MARGIN + d * HALF_W).floor(), (MARGIN + HEADROOM).floor());
        Self {
            width,
            depth,
            origin,
            size,
        }
    }

    /// A point on the floor, in tiles, on screen.
    pub fn screen(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.origin.0 + (x - y) * HALF_W,
            self.origin.1 + (x + y) * HALF_H,
        )
    }

    pub fn pixel(&self, x: f32, y: f32) -> (i32, i32) {
        let (sx, sy) = self.screen(x, y);
        (sx.round() as i32, sy.round() as i32)
    }

    /// The middle of a tile, on screen: where a creature on it stands.
    pub fn centre(&self, tile: Tile) -> (i32, i32) {
        self.pixel(f32::from(tile.x) + 0.5, f32::from(tile.y) + 0.5)
    }

    /// The floor position under a point on screen.
    pub fn floor_at(&self, sx: f32, sy: f32) -> (f32, f32) {
        let u = (sx - self.origin.0) / HALF_W;
        let v = (sy - self.origin.1) / HALF_H;
        ((u + v) / 2.0, (v - u) / 2.0)
    }

    /// The tile under a point on screen, if it is on the board.
    pub fn tile_at(&self, sx: f32, sy: f32) -> Option<Tile> {
        let (fx, fy) = self.floor_at(sx, sy);
        (fx >= 0.0 && fy >= 0.0 && fx < f32::from(self.width) && fy < f32::from(self.depth))
            .then(|| Tile::new(fx.floor() as i8, fy.floor() as i8))
    }

    /// A tile's four corners on screen, far one first and clockwise, lifted `lift` pixels.
    pub fn diamond(&self, tile: Tile, lift: f32) -> [(f32, f32); 4] {
        let (x, y) = (f32::from(tile.x), f32::from(tile.y));
        [(x, y), (x + 1.0, y), (x + 1.0, y + 1.0), (x, y + 1.0)].map(|(fx, fy)| {
            let (sx, sy) = self.screen(fx, fy);
            (sx, sy - lift)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tile_is_found_again_from_its_own_middle() {
        let view = View::new(9, 7);
        for y in 0..7 {
            for x in 0..9 {
                let tile = Tile::new(x, y);
                let (sx, sy) = view.centre(tile);
                assert_eq!(view.tile_at(sx as f32, sy as f32), Some(tile));
            }
        }
        assert_eq!(view.tile_at(0.0, 0.0), None);
    }

    #[test]
    fn the_whole_board_fits_its_picture() {
        let view = View::new(9, 7);
        for (x, y) in [(0.0, 0.0), (9.0, 0.0), (9.0, 7.0), (0.0, 7.0)] {
            let (sx, sy) = view.screen(x, y);
            assert!(sx >= 0.0 && sx <= view.size.0 as f32, "{sx}");
            assert!(sy >= 0.0 && sy + SLAB as f32 <= view.size.1 as f32, "{sy}");
        }
    }
}
