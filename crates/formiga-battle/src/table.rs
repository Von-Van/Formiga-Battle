//! One sitting at the board, without the window: the two rosters, the match, how everyone looks
//! and faces, the orders the Handler has given so far, and the turn being shown. The window only
//! asks this what to show and tells it what was clicked, so tests drive the same code.

use crate::announce::Callout;
use crate::friend::{Ask, Line, Told};
use crate::look::Look;
use crate::playback::{Frame, Playback};
use crate::rewards;
use crate::scene::{Choice, Marks};
use formiga_battle_contract::{BattleRecord, BattleResult};
use formiga_battle_contract::{BattleSnapshot, Contender};
use formiga_battle_core::Event;
use formiga_battle_core::ai::{self, Coach};
use formiga_battle_core::interpret::distances;
use formiga_battle_core::notebook::Page;
use formiga_battle_core::recap::recap_naming;
use formiga_battle_core::rng::{self, Purpose};
use formiga_battle_core::{
    Arena, Match, Order, Orders, Outcome, Profile, Ruleset, Side, Slot, TEAM_SIZE, Target, Tile,
    TurnReport,
};
use formiga_battle_room::roster::{check_team, room_match};
use formiga_battle_room::{Build, Entrant, View, relay_base};
use std::path::PathBuf;

/// The session Desktop opened, and the answer being written into it.
pub struct Session {
    pub directory: PathBuf,
    pub result: BattleResult,
}

/// A match with a friend, through the relay.
pub struct Friend {
    pub line: Line,
    /// The room's code, once the relay has opened or joined it.
    pub code: Option<String>,
    /// The match has begun and is on the board.
    pub seated: bool,
    /// Both Handlers' names, left then right.
    pub handlers: [String; 2],
    /// The newest view of the room.
    pub view: Option<View>,
    /// The turn this Handler's orders have gone to the relay for.
    pub sent_turn: Option<u8>,
    pub opponent_ready: bool,
    pub opponent_away: bool,
    /// When this turn locks, on the window's clock.
    pub deadline: f32,
    /// What the match earned, kept back until the relay confirms both windows came to the same
    /// board after its last turn.
    pub held: Option<rewards::Earned>,
}

impl Friend {
    pub fn sent(&self, turn: u8) -> bool {
        self.sent_turn == Some(turn)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Move,
    Hold,
    Contest,
    /// Use what it carries; only for one carrying something.
    Use,
}

impl Verb {
    pub const ALL: [Self; 4] = [Self::Move, Self::Hold, Self::Contest, Self::Use];

    pub const fn word(self) -> &'static str {
        match self {
            Self::Move => "MOVE",
            Self::Hold => "HOLD",
            Self::Contest => "CONTEST",
            Self::Use => "USE",
        }
    }

    pub const fn hint(self) -> &'static str {
        match self {
            Self::Move => "Reach a tile. Walks round anyone holding; open to a contest.",
            Self::Hold => "Defend a tile. Turns a contest away; holds the beacon.",
            Self::Contest => "Disrupt someone not holding, or pull the lever or turn the tap.",
            Self::Use => {
                "Use what it carries: a puff blows them along, a twig blocks a tile, a leaf keeps a friend dry."
            }
        }
    }
}

/// What a click on the board does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aim {
    /// Chooses one of the Handler's to give an order to.
    Choose(u8),
    /// Gives the chosen one this order.
    Give(Order),
    Nothing,
}

pub struct Shown {
    pub playback: Playback,
    /// What is announced over the board as it plays, in order.
    pub callouts: Vec<Callout>,
    pub report: TurnReport,
    /// When it began showing, on the window's clock.
    pub started: f32,
}

pub struct Table {
    /// The Handler's companions, each with the profile Desktop's record reads as.
    pub home: Vec<(Contender, Profile)>,
    /// The visiting colony's.
    pub away: Vec<(Contender, Profile)>,
    /// How the Handler's companions get on, as Desktop's roster has it.
    home_bonds: Vec<formiga_travel::TravelRelationship>,
    pub arena: Arena,
    pub ruleset: Ruleset,
    pub seed: u64,
    pub reduce_motion: bool,
    pub picked: Vec<usize>,
    pub m: Option<Match>,
    /// The two teams on the board, by roster place within `home` and `away`.
    pub teams: [Vec<usize>; 2],
    pub looks: [Vec<Look>; 2],
    pub facing: [[bool; TEAM_SIZE]; 2],
    /// The Handler's orders so far this turn.
    pub orders: Orders,
    pub selected: Option<u8>,
    pub verb: Option<Verb>,
    /// When the current turn's orders began, on the window's clock.
    pub turn_began: f32,
    pub shown: Option<Shown>,
    /// Every turn told, newest last.
    pub log: Vec<(u8, Vec<String>)>,
    /// What Desktop keeps of Battle for this colony, with this sitting's matches applied.
    pub record: BattleRecord,
    /// The session Desktop opened, if it did: what this sitting earned is written into it after
    /// every match, for Desktop to apply when Battle closes. A rehearsal keeps nothing.
    pub session: Option<Session>,
    /// Every event of the match so far, for the notebook at its end.
    match_events: Vec<Event>,
    /// What the last match earned, in words: shown with the result.
    pub earned: Vec<String>,
    /// The side the Handler plays: left against the AI, either against a friend.
    pub you: Side,
    /// Who is on the board, as Desktop's companions, by side and place.
    pub board_teams: [Vec<Contender>; 2],
    /// The other side's name in recaps.
    pub other_name: String,
    pub friend: Option<Friend>,
    /// Where friend matches go: the site's relay, unless development points elsewhere.
    pub relay: String,
    /// The version of the Desktop whose roster this is, as its snapshot says.
    desktop_version: String,
    /// Something the room said for the Handler to read.
    pub notice: Option<String>,
    /// Why the match ended without a turn to show it, such as a forfeit.
    pub ended_because: Option<String>,
}

