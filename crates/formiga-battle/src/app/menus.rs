//! Getting ready, before a match: the Battleground, where the train brings the companions in, and the
//! notebook's pages after it, turned one by one as Desktop's journal turns its pages. Where to
//! play, then who goes and, for a friend, the room: open one and pass on its code, or join theirs.
//! Each page is a spread: what it is about on the left, and on the ruled right page what is
//! chosen so far and the way on. Field Lessons turn the same way: which lesson, then who plays
//! it.

use super::{
    BattleApp, Kind, Outing, SHARED, Stage, Step, card, dashed_rule, locked_behind, padlock,
    portrait_mat, rosette, shrunk, sticky, texture,
};
use crate::chrome::{self, ink, label_job};
use crate::feel::Feedback;
use crate::garden::Garden;
use crate::iso::View;
use crate::look::Look;
use crate::masthead::Masthead;
use crate::station::{Heard, Station};
use crate::ticket::Destination;
use eframe::egui::{self, Align2, Color32, FontId, Id, Rect, RichText, Sense, Stroke, pos2, vec2};
use formiga_battle_contract::Contender;
use formiga_battle_core::TEAM_SIZE;
use formiga_battle_core::lesson::Lesson;

/// How tall the station is drawn, in its own pixels, whatever the window: each of them as many
/// screen pixels as fit.
const STATION_ROWS: f32 = 216.0;
/// How often the picture at the head of the station's page is repainted.
const MASTHEAD_FRAMES_PER_SECOND: f32 = 12.0;
/// How often the station is repainted while anything moves at it.
const STATION_FRAMES_PER_SECOND: f32 = 30.0;
/// The station's name, lettered on its board.
const BATTLEGROUND: &str = "The Battleground";

/// How long after a page starts turning its cards start dropping onto it: once the leaf is
/// mostly over.
const TURN_DROP_DELAY: f32 = 0.16;

