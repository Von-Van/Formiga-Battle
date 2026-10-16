//! Found things and the map event, drawn at the board's pixel scale: a dandelion puff, a twig and
//! a leaf umbrella lying in the grass or carried, a twig wedged upright, a glint where something
//! is about to turn up, the sprinkler's warning on the ground and its spray, and a puff's gust.
//!
//! Everything follows the garden's rules: light from the upper left, edges in a darker shade of
//! their own colour, small enough that whoever stands on the tile is never lost behind it.

use crate::garden::{LEAF, PLANK};
use crate::iso::View;
use formiga_art::Canvas;
use formiga_art::paint::{self, Ramp, rgb, rgba};
use formiga_battle_core::{Item, Tile};
use std::f32::consts::TAU;

const BARK: Ramp = Ramp::new(0x3e2a1c, 0x5d3f2b, 0x7a5539, 0x98714f, 0xb68f6a);
const FLUFF: Ramp = Ramp::new(0x9a9384, 0xc9c2b0, 0xe8e2d2, 0xf6f2e6, 0xffffff);
const WATER: Ramp = Ramp::new(0x3c6f86, 0x5a94aa, 0x7fb8c8, 0xa8d6df, 0xdff3f2);

/// A found thing at a point: its bottom middle at `(x, y)`, full size or, carried, small.
pub fn icon(canvas: &mut Canvas, (x, y): (i32, i32), item: Item, small: bool) {
    match item {
        Item::Puff => puff(canvas, x, y, small),
        Item::Twig => twig(canvas, x, y, small),
        Item::Leaf => leaf(canvas, x, y, small),
    }
}

/// A dandelion clock on a short stem: a ball of seeds, lit from the upper left.
fn puff(canvas: &mut Canvas, x: i32, y: i32, small: bool) {
    let (stem, r) = if small { (3, 3) } else { (6, 4) };
    paint::vline(canvas, x, y - stem, stem, LEAF.shadow);
    paint::put(canvas, x + 1, y - 1, LEAF.base);
    let (cx, cy) = (x, y - stem - r);
    paint::ellipse(canvas, cx, cy, r, r, FLUFF.light);
    // The far side of the ball in shade, its edge a darker cream.
    for k in 0..24 {
        let a = k as f32 / 24.0 * TAU;
        let (px, py) = (
            cx + (a.cos() * r as f32).round() as i32,
            cy + (a.sin() * r as f32).round() as i32,
        );
        let lit = a.cos() * -0.7 + a.sin() * -0.7;
        paint::put(
            canvas,
            px,
            py,
            if lit > 0.3 { FLUFF.shine } else { FLUFF.base },
        );
    }
    paint::put(canvas, cx - 1, cy - 1, FLUFF.shine);
    paint::put(canvas, cx, cy, FLUFF.shadow);
    if !small {
        // A few seeds' feathery tips standing out from the ball.
        for (dx, dy) in [(-5, -1), (5, 0), (0, -5), (-3, -4), (4, -3)] {
            paint::put(canvas, cx + dx, cy + dy, FLUFF.shine);
        }
    }
}

/// A short forked stick with a leaf still on it, lying in the grass.
fn twig(canvas: &mut Canvas, x: i32, y: i32, small: bool) {
    let half = if small { 3 } else { 6 };
    let (a, b) = ((x - half, y - 1), (x + half, y - 1 - half / 2));
    paint::line(canvas, a, b, BARK.light);
    paint::line(canvas, (a.0, a.1 + 1), (b.0, b.1 + 1), BARK.shadow);
    // The fork, and a leaf on it.
    let fork = (x + 1, y - 2 - half / 4);
    paint::line(canvas, fork, (fork.0 + 1, fork.1 - half / 2), BARK.base);
    paint::put(canvas, fork.0 + 2, fork.1 - half / 2, LEAF.light);
    paint::put(canvas, fork.0 + 2, fork.1 - half / 2 - 1, LEAF.shine);
    paint::put(canvas, fork.0 + 3, fork.1 - half / 2, LEAF.base);
}

