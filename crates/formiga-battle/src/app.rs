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
//!
//! Field Lessons, from the station too, are matches like any other begun from a board a lesson
//! sets up (`formiga_battle_core::lesson`): the lesson's goal and notes take the field guide's
//! place, the timer waits, and nothing is kept.
//!
//! The match on screen is drawn in `match_screen`, and the little painted pieces every page is
//! made of (cards, notes, tokens, the watch) are in `widgets`.

mod handler;
mod match_screen;
mod menus;
mod playbooks;
mod registry;
mod season;
#[cfg(test)]
mod tests;
mod welcome;
mod where_to;
mod widgets;

use crate::announce::{Callout, Style};
use crate::chrome::{self, Kind, hand_job, ink, label_job};
use crate::garden::Garden;
use crate::host::Host;
use crate::iso::View;
use crate::look::Look;
use crate::scene::{self, TEAM};
use crate::sound::{Cue, Sound, Track};
use crate::table::{Aim, Table, Verb};
use eframe::egui::{self, Color32, FontId, Id, Rect, RichText, Sense, Stroke, pos2, vec2};
use formiga_art::Canvas;
use formiga_battle_contract::Contender;
use formiga_battle_core::lesson::{Lesson, Verdict};
use formiga_battle_core::{Directive, Facing, Match, Tile};
use formiga_battle_core::{PLAN_SIZE, Side, Slot, TEAM_SIZE, Victory};
use formiga_pencil::ruled;
use std::collections::HashMap;
use std::path::PathBuf;
// The little painted pieces every page uses (cards, notes, tokens, the watch), by name here
// and in the pages' own files.
use widgets::*;

/// The notebook's parts, in its own pixels: the cover's rim, the strip along the top and the foot,
/// and the fold between the pages.
const RIM: f32 = 9.0;
const STRIP: f32 = 25.0;
const FOOT: f32 = 19.0;
const FOLD: f32 = 3.0;
/// How far apart the ruled page's lines are, in points: a line of writing to each.
const RULE_PITCH: f32 = 20.0;
/// How long a page takes to settle in when it changes, in seconds.
const SETTLE: f32 = 0.22;
/// The side tabs in a match, left to right.
const TABS: [&str; 2] = ["Orders", "Notebook"];

/// What comes after the Field Lessons, said once the last is learned and on the page of lessons.
pub(crate) const AFTER_LESSONS: [&str; 3] = [
    "Turn 12 is the last. If nobody has won by then, the most beacon lamps wins, then the most composure left, then the most still standing.",
    "Close to Home, every place has a cup: win three matches there against the visiting team to fill it, and the next place opens.",
    "Out in the World, play a friend with a room's code. Your Handler's pages keep your name, pennant and trios, and the notebook what each companion was seen to do.",
];

/// What a friend match sends, said in full before the first is opened.
const SHARED: [&str; 5] = [
    "Battle sends these to Formiga's relay at formigaworld.com, and on to your friend's Battle: the name you type here, with your club, your pennant and your Handler standing; the name and pennant of your trio, if your three are one; your colony's Handler key, a random number Formiga Desktop made for it so your friend's Battle can know you again next time; for each of your three companions, its name, the number Formiga Desktop knows it by, when it was born, its parent's number if it is a little one, its look, size and what it wears, how it moves, its temperament, the habits it has picked up and the leanings it has learned, and how the three get on; your orders each turn; a check number for the board after each turn; and which versions of Battle and Formiga Desktop you have, with a fingerprint of your copy of Battle (the same for everyone with that copy), so only matching, unaltered copies play each other.",
    "Nothing else from your colony is sent. There is no account, no email, no device or computer ID. The Handler key is made from nothing about you, your colony or your computer, and says nothing about any of them.",
    "The relay keeps your room only while it is in use, and never keeps a copy. It deletes it two minutes after the match ends, fifteen minutes after it opens if your friend never joins, five minutes after neither Battle has checked in, and two hours after it opens in any case. Your seat's key is kept only in scrambled form.",
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
    /// Field Lessons: short situations that teach how a match works. Offline, and nothing kept.
    Lessons,
}

