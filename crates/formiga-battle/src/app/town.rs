//! The town on screen, in the notebook every Formiga app opens in: the place the Handler is in,
//! drawn at a whole number of screen pixels to each of its own and pasted on the left page; its
//! name on the strip along the top, the keys along the foot, and on the ruled right page what
//! there is to use and where the doors go, each a way to walk there.
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
const FRAME_DARK: Color32 = Color32::from_rgb(0x2f, 0x1c, 0x12);
const PAPER: Color32 = Color32::from_rgb(0xff, 0xf8, 0xe4);
const PLUM: Color32 = Color32::from_rgb(0x3b, 0x2b, 0x3a);
const INK_DARK: Color32 = Color32::from_rgb(0x23, 0x36, 0x2f);

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
        let mut town = Town::new(now);
        town.dress(&self.table.career.look);
        town.cups = self.cups_won();
        town.news = self.table.next_event_fixture().is_some();
        town.titled = !self.table.career.titles.is_empty();
        self.town = Some(town);
        self.stage = Stage::Town;
        self.turn = None;
        self.turned = now;
    }

    /// The cups won, in the Circuit's order, for the trophy stands in town.
    pub(super) fn cups_won(&self) -> [bool; 4] {
        let record = &self.table.record;
        std::array::from_fn(|i| {
            record.cup_wins(crate::rewards::CUPS[i].id) >= crate::rewards::CUP_WINS
        })
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
        if self.habitat.is_some() {
            self.habitat_frame(ui, ctx, now);
            return;
        }
        let open_map = self.map.is_none()
            && self.speech.is_none()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::M));
        if open_map || self.map.is_some() {
            // The town stands still under the map.
            self.walk_keys.clear();
        }
        let was_talking = self.speech.is_some();
        let talking = was_talking;
        // While someone is speaking the Handler stands still and listens.
        let mapped = open_map || self.map.is_some();
        let held = self.held_way(ctx).filter(|_| !talking && !mapped);
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
        let close = talking
            && !typing
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if close {
            self.speech = None;
        } else if talking && act {
            self.speech_on();
        }
        let talking = self.speech.is_some();
        let reduce = self.table.reduce_motion;
        let Some(town) = &mut self.town else {
            return;
        };
        let mut happened = town.tick(held, now);
        if next && !talking && !mapped {
            town.pick_next(back);
        }
        if act && !was_talking && !mapped && happened == Happened::Nothing {
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
        let clicked_on = response
            .interact_pointer_pos()
            .filter(|_| response.clicked());
        // A click goes on with the conversation, wherever it lands.
        let go_on = talking && clicked_on.is_some();
        if go_on {
        } else if let Some(pos) = clicked_on {
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

        // The prompt over what is in front, or what Tab picked, unless someone is speaking.
        if let Some(target) = town.offered().filter(|_| !talking) {
            let origin = town.view_origin(now);
            let (hx, hy) = town.place.head_of(target);
            let at = picture_rect.min
                + vec2(
                    ((hx * TILE + TILE / 2 - origin.0) as f32) * scale / ppp,
                    // A tile above the thing's own top row, clear of what stands on it (the habitat on
                    // the desk, the board on the wall).
                    (((hy - 1).max(0) * TILE - origin.1) as f32) * scale / ppp,
                );
            prompt(&painter, at, &town.prompt(target), picture_rect);
        }

        if go_on {
            self.speech_on();
        }
        self.speech_card(ctx, &painter, picture_rect);
        if open_map || self.map.is_some() {
            self.map_frame(ctx, ui.max_rect());
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
        if (place, what) == (PlaceId::Office, "habitat") {
            self.open_habitat();
            return;
        }
        // Someone, or something, with something to say says it first.
        if self.talk_or_open(place, what) {
            return;
        }
        let own = |step| (self.outing, step);
        let (outing, step) = match (place, what) {
            (PlaceId::Arena, "visitors" | "marlow") => (Outing::Home, Step::Where),
            (PlaceId::Arena, "friends") => (Outing::World, Step::Where),
            (PlaceId::Arena, "lessons") => (Outing::Lessons, Step::Lessons),
            (PlaceId::Clubhouse, "chalkboard") => own(Step::Playbooks),
            (PlaceId::Clubhouse, "shelves") | (PlaceId::Office, "sideboard") => own(Step::History),
            (PlaceId::GuildHall, _) | (PlaceId::Campus, "notices") => own(Step::Season),
            (PlaceId::Office, "bookcase") => own(Step::Settings),
            (PlaceId::Studio, _) => {
                // A fresh try-on, starting from the look kept.
                self.studio_draft = None;
                own(Step::Studio)
            }
            (PlaceId::Office, "habitat") | (PlaceId::Clubhouse, _) => own(Step::Handler),
            // The beacon and the greenhouse are only to look at.
            _ => return,
        };
        self.open_journal(outing, step);
    }
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
/// pasted on the page it is given with a dark edge: the painter, the page, where the picture went
/// and how many screen pixels each of its pixels took.
pub(super) fn show_picture(
    ui: &egui::Ui,
    ctx: &egui::Context,
    slot: &mut Option<egui::TextureHandle>,
    view: &Canvas,
) -> (egui::Painter, Rect, Rect, f32) {
    let full = ui.max_rect();
    let painter = ui.painter_at(full.expand(8.0));
    let ppp = ctx.pixels_per_point();
    let room = full.shrink(6.0);
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
        painter.rect_filled(
            picture_rect.translate(vec2(3.0, 3.0)).expand(3.0),
            0.0,
            Color32::from_black_alpha(60),
        );
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

impl BattleApp {
    /// The words along the notebook's top strip in town: where the Handler is.
    pub(super) fn strip_words(&self) -> String {
        match self.stage {
            Stage::Station => "The Battleground".to_owned(),
            Stage::Arrival => "Arriving".to_owned(),
            _ if self.habitat.is_some() => "The habitat".to_owned(),
            _ => self
                .town
                .as_ref()
                .map_or_else(|| "The Battleground".to_owned(), |t| t.place.name.clone()),
        }
    }

    /// The ruled right page beside the town: where the Handler is, what there is here to use and
    /// where the doors go, each a way to walk there; in the habitat, the companion picked.
    pub(super) fn town_notes(&mut self, ui: &mut egui::Ui, u: f32) {
        let now = self.clock;
        let heading = |ui: &mut egui::Ui, words: &str| {
            ruled::write(
                ui,
                RichText::new(words)
                    .font(formiga_pencil::hand_bold(24.0))
                    .color(ink::PAGE),
            );
        };
        let line = |ui: &mut egui::Ui, words: &str, colour: Color32| {
            ui.add(egui::Label::new(RichText::new(words).size(16.0).color(colour)).wrap());
        };
        if self.stage == Stage::Arrival {
            heading(ui, "Arriving");
            ui.add_space(u * 3.0);
            let names = self.companion_names();
            if !names.is_empty() {
                line(ui, &format!("{names}, from Formiga Desktop."), ink::PAGE);
            }
            return;
        }
        if let Some(habitat) = &self.habitat {
            heading(ui, "The habitat");
            ui.add_space(u * 3.0);
            let picked = habitat.picked();
            if let Some((contender, _)) = self.table.home.get(picked) {
                let character = &contender.traveler.character;
                ruled::write(
                    ui,
                    RichText::new(&contender.traveler.name)
                        .font(formiga_pencil::hand_bold(20.0))
                        .color(ink::PAGE),
                );
                line(ui, &character.phrase, ink::MUTED);
                line(ui, &character.traits.join(" · "), ink::MUTED);
                ui.add_space(u * 3.0);
                let note = match self.table.page(contender.traveler.id.0) {
                    Some(page) => page
                        .notes()
                        .into_iter()
                        .next()
                        .unwrap_or_else(|| "No habits noted yet.".to_owned()),
                    None => "Not in my notebook yet.".to_owned(),
                };
                line(ui, &format!("In the notebook: {note}"), ink::FOREST);
            }
            ui.add_space(u * 4.0);
            line(
                ui,
                "As Formiga Desktop keeps them. Battle never changes who they are.",
                ink::MUTED,
            );
            return;
        }
        let Some(town) = &self.town else {
            return;
        };
        heading(ui, &town.place.name);
        ui.add_space(u * 4.0);
        // What there is to use here, and where the doors go: each a way to walk there.
        let mut walk_to: Option<(i32, i32)> = None;
        let targets = town.place.targets();
        if !targets.is_empty() {
            ruled::write(ui, RichText::new("Here").size(17.0).color(ink::FOREST));
            for target in targets {
                let words = town.prompt(target);
                let tile = town.place.tiles_of(target)[0];
                if chrome::button(
                    ui,
                    &words,
                    Kind::Quiet,
                    false,
                    self.speech.is_none(),
                    None,
                    u,
                )
                .clicked()
                {
                    walk_to = Some(tile);
                }
            }
            ui.add_space(u * 3.0);
        }
        ruled::write(ui, RichText::new("Doors").size(17.0).color(ink::FOREST));
        for door in &town.place.doors {
            let words = format!("To {}", place_words(door.to));
            if chrome::button(
                ui,
                &words,
                Kind::Quiet,
                false,
                self.speech.is_none(),
                None,
                u,
            )
            .clicked()
            {
                walk_to = Some(door.tile);
            }
        }
        if let (Some(tile), Some(town)) = (walk_to, &mut self.town) {
            let happened = town.click(tile, now);
            let ctx = ui.ctx().clone();
            self.town_happened(&ctx, happened);
        }
    }
}

/// How a door's other side is named on the notes page.
fn place_words(place: PlaceId) -> &'static str {
    match place {
        PlaceId::Office => "the office",
        PlaceId::Campus => "the courtyard",
        PlaceId::Arena => "the Arena",
        PlaceId::Clubhouse => "the Clubhouse",
        PlaceId::GuildHall => "the Guild Hall",
        PlaceId::Studio => "the Handler Studio",
    }
}
