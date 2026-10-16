//! The window. It opens at the halt (`station`), where the train picks up the Handler's companions
//! and the Handler chooses where they are off to: out in the world to a friend, or close to home
//! against the visitors. Then the Handler's notebook (`chrome`) opens, its pages turned one by one
//! as Desktop's journal turns them (`menus`): where to play, who goes and, for a friend, the room.
//! In a match it lies open on the board on its left page and the Handler's notes on its right
//! one; the turn, the beacon and the timer on the leather along the top, and what to do next
//! along the foot. Everything it shows comes from the `Table`.

mod handler;
mod menus;

use crate::chrome::{self, Kind, ink, label_job};
use crate::garden::Garden;
use crate::host::Host;
use crate::iso::View;
use crate::look::Look;
use crate::scene::{self, TEAM};
use crate::table::{Table, Verb};
use eframe::egui::{self, Align2, Color32, FontId, Id, Rect, RichText, Sense, Stroke, pos2, vec2};
use formiga_art::Canvas;
use formiga_battle_contract::Contender;
use formiga_battle_core::recap::describe;
use formiga_battle_core::{Match, Order, Tile};
use formiga_battle_core::{Side, Slot, TEAM_SIZE, Victory};
use std::collections::HashMap;
use std::path::PathBuf;

/// The notebook's parts, in its own pixels: the cover's rim, the strip along the top and the foot,
/// and the fold between the pages.
const RIM: f32 = 9.0;
const STRIP: f32 = 25.0;
const FOOT: f32 = 17.0;
const FOLD: f32 = 3.0;
/// How long a page takes to settle in when it changes, in seconds.
const SETTLE: f32 = 0.22;
/// The side tabs in a match, left to right.
const TABS: [&str; 2] = ["Orders", "Notebook"];

/// What a friend match sends, said in full before the first is opened.
const SHARED: [&str; 5] = [
    "Battle sends these to Formiga's relay at formigaworld.com, and on to your friend's Battle: the name you type here; your three companions' names, looks and temperaments, and how they get on; your orders each turn; a check number for the board after each turn; and which versions of Battle and Formiga Desktop you have, with a fingerprint of your copy of Battle (the same for everyone with that copy), so only matching, unaltered copies play each other.",
    "Nothing else from your colony is sent. There is no account, no email, no device or computer ID.",
    "The relay keeps your room only while it is open. It deletes it when the match ends, or after five minutes with nobody in it, and never keeps a copy. Your seat's key is kept only in scrambled form.",
    "Like any website, formigaworld.com's host may keep its usual short-lived request logs (when a request came, and from what internet address). The relay itself logs nothing about your room.",
    "What the match earns is written to your colony on this computer when Battle closes.",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// At the halt, the train in with everyone aboard.
    Station,
    /// The notebook open on getting ready: where, who, and the friend's room.
    Journal,
    Playing,
}

/// How loud Battle is, for this sitting: Battle keeps no files, so each sitting starts as these
/// are set by default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundSettings {
    /// 0 to 1.
    pub music: f32,
    /// 0 to 1.
    pub effects: f32,
    pub muted: bool,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            music: 0.6,
            effects: 0.8,
            muted: false,
        }
    }
}

/// Where the companions are off to, chosen at the station.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outing {
    /// Out in the world: a friend's Battle, through the relay. Online.
    World,
    /// Close to home: the visiting team, on this computer. Offline.
    Home,
}

impl Outing {
    pub fn name(self) -> &'static str {
        match self {
            Self::World => "Out in the World",
            Self::Home => "Close to Home",
        }
    }

    /// The pages of the notebook this outing turns through.
    pub fn steps(self) -> &'static [Step] {
        match self {
            Self::World => &[Step::Where, Step::Who, Step::Room],
            Self::Home => &[Step::Where, Step::Who],
        }
    }
}

/// A page of getting ready, in the order they are turned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    Where,
    Who,
    Room,
    /// The Handler's own pages, opened from the station rather than on the way anywhere.
    History,
    Settings,
}

impl Step {
    pub fn tab(self) -> &'static str {
        match self {
            Self::Where => "Where",
            Self::Who => "Who",
            Self::Room => "Friend",
            Self::History => "History",
            Self::Settings => "Settings",
        }
    }
}

/// What a review picture shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Review {
    /// The halt, the train coming in or in.
    Station,
    /// Choosing where to play the visitors.
    Where,
    Picking,
    /// A friend's room about to be opened: three chosen, a name signed.
    Friend,
    /// The sample colony's history, and its settings.
    History,
    Settings,
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

/// Where everything is this frame, in points.
struct Layout {
    unit: f32,
    full: Rect,
    foot: Rect,
    left: Rect,
    right: Rect,
    patch: Rect,
    tabs: Vec<Rect>,
    /// Which tab is open.
    open_tab: usize,
}

/// The painted cover, kept until the window changes size or a tab is turned.
#[derive(Default)]
struct Cover {
    spread: Option<chrome::Spread>,
    texture: Option<egui::TextureHandle>,
}

pub struct BattleApp {
    host: Host,
    table: Table,
    stage: Stage,
    view: View,
    board: Option<egui::TextureHandle>,
    garden: Option<Garden>,
    cover: Cover,
    portraits: HashMap<(u64, String), egui::TextureHandle>,
    hover: Option<formiga_battle_core::Tile>,
    error: Option<String>,
    /// The side page shows the Handler's Notebook instead of the orders.
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
    styled: bool,
    /// When the pages last changed (stage or tab), for settling the new one in.
    turned: f32,
    /// Tiles just given an order, and when.
    pokes: Vec<(Tile, f32)>,
    /// When the match was seen to be over, for the stamp on the board.
    finished_at: Option<f32>,
    /// Whether the beacon is held by both at this moment of the turn shown.
    paused: bool,
    /// Where the companions are off to, and which page of getting ready is open.
    outing: Outing,
    step: Step,
    /// A page being turned.
    turn: Option<chrome::PageTurn>,
    /// The halt, once there is a window to size it to; and its picture this frame.
    station: Option<crate::station::Station>,
    station_picture: Option<(i64, egui::TextureHandle)>,
    /// The friend's room was asked for by its code, not opened.
    joining: bool,
    /// What was felt away from the window's context, told with the rest at the next frame.
    felt: Vec<crate::feel::Feedback>,
    /// Sound, for this sitting: how loud the music and the effects are, and whether all of it
    /// is off.
    pub(crate) sound: SoundSettings,
    /// The chosen arena, pictured under the companions while choosing.
    preview: Option<(String, egui::TextureHandle)>,
    /// The board's marks, painted for the key on its page.
    keys: Vec<egui::TextureHandle>,
    /// How large a board pixel is drawn, in points, this frame.
    board_scale: f32,
    /// Every built-in arena, with a little picture of it, for choosing where to play.
    arenas: Vec<(formiga_battle_core::Arena, Option<egui::TextureHandle>)>,
}

