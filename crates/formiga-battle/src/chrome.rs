//! The Handler's notebook round the window, in the same leather, paper and stitching as Formiga
//! Home's: a cover stitched round its edge with brass at the corners, a cloth patch carrying
//! Battle's name, the board on the left page and the Handler's notes on the ruled right one, and
//! the side's two pages (orders, notebook) as tabs stitched to the cover.
//!
//! The cover and pages are painted here at the notebook's own pixel scale, to the board's rules;
//! everything else on them (cards, buttons, labels, meters) is drawn by the window in the same
//! colours with the notebook's stepped edges and hard shadows, so the two read as one thing.
//! Only lettering is egui's.

use eframe::egui::{self, Color32};
use formiga_art::paint::{self, Ramp, rgb};
use formiga_art::{Canvas, Rgba};

/// A rectangle of the notebook, in its own pixels: left, top, width, height.
pub type Area = (i32, i32, i32, i32);

const LEATHER: Ramp = Ramp::new(0x2f1c12, 0x4a2e1e, 0x5b3a27, 0x6e4a33, 0x87604a);
const STITCH: Ramp = Ramp::new(0x8a7148, 0xb2955f, 0xcdb17a, 0xe2cc98, 0xf4e6c0);
const BRASS: Ramp = Ramp::new(0x6e4f1c, 0xa47a2c, 0xcf9f3e, 0xe8c25a, 0xfbe6a2);
const CLOTH: Ramp = Ramp::new(0x10281f, 0x173a2e, 0x1c4137, 0x285a4b, 0x3b7563);
const PAPER: Ramp = Ramp::new(0xc9b78e, 0xe8dab4, 0xf9f0d4, 0xfdf7e4, 0xfffcf2);
const RULE: Rgba = rgb(0xe9dbb2);
const MARGIN: Rgba = rgb(0xd99f89);

/// The leather, for the window's clear colour.
pub const COVER: Rgba = rgb(0x5b3a27);

/// The notebook's pixel, in points: as near a point and a half as whole screen pixels allow, so
/// it is as crisp as the board drawn on its page.
pub fn unit(pixels_per_point: f32) -> f32 {
    (1.5 * pixels_per_point).round().max(2.0) / pixels_per_point
}

/// The interface's colours, for what is written in the notebook rather than painted on it:
/// Home's inks by daylight.
pub mod ink {
    use eframe::egui::Color32;

    pub const PAGE: Color32 = Color32::from_rgb(35, 54, 47);
    /// Headings, and the accent the notebook is built round.
    pub const FOREST: Color32 = Color32::from_rgb(28, 65, 55);
    /// Present, but not asking for attention.
    pub const MUTED: Color32 = Color32::from_rgb(110, 119, 108);
    /// Whatever is chosen.
    pub const MINT: Color32 = Color32::from_rgb(174, 221, 186);
    /// The notebook's outlines: plum, as the creatures are outlined.
    pub const LINE: Color32 = Color32::from_rgb(59, 43, 58);
    /// The face of a card, a shade away from the page.
    pub const CARD: Color32 = Color32::from_rgb(255, 248, 228);
    /// The red of a rubber stamp.
    pub const STAMP: Color32 = Color32::from_rgb(181, 84, 63);
    /// Lettering on the leather.
    pub const COVER: Color32 = Color32::from_rgb(243, 230, 196);
    /// Lettering pressed into the leather.
    pub const DEBOSS: Color32 = Color32::from_rgb(178, 149, 112);
    /// A sticky note's paper, for what Battle has to say right now.
    pub const NOTE: Color32 = Color32::from_rgb(246, 220, 154);
    /// Brass, for the watch and the lock-in button's rim.
    pub const BRASS: Color32 = Color32::from_rgb(207, 159, 62);
    pub const BRASS_LIGHT: Color32 = Color32::from_rgb(232, 194, 90);
    pub const BRASS_EDGE: Color32 = Color32::from_rgb(110, 79, 28);
    /// A hard drop shadow, as Hill and Home cast them.
    pub const SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 60);
}

/// A tab stitched to the cover: the open one is a slip of paper, the others only lettering
/// pressed into the leather.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tab {
    pub at: Area,
    pub open: bool,
}

/// Where everything is, in the notebook's own pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spread {
    pub size: (i32, i32),
    pub left: Area,
    pub right: Area,
    pub patch: Area,
    pub tabs: Vec<Tab>,
    /// Ruled lines on the notes page: how far down the first is, and how far apart.
    pub rules: (i32, i32),
}