impl BattleApp {
    /// The pages the notebook is turning through: an outing's, or the Handler's own.
    pub(super) fn steps(&self) -> &'static [Step] {
        if self.own_pages() {
            &[
                Step::Handler,
                Step::Playbooks,
                Step::Season,
                Step::History,
                Step::Settings,
            ]
        } else {
            self.outing.steps()
        }
    }

    /// The notebook is open at the Handler's own pages, not on the way anywhere.
    pub(super) fn own_pages(&self) -> bool {
        matches!(
            self.step,
            Step::Handler | Step::Playbooks | Step::Season | Step::History | Step::Settings
        )
    }

    /// Opens the notebook at a page of getting ready for an outing, or at the Handler's own
    /// pages; a page turns as it opens, unless motion is reduced.
    pub(super) fn open_journal(&mut self, outing: Outing, step: Step) {
        self.outing = outing;
        self.step = step;
        self.stage = Stage::Journal;
        self.notebook_open = false;
        self.keep_arena_open();
        self.leaf(true);
    }

    /// A leaf of the notebook turning over now, forward or back, and the page under it dropping
    /// its cards in once it has mostly gone by.
    pub(super) fn leaf(&mut self, forward: bool) {
        let now = self.clock;
        if self.table.reduce_motion {
            self.turn = None;
            self.turned = now;
        } else {
            self.turn = Some(chrome::PageTurn {
                started: now,
                forward,
            });
            self.turned = now + TURN_DROP_DELAY;
            self.felt.push(crate::feel::Feedback::PageTurn);
        }
    }

    /// Back to the station. From the notebook, the train is waiting with everyone aboard; after a
    /// match, it comes in again for them, as it did the first time.
    pub(super) fn back_to_station(&mut self) {
        // An event fixture not yet played waits on the Season page.
        self.table.event_fixture = None;
        let now = self.clock;
        let after_match = self.stage == Stage::Playing;
        if let Some(station) = &mut self.station {
            if after_match {
                station.come_in(now);
            } else {
                station.wait_at_platform(now);
            }
        }
        self.stage = Stage::Station;
        self.turn = None;
        self.turned = now;
    }

    /// Against the visitors, only an arena whose cup is open can be chosen; close to home, a
    /// locked one chosen for a friend gives way to the Backyard.
    fn keep_arena_open(&mut self) {
        if self.outing == Outing::Home && !self.arena_open(&self.table.arena.id) {
            self.choose_arena(formiga_battle_core::Arena::backyard());
        }
    }

    fn arena_open(&self, id: &str) -> bool {
        crate::rewards::open_to_play(&self.table.record).contains(&id)
    }

    pub(super) fn choose_arena(&mut self, arena: formiga_battle_core::Arena) {
        self.view = View::new(arena.width, arena.height);
        self.table.arena = arena;
        self.garden = None;
    }

    /// A room is open and waiting for the friend: what it was opened with holds until it closes.
    fn room_waiting(&self) -> bool {
        self.table.friend.as_ref().is_some_and(|f| !f.seated)
    }

    /// Whether a page can be turned to now, and if not, why.
    pub(super) fn reachable(&self, step: Step) -> Result<(), String> {
        if self.room_waiting() && step != Step::Room && !self.own_pages() {
            return Err("Close the room first".to_owned());
        }
        if step == Step::Room && self.table.picked.len() < TEAM_SIZE {
            return Err("Choose three first".to_owned());
        }
        Ok(())
    }

    /// Turns the notebook to a page of getting ready: forward or back, the leaf crossing the
    /// spread; at once under reduced motion.
    pub(super) fn turn_to(&mut self, step: Step, now: f32) {
        if step == self.step || self.reachable(step).is_err() {
            return;
        }
        let forward = step > self.step;
        self.step = step;
        self.clock = now;
        self.leaf(forward);
    }

    /// The leather along the top while getting ready: the way back to the station, and where the
    /// companions are off to.
    pub(super) fn journal_strip(&mut self, ui: &mut egui::Ui, area: Rect, u: f32, _now: f32) {
        let waiting = self.room_waiting() && !self.own_pages();
        let own = self.own_pages();
        let mut back = false;
        ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = u * 4.0;
                let id = Id::new("to-station");
                let hovered = ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
                let colour = if waiting {
                    ink::DEBOSS.gamma_multiply(0.45)
                } else if hovered {
                    ink::COVER
                } else {
                    ink::DEBOSS
                };
                let response = ui.add(
                    egui::Label::new(label_job("‹ The station", chrome::LABEL, colour)).sense(
                        if waiting {
                            Sense::hover()
                        } else {
                            Sense::click()
                        },
                    ),
                );
                ui.ctx()
                    .data_mut(|d| d.insert_temp(id, response.hovered() && !waiting));
                if response.hovered() && !waiting {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                let response = if waiting {
                    response.on_hover_text("Close the room first")
                } else {
                    response
                        .on_hover_text("Back to the train, to choose again where they are off to")
                };
                back = response.clicked() && !waiting;
                ui.add_space(u * 6.0);
                if own {
                    ui.label(label_job(
                        "The Handler's notebook",
                        chrome::LABEL,
                        ink::COVER,
                    ));
                } else {
                    ui.label(label_job(self.outing.name(), chrome::LABEL, ink::COVER));
                    ui.label(label_job(
                        match self.outing {
                            Outing::World => "· online",
                            Outing::Home | Outing::Lessons => "· offline",
                        },
                        chrome::LABEL,
                        ink::DEBOSS,
                    ));
                }
            });
        });
        if back {
            self.back_to_station();
        }
    }

    /// What to do next, along the foot, while getting ready.
    pub(super) fn journal_hint(&self) -> String {
        match self.step {
            Step::Lessons => {
                "Choose a lesson, then turn the page to choose who plays it.".to_owned()
            }
            Step::Where => match self.outing {
                Outing::Home | Outing::Lessons => {
                    "Choose where to play the visiting team, then turn the page."
                }
                Outing::World => "Choose where to play your friend, then turn the page.",
            }
            .to_owned(),
            Step::Who => match (self.table.picked.len(), self.outing) {
                (0, _) => {
                    "You are their Handler: choose three of your companions to go.".to_owned()
                }
                (3, Outing::Home) => "Your three are ready: play the visiting team.".to_owned(),
                (3, Outing::Lessons) => "Your three are ready: begin the lesson.".to_owned(),
                (3, Outing::World) => {
                    "Your three are ready: turn the page to your friend's room.".to_owned()
                }
                (n, _) => format!("Choose {} more.", TEAM_SIZE - n),
            },
            Step::Room => match &self.table.friend {
                Some(f) if f.code.is_some() && !f.seated => {
                    "Give your friend the code. The match begins when they join.".to_owned()
                }
                Some(_) => "Waiting for the relay…".to_owned(),
                None => {
                    "Sign your name, read what is shared, then open a room or join one.".to_owned()
                }
            },
            Step::Handler => {
                "Sign your name, choose your pennant, and look after your trios.".to_owned()
            }
            Step::Playbooks => {
                "Write each trio's plans in stages; the one it plays by fills in its orders."
                    .to_owned()
            }
            Step::Season => match self.table.next_event_fixture() {
                Some((_, round, rival)) => format!("Your {round} against {} is next.", rival.name),
                None => "Enter an event, or see how you stand against each rival.".to_owned(),
            },
            Step::History => "Every match your colony has played, newest first.".to_owned(),
            Step::Settings => "Settings are kept for next time.".to_owned(),
        }
    }

    /// Keeps the station going, whatever is drawn: the keys that hurry the train in, what is
    /// heard of it, and the notebook turning to the first page of an outing once it has gone.
    pub(super) fn station_tick(&mut self, ctx: &egui::Context, now: f32) {
        let Some(station) = &mut self.station else {
            return;
        };
        let keyed = !ctx.egui_wants_keyboard_input()
            && ctx.input(|i| {
                i.key_pressed(egui::Key::Space)
                    || i.key_pressed(egui::Key::Enter)
                    || i.key_pressed(egui::Key::Escape)
            });
        if keyed && !station.stopped(now) && !station.leaving() {
            station.skip(now);
        }
        let last = ctx
            .data(|d| d.get_temp::<f32>(Id::new("station-heard")))
            .unwrap_or(now);
        for heard in station.heard(last, now) {
            crate::feel::emit(
                ctx,
                match heard {
                    Heard::Whistle => Feedback::TrainIn,
                    Heard::Brakes => Feedback::TrainStops,
                    Heard::Away => Feedback::TrainOut,
                },
            );
        }
        ctx.data_mut(|d| d.insert_temp(Id::new("station-heard"), now));
        if station.seen_off(now) {
            let outing = self.outing;
            self.open_journal(outing, Step::Where);
        }
    }

    /// The station's left page: the Battleground as a big picture pasted onto it, the train
    /// coming in with the companions at its windows. A click on it hurries the train in.
    pub(super) fn station_left(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        u: f32,
        now: f32,
    ) {
        let area = ui.max_rect();
        let ppp = ctx.pixels_per_point();
        let mat = u * 4.0;
        let room = area.shrink(mat + u * 2.0);
        let screen = room.size() * ppp;
        // As many screen pixels to the station's pixel as fit, with the station at least as wide
        // as its train needs.
        let scale = (screen.y / STATION_ROWS)
            .floor()
            .min((screen.x / 320.0).floor())
            .max(1.0);
        let size = (
            (screen.x / scale).floor() as i32,
            (screen.y / scale).floor() as i32,
        );
        let reduce = self.table.reduce_motion;
        let station = self.station.get_or_insert_with(|| {
            let looks = self
                .table
                .home
                .iter()
                .filter_map(|(c, _)| Look::of(c, reduce).ok())
                .collect();
            Station::new(looks, size, now, reduce)
        });
        station.resize(size);
        let size = station.size();
        // The picture, repainted only as often as anything at the station moves.
        let tick = if reduce && !station.leaving() {
            0
        } else {
            (now * STATION_FRAMES_PER_SECOND) as i64
        };
        let key = tick * 1_000_000 + i64::from(size.0) * 1000 + i64::from(size.1);
        if self.station_picture.as_ref().is_none_or(|(k, _)| *k != key) {
            let picture = station.compose(now);
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [picture.width() as usize, picture.height() as usize],
                &picture.rgba_bytes(),
            );
            match &mut self.station_picture {
                Some((k, t)) => {
                    t.set(image, egui::TextureOptions::NEAREST);
                    *k = key;
                }
                None => {
                    let t = ctx.load_texture("station", image, egui::TextureOptions::NEAREST);
                    self.station_picture = Some((key, t));
                }
            }
        }
        let k = scale / ppp;
        let drawn = vec2(size.0 as f32, size.1 as f32) * k;
        let min = room.center() - drawn / 2.0;
        let min = pos2((min.x * ppp).round() / ppp, (min.y * ppp).round() / ppp);
        let photo = Rect::from_min_size(min, drawn);
        let painter = ui.painter().clone();
        // Pasted in on a mat of card, as the places' pictures are.
        chrome::card(
            &painter,
            photo.expand(mat),
            u,
            ink::CARD,
            ink::LINE.gamma_multiply(0.55),
        );
        if let Some((_, picture)) = &self.station_picture {
            painter.image(
                picture.id(),
                photo,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        let station = self.station.as_mut().expect("the station");
        let hurry = ui.interact(photo, Id::new("station-hurry"), Sense::click());
        if hurry.clicked() && !station.stopped(now) && !station.leaving() {
            station.skip(now);
        }
        // The station's name, lettered on its board.
        let at = |x: i32, y: i32| photo.min + vec2(x as f32, y as f32) * k;
        let (sx, sy, sw, sh) = station.sign();
        let board = Rect::from_min_max(at(sx, sy), at(sx + sw, sy + sh));
        let maroon = Color32::from_rgb(0x7a, 0x2c, 0x33);
        let mut letters = (board.height() * 0.42).clamp(8.0, 18.0);
        let mut lettering = painter.layout_job(label_job(BATTLEGROUND, letters, maroon));
        while lettering.size().x > board.width() * 0.88 && letters > 7.0 {
            letters -= 0.5;
            lettering = painter.layout_job(label_job(BATTLEGROUND, letters, maroon));
        }
        painter.galley(
            board.center() - lettering.size() / 2.0 + vec2(0.0, u * 0.5),
            lettering,
            maroon,
        );
    }

    /// The picture at the head of the station's page, with the page's title on its ribbon.
    fn masthead(&mut self, ui: &mut egui::Ui, u: f32, now: f32) {
        let ctx = ui.ctx().clone();
        let width = ui.available_width();
        let (w, h) = (
            (width / u).floor() as i32,
            ((width * 0.42 / u) as i32).clamp(76, 118),
        );
        let reduce = self.table.reduce_motion;
        let flag = self
            .table
            .record
            .handler
            .as_ref()
            .map(|h| crate::identity::Style::of(&h.pennant));
        let home = &self.table.home;
        let masthead = self.masthead.get_or_insert_with(|| {
            let look = |i: usize| home.get(i).and_then(|(c, _)| Look::of(c, reduce).ok());
            Masthead::new([look(0), look(1)], reduce).with_flag(flag)
        });
        let tick = if reduce {
            0
        } else {
            (now * MASTHEAD_FRAMES_PER_SECOND) as i64
        };
        let key = tick * 1_000_000 + i64::from(w) * 1000 + i64::from(h);
        if self
            .masthead_picture
            .as_ref()
            .is_none_or(|(k, _)| *k != key)
        {
            let picture = masthead.paint(w, h, now);
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [picture.width() as usize, picture.height() as usize],
                &picture.rgba_bytes(),
            );
            match &mut self.masthead_picture {
                Some((k, t)) => {
                    t.set(image, egui::TextureOptions::NEAREST);
                    *k = key;
                }
                None => {
                    let t = ctx.load_texture("masthead", image, egui::TextureOptions::NEAREST);
                    self.masthead_picture = Some((key, t));
                }
            }
        }
        let (rect, _) = ui.allocate_exact_size(vec2(w as f32, h as f32) * u, Sense::hover());
        let ppp = ctx.pixels_per_point();
        let rect = Rect::from_min_size(
            pos2(
                (rect.min.x * ppp).round() / ppp,
                (rect.min.y * ppp).round() / ppp,
            ),
            rect.size(),
        );
        if let Some((_, picture)) = &self.masthead_picture {
            ui.painter().image(
                picture.id(),
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        // The page's title, lettered on the ribbon.
        let (rx, ry, rw, rh) = Masthead::ribbon(w, h);
        let ribbon = Rect::from_min_size(
            rect.min + vec2(rx as f32, ry as f32) * u,
            vec2(rw as f32, rh as f32) * u,
        );
        let painter = ui.painter();
        let mut size = (ribbon.height() * 0.5).clamp(9.0, 15.0);
        let mut title = painter.layout_job(label_job(BATTLEGROUND, size, ink::COVER));
        while title.size().x > ribbon.width() * 0.86 && size > 8.0 {
            size -= 0.5;
            title = painter.layout_job(label_job(BATTLEGROUND, size, ink::COVER));
        }
        painter.galley(
            ribbon.center() - title.size() / 2.0 + vec2(0.0, u * 0.5),
            title,
            ink::COVER,
        );
    }

    /// The station's ruled right page: where the companions are, and once the train has stopped,
    /// the two places they can be off to and the Handler's own pages.
    pub(super) fn station_right(&mut self, ui: &mut egui::Ui, u: f32, now: f32) {
        let Some(station) = &self.station else {
            return;
        };
        let leaving = station.leaving();
        let stopped = station.stopped(now);
        let since = station.stopped_at();
        let observation = if leaving {
            "Off they go…"
        } else if stopped {
            "All aboard! Where are they off to?"
        } else {
            "Your companions are coming in on the train. Click the picture to hurry it along."
        };
        self.masthead(ui, u, now);
        ui.add_space(u * 2.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(observation).size(13.5).color(ink::MUTED));
        });
        ui.add_space(u * 6.0);
        let width = ui.available_width();
        let h = 96.0;
        let mut chosen = None;
        let mut notes = false;
        let ready = stopped && !leaving;
        for (i, (outing, title, small, to, hint)) in [
            (
                Outing::World,
                "Out in the World",
                "Online",
                Destination::World,
                "Play a friend: open a room and give them its code, or join theirs.",
            ),
            (
                Outing::Home,
                "Close to Home",
                "Offline",
                Destination::Home,
                "Play the visiting team on this computer. Wins fill each place's cup.",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
            if ready {
                let picked = crate::feel::dropping(ui, since, i, u, |ui| {
                    crate::ticket::ticket(
                        ui,
                        rect,
                        Id::new(("destination", i)),
                        title,
                        small,
                        to,
                        u,
                    )
                    .on_hover_text(hint)
                    .clicked()
                });
                if picked {
                    chosen = Some(outing);
                }
            }
            ui.add_space(u * 6.0);
        }
        let mut lessons = false;
        if ready {
            crate::feel::dropping(ui, since, 2, u, |ui| {
                ui.add_space(u * 2.0);
                dashed_rule(ui, u);
                ui.add_space(u * 4.0);
                // A colony that has never played is told where to begin.
                if self.table.record.applied.is_empty() {
                    ui.label(
                        RichText::new("New here? Field Lessons show how a match works.")
                            .size(12.5)
                            .italics()
                            .color(ink::MUTED),
                    );
                    ui.add_space(u * 2.0);
                }
                lessons = chrome::button(
                    ui,
                    "Field Lessons",
                    Kind::Plain,
                    false,
                    true,
                    Some(width),
                    u,
                )
                .on_hover_text(
                    "Short lessons on the Backyard: the beacon, holding and facing, flanks, tricks, team orders, the sprinkler and found things, and who your companions are. Nothing is kept.",
                )
                .clicked();
                ui.add_space(u * 3.0);
                notes = chrome::button(
                    ui,
                    "Your Handler & history",
                    Kind::Plain,
                    false,
                    true,
                    Some(width),
                    u,
                )
                .on_hover_text(
                    "Your name, pennant and trios, matches played, cups, and how Battle looks and sounds",
                )
                .clicked();
            });
        }
        if notes {
            self.open_journal(self.outing, Step::Handler);
        } else if lessons {
            self.open_journal(Outing::Lessons, Step::Lessons);
        } else if let Some(outing) = chosen {
            self.outing = outing;
            if let Some(station) = &mut self.station {
                station.set_off(now);
            }
        }
    }

    /// The left page of getting ready.
    pub(super) fn journal_left(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        let steps = self.steps();
        let number = steps.iter().position(|s| *s == self.step).unwrap_or(0) + 1;
        let line = format!(
            "{} · page {} of {}",
            if self.own_pages() {
                "The Handler's notebook"
            } else {
                self.outing.name()
            },
            number,
            steps.len()
        );
        match self.step {
            Step::Where => {
                let observation = match self.outing {
                    Outing::Home | Outing::Lessons => {
                        "Pick a place for the match. Each has a cup to win against the visiting team, and winning one opens the next."
                    }
                    Outing::World => {
                        "Pick a place for the match. Every place is open for friends. If you join a friend's room instead, you play where they chose."
                    }
                };
                chrome::page_heading(ui, &line, "Where to?", observation);
                ui.add_space(u * 6.0);
                self.arena_picture(ui, ctx, u);
            }
            Step::Lessons => {
                chrome::page_heading(
                    ui,
                    &line,
                    "Field Lessons",
                    "Short situations on the Backyard, under the real rules, that show how a match works. Play them in any order, as often as you like. Nothing is kept.",
                );
                ui.add_space(u * 6.0);
                self.lesson_page(ui, u);
            }
            Step::Who => {
                chrome::page_heading(
                    ui,
                    &line,
                    "Who goes?",
                    "Choose three of your companions. They take the field in the order you choose them.",
                );
                ui.add_space(u * 6.0);
                self.companions(ui, ctx, u);
            }
            Step::Room => {
                chrome::page_heading(
                    ui,
                    &line,
                    "Your friend's room",
                    "Open a room and give your friend its code, or join theirs with the code they give you.",
                );
                ui.add_space(u * 6.0);
                self.room(ui, u);
            }
            Step::Handler => {
                chrome::page_heading(
                    ui,
                    &line,
                    "Your Handler",
                    "Who you are when your companions play: your name, your club and your pennant. Friends see them beside your team.",
                );
                ui.add_space(u * 6.0);
                egui::ScrollArea::vertical()
                    .id_salt("handler-card")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.handler_page(ui, ctx, u));
            }
            Step::Playbooks => {
                chrome::page_heading(
                    ui,
                    &line,
                    "Playbooks",
                    "A plan for a whole match, in up to three stages. Against the visitors, the one a trio plays by fills in its orders each turn, and you change any of them before you lock in.",
                );
                ui.add_space(u * 6.0);
                egui::ScrollArea::vertical()
                    .id_salt("playbooks-left")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.shelf_left(ui, u));
            }
            Step::Season => {
                chrome::page_heading(
                    ui,
                    &line,
                    "The Season",
                    "The Handlers who come to the Battleground, the events you enter against them, and how you stand. An event match counts toward the place's cup like any other.",
                );
                ui.add_space(u * 6.0);
                self.event_board(ui, u);
            }
            Step::History => {
                chrome::page_heading(
                    ui,
                    &line,
                    "History",
                    "Every match your colony has played, newest first, and who went.",
                );
                ui.add_space(u * 6.0);
                self.history(ui, ctx, u);
            }
            Step::Settings => {
                chrome::page_heading(
                    ui,
                    &line,
                    "Settings",
                    "How Battle looks and sounds. Battle keeps these on this computer for next time.",
                );
                ui.add_space(u * 6.0);
                self.settings(ui, u);
            }
        }
    }

    /// The ruled right page of getting ready: what is chosen so far, and the way on.
    pub(super) fn journal_right(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        egui::ScrollArea::vertical()
            .id_salt("journal-right")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                match self.step {
                    Step::Where => self.places(ui, u),
                    Step::Lessons => self.lessons(ui, u),
                    Step::Who => self.lineup(ui, ctx, u),
                    Step::Room => self.shared(ui, ctx, u),
                    Step::Handler => self.standing_and_trios(ui, ctx, u),
                    Step::Playbooks => self.shelf_right(ui, u),
                    Step::Season => self.standing_against_rivals(ui, u),
                    Step::History => self.totals(ui, u),
                    Step::Settings => self.keys_and_about(ui, u),
                }
                // The way back, under the way on.
                if let Some(word) = self.back_word() {
                    ui.add_space(u * 4.0);
                    if chrome::button(ui, &format!("‹ {word}"), Kind::Back, false, true, None, u)
                        .on_hover_text("Escape")
                        .clicked()
                    {
                        self.go_back();
                    }
                }
            });
    }

    /// Where the way back from this page goes, unless an open room holds it here.
    fn back_word(&self) -> Option<&'static str> {
        if self.room_waiting() && !self.own_pages() {
            return None;
        }
        let steps = self.steps();
        let at = steps.iter().position(|s| *s == self.step).unwrap_or(0);
        Some(match at.checked_sub(1).map(|i| steps[i]) {
            None => "The station",
            Some(Step::Where) => "Where to",
            Some(Step::Lessons) => "Lessons",
            Some(Step::Who) => "Who goes",
            Some(Step::Handler) => "Your Handler",
            Some(Step::Playbooks) => "Playbooks",
            Some(Step::Season) => "The Season",
            Some(Step::History) => "History",
            Some(_) => "Back",
        })
    }

    /// Back a page, or from the first to the station: the button under the way on, or Escape.
    pub(super) fn go_back(&mut self) {
        if self.back_word().is_none() {
            return;
        }
        let steps = self.steps();
        let at = steps.iter().position(|s| *s == self.step).unwrap_or(0);
        match at.checked_sub(1) {
            Some(i) => {
                let now = self.clock;
                self.turn_to(steps[i], now);
            }
            None => self.back_to_station(),
        }
    }

    /// The chosen place, as a picture pasted onto the page with a caption under it.
    fn arena_picture(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        let garden = super::garden_for(&mut self.garden, &mut self.view, &self.table.arena);
        let key = format!("preview-{}", self.table.arena.id);
        let fresh = !matches!(&self.preview, Some((k, _)) if *k == key);
        let picture = if fresh { Some(garden.picture()) } else { None };
        let (w, h) = (garden.ground.width() as f32, garden.ground.height() as f32);
        let caption_h = 86.0;
        let area = ui.available_rect_before_wrap();
        let room = Rect::from_min_max(area.min, pos2(area.max.x, area.max.y - caption_h));
        let ppp = ctx.pixels_per_point();
        let pad = u * 6.0;
        let pixels = (((room.width() - pad * 2.0) * ppp) / w)
            .min(((room.height() - pad * 2.0) * ppp) / h)
            .floor()
            .min(4.0);
        let texture = match (picture, &self.preview) {
            (Some(picture), _) => {
                let t = texture(ctx, &key, &picture);
                self.preview = Some((key, t.clone()));
                t
            }
            (None, Some((_, t))) => t.clone(),
            (None, None) => return,
        };
        let size = if pixels >= 1.0 {
            vec2(w, h) * pixels / ppp
        } else {
            let fit = ((room.width() - pad * 2.0) / w).min((room.height() - pad * 2.0) / h);
            vec2(w, h) * fit.max(0.1)
        };
        let min = pos2(room.center().x - size.x / 2.0, room.min.y + pad);
        let min = pos2((min.x * ppp).round() / ppp, (min.y * ppp).round() / ppp);
        let photo = Rect::from_min_size(min, size);
        // A mat of card round it, as a photograph is pasted in.
        chrome::card(
            ui.painter(),
            photo.expand(u * 4.0),
            u,
            ink::CARD,
            ink::LINE.gamma_multiply(0.55),
        );
        ui.painter().image(
            texture.id(),
            photo,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        ui.allocate_rect(
            Rect::from_min_max(area.min, pos2(area.max.x, photo.max.y + u * 8.0)),
            Sense::hover(),
        );
        let arena = self.table.arena.clone();
        let open = self.arena_open(&arena.id);
        let cup = crate::rewards::cup_for(&arena.id);
        ui.vertical_centered(|ui| {
            // The place's name on a garden sign planted under its picture.
            let w = 112;
            let sign = self.painted(ui.ctx(), format!("sign-{w}"), |_| {
                crate::keepsake::signboard(w)
            });
            let rect = super::registry::place(ui, &sign, 1.0, u).0;
            let (bx, by, bw, bh) = crate::keepsake::sign_board(w);
            super::registry::letter(
                ui.painter(),
                Rect::from_min_size(
                    rect.min + vec2(bx as f32, by as f32) * u,
                    vec2(bw as f32, bh as f32) * u,
                ),
                &arena.name,
                Color32::from_rgb(0x3f, 0x2a, 0x18),
                0.6,
            );
            ui.label(
                RichText::new(crate::garden::Dressing::of(&arena).blurb())
                    .size(13.0)
                    .color(ink::PAGE),
            );
            if let Some(cup) = cup {
                let wins = self
                    .table
                    .record
                    .cup_wins(cup.id)
                    .min(crate::rewards::CUP_WINS);
                ui.horizontal(|ui| {
                    // Centred by hand: a row of rosettes and a few words.
                    let words = if open {
                        // The rosettes count the wins.
                        cup.name.to_owned()
                    } else if self.outing == Outing::Home {
                        format!("Win the {} to open it", locked_behind(&arena.id))
                    } else {
                        format!(
                            "Open for friends · the {} opens it for the visitors",
                            locked_behind(&arena.id)
                        )
                    };
                    let galley = ui.painter().layout_job(label_job(&words, 9.0, ink::MUTED));
                    let width = galley.size().x
                        + if open {
                            f32::from(crate::rewards::CUP_WINS)
                                * (crate::keepsake::ROSETTE_W as f32 * u + u)
                                + u * 4.0
                        } else {
                            0.0
                        };
                    ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
                    ui.spacing_mut().item_spacing.x = u;
                    if open {
                        for k in 0..crate::rewards::CUP_WINS {
                            rosette(ui, crate::rewards::cup_index(cup.id), k < wins, u);
                        }
                        ui.add_space(u * 3.0);
                    }
                    ui.label(galley);
                });
            }
        });
    }

    /// Every place to play, on a card with its little picture, and the page's way on.
    fn places(&mut self, ui: &mut egui::Ui, u: f32) {
        chrome::kicker(ui, "Places");
        ui.add_space(u * 2.0);
        let ctx = ui.ctx().clone();
        let waiting = self.room_waiting();
        let home = self.outing == Outing::Home;
        let since = self.turned;
        let mut chosen = None;
        for i in 0..self.arenas.len() {
            let (arena, thumb) = &mut self.arenas[i];
            let picture = thumb
                .get_or_insert_with(|| {
                    let view = View::new(arena.width, arena.height);
                    let garden = Garden::new(arena, &view);
                    texture(
                        &ctx,
                        &format!("arena-{}", arena.id),
                        &shrunk(&garden.picture(), 3),
                    )
                })
                .clone();
            let arena = arena.clone();
            let here = self.table.arena.id == arena.id;
            let unlocked = self.arena_open(&arena.id);
            let choosable = !waiting && (unlocked || !home);
            let cup = crate::rewards::cup_for(&arena.id);
            let wins = cup.map_or(0, |c| {
                self.table
                    .record
                    .cup_wins(c.id)
                    .min(crate::rewards::CUP_WINS)
            });
            let (response, _) = crate::feel::dropping(ui, since, i, u, |ui| {
                card(
                    ui,
                    Id::new(("arena", i)),
                    here,
                    choosable || here,
                    choosable,
                    u,
                    |ui| {
                        ui.horizontal(|ui| {
                            let size = picture.size_vec2();
                            let w = 72.0;
                            let (rect, _) = ui
                                .allocate_exact_size(vec2(w, w * size.y / size.x), Sense::hover());
                            ui.painter().image(
                                picture.id(),
                                rect,
                                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                            ui.vertical(|ui| {
                                ui.label(label_job(
                                    &arena.name,
                                    chrome::LABEL,
                                    if here { ink::FOREST } else { ink::PAGE },
                                ));
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = u;
                                    if unlocked {
                                        let at = cup.map_or(0, |c| crate::rewards::cup_index(c.id));
                                        for k in 0..crate::rewards::CUP_WINS {
                                            rosette(ui, at, k < wins, u);
                                        }
                                    } else if home {
                                        padlock(
                                            ui,
                                            &format!(
                                                "Win the {} to play here",
                                                locked_behind(&arena.id)
                                            ),
                                            u,
                                        );
                                    } else {
                                        ui.label(label_job("Open for friends", 9.0, ink::MUTED));
                                    }
                                });
                            });
                        });
                    },
                )
            });
            let response = if !unlocked && home {
                response.on_hover_text(format!("Win the {} to play here", locked_behind(&arena.id)))
            } else {
                response
            };
            if response.hovered() && choosable && !here {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() && choosable && !here {
                chosen = Some(arena);
            }
            ui.add_space(u * 2.0);
        }
        if let Some(arena) = chosen {
            self.choose_arena(arena);
        }
        if waiting {
            ui.label(
                RichText::new(format!("Your room plays in the {}.", self.table.arena.name))
                    .size(11.5)
                    .italics()
                    .color(ink::MUTED),
            );
        }
        ui.add_space(u * 6.0);
        let width = ui.available_width();
        let now = self.clock;
        if chrome::button(
            ui,
            "Turn the page: who goes",
            Kind::Primary,
            false,
            true,
            Some(width),
            u,
        )
        .clicked()
        {
            self.turn_to(Step::Who, now);
        }
    }

    /// The chosen lesson, on the left page: what it asks, and what it says before it begins.
    fn lesson_page(&mut self, ui: &mut egui::Ui, u: f32) {
        let lesson = self.lesson;
        ui.label(label_job(
            &format!("Lesson {} \u{b7} {}", lesson.number(), lesson.title()),
            14.0,
            ink::FOREST,
        ));
        ui.add_space(u * 2.0);
        ui.label(
            RichText::new(lesson.goal())
                .size(14.0)
                .strong()
                .color(ink::PAGE),
        );
        ui.add_space(u * 3.0);
        egui::ScrollArea::vertical()
            .id_salt("lesson-page")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for note in lesson.notes() {
                    ui.label(RichText::new(*note).size(13.0).color(ink::PAGE));
                    ui.add_space(u * 2.0);
                }
                let yours = if lesson.plain() {
                    "Your three play it doing exactly as told, so the board does what the lesson says. In a real match, and in the last lesson, they are themselves."
                } else {
                    "Your three play this one as they really are."
                };
                ui.label(RichText::new(yours).size(12.0).italics().color(ink::MUTED));
            });
    }

    /// Every lesson, on a card, ticked once learned this sitting; and the page's way on.
    fn lessons(&mut self, ui: &mut egui::Ui, u: f32) {
        chrome::kicker(ui, "Lessons");
        ui.add_space(u * 2.0);
        let since = self.turned;
        let mut chosen = None;
        for (i, lesson) in Lesson::ALL.into_iter().enumerate() {
            let here = self.lesson == lesson;
            let learned = self.learned.contains(&lesson);
            let (response, _) = crate::feel::dropping(ui, since, i, u, |ui| {
                card(ui, Id::new(("lesson", i)), here, true, true, u, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(label_job(&format!("{}", lesson.number()), 13.0, ink::STAMP));
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(label_job(
                                lesson.title(),
                                chrome::LABEL,
                                if here { ink::FOREST } else { ink::PAGE },
                            ));
                            ui.label(
                                RichText::new(if learned { "Learned" } else { lesson.goal() })
                                    .size(11.5)
                                    .color(ink::MUTED),
                            );
                        });
                    });
                })
            });
            if response.hovered() && !here {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() && !here {
                chosen = Some(lesson);
            }
            ui.add_space(u * 2.0);
        }
        if let Some(lesson) = chosen {
            self.lesson = lesson;
        }
        ui.add_space(u * 2.0);
        dashed_rule(ui, u);
        ui.add_space(u * 2.0);
        chrome::kicker(ui, "After the lessons");
        for line in crate::app::AFTER_LESSONS {
            ui.label(RichText::new(line).size(11.5).color(ink::MUTED));
            ui.add_space(u);
        }
        ui.add_space(u * 4.0);
        let width = ui.available_width();
        let now = self.clock;
        if chrome::button(
            ui,
            "Turn the page: who goes",
            Kind::Primary,
            false,
            true,
            Some(width),
            u,
        )
        .clicked()
        {
            self.turn_to(Step::Who, now);
        }
    }

    /// Every companion, on a card: tap three to send them.
    fn companions(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        let count = self.table.home.len();
        let gap = u * 6.0;
        let width = ui.available_width();
        let per_row = ((width + gap) / (250.0 + gap)).floor().max(1.0) as usize;
        let card_w = ((width - gap * (per_row as f32 - 1.0)) / per_row as f32).floor();
        let full = self.table.picked.len() >= TEAM_SIZE;
        let waiting = self.room_waiting();
        let since = self.turned;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for row in (0..count).collect::<Vec<_>>().chunks(per_row) {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for &place in row {
                            let contender = self.table.home[place].0.clone();
                            let texture = self.portrait(ctx, &contender);
                            let chosen = self.table.picked.iter().position(|p| *p == place);
                            let note = match self.table.page(contender.traveler.id.0) {
                                Some(page) => {
                                    page.notes().into_iter().next().unwrap_or_else(|| {
                                        format!(
                                            "Played {}: no habits noted yet",
                                            times(page.matches)
                                        )
                                    })
                                }
                                None => "Not in your notebook yet".to_owned(),
                            };
                            let card = Companion {
                                place,
                                contender: &contender,
                                texture: &texture,
                                chosen,
                                note: &note,
                                spare: (full || waiting) && chosen.is_none(),
                            };
                            let response =
                                crate::feel::dropping(ui, since, place.min(9), u, |ui| {
                                    card.show(ui, card_w, u)
                                });
                            if response.clicked() && !waiting {
                                let was = self.table.picked.contains(&place);
                                self.table.toggle_pick(place);
                                if !was && self.table.picked.contains(&place) {
                                    crate::feel::start_hop(ui.ctx(), hop_id(place));
                                } else if was {
                                    crate::feel::emit(ui.ctx(), Feedback::Unpick);
                                }
                            }
                        }
                    });
                    ui.add_space(gap);
                }
                if let Some(error) = &self.error {
                    ui.colored_label(ink::STAMP, error);
                }
            });
    }

    /// The three chosen, in order, with room for whoever is still to be chosen; and the way on.
    fn lineup(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        chrome::kicker(ui, "Your three");
        ui.add_space(u * 2.0);
        let waiting = self.room_waiting();
        let mut take_out = None;
        for slot in 0..TEAM_SIZE {
            match self.table.picked.get(slot).copied() {
                Some(place) => {
                    let contender = self.table.home[place].0.clone();
                    let texture = self.portrait(ctx, &contender);
                    card(ui, Id::new(("slot", slot)), false, true, false, u, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(label_job(&format!("{}", slot + 1), 13.0, ink::STAMP));
                            portrait_mat(ui, &texture, 44.0, u, true);
                            // The button takes its room at the right first, so the words beside
                            // the portrait wrap rather than run under it.
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if !waiting
                                        && chrome::button(
                                            ui,
                                            "take out",
                                            Kind::Back,
                                            false,
                                            true,
                                            None,
                                            u,
                                        )
                                        .clicked()
                                    {
                                        take_out = Some(place);
                                    }
                                    ui.with_layout(
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.label(
                                                RichText::new(&contender.traveler.name)
                                                    .size(15.0)
                                                    .strong()
                                                    .color(ink::PAGE),
                                            );
                                            ui.label(
                                                RichText::new(&contender.traveler.character.phrase)
                                                    .size(12.0)
                                                    .color(ink::MUTED),
                                            );
                                        },
                                    );
                                },
                            );
                        });
                    });
                }
                None => {
                    let first = slot == self.table.picked.len();
                    let breath = if first {
                        crate::feel::breathe(ui.ctx(), 2.4)
                    } else {
                        0.0
                    };
                    empty_slot(ui, slot, breath, u)
                }
            }
            ui.add_space(u * 3.0);
        }
        if let Some(place) = take_out {
            self.table.toggle_pick(place);
            self.felt.push(Feedback::Unpick);
        }
        if self.table.picked.len() == TEAM_SIZE && !waiting {
            ui.add_space(u * 2.0);
            self.trio_corner(ui, ctx, u);
        }
        ui.add_space(u * 2.0);
        dashed_rule(ui, u);
        ui.add_space(u * 2.0);
        ui.horizontal(|ui| {
            let (playing, back_to) = match self.outing {
                Outing::Lessons => (
                    format!("Lesson {}: {}", self.lesson.number(), self.lesson.title()),
                    Step::Lessons,
                ),
                _ => (
                    format!("Playing in the {}", self.table.arena.name),
                    Step::Where,
                ),
            };
            ui.label(RichText::new(playing).size(12.5).color(ink::MUTED));
            if !waiting && chrome::button(ui, "change", Kind::Quiet, false, true, None, u).clicked()
            {
                let now = self.clock;
                self.turn_to(back_to, now);
            }
        });
        ui.add_space(u * 4.0);
        let width = ui.available_width();
        let short = TEAM_SIZE - self.table.picked.len().min(TEAM_SIZE);
        match self.outing {
            Outing::Home => {
                let open = self.arena_open(&self.table.arena.id);
                let ready = short == 0 && open;
                let fixture = self
                    .table
                    .event_fixture
                    .and_then(|_| self.table.next_event_fixture());
                let label = if !open {
                    format!("Win the {} first", locked_behind(&self.table.arena.id))
                } else if let (true, Some((_, round, rival))) = (ready, fixture) {
                    format!("Play your {round} against {}", rival.name)
                } else if ready {
                    "Play the visiting team".to_owned()
                } else {
                    format!("Choose {short} more")
                };
                if chrome::button(ui, &label, Kind::Primary, false, ready, Some(width), u)
                    .on_hover_text(
                        "Wins against the visitors fill the place's cup, and earn journal entries and keepsakes.",
                    )
                    .clicked()
                {
                    self.begin();
                }
            }
            Outing::Lessons => {
                let label = if short == 0 {
                    "Begin the lesson".to_owned()
                } else {
                    format!("Choose {short} more")
                };
                if chrome::button(ui, &label, Kind::Primary, false, short == 0, Some(width), u)
                    .on_hover_text("A lesson earns nothing and keeps nothing.")
                    .clicked()
                {
                    self.begin_lesson();
                }
            }
            Outing::World => {
                let label = if short == 0 {
                    "Turn the page: your friend's room".to_owned()
                } else {
                    format!("Choose {short} more")
                };
                if chrome::button(ui, &label, Kind::Primary, false, short == 0, Some(width), u)
                    .clicked()
                {
                    let now = self.clock;
                    self.turn_to(Step::Room, now);
                }
            }
        }
    }

    /// The friend's room, before it opens: a name signed, then open one or join one. Once open,
    /// its code, to read out or copy.
    fn room(&mut self, ui: &mut egui::Ui, u: f32) {
        let now = self.clock;
        if let Some(friend) = &self.table.friend {
            if friend.seated {
                ui.label(
                    RichText::new("Your last match is still closing on the relay…")
                        .size(13.0)
                        .color(ink::MUTED),
                );
                return;
            }
            match friend.code.clone() {
                Some(code) if !self.joining => {
                    chrome::kicker(ui, "Give your friend this code");
                    ui.add_space(u * 3.0);
                    ui.horizontal(|ui| {
                        let galley = ui.painter().layout_job(label_job(&code, 34.0, ink::FOREST));
                        let size = galley.size() + vec2(u * 24.0, u * 14.0);
                        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                        code_stub(ui.painter(), rect, u);
                        ui.painter().galley(
                            rect.center() - galley.size() / 2.0,
                            galley,
                            ink::FOREST,
                        );
                        ui.add_space(u * 6.0);
                        let id = Id::new("copied-at");
                        let copied = ui
                            .ctx()
                            .data(|d| d.get_temp::<f32>(id))
                            .is_some_and(|at| now - at < 2.0);
                        let word = if copied { "Copied" } else { "Copy the code" };
                        if chrome::button(ui, word, Kind::Plain, copied, true, None, u).clicked() {
                            ui.ctx().copy_text(code.clone());
                            ui.ctx().data_mut(|d| d.insert_temp(id, now));
                        }
                    });
                    ui.add_space(u * 4.0);
                    let dots = ".".repeat(1 + (now * 2.0) as usize % 3);
                    ui.label(
                        RichText::new(format!(
                            "Waiting for your friend to join{dots} The match begins as soon as they do."
                        ))
                        .size(13.0)
                        .color(ink::MUTED),
                    );
                }
                Some(code) => {
                    ui.label(RichText::new(format!("Joining room {code}…")).size(14.0));
                }
                None if self.joining => {
                    ui.label(
                        RichText::new(format!(
                            "Joining room {}…",
                            formiga_battle_room::code::normalize(&self.join_code)
                                .unwrap_or_else(|| self.join_code.clone())
                        ))
                        .size(14.0),
                    );
                }
                None => {
                    ui.label(
                        RichText::new("Opening the room…")
                            .size(14.0)
                            .color(ink::MUTED),
                    );
                }
            }
            ui.add_space(u * 4.0);
            if chrome::button(ui, "Close the room", Kind::Back, false, true, None, u).clicked() {
                self.table.leave_room();
                self.joining = false;
            }
            if let Some(notice) = self.table.notice.clone() {
                ui.add_space(u * 3.0);
                sticky(ui, &notice, u);
            }
            return;
        }

        // Signing the page: the name the friend sees.
        chrome::kicker(ui, "Your name for your friend");
        ui.add_space(u);
        let field = ui.add(
            egui::TextEdit::singleline(&mut self.handler)
                .font(FontId::proportional(20.0))
                .char_limit(24)
                .frame(egui::Frame::NONE)
                .hint_text("The name your friend knows you by")
                .desired_width(ui.available_width().min(360.0)),
        );
        // A line to sign on.
        let r = field.rect;
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(r.left(), r.bottom() + u), vec2(r.width(), u)),
            0.0,
            if field.has_focus() {
                ink::FOREST
            } else {
                ink::LINE.gamma_multiply(0.5)
            },
        );
        ui.add_space(u * 2.0);
        let shown = self.handler.trim();
        ui.label(
            RichText::new(if shown.is_empty() {
                "Your friend sees this beside your team. Left blank, you are “A Handler”."
                    .to_owned()
            } else {
                format!("Your friend will see “{shown}” beside your team.")
            })
            .size(12.0)
            .color(ink::MUTED),
        );
        ui.add_space(u * 8.0);

        let ready = self.table.picked.len() == TEAM_SIZE && self.understood;
        let typed = formiga_battle_room::code::normalize(&self.join_code);
        let half = ((ui.available_width() - u * 6.0) / 2.0).floor();
        let mut open = false;
        let mut join = false;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = u * 6.0;
            ui.allocate_ui_with_layout(vec2(half, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                card(ui, Id::new("open-room"), false, true, false, u, |ui| {
                    ui.set_min_height(132.0);
                    ui.label(label_job("Open a room", chrome::LABEL + 1.0, ink::FOREST));
                    ui.label(
                        RichText::new(
                            "Battle gives you a six-letter code to pass on. Your friend joins with it, and the match begins in the place you chose.",
                        )
                        .size(12.5),
                    );
                    ui.add_space(u * 3.0);
                    let width = ui.available_width();
                    open = chrome::button(ui, "Open a room", Kind::Primary, false, ready, Some(width), u)
                        .clicked();
                });
            });
            ui.allocate_ui_with_layout(vec2(half, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                card(ui, Id::new("join-room"), false, true, false, u, |ui| {
                    ui.set_min_height(132.0);
                    ui.label(label_job("Join their room", chrome::LABEL + 1.0, ink::FOREST));
                    ui.label(
                        RichText::new("Type the code your friend gives you. You play in the place they chose.")
                            .size(12.5),
                    );
                    ui.add_space(u * 2.0);
                    ui.horizontal(|ui| {
                        let edit = ui.add(
                            egui::TextEdit::singleline(&mut self.join_code)
                                .font(FontId::monospace(20.0))
                                .char_limit(9)
                                .hint_text("CODE")
                                .desired_width(110.0),
                        );
                        if edit.changed() {
                            self.join_code = self.join_code.to_uppercase();
                        }
                        let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        join = (chrome::button(ui, "Join", Kind::Primary, false, ready && typed.is_some(), None, u)
                            .clicked()
                            || enter)
                            && ready
                            && typed.is_some();
                    });
                    let trimmed = self.join_code.trim();
                    if !trimmed.is_empty() && typed.is_none() {
                        ui.label(
                            RichText::new(
                                "A code is six letters and numbers, with no 0, 1, 2, 5 or 8 to mix up.",
                            )
                            .size(11.5)
                            .color(ink::STAMP),
                        );
                    }
                });
            });
        });
        if !ready {
            ui.add_space(u * 3.0);
            let why = if self.table.picked.len() < TEAM_SIZE {
                "Choose three companions first."
            } else {
                "Read what a friend match shares, on the right, and tick that you understand."
            };
            ui.label(RichText::new(why).size(12.5).italics().color(ink::MUTED));
        }
        if open {
            let handler = self.handler.clone();
            self.joining = false;
            self.table.open_room(&handler, None);
        } else if join && let Some(code) = typed {
            let handler = self.handler.clone();
            self.joining = true;
            self.table.open_room(&handler, Some(&code));
        }
        if let Some(notice) = self.table.notice.clone() {
            ui.add_space(u * 3.0);
            sticky(ui, &notice, u);
        }
    }

    /// The ruled page beside the room: what a friend match shares, said in full before anything
    /// is sent, and the team going.
    fn shared(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        chrome::kicker(ui, "What a friend match shares");
        ui.add_space(u * 2.0);
        for line in SHARED {
            ui.label(RichText::new(format!("· {line}")).size(12.0));
            ui.add_space(u);
        }
        ui.add_space(u * 2.0);
        let open = self.table.friend.is_some();
        ui.add_enabled(
            !open,
            egui::Checkbox::new(&mut self.understood, "I understand what is shared"),
        );
        ui.add_space(u * 4.0);
        dashed_rule(ui, u);
        ui.add_space(u * 2.0);
        chrome::kicker(ui, "Going");
        ui.horizontal(|ui| {
            for place in self.table.picked.clone() {
                let contender = self.table.home[place].0.clone();
                let texture = self.portrait(ctx, &contender);
                portrait_mat(ui, &texture, 40.0, u, true)
            }
        });
        ui.label(
            RichText::new(format!(
                "To the {}. Open Circuit points so far: {}. Friend matches earn points, match history and notebook entries.",
                self.table.arena.name, self.table.record.circuit_points
            ))
            .size(11.5)
            .color(ink::MUTED),
        );
    }
}