impl BattleApp {
    pub fn new(host: Host, table: Table, review: Option<Review>, snap: Option<Snap>) -> Self {
        let view = View::new(table.arena.width, table.arena.height);
        let mut app = Self {
            host,
            table,
            stage: Stage::Station,
            view,
            board: None,
            garden: None,
            cover: Cover::default(),
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
            styled: false,
            turned: -1.0,
            pokes: Vec::new(),
            finished_at: None,
            paused: false,
            outing: Outing::Home,
            step: Step::Where,
            turn: None,
            station: None,
            station_picture: None,
            joining: false,
            felt: Vec::new(),
            sound: SoundSettings::default(),
            preview: None,
            keys: Vec::new(),
            board_scale: 2.0,
            arenas: formiga_battle_core::Arena::built_in()
                .into_iter()
                .map(|a| (a, None))
                .collect(),
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
            Some(Review::Where) => app.open_journal(Outing::Home, Step::Where),
            Some(Review::History) => {
                app.table.record = formiga_battle_contract::sample::record();
                app.open_journal(Outing::Home, Step::History);
            }
            Some(Review::Settings) => app.open_journal(Outing::Home, Step::Settings),
            Some(Review::Picking) => app.open_journal(Outing::Home, Step::Who),
            Some(Review::Friend) => {
                app.open_journal(Outing::World, Step::Room);
                app.table.picked = (0..TEAM_SIZE.min(app.table.home.len())).collect();
                app.handler = "Jake".to_owned();
            }
            Some(Review::Room) => {
                app.open_journal(Outing::World, Step::Room);
                app.table.picked = (0..TEAM_SIZE.min(app.table.home.len())).collect();
                app.handler = "Jake".to_owned();
                app.understood = true;
                app.table.open_room("Jake", None);
            }
            Some(Review::Station) | None => {}
        }
        app
    }

    fn begin(&mut self) {
        match self.table.start(self.clock) {
            Ok(()) => {
                self.stage = Stage::Playing;
                self.error = None;
                self.turned = self.clock;
                self.finished_at = None;
            }
            Err(error) => self.error = Some(format!("{error:#}")),
        }
    }

    /// How far a page has settled in since the pages last changed, eased; at once under reduced
    /// motion, where a page is cut to rather than faded in.
    fn settled(&self, now: f32) -> f32 {
        if self.table.reduce_motion {
            return 1.0;
        }
        let t = ((now - self.turned) / SETTLE).clamp(0.0, 1.0);
        1.0 - (1.0 - t) * (1.0 - t)
    }

    fn portrait(&mut self, ctx: &egui::Context, contender: &Contender) -> egui::TextureHandle {
        let reduce = self.table.reduce_motion;
        let key = (contender.traveler.id.0, contender.traveler.name.clone());
        self.portraits
            .entry(key)
            .or_insert_with(|| {
                let canvas = Look::of(contender, reduce)
                    .map(|mut look| snug(&look.portrait()))
                    .unwrap_or_else(|_| Canvas::new(48, 48));
                texture(
                    ctx,
                    &format!("portrait-{:x}", contender.traveler.id.0),
                    &canvas,
                )
            })
            .clone()
    }

    fn layout(&self, ctx: &egui::Context, full: Rect) -> Layout {
        let unit = chrome::unit(ctx.pixels_per_point());
        let snap = |v: f32| (v / unit).round() * unit;
        let (rim, strip, foot, fold) = (RIM * unit, STRIP * unit, FOOT * unit, FOLD * unit);
        let panel = snap((full.width() * 0.265).clamp(300.0, 400.0));
        let top = full.min.y + strip;
        let bottom = full.max.y - foot;
        let right = Rect::from_min_max(
            pos2(full.max.x - rim - panel, top),
            pos2(full.max.x - rim, bottom),
        );
        let left = Rect::from_min_max(
            pos2(full.min.x + rim, top),
            pos2(right.min.x - fold, bottom),
        );
        let measure = |text: &str, size: f32| {
            ctx.fonts_mut(|f| f.layout_job(label_job(text, size, ink::COVER)))
                .size()
                .x
        };
        let patch = Rect::from_min_size(
            pos2(full.min.x + rim + unit * 4.0, full.min.y + unit * 5.0),
            vec2(
                snap(measure("Formiga Battle", 12.0) + unit * 14.0),
                strip - unit * 9.0,
            ),
        );
        let mut tabs = Vec::new();
        let words = self.tab_words();
        let mut x = right.min.x + unit * 4.0;
        for word in &words {
            let w = snap(measure(word, chrome::LABEL) + unit * 12.0);
            tabs.push(Rect::from_min_size(
                pos2(x, patch.min.y),
                vec2(w, patch.height()),
            ));
            x += w + unit * 3.0;
        }
        let open_tab = match self.stage {
            Stage::Playing => usize::from(self.notebook_open),
            _ => self
                .steps()
                .iter()
                .position(|s| *s == self.step)
                .unwrap_or(0),
        };
        Layout {
            unit,
            full,
            foot: Rect::from_min_max(pos2(full.min.x, bottom), full.max),
            left,
            right,
            patch,
            tabs,
            open_tab,
        }
    }

    /// The tabs stitched to the cover: a match's two pages, or the pages of getting ready,
    /// numbered in the order they are turned.
    fn tab_words(&self) -> Vec<String> {
        match self.stage {
            Stage::Playing => TABS.iter().map(|w| (*w).to_owned()).collect(),
            Stage::Journal => self
                .steps()
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{} {}", i + 1, s.tab()))
                .collect(),
            Stage::Station => Vec::new(),
        }
    }