/// The whole notebook, open.
pub fn paint(spread: &Spread) -> Canvas {
    let (width, height) = spread.size;
    let mut canvas = Canvas::new(width.max(1) as u32, height.max(1) as u32);
    leather(&mut canvas, (0, 0, width, height), 3);
    let rim = LEATHER;
    paint::hline(&mut canvas, 0, 0, width, rim.edge);
    paint::vline(&mut canvas, 0, 0, height, rim.edge);
    paint::hline(&mut canvas, 0, height - 1, width, rim.edge);
    paint::vline(&mut canvas, width - 1, 0, height, rim.edge);
    paint::hline(&mut canvas, 1, 1, width - 2, rim.light);
    paint::vline(&mut canvas, 1, 1, height - 2, rim.light);
    paint::hline(&mut canvas, 1, height - 2, width - 2, rim.shadow);
    paint::vline(&mut canvas, width - 2, 1, height - 2, rim.shadow);
    stitching(&mut canvas, (4, 4, width - 8, height - 8));
    for corner in 0..4 {
        brass_corner(&mut canvas, (width, height), corner);
    }
    for (page, outer_left) in [(spread.left, true), (spread.right, false)] {
        page_shadow(&mut canvas, page);
        paper(&mut canvas, page, outer_left);
    }
    binding(&mut canvas, spread.left, spread.right);
    ruled(&mut canvas, spread.right, spread.rules);
    patch(&mut canvas, spread.patch);
    for tab in &spread.tabs {
        tab_slip(&mut canvas, tab);
    }
    canvas
}

/// Pebbled leather, a touch lighter towards the upper left.
fn leather(canvas: &mut Canvas, (x, y, w, h): Area, salt: u32) {
    let ramp = LEATHER;
    let (cw, ch) = (canvas.width() as f32, canvas.height() as f32);
    for py in y..y + h {
        for px in x..x + w {
            let light = 1.0 - (px as f32 / cw + py as f32 / ch) / 2.0;
            let base = paint::mix(ramp.shadow, ramp.base, 0.55 + light * 0.45);
            // Pebbles two pixels across, each lit on its upper left and shaded on its lower right.
            let pebble = paint::noise(px.div_euclid(2), py.div_euclid(2), salt) % 100;
            let corner = (px.rem_euclid(2), py.rem_euclid(2));
            let tone = match (pebble, corner) {
                (0..=13, (0, 0)) => paint::mix(base, ramp.light, 0.6),
                (0..=13, (1, 1)) => paint::mix(base, ramp.shadow, 0.5),
                (14..=21, _) => paint::mix(base, ramp.shadow, 0.35),
                (22..=24, (0, _)) => paint::mix(base, ramp.light, 0.35),
                _ => base,
            };
            canvas.set(px, py, tone);
        }
    }
}

/// A line of saddle stitches round `area`.
fn stitching(canvas: &mut Canvas, (x, y, w, h): Area) {
    let ramp = STITCH;
    let stitch = |canvas: &mut Canvas, sx: i32, sy: i32, across: bool| {
        if across {
            paint::hline(canvas, sx, sy, 3, ramp.base);
            paint::put(canvas, sx, sy, ramp.light);
            paint::put(canvas, sx + 3, sy, ramp.edge);
        } else {
            paint::vline(canvas, sx, sy, 3, ramp.base);
            paint::put(canvas, sx, sy, ramp.light);
            paint::put(canvas, sx, sy + 3, ramp.edge);
        }
    };
    let mut along = x + 2;
    while along + 3 < x + w - 1 {
        stitch(canvas, along, y, true);
        stitch(canvas, along, y + h - 1, true);
        along += 6;
    }
    let mut down = y + 2;
    while down + 3 < y + h - 1 {
        stitch(canvas, x, down, false);
        stitch(canvas, x + w - 1, down, false);
        down += 6;
    }
}

/// A brass plate folded over one corner of the cover, with a rivet.
fn brass_corner(canvas: &mut Canvas, (width, height): (i32, i32), corner: u8) {
    const ARM: i32 = 10;
    const THICK: i32 = 3;
    let ramp = BRASS;
    let (right, bottom) = (corner % 2 == 1, corner >= 2);
    let x = if right { width - ARM } else { 0 };
    let y = if bottom { height - ARM } else { 0 };
    let along_y = if bottom { height - THICK } else { 0 };
    let along_x = if right { width - THICK } else { 0 };
    paint::rect(canvas, x, along_y, ARM, THICK, ramp.base);
    paint::rect(canvas, along_x, y, THICK, ARM, ramp.base);
    paint::hline(canvas, x, along_y, ARM, ramp.light);
    paint::vline(canvas, along_x, y, ARM, ramp.light);
    paint::hline(canvas, x, along_y + THICK - 1, ARM, ramp.shadow);
    paint::vline(canvas, along_x + THICK - 1, y, ARM, ramp.shadow);
    let (ex, ey) = (
        if right { width - ARM - 1 } else { ARM },
        if bottom { height - ARM - 1 } else { ARM },
    );
    paint::vline(canvas, ex, along_y, THICK, ramp.edge);
    paint::hline(canvas, along_x, ey, THICK, ramp.edge);
    let (rx, ry) = (
        if right { width - 2 } else { 1 },
        if bottom { height - 2 } else { 1 },
    );
    paint::put(canvas, rx, ry, ramp.shine);
}

/// The soft shadow a page casts on the cover, down and to the right.
fn page_shadow(canvas: &mut Canvas, (x, y, w, h): Area) {
    let shade = Rgba::new(20, 10, 6, 90);
    for py in y + 2..y + h + 2 {
        for (dx, s) in [(0, shade), (1, paint::faded(shade, 45))] {
            let tone = canvas.get(x + w + dx, py);
            canvas.set(x + w + dx, py, paint::over(s, tone));
        }
    }
    for px in x + 2..x + w + 2 {
        for (dy, s) in [(0, shade), (1, paint::faded(shade, 45))] {
            let tone = canvas.get(px, y + h + dy);
            canvas.set(px, y + h + dy, paint::over(s, tone));
        }
    }
}

