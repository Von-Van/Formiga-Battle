//! How a creature carries out its order: the one place its profile meets the board.
//!
//! Every tick a creature still on its way looks at the steps that bring it closer, and at a
//! sideways step if it has detours left, and weighs them by what it is like: whether a step puts
//! it within reach of the other team, beside a friend, along the outside, against a wall, on a
//! tile marked for something about to land, or onto something lying on the ground. The
//! weights come from `Behavior`, pulled `PULL` as hard; the board decides which choices exist at
//! all. Every step later than the quickest way is weighed against them, and a sideways step costs
//! a tick, so a creature that goes round is later for it.
//!
//! A creature never chooses whether to obey. Its order sets where it is going and what it does
//! there; its profile only picks the way, and what to do with a moment to spare once it has
//! arrived.

use crate::event::Reason;
use crate::game::Match;
use crate::grid::Tile;
use crate::order::{Order, Slot, Target};
use crate::rng::{self, Purpose};

/// How hard a creature's leanings pull on the way it walks, against arriving on time, in percent
/// of what `Behavior` gives. Only about one step in ten has two ways there that arrive equally
/// soon, so beyond that every way a profile chooses costs time. At 400 about one creature of a
/// team's three does something of its own each turn (sometimes none, sometimes two); at 100
/// (rules version 4) it was about one team-turn in four. What a creature does with a moment to
/// spare is weighed as `Behavior` gives it.
const PULL: i32 = 400;

/// What it takes to shift from where it arrived with a moment to spare.
const SHIFT_COST: i32 = 150;

/// One step a creature means to take this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    pub to: Tile,
    pub reason: Reason,
    /// A sideways step, out of its detour budget.
    pub detour: bool,
    /// Its one follow-up step this turn.
    pub follow_up: bool,
}

/// How cautious a creature must be to hang back rather than walk in among the other team.
pub const WARY: i16 = 150;

/// Whether a wary creature, sent to walk or hold, would rather not take this step: it would put
/// it within reach of more of the other team than it is now. It waits one tick, once a turn, and
/// then goes on: a Handler who knows it can plan for the tick it loses. Told to contest, it goes
/// anyway: that is what it was sent for.
fn hangs_back(m: &Match, slot: Slot, to: Tile) -> bool {
    let f = m.fighter(slot);
    if m.behavior(slot).caution < WARY
        || f.hung_back
        || !matches!(f.order, Some(Order::Move { .. }))
    {
        return false;
    }
    let crowd = |t: Tile| {
        m.team(slot.side.other())
            .filter(|e| m.fighter(*e).at.distance(t) <= 1)
            .count()
    };
    crowd(to) > crowd(f.at)
}

/// The step a creature means to take this tick, if any. A plan to its own tile is hanging back.
pub fn plan_step(m: &Match, slot: Slot, now: u16) -> Option<Plan> {
    let f = m.fighter(slot);
    if f.out || f.interrupted || f.holding {
        return None;
    }
    let goals: Vec<Tile> = match f.order? {
        Order::Move { to } => {
            if f.arrived || f.at == to {
                return follow_up(m, slot);
            }
            vec![to]
        }
        Order::Hold { at } => vec![at],
        Order::Use { at } => {
            let item = f.carrying?;
            if f.spent || f.at.distance(at) <= item.reach() {
                return None;
            }
            m.arena
                .tiles()
                .filter(|t| t.distance(at) <= item.reach() && m.walkable(*t))
                .collect()
        }
        Order::Contest { target } => {
            if f.spent {
                return step_back(m, slot);
            }
            let there = match target {
                Target::Creature { slot: t } => {
                    if !m.standing(t) {
                        return None;
                    }
                    m.fighter(t).at
                }
                Target::Lever { at } => at,
            };
            if f.at.touches(there) {
                return None;
            }
            there
                .steps()
                .into_iter()
                .filter(|t| m.walkable(*t))
                .collect()
        }
    };
    let plan = route(m, slot, &goals, now)?;
    if hangs_back(m, slot, plan.to) {
        return Some(Plan {
            to: m.fighter(slot).at,
            reason: Reason::HungBack,
            detour: false,
            follow_up: false,
        });
    }
    Some(plan)
}

