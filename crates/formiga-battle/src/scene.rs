//! The board painted into one picture at Formiga's pixel scale, to the same rules as Formiga
//! Home's rooms: every material a ramp, light from the upper left, edges in a darker shade of
//! their own colour, grain from `paint::noise`. Gameplay lives in the tiles; everything drawn here
//! is decoration laid over them, and nothing drawn decides where anyone can walk.

use crate::iso::{SLAB, View};
use crate::look::{Look, Pose};
use crate::playback::{Burst, Figure, Frame};
use formiga_art::paint::{self, Ramp, rgb, rgba};
use formiga_art::{Canvas, FRAME_SIZE, Rgba};
use formiga_battle_core::arena::Terrain;
use formiga_battle_core::{Arena, Order, Side, Slot, TEAM_SIZE, Target, Tile};

const GRASS: Ramp = Ramp::new(0x3a5426, 0x52722f, 0x6a8f3c, 0x88ab52, 0xaecb78);
const SAND: Ramp = Ramp::new(0x8a6a34, 0xb08b4c, 0xcfab68, 0xe2c588, 0xf0dcaa);
const SOIL: Ramp = Ramp::new(0x3e2818, 0x5c3c26, 0x7a5236, 0x966a48, 0xb48a64);
const HEDGE: Ramp = Ramp::new(0x1e3a1a, 0x2c5424, 0x3e6e30, 0x548c3e, 0x78ae58);
const WOOD: Ramp = Ramp::new(0x553620, 0x7a5030, 0x9c6a42, 0xba8858, 0xd8aa78);
const STONE: Ramp = Ramp::new(0x55524c, 0x77736a, 0x9a958a, 0xbab5a8, 0xdad6ca);
const GLOW: Rgba = rgb(0xffe9a0);

/// Each team's colours: left, then right.
pub const TEAM: [Ramp; 2] = [
    Ramp::new(0x7a3226, 0xb05240, 0xe07a5f, 0xf0a088, 0xfac8b4),
    Ramp::new(0x1a5052, 0x2a7476, 0x3d9fa0, 0x6cc0c0, 0xa4dede),
];

/// The ground round the board.
pub const BACKDROP: Rgba = rgb(0x24301f);

/// How high a hedge stands.
const HEDGE_HEIGHT: f32 = 18.0;

/// What a tile can be chosen for, while an order is being given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Walk,
    Hold,
    Contest,
}

/// What the window lays over the board: who is chosen, where an order could go, and the orders
/// given so far.
#[derive(Clone, Debug, Default)]
pub struct Marks {
    pub selected: Option<Slot>,
    pub choices: Vec<(Tile, Choice)>,
    pub hover: Option<Tile>,
    pub orders: Vec<(Slot, Order)>,
}

/// The board at one moment, as a picture of `view.size`.
pub fn paint(
    arena: &Arena,
    frame: &Frame,
    looks: &mut [Vec<Look>; 2],
    marks: &Marks,
    capture_ticks: u8,
    now: f32,
    view: &View,
) -> Canvas {
    let mut canvas = Canvas::new(view.size.0, view.size.1);
    paint::rect(
        &mut canvas,
        0,
        0,
        view.size.0 as i32,
        view.size.1 as i32,
        BACKDROP,
    );
    ground(&mut canvas, arena, frame, capture_ticks, view);
    floor_marks(&mut canvas, frame, marks, view);

    // Everything standing, back to front by how far down the board its tile is.
    let mut standing: Vec<(i32, i32, Item)> = Vec::new();
    for tile in arena.tiles() {
        let depth = i32::from(tile.x) + i32::from(tile.y);
        match arena.terrain(tile) {
            Terrain::Obstacle => standing.push((depth, 0, Item::Hedge(tile))),
            Terrain::Gate => standing.push((depth, 0, Item::Gate(tile))),
            Terrain::Lever => standing.push((depth, 0, Item::Lever(tile))),
            Terrain::Open => {}
        }
    }
    for side in Side::BOTH {
        for i in 0..TEAM_SIZE as u8 {
            let slot = Slot::new(side, i);
            let f = frame.figure(slot);
            if f.opacity <= 0.0 {
                continue;
            }
            // By the floor point under its feet, so a creature walking between tiles is drawn in
            // the right turn.
            let depth = (f.at.0 + f.at.1 - 1.0).floor() as i32;
            standing.push((depth, 1, Item::Creature(slot)));
        }
    }
    standing.sort_by_key(|(depth, order, item)| (*depth, *order, item.key()));
    for (_, _, item) in standing {
        match item {
            Item::Hedge(tile) => hedge(&mut canvas, view, tile),
            Item::Gate(tile) => {
                let i = arena.gates.iter().position(|g| *g == tile);
                let open = i.is_some_and(|i| frame.gates_open[i]);
                gate(&mut canvas, view, tile, open);
            }
            Item::Lever(tile) => lever(
                &mut canvas,
                view,
                tile,
                frame.gates_open.first().copied().unwrap_or(false),
                frame.swing_pending,
                now,
            ),
            Item::Creature(slot) => {
                let f = frame.figure(slot);
                let look = &mut looks[slot.side.index()][slot.index as usize];
                creature(
                    &mut canvas,
                    view,
                    slot,
                    f,
                    look,
                    marks.selected == Some(slot),
                );
            }
        }
    }
    order_marks(&mut canvas, frame, marks, view, now);
    for effect in &frame.effects {
        burst(&mut canvas, view, effect.burst, effect.tile, effect.age);
    }
    canvas
}

