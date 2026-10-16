//! Millpond: a meadow by the mill. What stands in the way on this board is the pond itself, so
//! it is drawn flat into the ground and hides nobody; the gate is a footbridge over its channel,
//! lowered and raised by a winch on the near bank, and the beacon stands on a jetty of boards.
//! A watermill turns at the back, a willow hangs over the far corner, and ducks paddle the
//! channel.

use super::*;

/// The meadow's grass: a touch warmer and longer than the Backyard's mown lawn.
const MEADOW: Rgba = rgb(0x93c477);
const MEADOW_DARK: Rgba = rgb(0x86b96c);
/// A buttercup's yellow.
const BUTTERCUP: Rgba = rgb(0xf5d25e);
/// The mill's whitewash, and its slate roof.
const LIME: Ramp = Ramp::new(0x9a9080, 0xc8bea8, 0xe6dcc6, 0xf2ead8, 0xfcf8ee);
const SLATE: Ramp = Ramp::new(0x2e3440, 0x444c5a, 0x5c6676, 0x76808e, 0x96a0ac);
/// The willow's long, pale leaves.
const WILLOW: Ramp = Ramp::new(0x3a5a2a, 0x557a38, 0x739a48, 0x94b85e, 0xb8d47e);

/// One pixel of the meadow on the board: unmown, so a check rather than stripes, with broad
/// patches and speckle, and the seam round every tile.
pub(super) fn meadow(px: i32, py: i32, fx: f32, fy: f32, tile: Tile) -> Rgba {
    let base = if (tile.x + tile.y) % 2 == 0 {
        MEADOW
    } else {
        MEADOW_DARK
    };
    let p = patches(px, py, (24, 13), 173);
    let mut c = if p > 0.6 {
        paint::mix(base, GRASS.shine, (p - 0.6) * 0.8)
    } else if p < 0.35 {
        paint::mix(base, GRASS.base, (0.35 - p) * 0.9)
    } else {
        base
    };
    if paint::chance(px, py, 175, 20) {
        c = paint::mix(c, GRASS.shine, 0.5);
    } else if paint::chance(px, py, 177, 18) {
        c = paint::mix(c, GRASS.base, 0.6);
    }
    let (ux, uy) = (fx - fx.floor(), fy - fy.floor());
    if ux < 0.035 || uy < 0.035 {
        c = paint::mix(c, GRASS.shadow, 0.55);
    } else if ux > 0.965 || uy > 0.965 {
        c = paint::mix(c, GRASS.shine, 0.25);
    }
    c
}

/// One pixel of the long grass round the board.
pub(super) fn long_grass(px: i32, py: i32) -> Rgba {
    let p = patches(px, py, (14, 8), 179);
    let mut c = paint::mix(GRASS.base, GRASS.light, 0.2 + p * 0.6);
    if paint::chance(px, py, 181, 40) {
        c = GRASS.shadow;
    } else if paint::chance(px, py, 183, 30) {
        c = GRASS.shine;
    } else if paint::chance(px, py, 185, 3) {
        c = BUTTERCUP;
    }
    c
}

/// Something small in a tile's corner: a daisy, a buttercup, or clover.
pub(super) fn corner(canvas: &mut Canvas, x: i32, y: i32, m: u32) {
    match m % 4 {
        0 => daisy(canvas, x, y),
        1 => {
            paint::put(canvas, x, y, BUTTERCUP);
            paint::put(canvas, x + 1, y, paint::darker(BUTTERCUP, 0.2));
            paint::put(canvas, x, y + 1, LEAF.base);
        }
        _ => clover(canvas, x, y),
    }
}

