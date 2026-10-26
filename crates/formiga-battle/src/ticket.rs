//! A railway ticket to somewhere, for the station's two choices, painted at the notebook's own
//! pixel to Formiga's rules as the cover is: card stock with a grain, lit along its top and left
//! and shaded along its foot and right, outlined in a darker shade of its own colour; a stub of
//! woven cloth in the ticket's colour, stitched round, with a hole punched through it and a picture
//! of where it goes; a perforation to tear along, notched at both ends; and an engraved frame
//! round the card. Only the lettering is egui's.
//!
//! It lifts off the page when pointed at, and springs with a brass sparkle when taken.

use crate::chrome::{self, hand_job, ink, label_job};
use eframe::egui::{self, Color32};
use formiga_art::paint::{self, Ramp, rgb, rgba};
use formiga_art::{Canvas, Rgba};

const CARD: Ramp = Ramp::new(0xb8a682, 0xe2d3ae, 0xf6ecd0, 0xfcf5e2, 0xfffcf2);
const STITCH: Ramp = Ramp::new(0x8a7148, 0xb2955f, 0xcdb17a, 0xe2cc98, 0xf4e6c0);
const BRASS: Ramp = Ramp::new(0x6e4f1c, 0xa47a2c, 0xcf9f3e, 0xe8c25a, 0xfbe6a2);
/// The notebook's green, as its cloth patch is woven.
const FOREST: Ramp = Ramp::new(0x10281f, 0x173a2e, 0x1c4137, 0x285a4b, 0x3b7563);
/// The train's maroon.
const MAROON: Ramp = Ramp::new(0x3a1418, 0x5c2026, 0x7a2c33, 0x963c42, 0xb4585a);
const SEA: Ramp = Ramp::new(0x2c5a72, 0x3c7690, 0x5a94aa, 0x7fb8c8, 0xc6e8ee);
const LAND: Ramp = Ramp::new(0x2f5a2e, 0x447a3c, 0x5e9450, 0x7db36c, 0x9fd08a);
const ROOF: Ramp = Ramp::new(0x6e2a22, 0x96392c, 0xb84e38, 0xd06a4c, 0xe48e6c);
const PLASTER: Ramp = Ramp::new(0x9a8a6a, 0xd8c9a8, 0xf0e4c6, 0xfaf3e1, 0xffffff);
const DOOR: Ramp = Ramp::new(0x2f4a3c, 0x46705a, 0x5c8f74, 0x7aae92, 0xa2cdb4);
const BRICK: Ramp = Ramp::new(0x6a3328, 0x8f4636, 0xab5a42, 0xc4775a, 0xdc9c7c);
const LAMPLIT: Rgba = rgb(0xffe08a);
const CLEAR: Rgba = Rgba::new(0, 0, 0, 0);

/// Where a ticket goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Destination {
    /// Out in the world: a friend, online. The notebook's green, and a globe.
    World,
    /// Close to home: the visitors, offline. The train's maroon, and a cottage.
    Home,
}

impl Destination {
    fn ramp(self) -> Ramp {
        match self {
            Self::World => FOREST,
            Self::Home => MAROON,
        }
    }

    /// The ink its lettering is printed in.
    pub fn ink(self) -> Color32 {
        let c = self.ramp().base;
        Color32::from_rgb(c.r, c.g, c.b)
    }
}

/// How wide a ticket's stub is, in its pixels.
fn stub_width(width: i32, height: i32) -> i32 {
    ((height as f32 * 0.95) as i32).min(width * 3 / 10)
}

