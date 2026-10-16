//! Sandy Cove: a beach at low tide. The board is dry sand rippled by the wind, the beacon burns
//! halfway along a bar of wet sand, and sea-worn rocks with barnacles stand where nobody can
//! walk. The sea washes along the back left, a lighthouse stands on its rocks in the far corner,
//! and a row of beach huts lines the back right. Gulls wheel over it all and a crab scuttles
//! along the dunes.

use super::*;

const SAND: Ramp = Ramp::new(0x9a7a48, 0xc8a870, 0xe2c896, 0xeedcb0, 0xf8eed2);
const WET: Ramp = Ramp::new(0x6e5634, 0x8e7248, 0xa88c60, 0xc0a678, 0xdcc89c);
const MARRAM: Ramp = Ramp::new(0x5a6a3a, 0x7a8a4a, 0x9aaa5e, 0xb8c47a, 0xd6dc9e);
const SEA: Ramp = Ramp::new(0x2a5a78, 0x3a7898, 0x5a9ab4, 0x86bccc, 0xc8e6ea);
const KELP: Rgba = rgb(0x5a5a2a);
const HUTS: [Rgba; 3] = [rgb(0x7ab8d8), rgb(0xe8a0a8), rgb(0xf0d070)];

/// One pixel of the beach on the board: sand with wind ripples running across it, a faint check
/// so the tiles read, and the seam round every tile.
pub(super) fn sand(px: i32, py: i32, fx: f32, fy: f32, tile: Tile) -> Rgba {
    let base = if (tile.x + tile.y) % 2 == 0 {
        SAND.light
    } else {
        paint::mix(SAND.light, SAND.base, 0.45)
    };
    let p = patches(px, py, (24, 12), 211);
    let mut c = paint::mix(base, SAND.shine, (p - 0.5).max(0.0) * 0.6);
    // Ripples: short lighter and darker strokes, row by row.
    let row = (py + px / 3).rem_euclid(5);
    if row == 0 && paint::chance(px / 3, py, 213, 170) {
        c = paint::mix(c, SAND.shine, 0.5);
    } else if row == 1 && paint::chance(px / 3, py, 215, 120) {
        c = paint::mix(c, SAND.base, 0.45);
    }
    if paint::chance(px, py, 217, 10) {
        c = SAND.base;
    }
    let (ux, uy) = (fx - fx.floor(), fy - fy.floor());
    if ux < 0.035 || uy < 0.035 {
        c = paint::mix(c, SAND.shadow, 0.5);
    } else if ux > 0.965 || uy > 0.965 {
        c = paint::mix(c, SAND.shine, 0.3);
    }
    c
}

/// One pixel of the dunes round the board.
pub(super) fn dunes(px: i32, py: i32) -> Rgba {
    let p = patches(px, py, (16, 9), 219);
    let mut c = paint::mix(SAND.base, SAND.light, p * 0.7);
    if paint::chance(px, py, 221, 26) {
        c = SAND.shadow;
    } else if paint::chance(px, py, 223, 12) {
        c = MARRAM.base;
    }
    c
}

/// Something small in a tile's corner: a shell, a pebble, or a strand of kelp.
pub(super) fn corner(canvas: &mut Canvas, x: i32, y: i32, m: u32) {
    match m % 4 {
        0 => {
            paint::put(canvas, x, y, rgb(0xf8e0e0));
            paint::put(canvas, x + 1, y, rgb(0xe8b8b8));
            paint::put(canvas, x, y - 1, rgb(0xfff4f0));
        }
        1 => {
            paint::put(canvas, x, y, STONE.base);
            paint::put(canvas, x - 1, y, STONE.light);
        }
        2 => {
            for k in 0..3 {
                paint::put(canvas, x + k, y + (k % 2), KELP);
            }
        }
        _ => {
            // A starfish.
            for (dx, dy) in [(0, -1), (-1, 0), (1, 0), (-1, 1), (1, 1)] {
                paint::put(canvas, x + dx, y + dy, rgb(0xe08050));
            }
            paint::put(canvas, x, y, rgb(0xf0a070));
        }
    }
}