    /// Paints the cover and pages, if they have changed, and lays them under everything.
    fn cover(&mut self, ui: &mut egui::Ui, layout: &Layout) {
        let u = layout.unit;
        let px = |r: Rect| -> chrome::Area {
            let min = (r.min - layout.full.min) / u;
            let size = r.size() / u;
            (
                min.x.round() as i32,
                min.y.round() as i32,
                size.x.round() as i32,
                size.y.round() as i32,
            )
        };
        let size = layout.full.size() / u;
        let spread = chrome::Spread {
            size: (size.x.ceil() as i32, size.y.ceil() as i32),
            left: px(layout.left),
            right: px(layout.right),
            patch: px(layout.patch),
            tabs: layout
                .tabs
                .iter()
                .enumerate()
                .map(|(i, r)| chrome::Tab {
                    at: px(*r),
                    open: i == layout.open_tab,
                })
                .collect(),
            rules: (24, 14),
        };
        if self.cover.spread.as_ref() != Some(&spread) || self.cover.texture.is_none() {
            let canvas = chrome::paint(&spread);
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [canvas.width() as usize, canvas.height() as usize],
                &canvas.rgba_bytes(),
            );
            match &mut self.cover.texture {
                Some(t) => t.set(image, egui::TextureOptions::NEAREST),
                None => {
                    self.cover.texture = Some(ui.ctx().load_texture(
                        "notebook",
                        image,
                        egui::TextureOptions::NEAREST,
                    ))
                }
            }
            self.cover.spread = Some(spread.clone());
        }
        if let Some(texture) = &self.cover.texture {
            let rect = Rect::from_min_size(
                layout.full.min,
                vec2(spread.size.0 as f32, spread.size.1 as f32) * u,
            );
            ui.painter().image(
                texture.id(),
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        let painter = ui.painter();
        let title = painter.layout_job(label_job("Formiga Battle", 12.0, ink::COVER));
        painter.galley(
            layout.patch.center() - title.size() / 2.0 + vec2(0.0, u * 0.5),
            title,
            ink::COVER,
        );
    }

    fn frame(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if !self.styled {
            chrome::style(&ctx);
            self.styled = true;
        }
        self.clock = ctx.input(|input| input.time) as f32;
        crate::feel::set_still(&ctx, self.table.reduce_motion);
        // Until Battle has sounds to play, whatever is felt goes unheard.
        let mut heard = std::mem::take(&mut self.felt);
        heard.extend(crate::feel::take(&ctx));
        drop(heard);
        let now = self.clock;
        if self.turned < 0.0 {
            self.turned = now;
        }
        if self.table.friend.is_some() && self.table.poll(now) {
            self.stage = Stage::Playing;
            self.notebook_open = false;
            self.turned = now;
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
            if self.table.finished(now) {
                self.finished_at.get_or_insert(now);
            }
        }
        // The garden moves even while nobody does.
        ctx.request_repaint();
        self.keys(&ctx);
        self.pokes.retain(|(_, at)| now - at < 0.6);
        let full = ui.max_rect();
        if self.stage == Stage::Station {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| self.station_page(ui, full, now));
            self.take_snap(&ctx);
            return;
        }
        let layout = self.layout(&ctx, full);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                self.cover(ui, &layout);
                let u = layout.unit;
                let left = layout.left.shrink2(vec2(u * 8.0, u * 6.0));
                let right = layout.right.shrink2(vec2(u * 8.0, u * 6.0));
                // Clear of the margin line.
                let right = Rect::from_min_max(pos2(right.min.x + u * 6.0, right.min.y), right.max);
                let fade = self.settled(now);
                match self.stage {
                    Stage::Journal => {
                        // A page being turned shows itself turning; otherwise it settles in.
                        let fade = if self.turn.is_some() { 1.0 } else { fade };
                        ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
                            ui.multiply_opacity(fade);
                            self.journal_left(ui, &ctx, u);
                        });
                        ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                            ui.multiply_opacity(fade);
                            self.journal_right(ui, &ctx, u);
                        });
                    }
                    Stage::Station => {}
                    Stage::Playing => {
                        self.board_view(ui, &ctx, layout.left.shrink(u * 4.0), now, u);
                        self.board_page(ui, &ctx, layout.left, now, u);
                        ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                            ui.multiply_opacity(fade);
                            self.side_panel(ui, &ctx, now, u);
                        });
                    }
                }
                self.top_bar(ui, &layout, now);
                self.foot_bar(ui, &layout, now);
            });
        if let Some(turn) = self.turn {
            let painter = ctx
                .layer_painter(egui::LayerId::new(
                    egui::Order::Foreground,
                    Id::new("page-turn"),
                ))
                .with_clip_rect(layout.left.union(layout.right));
            if !chrome::turn_leaf(&painter, turn, now, layout.left, layout.right, layout.unit) {
                self.turn = None;
            }
        }
        self.take_snap(&ctx);
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if self.stage != Stage::Playing
            || self.table.shown.is_some()
            || ctx.egui_wants_keyboard_input()
        {
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

    /// The tabs and what is written on the leather along the top: the arena, the turn, how near
    /// each team is to lighting the beacon, and the timer.
    fn top_bar(&mut self, ui: &mut egui::Ui, layout: &Layout, now: f32) {
        let u = layout.unit;
        let words = self.tab_words();
        for (i, rect) in layout.tabs.iter().enumerate() {
            let open = i == layout.open_tab;
            let reachable = match self.stage {
                Stage::Journal => self.reachable(self.steps()[i]).is_ok(),
                _ => true,
            };
            let mut response = ui.interact(
                *rect,
                Id::new(("tab", i)),
                if reachable {
                    Sense::click()
                } else {
                    Sense::hover()
                },
            );
            if response.hovered() && !open && reachable {
                chrome::stepped(ui.painter(), *rect, u, Color32::from_rgb(0x87, 0x60, 0x4a));
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let colour = if open {
                ink::PAGE
            } else if reachable {
                ink::DEBOSS
            } else {
                ink::DEBOSS.gamma_multiply(0.45)
            };
            let galley = ui
                .painter()
                .layout_job(label_job(&words[i], chrome::LABEL, colour));
            ui.painter()
                .galley(rect.center() - galley.size() / 2.0, galley, colour);
            if self.stage == Stage::Journal
                && let Err(why) = self.reachable(self.steps()[i])
            {
                response = response.on_hover_text(why);
            }
            if response.clicked() && !open {
                match self.stage {
                    Stage::Journal => self.turn_to(self.steps()[i], now),
                    _ => {
                        self.notebook_open = i == 1;
                        self.turned = now;
                    }
                }
            }
        }
        let area = Rect::from_min_max(
            pos2(layout.patch.max.x + u * 8.0, layout.patch.min.y),
            pos2(layout.left.max.x - u * 4.0, layout.patch.max.y),
        );
        if self.stage == Stage::Journal {
            self.journal_strip(ui, area, u, now);
            return;
        }
        let paused = self.paused;
        ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = u * 4.0;
                ui.label(label_job(
                    &self.table.arena.name,
                    chrome::LABEL,
                    ink::DEBOSS,
                ));
                let Some(m) = &self.table.m else {
                    return;
                };
                ui.add_space(u * 4.0);
                let turn =
                    m.board.turn + u8::from(self.table.shown.is_none() && m.outcome().is_none());
                ui.label(label_job(
                    &format!("Turn {} of {}", turn.max(1), m.ruleset.turn_cap),
                    chrome::LABEL,
                    ink::COVER,
                ));
                ui.add_space(u * 4.0);
                let shown = self.table.frame_capture(now);
                for side in Side::BOTH {
                    let team = TEAM[side.index()];
                    let name = if side == self.table.you {
                        "You".to_owned()
                    } else {
                        self.table.other_short()
                    };
                    ui.label(label_job(&name, chrome::LABEL, color(team.light)));
                    beacon_meter(
                        ui,
                        shown[side.index()],
                        m.ruleset.capture_ticks,
                        team,
                        paused,
                        u,
                        now,
                    );
                    ui.add_space(u * 2.0);
                }
                if paused && (self.table.reduce_motion || (now * 2.0).sin() > -0.4) {
                    ui.label(label_job("Held by both", chrome::LABEL, ink::COVER));
                }
                if self.table.shown.is_none() && m.outcome().is_none() {
                    ui.add_space(u * 3.0);
                    let left = self.table.time_left(now);
                    let of = f32::from(self.table.ruleset.turn_seconds).max(left);
                    watch(ui, left, of, u);
                    let warn = left <= 10.0;
                    ui.label(label_job(
                        &format!("{}s", left.ceil() as u32),
                        12.0,
                        if warn {
                            Color32::from_rgb(0xff, 0xb0, 0x90)
                        } else {
                            ink::COVER
                        },
                    ));
                    let other = color(TEAM[self.table.you.other().index()].light);
                    let (word, colour) = match &self.table.friend {
                        None => ("Visitors ready".to_owned(), other),
                        Some(f) if f.opponent_away => {
                            (format!("{} is away", self.table.other_short()), ink::DEBOSS)
                        }
                        Some(f) if f.opponent_ready => {
                            (format!("{} is ready", self.table.other_short()), other)
                        }
                        Some(_) => (
                            format!("{} is choosing", self.table.other_short()),
                            ink::DEBOSS,
                        ),
                    };
                    ui.add_space(u * 2.0);
                    ui.label(label_job(&word, chrome::LABEL, colour));
                }
            });
        });
    }

    /// What to do next, along the leather at the foot; and who Battle is open for.
    fn foot_bar(&self, ui: &mut egui::Ui, layout: &Layout, now: f32) {
        let u = layout.unit;
        // Between the pages' edge and the stitching along the foot.
        let area = Rect::from_min_max(
            pos2(
                layout.foot.min.x + RIM * u + u * 4.0,
                layout.foot.min.y + u * 2.0,
            ),
            pos2(
                layout.foot.max.x - RIM * u - u * 4.0,
                layout.foot.max.y - u * 7.0,
            ),
        );
        let line = match self.stage {
            Stage::Station => String::new(),
            Stage::Journal => self.journal_hint(),
            Stage::Playing if self.table.finished(now) => {
                "The match is over. Play again from the notes page.".to_owned()
            }
            Stage::Playing if self.table.shown.is_some() => "The turn plays out…".to_owned(),
            Stage::Playing => match (self.table.selected, self.table.verb) {
                (Some(_), Some(Verb::Move)) => {
                    "Click a tile to walk to. Right-click or Escape cancels.".to_owned()
                }
                (Some(_), Some(Verb::Hold)) => {
                    "Click a tile to hold. Right-click or Escape cancels.".to_owned()
                }
                (Some(_), Some(Verb::Contest)) => {
                    "Click someone of theirs, or the lever.".to_owned()
                }
                _ => {
                    "Choose MOVE, HOLD or CONTEST (M, H, C), then where. Enter locks in.".to_owned()
                }
            },
        };
        let painter = ui.painter();
        painter.text(
            pos2(area.min.x, area.center().y),
            Align2::LEFT_CENTER,
            line,
            FontId::proportional(13.0),
            ink::COVER,
        );
        painter.text(
            pos2(area.max.x, area.center().y),
            Align2::RIGHT_CENTER,
            self.host.describe(),
            FontId::proportional(11.5),
            ink::DEBOSS,
        );
    }

    fn board_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, area: Rect, now: f32, u: f32) {
        let Some(frame) = self.table.frame(now) else {
            return;
        };
        self.paused = frame.paused;
        let mut marks = self.table.marks(self.hover, now);
        marks.pokes = self.pokes.iter().map(|(t, at)| (*t, now - at)).collect();
        let m = self.table.m.as_ref().expect("a match is on");
        let garden = self
            .garden
            .get_or_insert_with(|| Garden::new(&m.arena, &self.view));
        let canvas = scene::paint(
            &m.arena,
            garden,
            &frame,
            &mut self.table.looks,
            &marks,
            m.ruleset.capture_ticks,
            now,
            self.table.reduce_motion,
            &self.view,
        );
        // Scaled by whole screen pixels, as large as the page allows.
        let ppp = ctx.pixels_per_point();
        let (w, h) = (canvas.width() as f32, canvas.height() as f32);
        let pixels = ((area.width() * ppp) / w)
            .min((area.height() * ppp) / h)
            .floor()
            .max(1.0);
        let scale = pixels / ppp;
        let size = vec2(w * scale, h * scale);
        let min = area.center() - size / 2.0;
        let min = pos2((min.x * ppp).round() / ppp, (min.y * ppp).round() / ppp);
        let rect = Rect::from_min_size(min, size);
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
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let to_tile = |pos: egui::Pos2| {
            let local = (pos - rect.min) / scale;
            self.view.tile_at(local.x, local.y)
        };
        self.hover = response.hover_pos().and_then(to_tile);
        if self.hover.is_some() && self.table.shown.is_none() && !marks.choices.is_empty() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if self.table.shown.is_none()
            && response.clicked()
            && let Some(tile) = response.interact_pointer_pos().and_then(to_tile)
            && self.table.click(tile)
        {
            self.pokes.push((tile, now));
        }
        if response.secondary_clicked() {
            self.table.verb = None;
        }
        // A name tag over whoever is pointed at or chosen, saying how they stand.
        let painter = ui.painter().clone();
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
            let at = rect.min + vec2(hx as f32, hy as f32 - 7.0) * scale;
            let state = if f.off_balance {
                Some("off balance")
            } else if f.holding {
                Some("holding")
            } else {
                None
            };
            name_tag(&painter, at, &look.name, state, TEAM[slot.side.index()], u);
        }
        self.board_scale = scale;
        self.result_stamp(&painter, rect, now, u);
    }

    /// What is written on the board's page round it: where this is and what it is for at the
    /// top, the turn's slip when one plays out, and a key to the board's marks in the corner.
    fn board_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, page: Rect, now: f32, u: f32) {
        let painter = ui.painter().clone();
        let top = page.min + vec2(u * 8.0, u * 6.0);
        let name = painter.layout_job(label_job(
            &format!("The {}", self.table.arena.name),
            chrome::LABEL,
            ink::FOREST,
        ));
        let line = name.size().y;
        painter.galley(top, name, ink::FOREST);
        painter.text(
            top + vec2(0.0, line + u * 2.0),
            Align2::LEFT_TOP,
            "Keep someone steady in the yard to light the beacon.",
            FontId::proportional(12.0),
            ink::MUTED,
        );
        self.turn_banner(&painter, page, now, u);
        if self.keys.is_empty() {
            self.keys = scene::Key::ALL
                .iter()
                .enumerate()
                .map(|(i, key)| texture(ctx, &format!("key-{i}"), &key.picture()))
                .collect();
        }
        // The key, in the page's upper right corner, where the board leaves the paper bare.
        let ppp = ctx.pixels_per_point();
        let px = ppp.round().max(1.0) / ppp;
        let icon = vec2(34.0, 24.0) * px;
        let words: Vec<_> = scene::Key::ALL
            .iter()
            .map(|k| {
                painter.layout_no_wrap(k.words().to_owned(), FontId::proportional(11.5), ink::PAGE)
            })
            .collect();
        let widest = words.iter().map(|g| g.size().x).fold(0.0, f32::max);
        let row = icon.y + u;
        let size = vec2(
            icon.x + widest + u * 14.0,
            u * 15.0 + row * words.len() as f32,
        );
        let corner = pos2(page.max.x - u * 8.0 - size.x, page.min.y + u * 6.0);
        let rect = Rect::from_min_size(
            pos2((corner.x / u).round() * u, (corner.y / u).round() * u),
            size,
        );
        chrome::card(&painter, rect, u, ink::CARD, ink::LINE.gamma_multiply(0.45));
        let title = painter.layout_job(label_job("Reading the board", 9.5, ink::FOREST));
        painter.galley(rect.min + vec2(u * 5.0, u * 4.0), title, ink::FOREST);
        for (i, galley) in words.into_iter().enumerate() {
            let y = rect.min.y + u * 12.0 + i as f32 * row;
            let at = Rect::from_min_size(pos2(rect.min.x + u * 4.0, y), icon);
            painter.image(
                self.keys[i].id(),
                at,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            let h = galley.size().y;
            painter.galley(
                pos2(at.max.x + u * 4.0, at.center().y - h / 2.0),
                galley,
                ink::PAGE,
            );
        }
    }

    /// While a turn's orders are revealed, a slip drops onto the top of the board naming it.
    fn turn_banner(&self, painter: &egui::Painter, board: Rect, now: f32, u: f32) {
        let (Some(shown), Some(m)) = (&self.table.shown, &self.table.m) else {
            return;
        };
        let age = now - shown.started;
        let life = crate::playback::REVEAL_SECS + 0.5;
        if age > life || self.table.finished(now) {
            return;
        }
        let (drop, alpha) = if self.table.reduce_motion {
            (0.0, 1.0)
        } else {
            let t = (age / 0.25).min(1.0);
            let ease = 1.0 - (1.0 - t).powi(3);
            let out = ((life - age) / 0.25).clamp(0.0, 1.0);
            ((1.0 - ease) * -14.0 * u, out)
        };
        let galley = painter.layout_job(label_job(
            &format!("Turn {} plays out", m.board.turn),
            13.0,
            ink::FOREST,
        ));
        let size = galley.size() + vec2(u * 16.0, u * 9.0);
        let rect = Rect::from_center_size(
            pos2(
                board.center().x,
                board.min.y + u * 8.0 + size.y / 2.0 + drop,
            ),
            size,
        );
        let rect = Rect::from_min_size(
            pos2((rect.min.x / u).round() * u, (rect.min.y / u).round() * u),
            rect.size(),
        );
        let mut painter = painter.clone();
        painter.set_opacity(alpha);
        chrome::card(&painter, rect, u, ink::CARD, ink::LINE);
        // Stitched at each end, like the cover's tabs.
        for x in [rect.min.x + u * 3.0, rect.max.x - u * 4.0] {
            painter.rect_filled(
                Rect::from_min_size(pos2(x, rect.center().y - u * 0.5), vec2(u, u)),
                0.0,
                Color32::from_rgb(0x8a, 0x71, 0x48),
            );
        }
        painter.galley(rect.center() - galley.size() / 2.0, galley, ink::FOREST);
    }

    /// When the match is over, a rubber stamp comes down on the board saying how it ended.
    fn result_stamp(&self, painter: &egui::Painter, board: Rect, now: f32, u: f32) {
        let (Some(at), Some(outcome)) = (self.finished_at, self.table.outcome()) else {
            return;
        };
        let you = self.table.you;
        let word = match (outcome.winner, outcome.victory) {
            (Some(s), Some(Victory::Capture)) if s == you => "Beacon lit!",
            (Some(s), _) if s == you => "You win!",
            (Some(_), _) => "Well played",
            (None, _) => "A draw",
        };
        let t = if self.table.reduce_motion {
            1.0
        } else {
            ((now - at) / 0.32).clamp(0.0, 1.0)
        };
        let ease = 1.0 - (1.0 - t).powi(3);
        let grow = 1.0 + (1.0 - ease) * 0.8;
        let galley = painter.layout_job(label_job(word, 26.0 * grow, ink::STAMP));
        let centre = pos2(board.center().x, board.min.y + board.height() * 0.2);
        let angle: f32 = -0.09;
        let half = (galley.size() + vec2(u * 14.0, u * 10.0) * grow) / 2.0;
        let (s, c) = angle.sin_cos();
        let rotate = |v: egui::Vec2| centre + vec2(v.x * c - v.y * s, v.x * s + v.y * c);
        let mut painter = painter.clone();
        painter.set_opacity(ease);
        for (inset, width) in [(0.0, 2.0 * u), (3.0 * u, u)] {
            let h = half - vec2(inset, inset);
            let corners = [
                vec2(-h.x, -h.y),
                vec2(h.x, -h.y),
                vec2(h.x, h.y),
                vec2(-h.x, h.y),
            ]
            .map(rotate)
            .to_vec();
            painter.add(egui::Shape::closed_line(
                corners,
                Stroke::new(width, ink::STAMP),
            ));
        }
        let text_at = rotate(-galley.size() / 2.0);
        painter.add(egui::epaint::TextShape::new(text_at, galley, ink::STAMP).with_angle(angle));
    }

    fn side_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, now: f32, u: f32) {
        if self.notebook_open {
            chrome::kicker(ui, "What you have seen them do");
            ui.add_space(u * 2.0);
            self.notebook(ui, ctx, u);
            return;
        }
        if self.table.finished(now) {
            self.result(ui, u);
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
        chrome::kicker(
            ui,
            if giving {
                "Your orders"
            } else {
                "The turn plays out"
            },
        );
        ui.add_space(u * 2.0);
        let team = TEAM[you.index()];
        for i in 0..TEAM_SIZE as u8 {
            let contender = self.table.board_teams[you.index()][i as usize].clone();
            let texture = self.portrait(ctx, &contender);
            let chosen = giving && self.table.selected == Some(i);
            let order = self.table.orders[i as usize];
            let order_text = match (standing[i as usize], order) {
                (false, _) => "Sitting out, dizzy, for the rest of the match".to_owned(),
                (true, Some(order)) => speak(self.table.m.as_ref().expect("on"), &order),
                (true, None) => match self.table.fallback(i) {
                    Some(f) => format!(
                        "No order yet. Left to itself: {}",
                        speak(self.table.m.as_ref().expect("on"), &f)
                    ),
                    None => "No order yet".to_owned(),
                },
            };
            let has_order = order.is_some();
            let up = standing[i as usize];
            let verb = self.table.verb;
            let (response, (picked, cleared)) =
                card(ui, Id::new(("order-card", i)), chosen, up, true, u, |ui| {
                    ui.horizontal_top(|ui| {
                        portrait_mat(ui, &texture, 52.0, u, up);
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(label_job(
                                    &format!("{}", i + 1),
                                    chrome::LABEL,
                                    ink::MUTED,
                                ));
                                ui.label(
                                    RichText::new(&contender.traveler.name)
                                        .size(15.0)
                                        .strong()
                                        .color(ink::PAGE),
                                );
                                pips(ui, composure[i as usize], team, u);
                            });
                            let colour = if has_order { ink::PAGE } else { ink::MUTED };
                            ui.label(RichText::new(&order_text).size(12.5).color(colour));
                            let mut picked = None;
                            let mut cleared = false;
                            if giving && up {
                                ui.add_space(u);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = u * 2.0;
                                    for v in Verb::ALL {
                                        let on = chosen && verb == Some(v);
                                        if chrome::button(
                                            ui,
                                            v.word(),
                                            Kind::Plain,
                                            on,
                                            true,
                                            None,
                                            u,
                                        )
                                        .on_hover_text(v.hint())
                                        .clicked()
                                        {
                                            picked = Some(v);
                                        }
                                    }
                                    if has_order
                                        && chrome::button(
                                            ui,
                                            "clear",
                                            Kind::Quiet,
                                            false,
                                            true,
                                            None,
                                            u,
                                        )
                                        .on_hover_text("Clear this order")
                                        .clicked()
                                    {
                                        cleared = true;
                                    }
                                });
                            }
                            (picked, cleared)
                        })
                        .inner
                    })
                    .inner
                });
            if let Some(v) = picked {
                self.table.select(i);
                self.table.choose_verb(v);
            } else if cleared {
                self.table.clear_order(i);
            } else if giving && response.clicked() {
                self.table.select(i);
            }
            ui.add_space(u * 3.0);
        }
        if giving {
            ui.add_space(u * 2.0);
            let width = ui.available_width();
            if chrome::button(
                ui,
                "Lock in orders",
                Kind::Primary,
                false,
                true,
                Some(width),
                u,
            )
            .on_hover_text("Enter")
            .clicked()
            {
                self.table.lock(now);
            }
            ui.label(
                RichText::new(
                    "Anyone left without an order chooses for itself when the timer runs out.",
                )
                .size(11.5)
                .italics()
                .color(ink::MUTED),
            );
        }
        if sent && self.table.shown.is_none() {
            ui.add_space(u * 3.0);
            sticky(
                ui,
                &format!("Orders sent. Waiting for {}…", self.table.other_short()),
                u,
            );
        }
        if let Some(notice) = self.table.notice.clone() {
            ui.add_space(u * 3.0);
            sticky(ui, &notice, u);
        }
        ui.add_space(u * 4.0);
        dashed_rule(ui, u);
        self.recap(ui, u);
    }

    fn recap(&self, ui: &mut egui::Ui, u: f32) {
        chrome::kicker(ui, "What happened");
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if self.table.log.is_empty() {
                    ui.label(
                        RichText::new("Nothing yet: the first turn is yours.")
                            .size(12.5)
                            .italics()
                            .color(ink::MUTED),
                    );
                }
                for (turn, lines) in self.table.log.iter().rev().take(3).rev() {
                    ui.add_space(u);
                    ui.label(label_job(
                        &format!("Turn {turn}"),
                        chrome::LABEL,
                        ink::MUTED,
                    ));
                    for line in lines {
                        ui.label(RichText::new(format!("· {line}")).size(12.5));
                    }
                }
            });
    }

    fn result(&mut self, ui: &mut egui::Ui, u: f32) {
        let you = self.table.you;
        let outcome = self.table.outcome().expect("finished");
        let tally = self.table.m.as_ref().map_or(Tally::Beacon, tally);
        let ahead_you = match tally {
            Tally::Beacon => "Time ran out with your team ahead on the beacon.",
            Tally::Composure => {
                "Time ran out level on the beacon, with more composure left across your team."
            }
            Tally::Standing => {
                "Time ran out level on the beacon and on composure, with more of your team standing."
            }
        };
        let ahead_them = match tally {
            Tally::Beacon => "Time ran out with them ahead on the beacon.",
            Tally::Composure => {
                "Time ran out level on the beacon, with more composure left across their team."
            }
            Tally::Standing => {
                "Time ran out level on the beacon and on composure, with more of them standing."
            }
        };
        let (title, why) = match (outcome.winner, outcome.victory) {
            (Some(side), Some(Victory::Capture)) if side == you => (
                "You lit the beacon!",
                "Your team held the yard long enough.",
            ),
            (Some(side), Some(Victory::Knockout)) if side == you => {
                ("You win!", "The visitors are all sitting out, dizzy.")
            }
            (Some(side), _) if side == you => ("You win on the tally", ahead_you),
            (Some(_), Some(Victory::Capture)) => (
                "The visitors lit the beacon",
                "They held the yard long enough.",
            ),
            (Some(_), Some(Victory::Knockout)) => (
                "The visitors win",
                "Your team are all sitting out, dizzy, and none the worse for it.",
            ),
            (Some(_), _) => ("The visitors win on the tally", ahead_them),
            (None, _) => ("A draw", "Nobody could be split."),
        };
        chrome::kicker(ui, "The match is over");
        ui.label(RichText::new(title).size(22.0).color(ink::FOREST).strong());
        ui.label(RichText::new(why).size(13.0));
        if let Some(reason) = &self.table.ended_because {
            ui.label(RichText::new(reason).size(13.0).color(ink::MUTED));
        }
        ui.label(
            RichText::new(format!("After {} turns", outcome.turn))
                .size(12.5)
                .color(ink::MUTED),
        );
        ui.add_space(u * 5.0);
        let width = ui.available_width();
        if chrome::button(ui, "Play again", Kind::Primary, false, true, Some(width), u).clicked() {
            self.table.seed = self
                .table
                .seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.table.shown = None;
            self.portraits.clear();
            self.finished_at = None;
            self.notebook_open = false;
            let outing = self.outing;
            self.open_journal(outing, Step::Who);
            // Back through the notebook to choosing who goes.
            self.leaf(false);
        }
        if chrome::button(ui, "Back to the station", Kind::Back, false, true, None, u)
            .on_hover_text("Choose again where your companions are off to")
            .clicked()
        {
            self.table.shown = None;
            self.portraits.clear();
            self.finished_at = None;
            self.notebook_open = false;
            self.back_to_station();
        }
        if !self.table.earned.is_empty() {
            ui.add_space(u * 5.0);
            chrome::kicker(ui, "Earned");
            for line in &self.table.earned {
                ui.label(RichText::new(format!("· {line}")).size(12.5));
            }
            let kept = if self.table.session.is_some() {
                "Kept in your colony when Battle closes."
            } else {
                "A rehearsal: nothing is kept."
            };
            ui.label(RichText::new(kept).size(11.5).italics().color(ink::MUTED));
        }
        ui.add_space(u * 4.0);
        dashed_rule(ui, u);
        self.recap(ui, u);
    }

    /// The Handler's Notebook for the three on the board: what each has been seen to do.
    fn notebook(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for i in 0..TEAM_SIZE {
                    let contender = self.table.board_teams[self.table.you.index()][i].clone();
                    let texture = self.portrait(ctx, &contender);
                    let page = self.table.page(contender.traveler.id.0);
                    card(
                        ui,
                        Id::new(("notebook-card", i)),
                        false,
                        true,
                        false,
                        u,
                        |ui| {
                            ui.horizontal_top(|ui| {
                                portrait_mat(ui, &texture, 44.0, u, true);
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(&contender.traveler.name)
                                            .size(15.0)
                                            .strong()
                                            .color(ink::PAGE),
                                    );
                                    let Some(page) = page else {
                                        ui.label(
                                            RichText::new(
                                                "Nothing noted yet: play a match together.",
                                            )
                                            .size(12.5)
                                            .color(ink::MUTED),
                                        );
                                        return;
                                    };
                                    let played = if page.matches == 1 {
                                        "Played once".to_owned()
                                    } else {
                                        format!("Played {} times, won {}", page.matches, page.wins)
                                    };
                                    ui.label(label_job(&played, chrome::LABEL, ink::MUTED));
                                    let notes = page.notes();
                                    if notes.is_empty() {
                                        ui.label(
                                            RichText::new("No habits noted yet.")
                                                .size(12.5)
                                                .italics()
                                                .color(ink::MUTED),
                                        );
                                    }
                                    for note in notes.iter().take(4) {
                                        ui.label(RichText::new(format!("· {note}")).size(12.5));
                                    }
                                    for moment in page.moments.iter().rev().take(2) {
                                        ui.label(
                                            RichText::new(moment.as_str())
                                                .size(12.0)
                                                .italics()
                                                .color(ink::MUTED),
                                        );
                                    }
                                });
                            });
                        },
                    );
                    ui.add_space(u * 3.0);
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
        let c = chrome::COVER;
        [
            f32::from(c.r) / 255.0,
            f32::from(c.g) / 255.0,
            f32::from(c.b) / 255.0,
            1.0,
        ]
    }
}

