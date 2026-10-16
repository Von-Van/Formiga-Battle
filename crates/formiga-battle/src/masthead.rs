//! The picture at the head of the Battleground's page: a little scene pasted into the notebook
//! with tape at its corners. Two of the colony's own companions face each other across Battle's
//! brass beacon on a grassy mound, each by their team's pennant, under a line of bunting, with a
//! ribbon across the foot for the page's title. The flame flickers, the bunting stirs, and now and
//! then one of them cheers; under reduced motion it all holds still.
//!
//! Painted at the notebook's own pixel to Formiga's rules: ramps lit from the upper left, edges in
//! a darker shade of their own colour, grain from `paint::noise`. The companions are drawn only
//! through their own looks. Only the ribbon's lettering is egui's.

use crate::garden::{BRASS, GRASS, PLANK, STONE};
use crate::look::{Look, Pose};
use crate::scene::TEAM;
use formiga_art::paint::{self, Ramp, rgb, rgba};
use formiga_art::{Canvas, FRAME_SIZE, Rgba};

const CARD: Ramp = Ramp::new(0xb8a682, 0xe2d3ae, 0xf6ecd0, 0xfcf5e2, 0xfffcf2);
const SKY_HIGH: Rgba = rgb(0xa9d6ea);
const SKY_LOW: Rgba = rgb(0xe4f3ec);
const FLAME: Rgba = rgb(0xf2a03c);
const FLAME_HEART: Rgba = rgb(0xfde9b0);
const BUNTING: [Rgba; 3] = [rgb(0xe07a5f), rgb(0xf5d25e), rgb(0x3d9fa0)];
/// The ribbon: the notebook's green, as its cloth is.
const RIBBON: Ramp = Ramp::new(0x10281f, 0x173a2e, 0x1c4137, 0x285a4b, 0x3b7563);
const SHADOW: Rgba = rgba(0x24452b, 60);
const TAPE: Rgba = rgba(0xfffaf0, 150);

/// A rectangle of the picture, in its pixels: left, top, width, height.
pub type Area = (i32, i32, i32, i32);

pub struct Masthead {
    /// The two who face each other, left then right.
    pair: [Option<Look>; 2],
    reduce_motion: bool,
    /// The Handler's own pennant, flown on the left in place of the team's, once they have one.
    flag: Option<crate::identity::Style>,
}

impl Masthead {
    pub fn new(pair: [Option<Look>; 2], reduce_motion: bool) -> Self {
        Self {
            pair,
            reduce_motion,
            flag: None,
        }
    }

    /// The same picture, with the Handler's pennant flying on the left.
    pub fn with_flag(self, flag: Option<crate::identity::Style>) -> Self {
        Self { flag, ..self }
    }

    /// Where the ribbon is, for the window to letter the title on.
    pub fn ribbon(width: i32, height: i32) -> Area {
        let w = (width * 7 / 10).max(40);
        (width / 2 - w / 2, height - 20, w, 13)
    }

