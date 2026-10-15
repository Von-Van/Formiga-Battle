//! The window: the board on the left, scaled up whole; the Handler's cards on the right; the turn,
//! the beacon and the timer along the top. Everything it shows comes from the `Table`.

use crate::host::Host;
use crate::iso::View;
use crate::look::Look;
use crate::scene::{self, TEAM};
use crate::table::{Table, Verb};
use eframe::egui::{self, Align2, Color32, FontId, RichText, Sense, Stroke};
use formiga_art::Canvas;
use formiga_battle_contract::Contender;
use formiga_battle_core::recap::describe;
use formiga_battle_core::{Match, Order, Tile};
use formiga_battle_core::{Side, Slot, TEAM_SIZE, Victory};
use std::collections::HashMap;
use std::path::PathBuf;

const PANEL_W: f32 = 340.0;
const TOP_H: f32 = 54.0;
const INK: Color32 = Color32::from_rgb(0xf3, 0xec, 0xd8);
const FAINT: Color32 = Color32::from_rgb(0xb7, 0xb0, 0x9a);
const CARD: Color32 = Color32::from_rgb(0x33, 0x3d, 0x2c);
const NOTE: Color32 = Color32::from_rgb(0xe8, 0xd6, 0x9c);
/// What a friend match sends, said in full before the first is opened.
const SHARED: [&str; 5] = [
    "Battle sends these to Formiga's relay at formigaworld.com, and on to your friend's Battle: the name you type here; your three companions' names, looks and temperaments, and how they get on; your orders each turn; a check number for the board after each turn; and which versions of Battle and Formiga Desktop you have, with a fingerprint of your copy of Battle (the same for everyone with that copy), so only matching, unaltered copies play each other.",
    "Nothing else from your colony is sent. There is no account, no email, no device or computer ID.",
    "The relay keeps your room only while it is open. It deletes it when the match ends, or after five minutes with nobody in it, and never keeps a copy. Your seat's key is kept only in scrambled form.",
    "Like any website, formigaworld.com's host may keep its usual short-lived request logs (when a request came, and from what internet address). The relay itself logs nothing about your room.",
    "What the match earns is written to your colony on this computer when Battle closes.",
];

const CARD_CHOSEN: Color32 = Color32::from_rgb(0x4a, 0x55, 0x3a);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Picking,
    Playing,
}

/// What a review picture shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Review {
    Picking,
    Orders,
    Playing,
    /// A few matches played out of sight first, then the notebook open beside the board.
    Notebook,
    /// A room opened for a friend on the room service, its code showing.
    Room,
}

/// A review picture: after `at` seconds, save the window to `path` and close.
pub struct Snap {
    pub path: PathBuf,
    pub at: f32,
    pub asked: bool,
}

pub struct BattleApp {
    host: Host,
    table: Table,
    stage: Stage,
    view: View,
    board: Option<egui::TextureHandle>,
    portraits: HashMap<(u64, String), egui::TextureHandle>,
    hover: Option<formiga_battle_core::Tile>,
    error: Option<String>,
    /// The side panel shows the Handler's Notebook instead of the orders.
    notebook_open: bool,
    /// What the Handler types to play a friend.
    handler: String,
    join_code: String,
    /// The Handler has read what a friend match shares, this time.
    understood: bool,
    /// Every turn's orders come from the AI coach and lock at once: for review pictures.
    autoplay: bool,
    snap: Option<Snap>,
    clock: f32,
}

impl BattleApp {
    pub fn new(host: Host, table: Table, review: Option<Review>, snap: Option<Snap>) -> Self {
        let view = View::new(table.arena.width, table.arena.height);
        let mut app = Self {
            host,
            table,
            stage: Stage::Picking,
            view,
            board: None,
            portraits: HashMap::new(),
            hover: None,
            error: None,
            notebook_open: false,
            handler: String::new(),
            join_code: String::new(),
            understood: false,
            autoplay: review == Some(Review::Playing),
            snap,
            clock: 0.0,
        };
        match review {
            Some(Review::Playing) => app.begin(),
            Some(Review::Orders) => {
                app.begin();
                app.table.coach_orders();
                app.table.orders[1] = None;
                app.table.orders[2] = None;
                app.table.select(1);
                app.table.choose_verb(Verb::Move);
            }
            Some(Review::Notebook) => {
                // A colony that has played a little with friends, as Desktop would send it.
                app.table.record = formiga_battle_contract::sample::record();
                app.begin();
                app.notebook_open = true;
            }
            Some(Review::Room) => {
                app.handler = "Jake".to_owned();
                app.understood = true;
                app.table.open_room("Jake", None);
            }
            Some(Review::Picking) | None => {}
        }
        app
    }