/// The cup before an arena's own, which opens it.
fn locked_behind(arena: &str) -> &'static str {
    let cups = &crate::rewards::CUPS;
    cups.iter()
        .position(|c| c.arena == arena)
        .and_then(|i| i.checked_sub(1))
        .map_or(cups[0].name, |i| cups[i].name)
}

/// Which step of the turn cap's tally split the teams: the beacon first, then the composure left
/// across each team, then how many are still standing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tally {
    Beacon,
    Composure,
    Standing,
}

fn tally(m: &Match) -> Tally {
    let sum = |side: Side, f: &dyn Fn(Slot) -> u32| -> u32 {
        (0..TEAM_SIZE as u8).map(|i| f(Slot::new(side, i))).sum()
    };
    let [left, right] = m.board.capture;
    if left != right {
        return Tally::Beacon;
    }
    let composure = |side| sum(side, &|s| u32::from(m.fighter(s).composure));
    if composure(Side::BOTH[0]) != composure(Side::BOTH[1]) {
        return Tally::Composure;
    }
    Tally::Standing
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
            crate::garden::Dressing::of(arena).ground()
        }
    };
    match order {
        Order::Move { to } => format!("MOVE {}", place(*to).0),
        Order::Hold { at } => format!("HOLD {}", place(*at).1),
        Order::Contest { .. } => describe(m, order),
    }
}

