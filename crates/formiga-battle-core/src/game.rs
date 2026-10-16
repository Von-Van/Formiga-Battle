//! A match: the board, and how one turn of simultaneous orders resolves on it.
//!
//! Both teams' orders are locked together, then the turn plays out in a few ticks. Every tick
//! works from one shared picture of the board: everyone decides their step from where everyone
//! stood, every step happens at once, and then every contest is settled from where everyone
//! ended up. Nobody moves first.
//!
//! A tick, in order:
//!
//! 1. A gate whose swing was telegraphed last tick swings. Then whatever was marked to land this
//!    tick lands: things found on the board turn up, items used last tick take effect, the map
//!    event comes (`scenery`).
//! 2. Contests already next to their target are settled, all from the same picture (see
//!    `engage`), so someone walking away from a contester next to it is caught before it steps.
//! 3. Each creature still carrying out its order picks one step (`interpret`).
//! 4. The steps happen together. Two wanting one tile: a fair draw lets one in. Two wanting to
//!    swap, or a step into a tile whose creature did not leave: they stay where they were. Nobody
//!    is hurt by a bump. Anyone with empty paws who stepped onto something lying there picks it
//!    up.
//! 5. Anyone who has reached the tile it was told to hold starts holding.
//! 6. Contests that have just come into reach are settled the same way. Composure is lost as each
//!    is settled, at most `Ruleset::damage_cap` from one creature in a tick; anyone at zero is
//!    knocked out.
//! 7. The beacon counts: a team holding in the zone with nobody from the other team in it gains a
//!    tick of progress.
//! 8. The match ends by knockout, then by capture.

use crate::arena::{Arena, TEAM_SIZE, Terrain};
use crate::event::{Event, EventKind, Fill, Outcome, Victory};
use crate::grid::Tile;
use crate::items::Item;
use crate::order::{Order, Orders, Side, Slot, Target};
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
    /// Its contest has been settled this turn, whatever came of it.
    pub spent: bool,
    /// A contest stopped it; it stays where it is for the rest of the turn.
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
    /// Its contest this turn was turned away by a holder.
    #[serde(default)]
    pub turned_away: bool,
    /// What it picked up, if anything. One thing at a time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrying: Option<Item>,
    /// The last tick, counted from the start of the match, a leaf umbrella is open over it; 0 for
    /// none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub shelter_until: u16,
}

fn is_zero(n: &u16) -> bool {
    *n == 0
}

impl Fighter {
    fn new(at: Tile, composure: u8) -> Self {
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
            carrying: None,
            shelter_until: 0,
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
            || self.bumped
            || self.order.is_none()
            || match self.order {
                Some(Order::Move { .. }) => self.arrived,
                Some(Order::Contest { .. } | Order::Use { .. }) => self.spent,
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
}

/// One turn as it was played: what each team sent (with `None` for an order not given in time),
/// and the board's hash after it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnRecord {
    pub submitted: [Orders; 2],
    pub hash: u64,
}

/// What resolving one turn gave.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnReport {
    pub turn: u8,
    /// The orders carried out, the fallback's included; `None` for a creature already out.
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
}

impl Match {
    pub fn new(
        arena: Arena,
        ruleset: Ruleset,
        seed: u64,
        rosters: [Vec<Profile>; 2],
    ) -> Result<Self, MatchError> {
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
        let fighters =
            [0, 1].map(|side| arena.starts[side].map(|tile| Fighter::new(tile, ruleset.composure)));
        let board = Board {
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
        };
        Ok(Self {
            arena,
            ruleset,
            seed,
            rosters: [left, right],
            behaviors,
            board,
            history: Vec::new(),
        })
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
        if self.board.twigs.iter().any(|(t, _)| *t == tile) {
            return false;
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
            Order::Hold { at } => ground(at),
            Order::Contest {
                target: Target::Creature { slot: target },
            } => {
                target.side != slot.side
                    && (target.index as usize) < TEAM_SIZE
                    && self.standing(target)
            }
            Order::Contest {
                target: Target::Lever { at },
            } => self.arena.levers.contains(&at) || self.arena.switches.contains(&at),
            Order::Use { at } => {
                self.arena.contains(at)
                    && match self.fighter(slot).carrying {
                        Some(Item::Twig) => ground(at),
                        Some(_) => true,
                        None => false,
                    }
            }
        }
    }