    fn begin(&mut self) {
        match self.table.start(self.clock) {
            Ok(()) => {
                self.stage = Stage::Playing;
                self.error = None;
            }
            Err(error) => self.error = Some(format!("{error:#}")),
        }
    }

    fn portrait(&mut self, ctx: &egui::Context, contender: &Contender) -> egui::TextureHandle {
        let reduce = self.table.reduce_motion;
        let key = (contender.traveler.id.0, contender.traveler.name.clone());
        self.portraits
            .entry(key)
            .or_insert_with(|| {
                let canvas = Look::of(contender, reduce)
                    .map(|mut look| look.portrait())
                    .unwrap_or_else(|_| Canvas::new(48, 48));
                texture(
                    ctx,
                    &format!("portrait-{:x}", contender.traveler.id.0),
                    &canvas,
                )
            })
            .clone()
    }

    fn frame(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.clock = ctx.input(|input| input.time) as f32;
        let now = self.clock;
        if self.table.friend.is_some() {
            if self.table.poll(now) {
                self.stage = Stage::Playing;
                self.notebook_open = false;
            }
            ctx.request_repaint();
        }
        if self.stage == Stage::Playing {
            if self.autoplay && self.table.shown.is_none() && !self.table.finished(now) {
                self.table.coach_orders();
                self.table.lock(now);
            }
            if self.table.shown.is_none()
                && self.table.outcome().is_none()
                && self.table.time_left(now) <= 0.0
            {
                self.table.lock(now);
            }
            ctx.request_repaint();
        }
        self.keys(&ctx);
        let full = ui.max_rect();
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(color(scene::BACKDROP)))
            .show(ui, |ui| {
                let top = egui::Rect::from_min_size(full.min, egui::vec2(full.width(), TOP_H));
                let side = egui::Rect::from_min_max(
                    egui::pos2(full.max.x - PANEL_W, full.min.y + TOP_H),
                    full.max,
                );
                let stage = egui::Rect::from_min_max(
                    egui::pos2(full.min.x, full.min.y + TOP_H),
                    egui::pos2(full.max.x - PANEL_W, full.max.y),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(top.shrink(10.0)), |ui| {
                    self.top_bar(ui, now);
                });
                match self.stage {
                    Stage::Picking => {
                        ui.scope_builder(
                            egui::UiBuilder::new().max_rect(stage.shrink(16.0)),
                            |ui| {
                                self.picking(ui, &ctx);
                            },
                        );
                    }
                    Stage::Playing => {
                        self.board_view(ui, &ctx, stage, now);
                        ui.scope_builder(
                            egui::UiBuilder::new().max_rect(side.shrink(10.0)),
                            |ui| {
                                self.side_panel(ui, &ctx, now);
                            },
                        );
                    }
                }
            });
        self.take_snap(&ctx);
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if self.stage != Stage::Playing || self.table.shown.is_some() {
            return;
        }
        let pressed = |key| ctx.input(|i| i.key_pressed(key));
        if pressed(egui::Key::M) {
            self.table.choose_verb(Verb::Move);
        }
        if pressed(egui::Key::H) {
            self.table.choose_verb(Verb::Hold);
        }
        if pressed(egui::Key::C) {
            self.table.choose_verb(Verb::Contest);
        }
        if pressed(egui::Key::Escape) {
            self.table.verb = None;
        }
        if pressed(egui::Key::Enter) {
            self.table.lock(self.clock);
        }
        for (key, index) in [
            (egui::Key::Num1, 0),
            (egui::Key::Num2, 1),
            (egui::Key::Num3, 2),
        ] {
            if pressed(key) {
                self.table.select(index);
            }
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui, now: f32) {
        ui.horizontal_centered(|ui| {
            ui.label(
                RichText::new("Formiga Battle")
                    .size(22.0)
                    .color(INK)
                    .strong(),
            );
            ui.label(
                RichText::new(format!("  {}", self.table.arena.name))
                    .size(16.0)
                    .color(FAINT),
            );
            let Some(m) = &self.table.m else {
                ui.label(RichText::new(format!("   {}", self.host.describe())).color(FAINT));
                return;
            };
            ui.add_space(24.0);
            let turn = m.board.turn + u8::from(self.table.shown.is_none() && m.outcome().is_none());
            ui.label(
                RichText::new(format!("Turn {} of {}", turn.max(1), m.ruleset.turn_cap))
                    .size(16.0)
                    .color(INK),
            );
            ui.add_space(24.0);
            let shown = self.table.frame_capture(now);
            for side in Side::BOTH {
                let team = TEAM[side.index()];
                ui.label(
                    RichText::new(if side == self.table.you {
                        "You".to_owned()
                    } else {
                        self.table.other_short()
                    })
                    .color(color(team.light)),
                );
                beacon_bar(ui, shown[side.index()], m.ruleset.capture_ticks, team);
                ui.add_space(12.0);
            }
            if self.table.shown.is_none() && m.outcome().is_none() {
                let left = self.table.time_left(now).ceil() as u32;
                let warn = left <= 10;
                ui.add_space(12.0);
                ui.label(RichText::new(format!("{left}s")).size(20.0).color(if warn {
                    Color32::from_rgb(0xff, 0x9a, 0x7a)
                } else {
                    INK
                }));
                let other = color(TEAM[self.table.you.other().index()].light);
                match &self.table.friend {
                    None => {
                        ui.label(RichText::new("Visitors ready").color(other));
                    }
                    Some(f) if f.opponent_away => {
                        ui.label(
                            RichText::new(format!("{} is away", self.table.other_short()))
                                .color(FAINT),
                        );
                    }
                    Some(f) if f.opponent_ready => {
                        ui.label(
                            RichText::new(format!("{} is ready", self.table.other_short()))
                                .color(other),
                        );
                    }
                    Some(_) => {
                        ui.label(
                            RichText::new(format!("{} is choosing", self.table.other_short()))
                                .color(FAINT),
                        );
                    }
                }
            }
        });
    }