/// A card on the page round whatever `add` writes on it, painted under the writing: card
/// coloured on a hard shadow with a stepped edge, mint when chosen, faded for someone sitting
/// out. A `lively` card can be clicked, and lifts a unit when pointed at.
fn card<R>(
    ui: &mut egui::Ui,
    id: Id,
    chosen: bool,
    up: bool,
    lively: bool,
    u: f32,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> (egui::Response, R) {
    let mark = crate::feel::mark(ui);
    let under = ui.painter().add(egui::Shape::Noop);
    let inner = egui::Frame::new()
        .inner_margin(egui::Margin::same((u * 5.0).round() as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !up {
                ui.multiply_opacity(0.55);
            }
            add(ui)
        });
    let rect = inner.response.rect;
    let sense = if lively {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(rect, id, sense);
    let pointed = lively && up && response.hovered();
    let press = if lively && up {
        crate::feel::press(ui.ctx(), id, &response, crate::feel::Feel::Select)
    } else {
        crate::feel::Press::REST
    };
    let lift = press.hover;
    let fill = if chosen {
        ink::MINT
    } else if pointed {
        Color32::from_rgb(255, 252, 240)
    } else {
        ink::CARD
    };
    let edge = if chosen {
        ink::FOREST
    } else {
        ink::LINE.gamma_multiply(0.55)
    };
    let face = rect.translate(vec2(0.0, -(lift * u).round()));
    let shadow = rect.translate(vec2(u, u));
    let mut shapes = vec![egui::Shape::rect_filled(shadow.shrink(u), 0.0, ink::SHADOW)];
    shapes.push(egui::Shape::rect_filled(face.shrink(u), 0.0, fill));
    let (l, r, t, b) = (face.left(), face.right(), face.top(), face.bottom());
    for strip in [
        Rect::from_min_max(pos2(l + u, t), pos2(r - u, t + u)),
        Rect::from_min_max(pos2(l + u, b - u), pos2(r - u, b)),
        Rect::from_min_max(pos2(l, t + u), pos2(l + u, b - u)),
        Rect::from_min_max(pos2(r - u, t + u), pos2(r, b - u)),
    ] {
        shapes.push(egui::Shape::rect_filled(strip, 0.0, edge));
    }
    ui.painter().set(under, egui::Shape::Vec(shapes));
    crate::feel::transform_since(ui, mark, press.transform(rect, u));
    (response, inner.inner)
}

/// A portrait on a little mat of paper, so it sits on the card like a pasted-in photo.
fn portrait_mat(ui: &mut egui::Ui, texture: &egui::TextureHandle, size: f32, u: f32, up: bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect.shrink(u), 0.0, Color32::from_rgb(0xf3, 0xe9, 0xcc));
    chrome::stepped(painter, rect, u, ink::LINE.gamma_multiply(0.3));
    let tint = if up {
        Color32::WHITE
    } else {
        Color32::from_gray(170)
    };
    painter.image(
        texture.id(),
        rect.shrink(u * 3.0),
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        tint,
    );
}

/// What Battle has to say right now, on a sticky note.
fn sticky(ui: &mut egui::Ui, text: &str, u: f32) {
    let width = ui.available_width();
    let galley = ui.painter().layout(
        text.to_owned(),
        FontId::proportional(12.5),
        ink::PAGE,
        width - u * 12.0,
    );
    let size = vec2(width, galley.size().y + u * 10.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    chrome::card(
        ui.painter(),
        rect,
        u,
        ink::NOTE,
        Color32::from_rgb(0xb8, 0x9a, 0x52),
    );
    // A strip of tape across its top.
    let tape = Rect::from_center_size(
        pos2(rect.center().x, rect.min.y + u),
        vec2(u * 18.0, u * 4.0),
    );
    ui.painter().rect_filled(
        tape,
        0.0,
        Color32::from_rgba_unmultiplied(255, 255, 255, 110),
    );
    ui.painter()
        .galley(rect.min + vec2(u * 6.0, u * 5.0), galley, ink::PAGE);
}

/// A dashed rule across the page, three on and two off, as Hill rules its cards.
fn dashed_rule(ui: &mut egui::Ui, u: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), u * 3.0), Sense::hover());
    let mut x = rect.min.x;
    while x < rect.max.x {
        let w = (u * 3.0).min(rect.max.x - x);
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(x, rect.center().y), vec2(w, u)),
            0.0,
            Color32::from_rgb(0xd8, 0xc8, 0xa0),
        );
        x += u * 5.0;
    }
}

