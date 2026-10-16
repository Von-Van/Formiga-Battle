//! The window. It opens at the Battleground (`station`), where the train comes in with the Handler's
//! companions aboard and the Handler chooses where they are off to: out in the world to a friend,
//! or close to home against the visitors. Then the Handler's notebook (`chrome`) opens, its pages
//! turned one by one as Desktop's journal turns them (`menus`): where to play, who goes and, for a
//! friend, the room. In a match it lies open on the board on its left page and the Handler's notes
//! on its right one; the turn, the beacon and the timer on the leather along the top, and what to
//! do next along the foot. Everything it shows comes from the `Table`.
//!
//! While a turn plays out it is told over the board (`announce`): a headline on a slip at the
//! top, a cartoon word where it happened, and its sound. The last two headlines stay on a card
//! in the board's corner, and the whole match, in full sentences, opens from there. The right
//! page keeps the orders and a field guide to the arena, the rules and the controls.

mod handler;
mod menus;

use crate::announce::{Callout, Style};
use crate::chrome::{self, Kind, ink, label_job};
use crate::garden::Garden;
use crate::host::Host;
use crate::iso::View;
use crate::look::Look;
use crate::scene::{self, TEAM};
use crate::sound::{Cue, Sound, Track};
use crate::table::{Aim, Table, Verb};
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
    /// At the Battleground, the notebook's first spread: the train in with everyone aboard.
    Station,
    /// The notebook open on getting ready: where, who, and the friend's room.
    Journal,
    Playing,
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
    /// The Battleground, the train coming in or in.
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
    /// As `Playing`, with the whole match open over the board.
    Account,
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
    /// The Battleground, once there is a page to size it to; and its picture this frame.
    station: Option<crate::station::Station>,
    station_picture: Option<(i64, egui::TextureHandle)>,
    /// The friend's room was asked for by its code, not opened.
    joining: bool,
    /// What was felt away from the window's context, told with the rest at the next frame.
    felt: Vec<crate::feel::Feedback>,
    /// The chosen arena, pictured under the companions while choosing.
    preview: Option<(String, egui::TextureHandle)>,
    /// The board's marks, painted for the key on its page.
    keys: Vec<egui::TextureHandle>,
    /// How large a board pixel is drawn, in points, this frame.
    board_scale: f32,
    /// Every built-in arena, with a little picture of it, for choosing where to play.
    arenas: Vec<(formiga_battle_core::Arena, Option<egui::TextureHandle>)>,
    sound: Sound,
    /// The turn being announced (when it began showing) and how many of its callouts have been.
    announced: (f32, usize),
    /// Callouts on the board now, and when each came.
    live: Vec<(Callout, f32)>,
    /// The last two headlines, newest last, and whose good news each was.
    recent: Vec<(String, Option<Side>)>,
    /// The whole match, told in full, is open over the board.
    account_open: bool,
    /// The last of the timer's final seconds ticked.
    ticked: Option<u32>,
    /// The end of the match has been heard.
    ended_heard: bool,
    /// The card of sound volumes is open.
    sound_open: bool,
    /// Which part of the field guide is showing.
    guide: Guide,
    /// When the match's ready-up began, and whether its "Go!" has been heard.
    ready: Option<(f32, bool)>,
    /// The turn whose orders began last, as the table's clock has it, for its chirp.
    orders_heard: f32,
}

/// The field guide's topics, one showing at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Guide {
    Win,
    Orders,
    Marks,
}

impl Guide {
    const ALL: [Self; 3] = [Self::Win, Self::Orders, Self::Marks];

    fn word(self) -> &'static str {
        match self {
            Self::Win => "How to win",
            Self::Orders => "Orders",
            Self::Marks => "Marks",
        }
    }
}

/// A field guide row's picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Glyph {
    /// One of the board's marks, by its place in `scene::Key::ALL`.
    Key(usize),
    Stars,
    Watch,
}

/// The line over the board this frame, laid out in board pixels.
struct Caption {
    text: String,
    good_for: Option<Side>,
    alpha: f32,
    rect: Rect,
    /// Where it points down to.
    tip: egui::Pos2,
}

/// The least time a headline is shown before the next takes its place.
const HEADLINE_SECS: f32 = 0.8;

/// How long the ready-up takes: "Ready?", then "Go!".
const READY_SECS: f32 = 1.5;
/// When in it "Go!" comes.
const GO_AT: f32 = 0.85;

