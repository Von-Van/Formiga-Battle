//! The Handler's things, painted as the notebook's other pictures are: at its own pixel, lit from
//! the upper left, edges a darker shade of their own colour, grain from `paint::noise`. A pennant
//! on its cane stirring in a tuft of grass; sample tiles to choose a pennant from, as a tailor's
//! swatch card; a brass nameplate for a Handler's title; a ribbon rosette for each cup; a trio's
//! team picture, three of them on a strip of lawn by their pennant, pasted in with tape; and the
//! banner a match opens under. The companions are drawn only through their own looks. Words on
//! them are egui's, lettered on top where each says.

use crate::garden::{BRASS, GRASS, PLANK};
use crate::identity::{FIELDS, SHAPES, Style, TRIMS};
use crate::look::{Look, Pose};
use crate::pennant::{CLOTH_H, CLOTH_W, cloth};
use formiga_art::paint::{self, Ramp, rgb, rgba};
use formiga_art::{Canvas, FRAME_SIZE, Rgba};

const CARD: Ramp = Ramp::new(0xb8a682, 0xe2d3ae, 0xf6ecd0, 0xfcf5e2, 0xfffcf2);
const SKY_HIGH: Rgba = rgb(0xa9d6ea);
const SKY_LOW: Rgba = rgb(0xe4f3ec);
const SHADOW: Rgba = rgba(0x000000, 60);
const SOFT_SHADOW: Rgba = rgba(0x24452b, 60);
const TAPE: Rgba = rgba(0xfffaf0, 150);
/// The notebook's green cloth, as its ribbons are.
const RIBBON: Ramp = Ramp::new(0x10281f, 0x173a2e, 0x1c4137, 0x285a4b, 0x3b7563);
const THREAD: Rgba = rgb(0xcdb17a);
const THREAD_DARK: Rgba = rgb(0xb2955f);
const IRON: Rgba = rgb(0x48434a);

/// A rectangle of a picture, in its pixels: left, top, width, height.
pub type Area = (i32, i32, i32, i32);

/// Which of a pennant's four choices a tile shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Shape,
    Field,
    Symbol,
    Trim,
}

/// How large a sample tile is, its shadow included.
pub const TILE: i32 = 22;

/// The cloth stirring: each column lifted or dropped a little, more toward the fly, by a wave
/// running out along it. At `t` 0 it hangs as it was cut.
fn stirred(cloth: &Canvas, t: f32) -> Canvas {
    let (w, h) = (cloth.width() as i32, cloth.height() as i32);
    let mut out = Canvas::new(w as u32, h as u32 + 4);
    for x in 0..w {
        let reach = x as f32 / w as f32;
        let dy = if t == 0.0 {
            0
        } else {
            ((x as f32 * 0.42 - t * 4.5).sin() * 1.6 * reach).round() as i32
        };
        // Where the cloth dips, it is turned a little away from the light.
        let dim = dy > 0;
        for y in 0..h {
            let c = cloth.get(x, y);
            if c.a == 0 {
                continue;
            }
            let c = if dim { paint::darker(c, 0.12) } else { c };
            out.set(x, y + 2 + dy, c);
        }
    }
    out
}

/// A pennant flying from a cane planted in a tuft of grass, `t` seconds into its stirring.
pub const STANDARD_W: u32 = CLOTH_W as u32 + 10;
pub const STANDARD_H: u32 = CLOTH_H as u32 + 26;

pub fn standard(style: Style, t: f32) -> Canvas {
    let (w, h) = (STANDARD_W as i32, STANDARD_H as i32);
    let mut canvas = Canvas::new(w as u32, h as u32);
    let foot = h - 4;
    let x = 4;
    // The tuft it stands in, and its shadow.
    paint::hline(&mut canvas, x - 3, foot + 1, 10, SOFT_SHADOW);
    for k in 0..9 {
        let gx = x - 4 + k;
        let tall = 2 + (paint::noise(gx, foot, 811) % 3) as i32;
        paint::vline(&mut canvas, gx, foot - tall, tall, GRASS.light);
        paint::put(&mut canvas, gx, foot - tall, GRASS.shine);
        paint::put(&mut canvas, gx, foot, GRASS.shadow);
    }
    // The cane, with a brass cap.
    paint::vline(&mut canvas, x, 3, foot - 4, PLANK.shadow);
    paint::vline(&mut canvas, x + 1, 3, foot - 4, PLANK.light);
    paint::rect(&mut canvas, x - 1, 1, 4, 2, BRASS.base);
    paint::hline(&mut canvas, x - 1, 1, 4, BRASS.shine);
    paint::put(&mut canvas, x + 2, 2, BRASS.edge);
    // The cloth, tied on at the top.
    let flag = stirred(&cloth(style), t);
    paint::blit(&mut canvas, &flag, x + 2, 1);
    paint::put(&mut canvas, x + 2, 4, BRASS.light);
    paint::put(&mut canvas, x + 2, 3 + CLOTH_H - 1, BRASS.light);
    canvas
}