impl Outing {
    pub fn name(self) -> &'static str {
        match self {
            Self::World => "Out in the World",
            Self::Home => "Close to Home",
            Self::Lessons => "Field Lessons",
        }
    }

    /// The pages of the notebook this outing turns through.
    pub fn steps(self) -> &'static [Step] {
        match self {
            Self::World => &[Step::Where, Step::Who, Step::Room],
            Self::Home => &[Step::Where, Step::Who],
            Self::Lessons => &[Step::Lessons, Step::Who],
        }
    }
}

/// A page of getting ready, in the order they are turned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    Where,
    /// Which Field Lesson.
    Lessons,
    Who,
    Room,
    /// The Handler's own pages, opened from the station rather than on the way anywhere: who
    /// they are and their trios, the history, and the settings.
    Handler,
    /// Each trio's playbooks: its plans for a match, in stages.
    Playbooks,
    /// The Handler's place at the Battleground: the event under way, titles won, and how they
    /// stand against each rival.
    Season,
    History,
    Settings,
}

impl Step {
    pub fn tab(self) -> &'static str {
        match self {
            Self::Where => "Where",
            Self::Lessons => "Lessons",
            Self::Who => "Who",
            Self::Room => "Friend",
            Self::Handler => "Handler",
            Self::Playbooks => "Playbooks",
            Self::Season => "Season",
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
    /// The Handler's registry: their card and pennant, standing and trios.
    Handler,
    /// The Season page, with an event entered and its first semi-final played.
    Season,
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
    /// Choosing a Field Lesson.
    Lessons,
    /// A Field Lesson on the board, its orders not yet given: the lesson on flanks.
    Lesson,
    /// The Battleground with the welcome card over it, as Battle opened by itself shows it.
    Welcome,
    /// A trio's playbooks, one open at its second stage.
    Playbooks,
    /// A match against the visitors with the trio's playbook filling in the orders.
    Planned,
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
    /// The board's pixels as last sent to its texture, filled again in place each frame once
    /// the texture has taken the last ones.
    board_pixels: std::sync::Arc<egui::ColorImage>,
    garden: Option<Garden>,
    cover: Cover,
    portraits: HashMap<(u64, String), egui::TextureHandle>,
    hover: Option<formiga_battle_core::Tile>,
    /// The square the arrow keys have moved to, for giving orders without a mouse.
    cursor: Option<formiga_battle_core::Tile>,
    error: Option<String>,
    /// The side page shows the Handler's Notebook instead of the orders.
    notebook_open: bool,
    /// The Handler's name as typed, on the Friend page or their own.
    handler: String,
    /// Their club as typed.
    club: String,
    /// The pennant being chosen before the Handler has signed a name to keep it under.
    draft_style: crate::identity::Style,
    /// Each pennant's picture, painted once.
    flags: HashMap<crate::identity::Style, egui::TextureHandle>,
    /// The Handler's painted pictures (tiles, plates, rosettes, team pictures, banners), each
    /// painted once under its key and kept while it is shown.
    art: Kept,
    /// The ruled page's lines, in points: where the first is on the screen, and how far apart
    /// they are, for writing that sits on them.
    rules: (f32, f32),
    /// A name for the three picked, as typed.
    trio_draft: String,
    /// A trio being renamed.
    renaming: Option<registry::Renaming>,
    /// The trio the Playbooks page is open at, by id.
    shelf_trio: Option<String>,
    /// Which of its playbooks is open, and which stage of it.
    shelf_book: usize,
    shelf_stage: usize,
    /// "Remove this playbook" was pressed once: the next press removes it.
    shelf_removing: bool,
    join_code: String,
    /// The Handler has read what a friend match shares, this time.
    understood: bool,
    /// The welcome card is up: Battle was opened by itself, not by Formiga.
    welcome: bool,
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
    /// The picture at the head of the station's page, and its picture this frame.
    masthead: Option<crate::masthead::Masthead>,
    masthead_picture: Option<(i64, egui::TextureHandle)>,
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
    /// The Field Lesson chosen, and the ones learned this sitting.
    lesson: Lesson,
    learned: Vec<Lesson>,
    /// The friends played, opened out on the Circuit page.
    friends_open: bool,
    /// How far the mouse wheel has turned over the board since it last turned a HOLD, in points.
    wheel: f32,
    /// Keeps how Battle looks and sounds for next time; none for a window opened for its picture.
    kept: Option<crate::settings::Keeper>,
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

/// How far the mouse wheel turns, in points, to turn a HOLD a quarter.
const WHEEL_NOTCH: f32 = 24.0;

impl BattleApp {
    pub fn new(
        host: Host,
        table: Table,
        review: Option<Review>,
        snap: Option<Snap>,
        sound: Sound,
    ) -> Self {
        let view = View::new(table.arena.width, table.arena.height);
        let card = table.record.handler.clone();
        let mut app = Self {
            host,
            table,
            stage: Stage::Station,
            view,
            board: None,
            board_pixels: std::sync::Arc::default(),
            garden: None,
            cover: Cover::default(),
            portraits: HashMap::new(),
            hover: None,
            cursor: None,
            error: None,
            notebook_open: false,
            handler: card.as_ref().map(|c| c.name.clone()).unwrap_or_default(),
            club: card
                .as_ref()
                .and_then(|c| c.club.clone())
                .unwrap_or_default(),
            draft_style: crate::identity::Style::default(),
            flags: HashMap::new(),
            art: Kept::default(),
            rules: (0.0, 21.0),
            trio_draft: String::new(),
            renaming: None,
            shelf_trio: None,
            shelf_book: 0,
            shelf_stage: 0,
            shelf_removing: false,
            join_code: String::new(),
            understood: false,
            welcome: false,
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
            masthead: None,
            masthead_picture: None,
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
            lesson: Lesson::Beacon,
            learned: Vec::new(),
            friends_open: false,
            wheel: 0.0,
            kept: None,
        };
        match review {
            Some(Review::Playing) => app.begin(),
            Some(Review::Account) => {
                app.begin();
                app.account_open = true;
            }
            Some(Review::Orders) => {
                app.begin();
                // A few turns in, so the card says what came of last turn's orders.
                app.table.play_ahead(3);
                // One order as the coach would give it, a HOLD of two of the yard's squares
                // (one named twice) facing across, and PRESS in hand for the third.
                app.table.coach_orders();
                let zone = app.table.arena.zone.clone();
                app.table.orders[1] = Some(Directive::Hold {
                    squares: zone.iter().take(2).copied().collect(),
                    facing: Facing::across(app.table.you),
                });
                app.table.turning = Some(1);
                app.table.orders[2] = None;
                app.table.take_token(Verb::Press);
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
            Some(Review::Season) => {
                app.table.begin_next_event();
                app.open_journal(Outing::Home, Step::Season);
            }
            Some(Review::Handler) => {
                app.table.record = formiga_battle_contract::sample::record();
                app.handler = "Jo".to_owned();
                app.club = "Hedge End Club".to_owned();
                app.open_journal(Outing::Home, Step::Handler);
            }
            Some(Review::Picking) => {
                // A colony with a trio, its three picked.
                app.table.record = formiga_battle_contract::sample::record();
                app.open_journal(Outing::Home, Step::Who);
            }
            Some(Review::Playbooks) => {
                app.table.record = formiga_battle_contract::sample::record();
                app.sample_playbooks();
                app.shelf_stage = 1;
                app.open_journal(Outing::Home, Step::Playbooks);
            }
            Some(Review::Planned) => {
                app.table.record = formiga_battle_contract::sample::record();
                app.sample_playbooks();
                app.begin();
            }
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
            Some(Review::Lessons) => {
                app.lesson = Lesson::Flank;
                app.open_journal(Outing::Lessons, Step::Lessons);
            }
            Some(Review::Lesson) => {
                app.lesson = Lesson::Flank;
                app.outing = Outing::Lessons;
                app.begin_lesson();
            }
            Some(Review::Welcome) => app.welcome = true,
            Some(Review::Station) | None => {}
        }
        app
    }

    /// With the welcome card up, for Battle opened by itself: the sample colony is there to look
    /// around with, but a player's own companions come from Formiga.
    pub fn welcoming(mut self) -> Self {
        self.welcome = true;
        self
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
        self.field(None);
    }

    /// Puts the chosen Field Lesson on the board, on the Backyard.
    fn begin_lesson(&mut self) {
        if self.table.arena.id != formiga_battle_core::Arena::backyard().id {
            self.choose_arena(formiga_battle_core::Arena::backyard());
        }
        self.field(Some(self.lesson));
    }

    fn field(&mut self, lesson: Option<Lesson>) {
        // The first turn's clock starts once the ready-up is over.
        let now = self.clock + READY_SECS;
        let started = match lesson {
            Some(lesson) => self.table.start_lesson(lesson, now),
            None => self.table.start(now),
        };
        match started {
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
        let can = self.table.shown.is_none()
            && self.table.outcome().is_none()
            && !sent
            && !self.table.room_gone();
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
            let cue = match self.table.lesson.as_ref().map(|run| &run.verdict) {
                Some(Verdict::Learned(_)) => Cue::Fanfare,
                Some(_) => Cue::Shrug,
                None => match self.table.outcome().and_then(|o| o.winner) {
                    Some(side) if side == self.table.you => Cue::Fanfare,
                    Some(_) => Cue::Womp,
                    None => Cue::Shrug,
                },
            };
            self.sound.play(cue);
            if let Some(run) = &self.table.lesson
                && matches!(run.verdict, Verdict::Learned(_))
                && !self.learned.contains(&run.lesson)
            {
                self.learned.push(run.lesson);
            }
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
        // The tabs start over the right page, or further left if they would otherwise run into
        // the sound control at its top right.
        let widths: f32 = words
            .iter()
            .map(|w| snap(measure(w, chrome::LABEL) + unit * 12.0) + unit * 3.0)
            .sum();
        let sound = measure("Muted", chrome::LABEL) + unit * 27.0;
        let mut x = (right.min.x + unit * 4.0)
            .min(snap(right.max.x - sound - widths))
            .max(left.min.x);
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
            // As far apart on any screen as a line of writing needs, however many screen
            // pixels the notebook's pixels are.
            rules: (24, (RULE_PITCH / u).round() as i32),
        };
        self.rules = (
            layout.full.min.y + (spread.right.1 + spread.rules.0) as f32 * u,
            spread.rules.1 as f32 * u,
        );
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

    /// Keeps the sound, motion and window size as they change, for the next sitting.
    pub fn keep(&mut self, keeper: crate::settings::Keeper) {
        self.kept = Some(keeper);
    }

    pub(crate) fn frame(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if !self.styled {
            chrome::style(&ctx);
            self.styled = true;
            // The pencil hand's fonts are egui's from its next pass: run that pass at once, so the
            // first frame shown is already written in it.
            ctx.request_discard("the pencil hand is installed");
            return;
        }
        self.clock = ctx.input(|input| input.time) as f32;
        if let Some(keeper) = &mut self.kept {
            let (closing, inner) = ctx.input(|input| {
                let viewport = input.viewport();
                (viewport.close_requested(), viewport.inner_rect)
            });
            let window = inner.map(|rect| [rect.width(), rect.height()]);
            keeper.note(self.sound.levels(), window, self.clock, closing);
        }
        // Pictures not shown for a while are let go, outside the context's lock.
        let shown = ctx.cumulative_frame_nr();
        drop(self.art.sweep(shown));
        drop(ctx.data_mut(|d| d.get_temp_mut_or_default::<Kept>(kept_id()).sweep(shown)));
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
            // The room plays the opener's place: the joiner's board, name and cup follow it.
            if let Some(arena) = self.table.m.as_ref().map(|m| m.arena.clone()) {
                self.choose_arena(arena);
            }
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
                // What is written on the ruled right page rests on its rules; anything else is
                // written as it comes.
                let ruling = formiga_pencil::ruled::RuleGrid {
                    top: self.rules.0,
                    pitch: self.rules.1,
                };
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
                            ruling.keep(ui.ctx());
                            self.journal_right(ui, &ctx, u);
                            formiga_pencil::ruled::RuleGrid::forget(ui.ctx());
                        });
                    }
                    Stage::Station => {
                        ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
                            ui.multiply_opacity(fade);
                            self.station_left(ui, &ctx, u, now);
                        });
                        ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                            ui.multiply_opacity(fade);
                            ruling.keep(ui.ctx());
                            self.station_right(ui, u, now);
                            formiga_pencil::ruled::RuleGrid::forget(ui.ctx());
                        });
                    }
                    Stage::Playing => {
                        self.board_view(ui, &ctx, layout.left.shrink(u * 4.0), now, u);
                        self.board_page(ui, layout.left, now, u);
                        ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                            ui.multiply_opacity(fade);
                            ruling.keep(ui.ctx());
                            self.side_panel(ui, &ctx, now, u);
                            formiga_pencil::ruled::RuleGrid::forget(ui.ctx());
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
        if self.welcome {
            self.welcome_card(&ctx, layout.full, layout.unit);
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
                self.table.put_down();
                self.sound.play(Cue::Cancel);
            }
        }
        if self.table.shown.is_some() {
            return;
        }
        for (key, verb) in [
            (egui::Key::P, Verb::Press),
            (egui::Key::H, Verb::Hold),
            (egui::Key::T, Verb::Trick),
            (egui::Key::I, Verb::Interact),
            (egui::Key::U, Verb::UseItem),
        ] {
            if pressed(key) && self.table.take_token(verb) {
                self.sound.play(Cue::Click);
            }
        }
        // R turns the HOLD in hand, or the one last given, a quarter clockwise; with Shift, back.
        if pressed(egui::Key::R) {
            let back = ctx.input(|i| i.modifiers.shift);
            if self.table.turn_hold(back) {
                self.sound.play(Cue::Select);
            }
        }
        if pressed(egui::Key::Enter) {
            self.lock_in(self.clock);
        }
        // The arrow keys move a square cursor over the board, from the yard's middle; Space gives
        // the order in hand there, as a click would, and Shift+Space holds it, as a right click.
        if let Some(m) = &self.table.m {
            let (w, h) = (m.arena.width as i8, m.arena.height as i8);
            let centre = m.arena.zone_centre();
            for (key, dx, dy) in [
                (egui::Key::ArrowLeft, -1, 0),
                (egui::Key::ArrowRight, 1, 0),
                (egui::Key::ArrowUp, 0, -1),
                (egui::Key::ArrowDown, 0, 1),
            ] {
                if pressed(key) {
                    let from = self.cursor.unwrap_or(centre);
                    let to = if self.cursor.is_none() {
                        centre
                    } else {
                        formiga_battle_core::Tile::new(
                            (from.x + dx).clamp(0, w - 1),
                            (from.y + dy).clamp(0, h - 1),
                        )
                    };
                    self.cursor = Some(to);
                }
            }
        }
        if pressed(egui::Key::Space)
            && let Some(tile) = self.cursor
            && self.table.outcome().is_none()
        {
            let hold = ctx.input(|i| i.modifiers.shift);
            let now = self.clock;
            self.press_tile(tile, hold, now);
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

    /// A square chosen while giving orders, by a click or by Space at the keys' cursor: `hold` for
    /// a right click or Shift+Space.
    fn press_tile(&mut self, tile: formiga_battle_core::Tile, hold: bool, now: f32) {
        if self.table.shown.is_some() || self.table.outcome().is_some() {
            return;
        }
        // An order picked by its button or key is let go by a right click, as before.
        if hold && self.table.verb.is_some() {
            self.table.put_down();
            self.sound.play(Cue::Cancel);
            return;
        }
        match self.table.press(tile, hold) {
            Aim::Give(_) => {
                self.pokes.push((tile, now));
                self.sound.play(Cue::Order);
            }
            Aim::Carrier(_) => self.sound.play(Cue::Select),
            Aim::Nothing => {}
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

/// When a line over the board born at `born` is shown: the first moment from then on that keeps
/// `HEADLINE_SECS` clear of every line already placed. The placed lines are passed over once,
/// earliest first, so it always settles: stepping past a line by adding `HEADLINE_SECS` can land
/// a hair under `HEADLINE_SECS` from it in floating point, and asking again whether that line is
/// too close would step to the same moment for ever. Two lines born in the same frame did just
/// that, and the window stopped answering.
/// The garden painted for `arena`, kept until another place is shown: when the one kept is of
/// another place, it is painted afresh with a view to match.
pub(super) fn garden_for<'a>(
    garden: &'a mut Option<Garden>,
    view: &mut View,
    arena: &formiga_battle_core::Arena,
) -> &'a mut Garden {
    if garden.as_ref().is_none_or(|g| g.arena != arena.id) {
        *view = View::new(arena.width, arena.height);
        *garden = Some(Garden::new(arena, view));
    }
    garden.as_mut().expect("just painted")
}

fn headline_moment(born: f32, placed: &[f32]) -> f32 {
    let mut placed = placed.to_vec();
    placed.sort_by(f32::total_cmp);
    let mut shown = born;
    for other in placed {
        if (shown - other).abs() < HEADLINE_SECS {
            shown = other + HEADLINE_SECS;
        }
    }
    shown
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

/// A team order in a few words, as the order cards and the pointer's card say it.
pub fn speak_order(m: &Match, you: Side, order: &formiga_battle_core::Directive) -> String {
    use formiga_battle_core::Directive;
    let squares = |ts: &[Tile]| -> String {
        let yard = ts.iter().filter(|t| m.arena.in_zone(**t)).count();
        let ground = crate::garden::Dressing::of(&m.arena).ground().1;
        match (ts.len(), yard) {
            (1, 1) => "a square of the yard".to_owned(),
            (1, _) => format!("a square {ground}"),
            (n, y) if y == n => format!("{n} squares of the yard"),
            (n, _) => format!("{n} squares"),
        }
    };
    let theirs = |ts: &[Tile]| match ts {
        [t] => m.occupant(*t).filter(|o| o.side != you),
        _ => None,
    };
    match order {
        Directive::Press { squares: ts } => match theirs(ts) {
            Some(o) => format!("PRESS {} off its square", m.profile(o).name),
            None => format!("PRESS {}", squares(ts)),
        },
        Directive::Hold {
            squares: ts,
            facing,
        } => format!("HOLD {}, facing {}", squares(ts), facing.name()),
        Directive::Trick { squares: ts } => match theirs(ts) {
            Some(o) => format!("TRICK {} from its side or back", m.profile(o).name),
            None => format!("TRICK whoever holds {}", squares(ts)),
        },
        Directive::Interact { at } => {
            let thing = m
                .board
                .ground
                .iter()
                .chain(m.board.glint.iter())
                .find(|(g, _)| g == at)
                .map(|(_, i)| i.name());
            match thing {
                Some(item) => format!("INTERACT: fetch the {item}"),
                None if m.arena.levers.contains(at) => "INTERACT: pull the lever".to_owned(),
                None => "INTERACT: set off the switch".to_owned(),
            }
        }
        Directive::UseItem { by, at } => {
            let slot = Slot::new(you, *by);
            let user = m.fighter(slot).at;
            let place = if *at == user {
                "where it stands".to_owned()
            } else {
                match m.occupant(*at) {
                    Some(o) if o.side == you => format!("over {}", m.profile(o).name),
                    Some(o) => format!("on {}", m.profile(o).name),
                    None => "on a square in reach".to_owned(),
                }
            };
            format!("USE ITEM: {} {place}", carried(m, slot))
        }
    }
}

/// One of the team and what it carries, in a few words: "Pip's twig".
fn carried(m: &Match, slot: Slot) -> String {
    let name = &m.profile(slot).name;
    match m.fighter(slot).carrying {
        Some(item) => format!("{name}'s {}", item.name()),
        None => name.clone(),
    }
}

/// A painted picture kept in egui's memory under its key, painted the first time it is asked
/// for: for the little pictures drawn by functions that keep nothing of their own.
fn kept_picture(
    ctx: &egui::Context,
    key: &str,
    paint: impl FnOnce() -> Canvas,
) -> egui::TextureHandle {
    let shown = ctx.cumulative_frame_nr();
    let id = kept_id();
    if let Some(t) = ctx.data_mut(|d| d.get_temp_mut_or_default::<Kept>(id).get(key, shown)) {
        return t;
    }
    // Painted, and made a texture, outside the context's lock.
    let t = texture(ctx, key, &paint());
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Kept>(id)
            .keep(key.to_owned(), t.clone(), shown);
    });
    t
}

