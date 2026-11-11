//! A place as it is seen: its ground, then everyone in it from the back row forward, then
//! whatever stands in front of them. Until a place has its hand-drawn pictures, its ground is a
//! stand-in painted here from its tiles, plain enough that nobody mistakes it for the art, to
//! Home's rules all the same: every material a ramp, light from the upper left, edges in a darker
//! shade of their own colour, grain from `paint::noise`.

use super::map::{Ground, Place};
use super::sheet::{CELL, FEET};
use formiga_art::{Canvas, Rgba, paint};

/// A tile's size, in pixels.
pub const TILE: i32 = 32;
/// How much of a place the window shows at once, in pixels: a room exactly, the campus in part.
pub const VIEW: (i32, i32) = (416, 234);

/// A material: its four shades, darkest first.
type Ramp = [Rgba; 4];

const fn rgb(hex: u32) -> Rgba {
    Rgba::new((hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255)
}

const WOOD: Ramp = [rgb(0x5b3a27), rgb(0x7f5436), rgb(0xa06d44), rgb(0xbf8a57)];
const RUG: Ramp = [rgb(0x4a2230), rgb(0x6b2f3c), rgb(0x8a3f48), rgb(0xa8585a)];
const STONE: Ramp = [rgb(0x7d6f5a), rgb(0xa4937a), rgb(0xc4b393), rgb(0xdccdaa)];
const GRASS: Ramp = [rgb(0x3d5a32), rgb(0x52773f), rgb(0x6b934c), rgb(0x86ad5e)];
const PLASTER: Ramp = [rgb(0x8f7a5c), rgb(0xb59d76), rgb(0xd3bf95), rgb(0xe9dbb5)];
const WATER: Ramp = [rgb(0x2c4f66), rgb(0x3b6a85), rgb(0x4f87a3), rgb(0x79aec4)];
const LEAF: Ramp = [rgb(0x22382a), rgb(0x2f4f37), rgb(0x3f6a45), rgb(0x578656)];
const BRASS: Ramp = [rgb(0x5e4520), rgb(0x8a6a2c), rgb(0xb48d3c), rgb(0xd8b45a)];
const DOOR: Ramp = [rgb(0x2f1c12), rgb(0x48291a), rgb(0x5b3a27), rgb(0x7a5034)];

/// The stand-in ground of a place: each tile painted as what it is.
pub fn stand_in(place: &Place) -> Canvas {
    let mut canvas = Canvas::new((place.width * TILE) as u32, (place.height * TILE) as u32);
    for y in 0..place.height {
        for x in 0..place.width {
            tile(&mut canvas, place, (x, y));
        }
    }
    canvas
}

fn tile(canvas: &mut Canvas, place: &Place, (tx, ty): (i32, i32)) {
    let ground = place.ground((tx, ty));
    let (ox, oy) = (tx * TILE, ty * TILE);
    let salt = place.id.key().len() as u32;
    // A flat material with a little grain, and its top and left edges lit.
    let fill = |canvas: &mut Canvas, ramp: Ramp, edges: bool| {
        for y in 0..TILE {
            for x in 0..TILE {
                let (px, py) = (ox + x, oy + y);
                let grain = paint::noise(px, py, salt) % 9;
                let mut shade = if grain == 0 { 1 } else { 2 };
                if edges && (x == 0 || y == 0) {
                    shade = 3;
                } else if edges && (x == TILE - 1 || y == TILE - 1) {
                    shade = 0;
                }
                canvas.set(px, py, ramp[shade]);
            }
        }
    };
    match ground {
        Ground::Floor => {
            // Boards running across, a seam every eight pixels.
            fill(canvas, WOOD, false);
            for y in (0..TILE).step_by(8) {
                for x in 0..TILE {
                    canvas.set(ox + x, oy + y, WOOD[1]);
                }
            }
        }
        Ground::Rug => fill(canvas, RUG, false),
        Ground::Lane => fill(canvas, STONE, true),
        Ground::Grass => fill(canvas, GRASS, false),
        Ground::Wall => {
            // Inside, plaster; outside, the whole block a roof seen from above.
            let ramp = if place.id == super::map::PlaceId::Campus {
                RUG
            } else {
                PLASTER
            };
            fill(canvas, ramp, true);
        }
        Ground::Water => {
            fill(canvas, WATER, false);
            for x in (4..TILE - 4).step_by(9) {
                canvas.fill_rect(ox + x, oy + 12 + (x % 3) * 4, 5, 1, WATER[3]);
            }
        }
        Ground::Tree => {
            fill(canvas, GRASS, false);
            canvas.fill_circle(ox + 16, oy + 15, 14, LEAF[0]);
            canvas.fill_circle(ox + 15, oy + 14, 12, LEAF[2]);
            canvas.fill_circle(ox + 12, oy + 10, 5, LEAF[3]);
        }
        Ground::Ledge => {
            fill(canvas, GRASS, false);
            canvas.fill_rect(ox, oy + TILE - 10, TILE, 10, STONE[0]);
            canvas.fill_rect(ox, oy + TILE - 10, TILE, 2, STONE[2]);
        }
        Ground::Thing(_) => {
            // A plain crate of brass and wood: the tile it stands on is what the art replaces.
            let under = if place.id == super::map::PlaceId::Campus {
                GRASS
            } else {
                WOOD
            };
            fill(canvas, under, false);
            canvas.fill_rect(ox + 3, oy + 3, TILE - 6, TILE - 6, BRASS[0]);
            canvas.fill_rect(ox + 4, oy + 4, TILE - 8, TILE - 8, BRASS[2]);
            canvas.fill_rect(ox + 4, oy + 4, TILE - 8, 2, BRASS[3]);
        }
        Ground::Door(_) => {
            fill(canvas, DOOR, false);
            canvas.fill_rect(ox + 6, oy + 2, TILE - 12, TILE - 2, DOOR[1]);
            canvas.fill_rect(ox + 6, oy + 2, TILE - 12, 2, DOOR[3]);
            canvas.fill_rect(ox + TILE - 11, oy + 16, 2, 2, BRASS[3]);
        }
    }
}

/// Someone drawn standing on a tile: a 48-pixel frame with its feet at the tile's foot, where
/// feet stand on a top-down floor.
pub fn stand(canvas: &mut Canvas, frame: &Canvas, feet: (f32, f32)) {
    let x = (feet.0 * TILE as f32).round() as i32 + TILE / 2 - CELL / 2;
    let y = (feet.1 * TILE as f32).round() as i32 + TILE - 4 - FEET;
    for fy in 0..CELL {
        for fx in 0..CELL {
            let top = frame.get(fx, fy);
            if top.a == 0 {
                continue;
            }
            let bottom = canvas.get(x + fx, y + fy);
            canvas.set(x + fx, y + fy, paint::over(top, bottom));
        }
    }
}

/// The part of a place the window shows, in its pixels: centred on the Handler and kept inside
/// the place, or the whole place centred when it is smaller than the window.
pub fn camera(place: &Place, feet: (f32, f32)) -> (i32, i32) {
    let (w, h) = (place.width * TILE, place.height * TILE);
    let axis = |size: i32, view: i32, at: f32| {
        if size <= view {
            (size - view) / 2
        } else {
            let centre = (at * TILE as f32) as i32 + TILE / 2;
            (centre - view / 2).clamp(0, size - view)
        }
    };
    (axis(w, VIEW.0, feet.0), axis(h, VIEW.1, feet.1))
}

/// A frame tinted toward a colour, for the townspeople until they have sheets of their own.
pub fn tinted(frame: &Canvas, tint: Rgba) -> Canvas {
    let mut out = frame.clone();
    for y in 0..CELL {
        for x in 0..CELL {
            let p = frame.get(x, y);
            if p.a == 0 {
                continue;
            }
            let mix = |a: u8, b: u8| ((u16::from(a) + u16::from(b) * 2) / 3) as u8;
            out.set(
                x,
                y,
                Rgba::new(mix(p.r, tint.r), mix(p.g, tint.g), mix(p.b, tint.b), p.a),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::town::map::PlaceId;

    #[test]
    fn a_room_fits_the_window_and_the_campus_scrolls_inside_itself() {
        let office = PlaceId::Office.place();
        assert_eq!(office.width * TILE, VIEW.0);
        let campus = PlaceId::Campus.place();
        for x in 0..campus.width {
            for y in 0..campus.height {
                let (cx, cy) = camera(&campus, (x as f32, y as f32));
                assert!(cx >= 0 && cy >= 0);
                assert!(cx + VIEW.0 <= campus.width * TILE);
                assert!(cy + VIEW.1 <= campus.height * TILE);
            }
        }
    }

    #[test]
    fn every_place_has_a_stand_in_the_size_of_its_tiles() {
        for id in PlaceId::ALL {
            let place = id.place();
            let picture = stand_in(&place);
            assert_eq!(picture.width() as i32, place.width * TILE);
            assert!(picture.pixels().iter().all(|p| p.a == 255));
        }
    }
}