/// The frames a stirring pennant loops through, so each is painted once.
pub const STIR_FRAMES: usize = 8;

/// The moment of the loop frame `k` shows.
pub fn stir_at(k: usize) -> f32 {
    0.01 + k as f32 / STIR_FRAMES as f32 * std::f32::consts::TAU / 4.5
}

/// A small card of cloth, the tile a pennant's part is chosen from: `index` of the part's list,
/// shown as it would be on the Handler's pennant. A chosen tile is pinned with brass; one
/// pointed at is lifted.
pub fn tile(style: Style, part: Part, index: usize, chosen: bool, lit: bool) -> Canvas {
    let mut canvas = Canvas::new(TILE as u32, TILE as u32);
    let lift = i32::from(lit && !chosen);
    let (x, y, s) = (1, 1 - lift, TILE - 4);
    paint::rect(&mut canvas, x + 2, 3, s, s, SHADOW);
    let card = if chosen { BRASS } else { CARD };
    paint::rect(&mut canvas, x, y, s, s, card.edge);
    paint::rect(&mut canvas, x + 1, y + 1, s - 2, s - 2, card.light);
    paint::hline(&mut canvas, x + 1, y + 1, s - 2, card.shine);
    let inner = (x + 2, y + 2, s - 4, s - 4);
    let mut shown = style;
    match part {
        Part::Shape => shown.shape = index,
        Part::Field => shown.field = index,
        Part::Symbol => shown.symbol = index,
        Part::Trim => shown.trim = index,
    }
    match part {
        // The cut, small, in the Handler's cloth.
        Part::Shape => {
            let ramp = shown.ramp();
            let (ix, iy, iw, ih) = inner;
            paint::rect(&mut canvas, ix, iy, iw, ih, CARD.base);
            let mid = iy + ih / 2;
            for py in iy + 2..iy + ih - 2 {
                for px in ix + 2..ix + iw - 1 {
                    let off = (py - mid).abs();
                    let fly = px - ix - 2;
                    let reach = iw - 4;
                    let inside = match index % SHAPES.len() {
                        0 => off * reach <= (ih / 2 - 2) * (reach - fly),
                        1 => fly <= reach - (ih / 2 - 2) + off,
                        _ => true,
                    };
                    if inside {
                        canvas.set(px, py, if py == iy + 2 { ramp.light } else { ramp.base });
                    }
                }
            }
            paint::vline(&mut canvas, ix + 1, iy + 1, ih - 1, PLANK.shadow);
        }
        // A woven square of the cloth.
        Part::Field => {
            let ramp = FIELDS[index % FIELDS.len()].2;
            let (ix, iy, iw, ih) = inner;
            for py in iy..iy + ih {
                for px in ix..ix + iw {
                    let weave = (px + py) % 2 == 0;
                    let c = if px == ix || py == iy {
                        ramp.light
                    } else if px == ix + iw - 1 || py == iy + ih - 1 {
                        ramp.shadow
                    } else if weave {
                        ramp.base
                    } else {
                        paint::mix(ramp.base, ramp.light, 0.35)
                    };
                    canvas.set(px, py, c);
                }
            }
            // A frayed foot, as a cut sample has.
            for px in ix..ix + iw {
                if paint::noise(px, iy, 919).is_multiple_of(3) {
                    canvas.set(px, iy + ih, ramp.shadow);
                }
            }
        }
        // The symbol on a square of the cloth, or the trim round it.
        Part::Symbol | Part::Trim => {
            let full = cloth(Style { shape: 2, ..shown });
            let (ix, iy, iw, ih) = inner;
            let (ox, oy) = match part {
                Part::Symbol => (8 - (iw - 7) / 2, (CLOTH_H - ih) / 2),
                _ => (0, 0),
            };
            for py in 0..ih {
                for px in 0..iw {
                    let c = full.get(ox + px, oy + py);
                    if c.a > 0 {
                        canvas.set(ix + px, iy + py, c);
                    }
                }
            }
            if part == Part::Trim {
                // The sample's cut edges, so only its two finished sides show the trim.
                let ramp = shown.ramp();
                paint::vline(&mut canvas, ix + iw - 1, iy, ih, ramp.base);
                paint::hline(&mut canvas, ix, iy + ih - 1, iw, ramp.base);
                if index.is_multiple_of(TRIMS.len()) {
                    paint::hline(&mut canvas, ix, iy, iw, ramp.edge);
                }
            }
        }
    }
    if chosen {
        // A brass pin through its top.
        let cx = x + s / 2;
        paint::put(&mut canvas, cx, y, BRASS.shine);
        paint::put(&mut canvas, cx - 1, y, BRASS.base);
        paint::put(&mut canvas, cx, y + 1, BRASS.edge);
    }
    canvas
}