/// A page: paper with a little grain, the pages under it showing at its outer side, and shading
/// where it curves down into the binding.
fn paper(canvas: &mut Canvas, (x, y, w, h): Area, outer_left: bool) {
    let ramp = PAPER;
    for py in y..y + h {
        for px in x..x + w {
            let from_binding = if outer_left { x + w - 1 - px } else { px - x };
            let curl = (1.0 - from_binding as f32 / 14.0).max(0.0);
            let fibre = paint::noise(px, py, 211) % 100;
            let base = match fibre {
                0..=3 => paint::mix(ramp.base, ramp.shadow, 0.35),
                4..=6 => ramp.light,
                _ => ramp.base,
            };
            canvas.set(px, py, paint::mix(base, ramp.shadow, curl * curl * 0.7));
        }
    }
    paint::hline(canvas, x, y, w, ramp.shadow);
    paint::hline(canvas, x, y + h - 1, w, ramp.edge);
    let outer = if outer_left { x } else { x + w - 1 };
    paint::vline(canvas, outer, y, h, ramp.edge);
    let step = if outer_left { -1 } else { 1 };
    for (depth, tone) in [(1, ramp.shadow), (2, ramp.edge)] {
        paint::vline(canvas, outer + step * depth, y + depth, h - depth, tone);
        paint::hline(canvas, x + depth * step.max(0), y + h - 1 + depth, w, tone);
    }
}

/// Where the two pages meet: the crease, with the binding's thread showing in it.
fn binding(canvas: &mut Canvas, left: Area, right: Area) {
    let fold = left.0 + left.2;
    let (top, bottom) = (
        left.1.max(right.1),
        (left.1 + left.3).min(right.1 + right.3),
    );
    for x in fold..right.0 {
        paint::vline(canvas, x, top, bottom - top, PAPER.edge);
    }
    let mut y = top + 8;
    while y + 6 < bottom {
        paint::vline(canvas, fold, y, 4, STITCH.light);
        paint::put(canvas, fold, y + 4, STITCH.shadow);
        y += 24;
    }
}

/// The notes page's ruling, and its margin line.
fn ruled(canvas: &mut Canvas, (x, y, w, h): Area, (first, pitch): (i32, i32)) {
    if pitch > 0 {
        let mut line = y + first;
        while line < y + h - 4 {
            for px in x + 3..x + w - 3 {
                if !paint::chance(px, line, 223, 18) {
                    canvas.set(px, line, RULE);
                }
            }
            line += pitch;
        }
    }
    paint::vline(canvas, x + 8, y + 1, h - 2, MARGIN);
}

/// The cloth patch on the cover Battle's name is written on: a woven green, stitched down.
fn patch(canvas: &mut Canvas, (x, y, w, h): Area) {
    if w <= 0 || h <= 0 {
        return;
    }
    let ramp = CLOTH;
    for py in y + 1..y + h + 1 {
        for px in x + 1..x + w + 1 {
            let tone = canvas.get(px, py);
            canvas.set(px, py, paint::over(Rgba::new(10, 5, 3, 110), tone));
        }
    }
    for py in y..y + h {
        for px in x..x + w {
            let tone = if (px + py) % 2 == 0 {
                ramp.base
            } else {
                ramp.light
            };
            let tone = if paint::chance(px, py, 227, 24) {
                ramp.shadow
            } else {
                tone
            };
            canvas.set(px, py, tone);
        }
    }
    stepped_edge(canvas, (x, y, w, h), ramp.edge);
    let mut along = x + 3;
    while along + 2 < x + w - 2 {
        paint::hline(canvas, along, y + 2, 2, STITCH.base);
        paint::hline(canvas, along, y + h - 3, 2, STITCH.shadow);
        along += 4;
    }
}

/// A one-pixel outline with its corner pixels left out: the stepped corner every box in
/// Formiga's notebooks has rather than a rounded one.
fn stepped_edge(canvas: &mut Canvas, (x, y, w, h): Area, colour: Rgba) {
    paint::hline(canvas, x + 1, y, w - 2, colour);
    paint::hline(canvas, x + 1, y + h - 1, w - 2, colour);
    paint::vline(canvas, x, y + 1, h - 2, colour);
    paint::vline(canvas, x + w - 1, y + 1, h - 2, colour);
    for (cx, cy) in [
        (x, y),
        (x + w - 1, y),
        (x, y + h - 1),
        (x + w - 1, y + h - 1),
    ] {
        canvas.set(cx, cy, Rgba::new(0, 0, 0, 0));
    }
}

/// A tab on the cover: the open one a slip of paper stitched on at both ends.
fn tab_slip(canvas: &mut Canvas, tab: &Tab) {
    if !tab.open {
        return;
    }
    let (x, y, w, h) = tab.at;
    let ramp = PAPER;
    paint::rect(canvas, x + 1, y + 1, w - 2, h - 2, ramp.base);
    paint::hline(canvas, x + 1, y + 1, w - 2, ramp.light);
    paint::hline(canvas, x + 1, y + h - 2, w - 2, ramp.shadow);
    stepped_edge(canvas, tab.at, ramp.edge);
    paint::put(canvas, x + 2, y + h / 2, STITCH.edge);
    paint::put(canvas, x + w - 3, y + h / 2, STITCH.edge);
}

