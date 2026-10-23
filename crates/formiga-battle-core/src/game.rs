//! A match: the board, and how one turn of simultaneous orders resolves on it.
//!
//! Both teams' plans are locked together and each team shares its own out among its creatures
//! (`muster`), then the turn plays out in a few ticks. Every tick
//! works from one shared picture of the board: everyone decides their step from where everyone
//! stood, every step happens at once, and then every press, trick and interaction is settled from where everyone
//! ended up. Nobody moves first.
//!
//! A tick, in order:
//!
//! 1. A gate whose swing was telegraphed last tick swings. Then whatever was marked to land this
//!    tick lands: things found on the board turn up, items used last tick take effect, the map
//!    event comes (`scenery`). Anyone whose job has just come to nothing thinks again, once a
//!    turn, from its team's plan (`muster`).
//!    On the first tick, every USE ITEM given for the turn is used, before anything else settles.
//! 2. PRESSes, TRICKs and INTERACTs already next to what they are aimed at are settled (see
//!    `engage`), so someone walking away from a presser next to it is caught before it steps.
//! 3. Each creature still carrying out its order picks one step (`interpret`).
//! 4. The steps happen together. Two wanting one tile: a fair draw lets one in. Two wanting to
//!    swap, or a step into a tile whose creature did not leave: they stay where they were. Nobody
//!    is hurt by a bump. Anyone with empty paws who stepped onto something lying there picks it
//!    up.
//! 5. Anyone who has reached the tile it was told to hold starts holding.
//! 6. Those that have just come into reach are settled the same way. Composure is lost as each
//!    is settled, at most `Ruleset::damage_cap` from one creature in a tick; anyone at zero is
//!    knocked out.
//! 7. The beacon counts: a team holding in the zone with nobody from the other team in it gains a
//!    tick of progress, and a team that broke a hold in the zone by flanking it this tick gains
//!    one more (a breakthrough).
//! 8. The match ends by knockout, then by capture.

use crate::arena::{Arena, TEAM_SIZE, Terrain};
use crate::event::{Event, EventKind, Fill, Outcome, Victory};
use crate::grid::Tile;
use crate::items::Item;
use crate::muster::Task;
use crate::order::{Facing, Order, Orders, Quarter, Side, Slot, TeamPlan};
use crate::profile::{Behavior, Profile};
use crate::rng::{self, Purpose};
use crate::ruleset::{CaptureRule, RULES_VERSION, Ruleset};
use crate::scenery::Pending;
use crate::{ai, interpret};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// One creature on the board, and how far through this turn's order it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fighter {
    pub at: Tile,
    pub composure: u8,
    /// Knocked out: off the board for the rest of this match.
    pub out: bool,
    /// This turn's order, once orders are locked.
    pub order: Option<Order>,
    /// Standing on the tile it was told to hold, holding it.
    pub holding: bool,
    /// Its PRESS, TRICK or INTERACT has been settled this turn, whatever came of it.
    pub spent: bool,
    /// Pressed, flanked, tricked or otherwise knocked off balance: it stays where it is for the
    /// rest of the turn and does not count in the zone.
    pub interrupted: bool,
    /// It reached the tile it was told to move to.
    pub arrived: bool,
    /// It has used its one follow-up step this turn.
    pub followed_up: bool,
    /// Steps that do not bring it closer it may still take this turn.
    pub detours_left: u8,
    /// It has waited a tick this turn rather than walk in among the other team; it does not
    /// wait twice in a turn.
    #[serde(default)]
    pub hung_back: bool,
    /// Its last step was bumped. Until it steps again, others plan round it as if it were
    /// standing still, and it plans round them, so two creatures meeting in a narrow way do not
    /// bump each other for ever.
    #[serde(default)]
    pub bumped: bool,
    /// Its PRESS this turn was turned away by a holder.
    #[serde(default)]
    pub turned_away: bool,
    /// The way it faces while it holds: set when it starts holding, and fixed for the turn.
    pub facing: Facing,
    /// The tile it was told to USE ITEM on this turn, until it does, at the start of the first
    /// tick.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub using: Option<Tile>,
    /// It does nothing more this tick, neither stepping nor settling anything: it is using what
    /// it carried, or has just hopped.
    #[serde(default, skip_serializing_if = "is_false")]
    pub pause: bool,
    /// A TRICK broke its hold: it stands where it is and holds nothing more this turn, but it
    /// is still on its feet, and still counts in the zone.
    #[serde(default, skip_serializing_if = "is_false")]
    pub broken: bool,
    /// What it picked up, if anything. One thing at a time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrying: Option<Item>,
    /// The last tick, counted from the start of the match, a leaf umbrella is open over it; 0 for
    /// none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub shelter_until: u16,
    /// The job it took on from its team's plan this turn, or last turn until the next is shared
    /// out; `None` for a creature given an order of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<Task>,
    /// It has thought again this turn; it does not twice.
    #[serde(default, skip_serializing_if = "is_false")]
    pub rethought: bool,
    /// It stepped into honey: it stays where it is for the rest of the turn, and can still hold.
    #[serde(default)]
    pub stuck: bool,
}