/// A brass plate, `width` pixels wide, to letter a Handler's title on: screwed on at each end,
/// lit along its top, with a sunk panel where the words go.
pub const PLATE_H: i32 = 15;

pub fn plate(width: i32) -> Canvas {
    let w = width.max(24);
    let mut canvas = Canvas::new(w as u32, PLATE_H as u32 + 2);
    paint::rect(&mut canvas, 2, 2, w - 2, PLATE_H, SHADOW);
    paint::bevel(&mut canvas, 0, 0, w - 1, PLATE_H, BRASS);
    // The sunk panel.
    paint::hline(&mut canvas, 6, 3, w - 13, BRASS.shadow);
    paint::vline(&mut canvas, 6, 3, PLATE_H - 6, BRASS.shadow);
    paint::hline(&mut canvas, 6, PLATE_H - 4, w - 13, BRASS.shine);
    paint::vline(&mut canvas, w - 8, 3, PLATE_H - 6, BRASS.shine);
    for py in 4..PLATE_H - 4 {
        for px in 7..w - 8 {
            if paint::noise(px, py, 431).is_multiple_of(23) {
                canvas.set(px, py, BRASS.light);
            }
        }
    }
    // A screw at each end, its slot turned this way and that.
    for (sx, slant) in [(3, 1), (w - 5, -1)] {
        let sy = PLATE_H / 2;
        paint::rect(&mut canvas, sx - 1, sy - 1, 3, 3, BRASS.shadow);
        paint::put(&mut canvas, sx - 1, sy - 1, BRASS.shine);
        paint::put(&mut canvas, sx - slant, sy - 1, BRASS.edge);
        paint::put(&mut canvas, sx, sy, BRASS.edge);
        paint::put(&mut canvas, sx + slant, sy + 1, BRASS.edge);
    }
    canvas
}

/// The ribbon colours of the four cups' rosettes, in the order the cups open.
const CUP_RIBBONS: [usize; 4] = [0, 1, 2, 4];

/// A rosette for a cup of the Championship Circuit, `cup` by its place in `rewards::CUPS`: its
/// ribbon in the cup's colour, pleated round a brass button, with two tails. One not yet won is
/// a faint outline pencilled on the page.
pub const ROSETTE_W: u32 = 15;
pub const ROSETTE_H: u32 = 22;

