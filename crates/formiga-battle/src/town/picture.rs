//! A place as it is seen, from Jake's art and the art pack's layout guide: its background, with
//! the shadows, rugs and wall pieces laid on it; then the furniture, buildings and trees and
//! everyone in the place, back to front by where their feet are; then whatever stands in front
//! of them (the place's foreground, the tops of trees and lamps, the near edge of a desk).
//!
//! Where each piece stands is in the place's `.props` file in `town/`, written by
//! `scripts/town_art.py` from the guide:
//!
//! - `background <file>` and `foreground <file>`: the place's two whole pictures;
//! - `floor <file> <x> <y>` and `wall <file> <x> <y>`: pieces laid on the background;
//! - `sort <file> <x> <y> <sort> <shadow> <opacity> <over> <rows>`: a piece drawn among the
//!   people at depth `sort` (its feet, in the place's pixels), with its shadow (`-` for none) laid
//!   on the floor at `opacity` of 255, and what of it stands in front of people: a picture of its
//!   own (`over`), or its top `rows` rows;
//! - `cup <slot> <x> <y> <sort> <stand> <sx> <sy>`: a place for a cup on a sideboard: the won cup
//!   standing with its foot's middle at `x, y`, or the empty stand at `sx, sy`;
//! - `train <x> <y> <sort> <front> <loop> <still> <sx> <sy>`: the office habitat's little train,
//!   running round `loop` (its points from the habitat's corner at `x, y`, as `x,y;x,y;…`) at
//!   depth `sort`, and in front of people too if `front` is 1; with reduced motion it stands at
//!   the platform, the picture `still` at `sx, sy`.

use super::art;
use super::map::Place;
use super::sheet::{CELL, FEET, decode};
use formiga_art::{Canvas, Rgba, paint};
use std::collections::HashMap;

/// A tile's size, in pixels.
pub const TILE: i32 = 32;
/// How much of a place the window shows at once, in pixels: a room nearly whole, the campus in
/// part.
pub const VIEW: (i32, i32) = (416, 234);
/// The shade every shadow is laid in.
const SHADE: Rgba = Rgba::new(0x23, 0x14, 0x20, 255);
/// A cup on the sideboard: a cell of `trophy-cups.png`.
const CUP: (i32, i32) = (24, 32);
/// How fast the habitat's train runs round its loop, in pixels a second, and how often its
/// wheels turn a frame.
const TRAIN_SPEED: f32 = 12.0;
const TRAIN_FRAMES_PER_SECOND: f32 = 8.0;
/// The lamps that flicker, by their lantern's picture.
const LAMP: &str = "prop-campus-lamp-post-fg.png";
const NOTICE_BOARD: &str = "prop-campus-notice-board.png";

/// A stretch of the train's loop: from, to, and how long it is.
type Leg = ((i32, i32), (i32, i32), f32);

/// The office habitat's little train.
#[derive(Clone, Debug)]
struct Train {
    corner: (i32, i32),
    points: Vec<(i32, i32)>,
    sort: i32,
    front: bool,
    across: Canvas,
    upright: Canvas,
    still: (Canvas, i32, i32),
    /// Where among the pieces it is drawn.
    at: usize,
}

impl Train {
    /// The train now: its picture and where its top-left corner goes.
    fn now(&self, now: f32, still: bool) -> (Canvas, i32, i32) {
        if still || self.points.len() < 2 {
            return self.still.clone();
        }
        let n = self.points.len();
        let legs: Vec<Leg> = (0..n)
            .map(|i| {
                let (a, b) = (self.points[i], self.points[(i + 1) % n]);
                let len = (((b.0 - a.0).pow(2) + (b.1 - a.1).pow(2)) as f32).sqrt();
                (a, b, len)
            })
            .collect();
        let total: f32 = legs.iter().map(|l| l.2).sum();
        let mut d = (now * TRAIN_SPEED).rem_euclid(total);
        let (a, b, len) = legs
            .iter()
            .copied()
            .find(|l| {
                if d <= l.2 {
                    true
                } else {
                    d -= l.2;
                    false
                }
            })
            .unwrap_or(legs[0]);
        let t = if len > 0.0 { d / len } else { 0.0 };
        let at = (
            a.0 as f32 + (b.0 - a.0) as f32 * t,
            a.1 as f32 + (b.1 - a.1) as f32 * t,
        );
        let frame = (now * TRAIN_FRAMES_PER_SECOND) as i32 % 4;
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        // Running across, the strip faces east and is turned for west; up and down, the upright
        // strip faces north and is turned for south.
        let (strip, size, anchor, flip_x, flip_y) = if dx.abs() >= dy.abs() {
            (&self.across, (22, 8), (11, 4), dx < 0, false)
        } else {
            (&self.upright, (8, 22), (4, 11), false, dy > 0)
        };
        let mut picture = Canvas::new(size.0 as u32, size.1 as u32);
        for y in 0..size.1 {
            for x in 0..size.0 {
                let (sx, sy) = (frame * size.0 + x, y);
                let (tx, ty) = (
                    if flip_x { size.0 - 1 - x } else { x },
                    if flip_y { size.1 - 1 - y } else { y },
                );
                picture.set(tx, ty, strip.get(sx, sy));
            }
        }
        (
            picture,
            self.corner.0 + at.0.round() as i32 - anchor.0,
            self.corner.1 + at.1.round() as i32 - anchor.1,
        )
    }
}