/// A rosette for a cup win: brass when won, an empty ring when still to come.
fn rosette(ui: &mut egui::Ui, won: bool, u: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(u * 10.0, u * 10.0), Sense::hover());
    let c = rect.center();
    let painter = ui.painter();
    if won {
        for k in 0..8 {
            let a = k as f32 / 8.0 * std::f32::consts::TAU;
            painter.circle_filled(c + vec2(a.cos(), a.sin()) * u * 3.0, u * 1.8, ink::BRASS);
        }
        painter.circle_filled(c, u * 2.6, ink::BRASS_LIGHT);
        painter.circle_stroke(c, u * 2.6, Stroke::new(u * 0.6, ink::BRASS_EDGE));
    } else {
        painter.circle_stroke(
            c,
            u * 3.6,
            Stroke::new(u * 0.8, ink::LINE.gamma_multiply(0.35)),
        );
    }
}

/// Composure as three pips: filled in the team's colour, empty as rings.
fn pips(ui: &mut egui::Ui, composure: u8, team: formiga_art::paint::Ramp, u: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(u * 22.0, u * 8.0), Sense::hover());
    for k in 0..3u8 {
        let centre = pos2(
            rect.min.x + u * 4.0 + f32::from(k) * u * 7.0,
            rect.center().y,
        );
        if k < composure {
            ui.painter()
                .circle_filled(centre, u * 2.8, color(team.base));
            ui.painter()
                .circle_stroke(centre, u * 2.8, Stroke::new(u * 0.7, color(team.edge)));
            ui.painter()
                .circle_filled(centre - vec2(u, u), u * 0.8, color(team.shine));
        } else {
            ui.painter()
                .circle_stroke(centre, u * 2.6, Stroke::new(u * 0.7, ink::MUTED));
        }
    }
}