pub fn rosette(cup: usize, won: bool) -> Canvas {
    let mut canvas = Canvas::new(ROSETTE_W, ROSETTE_H);
    let (cx, cy) = (7, 7);
    let ramp = FIELDS[CUP_RIBBONS[cup % CUP_RIBBONS.len()]].2;
    if !won {
        let pencil = rgba(0x6e5a44, 170);
        for k in 0..20 {
            let a = k as f32 / 20.0 * std::f32::consts::TAU;
            if k % 2 == 0 {
                paint::put(
                    &mut canvas,
                    cx + (a.cos() * 6.0).round() as i32,
                    cy + (a.sin() * 6.0).round() as i32,
                    pencil,
                );
            }
        }
        paint::line(&mut canvas, (cx - 2, cy + 7), (cx - 4, cy + 13), pencil);
        paint::line(&mut canvas, (cx + 2, cy + 7), (cx + 4, cy + 13), pencil);
        return canvas;
    }
    // The tails first, behind, each cut in a V at its foot.
    for (dx, lean) in [(-3, -1), (1, 1)] {
        for k in 0..12 {
            let x0 = cx + dx + (k * lean) / 5;
            let y = cy + 3 + k;
            let notch = k >= 10;
            for px in 0..3 {
                if notch && px == 1 {
                    continue;
                }
                let c = if px == 0 { ramp.light } else { ramp.base };
                paint::put(&mut canvas, x0 + px, y, c);
            }
            paint::put(&mut canvas, x0 + 3, y, ramp.edge);
        }
    }
    // The pleated ring, light and dark by turns, edged.
    for py in cy - 7..=cy + 7 {
        for px in cx - 7..=cx + 7 {
            let (dx, dy) = ((px - cx) as f32, (py - cy) as f32);
            let r = (dx * dx + dy * dy).sqrt();
            if r > 6.9 {
                continue;
            }
            let a = dy.atan2(dx);
            let pleat = ((a / std::f32::consts::TAU * 16.0).floor() as i32).rem_euclid(2) == 0;
            let c = if r > 6.0 {
                ramp.edge
            } else if pleat {
                ramp.light
            } else {
                ramp.base
            };
            paint::put(&mut canvas, px, py, c);
        }
    }
    // The brass button in the middle.
    paint::ellipse(&mut canvas, cx, cy, 3, 3, BRASS.edge);
    paint::ellipse(&mut canvas, cx, cy, 2, 2, BRASS.base);
    paint::put(&mut canvas, cx - 1, cy - 1, BRASS.shine);
    paint::put(&mut canvas, cx, cy - 1, BRASS.light);
    canvas
}

/// A strip of ribbon of the notebook's green, with its ends folded back and notched and a stitch
/// along each edge: what a trio's name or a match's opening is lettered on.
pub fn ribbon(canvas: &mut Canvas, (x, y, w, h): Area) {
    for (tx, dir) in [(x - 7, 1), (x + w - 3, -1)] {
        for dy in 0..h {
            for dx in 0..10 {
                let notch =
                    (dy - h / 2).abs() < h / 4 + 1 && ((dir > 0 && dx < 3) || (dir < 0 && dx > 6));
                if !notch {
                    canvas.set(tx + dx, y + 2 + dy, RIBBON.shadow);
                }
            }
        }
    }
    paint::bevel(canvas, x, y, w, h, RIBBON);
    for py in y + 2..y + h - 2 {
        for px in x + 2..x + w - 2 {
            if (px + py) % 2 == 0 {
                paint::put(canvas, px, py, paint::faded(RIBBON.light, 90));
            }
        }
    }
    let mut along = x + 3;
    while along + 2 < x + w - 3 {
        paint::hline(canvas, along, y + 2, 2, THREAD);
        paint::hline(canvas, along, y + h - 3, 2, THREAD_DARK);
        along += 4;
    }
}

/// Where a team picture's ribbon is, for the window to letter the trio's name on.
pub fn team_ribbon(width: i32, height: i32) -> Area {
    let w = (width * 3 / 4).max(40);
    (width / 2 - w / 2, height - 19, w, 14)
}

