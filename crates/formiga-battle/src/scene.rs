//! The board painted into one picture at Formiga's pixel scale: the garden (`garden`), and over it
//! everyone standing in it, back to front, and the marks that say what is chosen, where an order
//! could go, and what each order is. Gameplay lives in the tiles; the garden is decoration, and
//! nothing drawn here decides where anyone can walk.
//!
//! The marks are drawn the way Hill picks things out: corner brackets on a tile rather than a
//! filled one, in cream or gold, so the lawn still shows through and the board never turns into
//! a grid of overlays.

use crate::garden::{self, Garden, Sprite};
use crate::iso::View;
use crate::look::{Look, Pose};
use crate::playback::{Burst, Figure, Frame};
use formiga_art::paint::{self, Ramp, rgb, rgba};
use formiga_art::{Canvas, FRAME_SIZE, Rgba};
use formiga_battle_core::arena::Terrain;
use formiga_battle_core::{Arena, Order, Side, Slot, TEAM_SIZE, Target, Tile};
use std::f32::consts::{FRAC_PI_2, TAU};

/// Each team's colours: left, then right. Warm coral and sea teal, as before, a touch softened to
/// sit with Hill's and Home's palettes.
pub const TEAM: [Ramp; 2] = [
    Ramp::new(0x7a3226, 0xb05240, 0xe07a5f, 0xf0a088, 0xfac8b4),
    Ramp::new(0x1a5052, 0x2a7476, 0x3d9fa0, 0x6cc0c0, 0xa4dede),
];

/// Cream, for whatever is chosen: Hill's hover cream and Home's hover ring.
const CREAM: Rgba = rgb(0xfff3c4);
/// Gold, for whatever is pointed at.
const GOLD: Rgba = rgb(0xf5d25e);

/// What a tile can be chosen for, while an order is being given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Walk,
    Hold,
    Contest,
    /// Where what it carries could be used.
    Use,
}

/// A cartoon word to paint over the board: its picture, where its top-left corner goes, and how
/// much of it shows, out of 255.
pub type Lettered = (Canvas, i32, i32, u8);

/// What the window lays over the board: who is chosen, where an order could go, the orders given
/// so far, and the little flourish wherever the Handler last clicked.
#[derive(Clone, Debug, Default)]
pub struct Marks {
    pub selected: Option<Slot>,
    pub choices: Vec<(Tile, Choice)>,
    pub hover: Option<Tile>,
    pub orders: Vec<(Slot, Order)>,
    /// Tiles just given an order, and how many seconds ago.
    pub pokes: Vec<(Tile, f32)>,
    /// Cartoon words (`lettering`), over everything: each picture, where its top-left corner
    /// goes, and how much of it shows, out of 255.
    pub lettering: Vec<Lettered>,
}