/// A ticket `width` by `height` of its pixels, lit a little when pointed at.
pub fn paint(width: i32, height: i32, to: Destination, lit: bool) -> Canvas {
    let mut canvas = Canvas::new(width.max(8) as u32, height.max(8) as u32);
    let (w, h) = (canvas.width() as i32, canvas.height() as i32);
    let ramp = to.ramp();
    let stub = stub_width(w, h);
    // The card, with a fibre in it, warmer when lit.
    for y in 0..h {
        for x in 0..w {
            let fibre = paint::noise(x, y, 521) % 100;
            let tone = match fibre {
                0..=4 => CARD.shadow,
                5..=9 => CARD.light,
                _ => CARD.base,
            };
            let tone = if lit {
                paint::mix(tone, CARD.shine, 0.35)
            } else {
                tone
            };
            canvas.set(x, y, tone);
        }
    }
    // The stub: cloth woven in the ticket's colour, as the cover's patch is.
    for y in 1..h - 1 {
        for x in 1..stub {
            let woven = if (x + y) % 2 == 0 {
                ramp.base
            } else {
                ramp.light
            };
            let tone = if paint::chance(x, y, 523, 24) {
                ramp.shadow
            } else {
                woven
            };
            let tone = if lit {
                paint::mix(tone, ramp.shine, 0.18)
            } else {
                tone
            };
            canvas.set(x, y, tone);
        }
    }
    // Stitched round, two pixels in.
    let mut along = 4;
    while along + 2 < stub - 3 {
        paint::hline(&mut canvas, along, 3, 2, STITCH.base);
        paint::hline(&mut canvas, along, h - 4, 2, STITCH.shadow);
        along += 4;
    }
    let mut down = 5;
    while down + 2 < h - 4 {
        paint::vline(&mut canvas, 3, down, 2, STITCH.base);
        down += 4;
    }
    // A hole punched through, with the stub's edge round it.
    let hole = (stub / 2, 7);
    paint::ellipse(&mut canvas, hole.0, hole.1, 2, 2, ramp.edge);
    paint::ellipse(&mut canvas, hole.0, hole.1, 1, 1, CLEAR);
    for (dx, dy) in [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (0, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ] {
        canvas.set(hole.0 + dx, hole.1 + dy, CLEAR);
    }
    paint::put(&mut canvas, hole.0 + 1, hole.1 + 2, ramp.light);
    // Its picture, a little below the middle of what is left of the stub.
    let middle = (stub / 2, (h + 10) / 2);
    match to {
        Destination::World => globe(&mut canvas, middle, (h / 4).clamp(7, 14)),
        Destination::Home => cottage(
            &mut canvas,
            middle,
            ((stub - 4) / 24).min(h / 24).clamp(1, 3),
        ),
    }
    // The perforation: a row of holes to tear along, each with a shade under it.
    let mut y = 4;
    while y < h - 4 {
        canvas.set(stub, y, CLEAR);
        paint::put(&mut canvas, stub + 1, y, rgba(0x3b2b3a, 50));
        paint::put(&mut canvas, stub, y + 1, rgba(0x3b2b3a, 30));
        y += 3;
    }
    // An engraved frame round the card, its corners turned in.
    let (fx, fy, fw, fh) = (stub + 5, 4, w - stub - 9, h - 8);
    let rule = paint::faded(ramp.base, 120);
    paint::hline(&mut canvas, fx + 2, fy, fw - 4, rule);
    paint::hline(&mut canvas, fx + 2, fy + fh - 1, fw - 4, rule);
    paint::vline(&mut canvas, fx, fy + 2, fh - 4, rule);
    paint::vline(&mut canvas, fx + fw - 1, fy + 2, fh - 4, rule);
    for (cx, cy) in [
        (fx, fy),
        (fx + fw - 2, fy),
        (fx, fy + fh - 2),
        (fx + fw - 2, fy + fh - 2),
    ] {
        paint::rect(&mut canvas, cx, cy, 2, 2, paint::faded(ramp.light, 150));
    }
    // A faint engraved hatch inside it, as a ticket's ground is printed.
    for y in fy + 2..fy + fh - 2 {
        for x in fx + 2..fx + fw - 2 {
            if (x + y) % 7 == 0 {
                paint::put(&mut canvas, x, y, paint::faded(ramp.base, 16));
            }
        }
    }
    // Lit along its top and left, shaded along its foot and right, outlined in its own colour.
    let card_edge = paint::mix(CARD.edge, ramp.edge, 0.35);
    paint::hline(&mut canvas, stub + 1, 1, w - stub - 2, CARD.shine);
    paint::hline(&mut canvas, stub + 1, h - 2, w - stub - 2, CARD.shadow);
    paint::vline(&mut canvas, w - 2, 1, h - 2, CARD.shadow);
    paint::hline(&mut canvas, 1, 1, stub - 1, ramp.shine);
    paint::vline(&mut canvas, 1, 1, h - 2, ramp.light);
    paint::hline(&mut canvas, 1, h - 2, stub - 1, ramp.shadow);
    paint::hline(&mut canvas, 1, 0, w - 2, card_edge);
    paint::hline(&mut canvas, 1, h - 1, w - 2, card_edge);
    paint::vline(&mut canvas, 0, 1, h - 2, ramp.edge);
    paint::vline(&mut canvas, w - 1, 1, h - 2, card_edge);
    for (cx, cy) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
        canvas.set(cx, cy, CLEAR);
    }
    // The perforation's notches, bitten out of the edge last.
    for notch in [0, h - 1] {
        for dy in -3i32..=3 {
            for dx in -3i32..=3 {
                if dx * dx + dy * dy <= 9 {
                    canvas.set(stub + dx, notch + dy, CLEAR);
                }
            }
        }
    }
    canvas
}

