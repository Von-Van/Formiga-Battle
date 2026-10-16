//! A Handler's pennant, painted at the notebook's own pixel to Formiga's rules: a cloth cut to its
//! shape on a garden cane, lit from the upper left, its edge a darker shade of its own colour,
//! with its symbol in the middle and its trim round it. The same few pixels everywhere it flies:
//! the Handler page, the station's masthead, the match's top bar and the results.

use crate::identity::Style;
use formiga_art::paint::{self, Ramp, rgb, rgba};
use formiga_art::{Canvas, Rgba};

/// The cloth's size, in its pixels.
pub const CLOTH_W: i32 = 23;
pub const CLOTH_H: i32 = 15;
/// The whole picture: the cloth with its cane at the left.
pub const WIDTH: u32 = CLOTH_W as u32 + 3;
pub const HEIGHT: u32 = CLOTH_H as u32 + 6;

const CANE: Ramp = Ramp::new(0x4a3420, 0x6e5034, 0x8e6c48, 0xb08c62, 0xd0b088);
const BRASS: Ramp = Ramp::new(0x6a4a12, 0x9a7020, 0xc89a34, 0xe8c25a, 0xf8e3a0);
const THREAD_LIGHT: Rgba = rgb(0xfff3d6);
const THREAD_DARK: Rgba = rgb(0x3a2a1a);
const SHADOW: Rgba = rgba(0x000000, 50);

/// Each symbol as a 7 by 7 picture, `#` filled, by its place in `identity::SYMBOLS`.
pub const GLYPHS: [[&str; 7]; 12] = [
    // Acorn
    [
        "..#....", ".#####.", "#######", ".#...#.", ".#...#.", "..#.#..", "...#...",
    ],
    // Leaf
    [
        "....###", "..#####", ".####.#", ".###.##", "##.####", "#.####.", "#......",
    ],
    // Sun
    [
        "#..#..#", ".#...#.", "..###..", "#.###.#", "..###..", ".#...#.", "#..#..#",
    ],
    // Moon
    [
        "..###..", ".##....", "##.....", "##.....", "##.....", ".##....", "..###..",
    ],
    // Star
    [
        "...#...", "...#...", "#######", ".#####.", "..###..", ".##.##.", "##...##",
    ],
    // Shell
    [
        "..###..", ".#.#.#.", "#.#.#.#", "#.#.#.#", "#######", ".#####.", "...#...",
    ],
    // Mushroom
    [
        "..###..", ".##.##.", "#######", "#######", "..#.#..", "..#.#..", ".#####.",
    ],
    // Flower
    [
        "..#.#..", ".#####.", "##.#.##", ".#####.", "..#.#..", "...#...", "..##...",
    ],
    // Wave
    [
        ".......", "..##...", ".#..#..", "#....#.", ".......", "..##..#", ".#..##.",
    ],
    // Key
    [
        ".###...", "#...#..", "#...#..", ".###...", "..#....", "..##...", "..###..",
    ],
    // Feather
    [
        ".....##", "....###", "...###.", "..###..", ".###...", ".#.....", "#......",
    ],
    // Bell
    [
        "...#...", "..###..", ".#####.", ".#####.", ".#####.", "#######", "...#...",
    ],
];

/// Whether a pixel of the cloth, from its top left, is inside its cut.
fn inside(shape: usize, x: i32, y: i32) -> bool {
    if !(0..CLOTH_W).contains(&x) || !(0..CLOTH_H).contains(&y) {
        return false;
    }
    let mid = (CLOTH_H - 1) / 2;
    let off = (y - mid).abs();
    match shape {
        // Tapering to a point at the fly.
        0 => off * (CLOTH_W - 1) <= mid * (CLOTH_W - 1 - x),
        // Square at the hoist, with a notch cut into the fly.
        1 => x <= CLOTH_W - 1 - mid + off,
        _ => true,
    }
}

/// Where the symbol sits on the cloth: nearer the hoist on a pennon, which narrows.
fn symbol_at(shape: usize) -> (i32, i32) {
    let x = match shape {
        0 => 2,
        1 => 6,
        _ => 8,
    };
    (x, (CLOTH_H - 7) / 2)
}