/// What a companion's hop as it is chosen goes by.
fn hop_id(place: usize) -> Id {
    Id::new(("companion-hop", place))
}

/// "once", "twice", or "n times".
fn times(n: u32) -> String {
    match n {
        1 => "once".to_owned(),
        2 => "twice".to_owned(),
        n => format!("{n} times"),
    }
}

/// A place in the team still to fill: a dashed outline, numbered.
fn empty_slot(ui: &mut egui::Ui, slot: usize, breath: f32, u: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 60.0), Sense::hover());
    let painter = ui.painter();
    // The next place to fill breathes, from the page's plum towards the notebook's green.
    let colour = ink::LINE
        .gamma_multiply(0.35)
        .lerp_to_gamma(ink::FOREST, breath * 0.7);
    let dash = |a: egui::Pos2, b: egui::Pos2| {
        let along = b - a;
        let length = along.length();
        let step = along / length;
        let mut t = 0.0;
        while t < length {
            let end = (t + u * 3.0).min(length);
            painter.line_segment([a + step * t, a + step * end], Stroke::new(u, colour));
            t += u * 5.0;
        }
    };
    let r = rect.shrink(u);
    dash(r.left_top(), r.right_top());
    dash(r.left_bottom(), r.right_bottom());
    dash(r.left_top(), r.left_bottom());
    dash(r.right_top(), r.right_bottom());
    painter.text(
        pos2(r.left() + u * 6.0, r.center().y),
        Align2::LEFT_CENTER,
        format!("{}", slot + 1),
        FontId::monospace(13.0),
        ink::MUTED,
    );
    painter.text(
        pos2(r.left() + u * 16.0, r.center().y),
        Align2::LEFT_CENTER,
        "Choose a companion",
        FontId::proportional(12.5),
        ink::MUTED,
    );
}

