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
    let dist = walks(m, slot);
    let w = m.arena.width as usize;
    let centre = m.arena.zone_centre();
    m.arena
        .zone
        .iter()
        .copied()
        .filter(|t| m.occupant(*t).is_none_or(|o| o == slot) && !taken.contains(t))
        .filter_map(|t| dist[t.y as usize * w + t.x as usize].map(|d| (t, d)))
        .min_by_key(|(t, d)| (*d, t.distance(centre), t.y, m.arena.across(slot.side, *t)))
        .map(|(t, _)| t)
}

/// How many steps every tile is from where this creature stands, walking round anyone standing
/// still; straight through them where there is no way round.
fn walks(m: &Match, slot: Slot) -> Vec<Option<u8>> {
    let from = m.fighter(slot).at;
    let around = distances(m, slot, &[from], true);
    let through = distances(m, slot, &[from], false);
    around
        .into_iter()
        .zip(through)
        .map(|(a, t)| a.or(t.map(|t| t.saturating_add(4))))
        .collect()
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
    /// Digs in: every creature holds a tile of the zone and never leaves it. The strategy the
    /// encirclement rules are tested against.
    Turtle,
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
            Self::Turtle => turtle(m, side),
            Self::Generic => crate::plan::think_ahead(m, side, false),
        }
    }
}

/// Two places to flank a holder from: sides of it that are open (or already held by one of
/// `attackers`) and that no other holder of its team, beside it, also touches, opposite each
/// other if there are such. `None` if it has fewer than two.
pub fn flank_places(m: &Match, target: Slot, attackers: Side) -> Option<[Tile; 2]> {
    let at = m.fighter(target).at;
    let guarded = |t: Tile| {
        m.team(target.side).any(|h| {
            let there = m.fighter(h).at;
            h != target && m.fighter(h).holding && there.beside(at) && there.touches(t)
        })
    };
    let open: Vec<Tile> = at
        .steps()
        .into_iter()
        .filter(|t| {
            m.walkable(*t) && m.occupant(*t).is_none_or(|o| o.side == attackers) && !guarded(*t)
        })
        .collect();
    // Places inside the zone first: standing there, a flanker also keeps the zone contested.
    let mut open = open;
    open.sort_by_key(|t| (!m.arena.in_zone(*t), t.y, m.arena.across(attackers, *t)));
    let inside = |p: &[Tile; 2]| p.iter().filter(|t| m.arena.in_zone(**t)).count();
    let opposite = open
        .iter()
        .filter_map(|a| {
            let b = Tile::new(2 * at.x - a.x, 2 * at.y - a.y);
            open.contains(&b).then_some([*a, b])
        })
        .max_by_key(|p| inside(p));
    opposite.or_else(|| (open.len() >= 2).then(|| [open[0], open[1]]))
}

/// Whether a holder would be flanked if everyone of the other team standing beside it contested
/// it now: two or more on its sides, more than the holders of its own team, itself included,
/// that would turn them away.
pub fn flank_threatened(m: &Match, target: Slot) -> bool {
    let f = m.fighter(target);
    if !f.holding {
        return false;
    }
    let at = f.at;
    let around: Vec<Tile> = m
        .team(target.side.other())
        .map(|e| m.fighter(e).at)
        .filter(|t| t.touches(at))
        .collect();
    let guards = m
        .team(target.side)
        .filter(|h| {
            *h == target
                || (m.fighter(*h).holding
                    && m.fighter(*h).at.beside(at)
                    && around.iter().any(|t| t.touches(m.fighter(*h).at)))
        })
        .count();
    around.len() >= 2 && around.len() > guards
}

fn turtle(m: &Match, side: Side) -> Orders {
    let mut orders: Orders = [None; TEAM_SIZE];
    let mut taken = Vec::new();
    for s in m.team(side) {
        let at = m.fighter(s).at;
        let tile = if m.arena.in_zone(at) {
            at
        } else {
            nearest_free_zone_tile(m, s, &taken).unwrap_or(at)
        };
        taken.push(tile);
        orders[s.index as usize] = Some(Order::Hold { at: tile });
    }
    orders
}