fn is_zero(n: &u16) -> bool {
    *n == 0
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Fighter {
    fn new(at: Tile, composure: u8, facing: Facing) -> Self {
        Self {
            at,
            composure,
            out: false,
            order: None,
            holding: false,
            spent: false,
            interrupted: false,
            arrived: false,
            followed_up: false,
            detours_left: 0,
            hung_back: false,
            bumped: false,
            turned_away: false,
            facing,
            using: None,
            pause: false,
            broken: false,
            carrying: None,
            shelter_until: 0,
            task: None,
            rethought: false,
            stuck: false,
        }
    }

    /// Under a leaf umbrella at this tick of the match.
    pub fn sheltered(&self, now: u16) -> bool {
        self.shelter_until >= now
    }

    /// Not walking anywhere this tick, as far as anyone planning a route can tell.
    pub fn stationary(&self) -> bool {
        self.holding
            || self.interrupted
            || self.stuck
            || self.bumped
            || self.pause
            || self.broken
            || self.order.is_none()
            || match self.order {
                Some(Order::Move { .. }) => self.arrived,
                Some(o) if o.settles() => self.spent,
                _ => false,
            }
    }
}

/// Everything about a match that changes as it is played. Its hash is what two machines compare.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    /// The turn being or last resolved; 0 before the first.
    pub turn: u8,
    pub fighters: [[Fighter; TEAM_SIZE]; 2],
    pub gates_open: Vec<bool>,
    /// The tick, counted from the start of the match, the gates swing at, once a lever is pulled.
    pub swing_at: Option<u16>,
    /// The first turn the lever can be pulled in.
    pub lever_ready_turn: u8,
    /// Each team's beacon progress, in ticks: left, then right.
    pub capture: [u8; 2],
    pub outcome: Option<Outcome>,
    /// The tick, counted from the start of the match, the map event lands at, once it is warned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_at: Option<u16>,
    /// The first turn a switch can set the map event off in.
    #[serde(default)]
    pub switch_ready_turn: u8,
    /// Things lying on the ground, to be picked up.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ground: Vec<(Tile, Item)>,
    /// Spots glinting: what turns up on them at the start of the next turn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub glint: Vec<(Tile, Item)>,
    /// Twigs wedged into tiles, each with the last tick it stands.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub twigs: Vec<(Tile, u16)>,
    /// Items used, marked on their tiles, landing next tick.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<Pending>,
    /// Where the map event lands, once it is warned.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_tiles: Vec<Tile>,
    /// The switches that set it off, when it lands round them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_from: Vec<Tile>,
    /// Ground gone under, each tile with the last tick nobody can stand there.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub closed: Vec<(Tile, u16)>,
    /// Twigs floating on water, each with the last tick anyone can stand on it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rafts: Vec<(Tile, u16)>,
    /// Sticky ground, each tile with the last tick it stays sticky.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub honey: Vec<(Tile, u16)>,
}

/// One turn as it was played: each team's plan as it was sent (empty for none in time), or each
/// creature's order for a turn set up order by order, and the board's hash after it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnRecord {
    pub submitted: [TeamPlan; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orders: Option<[Orders; 2]>,
    pub hash: u64,
}