/// Marram grass: a clump of pale blades leaning in the wind.
pub(super) fn marram(canvas: &mut Canvas, x: i32, y: i32, k: i32) {
    let lean = 1 + (k % 2);
    for (dx, h) in [(-2, 3), (-1, 5), (0, 6), (1, 4), (2, 3)] {
        for up in 0..h {
            let px = x + dx + (up * lean) / 5;
            let c = if up == h - 1 {
                MARRAM.shine
            } else if up < 2 {
                MARRAM.shadow
            } else {
                MARRAM.light
            };
            paint::put(canvas, px, y - up, c);
        }
    }
}

/// Along the two near edges: dunes with marram grass, shells and a stranded starfish.
pub(super) fn beds(canvas: &mut Canvas, view: &View, w: f32, d: f32, homes: &mut Vec<(i32, i32)>) {
    let n = NEAR_BORDER;
    let mut k = 0i32;
    let mut place = |canvas: &mut Canvas, x: f32, y: f32| {
        let m = paint::noise(k, 13, 97);
        let (sx, sy) = view.pixel(x, y);
        if m.is_multiple_of(3) {
            corner(canvas, sx, sy, m);
        } else {
            marram(canvas, sx, sy, k);
            homes.push((sx, sy - 6));
        }
        k += 1;
    };
    let mut t = -FAR_BORDER + 0.7;
    while t < d + n - 0.2 {
        place(canvas, w + 0.2, t);
        t += 0.27;
    }
    let mut t = -FAR_BORDER + 0.7;
    while t < w + n - 0.2 {
        place(canvas, t, d + 0.2);
        t += 0.27;
    }
}

/// One tile of the sandbar: wet sand, glossy where the water still stands on it, edged with a
/// line of pebbles where it meets the dry sand.
pub(super) fn bar(canvas: &mut Canvas, arena: &Arena, view: &View, tile: Tile) {
    let open = |dx: i8, dy: i8| !arena.in_zone(Tile::new(tile.x + dx, tile.y + dy));
    let (west, north, east, south) = (open(-1, 0), open(0, -1), open(1, 0), open(0, 1));
    let span = f32::from(arena.width) + f32::from(arena.height);
    paint::fill_polygon(&view.diamond(tile, 0.0), |px, py| {
        let (fx, fy) = view.floor_at(px as f32 + 0.5, py as f32 + 0.5);
        let (ux, uy) = (fx - fx.floor(), fy - fy.floor());
        let edge = [(west, ux), (north, uy), (east, 1.0 - ux), (south, 1.0 - uy)]
            .iter()
            .filter(|(e, _)| *e)
            .map(|(_, at)| *at)
            .fold(1.0f32, f32::min);
        let p = patches(px, py, (12, 7), 225);
        let mut c = paint::mix(WET.base, WET.light, p * 0.6);
        // Water still standing in hollows, catching the sky.
        if p > 0.72 {
            c = paint::mix(SEA.light, SEA.shine, (p - 0.72) * 2.0);
        }
        if paint::chance(px, py, 227, 12) {
            c = WET.shine;
        }
        if edge < 0.06 {
            c = if paint::chance(px, py, 229, 120) {
                STONE.light
            } else {
                STONE.base
            };
        } else if edge < 0.09 {
            c = WET.shadow;
        }
        if ux < 0.03 || uy < 0.03 {
            c = paint::mix(c, WET.shadow, 0.5);
        }
        paint::put(canvas, px, py, lit(c, WET, ((fy - fx) / span) * 0.16));
    });
}

