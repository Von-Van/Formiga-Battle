//! The town on screen: the place the Handler is in, drawn at a whole number of screen pixels to
//! each of its own, in a dark wooden frame with the place's name above it and the keys below.
//! WASD or the arrow keys walk, E (or Enter, or Space) uses what is in front, Tab goes to the
//! next thing to use, and a click walks to a tile or to whatever stands there and uses it.
//!
//! Using something opens the page of the notebook Battle already has for it; closing that page
//! comes back to the same spot (`back_to_station`).

use super::*;
use crate::town::map::{Facing, PlaceId};
use crate::town::picture::{TILE, VIEW};
use crate::town::{Happened, Town};

/// The frame round the picture and the words on it.
pub(super) const FRAME: Color32 = Color32::from_rgb(0x5b, 0x3a, 0x27);
const FRAME_DARK: Color32 = Color32::from_rgb(0x2f, 0x1c, 0x12);
pub(super) const CREAM: Color32 = Color32::from_rgb(0xf3, 0xe6, 0xc4);
pub(super) const BRASS: Color32 = Color32::from_rgb(0xcd, 0xb1, 0x7a);
const FOREST: Color32 = Color32::from_rgb(0x1c, 0x41, 0x37);
const PAPER: Color32 = Color32::from_rgb(0xff, 0xf8, 0xe4);
const PLUM: Color32 = Color32::from_rgb(0x3b, 0x2b, 0x3a);
const INK_DARK: Color32 = Color32::from_rgb(0x23, 0x36, 0x2f);
/// The bars above and below the picture, in points.
pub(super) const BAR: f32 = 40.0;

/// The keys that walk, each with its way.
const WALK_KEYS: [(egui::Key, Facing); 8] = [
    (egui::Key::W, Facing::Up),
    (egui::Key::ArrowUp, Facing::Up),
    (egui::Key::A, Facing::Left),
    (egui::Key::ArrowLeft, Facing::Left),
    (egui::Key::S, Facing::Down),
    (egui::Key::ArrowDown, Facing::Down),
    (egui::Key::D, Facing::Right),
    (egui::Key::ArrowRight, Facing::Right),
];

impl BattleApp {
    /// Into the town, at the office desk.
    pub(super) fn enter_town(&mut self) {
        let now = self.clock;
        self.town = Some(Town::new(now));
        self.stage = Stage::Town;
        self.turn = None;
        self.turned = now;
    }

    /// Where the way back from the notebook goes: the place in town the Handler came from, or
    /// the station when there is no town. As a tab's words, a button's, and a page's name.
    pub(super) fn home_words(&self) -> (&'static str, &'static str, &'static str) {
        match self.town.as_ref().map(|t| t.place.id) {
            None => ("‹ The station", "‹ Back to the station", "The station"),
            Some(PlaceId::Office) => ("‹ The office", "‹ Back to the office", "The office"),
            Some(PlaceId::Campus) => (
                "‹ The courtyard",
                "‹ Back to the courtyard",
                "The courtyard",
            ),
            Some(PlaceId::Arena) => ("‹ The Arena", "‹ Back to the Arena", "The Arena"),
            Some(PlaceId::Clubhouse) => (
                "‹ The Clubhouse",
                "‹ Back to the Clubhouse",
                "The Clubhouse",
            ),
            Some(PlaceId::GuildHall) => (
                "‹ The Guild Hall",
                "‹ Back to the Guild Hall",
                "The Guild Hall",
            ),
            Some(PlaceId::Studio) => ("‹ The Studio", "‹ Back to the Studio", "The Studio"),
        }
    }

    /// The direction held now: the walking key pressed last among those still down.
    fn held_way(&mut self, ctx: &egui::Context) -> Option<Facing> {
        if ctx.egui_wants_keyboard_input() {
            self.walk_keys.clear();
            return None;
        }
        ctx.input(|i| {
            for (key, facing) in WALK_KEYS {
                if i.key_pressed(key) {
                    self.walk_keys.retain(|(k, _)| *k != key);
                    self.walk_keys.push((key, facing));
                }
            }
            self.walk_keys.retain(|(k, _)| i.key_down(*k));
        });
        self.walk_keys.last().map(|(_, f)| *f)
    }

