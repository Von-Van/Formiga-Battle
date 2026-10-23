//! Orders nobody gave: the neutral fallback for a creature with no order when the timer runs out,
//! and the coaches that give a whole team its plan in an AI match or a tournament.

use crate::game::Match;
use crate::grid::Tile;
use crate::interpret::distances;
use crate::order::{Directive, Facing, Order, Side, Slot, TeamPlan};
use crate::rng::{self, Purpose};
use crate::style::{Archetype, Book};

/// The order a creature gives itself when its Handler gave none in time. It is the same for both
/// teams and only looks at the board as it stands and at the creature's own temperament: in the
/// zone it holds; next to someone of the other team who is not holding, a cautious creature holds
/// its ground and a bold one presses them; anywhere else it heads for the nearest free tile of
/// the zone to hold it. It holds facing whichever way the other team is nearest.
pub fn fallback(m: &Match, slot: Slot) -> Order {
    let f = m.fighter(slot);
    let hold = |at: Tile| Order::Hold {
        at,
        facing: face(m, slot.side, at),
    };
    if m.arena.in_zone(f.at) {
        return hold(f.at);
    }
    let open_enemy = m
        .team(slot.side.other())
        .find(|e| m.fighter(*e).at.touches(f.at) && !m.fighter(*e).holding);
    if let Some(enemy) = open_enemy {
        return if m.behavior(slot).caution > 0 {
            hold(f.at)
        } else {
            Order::Press { target: enemy }
        };
    }
    hold(nearest_free_zone_tile(m, slot, &[]).unwrap_or(f.at))
}

/// The way to hold `at` facing: toward the nearest of the other team, the one nearer the middle
/// of the board first among equals, then the upper one, then the one nearer its own side;
/// toward the other team's side of the board if nobody of theirs is standing anywhere else.
pub fn face(m: &Match, side: Side, at: Tile) -> Facing {
    m.team(side.other())
        .map(|e| m.fighter(e).at)
        .filter(|t| *t != at)
        .min_by_key(|t| {
            (
                t.distance(at),
                t.distance(m.arena.zone_centre()),
                t.y,
                m.arena.across(side, *t),
            )
        })
        .and_then(|t| Facing::toward(at, t))
        .unwrap_or(Facing::across(side))
}

/// The free zone tile this creature can reach soonest, the zone's middle first among equals,
/// passing over any already `taken` by a teammate's order, and over any marked for the map event
/// while another will do.
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
        .min_by_key(|(t, d)| {
            (
                crate::interpret::warned(m, *t),
                *d,
                t.distance(centre),
                t.y,
                m.arena.across(slot.side, *t),
            )
        })
        .map(|(t, _)| t)
}

/// How many steps every tile is from where this creature stands, walking round anyone standing
/// still; straight through them where there is no way round.
pub(crate) fn walks(m: &Match, slot: Slot) -> Vec<Option<u8>> {
    let from = m.fighter(slot).at;
    let around = distances(m, slot, &[from], true);
    let through = distances(m, slot, &[from], false);
    around
        .into_iter()
        .zip(through)
        .map(|(a, t)| a.or(t.map(|t| t.saturating_add(4))))
        .collect()
}

/// Who gives a team its plan in a match nobody is playing by hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coach {
    /// Gives no plan at all, so every creature falls back on its own.
    Absent,
    /// Plays the beacon: presses onto a square of it (or holds the one already held, facing the
    /// other team), flanks or tricks any of theirs holding in it and presses any of theirs in it
    /// not holding, and with what is left goes for the tap, something lying near, the lever or one
    /// of theirs wandering close, chosen with a little chance so it cannot be read like a script;
    /// anything carried is used where it plainly helps. It knows nothing about any creature's
    /// temperament: the team shares the plan out by itself.
    Exhibition,
    /// Thinks each turn through, knowing exactly how each of its creatures will share out and
    /// carry out a plan.
    Familiar,
    /// Digs in: holds a tile of the zone for every creature, all facing the other team's side,
    /// and never leaves them. The strategy flanks and tricks are tested against.
    Turtle,
    /// Thinks each turn through the same way, imagining every creature as one that leans no way
    /// at all: the measure `Familiar` is held against.
    Generic,
    /// Plays the Exhibition coach's book with one of the four styles' numbers (`style.rs`): a
    /// Handler who can be told from another by what they order.
    Styled(Archetype),
}