impl Table {
    /// A table for the Handler's roster against a visiting colony. A colony of fewer than three is
    /// joined by friends from next door so it can field a team.
    pub fn new(roster: BattleSnapshot, visitors: BattleSnapshot, seed: u64) -> Self {
        let record = roster.record.clone().unwrap_or_default();
        let reduce_motion = roster.presentation.reduce_motion;
        let pair = |s: &BattleSnapshot| -> Vec<(Contender, Profile)> {
            s.contenders
                .iter()
                .cloned()
                .zip(formiga_battle_room::roster::profiles(s))
                .collect()
        };
        let mut home = pair(&roster);
        let mut friends = pair(&visitors);
        while home.len() < TEAM_SIZE && !friends.is_empty() {
            home.push(friends.remove(friends.len() - 1));
        }
        let picked = (0..home.len().min(TEAM_SIZE)).collect();
        Self {
            home,
            away: friends,
            home_bonds: roster.relationships.clone(),
            arena: Arena::backyard(),
            ruleset: Ruleset::default(),
            seed,
            reduce_motion,
            picked,
            m: None,
            teams: [Vec::new(), Vec::new()],
            looks: [Vec::new(), Vec::new()],
            facing: [[true; TEAM_SIZE], [false; TEAM_SIZE]],
            orders: [None; TEAM_SIZE],
            selected: None,
            verb: None,
            turn_began: 0.0,
            shown: None,
            log: Vec::new(),
            record,
            session: None,
            match_events: Vec::new(),
            earned: Vec::new(),
            you: Side::Left,
            board_teams: [Vec::new(), Vec::new()],
            other_name: "The visiting team".to_owned(),
            friend: None,
            relay: relay_base(),
            desktop_version: roster.desktop_version.clone(),
            notice: None,
            ended_because: None,
        }
    }

    /// A companion's notebook page, if it has played a friend: read from what Desktop keeps.
    pub fn page(&self, creature_id: u64) -> Option<Page> {
        let page = self
            .record
            .notebook
            .iter()
            .find(|p| p.creature_id == creature_id && p.matches > 0)?;
        let teammates: Vec<(u64, String)> = self
            .home
            .iter()
            .map(|(c, _)| (c.traveler.id.0, c.traveler.name.clone()))
            .collect();
        Some(rewards::page_for_showing(page, &teammates))
    }

    /// What the match earned, applied to the record here and written for Desktop.
    fn finish_match(&mut self) {
        let Some(m) = &self.m else {
            return;
        };
        // A match whose two boards came out differently was not played by the same rules on both
        // sides, so nobody can say who won it: it earns nothing.
        if self
            .friend
            .as_ref()
            .and_then(|f| f.view.as_ref())
            .is_some_and(|v| v.out_of_step)
        {
            self.earned = vec![
                "Your board and your friend's came out differently, so this match earned nothing."
                    .to_owned(),
            ];
            return;
        }
        let now = time::OffsetDateTime::now_utc();
        let earned = match &self.friend {
            Some(friend) => rewards::friend_match(
                m,
                self.you,
                &self.match_events,
                &self.record,
                &friend.handlers[self.you.other().index()],
                now,
            ),
            None => rewards::ai_match(m, self.you, &self.record),
        };
        // A match with a friend that ended on the board counts once the relay has both windows'
        // boards after the last turn and they are the same. One that ended by someone leaving or
        // being away has no last board to wait for.
        let forfeit =
            m.outcome().and_then(|o| o.victory) == Some(formiga_battle_core::Victory::Forfeit);
        let confirmed = self
            .friend
            .as_ref()
            .map(|f| f.view.as_ref().is_some_and(|v| v.confirmed));
        match (confirmed, &mut self.friend) {
            (Some(false), Some(friend)) if !forfeit => {
                self.earned = vec!["Checking your board against your friend's…".to_owned()];
                friend.held = Some(earned);
            }
            _ => self.keep(earned),
        }
    }

    /// What a friend match earned and was kept back, now that the relay has said how the two
    /// boards came out: kept if they were the same, dropped if not, or if the room closed first.
    fn settle_held(&mut self, confirmed: bool, why_not: &str) {
        let Some(earned) = self.friend.as_mut().and_then(|f| f.held.take()) else {
            return;
        };
        if confirmed {
            self.keep(earned);
        } else {
            self.earned = vec![why_not.to_owned()];
        }
    }

    /// Applies what a match earned to the record here and writes it for Desktop.
    fn keep(&mut self, earned: rewards::Earned) {
        self.record.apply(&earned.result);
        self.earned = earned.lines;
        if let Some(session) = &mut self.session {
            session.result.matches.push(earned.result);
            if let Err(error) = session.result.write(&session.directory) {
                self.notice = Some(format!(
                    "What this match earned could not be kept for Desktop: {error}"
                ));
            }
        }
    }

    pub fn toggle_pick(&mut self, index: usize) {
        if let Some(at) = self.picked.iter().position(|p| *p == index) {
            self.picked.remove(at);
        } else if self.picked.len() < TEAM_SIZE {
            self.picked.push(index);
        }
    }