/// The board at one moment, as a picture of `view.size`. `still` holds the garden's own motion
/// and the marks' pulsing, for reduced motion.
#[allow(clippy::too_many_arguments)]
pub fn paint(
    arena: &Arena,
    garden: &Garden,
    frame: &Frame,
    looks: &mut [Vec<Look>; 2],
    marks: &Marks,
    capture_ticks: u8,
    now: f32,
    still: bool,
    view: &View,
) -> Canvas {
    let mut canvas = garden.ground.clone();
    beacon(&mut canvas, arena, frame, capture_ticks, now, still, view);
    garden.behind(&mut canvas, view, now, still);
    floor_marks(&mut canvas, frame, marks, view, now, still);

    // Everyone and everything standing, back to front by how far down the board it is.
    let mut standing: Vec<(i32, i32, Item)> = Vec::new();
    for tile in arena.tiles() {
        let depth = i32::from(tile.x) + i32::from(tile.y);
        match arena.terrain(tile) {
            Terrain::Obstacle => standing.push((depth, 0, Item::Piece(tile))),
            Terrain::Gate => standing.push((depth, 0, Item::Gate(tile))),
            Terrain::Lever => standing.push((depth, 0, Item::Lever(tile))),
            Terrain::Open => {}
        }
    }
    // Each creature's picture as it will be drawn, for seeing pieces through only where they would
    // hide its shape, and its shadow and rings, painted on the ground under everyone so they never
    // lie across a neighbour or over a hedge in front.
    let mut bodies: Vec<Body> = Vec::new();
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
            footing(
                &mut canvas,
                view,
                slot,
                f,
                marks.selected == Some(slot),
                now,
                still,
            );
            let look = &mut looks[side.index()][i as usize];
            let (x, y) = placed(view, f, look, now, still);
            let pic = look.frame(f.pose, f.posed_for, f.facing_right).clone();
            bodies.push(Body { depth, pic, x, y });
        }
    }
    standing.sort_by_key(|(depth, order, item)| (*depth, *order, item.key()));
    for (depth, _, item) in standing {
        match item {
            Item::Piece(tile) => {
                if let Some(sprite) = garden.pieces.get(&tile) {
                    see_through(&mut canvas, sprite, depth, &bodies);
                }
            }
            Item::Gate(tile) => {
                let i = arena.gates.iter().position(|g| *g == tile);
                let open = i.is_some_and(|i| frame.gates_open[i]);
                if let Some(sprites) = garden.gates.get(&tile) {
                    see_through(&mut canvas, &sprites[usize::from(open)], depth, &bodies);
                }
            }
            Item::Lever(tile) => garden.lever(
                &mut canvas,
                view,
                tile,
                frame.gates_open.first().copied().unwrap_or(false),
                frame.swing_pending,
                if still { 0.0 } else { now },
            ),
            Item::Creature(slot) => {
                let f = frame.figure(slot);
                let look = &mut looks[slot.side.index()][slot.index as usize];
                let (x, y) = placed(view, f, look, now, still);
                let sprite = look.frame(f.pose, f.posed_for, f.facing_right);
                paint::blit_faded(&mut canvas, sprite, x, y, (f.opacity * 255.0).round() as u8);
            }
        }
    }
    // Composure and the off-balance swirl over everyone, so a neighbour standing in front never
    // covers them and theirs never sit on someone else's face.
    for side in Side::BOTH {
        for i in 0..TEAM_SIZE as u8 {
            let slot = Slot::new(side, i);
            let f = frame.figure(slot);
            if f.opacity > 0.0 && !f.out {
                let look = &looks[side.index()][i as usize];
                tags(&mut canvas, view, slot, f, look, now, still);
            }
        }
    }
    order_marks(&mut canvas, frame, marks, view, now, still);
    for effect in &frame.effects {
        burst(&mut canvas, view, effect, now);
    }
    for &(tile, age) in &marks.pokes {
        poke(&mut canvas, view, tile, age, still);
    }
    garden.over(&mut canvas, view, now, still);
    for (word, x, y, opacity) in &marks.lettering {
        paint::blit_faded(&mut canvas, word, *x, *y, *opacity);
    }
    canvas
}

#[derive(Clone, Copy)]
enum Item {
    Piece(Tile),
    Gate(Tile),
    Lever(Tile),
    Creature(Slot),
}

impl Item {
    fn key(&self) -> (i32, i32) {
        match self {
            Self::Piece(t) | Self::Gate(t) | Self::Lever(t) => (i32::from(t.y), 0),
            Self::Creature(s) => (s.side.index() as i32, i32::from(s.index)),
        }
    }
}

/// A creature's picture where it will be drawn, and how far down the board it stands.
struct Body {
    depth: i32,
    pic: Canvas,
    x: i32,
    y: i32,
}

impl Body {
    /// Whether its picture covers this canvas pixel.
    fn covers(&self, px: i32, py: i32) -> bool {
        let (sx, sy) = (px - self.x, py - self.y);
        sx >= 0
            && sy >= 0
            && sx < self.pic.width() as i32
            && sy < self.pic.height() as i32
            && self.pic.get(sx, sy).a > 0
    }

    fn overlaps(&self, (l, t, r, b): (i32, i32, i32, i32)) -> bool {
        self.x < r
            && self.x + self.pic.width() as i32 > l
            && self.y < b
            && self.y + self.pic.height() as i32 > t
    }
}