/// The pennant's cloth alone, `CLOTH_W` by `CLOTH_H`, for flying from a cane drawn elsewhere.
pub fn cloth(style: Style) -> Canvas {
    let mut canvas = Canvas::new(CLOTH_W as u32, CLOTH_H as u32);
    let ramp = style.ramp();
    let shape = style.shape % crate::identity::SHAPES.len();
    let edge_at = |x: i32, y: i32, d: i32| {
        inside(shape, x, y)
            && [(d, 0), (-d, 0), (0, d), (0, -d)]
                .iter()
                .any(|(dx, dy)| !inside(shape, x + dx, y + dy))
    };
    for y in 0..CLOTH_H {
        for x in 0..CLOTH_W {
            if !inside(shape, x, y) {
                continue;
            }
            let colour = if edge_at(x, y, 1) {
                ramp.edge
            } else if !inside(shape, x, y - 2) || !inside(shape, x - 2, y) {
                ramp.light
            } else if !inside(shape, x, y + 2) {
                ramp.shadow
            } else if paint::noise(x, y, 977) % 100 < 7 {
                ramp.light
            } else {
                ramp.base
            };
            canvas.set(x, y, colour);
        }
    }
    // The trim, just inside the edge.
    let light_field = style.field % crate::identity::FIELDS.len() >= 6;
    let thread = if light_field {
        THREAD_DARK
    } else {
        THREAD_LIGHT
    };
    for y in 0..CLOTH_H {
        for x in 0..CLOTH_W {
            let ring = edge_at(x, y, 2) && !edge_at(x, y, 1);
            if !ring {
                continue;
            }
            match style.trim % crate::identity::TRIMS.len() {
                1 if (x + y) % 3 != 0 => canvas.set(x, y, paint::faded(thread, 200)),
                2 => canvas.set(
                    x,
                    y,
                    if y < CLOTH_H / 2 {
                        BRASS.light
                    } else {
                        BRASS.base
                    },
                ),
                _ => {}
            }
        }
    }
    // The symbol, in the colour that reads best on the cloth, with a darker shade under it.
    let ink = if light_field { ramp.edge } else { THREAD_LIGHT };
    let shade = if light_field { ramp.light } else { ramp.edge };
    let (sx, sy) = symbol_at(shape);
    let glyph = &GLYPHS[style.symbol % GLYPHS.len()];
    for (gy, row) in glyph.iter().enumerate() {
        for (gx, c) in row.bytes().enumerate() {
            if c != b'#' {
                continue;
            }
            let (x, y) = (sx + gx as i32, sy + gy as i32);
            if inside(shape, x + 1, y + 1) && !edge_at(x + 1, y + 1, 1) {
                canvas.set(x + 1, y + 1, shade);
            }
        }
    }
    for (gy, row) in glyph.iter().enumerate() {
        for (gx, c) in row.bytes().enumerate() {
            let (x, y) = (sx + gx as i32, sy + gy as i32);
            if c == b'#' && inside(shape, x, y) {
                canvas.set(x, y, ink);
            }
        }
    }
    canvas
}

/// The pennant on its cane, `WIDTH` by `HEIGHT`: what the notebook shows of a Handler or a trio.
pub fn flag(style: Style) -> Canvas {
    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    let h = HEIGHT as i32;
    paint::ellipse(&mut canvas, 2, h - 1, 2, 1, SHADOW);
    paint::vline(&mut canvas, 0, 1, h - 2, CANE.shadow);
    paint::vline(&mut canvas, 1, 1, h - 2, CANE.light);
    paint::put(&mut canvas, 0, 0, BRASS.shine);
    paint::put(&mut canvas, 1, 0, BRASS.base);
    paint::blit(&mut canvas, &cloth(style), 2, 1);
    canvas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{FIELDS, SHAPES, SYMBOLS, TRIMS};

    #[test]
    fn every_glyph_is_seven_by_seven() {
        for glyph in GLYPHS {
            assert!(glyph.iter().all(|row| row.len() == 7));
        }
        assert_eq!(GLYPHS.len(), SYMBOLS.len());
    }

    #[test]
    fn every_pennant_is_its_own_picture_and_its_symbol_shows() {
        let mut seen = std::collections::HashSet::new();
        for shape in 0..SHAPES.len() {
            for field in 0..FIELDS.len() {
                for symbol in 0..SYMBOLS.len() {
                    for trim in 0..TRIMS.len() {
                        let style = Style {
                            shape,
                            field,
                            symbol,
                            trim,
                        };
                        let picture = flag(style);
                        assert!(
                            seen.insert(picture.rgba_bytes()),
                            "{style:?} looks like another"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_pennon_tapers_and_a_swallowtail_is_notched() {
        let mid = (CLOTH_H - 1) / 2;
        assert!(inside(0, CLOTH_W - 1, mid) && !inside(0, CLOTH_W - 1, 0));
        assert!(inside(1, CLOTH_W - 1, 0) && !inside(1, CLOTH_W - 1, mid));
        assert!(inside(2, CLOTH_W - 1, 0) && inside(2, CLOTH_W - 1, mid));
    }
}