/// A team's progress toward lighting the beacon: a row of little lamps on the leather, lit one by
/// one; while both teams hold the yard, the row is crossed with a hatch and gains nothing.
fn beacon_meter(
    ui: &mut egui::Ui,
    progress: u8,
    of: u8,
    team: formiga_art::paint::Ramp,
    paused: bool,
    u: f32,
    now: f32,
) {
    let lamp = vec2(u * 5.0, u * 8.0);
    let gap = u * 1.5;
    let count = of.max(1);
    let width = f32::from(count) * (lamp.x + gap) - gap + u * 4.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, lamp.y + u * 4.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect.shrink(u), 0.0, Color32::from_rgb(0x2f, 0x1c, 0x12));
    chrome::stepped(painter, rect, u, Color32::from_rgb(0x87, 0x60, 0x4a));
    for k in 0..count {
        let at = pos2(
            rect.min.x + u * 2.0 + f32::from(k) * (lamp.x + gap),
            rect.min.y + u * 2.0,
        );
        let r = Rect::from_min_size(at, lamp);
        if k < progress {
            painter.rect_filled(r, 0.0, color(team.base));
            painter.rect_filled(
                Rect::from_min_size(r.min, vec2(r.width(), u)),
                0.0,
                color(team.shine),
            );
            // The newest lamp glows a little brighter, on and off.
            if k + 1 == progress && !paused && (now * 3.0).sin() > 0.0 {
                painter.rect_filled(r.shrink(u), 0.0, color(team.light));
            }
        } else {
            painter.rect_filled(r, 0.0, Color32::from_rgb(0x4a, 0x2e, 0x1e));
        }
    }
    if paused {
        let clip = painter.clone().with_clip_rect(rect.shrink(u));
        let mut x = rect.min.x - rect.height();
        while x < rect.max.x {
            clip.line_segment(
                [pos2(x, rect.max.y), pos2(x + rect.height(), rect.min.y)],
                Stroke::new(u, Color32::from_rgba_unmultiplied(243, 230, 196, 110)),
            );
            x += u * 4.0;
        }
    }
    response.on_hover_text(if paused {
        format!("Beacon {progress} of {of}: held by both teams, so nobody gains")
    } else {
        format!("Beacon {progress} of {of}")
    });
}