    /// Three of the visitors, chosen by the seed.
    fn opponents(&self) -> Vec<usize> {
        let mut pool: Vec<usize> = (0..self.away.len()).collect();
        let mut chosen = Vec::new();
        let mut k = 0;
        while chosen.len() < TEAM_SIZE && !pool.is_empty() {
            let i = (rng::roll(self.seed, Purpose::Roster, &[k, 7]) as usize) % pool.len();
            chosen.push(pool.remove(i));
            k += 1;
        }
        chosen
    }

    /// Puts both teams on the board. Fails only if a look cannot be drawn.
    pub fn start(&mut self, now: f32) -> anyhow::Result<()> {
        anyhow::ensure!(self.picked.len() == TEAM_SIZE, "pick three");
        // The visitors play only on arenas whose cup is open; anywhere else, the Backyard.
        if !rewards::open_to_play(&self.record).contains(&self.arena.id.as_str()) {
            self.arena = Arena::backyard();
        }
        let away = self.opponents();
        anyhow::ensure!(
            away.len() == TEAM_SIZE,
            "the visitors need three to field a team"
        );
        let profiles = |pool: &[(Contender, Profile)], picks: &[usize]| -> Vec<Profile> {
            picks.iter().map(|i| pool[*i].1.clone()).collect()
        };
        let mut rosters = [
            profiles(&self.home, &self.picked),
            profiles(&self.away, &away),
        ];
        // A visitor who shares a name with one of the Handler's is told apart in every recap.
        let ours: Vec<String> = rosters[0].iter().map(|p| p.name.clone()).collect();
        for p in &mut rosters[1] {
            if ours.contains(&p.name) {
                p.name = format!("Visiting {}", p.name);
            }
        }
        let m = Match::new(self.arena.clone(), self.ruleset, self.seed, rosters)?;
        let look = |pool: &[(Contender, Profile)], picks: &[usize]| -> anyhow::Result<Vec<Look>> {
            picks
                .iter()
                .map(|i| Look::of(&pool[*i].0, self.reduce_motion))
                .collect()
        };
        self.looks = [look(&self.home, &self.picked)?, look(&self.away, &away)?];
        for (look, p) in self.looks[1].iter_mut().zip(&m.rosters[1]) {
            look.name = p.name.clone();
        }
        self.board_teams = [
            self.picked
                .iter()
                .map(|i| self.home[*i].0.clone())
                .collect(),
            away.iter().map(|i| self.away[*i].0.clone()).collect(),
        ];
        self.teams = [self.picked.clone(), away];
        self.you = Side::Left;
        self.other_name = "The visiting team".to_owned();
        self.friend = None;
        self.begin_match(m, now);
        Ok(())
    }

    fn begin_match(&mut self, m: Match, now: f32) {
        self.facing = [[true; TEAM_SIZE], [false; TEAM_SIZE]];
        self.m = Some(m);
        self.shown = None;
        self.begin_orders(now);
        self.log.clear();
        self.match_events.clear();
        self.earned.clear();
        self.ended_because = None;
    }

    /// The three the Handler has picked, and how they get on: the team sent to a room.
    fn picked_team(&self) -> (Vec<Contender>, Vec<formiga_travel::TravelRelationship>) {
        let team: Vec<Contender> = self
            .picked
            .iter()
            .map(|i| self.home[*i].0.clone())
            .collect();
        let ids: Vec<u64> = team.iter().map(|c| c.traveler.id.0).collect();
        let bonds = self
            .home_bonds
            .iter()
            .filter(|r| ids.contains(&r.a.0) && ids.contains(&r.b.0))
            .copied()
            .collect();
        (team, bonds)
    }

    /// Opens a room on the relay for a friend (`code` is `None`), or joins the friend's room by
    /// its code.
    pub fn open_room(&mut self, handler: &str, code: Option<&str>) {
        self.notice = None;
        if self.picked.len() != TEAM_SIZE {
            self.notice = Some("Choose three first.".to_owned());
            return;
        }
        let (team, bonds) = self.picked_team();
        let entrant = Entrant {
            protocol: formiga_battle_room::RELAY_VERSION,
            rules_version: formiga_battle_core::RULES_VERSION,
            build: self.build(),
            handler: handler.trim().to_owned(),
            team,
            bonds,
            arena: self.arena.id.clone(),
        };
        self.friend = Some(Friend {
            line: Line::start(self.relay.clone(), entrant, code.map(str::to_owned)),
            code: None,
            seated: false,
            handlers: [String::new(), String::new()],
            view: None,
            sent_turn: None,
            opponent_ready: false,
            opponent_away: false,
            deadline: 0.0,
            held: None,
        });
    }

    /// What this Battle plays with: the Desktop that opened it, or none in a rehearsal. The line
    /// to the relay adds the program's digest.
    fn build(&self) -> Build {
        Build::this(self.session.as_ref().map(|_| self.desktop_version.clone()))
    }

    /// Leaves the room, or the match with a friend. Leaving a match forfeits it.
    pub fn leave_room(&mut self) {
        if let Some(friend) = self.friend.take() {
            friend.line.ask(Ask::Leave);
        }
    }