/// A broad leaf folded on its stalk: a leaf umbrella waiting to be opened.
fn leaf(canvas: &mut Canvas, x: i32, y: i32, small: bool) {
    let (rx, ry) = if small { (3, 2) } else { (6, 3) };
    let (cx, cy) = (x, y - ry - 1);
    paint::ellipse(canvas, cx, cy, rx, ry, LEAF.base);
    paint::ellipse(canvas, cx - 1, cy - 1, rx - 2, (ry - 1).max(1), LEAF.light);
    paint::put(canvas, cx - rx + 2, cy - 1, LEAF.shine);
    paint::line(
        canvas,
        (cx - rx + 1, cy + 1),
        (cx + rx, cy - 1),
        LEAF.shadow,
    );
    paint::hline(canvas, cx - rx + 1, cy + ry, rx * 2 - 1, LEAF.edge);
    paint::line(canvas, (cx + rx, cy), (cx + rx + 2, cy + 2), PLANK.shadow);
}

/// Something lying in the grass on its tile, with a soft shadow, bobbing a little as if the wind
/// were at it.
pub fn on_ground(canvas: &mut Canvas, view: &View, tile: Tile, item: Item, now: f32, still: bool) {
    let (cx, cy) = view.centre(tile);
    paint::ellipse(canvas, cx, cy, 6, 2, rgba(0x24452b, 70));
    let bob = if still {
        0
    } else {
        ((now * 2.6 + f32::from(tile.x)).sin() * 0.9).round() as i32
    };
    icon(canvas, (cx, cy + bob.min(0)), item, false);
}

/// A twig wedged upright in the ground: a stick taller than a creature's knee, forked at the top,
/// with its two last leaves.
pub fn wedged(canvas: &mut Canvas, view: &View, tile: Tile) {
    let (cx, cy) = view.centre(tile);
    paint::ellipse(canvas, cx + 3, cy, 7, 2, rgba(0x24452b, 80));
    // A little mound of turned earth at its foot.
    paint::ellipse(canvas, cx, cy, 4, 1, BARK.shadow);
    paint::vline(canvas, cx - 1, cy - 18, 18, BARK.shine);
    paint::vline(canvas, cx, cy - 20, 20, BARK.light);
    paint::vline(canvas, cx + 1, cy - 19, 19, BARK.shadow);
    paint::vline(canvas, cx + 2, cy - 16, 16, BARK.edge);
    paint::put(canvas, cx, cy - 12, BARK.edge);
    paint::line(canvas, (cx, cy - 14), (cx - 5, cy - 22), BARK.light);
    paint::line(canvas, (cx + 1, cy - 16), (cx + 5, cy - 24), BARK.base);
    for (lx, ly) in [(cx - 6, cy - 23), (cx + 5, cy - 25)] {
        paint::ellipse(canvas, lx, ly, 2, 1, LEAF.base);
        paint::put(canvas, lx - 1, ly - 1, LEAF.shine);
    }
}

/// A glint where something is about to turn up: a four-pointed twinkle over the grass, breathing.
pub fn glint(canvas: &mut Canvas, view: &View, tile: Tile, now: f32, still: bool) {
    let (cx, cy) = view.centre(tile);
    let breath = if still {
        0.6
    } else {
        0.5 + 0.5 * (now * 4.0 + f32::from(tile.y)).sin()
    };
    let reach = 2 + (breath * 3.0).round() as i32;
    let y = cy - 6;
    let gold = rgba(0xfbe6a2, (120.0 + 135.0 * breath) as u8);
    paint::hline(canvas, cx - reach, y, reach * 2 + 1, gold);
    paint::vline(canvas, cx, y - reach, reach * 2 + 1, gold);
    paint::put(canvas, cx, y, rgb(0xffffff));
    paint::put(canvas, cx - 1, y - 1, rgba(0xffffff, 140));
    paint::put(canvas, cx + 1, y + 1, rgba(0xffffff, 140));
}

/// The ground the sprinkler is about to sweep: the tile washed pale blue, breathing, and a ring
/// of droplets round it turning slowly, so both Handlers see which tiles will be wet.
pub fn warned(canvas: &mut Canvas, view: &View, tile: Tile, now: f32, still: bool) {
    let (cx, cy) = view.centre(tile);
    let breath = if still {
        0.5
    } else {
        0.5 + 0.5 * (now * 3.0).sin()
    };
    let mid = view.screen(f32::from(tile.x) + 0.5, f32::from(tile.y) + 0.5);
    let face = view
        .diamond(tile, 0.0)
        .map(|p| (p.0 + (mid.0 - p.0) * 0.1, p.1 + (mid.1 - p.1) * 0.1));
    paint::polygon(canvas, &face, rgba(0x7fb8c8, (50.0 + 50.0 * breath) as u8));
    let turn = if still { 0.0 } else { now * 0.8 };
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU + turn;
        let (px, py) = (
            cx + (a.cos() * 14.0).round() as i32,
            cy + (a.sin() * 6.0).round() as i32,
        );
        // A droplet: round below, pointed above.
        paint::put(canvas, px, py - 2, WATER.light);
        paint::rect(canvas, px - 1, py - 1, 2, 2, WATER.base);
        paint::put(canvas, px - 1, py - 1, WATER.shine);
    }
}

