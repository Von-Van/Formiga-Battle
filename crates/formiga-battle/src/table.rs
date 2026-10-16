//! One sitting at the board, without the window: the two rosters, the match, how everyone looks
//! and faces, the orders the Handler has given so far, and the turn being shown. The window only
//! asks this what to show and tells it what was clicked, so tests drive the same code.

use crate::announce::Callout;
use crate::friend::{Ask, Line, Told};
use crate::look::Look;
use crate::playback::{Frame, Playback};
use crate::rewards;
use crate::scene::{Choice, Marks, PlanKind, PlanMark};
use formiga_battle_contract::{BattleRecord, BattleResult};
use formiga_battle_contract::{BattleSnapshot, Contender};
use formiga_battle_core::Event;
use formiga_battle_core::ai::Coach;
use formiga_battle_core::muster::checked;
use formiga_battle_core::notebook::Page;
use formiga_battle_core::recap::recap_naming;
use formiga_battle_core::rng::{self, Purpose};
use formiga_battle_core::{
    Arena, Directive, Facing, Match, Outcome, PLAN_SIZE, Profile, Ruleset, Side, Slot, TEAM_SIZE,
    TeamPlan, Tile, TurnReport, order::MAX_SQUARES,
};
use formiga_battle_room::roster::{check_team, room_match};
use formiga_battle_room::{Build, Card, Entrant, View, relay_base};
use std::path::PathBuf;

/// The session Desktop opened, and the answer being written into it.
pub struct Session {
    pub directory: PathBuf,
    pub result: BattleResult,
}

/// The longest a reason from the relay for a match ending is shown.
const REASON_CHARS: usize = 160;

/// A Handler's name as the relay passed it on, cleaned again on arrival, as the relay cleaned it
/// on its way in: it is shown in the window and kept in the history Desktop is sent.
fn handler_name(raw: &str) -> String {
    let name = formiga_expansion_rulebook::sanitize_text(raw, 24);
    if name.is_empty() {
        "A Handler".to_owned()
    } else {
        name
    }
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
    /// What each Handler shows of themselves besides their name, left then right, cleaned again
    /// on arrival.
    pub cards: [Option<Card>; 2],
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

/// The kinds of team order: the four that take one of the three places in the plan, and USE
/// ITEM, a bonus for each of the team carrying something.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Press,
    Hold,
    Trick,
    Interact,
    UseItem,
}

impl Verb {
    /// The orders that take one of the three places.
    pub const ALL: [Self; 4] = [Self::Press, Self::Hold, Self::Trick, Self::Interact];

    pub const fn word(self) -> &'static str {
        match self {
            Self::Press => "PRESS",
            Self::Hold => "HOLD",
            Self::Trick => "TRICK",
            Self::Interact => "INTERACT",
            Self::UseItem => "USE ITEM",
        }
    }

    /// The key that picks it up.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Press => "P",
            Self::Hold => "H",
            Self::Trick => "T",
            Self::Interact => "I",
            Self::UseItem => "U",
        }
    }

    /// What it does, in a few words, for the order buttons and the field guide.
    pub const fn gloss(self) -> &'static str {
        match self {
            Self::Press => "push theirs off, take what they carry",
            Self::Hold => "keep squares, facing one way",
            Self::Trick => "round a holder's side or back",
            Self::Interact => "levers, taps, found things",
            Self::UseItem => "a bonus, at the start of the turn",
        }
    }

    /// What it does and what beats it, in plain words.
    pub const fn hint(self) -> &'static str {
        match self {
            Self::Press => {
                "PRESS pushes back anyone not holding and steals what it carries; turned away by a HOLD, and hurt by one facing it; two from different sides overwhelm a lone holder."
            }
            Self::Hold => {
                "HOLD keeps a square facing one way; turns away a PRESS and hurts one coming head on; open to a TRICK from the side or behind. R or the mouse wheel turns which way it faces."
            }
            Self::Trick => {
                "TRICK slips round a holder's side or back to break its hold; fails from the front; caught by a PRESS."
            }
            Self::Interact => "INTERACT works levers, taps and fetches found things.",
            Self::UseItem => {
                "USE ITEM: a bonus for a creature carrying something, used at the start of the turn. It does not take one of the three orders."
            }
        }
    }

    /// Whether it names squares, one after another, as the pointer is clicked or dragged.
    pub const fn names_squares(self) -> bool {
        matches!(self, Self::Press | Self::Hold | Self::Trick)
    }

    pub fn of(d: &Directive) -> Self {
        match d {
            Directive::Press { .. } => Self::Press,
            Directive::Hold { .. } => Self::Hold,
            Directive::Trick { .. } => Self::Trick,
            Directive::Interact { .. } => Self::Interact,
            Directive::UseItem { .. } => Self::UseItem,
        }
    }
}