    /// Whatever the relay has said since the last frame. True if a match with a friend began.
    pub fn poll(&mut self, now: f32) -> bool {
        let mut began = false;
        while let Some(told) = self.friend.as_ref().and_then(|f| f.line.told()) {
            match told {
                Told::Seated { code } => {
                    if let Some(f) = &mut self.friend {
                        f.code = Some(code);
                    }
                }
                Told::View(view) => {
                    self.notice = None;
                    began |= self.seen(*view, now);
                }
                Told::Problem(why) => self.notice = Some(why),
                Told::Gone(why) => {
                    if self.outcome().is_none() {
                        self.notice = Some(why);
                    }
                    self.settle_held(
                        false,
                        "The room closed before both boards could be checked, so this match \
                         earned nothing.",
                    );
                    let playing = self.friend.as_ref().is_some_and(|f| f.seated);
                    if !playing || self.outcome().is_some() {
                        self.friend = None;
                    }
                    break;
                }
            }
        }
        self.catch_up(now);
        // Orders not sent by the time the turn locks are sent at the last moment.
        if let Some(friend) = &self.friend
            && friend.seated
            && self.shown.is_none()
            && self.outcome().is_none()
            && self
                .m
                .as_ref()
                .is_some_and(|m| !friend.sent(m.board.turn + 1))
            && friend.deadline - now < 2.0
        {
            self.lock(now);
        }
        began
    }

    /// A new view of the room.
    fn seen(&mut self, view: View, now: f32) -> bool {
        let mine = self.build();
        let Some(friend) = &mut self.friend else {
            return false;
        };
        friend.deadline = now + view.deadline_ms as f32 / 1000.0;
        friend.opponent_ready = view.opponent_ready;
        friend.opponent_away = view.opponent_quiet_ms > 8_000;
        if view.out_of_step {
            self.notice = Some(
                "Your board and your friend's came out differently; this match cannot be trusted."
                    .to_owned(),
            );
        }
        if friend.held.is_some() && (view.confirmed || view.out_of_step) {
            let confirmed = view.confirmed;
            self.settle_held(
                confirmed,
                "Your board and your friend's came out differently, so this match earned nothing.",
            );
        }
        let Some(friend) = &mut self.friend else {
            return false;
        };
        let mut began = false;
        if !friend.seated
            && let Some([a, b]) = &view.entrants
        {
            // The relay checks nothing of the rules: each window checks both teams itself, and that
            // both sides play the same Battle and Desktop by the rules this Battle knows.
            let unfit = [a, b].into_iter().find_map(|e| {
                if e.rules_version != formiga_battle_core::RULES_VERSION
                    || e.build.battle != mine.battle
                    || e.build.desktop != mine.desktop
                    || view.ruleset != Ruleset::default()
                {
                    Some("Your Battle and your friend's are different versions; both need the newest.".to_owned())
                } else {
                    check_team(&e.team, &e.bonds)
                        .err()
                        .map(|reason| format!("A team cannot play: {reason}."))
                }
            });
            if let Some(why) = unfit {
                self.notice = Some(why);
                self.leave_room();
                return false;
            }
            // The opener's arena, which both Battles must have: they are the same version.
            let Some(arena) = Arena::by_id(&a.arena) else {
                self.notice = Some(
                    "Your friend picked an arena this Battle does not have; both need the newest."
                        .to_owned(),
                );
                self.leave_room();
                return false;
            };
            let handlers = [a.handler.clone(), b.handler.clone()];
            match room_match(
                arena,
                view.ruleset,
                view.seed,
                [&a.team, &b.team],
                [&a.bonds, &b.bonds],
            ) {
                Ok(m) => {
                    friend.seated = true;
                    friend.handlers = handlers.clone();
                    let teams = [a.team.clone(), b.team.clone()];
                    let you = view.you;
                    friend.view = Some(view);
                    self.seat_friend_match(m, you, &handlers, teams, now);
                    return true;
                }
                Err(error) => self.notice = Some(format!("The match could not begin: {error}")),
            }
        }
        if let Some(friend) = &mut self.friend {
            friend.view = Some(view);
            began = false;
        }
        began
    }

    /// Works out the next locked turn not yet shown here, once the last has been shown, and
    /// tells the relay the board it came to. A forfeit the relay reports ends the match.
    fn catch_up(&mut self, now: f32) {
        if self.shown.is_some() {
            return;
        }
        let Some(friend) = &self.friend else {
            return;
        };
        let Some(view) = &friend.view else {
            return;
        };
        let Some(m) = &mut self.m else {
            return;
        };
        if !friend.seated {
            return;
        }
        let next = view
            .locked
            .iter()
            .find(|l| l.turn == m.board.turn + 1)
            .copied();
        let ending = view.ending.clone();
        if let Some(locked) = next
            && m.outcome().is_none()
        {
            let before = m.board.clone();
            let report = m.resolve_turn(locked.submitted);
            friend.line.ask(Ask::Board {
                turn: report.turn,
                hash: report.hash,
                finished: m.outcome().is_some(),
            });
            self.show_turn(before, report, now);
            return;
        }
        if let Some(ending) = ending
            && m.outcome().is_none()
            && next.is_none()
            && let Some(side) = ending.forfeited
        {
            m.forfeit(side);
            self.ended_because = Some(ending.reason);
            self.finish_match();
        }
    }