/// A sea-worn rock on a board tile: rounded, darker and wet at its foot, with barnacles and a
/// fringe of kelp.
pub(super) fn rock(view: &View, tile: Tile) -> Sprite {
    let mut full = Canvas::new(view.size.0, view.size.1);
    let (cx, cy) = view.centre(tile);
    let salt = paint::noise(i32::from(tile.x), i32::from(tile.y), 231);
    let (rx, ry) = (14.0f32, 11.0f32);
    let centre = (cx as f32, cy as f32 - 8.0);
    for py in (cy - 20)..=(cy + 2) {
        for px in (cx - 16)..=(cx + 16) {
            let u = (px as f32 + 0.5 - centre.0) / rx;
            let v = (py as f32 + 0.5 - centre.1) / ry;
            let lump = (paint::noise(px / 4, py / 3, salt) % 100) as f32 / 700.0;
            let d = u.abs().powf(2.2) + v.abs().powf(2.2);
            if d > 1.0 - lump || py > cy + 1 {
                continue;
            }
            let lit = -0.6 * u - 0.8 * v;
            let level = if lit > 0.45 {
                4
            } else if lit > 0.05 {
                3
            } else if lit > -0.4 {
                2
            } else {
                1
            };
            let mut c = tone(STONE, level);
            // Wet and darker towards its foot.
            if v > 0.45 {
                c = paint::mix(c, rgb(0x3a3a42), 0.35);
            }
            if v > 0.55 && paint::noise(px, py / 2, salt + 1).is_multiple_of(3) {
                c = KELP;
            }
            if v > -0.2 && v < 0.5 && paint::noise(px, py, salt + 2).is_multiple_of(23) {
                c = rgb(0xf0ece0);
            }
            paint::put(&mut full, px, py, c);
        }
    }
    edged(&mut full, 0.45);
    Sprite::cut(&full)
}

/// The back of the cove: the sea along the back left with the lighthouse on its rocks in the far
/// corner, and three beach huts along the back right. Returns where the light flashes, where
/// gulls perch, and the sea's strip for its waves.
pub(super) fn back(
    canvas: &mut Canvas,
    view: &View,
    w: f32,
    d: f32,
) -> (Points, Vec<Perch>, [(f32, f32); 2]) {
    let f = FAR_BORDER;
    // The sea, along the back left and round the far corner.
    let sea = quad(view, (-f, -f, -0.22, d + NEAR_BORDER), 0.0);
    paint::fill_polygon(&sea, |px, py| {
        let (fx, _) = view.floor_at(px as f32 + 0.5, py as f32 + 0.5);
        // Shallower towards the beach.
        let depth = ((-0.22 - fx) / (f - 0.22)).clamp(0.0, 1.0);
        let mut c = if depth < 0.18 {
            SEA.light
        } else if depth < 0.55 {
            SEA.base
        } else {
            SEA.shadow
        };
        if py % 4 == 0 && paint::chance(px / 2, py, 233, 70) {
            c = SEA.light;
        }
        paint::put(canvas, px, py, c);
    });
    // Wet sand at the water's edge.
    let shore = quad(view, (-0.22, -f, -0.08, d + NEAR_BORDER), 0.0);
    paint::fill_polygon(&shore, |px, py| {
        let c = if paint::chance(px, py, 235, 50) {
            WET.light
        } else {
            WET.base
        };
        paint::put(canvas, px, py, c);
    });
    let mut layer = Canvas::new(canvas.width(), canvas.height());
    // The lighthouse, on rocks in the far corner.
    let (lx, ly) = view.pixel(-0.5, -0.5);
    for (dx, dy, r) in [(-8, 2, 6), (7, 3, 5), (0, 4, 7), (-3, -1, 5)] {
        for oy in -r..=r / 2 {
            for ox in -r - 2..=r + 2 {
                if ox * ox * 2 + oy * oy * 5 > r * r * 3 {
                    continue;
                }
                let level = if ox + oy < -2 {
                    3
                } else if ox + oy < 3 {
                    2
                } else {
                    1
                };
                paint::put(&mut layer, lx + dx + ox, ly + dy + oy, tone(STONE, level));
            }
        }
    }
    let tall = 52;
    for up in 0..tall {
        let half = 7 - up * 3 / tall;
        let band = (up / 9) % 2 == 0;
        for dx in -half..=half {
            let side = dx as f32 / half as f32;
            let level = if side < -0.35 {
                3
            } else if side < 0.3 {
                2
            } else {
                1
            };
            let ramp = if band {
                Ramp::new(0x7a2a22, 0xa83a2e, 0xc8503e, 0xe07058, 0xf09a80)
            } else {
                Ramp::new(0x9a9488, 0xc8c2b4, 0xeae4d6, 0xf6f2e8, 0xffffff)
            };
            paint::put(&mut layer, lx + dx, ly - up, tone(ramp, level));
        }
    }
    // The gallery and the lamp room.
    let top = ly - tall;
    paint::rect(&mut layer, lx - 7, top - 1, 15, 2, IRON.base);
    paint::hline(&mut layer, lx - 7, top - 1, 15, IRON.light);
    paint::rect(&mut layer, lx - 4, top - 8, 9, 7, rgb(0xfff1c4));
    paint::vline(&mut layer, lx - 4, top - 8, 7, IRON.base);
    paint::vline(&mut layer, lx + 4, top - 8, 7, IRON.base);
    paint::vline(&mut layer, lx, top - 8, 7, IRON.light);
    for k in 0..5 {
        paint::hline(
            &mut layer,
            lx - 5 + k,
            top - 9 - k,
            11 - k * 2,
            if k == 0 { IRON.light } else { IRON.base },
        );
    }
    paint::put(&mut layer, lx, top - 14, IRON.shine);
    let lamp = vec![(lx, top - 5)];
    // Beach huts along the back right, in three colours.
    let mut perches = Vec::new();
    for (k, x) in [w - 4.6, w - 3.0, w - 1.4].iter().enumerate() {
        let r = (*x, -f + 0.08, x + 1.0, -0.2);
        cast(canvas, view, r, 24.0);
        hut(&mut layer, view, r, HUTS[k % 3]);
        let ridge = lifted(view, x + 0.5, (r.1 + r.3) / 2.0, 30.0);
        perches.push((ridge.0.round() as i32, ridge.1.round() as i32 - 1));
    }
    // A rowing boat pulled up on the sand by the left corner.
    let (bx, by) = view.pixel(-0.55, d - 0.5);
    paint::ellipse(canvas, bx + 2, by + 1, 9, 3, SHADOW);
    for dx in -9i32..=9 {
        let depth = 4 - (dx.abs() * dx.abs()) / 24;
        for up in 0..depth.max(1) {
            let c = if up == depth - 1 {
                PLANK.light
            } else {
                rgb(0x4a7a9a)
            };
            paint::put(&mut layer, bx + dx, by - up, c);
        }
        paint::put(&mut layer, bx + dx, by - depth.max(1), PLANK.shine);
    }
    paint::hline(&mut layer, bx - 6, by - 3, 13, PLANK.shadow);
    edged(&mut layer, 0.45);
    paint::blit(canvas, &layer, 0, 0);
    (lamp, perches, [(-f, -f), (-0.22, d + NEAR_BORDER)])
}