/// Along the two near edges: a wet margin of reeds and bulrushes, with flag irises.
pub(super) fn beds(canvas: &mut Canvas, view: &View, w: f32, d: f32, homes: &mut Vec<(i32, i32)>) {
    let n = NEAR_BORDER;
    let strips = [
        (w + 0.05, -FAR_BORDER + 0.5, w + n - 0.05, d + n - 0.05),
        (-FAR_BORDER + 0.5, d + 0.05, w + n - 0.05, d + n - 0.05),
    ];
    for r in strips {
        paint::fill_polygon(&quad(view, r, 0.0), |px, py| {
            let c = if paint::chance(px, py, 19, 50) {
                GRASS.shadow
            } else if paint::chance(px, py, 23, 30) {
                SOIL.shadow
            } else {
                paint::mix(GRASS.base, SOIL.base, 0.35)
            };
            paint::put(canvas, px, py, c);
        });
    }
    let mut k = 0u32;
    let mut place = |canvas: &mut Canvas, x: f32, y: f32| {
        let m = paint::noise(k as i32, 11, 97);
        let (sx, sy) = view.pixel(x, y);
        match m % 3 {
            0 => {
                // A bulrush: a tall stem, its brown head, and a leaf.
                paint::vline(canvas, sx, sy - 6, 6, LEAF.base);
                paint::rect(canvas, sx, sy - 9, 1, 3, rgb(0x6a4a2a));
                paint::put(canvas, sx, sy - 10, LEAF.light);
                paint::put(canvas, sx + 1, sy - 3, LEAF.light);
                homes.push((sx, sy - 10));
            }
            1 => {
                // A flag iris.
                paint::vline(canvas, sx, sy - 4, 4, LEAF.base);
                paint::put(canvas, sx - 1, sy - 2, LEAF.light);
                paint::put(canvas, sx, sy - 5, BUTTERCUP);
                paint::put(canvas, sx - 1, sy - 5, BUTTERCUP);
                paint::put(canvas, sx + 1, sy - 5, paint::darker(BUTTERCUP, 0.25));
                homes.push((sx, sy - 5));
            }
            _ => {
                for (dx, h) in [(-1, 3), (0, 5), (1, 4)] {
                    paint::vline(canvas, sx + dx, sy - h, h, LEAF.light);
                    paint::put(canvas, sx + dx, sy, LEAF.shadow);
                }
            }
        }
        k += 1;
    };
    let mut t = -FAR_BORDER + 0.7;
    while t < d + n - 0.2 {
        place(canvas, w + 0.2, t);
        t += 0.24;
    }
    let mut t = -FAR_BORDER + 0.7;
    while t < w + n - 0.2 {
        place(canvas, t, d + 0.2);
        t += 0.24;
    }
}