/// The size every printed label is set at, as Home's notebook sets them.
pub const LABEL: f32 = 10.5;

/// A printed label: small spaced capitals.
pub fn label_job(text: &str, size: f32, color: Color32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &text.to_uppercase(),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::monospace(size),
            color,
            extra_letter_spacing: (size * 0.08).round().max(0.5),
            ..Default::default()
        },
    );
    job
}

/// A section's printed label on a page.
pub fn kicker(ui: &mut egui::Ui, text: &str) {
    ui.label(label_job(text, LABEL, ink::FOREST));
}

/// A one-unit outline round `rect` with its corners left out: the notebook's stepped edge.
pub fn stepped(painter: &egui::Painter, rect: egui::Rect, unit: f32, colour: Color32) {
    let (l, r, t, b) = (rect.left(), rect.right(), rect.top(), rect.bottom());
    for strip in [
        egui::Rect::from_min_max(egui::pos2(l + unit, t), egui::pos2(r - unit, t + unit)),
        egui::Rect::from_min_max(egui::pos2(l + unit, b - unit), egui::pos2(r - unit, b)),
        egui::Rect::from_min_max(egui::pos2(l, t + unit), egui::pos2(l + unit, b - unit)),
        egui::Rect::from_min_max(egui::pos2(r - unit, t + unit), egui::pos2(r, b - unit)),
    ] {
        painter.rect_filled(strip, 0.0, colour);
    }
}

/// A slip of card with a stepped edge and a hard shadow under it: the notebook's card.
pub fn card(painter: &egui::Painter, rect: egui::Rect, unit: f32, fill: Color32, edge: Color32) {
    let shadow = rect.translate(egui::vec2(unit, unit));
    painter.rect_filled(shadow.shrink(unit), 0.0, ink::SHADOW);
    stepped(painter, shadow, unit, ink::SHADOW);
    painter.rect_filled(rect.shrink(unit), 0.0, fill);
    stepped(painter, rect, unit, edge);
}

/// How a button looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A card-coloured button for an everyday choice: MOVE, HOLD, CONTEST.
    Plain,
    /// The one thing the page is for: forest green with cream lettering and a brass rim.
    Primary,
    /// A word on the page to click, with no card round it.
    Quiet,
    /// The way back, or something undone: a card-coloured button that shakes rather than
    /// sparkles when pressed.
    Back,
}

/// A button in the notebook's way, after Hill's: printed capitals on a slip of card with a
/// stepped edge and a lip along its foot, mint when pointed at or chosen; pressing it takes the
/// lip away and sinks the lettering a unit, so it feels pressed.
pub fn button(
    ui: &mut egui::Ui,
    text: &str,
    kind: Kind,
    selected: bool,
    enabled: bool,
    width: Option<f32>,
    unit: f32,
) -> egui::Response {
    let size = match kind {
        Kind::Primary => 13.0,
        _ => LABEL,
    };
    let colour = match kind {
        Kind::Primary => ink::COVER,
        _ if !enabled => ink::MUTED,
        _ => ink::PAGE,
    };
    let galley = ui.painter().layout_job(label_job(text, size, colour));
    let pad = egui::vec2(
        unit * 5.0,
        unit * if kind == Kind::Primary { 5.0 } else { 3.0 },
    );
    let natural = galley.size() + pad * 2.0 + egui::vec2(0.0, unit);
    let size = egui::vec2(width.unwrap_or(natural.x).max(natural.x), natural.y);
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let pressed = enabled && response.is_pointer_button_down_on();
    let feel = match kind {
        Kind::Primary => crate::feel::Feel::Confirm,
        Kind::Back => crate::feel::Feel::Cancel,
        _ => crate::feel::Feel::Select,
    };
    let press = crate::feel::press(ui.ctx(), response.id, &response, feel);
    ui.with_visual_transform(press.transform(rect, unit), |ui| {
        let painter = ui.painter();
        let lip = if pressed || selected { 0.0 } else { unit };
        let face = egui::Rect::from_min_max(
            rect.min + egui::vec2(0.0, unit - lip),
            rect.max - egui::vec2(0.0, lip),
        );
        match kind {
            Kind::Quiet => {
                if response.hovered() && enabled {
                    painter.hline(
                        rect.x_range(),
                        rect.bottom() - unit,
                        egui::Stroke::new(unit, ink::FOREST),
                    );
                }
            }
            Kind::Plain | Kind::Back => {
                let fill = if !enabled {
                    ink::CARD.gamma_multiply(0.85)
                } else if selected {
                    ink::MINT
                } else if response.hovered() {
                    Color32::from_rgb(214, 238, 218)
                } else {
                    ink::CARD
                };
                if lip > 0.0 {
                    let foot = egui::Rect::from_min_max(
                        egui::pos2(rect.left(), rect.bottom() - unit * 2.0),
                        rect.max,
                    );
                    painter.rect_filled(
                        foot.shrink2(egui::vec2(unit, 0.0)),
                        0.0,
                        ink::LINE.gamma_multiply(0.35),
                    );
                }
                painter.rect_filled(face.shrink(unit), 0.0, fill);
                let edge = if selected {
                    ink::FOREST
                } else {
                    ink::LINE.gamma_multiply(if enabled { 0.7 } else { 0.35 })
                };
                stepped(painter, face, unit, edge);
            }
            Kind::Primary => {
                let (fill, rim) = if !enabled {
                    (ink::MUTED, ink::LINE.gamma_multiply(0.4))
                } else if response.hovered() {
                    (Color32::from_rgb(40, 90, 75), ink::BRASS_LIGHT)
                } else {
                    (ink::FOREST, ink::BRASS)
                };
                if lip > 0.0 {
                    let foot = egui::Rect::from_min_max(
                        egui::pos2(rect.left(), rect.bottom() - unit * 2.0),
                        rect.max,
                    );
                    painter.rect_filled(foot.shrink2(egui::vec2(unit, 0.0)), 0.0, ink::BRASS_EDGE);
                }
                painter.rect_filled(face.shrink(unit), 0.0, rim);
                painter.rect_filled(face.shrink(unit * 2.0), 0.0, fill);
                stepped(painter, face, unit, ink::BRASS_EDGE);
                // A line of light along its top, as the brass catches it.
                painter.hline(
                    (face.left() + unit * 2.0)..=(face.right() - unit * 2.0),
                    face.top() + unit * 2.5,
                    egui::Stroke::new(unit, Color32::from_rgba_unmultiplied(255, 255, 255, 30)),
                );
            }
        }
        let at = face.center() - galley.size() / 2.0;
        painter.galley(at, galley, colour);
        if let Some(through) = press.sparkle {
            crate::feel::sparkle(painter, face, through, unit);
        }
    });
    response
}