/// Ground a puff is about to blow across: a few wisps of wind drifting over it.
pub fn gusty(canvas: &mut Canvas, view: &View, tile: Tile, now: f32, still: bool) {
    let (cx, cy) = view.centre(tile);
    let drift = if still { 0 } else { ((now * 9.0) as i32) % 6 };
    for (k, dy) in [-3, 0, 3].into_iter().enumerate() {
        let x = cx - 8 + drift + k as i32 * 3;
        let y = cy - 4 + dy;
        paint::hline(canvas, x, y, 6, rgba(0xffffff, 150));
        paint::put(canvas, x + 6, y - 1, rgba(0xffffff, 120));
    }
}

/// A garden tap on a short post: a green standpipe with a brass spout and a red wheel on top. A
/// drip at the spout while it is ready to be turned; the wheel turned and a puddle under it while
/// it rests.
pub fn tap(canvas: &mut Canvas, view: &View, tile: Tile, ready: bool, now: f32, still: bool) {
    const PAINT: Ramp = Ramp::new(0x2f4a3c, 0x46705a, 0x5c8f74, 0x7aae92, 0xa2cdb4);
    const BRASS: Ramp = crate::garden::BRASS;
    let (cx, cy) = view.centre(tile);
    paint::ellipse(canvas, cx + 4, cy, 7, 2, rgba(0x24452b, 80));
    if !ready {
        paint::ellipse(canvas, cx - 6, cy + 1, 5, 2, rgba(0x7fb8c8, 150));
        paint::put(canvas, cx - 8, cy, rgba(0xdff3f2, 200));
    }
    // The standpipe: lit on its left, in shade on its right.
    for dy in 0..16 {
        paint::put(canvas, cx - 2, cy - dy, PAINT.light);
        paint::put(canvas, cx - 1, cy - dy, PAINT.base);
        paint::put(canvas, cx, cy - dy, PAINT.base);
        paint::put(canvas, cx + 1, cy - dy, PAINT.shadow);
        paint::put(canvas, cx + 2, cy - dy, PAINT.edge);
    }
    paint::hline(canvas, cx - 2, cy - 16, 5, PAINT.shine);
    // The spout, pointing toward the yard's side of the board.
    paint::hline(canvas, cx - 6, cy - 9, 5, BRASS.light);
    paint::hline(canvas, cx - 6, cy - 8, 5, BRASS.shadow);
    paint::vline(canvas, cx - 6, cy - 8, 2, BRASS.base);
    // The wheel: a red cross on a brass boss, turned a quarter while it rests.
    paint::put(canvas, cx, cy - 18, BRASS.shine);
    let red = (rgb(0xc8463a), rgb(0x6a2218));
    if ready {
        paint::hline(canvas, cx - 3, cy - 18, 7, red.0);
        paint::vline(canvas, cx, cy - 20, 3, red.0);
        paint::hline(canvas, cx - 3, cy - 17, 7, red.1);
    } else {
        paint::line(canvas, (cx - 2, cy - 20), (cx + 2, cy - 16), red.0);
        paint::line(canvas, (cx + 2, cy - 20), (cx - 2, cy - 16), red.0);
        paint::put(canvas, cx + 1, cy - 16, red.1);
    }
    if ready && !still {
        // A drip forming at the spout and falling, now and then.
        let t = (now * 0.9 + f32::from(tile.x) * 0.37).fract();
        let drop = (t * 10.0) as i32;
        if t > 0.35 {
            paint::put(canvas, cx - 6, cy - 6 + drop - 3, rgba(0xa8d6df, 220));
        } else {
            paint::put(canvas, cx - 6, cy - 6, rgba(0xdff3f2, (t * 600.0) as u8));
        }
    }
}