/// The pond, over every tile under water: a muddy bank wherever it meets the meadow, shallows
/// along the bank, deeper water in the middle with rows of ripples, lily pads, and reeds.
pub(super) fn water(canvas: &mut Canvas, arena: &Arena, view: &View, water: &[Tile]) {
    let wet = |t: Tile| water.contains(&t);
    for &tile in water {
        let land = |dx: i8, dy: i8| !wet(Tile::new(tile.x + dx, tile.y + dy));
        let (west, north, east, south) = (land(-1, 0), land(0, -1), land(1, 0), land(0, 1));
        let _ = arena;
        paint::fill_polygon(&view.diamond(tile, 0.0), |px, py| {
            let (fx, fy) = view.floor_at(px as f32 + 0.5, py as f32 + 0.5);
            let (ux, uy) = (fx - fx.floor(), fy - fy.floor());
            let mut bank = [(west, ux), (north, uy), (east, 1.0 - ux), (south, 1.0 - uy)]
                .iter()
                .filter(|(edge, _)| *edge)
                .map(|(_, at)| *at)
                .fold(1.0f32, f32::min);
            // Where the bank turns a corner it curves, so the pond is never a square.
            let round = 0.32;
            for (a, b, du, dv) in [
                (west, north, ux, uy),
                (east, north, 1.0 - ux, uy),
                (east, south, 1.0 - ux, 1.0 - uy),
                (west, south, ux, 1.0 - uy),
            ] {
                if a && b && du < round && dv < round {
                    let away = ((round - du).powi(2) + (round - dv).powi(2)).sqrt();
                    bank = bank.min((round - away).max(0.0));
                }
            }
            if bank <= 0.0 {
                // Beyond the curve, the meadow carries on.
                let tile_shade = if (tile.x + tile.y) % 2 == 0 {
                    MEADOW
                } else {
                    MEADOW_DARK
                };
                paint::put(canvas, px, py, paint::mix(tile_shade, GRASS.base, 0.15));
                return;
            }
            let c = if bank < 0.06 {
                if paint::chance(px, py, 3, 60) {
                    GRASS.base
                } else {
                    SOIL.base
                }
            } else if bank < 0.1 {
                SOIL.light
            } else if bank < 0.18 {
                WATER.light
            } else if bank > 0.36 {
                WATER.shadow
            } else {
                WATER.base
            };
            // Ripples in rows, broken.
            let c = if bank >= 0.18 && py % 4 == 0 && paint::chance(px / 2, py, 7, 90) {
                WATER.light
            } else {
                c
            };
            paint::put(canvas, px, py, c);
        });
        // Lily pads, and now and then a flower on one.
        let n = paint::noise(i32::from(tile.x), i32::from(tile.y), 187);
        if !n.is_multiple_of(3) {
            for j in 0..(1 + n % 2) {
                let m = paint::noise(i32::from(tile.x), j as i32, 189 + n % 7);
                let (sx, sy) = view.pixel(
                    f32::from(tile.x) + 0.3 + (m % 40) as f32 / 100.0,
                    f32::from(tile.y) + 0.3 + ((m >> 6) % 40) as f32 / 100.0,
                );
                paint::ellipse(canvas, sx, sy, 3, 1, LEAF.base);
                paint::hline(canvas, sx - 2, sy - 1, 3, LEAF.light);
                paint::put(canvas, sx + 1, sy, WATER.base);
                if m.is_multiple_of(3) {
                    paint::put(canvas, sx - 1, sy - 1, rgb(0xf4b8c8));
                    paint::put(canvas, sx, sy - 2, rgb(0xfbe0e8));
                }
            }
        }
        // Reeds along the banks that face the viewer.
        if south || east {
            for j in 0..3 {
                let m = paint::noise(i32::from(tile.y), j, 191 + tile.x as u32);
                let (u, v) = if south && (j % 2 == 0 || !east) {
                    (0.2 + (m % 60) as f32 / 100.0, 0.86)
                } else {
                    (0.86, 0.2 + (m % 60) as f32 / 100.0)
                };
                let (sx, sy) = view.pixel(f32::from(tile.x) + u, f32::from(tile.y) + v);
                for (dx, h) in [(-1, 3), (0, 5), (1, 4)] {
                    paint::vline(canvas, sx + dx, sy - h, h, LEAF.light);
                    paint::put(canvas, sx + dx, sy - h, LEAF.shine);
                }
            }
        }
    }
}