#[derive(Clone, Copy)]
enum Item {
    Hedge(Tile),
    Gate(Tile),
    Lever(Tile),
    Creature(Slot),
}

impl Item {
    fn key(&self) -> (i32, i32) {
        match self {
            Self::Hedge(t) | Self::Gate(t) | Self::Lever(t) => (i32::from(t.y), 0),
            Self::Creature(s) => (s.side.index() as i32, i32::from(s.index)),
        }
    }
}

/// Where a creature's frame goes for it to stand at its feet.
pub fn frame_origin(view: &View, f: &Figure, look: &Look) -> (i32, i32) {
    let (sx, sy) = view.pixel(f.at.0, f.at.1);
    (sx - FRAME_SIZE as i32 / 2, sy - look.foot_row)
}

/// Over a creature's head, for its name and pips.
pub fn head(view: &View, f: &Figure, look: &Look) -> (i32, i32) {
    let (x, y) = frame_origin(view, f, look);
    (x + FRAME_SIZE as i32 / 2, y + 6)
}

fn lit(tone: Rgba, ramp: Ramp, light: f32) -> Rgba {
    if light > 0.0 {
        paint::mix(tone, ramp.shine, light)
    } else {
        paint::mix(tone, ramp.shadow, -light)
    }
}

fn ground(canvas: &mut Canvas, arena: &Arena, frame: &Frame, capture_ticks: u8, view: &View) {
    let (w, d) = (f32::from(arena.width), f32::from(arena.height));
    // The near edges' cross-section first, so the tiles sit on it.
    let corner = |x: f32, y: f32| view.screen(x, y);
    let (left, near, right) = (corner(0.0, d), corner(w, d), corner(w, 0.0));
    let slab = SLAB as f32;
    paint::fill_polygon(
        &[left, near, (near.0, near.1 + slab), (left.0, left.1 + slab)],
        |px, py| {
            let tone = if paint::chance(px, py, 7, 40) {
                SOIL.shadow
            } else {
                SOIL.light
            };
            paint::put(canvas, px, py, tone);
        },
    );
    paint::fill_polygon(
        &[
            near,
            right,
            (right.0, right.1 + slab),
            (near.0, near.1 + slab),
        ],
        |px, py| {
            let tone = if paint::chance(px, py, 9, 40) {
                SOIL.edge
            } else {
                SOIL.base
            };
            paint::put(canvas, px, py, tone);
        },
    );
    paint::line(
        canvas,
        (left.0 as i32, (left.1 + slab) as i32),
        (near.0 as i32, (near.1 + slab) as i32),
        SOIL.edge,
    );
    paint::line(
        canvas,
        (near.0 as i32, (near.1 + slab) as i32),
        (right.0 as i32, (right.1 + slab) as i32),
        SOIL.edge,
    );
    // Whoever leads on the beacon tints its yard, more the nearer they are to lighting it.
    let leader = if frame.capture[0] > frame.capture[1] {
        Some(0)
    } else if frame.capture[1] > frame.capture[0] {
        Some(1)
    } else {
        None
    };
    let lead = leader.map(|s| {
        (
            TEAM[s],
            f32::from(frame.capture[s]) / f32::from(capture_ticks.max(1)),
        )
    });
    let centre = arena.zone_centre();
    for tile in arena.tiles() {
        let zone = arena.in_zone(tile);
        let ramp = if zone { SAND } else { GRASS };
        let corners = view.diamond(tile, 0.0);
        paint::fill_polygon(&corners, |px, py| {
            let (fx, fy) = view.floor_at(px as f32 + 0.5, py as f32 + 0.5);
            let (ux, uy) = (fx - fx.floor(), fy - fy.floor());
            let mut tone = if zone {
                if paint::chance(px, py, 3, 30) {
                    ramp.light
                } else {
                    ramp.base
                }
            } else {
                // Mown in stripes, one tile wide.
                let stripe = (i32::from(tile.x) + i32::from(tile.y)) % 2 == 0;
                let tone = if stripe {
                    ramp.base
                } else {
                    paint::mix(ramp.base, ramp.light, 0.35)
                };
                if paint::chance(px, py, 5, 26) {
                    ramp.light
                } else if paint::chance(px, py, 6, 22) {
                    ramp.shadow
                } else {
                    tone
                }
            };
            // Lit from the upper left.
            tone = lit(tone, ramp, ((fy - fx) / (w + d)) * 0.22);
            // A faint seam round every tile, so the grid reads.
            if ux < 0.035 || uy < 0.035 {
                tone = paint::mix(tone, ramp.shadow, 0.55);
            }
            if zone && let Some((team, share)) = lead {
                tone = paint::mix(tone, team.light, 0.15 + 0.45 * share);
            }
            paint::put(canvas, px, py, tone);
        });
    }
    // The beacon's mark in the middle of its yard.
    let (cx, cy) = view.centre(centre);
    paint::ellipse(canvas, cx, cy, 10, 5, SAND.edge);
    paint::ellipse(canvas, cx, cy, 9, 4, SAND.shine);
    paint::ellipse(canvas, cx, cy, 5, 2, GLOW);
}