impl BattleApp {
    pub fn new(
        host: Host,
        table: Table,
        review: Option<Review>,
        snap: Option<Snap>,
        sound: Sound,
    ) -> Self {
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
            autoplay: matches!(review, Some(Review::Playing | Review::Account)),
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
            preview: None,
            keys: Vec::new(),
            board_scale: 2.0,
            arenas: formiga_battle_core::Arena::built_in()
                .into_iter()
                .map(|a| (a, None))
                .collect(),
            sound,
            announced: (-1.0, 0),
            live: Vec::new(),
            recent: Vec::new(),
            account_open: false,
            ticked: None,
            ended_heard: false,
            sound_open: false,
            guide: Guide::Win,
            ready: None,
            orders_heard: 0.0,
        };
        match review {
            Some(Review::Playing) => app.begin(),
            Some(Review::Account) => {
                app.begin();
                app.account_open = true;
            }
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

    /// Plays what was felt last frame. A companion chosen or taken out is heard as that, not
    /// as the click on its card as well.
    fn hear(&mut self, felt: &[crate::feel::Feedback]) {
        use crate::feel::Feedback;
        let picked = felt
            .iter()
            .any(|f| matches!(f, Feedback::Hop | Feedback::Unpick));
        for f in felt {
            let cue = match f {
                Feedback::Hover => Cue::Hover,
                Feedback::Select if picked => continue,
                Feedback::Select => Cue::Click,
                Feedback::Confirm => Cue::Confirm,
                Feedback::Cancel => Cue::Back,
                Feedback::PageTurn => Cue::PageTurn,
                Feedback::Hop => Cue::Pick,
                Feedback::Unpick => Cue::Unpick,
                Feedback::TrainIn => Cue::TrainArrive,
                Feedback::TrainStops => continue,
                Feedback::TrainOut => Cue::TrainDepart,
            };
            self.sound.play(cue);
        }
    }

    fn begin(&mut self) {
        // The first turn's clock starts once the ready-up is over.
        match self.table.start(self.clock + READY_SECS) {
            Ok(()) => {
                self.stage = Stage::Playing;
                self.error = None;
                self.turned = self.clock;
                self.fresh_match();
                self.ready_up();
            }
            Err(error) => self.error = Some(format!("{error:#}")),
        }
    }

    /// "Ready?", then "Go!" over the board as a match begins.
    fn ready_up(&mut self) {
        self.ready = Some((self.clock, false));
        self.orders_heard = self.table.turn_began;
        self.sound.play(Cue::Begin);
    }

    /// Forgets what was announced in the last match.
    fn fresh_match(&mut self) {
        self.finished_at = None;
        self.announced = (-1.0, 0);
        self.live.clear();
        self.recent.clear();
        self.account_open = false;
        self.ticked = None;
        self.ended_heard = false;
    }

    /// Locks in the orders, if they can be: the button, Enter, or nothing left to give.
    fn lock_in(&mut self, now: f32) {
        if self.lock_quietly(now) {
            self.sound.play(Cue::Lock);
        }
    }

    /// Locks in the orders without a sound of its own: for the button, which the interaction kit
    /// already answers. True if they were locked.
    fn lock_quietly(&mut self, now: f32) -> bool {
        let sent = self.table.friend.as_ref().is_some_and(|f| {
            self.table
                .m
                .as_ref()
                .is_some_and(|m| f.sent(m.board.turn + 1))
        });
        let can = self.table.shown.is_none() && self.table.outcome().is_none() && !sent;
        if can {
            self.table.lock(now);
        }
        can
    }

    /// Fires whatever of the turn being shown has come due: its sound, its word on the board and
    /// its headline, which also goes on the card of the last two.
    fn announce(&mut self, now: f32) {
        self.live.retain(|(_, born)| now - born < 4.0);
        let Some(shown) = &self.table.shown else {
            return;
        };
        if self.announced.0 != shown.started {
            // A new turn: the last one's words and slips make way for it.
            self.announced = (shown.started, 0);
            self.live.clear();
            self.sound.play(Cue::Reveal);
        }
        let t = now - shown.started;
        while let Some(c) = shown.callouts.get(self.announced.1)
            && c.at <= t
        {
            if let Some(cue) = c.cue {
                self.sound.play(cue);
            }
            // The same news twice running is kept once.
            if let Some(headline) = &c.headline
                && self.recent.last().is_none_or(|(last, _)| last != headline)
            {
                self.recent.push((headline.clone(), c.good_for));
                if self.recent.len() > 2 {
                    self.recent.remove(0);
                }
            }
            if c.word.is_some() || c.headline.is_some() {
                self.live.push((c.clone(), now));
            }
            self.announced.1 += 1;
        }
    }

    /// The sounds that come with the clock: the music for where the Handler is, the timer's
    /// last seconds, and the end of a match.
    fn listen(&mut self, now: f32) {
        let finished = self.stage == Stage::Playing && self.table.finished(now);
        let track = if self.stage == Stage::Playing && !finished {
            Track::Match
        } else {
            Track::Lobby
        };
        self.sound.music(Some(track));
        if self.stage != Stage::Playing {
            return;
        }
        if let Some((at, go)) = &mut self.ready
            && !*go
            && now - *at >= GO_AT
        {
            *go = true;
            self.sound.play(Cue::Whistle);
        }
        // Each new turn's orders begin with a chirp from the team, ready for them.
        if self.table.shown.is_none()
            && self.table.outcome().is_none()
            && self.table.turn_began != self.orders_heard
        {
            self.orders_heard = self.table.turn_began;
            self.sound.play(Cue::Ready);
        }
        if self.table.shown.is_none() && self.table.outcome().is_none() {
            let left = self.table.time_left(now);
            let second = left.ceil() as u32;
            if (1..=5).contains(&second) && self.ticked != Some(second) {
                self.ticked = Some(second);
                self.sound.play(Cue::Tick);
            }
        }
        if finished && !self.ended_heard {
            self.ended_heard = true;
            self.sound
                .play(match self.table.outcome().and_then(|o| o.winner) {
                    Some(side) if side == self.table.you => Cue::Fanfare,
                    Some(_) => Cue::Womp,
                    None => Cue::Shrug,
                });
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
        let mut felt = std::mem::take(&mut self.felt);
        felt.extend(crate::feel::take(&ctx));
        self.hear(&felt);
        let now = self.clock;
        if self.turned < 0.0 {
            self.turned = now;
        }
        if self.table.friend.is_some() && self.table.poll(now) {
            self.stage = Stage::Playing;
            self.notebook_open = false;
            self.turned = now;
            self.fresh_match();
            self.ready_up();
        }
        if self.stage == Stage::Playing {
            let readying = self.ready.is_some_and(|(at, _)| now - at < READY_SECS);
            if self.autoplay && !readying && self.table.shown.is_none() && !self.table.finished(now)
            {
                self.table.coach_orders();
                self.lock_in(now);
            }
            if self.table.shown.is_none()
                && self.table.outcome().is_none()
                && self.table.time_left(now) <= 0.0
            {
                self.lock_in(now);
            }
            if self.table.finished(now) {
                self.finished_at.get_or_insert(now);
            }
            self.announce(now);
        }
        self.listen(now);
        // The garden moves even while nobody does.
        ctx.request_repaint();
        self.keys(&ctx);
        self.pokes.retain(|(_, at)| now - at < 0.6);
        let full = ui.max_rect();
        if self.stage == Stage::Station {
            self.station_tick(&ctx, now);
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
                    Stage::Station => {
                        ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
                            ui.multiply_opacity(fade);
                            self.station_left(ui, &ctx, u, now);
                        });
                        ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                            ui.multiply_opacity(fade);
                            self.station_right(ui, u, now);
                        });
                    }
                    Stage::Playing => {
                        self.board_view(ui, &ctx, layout.left.shrink(u * 4.0), now, u);
                        self.board_page(ui, layout.left, now, u);
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
        if self.stage == Stage::Journal
            && !ctx.egui_wants_keyboard_input()
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            crate::feel::emit(ctx, crate::feel::Feedback::Cancel);
            self.go_back();
            return;
        }
        if self.stage != Stage::Playing || ctx.egui_wants_keyboard_input() {
            return;
        }
        let pressed = |key| ctx.input(|i| i.key_pressed(key));
        if pressed(egui::Key::L) {
            self.toggle_account_by_key();
        }
        if pressed(egui::Key::Escape) {
            if self.account_open {
                self.toggle_account_by_key();
            } else if self.table.verb.is_some() {
                self.table.verb = None;
                self.sound.play(Cue::Cancel);
            }
        }
        if self.table.shown.is_some() {
            return;
        }
        for (key, verb) in [
            (egui::Key::M, Verb::Move),
            (egui::Key::H, Verb::Hold),
            (egui::Key::C, Verb::Contest),
            (egui::Key::U, Verb::Use),
        ] {
            if pressed(key) && self.table.selected.is_some() {
                self.table.choose_verb(verb);
                self.sound.play(Cue::Click);
            }
        }
        if pressed(egui::Key::Enter) {
            self.lock_in(self.clock);
        }
        for (key, index) in [
            (egui::Key::Num1, 0),
            (egui::Key::Num2, 1),
            (egui::Key::Num3, 2),
        ] {
            if pressed(key) {
                self.table.select(index);
                if self.table.selected == Some(index) {
                    self.sound.play(Cue::Select);
                }
            }
        }
    }

    /// Opens or closes the whole match. Buttons are heard through the interaction kit; the L
    /// key is heard here.
    fn toggle_account(&mut self) {
        self.account_open = !self.account_open;
    }

    fn toggle_account_by_key(&mut self) {
        self.toggle_account();
        self.sound.play(if self.account_open {
            Cue::Pick
        } else {
            Cue::Unpick
        });
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
            if self.stage == Stage::Playing && !open {
                crate::feel::press(
                    ui.ctx(),
                    Id::new(("tab", i)),
                    &response,
                    crate::feel::Feel::Select,
                );
            }
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
        self.sound_button(ui, layout);
        let area = Rect::from_min_max(
            pos2(layout.patch.max.x + u * 8.0, layout.patch.min.y),
            pos2(layout.left.max.x - u * 4.0, layout.patch.max.y),
        );
        if self.stage == Stage::Journal {
            self.journal_strip(ui, area, u, now);
            return;
        }
        if self.stage == Stage::Station {
            ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(label_job("The Battleground", chrome::LABEL, ink::COVER));
                });
            });
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

    /// The sound, on the leather at the top right: a little speaker that opens a card with the
    /// music's and the sounds' volumes and a mute.
    fn sound_button(&mut self, ui: &mut egui::Ui, layout: &Layout) {
        let u = layout.unit;
        let levels = self.sound.levels();
        let word = if levels.muted { "Muted" } else { "Sound" };
        let galley = ui
            .painter()
            .layout_job(label_job(word, chrome::LABEL, ink::DEBOSS));
        let size = vec2(galley.size().x + u * 23.0, layout.patch.height());
        let rect = Rect::from_min_size(
            pos2(layout.right.max.x - size.x - u * 2.0, layout.patch.min.y),
            size,
        );
        let response = ui
            .interact(rect, Id::new("sound"), Sense::click())
            .on_hover_text("Music and sound volume");
        crate::feel::press(
            ui.ctx(),
            Id::new("sound"),
            &response,
            if self.sound_open {
                crate::feel::Feel::Cancel
            } else {
                crate::feel::Feel::Select
            },
        );
        let painter = ui.painter();
        let colour = if response.hovered() || self.sound_open {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            chrome::stepped(painter, rect, u, Color32::from_rgb(0x87, 0x60, 0x4a));
            ink::COVER
        } else {
            ink::DEBOSS
        };
        let waves = if levels.muted || levels.music + levels.sounds == 0.0 {
            0
        } else if levels.music.max(levels.sounds) < 0.5 {
            1
        } else {
            2
        };
        speaker(
            painter,
            pos2(rect.min.x + u * 7.0, rect.center().y),
            waves,
            colour,
            u,
        );
        let galley = painter.layout_job(label_job(word, chrome::LABEL, colour));
        painter.galley(
            pos2(
                rect.min.x + u * 18.0,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            colour,
        );
        if response.clicked() {
            self.sound_open = !self.sound_open;
        }
        if self.sound_open {
            self.sound_card(ui.ctx(), rect, u);
        }
    }

    /// The card of volumes under the speaker: music, sounds, and a mute for everything.
    fn sound_card(&mut self, ctx: &egui::Context, under: Rect, u: f32) {
        let mut levels = self.sound.levels();
        let before = levels;
        let width = 230.0;
        let mut close = false;
        egui::Area::new(Id::new("sound-card"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(under.max.x - width, under.max.y + u * 3.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(ink::CARD)
                    .stroke(Stroke::new(u, ink::LINE))
                    .shadow(egui::Shadow {
                        offset: [2, 3],
                        blur: 0,
                        spread: 0,
                        color: Color32::from_black_alpha(60),
                    })
                    .inner_margin(egui::Margin::same((u * 6.0).round() as i8))
                    .show(ui, |ui| {
                        ui.set_width(width - u * 12.0);
                        ui.horizontal(|ui| {
                            chrome::kicker(ui, "Sound");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if chrome::button(ui, "close", Kind::Back, false, true, None, u)
                                        .clicked()
                                    {
                                        close = true;
                                    }
                                },
                            );
                        });
                        ui.spacing_mut().slider_width = width - u * 12.0 - 70.0;
                        for (word, level) in
                            [("Music", &mut levels.music), ("Sounds", &mut levels.sounds)]
                        {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    vec2(56.0, 18.0),
                                    egui::Label::new(RichText::new(word).size(12.5)),
                                );
                                ui.add(egui::Slider::new(level, 0.0..=1.0).show_value(false));
                            });
                        }
                        ui.checkbox(&mut levels.muted, "Mute everything");
                    });
            });
        if levels != before {
            let louder = levels.sounds > before.sounds || (before.muted && !levels.muted);
            self.sound.set_levels(levels);
            // A sound to hear how loud the sounds now are.
            if louder || levels.sounds < before.sounds {
                self.sound.play(Cue::Tick);
            }
        }
        if close {
            self.sound_open = false;
        }
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
            Stage::Station => match &self.station {
                Some(s) if s.leaving() => "Off they go…".to_owned(),
                Some(s) if s.stopped(now) => {
                    "Choose where they are off to, or look back over your history.".to_owned()
                }
                _ => "Click the picture, or press Space, to hurry the train in.".to_owned(),
            },
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
                    "Click someone of theirs, or the lever. Right-click or Escape cancels."
                        .to_owned()
                }
                (Some(i), None) => format!(
                    "{}: click the ground to MOVE, theirs or the lever to CONTEST; right-click to HOLD. Enter locks in.",
                    self.table.m.as_ref().map_or(String::new(), |m| m
                        .profile(Slot::new(self.table.you, i))
                        .name
                        .clone())
                ),
                (Some(_), Some(Verb::Use)) => {
                    "Click where to use it. Right-click or Escape cancels.".to_owned()
                }
                (None, _) => "Click one of yours on the board to give it an order.".to_owned(),
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
        // The board's scale is known before it is painted: it is drawn as large as the page
        // allows, by whole screen pixels.
        let scale = {
            let ppp = ctx.pixels_per_point();
            let (w, h) = (self.view.size.0 as f32, self.view.size.1 as f32);
            ((area.width() * ppp) / w)
                .min((area.height() * ppp) / h)
                .floor()
                .max(1.0)
                / ppp
        };
        let (lettering, caption) = self.overlays(&frame, ctx, scale, u, now);
        marks.lettering = lettering;
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
        let giving = self.table.shown.is_none() && self.table.outcome().is_none();
        let aims = self
            .hover
            .filter(|_| giving)
            .map(|tile| (self.table.aim(tile, false), self.table.aim(tile, true)));
        if aims.is_some_and(|(plain, right)| plain != Aim::Nothing || right != Aim::Nothing) {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let pressed = if response.clicked() {
            Some(false)
        } else if response.secondary_clicked() {
            Some(true)
        } else {
            None
        };
        if giving
            && let Some(hold) = pressed
            && let Some(tile) = response.interact_pointer_pos().and_then(to_tile)
        {
            // An order picked by its button or key is let go by a right click, as before.
            if hold && self.table.verb.is_some() {
                self.table.verb = None;
                self.sound.play(Cue::Cancel);
            } else {
                match self.table.press(tile, hold) {
                    Aim::Choose(_) => self.sound.play(Cue::Select),
                    Aim::Give(_) => {
                        self.pokes.push((tile, now));
                        self.sound.play(Cue::Order);
                    }
                    Aim::Nothing => {}
                }
            }
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
        if self.table.shown.is_none()
            && self.table.outcome().is_none()
            && let Some(slot) = marks.selected
        {
            let f = frame.figure(slot);
            let look = &self.table.looks[slot.side.index()][slot.index as usize];
            let (hx, hy) = scene::head(&self.view, f, look);
            let bob = if self.table.reduce_motion {
                0.0
            } else {
                (now * 5.0).sin().abs() * 3.0
            };
            let tip =
                rect.min + vec2(hx as f32, hy as f32 - 7.0) * scale - vec2(0.0, u * 22.0 + bob * u);
            pointer_arrow(&painter, tip, color(TEAM[slot.side.index()].light), u);
        }
        if let Some(caption) = &caption {
            self.caption(&painter, rect, scale, caption, u);
        }
        self.confetti(&painter, rect, now, u);
        self.result_stamp(&painter, rect, now, u);
        if let (Some(pointer), Some((plain, right))) = (response.hover_pos(), aims) {
            self.cursor_hint(&painter, pointer, plain, right, u);
        }
    }

    /// Lays out this frame's words and line over the board, in board pixels, so none covers a
    /// face or another: the line over the head of whoever it is about, each cartoon word above
    /// whoever it happened to (or beside them, if that is taken), composure lost beside the head,
    /// and "READY?" then "GO!" as a match begins. The words are Formiga pixel art for the board
    /// to paint (`lettering`); the line is drawn on the notebook's own card after it.
    fn overlays(
        &self,
        frame: &crate::playback::Frame,
        ctx: &egui::Context,
        scale: f32,
        u: f32,
        now: f32,
    ) -> (Vec<scene::Lettered>, Option<Caption>) {
        use crate::lettering::{Paint, word};
        let reduce = self.table.reduce_motion;
        let (w, h) = (self.view.size.0 as f32, self.view.size.1 as f32);
        // Every face on the board, with the pips over it: kept clear.
        let heads: Vec<(Slot, f32, f32)> = Slot::all()
            .filter(|s| !frame.figure(*s).out)
            .map(|s| {
                let look = &self.table.looks[s.side.index()][s.index as usize];
                let (hx, hy) = scene::head(&self.view, frame.figure(s), look);
                (s, hx as f32, hy as f32)
            })
            .collect();
        let mut taken: Vec<Rect> = heads
            .iter()
            .map(|(_, x, y)| Rect::from_min_max(pos2(x - 14.0, y - 10.0), pos2(x + 14.0, y + 24.0)))
            .collect();
        let anchor = |c: &Callout| -> Option<(f32, f32, Option<Rect>)> {
            if let Some(slot) = c.about
                && let Some((_, x, y)) = heads.iter().find(|(s, _, _)| *s == slot)
            {
                let own = Rect::from_min_max(pos2(x - 14.0, y - 10.0), pos2(x + 14.0, y + 24.0));
                return Some((*x, y - 10.0, Some(own)));
            }
            let (x, y) = c.place?;
            let (sx, sy) = self.view.screen(x, y);
            Some((sx, sy - 26.0, None))
        };
        let clear = |r: Rect, taken: &[Rect]| {
            r.min.x >= 1.0
                && r.max.x <= w - 1.0
                && r.min.y >= 1.0
                && !taken.iter().any(|t| t.intersects(r))
        };

        // The line: one at a time, each given a moment of its own before the next.
        let mut told: Vec<(&Callout, f32)> = Vec::new();
        for (c, born) in &self.live {
            if c.headline.is_none() || anchor(c).is_none() {
                continue;
            }
            let shown = told
                .last()
                .map_or(*born, |(_, last)| born.max(last + HEADLINE_SECS));
            // A line that would come too long after its moment is left to the card of the last
            // two, so the board never falls behind the turn.
            if shown - born > 1.0 {
                continue;
            }
            told.push((c, shown));
        }
        let mut caption = None;
        // Once the match is over, its stamp says so; no line over it.
        if self.finished_at.is_some() {
            told.clear();
        }
        if let Some(k) = told.iter().rposition(|(_, shown)| *shown <= now) {
            let (c, shown) = told[k];
            let age = now - shown;
            let fade_in = if reduce { 1.0 } else { (age / 0.15).min(1.0) };
            let alpha = fade_in * ((2.2 - age) / 0.4).clamp(0.0, 1.0);
            if let (Some(text), Some((ax, ay, _)), true) = (&c.headline, anchor(c), alpha > 0.0) {
                let size = ctx
                    .fonts_mut(|f| {
                        f.layout_no_wrap(text.clone(), FontId::proportional(13.0), ink::PAGE)
                    })
                    .size();
                let size = (size + vec2(u * 16.0, u * 6.0)) / scale;
                let x = (ax - size.x / 2.0).clamp(2.0, (w - size.x - 2.0).max(2.0));
                // Room under it for its own word, which goes just over the head.
                let room = c.word.map_or(0.0, |text| {
                    let big = i32::from(c.style == Style::Big) + 1;
                    word(text, big, Paint::CREAM).height() as f32 + 4.0
                });
                let mut r = Rect::from_min_size(pos2(x, ay - 4.0 - room - size.y), size);
                // Up, clear of anyone else's face.
                for _ in 0..12 {
                    if clear(r, &taken) || r.min.y < 4.0 {
                        break;
                    }
                    r = r.translate(vec2(0.0, -4.0));
                }
                taken.push(r);
                caption = Some(Caption {
                    text: text.clone(),
                    good_for: c.good_for,
                    alpha,
                    rect: r,
                    tip: pos2(ax.clamp(r.min.x + 4.0, r.max.x - 4.0), ay),
                });
            }
        }

        // The words, newest first, each in the first clear spot round whoever it is about.
        let mut out = Vec::new();
        let mut words = 0;
        let mut shown_damage = false;
        for (k, (c, born)) in self.live.iter().enumerate().rev() {
            let Some(text) = c.word else {
                continue;
            };
            let Some((ax, ay, _)) = anchor(c) else {
                continue;
            };
            let age = now - born;
            let life = match c.style {
                Style::Big => 1.5,
                Style::Word => 1.0,
                Style::Damage => 1.1,
            };
            if age > life {
                continue;
            }
            if c.style != Style::Damage {
                words += 1;
                if words > 3 {
                    continue;
                }
            }
            let alpha = 1.0 - ((age - life * 0.65) / (life * 0.35)).clamp(0.0, 1.0);
            let popping = !reduce && age < 0.1;
            let (picture, rise) = match c.style {
                Style::Damage => (
                    word(text, 1, Paint::HURT),
                    if reduce { 0.0 } else { age * 14.0 },
                ),
                style => {
                    let colours = match c.good_for {
                        _ if style == Style::Big => Paint::GOLD,
                        Some(side) => Paint::team(TEAM[side.index()]),
                        None => Paint::CREAM,
                    };
                    let scale = if style == Style::Big { 2 } else { 1 } + i32::from(popping);
                    (
                        word(text, scale, colours),
                        if reduce { 0.0 } else { age * 8.0 },
                    )
                }
            };
            let (pw, ph) = (picture.width() as f32, picture.height() as f32);
            let above = pos2(ax - pw / 2.0, ay - ph - 2.0);
            let left = pos2(ax - 16.0 - pw, ay + 4.0);
            let right = pos2(ax + 16.0, ay + 4.0);
            let higher = pos2(ax - pw / 2.0, ay - ph * 2.0 - 4.0);
            let above_left = pos2(ax - pw - 4.0, ay - ph - 2.0);
            let above_right = pos2(ax + 4.0, ay - ph - 2.0);
            let lean = if k % 2 == 0 {
                [right, left]
            } else {
                [left, right]
            };
            let tries = if c.style == Style::Damage {
                [lean[0], lean[1], above, above_right, above_left, higher]
            } else {
                [above, above_right, above_left, lean[0], lean[1], higher]
            };
            let found = tries
                .into_iter()
                .map(|p| Rect::from_min_size(p, vec2(pw, ph)))
                .find(|r| clear(*r, &taken));
            // An older word with nowhere clear to go makes way; the newest word, and the newest
            // number, always show.
            let newest = if c.style == Style::Damage {
                !shown_damage
            } else {
                words == 1
            };
            shown_damage |= c.style == Style::Damage;
            let Some(spot) =
                found.or_else(|| newest.then(|| Rect::from_min_size(tries[0], vec2(pw, ph))))
            else {
                continue;
            };
            taken.push(spot);
            out.push((
                picture,
                spot.min.x.round() as i32,
                (spot.min.y - rise).round() as i32,
                (alpha.clamp(0.0, 1.0) * 255.0) as u8,
            ));
        }

        if let Some((at, _)) = self.ready {
            let age = now - at;
            if age <= READY_SECS {
                let (text, since, colours) = if age < GO_AT {
                    ("READY?", age, Paint::CREAM)
                } else {
                    ("GO!", age - GO_AT, Paint::GOLD)
                };
                let size = 3 + i32::from(!reduce && since < 0.1);
                let alpha = if age < GO_AT {
                    1.0
                } else {
                    1.0 - ((age - READY_SECS + 0.25) / 0.25).clamp(0.0, 1.0)
                };
                let picture = word(text, size, colours);
                let x = (w - picture.width() as f32) / 2.0;
                let y = h * 0.42 - picture.height() as f32 / 2.0;
                out.push((
                    picture,
                    x.round() as i32,
                    y.round() as i32,
                    (alpha * 255.0) as u8,
                ));
            }
        }
        (out, caption)
    }

    /// The line over the board, on a little card pointing down at whoever it is about.
    fn caption(&self, painter: &egui::Painter, board: Rect, scale: f32, caption: &Caption, u: f32) {
        let rect = Rect::from_min_max(
            board.min + caption.rect.min.to_vec2() * scale,
            board.min + caption.rect.max.to_vec2() * scale,
        );
        let rect = Rect::from_min_size(
            pos2((rect.min.x / u).round() * u, (rect.min.y / u).round() * u),
            rect.size(),
        );
        let tip = board.min + caption.tip.to_vec2() * scale;
        let mut painter = painter.clone();
        painter.set_opacity(caption.alpha);
        chrome::card(&painter, rect, u, ink::CARD, ink::LINE);
        tick(&painter, rect, caption.good_for, u);
        let px = tip.x.clamp(rect.min.x + u * 4.0, rect.max.x - u * 4.0);
        if tip.y > rect.max.y + u * 2.0 {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(px - u * 2.5, rect.max.y - u),
                    pos2(px + u * 2.5, rect.max.y - u),
                    pos2(px, rect.max.y + u * 2.5),
                ],
                ink::LINE,
                Stroke::NONE,
            ));
        }
        let galley =
            painter.layout_no_wrap(caption.text.clone(), FontId::proportional(13.0), ink::PAGE);
        painter.galley(
            pos2(
                rect.min.x + u * 9.0,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            ink::PAGE,
        );
    }

    /// Confetti over the board when the Handler's team wins: a short shower, the same every time.
    fn confetti(&self, painter: &egui::Painter, board: Rect, now: f32, u: f32) {
        let (Some(at), Some(outcome)) = (self.finished_at, self.table.outcome()) else {
            return;
        };
        if outcome.winner != Some(self.table.you) || self.table.reduce_motion {
            return;
        }
        let age = now - at;
        if age > 3.0 {
            return;
        }
        let team = TEAM[self.table.you.index()];
        let colours = [
            color(team.light),
            color(team.shine),
            ink::BRASS_LIGHT,
            ink::MINT,
            Color32::from_rgb(0xff, 0xf3, 0xc4),
        ];
        let fade = 1.0 - ((age - 2.2) / 0.8).clamp(0.0, 1.0);
        for k in 0..70u32 {
            // A scrap's own numbers, from its index alone.
            let h = k.wrapping_mul(2_654_435_761).rotate_left(7) ^ 0x5bd1_e995;
            let unit = |shift: u32| ((h >> shift) & 0xff) as f32 / 255.0;
            let start = unit(0) * 0.6;
            let t = age - start;
            if t < 0.0 {
                continue;
            }
            let x = board.min.x
                + board.width() * (0.1 + 0.8 * unit(8))
                + (t * (2.0 + 3.0 * unit(16))).sin() * u * 6.0;
            let y =
                board.min.y + board.height() * 0.05 + t * board.height() * (0.25 + 0.15 * unit(24));
            if y > board.max.y {
                continue;
            }
            let spin = (t * (4.0 + 6.0 * unit(4))).sin();
            let size = vec2(u * 3.0 * spin.abs().max(0.25), u * 2.0);
            painter.rect_filled(
                Rect::from_center_size(pos2(x, y), size),
                0.0,
                colours[(h % colours.len() as u32) as usize].gamma_multiply(fade),
            );
        }
    }

    /// A little card by the pointer saying what a click there would do, and a right click.
    fn cursor_hint(
        &self,
        painter: &egui::Painter,
        pointer: egui::Pos2,
        plain: Aim,
        right: Aim,
        u: f32,
    ) {
        let Some(m) = &self.table.m else {
            return;
        };
        let say = |aim: Aim| match aim {
            Aim::Choose(i) => Some(format!(
                "choose {}",
                m.profile(Slot::new(self.table.you, i)).name
            )),
            Aim::Give(order) => Some(speak(m, &order, self.table.carrying())),
            Aim::Nothing => None,
        };
        let mut lines = Vec::new();
        if let Some(text) = say(plain) {
            lines.push((false, text));
        }
        if self.table.verb.is_some() {
            lines.push((true, "cancel".to_owned()));
        } else if let Some(text) = say(right).filter(|_| right != plain) {
            lines.push((true, text));
        }
        if lines.is_empty() {
            return;
        }
        let galleys: Vec<_> = lines
            .into_iter()
            .map(|(right, text)| {
                (
                    right,
                    painter.layout_no_wrap(text, FontId::proportional(12.5), ink::PAGE),
                )
            })
            .collect();
        let icon = vec2(u * 6.0, u * 9.0);
        let row = galleys
            .iter()
            .map(|(_, g)| g.size().y)
            .fold(icon.y, f32::max)
            + u * 2.0;
        let width = galleys.iter().map(|(_, g)| g.size().x).fold(0.0, f32::max) + icon.x + u * 14.0;
        let height = row * galleys.len() as f32 + u * 6.0;
        let min = pointer + vec2(u * 10.0, u * 12.0);
        let rect = Rect::from_min_size(
            pos2((min.x / u).round() * u, (min.y / u).round() * u),
            vec2(width, height),
        );
        chrome::card(painter, rect, u, ink::CARD, ink::LINE);
        for (k, (right, galley)) in galleys.into_iter().enumerate() {
            let top = rect.min.y + u * 3.0 + k as f32 * row;
            let at = Rect::from_min_size(
                pos2(rect.min.x + u * 5.0, top + (row - icon.y) / 2.0 - u),
                icon,
            );
            mouse(painter, at, right, u);
            let h = galley.size().y;
            painter.galley(
                pos2(at.max.x + u * 4.0, top + (row - h) / 2.0 - u),
                galley,
                ink::PAGE,
            );
        }
    }

    /// What is written on the board's page round it: the turn's slip and its headlines across
    /// the top, the last two headlines on a card in the upper right (the whole match opens from
    /// there), and the controls in the lower left while orders are being given.
    fn board_page(&mut self, ui: &mut egui::Ui, page: Rect, now: f32, u: f32) {
        let painter = ui.painter().clone();
        self.turn_banner(&painter, page, now, u);
        self.orders_banner(&painter, page, now, u);
        let card = self.recent_card(ui, page, u);
        if self.account_open {
            self.account(ui.ctx(), page, card, u);
        }
        if self.table.shown.is_none() && self.table.outcome().is_none() {
            controls(&painter, page, u);
        }
    }

    /// The last two headlines, kept on a card in the board's upper right as a reminder, newest
    /// first; and under them, the way into the whole match told in full. Returns the card.
    fn recent_card(&mut self, ui: &mut egui::Ui, page: Rect, u: f32) -> Rect {
        let painter = ui.painter().clone();
        let width = (page.width() * 0.3).clamp(220.0, 300.0);
        let inner = width - u * 14.0;
        let lines: Vec<(std::sync::Arc<egui::Galley>, Option<Side>)> = if self.recent.is_empty() {
            vec![(
                painter.layout(
                    "Nothing yet: the first turn is yours.".to_owned(),
                    FontId::proportional(12.5),
                    ink::MUTED,
                    inner,
                ),
                None,
            )]
        } else {
            self.recent
                .iter()
                .rev()
                .enumerate()
                .map(|(k, (text, side))| {
                    let colour = if k == 0 { ink::PAGE } else { ink::MUTED };
                    (
                        painter.layout(text.clone(), FontId::proportional(12.5), colour, inner),
                        *side,
                    )
                })
                .collect()
        };
        let title = painter.layout_job(label_job("Just now", 9.5, ink::FOREST));
        let link_text = if self.account_open {
            "Close the whole match (L)"
        } else {
            "The whole match (L)"
        };
        let link = painter.layout_job(label_job(link_text, 9.0, ink::FOREST));
        let body: f32 = lines.iter().map(|(g, _)| g.size().y + u * 2.0).sum();
        let height = u * 9.0 + title.size().y + body + link.size().y + u * 6.0;
        let corner = pos2(page.max.x - u * 8.0 - width, page.min.y + u * 6.0);
        let rect = Rect::from_min_size(
            pos2((corner.x / u).round() * u, (corner.y / u).round() * u),
            vec2(width, height),
        );
        chrome::card(&painter, rect, u, ink::CARD, ink::LINE.gamma_multiply(0.45));
        let mut y = rect.min.y + u * 4.0;
        painter.galley(pos2(rect.min.x + u * 5.0, y), title.clone(), ink::FOREST);
        y += title.size().y + u * 3.0;
        for (galley, side) in lines {
            let h = galley.size().y;
            if side.is_some() || !self.recent.is_empty() {
                let mark = Rect::from_min_size(
                    pos2(rect.min.x + u * 5.0, y + u),
                    vec2(u * 1.5, h - u * 2.0),
                );
                painter.rect_filled(
                    mark,
                    0.0,
                    side.map_or(ink::LINE.gamma_multiply(0.4), |s| {
                        color(TEAM[s.index()].base)
                    }),
                );
            }
            painter.galley(pos2(rect.min.x + u * 9.0, y), galley, ink::PAGE);
            y += h + u * 2.0;
        }
        y += u;
        let link_rect =
            Rect::from_min_size(pos2(rect.min.x + u * 5.0, y), link.size() + vec2(0.0, u));
        let response = ui.interact(link_rect, Id::new("whole-match"), Sense::click());
        crate::feel::press(
            ui.ctx(),
            Id::new("whole-match"),
            &response,
            if self.account_open {
                crate::feel::Feel::Cancel
            } else {
                crate::feel::Feel::Select
            },
        );
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            painter.hline(
                link_rect.x_range(),
                link_rect.bottom(),
                Stroke::new(u * 0.7, ink::FOREST),
            );
        }
        painter.galley(link_rect.min, link, ink::FOREST);
        if response.clicked() {
            self.toggle_account();
        }
        rect
    }

    /// The whole match told in full sentences, every turn, in a little menu that opens under the
    /// card of the last two: for checking what happened.
    fn account(&mut self, ctx: &egui::Context, page: Rect, card: Rect, u: f32) {
        let width = card.width().max(320.0);
        let top = card.max.y + u * 3.0;
        let mut close = false;
        egui::Area::new(Id::new("account"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(card.max.x - width, top))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(ink::CARD)
                    .stroke(Stroke::new(u, ink::LINE))
                    .shadow(egui::Shadow {
                        offset: [2, 3],
                        blur: 0,
                        spread: 0,
                        color: Color32::from_black_alpha(60),
                    })
                    .inner_margin(egui::Margin::same((u * 6.0).round() as i8))
                    .show(ui, |ui| {
                        ui.set_width(width - u * 12.0);
                        ui.horizontal(|ui| {
                            chrome::kicker(ui, "The whole match");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if chrome::button(ui, "close", Kind::Back, false, true, None, u)
                                        .clicked()
                                    {
                                        close = true;
                                    }
                                },
                            );
                        });
                        egui::ScrollArea::vertical()
                            .max_height((page.max.y - top - u * 16.0).max(120.0))
                            .min_scrolled_height((page.max.y - top - u * 16.0).clamp(120.0, 420.0))
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
                                for (turn, lines) in &self.table.log {
                                    ui.add_space(u);
                                    ui.label(label_job(
                                        &format!("Turn {turn}"),
                                        chrome::LABEL,
                                        ink::MUTED,
                                    ));
                                    if lines.is_empty() {
                                        ui.label(
                                            RichText::new("· Everyone went where they were told.")
                                                .size(12.5)
                                                .color(ink::MUTED),
                                        );
                                    }
                                    for line in lines {
                                        ui.label(RichText::new(format!("· {line}")).size(12.5));
                                    }
                                }
                            });
                    });
            });
        if close {
            self.toggle_account();
        }
    }

    /// While a turn's orders are revealed, a slip drops onto the top of the board naming it.
    fn turn_banner(&self, painter: &egui::Painter, board: Rect, now: f32, u: f32) {
        let (Some(shown), Some(m)) = (&self.table.shown, &self.table.m) else {
            return;
        };
        let age = now - shown.started;
        // Gone by the first tick, when the turn's own headlines begin.
        let life = crate::playback::REVEAL_SECS;
        if age > life || self.table.finished(now) {
            return;
        }
        let (drop, alpha) = if self.table.reduce_motion {
            (0.0, 1.0)
        } else {
            let t = (age / 0.25).min(1.0);
            let ease = 1.0 - (1.0 - t).powi(3);
            let out = ((life - age) / 0.2).clamp(0.0, 1.0);
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

    /// As each turn's orders begin, a slip in the Handler's colour says so: whose move it is.
    fn orders_banner(&self, painter: &egui::Painter, page: Rect, now: f32, u: f32) {
        let Some(m) = &self.table.m else {
            return;
        };
        let age = now - self.table.turn_began;
        let life = 1.4;
        if self.table.shown.is_some() || m.outcome().is_some() || !(0.0..life).contains(&age) {
            return;
        }
        let (drop, alpha) = if self.table.reduce_motion {
            (0.0, 1.0)
        } else {
            let t = (age / 0.22).min(1.0);
            let out = ((life - age) / 0.3).clamp(0.0, 1.0);
            ((1.0 - (1.0 - t).powi(3)) * 14.0 * u - 14.0 * u, out)
        };
        let galley = painter.layout_job(label_job(
            &format!("Turn {}: your orders", m.board.turn + 1),
            13.0,
            ink::FOREST,
        ));
        let size = galley.size() + vec2(u * 22.0, u * 9.0);
        let rect = Rect::from_min_size(
            pos2(page.center().x - size.x / 2.0, page.min.y + u * 8.0 + drop),
            size,
        );
        let rect = Rect::from_min_size(
            pos2((rect.min.x / u).round() * u, (rect.min.y / u).round() * u),
            rect.size(),
        );
        let mut painter = painter.clone();
        painter.set_opacity(alpha);
        chrome::card(&painter, rect, u, ink::MINT, ink::FOREST);
        tick(&painter, rect, Some(self.table.you), u);
        painter.galley(
            pos2(
                rect.min.x + u * 13.0,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            ink::FOREST,
        );
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
            let carried = self
                .table
                .m
                .as_ref()
                .and_then(|m| m.fighter(Slot::new(you, i)).carrying);
            let order_text = match (standing[i as usize], order) {
                (false, _) => "Sitting out, dizzy".to_owned(),
                (true, Some(order)) => speak(self.table.m.as_ref().expect("on"), &order, carried),
                (true, None) => match self.table.fallback(i) {
                    Some(f) => format!(
                        "No order: will {}",
                        speak(self.table.m.as_ref().expect("on"), &f, carried)
                    ),
                    None => "No order".to_owned(),
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
                            if let Some(item) = carried {
                                ui.label(
                                    RichText::new(format!("Carrying the {}", item.name()))
                                        .size(11.5)
                                        .italics()
                                        .color(ink::FOREST),
                                );
                            }
                            let mut picked = None;
                            let mut cleared = false;
                            if giving && up {
                                ui.add_space(u);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = u * 2.0;
                                    for v in Verb::ALL {
                                        if v == Verb::Use && carried.is_none() {
                                            continue;
                                        }
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
                                            Kind::Back,
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
            // Heard through the interaction kit, as every button and card is.
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
            .on_hover_text(
                "Enter. Anyone without an order chooses for itself when the timer runs out.",
            )
            .clicked()
            {
                self.lock_quietly(now);
            }
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
        self.field_guide(ui, ctx, u);
    }

    /// A glance at the arena and the rules under the orders: one topic at a time (how to win,
    /// the orders, the board's marks), each a few short lines with a picture, the detail on
    /// hover. The controls are on the board itself.
    fn field_guide(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        if self.keys.is_empty() {
            self.keys = scene::Key::ALL
                .iter()
                .enumerate()
                .map(|(i, key)| texture(ctx, &format!("key-{i}"), &key.picture()))
                .collect();
        }
        let arena = &self.table.arena;
        ui.horizontal(|ui| {
            chrome::kicker(ui, &format!("The {}", arena.name));
        })
        .response
        .on_hover_text(crate::garden::Dressing::of(arena).blurb());
        ui.add_space(u * 2.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = u * 2.0;
            for topic in Guide::ALL {
                if chrome::button(
                    ui,
                    topic.word(),
                    Kind::Plain,
                    self.guide == topic,
                    true,
                    None,
                    u,
                )
                .clicked()
                {
                    self.guide = topic;
                }
            }
        });
        ui.add_space(u * 3.0);
        let ruleset = self.table.ruleset;
        let key =
            |k: scene::Key| Glyph::Key(scene::Key::ALL.iter().position(|x| *x == k).unwrap_or(0));
        let rows: Vec<(Glyph, String, String, String)> = match self.guide {
            Guide::Win => vec![
                (
                    key(scene::Key::Beacon),
                    "Light the beacon".to_owned(),
                    format!("hold the yard alone, {} ticks", ruleset.capture_ticks),
                    "Keep someone steady in the yard while none of theirs is there. Each tick of it lights a lamp; light them all and you win.".to_owned(),
                ),
                (
                    Glyph::Stars,
                    "Or make them dizzy".to_owned(),
                    "all three of theirs".to_owned(),
                    "A Formiga out of composure sits out, dizzy, for the rest of the match. Nobody is hurt.".to_owned(),
                ),
                (
                    Glyph::Watch,
                    format!("Turn {}", ruleset.turn_cap),
                    "nearest the beacon wins".to_owned(),
                    "If time runs out: the most beacon lamps, then the most composure left, then the most still standing.".to_owned(),
                ),
            ],
            Guide::Orders => vec![
                (
                    key(scene::Key::Move),
                    "MOVE".to_owned(),
                    "walk to a tile".to_owned(),
                    Verb::Move.hint().to_owned(),
                ),
                (
                    key(scene::Key::Holding),
                    "HOLD".to_owned(),
                    "stand firm, light the beacon".to_owned(),
                    Verb::Hold.hint().to_owned(),
                ),
                (
                    key(scene::Key::Contest),
                    "CONTEST".to_owned(),
                    "bump theirs, or pull the lever".to_owned(),
                    Verb::Contest.hint().to_owned(),
                ),
                (
                    Glyph::Stars,
                    "USE".to_owned(),
                    "what it found on the ground".to_owned(),
                    Verb::Use.hint().to_owned(),
                ),
            ],
            Guide::Marks => scene::Key::ALL
                .iter()
                .map(|k| (key(*k), k.words().to_owned(), String::new(), String::new()))
                .collect(),
        };
        for (glyph, label, gloss, hover) in rows {
            let response = ui
                .horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = u * 3.0;
                    self.glyph(ui, ctx, glyph, u);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new(&label).size(13.0).strong().color(ink::PAGE));
                        if !gloss.is_empty() {
                            ui.label(RichText::new(&gloss).size(11.5).color(ink::MUTED));
                        }
                    });
                })
                .response;
            if !hover.is_empty() {
                response.on_hover_text(hover);
            }
            ui.add_space(u * 1.5);
        }
    }

    /// A row's picture in the field guide: one of the board's marks, stars, or a little watch.
    fn glyph(&self, ui: &mut egui::Ui, ctx: &egui::Context, glyph: Glyph, u: f32) {
        let ppp = ctx.pixels_per_point();
        let px = ppp.round().max(1.0) / ppp;
        let size = vec2(34.0, 24.0) * px;
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter();
        match glyph {
            Glyph::Key(i) => {
                painter.image(
                    self.keys[i].id(),
                    rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            Glyph::Stars => {
                for (k, dx) in [-1.0f32, 0.0, 1.0].into_iter().enumerate() {
                    let c = rect.center() + vec2(dx * u * 6.0, if k == 1 { -u * 2.0 } else { u });
                    star(painter, c, u * 3.2, ink::BRASS_LIGHT, ink::BRASS_EDGE);
                }
            }
            Glyph::Watch => {
                let c = rect.center();
                let r = u * 6.0;
                painter.circle_filled(c, r, ink::BRASS);
                painter.circle_stroke(c, r, Stroke::new(u * 0.7, ink::BRASS_EDGE));
                painter.circle_filled(c, r - u * 1.5, ink::CARD);
                painter.line_segment(
                    [c, c + vec2(0.0, -r + u * 2.5)],
                    Stroke::new(u * 0.8, ink::LINE),
                );
                painter.line_segment(
                    [c, c + vec2(r - u * 3.0, 0.0)],
                    Stroke::new(u * 0.8, ink::LINE),
                );
            }
        }
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
            self.fresh_match();
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
            self.fresh_match();
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
        let width = ui.available_width();
        let read = if self.account_open {
            "Close the whole match"
        } else {
            "Read the whole match"
        };
        if chrome::button(ui, read, Kind::Plain, false, true, Some(width), u).clicked() {
            self.toggle_account();
        }
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
/// An order in the Handler's words: where, by what is there, never by coordinates. `carrying`
/// is what the one given it carries, for a USE.
pub fn speak(m: &Match, order: &Order, carrying: Option<formiga_battle_core::Item>) -> String {
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
        Order::Contest {
            target: formiga_battle_core::Target::Lever { at },
        } if m.arena.switches.contains(at) => "CONTEST: turn the tap".to_owned(),
        Order::Contest { .. } => describe(m, order),
        Order::Use { at } => match carrying {
            Some(item) => format!("USE the {} {}", item.name(), place(*at).1),
            None => format!("USE what it carries {}", place(*at).1),
        },
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

/// A little speaker, a box and a cone, with `waves` coming off it, or a cross when there are none.
fn speaker(painter: &egui::Painter, c: egui::Pos2, waves: u8, colour: Color32, u: f32) {
    painter.rect_filled(
        Rect::from_center_size(c - vec2(u * 1.5, 0.0), vec2(u * 2.0, u * 3.0)),
        0.0,
        colour,
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            c + vec2(-u * 0.5, -u * 1.5),
            c + vec2(u * 2.0, -u * 3.5),
            c + vec2(u * 2.0, u * 3.5),
            c + vec2(-u * 0.5, u * 1.5),
        ],
        colour,
        Stroke::NONE,
    ));
    for k in 0..waves {
        let r = u * (4.0 + 2.0 * f32::from(k));
        let arc: Vec<egui::Pos2> = (0..=8)
            .map(|i| {
                let a = -0.8 + 1.6 * i as f32 / 8.0;
                c + vec2(u * 1.5 + a.cos() * r, a.sin() * r)
            })
            .collect();
        painter.add(egui::Shape::line(arc, Stroke::new(u * 0.8, colour)));
    }
    if waves == 0 {
        let x = c + vec2(u * 5.0, 0.0);
        for d in [vec2(u * 1.6, u * 1.6), vec2(u * 1.6, -u * 1.6)] {
            painter.line_segment([x - d, x + d], Stroke::new(u * 0.8, colour));
        }
    }
}

/// A little arrow pointing down at whoever is chosen, in its team's colour on a dark edge.
fn pointer_arrow(painter: &egui::Painter, tip: egui::Pos2, fill: Color32, u: f32) {
    let shape = |grow: f32| {
        vec![
            tip + vec2(-u * 5.0 - grow, -u * 6.0 - grow),
            tip + vec2(u * 5.0 + grow, -u * 6.0 - grow),
            tip + vec2(0.0, grow),
        ]
    };
    painter.add(egui::Shape::convex_polygon(
        shape(u * 1.2),
        ink::LINE,
        Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(shape(0.0), fill, Stroke::NONE));
}

/// A five-pointed star, for the dizzy.
fn star(painter: &egui::Painter, c: egui::Pos2, r: f32, fill: Color32, edge: Color32) {
    let points: Vec<egui::Pos2> = (0..10)
        .map(|k| {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
            let reach = if k % 2 == 0 { r } else { r * 0.45 };
            c + vec2(a.cos(), a.sin()) * reach
        })
        .collect();
    // A star is not convex: drawn as five points round a middle.
    for k in 0..5 {
        let tri = vec![
            c,
            points[(2 * k + 9) % 10],
            points[2 * k],
            points[2 * k + 1],
        ];
        painter.add(egui::Shape::convex_polygon(tri, fill, Stroke::NONE));
    }
    painter.add(egui::Shape::closed_line(
        points,
        Stroke::new(r * 0.18, edge),
    ));
}

/// A tick of whose good news it is down a slip's left side: the team's colour, or the ink.
fn tick(painter: &egui::Painter, rect: Rect, side: Option<Side>, u: f32) {
    painter.rect_filled(
        Rect::from_min_size(
            rect.min + vec2(u * 4.0, u * 3.0),
            vec2(u * 2.0, rect.height() - u * 6.0),
        ),
        0.0,
        side.map_or(ink::LINE.gamma_multiply(0.4), |s| {
            color(TEAM[s.index()].base)
        }),
    );
}

/// A little mouse, its left button lit for a click or its right one for a right click.
fn mouse(painter: &egui::Painter, rect: Rect, right: bool, u: f32) {
    painter.rect_filled(rect, u * 2.5, ink::LINE);
    painter.rect_filled(rect.shrink(u * 0.8), u * 2.0, ink::CARD);
    let split = rect.min.y + rect.height() * 0.42;
    let half = Rect::from_min_max(
        pos2(
            if right {
                rect.center().x
            } else {
                rect.min.x + u * 0.8
            },
            rect.min.y + u * 0.8,
        ),
        pos2(
            if right {
                rect.max.x - u * 0.8
            } else {
                rect.center().x
            },
            split,
        ),
    );
    painter.rect_filled(half, u, ink::STAMP);
    painter.hline(rect.x_range(), split, Stroke::new(u * 0.6, ink::LINE));
    painter.vline(
        rect.center().x,
        rect.min.y..=split,
        Stroke::new(u * 0.6, ink::LINE),
    );
}

/// The controls, on a card in the board page's lower left while orders are given: what a click
/// and a right click do, and the keys.
fn controls(painter: &egui::Painter, page: Rect, u: f32) {
    let rows: [(Option<bool>, &str); 3] = [
        (Some(false), "Move, contest, or choose"),
        (Some(true), "Hold there"),
        (None, "Enter lock in  ·  Esc let go  ·  L whole match"),
    ];
    let galleys: Vec<_> = rows
        .iter()
        .map(|(_, text)| {
            painter.layout_no_wrap((*text).to_owned(), FontId::proportional(11.5), ink::PAGE)
        })
        .collect();
    let icon = vec2(u * 6.0, u * 9.0);
    let row = galleys.iter().map(|g| g.size().y).fold(icon.y, f32::max) + u * 1.5;
    let title = painter.layout_job(label_job("Controls", 9.5, ink::FOREST));
    let width = galleys
        .iter()
        .map(|g| g.size().x)
        .fold(title.size().x, f32::max)
        + icon.x
        + u * 16.0;
    let height = u * 10.0 + title.size().y + row * rows.len() as f32;
    let corner = pos2(page.min.x + u * 8.0, page.max.y - u * 6.0 - height);
    let rect = Rect::from_min_size(
        pos2((corner.x / u).round() * u, (corner.y / u).round() * u),
        vec2(width, height),
    );
    let mut painter = painter.clone();
    painter.set_opacity(0.92);
    chrome::card(&painter, rect, u, ink::CARD, ink::LINE.gamma_multiply(0.45));
    painter.galley(
        rect.min + vec2(u * 5.0, u * 4.0),
        title.clone(),
        ink::FOREST,
    );
    let mut y = rect.min.y + u * 7.0 + title.size().y;
    for ((button, _), galley) in rows.iter().zip(galleys) {
        let x = rect.min.x + u * 5.0;
        if let Some(right) = button {
            mouse(
                &painter,
                Rect::from_min_size(pos2(x, y + (row - icon.y) / 2.0 - u * 0.5), icon),
                *right,
                u,
            );
        }
        let h = galley.size().y;
        painter.galley(
            pos2(x + icon.x + u * 4.0, y + (row - h) / 2.0 - u * 0.5),
            galley,
            ink::PAGE,
        );
        y += row;
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
