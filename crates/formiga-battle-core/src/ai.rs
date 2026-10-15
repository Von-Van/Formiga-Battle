//! Orders nobody gave: the neutral fallback for an order missing when the timer runs out, and the
//! coaches that give a whole team's orders in an AI match or a tournament.

use crate::arena::TEAM_SIZE;
use crate::game::Match;
use crate::grid::Tile;
use crate::interpret::distances;
use crate::order::{Order, Orders, Side, Slot, Target};
use crate::rng::{self, Purpose};

/// The order a creature gives itself when its Handler gave none in time. It is the same for both
/// teams and only looks at the board as it stands and at the creature's own temperament: in the
/// zone it holds; next to someone of the other team who is not holding, a cautious creature holds
/// its ground and a bold one contests them; anywhere else it heads for the nearest free tile of
/// the zone to hold it.
pub fn fallback(m: &Match, slot: Slot) -> Order {
    let f = m.fighter(slot);
    if m.arena.in_zone(f.at) {
        return Order::Hold { at: f.at };
    }
    let open_enemy = m
        .team(slot.side.other())
        .find(|e| m.fighter(*e).at.touches(f.at) && !m.fighter(*e).holding);
    if let Some(enemy) = open_enemy {
        return if m.behavior(slot).caution > 0 {
            Order::Hold { at: f.at }
        } else {
            Order::Contest {
                target: Target::Creature { slot: enemy },
            }
        };
    }
    Order::Hold {
        at: nearest_free_zone_tile(m, slot, &[]).unwrap_or(f.at),
    }
}

/// The free zone tile this creature can reach soonest, the zone's middle first among equals,
/// passing over any already `taken` by a teammate's order.
pub fn nearest_free_zone_tile(m: &Match, slot: Slot, taken: &[Tile]) -> Option<Tile> {
    let from = m.fighter(slot).at;
    let dist = distances(m, slot, &[from], false);
    let w = m.arena.width as usize;
    let centre = m.arena.zone_centre();
    m.arena
        .zone
        .iter()
        .copied()
        .filter(|t| m.occupant(*t).is_none_or(|o| o == slot) && !taken.contains(t))
        .filter_map(|t| dist[t.y as usize * w + t.x as usize].map(|d| (t, d)))
        .min_by_key(|(t, d)| (*d, t.distance(centre), t.y, t.x))
        .map(|(t, _)| t)
}

/// Who gives a team its orders in a match nobody is playing by hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coach {
    /// Gives no orders at all, so every creature falls back on its own.
    Absent,
    /// Plays the beacon: one creature holds it, two press a holder of the other team in it who
    /// has nobody holding beside it, and the rest guard, chase or try the lever, chosen with a
    /// little chance so it cannot be read like a script. It knows nothing about any creature's
    /// temperament.
    Exhibition,
    /// Thinks each turn through, knowing exactly how each of its creatures carries out an order.
    Familiar,
    /// Thinks each turn through the same way, imagining every creature as one that leans no way
    /// at all: the measure `Familiar` is held against.
    Generic,
}

impl Coach {
    pub fn orders(self, m: &Match, side: Side) -> Orders {
        match self {
            Self::Absent => [None; TEAM_SIZE],
            Self::Exhibition => exhibition(m, side),
            Self::Familiar => crate::plan::think_ahead(m, side, true),
            Self::Generic => crate::plan::think_ahead(m, side, false),
        }
    }
}

fn exhibition(m: &Match, side: Side) -> Orders {
    let mut orders: Orders = [None; TEAM_SIZE];
    let mut free: Vec<Slot> = m.team(side).collect();
    if free.is_empty() {
        return orders;
    }
    let turn = u64::from(m.board.turn) + 1;
    let roll = |k: u64| rng::roll(m.seed, Purpose::Coach, &[turn, side.index() as u64, k]) % 100;
    let put = |orders: &mut Orders, slot: Slot, order: Order| {
        orders[slot.index as usize] = Some(order);
    };
    let zone_dist = distances(m, free[0], &m.arena.zone, false);
    let w = m.arena.width as usize;
    let to_zone =
        |s: &Slot| zone_dist[m.fighter(*s).at.y as usize * w + m.fighter(*s).at.x as usize];

    // The anchor holds the beacon.
    free.sort_by_key(|s| (to_zone(s).unwrap_or(u8::MAX), *s));
    let anchor = free.remove(0);
    let anchor_tile = nearest_free_zone_tile(m, anchor, &[]).unwrap_or(m.fighter(anchor).at);
    put(&mut orders, anchor, Order::Hold { at: anchor_tile });
    let mut taken = vec![anchor_tile];

    // Press someone of the other team in the zone, always two together: one contest against a
    // holder is only ever turned away.
    let intruders: Vec<Slot> = m
        .team(side.other())
        .filter(|e| m.arena.in_zone(m.fighter(*e).at))
        .collect();
    let lone = |e: &Slot| {
        !m.fighter(*e).holding
            || !m.team(side.other()).any(|o| {
                o != *e && m.fighter(o).holding && m.fighter(o).at.beside(m.fighter(*e).at)
            })
    };
    if free.len() >= 2
        && roll(1) < 80
        && let Some(target) = intruders.iter().copied().filter(lone).min_by_key(|e| {
            (
                free.iter()
                    .map(|s| u32::from(m.fighter(*s).at.distance(m.fighter(*e).at)))
                    .sum::<u32>(),
                *e,
            )
        })
    {
        let there = m.fighter(target).at;
        free.sort_by_key(|s| (m.fighter(*s).at.distance(there), *s));
        for s in free.drain(..2).collect::<Vec<_>>() {
            put(
                &mut orders,
                s,
                Order::Contest {
                    target: Target::Creature { slot: target },
                },
            );
        }
    }

    // Everyone else guards, chases or tries the lever.
    for (k, s) in free.into_iter().enumerate() {
        let at = m.fighter(s).at;
        let chase = m
            .team(side.other())
            .filter(|e| !m.fighter(*e).holding && m.fighter(*e).at.distance(at) <= 4)
            .min_by_key(|e| (m.fighter(*e).at.distance(at), *e));
        let lever = m
            .arena
            .levers
            .first()
            .copied()
            .filter(|_| m.lever_ready() && m.board.gates_open.iter().any(|open| !open));
        let r = roll(10 + k as u64);
        let guard = m
            .arena
            .zone
            .iter()
            .copied()
            .filter(|t| {
                t.touches(anchor_tile)
                    && m.occupant(*t).is_none_or(|o| o == s)
                    && !taken.contains(t)
            })
            .min_by_key(|t| (t.distance(at), t.y, t.x));
        let order = match (chase, lever, guard) {
            (Some(enemy), _, _) if r < 55 => Order::Contest {
                target: Target::Creature { slot: enemy },
            },
            (_, Some(lever), _) if r >= 85 => Order::Contest {
                target: Target::Lever { at: lever },
            },
            (_, _, Some(guard)) => Order::Hold { at: guard },
            _ => Order::Hold {
                at: nearest_free_zone_tile(m, s, &taken).unwrap_or(at),
            },
        };
        if let Order::Hold { at } = order {
            taken.push(at);
        }
        put(&mut orders, s, order);
    }
    orders
}