fn diamond_outline(canvas: &mut Canvas, view: &View, tile: Tile, inset: f32, color: Rgba) {
    let c = view.diamond(tile, 0.0);
    let mid = view.screen(f32::from(tile.x) + 0.5, f32::from(tile.y) + 0.5);
    let pull = |p: (f32, f32)| {
        (
            (p.0 + (mid.0 - p.0) * inset).round() as i32,
            (p.1 + (mid.1 - p.1) * inset).round() as i32,
        )
    };
    for i in 0..4 {
        paint::line(canvas, pull(c[i]), pull(c[(i + 1) % 4]), color);
    }
}

fn diamond_fill(canvas: &mut Canvas, view: &View, tile: Tile, color: Rgba) {
    let c = view.diamond(tile, 0.0);
    paint::polygon(canvas, &c, color);
}

fn floor_marks(canvas: &mut Canvas, frame: &Frame, marks: &Marks, view: &View) {
    for (tile, choice) in &marks.choices {
        let color = match choice {
            Choice::Walk => rgba(0xffffff, 46),
            Choice::Hold => rgba(0x9fd8ff, 70),
            Choice::Contest => rgba(0xff6a4a, 84),
        };
        diamond_fill(canvas, view, *tile, color);
    }
    if let Some(slot) = marks.selected {
        let f = frame.figure(slot);
        diamond_outline(canvas, view, f.tile, 0.08, rgb(0xfff6c8));
        diamond_outline(canvas, view, f.tile, 0.16, rgba(0xfff6c8, 120));
    }
    if let Some(tile) = marks.hover {
        diamond_outline(canvas, view, tile, 0.12, rgba(0xffffff, 200));
    }
    // A hold is marked on its tile, under whoever stands there.
    for (slot, order) in &marks.orders {
        if let Order::Hold { at } = order {
            let team = TEAM[slot.side.index()];
            diamond_outline(canvas, view, *at, 0.1, team.shine);
            diamond_outline(canvas, view, *at, 0.2, team.light);
        }
    }
}