/// A brass switch on a slip of card, for a setting that is on or off: forest green behind the
/// knob when on, the knob sliding across as it changes. Flips `on` when clicked.
pub fn switch(ui: &mut egui::Ui, on: &mut bool, unit: f32) -> egui::Response {
    let size = egui::vec2(unit * 22.0, unit * 12.0);
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let press = crate::feel::press(ui.ctx(), response.id, &response, crate::feel::Feel::Select);
    let across = if crate::feel::still(ui.ctx()) {
        f32::from(u8::from(*on))
    } else {
        ui.ctx()
            .animate_bool_with_time(response.id.with("knob"), *on, 0.12)
    };
    let mark = crate::feel::mark(ui);
    let painter = ui.painter();
    let track = rect.shrink2(egui::vec2(0.0, unit * 2.0));
    let fill = if *on {
        ink::FOREST
    } else if press.hover > 0.5 {
        Color32::from_rgb(214, 238, 218)
    } else {
        ink::CARD
    };
    painter.rect_filled(
        track.translate(egui::vec2(unit, unit)).shrink(unit),
        0.0,
        ink::SHADOW,
    );
    painter.rect_filled(track.shrink(unit), 0.0, fill);
    stepped(painter, track, unit, ink::LINE.gamma_multiply(0.7));
    let knob_w = unit * 10.0;
    let x = rect.left() + (rect.width() - knob_w) * across;
    let knob =
        egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(knob_w, rect.height()));
    painter.rect_filled(knob.shrink(unit), 0.0, ink::BRASS);
    painter.rect_filled(
        egui::Rect::from_min_size(
            knob.min + egui::vec2(unit * 2.0, unit * 2.0),
            egui::vec2(knob_w - unit * 4.0, unit),
        ),
        0.0,
        ink::BRASS_LIGHT,
    );
    stepped(painter, knob, unit, ink::BRASS_EDGE);
    crate::feel::transform_since(ui, mark, press.transform(rect, unit));
    response
}

/// A dial along a ruled track, for a setting from nothing to all of it: the track filled in green
/// up to a brass slider, marked every tenth. Drag or click along it to set `value`, 0 to 1.
pub fn dial(
    ui: &mut egui::Ui,
    value: &mut f32,
    width: f32,
    enabled: bool,
    unit: f32,
) -> egui::Response {
    let size = egui::vec2(width, unit * 12.0);
    let sense = if enabled {
        egui::Sense::click_and_drag()
    } else {
        egui::Sense::hover()
    };
    let (rect, mut response) = ui.allocate_exact_size(size, sense);
    let inner = rect.shrink2(egui::vec2(unit * 4.0, 0.0));
    if enabled
        && (response.dragged() || response.clicked())
        && let Some(p) = response.interact_pointer_pos()
    {
        let v = ((p.x - inner.left()) / inner.width()).clamp(0.0, 1.0);
        // Settles on the tenths, so it is easy to set it where it was.
        let v = (v * 20.0).round() / 20.0;
        if (v - *value).abs() > f32::EPSILON {
            *value = v;
            response.mark_changed();
        }
    }
    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.drag_stopped() {
        crate::feel::emit(ui.ctx(), crate::feel::Feedback::Select);
    }
    let painter = ui.painter();
    let y = rect.center().y;
    let faint = if enabled { 1.0 } else { 0.4 };
    let track = egui::Rect::from_min_max(
        egui::pos2(inner.left(), y - unit * 1.5),
        egui::pos2(inner.right(), y + unit * 1.5),
    );
    painter.rect_filled(track.translate(egui::vec2(unit, unit)), 0.0, ink::SHADOW);
    painter.rect_filled(track, 0.0, ink::CARD.gamma_multiply(faint));
    let filled = egui::Rect::from_min_max(
        track.min,
        egui::pos2(inner.left() + inner.width() * *value, track.max.y),
    );
    painter.rect_filled(filled, 0.0, ink::FOREST.gamma_multiply(faint));
    stepped(
        painter,
        track.expand(unit),
        unit,
        ink::LINE.gamma_multiply(0.6 * faint),
    );
    for k in 0..=10 {
        let x = inner.left() + inner.width() * k as f32 / 10.0;
        let tall = if k % 5 == 0 { unit * 3.0 } else { unit * 1.5 };
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(x - unit / 2.0, track.max.y + unit * 1.5),
                egui::vec2(unit, tall),
            ),
            0.0,
            ink::LINE.gamma_multiply(0.35 * faint),
        );
    }
    let knob = egui::Rect::from_center_size(
        egui::pos2(inner.left() + inner.width() * *value, y),
        egui::vec2(unit * 5.0, unit * 10.0),
    );
    painter.rect_filled(
        knob.shrink(unit),
        0.0,
        if enabled { ink::BRASS } else { ink::MUTED },
    );
    stepped(painter, knob, unit, ink::BRASS_EDGE.gamma_multiply(faint));
    response
}