/// One tile of the jetty: boards running across the board, nailed at their ends, darker at the
/// edges that stand over the water.
pub(super) fn deck(canvas: &mut Canvas, arena: &Arena, view: &View, tile: Tile, water: &[Tile]) {
    let open = |dx: i8, dy: i8| !arena.in_zone(Tile::new(tile.x + dx, tile.y + dy));
    let (west, north, east, south) = (open(-1, 0), open(0, -1), open(1, 0), open(0, 1));
    let span = f32::from(arena.width) + f32::from(arena.height);
    let _ = water;
    paint::fill_polygon(&view.diamond(tile, 0.0), |px, py| {
        let (fx, fy) = view.floor_at(px as f32 + 0.5, py as f32 + 0.5);
        let (ux, uy) = (fx - fx.floor(), fy - fy.floor());
        let board = (uy * 5.0).floor() as i32;
        let along = (uy * 5.0).fract();
        let n = paint::noise(i32::from(tile.x) * 7 + board, i32::from(tile.y), 193);
        let mut c = if along < 0.12 {
            PLANK.shadow
        } else if along > 0.85 {
            PLANK.light
        } else {
            tone(
                PLANK,
                2 + i32::from(n.is_multiple_of(4)) - i32::from(n.is_multiple_of(5)),
            )
        };
        if paint::chance(px, py, 195, 16) {
            c = paint::mix(c, PLANK.shadow, 0.4);
        }
        // Nails at each board's ends.
        if (ux - 0.08).abs() < 0.03 && (along - 0.5).abs() < 0.15 {
            c = IRON.light;
        }
        let edge = [(west, ux), (north, uy), (east, 1.0 - ux), (south, 1.0 - uy)]
            .iter()
            .filter(|(e, _)| *e)
            .map(|(_, at)| *at)
            .fold(1.0f32, f32::min);
        if edge < 0.05 {
            c = PLANK.edge;
        } else if edge < 0.08 {
            c = PLANK.shadow;
        }
        paint::put(canvas, px, py, lit(c, PLANK, ((fy - fx) / span) * 0.16));
    });
    // A mooring post at each of the jetty's outer corners.
    let (x, y) = (f32::from(tile.x), f32::from(tile.y));
    for (cu, cv, show) in [
        (0.08, 0.92, west && south),
        (0.92, 0.92, east && south),
        (0.92, 0.08, east && north),
    ] {
        if show {
            let (sx, sy) = view.pixel(x + cu, y + cv);
            paint::rect(canvas, sx - 1, sy - 5, 3, 5, PLANK.shadow);
            paint::vline(canvas, sx - 1, sy - 5, 5, PLANK.light);
            paint::hline(canvas, sx - 1, sy - 6, 3, PLANK.shine);
        }
    }
}

/// The footbridge over the channel: lowered, boards laid across it with a rail either side;
/// raised, the span stood up on its end on the far bank, on its chains.
pub(super) fn bridge(view: &View, tile: Tile, open: bool) -> Sprite {
    let mut full = Canvas::new(view.size.0, view.size.1);
    let (x, y) = (f32::from(tile.x), f32::from(tile.y));
    let post = |canvas: &mut Canvas, (px, py): (i32, i32), h: i32| {
        paint::rect(canvas, px - 1, py - h, 3, h, PLANK.base);
        paint::vline(canvas, px - 1, py - h, h, PLANK.light);
        paint::vline(canvas, px + 1, py - h, h, PLANK.shadow);
        paint::hline(canvas, px - 1, py - h - 1, 3, PLANK.shine);
    };
    if open {
        let r = (x + 0.22, y - 0.05, x + 0.78, y + 1.05);
        upright(&mut full, view, r, 2.0, |face, px, py| {
            Some(match face {
                Face::Top => {
                    let (fx, fy) = view.floor_at(px as f32 + 0.5, py as f32 + 2.5);
                    let _ = fx;
                    if (fy * 6.0).fract() < 0.18 {
                        PLANK.shadow
                    } else if paint::chance(px, py, 197, 30) {
                        PLANK.light
                    } else {
                        PLANK.base
                    }
                }
                Face::Left => PLANK.shadow,
                Face::Right => PLANK.edge,
            })
        });
        for u in [0.22, 0.78] {
            let a = view.pixel(x + u, y - 0.02);
            let b = view.pixel(x + u, y + 1.02);
            post(&mut full, a, 9);
            post(&mut full, b, 9);
            paint::line(&mut full, (a.0, a.1 - 8), (b.0, b.1 - 8), PLANK.light);
            paint::line(&mut full, (a.0, a.1 - 7), (b.0, b.1 - 7), PLANK.shadow);
        }
    } else {
        // Stood up on the far bank, its boards facing the viewer.
        let r = (x + 0.22, y + 0.02, x + 0.78, y + 0.1);
        upright(&mut full, view, r, 22.0, |face, px, py| {
            Some(match face {
                Face::Left => {
                    if px % 4 == 0 {
                        PLANK.shadow
                    } else if paint::chance(px, py, 199, 30) {
                        PLANK.light
                    } else {
                        PLANK.base
                    }
                }
                Face::Right => PLANK.shadow,
                Face::Top => PLANK.light,
            })
        });
        for u in [0.18, 0.82] {
            let p = view.pixel(x + u, y + 0.06);
            post(&mut full, p, 26);
        }
        // Its chains, up to the posts' tops.
        let top = view.pixel(x + 0.5, y + 0.06);
        for u in [0.18, 0.82] {
            let p = view.pixel(x + u, y + 0.06);
            for k in 0..6 {
                let t = k as f32 / 6.0;
                let cx = (top.0 as f32 + (p.0 - top.0) as f32 * t).round() as i32;
                let cy = (top.1 as f32 - 22.0 + (p.1 - top.1) as f32 * t - 4.0 * t).round() as i32;
                paint::put(&mut full, cx, cy, IRON.light);
            }
        }
    }
    edged(&mut full, 0.45);
    Sprite::cut(&full)
}