impl Coach {
    pub fn plan(self, m: &Match, side: Side) -> TeamPlan {
        match self {
            Self::Absent => Vec::new(),
            Self::Exhibition => book(m, side, &Book::EXHIBITION),
            Self::Styled(archetype) => book(m, side, &archetype.book()),
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
            h != target && crate::muster::holds(m, h) && there.beside(at) && there.touches(t)
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

/// Whether a holder would be flanked if everyone of the other team standing beside it pressed it
/// now: two or more on its sides, more than the holders of its own team, itself included, that
/// would turn them away.
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

fn turtle(m: &Match, side: Side) -> TeamPlan {
    let mut squares = Vec::new();
    for s in m.team(side) {
        let at = m.fighter(s).at;
        let tile = if m.arena.in_zone(at) {
            at
        } else {
            nearest_free_zone_tile(m, s, &squares).unwrap_or(at)
        };
        if !squares.contains(&tile) {
            squares.push(tile);
        }
    }
    if squares.is_empty() {
        return Vec::new();
    }
    vec![Directive::Hold {
        squares,
        facing: Facing::across(side),
    }]
}

/// The opening book: the Exhibition coach's way of playing, with `style`'s numbers for every
/// chance it takes. Every roll is numbered the same whatever the style, so the Exhibition
/// coach's matches are the same as they were before there were styles.
fn book(m: &Match, side: Side, style: &Book) -> TeamPlan {
    let mut plan: TeamPlan = Vec::new();
    let team: Vec<Slot> = m.team(side).collect();
    if team.is_empty() {
        return plan;
    }
    let turn = u64::from(m.board.turn) + 1;
    let roll = |k: u64| rng::roll(m.seed, Purpose::Coach, &[turn, side.index() as u64, k]) % 100;
    let centre = m.arena.zone_centre();
    let w = m.arena.width as usize;
    let walk: Vec<(Slot, Vec<Option<u8>>)> = team.iter().map(|s| (*s, walks(m, *s))).collect();
    let steps = |s: Slot, t: Tile| {
        walk.iter()
            .find(|(w, _)| *w == s)
            .and_then(|(_, d)| d[t.y as usize * w + t.x as usize])
            .map_or(u32::from(u8::MAX), u32::from)
    };
    let nearest = |t: Tile| team.iter().map(|s| steps(*s, t)).min().unwrap_or(u32::MAX);

    // The beacon: hold the square of it one of ours already holds, facing the other team, or press
    // onto the free one the team can walk to soonest.
    let held = team
        .iter()
        .copied()
        .filter(|s| m.arena.in_zone(m.fighter(*s).at))
        .min_by_key(|s| {
            let at = m.fighter(*s).at;
            (
                crate::interpret::warned(m, at),
                !m.fighter(*s).holding,
                at.distance(centre),
                *s,
            )
        });
    // A free square of the zone the team can walk to soonest, other than any in `not`.
    let free_square = |not: &[Tile]| {
        team.iter()
            .filter_map(|s| nearest_free_zone_tile(m, *s, not).map(|t| (*s, t)))
            .min_by_key(|(s, t)| (steps(*s, *t), t.distance(centre), *s))
            .map(|(_, t)| t)
    };
    let anchor_order;
    let anchor = match held {
        Some(s) => {
            let at = m.fighter(s).at;
            anchor_order = Directive::Hold {
                squares: vec![at],
                facing: face(m, side, at),
            };
            plan.push(anchor_order.clone());
            at
        }
        None => {
            let tile = free_square(&[]).unwrap_or(centre);
            anchor_order = Directive::Press {
                squares: vec![tile],
            };
            plan.push(anchor_order.clone());
            tile
        }
    };

    // Clear the zone of the other team: a holder of theirs in it, which the team flanks or tricks,
    // or one of theirs in it not holding, which it presses, most of the time.
    let in_zone: Vec<Slot> = m
        .team(side.other())
        .filter(|e| m.arena.in_zone(m.fighter(*e).at))
        .collect();
    let holds = |e: Slot| crate::muster::holds(m, e);
    let flankable = in_zone
        .iter()
        .copied()
        .filter(|e| holds(*e) && flank_places(m, *e, side).is_some())
        .min_by_key(|e| (m.fighter(*e).at.distance(centre), *e));
    let dug_in = in_zone
        .iter()
        .copied()
        .filter(|e| holds(*e))
        .min_by_key(|e| (m.fighter(*e).at.distance(centre), *e));
    let loose = in_zone
        .iter()
        .copied()
        .filter(|e| !holds(*e))
        .min_by_key(|e| (m.fighter(*e).at.distance(centre), *e));
    // Each order takes a creature, a flank two; the team has only so many, and a style may keep
    // some at home beyond the one on the anchor.
    let mut hands = team.len().saturating_sub(1 + usize::from(style.keep_home));
    // A style that goes round rather than through tricks a holder before it thinks of flanking.
    let tricking_first =
        dug_in.filter(|_| style.prefers_trick && hands >= 1 && roll(4) < u64::from(style.trick));
    let flanking = flankable
        .filter(|_| tricking_first.is_none() && hands >= 2 && roll(1) < u64::from(style.flank));
    if let Some(e) = tricking_first {
        hands -= 1;
        plan.push(Directive::Trick {
            squares: vec![m.fighter(e).at],
        });
    } else if let Some(e) = flanking {
        hands -= 2;
        plan.push(Directive::Press {
            squares: vec![m.fighter(e).at],
        });
        // The flank takes two; named twice, the beacon's square keeps the third where it is.
        plan.push(anchor_order.clone());
    } else if let Some(e) = dug_in
        && hands >= 1
        && roll(4) < u64::from(style.trick)
    {
        hands -= 1;
        plan.push(Directive::Trick {
            squares: vec![m.fighter(e).at],
        });
    } else if let Some(e) = loose
        && hands >= 1
        && roll(2) < u64::from(style.press_loose)
    {
        hands -= 1;
        plan.push(Directive::Press {
            squares: vec![m.fighter(e).at],
        });
    }

    // Keep a second of ours in the zone, so the other team cannot light the beacon by holding
    // it while ours are elsewhere.
    let in_zone_ours = team
        .iter()
        .filter(|s| m.arena.in_zone(m.fighter(**s).at))
        .count();
    if plan.len() < crate::order::PLAN_SIZE
        && hands >= 1
        && in_zone_ours < 2
        && let Some(t) = free_square(&[anchor])
        && roll(3) < u64::from(style.second_home)
    {
        hands -= 1;
        plan.push(Directive::Press { squares: vec![t] });
    }

    // With what is left: the tap, something lying near, one of theirs wandering close, the lever;
    // or a second hand at the beacon.
    let soaks = |sd: Side| {
        m.team(sd)
            .filter(|x| m.arena.in_scenery(m.fighter(*x).at))
            .count()
    };
    let tap = m
        .arena
        .switches
        .iter()
        .copied()
        .filter(|_| m.switch_ready() && soaks(side.other()) > 0 && soaks(side) == 0)
        .min_by_key(|t| {
            let reach = t
                .steps()
                .into_iter()
                .filter(|n| m.arena.contains(*n))
                .map(&nearest)
                .min();
            (reach, *t)
        });
    let find = m
        .board
        .ground
        .iter()
        .chain(m.board.glint.iter())
        .map(|(t, _)| *t)
        .filter(|t| {
            team.iter()
                .any(|s| m.fighter(*s).carrying.is_none() && steps(*s, *t) <= 3)
        })
        .min_by_key(|t| (nearest(*t), m.arena.across(side, *t), t.y));
    let chase = m
        .team(side.other())
        .filter(|e| {
            !m.fighter(*e).holding && !in_zone.contains(e) && nearest(m.fighter(*e).at) <= 4
        })
        .min_by_key(|e| (nearest(m.fighter(*e).at), *e));
    let lever = m
        .arena
        .levers
        .first()
        .copied()
        .filter(|_| m.lever_ready() && m.board.gates_open.iter().any(|open| !open));
    for k in 10..20 {
        if plan.len() >= crate::order::PLAN_SIZE || hands == 0 {
            break;
        }
        let r = roll(k);
        let next = if let Some(t) = tap.filter(|_| r >= u64::from(style.tap_from)) {
            Directive::Interact { at: t }
        } else if let Some(t) = find.filter(|_| r < u64::from(style.find_below)) {
            Directive::Interact { at: t }
        } else if let Some(e) = chase.filter(|_| r < u64::from(style.chase_below)) {
            Directive::Press {
                squares: vec![m.fighter(e).at],
            }
        } else if let Some(t) = lever.filter(|_| r >= u64::from(style.lever_from)) {
            Directive::Interact { at: t }
        } else {
            Directive::Hold {
                squares: vec![anchor],
                facing: face(m, side, anchor),
            }
        };
        if plan.contains(&next) && !matches!(next, Directive::Hold { .. }) {
            continue;
        }
        plan.push(next);
        hands -= 1;
    }
    plan.extend(item_uses(m, side, &[], |k| {
        roll(30 + k) < u64::from(style.item_use)
    }));
    plan
}

/// A USE ITEM for each of the team carrying something with a plain use for it near `squares` (the
/// zone, if none), where `go` says so for its place in the team.
pub fn item_uses(m: &Match, side: Side, squares: &[Tile], go: impl Fn(u64) -> bool) -> TeamPlan {
    m.team(side)
        .filter(|s| go(u64::from(s.index)))
        .filter_map(|s| {
            crate::muster::use_for(m, s, squares, false)
                .map(|at| Directive::UseItem { by: s.index, at })
        })
        .collect()
}