fn kept_id() -> Id {
    Id::new("kept-pictures")
}

/// Painted pictures kept as textures under their keys, each let go once it has gone unshown for
/// a while. Pictures sized to the window are painted again for every width it is dragged
/// through, and the old widths are never shown again.
#[derive(Clone, Default)]
pub(crate) struct Kept {
    /// Each picture, and the last frame it was shown in.
    pictures: HashMap<String, (egui::TextureHandle, u64)>,
}

impl Kept {
    /// Frames a picture can go unshown before it is let go: a few seconds of a running window.
    const UNSHOWN_FRAMES: u64 = 600;

    /// The picture under `key`, shown in frame `shown`, if it is kept.
    pub(crate) fn get(&mut self, key: &str, shown: u64) -> Option<egui::TextureHandle> {
        self.pictures.get_mut(key).map(|(t, last)| {
            *last = shown;
            t.clone()
        })
    }

    pub(crate) fn keep(&mut self, key: String, texture: egui::TextureHandle, shown: u64) {
        self.pictures.insert(key, (texture, shown));
    }

    /// Lets go of the pictures not shown for a while, handing them back to be dropped where no
    /// lock is held.
    #[must_use]
    pub(crate) fn sweep(&mut self, now: u64) -> Vec<egui::TextureHandle> {
        let stale: Vec<String> = self
            .pictures
            .iter()
            .filter(|(_, (_, last))| now.saturating_sub(*last) > Self::UNSHOWN_FRAMES)
            .map(|(key, _)| key.clone())
            .collect();
        stale
            .iter()
            .filter_map(|key| self.pictures.remove(key))
            .map(|(t, _)| t)
            .collect()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.pictures.len()
    }
}