/// The longest straight run of water along a row, for the ducks: from its first tile's middle to
/// its last's.
pub(super) fn paddle(water: &[Tile]) -> Option<((f32, f32), (f32, f32))> {
    let mut best: Option<(Tile, Tile)> = None;
    for &start in water {
        if water.contains(&Tile::new(start.x - 1, start.y)) {
            continue;
        }
        let mut end = start;
        while water.contains(&Tile::new(end.x + 1, end.y)) {
            end = Tile::new(end.x + 1, end.y);
        }
        let length = |(a, b): (Tile, Tile)| b.x - a.x;
        if best.is_none_or(|run| length((start, end)) > length(run)) {
            best = Some((start, end));
        }
    }
    best.map(|(a, b)| {
        (
            (f32::from(a.x) + 0.35, f32::from(a.y) + 0.55),
            (f32::from(b.x) + 0.65, f32::from(b.y) + 0.55),
        )
    })
}

/// The light moving on the water.
pub(super) fn shimmer(canvas: &mut Canvas, view: &View, water: &[Tile], t: f32, still: bool) {
    let beat = if still { 0 } else { (t * 2.0).floor() as i32 };
    for (k, tile) in water.iter().enumerate() {
        for j in 0..3 {
            let n = paint::noise(k as i32 * 3 + j, beat, 201);
            if n.is_multiple_of(2) {
                continue;
            }
            let (sx, sy) = view.pixel(
                f32::from(tile.x) + 0.25 + (n % 50) as f32 / 100.0,
                f32::from(tile.y) + 0.25 + ((n >> 6) % 50) as f32 / 100.0,
            );
            paint::hline(canvas, sx, sy, 2, rgba(0xdff3f2, 200));
        }
    }
}

/// A mother duck and her duckling, paddling up and down the channel, a ripple behind them.
pub(super) fn ducks(
    canvas: &mut Canvas,
    view: &View,
    ((ax, ay), (bx, by)): ((f32, f32), (f32, f32)),
    t: f32,
    still: bool,
) {
    let length = (bx - ax).abs().max(0.1);
    let period = length * 6.0;
    let phase = if still { 0.25 } else { (t / period).fract() };
    let (s, right) = if phase < 0.5 {
        (phase * 2.0, true)
    } else {
        (2.0 - phase * 2.0, false)
    };
    for (k, lag) in [(0, 0.0f32), (1, 0.06)] {
        let s = if right {
            (s - lag).max(0.0)
        } else {
            (s + lag).min(1.0)
        };
        let (x, y) = view.pixel(ax + (bx - ax) * s, ay + (by - ay) * s);
        let f = if right { 1 } else { -1 };
        let bob = if still {
            0
        } else {
            ((t * 3.0 + k as f32).sin() * 0.6).round() as i32
        };
        let y = y + bob;
        // Its ripple, a V opening behind it.
        for r in 2..5 {
            paint::put(canvas, x - f * (r + 2), y + r / 2, rgba(0xdff3f2, 150));
            paint::put(canvas, x - f * (r + 2), y - r / 2 + 1, rgba(0xdff3f2, 90));
        }
        if k == 0 {
            paint::rect(canvas, x - 3, y - 2, 6, 3, rgb(0x8a6a4a));
            paint::hline(canvas, x - 3, y - 2, 6, rgb(0xb89a78));
            paint::rect(canvas, x + f * 2, y - 5, 2, 3, rgb(0x2e6a4a));
            paint::put(canvas, x + f * 3, y - 4, rgb(0x1e1a18));
            paint::put(canvas, x + f * 4, y - 3, rgb(0xe8a040));
            paint::put(canvas, x - f * 3, y - 3, rgb(0x5a4030));
        } else {
            paint::rect(canvas, x - 1, y - 1, 3, 2, rgb(0xe8d070));
            paint::put(canvas, x + f, y - 2, rgb(0xf0dc80));
            paint::put(canvas, x + f * 2, y - 1, rgb(0xe8a040));
        }
    }
}