/// How long a page takes to turn, in seconds: Desktop's journal turns in a little longer, but
/// this leaf crosses two pages at once.
pub const TURN_SECONDS: f32 = 0.62;

/// A page of the notebook being turned, since `started`: forward to the next page, or back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageTurn {
    pub started: f32,
    pub forward: bool,
}

/// Paints the leaf of a page being turned over the spread, after the page it turns to has been
/// written: the leaf's free edge sweeps across from one outer edge to the other, quick to lift and
/// slow to settle as Desktop's journal turns, covering what is still the old page in blank paper
/// and shading the new one where the leaf still lies over it. False once the leaf has settled.
pub fn turn_leaf(
    painter: &egui::Painter,
    turn: PageTurn,
    now: f32,
    left: egui::Rect,
    right: egui::Rect,
    unit: f32,
) -> bool {
    let t = ((now - turn.started) / TURN_SECONDS).clamp(0.0, 1.0);
    if t >= 1.0 {
        return false;
    }
    let eased = 1.0 - (1.0 - t).powi(3);
    let paper = Color32::from_rgb(0xf9, 0xf0, 0xd4);
    let (start, end) = if turn.forward {
        (right.right(), left.left())
    } else {
        (left.left(), right.right())
    };
    let edge = start + (end - start) * eased;
    let fold = (left.right() + right.left()) / 2.0;
    // What is still the old page, which the leaf has not yet uncovered.
    let blank = |rect: egui::Rect, ruled: bool| {
        if rect.width() <= 0.0 {
            return;
        }
        painter.rect_filled(rect, 0.0, paper);
        if ruled {
            let mut y = right.top() + unit * 24.0;
            while y < right.bottom() - unit * 4.0 {
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(rect.left(), y),
                        egui::pos2(rect.right(), y + unit),
                    ),
                    0.0,
                    Color32::from_rgb(0xe9, 0xdb, 0xb2),
                );
                y += unit * 14.0;
            }
        }
    };
    let span = |a: f32, b: f32, page: egui::Rect| {
        egui::Rect::from_min_max(
            egui::pos2(a.min(b).max(page.left()), page.top()),
            egui::pos2(a.max(b).min(page.right()), page.bottom()),
        )
    };
    // The leaf itself: on the side it left, its front is the old page; on the side it is coming
    // down on, its back is the new page, shaded the nearer the edge still curling over.
    let (from, onto, from_ruled) = if turn.forward {
        (right, left, true)
    } else {
        (left, right, false)
    };
    let over_from = if turn.forward {
        edge > fold
    } else {
        edge < fold
    };
    if over_from {
        blank(onto, !from_ruled);
        let leaf = span(edge, fold, from);
        blank(leaf, from_ruled);
        painter.rect_filled(leaf, 0.0, Color32::from_black_alpha((eased * 50.0) as u8));
    } else {
        let old = if turn.forward {
            span(onto.left(), edge, onto)
        } else {
            span(edge, onto.right(), onto)
        };
        blank(old, !from_ruled);
        let leaf = span(edge, fold, onto);
        painter.rect_filled(
            leaf,
            0.0,
            Color32::from_black_alpha(((1.0 - eased) * 60.0) as u8),
        );
    }
    // A soft shadow beside the free edge, on whatever lies under it.
    let width = unit * 14.0 * (1.0 - eased);
    let away = if (turn.forward && over_from) || (!turn.forward && !over_from) {
        1.0
    } else {
        -1.0
    };
    let steps = 6;
    for k in 0..steps {
        let a = edge + away * width * k as f32 / steps as f32;
        let b = edge + away * width * (k + 1) as f32 / steps as f32;
        let page = if a.min(b) >= fold { right } else { left };
        let strip = span(a, b, page);
        let alpha = (70.0 * (1.0 - eased) * (1.0 - k as f32 / steps as f32)) as u8;
        painter.rect_filled(strip, 0.0, Color32::from_black_alpha(alpha));
    }
    let edge_line = span(
        edge - unit / 2.0,
        edge + unit / 2.0,
        if edge >= fold { right } else { left },
    );
    painter.rect_filled(edge_line, 0.0, Color32::from_rgb(0xc9, 0xb7, 0x8e));
    true
}

