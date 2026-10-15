//! One sitting at the board, without the window: the two rosters, the match, how everyone looks
//! and faces, the orders the Handler has given so far, and the turn being shown. The window only
//! asks this what to show and tells it what was clicked, so tests drive the same code.

use crate::look::Look;
use crate::playback::{Frame, Playback};
use crate::scene::{Choice, Marks};
use crate::store::{self, Notebook};
use formiga_battle_contract::{BattleSnapshot, Contender};
use formiga_battle_core::Event;
use formiga_battle_core::ai::{self, Coach};
use formiga_battle_core::interpret::distances;
use formiga_battle_core::notebook::{Page, observe};
use formiga_battle_core::recap::recap_naming;
use formiga_battle_core::rng::{self, Purpose};
use formiga_battle_core::{
    Arena, Match, Order, Orders, Outcome, Profile, Ruleset, Side, Slot, TEAM_SIZE, Target, Tile,
    TurnReport,
};
use std::path::PathBuf;

/// The Handler always plays the left team; the AI the right.
pub const YOU: Side = Side::Left;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Move,
    Hold,
    Contest,
}

impl Verb {
    pub const ALL: [Self; 3] = [Self::Move, Self::Hold, Self::Contest];

    pub const fn word(self) -> &'static str {
        match self {
            Self::Move => "MOVE",
            Self::Hold => "HOLD",
            Self::Contest => "CONTEST",
        }
    }

    pub const fn hint(self) -> &'static str {
        match self {
            Self::Move => "Reach a tile. Walks round anyone holding; open to a contest.",
            Self::Hold => "Defend a tile. Turns a contest away; holds the beacon.",
            Self::Contest => "Disrupt someone not holding, or pull the lever.",
        }
    }
}

pub struct Shown {
    pub playback: Playback,
    pub report: TurnReport,
    /// When it began showing, on the window's clock.
    pub started: f32,
}

pub struct Table {
    /// The Handler's companions, each with the profile Desktop's record reads as.
    pub home: Vec<(Contender, Profile)>,
    /// The visiting colony's.
    pub away: Vec<(Contender, Profile)>,
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
    /// The colony the Handler's companions come from, so their notebook pages are theirs.
    pub colony_id: String,
    pub notebook: Notebook,
    /// Where the notebook is kept; `None` keeps it only for this sitting, as review pictures do.
    pub notebook_folder: Option<PathBuf>,
    /// Every event of the match so far, for the notebook at its end.
    match_events: Vec<Event>,
    /// What this match added to the notebook, by name: shown with the result.
    pub new_moments: Vec<(String, String)>,
}

impl Table {
    /// A table for the Handler's roster against a visiting colony. A colony of fewer than three is
    /// joined by friends from next door so it can field a team.
    pub fn new(roster: BattleSnapshot, visitors: BattleSnapshot, seed: u64) -> Self {
        let colony_id = roster.colony_id.clone();
        let reduce_motion = roster.presentation.reduce_motion;
        let pair = |s: &BattleSnapshot| -> Vec<(Contender, Profile)> {
            s.contenders
                .iter()
                .cloned()
                .zip(crate::roster::profiles(s))
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
            colony_id,
            notebook: Notebook::default(),
            notebook_folder: None,
            match_events: Vec::new(),
            new_moments: Vec::new(),
        }
    }

    /// Keeps the notebook in `folder` from now on, starting from what is already there.
    pub fn keep_notebook_in(&mut self, folder: PathBuf) {
        self.notebook = store::load(&folder);
        self.notebook_folder = Some(folder);
    }

    /// A companion's page, if it has played.
    pub fn page(&self, creature_id: u64) -> Option<&Page> {
        self.notebook
            .page(&self.colony_id, creature_id)
            .filter(|p| p.matches > 0)
    }