/// Steps from every tile to the nearest goal, walking only where the ground allows and, if
/// `around` is set, round anyone standing still.
pub fn distances(m: &Match, me: Slot, goals: &[Tile], around: bool) -> Vec<Option<u8>> {
    let w = m.arena.width as usize;
    let index = |t: Tile| t.y as usize * w + t.x as usize;
    let mut dist = vec![None; w * m.arena.height as usize];
    let mut queue = std::collections::VecDeque::new();
    for goal in goals {
        if m.arena.contains(*goal) && dist[index(*goal)].is_none() {
            dist[index(*goal)] = Some(0u8);
            queue.push_back(*goal);
        }
    }
    let blocked = |t: Tile| {
        around
            && m.occupant(t)
                .is_some_and(|o| o != me && m.fighter(o).stationary())
    };
    while let Some(tile) = queue.pop_front() {
        let d = dist[index(tile)].expect("queued tiles have a distance");
        for next in tile.steps() {
            if m.arena.contains(next)
                && dist[index(next)].is_none()
                && m.walkable(next)
                && !blocked(next)
            {
                dist[index(next)] = Some(d.saturating_add(1));
                queue.push_back(next);
            }
        }
    }
    dist
}

/// How a step looks to this creature, part by part, so the choice can be explained.
#[derive(Clone, Copy, Debug, Default)]
struct Weighed {
    exposure: i32,
    centre: i32,
    together: i32,
    apart: i32,
    lane: i32,
    cover: i32,
    /// Marked for something about to land on it.
    warned: i32,
    /// Something lies there to pick up.
    find: i32,
    /// The teammate it would be beside that it likes best, and the one it likes least.
    friend: Option<Slot>,
    shunned: Option<Slot>,
}

impl Weighed {
    /// Adds a further tile's worth; the friend and the one shunned are the first met.
    fn add(&mut self, other: &Self) {
        self.exposure += other.exposure;
        self.centre += other.centre;
        self.together += other.together;
        self.apart += other.apart;
        self.lane += other.lane;
        self.cover += other.cover;
        self.warned += other.warned;
        self.find += other.find;
        self.friend = self.friend.or(other.friend);
        self.shunned = self.shunned.or(other.shunned);
    }

    /// A whole way's worth: danger counts on every tile it passes, while company and the lane
    /// it keeps to count as the way's average, so a long way round is never worth more company
    /// for being long.
    fn along(&self, steps: usize) -> i32 {
        let steps = steps.max(1) as i32;
        self.exposure
            + self.centre
            + self.cover
            + self.warned
            + self.find
            + (self.together + self.apart + self.lane) / steps
    }

    /// The same tile, its every part pulling `percent` as hard.
    fn scaled(mut self, percent: i32) -> Self {
        for part in [
            &mut self.exposure,
            &mut self.centre,
            &mut self.together,
            &mut self.apart,
            &mut self.lane,
            &mut self.cover,
            &mut self.warned,
            &mut self.find,
        ] {
            *part = *part * percent / 100;
        }
        self
    }

    fn total(&self) -> i32 {
        self.exposure
            + self.centre
            + self.together
            + self.apart
            + self.lane
            + self.cover
            + self.warned
            + self.find
    }
}