/// A standing piece, seen through wherever it stands in front of someone it would hide: Home's
/// rule for furniture, so nobody on the board is ever lost behind a hedge. Only the creature's own
/// shape shows through, so the piece never has a box cut out of it.
fn see_through(canvas: &mut Canvas, sprite: &Sprite, depth: i32, bodies: &[Body]) {
    let hidden: Vec<&Body> = bodies
        .iter()
        .filter(|b| b.depth < depth && b.overlaps(sprite.bounds()))
        .collect();
    if hidden.is_empty() {
        paint::blit(canvas, &sprite.pic, sprite.x, sprite.y);
    } else {
        paint::blit_through(canvas, &sprite.pic, sprite.x, sprite.y, |px, py| {
            if hidden.iter().any(|b| b.covers(px, py)) {
                110
            } else {
                255
            }
        });
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

/// A slow breath for marks that want attention, or a steady middle under reduced motion.
fn pulse(now: f32, still: bool, speed: f32) -> f32 {
    if still {
        0.5
    } else {
        0.5 + 0.5 * (now * speed).sin()
    }
}

/// The beacon's yard warmed by whoever leads on it, or guttering while both hold it.
fn beacon(
    canvas: &mut Canvas,
    arena: &Arena,
    frame: &Frame,
    capture_ticks: u8,
    now: f32,
    still: bool,
    view: &View,
) {
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
    let paused = frame.paused.then_some(TEAM);
    garden::beacon_light(canvas, arena, view, lead, paused, now, still);
}

/// Pixel corner brackets round a tile, pulled `inset` of the way in: Hill's way of picking a
/// thing out without covering it.
fn brackets(canvas: &mut Canvas, view: &View, tile: Tile, inset: f32, arm: f32, color: Rgba) {
    let c = view.diamond(tile, 0.0);
    let mid = view.screen(f32::from(tile.x) + 0.5, f32::from(tile.y) + 0.5);
    let pull = |p: (f32, f32)| (p.0 + (mid.0 - p.0) * inset, p.1 + (mid.1 - p.1) * inset);
    let corners = c.map(pull);
    for i in 0..4 {
        let p = corners[i];
        for q in [corners[(i + 1) % 4], corners[(i + 3) % 4]] {
            let end = (p.0 + (q.0 - p.0) * arm, p.1 + (q.1 - p.1) * arm);
            paint::line(
                canvas,
                (p.0.round() as i32, p.1.round() as i32),
                (end.0.round() as i32, end.1.round() as i32),
                color,
            );
        }
    }
}

fn diamond_outline(canvas: &mut Canvas, view: &View, tile: Tile, inset: f32, color: Rgba) {
    brackets(canvas, view, tile, inset, 1.0, color);
}

/// A tile's face, pulled in a little from its seams.
fn diamond_fill(canvas: &mut Canvas, view: &View, tile: Tile, inset: f32, color: Rgba) {
    let c = view.diamond(tile, 0.0);
    let mid = view.screen(f32::from(tile.x) + 0.5, f32::from(tile.y) + 0.5);
    let pulled = c.map(|p| (p.0 + (mid.0 - p.0) * inset, p.1 + (mid.1 - p.1) * inset));
    paint::polygon(canvas, &pulled, color);
}

fn floor_marks(
    canvas: &mut Canvas,
    frame: &Frame,
    marks: &Marks,
    view: &View,
    now: f32,
    still: bool,
) {
    let breath = pulse(now, still, 3.2);
    for (tile, choice) in &marks.choices {
        let (fill, edge) = match choice {
            Choice::Walk => (rgb(0xfffcf2), CREAM),
            Choice::Hold => (rgb(0xbfe6ff), rgb(0xdff3ff)),
            Choice::Contest => (rgb(0xff8a6a), rgb(0xffc0a8)),
            Choice::Use => (GOLD, rgb(0xfff0a0)),
        };
        diamond_fill(
            canvas,
            view,
            *tile,
            0.08,
            paint::faded(fill, (34.0 + 26.0 * breath) as u8),
        );
        brackets(canvas, view, *tile, 0.1, 0.28, paint::faded(edge, 210));
    }
    if let Some(slot) = marks.selected {
        let f = frame.figure(slot);
        diamond_fill(canvas, view, f.tile, 0.06, paint::faded(CREAM, 50));
        diamond_outline(
            canvas,
            view,
            f.tile,
            0.06,
            paint::faded(CREAM, (160.0 + 90.0 * breath) as u8),
        );
    }
    if let Some(tile) = marks.hover {
        let lift = if still { 0.0 } else { (now * 6.0).sin() * 0.02 };
        brackets(canvas, view, tile, 0.04 + lift, 0.34, GOLD);
        brackets(canvas, view, tile, 0.1 + lift, 0.3, paint::faded(GOLD, 120));
    }
    // A hold is marked on its tile, under whoever stands there.
    for (slot, order) in &marks.orders {
        if let Order::Hold { at } = order {
            let team = TEAM[slot.side.index()];
            diamond_fill(canvas, view, *at, 0.14, paint::faded(team.light, 60));
            diamond_outline(canvas, view, *at, 0.14, paint::faded(team.shine, 220));
        }
    }
}

fn order_marks(
    canvas: &mut Canvas,
    frame: &Frame,
    marks: &Marks,
    view: &View,
    now: f32,
    still: bool,
) {
    for (slot, order) in &marks.orders {
        let team = TEAM[slot.side.index()];
        let from = frame.figure(*slot).tile;
        match order {
            Order::Move { to } => {
                // Little footprints from where it stands to where it is going, and a pennant
                // there on a garden cane.
                let (a, b) = (view.centre(from), view.centre(*to));
                let (dx, dy) = ((b.0 - a.0) as f32, (b.1 - a.1) as f32);
                let length = (dx * dx + dy * dy).sqrt().max(1.0);
                let (nx, ny) = (-dy / length, dx / length);
                let steps = (length / 7.0).floor() as i32;
                for i in 1..steps {
                    let t = i as f32 / steps as f32;
                    let side = if i % 2 == 0 { 1.5 } else { -1.5 };
                    let x = a.0 as f32 + dx * t + nx * side;
                    let y = a.1 as f32 + dy * t + ny * side * 0.5;
                    let (x, y) = (x.round() as i32, y.round() as i32);
                    paint::rect(canvas, x, y, 2, 1, paint::faded(team.edge, 150));
                    paint::put(canvas, x, y, paint::faded(team.shine, 230));
                }
                pennant(canvas, b, team, now, still);
            }
            Order::Hold { .. } => {}
            // Where something is to be used: a gold diamond on its tile, in the team's light.
            Order::Use { at } => {
                let team = TEAM[slot.side.index()];
                diamond_fill(canvas, view, *at, 0.2, paint::faded(team.light, 70));
                brackets(canvas, view, *at, 0.12, 0.3, GOLD);
            }
            Order::Contest { target } => {
                let (at, lever) = match target {
                    Target::Creature { slot } => (frame.figure(*slot).tile, false),
                    Target::Lever { at } => (*at, true),
                };
                let (x, y) = view.centre(at);
                // A ring round the target's feet, turning, and a chevron over its head.
                let turn = if still { 0.0 } else { now * 1.6 };
                for k in 0..12 {
                    if k % 3 == 2 {
                        continue;
                    }
                    let a = turn + k as f32 / 12.0 * TAU;
                    let p = (
                        x + (a.cos() * 13.0).round() as i32,
                        y + (a.sin() * 6.0).round() as i32,
                    );
                    paint::put(canvas, p.0, p.1, team.shine);
                    paint::put(canvas, p.0 + 1, p.1, team.light);
                }
                let bob = if still {
                    0
                } else {
                    ((now * 4.0).sin() * 1.5).round() as i32
                };
                let over = if lever { 32 } else { 52 };
                chevron(canvas, (x, y - over + bob), team);
            }
        }
    }
}

/// A pennant on a garden cane, where a move is going.
fn pennant(canvas: &mut Canvas, at: (i32, i32), team: Ramp, now: f32, still: bool) {
    let (x, y) = at;
    paint::ellipse(canvas, x + 1, y, 3, 1, rgba(0x24452b, 60));
    paint::vline(canvas, x, y - 14, 14, garden::PLANK.shadow);
    paint::vline(canvas, x + 1, y - 14, 13, garden::PLANK.light);
    let stir = if still { 0.0 } else { (now * 3.0).sin() };
    let tip = (x as f32 + 9.0 + stir, y as f32 - 10.5 + stir * 0.5);
    paint::polygon(
        canvas,
        &[
            (x as f32 + 2.0, y as f32 - 14.0),
            tip,
            (x as f32 + 2.0, y as f32 - 7.0),
        ],
        team.base,
    );
    paint::line(
        canvas,
        (x + 2, y - 14),
        (tip.0.round() as i32, tip.1.round() as i32),
        team.light,
    );
    paint::put(canvas, x, y - 15, team.shine);
}

/// A downward chevron over a contest's target.
fn chevron(canvas: &mut Canvas, at: (i32, i32), team: Ramp) {
    let (x, y) = at;
    for (i, row) in (0..5).enumerate() {
        let half = 5 - i as i32;
        paint::hline(canvas, x - half - 1, y + row, 3, team.edge);
        paint::hline(canvas, x + half - 1, y + row, 3, team.edge);
        paint::hline(canvas, x - half, y + row, 1, team.shine);
        paint::hline(canvas, x + half, y + row, 1, team.light);
    }
    paint::rect(canvas, x - 1, y + 5, 3, 2, team.edge);
    paint::put(canvas, x, y + 5, team.light);
}

/// A creature's shadow, Hill's two flat ellipses, and its team's ring round its feet: whole while
/// it stands steady, broken while it is off balance, doubled while it holds.
fn footing(
    canvas: &mut Canvas,
    view: &View,
    slot: Slot,
    f: &Figure,
    chosen: bool,
    now: f32,
    still: bool,
) {
    let team = TEAM[slot.side.index()];
    let (fx, fy) = view.pixel(f.at.0, f.at.1);
    let alpha = (f.opacity * 255.0).round() as u8;
    paint::ellipse(canvas, fx + 1, fy, 10, 3, rgba(0x2c3a24, alpha / 4));
    paint::ellipse(canvas, fx + 1, fy, 7, 2, rgba(0x2c3a24, alpha / 6));
    let ring = if chosen { CREAM } else { team.base };
    let gappy = f.off_balance && !f.out;
    ellipse_ring(
        canvas,
        fx,
        fy,
        13,
        5,
        paint::faded(team.edge, alpha / 2),
        gappy,
        1,
    );
    ellipse_ring(
        canvas,
        fx,
        fy - 1,
        13,
        5,
        paint::faded(ring, alpha),
        gappy,
        0,
    );
    if f.holding {
        ellipse_ring(
            canvas,
            fx,
            fy - 1,
            16,
            7,
            paint::faded(team.light, alpha),
            false,
            0,
        );
        // Four little studs round the hold, like a shield's rim.
        for k in 0..4 {
            let a = k as f32 * FRAC_PI_2 + 0.78;
            let p = (
                fx + (a.cos() * 16.0).round() as i32,
                fy - 1 + (a.sin() * 7.0).round() as i32,
            );
            paint::rect(
                canvas,
                p.0 - 1,
                p.1 - 1,
                2,
                2,
                paint::faded(team.shine, alpha),
            );
        }
    }
    if chosen {
        let grow = if still { 0.0 } else { (now * 3.2).sin() * 1.0 };
        ellipse_ring(
            canvas,
            fx,
            fy - 1,
            (17.0 + grow).round() as i32,
            (7.0 + grow * 0.5).round() as i32,
            paint::faded(CREAM, 150),
            true,
            0,
        );
    }
}

/// Off balance, it sways a pixel either way until the turn is over.
fn sway(f: &Figure, now: f32, still: bool) -> i32 {
    if f.off_balance && !f.out && !still {
        ((now * 5.0).sin() * 1.4).round() as i32
    } else {
        0
    }
}

/// Where a creature's picture is drawn this moment: at its feet, swaying or slumped as it is.
fn placed(view: &View, f: &Figure, look: &Look, now: f32, still: bool) -> (i32, i32) {
    let (ox, oy) = frame_origin(view, f, look);
    if f.pose == Pose::Dizzy {
        (ox, oy + 4)
    } else {
        (ox + sway(f, now, still), oy)
    }
}

/// Over a creature's head: its composure, and the swirl while it is off balance.
fn tags(
    canvas: &mut Canvas,
    view: &View,
    slot: Slot,
    f: &Figure,
    look: &Look,
    now: f32,
    still: bool,
) {
    let (ox, oy) = frame_origin(view, f, look);
    let top = head_point(ox, oy);
    pips(canvas, top, f.composure, TEAM[slot.side.index()]);
    if f.off_balance {
        swirl(canvas, (top.0 + sway(f, now, still), top.1 - 6), now, still);
    }
}

fn head_point(ox: i32, oy: i32) -> (i32, i32) {
    (ox + FRAME_SIZE as i32 / 2, oy + 6)
}

/// Off balance: a little wobbly swirl over its head, cream, that keeps turning till the turn ends.
fn swirl(canvas: &mut Canvas, (x, y): (i32, i32), now: f32, still: bool) {
    let turn = if still { 0.0 } else { now * 5.0 };
    for k in 0..7 {
        let a = turn + k as f32 * 0.8;
        let r = 1.5 + k as f32 * 0.5;
        let p = (
            x + (a.cos() * r).round() as i32,
            y + (a.sin() * r * 0.6).round() as i32,
        );
        paint::put(
            canvas,
            p.0,
            p.1,
            if k > 4 { rgb(0xfff1c4) } else { rgb(0xc8b8a0) },
        );
    }
}

/// Composure, as three little pips: filled in the team's colour, empty as cream rings.
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
            if full { team.edge } else { rgb(0x6e776c) },
        );
        if full {
            paint::put(canvas, px, y, team.shine);
            paint::put(canvas, px + 1, y + 1, team.base);
        } else {
            paint::put(canvas, px, y, rgb(0xfff8e4));
        }
    }
}