/// The turn's timer, as a little brass watch whose face empties as the time runs down.
fn watch(ui: &mut egui::Ui, left: f32, of: f32, u: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(u * 13.0, u * 13.0), Sense::hover());
    let c = rect.center() + vec2(0.0, u * 0.5);
    let r = u * 5.5;
    let painter = ui.painter();
    painter.circle_filled(c + vec2(u, u), r, ink::SHADOW);
    painter.rect_filled(
        Rect::from_center_size(c - vec2(0.0, r + u * 0.5), vec2(u * 2.0, u * 1.5)),
        0.0,
        ink::BRASS_LIGHT,
    );
    painter.circle_filled(c, r, ink::BRASS);
    painter.circle_stroke(c, r, Stroke::new(u * 0.7, ink::BRASS_EDGE));
    painter.circle_filled(c, r - u * 1.5, ink::CARD);
    let share = (left / of.max(1.0)).clamp(0.0, 1.0);
    let fill = if left <= 10.0 {
        ink::STAMP
    } else {
        ink::FOREST
    };
    if share > 0.0 {
        let steps = 24;
        let mut points = vec![c];
        for k in 0..=steps {
            let a = -std::f32::consts::FRAC_PI_2
                + share * std::f32::consts::TAU * k as f32 / steps as f32;
            points.push(c + vec2(a.cos(), a.sin()) * (r - u * 2.0));
        }
        // A pie is not convex past half way, so it is drawn as a fan of slices.
        for pair in points[1..].windows(2) {
            painter.add(egui::Shape::convex_polygon(
                vec![c, pair[0], pair[1]],
                fill.gamma_multiply(0.85),
                Stroke::NONE,
            ));
        }
    }
    painter.circle_filled(c, u * 0.8, ink::LINE);
}

/// A name tag over someone on the board: Home's hover tag, card on a hard shadow with a stepped
/// edge, with a word for how they stand under the name.
fn name_tag(
    painter: &egui::Painter,
    at: egui::Pos2,
    name: &str,
    state: Option<&str>,
    team: formiga_art::paint::Ramp,
    u: f32,
) {
    let galley = painter.layout_no_wrap(name.to_owned(), FontId::proportional(13.0), ink::PAGE);
    let state = state.map(|s| painter.layout_job(label_job(s, 9.0, ink::MUTED)));
    let w = galley
        .size()
        .x
        .max(state.as_ref().map_or(0.0, |s| s.size().x))
        + u * 11.0;
    let h = galley.size().y + state.as_ref().map_or(0.0, |s| s.size().y) + u * 4.0;
    let rect = Rect::from_min_size(
        pos2(
            ((at.x - w / 2.0) / u).round() * u,
            ((at.y - h - u * 3.0) / u).round() * u,
        ),
        vec2(w, h),
    );
    chrome::card(painter, rect, u, ink::CARD, ink::LINE);
    // A tick of the team's colour down its left side.
    painter.rect_filled(
        Rect::from_min_size(
            rect.min + vec2(u * 2.0, u * 2.0),
            vec2(u, rect.height() - u * 4.0),
        ),
        0.0,
        color(team.base),
    );
    // A little point down towards whoever it names.
    let tip = pos2(rect.center().x, rect.max.y + u * 2.0);
    painter.add(egui::Shape::convex_polygon(
        vec![
            pos2(tip.x - u * 2.0, rect.max.y - u),
            pos2(tip.x + u * 2.0, rect.max.y - u),
            tip,
        ],
        ink::LINE,
        Stroke::NONE,
    ));
    let top = rect.min + vec2(u * 6.0, u * 2.0);
    let line = galley.size().y;
    painter.galley(top, galley, ink::PAGE);
    if let Some(state) = state {
        painter.galley(top + vec2(0.0, line), state, ink::MUTED);
    }
}

pub fn color(c: formiga_art::Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}

/// A creature's frame cut down to what is drawn in it, standing in a square: a portrait.
fn snug(frame: &Canvas) -> Canvas {
    let Some((x0, y0, x1, y1)) = frame.alpha_bounds() else {
        return frame.clone();
    };
    let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
    let side = w.max(h) + 2;
    let mut out = Canvas::new(side, side);
    let (ox, oy) = ((side - w) / 2, side - h - 1);
    for y in 0..h {
        for x in 0..w {
            out.set(
                (ox + x) as i32,
                (oy + y) as i32,
                frame.get((x0 + x) as i32, (y0 + y) as i32),
            );
        }
    }
    out
}

/// A picture made `by` times smaller, each pixel the average of the ones it stands for: for the
/// arenas' little pictures, where every pixel of the board cannot be shown.
fn shrunk(canvas: &Canvas, by: u32) -> Canvas {
    let (w, h) = (canvas.width() / by, canvas.height() / by);
    let mut out = Canvas::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (mut r, mut g, mut b, mut a) = (0u32, 0u32, 0u32, 0u32);
            for dy in 0..by {
                for dx in 0..by {
                    let p = canvas.get((x * by + dx) as i32, (y * by + dy) as i32);
                    let alpha = u32::from(p.a);
                    r += u32::from(p.r) * alpha;
                    g += u32::from(p.g) * alpha;
                    b += u32::from(p.b) * alpha;
                    a += alpha;
                }
            }
            if let (Some(r), Some(g), Some(b)) =
                (r.checked_div(a), g.checked_div(a), b.checked_div(a))
            {
                out.set(
                    x as i32,
                    y as i32,
                    formiga_art::Rgba::new(r as u8, g as u8, b as u8, (a / (by * by)) as u8),
                );
            }
        }
    }
    out
}

fn texture(ctx: &egui::Context, name: &str, canvas: &Canvas) -> egui::TextureHandle {
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [canvas.width() as usize, canvas.height() as usize],
        &canvas.rgba_bytes(),
    );
    ctx.load_texture(name, image, egui::TextureOptions::NEAREST)
}