/// What one of the team will probably do with the plan as it stands, and why, in a few words
/// for its name tag: "probably flanks, keen to go".
fn likely_words(
    table: &Table,
    task: &formiga_battle_core::Task,
    why: &formiga_battle_core::Why,
) -> String {
    use formiga_battle_core::{Role, Why};
    let name = |s: &Slot| {
        table
            .m
            .as_ref()
            .map_or(String::new(), |m| m.profile(*s).name.clone())
    };
    let job = match task.role {
        Role::Claim => "holds",
        Role::Oust => "moves one off",
        Role::Flank => "flanks",
        Role::Trick => "tricks",
        Role::Pin => "waits beside",
        Role::Guard => "guards",
        Role::Intercept => "heads one off",
        Role::Fetch => "fetches",
        Role::Pull => "pulls it",
        Role::Wait => "waits by it",
        Role::Spare => "sees for itself",
    };
    let leaning = match why {
        Why::Bold => Some("keen to go".to_owned()),
        Why::Careful => Some("would rather hold".to_owned()),
        Why::Friendly { ally } => Some(format!("to be by {}", name(ally))),
        Why::Avoiding { ally } => Some(format!("away from {}", name(ally))),
        Why::Curious => Some("curious".to_owned()),
        Why::Whim => Some("on a whim".to_owned()),
        Why::Alone => Some("alone".to_owned()),
        Why::Nearest | Why::KeptAtIt => None,
    };
    match leaning {
        Some(l) => format!("probably {job}, {l}"),
        None => format!("probably {job}"),
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

/// A canvas as an image for a texture, written over `image` in place: the same pixels as
/// `ColorImage::from_rgba_unmultiplied`, without two fresh copies of the board each frame.
fn into_image(canvas: &Canvas, image: &mut egui::ColorImage) {
    let size = [canvas.width() as usize, canvas.height() as usize];
    image.size = size;
    image.source_size = vec2(size[0] as f32, size[1] as f32);
    image.pixels.clear();
    image.pixels.extend(
        canvas
            .pixels()
            .iter()
            .map(|p| Color32::from_rgba_unmultiplied(p.r, p.g, p.b, p.a)),
    );
}

fn texture(ctx: &egui::Context, name: &str, canvas: &Canvas) -> egui::TextureHandle {
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [canvas.width() as usize, canvas.height() as usize],
        &canvas.rgba_bytes(),
    );
    ctx.load_texture(name, image, egui::TextureOptions::NEAREST)
}