/// A beach hut: boarded walls in one bright colour under a white gable roof, its door towards the
/// board.
fn hut(canvas: &mut Canvas, view: &View, r: (f32, f32, f32, f32), paint_colour: Rgba) {
    let mut full = Canvas::new(canvas.width(), canvas.height());
    let wall = 20.0;
    upright(&mut full, view, r, wall, |face, px, _| {
        let shade = match face {
            Face::Left => 0.0,
            Face::Right => 0.3,
            Face::Top => return None,
        };
        let board = if px % 3 == 0 { 0.18 } else { 0.0 };
        Some(paint::mix(paint_colour, rgb(0x2a2a3a), shade + board))
    });
    let (x0, y0, x1, y1) = r;
    let ridge_y = (y0 + y1) / 2.0;
    let gable = [
        lifted(view, x1, y1, wall),
        lifted(view, x1, y0, wall),
        lifted(view, x1, ridge_y, wall + 9.0),
    ];
    paint::polygon(
        &mut full,
        &gable,
        paint::mix(paint_colour, rgb(0x2a2a3a), 0.35),
    );
    let o = 0.08;
    let roof = [
        lifted(view, x0 - o, ridge_y, wall + 9.0),
        lifted(view, x1 + o, ridge_y, wall + 9.0),
        lifted(view, x1 + o, y1 + o, wall - 2.0),
        lifted(view, x0 - o, y1 + o, wall - 2.0),
    ];
    paint::fill_polygon(&roof, |px, py| {
        let c = if (px + py) % 4 == 0 {
            rgb(0xd8d4cc)
        } else {
            rgb(0xf4f0e8)
        };
        paint::put(&mut full, px, py, c);
    });
    let door = [
        lifted(view, x0 + 0.32, y1, 15.0),
        lifted(view, x0 + 0.68, y1, 15.0),
        lifted(view, x0 + 0.68, y1, 0.0),
        lifted(view, x0 + 0.32, y1, 0.0),
    ];
    paint::polygon(&mut full, &door, rgb(0xf4f0e8));
    let knob = lifted(view, x0 + 0.62, y1, 7.0);
    paint::put(
        &mut full,
        knob.0.round() as i32,
        knob.1.round() as i32,
        BRASS.base,
    );
    edged(&mut full, 0.4);
    paint::blit(canvas, &full, 0, 0);
}