fn order_marks(canvas: &mut Canvas, frame: &Frame, marks: &Marks, view: &View, now: f32) {
    for (slot, order) in &marks.orders {
        let team = TEAM[slot.side.index()];
        let from = frame.figure(*slot).tile;
        match order {
            Order::Move { to } => {
                // Dots from where it stands to where it is going, and a flag there.
                let (a, b) = (view.centre(from), view.centre(*to));
                let steps = ((b.0 - a.0).abs().max((b.1 - a.1).abs()) / 3).max(1);
                for i in 1..steps {
                    if i % 2 == 0 {
                        continue;
                    }
                    let x = a.0 + (b.0 - a.0) * i / steps;
                    let y = a.1 + (b.1 - a.1) * i / steps;
                    paint::rect(canvas, x, y, 2, 1, team.shine);
                }
                flag(canvas, b, team);
            }
            Order::Hold { .. } => {}
            Order::Contest { target } => {
                let at = match target {
                    Target::Creature { slot } => frame.figure(*slot).tile,
                    Target::Lever { at } => *at,
                };
                let (x, y) = view.centre(at);
                let bob = ((now * 4.0).sin() * 1.5).round() as i32;
                chevron(canvas, (x, y - 50 + bob), team);
            }
        }
    }
}

fn flag(canvas: &mut Canvas, at: (i32, i32), team: Ramp) {
    let (x, y) = at;
    paint::vline(canvas, x, y - 12, 12, WOOD.edge);
    paint::polygon(
        canvas,
        &[
            (x as f32 + 1.0, y as f32 - 12.0),
            (x as f32 + 8.0, y as f32 - 9.5),
            (x as f32 + 1.0, y as f32 - 7.0),
        ],
        team.base,
    );
    paint::put(canvas, x + 1, y - 12, team.shine);
}

/// A downward chevron over a contest's target.
fn chevron(canvas: &mut Canvas, at: (i32, i32), team: Ramp) {
    let (x, y) = at;
    for (i, row) in (0..5).enumerate() {
        let half = 5 - i as i32;
        paint::hline(canvas, x - half, y + row, 2, team.edge);
        paint::hline(canvas, x + half - 1, y + row, 2, team.edge);
        paint::hline(canvas, x - half + 1, y + row, 1, team.light);
        paint::hline(canvas, x + half - 1, y + row, 1, team.light);
    }
    paint::rect(canvas, x - 1, y + 5, 2, 2, team.edge);
}

/// An isometric block on a tile, `height` pixels tall.
fn block(canvas: &mut Canvas, view: &View, tile: Tile, height: f32, ramp: Ramp, salt: u32) {
    let low = view.diamond(tile, 0.0);
    let high = view.diamond(tile, height);
    // Faces toward the viewer: the left one (W to S) half-lit, the right one (S to E) in shade.
    let left = [high[3], high[2], low[2], low[3]];
    let right = [high[2], high[1], low[1], low[2]];
    paint::fill_polygon(&left, |px, py| {
        let tone = if paint::chance(px, py, salt, 50) {
            ramp.base
        } else {
            ramp.shadow
        };
        paint::put(canvas, px, py, tone);
    });
    paint::fill_polygon(&right, |px, py| {
        let tone = if paint::chance(px, py, salt + 1, 50) {
            ramp.shadow
        } else {
            ramp.edge
        };
        paint::put(canvas, px, py, paint::mix(tone, ramp.shadow, 0.3));
    });
    paint::fill_polygon(&high, |px, py| {
        let tone = if paint::chance(px, py, salt + 2, 60) {
            ramp.light
        } else if paint::chance(px, py, salt + 3, 20) {
            ramp.shine
        } else {
            ramp.base
        };
        paint::put(canvas, px, py, tone);
    });
    let p = |q: (f32, f32)| (q.0.round() as i32, q.1.round() as i32);
    for (a, b) in [
        (high[0], high[1]),
        (high[1], high[2]),
        (high[2], high[3]),
        (high[3], high[0]),
        (high[3], low[3]),
        (high[2], low[2]),
        (high[1], low[1]),
        (low[3], low[2]),
        (low[2], low[1]),
    ] {
        paint::line(canvas, p(a), p(b), ramp.edge);
    }
}

fn hedge(canvas: &mut Canvas, view: &View, tile: Tile) {
    block(canvas, view, tile, HEDGE_HEIGHT, HEDGE, 11);
    // A few leaves catching the light along its top.
    let top = view.diamond(tile, HEDGE_HEIGHT);
    let (cx, cy) = ((top[0].0 + top[2].0) / 2.0, (top[0].1 + top[2].1) / 2.0);
    for (dx, dy) in [
        (-9, 0),
        (6, -3),
        (1, 3),
        (12, 1),
        (-3, -4),
        (-14, 2),
        (8, 4),
    ] {
        paint::put(canvas, cx as i32 + dx, cy as i32 + dy, HEDGE.shine);
    }
}