/// A little globe on a brass stand: its seas and its land lit from the upper left, rimmed in
/// their own darker shade.
fn globe(canvas: &mut Canvas, (cx, cy): (i32, i32), r: i32) {
    let cy = cy - 2;
    // The stand under it.
    paint::rect(canvas, cx - r / 2, cy + r + 2, r + 1, 2, BRASS.base);
    paint::hline(canvas, cx - r / 2, cy + r + 2, r + 1, BRASS.light);
    paint::hline(canvas, cx - r / 2, cy + r + 4, r + 1, BRASS.edge);
    paint::vline(canvas, cx, cy + r, 2, BRASS.shadow);
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = dx * dx + dy * dy;
            if d2 > r * r {
                continue;
            }
            let (nx, ny) = (dx as f32 / r as f32, dy as f32 / r as f32);
            let nz = (1.0 - nx * nx - ny * ny).max(0.0).sqrt();
            let light = -0.5 * nx - 0.6 * ny + 0.6 * nz;
            // Land in blocks of three, so it reads as coasts at this size.
            let land = paint::noise((dx + 40) / 3, (dy + 40) / 3, 541) % 100 < 40;
            let ramp = if land { LAND } else { SEA };
            let tone = if d2 > (r - 1) * (r - 1) {
                ramp.edge
            } else if light > 0.8 {
                ramp.shine
            } else if light > 0.45 {
                ramp.light
            } else if light > 0.05 {
                ramp.base
            } else {
                ramp.shadow
            };
            canvas.set(cx + dx, cy + dy, tone);
        }
    }
    // A brass meridian down its right side, as a globe on a desk has.
    paint::vline(canvas, cx + r + 1, cy - r / 2, r, BRASS.shadow);
    paint::put(canvas, cx + r + 1, cy - r / 2 - 1, BRASS.light);
}

/// A cottage: a tiled roof lit on its left slope, cream walls, a lamp in the window, a green
/// door with a brass knob, and a chimney.
fn cottage(canvas: &mut Canvas, (cx, cy): (i32, i32), s: i32) {
    let w = 12 * s;
    let left = cx - w;
    let top = cy - 11 * s;
    let px = |canvas: &mut Canvas, x: i32, y: i32, ww: i32, hh: i32, c: Rgba| {
        paint::rect(canvas, left + x * s, top + y * s, ww * s, hh * s, c);
    };
    // The chimney, behind the roof.
    px(canvas, 15, 1, 3, 6, BRICK.base);
    px(canvas, 15, 1, 1, 6, BRICK.light);
    px(canvas, 14, 0, 5, 1, BRICK.edge);
    // The roof, course by course, its left slope lit and its right in shade.
    for row in 0..9 {
        let half = 3 + row;
        let y = 2 + row;
        for x in (12 - half)..(12 + half) {
            let tone = if x == 12 - half || x == 12 + half - 1 {
                ROOF.edge
            } else if row % 2 == 1 && (x + row) % 3 == 0 {
                ROOF.shadow
            } else if x < 12 {
                if row == 0 { ROOF.shine } else { ROOF.light }
            } else {
                ROOF.base
            };
            px(canvas, x, y, 1, 1, tone);
        }
    }
    px(canvas, 1, 11, 22, 1, ROOF.edge);
    // The walls.
    px(canvas, 2, 12, 20, 9, PLASTER.base);
    px(canvas, 2, 12, 20, 1, PLASTER.shadow);
    px(canvas, 2, 13, 1, 8, PLASTER.light);
    px(canvas, 21, 12, 1, 9, PLASTER.shadow);
    px(canvas, 1, 12, 1, 9, PLASTER.edge);
    px(canvas, 22, 12, 1, 9, PLASTER.edge);
    // The window, lit, with its cross bars.
    px(canvas, 4, 14, 6, 5, rgb(0x6e4a33));
    px(canvas, 5, 15, 4, 3, LAMPLIT);
    px(canvas, 5, 15, 1, 1, rgb(0xfff6d8));
    px(canvas, 7, 15, 1, 3, rgb(0x6e4a33));
    px(canvas, 5, 16, 4, 1, rgb(0x6e4a33));
    // The door, and its knob.
    px(canvas, 13, 14, 5, 7, DOOR.edge);
    px(canvas, 14, 15, 3, 6, DOOR.base);
    px(canvas, 14, 15, 1, 6, DOOR.light);
    px(canvas, 16, 18, 1, 1, BRASS.light);
    // The doorstep, and the ground line.
    px(canvas, 12, 21, 7, 1, rgb(0xa69d94));
    px(canvas, 1, 21, 22, 1, PLASTER.edge);
}