/// A page's heading, as Desktop's journal heads its pages: a printed line saying which page of
/// the notebook this is, the page's title, and a line on what it is for.
pub fn page_heading(ui: &mut egui::Ui, line: &str, title: &str, observation: &str) {
    ui.label(label_job(line, LABEL, ink::FOREST));
    ui.label(
        egui::RichText::new(title)
            .size(26.0)
            .color(ink::PAGE)
            .strong(),
    );
    ui.label(
        egui::RichText::new(observation)
            .size(13.5)
            .color(ink::MUTED),
    );
}

/// A little picture for a ticket's stub, a row of characters at a time: `x` in the stub's
/// cream, `o` in brass, anything else left as the stub.
pub type Icon = &'static [&'static str];

/// A house with a lit window and a door: close to home.
pub const HOUSE: Icon = &[
    "......x......",
    ".....xxx.....",
    "....xxxxx....",
    "...xxxxxxx...",
    "..xxxxxxxxx..",
    ".xxxxxxxxxxx.",
    "..xxxxxxxxx..",
    "..xooxxxoox..",
    "..xooxxxoox..",
    "..xxxxooxxx..",
    "..xxxxooxxx..",
    "..xxxxooxxx..",
];

/// The world, its seas in cream and its land in brass: out in the world.
pub const GLOBE: Icon = &[
    "....xxxxx....",
    "..xxoxxxxxx..",
    ".xoooxxxoxxx.",
    ".xooxxxxooxx.",
    "xxooxxxxxooxx",
    "xxxoxxxxxxoxx",
    "xxxxxxooxxxxx",
    "xxxxxooooxxxx",
    ".xxxxxoooxxx.",
    ".xxxxxxoxxxx.",
    "..xxxxxxxxx..",
    "....xxxxx....",
];

/// A railway ticket to somewhere, for the station's two choices: a stub in the ticket's own
/// colour with a picture of where it goes, torn along a perforation from the card, which has the
/// place printed large and a word about it stamped small and centred beneath. It lifts off the
/// page when pointed at, and springs with a brass sparkle when taken.
#[allow(clippy::too_many_arguments)]
pub fn ticket(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: egui::Id,
    title: &str,
    small: &str,
    band: Color32,
    icon: Icon,
    unit: f32,
) -> egui::Response {
    let response = ui.interact(rect, id, egui::Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let press = crate::feel::press(ui.ctx(), id, &response, crate::feel::Feel::Confirm);
    let lift = (press.hover * unit * 2.0).round();
    let pressed = response.is_pointer_button_down_on();
    let mut face = rect;
    ui.with_visual_transform(press.transform(rect, unit), |ui| {
        let painter = ui.painter();
        face = rect.translate(egui::vec2(0.0, if pressed { 0.0 } else { -lift }));
        // A hard shadow that grows as it lifts.
        let shadow = rect.translate(egui::vec2(unit * 2.0, unit * 2.0 + lift / 2.0));
        painter.rect_filled(shadow.shrink(unit), 0.0, ink::SHADOW);
        let card = if press.hover > 0.5 {
            Color32::from_rgb(0xff, 0xfa, 0xea)
        } else {
            Color32::from_rgb(0xfb, 0xf3, 0xdc)
        };
        painter.rect_filled(face.shrink(unit), 0.0, card);
        let edge = band.gamma_multiply(0.8);
        // The stub, in the ticket's colour, a little lighter when pointed at.
        let stub_w = ((face.height() * 0.95).min(face.width() * 0.3) / unit).round() * unit;
        let stub = egui::Rect::from_min_max(
            face.min + egui::vec2(unit, unit),
            egui::pos2(face.left() + stub_w, face.bottom() - unit),
        );
        let fill = if press.hover > 0.5 {
            band.lerp_to_gamma(Color32::WHITE, 0.12)
        } else {
            band
        };
        painter.rect_filled(stub, 0.0, fill);
        // Its picture, drawn a stub's pixel at a time.
        let rows = icon.len() as f32;
        let cols = icon.iter().map(|r| r.len()).max().unwrap_or(1) as f32;
        let pixel = ((stub.height() * 0.62 / rows / unit).floor().max(1.0)) * unit;
        let origin = stub.center() - egui::vec2(cols * pixel, rows * pixel) / 2.0;
        let origin = egui::pos2(
            (origin.x / unit).round() * unit,
            (origin.y / unit).round() * unit,
        );
        for (y, row) in icon.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                let colour = match c {
                    'x' => Color32::from_rgb(0xfb, 0xf3, 0xdc),
                    'o' => ink::BRASS_LIGHT,
                    _ => continue,
                };
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        origin + egui::vec2(x as f32 * pixel, y as f32 * pixel),
                        egui::vec2(pixel, pixel),
                    ),
                    0.0,
                    colour,
                );
            }
        }
        // The perforation the stub tears along, with a notch bitten out at either end.
        let mut y = face.top() + unit * 4.0;
        while y < face.bottom() - unit * 4.0 {
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(stub.right(), y),
                    egui::vec2(unit, unit * 2.0),
                ),
                0.0,
                edge,
            );
            y += unit * 4.0;
        }
        let page = Color32::from_rgb(0xf9, 0xf0, 0xd4);
        for notch_y in [face.top(), face.bottom()] {
            painter.circle_filled(
                egui::pos2(stub.right() + unit / 2.0, notch_y),
                unit * 3.0,
                page,
            );
        }
        stepped(painter, face, unit, edge);
        // The card: who it admits, the place, and the word stamped under it.
        let body =
            egui::Rect::from_min_max(egui::pos2(stub.right() + unit * 2.0, face.top()), face.max);
        let admit = painter.layout_job(label_job("Admit three", 8.5, ink::MUTED));
        painter.galley(
            body.min + egui::vec2(unit * 5.0, unit * 4.0),
            admit,
            ink::MUTED,
        );
        let big = painter.layout_job(label_job(title, 18.0, band));
        let little = painter.layout_job(label_job(small, LABEL, band));
        let stamp = egui::Rect::from_center_size(
            egui::pos2(0.0, 0.0),
            little.size() + egui::vec2(unit * 8.0, unit * 4.0),
        );
        let gap = unit * 4.0;
        let total = big.size().y + gap + stamp.height();
        let top = body.center().y - total / 2.0 + unit * 2.0;
        let big_h = big.size().y;
        painter.galley(
            egui::pos2(body.center().x - big.size().x / 2.0, top),
            big,
            band,
        );
        let stamp = egui::Rect::from_center_size(
            egui::pos2(body.center().x, top + big_h + gap + stamp.height() / 2.0),
            stamp.size(),
        );
        stepped(painter, stamp, unit, band.gamma_multiply(0.7));
        painter.galley(stamp.center() - little.size() / 2.0, little, band);
    });
    if let Some(through) = press.sparkle {
        crate::feel::sparkle(ui.painter(), face, through, unit);
    }
    response
}