/// A ticket stub for a room's code: card with a notch bitten out of each end.
fn code_stub(painter: &egui::Painter, rect: Rect, u: f32) {
    chrome::card(
        painter,
        rect,
        u,
        ink::NOTE,
        Color32::from_rgb(0xb8, 0x9a, 0x52),
    );
    for x in [rect.left(), rect.right()] {
        painter.circle_filled(
            pos2(x, rect.center().y),
            u * 4.0,
            Color32::from_rgb(0xf9, 0xf0, 0xd4),
        );
    }
    let mut y = rect.top() + u * 3.0;
    while y < rect.bottom() - u * 3.0 {
        painter.rect_filled(
            Rect::from_min_size(pos2(rect.left() + u * 9.0, y), vec2(u, u * 2.0)),
            0.0,
            Color32::from_rgb(0xb8, 0x9a, 0x52),
        );
        y += u * 4.0;
    }
}

/// A companion's card while choosing: its portrait, name and temperament, its habits in a word
/// or three and what the notebook says of it, all on the card; chosen, it turns mint and gets
/// its place in the team stamped on its corner. A spare one, while three are chosen, is faded.
struct Companion<'a> {
    place: usize,
    contender: &'a Contender,
    texture: &'a egui::TextureHandle,
    chosen: Option<usize>,
    note: &'a str,
    spare: bool,
}