    /// Puts a match with a friend on the board, from either side.
    fn seat_friend_match(
        &mut self,
        mut m: Match,
        you: Side,
        handlers: &[String; 2],
        teams: [Vec<Contender>; 2],
        now: f32,
    ) {
        // A friend's companion who shares a name with one of the Handler's is told apart.
        let ours: Vec<String> = m.rosters[you.index()]
            .iter()
            .map(|p| p.name.clone())
            .collect();
        for p in &mut m.rosters[you.other().index()] {
            if ours.contains(&p.name) {
                p.name = format!("{}'s {}", handlers[you.other().index()], p.name);
            }
        }
        let looks: anyhow::Result<Vec<Vec<Look>>> = teams
            .iter()
            .map(|team| {
                team.iter()
                    .map(|c| Look::of(c, self.reduce_motion))
                    .collect()
            })
            .collect();
        match looks {
            Ok(mut looks) => {
                for (look, p) in looks[you.other().index()]
                    .iter_mut()
                    .zip(&m.rosters[you.other().index()])
                {
                    look.name = p.name.clone();
                }
                let right = looks.pop().expect("two teams");
                let left = looks.pop().expect("two teams");
                self.looks = [left, right];
            }
            Err(error) => {
                self.notice = Some(format!("A companion's look could not be drawn: {error}"));
                return;
            }
        }
        self.you = you;
        self.other_name = format!("{}'s team", handlers[you.other().index()]);
        self.board_teams = teams;
        self.begin_match(m, now);
    }

    /// Shows a resolved turn, and notes the match if it was the last.
    fn show_turn(
        &mut self,
        before: formiga_battle_core::game::Board,
        report: TurnReport,
        now: f32,
    ) {
        let Some(m) = &self.m else {
            return;
        };
        let playback = crate::playback::Playback::new(
            &before,
            &m.arena.gates,
            &self.facing,
            &report.events,
            m.ruleset.ticks_per_turn,
        );
        let mut names = ["", ""];
        names[self.you.index()] = "Your team";
        names[self.you.other().index()] = &self.other_name;
        self.log
            .push((report.turn, recap_naming(m, &report.events, names)));
        self.match_events.extend(report.events.iter().cloned());
        let other = self.other_short();
        let mut teams = ["", ""];
        teams[self.you.index()] = "you";
        teams[self.you.other().index()] = if self.friend.is_some() {
            &other
        } else {
            "the visitors"
        };
        let callouts = crate::announce::callouts(m, &playback, teams);
        let over = m.outcome().is_some();
        self.shown = Some(Shown {
            callouts,
            playback,
            report,
            started: now,
        });
        self.selected = None;
        self.verb = None;
        if over {
            self.finish_match();
        }
    }

    fn begin_orders(&mut self, now: f32) {
        self.orders = [None; TEAM_SIZE];
        self.verb = None;
        self.turn_began = now;
        self.selected = self
            .m
            .as_ref()
            .and_then(|m| m.team(self.you).next().map(|s| s.index));
    }

    pub fn outcome(&self) -> Option<Outcome> {
        self.m.as_ref().and_then(Match::outcome)
    }

    /// What the other side is called in a few words: the visitors, or the friend's name.
    pub fn other_short(&self) -> String {
        match &self.friend {
            Some(f) => f.handlers[self.you.other().index()].clone(),
            None => "Visitors".to_owned(),
        }
    }

    /// Seconds left on the timer: the room's, against a friend.
    pub fn time_left(&self, now: f32) -> f32 {
        match &self.friend {
            Some(f) => (f.deadline - now).max(0.0),
            None => (f32::from(self.ruleset.turn_seconds) - (now - self.turn_began)).max(0.0),
        }
    }

    /// Chooses a creature of the Handler's to give an order to.
    pub fn select(&mut self, index: u8) {
        if self
            .m
            .as_ref()
            .is_some_and(|m| m.standing(Slot::new(self.you, index)))
        {
            self.selected = Some(index);
            self.verb = None;
        }
    }

    pub fn choose_verb(&mut self, verb: Verb) {
        if verb == Verb::Use && self.carrying().is_none() {
            return;
        }
        if self.selected.is_some() {
            self.verb = Some(verb);
        }
    }

    /// What the chosen one carries, if anything.
    pub fn carrying(&self) -> Option<formiga_battle_core::Item> {
        let (m, index) = (self.m.as_ref()?, self.selected?);
        m.fighter(Slot::new(self.you, index)).carrying
    }

    /// The tiles the chosen order could be aimed at, as the board marks them. With someone chosen
    /// and no order picked yet, a click does what the tile suggests (`aim`), so the board marks
    /// where it can walk to and whom it can contest.
    pub fn choices(&self) -> Vec<(Tile, Choice)> {
        let (Some(m), Some(index)) = (&self.m, self.selected) else {
            return Vec::new();
        };
        let slot = Slot::new(self.you, index);
        let reach = |choice: Choice| -> Vec<(Tile, Choice)> {
            let from = m.fighter(slot).at;
            let dist = distances(m, slot, &[from], false);
            let w = m.arena.width as usize;
            m.arena
                .tiles()
                .filter(|t| m.walkable(*t))
                .filter(|t| {
                    dist[t.y as usize * w + t.x as usize]
                        .is_some_and(|d| d <= m.ruleset.ticks_per_turn)
                })
                .map(|t| (t, choice))
                .collect()
        };
        let contests = || {
            m.team(self.you.other())
                .map(|e| (m.fighter(e).at, Choice::Contest))
                .chain(m.arena.levers.iter().map(|l| (*l, Choice::Contest)))
                .chain(m.arena.switches.iter().map(|t| (*t, Choice::Contest)))
        };
        match self.verb {
            Some(Verb::Move) => reach(Choice::Walk),
            Some(Verb::Hold) => reach(Choice::Hold),
            Some(Verb::Contest) => contests().collect(),
            Some(Verb::Use) => {
                // Anywhere it could reach this turn and still be within the item's reach of.
                let Some(item) = m.fighter(slot).carrying else {
                    return Vec::new();
                };
                let from = m.fighter(slot).at;
                let far = m.ruleset.ticks_per_turn + item.reach();
                m.arena
                    .tiles()
                    .filter(|t| from.distance(*t) <= far)
                    .filter(|t| m.valid(slot, &Order::Use { at: *t }))
                    .map(|t| (t, Choice::Use))
                    .collect()
            }
            None => {
                let mut all = reach(Choice::Walk);
                all.retain(|(t, _)| m.occupant(*t).is_none_or(|s| s.side == self.you));
                all.extend(contests());
                all
            }
        }
    }