    /// Locks both teams' orders, fills any that are missing or cannot be carried out with the
    /// fallback (the same for both teams), and plays the turn out.
    pub fn resolve_turn(&mut self, submitted: [Orders; 2]) -> TurnReport {
        let mut events = Vec::new();
        if self.board.outcome.is_some() {
            return TurnReport {
                turn: self.board.turn,
                orders: [[None; TEAM_SIZE]; 2],
                events,
                hash: self.hash(),
            };
        }
        self.board.turn += 1;
        let turn = self.board.turn;
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
            let f = self.fighter_mut(slot);
            f.order = order;
            f.spent = false;
            f.interrupted = false;
            f.arrived = false;
            f.followed_up = false;
            f.detours_left = detours;
            f.bumped = false;
            f.hung_back = false;
            f.turned_away = false;
            f.holding = matches!(order, Some(Order::Hold { at }) if at == f.at) && !f.out;
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
        self.history.push(TurnRecord { submitted, hash });
        TurnReport {
            turn,
            orders,
            events,
            hash,
        }
    }

    fn tick(&mut self, turn: u8, tick: u8, events: &mut Vec<Event>) {
        let now = self.clock(turn, tick);
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

        // Then whatever was marked to land this tick.
        self.land(turn, tick, now, events);

        // 2. Contests already in reach catch their target before anyone steps.
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
                Some(Order::Hold { at }) if !f.holding && !f.interrupted => {
                    // Someone already standing on the tile: hold beside it instead.
                    let short = f.at.touches(at) && self.occupant(at).is_some_and(|o| o != slot);
                    if f.at == at || short {
                        self.fighter_mut(slot).holding = true;
                        push(EventKind::Holding { slot, at: f.at });
                    }
                }
                _ => {}
            }
        }

        // 6. Contests that have just come into reach.
        self.settle(turn, tick, now, &mut lost, events);
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
        for side in Side::BOTH {
            let (s, o) = (side.index(), side.other().index());
            if progressed(side) {
                self.board.capture[s] = (self.board.capture[s] + 1).min(self.ruleset.capture_ticks);
                if self.ruleset.capture_decays {
                    self.board.capture[o] = self.board.capture[o].saturating_sub(1);
                }
                push(EventKind::CaptureProgress {
                    side,
                    progress: self.board.capture[s],
                });
            }
        }
        if (holders[0] > 0 || holders[1] > 0)
            && present[0] > 0
            && present[1] > 0
            && !progressed(Side::Left)
            && !progressed(Side::Right)
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

    /// Settles the contests in reach and takes the composure they cost, never more than
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

    /// Settles every contest that has reached its target, all from the same picture of the
    /// board.
    ///
    /// | Target | What happens |
    /// |---|---|
    /// | Not holding (moving, arrived, done) | Interrupted: it stops for the turn and loses one composure for each contester |
    /// | Holding | Countered: it and each holder of its team beside it, corner to corner, that a contester has reached costs one contester one composure |
    /// | Holding, contested from two or more sides by more than its holders | Flanked: the hold breaks, it loses one composure and is pushed off its tile, or knocked off balance where it stands |
    /// | Busy with its own contest this tick | A scuffle: nobody is hurt |
    ///
    /// A creature contests once a turn. On the way to a target three or more tiles off, an
    /// opportunist may take a contest against someone not holding who walks into reach instead.
    fn engage(
        &mut self,
        turn: u8,
        tick: u8,
        now: u16,
        damage: &mut [[u8; TEAM_SIZE]; 2],
        events: &mut Vec<Event>,
    ) {
        let mut push = |kind| events.push(Event { turn, tick, kind });
        let mut contests: Vec<(Slot, Slot)> = Vec::new();
        let mut pulls: Vec<(Slot, Tile)> = Vec::new();
        let mut turns: Vec<(Slot, Tile)> = Vec::new();
        let mut uses: Vec<(Slot, Tile)> = Vec::new();
        let mut knocked: Vec<Slot> = Vec::new();
        for c in self.standing_slots().collect::<Vec<_>>() {
            let f = self.fighter(c).clone();
            if f.spent || f.interrupted {
                continue;
            }
            match f.order {
                Some(Order::Contest {
                    target: Target::Creature { slot: t },
                }) => {
                    if !self.standing(t) {
                        self.fighter_mut(c).spent = true;
                        continue;
                    }
                    let there = self.fighter(t).at;
                    if f.at.touches(there) {
                        contests.push((c, t));
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
                            push(EventKind::TookOpportunity {
                                slot: c,
                                meant: t,
                                took,
                            });
                            contests.push((c, took));
                        }
                    }
                }
                Some(Order::Contest {
                    target: Target::Lever { at },
                }) if f.at.touches(at) => {
                    if self.arena.switches.contains(&at) {
                        turns.push((c, at));
                    } else {
                        pulls.push((c, at));
                    }
                }
                Some(Order::Use { at }) => match f.carrying {
                    Some(item) if f.at.distance(at) <= item.reach() => uses.push((c, at)),
                    Some(_) => {}
                    // Let go of on the way: there is nothing left to use.
                    None => self.fighter_mut(c).spent = true,
                },
                _ => {}
            }
        }
        let engaging: BTreeSet<Slot> = contests
            .iter()
            .map(|(c, _)| *c)
            .chain(pulls.iter().map(|(c, _)| *c))
            .chain(turns.iter().map(|(c, _)| *c))
            .chain(uses.iter().map(|(c, _)| *c))
            .collect();
        let mut targets: Vec<Slot> = contests.iter().map(|(_, t)| *t).collect();
        targets.sort();
        targets.dedup();
        let mut countered: BTreeSet<Slot> = BTreeSet::new();
        for t in targets {
            let there = self.fighter(t).at;
            let mut cs: Vec<Slot> = contests
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
            if engaging.contains(&t) {
                for c in cs {
                    let mutual = contests.contains(&(t, c));
                    // A mutual scuffle is told once.
                    if !mutual || c < t {
                        push(EventKind::Scuffle {
                            contester: c,
                            target: t,
                            mutual,
                        });
                    }
                }
            } else if self.fighter(t).holding {
                // The one contested, and holders of its team beside it, corner to corner, that a
                // contester has come within reach of: side by side, two holders share the two
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
                    // Flanked: contested from more sides than it has holders to turn them away,
                    // the hold breaks. It loses one composure, however many came at it, and is
                    // pushed off its tile away from a contester, out of the zone if it can be,
                    // or knocked off balance where it stands if every way out is blocked.
                    let pushed_to = cs
                        .iter()
                        .filter_map(|c| {
                            let from = self.fighter(*c).at;
                            let away = Tile::new(2 * there.x - from.x, 2 * there.y - from.y);
                            (self.arena.contains(away)
                                && self.walkable(away)
                                && self.occupant(away).is_none())
                            .then_some(away)
                        })
                        .min_by_key(|w| {
                            (self.arena.in_zone(*w), w.y, self.arena.across(t.side, *w))
                        });
                    damage[t.side.index()][t.index as usize] += 1;
                    let f = self.fighter_mut(t);
                    f.holding = false;
                    f.interrupted = true;
                    if let Some(to) = pushed_to {
                        f.at = to;
                    }
                    knocked.push(t);
                    push(EventKind::Flanked {
                        target: t,
                        by: cs,
                        from: there,
                        pushed_to,
                    });
                } else {
                    let mut hits = vec![0u8; cs.len()];
                    for h in &holders {
                        let reach = self.fighter(*h).at;
                        if let Some(i) = (0..cs.len())
                            .filter(|i| self.fighter(cs[*i]).at.touches(reach))
                            .min_by_key(|i| hits[*i])
                        {
                            hits[i] += 1;
                            damage[cs[i].side.index()][cs[i].index as usize] += 1;
                            countered.insert(*h);
                        }
                    }
                    for c in &cs {
                        self.fighter_mut(*c).turned_away = true;
                    }
                    push(EventKind::Countered {
                        target: t,
                        holders,
                        contesters: cs,
                    });
                }
            } else if self.fighter(t).sheltered(now) {
                // Under a leaf umbrella, a contest bounces off: nobody is hurt.
                push(EventKind::Sheltered { target: t, by: cs });
            } else {
                damage[t.side.index()][t.index as usize] += cs.len() as u8;
                self.fighter_mut(t).interrupted = true;
                knocked.push(t);
                push(EventKind::Interrupted { target: t, by: cs });
            }
        }
        pulls.sort();
        for (c, lever) in pulls {
            self.fighter_mut(c).spent = true;
            let defenders: Vec<Slot> = self
                .team(c.side.other())
                .filter(|h| self.fighter(*h).holding && self.fighter(*h).at.touches(lever))
                .collect();
            if !defenders.is_empty() {
                self.fighter_mut(c).turned_away = true;
                damage[c.side.index()][c.index as usize] += defenders.len() as u8;
                push(EventKind::LeverDefended {
                    slot: c,
                    lever,
                    holders: defenders,
                });
            } else if !self.lever_ready() {
                push(EventKind::LeverResting { slot: c, lever });
            } else {
                let at = now + u16::from(self.arena.lever.swing_ticks);
                self.board.swing_at = Some(at);
                let per = u16::from(self.ruleset.ticks_per_turn);
                // The clock counts turn 1's first tick as `per + 1`.
                let (swings_turn, swings_tick) =
                    (((at - 1) / per) as u8, ((at - 1) % per + 1) as u8);
                push(EventKind::LeverPulled {
                    slot: c,
                    lever,
                    swings_turn,
                    swings_tick,
                });
            }
        }
        turns.sort();
        for (c, switch) in turns {
            self.fighter_mut(c).spent = true;
            let defenders: Vec<Slot> = self
                .team(c.side.other())
                .filter(|h| self.fighter(*h).holding && self.fighter(*h).at.touches(switch))
                .collect();
            if !defenders.is_empty() {
                self.fighter_mut(c).turned_away = true;
                damage[c.side.index()][c.index as usize] += defenders.len() as u8;
                push(EventKind::SwitchDefended {
                    slot: c,
                    switch,
                    holders: defenders,
                });
            } else if !self.switch_ready() {
                push(EventKind::SwitchResting { slot: c, switch });
            } else {
                let rest = self.arena.scenery.as_ref().map_or(0, |s| s.rest);
                self.board.event_at = Some(now + 1);
                self.board.switch_ready_turn = turn.saturating_add(rest).saturating_add(1);
                push(EventKind::SwitchTurned { slot: c, switch });
                let tiles = self
                    .arena
                    .scenery
                    .as_ref()
                    .map(|s| s.tiles.clone())
                    .unwrap_or_default();
                let (lands_turn, lands_tick) = self.when(now + 1);
                push(EventKind::SceneryWarned {
                    tiles,
                    lands_turn,
                    lands_tick,
                });
            }
        }
        uses.sort();
        for (c, at) in uses {
            self.use_item(c, at, turn, tick, now, events);
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

    /// At the turn cap: more beacon progress wins, then more composure left across the team, then
    /// more creatures standing. Anything level after that is a draw.
    fn decide_at_cap(&self) -> Outcome {
        let score = |side: Side| {
            let team: Vec<_> = self.team(side).collect();
            (
                self.board.capture[side.index()],
                team.iter()
                    .map(|s| u32::from(self.fighter(*s).composure))
                    .sum::<u32>(),
                team.len(),
            )
        };
        let (left, right) = (score(Side::Left), score(Side::Right));
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