    /// Writes what the match showed of the Handler's team into the notebook.
    fn note_the_match(&mut self) {
        let Some(m) = &self.m else {
            return;
        };
        let won = m.outcome().and_then(|o| o.winner) == Some(YOU);
        let team: Vec<(u64, String)> = m.rosters[YOU.index()]
            .iter()
            .map(|p| (p.creature_id, p.name.clone()))
            .collect();
        self.new_moments.clear();
        for seen in observe(m, YOU, &self.match_events) {
            let name = team
                .iter()
                .find(|(id, _)| *id == seen.creature_id)
                .map(|(_, n)| n.clone())
                .unwrap_or_default();
            for moment in &seen.moments {
                self.new_moments.push((name.clone(), moment.clone()));
            }
            self.notebook
                .page_mut(&self.colony_id, seen.creature_id)
                .record(&seen, won, &name, &team);
        }
        if let Some(folder) = &self.notebook_folder
            && let Err(error) = store::save(folder, &self.notebook)
        {
            eprintln!("formiga-battle: the notebook could not be kept: {error:#}");
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
        self.teams = [self.picked.clone(), away];
        self.facing = [[true; TEAM_SIZE], [false; TEAM_SIZE]];
        self.m = Some(m);
        self.begin_orders(now);
        self.log.clear();
        self.match_events.clear();
        self.new_moments.clear();
        Ok(())
    }

    fn begin_orders(&mut self, now: f32) {
        self.orders = [None; TEAM_SIZE];
        self.verb = None;
        self.turn_began = now;
        self.selected = self
            .m
            .as_ref()
            .and_then(|m| m.team(YOU).next().map(|s| s.index));
    }

    pub fn outcome(&self) -> Option<Outcome> {
        self.m.as_ref().and_then(Match::outcome)
    }

    /// Seconds left on the timer.
    pub fn time_left(&self, now: f32) -> f32 {
        (f32::from(self.ruleset.turn_seconds) - (now - self.turn_began)).max(0.0)
    }

    /// Chooses a creature of the Handler's to give an order to.
    pub fn select(&mut self, index: u8) {
        if self
            .m
            .as_ref()
            .is_some_and(|m| m.standing(Slot::new(YOU, index)))
        {
            self.selected = Some(index);
            self.verb = None;
        }
    }

    pub fn choose_verb(&mut self, verb: Verb) {
        if self.selected.is_some() {
            self.verb = Some(verb);
        }
    }

    /// The tiles the chosen order could be aimed at, as the board marks them.
    pub fn choices(&self) -> Vec<(Tile, Choice)> {
        let (Some(m), Some(index), Some(verb)) = (&self.m, self.selected, self.verb) else {
            return Vec::new();
        };
        let slot = Slot::new(YOU, index);
        match verb {
            Verb::Move | Verb::Hold => {
                let from = m.fighter(slot).at;
                let dist = distances(m, slot, &[from], false);
                let w = m.arena.width as usize;
                let choice = if verb == Verb::Move {
                    Choice::Walk
                } else {
                    Choice::Hold
                };
                m.arena
                    .tiles()
                    .filter(|t| m.walkable(*t))
                    .filter(|t| {
                        dist[t.y as usize * w + t.x as usize]
                            .is_some_and(|d| d <= m.ruleset.ticks_per_turn)
                    })
                    .map(|t| (t, choice))
                    .collect()
            }
            Verb::Contest => m
                .team(YOU.other())
                .map(|e| (m.fighter(e).at, Choice::Contest))
                .chain(m.arena.levers.iter().map(|l| (*l, Choice::Contest)))
                .collect(),
        }
    }

    /// A click on a tile, while giving orders: aims the chosen order there, or chooses whoever of
    /// the Handler's stands there. True if it changed anything.
    pub fn click(&mut self, tile: Tile) -> bool {
        let Some(m) = &self.m else {
            return false;
        };
        let own = m.occupant(tile).filter(|s| s.side == YOU);
        let (Some(index), Some(verb)) = (self.selected, self.verb) else {
            if let Some(s) = own {
                self.select(s.index);
                return true;
            }
            return false;
        };
        let slot = Slot::new(YOU, index);
        let order = match verb {
            Verb::Move => Order::Move { to: tile },
            Verb::Hold => Order::Hold { at: tile },
            Verb::Contest => {
                if let Some(enemy) = m.occupant(tile).filter(|s| s.side != YOU) {
                    Order::Contest {
                        target: Target::Creature { slot: enemy },
                    }
                } else if m.arena.levers.contains(&tile) {
                    Order::Contest {
                        target: Target::Lever { at: tile },
                    }
                } else {
                    if let Some(s) = own {
                        self.select(s.index);
                        return true;
                    }
                    return false;
                }
            }
        };
        if !m.valid(slot, &order) {
            return false;
        }
        self.orders[index as usize] = Some(order);
        self.verb = None;
        // On to the next creature still without an order.
        self.selected = m
            .team(YOU)
            .map(|s| s.index)
            .find(|i| self.orders[*i as usize].is_none())
            .or(Some(index));
        true
    }

    pub fn clear_order(&mut self, index: u8) {
        self.orders[index as usize] = None;
    }

    /// What the board shows over itself just now.
    pub fn marks(&self, hover: Option<Tile>, now: f32) -> Marks {
        let mut marks = Marks {
            selected: self.selected.map(|i| Slot::new(YOU, i)),
            choices: self.choices(),
            hover,
            orders: Vec::new(),
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
                        marks.orders.push((Slot::new(YOU, i as u8), *order));
                    }
                }
            }
        }
        marks
    }

    /// Locks the Handler's orders with the AI's and starts showing the turn.
    pub fn lock(&mut self, now: f32) {
        let Some(m) = &mut self.m else {
            return;
        };
        if self.shown.is_some() || m.outcome().is_some() {
            return;
        }
        let away = Coach::Exhibition.orders(m, YOU.other());
        let before = m.board.clone();
        let report = m.resolve_turn([self.orders, away]);
        let playback = crate::playback::Playback::new(
            &before,
            &m.arena.gates,
            &self.facing,
            &report.events,
            m.ruleset.ticks_per_turn,
        );
        self.log.push((
            report.turn,
            recap_naming(m, &report.events, ["Your team", "The visiting team"]),
        ));
        self.match_events.extend(report.events.iter().cloned());
        let over = m.outcome().is_some();
        self.shown = Some(Shown {
            playback,
            report,
            started: now,
        });
        self.selected = None;
        self.verb = None;
        if over {
            self.note_the_match();
        }
    }

    /// The Handler's orders, as the AI coach would give them: for review pictures.
    pub fn coach_orders(&mut self) {
        if let Some(m) = &self.m {
            self.orders = Coach::Exhibition.orders(m, YOU);
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
            .map(|m| ai::fallback(m, Slot::new(YOU, index)))
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
        let teammate = m.fighter(Slot::new(YOU, 1)).at;
        let enemy = m.fighter(Slot::new(YOU.other(), 0)).at;
        assert!(t.click(teammate), "clicking a teammate chooses it instead");
        assert_eq!(t.selected, Some(1));
        assert_eq!(t.orders[0], None);
        t.choose_verb(Verb::Contest);
        assert!(t.click(enemy));
        assert!(matches!(t.orders[1], Some(Order::Contest { .. })));
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
    fn a_finished_match_is_written_into_the_notebook_and_kept() {
        let folder =
            std::env::temp_dir().join(format!("formiga-battle-table-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let mut t = Table::new(sample::snapshot(), sample::visitors(9), 5);
        t.keep_notebook_in(folder.clone());
        t.start(0.0).unwrap();
        let mut now = 0.0;
        while !t.finished(now) {
            t.coach_orders();
            t.lock(now);
            now += 20.0;
            t.frame(now);
        }
        let ours: Vec<u64> = t.m.as_ref().unwrap().rosters[0]
            .iter()
            .map(|p| p.creature_id)
            .collect();
        for id in &ours {
            assert_eq!(t.page(*id).map(|p| p.matches), Some(1));
        }
        let kept = store::load(&folder);
        assert_eq!(kept, t.notebook);
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