/// A leaf umbrella open over a creature's head, its stalk held at the shoulder: `(x, y)` is the
/// top of its head. It sits above the composure pips, so both show.
pub fn umbrella(canvas: &mut Canvas, (x, y): (i32, i32)) {
    let (cx, top) = (x + 2, y - 22);
    // The stalk, down past the pips to the shoulder.
    paint::vline(canvas, cx + 1, top + 6, 26, PLANK.shadow);
    paint::vline(canvas, cx, top + 6, 26, PLANK.light);
    for row in 0..8i32 {
        let half = 13 - (7 - row) * (7 - row) / 4;
        for dx in -half..=half {
            let color = if row == 7 || dx.abs() == half {
                LEAF.edge
            } else if dx < -half / 3 {
                LEAF.light
            } else if dx > half / 2 {
                LEAF.shadow
            } else {
                LEAF.base
            };
            paint::put(canvas, cx + dx, top + row, color);
        }
    }
    // The leaf's ribs, and a glint of light on its upper left.
    paint::vline(canvas, cx, top + 1, 6, LEAF.shadow);
    paint::line(canvas, (cx, top + 3), (cx - 8, top + 6), LEAF.shadow);
    paint::line(canvas, (cx, top + 3), (cx + 8, top + 6), LEAF.edge);
    paint::hline(canvas, cx - 6, top + 2, 4, LEAF.shine);
    paint::put(canvas, cx - 1, top - 1, LEAF.base);
}

/// The sprinkler's spray landing on a tile, `age` seconds in: a jet thrown up from the middle,
/// droplets arcing out and falling back round the tile, and a ripple spreading on the wet ground.
pub fn splash(canvas: &mut Canvas, view: &View, tile: Tile, age: f32, fade: u8) {
    let (cx, cy) = view.centre(tile);
    let t = (age / 0.7).clamp(0.0, 1.0);
    let ripple = 5 + (t * 16.0) as i32;
    for k in 0..20 {
        let a = k as f32 / 20.0 * TAU;
        paint::put(
            canvas,
            cx + (a.cos() * ripple as f32) as i32,
            cy + (a.sin() * ripple as f32 * 0.5) as i32,
            paint::faded(WATER.shine, fade),
        );
    }
    // The jet, tallest early on.
    let jet = (30.0 * (1.0 - t)) as i32;
    for dy in 0..jet {
        if dy % 3 != 2 {
            paint::put(canvas, cx, cy - 4 - dy, paint::faded(WATER.light, fade));
            paint::put(canvas, cx + 1, cy - 4 - dy, paint::faded(WATER.base, fade));
        }
    }
    for k in 0..14 {
        let a = k as f32 / 14.0 * TAU + f32::from(tile.x) * 0.7;
        let out = 3.0 + t * 18.0;
        // Up and back down over the splash's life.
        let rise = 28.0 * 4.0 * t * (1.0 - t);
        let (px, py) = (
            cx + (a.cos() * out) as i32,
            cy - 2 + (a.sin() * out * 0.5) as i32 - rise as i32,
        );
        paint::rect(canvas, px, py, 2, 2, paint::faded(WATER.light, fade));
        paint::put(canvas, px, py, paint::faded(WATER.shine, fade));
    }
}

/// A creature the sprinkler has soaked, for the rest of the turn: drops running off it and a
/// puddle at its feet. `(x, y)` is the top of its head, `feet` the floor under it.
pub fn dripping(canvas: &mut Canvas, (x, y): (i32, i32), feet: (i32, i32), now: f32, still: bool) {
    paint::ellipse(canvas, feet.0, feet.1, 9, 3, rgba(0x7fb8c8, 110));
    for k in 0..3 {
        let fall = if still {
            0.3 * k as f32
        } else {
            (now * 1.4 + k as f32 * 0.33).fract()
        };
        let dx = [-9, 2, 10][k];
        let py = y + 4 + (fall * 20.0) as i32;
        paint::put(canvas, x + dx, py - 1, WATER.shine);
        paint::rect(canvas, x + dx, py, 1, 2, WATER.light);
    }
}

/// A puff's gust landing on a tile: streaks of wind and dandelion seeds blown up and away.
pub fn gust(canvas: &mut Canvas, view: &View, tile: Tile, age: f32, fade: u8) {
    let (cx, cy) = view.centre(tile);
    let reach = (age * 30.0) as i32;
    for (k, dy) in [-8, -4, 0].into_iter().enumerate() {
        let x = cx - 10 + reach + k as i32 * 2;
        paint::hline(canvas, x, cy + dy - 6, 8, rgba(0xffffff, fade));
    }
    for k in 0..6 {
        let a = k as f32 * 1.1;
        let (px, py) = (
            cx + (a.cos() * 6.0) as i32 + reach / 2,
            cy - 10 - (age * 30.0) as i32 + (a.sin() * 4.0) as i32,
        );
        paint::put(canvas, px, py, paint::faded(FLUFF.shine, fade));
    }
}
