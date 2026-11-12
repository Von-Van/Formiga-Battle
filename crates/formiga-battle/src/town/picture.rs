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
//!   standing with its foot's middle at `x, y`, or the empty stand at `sx, sy`.

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
    /// What stands in front of everyone, laid over last.
    front: Canvas,
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
                _ => return Err(format!("line {}: {line:?} is not a piece", n + 1)),
            }
        }
        let ground = ground.ok_or("no background")?;
        let mut front_layer = front.unwrap_or_else(|| Canvas::new(ground.width(), ground.height()));
        for (pic, x, y) in &overs {
            over(&mut front_layer, pic, *x, *y);
        }
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
        })
    }

    /// The pieces drawn among the people, with the cups won on their sideboards, in the order
    /// they are written (pieces on a piece come after it).
    pub fn pieces(&self, won: [bool; 4]) -> Vec<Piece> {
        let mut out = self.pieces.clone();
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

    /// What stands in front of everyone.
    pub fn front(&self) -> &Canvas {
        &self.front
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
        let none = office.pieces([false; 4]);
        let one = office.pieces([true, false, false, false]);
        assert_eq!(none.len(), one.len());
        let differs = none
            .iter()
            .zip(&one)
            .filter(|(a, b)| a.picture != b.picture)
            .count();
        assert_eq!(differs, 1);
    }
}