/// A piece drawn among the people.
#[derive(Clone, Debug)]
pub struct Piece {
    pub picture: Canvas,
    pub x: i32,
    pub y: i32,
    pub sort: i32,
}

#[derive(Clone, Debug)]
struct CupPlace {
    slot: usize,
    foot: (i32, i32),
    sort: i32,
    stand: Canvas,
    stand_at: (i32, i32),
}

/// A place's art, read and laid out.
#[derive(Clone, Debug)]
pub struct Scenery {
    /// The background with everything laid on it that never stands in front of anyone.
    pub ground: Canvas,
    pieces: Vec<Piece>,
    cups: Vec<CupPlace>,
    cup_pictures: Option<Canvas>,
    /// What stands in front of everyone, laid over last: below the habitat's train, if it is in
    /// front of people too, and above it.
    front: Canvas,
    front_above: Option<Canvas>,
    train: Option<Train>,
    /// The lanterns that flicker, and the notice board, by their top-left corners.
    pub lamps: Vec<(i32, i32)>,
    pub notice_board: Option<(i32, i32)>,
}

impl Scenery {
    /// A place's scenery from its props file; a picture it names that is not built in is a
    /// broken file, which the tests catch.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut cache: HashMap<String, Canvas> = HashMap::new();
        let mut picture = |name: &str| -> Result<Canvas, String> {
            if let Some(c) = cache.get(name) {
                return Ok(c.clone());
            }
            let c = art::bytes(name)
                .and_then(decode)
                .ok_or_else(|| format!("{name} is not built in"))?;
            cache.insert(name.to_owned(), c.clone());
            Ok(c)
        };
        let mut ground: Option<Canvas> = None;
        let mut front: Option<Canvas> = None;
        let mut pieces = Vec::new();
        let mut cups = Vec::new();
        let mut overs: Vec<(Canvas, i32, i32)> = Vec::new();
        let mut train: Option<Train> = None;
        let mut split: Option<usize> = None;
        let mut lamps = Vec::new();
        let mut notice_board = None;
        for (n, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let words: Vec<&str> = line.split_whitespace().collect();
            let num = |w: &str| {
                w.parse::<i32>()
                    .map_err(|_| format!("line {}: {w:?} is not a number", n + 1))
            };
            match words.as_slice() {
                ["background", file] => ground = Some(picture(file)?),
                ["foreground", file] => {
                    let fg = picture(file)?;
                    front = Some(fg);
                }
                [layer @ ("floor" | "wall"), file, x, y] => {
                    let _ = layer;
                    let g = ground
                        .as_mut()
                        .ok_or_else(|| format!("line {}: a piece before the background", n + 1))?;
                    over(g, &picture(file)?, num(x)?, num(y)?);
                }
                ["sort", file, x, y, sort, shadow, opacity, top, rows] => {
                    let (x, y) = (num(x)?, num(y)?);
                    if *shadow != "-" {
                        let g = ground.as_mut().ok_or_else(|| {
                            format!("line {}: a piece before the background", n + 1)
                        })?;
                        let mask = picture(shadow)?;
                        let alpha = num(opacity)?.clamp(0, 255) as u8;
                        let tint = Rgba::new(SHADE.r, SHADE.g, SHADE.b, alpha);
                        for my in 0..mask.height() as i32 {
                            for mx in 0..mask.width() as i32 {
                                if mask.get(mx, my).a > 0 {
                                    paint::put(g, x + mx, y + my, tint);
                                }
                            }
                        }
                    }
                    let art = picture(file)?;
                    if *top == LAMP {
                        lamps.push((x, y));
                    }
                    if *file == NOTICE_BOARD {
                        notice_board = Some((x, y));
                    }
                    if *top != "-" {
                        overs.push((picture(top)?, x, y));
                    } else {
                        let rows = num(rows)?;
                        if rows > 0 {
                            let mut part = Canvas::new(art.width(), rows as u32);
                            for py in 0..rows {
                                for px in 0..art.width() as i32 {
                                    part.set(px, py, art.get(px, py));
                                }
                            }
                            overs.push((part, x, y));
                        }
                    }
                    pieces.push(Piece {
                        picture: art,
                        x,
                        y,
                        sort: num(sort)?,
                    });
                }
                ["cup", slot, x, y, sort, stand, sx, sy] => cups.push(CupPlace {
                    slot: num(slot)? as usize,
                    foot: (num(x)?, num(y)?),
                    sort: num(sort)?,
                    stand: picture(stand)?,
                    stand_at: (num(sx)?, num(sy)?),
                }),
                ["train", x, y, sort, front, points, still, sx, sy] => {
                    let points = points
                        .split(';')
                        .map(|p| {
                            let (a, b) = p.split_once(',').unwrap_or((p, "0"));
                            Ok((num(a)?, num(b)?))
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    let front = *front == "1";
                    if front {
                        split = Some(overs.len());
                    }
                    train = Some(Train {
                        corner: (num(x)?, num(y)?),
                        points,
                        sort: num(sort)?,
                        front,
                        across: picture("prop-habitat-train.png")?,
                        upright: picture("prop-habitat-train-vertical.png")?,
                        still: (picture(still)?, num(sx)?, num(sy)?),
                        at: pieces.len(),
                    });
                }
                _ => return Err(format!("line {}: {line:?} is not a piece", n + 1)),
            }
        }
        let ground = ground.ok_or("no background")?;
        let mut front_layer = front.unwrap_or_else(|| Canvas::new(ground.width(), ground.height()));
        let split = split.unwrap_or(overs.len());
        for (pic, x, y) in &overs[..split] {
            over(&mut front_layer, pic, *x, *y);
        }
        let front_above = (split < overs.len()).then(|| {
            let mut above = Canvas::new(ground.width(), ground.height());
            for (pic, x, y) in &overs[split..] {
                over(&mut above, pic, *x, *y);
            }
            above
        });
        let cup_pictures = if cups.is_empty() {
            None
        } else {
            Some(picture("trophy-cups.png")?)
        };
        Ok(Self {
            ground,
            pieces,
            cups,
            cup_pictures,
            front: front_layer,
            front_above,
            train,
            lamps,
            notice_board,
        })
    }

    /// The pieces drawn among the people, with the cups won on their sideboards, in the order
    /// they are written (pieces on a piece come after it).
    pub fn pieces(&self, won: [bool; 4], now: f32, still: bool) -> Vec<Piece> {
        let mut out = self.pieces.clone();
        if let Some(train) = &self.train {
            let (picture, x, y) = train.now(now, still);
            let piece = Piece {
                picture,
                x,
                y,
                sort: train.sort,
            };
            out.insert(train.at.min(out.len()), piece);
        }
        for cup in &self.cups {
            let slot = cup.slot.min(3);
            let picture = match (&self.cup_pictures, won[slot]) {
                (Some(strip), true) => {
                    let mut cell = Canvas::new(CUP.0 as u32, CUP.1 as u32);
                    for y in 0..CUP.1 {
                        for x in 0..CUP.0 {
                            cell.set(x, y, strip.get(slot as i32 * CUP.0 + x, y));
                        }
                    }
                    Piece {
                        picture: cell,
                        x: cup.foot.0 - CUP.0 / 2,
                        y: cup.foot.1 - CUP.1,
                        sort: cup.sort,
                    }
                }
                _ => Piece {
                    picture: cup.stand.clone(),
                    x: cup.stand_at.0,
                    y: cup.stand_at.1,
                    sort: cup.sort,
                },
            };
            out.push(picture);
        }
        out
    }

    /// Lays what stands in front of everyone over `world`, with the habitat's train among it
    /// when its desk's near edge stands in front of people.
    pub fn lay_front(&self, world: &mut Canvas, now: f32, still: bool) {
        over(world, &self.front, 0, 0);
        if let Some(train) = self.train.as_ref().filter(|t| t.front) {
            let (picture, x, y) = train.now(now, still);
            over(world, &picture, x, y);
        }
        if let Some(above) = &self.front_above {
            over(world, above, 0, 0);
        }
    }
}

/// `top` laid over `canvas` with its top-left corner at `(x, y)`.
pub fn over(canvas: &mut Canvas, top: &Canvas, x: i32, y: i32) {
    for ty in 0..top.height() as i32 {
        for tx in 0..top.width() as i32 {
            let p = top.get(tx, ty);
            if p.a > 0 {
                paint::put(canvas, x + tx, y + ty, p);
            }
        }
    }
}

/// Where a frame's top-left corner goes for feet on a tile, in the place's pixels: feet stand
/// near a top-down tile's foot.
fn frame_origin(feet: (f32, f32)) -> (i32, i32) {
    (
        (feet.0 * TILE as f32).round() as i32 + TILE / 2 - CELL / 2,
        (feet.1 * TILE as f32).round() as i32 + TILE - 4 - FEET,
    )
}

/// The depth someone is drawn at, by their feet: the pixel row the feet stand on.
pub fn feet_depth(feet: (f32, f32)) -> i32 {
    frame_origin(feet).1 + FEET
}

/// The part of a place the window shows, in its pixels: centred on the Handler and kept inside
/// the place, or the whole place centred when it is smaller than the window.
pub fn camera(place: &Place, feet: (f32, f32)) -> (i32, i32) {
    let (w, h) = (place.width * TILE, place.height * TILE);
    let axis = |size: i32, view: i32, at: f32| {
        if size <= view {
            (size - view) / 2
        } else if size - view <= TILE {
            // A room a few pixels taller than the window keeps its back wall in sight, and slides
            // down to its door only as the Handler walks the last rows towards it.
            let rows = size / TILE;
            let t = (at - (rows - 3) as f32).clamp(0.0, 1.0);
            ((size - view) as f32 * t).round() as i32
        } else {
            let centre = (at * TILE as f32) as i32 + TILE / 2;
            (centre - view / 2).clamp(0, size - view)
        }
    };
    (axis(w, VIEW.0, feet.0), axis(h, VIEW.1, feet.1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::town::map::PlaceId;

    #[test]
    fn a_room_fits_the_window_across_and_the_campus_scrolls_inside_itself() {
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
    fn a_room_shows_its_back_wall_until_the_handler_nears_the_door() {
        let office = PlaceId::Office.place();
        assert_eq!(camera(&office, (6.0, 3.0)).1, 0);
        let (_, at_door) = camera(&office, (6.0, 6.0));
        assert_eq!(at_door, office.height * TILE - VIEW.1);
    }

    #[test]
    fn the_habitat_train_runs_round_its_loop_and_stands_still_with_reduced_motion() {
        let office = PlaceId::Office.scenery();
        let train = office.train.as_ref().expect("the office has its train");
        let places: std::collections::BTreeSet<(i32, i32)> = (0..40)
            .map(|i| {
                let (_, x, y) = train.now(i as f32 * 0.5, false);
                (x, y)
            })
            .collect();
        assert!(places.len() > 20, "it moves");
        let (_, x, y) = train.now(3.0, true);
        assert_eq!((x, y), (train.still.1, train.still.2));
    }

    #[test]
    fn every_place_has_its_art_the_size_of_its_tiles() {
        for id in PlaceId::ALL {
            let place = id.place();
            let scenery = id.scenery();
            assert_eq!(
                scenery.ground.width() as i32,
                place.width * TILE,
                "{}",
                id.key()
            );
            assert_eq!(
                scenery.ground.height() as i32,
                place.height * TILE,
                "{}",
                id.key()
            );
            assert!(
                scenery.ground.pixels().iter().all(|p| p.a == 255),
                "{}",
                id.key()
            );
        }
    }

    #[test]
    fn a_won_cup_stands_where_its_empty_stand_was() {
        let office = PlaceId::Office.scenery();
        let none = office.pieces([false; 4], 0.0, true);
        let one = office.pieces([true, false, false, false], 0.0, true);
        assert_eq!(none.len(), one.len());
        let differs = none
            .iter()
            .zip(&one)
            .filter(|(a, b)| a.picture != b.picture)
            .count();
        assert_eq!(differs, 1);
    }
}