/// A trio's team picture, `width` by `height`: three of them standing on a strip of lawn under
/// the sky, their pennant planted beside them, on a card taped into the notebook, with a ribbon
/// across its foot for their name. A place whose companion has left the colony is an empty patch
/// of trodden grass. A retired trio's picture has faded.
pub fn team_picture(
    width: i32,
    height: i32,
    style: Style,
    looks: &mut [Option<Look>],
    retired: bool,
) -> Canvas {
    let (w, h) = (width.max(90), height.max(56));
    let mut canvas = Canvas::new(w as u32, h as u32);
    let (cx, cy, cw, ch) = (2, 3, w - 6, h - 9);
    paint::rect(&mut canvas, cx + 2, cy + 2, cw, ch, SHADOW);
    paint::rect(&mut canvas, cx, cy, cw, ch, CARD.edge);
    paint::rect(&mut canvas, cx + 1, cy + 1, cw - 2, ch - 2, CARD.base);
    let (sx, sy, sw, sh) = (cx + 4, cy + 4, cw - 8, ch - 20);
    let ground = sy + sh * 4 / 5;
    for y in sy..sy + sh {
        for x in sx..sx + sw {
            let tone = if y < ground {
                let k = (y - sy) as f32 / (ground - sy).max(1) as f32;
                let jitter = (paint::noise(x, y, 701) % 100) as f32 / 100.0 - 0.5;
                paint::mix(
                    SKY_HIGH,
                    SKY_LOW,
                    ((k * 4.0 + jitter * 0.7).floor() / 4.0).clamp(0.0, 1.0),
                )
            } else {
                match paint::noise(x, y, 709) % 100 {
                    0..=11 => GRASS.shadow,
                    12..=19 => GRASS.shine,
                    _ => GRASS.light,
                }
            };
            canvas.set(x, y, tone);
        }
    }
    for x in sx..sx + sw {
        canvas.set(x, ground, GRASS.edge);
        canvas.set(x, ground + 1, GRASS.shine);
    }
    // Their pennant, on a short cane planted at the left.
    let fx = sx + 3;
    let top = (ground - CLOTH_H - 8).max(sy + 1);
    paint::vline(&mut canvas, fx, top, ground + 3 - top, PLANK.shadow);
    paint::vline(&mut canvas, fx + 1, top, ground + 3 - top, PLANK.light);
    paint::rect(&mut canvas, fx - 1, top - 1, 4, 2, BRASS.base);
    paint::blit(&mut canvas, &cloth(style), fx + 2, top + 1);
    // The three, spaced along the lawn, facing the same way, feet on it.
    let left = fx + CLOTH_W + 4;
    let room = (sx + sw - 2 - left).max(1);
    let n = looks.len().max(1) as i32;
    for (k, look) in looks.iter_mut().enumerate() {
        let x = left + room * (2 * k as i32 + 1) / (2 * n);
        let foot = ground + 3;
        paint::ellipse(&mut canvas, x + 1, foot + 1, 6, 1, SOFT_SHADOW);
        match look {
            Some(look) => {
                let top = foot - look.foot_row;
                let frame = look.frame(Pose::Idle, 0.0, true);
                paint::blit(&mut canvas, frame, x - FRAME_SIZE as i32 / 2, top);
            }
            None => {
                // Trodden grass where one of them used to stand.
                for k in -4..=4 {
                    paint::put(&mut canvas, x + k, foot, GRASS.shadow);
                    if k % 2 == 0 {
                        paint::put(&mut canvas, x + k, foot - 1, GRASS.edge);
                    }
                }
            }
        }
    }
    // Trim what strayed past the scene back to the card.
    for y in 0..h {
        for x in 0..w {
            let in_scene = (sx..sx + sw).contains(&x) && (sy..sy + sh).contains(&y);
            let in_card = x > cx && y > cy && x < cx + cw - 1 && y < cy + ch - 1;
            if in_card && !in_scene {
                let fibre = paint::noise(x, y, 613) % 100;
                canvas.set(x, y, if fibre < 6 { CARD.light } else { CARD.base });
            }
        }
    }
    paint::hline(&mut canvas, sx - 1, sy - 1, sw + 2, CARD.shadow);
    paint::vline(&mut canvas, sx - 1, sy - 1, sh + 2, CARD.shadow);
    paint::hline(&mut canvas, sx - 1, sy + sh, sw + 2, CARD.shine);
    paint::vline(&mut canvas, sx + sw, sy - 1, sh + 2, CARD.shine);
    for (tx, lean) in [(cx - 3, 1), (cx + cw - 13, -1)] {
        for i in 0..16 {
            let y0 = cy - 2 + if lean > 0 { i / 4 } else { 3 - i / 4 };
            paint::vline(&mut canvas, tx + i, y0, 5, TAPE);
        }
    }
    ribbon(&mut canvas, team_ribbon(w, h));
    if retired {
        for y in 0..h {
            for x in 0..w {
                let c = canvas.get(x, y);
                if c.a > 0 {
                    let grey = ((c.r as u32 * 3 + c.g as u32 * 5 + c.b as u32 * 2) / 10) as u8;
                    let faded = paint::mix(c, Rgba::new(grey, grey, grey, c.a), 0.7);
                    canvas.set(x, y, paint::mix(faded, CARD.light, 0.25));
                }
            }
        }
    }
    canvas
}

/// The banner a match opens under, `width` pixels wide: a ribbon strung between two canes, each
/// flying its side's pennant (none for the visitors, who fly the team's colours). Its two halves
/// are where the window letters each side's name.
pub const BANNER_H: i32 = CLOTH_H + 24;

