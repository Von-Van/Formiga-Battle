//! What the board does by itself, and what found things do: the arena's map event, items turning
//! up, being picked up, carried, dropped and used.
//!
//! Everything here is warned before it lands, and the warning is on the board for both teams to
//! see: the map event a whole turn before it comes round by itself, or a tick before when a switch
//! sets it off; a found thing glints on its spots a whole turn before it turns up; an item's effect
//! is marked on its tiles the tick it is used. Nothing here costs composure. Its effects are ones
//! a match already has: off balance, a push of a tile or two, a tile nobody can stand on.

use crate::arena::{Effect, Terrain};
use crate::event::{Blown, Event, EventKind};
use crate::game::Match;
use crate::grid::Tile;
use crate::items::{self, Item};
use crate::order::{Order, Side, Slot};
use crate::rng::{self, Purpose};
use serde::{Deserialize, Serialize};

/// An item used and marked on its tiles, landing at a tick counted from the start of the match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pending {
    pub at: u16,
    pub by: Slot,
    pub item: Item,
    /// Where its user stood when it used it.
    pub from: Tile,
    pub target: Tile,
}

impl Match {
    /// At the end of a turn, what the next one brings: the map event, if it comes round by itself
    /// next turn, and found things glinting on the spots they will turn up on.
    pub(crate) fn foretell(&mut self, turn: u8, events: &mut Vec<Event>) {
        let next = turn.saturating_add(1);
        if next > self.ruleset.turn_cap {
            return;
        }
        let tick = self.ruleset.ticks_per_turn;
        if let Some(scenery) = self.arena.scenery.clone()
            && scenery.due(next)
            && self.board.event_at.is_none()
        {
            // Round a switch, by itself it comes round a mirrored pair of them, drawn at random.
            let from = if scenery.around {
                let pairs = self.arena.switch_pairs();
                let roll = rng::roll(self.seed, Purpose::Find, &[u64::from(next), 3]);
                pairs[(roll % pairs.len() as u64) as usize].clone()
            } else {
                Vec::new()
            };
            let tiles = self.arena.scenery_tiles(&from);
            // Mid-turn, so anyone sent away has a tick to step off.
            let lands_tick = 2.min(tick);
            self.board.event_at = Some(self.clock(next, lands_tick));
            self.board.event_tiles = tiles.clone();
            self.board.event_from = from;
            events.push(Event {
                turn,
                tick,
                kind: EventKind::SceneryWarned {
                    tiles,
                    lands_turn: next,
                    lands_tick,
                },
            });
        }
        if next >= items::FIRST_TURN && (next - items::FIRST_TURN).is_multiple_of(items::EVERY) {
            let free = |t: &Tile| {
                !self.board.ground.iter().any(|(g, _)| g == t)
                    && !self.board.twigs.iter().any(|(w, _)| w == t)
            };
            let pairs: Vec<Vec<Tile>> = self
                .arena
                .item_pairs()
                .into_iter()
                .filter(|pair| pair.iter().all(free))
                .collect();
            if pairs.is_empty() {
                return;
            }
            let roll = |k: u64| rng::roll(self.seed, Purpose::Find, &[u64::from(next), k]);
            let spots = pairs[(roll(1) % pairs.len() as u64) as usize].clone();
            let total: u64 = Item::ALL.iter().map(|i| i.weight()).sum();
            let mut pick = roll(2) % total;
            let item = Item::ALL
                .into_iter()
                .find(|i| {
                    if pick < i.weight() {
                        true
                    } else {
                        pick -= i.weight();
                        false
                    }
                })
                .expect("the pick falls on an item");
            self.board.glint = spots.iter().map(|t| (*t, item)).collect();
            events.push(Event {
                turn,
                tick,
                kind: EventKind::ItemsGlint {
                    spots,
                    item,
                    appears_turn: next,
                },
            });
        }
    }