    /// The picture, `width` by `height` of its pixels, `now`.
    pub fn paint(&mut self, width: i32, height: i32, now: f32) -> Canvas {
        let (w, h) = (width.max(80), height.max(60));
        let mut canvas = Canvas::new(w as u32, h as u32);
        let t = if self.reduce_motion { 0.0 } else { now };
        // The card it is drawn on, with a hard shadow down to the right.
        let (cx, cy, cw, ch) = (2, 3, w - 6, h - 9);
        paint::rect(&mut canvas, cx + 2, cy + 2, cw, ch, rgba(0x000000, 60));
        paint::rect(&mut canvas, cx, cy, cw, ch, CARD.edge);
        paint::rect(&mut canvas, cx + 1, cy + 1, cw - 2, ch - 2, CARD.base);
        // The scene inside a margin of card: sky, then the mound.
        let (sx, sy, sw, sh) = (cx + 4, cy + 4, cw - 8, ch - 8);
        let ground = sy + sh * 7 / 10;
        for y in sy..sy + sh {
            for x in sx..sx + sw {
                let tone = if y < ground {
                    let k = (y - sy) as f32 / (ground - sy).max(1) as f32;
                    let jitter = (paint::noise(x, y, 601) % 100) as f32 / 100.0 - 0.5;
                    let band = ((k * 5.0 + jitter * 0.7).floor() / 5.0).clamp(0.0, 1.0);
                    paint::mix(SKY_HIGH, SKY_LOW, band)
                } else {
                    let n = paint::noise(x, y, 607) % 100;
                    match n {
                        0..=11 => GRASS.shadow,
                        12..=19 => GRASS.shine,
                        _ => GRASS.light,
                    }
                };
                canvas.set(x, y, tone);
            }
        }
        // The mound's crest, a little rounded, lit along its top.
        for x in sx..sx + sw {
            let u = (x - sx) as f32 / sw as f32 - 0.5;
            let rise = ((1.0 - 4.0 * u * u).max(0.0) * 4.0) as i32;
            for y in ground - rise..ground {
                canvas.set(x, y, GRASS.light);
            }
            canvas.set(x, ground - rise, GRASS.edge);
            canvas.set(x, ground - rise + 1, GRASS.shine);
        }
        // A distant line of hills behind it.
        for x in sx..sx + sw {
            let top = ground
                - 9
                - (3.0 * ((x as f32) * 0.09).sin() + 2.0 * ((x as f32) * 0.21 + 1.0).sin()) as i32;
            for y in top..ground - 4 {
                if canvas.get(x, y) != GRASS.light {
                    canvas.set(
                        x,
                        y,
                        if y == top {
                            rgb(0x7ba79b)
                        } else {
                            rgb(0x9cc4b0)
                        },
                    );
                }
            }
        }
        // Bunting across the top, sagging and stirring.
        let (left, right) = (sx + 2, sx + sw - 3);
        let top = sy + 3;
        let sag = 6.0;
        let at = |x: i32| -> i32 {
            let u = (x - left) as f32 / (right - left).max(1) as f32;
            top + (sag * (1.0 - (2.0 * u - 1.0).powi(2))) as i32
        };
        for x in left..=right {
            paint::put(&mut canvas, x, at(x), rgb(0x6e4a33));
        }
        let mut k = 0;
        let mut x = left + 4;
        while x + 5 < right {
            let y = at(x + 2) + 1;
            let stir = ((t * 2.2 + k as f32 * 0.8).sin() * 1.2).round() as i32;
            let colour = BUNTING[k % BUNTING.len()];
            paint::polygon(
                &mut canvas,
                &[
                    (x as f32, y as f32),
                    ((x + 5) as f32, y as f32),
                    ((x + 2 + stir) as f32 + 0.5, (y + 6) as f32),
                ],
                colour,
            );
            paint::hline(
                &mut canvas,
                x,
                y,
                5,
                paint::mix(colour, rgb(0xffffff), 0.35),
            );
            x += 8;
            k += 1;
        }
        // The beacon on its stone plinth, its flame flickering.
        let mid = sx + sw / 2;
        let foot = ground - 3;
        paint::ellipse(&mut canvas, mid + 2, foot + 1, 6, 1, SHADOW);
        paint::bevel(&mut canvas, mid - 5, foot - 4, 11, 5, STONE);
        paint::vline(&mut canvas, mid - 1, foot - 16, 12, BRASS.light);
        paint::vline(&mut canvas, mid, foot - 16, 12, BRASS.base);
        paint::vline(&mut canvas, mid + 1, foot - 16, 12, BRASS.shadow);
        paint::hline(&mut canvas, mid - 4, foot - 17, 9, BRASS.base);
        paint::hline(&mut canvas, mid - 4, foot - 18, 9, BRASS.light);
        paint::put(&mut canvas, mid - 4, foot - 17, BRASS.edge);
        paint::put(&mut canvas, mid + 4, foot - 17, BRASS.edge);
        paint::put(&mut canvas, mid - 3, foot - 18, BRASS.shine);
        let flicker = ((t * 9.0).sin() + (t * 13.0).sin() * 0.5).round() as i32;
        paint::ellipse(&mut canvas, mid, foot - 22, 3, 4 + flicker.max(0), FLAME);
        paint::vline(&mut canvas, mid, foot - 23, 3, FLAME_HEART);
        paint::put(
            &mut canvas,
            mid + flicker,
            foot - 27 - flicker.max(0),
            FLAME,
        );
        // A pennant for each team, flying towards the beacon.
        let spread = (sw * 3 / 10).min(70);
        match self.flag {
            Some(style) => own_pennant(&mut canvas, mid - spread - 24, foot, style),
            None => pennant(&mut canvas, mid - spread - 24, foot, 1, TEAM[0], t),
        }
        pennant(&mut canvas, mid + spread + 24, foot, -1, TEAM[1], t + 1.3);
        // The two companions, facing each other across it; now and then one cheers.
        for (side, look) in self.pair.iter_mut().enumerate() {
            let Some(look) = look else {
                continue;
            };
            let facing_right = side == 0;
            let phase = t + side as f32 * 3.0;
            let pose = if !self.reduce_motion && phase % 6.0 < 1.1 {
                Pose::Cheer
            } else {
                Pose::Idle
            };
            let x = if side == 0 {
                mid - spread
            } else {
                mid + spread
            };
            paint::ellipse(&mut canvas, x + 1, foot + 1, 7, 1, SHADOW);
            let top = foot - look.foot_row;
            let frame = look.frame(pose, phase, facing_right);
            paint::blit(&mut canvas, frame, x - FRAME_SIZE as i32 / 2, top);
        }
        // Whatever strayed past the scene's edge is trimmed back to the card's margin.
        for y in 0..h {
            for x in 0..w {
                let in_scene = (sx..sx + sw).contains(&x) && (sy..sy + sh).contains(&y);
                let in_card = (cx..cx + cw).contains(&x) && (cy..cy + ch).contains(&y);
                if in_card && !in_scene && x > cx && y > cy && x < cx + cw - 1 && y < cy + ch - 1 {
                    let fibre = paint::noise(x, y, 613) % 100;
                    canvas.set(x, y, if fibre < 6 { CARD.light } else { CARD.base });
                }
            }
        }
        // The scene's own thin frame, and the card lit along its top and left.
        paint::hline(&mut canvas, sx - 1, sy - 1, sw + 2, CARD.shadow);
        paint::vline(&mut canvas, sx - 1, sy - 1, sh + 2, CARD.shadow);
        paint::hline(&mut canvas, sx - 1, sy + sh, sw + 2, CARD.shine);
        paint::vline(&mut canvas, sx + sw, sy - 1, sh + 2, CARD.shine);
        paint::hline(&mut canvas, cx + 1, cy + 1, cw - 2, CARD.shine);
        paint::vline(&mut canvas, cx + 1, cy + 1, ch - 2, CARD.light);
        for (x, y) in [
            (cx, cy),
            (cx + cw - 1, cy),
            (cx, cy + ch - 1),
            (cx + cw - 1, cy + ch - 1),
        ] {
            canvas.set(x, y, Rgba::new(0, 0, 0, 0));
        }
        // Tape at its two top corners, holding it to the page.
        for (tx, lean) in [(cx - 3, 1), (cx + cw - 13, -1)] {
            for i in 0..16 {
                let y0 = cy - 2 + if lean > 0 { i / 4 } else { 3 - i / 4 };
                paint::vline(&mut canvas, tx + i, y0, 5, TAPE);
            }
        }
        ribbon(&mut canvas, Self::ribbon(w, h));
        canvas
    }
}