    pub(super) fn town_frame(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, now: f32) {
        let held = self.held_way(ctx);
        let typing = ctx.egui_wants_keyboard_input();
        let (act, next, back) = ctx.input_mut(|i| {
            if typing {
                return (false, false, false);
            }
            let act = i.consume_key(egui::Modifiers::NONE, egui::Key::E)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::Space);
            let back = i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab);
            let next = i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
            (act, next || back, back)
        });
        let reduce = self.table.reduce_motion;
        let Some(town) = &mut self.town else {
            return;
        };
        let mut happened = town.tick(held, now);
        if next {
            town.pick_next(back);
        }
        if act && happened == Happened::Nothing {
            happened = town.act(now);
        }

        let view = town.compose(now, reduce);
        let (painter, _full, picture_rect, scale) =
            show_picture(ui, ctx, &mut self.town_picture, &view);
        let ppp = ctx.pixels_per_point();
        // Coming in through a door, the place comes up out of the dark.
        let since = now - town.entered;
        if since < crate::town::ENTER && !reduce {
            let a = 1.0 - since / crate::town::ENTER;
            painter.rect_filled(
                picture_rect,
                0.0,
                Color32::from_black_alpha((a * 255.0) as u8),
            );
        }

        // A click on the picture: walk to that tile, or to what stands there.
        let response = ui.interact(picture_rect, Id::new("town-picture"), Sense::click());
        if let Some(pos) = response
            .interact_pointer_pos()
            .filter(|_| response.clicked())
        {
            let origin = town.view_origin(now);
            let px = (pos - picture_rect.min) * ppp / scale;
            let tile = (
                (origin.0 as f32 + px.x).div_euclid(TILE as f32) as i32,
                (origin.1 as f32 + px.y).div_euclid(TILE as f32) as i32,
            );
            let clicked = town.click(tile, now);
            if clicked != Happened::Nothing {
                happened = clicked;
            }
        }

        // The prompt over what is in front, or what Tab picked.
        if let Some(target) = town.offered() {
            let origin = town.view_origin(now);
            let (hx, hy) = town.place.head_of(target);
            let at = picture_rect.min
                + vec2(
                    ((hx * TILE + TILE / 2 - origin.0) as f32) * scale / ppp,
                    ((hy * TILE - origin.1) as f32) * scale / ppp,
                );
            prompt(&painter, at, &town.prompt(target), picture_rect);
        }

        // The place's name above the picture, the Handler's on the right; the keys below.
        let name = town.place.name.to_uppercase();
        plate(
            &painter,
            pos2(picture_rect.min.x, picture_rect.min.y - BAR / 2.0),
            &name,
        );
        let who = match (self.handler.trim(), self.club.trim()) {
            ("", _) => String::new(),
            (name, "") => name.to_owned(),
            (name, club) => format!("{name} · {club}"),
        };
        painter.text(
            pos2(picture_rect.max.x, picture_rect.min.y - BAR / 2.0),
            egui::Align2::RIGHT_CENTER,
            who,
            FontId::proportional(16.0),
            CREAM,
        );
        painter.text(
            pos2(picture_rect.min.x, picture_rect.max.y + BAR / 2.0),
            egui::Align2::LEFT_CENTER,
            "WASD or arrows: walk   ·   E: use   ·   Tab: next thing   ·   or click",
            FontId::monospace(12.0),
            CREAM,
        );
        if let Host::Rehearsal { .. } = self.host {
            painter.text(
                pos2(picture_rect.max.x, picture_rect.max.y + BAR / 2.0),
                egui::Align2::RIGHT_CENTER,
                "Rehearsal: the sample colony",
                FontId::proportional(14.0),
                BRASS,
            );
        }
        self.town_happened(ctx, happened);
    }

    /// What a step, a bump, a door or a use does outside the town.
    fn town_happened(&mut self, ctx: &egui::Context, happened: Happened) {
        match happened {
            Happened::Nothing | Happened::Stepped | Happened::Bumped => {}
            Happened::Entered(_) => {
                crate::feel::emit(ctx, crate::feel::Feedback::PageTurn);
            }
            Happened::Used(place, thing) => self.use_in_town(place, &thing),
            Happened::Spoke(place, person) => self.use_in_town(place, &person),
        }
    }

    /// The page a thing or a person in town opens.
    pub(super) fn use_in_town(&mut self, place: PlaceId, what: &str) {
        let own = |step| (self.outing, step);
        let (outing, step) = match (place, what) {
            (PlaceId::Arena, "visitors" | "marlow") => (Outing::Home, Step::Where),
            (PlaceId::Arena, "friends") => (Outing::World, Step::Where),
            (PlaceId::Arena, "lessons") => (Outing::Lessons, Step::Lessons),
            (PlaceId::Clubhouse, "chalkboard") => own(Step::Playbooks),
            (PlaceId::Clubhouse, "shelves") | (PlaceId::Office, "sideboard") => own(Step::History),
            (PlaceId::GuildHall, _) | (PlaceId::Campus, "notices") => own(Step::Season),
            (PlaceId::Office, "bookcase") => own(Step::Settings),
            (PlaceId::Office, "habitat") | (PlaceId::Clubhouse, _) | (PlaceId::Studio, _) => {
                own(Step::Handler)
            }
            // The beacon and the greenhouse are only to look at.
            _ => return,
        };
        self.open_journal(outing, step);
    }
}