    /// At the start of a tick, after any gate swings: umbrellas fold, and twigs, rafts, honey
    /// and ground gone under come back when their time is up; glinting things turn up, items used
    /// last tick land, and the map event comes.
    pub(crate) fn land(&mut self, turn: u8, tick: u8, now: u16, events: &mut Vec<Event>) {
        let push = |events: &mut Vec<Event>, kind| events.push(Event { turn, tick, kind });
        for slot in self.standing_slots().collect::<Vec<_>>() {
            let f = self.fighter_mut(slot);
            if f.shelter_until != 0 && f.shelter_until < now {
                f.shelter_until = 0;
                push(events, EventKind::ShelterFolded { slot });
            }
        }
        let over = |list: &mut Vec<(Tile, u16)>| -> Vec<Tile> {
            let (gone, left): (Vec<_>, Vec<_>) = std::mem::take(list)
                .into_iter()
                .partition(|(_, until)| *until < now);
            *list = left;
            gone.into_iter().map(|(t, _)| t).collect()
        };
        for at in over(&mut self.board.twigs) {
            push(events, EventKind::TwigFell { at });
        }
        for at in over(&mut self.board.rafts) {
            push(events, EventKind::RaftSank { at });
        }
        let dried = over(&mut self.board.honey);
        if !dried.is_empty() {
            push(events, EventKind::HoneyDried { tiles: dried });
        }
        let reopened = over(&mut self.board.closed);
        if !reopened.is_empty() {
            push(events, EventKind::GroundReopened { tiles: reopened });
        }
        if tick == 1 {
            for (at, item) in std::mem::take(&mut self.board.glint) {
                let taken = self.board.ground.iter().any(|(g, _)| *g == at)
                    || self.board.twigs.iter().any(|(t, _)| *t == at);
                if taken {
                    continue;
                }
                self.board.ground.push((at, item));
                push(events, EventKind::ItemAppeared { at, item });
                if let Some(slot) = self.occupant(at) {
                    self.pick_up(slot, turn, tick, events);
                }
            }
        }
        let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.board.pending)
            .into_iter()
            .partition(|p| p.at <= now);
        self.board.pending = later;
        for p in due {
            match p.item {
                Item::Puff => self.puff(p, turn, tick, now, events),
                Item::Twig => self.wedge(p, turn, tick, events),
                Item::Leaf => self.shelter(p, turn, tick, events),
                Item::Bell => self.ring(p, turn, tick, now, events),
                Item::Honey => self.spill(p, turn, tick, events),
                Item::Seed => self.hop(p, turn, tick, events),
            }
        }
        if self.board.event_at.is_some_and(|at| at <= now)
            && let Some(scenery) = self.arena.scenery.clone()
        {
            self.board.event_at = None;
            let tiles = std::mem::take(&mut self.board.event_tiles);
            let from = std::mem::take(&mut self.board.event_from);
            let mut hit = Vec::new();
            let mut sheltered = Vec::new();
            let mut moved = Vec::new();
            for slot in self.standing_slots().collect::<Vec<_>>() {
                let f = self.fighter(slot);
                if !tiles.contains(&f.at) {
                    continue;
                }
                // A leaf umbrella keeps off spray and waves, but not the water rising.
                if f.sheltered(now) && scenery.effect != Effect::Close {
                    sheltered.push(slot);
                    continue;
                }
                let at = f.at;
                if scenery.effect != Effect::Soak {
                    let to = self.washed_to(at, &tiles, scenery.effect == Effect::Close);
                    if let Some(to) = to {
                        self.fighter_mut(slot).at = to;
                    }
                    moved.push(Blown {
                        slot,
                        from: at,
                        to,
                        sailed: false,
                    });
                }
                let f = self.fighter_mut(slot);
                f.holding = false;
                f.interrupted = true;
                hit.push(slot);
            }
            let mut fell = Vec::new();
            if scenery.effect == Effect::Close {
                let until = self.clock(turn, self.ruleset.ticks_per_turn);
                for tile in &tiles {
                    // What lay or stood there floats off.
                    self.board.ground.retain(|(g, _)| g != tile);
                    if let Some(i) = self.board.twigs.iter().position(|(t, _)| t == tile) {
                        self.board.twigs.remove(i);
                        fell.push(*tile);
                    }
                    self.board.closed.push((*tile, until));
                }
            }
            push(
                events,
                EventKind::SceneryLanded {
                    effect: scenery.effect,
                    tiles,
                    hit: hit.clone(),
                    sheltered,
                    moved,
                },
            );
            for at in fell {
                push(events, EventKind::TwigFell { at });
            }
            for slot in hit {
                self.stick(slot, turn, tick, events);
                self.drop_carried(slot, turn, tick, events);
            }
            // A shaken tree drops a twig beside it, ready to pick up.
            for switch in from {
                let side = if self.arena.across(Side::Left, switch) < self.arena.width as i8 / 2 {
                    Side::Left
                } else {
                    Side::Right
                };
                let spot = switch
                    .steps()
                    .into_iter()
                    .filter(|t| {
                        self.arena.contains(*t)
                            && self.walkable(*t)
                            && !self.board.ground.iter().any(|(g, _)| g == t)
                    })
                    .min_by_key(|t| (t.y, self.arena.across(side, *t)));
                if let Some(at) = spot {
                    self.board.ground.push((at, Item::Twig));
                    push(
                        events,
                        EventKind::ItemAppeared {
                            at,
                            item: Item::Twig,
                        },
                    );
                    if let Some(slot) = self.occupant(at) {
                        self.pick_up(slot, turn, tick, events);
                    }
                }
            }
        }
    }

    /// Where a wave or the rising water washes someone standing at `at`: the open, free tile
    /// beside it nearest the board's middle row and off the event's `tiles`, upper first among
    /// equals, then the one nearer its own side. Rising water washes it off the yard altogether.
    /// `None` if there is nowhere to go.
    fn washed_to(&self, at: Tile, tiles: &[Tile], off_the_yard: bool) -> Option<Tile> {
        let middle = (i16::from(self.arena.height) - 1) / 2;
        let side = if self.arena.across(Side::Left, at) < self.arena.width as i8 / 2 {
            Side::Left
        } else {
            Side::Right
        };
        at.steps()
            .into_iter()
            .filter(|t| {
                self.arena.contains(*t)
                    && self.walkable(*t)
                    && self.occupant(*t).is_none()
                    && !tiles.contains(t)
                    && !(off_the_yard && self.arena.in_zone(*t))
            })
            .min_by_key(|t| {
                (
                    (i16::from(t.y) - middle).abs(),
                    t.y,
                    self.arena.across(side, *t),
                )
            })
    }

    /// Sets the map event off from a switch: it lands a tick later, and every switch rests.
    pub(crate) fn set_off(
        &mut self,
        slot: Slot,
        switch: Tile,
        turn: u8,
        tick: u8,
        now: u16,
        events: &mut Vec<Event>,
    ) {
        let rest = self.arena.scenery.as_ref().map_or(0, |s| s.rest);
        let around = self.arena.scenery.as_ref().is_some_and(|s| s.around);
        let from = if around { vec![switch] } else { Vec::new() };
        let tiles = self.arena.scenery_tiles(&from);
        self.board.event_at = Some(now + 1);
        self.board.event_tiles = tiles.clone();
        self.board.event_from = from;
        self.board.switch_ready_turn = turn.saturating_add(rest).saturating_add(1);
        let (lands_turn, lands_tick) = self.when(now + 1);
        events.push(Event {
            turn,
            tick,
            kind: EventKind::SwitchTurned { slot, switch },
        });
        events.push(Event {
            turn,
            tick,
            kind: EventKind::SceneryWarned {
                tiles,
                lands_turn,
                lands_tick,
            },
        });
    }

    /// Stepped, pushed or hopped into honey, it is stuck there for the rest of the turn.
    pub(crate) fn stick(&mut self, slot: Slot, turn: u8, tick: u8, events: &mut Vec<Event>) {
        let at = self.fighter(slot).at;
        if self.fighter(slot).stuck || self.fighter(slot).out || !self.sticky(at) {
            return;
        }
        self.fighter_mut(slot).stuck = true;
        events.push(Event {
            turn,
            tick,
            kind: EventKind::Stuck { slot, at },
        });
    }

    /// Picks up whatever lies where it stands, if its paws are empty.
    pub(crate) fn pick_up(&mut self, slot: Slot, turn: u8, tick: u8, events: &mut Vec<Event>) {
        let f = self.fighter(slot);
        if f.out || f.carrying.is_some() {
            return;
        }
        let at = f.at;
        let Some(i) = self.board.ground.iter().position(|(g, _)| *g == at) else {
            return;
        };
        let (_, item) = self.board.ground.remove(i);
        self.fighter_mut(slot).carrying = Some(item);
        events.push(Event {
            turn,
            tick,
            kind: EventKind::PickedUp { slot, item, at },
        });
    }

    /// Knocked off its feet, it lets go of what it carries: it lies where it ends up, or is lost
    /// if something already lies there.
    pub(crate) fn drop_carried(&mut self, slot: Slot, turn: u8, tick: u8, events: &mut Vec<Event>) {
        let f = self.fighter_mut(slot);
        let Some(item) = f.carrying.take() else {
            return;
        };
        let here = f.at;
        let at = (!self.board.ground.iter().any(|(g, _)| *g == here)).then_some(here);
        if let Some(at) = at {
            self.board.ground.push((at, item));
        }
        events.push(Event {
            turn,
            tick,
            kind: EventKind::Dropped { slot, item, at },
        });
    }

    /// Uses what it carries on `at`: it is marked on its tiles now and lands next tick.
    pub(crate) fn use_item(
        &mut self,
        slot: Slot,
        at: Tile,
        turn: u8,
        tick: u8,
        now: u16,
        events: &mut Vec<Event>,
    ) {
        let f = self.fighter_mut(slot);
        f.spent = true;
        let Some(item) = f.carrying.take() else {
            return;
        };
        let from = f.at;
        let tiles = match item {
            Item::Puff => items::puff_tiles(from, at),
            Item::Honey => items::honey_tiles(at),
            Item::Bell => vec![from],
            Item::Twig | Item::Leaf | Item::Seed => vec![at],
        };
        if tiles.is_empty() {
            events.push(Event {
                turn,
                tick,
                kind: EventKind::ItemFizzled { slot, item, at },
            });
            return;
        }
        self.board.pending.push(Pending {
            at: now + 1,
            by: slot,
            item,
            from,
            target: at,
        });
        events.push(Event {
            turn,
            tick,
            kind: EventKind::ItemUsed {
                slot,
                item,
                at,
                tiles,
            },
        });
    }

    /// A puff lands: everyone on its tiles is blown one tile on (two under a leaf umbrella,
    /// which catches it like a sail) and knocked off balance, unless it is holding with a holder
    /// of its team beside it, corner to corner, to brace against. Twigs there blow over.
    fn puff(&mut self, p: Pending, turn: u8, tick: u8, now: u16, events: &mut Vec<Event>) {
        let Some((dx, dy)) = items::blowing(p.from, p.target) else {
            return;
        };
        let mut blown = Vec::new();
        let mut braced = Vec::new();
        let mut twigs = Vec::new();
        for tile in items::puff_tiles(p.from, p.target) {
            if let Some(i) = self.board.twigs.iter().position(|(t, _)| *t == tile) {
                self.board.twigs.remove(i);
                twigs.push(tile);
            }
            let Some(slot) = self.occupant(tile) else {
                continue;
            };
            let f = self.fighter(slot);
            let backed = f.holding
                && self.team(slot.side).any(|h| {
                    h != slot && self.fighter(h).holding && self.fighter(h).at.beside(tile)
                });
            if backed {
                braced.push(slot);
                continue;
            }
            let sailed = f.sheltered(now);
            let mut to = None;
            for n in 1..=(if sailed { 2 } else { 1 }) {
                let next = Tile::new(tile.x + dx * n, tile.y + dy * n);
                if !self.arena.contains(next)
                    || !self.walkable(next)
                    || self.occupant(next).is_some()
                {
                    break;
                }
                to = Some(next);
                // Honey stops it where it lands.
                if self.sticky(next) {
                    break;
                }
            }
            let f = self.fighter_mut(slot);
            f.holding = false;
            f.interrupted = true;
            if let Some(to) = to {
                f.at = to;
            }
            blown.push(Blown {
                slot,
                from: tile,
                to,
                sailed,
            });
        }
        events.push(Event {
            turn,
            tick,
            kind: EventKind::Puffed {
                by: p.by,
                blown: blown.clone(),
                braced,
                twigs,
            },
        });
        for b in blown {
            self.stick(b.slot, turn, tick, events);
            self.drop_carried(b.slot, turn, tick, events);
        }
    }

    /// A twig lands: wedged into its tile for `items::TWIG_TURNS` turns, if nobody and nothing is
    /// there, anyone could stand there and it is not sticky with honey; or, thrown into water,
    /// floating there as a raft anyone can stand on until the end of the next turn.
    fn wedge(&mut self, p: Pending, turn: u8, tick: u8, events: &mut Vec<Event>) {
        let at = p.target;
        let kind = if self.floats(at) && !self.board.rafts.iter().any(|(t, _)| *t == at) {
            let until_turn = turn.saturating_add(1);
            let until = self.clock(until_turn, self.ruleset.ticks_per_turn);
            self.board.rafts.push((at, until));
            EventKind::RaftFloated {
                by: p.by,
                at,
                until_turn,
            }
        } else if self.arena.contains(at)
            && self.walkable(at)
            && self.occupant(at).is_none()
            && !self.sticky(at)
            && !self.board.rafts.iter().any(|(t, _)| *t == at)
            && !self.board.ground.iter().any(|(g, _)| *g == at)
        {
            let until_turn = turn.saturating_add(items::TWIG_TURNS);
            let until = self.clock(until_turn, self.ruleset.ticks_per_turn);
            self.board.twigs.push((at, until));
            EventKind::TwigWedged {
                by: p.by,
                at,
                until_turn,
            }
        } else {
            EventKind::ItemFizzled {
                slot: p.by,
                item: p.item,
                at,
            }
        };
        events.push(Event { turn, tick, kind });
    }

    /// A pebble bell rings where its user stands: everyone of the other team within
    /// `items::BELL_REACH` who has not settled a contest this turn loses it. Rung beside a switch,
    /// it sets the map event off too, if the switch is ready.
    fn ring(&mut self, p: Pending, turn: u8, tick: u8, now: u16, events: &mut Vec<Event>) {
        if !self.standing(p.by) {
            return;
        }
        let at = self.fighter(p.by).at;
        let hushed: Vec<Slot> = self
            .team(p.by.side.other())
            .filter(|e| {
                let f = self.fighter(*e);
                f.at.distance(at) <= items::BELL_REACH
                    && !f.spent
                    && matches!(f.order, Some(Order::Contest { .. }))
            })
            .collect();
        for e in &hushed {
            self.fighter_mut(*e).spent = true;
        }
        events.push(Event {
            turn,
            tick,
            kind: EventKind::Rung {
                by: p.by,
                at,
                hushed,
            },
        });
        let switch = self
            .arena
            .switches
            .iter()
            .copied()
            .filter(|w| w.touches(at))
            .min();
        if let Some(switch) = switch
            && self.switch_ready()
        {
            self.set_off(p.by, switch, turn, tick, now, events);
        }
    }

    /// A honey drop spills over its tile and the four round it, where anyone could stand: sticky
    /// until the end of the turn `items::HONEY_TURNS` on. Anyone already standing there is stuck.
    fn spill(&mut self, p: Pending, turn: u8, tick: u8, events: &mut Vec<Event>) {
        let tiles: Vec<Tile> = items::honey_tiles(p.target)
            .into_iter()
            .filter(|t| self.arena.contains(*t) && self.walkable(*t) && !self.sticky(*t))
            .filter(|t| !self.board.rafts.iter().any(|(r, _)| r == t))
            .collect();
        if tiles.is_empty() {
            events.push(Event {
                turn,
                tick,
                kind: EventKind::ItemFizzled {
                    slot: p.by,
                    item: p.item,
                    at: p.target,
                },
            });
            return;
        }
        let until_turn = turn.saturating_add(items::HONEY_TURNS);
        let until = self.clock(until_turn, self.ruleset.ticks_per_turn);
        self.board.honey.extend(tiles.iter().map(|t| (*t, until)));
        events.push(Event {
            turn,
            tick,
            kind: EventKind::HoneySpilled {
                by: p.by,
                tiles: tiles.clone(),
                until_turn,
            },
        });
        for slot in self.standing_slots().collect::<Vec<_>>() {
            if tiles.contains(&self.fighter(slot).at) {
                self.stick(slot, turn, tick, events);
            }
        }
    }

    /// A springy seed hops its user onto the tile, if it is free ground no more than two tiles
    /// off and the way there does not cross a wall or water: over anyone, or a twig, in between.
    fn hop(&mut self, p: Pending, turn: u8, tick: u8, events: &mut Vec<Event>) {
        if !self.standing(p.by) {
            return;
        }
        let from = self.fighter(p.by).at;
        let to = p.target;
        let solid = |m: &Self, t: Tile| {
            m.arena.terrain(t) != Terrain::Open
                && m.arena.terrain(t) != Terrain::Gate
                && !m.board.rafts.iter().any(|(r, _)| *r == t)
        };
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let over_clear = match (dx.abs(), dy.abs()) {
            (2, 0) | (0, 2) => !solid(self, Tile::new(from.x + dx / 2, from.y + dy / 2)),
            (1, 1) => {
                !solid(self, Tile::new(from.x + dx, from.y))
                    || !solid(self, Tile::new(from.x, from.y + dy))
            }
            (1, 0) | (0, 1) => true,
            _ => false,
        };
        let lands = over_clear
            && self.arena.contains(to)
            && self.walkable(to)
            && self.occupant(to).is_none();
        if !lands {
            events.push(Event {
                turn,
                tick,
                kind: EventKind::ItemFizzled {
                    slot: p.by,
                    item: p.item,
                    at: to,
                },
            });
            return;
        }
        self.fighter_mut(p.by).at = to;
        events.push(Event {
            turn,
            tick,
            kind: EventKind::Hopped {
                slot: p.by,
                from,
                to,
            },
        });
        self.pick_up(p.by, turn, tick, events);
        self.stick(p.by, turn, tick, events);
    }

    /// A leaf umbrella opens over whoever of its user's team stands on its tile, until the end of
    /// the next turn.
    fn shelter(&mut self, p: Pending, turn: u8, tick: u8, events: &mut Vec<Event>) {
        let kind = match self.occupant(p.target).filter(|o| o.side == p.by.side) {
            Some(slot) => {
                let until_turn = turn.saturating_add(1);
                self.fighter_mut(slot).shelter_until =
                    self.clock(until_turn, self.ruleset.ticks_per_turn);
                EventKind::Sheltering {
                    by: p.by,
                    slots: vec![slot],
                    until_turn,
                }
            }
            None => EventKind::ItemFizzled {
                slot: p.by,
                item: p.item,
                at: p.target,
            },
        };
        events.push(Event { turn, tick, kind });
    }
}