    /// What a click on `tile` would do while giving orders: a plain click (`hold` false) or a
    /// right click (`hold` true). On one of the Handler's own it chooses that one. Otherwise,
    /// with an order picked (by its button or its key) it aims that order; with none, a plain
    /// click contests whoever of theirs or the lever is there, holds where the chosen one already
    /// stands, or walks there, and a right click holds there.
    pub fn aim(&self, tile: Tile, hold: bool) -> Aim {
        let Some(m) = &self.m else {
            return Aim::Nothing;
        };
        if self.shown.is_some() || m.outcome().is_some() {
            return Aim::Nothing;
        }
        let selected = self.selected.map(|i| Slot::new(self.you, i));
        let own = m
            .occupant(tile)
            .filter(|s| s.side == self.you && m.standing(*s));
        if let Some(s) = own
            && Some(s) != selected
        {
            return Aim::Choose(s.index);
        }
        let Some(slot) = selected else {
            return Aim::Nothing;
        };
        let enemy = m
            .occupant(tile)
            .filter(|s| s.side != self.you && m.standing(*s));
        let contest = || {
            if let Some(enemy) = enemy {
                Some(Order::Contest {
                    target: Target::Creature { slot: enemy },
                })
            } else if m.arena.levers.contains(&tile) || m.arena.switches.contains(&tile) {
                // A tap is turned as a lever is pulled.
                Some(Order::Contest {
                    target: Target::Lever { at: tile },
                })
            } else {
                None
            }
        };
        let order = match (self.verb, hold) {
            (Some(Verb::Move), _) => Some(Order::Move { to: tile }),
            (Some(Verb::Hold), _) | (None, true) => Some(Order::Hold { at: tile }),
            (Some(Verb::Contest), _) => contest(),
            (Some(Verb::Use), _) => Some(Order::Use { at: tile }),
            (None, false) => contest().or(Some(if m.fighter(slot).at == tile {
                Order::Hold { at: tile }
            } else {
                Order::Move { to: tile }
            })),
        };
        match order {
            Some(order) if m.valid(slot, &order) => Aim::Give(order),
            _ => Aim::Nothing,
        }
    }

    /// A click on a tile while giving orders, as `aim` says: chooses one of the Handler's, or
    /// gives the chosen one its order and moves on to the next still without one.
    pub fn press(&mut self, tile: Tile, hold: bool) -> Aim {
        let aim = self.aim(tile, hold);
        match aim {
            Aim::Choose(index) => self.select(index),
            Aim::Give(order) => {
                let index = self
                    .selected
                    .expect("an order is aimed for whoever is chosen");
                self.orders[index as usize] = Some(order);
                self.verb = None;
                // On to the next creature still without an order.
                if let Some(m) = &self.m {
                    self.selected = m
                        .team(self.you)
                        .map(|s| s.index)
                        .find(|i| self.orders[*i as usize].is_none())
                        .or(Some(index));
                }
            }
            Aim::Nothing => {}
        }
        aim
    }

    /// A plain click on a tile. True if it changed anything.
    #[cfg(test)]
    pub fn click(&mut self, tile: Tile) -> bool {
        self.press(tile, false) != Aim::Nothing
    }

    pub fn clear_order(&mut self, index: u8) {
        self.orders[index as usize] = None;
    }

    /// What the board shows over itself just now.
    pub fn marks(&self, hover: Option<Tile>, now: f32) -> Marks {
        let mut marks = Marks {
            selected: self.selected.map(|i| Slot::new(self.you, i)),
            choices: self.choices(),
            hover,
            orders: Vec::new(),
            ..Marks::default()
        };
        match &self.shown {
            // While a turn is shown, both teams' orders, until the first tick.
            Some(shown) if now - shown.started < crate::playback::REVEAL_SECS => {
                marks.selected = None;
                for slot in Slot::all() {
                    if let Some(order) = shown.report.orders[slot.side.index()][slot.index as usize]
                    {
                        marks.orders.push((slot, order));
                    }
                }
            }
            Some(_) => marks.selected = None,
            None => {
                for (i, order) in self.orders.iter().enumerate() {
                    if let Some(order) = order {
                        marks.orders.push((Slot::new(self.you, i as u8), *order));
                    }
                }
            }
        }
        marks
    }

    /// Locks the Handler's orders: against the AI, resolves the turn and starts showing it;
    /// against a friend, sends them to the room, which resolves the turn when both are in.
    pub fn lock(&mut self, now: f32) {
        let Some(m) = &mut self.m else {
            return;
        };
        if self.shown.is_some() || m.outcome().is_some() {
            return;
        }
        if let Some(friend) = &mut self.friend {
            let turn = m.board.turn + 1;
            if !friend.seated || friend.sent(turn) {
                return;
            }
            friend.line.ask(Ask::Orders {
                turn,
                orders: self.orders,
            });
            friend.sent_turn = Some(turn);
            self.selected = None;
            self.verb = None;
            return;
        }
        let away = Coach::Exhibition.orders(m, self.you.other());
        let before = m.board.clone();
        let report = m.resolve_turn([self.orders, away]);
        self.show_turn(before, report, now);
    }