/// What a tile is like for this creature, ignoring `chasing` (whom it is walking up to on
/// purpose).
fn weigh(m: &Match, slot: Slot, tile: Tile, chasing: Option<Slot>, passing_zone: bool) -> Weighed {
    let b = m.behavior(slot);
    let caution = i32::from(b.caution);
    let enemies: Vec<Slot> = m.team(slot.side.other()).collect();
    let reachable = enemies
        .iter()
        .filter(|e| Some(**e) != chasing && m.fighter(**e).at.distance(tile) <= 1)
        .count() as i32;
    let mut w = Weighed {
        exposure: -caution.max(0) * reachable,
        ..Weighed::default()
    };
    if passing_zone
        && m.arena.in_zone(tile)
        && enemies.iter().any(|e| m.arena.in_zone(m.fighter(*e).at))
    {
        w.centre = -caution.max(0) / 2;
    }
    let me = m.profile(slot);
    let mut best = i32::MIN;
    let mut worst = i32::MAX;
    for ally in m.team(slot.side).filter(|a| *a != slot) {
        if !m.fighter(ally).at.beside(tile) {
            continue;
        }
        let bond = me.bond_with(m.profile(ally).creature_id);
        let liking = match bond.map(|b| b.affinity) {
            Some(crate::profile::Strength::High) => 100,
            Some(crate::profile::Strength::Medium) => 50,
            _ => 0,
        };
        let dislike = match bond.map(|b| b.avoidance) {
            Some(crate::profile::Strength::High) => -200,
            Some(crate::profile::Strength::Medium) => -100,
            _ => 0,
        };
        w.together += i32::from(b.togetherness) / 2 + liking;
        w.apart += dislike;
        if liking > best {
            best = liking;
            w.friend = Some(ally);
        }
        if dislike < worst && dislike < 0 {
            worst = dislike;
            w.shunned = Some(ally);
        }
    }
    let middle = i32::from(m.arena.zone_centre().y);
    let reach = (i32::from(m.arena.height) / 2).max(1);
    w.lane = (i32::from(tile.y) - middle).abs() * i32::from(b.wanderlust) / reach;
    let walls = tile.steps().iter().filter(|t| !m.walkable(**t)).count() as i32;
    w.cover = walls * caution.max(0) / 4;
    // A cautious creature keeps off a tile marked for something about to land, half as much as
    // it keeps out of reach of one of the other team; a bold one walks straight on and takes its
    // chances.
    if warned(m, tile) {
        w.warned = -caution.max(0) / 2;
    }
    // Something lying there, and paws to carry it: a curious or playful creature is drawn to it,
    // one that keeps to its way is not.
    if m.fighter(slot).carrying.is_none() && m.board.ground.iter().any(|(g, _)| *g == tile) {
        w.find = (i32::from(b.wanderlust).max(0) + i32::from(b.opportunism).max(0)) / 2;
    }
    w
}

/// Whether something is marked to land on this tile: the map event once it is warned, or an item
/// used and about to take effect.
pub fn warned(m: &Match, tile: Tile) -> bool {
    (m.board.event_at.is_some() && m.arena.in_scenery(tile))
        || m.board.pending.iter().any(|p| match p.item {
            crate::items::Item::Puff => crate::items::puff_tiles(p.from, p.target).contains(&tile),
            _ => false,
        })
}

/// Which part of a choice made the difference, as a reason the recap can say.
fn explain(chosen: &Weighed, plain: &Weighed) -> Reason {
    let parts = [
        (chosen.exposure - plain.exposure, 0),
        (chosen.centre - plain.centre, 1),
        (chosen.together - plain.together, 2),
        (chosen.apart - plain.apart, 3),
        (chosen.lane - plain.lane, 4),
        (chosen.cover - plain.cover, 5),
        (chosen.warned - plain.warned, 6),
        (chosen.find - plain.find, 7),
    ];
    let (gain, part) = parts
        .into_iter()
        .max_by_key(|(gain, part)| (*gain, -part))
        .expect("there are parts");
    if gain <= 0 {
        return Reason::Direct;
    }
    match part {
        0 if chosen.exposure > 0 => Reason::Bold,
        0 => Reason::AvoidedExposure,
        1 if chosen.centre > 0 => Reason::Bold,
        1 => Reason::AvoidedContestedCentre,
        2 => chosen
            .friend
            .map_or(Reason::Direct, |ally| Reason::KeptNearAlly { ally }),
        3 => plain
            .shunned
            .map_or(Reason::Direct, |ally| Reason::KeptApartFrom { ally }),
        4 => Reason::SideRoute,
        5 => Reason::TookCover,
        6 => Reason::SkirtedWarning,
        _ => Reason::WentForItem,
    }
}