/// The page's visuals: ink on paper, mint for whatever is chosen, and the notebook's plum outline
/// instead of rounded grey boxes. Home's, so a text field or a scroll bar reads the same in both.
pub fn style(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Light);
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.style_mut_of(theme, |style| {
            *style = egui::Style {
                visuals: egui::Visuals::light(),
                ..style.clone()
            };
            style.interaction.selectable_labels = false;
            let visuals = &mut style.visuals;
            visuals.override_text_color = Some(ink::PAGE);
            visuals.weak_text_color = Some(ink::MUTED);
            visuals.panel_fill = Color32::TRANSPARENT;
            visuals.window_fill = ink::CARD;
            visuals.window_stroke = egui::Stroke::new(1.0, ink::LINE);
            visuals.window_corner_radius = egui::CornerRadius::ZERO;
            visuals.menu_corner_radius = egui::CornerRadius::ZERO;
            visuals.popup_shadow = egui::Shadow {
                offset: [2, 3],
                blur: 0,
                spread: 0,
                color: Color32::from_black_alpha(60),
            };
            visuals.selection.bg_fill = ink::MINT;
            visuals.selection.stroke = egui::Stroke::new(1.0, ink::FOREST);
            visuals.extreme_bg_color = ink::CARD;
            visuals.faint_bg_color = ink::CARD;
            let widgets = &mut visuals.widgets;
            for state in [
                &mut widgets.noninteractive,
                &mut widgets.inactive,
                &mut widgets.hovered,
                &mut widgets.active,
                &mut widgets.open,
            ] {
                state.corner_radius = egui::CornerRadius::same(1);
                state.fg_stroke = egui::Stroke::new(1.0, ink::PAGE);
                state.expansion = 0.0;
            }
            widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
            widgets.noninteractive.bg_stroke =
                egui::Stroke::new(1.0, ink::LINE.gamma_multiply(0.3));
            widgets.inactive.bg_fill = ink::CARD;
            widgets.inactive.weak_bg_fill = ink::CARD;
            widgets.inactive.bg_stroke = egui::Stroke::new(1.0, ink::LINE.gamma_multiply(0.6));
            widgets.hovered.bg_fill = ink::MINT;
            widgets.hovered.weak_bg_fill = ink::MINT;
            widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ink::LINE);
            widgets.active.bg_fill = ink::MINT.gamma_multiply(0.85);
            widgets.active.weak_bg_fill = ink::MINT.gamma_multiply(0.85);
            widgets.active.bg_stroke = egui::Stroke::new(1.0, ink::FOREST);
            style.spacing.button_padding = egui::vec2(6.0, 2.0);
            style.spacing.item_spacing = egui::vec2(6.0, 4.0);
            style.spacing.scroll.floating = true;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_notebook_pixel_is_a_whole_number_of_screen_pixels() {
        for ppp in [1.0_f32, 1.25, 1.5, 2.0, 3.0] {
            let pixels = unit(ppp) * ppp;
            assert!((pixels - pixels.round()).abs() < 1e-4, "{ppp}: {pixels}");
            assert!(pixels >= 2.0);
        }
        assert_eq!(unit(2.0), 1.5);
    }
}