/// The sea's waves: lines of foam rolling in to the shore and drawing back.
pub(super) fn waves(canvas: &mut Canvas, view: &View, sea: [(f32, f32); 2], t: f32, still: bool) {
    let [(x0, y0), (x1, y1)] = sea;
    for k in 0..3 {
        let age = if still {
            k as f32 / 3.0
        } else {
            (t * 0.25 + k as f32 / 3.0).fract()
        };
        let x = x0 + (x1 - x0) * (0.25 + age * 0.7);
        let alpha = (220.0 * (1.0 - age).min(age * 4.0).min(1.0)) as u8;
        let mut y = y0 + 0.1;
        while y < y1 - 0.1 {
            let n = paint::noise((y * 10.0) as i32, k, 237);
            if !n.is_multiple_of(3) {
                let (sx, sy) = view.pixel(x + ((n % 7) as f32 - 3.0) * 0.01, y);
                paint::hline(canvas, sx, sy, 3, paint::faded(SEA.shine, alpha));
            }
            y += 0.12;
        }
    }
}

/// Gulls gliding over the back of the cove, wings rising and falling.
pub(super) fn gulls(canvas: &mut Canvas, view: &View, t: f32, still: bool) {
    let (cx, cy) = view.pixel(-0.2, 1.5);
    for k in 0..2 {
        let s = if still {
            k as f32 * 2.0
        } else {
            t * 0.3 + k as f32 * 2.6
        };
        let x = cx + (s.cos() * 34.0) as i32 + k * 12;
        let y = cy - 54 + (s.sin() * 8.0) as i32 - k * 6;
        let up = !still && ((t * 4.0 + k as f32) as i32) % 2 == 0;
        let c = rgb(0xf8f8f4);
        let edge = rgb(0x6a6a72);
        paint::put(canvas, x, y, c);
        if up {
            paint::put(canvas, x - 1, y - 1, c);
            paint::put(canvas, x - 2, y - 2, edge);
            paint::put(canvas, x + 1, y - 1, c);
            paint::put(canvas, x + 2, y - 2, edge);
        } else {
            paint::hline(canvas, x - 3, y, 3, c);
            paint::hline(canvas, x + 1, y, 3, c);
            paint::put(canvas, x - 3, y + 1, edge);
            paint::put(canvas, x + 3, y + 1, edge);
        }
    }
}

/// A crab scuttling sideways along the near dunes, now and then stopping.
pub(super) fn crab(canvas: &mut Canvas, view: &View, w: f32, d: f32, t: f32, still: bool) {
    let span = w - 1.0;
    let phase = if still {
        0.3
    } else {
        (t / (span * 3.0)).fract()
    };
    let s = if phase < 0.5 {
        phase * 2.0
    } else {
        2.0 - phase * 2.0
    };
    let (x, y) = view.pixel(0.5 + span * s, d + 0.22);
    let step = !still && ((t * 8.0) as i32) % 2 == 0;
    let shell = rgb(0xd0603a);
    paint::rect(canvas, x - 2, y - 2, 5, 2, shell);
    paint::hline(canvas, x - 2, y - 2, 5, rgb(0xe88a5a));
    paint::put(canvas, x - 3, y - 3, shell);
    paint::put(canvas, x + 3, y - 3, shell);
    paint::put(canvas, x - 1, y - 3, rgb(0x1e1a18));
    paint::put(canvas, x + 1, y - 3, rgb(0x1e1a18));
    let legs = if step { [-3, -1, 1, 3] } else { [-2, -1, 1, 2] };
    for dx in legs {
        paint::put(canvas, x + dx, y, rgb(0xa84a2a));
    }
}