/// Picks the way there. It looks ahead over the steps left in the turn, at every way that loses
/// at most its detour budget, and weighs each whole way: what every tile on it is like, less
/// `TIME_COST` for each step it would get there later than the quickest way. Then it takes the
/// first step of the best, and looks again next tick, from wherever everyone has got to.
fn route(m: &Match, slot: Slot, goals: &[Tile], now: u16) -> Option<Plan> {
    let f = m.fighter(slot);
    let mut dist = distances(m, slot, goals, true);
    let w = m.arena.width as usize;
    let index = |t: Tile| t.y as usize * w + t.x as usize;
    if dist[index(f.at)].is_none() {
        dist = distances(m, slot, goals, false);
    }
    let here = dist[index(f.at)]?;
    let chasing = match f.order {
        Some(Order::Contest {
            target: Target::Creature { slot: t },
        }) => Some(t),
        _ => None,
    };
    let passing_zone = !goals.iter().any(|g| m.arena.in_zone(*g));
    let per = u16::from(m.ruleset.ticks_per_turn);
    let tick = now.saturating_sub(u16::from(m.board.turn) * per);
    let depth = (per + 1).saturating_sub(tick).clamp(1, 3) as usize;
    let look = Look {
        m,
        slot,
        dist: &dist,
        chasing,
        passing_zone,
        here,
    };
    let mut ways: Vec<Way> = Vec::new();
    look.extend(
        &mut vec![f.at],
        &mut Weighed::default(),
        0,
        f.detours_left,
        depth,
        &mut ways,
    );
    if ways.is_empty() {
        return None;
    }
    let tie = |t: Tile| {
        rng::roll(
            m.seed,
            Purpose::TieBreak,
            &[u64::from(now), slot.key(), t.x as u64, t.y as u64],
        )
    };
    // The plain way: what a creature with no leanings at all would take.
    let plain = ways
        .iter()
        .filter(|w| w.late == 0)
        .max_by_key(|w| (std::cmp::Reverse(w.first_detour), tie(w.first)))
        .copied();
    ways.sort_by(|a, b| b.score.cmp(&a.score).then(tie(b.first).cmp(&tie(a.first))));
    // Ways that begin with the same step are one choice.
    let mut choices: Vec<Way> = Vec::new();
    for way in ways {
        if !choices.iter().any(|c| c.first == way.first) {
            choices.push(way);
        }
    }
    let mut chosen = choices[0];
    let mut reason = match plain {
        Some(plain) if plain.first != chosen.first => explain(&chosen.weighed, &plain.weighed),
        _ => Reason::Direct,
    };
    if let Some(second) = choices.get(1).copied()
        && chosen.late == 0
        && second.late == 0
        && rng::chance(
            m.seed,
            Purpose::Impulse,
            &[u64::from(now), slot.key()],
            u32::from(m.behavior(slot).variability),
        )
    {
        chosen = second;
        reason = Reason::OnImpulse;
    }
    Some(Plan {
        to: chosen.first,
        reason,
        detour: chosen.first_detour,
        follow_up: false,
    })
}

/// What each step later than the quickest way costs a way.
const TIME_COST: i32 = 220;

/// One way a creature could go over the steps left in the turn.
#[derive(Clone, Copy, Debug)]
struct Way {
    first: Tile,
    /// Its first step brings it no closer.
    first_detour: bool,
    weighed: Weighed,
    /// How many steps later than the quickest way it gets there.
    late: u8,
    score: i32,
}

struct Look<'a> {
    m: &'a Match,
    slot: Slot,
    dist: &'a [Option<u8>],
    chasing: Option<Slot>,
    passing_zone: bool,
    here: u8,
}