/// The ticket as a button, painted for its size and how it is pointed at, and lettered on top:
/// who it admits, where it goes, the word stamped beneath, and how the Handler is getting on
/// there.
#[allow(clippy::too_many_arguments)]
pub fn ticket(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: egui::Id,
    title: &str,
    small: &str,
    status: &str,
    to: Destination,
    unit: f32,
) -> egui::Response {
    let response = ui.interact(rect, id, egui::Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let press = crate::feel::press(ui.ctx(), id, &response, crate::feel::Feel::Confirm);
    let lit = press.hover > 0.5;
    let (w, h) = (
        (rect.width() / unit).round() as i32,
        (rect.height() / unit).round() as i32,
    );
    // Painted once for each size and state, and kept.
    let key = id.with(("ticket-art", w, h, lit));
    let texture = ui
        .ctx()
        .data(|d| d.get_temp::<egui::TextureHandle>(key))
        .unwrap_or_else(|| {
            let canvas = paint(w, h, to, lit);
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [canvas.width() as usize, canvas.height() as usize],
                &canvas.rgba_bytes(),
            );
            let t = ui.ctx().load_texture(
                format!("ticket-{w}-{h}-{lit}-{to:?}"),
                image,
                egui::TextureOptions::NEAREST,
            );
            ui.ctx().data_mut(|d| d.insert_temp(key, t.clone()));
            t
        });
    let lift = (press.hover * unit * 2.0).round();
    let pressed = response.is_pointer_button_down_on();
    let mut face = rect;
    ui.with_visual_transform(press.transform(rect, unit), |ui| {
        let painter = ui.painter();
        face = egui::Rect::from_min_size(
            rect.min + egui::vec2(0.0, if pressed { 0.0 } else { -lift }),
            egui::vec2(w as f32, h as f32) * unit,
        );
        // A hard shadow that grows as it lifts.
        let shadow = rect.translate(egui::vec2(unit * 2.0, unit * 2.0 + lift / 2.0));
        painter.rect_filled(shadow.shrink(unit), 0.0, ink::SHADOW);
        painter.image(
            texture.id(),
            face,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let colour = to.ink();
        let stub = stub_width(w, h) as f32 * unit;
        let body = egui::Rect::from_min_max(egui::pos2(face.left() + stub, face.top()), face.max);
        let admit = painter.layout_job(label_job("Admit three", 8.5, ink::MUTED));
        painter.galley(
            body.min + egui::vec2(unit * 9.0, unit * 7.0),
            admit,
            ink::MUTED,
        );
        // As large as the card has room for, up to its usual size.
        let mut letters = 18.0;
        let mut big = painter.layout_job(hand_job(title, letters, colour));
        while big.size().x > body.width() - unit * 14.0 && letters > 10.0 {
            letters -= 0.5;
            big = painter.layout_job(hand_job(title, letters, colour));
        }
        let little = painter.layout_job(label_job(small, chrome::LABEL, ink::STAMP));
        let stamp_size = little.size() + egui::vec2(unit * 8.0, unit * 4.0);
        let gap = unit * 3.0;
        let big_size = big.size();
        let note = painter.layout_no_wrap(
            status.to_owned(),
            formiga_pencil::hand_bold(chrome::HAND),
            ink::FOREST,
        );
        let note_size = note.size();
        let total = big_size.y + gap + stamp_size.y + gap + note_size.y;
        let top = body.center().y - total / 2.0 + unit * 3.0;
        painter.galley(
            egui::pos2(body.center().x - big_size.x / 2.0, top),
            big,
            colour,
        );
        // Stamped in the stamp's red, a little off true, as a stamp is.
        let stamp = egui::Rect::from_center_size(
            egui::pos2(
                body.center().x + unit,
                top + big_size.y + gap + stamp_size.y / 2.0,
            ),
            stamp_size,
        );
        chrome::stepped(painter, stamp, unit, ink::STAMP.gamma_multiply(0.8));
        painter.galley(stamp.center() - little.size() / 2.0, little, ink::STAMP);
        // How the Handler is getting on there, jotted under it.
        painter.galley(
            egui::pos2(body.center().x - note_size.x / 2.0, stamp.bottom() + gap),
            note,
            ink::FOREST,
        );
    });
    if let Some(through) = press.sparkle {
        crate::feel::sparkle(ui.painter(), face, through, unit);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ticket_has_its_corners_and_notches_cut_and_its_card_whole() {
        let t = paint(300, 64, Destination::Home, false);
        assert_eq!(t.get(0, 0).a, 0);
        assert_eq!(t.get(stub_width(300, 64), 0).a, 0, "a notch at the top");
        assert_eq!(t.get(200, 30).a, 255);
        assert_ne!(
            paint(300, 64, Destination::World, true),
            paint(300, 64, Destination::World, false)
        );
    }
}