    /// The Handler's orders, as the AI coach would give them: for review pictures.
    pub fn coach_orders(&mut self) {
        if let Some(m) = &self.m {
            self.orders = Coach::Exhibition.orders(m, self.you);
        }
    }

    /// The board at `now`, and whether a turn is still being shown. Once one has been shown
    /// through, the next turn's orders begin.
    pub fn frame(&mut self, now: f32) -> Option<Frame> {
        let m = self.m.as_ref()?;
        if let Some(shown) = &self.shown {
            let t = now - shown.started;
            let frame = shown.playback.at(t);
            if t >= shown.playback.duration() && m.outcome().is_none() {
                for slot in Slot::all() {
                    self.facing[slot.side.index()][slot.index as usize] =
                        frame.figure(slot).facing_right;
                }
                let still = Frame::still(&m.board, &self.facing);
                self.shown = None;
                self.begin_orders(now);
                return Some(still);
            }
            return Some(frame);
        }
        Some(Frame::still(&m.board, &self.facing))
    }

    /// Each team's beacon progress as the board shows it just now.
    pub fn frame_capture(&self, now: f32) -> [u8; 2] {
        match (&self.shown, &self.m) {
            (Some(shown), _) => shown.playback.at(now - shown.started).capture,
            (None, Some(m)) => m.board.capture,
            (None, None) => [0, 0],
        }
    }

    /// Whether the turn being shown is the last, and has been shown through.
    pub fn finished(&self, now: f32) -> bool {
        self.outcome().is_some()
            && self
                .shown
                .as_ref()
                .is_none_or(|s| now - s.started >= s.playback.duration())
    }