    fn picking(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label(RichText::new("Choose three to play").size(20.0).color(INK));
        ui.label(
            RichText::new(
                "You are their Handler: each turn you give each of them one order, and they carry it out their own way.",
            )
            .color(FAINT),
        );
        let record = &self.table.record;
        let mut standing = vec![format!("Open Circuit points: {}", record.circuit_points)];
        standing.push(format!(
            "Backyard Cup: {} of {} wins",
            record
                .cup_wins(crate::rewards::BACKYARD_CUP)
                .min(crate::rewards::CUP_WINS),
            crate::rewards::CUP_WINS
        ));
        if record
            .unlocked
            .iter()
            .any(|u| u == crate::rewards::WOODLAND_CUP)
        {
            standing.push("Woodland Cup unlocked".to_owned());
        }
        ui.label(RichText::new(standing.join("   ·   ")).color(NOTE));
        ui.label(
            RichText::new(
                "Friend matches earn Open Circuit points, match history and notebook entries. Matches against the visitors earn cup progress, journal entries and keepsakes.",
            )
            .size(12.0)
            .color(FAINT),
        );
        ui.add_space(10.0);
        let count = self.table.home.len();
        let per_row = ((ui.available_width() + 12.0) / 262.0).floor().max(1.0) as usize;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for row in (0..count).collect::<Vec<_>>().chunks(per_row) {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    for &place in row {
                        let contender = self.table.home[place].0.clone();
                        let texture = self.portrait(ctx, &contender);
                        let chosen = self.table.picked.contains(&place);
                        let response = egui::Frame::new()
                            .fill(if chosen { CARD_CHOSEN } else { CARD })
                            .stroke(if chosen {
                                Stroke::new(2.0, color(TEAM[0].light))
                            } else {
                                Stroke::new(2.0, CARD)
                            })
                            .corner_radius(8.0)
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.set_width(226.0);
                                ui.set_min_height(118.0);
                                ui.horizontal_top(|ui| {
                                    ui.add(egui::Image::new((
                                        texture.id(),
                                        egui::vec2(96.0, 96.0),
                                    )));
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(&contender.traveler.name)
                                                .size(17.0)
                                                .color(INK)
                                                .strong(),
                                        );
                                        ui.label(
                                            RichText::new(&contender.traveler.character.phrase)
                                                .color(FAINT),
                                        );
                                        ui.label(
                                            RichText::new(
                                                contender.traveler.character.traits.join(" · "),
                                            )
                                            .size(12.0)
                                            .color(FAINT),
                                        );
                                        let note = match self.table.page(contender.traveler.id.0) {
                                            Some(page) => {
                                                page.notes().into_iter().next().unwrap_or_else(
                                                    || {
                                                        "In your notebook; no habits noted yet"
                                                            .to_owned()
                                                    },
                                                )
                                            }
                                            None => "Not in your notebook yet".to_owned(),
                                        };
                                        ui.label(
                                            RichText::new(note).size(12.0).italics().color(NOTE),
                                        );
                                    });
                                });
                            })
                            .response
                            .interact(Sense::click());
                        if response.clicked() {
                            self.table.toggle_pick(place);
                        }
                    }
                });
                ui.add_space(8.0);
            }
            ui.add_space(14.0);
            let ready = self.table.picked.len() == TEAM_SIZE;
            let label = if ready {
                "Play the visitors in the Backyard".to_owned()
            } else {
                format!("Choose {} more", TEAM_SIZE - self.table.picked.len())
            };
            if ui
                .add_enabled(
                    ready,
                    egui::Button::new(RichText::new(label).size(17.0))
                        .min_size(egui::vec2(280.0, 38.0)),
                )
                .clicked()
            {
                self.begin();
            }
            ui.add_space(18.0);
            self.lobby(ui);
            if let Some(error) = &self.error {
                ui.colored_label(Color32::from_rgb(0xff, 0x9a, 0x7a), error);
            }
        });
    }

    fn board_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, area: egui::Rect, now: f32) {
        let Some(frame) = self.table.frame(now) else {
            return;
        };
        let marks = self.table.marks(self.hover, now);
        let m = self.table.m.as_ref().expect("a match is on");
        let canvas = scene::paint(
            &m.arena,
            &frame,
            &mut self.table.looks,
            &marks,
            m.ruleset.capture_ticks,
            now,
            &self.view,
        );
        let (w, h) = (canvas.width() as f32, canvas.height() as f32);
        let scale = ((area.width() - 16.0) / w)
            .min((area.height() - 16.0) / h)
            .floor()
            .max(1.0);
        let rect = egui::Rect::from_center_size(area.center(), egui::vec2(w * scale, h * scale));
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [canvas.width() as usize, canvas.height() as usize],
            &canvas.rgba_bytes(),
        );
        match &mut self.board {
            Some(texture) => texture.set(image, egui::TextureOptions::NEAREST),
            None => {
                self.board = Some(ctx.load_texture("board", image, egui::TextureOptions::NEAREST))
            }
        }
        let texture = self.board.as_ref().expect("just made").id();
        let response = ui.allocate_rect(rect, Sense::click());
        ui.painter().image(
            texture,
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let to_tile = |pos: egui::Pos2| {
            let local = (pos - rect.min) / scale;
            self.view.tile_at(local.x, local.y)
        };
        self.hover = response.hover_pos().and_then(to_tile);
        if self.table.shown.is_none()
            && response.clicked()
            && let Some(tile) = response.interact_pointer_pos().and_then(to_tile)
        {
            self.table.click(tile);
        }
        if response.secondary_clicked() {
            self.table.verb = None;
        }
        // Names over whoever is pointed at or chosen.
        let painter = ui.painter_at(rect);
        for slot in Slot::all() {
            let f = frame.figure(slot);
            if f.out {
                continue;
            }
            let pointed = self.hover == Some(f.tile) || marks.selected == Some(slot);
            if !pointed {
                continue;
            }
            let look = &self.table.looks[slot.side.index()][slot.index as usize];
            let (hx, hy) = scene::head(&self.view, f, look);
            let at = rect.min + egui::vec2(hx as f32, hy as f32 - 5.0) * scale;
            let text = painter.layout_no_wrap(look.name.clone(), FontId::proportional(14.0), INK);
            let back = egui::Rect::from_center_size(
                at - egui::vec2(0.0, 10.0),
                text.size() + egui::vec2(10.0, 4.0),
            );
            painter.rect_filled(back, 4.0, Color32::from_black_alpha(170));
            painter.text(
                at - egui::vec2(0.0, 10.0),
                Align2::CENTER_CENTER,
                &look.name,
                FontId::proportional(14.0),
                INK,
            );
        }
    }

    fn side_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, now: f32) {
        if self.table.finished(now) {
            self.result(ui);
            return;
        }
        let you = self.table.you;
        let sent = self.table.friend.as_ref().is_some_and(|f| {
            self.table
                .m
                .as_ref()
                .is_some_and(|m| f.sent(m.board.turn + 1))
        });
        let giving = self.table.shown.is_none() && !sent;
        let Some(m) = &self.table.m else {
            return;
        };
        let standing: Vec<bool> = (0..TEAM_SIZE as u8)
            .map(|i| m.standing(Slot::new(you, i)))
            .collect();
        let composure: Vec<u8> = (0..TEAM_SIZE as u8)
            .map(|i| m.fighter(Slot::new(you, i)).composure)
            .collect();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if self.notebook_open {
                    "Your notebook"
                } else if giving {
                    "Your orders"
                } else {
                    "The turn plays out"
                })
                .size(18.0)
                .color(INK),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .selectable_label(self.notebook_open, "Notebook")
                    .on_hover_text("What you have seen each of them do")
                    .clicked()
                {
                    self.notebook_open = !self.notebook_open;
                }
            });
        });
        ui.add_space(4.0);
        if self.notebook_open {
            self.notebook(ui, ctx);
            return;
        }
        for i in 0..TEAM_SIZE as u8 {
            let contender = self.table.board_teams[you.index()][i as usize].clone();
            let texture = self.portrait(ctx, &contender);
            let chosen = giving && self.table.selected == Some(i);
            let name = contender.traveler.name.clone();
            let order = self.table.orders[i as usize];
            let order_text = match (standing[i as usize], order) {
                (false, _) => "Knocked out for this match".to_owned(),
                (true, Some(order)) => speak(self.table.m.as_ref().expect("on"), &order),
                (true, None) => match self.table.fallback(i) {
                    Some(f) => format!(
                        "No order yet. Left to itself: {}",
                        speak(self.table.m.as_ref().expect("on"), &f)
                    ),
                    None => "No order yet".to_owned(),
                },
            };
            let response = egui::Frame::new()
                .fill(if chosen { CARD_CHOSEN } else { CARD })
                .stroke(if chosen {
                    Stroke::new(2.0, color(TEAM[0].light))
                } else {
                    Stroke::NONE
                })
                .corner_radius(8.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.add(egui::Image::new((texture.id(), egui::vec2(64.0, 64.0))));
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("{} {}", i + 1, name))
                                        .size(16.0)
                                        .color(INK)
                                        .strong(),
                                );
                                pips(ui, composure[i as usize], TEAM[0]);
                            });
                            ui.label(RichText::new(&order_text).size(13.0).color(FAINT));
                            if giving && standing[i as usize] {
                                ui.horizontal(|ui| {
                                    for verb in Verb::ALL {
                                        let on = chosen && self.table.verb == Some(verb);
                                        let button = egui::Button::new(
                                            RichText::new(verb.word()).size(13.0),
                                        )
                                        .selected(on);
                                        if ui.add(button).on_hover_text(verb.hint()).clicked() {
                                            self.table.select(i);
                                            self.table.choose_verb(verb);
                                        }
                                    }
                                    if order.is_some()
                                        && ui
                                            .small_button("clear")
                                            .on_hover_text("Clear this order")
                                            .clicked()
                                    {
                                        self.table.clear_order(i);
                                    }
                                });
                            }
                        });
                    });
                })
                .response
                .interact(Sense::click());
            if giving && response.clicked() {
                self.table.select(i);
            }
            ui.add_space(4.0);
        }
        if giving {
            ui.add_space(4.0);
            let hint = match (self.table.selected, self.table.verb) {
                (Some(_), Some(Verb::Move)) => "Click a tile to walk to. Right-click to cancel.",
                (Some(_), Some(Verb::Hold)) => "Click a tile to hold. Right-click to cancel.",
                (Some(_), Some(Verb::Contest)) => "Click someone of theirs, or the lever.",
                _ => {
                    "Choose MOVE, HOLD or CONTEST, then where. Anyone left without an order chooses for itself when the timer runs out."
                }
            };
            ui.label(RichText::new(hint).size(13.0).color(FAINT));
            ui.add_space(6.0);
            if ui
                .add(
                    egui::Button::new(RichText::new("Lock in orders (Enter)").size(16.0))
                        .min_size(egui::vec2(ui.available_width(), 34.0)),
                )
                .clicked()
            {
                self.table.lock(now);
            }
        }
        if sent && self.table.shown.is_none() {
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!(
                    "Orders sent. Waiting for {}…",
                    self.table.other_short()
                ))
                .color(NOTE),
            );
        }
        if let Some(notice) = self.table.notice.clone() {
            ui.add_space(6.0);
            ui.colored_label(Color32::from_rgb(0xff, 0xc0, 0x8a), &notice);
        }
        ui.add_space(10.0);
        ui.separator();
        self.recap(ui);
    }

    fn recap(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for (turn, lines) in self.table.log.iter().rev().take(3).rev() {
                    ui.label(
                        RichText::new(format!("Turn {turn}"))
                            .size(13.0)
                            .color(INK)
                            .strong(),
                    );
                    for line in lines {
                        ui.label(RichText::new(format!("· {line}")).size(13.0).color(FAINT));
                    }
                    ui.add_space(4.0);
                }
            });
    }

    fn result(&mut self, ui: &mut egui::Ui) {
        let you = self.table.you;
        let outcome = self.table.outcome().expect("finished");
        let (title, why) = match (outcome.winner, outcome.victory) {
            (Some(side), Some(Victory::Capture)) if side == you => (
                "You lit the beacon!",
                "Your team held the yard long enough.",
            ),
            (Some(side), Some(Victory::Knockout)) if side == you => {
                ("You win!", "The visitors are all sitting out, dizzy.")
            }
            (Some(side), _) if side == you => (
                "You win on the tally",
                "Time ran out with your team ahead on the beacon.",
            ),
            (Some(_), Some(Victory::Capture)) => (
                "The visitors lit the beacon",
                "They held the yard long enough.",
            ),
            (Some(_), Some(Victory::Knockout)) => (
                "The visitors win",
                "Your team are all sitting out, dizzy, and none the worse for it.",
            ),
            (Some(_), _) => (
                "The visitors win on the tally",
                "Time ran out with them ahead on the beacon.",
            ),
            (None, _) => ("A draw", "Nobody could be split."),
        };
        ui.label(RichText::new(title).size(24.0).color(INK).strong());
        ui.label(RichText::new(why).color(FAINT));
        if let Some(reason) = &self.table.ended_because {
            ui.label(RichText::new(reason).color(FAINT));
        }
        ui.label(RichText::new(format!("After {} turns", outcome.turn)).color(FAINT));
        ui.add_space(10.0);
        if ui
            .add(
                egui::Button::new(RichText::new("Play again").size(16.0))
                    .min_size(egui::vec2(ui.available_width(), 34.0)),
            )
            .clicked()
        {
            self.table.seed = self
                .table
                .seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.table.shown = None;
            self.portraits.clear();
            self.stage = Stage::Picking;
        }
        if !self.table.earned.is_empty() {
            ui.add_space(10.0);
            ui.label(RichText::new("Earned").size(15.0).color(NOTE));
            for line in &self.table.earned {
                ui.label(RichText::new(format!("· {line}")).size(13.0).color(FAINT));
            }
            let kept = if self.table.session.is_some() {
                "Kept in your colony when Battle closes."
            } else {
                "A rehearsal: nothing is kept."
            };
            ui.label(RichText::new(kept).size(12.0).italics().color(FAINT));
        }
        ui.add_space(10.0);
        ui.separator();
        self.recap(ui);
    }

    /// Playing a friend: open a room and pass on its code, or join one with the code a friend
    /// gave. What is shared is said here, every time, before anything is sent.
    fn lobby(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.label(RichText::new("Play a friend").size(18.0).color(INK));
        if let Some(friend) = &self.table.friend {
            match &friend.code {
                Some(code) => {
                    ui.label(
                        RichText::new("Your room is open. Give your friend this code:")
                            .color(FAINT),
                    );
                    ui.label(RichText::new(code).size(30.0).color(NOTE).strong());
                    ui.label(RichText::new("The match begins when they join.").color(FAINT));
                }
                None => {
                    ui.label(RichText::new("Opening the room…").color(FAINT));
                }
            }
            if ui.button("Cancel").clicked() {
                self.table.leave_room();
            }
        } else {
            egui::Frame::new()
                .fill(CARD)
                .corner_radius(8.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.set_max_width(640.0);
                    ui.label(
                        RichText::new("What a friend match shares")
                            .color(INK)
                            .strong(),
                    );
                    for line in SHARED {
                        ui.label(RichText::new(format!("· {line}")).size(13.0).color(FAINT));
                    }
                    ui.checkbox(&mut self.understood, "I understand what is shared");
                });
            let ready = self.table.picked.len() == TEAM_SIZE && self.understood;
            ui.horizontal(|ui| {
                ui.label(RichText::new("Your name for your friend").color(FAINT));
                ui.add(egui::TextEdit::singleline(&mut self.handler).desired_width(180.0));
            });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(ready, egui::Button::new("Open a room for a friend"))
                    .clicked()
                {
                    let handler = self.handler.clone();
                    self.table.open_room(&handler, None);
                }
                ui.label(RichText::new("or").color(FAINT));
                ui.add(
                    egui::TextEdit::singleline(&mut self.join_code)
                        .hint_text("code")
                        .desired_width(90.0),
                );
                if ui
                    .add_enabled(
                        ready && !self.join_code.trim().is_empty(),
                        egui::Button::new("Join"),
                    )
                    .clicked()
                {
                    let (handler, code) = (self.handler.clone(), self.join_code.clone());
                    self.table.open_room(&handler, Some(&code));
                }
            });
        }
        if let Some(notice) = &self.table.notice {
            ui.colored_label(Color32::from_rgb(0xff, 0xc0, 0x8a), notice);
        }
    }

    /// The Handler's Notebook for the three on the board: what each has been seen to do.
    fn notebook(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for i in 0..TEAM_SIZE {
                let contender = self.table.board_teams[self.table.you.index()][i].clone();
                let texture = self.portrait(ctx, &contender);
                let page = self.table.page(contender.traveler.id.0);
                egui::Frame::new()
                    .fill(CARD)
                    .corner_radius(8.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_top(|ui| {
                            ui.add(egui::Image::new((texture.id(), egui::vec2(48.0, 48.0))));
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(&contender.traveler.name)
                                        .size(16.0)
                                        .color(INK)
                                        .strong(),
                                );
                                let Some(page) = page else {
                                    ui.label(
                                        RichText::new("Nothing noted yet: play a match together.")
                                            .size(13.0)
                                            .color(FAINT),
                                    );
                                    return;
                                };
                                let played = if page.matches == 1 {
                                    "Played once".to_owned()
                                } else {
                                    format!("Played {} times, won {}", page.matches, page.wins)
                                };
                                ui.label(RichText::new(played).size(12.0).color(FAINT));
                                let notes = page.notes();
                                if notes.is_empty() {
                                    ui.label(
                                        RichText::new("No habits noted yet.")
                                            .size(13.0)
                                            .italics()
                                            .color(FAINT),
                                    );
                                }
                                for note in notes.iter().take(4) {
                                    ui.label(
                                        RichText::new(format!("· {note}")).size(13.0).color(NOTE),
                                    );
                                }
                                for moment in page.moments.iter().rev().take(2) {
                                    ui.label(
                                        RichText::new(moment.as_str())
                                            .size(12.0)
                                            .italics()
                                            .color(FAINT),
                                    );
                                }
                            });
                        });
                    });
                ui.add_space(4.0);
            }
        });
    }

    /// Ask for the picture when it is time, and save it when it comes.
    fn take_snap(&mut self, ctx: &egui::Context) {
        let Some(snap) = &mut self.snap else {
            return;
        };
        if !snap.asked && self.clock >= snap.at {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            snap.asked = true;
        }
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            let [width, height] = image.size;
            let mut canvas = Canvas::new(width as u32, height as u32);
            for (index, pixel) in image.pixels.iter().enumerate() {
                let [r, g, b, a] = pixel.to_srgba_unmultiplied();
                canvas.set(
                    (index % width) as i32,
                    (index / width) as i32,
                    formiga_art::Rgba::new(r, g, b, a),
                );
            }
            if let Err(error) = crate::write_png(&snap.path, &canvas, 1) {
                eprintln!("formiga-battle: {error:#}");
            }
            self.snap = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}