/// A pennant on a garden cane standing at `x`, its foot at `foot`, flying towards `toward`
/// (1 is right, -1 left), its tip stirring.
fn pennant(canvas: &mut Canvas, x: i32, foot: i32, toward: i32, team: Ramp, t: f32) {
    paint::ellipse(canvas, x + 1, foot + 1, 2, 1, SHADOW);
    paint::vline(canvas, x, foot - 26, 26, PLANK.shadow);
    paint::vline(canvas, x + toward.max(0), foot - 25, 25, PLANK.light);
    let stir = (t * 2.6).sin() * 1.2;
    let base = (x + toward) as f32;
    let tip = (base + 10.0 * toward as f32, (foot - 20) as f32 + stir);
    paint::polygon(
        canvas,
        &[(base, (foot - 26) as f32), tip, (base, (foot - 14) as f32)],
        team.base,
    );
    let tip_i = (tip.0 as i32, tip.1.round() as i32);
    paint::line(canvas, (x + toward, foot - 26), tip_i, team.light);
    paint::line(canvas, (x + toward, foot - 14), tip_i, team.shadow);
    paint::put(canvas, x, foot - 27, team.shine);
}

/// The Handler's own pennant on a garden cane at `x`, flying towards the beacon.
fn own_pennant(canvas: &mut Canvas, x: i32, foot: i32, style: crate::identity::Style) {
    paint::ellipse(canvas, x + 1, foot + 1, 2, 1, SHADOW);
    paint::vline(canvas, x, foot - 26, 26, PLANK.shadow);
    paint::vline(canvas, x + 1, foot - 25, 25, PLANK.light);
    paint::put(canvas, x, foot - 27, BRASS.shine);
    paint::blit(canvas, &crate::pennant::cloth(style), x + 2, foot - 26);
}