/// A gate across its tile, along the wall it stands in: shut, two rails between two posts;
/// open, the rails swung back against one post.
fn gate(canvas: &mut Canvas, view: &View, tile: Tile, open: bool) {
    let (x, y) = (f32::from(tile.x), f32::from(tile.y));
    let a = view.pixel(x + 0.12, y + 0.5);
    let b = view.pixel(x + 0.88, y + 0.5);
    let post = |canvas: &mut Canvas, (px, py): (i32, i32)| {
        paint::rect(canvas, px - 1, py - 22, 3, 22, WOOD.edge);
        paint::vline(canvas, px, py - 21, 20, WOOD.light);
        paint::put(canvas, px, py - 22, WOOD.shine);
    };
    post(canvas, a);
    if open {
        let swung = view.pixel(x + 0.12, y + 1.0);
        for lift in [6, 15] {
            paint::line(
                canvas,
                (a.0, a.1 - lift),
                (swung.0, swung.1 - lift),
                WOOD.edge,
            );
            paint::line(
                canvas,
                (a.0, a.1 - lift - 1),
                (swung.0, swung.1 - lift - 1),
                WOOD.light,
            );
        }
    } else {
        for lift in [6, 15] {
            paint::line(canvas, (a.0, a.1 - lift), (b.0, b.1 - lift), WOOD.edge);
            paint::line(
                canvas,
                (a.0, a.1 - lift - 1),
                (b.0, b.1 - lift - 1),
                WOOD.base,
            );
            paint::line(
                canvas,
                (a.0, a.1 - lift - 2),
                (b.0, b.1 - lift - 2),
                WOOD.light,
            );
        }
    }
    post(canvas, b);
}

/// The lever on its stone: leaning left while the gates are as they started, right once swung,
/// its knob glowing while a swing is coming.
fn lever(canvas: &mut Canvas, view: &View, tile: Tile, swung: bool, pending: bool, now: f32) {
    block(canvas, view, tile, 6.0, STONE, 23);
    let (cx, cy) = view.pixel(f32::from(tile.x) + 0.5, f32::from(tile.y) + 0.5);
    let base = (cx, cy - 6);
    let tip = if swung {
        (cx + 8, cy - 22)
    } else {
        (cx - 8, cy - 22)
    };
    paint::line(canvas, base, tip, WOOD.edge);
    paint::line(canvas, (base.0 + 1, base.1), (tip.0 + 1, tip.1), WOOD.light);
    let knob = if pending && (now * 6.0).sin() > -0.3 {
        GLOW
    } else {
        rgb(0xc0463a)
    };
    paint::ellipse(canvas, tip.0, tip.1, 3, 3, rgb(0x5a1e18));
    paint::ellipse(canvas, tip.0, tip.1, 2, 2, knob);
}

fn creature(
    canvas: &mut Canvas,
    view: &View,
    slot: Slot,
    f: &Figure,
    look: &mut Look,
    chosen: bool,
) {
    let team = TEAM[slot.side.index()];
    let (fx, fy) = view.pixel(f.at.0, f.at.1);
    let alpha = (f.opacity * 255.0).round() as u8;
    // Its team's ring under its feet; doubled while it holds.
    paint::ellipse(canvas, fx, fy, 14, 6, rgba(0x10180c, alpha / 3));
    paint::ellipse(
        canvas,
        fx,
        fy,
        12,
        5,
        paint::faded(team.base, (alpha as f32 * 0.55) as u8),
    );
    let ring = if chosen { rgb(0xfff6c8) } else { team.base };
    ellipse_ring(canvas, fx, fy, 14, 6, paint::faded(ring, alpha));
    if f.holding {
        ellipse_ring(canvas, fx, fy, 16, 7, paint::faded(team.light, alpha));
        ellipse_ring(canvas, fx, fy, 17, 8, paint::faded(team.base, alpha));
    }
    let (ox, oy) = frame_origin(view, f, look);
    let sprite = look.frame(f.pose, f.posed_for, f.facing_right);
    if f.pose == Pose::Dizzy {
        paint::blit_faded(canvas, sprite, ox, oy + 4, alpha);
    } else {
        paint::blit_faded(canvas, sprite, ox, oy, alpha);
    }
    if !f.out {
        pips(canvas, head_point(ox, oy), f.composure, team);
    }
}