    /// What the fallback would choose for a creature still without an order.
    pub fn fallback(&self, index: u8) -> Option<Order> {
        self.m
            .as_ref()
            .map(|m| ai::fallback(m, Slot::new(self.you, index)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_battle_contract::sample;

    fn table() -> Table {
        let mut t = Table::new(sample::snapshot(), sample::visitors(9), 5);
        t.start(0.0).unwrap();
        t
    }

    #[test]
    fn a_handler_aims_an_order_by_choosing_a_creature_a_verb_and_a_tile() {
        let mut t = table();
        assert_eq!(t.selected, Some(0));
        t.choose_verb(Verb::Hold);
        let (tile, _) = t.choices()[0];
        assert!(t.click(tile));
        assert_eq!(t.orders[0], Some(Order::Hold { at: tile }));
        assert_eq!(
            t.selected,
            Some(1),
            "on to the next creature without an order"
        );
    }

    #[test]
    fn a_contest_can_only_be_aimed_at_the_other_team_or_the_lever() {
        let mut t = table();
        t.choose_verb(Verb::Contest);
        let m = t.m.as_ref().unwrap();
        let teammate = m.fighter(Slot::new(Side::Left, 1)).at;
        let enemy = m.fighter(Slot::new(Side::Right, 0)).at;
        assert!(t.click(teammate), "clicking a teammate chooses it instead");
        assert_eq!(t.selected, Some(1));
        assert_eq!(t.orders[0], None);
        t.choose_verb(Verb::Contest);
        assert!(t.click(enemy));
        assert!(matches!(t.orders[1], Some(Order::Contest { .. })));
    }

    #[test]
    fn with_no_order_picked_a_click_does_what_the_tile_suggests() {
        let mut t = table();
        let m = t.m.as_ref().unwrap();
        let own = m.fighter(Slot::new(Side::Left, 0)).at;
        let enemy_slot = Slot::new(Side::Right, 0);
        let enemy = m.fighter(enemy_slot).at;
        let (ground, _) = *t
            .choices()
            .iter()
            .find(|(tile, choice)| *choice == Choice::Walk && *tile != own)
            .expect("somewhere to walk");
        assert_eq!(t.aim(own, false), Aim::Give(Order::Hold { at: own }));
        assert_eq!(t.aim(ground, false), Aim::Give(Order::Move { to: ground }));
        assert_eq!(t.aim(ground, true), Aim::Give(Order::Hold { at: ground }));
        assert_eq!(
            t.aim(enemy, false),
            Aim::Give(Order::Contest {
                target: Target::Creature { slot: enemy_slot }
            })
        );
        assert_eq!(t.press(ground, true), Aim::Give(Order::Hold { at: ground }));
        assert_eq!(t.orders[0], Some(Order::Hold { at: ground }));
        assert_eq!(t.selected, Some(1));
    }

    #[test]
    fn a_click_on_a_teammate_chooses_it_whatever_the_button() {
        let mut t = table();
        let mate = t.m.as_ref().unwrap().fighter(Slot::new(Side::Left, 2)).at;
        assert_eq!(t.press(mate, true), Aim::Choose(2));
        assert_eq!(t.selected, Some(2));
        assert_eq!(t.orders, [None; TEAM_SIZE]);
    }

    #[test]
    fn nothing_can_be_aimed_while_a_turn_plays_out() {
        let mut t = table();
        let own = t.m.as_ref().unwrap().fighter(Slot::new(Side::Left, 0)).at;
        t.lock(0.0);
        assert_eq!(t.aim(own, false), Aim::Nothing);
    }

    #[test]
    fn a_locked_turn_is_shown_through_and_the_next_turns_orders_begin() {
        let mut t = table();
        t.lock(1.0);
        assert!(t.shown.is_some());
        let length = t.shown.as_ref().unwrap().playback.duration();
        t.frame(1.0 + length / 2.0);
        assert!(t.shown.is_some());
        t.frame(1.0 + length + 0.1);
        assert!(t.shown.is_none());
        assert_eq!(t.orders, [None; TEAM_SIZE]);
        assert_eq!(t.log.len(), 1);
        assert_eq!(t.m.as_ref().unwrap().board.turn, 1);
    }

    #[test]
    fn a_whole_match_can_be_played_through_the_table() {
        let mut t = table();
        let mut now = 0.0;
        while !t.finished(now) {
            t.coach_orders();
            t.lock(now);
            now += 20.0;
            t.frame(now);
        }
        assert!(t.outcome().is_some());
    }

    #[test]
    fn a_visitor_with_one_of_your_names_is_told_apart() {
        let mut t = table();
        let ours = t.m.as_ref().unwrap().rosters[0][0].name.clone();
        t.away[t.teams[1][0]].0.traveler.name = ours.clone();
        t.away[t.teams[1][0]].1.name = ours.clone();
        t.start(0.0).unwrap();
        let theirs: Vec<String> = t.m.as_ref().unwrap().rosters[1]
            .iter()
            .map(|p| p.name.clone())
            .collect();
        assert!(!theirs.contains(&ours), "{theirs:?}");
    }

    #[test]
    fn an_ai_match_earns_its_rewards_and_writes_them_into_desktops_session() {
        let dir = std::env::temp_dir().join(format!("formiga-battle-table-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut t = Table::new(sample::snapshot(), sample::visitors(9), 5);
        t.session = Some(Session {
            directory: dir.clone(),
            result: BattleResult::new(
                formiga_battle_contract::SessionId::parse("00000000000000000000000000000000")
                    .unwrap(),
                "0".repeat(64),
                "0.1.0",
            ),
        });
        t.start(0.0).unwrap();
        let mut now = 0.0;
        while !t.finished(now) {
            t.coach_orders();
            t.lock(now);
            now += 20.0;
            t.frame(now);
        }
        assert!(!t.earned.is_empty());
        assert_eq!(t.record.circuit_points, 0, "the AI gives no points");
        let written = BattleResult::decode(
            &std::fs::read(dir.join(formiga_battle_contract::RESULT_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(written.matches.len(), 1);
        assert_eq!(written.matches[0].mode, formiga_battle_contract::Mode::Ai);
    }

    #[test]
    fn two_tables_play_a_whole_match_through_a_relay() {
        let relay =
            formiga_battle_room::serve_relay("127.0.0.1:0", formiga_battle_room::Limits::default())
                .unwrap();
        let mut host = Table::new(sample::snapshot(), sample::visitors(9), 1);
        let mut guest = Table::new(sample::visitors(11), sample::visitors(9), 2);
        host.relay = relay.base();
        guest.relay = relay.base();
        host.open_room("Jake", None);
        let mut now = 0.0;
        // Opening a room first fingerprints this program, which takes a while in a debug build
        // with the other tests busy, so the wait is by the wall clock rather than by the table's.
        let opened = std::time::Instant::now();
        let code = loop {
            host.poll(now);
            if let Some(code) = host.friend.as_ref().and_then(|f| f.code.clone()) {
                break code;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
            now += 0.005;
            assert!(
                opened.elapsed().as_secs() < 60,
                "the room opens: {:?}",
                host.notice
            );
        };
        guest.open_room("Friend", Some(&code.to_lowercase()));
        let mut began = [false, false];
        while !(began[0] && began[1]) {
            began[0] |= host.poll(now);
            began[1] |= guest.poll(now);
            std::thread::sleep(std::time::Duration::from_millis(20));
            now += 0.02;
            assert!(now < 20.0, "both see the match begin");
        }
        assert_eq!(host.you, Side::Left);
        assert_eq!(guest.you, Side::Right);
        assert_eq!(host.other_short(), "Friend");
        let mut rounds = 0;
        while !(host.finished(now) && guest.finished(now)) {
            for t in [&mut host, &mut guest] {
                let turn = t.m.as_ref().unwrap().board.turn + 1;
                if t.shown.is_none()
                    && t.outcome().is_none()
                    && !t.friend.as_ref().unwrap().sent(turn)
                {
                    t.coach_orders();
                    t.lock(now);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
            now += 2.0;
            host.poll(now);
            guest.poll(now);
            host.frame(now);
            guest.frame(now);
            rounds += 1;
            assert!(rounds < 600, "the match ends");
        }
        let (a, b) = (host.m.as_ref().unwrap(), guest.m.as_ref().unwrap());
        assert_eq!(a.board, b.board, "both see the same board");
        assert!(a.outcome().is_some());
        // What it earned is kept once the relay has confirmed both boards.
        let held = |t: &Table| t.friend.as_ref().is_some_and(|f| f.held.is_some());
        while held(&host) || held(&guest) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            now += 1.0;
            host.poll(now);
            guest.poll(now);
            rounds += 1;
            assert!(rounds < 700, "the boards are confirmed");
        }
        for t in [&host, &guest] {
            assert!(
                t.earned.iter().any(|l| l.contains("Open Circuit points")),
                "{:?}",
                t.earned
            );
        }
        relay.stop();
    }

    #[test]
    fn a_colony_of_one_is_joined_by_friends_from_next_door() {
        let mut roster = sample::snapshot();
        roster.contenders.truncate(1);
        roster.relationships.clear();
        let t = Table::new(roster, sample::visitors(9), 5);
        assert_eq!(t.home.len(), 3);
        assert!(t.away.len() >= 3);
    }
}