/// The place's name on a small green plate edged in brass.
pub(super) fn plate(painter: &egui::Painter, left_centre: egui::Pos2, words: &str) {
    let galley = painter.layout_no_wrap(words.to_owned(), FontId::monospace(13.0), CREAM);
    let rect = Rect::from_min_size(
        left_centre - vec2(0.0, galley.size().y / 2.0 + 4.0),
        galley.size() + vec2(20.0, 8.0),
    );
    painter.rect_filled(rect, 0.0, FOREST);
    painter.rect_stroke(rect, 0.0, Stroke::new(2.0, BRASS), egui::StrokeKind::Inside);
    painter.galley(rect.min + vec2(10.0, 4.0), galley, CREAM);
}

/// A prompt card over a thing: the key to press and what it does, with a notch pointing down at
/// it, kept inside the picture.
fn prompt(painter: &egui::Painter, over: egui::Pos2, words: &str, inside: Rect) {
    let text = painter.layout_no_wrap(words.to_owned(), FontId::proportional(16.0), INK_DARK);
    let key = painter.layout_no_wrap("E".to_owned(), FontId::monospace(13.0), PAPER);
    let pad = 6.0;
    let key_box = key.size() + vec2(12.0, 4.0);
    let size = vec2(
        pad + key_box.x + 8.0 + text.size().x + pad * 2.0,
        text.size().y.max(key_box.y) + pad * 2.0,
    );
    let mut min = over - vec2(size.x / 2.0, size.y + 10.0);
    min.x = min.x.clamp(
        inside.min.x + 4.0,
        (inside.max.x - size.x - 4.0).max(inside.min.x),
    );
    min.y = min.y.max(inside.min.y + 4.0);
    let card = Rect::from_min_size(min, size);
    painter.rect_filled(
        card.translate(vec2(3.0, 3.0)),
        0.0,
        Color32::from_black_alpha(70),
    );
    painter.rect_filled(card, 0.0, PAPER);
    painter.rect_stroke(card, 0.0, Stroke::new(2.0, PLUM), egui::StrokeKind::Inside);
    let key_rect = Rect::from_min_size(
        pos2(card.min.x + pad, card.center().y - key_box.y / 2.0),
        key_box,
    );
    painter.rect_filled(key_rect, 0.0, PLUM);
    painter.galley(key_rect.min + vec2(6.0, 2.0), key, PAPER);
    painter.galley(
        pos2(key_rect.max.x + 8.0, card.center().y - text.size().y / 2.0),
        text,
        INK_DARK,
    );
    let notch_x = over.x.clamp(card.min.x + 10.0, card.max.x - 10.0);
    painter.add(egui::Shape::convex_polygon(
        vec![
            pos2(notch_x - 7.0, card.max.y),
            pos2(notch_x + 7.0, card.max.y),
            pos2(notch_x, card.max.y + 7.0),
        ],
        PLUM,
        Stroke::NONE,
    ));
}

/// A picture of the town (or the arrival) at a whole number of screen pixels to each of its own,
/// in the dark wooden frame with room above and below for its words: the painter, the whole
/// window, where the picture went and how many screen pixels each of its pixels took.
pub(super) fn show_picture(
    ui: &egui::Ui,
    ctx: &egui::Context,
    slot: &mut Option<egui::TextureHandle>,
    view: &Canvas,
) -> (egui::Painter, Rect, Rect, f32) {
    let full = ui.max_rect();
    let painter = ui.painter_at(full);
    painter.rect_filled(full, 0.0, FRAME);
    let ppp = ctx.pixels_per_point();
    let room = Rect::from_min_max(
        pos2(full.min.x + 16.0, full.min.y + BAR),
        pos2(full.max.x - 16.0, full.max.y - BAR),
    );
    let scale = ((room.width() * ppp) / VIEW.0 as f32)
        .min((room.height() * ppp) / VIEW.1 as f32)
        .floor()
        .max(1.0);
    let size = vec2(VIEW.0 as f32, VIEW.1 as f32) * scale / ppp;
    let picture_rect = Rect::from_center_size(room.center(), size);
    match slot {
        Some(t) => {
            let mut image = egui::ColorImage::filled([1, 1], Color32::TRANSPARENT);
            into_image(view, &mut image);
            t.set(image, egui::TextureOptions::NEAREST);
        }
        None => *slot = Some(texture(ctx, "town", view)),
    }
    if let Some(t) = slot {
        painter.rect_filled(picture_rect.expand(3.0), 0.0, FRAME_DARK);
        painter.image(
            t.id(),
            picture_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    (painter, full, picture_rect, scale)
}