/// The ribbon across the picture's foot, its ends folded back and notched.
fn ribbon(canvas: &mut Canvas, (x, y, w, h): Area) {
    // The folded-back tails, a shade darker, behind it.
    for (tx, dir) in [(x - 8, 1), (x + w - 2, -1)] {
        for dy in 0..h {
            for dx in 0..10 {
                let notch = (dy - h / 2).abs() < 3 && ((dir > 0 && dx < 3) || (dir < 0 && dx > 6));
                if !notch {
                    canvas.set(tx + dx, y + 3 + dy, RIBBON.shadow);
                }
            }
        }
    }
    paint::bevel(canvas, x, y, w, h, RIBBON);
    // A woven grain, as the notebook's cloth has.
    for py in y + 2..y + h - 2 {
        for px in x + 2..x + w - 2 {
            if (px + py) % 2 == 0 {
                paint::put(canvas, px, py, paint::faded(RIBBON.light, 90));
            }
        }
    }
    // Stitches along both its edges.
    let mut along = x + 3;
    while along + 2 < x + w - 3 {
        paint::hline(canvas, along, y + 2, 2, rgb(0xcdb17a));
        paint::hline(canvas, along, y + h - 3, 2, rgb(0xb2955f));
        along += 4;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_has_its_ribbon_and_still_holds_still_under_reduced_motion() {
        let mut still = Masthead::new([None, None], true);
        assert_eq!(still.paint(240, 96, 0.0), still.paint(240, 96, 3.7));
        let (x, y, w, h) = Masthead::ribbon(240, 96);
        let picture = still.paint(240, 96, 0.0);
        assert_eq!(picture.get(x + w / 2, y + h / 2).a, 255);
        let mut lively = Masthead::new([None, None], false);
        assert_ne!(lively.paint(240, 96, 0.0), lively.paint(240, 96, 0.6));
        let mut flown =
            Masthead::new([None, None], true).with_flag(Some(crate::identity::Style::default()));
        assert_ne!(
            flown.paint(240, 96, 0.0),
            picture,
            "the Handler's pennant flies"
        );
    }
}