fn head_point(ox: i32, oy: i32) -> (i32, i32) {
    (ox + FRAME_SIZE as i32 / 2, oy + 6)
}

/// Composure, as three little pips: filled in the team's colour, empty in grey.
fn pips(canvas: &mut Canvas, at: (i32, i32), composure: u8, team: Ramp) {
    let (x, y) = at;
    for i in 0..3 {
        let px = x - 5 + i * 4;
        let full = (i as u8) < composure;
        paint::rect(
            canvas,
            px - 1,
            y - 1,
            3,
            3,
            if full { team.edge } else { rgb(0x3a3a36) },
        );
        paint::put(canvas, px, y, if full { team.shine } else { rgb(0x6a6a64) });
    }
}

fn ellipse_ring(canvas: &mut Canvas, cx: i32, cy: i32, rx: i32, ry: i32, color: Rgba) {
    let steps = 48;
    let mut last: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let a = i as f32 / steps as f32 * std::f32::consts::TAU;
        let p = (
            cx + (a.cos() * rx as f32).round() as i32,
            cy + (a.sin() * ry as f32).round() as i32,
        );
        if last != Some(p) {
            paint::put(canvas, p.0, p.1, color);
        }
        last = Some(p);
    }
}

fn burst(canvas: &mut Canvas, view: &View, burst: Burst, tile: Tile, age: f32) {
    let (cx, cy) = view.centre(tile);
    let fade = ((1.0 - age / 0.7).clamp(0.0, 1.0) * 255.0) as u8;
    match burst {
        Burst::Hit => {
            let r = 4.0 + age * 14.0;
            let (x, y) = (cx, cy - 22);
            for k in 0..8 {
                let a = k as f32 / 8.0 * std::f32::consts::TAU + 0.3;
                let p0 = (
                    x + (a.cos() * r * 0.4) as i32,
                    y + (a.sin() * r * 0.4) as i32,
                );
                let p1 = (x + (a.cos() * r) as i32, y + (a.sin() * r) as i32);
                paint::line(canvas, p0, p1, rgba(0xfff3b0, fade));
            }
            paint::ellipse(canvas, x, y, 2, 2, rgba(0xffffff, fade));
        }
        Burst::Shield => {
            let (x, y) = (cx, cy - 18);
            let r = 10 + (age * 6.0) as i32;
            for k in -6..=6 {
                let a = std::f32::consts::FRAC_PI_2 + k as f32 * 0.22;
                let p = (
                    x + (a.cos() * r as f32) as i32,
                    y - (a.sin() * r as f32) as i32,
                );
                paint::rect(canvas, p.0, p.1, 2, 2, rgba(0xbfe6ff, fade));
            }
        }
        Burst::Scuffle | Burst::Bump => {
            for (k, (dx, dy)) in [(-6, -4), (5, -6), (0, -10), (-3, -14), (7, -12)]
                .iter()
                .enumerate()
            {
                if burst == Burst::Bump && k > 2 {
                    break;
                }
                let grow = (age * 6.0) as i32;
                paint::ellipse(
                    canvas,
                    cx + dx + dx.signum() * grow,
                    cy + dy,
                    2 + grow / 2,
                    2 + grow / 2,
                    rgba(0xd8d0c0, fade / 2 + 40),
                );
            }
        }
        Burst::Stars => {
            for k in 0..3 {
                let a = age * 7.0 + k as f32 * std::f32::consts::TAU / 3.0;
                let p = (
                    cx + (a.cos() * 9.0) as i32,
                    cy - 26 + (a.sin() * 3.0) as i32,
                );
                star(canvas, p, rgba(0xffe066, fade));
            }
        }
        Burst::Spark => {
            let (x, y) = (cx, cy - 12);
            for k in 0..4 {
                let a = k as f32 * std::f32::consts::FRAC_PI_2 + age * 3.0;
                let r = 3.0 + age * 10.0;
                star(
                    canvas,
                    (x + (a.cos() * r) as i32, y + (a.sin() * r * 0.6) as i32),
                    rgba(0xfff3b0, fade),
                );
            }
        }
    }
}

fn star(canvas: &mut Canvas, (x, y): (i32, i32), color: Rgba) {
    paint::put(canvas, x, y, color);
    paint::put(canvas, x - 1, y, color);
    paint::put(canvas, x + 1, y, color);
    paint::put(canvas, x, y - 1, color);
    paint::put(canvas, x, y + 1, color);
}
