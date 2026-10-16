//! What the board does by itself, and what found things do: the arena's map event, items turning
//! up, being picked up, carried, dropped and used.
//!
//! Everything here is warned before it lands, and the warning is on the board for both teams to
//! see: the map event a whole turn before it comes round by itself, or a tick before when a switch
//! sets it off; a found thing glints on its spots a whole turn before it turns up; an item's effect
//! is marked on its tiles the tick it is used. Nothing here costs composure. Its effects are ones
//! a match already has: off balance, a push of a tile or two, a tile nobody can stand on.

use crate::arena::Effect;
use crate::event::{Blown, Event, EventKind};
use crate::game::Match;
use crate::grid::Tile;
use crate::items::{self, Item};
use crate::order::Slot;
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
            // Mid-turn, so anyone sent away has a tick to step off.
            let lands_tick = 2.min(tick);
            self.board.event_at = Some(self.clock(next, lands_tick));
            events.push(Event {
                turn,
                tick,
                kind: EventKind::SceneryWarned {
                    tiles: scenery.tiles,
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

    /// At the start of a tick, after any gate swings: umbrellas fold and twigs fall when their
    /// time is up, glinting things turn up, items used last tick land, and the map event comes.
    pub(crate) fn land(&mut self, turn: u8, tick: u8, now: u16, events: &mut Vec<Event>) {
        let push = |events: &mut Vec<Event>, kind| events.push(Event { turn, tick, kind });
        for slot in self.standing_slots().collect::<Vec<_>>() {
            let f = self.fighter_mut(slot);
            if f.shelter_until != 0 && f.shelter_until < now {
                f.shelter_until = 0;
                push(events, EventKind::ShelterFolded { slot });
            }
        }
        let (fallen, standing): (Vec<_>, Vec<_>) = std::mem::take(&mut self.board.twigs)
            .into_iter()
            .partition(|(_, until)| *until < now);
        self.board.twigs = standing;
        for (at, _) in fallen {
            push(events, EventKind::TwigFell { at });
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
            }
        }
        if self.board.event_at.is_some_and(|at| at <= now)
            && let Some(scenery) = self.arena.scenery.clone()
        {
            self.board.event_at = None;
            let mut hit = Vec::new();
            let mut sheltered = Vec::new();
            for slot in self.standing_slots().collect::<Vec<_>>() {
                let f = self.fighter(slot);
                if !scenery.tiles.contains(&f.at) {
                    continue;
                }
                if f.sheltered(now) {
                    sheltered.push(slot);
                    continue;
                }
                match scenery.effect {
                    Effect::Soak => {
                        let f = self.fighter_mut(slot);
                        f.holding = false;
                        f.interrupted = true;
                    }
                }
                hit.push(slot);
            }
            push(
                events,
                EventKind::SceneryLanded {
                    effect: scenery.effect,
                    tiles: scenery.tiles,
                    hit: hit.clone(),
                    sheltered,
                },
            );
            for slot in hit {
                self.drop_carried(slot, turn, tick, events);
            }
        }
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
            Item::Twig | Item::Leaf => vec![at],
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
            self.drop_carried(b.slot, turn, tick, events);
        }
    }

    /// A twig lands: wedged into its tile for `items::TWIG_TURNS` turns, if nobody and nothing is
    /// there and anyone could stand there.
    fn wedge(&mut self, p: Pending, turn: u8, tick: u8, events: &mut Vec<Event>) {
        let at = p.target;
        let fits = self.arena.contains(at)
            && self.walkable(at)
            && self.occupant(at).is_none()
            && !self.board.ground.iter().any(|(g, _)| *g == at);
        let kind = if fits {
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