/// An ellipse's outline, whole or broken into dashes.
#[allow(clippy::too_many_arguments)]
fn ellipse_ring(
    canvas: &mut Canvas,
    cx: i32,
    cy: i32,
    rx: i32,
    ry: i32,
    color: Rgba,
    dashed: bool,
    salt: i32,
) {
    let steps = 56;
    let mut last: Option<(i32, i32)> = None;
    for i in 0..=steps {
        if dashed && (i + salt) % 7 >= 4 {
            last = None;
            continue;
        }
        let a = i as f32 / steps as f32 * TAU;
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

/// Where the Handler just gave an order: a gold diamond spreading out from the tile, and a puff
/// of grass.
fn poke(canvas: &mut Canvas, view: &View, tile: Tile, age: f32, still: bool) {
    let life = 0.45;
    if age > life {
        return;
    }
    let s = age / life;
    let fade = ((1.0 - s) * 230.0) as u8;
    let inset = if still { 0.05 } else { 0.25 - 0.3 * s };
    brackets(canvas, view, tile, inset, 1.0, paint::faded(GOLD, fade));
    if !still {
        let (x, y) = view.centre(tile);
        for k in 0..5 {
            let a = k as f32 / 5.0 * TAU + 0.4;
            let r = 4.0 + s * 9.0;
            let p = (
                x + (a.cos() * r).round() as i32,
                y + (a.sin() * r * 0.5 - s * 6.0).round() as i32,
            );
            paint::put(canvas, p.0, p.1, paint::faded(garden::GRASS.shine, fade));
        }
    }
}

fn burst(canvas: &mut Canvas, view: &View, effect: &crate::playback::Effect, now: f32) {
    let (cx, cy) = view.centre(effect.tile);
    let age = effect.age;
    let fade = ((1.0 - age / 0.7).clamp(0.0, 1.0) * 255.0) as u8;
    match effect.burst {
        Burst::Hit => {
            // A cartoon "pow": a burst of rays round a cream star.
            let r = 4.0 + age * 14.0;
            let (x, y) = (cx, cy - 22);
            for k in 0..8 {
                let a = k as f32 / 8.0 * TAU + 0.3;
                let long = if k % 2 == 0 { 1.0 } else { 0.65 };
                let p0 = (
                    x + (a.cos() * r * 0.4) as i32,
                    y + (a.sin() * r * 0.4) as i32,
                );
                let p1 = (
                    x + (a.cos() * r * long) as i32,
                    y + (a.sin() * r * long) as i32,
                );
                paint::line(canvas, p0, p1, rgba(0xfff3b0, fade));
            }
            star(canvas, (x, y), rgba(0xffffff, fade));
        }
        Burst::Shield => {
            let (x, y) = (cx, cy - 18);
            let r = 10 + (age * 6.0) as i32;
            for k in -6..=6 {
                let a = FRAC_PI_2 + k as f32 * 0.22;
                let p = (
                    x + (a.cos() * r as f32) as i32,
                    y - (a.sin() * r as f32) as i32,
                );
                paint::rect(canvas, p.0, p.1, 2, 2, rgba(0xbfe6ff, fade));
            }
            paint::put(canvas, x - 1, y - r - 1, rgba(0xffffff, fade));
        }
        Burst::Braced { partner } => {
            // Two holders turning a flank away together: a bright thread between them at
            // shoulder height, and a twinkle at each end.
            let (px, py) = view.centre(partner);
            let (a, b) = ((cx, cy - 20), (px, py - 20));
            let steps = ((b.0 - a.0).abs().max((b.1 - a.1).abs()) / 2).max(1);
            let shimmer = (now * 12.0) as i32;
            for i in 0..=steps {
                if (i + shimmer) % 3 == 0 {
                    continue;
                }
                let x = a.0 + (b.0 - a.0) * i / steps;
                let y = a.1 + (b.1 - a.1) * i / steps;
                paint::put(canvas, x, y, rgba(0xdff3ff, fade));
            }
            star(canvas, a, rgba(0xffffff, fade));
            star(canvas, b, rgba(0xffffff, fade));
        }
        Burst::Broken => {
            // A hold broken from two sides: its shield flies apart in shards.
            let (x, y) = (cx, cy - 18);
            for k in 0..7 {
                let a = FRAC_PI_2 + (k as f32 - 3.0) * 0.4;
                let out = 10.0 + age * 18.0;
                let drop = age * age * 30.0;
                let p = (
                    x + (a.cos() * out) as i32,
                    y - (a.sin() * out * 0.8) as i32 + drop as i32,
                );
                paint::rect(canvas, p.0, p.1, 2, 1, rgba(0xbfe6ff, fade));
                paint::put(canvas, p.0, p.1 - 1, rgba(0xffffff, fade));
            }
            if age < 0.25 {
                for k in -2..=2 {
                    paint::put(canvas, x + k, y - 12 + k.abs(), rgba(0xffffff, 230));
                }
            }
        }
        Burst::Hesitate => {
            // Hung back: a bead of worry beside its head, and three dots.
            let (x, y) = (cx + 9, cy - 34 + (age * 4.0) as i32);
            paint::put(canvas, x, y - 1, rgba(0xdff3ff, fade));
            paint::rect(canvas, x - 1, y, 3, 2, rgba(0x9fd0f0, fade));
            paint::put(canvas, x - 1, y, rgba(0xffffff, fade));
            let dots = ((age * 5.0) as i32).min(3);
            for k in 0..dots {
                paint::rect(canvas, cx - 4 + k * 3, cy - 40, 2, 2, rgba(0xfff8e4, fade));
            }
        }
        Burst::Scuffle | Burst::Bump => {
            for (k, (dx, dy)) in [(-6, -4), (5, -6), (0, -10), (-3, -14), (7, -12)]
                .iter()
                .enumerate()
            {
                if effect.burst == Burst::Bump && k > 2 {
                    break;
                }
                let grow = (age * 6.0) as i32;
                let (x, y) = (cx + dx + dx.signum() * grow, cy + dy);
                let r = 2 + grow / 2;
                paint::ellipse(canvas, x, y, r, r, rgba(0xe8e0cc, fade / 2 + 40));
                paint::put(canvas, x - r / 2, y - r / 2, rgba(0xfffcf2, fade / 2 + 40));
            }
        }
        Burst::Stars => {
            for k in 0..3 {
                let a = age * 7.0 + k as f32 * TAU / 3.0;
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
                let a = k as f32 * FRAC_PI_2 + age * 3.0;
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

/// One of the board's marks, for the key on the page beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Holding,
    OffBalance,
    Move,
    Contest,
    Beacon,
}

impl Key {
    pub const ALL: [Self; 5] = [
        Self::Move,
        Self::Holding,
        Self::Contest,
        Self::OffBalance,
        Self::Beacon,
    ];

    pub const fn words(self) -> &'static str {
        match self {
            Self::Move => "Moving there",
            Self::Holding => "Holding its ground",
            Self::Contest => "Contesting",
            Self::OffBalance => "Off balance this turn",
            Self::Beacon => "The beacon",
        }
    }

    /// The mark itself, painted as the board paints it, in the left team's colours.
    pub fn picture(self) -> Canvas {
        let mut canvas = Canvas::new(34, 24);
        let team = TEAM[0];
        let (cx, cy) = (17, 18);
        match self {
            Self::Holding => {
                ellipse_ring(&mut canvas, cx, cy, 11, 4, team.base, false, 0);
                ellipse_ring(&mut canvas, cx, cy, 14, 5, team.light, false, 0);
                for k in 0..4 {
                    let a = k as f32 * FRAC_PI_2 + 0.78;
                    let p = (
                        cx + (a.cos() * 14.0).round() as i32,
                        cy + (a.sin() * 5.0).round() as i32,
                    );
                    paint::rect(&mut canvas, p.0 - 1, p.1 - 1, 2, 2, team.shine);
                }
            }
            Self::OffBalance => {
                ellipse_ring(&mut canvas, cx, cy, 12, 4, team.base, true, 0);
                swirl(&mut canvas, (cx, cy - 10), 0.0, true);
            }
            Self::Move => {
                for i in 0..4 {
                    let x = 4 + i * 4;
                    let y = cy - i + if i % 2 == 0 { 1 } else { -1 };
                    paint::rect(&mut canvas, x, y, 2, 1, team.edge);
                    paint::put(&mut canvas, x, y, team.shine);
                }
                pennant(&mut canvas, (22, 20), team, 0.0, true);
            }
            Self::Contest => {
                for k in 0..12 {
                    if k % 3 == 2 {
                        continue;
                    }
                    let a = k as f32 / 12.0 * TAU;
                    let p = (
                        cx + (a.cos() * 12.0).round() as i32,
                        cy + (a.sin() * 4.0).round() as i32,
                    );
                    paint::put(&mut canvas, p.0, p.1, team.shine);
                    paint::put(&mut canvas, p.0 + 1, p.1, team.light);
                }
                chevron(&mut canvas, (cx, 3), team);
            }
            Self::Beacon => {
                let stone = garden::STONE;
                paint::ellipse(&mut canvas, cx, cy, 11, 4, stone.edge);
                paint::ellipse(&mut canvas, cx, cy, 10, 3, stone.light);
                paint::ellipse(&mut canvas, cx, cy, 6, 2, garden::BRASS.base);
                paint::ellipse(&mut canvas, cx, cy, 4, 1, rgb(0x3a2a24));
                for (dx, h) in [(-2, 3), (-1, 6), (0, 8), (1, 6), (2, 3)] {
                    for up in 0..h {
                        let c = if up < h / 3 {
                            rgb(0xfff1c4)
                        } else if up < h * 2 / 3 {
                            team.shine
                        } else {
                            team.light
                        };
                        paint::put(&mut canvas, cx + dx, cy - 1 - up, c);
                    }
                }
            }
        }
        canvas
    }
}