/// What a click on the board does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Aim {
    /// Sets the chosen order to this; a USE ITEM goes beside the three, for its user.
    Give(Directive),
    /// With USE ITEM in hand, picks which of the team carrying something uses it, by roster place.
    Carrier(u8),
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
    /// The Handler's orders so far this turn: three places, each empty or one team order.
    pub orders: [Option<Directive>; PLAN_SIZE],
    /// Which of the three orders a click on the board gives.
    pub selected: Option<u8>,
    pub verb: Option<Verb>,
    /// The bonus USE ITEMs given so far this turn: for each of the team by roster place, the tile
    /// it is to use what it carries on.
    pub uses: [Option<Tile>; TEAM_SIZE],
    /// With USE ITEM in hand, which of the team carrying something it is for, by roster place.
    pub user: Option<u8>,
    /// The HOLD the R key and the mouse wheel turn: the one last given or added to.
    pub turning: Option<u8>,
    /// When the current turn's orders began, on the window's clock.
    pub turn_began: f32,
    pub shown: Option<Shown>,
    /// Every turn told, newest last.
    pub log: Vec<(u8, Vec<String>)>,
    /// What came of each of the Handler's orders last turn, a line each.
    pub told: Vec<String>,
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
            orders: [None, None, None],
            selected: None,
            verb: None,
            uses: [None; TEAM_SIZE],
            user: None,
            turning: None,
            turn_began: 0.0,
            shown: None,
            log: Vec::new(),
            told: Vec::new(),
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
                friend.cards[self.you.other().index()].as_ref(),
                now,
            ),
            None => rewards::ai_match(m, self.you, &self.match_events, &self.record),
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
        if let Some(trio) = earned
            .result
            .rewards
            .trio
            .as_ref()
            .and_then(|played| self.record.trios.iter().find(|t| t.id == played.id))
        {
            self.earned.push(format!(
                "{}: {}",
                trio.name,
                crate::identity::record_words(trio)
            ));
        }
        if let Some(session) = &mut self.session {
            session.result.matches.push(earned.result);
        }
        self.write_session("What this match earned");
    }

    /// Writes the session's answer for Desktop again, whole, with the Handler card and trios as
    /// they stand. A rehearsal keeps nothing.
    fn write_session(&mut self, what: &str) {
        let Some(session) = &mut self.session else {
            return;
        };
        let registry = crate::identity::registry(&self.record);
        session.result.registry = (!registry.is_empty()).then_some(registry);
        if let Err(error) = session.result.write(&session.directory) {
            self.notice = Some(format!("{what} could not be kept for Desktop: {error}"));
        }
    }

    /// The Handler card is now this.
    pub fn set_handler(&mut self, card: formiga_battle_contract::HandlerCard) {
        self.register(formiga_battle_contract::Registry {
            handler: Some(card),
            trios: Vec::new(),
        });
    }

    /// A trio named, renamed, given a pennant, retired or given new members: kept here at once,
    /// and for Desktop when Battle closes.
    pub fn set_trio(&mut self, card: formiga_battle_contract::TrioCard) {
        self.register(formiga_battle_contract::Registry {
            handler: None,
            trios: vec![card],
        });
    }

    fn register(&mut self, registry: formiga_battle_contract::Registry) {
        if !registry.holds() {
            self.notice = Some("That is not something your notebook can keep.".to_owned());
            return;
        }
        self.record
            .register(&registry, time::OffsetDateTime::now_utc());
        self.write_session("Your Handler card and trios");
    }

    /// What a friend is shown of this Handler besides their name: their club, pennant and
    /// standing, and the trio the three picked are, if they are one.
    pub fn shared_card(&self) -> Option<Card> {
        let ids: Vec<u64> = self
            .picked
            .iter()
            .filter_map(|&i| self.home.get(i))
            .map(|(c, _)| c.traveler.id.0)
            .collect();
        let trio = crate::identity::trio_of(&self.record, &ids);
        let handler = self.record.handler.as_ref();
        Card {
            club: handler.and_then(|h| h.club.clone()),
            pennant: handler.map(|h| h.pennant.clone()),
            standing: Some(crate::identity::Standing::of(&self.record).line()),
            trio: trio.map(|t| t.name.clone()),
            trio_pennant: trio.and_then(|t| t.pennant.clone()),
        }
        .cleaned()
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
        self.told.clear();
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
        // The name signed here is the Handler's from now on.
        if let Some(card) = crate::identity::card_with_name(&self.record, handler)
            && self.record.handler.as_ref() != Some(&card)
        {
            self.set_handler(card);
        }
        let entrant = Entrant {
            protocol: formiga_battle_room::RELAY_VERSION,
            rules_version: formiga_battle_core::RULES_VERSION,
            build: self.build(),
            handler: handler.trim().to_owned(),
            team,
            bonds,
            arena: self.arena.id.clone(),
            card: self.shared_card(),
        };
        self.friend = Some(Friend {
            line: Line::start(self.relay.clone(), entrant, code.map(str::to_owned)),
            code: None,
            seated: false,
            handlers: [String::new(), String::new()],
            cards: [None, None],
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
            let handlers = [handler_name(&a.handler), handler_name(&b.handler)];
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
                    friend.cards = [
                        a.card.as_ref().and_then(Card::cleaned),
                        b.card.as_ref().and_then(Card::cleaned),
                    ];
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
            .cloned();
        let ending = view.ending.clone();
        if let Some(locked) = &next
            && m.outcome().is_none()
        {
            let before = m.board.clone();
            let report = m.resolve_turn(locked.submitted.clone());
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
            self.ended_because = Some(formiga_expansion_rulebook::sanitize_text(
                &ending.reason,
                REASON_CHARS,
            ));
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
        self.told = formiga_battle_core::recap::orders_told(m, self.you, &report.events);
        self.match_events.extend(report.events.iter().cloned());
        let other = self.other_short();
        let mut teams = ["", ""];
        teams[self.you.index()] = "you";
        teams[self.you.other().index()] = if self.friend.is_some() {
            &other
        } else {
            "the visitors"
        };
        let callouts = crate::announce::callouts(m, &playback, &report.events, teams, self.you);
        let over = m.outcome().is_some();
        self.shown = Some(Shown {
            callouts,
            playback,
            report,
            started: now,
        });
        self.selected = None;
        self.put_down();
        if over {
            self.finish_match();
        }
    }

    fn begin_orders(&mut self, now: f32) {
        self.orders = [None, None, None];
        self.uses = [None; TEAM_SIZE];
        self.put_down();
        self.turning = None;
        self.turn_began = now;
        self.selected = Some(0);
    }

    /// Puts down whatever order is in hand, and the creature a USE ITEM was for.
    pub fn put_down(&mut self) {
        self.verb = None;
        self.user = None;
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

    /// Chooses which of the three orders a click on the board gives.
    pub fn select(&mut self, index: u8) {
        if usize::from(index) < PLAN_SIZE {
            self.selected = Some(index);
            self.put_down();
        }
    }

    pub fn choose_verb(&mut self, verb: Verb) {
        if verb == Verb::UseItem {
            self.take_token(verb);
        } else if self.selected.is_some() {
            self.verb = Some(verb);
            self.user = None;
        }
    }

    /// The team's plan as given so far: the orders in their places, then each USE ITEM.
    pub fn plan(&self) -> TeamPlan {
        let mut plan: TeamPlan = self.orders.iter().flatten().cloned().collect();
        plan.extend(
            self.uses
                .iter()
                .enumerate()
                .filter_map(|(by, at)| at.map(|at| Directive::UseItem { by: by as u8, at })),
        );
        plan
    }

    /// Those of the Handler's team who can be told to USE ITEM this turn, by roster place: each
    /// one standing and carrying something.
    pub fn carriers(&self) -> Vec<u8> {
        let Some(m) = &self.m else {
            return Vec::new();
        };
        (0..TEAM_SIZE as u8)
            .filter(|i| {
                let slot = Slot::new(self.you, *i);
                m.standing(slot) && m.fighter(slot).carrying.is_some()
            })
            .collect()
    }

    /// Who of the Handler's team would probably take which job if the plan were locked as it
    /// stands, and why: the team's own sharing out, from the board as it is, for this team only.
    /// It says nothing of how each will walk, who will think again, or what the other team will
    /// do, so it is a likelihood, never the turn itself. Empty unless orders are being given.
    pub fn likely(&self) -> Vec<(Slot, formiga_battle_core::Task, formiga_battle_core::Why)> {
        let Some(m) = &self.m else {
            return Vec::new();
        };
        let plan = self.plan();
        if plan.is_empty() || self.shown.is_some() || m.outcome().is_some() {
            return Vec::new();
        }
        let (_, _, events) =
            formiga_battle_core::muster::muster(m, self.you, &plan, m.board.turn + 1);
        events
            .into_iter()
            .filter_map(|e| match e.kind {
                formiga_battle_core::EventKind::Assigned {
                    slot, task, why, ..
                } => Some((slot, task, why)),
                _ => None,
            })
            .collect()
    }

    /// Whether this order could be carried out at all just now. A USE ITEM needs its user
    /// standing, carrying something, and the tile within its reach.
    fn fits(&self, d: &Directive) -> bool {
        self.m.as_ref().is_some_and(|m| match d {
            Directive::UseItem { by, at } => {
                let slot = Slot::new(self.you, *by);
                usize::from(*by) < TEAM_SIZE && m.standing(slot) && m.can_use(slot, *at)
            }
            _ => checked(m, self.you, &vec![d.clone()]).0.len() == 1,
        })
    }

    /// The way a new HOLD on `tile` faces to begin with: toward the nearest of the other team
    /// still standing, or across the board toward their side if there is no telling.
    pub fn first_facing(&self, tile: Tile) -> Facing {
        let across = Facing::across(self.you);
        let Some(m) = &self.m else {
            return across;
        };
        m.team(self.you.other())
            .filter(|s| m.standing(*s))
            .map(|s| m.fighter(s).at)
            .min_by_key(|at| (tile.distance(*at), at.y, at.x))
            .and_then(|at| Facing::toward(tile, at))
            .unwrap_or(across)
    }

    /// Turns the HOLD in this place of the plan a quarter clockwise, or back the other way. True
    /// if there was a HOLD there to turn.
    pub fn turn_order(&mut self, index: u8, back: bool) -> bool {
        let Some(Some(Directive::Hold { facing, .. })) = self.orders.get_mut(usize::from(index))
        else {
            return false;
        };
        *facing = if back {
            facing.turned().turned().turned()
        } else {
            facing.turned()
        };
        self.turning = Some(index);
        true
    }

    /// The HOLD the R key and the mouse wheel turn: the one in hand, or else the one last given
    /// or added to.
    pub fn turnable(&self) -> Option<u8> {
        let hold = |i: &u8| matches!(self.orders[usize::from(*i)], Some(Directive::Hold { .. }));
        self.selected
            .filter(|i| self.verb == Some(Verb::Hold) && hold(i))
            .or(self.turning.filter(hold))
    }

    /// Turns the HOLD `turnable` names. True if there was one.
    pub fn turn_hold(&mut self, back: bool) -> bool {
        self.turnable().is_some_and(|i| self.turn_order(i, back))
    }

    /// The squares the plan names, each with its order's colour and kind, a square named by more
    /// than one order stacked; and where each USE ITEM is to land, from its user.
    pub fn plan_marks(&self) -> Vec<PlanMark> {
        let mut out: Vec<PlanMark> = Vec::new();
        for (i, d) in self.orders.iter().enumerate() {
            let Some(d) = d else {
                continue;
            };
            let (kind, facing) = match d {
                Directive::Press { .. } => (PlanKind::Press, None),
                Directive::Hold { facing, .. } => (PlanKind::Hold, Some(*facing)),
                Directive::Trick { .. } => (PlanKind::Trick, None),
                Directive::Interact { .. } | Directive::UseItem { .. } => {
                    (PlanKind::Interact, None)
                }
            };
            for t in d.squares() {
                let stack = out.iter().filter(|m| m.tile == *t).count() as u8;
                out.push(PlanMark {
                    tile: *t,
                    order: i as u8,
                    kind,
                    stack,
                    chosen: self.selected == Some(i as u8)
                        && self.verb.is_some_and(|v| v != Verb::UseItem),
                    facing,
                    from: None,
                });
            }
        }
        for (by, at) in self.uses.iter().enumerate() {
            let Some(at) = at else {
                continue;
            };
            let stack = out.iter().filter(|m| m.tile == *at).count() as u8;
            out.push(PlanMark {
                tile: *at,
                order: by as u8,
                kind: PlanKind::UseItem,
                stack,
                chosen: self.verb == Some(Verb::UseItem) && self.user == Some(by as u8),
                facing: None,
                from: self
                    .m
                    .as_ref()
                    .map(|m| m.fighter(Slot::new(self.you, by as u8)).at),
            });
        }
        out
    }

    /// Adds a square to the PRESS, HOLD or TRICK being given, as the pointer is dragged across
    /// the board; never takes one out. True if it changed anything.
    pub fn add_square(&mut self, tile: Tile) -> bool {
        let (Some(index), Some(verb)) = (self.selected, self.verb) else {
            return false;
        };
        if !verb.names_squares() {
            return false;
        }
        let current = self.orders[usize::from(index)]
            .clone()
            .filter(|d| Verb::of(d) == verb);
        let mut squares = current
            .as_ref()
            .map_or_else(Vec::new, |d| d.squares().to_vec());
        if squares.contains(&tile) || squares.len() >= MAX_SQUARES {
            return false;
        }
        squares.push(tile);
        let order = self.with_squares(verb, squares, current.as_ref());
        if !self.fits(&order) {
            return false;
        }
        self.orders[usize::from(index)] = Some(order);
        if verb == Verb::Hold {
            self.turning = Some(index);
        }
        true
    }

    /// An order of this kind naming these squares. A HOLD keeps the way `current` faced, or a new
    /// one faces the way `first_facing` says from its first square.
    fn with_squares(
        &self,
        verb: Verb,
        squares: Vec<Tile>,
        current: Option<&Directive>,
    ) -> Directive {
        match verb {
            Verb::Hold => {
                let facing = match current {
                    Some(Directive::Hold { facing, .. }) => *facing,
                    _ => squares
                        .first()
                        .map_or(Facing::across(self.you), |t| self.first_facing(*t)),
                };
                Directive::Hold { squares, facing }
            }
            Verb::Trick => Directive::Trick { squares },
            _ => Directive::Press { squares },
        }
    }

    /// Picks an order to give: into the chosen place in the plan if it is empty or already of
    /// this kind, otherwise into the first empty one. False if all three are given. USE ITEM
    /// takes no place: it is for whoever of the team carries something (the only one, if there is
    /// one; picked up again, the next), and false if nobody does.
    pub fn take_token(&mut self, verb: Verb) -> bool {
        if verb == Verb::UseItem {
            let carriers = self.carriers();
            let user = match (self.verb, self.user) {
                (Some(Verb::UseItem), Some(by)) => carriers
                    .iter()
                    .copied()
                    .find(|c| *c > by)
                    .or(carriers.first().copied()),
                _ if carriers.len() == 1 => carriers.first().copied(),
                _ => None,
            };
            if carriers.is_empty() {
                return false;
            }
            self.verb = Some(verb);
            self.user = user;
            return true;
        }
        let fits_here = |i: u8| {
            self.orders[usize::from(i)]
                .as_ref()
                .is_none_or(|d| Verb::of(d) == verb)
        };
        let place = self
            .selected
            .filter(|i| fits_here(*i) && self.verb.is_some_and(|v| v == verb))
            .or_else(|| (0..PLAN_SIZE as u8).find(|i| self.orders[usize::from(*i)].is_none()));
        match place {
            Some(i) => {
                self.selected = Some(i);
                self.verb = Some(verb);
                self.user = None;
                true
            }
            None => false,
        }
    }

    /// USE ITEM in hand for this one of the team, by roster place. False unless it carries
    /// something.
    pub fn pick_user(&mut self, by: u8) -> bool {
        if !self.carriers().contains(&by) {
            return false;
        }
        self.verb = Some(Verb::UseItem);
        self.user = Some(by);
        true
    }

    /// What the board marks: with INTERACT picked, everything there is to interact with; with
    /// USE ITEM, whoever carries something, and every tile in reach of the one chosen.
    pub fn choices(&self) -> Vec<(Tile, Choice)> {
        let Some(m) = &self.m else {
            return Vec::new();
        };
        let mut marks: Vec<(Tile, Choice)> = Vec::new();
        match self.verb {
            Some(Verb::Interact) => {
                let named: Vec<Tile> = self
                    .orders
                    .iter()
                    .flatten()
                    .flat_map(|d| d.squares().to_vec())
                    .collect();
                for t in m.arena.tiles() {
                    if !named.contains(&t) && self.fits(&Directive::Interact { at: t }) {
                        marks.push((t, Choice::Interact));
                    }
                }
            }
            Some(Verb::UseItem) => {
                if let Some(by) = self.user {
                    for t in m.arena.tiles() {
                        if self.fits(&Directive::UseItem { by, at: t }) {
                            marks.push((t, Choice::Reach));
                        }
                    }
                }
                for c in self.carriers() {
                    if self.user != Some(c) {
                        marks.push((m.fighter(Slot::new(self.you, c)).at, Choice::Carrier));
                    }
                }
            }
            _ => {}
        }
        marks
    }

    /// What a click on `tile` would do while giving orders: a plain click (`hold` false) or a
    /// right click (`hold` true). With PRESS, HOLD or TRICK picked, it adds the square to the
    /// chosen order, or takes it out again; with INTERACT picked, it sets the chosen order to
    /// that. With USE ITEM picked, a tile in reach of the one chosen is where it uses what it
    /// carries, and another of the team carrying something is chosen instead. With none picked, a
    /// plain click interacts with whatever is there to interact with and otherwise presses the
    /// square, and a right click holds it.
    pub fn aim(&self, tile: Tile, hold: bool) -> Aim {
        let Some(m) = &self.m else {
            return Aim::Nothing;
        };
        if self.shown.is_some() || m.outcome().is_some() {
            return Aim::Nothing;
        }
        if self.verb == Some(Verb::UseItem) {
            if hold {
                return Aim::Nothing;
            }
            if let Some(by) = self.user {
                let order = Directive::UseItem { by, at: tile };
                if self.fits(&order) {
                    return if self.uses[usize::from(by)] == Some(tile) {
                        Aim::Nothing
                    } else {
                        Aim::Give(order)
                    };
                }
            }
            return match self
                .carriers()
                .into_iter()
                .find(|c| m.fighter(Slot::new(self.you, *c)).at == tile)
            {
                Some(by) if self.user != Some(by) => Aim::Carrier(by),
                _ => Aim::Nothing,
            };
        }
        let Some(index) = self.selected else {
            return Aim::Nothing;
        };
        let current = self.orders[usize::from(index)].clone();
        let squares = |verb: Verb| -> Vec<Tile> {
            match &current {
                Some(d) if Verb::of(d) == verb => {
                    let mut squares = d.squares().to_vec();
                    match squares.iter().position(|t| *t == tile) {
                        Some(i) if squares.len() > 1 => {
                            squares.remove(i);
                        }
                        Some(_) => {}
                        None => squares.push(tile),
                    }
                    squares
                }
                _ => vec![tile],
            }
        };
        let interact = Directive::Interact { at: tile };
        let order = match (self.verb, hold) {
            (Some(verb), _) if verb.names_squares() => {
                let same = current.as_ref().filter(|d| Verb::of(d) == verb);
                self.with_squares(verb, squares(verb), same)
            }
            (Some(_), _) => interact,
            (None, true) => Directive::Hold {
                squares: vec![tile],
                facing: self.first_facing(tile),
            },
            (None, false) if self.fits(&interact) => interact,
            (None, false) => Directive::Press {
                squares: vec![tile],
            },
        };
        if Some(&order) == current.as_ref() || !self.fits(&order) {
            return Aim::Nothing;
        }
        Aim::Give(order)
    }

    /// A click on a tile while giving orders, as `aim` says. A PRESS, HOLD or TRICK being built
    /// stays chosen for more squares; anything else moves on to the next order still empty. A
    /// USE ITEM given goes beside the three, and the order in hand is put down.
    pub fn press(&mut self, tile: Tile, hold: bool) -> Aim {
        let aim = self.aim(tile, hold);
        match &aim {
            Aim::Give(Directive::UseItem { by, at }) => {
                self.uses[usize::from(*by)] = Some(*at);
                self.put_down();
            }
            Aim::Give(order) => {
                let index = self
                    .selected
                    .expect("an order is aimed at the chosen place");
                self.orders[usize::from(index)] = Some(order.clone());
                if matches!(order, Directive::Hold { .. }) {
                    self.turning = Some(index);
                }
                if !self.verb.is_some_and(Verb::names_squares) {
                    self.put_down();
                    self.selected = (0..PLAN_SIZE as u8)
                        .find(|i| self.orders[usize::from(*i)].is_none())
                        .or(Some(index));
                }
            }
            Aim::Carrier(by) => self.user = Some(*by),
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
        if let Some(order) = self.orders.get_mut(usize::from(index)) {
            *order = None;
        }
        if self.turning == Some(index) {
            self.turning = None;
        }
    }

    /// Takes back the USE ITEM given to this one of the team, by roster place.
    pub fn clear_use(&mut self, by: u8) {
        if let Some(at) = self.uses.get_mut(usize::from(by)) {
            *at = None;
        }
    }

    /// What the board shows over itself just now.
    pub fn marks(&self, hover: Option<Tile>, now: f32) -> Marks {
        let mut marks = Marks {
            selected: None,
            choices: self.choices(),
            plan: if self.shown.is_none() {
                self.plan_marks()
            } else {
                Vec::new()
            },
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
            None => {}
        }
        marks
    }

    /// Locks the Handler's orders: against the AI, resolves the turn and starts showing it;
    /// against a friend, sends them to the room, which resolves the turn when both are in.
    pub fn lock(&mut self, now: f32) {
        let ours = self.plan();
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
            friend.line.ask(Ask::Orders { turn, orders: ours });
            friend.sent_turn = Some(turn);
            self.selected = None;
            self.put_down();
            return;
        }
        let away = Coach::Exhibition.plan(m, self.you.other());
        let plans = match self.you {
            Side::Left => [ours, away],
            Side::Right => [away, ours],
        };
        let before = m.board.clone();
        let report = m.resolve_turn(plans);
        self.show_turn(before, report, now);
    }

    /// Plays `turns` turns with the AI coach giving the Handler's orders too, each shown through
    /// at once: for review pictures of a match already under way.
    pub fn play_ahead(&mut self, turns: u8) {
        for _ in 0..turns {
            self.coach_orders();
            self.lock(0.0);
            if let Some(shown) = &mut self.shown {
                shown.started = -1.0e6;
            }
            self.frame(0.0);
        }
    }

    /// The Handler's orders, as the AI coach would give them: for review pictures.
    pub fn coach_orders(&mut self) {
        if let Some(m) = &self.m {
            let plan = Coach::Exhibition.plan(m, self.you);
            let mut tactical = plan.iter().filter(|d| d.tactical()).cloned();
            self.orders = [(); PLAN_SIZE].map(|_| tactical.next());
            self.uses = [None; TEAM_SIZE];
            for d in &plan {
                if let Directive::UseItem { by, at } = d
                    && let Some(using) = self.uses.get_mut(usize::from(*by))
                {
                    *using = Some(*at);
                }
            }
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
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_handler_name_from_the_relay_is_cleaned_before_it_is_shown_or_kept() {
        assert_eq!(handler_name("  Sam\u{202e}\u{0007}  "), "Sam");
        assert_eq!(handler_name("\u{200b}"), "A Handler");
        assert_eq!(handler_name(&"x".repeat(80)).chars().count(), 24);
    }

    use super::*;
    use formiga_battle_contract::sample;

    fn table() -> Table {
        let mut t = Table::new(sample::snapshot(), sample::visitors(9), 5);
        t.start(0.0).unwrap();
        t
    }

    fn yard(t: &Table) -> Tile {
        t.m.as_ref().unwrap().arena.zone_centre()
    }

    #[test]
    fn a_handler_builds_a_team_order_square_by_square() {
        let mut t = table();
        assert_eq!(t.selected, Some(0));
        t.choose_verb(Verb::Press);
        let centre = yard(&t);
        let next = Tile::new(centre.x - 1, centre.y);
        assert!(t.click(centre));
        assert!(t.click(next));
        assert_eq!(
            t.orders[0],
            Some(Directive::Press {
                squares: vec![centre, next]
            })
        );
        assert_eq!(t.selected, Some(0), "a PRESS stays chosen for more squares");
        assert!(t.click(next), "a square clicked again comes out");
        assert_eq!(
            t.orders[0],
            Some(Directive::Press {
                squares: vec![centre]
            })
        );
        assert!(
            t.plan_marks()
                .iter()
                .any(|m| m.tile == centre && m.kind == PlanKind::Press && m.order == 0)
        );
    }

    #[test]
    fn dragging_adds_squares_to_the_order_in_hand_and_never_takes_one_out() {
        let mut t = table();
        let zone = t.arena.zone.clone();
        assert!(t.take_token(Verb::Hold));
        assert!(t.add_square(zone[0]));
        assert!(t.add_square(zone[1]));
        assert!(!t.add_square(zone[1]), "a square already named stays");
        assert_eq!(
            t.orders[0],
            Some(Directive::Hold {
                squares: vec![zone[0], zone[1]],
                facing: t.first_facing(zone[0]),
            })
        );
    }

    #[test]
    fn a_trick_names_squares_as_a_press_does() {
        let mut t = table();
        let zone = t.arena.zone.clone();
        assert!(t.take_token(Verb::Trick));
        assert!(t.click(zone[0]));
        assert!(t.add_square(zone[1]));
        assert_eq!(
            t.orders[0],
            Some(Directive::Trick {
                squares: vec![zone[0], zone[1]]
            })
        );
        assert!(t.plan_marks().iter().all(|m| m.kind == PlanKind::Trick));
    }

    #[test]
    fn a_hold_faces_their_nearest_and_turns_a_quarter_at_a_time() {
        let mut t = table();
        let centre = yard(&t);
        let first = t.first_facing(centre);
        let m = t.m.as_ref().unwrap();
        let nearest = m
            .team(Side::Right)
            .map(|s| m.fighter(s).at)
            .min_by_key(|at| (centre.distance(*at), at.y, at.x))
            .unwrap();
        assert_eq!(Some(first), Facing::toward(centre, nearest));
        assert!(t.take_token(Verb::Hold));
        assert!(t.click(centre));
        let facing = |t: &Table| match &t.orders[0] {
            Some(Directive::Hold { facing, .. }) => *facing,
            other => panic!("{other:?}"),
        };
        assert_eq!(facing(&t), first);
        assert!(t.turn_hold(false));
        assert_eq!(facing(&t), first.turned());
        // More squares keep the way it faces.
        assert!(t.add_square(Tile::new(centre.x - 1, centre.y)));
        assert_eq!(facing(&t), first.turned());
        assert!(t.turn_hold(true));
        assert_eq!(facing(&t), first);
        // The facing is on the board's marks.
        assert!(
            t.plan_marks()
                .iter()
                .all(|m| m.kind == PlanKind::Hold && m.facing == Some(first))
        );
        // Put down and given elsewhere, the R key still turns the one last given.
        t.put_down();
        t.selected = Some(1);
        assert!(t.turn_hold(false));
        assert_eq!(facing(&t), first.turned());
        t.clear_order(0);
        assert!(!t.turn_hold(false), "nothing left to turn");
    }

    #[test]
    fn a_square_named_by_two_orders_is_marked_twice_stacked() {
        let mut t = table();
        let centre = yard(&t);
        t.take_token(Verb::Press);
        t.press(centre, false);
        t.verb = None;
        t.take_token(Verb::Hold);
        t.press(centre, false);
        let marks: Vec<_> = t
            .plan_marks()
            .into_iter()
            .filter(|m| m.tile == centre)
            .collect();
        assert_eq!(marks.len(), 2);
        assert_eq!(marks[1].stack, 1);
        assert_ne!(marks[0].order, marks[1].order);
    }

    #[test]
    fn with_all_three_orders_given_no_more_can_be_picked() {
        let mut t = table();
        let zone = t.arena.zone.clone();
        for (i, tile) in zone.iter().take(3).enumerate() {
            assert!(t.take_token(Verb::Hold), "order {i}");
            t.press(*tile, false);
            t.verb = None;
        }
        t.selected = None;
        assert!(!t.take_token(Verb::Press));
    }

    #[test]
    fn with_no_order_picked_a_click_presses_a_square_and_a_right_click_holds_it() {
        let mut t = table();
        let centre = yard(&t);
        assert_eq!(
            t.press(centre, false),
            Aim::Give(Directive::Press {
                squares: vec![centre]
            })
        );
        assert_eq!(t.selected, Some(1), "on to the next order still empty");
        assert_eq!(
            t.press(centre, true),
            Aim::Give(Directive::Hold {
                squares: vec![centre],
                facing: t.first_facing(centre),
            })
        );
        assert_eq!(t.selected, Some(2));
        assert_eq!(t.plan().len(), 2);
        assert_eq!(t.turnable(), Some(1), "the HOLD just given turns");
    }

    #[test]
    fn use_item_is_a_bonus_for_whoever_carries_something() {
        let mut t = table();
        assert!(!t.take_token(Verb::UseItem), "nobody carries anything");
        let carrier = Slot::new(Side::Left, 1);
        let at = {
            let m = t.m.as_mut().unwrap();
            m.board.fighters[0][1].carrying = Some(formiga_battle_core::Item::Bell);
            m.fighter(carrier).at
        };
        // Three orders given: USE ITEM still goes beside them.
        let zone = t.arena.zone.clone();
        for tile in zone.iter().take(3) {
            t.take_token(Verb::Hold);
            t.press(*tile, false);
            t.verb = None;
        }
        assert!(t.take_token(Verb::UseItem));
        assert_eq!(t.user, Some(1), "the only one carrying something is chosen");
        assert!(
            t.choices()
                .iter()
                .any(|(tile, c)| *tile == at && *c == Choice::Reach)
        );
        let far = Tile::new(at.x + 4, at.y);
        assert_eq!(t.aim(far, false), Aim::Nothing, "out of the bell's reach");
        assert_eq!(
            t.press(at, false),
            Aim::Give(Directive::UseItem { by: 1, at })
        );
        assert_eq!(t.verb, None);
        let plan = t.plan();
        assert_eq!(plan.iter().filter(|d| d.tactical()).count(), PLAN_SIZE);
        assert!(plan.contains(&Directive::UseItem { by: 1, at }));
        assert!(
            t.plan_marks()
                .iter()
                .any(|m| m.kind == PlanKind::UseItem && m.tile == at && m.from == Some(at))
        );
        // One for each carrier: given again, it moves rather than adds.
        let near = Tile::new(at.x + 1, at.y);
        assert!(t.take_token(Verb::UseItem));
        t.press(near, false);
        assert_eq!(
            t.plan()
                .iter()
                .filter(|d| matches!(d, Directive::UseItem { .. }))
                .count(),
            1
        );
        t.clear_use(1);
        assert!(!t.plan().iter().any(|d| !d.tactical()));
    }

    #[test]
    fn with_two_carrying_something_use_item_asks_which() {
        let mut t = table();
        let at = {
            let m = t.m.as_mut().unwrap();
            m.board.fighters[0][0].carrying = Some(formiga_battle_core::Item::Bell);
            m.board.fighters[0][2].carrying = Some(formiga_battle_core::Item::Bell);
            m.fighter(Slot::new(Side::Left, 2)).at
        };
        assert!(t.take_token(Verb::UseItem));
        assert_eq!(t.user, None);
        assert_eq!(t.press(at, false), Aim::Carrier(2));
        assert_eq!(t.user, Some(2));
        assert!(t.take_token(Verb::UseItem), "picked up again, the next");
        assert_eq!(t.user, Some(0));
    }

    #[test]
    fn interact_needs_something_there_to_interact_with() {
        let mut t = table();
        t.choose_verb(Verb::Interact);
        let centre = yard(&t);
        assert_eq!(t.aim(centre, false), Aim::Nothing);
        let lever = t.m.as_ref().unwrap().arena.levers[0];
        assert_eq!(
            t.aim(lever, false),
            Aim::Give(Directive::Interact { at: lever })
        );
    }

    #[test]
    fn a_cleared_order_comes_out_of_the_plan() {
        let mut t = table();
        let centre = yard(&t);
        t.press(centre, false);
        t.clear_order(0);
        assert!(t.plan().is_empty());
    }

    #[test]
    fn nothing_can_be_aimed_while_a_turn_plays_out() {
        let mut t = table();
        let centre = yard(&t);
        t.lock(0.0);
        assert_eq!(t.aim(centre, false), Aim::Nothing);
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
        assert_eq!(t.orders, [None, None, None]);
        assert_eq!(t.log.len(), 1);
        assert_eq!(t.m.as_ref().unwrap().board.turn, 1);
    }

    #[test]
    fn while_orders_are_given_the_team_says_who_will_probably_go_where_and_nothing_of_theirs() {
        let mut t = table();
        assert!(t.likely().is_empty(), "no plan, nothing to say");
        t.coach_orders();
        let likely = t.likely();
        assert!(!likely.is_empty());
        assert!(likely.iter().all(|(slot, _, _)| slot.side == t.you));
        // Locked, it is the turn's own business.
        t.lock(0.0);
        assert!(t.likely().is_empty());
        // And what came of each order is told once the turn has been played.
        assert!(!t.told.is_empty());
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
    fn a_trio_named_before_a_match_plays_it_and_desktop_is_told_both() {
        let dir = std::env::temp_dir().join(format!("formiga-battle-trio-{}", std::process::id()));
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
        // Signing a name for a friend makes it the Handler's.
        assert!(t.record.handler.is_none());
        let card = crate::identity::card_with_name(&t.record, "Jo").unwrap();
        t.set_handler(card);
        let ids: Vec<u64> = t
            .picked
            .iter()
            .map(|&i| t.home[i].0.traveler.id.0)
            .collect();
        t.set_trio(crate::identity::named("The Thistle Three", &ids).unwrap());
        let read = |dir: &std::path::Path| {
            BattleResult::decode(
                &std::fs::read(dir.join(formiga_battle_contract::RESULT_FILE)).unwrap(),
            )
            .unwrap()
        };
        let named = read(&dir);
        assert!(
            named.matches.is_empty(),
            "nothing played yet, and still written"
        );
        let registry = named.registry.expect("the card and trio, for Desktop");
        assert_eq!(registry.handler.unwrap().name, "Jo");
        assert_eq!(registry.trios[0].name, "The Thistle Three");
        let card = t.shared_card().expect("a card for a friend");
        assert_eq!(card.trio.as_deref(), Some("The Thistle Three"));
        assert_eq!(card.standing.as_deref(), Some("New Handler"));

        t.start(0.0).unwrap();
        let mut now = 0.0;
        while !t.finished(now) {
            t.coach_orders();
            t.lock(now);
            now += 20.0;
            t.frame(now);
        }
        assert!(
            t.earned
                .iter()
                .any(|l| l.starts_with("The Thistle Three: 1 played"))
        );
        assert_eq!(t.record.trios[0].visitors.played, 1);
        let written = read(&dir);
        assert_eq!(
            written.matches[0].rewards.trio.as_ref().unwrap().id,
            t.record.trios[0].id
        );
        assert!(written.registry.is_some());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn two_tables_play_a_whole_match_through_a_relay() {
        // A relay named in FORMIGA_BATTLE_RELAY, such as the site's, or one of our own.
        let named = std::env::var(formiga_battle_room::RELAY_ENV)
            .ok()
            .filter(|v| !v.trim().is_empty());
        let local = match named {
            Some(_) => None,
            None => Some(
                formiga_battle_room::serve_relay(
                    "127.0.0.1:0",
                    formiga_battle_room::Limits::default(),
                )
                .unwrap(),
            ),
        };
        let base = named.unwrap_or_else(|| local.as_ref().unwrap().base());
        let mut host = Table::new(sample::snapshot(), sample::visitors(9), 1);
        let mut guest = Table::new(sample::visitors(11), sample::visitors(9), 2);
        host.relay = base.clone();
        guest.relay = base;
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
        // Joining fingerprints the program too, so this wait is also by the wall clock.
        let joined = std::time::Instant::now();
        let mut began = [false, false];
        while !(began[0] && began[1]) {
            began[0] |= host.poll(now);
            began[1] |= guest.poll(now);
            std::thread::sleep(std::time::Duration::from_millis(20));
            now += 0.02;
            assert!(
                joined.elapsed().as_secs() < 60,
                "both see the match begin: {:?} {:?}",
                host.notice,
                guest.notice
            );
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
        if let Some(relay) = local {
            relay.stop();
        }
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