/// What resolving one turn gave.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnReport {
    pub turn: u8,
    /// Each creature's order as its team shared out the plan, the fallback's included; `None`
    /// for a creature already out. One that thought again partway through says so in the events.
    pub orders: [Orders; 2],
    pub events: Vec<Event>,
    pub hash: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MatchError {
    #[error("each team needs exactly {TEAM_SIZE} creatures")]
    RosterSize,
    #[error("the same creature cannot be on the board twice")]
    Duplicate,
    #[error("the rules cannot be played: {0}")]
    Rules(#[from] crate::ruleset::RulesetError),
}

#[derive(Clone, Debug)]
pub struct Match {
    pub arena: Arena,
    pub ruleset: Ruleset,
    pub seed: u64,
    pub rosters: [[Profile; TEAM_SIZE]; 2],
    behaviors: [[Behavior; TEAM_SIZE]; 2],
    pub board: Board,
    pub history: Vec<TurnRecord>,
    /// Each team's plan for the turn being resolved, to think again from.
    pub(crate) plans: [TeamPlan; 2],
    /// The board as it was when the first turn began, kept so a match set up part way (a
    /// lesson, a test) can be recorded and replayed from where it started.
    pub(crate) start: Option<Board>,
}

impl Match {
    pub fn new(
        arena: Arena,
        ruleset: Ruleset,
        seed: u64,
        rosters: [Vec<Profile>; 2],
    ) -> Result<Self, MatchError> {
        ruleset.validate()?;
        let [left, right] = rosters;
        let left: [Profile; TEAM_SIZE] = left.try_into().map_err(|_| MatchError::RosterSize)?;
        let right: [Profile; TEAM_SIZE] = right.try_into().map_err(|_| MatchError::RosterSize)?;
        // A creature can meet a friend from another colony with the same id, but never itself.
        for team in [&left, &right] {
            let ids: BTreeSet<_> = team.iter().map(|p| p.creature_id).collect();
            if ids.len() != TEAM_SIZE {
                return Err(MatchError::Duplicate);
            }
        }
        let behaviors = [
            left.clone().map(|p| p.behavior()),
            right.clone().map(|p| p.behavior()),
        ];
        let board = Self::starting_board(&arena, &ruleset);
        Ok(Self {
            arena,
            ruleset,
            seed,
            rosters: [left, right],
            behaviors,
            board,
            history: Vec::new(),
            plans: [Vec::new(), Vec::new()],
            start: None,
        })
    }

    /// The board as every match on this arena begins: both teams on the arena's start tiles,
    /// facing across, full composure, the gates as the arena has them, nothing lying about.
    pub fn starting_board(arena: &Arena, ruleset: &Ruleset) -> Board {
        let fighters = Side::BOTH.map(|side| {
            arena.starts[side.index()]
                .map(|tile| Fighter::new(tile, ruleset.composure, Facing::across(side)))
        });
        Board {
            turn: 0,
            fighters,
            gates_open: arena.gates_open.clone(),
            swing_at: None,
            lever_ready_turn: 1,
            capture: [0, 0],
            outcome: None,
            event_at: None,
            switch_ready_turn: 0,
            ground: Vec::new(),
            glint: Vec::new(),
            twigs: Vec::new(),
            pending: Vec::new(),
            event_tiles: Vec::new(),
            event_from: Vec::new(),
            closed: Vec::new(),
            rafts: Vec::new(),
            honey: Vec::new(),
        }
    }

    pub fn fighter(&self, slot: Slot) -> &Fighter {
        &self.board.fighters[slot.side.index()][slot.index as usize]
    }

    pub(crate) fn fighter_mut(&mut self, slot: Slot) -> &mut Fighter {
        &mut self.board.fighters[slot.side.index()][slot.index as usize]
    }

    pub fn profile(&self, slot: Slot) -> &Profile {
        &self.rosters[slot.side.index()][slot.index as usize]
    }

    /// The same match, with one team's creatures taken to be these instead: what a coach who
    /// imagines them so expects to happen. The board is unchanged.
    pub fn imagining(&self, side: Side, profiles: [Profile; TEAM_SIZE]) -> Self {
        let mut m = self.clone();
        m.behaviors[side.index()] = profiles.clone().map(|p| p.behavior());
        m.rosters[side.index()] = profiles;
        m
    }

    pub fn behavior(&self, slot: Slot) -> Behavior {
        self.behaviors[slot.side.index()][slot.index as usize]
    }

    pub fn standing(&self, slot: Slot) -> bool {
        !self.fighter(slot).out
    }

    /// Every creature still on the board, left team first.
    pub fn standing_slots(&self) -> impl Iterator<Item = Slot> + '_ {
        Slot::all().filter(|s| self.standing(*s))
    }

    /// Every creature on a side, knocked out or not.
    pub fn team_all(&self, side: Side) -> impl Iterator<Item = Slot> + '_ {
        Slot::all().filter(move |s| s.side == side)
    }

    pub fn team(&self, side: Side) -> impl Iterator<Item = Slot> + '_ {
        self.standing_slots().filter(move |s| s.side == side)
    }

    pub fn occupant(&self, tile: Tile) -> Option<Slot> {
        self.standing_slots().find(|s| self.fighter(*s).at == tile)
    }

    /// Whether the ground lets anyone stand on it just now.
    pub fn walkable(&self, tile: Tile) -> bool {
        let on = |list: &[(Tile, u16)]| list.iter().any(|(t, _)| *t == tile);
        if on(&self.board.twigs) || on(&self.board.closed) {
            return false;
        }
        if on(&self.board.rafts) {
            return true;
        }
        match self.arena.terrain(tile) {
            Terrain::Open => true,
            Terrain::Gate => self
                .arena
                .gates
                .iter()
                .position(|g| *g == tile)
                .is_some_and(|i| self.board.gates_open[i]),
            Terrain::Obstacle | Terrain::Lever => false,
        }
    }

    /// Whether the lever can be pulled this turn.
    pub fn lever_ready(&self) -> bool {
        self.board.swing_at.is_none() && self.board.turn >= self.board.lever_ready_turn
    }

    pub fn outcome(&self) -> Option<Outcome> {
        self.board.outcome
    }

    /// Whether a twig thrown here floats as a raft: open water, not a switch standing in it.
    pub fn floats(&self, tile: Tile) -> bool {
        self.arena.water
            && self.arena.contains(tile)
            && self.arena.terrain(tile) == Terrain::Obstacle
            && !self.arena.switches.contains(&tile)
    }

    /// Whether this tile is sticky with honey.
    pub fn sticky(&self, tile: Tile) -> bool {
        self.board.honey.iter().any(|(t, _)| *t == tile)
    }

    /// Whether a switch can set the map event off this turn.
    pub fn switch_ready(&self) -> bool {
        self.arena.scenery.is_some()
            && self.board.event_at.is_none()
            && self.board.turn >= self.board.switch_ready_turn
    }

    /// The tick, counted from the start of the match.
    pub(crate) fn clock(&self, turn: u8, tick: u8) -> u16 {
        u16::from(turn) * u16::from(self.ruleset.ticks_per_turn) + u16::from(tick)
    }

    /// Names the board exactly.
    pub fn hash(&self) -> u64 {
        let bytes = serde_json::to_vec(&(RULES_VERSION, &self.board)).expect("a board serializes");
        rng::fnv1a(&bytes)
    }

    /// Whether an order can be carried out at all by this creature. One that cannot is treated
    /// as missing, so a room never has to trust what a client sends.
    pub fn valid(&self, slot: Slot, order: &Order) -> bool {
        let ground = |tile: Tile| {
            matches!(self.arena.terrain(tile), Terrain::Open | Terrain::Gate)
                && self.arena.contains(tile)
        };
        match *order {
            Order::Move { to } => ground(to),
            Order::Hold { at, .. } => ground(at),
            Order::Press { target } | Order::Trick { target } => {
                target.side != slot.side
                    && (target.index as usize) < TEAM_SIZE
                    && self.standing(target)
            }
            Order::Interact { at } => {
                self.arena.levers.contains(&at) || self.arena.switches.contains(&at)
            }
        }
    }

    /// Whether this creature can USE ITEM on `at` this turn: it carries something, and `at` is
    /// within the item's reach of where it stands now.
    pub fn can_use(&self, slot: Slot, at: Tile) -> bool {
        let ground = |tile: Tile| {
            matches!(self.arena.terrain(tile), Terrain::Open | Terrain::Gate)
                && self.arena.contains(tile)
        };
        let f = self.fighter(slot);
        !f.out
            && self.arena.contains(at)
            && match f.carrying {
                Some(item) if f.at.distance(at) <= item.reach() => match item {
                    Item::Twig => ground(at) || self.floats(at),
                    _ => true,
                },
                _ => false,
            }
    }

    /// Locks both teams' plans, has each team share its own out among its creatures, fills any
    /// order still missing with the fallback (the same for both teams), and plays the turn out.
    pub fn resolve_turn(&mut self, plans: [TeamPlan; 2]) -> TurnReport {
        self.resolve(plans, None)
    }

    /// Plays a turn out from each creature's order, given one by one rather than shared out from
    /// a plan: how the tests set up one rule at a time. Orders missing or that cannot be carried
    /// out are filled by the fallback.
    pub fn resolve_orders(&mut self, submitted: [Orders; 2]) -> TurnReport {
        self.resolve([Vec::new(), Vec::new()], Some(submitted))
    }

    /// The same, with each team's USE ITEMs taken from `plans`; anything else in them is
    /// ignored.
    pub fn resolve_orders_using(
        &mut self,
        submitted: [Orders; 2],
        plans: [TeamPlan; 2],
    ) -> TurnReport {
        self.resolve(plans, Some(submitted))
    }

    fn resolve(&mut self, plans: [TeamPlan; 2], given: Option<[Orders; 2]>) -> TurnReport {
        let mut events = Vec::new();
        if self.board.outcome.is_some() {
            return TurnReport {
                turn: self.board.turn,
                orders: [[None; TEAM_SIZE]; 2],
                events,
                hash: self.hash(),
            };
        }
        if self.history.is_empty() && self.start.is_none() {
            self.start = Some(self.board.clone());
        }
        self.board.turn += 1;
        let turn = self.board.turn;
        // Both teams share out their plans from the board as it stood when the timer ran out.
        let mut submitted: [Orders; 2] = given.unwrap_or([[None; TEAM_SIZE]; 2]);
        let mut tasks: [[Option<Task>; TEAM_SIZE]; 2] = [[None; TEAM_SIZE]; 2];
        let mut uses: [[Option<Tile>; TEAM_SIZE]; 2] = [[None; TEAM_SIZE]; 2];
        for side in Side::BOTH {
            let plan = &plans[side.index()];
            if given.is_none() {
                let (orders, assigned, said) = crate::muster::muster(self, side, plan, turn);
                submitted[side.index()] = orders;
                tasks[side.index()] = assigned;
                events.extend(said);
            }
            let (using, dropped) = crate::muster::uses(self, side, plan);
            uses[side.index()] = using;
            events.extend(dropped.into_iter().map(|index| Event {
                turn,
                tick: 0,
                kind: EventKind::OrderDropped { side, index },
            }));
        }
        self.plans = plans.clone();
        // Every fallback order is chosen from the board as it stood when the timer ran out.
        let mut orders: [Orders; 2] = [[None; TEAM_SIZE]; 2];
        for slot in Slot::all() {
            if !self.standing(slot) {
                continue;
            }
            let given = submitted[slot.side.index()][slot.index as usize];
            let order = match given {
                Some(order) if self.valid(slot, &order) => order,
                _ => {
                    let order = ai::fallback(self, slot);
                    let fill = if given.is_some() {
                        Fill::Invalid
                    } else {
                        Fill::Missing
                    };
                    events.push(Event {
                        turn,
                        tick: 0,
                        kind: EventKind::OrderFilled { slot, order, fill },
                    });
                    order
                }
            };
            orders[slot.side.index()][slot.index as usize] = Some(order);
        }
        let detours = self.ruleset.detour_budget;
        for slot in Slot::all() {
            let order = orders[slot.side.index()][slot.index as usize];
            let task = tasks[slot.side.index()][slot.index as usize];
            let using = uses[slot.side.index()][slot.index as usize];
            let f = self.fighter_mut(slot);
            f.order = order;
            f.task = task;
            f.using = using;
            f.pause = false;
            f.broken = false;
            f.rethought = false;
            f.spent = false;
            f.interrupted = false;
            f.arrived = false;
            f.followed_up = false;
            f.detours_left = detours;
            f.bumped = false;
            f.hung_back = false;
            f.turned_away = false;
            f.stuck = false;
            f.holding = !f.out && order.and_then(|o| o.hold_at()) == Some(f.at);
            if let (true, Some(Order::Hold { facing, .. })) = (f.holding, order) {
                f.facing = facing;
            }
            if f.holding {
                let at = f.at;
                events.push(Event {
                    turn,
                    tick: 0,
                    kind: EventKind::Holding { slot, at },
                });
            }
        }
        for tick in 1..=self.ruleset.ticks_per_turn {
            self.tick(turn, tick, &mut events);
            if self.board.outcome.is_some() {
                break;
            }
        }
        if self.board.outcome.is_none() {
            self.foretell(turn, &mut events);
        }
        if self.board.outcome.is_none() && turn >= self.ruleset.turn_cap {
            let outcome = self.decide_at_cap();
            self.board.outcome = Some(outcome);
            events.push(Event {
                turn,
                tick: self.ruleset.ticks_per_turn,
                kind: EventKind::Finished { outcome },
            });
        }
        let hash = self.hash();
        self.history.push(TurnRecord {
            submitted: plans,
            orders: given,
            hash,
        });
        TurnReport {
            turn,
            orders,
            events,
            hash,
        }
    }

    fn tick(&mut self, turn: u8, tick: u8, events: &mut Vec<Event>) {
        let now = self.clock(turn, tick);
        for slot in Slot::all() {
            self.fighter_mut(slot).pause = false;
        }
        let mut push = |kind| events.push(Event { turn, tick, kind });

        // 1. A telegraphed swing.
        if self.board.swing_at == Some(now) {
            self.board.swing_at = None;
            self.board.lever_ready_turn = turn
                .saturating_add(self.arena.lever.cooldown_turns)
                .saturating_add(1);
            for (i, gate) in self.arena.gates.clone().into_iter().enumerate() {
                let open = !self.board.gates_open[i];
                let wedged = self.board.twigs.iter().any(|(t, _)| *t == gate);
                match self.occupant(gate) {
                    Some(by) if !open => push(EventKind::GateJammed { gate, by }),
                    None if !open && wedged => push(EventKind::GateWedged { gate }),
                    _ => {
                        self.board.gates_open[i] = open;
                        push(EventKind::GateSwung { gate, open });
                    }
                }
            }
        }

        // Then whatever was marked to land this tick, and anyone whose job came to nothing
        // thinks again.
        self.land(turn, tick, now, events);
        self.rethink(turn, tick, events);

        // On the first tick, every USE ITEM: the user stands still for it.
        if tick == 1 {
            self.use_items(turn, tick, now, events);
        }

        // 2. Presses, tricks and interactions already in reach settle before anyone steps.
        let mut lost = [[0u8; TEAM_SIZE]; 2];
        self.settle(turn, tick, now, &mut lost, events);
        let mut push = |kind| events.push(Event { turn, tick, kind });

        // 3. Everyone picks a step from the same picture.
        let mut plans: Vec<(Slot, interpret::Plan)> = self
            .standing_slots()
            .filter_map(|slot| interpret::plan_step(self, slot, now).map(|plan| (slot, plan)))
            .collect();
        let hung: Vec<Slot> = plans
            .iter()
            .filter(|(slot, plan)| plan.to == self.fighter(*slot).at)
            .map(|(slot, _)| *slot)
            .collect();
        for slot in &hung {
            let at = self.fighter(*slot).at;
            self.fighter_mut(*slot).hung_back = true;
            push(EventKind::HungBack { slot: *slot, at });
        }
        plans.retain(|(slot, _)| !hung.contains(slot));
        for (slot, plan) in &plans {
            let f = self.fighter_mut(*slot);
            if plan.detour {
                f.detours_left = f.detours_left.saturating_sub(1);
            }
            if plan.follow_up {
                f.followed_up = true;
            }
        }

        // 4. Every step at once.
        let mut moving: Vec<(Slot, Tile)> = plans.iter().map(|(s, p)| (*s, p.to)).collect();
        let mut failed: BTreeSet<Slot> = BTreeSet::new();
        for (slot, to) in &moving {
            let from = self.fighter(*slot).at;
            // Two cannot pass through each other.
            let swapping = moving
                .iter()
                .any(|(other, other_to)| *other_to == from && self.fighter(*other).at == *to);
            // Two or more wanting one tile: a fair draw, the same for both teams, lets one in.
            let rivals: Vec<Slot> = moving
                .iter()
                .filter(|(_, other_to)| other_to == to)
                .map(|(other, _)| *other)
                .collect();
            let first = rivals.iter().copied().max_by_key(|r| {
                rng::roll(
                    self.seed,
                    Purpose::TieBreak,
                    &[u64::from(now), r.key(), to.x as u64, to.y as u64, 7],
                )
            });
            if swapping || first != Some(*slot) {
                failed.insert(*slot);
            }
        }
        loop {
            let blocked: Vec<Slot> = moving
                .iter()
                .filter(|(slot, to)| {
                    !failed.contains(slot)
                        && self.occupant(*to).is_some_and(|o| {
                            o != *slot
                                && (failed.contains(&o) || !moving.iter().any(|(m, _)| *m == o))
                        })
                })
                .map(|(slot, _)| *slot)
                .collect();
            if blocked.is_empty() {
                break;
            }
            failed.extend(blocked);
        }
        moving.sort_by_key(|(slot, _)| *slot);
        for (slot, to) in &moving {
            let reason = plans
                .iter()
                .find(|(s, _)| s == slot)
                .map(|(_, p)| p.reason)
                .expect("every step has a plan");
            self.fighter_mut(*slot).bumped = failed.contains(slot);
            if failed.contains(slot) {
                push(EventKind::Bumped {
                    slot: *slot,
                    wanted: *to,
                });
            } else {
                let from = self.fighter(*slot).at;
                self.fighter_mut(*slot).at = *to;
                push(EventKind::Stepped {
                    slot: *slot,
                    from,
                    to: *to,
                    reason,
                });
            }
        }
        for (slot, _) in &moving {
            if !failed.contains(slot) {
                self.pick_up(*slot, turn, tick, events);
                self.stick(*slot, turn, tick, events);
            }
        }
        let mut push = |kind| events.push(Event { turn, tick, kind });

        // 5. Arrivals.
        for slot in self.standing_slots().collect::<Vec<_>>() {
            let f = self.fighter(slot).clone();
            match f.order {
                Some(Order::Move { to }) if f.at == to && !f.arrived => {
                    self.fighter_mut(slot).arrived = true;
                }
                Some(Order::Hold { at, facing }) if !f.holding && !f.interrupted && !f.broken => {
                    // Someone already standing on the tile: hold beside it instead.
                    let short = f.at.touches(at) && self.occupant(at).is_some_and(|o| o != slot);
                    if f.at == at || short {
                        let f = self.fighter_mut(slot);
                        f.holding = true;
                        f.facing = facing;
                        push(EventKind::Holding { slot, at: f.at });
                    }
                }
                _ => {}
            }
        }

        // 6. Presses, tricks and interactions that have just come into reach.
        self.settle(turn, tick, now, &mut lost, events);
        // A hold in the zone broken by a flank this tick is a breakthrough for the flankers.
        let mut broke: [Option<Slot>; 2] = [None, None];
        for e in events.iter().filter(|e| e.turn == turn && e.tick == tick) {
            if let EventKind::Flanked {
                target, by, from, ..
            } = &e.kind
                && self.arena.in_zone(*from)
                && let Some(c) = by.first()
            {
                broke[c.side.index()].get_or_insert(*target);
            }
        }
        let mut push = |kind| events.push(Event { turn, tick, kind });

        // 7. The beacon.
        let mut holders = [0u8; 2];
        let mut present = [0u8; 2];
        for slot in self.standing_slots() {
            let f = self.fighter(slot);
            // Knocked off balance, a creature in the zone does not contest it this tick.
            if self.arena.in_zone(f.at) && !f.interrupted {
                present[slot.side.index()] += 1;
                if f.holding {
                    holders[slot.side.index()] += 1;
                }
            }
        }
        let rule = self.ruleset.capture_rule;
        let progressed = |side: Side| {
            let (s, o) = (side.index(), side.other().index());
            match rule {
                CaptureRule::Uncontested => holders[s] > 0 && present[o] == 0,
                CaptureRule::Majority => holders[s] > present[o],
            }
        };
        // A tick for holding the zone alone, and one for a breakthrough. Only one team can gain
        // in a tick: if both would, neither does.
        let gain = Side::BOTH
            .map(|side| u8::from(progressed(side)) + u8::from(broke[side.index()].is_some()));
        let both = gain[0] > 0 && gain[1] > 0;
        for side in Side::BOTH {
            let (s, o) = (side.index(), side.other().index());
            if gain[s] == 0 || both {
                continue;
            }
            if let Some(target) = broke[s] {
                push(EventKind::BrokeThrough { side, target });
            }
            self.board.capture[s] =
                (self.board.capture[s] + gain[s]).min(self.ruleset.capture_ticks);
            if self.ruleset.capture_decays {
                self.board.capture[o] = self.board.capture[o].saturating_sub(gain[s]);
            }
            push(EventKind::CaptureProgress {
                side,
                progress: self.board.capture[s],
            });
        }
        if both
            || (holders[0] > 0 || holders[1] > 0)
                && present[0] > 0
                && present[1] > 0
                && gain == [0, 0]
        {
            push(EventKind::CaptureContested);
        }

        // 8. The end.
        let standing = Side::BOTH.map(|side| self.team(side).count());
        let outcome = match standing {
            [0, 0] => Some((None, None)),
            [0, _] => Some((Some(Side::Right), Some(Victory::Knockout))),
            [_, 0] => Some((Some(Side::Left), Some(Victory::Knockout))),
            _ => Side::BOTH
                .into_iter()
                .find(|side| self.board.capture[side.index()] >= self.ruleset.capture_ticks)
                .map(|side| (Some(side), Some(Victory::Capture))),
        };
        if let Some((winner, victory)) = outcome {
            let outcome = Outcome {
                winner,
                victory,
                turn,
            };
            self.board.outcome = Some(outcome);
            push(EventKind::Finished { outcome });
        }
    }

    /// Settles the presses, tricks and interactions in reach and takes the composure they cost, never more than
    /// `Ruleset::damage_cap` from one creature in one tick. Anyone at zero is knocked out.
    fn settle(
        &mut self,
        turn: u8,
        tick: u8,
        now: u16,
        lost: &mut [[u8; TEAM_SIZE]; 2],
        events: &mut Vec<Event>,
    ) {
        let mut damage = [[0u8; TEAM_SIZE]; 2];
        self.engage(turn, tick, now, &mut damage, events);
        for slot in Slot::all() {
            let (s, i) = (slot.side.index(), slot.index as usize);
            let room = self.ruleset.damage_cap.saturating_sub(lost[s][i]);
            let taken = damage[s][i].min(room);
            if taken == 0 || !self.standing(slot) {
                continue;
            }
            lost[s][i] += taken;
            let f = self.fighter_mut(slot);
            f.composure = f.composure.saturating_sub(taken);
            let left = f.composure;
            events.push(Event {
                turn,
                tick,
                kind: EventKind::Composure {
                    slot,
                    lost: taken,
                    left,
                },
            });
            if left == 0 {
                f.out = true;
                f.holding = false;
                events.push(Event {
                    turn,
                    tick,
                    kind: EventKind::KnockedOut { slot },
                });
                self.drop_carried(slot, turn, tick, events);
            }
        }
    }

    /// Every USE ITEM given for the turn, at the start of its first tick: each is used from where
    /// its user stands, which takes its whole tick: no step, no press, trick or interaction. One that has nothing in its paws any
    /// more, or whose tile is out of reach, comes to nothing.
    fn use_items(&mut self, turn: u8, tick: u8, now: u16, events: &mut Vec<Event>) {
        for slot in self.standing_slots().collect::<Vec<_>>() {
            let Some(at) = self.fighter_mut(slot).using.take() else {
                continue;
            };
            self.fighter_mut(slot).pause = true;
            if self.can_use(slot, at) {
                self.use_item(slot, at, turn, tick, now, events);
            } else if let Some(item) = self.fighter(slot).carrying {
                events.push(Event {
                    turn,
                    tick,
                    kind: EventKind::ItemFizzled { slot, item, at },
                });
            }
        }
    }

    /// Where a creature at `there` is pushed by `by`: one tile straight on from one of them,
    /// onto free open ground, out of the zone if any of those ways leads out of it; ties go to
    /// the upper tile, then the one nearer its own side of the board. `None` if every way is
    /// blocked.
    fn pushed_from(&self, target: Slot, there: Tile, by: &[Slot]) -> Option<Tile> {
        by.iter()
            .filter_map(|c| {
                let from = self.fighter(*c).at;
                let away = Tile::new(2 * there.x - from.x, 2 * there.y - from.y);
                (self.arena.contains(away) && self.walkable(away) && self.occupant(away).is_none())
                    .then_some(away)
            })
            .min_by_key(|w| {
                (
                    self.arena.in_zone(*w),
                    w.y,
                    self.arena.across(target.side, *w),
                )
            })
    }

    /// Caught by a PRESS: pushed one tile on and stopped for the turn, its paws emptied into the
    /// first of `by` with room for what it carried (or onto the ground if none has).
    fn caught(
        &mut self,
        target: Slot,
        by: Vec<Slot>,
        tricking: bool,
        turn: u8,
        tick: u8,
        events: &mut Vec<Event>,
    ) {
        let there = self.fighter(target).at;
        let pushed_to = self.pushed_from(target, there, &by);
        let f = self.fighter_mut(target);
        f.interrupted = true;
        f.holding = false;
        if let Some(to) = pushed_to {
            f.at = to;
        }
        let item = f.carrying;
        let thief = by
            .iter()
            .copied()
            .find(|c| self.standing(*c) && self.fighter(*c).carrying.is_none());
        events.push(Event {
            turn,
            tick,
            kind: EventKind::Pressed {
                target,
                by,
                pushed_to,
                tricking,
            },
        });
        match (item, thief) {
            (Some(item), Some(thief)) => {
                self.fighter_mut(target).carrying = None;
                self.fighter_mut(thief).carrying = Some(item);
                events.push(Event {
                    turn,
                    tick,
                    kind: EventKind::Stole {
                        by: thief,
                        from: target,
                        item,
                    },
                });
            }
            (Some(_), None) => self.drop_carried(target, turn, tick, events),
            _ => {}
        }
        if pushed_to.is_some() {
            self.stick(target, turn, tick, events);
        }
    }

    /// Settles every PRESS, TRICK and INTERACT that has reached what it is aimed at, presses
    /// first, all from the same picture of the board.
    ///
    /// | Pressed | What happens |
    /// |---|---|
    /// | Pressing too (on its way or settling) | A collision: each loses one composure and both stop for the turn |
    /// | Holding | Repelled: it and each holder of its team beside it, corner to corner, that a presser has reached turns it away; one facing a presser costs it one composure |
    /// | Holding, pressed from two or more sides by more than its holders | Flanked: the hold breaks, it loses one composure and is pushed off its tile, or knocked off balance where it stands |
    /// | Anyone else | Pushed one tile on (off balance where it stands if blocked), stopped for the turn, and robbed of what it carries |
    ///
    /// | Tricked | What happens |
    /// |---|---|
    /// | Holding, from the side or behind | The hold breaks for the turn: it loses one composure and stands where it is, still on its feet |
    /// | Holding, from the front, or from in front of a holder of its team | Read: nothing happens |
    /// | Pressing | The trickster is caught: pushed, stopped and robbed by the one it tried to trick |
    /// | Anyone else | Nothing yet: the trickster waits beside it for its chance |
    ///
    /// Each settles once a turn. On the way to a target three or more tiles off, an opportunist
    /// may press someone not holding who walks into reach instead. A creature pressed this tick
    /// gets neither its TRICK nor its INTERACT.
    fn engage(
        &mut self,
        turn: u8,
        tick: u8,
        now: u16,
        damage: &mut [[u8; TEAM_SIZE]; 2],
        events: &mut Vec<Event>,
    ) {
        let mut presses: Vec<(Slot, Slot)> = Vec::new();
        let mut tricks: Vec<(Slot, Slot)> = Vec::new();
        let mut works: Vec<(Slot, Tile)> = Vec::new();
        let mut knocked: Vec<Slot> = Vec::new();
        // Pressing, on its way or settling now: a press that meets one collides.
        let pressing: BTreeSet<Slot> = self
            .standing_slots()
            .filter(|s| {
                let f = self.fighter(*s);
                matches!(f.order, Some(Order::Press { .. })) && !f.spent && !f.interrupted
            })
            .collect();
        for c in self.standing_slots().collect::<Vec<_>>() {
            let f = self.fighter(c).clone();
            if f.spent || f.interrupted || f.pause {
                continue;
            }
            match f.order {
                Some(Order::Press { target: t }) => {
                    if !self.standing(t) {
                        self.fighter_mut(c).spent = true;
                        continue;
                    }
                    let there = self.fighter(t).at;
                    if f.at.touches(there) {
                        presses.push((c, t));
                    } else if f.at.distance(there) >= 3 {
                        let opportunism = self.behavior(c).opportunism;
                        let nearby: Vec<Slot> = self
                            .team(c.side.other())
                            .filter(|e| {
                                *e != t
                                    && self.fighter(*e).at.touches(f.at)
                                    && !self.fighter(*e).holding
                            })
                            .collect();
                        if let Some(took) = nearby.first().copied()
                            && opportunism > 0
                            && rng::chance(
                                self.seed,
                                Purpose::Opportunity,
                                &[u64::from(now), c.key(), took.key()],
                                opportunism as u32,
                            )
                        {
                            events.push(Event {
                                turn,
                                tick,
                                kind: EventKind::TookOpportunity {
                                    slot: c,
                                    meant: t,
                                    took,
                                },
                            });
                            presses.push((c, took));
                        }
                    }
                }
                Some(Order::Trick { target: t }) => {
                    if !self.standing(t) {
                        self.fighter_mut(c).spent = true;
                        continue;
                    }
                    if f.at.touches(self.fighter(t).at) {
                        tricks.push((c, t));
                    }
                }
                Some(Order::Interact { at }) if f.at.touches(at) => works.push((c, at)),
                _ => {}
            }
        }

        // Presses.
        let mut targets: Vec<Slot> = presses.iter().map(|(_, t)| *t).collect();
        targets.sort();
        targets.dedup();
        let mut countered: BTreeSet<Slot> = BTreeSet::new();
        let mut collided: BTreeSet<(Slot, Slot)> = BTreeSet::new();
        for t in targets {
            let there = self.fighter(t).at;
            let mut cs: Vec<Slot> = presses
                .iter()
                .filter(|(_, target)| *target == t)
                .map(|(c, _)| *c)
                .collect();
            cs.sort_by_key(|c| {
                (
                    self.fighter(*c).at.distance(there),
                    rng::roll(self.seed, Purpose::Pairing, &[u64::from(now), c.key()]),
                )
            });
            for c in &cs {
                self.fighter_mut(*c).spent = true;
            }
            if pressing.contains(&t) {
                // Press against press: each loses one, and both stop. Two pressing each other
                // collide once.
                cs.retain(|c| !collided.contains(&(t, *c)));
                if cs.is_empty() {
                    continue;
                }
                damage[t.side.index()][t.index as usize] += 1;
                self.fighter_mut(t).interrupted = true;
                for c in &cs {
                    collided.insert((*c, t));
                    damage[c.side.index()][c.index as usize] += 1;
                    self.fighter_mut(*c).interrupted = true;
                }
                events.push(Event {
                    turn,
                    tick,
                    kind: EventKind::Collided { target: t, by: cs },
                });
            } else if self.fighter(t).holding {
                // The one pressed, and holders of its team beside it, corner to corner, that a
                // presser has come within reach of: side by side, two holders share the two
                // tiles they both touch.
                let holders: Vec<Slot> = self
                    .team(t.side)
                    .filter(|h| {
                        let at = self.fighter(*h).at;
                        self.fighter(*h).holding
                            && !countered.contains(h)
                            && (*h == t || at.beside(there))
                            && cs.iter().any(|c| self.fighter(*c).at.touches(at))
                    })
                    .collect();
                if cs.len() >= 2 && cs.len() > holders.len() {
                    // Flanked: pressed from more sides than it has holders to turn them away,
                    // the hold breaks. It loses one composure, however many came at it, and is
                    // pushed off its tile away from a presser, out of the zone if it can be, or
                    // knocked off balance where it stands if every way out is blocked.
                    let pushed_to = self.pushed_from(t, there, &cs);
                    damage[t.side.index()][t.index as usize] += 1;
                    let f = self.fighter_mut(t);
                    f.holding = false;
                    f.interrupted = true;
                    if let Some(to) = pushed_to {
                        f.at = to;
                    }
                    knocked.push(t);
                    // The flank takes the ground: flankers standing in the zone start holding
                    // where they stand, facing where the holder stood.
                    let taking: Vec<(Slot, Tile)> = cs
                        .iter()
                        .map(|c| (*c, self.fighter(*c).at))
                        .filter(|(_, at)| self.arena.in_zone(*at))
                        .collect();
                    events.push(Event {
                        turn,
                        tick,
                        kind: EventKind::Flanked {
                            target: t,
                            by: cs,
                            from: there,
                            pushed_to,
                        },
                    });
                    for (c, at) in taking {
                        let f = self.fighter_mut(c);
                        f.holding = true;
                        f.facing = Facing::toward(at, there).unwrap_or(f.facing);
                        events.push(Event {
                            turn,
                            tick,
                            kind: EventKind::Holding { slot: c, at },
                        });
                    }
                } else {
                    // Repelled. A holder facing a presser costs it one composure, at most once a
                    // tick, spread so no presser takes a second before every other has taken one.
                    let mut hits = vec![0u8; cs.len()];
                    let mut faced = Vec::new();
                    for h in &holders {
                        let ahead = self.fighter(*h).facing.ahead(self.fighter(*h).at);
                        if let Some(i) = (0..cs.len())
                            .filter(|i| self.fighter(cs[*i]).at == ahead)
                            .min_by_key(|i| hits[*i])
                        {
                            hits[i] += 1;
                            damage[cs[i].side.index()][cs[i].index as usize] += 1;
                            countered.insert(*h);
                            faced.push(cs[i]);
                        }
                    }
                    for c in &cs {
                        self.fighter_mut(*c).turned_away = true;
                    }
                    events.push(Event {
                        turn,
                        tick,
                        kind: EventKind::Repelled {
                            target: t,
                            holders,
                            pressers: cs,
                            faced,
                        },
                    });
                }
            } else if self.fighter(t).sheltered(now) {
                // Under a leaf umbrella, a press bounces off: nobody is hurt.
                events.push(Event {
                    turn,
                    tick,
                    kind: EventKind::Sheltered { target: t, by: cs },
                });
            } else {
                let tricking = matches!(self.fighter(t).order, Some(Order::Trick { .. }));
                self.caught(t, cs, tricking, turn, tick, events);
            }
        }

        // Tricks, from where everyone stands now.
        tricks.sort();
        let mut landed: Vec<(Slot, Vec<Slot>)> = Vec::new();
        for (c, t) in tricks {
            if self.fighter(c).interrupted || self.fighter(c).spent || !self.standing(c) {
                continue;
            }
            let target = self.fighter(t).clone();
            if target.holding {
                self.fighter_mut(c).spent = true;
                // Seen from the front, by the holder or by a holder of its team beside it.
                let from = self.fighter(c).at;
                let seen = (Quarter::of(target.at, target.facing, from) == Some(Quarter::Front))
                    .then_some(t)
                    .or_else(|| crate::muster::faced_by(self, t.side, from));
                match seen {
                    None => match landed.iter_mut().find(|(x, _)| *x == t) {
                        Some((_, by)) => by.push(c),
                        None => landed.push((t, vec![c])),
                    },
                    Some(reader) => events.push(Event {
                        turn,
                        tick,
                        kind: EventKind::TrickRead {
                            target: reader,
                            by: c,
                        },
                    }),
                }
            } else if pressing.contains(&t) && !target.interrupted {
                // Tricking one that is pressing: it is read and caught by it.
                self.fighter_mut(c).spent = true;
                self.caught(c, vec![t], true, turn, tick, events);
            }
        }
        for (t, by) in landed {
            if !self.fighter(t).holding {
                continue;
            }
            let from = self.fighter(t).at;
            damage[t.side.index()][t.index as usize] += 1;
            let f = self.fighter_mut(t);
            f.holding = false;
            f.broken = true;
            events.push(Event {
                turn,
                tick,
                kind: EventKind::Tricked {
                    target: t,
                    by,
                    from,
                },
            });
        }

        // Levers and switches, unless whoever was working one was pressed first.
        works.sort();
        for (c, at) in works {
            if self.fighter(c).interrupted || !self.standing(c) {
                continue;
            }
            self.fighter_mut(c).spent = true;
            let defenders: Vec<Slot> = self
                .team(c.side.other())
                .filter(|h| self.fighter(*h).holding && self.fighter(*h).at.touches(at))
                .collect();
            let switch = self.arena.switches.contains(&at);
            if !defenders.is_empty() {
                self.fighter_mut(c).turned_away = true;
                damage[c.side.index()][c.index as usize] += defenders.len() as u8;
                let kind = if switch {
                    EventKind::SwitchDefended {
                        slot: c,
                        switch: at,
                        holders: defenders,
                    }
                } else {
                    EventKind::LeverDefended {
                        slot: c,
                        lever: at,
                        holders: defenders,
                    }
                };
                events.push(Event { turn, tick, kind });
            } else if switch {
                if self.switch_ready() {
                    self.set_off(c, at, turn, tick, now, events);
                } else {
                    events.push(Event {
                        turn,
                        tick,
                        kind: EventKind::SwitchResting {
                            slot: c,
                            switch: at,
                        },
                    });
                }
            } else if !self.lever_ready() {
                events.push(Event {
                    turn,
                    tick,
                    kind: EventKind::LeverResting { slot: c, lever: at },
                });
            } else {
                let swing = now + u16::from(self.arena.lever.swing_ticks);
                self.board.swing_at = Some(swing);
                let (swings_turn, swings_tick) = self.when(swing);
                events.push(Event {
                    turn,
                    tick,
                    kind: EventKind::LeverPulled {
                        slot: c,
                        lever: at,
                        swings_turn,
                        swings_tick,
                    },
                });
            }
        }
        for slot in knocked {
            self.drop_carried(slot, turn, tick, events);
        }
    }

    /// The turn and tick a tick counted from the start of the match falls in.
    pub(crate) fn when(&self, at: u16) -> (u8, u8) {
        let per = u16::from(self.ruleset.ticks_per_turn);
        // The clock counts turn 1's first tick as `per + 1`.
        (((at - 1) / per) as u8, ((at - 1) % per + 1) as u8)
    }

    /// Ends the match for a side whose Handler has been away too long: the other side wins. Only
    /// a room ever does this; nothing in the rules does.
    pub fn forfeit(&mut self, side: Side) {
        if self.board.outcome.is_none() {
            self.board.outcome = Some(Outcome {
                winner: Some(side.other()),
                victory: Some(Victory::Forfeit),
                turn: self.board.turn,
            });
        }
    }

    /// What the turn cap weighs for a team, in its order: beacon progress, composure left across
    /// the team, creatures standing.
    pub fn cap_tally(&self, side: Side) -> (u8, u32, usize) {
        let team: Vec<_> = self.team(side).collect();
        (
            self.board.capture[side.index()],
            team.iter()
                .map(|s| u32::from(self.fighter(*s).composure))
                .sum::<u32>(),
            team.len(),
        )
    }

    /// At the turn cap: more beacon progress wins, then more composure left across the team, then
    /// more creatures standing. Anything level after that is a draw.
    fn decide_at_cap(&self) -> Outcome {
        let (left, right) = (self.cap_tally(Side::Left), self.cap_tally(Side::Right));
        let winner = match left.cmp(&right) {
            std::cmp::Ordering::Greater => Some(Side::Left),
            std::cmp::Ordering::Less => Some(Side::Right),
            std::cmp::Ordering::Equal => None,
        };
        Outcome {
            winner,
            victory: winner.map(|_| Victory::TurnCap),
            turn: self.board.turn,
        }
    }
}