impl Look<'_> {
    fn distance(&self, t: Tile) -> Option<u8> {
        self.dist[t.y as usize * self.m.arena.width as usize + t.x as usize]
    }

    fn extend(
        &self,
        path: &mut Vec<Tile>,
        sum: &mut Weighed,
        detours: u8,
        budget: u8,
        depth: usize,
        ways: &mut Vec<Way>,
    ) {
        let last = *path.last().expect("a way starts somewhere");
        let d = self.distance(last).unwrap_or(u8::MAX);
        let steps = path.len() - 1;
        if steps == depth || (steps > 0 && d == 0) {
            // Steps taken plus steps still to go, against the quickest way there.
            let late = (steps as u8).saturating_add(d).saturating_sub(self.here);
            ways.push(Way {
                first: path[1],
                first_detour: self.distance(path[1]).unwrap_or(u8::MAX) >= self.here,
                weighed: *sum,
                late,
                score: sum.along(steps) - TIME_COST * i32::from(late),
            });
            return;
        }
        for next in last.steps() {
            if !self.m.arena.contains(next) || !self.m.walkable(next) || path.contains(&next) {
                continue;
            }
            // Nobody walks through someone standing still; anyone walking may have moved on.
            if self
                .m
                .occupant(next)
                .is_some_and(|o| o != self.slot && self.m.fighter(o).stationary())
            {
                continue;
            }
            let Some(nd) = self.distance(next) else {
                continue;
            };
            let detour = nd >= d;
            if detour && detours >= budget {
                continue;
            }
            let w = weigh(self.m, self.slot, next, self.chasing, self.passing_zone).scaled(PULL);
            let before = *sum;
            sum.add(&w);
            path.push(next);
            self.extend(path, sum, detours + u8::from(detour), budget, depth, ways);
            path.pop();
            *sum = before;
        }
    }
}

/// Arrived with a moment to spare: stay put, or shift one tile beside a friend or out of reach.
/// A patient creature stays where it was sent.
fn follow_up(m: &Match, slot: Slot) -> Option<Plan> {
    let f = m.fighter(slot);
    if f.followed_up {
        return None;
    }
    let b = m.behavior(slot);
    let here = weigh(m, slot, f.at, None, false);
    let stay = here.total() + i32::from(b.patience) + SHIFT_COST;
    let best =
        f.at.steps()
            .into_iter()
            .filter(|t| m.arena.contains(*t) && m.walkable(*t) && m.occupant(*t).is_none())
            .map(|t| (t, weigh(m, slot, t, None, false)))
            .max_by_key(|(t, w)| {
                (
                    w.total(),
                    -(t.y as i32),
                    -(m.arena.across(slot.side, *t) as i32),
                )
            })?;
    if best.1.total() <= stay {
        return None;
    }
    let toward_friend = best.1.together - here.together;
    let away = (best.1.exposure + best.1.cover) - (here.exposure + here.cover);
    let reason = match best.1.friend {
        Some(ally) if toward_friend >= away => Reason::FollowedUpToAlly { ally },
        _ => Reason::FollowedUpToCover,
    };
    Some(Plan {
        to: best.0,
        reason,
        detour: false,
        follow_up: true,
    })
}

/// Turned away by a holder, a creature that steps back takes one step out of reach, if there is
/// one. After a contest that landed it stays where it is.
fn step_back(m: &Match, slot: Slot) -> Option<Plan> {
    let f = m.fighter(slot);
    if f.followed_up || !f.turned_away || !m.behavior(slot).steps_back {
        return None;
    }
    let reach = |t: Tile| {
        m.team(slot.side.other())
            .filter(|e| m.fighter(*e).at.distance(t) <= 1)
            .count()
    };
    let now = reach(f.at);
    let (to, after) =
        f.at.steps()
            .into_iter()
            .filter(|t| m.arena.contains(*t) && m.walkable(*t) && m.occupant(*t).is_none())
            .map(|t| (t, reach(t)))
            .min_by_key(|(t, r)| (*r, t.y, m.arena.across(slot.side, *t)))?;
    (after < now).then_some(Plan {
        to,
        reason: Reason::SteppedBack,
        detour: false,
        follow_up: true,
    })
}