/// The mill's wheel turning on the end of the mill, its paddles dipping in the race.
pub(super) fn wheel(canvas: &mut Canvas, (cx, cy): (i32, i32), t: f32, still: bool) {
    let turn = if still { 0.0 } else { t * 0.9 };
    let (rx, ry) = (7.0f32, 12.0f32);
    for k in 0..8 {
        let a = turn + k as f32 / 8.0 * TAU;
        let (ex, ey) = (a.cos() * rx, a.sin() * ry);
        paint::line(
            canvas,
            (cx, cy),
            (cx + ex.round() as i32, cy + ey.round() as i32),
            PLANK.shadow,
        );
        // A paddle at the spoke's end.
        let (px, py) = (
            cx + (ex * 1.1).round() as i32,
            cy + (ey * 1.1).round() as i32,
        );
        paint::rect(canvas, px - 1, py - 1, 2, 2, PLANK.base);
    }
    for k in 0..48 {
        let a = k as f32 / 48.0 * TAU;
        paint::put(
            canvas,
            cx + (a.cos() * rx).round() as i32,
            cy + (a.sin() * ry).round() as i32,
            PLANK.light,
        );
    }
    paint::rect(canvas, cx - 1, cy - 1, 2, 2, IRON.base);
    // Water spilling off its lowest paddles.
    if !still {
        for k in 0..3 {
            let age = (t * 1.5 + k as f32 / 3.0).fract();
            paint::put(
                canvas,
                cx + 3 + k,
                cy + 10 + (age * 5.0) as i32,
                rgba(0xdff3f2, (220.0 * (1.0 - age)) as u8),
            );
        }
    }
}

/// A dragonfly: a long thin body and two pairs of glassy wings.
pub(super) fn dragonfly(
    canvas: &mut Canvas,
    (x, y): (i32, i32),
    body: Rgba,
    edge: Rgba,
    open: bool,
) {
    paint::hline(canvas, x - 2, y, 5, body);
    paint::put(canvas, x + 2, y, edge);
    let wing = rgba(0xffffff, if open { 170 } else { 110 });
    if open {
        paint::hline(canvas, x - 1, y - 2, 2, wing);
        paint::hline(canvas, x + 1, y - 2, 2, wing);
        paint::put(canvas, x, y - 1, wing);
    } else {
        paint::put(canvas, x, y - 1, wing);
        paint::put(canvas, x + 1, y - 1, wing);
    }
}