impl eframe::App for BattleApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame(ui);
    }

    /// A window opened only to have its picture taken answers to nobody.
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if self.snap.is_some() {
            raw_input
                .events
                .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let c = scene::BACKDROP;
        [
            f32::from(c.r) / 255.0,
            f32::from(c.g) / 255.0,
            f32::from(c.b) / 255.0,
            1.0,
        ]
    }
}

/// An order in the Handler's words: where, by what is there, never by coordinates.
pub fn speak(m: &Match, order: &Order) -> String {
    let place = |tile: Tile| -> (&'static str, &'static str) {
        let arena = &m.arena;
        if arena.in_zone(tile) {
            ("into the yard", "in the yard")
        } else if arena.levers.iter().any(|l| l.touches(tile)) {
            ("to the lever", "by the lever")
        } else if arena.gates.contains(&tile) {
            ("to the gate", "in the gateway")
        } else if tile.x <= 2 {
            ("back to your side", "on your side")
        } else if i16::from(tile.x) >= i16::from(arena.width) - 3 {
            ("over to their side", "on their side")
        } else {
            ("across the lawn", "on the lawn")
        }
    };
    match order {
        Order::Move { to } => format!("MOVE {}", place(*to).0),
        Order::Hold { at } => format!("HOLD {}", place(*at).1),
        Order::Contest { .. } => describe(m, order),
    }
}