pub fn banner(width: i32, flags: [Option<Style>; 2], team: [Ramp; 2]) -> Canvas {
    let w = width.max(120);
    let mut canvas = Canvas::new(w as u32, BANNER_H as u32);
    let foot = BANNER_H - 2;
    let (left, right) = (6, w - 7);
    // The ribbon, strung between the canes a little below their tops.
    ribbon(&mut canvas, banner_ribbon(w));
    for (side, x) in [(0usize, left), (1, right)] {
        paint::vline(&mut canvas, x, 1, foot - 1, PLANK.shadow);
        paint::vline(&mut canvas, x + 1, 1, foot - 1, PLANK.light);
        paint::rect(&mut canvas, x - 1, 0, 4, 2, BRASS.base);
        paint::hline(&mut canvas, x - 1, 0, 4, BRASS.shine);
        match flags[side] {
            Some(style) => {
                let mut flag = cloth(style);
                if side == 1 {
                    flag = mirrored(&flag);
                }
                let fx = if side == 0 { x + 2 } else { x - CLOTH_W };
                paint::blit(&mut canvas, &flag, fx, 2);
            }
            None => {
                // A plain pennant in the team's colours.
                let tip = if side == 0 { x + 14 } else { x - 13 };
                let base = if side == 0 { x + 2 } else { x - 1 };
                paint::polygon(
                    &mut canvas,
                    &[(base as f32, 2.0), (tip as f32, 7.0), (base as f32, 12.0)],
                    team[side].base,
                );
                paint::line(&mut canvas, (base, 2), (tip, 7), team[side].light);
                paint::line(&mut canvas, (base, 12), (tip, 7), team[side].shadow);
            }
        }
        paint::put(&mut canvas, x, foot, IRON);
        paint::put(&mut canvas, x + 1, foot, IRON);
    }
    canvas
}

/// Where the banner's ribbon is.
pub fn banner_ribbon(width: i32) -> Area {
    (CLOTH_W + 10, CLOTH_H - 4, width - (CLOTH_W + 10) * 2, 18)
}

/// A picture turned to face the other way.
fn mirrored(canvas: &Canvas) -> Canvas {
    let (w, h) = (canvas.width() as i32, canvas.height() as i32);
    let mut out = Canvas::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            out.set(w - 1 - x, y, canvas.get(x, y));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::SYMBOLS;

    #[test]
    fn every_tile_of_every_part_is_its_own_picture() {
        let style = Style::default();
        for (part, n) in [
            (Part::Shape, SHAPES.len()),
            (Part::Field, FIELDS.len()),
            (Part::Symbol, SYMBOLS.len()),
            (Part::Trim, TRIMS.len()),
        ] {
            let tiles: Vec<Canvas> = (0..n).map(|i| tile(style, part, i, false, false)).collect();
            for i in 0..n {
                for j in i + 1..n {
                    assert_ne!(tiles[i], tiles[j], "{part:?} {i} and {j} look the same");
                }
                assert_ne!(tile(style, part, i, true, false), tiles[i], "chosen shows");
            }
        }
    }

    #[test]
    fn a_pennant_stirs_and_holds_still_at_rest() {
        let style = Style::default();
        assert_ne!(standard(style, stir_at(1)), standard(style, stir_at(3)));
        assert_eq!(standard(style, 0.0), standard(style, 0.0));
    }

    #[test]
    fn a_won_cup_has_its_rosette_and_one_still_to_win_is_pencilled() {
        for cup in 0..4 {
            let won = rosette(cup, true);
            assert_eq!(won.get(7, 7).a, 255);
            assert_ne!(won, rosette(cup, false));
        }
        assert_ne!(rosette(0, true), rosette(1, true));
    }

    #[test]
    fn a_team_picture_has_its_ribbon_and_fades_when_retired() {
        let mut nobody = [None, None, None];
        let picture = team_picture(160, 64, Style::default(), &mut nobody, false);
        let (x, y, w, h) = team_ribbon(160, 64);
        assert_eq!(picture.get(x + w / 2, y + h / 2).a, 255);
        assert_ne!(
            team_picture(160, 64, Style::default(), &mut nobody, true),
            picture
        );
        let plate = plate(120);
        assert_eq!(plate.get(60, PLATE_H / 2).a, 255);
        let banner = banner(200, [Some(Style::default()), None], crate::scene::TEAM);
        let (bx, by, bw, bh) = banner_ribbon(200);
        assert_eq!(banner.get(bx + bw / 2, by + bh / 2).a, 255);
    }
}