/// The back of the meadow: a willow over the far corner, a post-and-rail fence along the back
/// left, and the watermill at the right-hand end with its race. Returns where light moves in the
/// willow, where the robin perches, and where the mill's wheel turns.
pub(super) fn back(
    canvas: &mut Canvas,
    view: &View,
    w: f32,
    d: f32,
) -> (Points, Vec<Perch>, (i32, i32)) {
    let f = FAR_BORDER;
    let mut layer = Canvas::new(canvas.width(), canvas.height());
    // The fence along the back left, and along the back right as far as the mill, both from one
    // post in the far corner (behind the willow's trunk), so the two runs meet rather than stop
    // short of each other.
    let mut perches = Vec::new();
    let mut posts = Vec::new();
    let mut t = -0.5;
    while t < d - 0.5 {
        posts.push((-0.5, t));
        t += 0.75;
    }
    let mut fence_r = vec![(-0.5, -0.5)];
    let mut t = 0.25;
    while t < w - 2.8 {
        fence_r.push((t, -0.5));
        t += 0.75;
    }
    for run in [&posts, &fence_r] {
        for pair in run.windows(2) {
            let (a, b) = (
                view.pixel(pair[0].0, pair[0].1),
                view.pixel(pair[1].0, pair[1].1),
            );
            for lift in [5, 10] {
                paint::line(&mut layer, (a.0, a.1 - lift), (b.0, b.1 - lift), PLANK.base);
                paint::line(
                    &mut layer,
                    (a.0, a.1 - lift - 1),
                    (b.0, b.1 - lift - 1),
                    PLANK.light,
                );
            }
        }
        for (k, &(x, y)) in run.iter().enumerate() {
            // The corner post is the left run's; the right run only takes its rails from it.
            if k == 0 && std::ptr::eq(run, &fence_r) {
                continue;
            }
            let (sx, sy) = view.pixel(x, y);
            paint::ellipse(canvas, sx + 2, sy + 1, 3, 1, SHADOW);
            paint::rect(&mut layer, sx - 1, sy - 13, 3, 13, PLANK.base);
            paint::vline(&mut layer, sx - 1, sy - 13, 13, PLANK.light);
            paint::vline(&mut layer, sx + 1, sy - 13, 13, PLANK.shadow);
            paint::hline(&mut layer, sx - 1, sy - 14, 3, PLANK.shine);
            if k % 2 == 1 {
                perches.push((sx, sy - 15));
            }
        }
    }
    // The willow over the far corner: a crown, and long strands hanging from it.
    let (tx, ty) = view.pixel(-0.42, -0.42);
    dappled(canvas, (tx + 14, ty + 6), 26, 10);
    let mut tree = Canvas::new(canvas.width(), canvas.height());
    let leaves = oak(&mut tree, (tx, ty + 2), &OAK_CROWN, 631);
    // Wash the crown over to the willow's paler green, and hang its strands.
    let crown = tree.clone();
    for py in 0..crown.height() as i32 {
        for px in 0..crown.width() as i32 {
            let c = crown.get(px, py);
            if c.a > 0 && c.g > c.r {
                tree.set(px, py, paint::mix(c, WILLOW.light, 0.35));
            }
        }
    }
    for k in 0..16 {
        let x = tx - 22 + k * 3;
        let top = ty - 30 - ((k - 8).abs() * 2);
        let length = 20 + (paint::noise(k, 0, 633) % 12) as i32;
        for j in 0..length {
            let sway = (j * j) / 120;
            let c = match j % 4 {
                0 => WILLOW.shine,
                1 | 2 => WILLOW.light,
                _ => WILLOW.shadow,
            };
            paint::put(&mut tree, x + sway, top + j, c);
            if j % 5 == 2 {
                paint::put(&mut tree, x + sway + 1, top + j, WILLOW.base);
            }
        }
    }
    edged(&mut tree, 0.45);
    paint::blit(&mut layer, &tree, 0, 0);
    // The mill.
    let mill = (w - 2.5, -f + 0.05, w + NEAR_BORDER - 0.15, -0.15);
    cast(canvas, view, mill, 30.0);
    // Its race, a strip of water along its foot.
    let race = quad(
        view,
        (mill.0 - 0.1, mill.3 + 0.02, mill.2 + 0.1, mill.3 + 0.14),
        0.0,
    );
    paint::fill_polygon(&race, |px, py| {
        let c = if py % 3 == 0 { WATER.light } else { WATER.base };
        paint::put(canvas, px, py, c);
    });
    let mut house = Canvas::new(canvas.width(), canvas.height());
    let wall = 26.0;
    upright(&mut house, view, mill, wall, |face, px, py| {
        let low = quad(view, mill, 0.0);
        let (a, b) = match face {
            Face::Left => (low[3], low[2]),
            Face::Right => (low[2], low[1]),
            Face::Top => return None,
        };
        let foot = a.1 + (px as f32 + 0.5 - a.0) * (b.1 - a.1) / (b.0 - a.0);
        let up = foot - py as f32;
        let level = if face == Face::Left { 2 } else { 1 };
        // Stone to a third of its height, whitewash above.
        Some(if up < 9.0 {
            let row = (up / 3.0).floor() as i32;
            let shift = if row % 2 == 0 { 0 } else { 3 };
            if (up % 3.0) < 1.0 || (px + shift) % 6 == 0 {
                tone(MORTAR, level)
            } else {
                tone(
                    STONE,
                    level + i32::from(paint::noise((px + shift) / 6, row, 637).is_multiple_of(4)),
                )
            }
        } else {
            tone(LIME, level + 1)
        })
    });
    // Timbers across the whitewash, and windows.
    let front = |u: f32, lift: f32| lifted(view, mill.0 + u, mill.3, lift);
    for u in [0.0, 0.9, 1.8, 2.6] {
        let (a, b) = (front(u, 9.0), front(u, wall));
        paint::line(
            &mut house,
            (a.0.round() as i32, a.1.round() as i32),
            (b.0.round() as i32, b.1.round() as i32),
            PLANK.shadow,
        );
    }
    let beam = (front(0.0, 17.0), front(2.6, 17.0));
    paint::line(
        &mut house,
        (beam.0.0.round() as i32, beam.0.1.round() as i32),
        (beam.1.0.round() as i32, beam.1.1.round() as i32),
        PLANK.shadow,
    );
    for u in [0.35, 1.3, 2.15] {
        let pane = [
            front(u, 23.0),
            front(u + 0.3, 23.0),
            front(u + 0.3, 19.0),
            front(u, 19.0),
        ];
        paint::fill_polygon(&pane, |px, py| {
            let c = if (px + py) % 5 == 0 {
                rgb(0xe9f6f7)
            } else {
                rgb(0x8cb8c8)
            };
            paint::put(&mut house, px, py, c);
        });
    }
    let door = [
        front(1.0, 15.0),
        front(1.4, 15.0),
        front(1.4, 0.0),
        front(1.0, 0.0),
    ];
    paint::polygon(&mut house, &door, PAINT.base);
    // The slate roof, and its gable.
    let ridge_y = (mill.1 + mill.3) / 2.0;
    let gable = [
        lifted(view, mill.2, mill.3, wall),
        lifted(view, mill.2, mill.1, wall),
        lifted(view, mill.2, ridge_y, wall + 13.0),
    ];
    paint::polygon(&mut house, &gable, LIME.shadow);
    let o = 0.12;
    let roof = [
        lifted(view, mill.0 - o, ridge_y, wall + 13.0),
        lifted(view, mill.2 + o, ridge_y, wall + 13.0),
        lifted(view, mill.2 + o, mill.3 + o, wall - 3.0),
        lifted(view, mill.0 - o, mill.3 + o, wall - 3.0),
    ];
    let top = roof[0].1.min(roof[1].1);
    paint::fill_polygon(&roof, |px, py| {
        let row = ((py as f32 - top) / 3.0).floor() as i32;
        let shift = if row % 2 == 0 { 0 } else { 2 };
        let seam = (py as f32 - top).rem_euclid(3.0) < 1.0 || (px + shift).rem_euclid(5) == 0;
        let n = paint::noise((px + shift) / 5, row, 641);
        let c = if seam {
            SLATE.shadow
        } else if n.is_multiple_of(5) {
            SLATE.light
        } else {
            SLATE.base
        };
        paint::put(&mut house, px, py, c);
    });
    paint::line(
        &mut house,
        (roof[0].0.round() as i32, roof[0].1.round() as i32),
        (roof[1].0.round() as i32, roof[1].1.round() as i32),
        SLATE.shine,
    );
    edged(&mut house, 0.45);
    paint::blit(&mut layer, &house, 0, 0);
    paint::blit(canvas, &layer, 0, 0);
    // The wheel turns on the mill's right-hand end, over the race.
    let hub = lifted(view, mill.2 + 0.06, (mill.1 + mill.3) / 2.0 + 0.12, 13.0);
    (
        leaves,
        perches,
        (hub.0.round() as i32, hub.1.round() as i32),
    )
}