/// Composure as three pips: filled in the team's colour, empty in grey.
fn pips(ui: &mut egui::Ui, composure: u8, team: formiga_art::paint::Ramp) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(42.0, 14.0), Sense::hover());
    for k in 0..3u8 {
        let centre = egui::pos2(rect.min.x + 7.0 + f32::from(k) * 13.0, rect.center().y);
        if k < composure {
            ui.painter().circle_filled(centre, 5.0, color(team.base));
            ui.painter()
                .circle_stroke(centre, 5.0, Stroke::new(1.5, color(team.light)));
        } else {
            ui.painter()
                .circle_stroke(centre, 5.0, Stroke::new(1.5, Color32::from_gray(110)));
        }
    }
}

/// A team's progress toward lighting the beacon.
fn beacon_bar(ui: &mut egui::Ui, progress: u8, of: u8, team: formiga_art::paint::Ramp) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(130.0, 20.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 5.0, Color32::from_black_alpha(120));
    let share = f32::from(progress) / f32::from(of.max(1));
    if share > 0.0 {
        let fill =
            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * share, rect.height()));
        painter.rect_filled(fill, 5.0, color(team.base));
    }
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        format!("beacon {progress}/{of}"),
        FontId::proportional(13.0),
        INK,
    );
}

pub fn color(c: formiga_art::Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}

fn texture(ctx: &egui::Context, name: &str, canvas: &Canvas) -> egui::TextureHandle {
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [canvas.width() as usize, canvas.height() as usize],
        &canvas.rgba_bytes(),
    );
    ctx.load_texture(name, image, egui::TextureOptions::NEAREST)
}