impl Companion<'_> {
    fn show(&self, ui: &mut egui::Ui, width: f32, u: f32) -> egui::Response {
        let id = Id::new(("companion", self.place));
        let hop = crate::feel::hop(ui.ctx(), hop_id(self.place));
        let (response, _) = ui
            .allocate_ui(vec2(width, 0.0), |ui| {
                card(ui, id, self.chosen.is_some(), !self.spare, true, u, |ui| {
                    ui.set_min_height(96.0);
                    ui.horizontal_top(|ui| {
                        let mark = crate::feel::mark(ui);
                        portrait_mat(ui, self.texture, 64.0, u, true);
                        crate::feel::transform_since(
                            ui,
                            mark,
                            egui::emath::TSTransform::from_translation(vec2(
                                0.0,
                                -(hop * u).round(),
                            )),
                        );
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(&self.contender.traveler.name)
                                    .size(16.0)
                                    .strong()
                                    .color(ink::PAGE),
                            );
                            ui.label(
                                RichText::new(&self.contender.traveler.character.phrase)
                                    .size(12.5)
                                    .color(ink::MUTED),
                            );
                            ui.label(label_job(
                                &self.contender.traveler.character.traits.join(" · "),
                                9.0,
                                ink::MUTED,
                            ));
                            ui.label(
                                RichText::new(self.note)
                                    .size(12.0)
                                    .italics()
                                    .color(ink::FOREST),
                            );
                        });
                    });
                })
            })
            .inner;
        let response = if self.spare {
            response.on_hover_text("Your three are chosen: take one out to send this one instead")
        } else {
            response
        };
        if response.hovered() && !self.spare {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if let Some(n) = self.chosen {
            // Its place in the team, stamped on the corner.
            let rect = response.rect;
            let centre = pos2(rect.max.x - u * 9.0, rect.min.y + u * 9.0);
            let painter = ui.painter();
            // Stamped down as it is chosen: a touch large, then pressed flat.
            let pop = 1.0 + hop / 12.0;
            painter.circle_filled(centre, u * 6.5 * pop, ink::STAMP);
            painter.circle_stroke(centre, u * 5.0 * pop, Stroke::new(u * 0.7, ink::CARD));
            painter.text(
                centre,
                Align2::CENTER_CENTER,
                format!("{}", n + 1),
                FontId::monospace(12.0),
                ink::CARD,
            );
        }
        response
    }
}