/// Two creatures and the side each is to take.
type Flankers = [(Slot, Tile); 2];

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
    let centre = m.arena.zone_centre();
    let w = m.arena.width as usize;
    let walk: Vec<(Slot, Vec<Option<u8>>)> = free.iter().map(|s| (*s, walks(m, *s))).collect();
    let steps = |s: Slot, t: Tile| {
        walk.iter()
            .find(|(w, _)| *w == s)
            .and_then(|(_, d)| d[t.y as usize * w + t.x as usize])
            .map_or(u32::from(u8::MAX), u32::from)
    };

    // The anchor keeps the beacon contested: whoever already holds in the zone stays put, and
    // otherwise whoever can walk in soonest does, and holds.
    let holding_in_zone = free
        .iter()
        .copied()
        .filter(|s| m.fighter(*s).holding && m.arena.in_zone(m.fighter(*s).at))
        .min_by_key(|s| (m.fighter(*s).at.distance(centre), *s));
    let (anchor, anchor_tile) = match holding_in_zone {
        Some(s) => (s, m.fighter(s).at),
        None => free
            .iter()
            .filter_map(|s| nearest_free_zone_tile(m, *s, &[]).map(|t| (*s, t)))
            .min_by_key(|(s, t)| (steps(*s, *t), t.distance(centre), *s))
            .unwrap_or((free[0], m.fighter(free[0]).at)),
    };
    free.retain(|s| *s != anchor);
    put(&mut orders, anchor, Order::Hold { at: anchor_tile });
    let mut taken = vec![anchor_tile];

    // Clear the zone of the other team. A holder in it is flanked: two take up places on two
    // different sides no other holder guards, opposite sides if they can, and contest together
    // from there. Walking into place first is what makes it work: contests already in reach
    // land before anyone steps.
    let in_zone: Vec<Slot> = m
        .team(side.other())
        .filter(|e| m.arena.in_zone(m.fighter(*e).at))
        .collect();
    let holds =
        |e: Slot| m.fighter(e).holding || matches!(m.fighter(e).order, Some(Order::Hold { .. }));
    if free.len() >= 2 {
        // For each holder that can be flanked, the two free who would take the least walking
        // between them to get there, each to the place that makes the pair's walk shortest.
        let mut plans: Vec<(u32, Slot, Flankers)> = Vec::new();
        for e in in_zone.iter().copied().filter(|e| holds(*e)) {
            let Some(places) = flank_places(m, e, side) else {
                continue;
            };
            for (i, a) in free.iter().copied().enumerate() {
                for b in free.iter().copied().skip(i + 1) {
                    let straight = steps(a, places[0]) + steps(b, places[1]);
                    let crossed = steps(a, places[1]) + steps(b, places[0]);
                    let (cost, pair) = if straight <= crossed {
                        (straight, [(a, places[0]), (b, places[1])])
                    } else {
                        (crossed, [(a, places[1]), (b, places[0])])
                    };
                    plans.push((cost, e, pair));
                }
            }
        }
        // The cheapest, which is the one already furthest along, so the two keep walking to the
        // same places rather than changing their minds as they go; the middle first among
        // equals.
        let best = plans
            .into_iter()
            .filter(|(cost, _, _)| *cost <= 8)
            .min_by_key(|(cost, e, _)| (*cost, m.fighter(*e).at.distance(centre), *e));
        if let Some((_, target, pair)) = best {
            let in_place = pair.iter().all(|(s, place)| m.fighter(*s).at == *place);
            for (s, place) in pair {
                free.retain(|f| *f != s);
                taken.push(place);
                let order = if in_place {
                    Order::Contest {
                        target: Target::Creature { slot: target },
                    }
                } else {
                    // Into place, and wait there holding for the other: a holder stays put
                    // where a walker with time to spare might wander off.
                    Order::Hold { at: place }
                };
                put(&mut orders, s, order);
            }
        }
    }

    // Anyone of theirs in the zone who is not holding is simply contested.
    let loose = in_zone
        .iter()
        .copied()
        .filter(|e| !holds(*e))
        .min_by_key(|e| (m.fighter(*e).at.distance(centre), *e));
    if let Some(target) = loose
        && !free.is_empty()
        && roll(2) < 80
    {
        let there = m.fighter(target).at;
        free.sort_by_key(|s| (steps(*s, there), *s));
        let s = free.remove(0);
        put(
            &mut orders,
            s,
            Order::Contest {
                target: Target::Creature { slot: target },
            },
        );
    }

    // Everyone else guards, chases or tries the lever.
    for (k, s) in free.into_iter().enumerate() {
        let at = m.fighter(s).at;
        let chase = m
            .team(side.other())
            .filter(|e| !m.fighter(*e).holding && steps(s, m.fighter(*e).at) <= 4)
            .min_by_key(|e| (steps(s, m.fighter(*e).at), *e));
        let lever = m
            .arena
            .levers
            .first()
            .copied()
            .filter(|_| m.lever_ready() && m.board.gates_open.iter().any(|open| !open));
        let r = roll(10 + k as u64);
        // Corner to corner with the anchor, where a flank on either is turned away by both.
        let guard = m
            .arena
            .tiles()
            .filter(|t| {
                t.beside(anchor_tile)
                    && !t.touches(anchor_tile)
                    && m.walkable(*t)
                    && m.occupant(*t).is_none_or(|o| o == s)
                    && !taken.contains(t)
            })
            .min_by_key(|t| {
                (
                    !m.arena.in_zone(*t),
                    steps(s, *t),
                    t.y,
                    m.arena.across(side, *t),
                )
            });
        let order = match (chase, lever, guard) {
            (Some(enemy), _, _) if r < 45 => Order::Contest {
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
