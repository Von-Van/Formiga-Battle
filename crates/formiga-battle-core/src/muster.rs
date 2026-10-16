//! How a team shares out its Handler's plan.
//!
//! A Handler never tells a creature what to do. They give the team up to three orders (TAKE some
//! squares, DEFEND some squares, INTERACT with something) and the team works out among itself
//! who goes where. Each order becomes a few jobs: walking onto a square and holding it, moving one
//! of theirs off it, two taking a side each of a holder to flank it, standing guard beside a
//! square, heading off one of theirs coming at it, fetching a found thing, pulling the lever,
//! using what one carries. Each creature then takes the job that suits the team best.
//!
//! What suits the team is, first, what the plan asks for: a square named in two orders is worth
//! twice one named once, and brings a second creature to stand guard beside it. Then how far
//! each creature has to walk, and whether it is already doing that job. Last, and least, who it
//! is: a bold creature would rather be the one that meets the other team, a wary one the one that
//! stands and holds, a sociable one the one that keeps beside a teammate, a curious one the one
//! that goes for something lying on the ground, and anyone may go the other way on a whim. Who a
//! creature is never adds up to more than about two steps' walking (`LEANING_CAP`), so it decides
//! between jobs that are nearly as good as each other, and never takes a creature off a job only
//! it can do.
//!
//! A creature never refuses its job. Its leanings show in which job it takes, in the way it walks
//! there (`interpret`), and in what it does when the plan meets the board: alone where two were
//! needed, a bold one goes at the holder anyway and a wary one stands beside it waiting; when the
//! ground it was sent to is marked for the sprinkler, a wary one finds another job and a bold one
//! stays. Once a turn, when its job comes to nothing (what it went for is taken, the one it went
//! after is knocked out, a teammate got there first, a teammate is knocked out and leaves a job
//! undone), a creature thinks again, from the same plan.

use crate::ai;
use crate::arena::{TEAM_SIZE, Terrain};
use crate::event::{Event, EventKind};
use crate::game::Match;
use crate::grid::Tile;
use crate::items::{self, Item};
use crate::order::{
    Directive, MAX_SQUARES, Order, Orders, PLAN_SIZE, Side, Slot, Target, TeamPlan,
};
use crate::rng::{self, Purpose};
use serde::{Deserialize, Serialize};

/// What a job asks a creature to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Walk onto the square and hold it, or keep holding it.
    Claim,
    /// Contest one of theirs: the one on a square of the plan, or one carrying something.
    Oust,
    /// Take a side of a holder of theirs, then contest it together with another from the other
    /// side.
    Flank,
    /// Alone where two were needed to flank: stand beside the holder, keeping it company until
    /// help comes.
    Pin,
    /// Hold beside a square of the plan, corner to corner, so a flank on either is turned away by
    /// both.
    Guard,
    /// Head off one of theirs coming at a square of the plan.
    Intercept,
    /// Go and pick up a found thing.
    Fetch,
    /// Pull the lever, or turn the tap.
    Pull,
    /// The lever or the tap is resting: stand by it until it is ready, and keep the other team
    /// off it.
    Wait,
    /// Use what it carries.
    Use,
    /// Nothing in the plan for it: it does what it would on its own.
    Spare,
}

/// The job a creature took on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Task {
    /// Which of the team's orders it serves, by place in the plan; `None` for a job it found for
    /// itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directive: Option<u8>,
    pub role: Role,
    /// The square or the thing it is about.
    pub at: Tile,
}

/// Why a creature took the job it did, rather than the one a creature that leans no way at all
/// would have taken in its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum Why {
    /// It was the nearest, or the one best placed.
    Nearest,
    /// It was already there, or already on this job.
    KeptAtIt,
    /// Keen to meet the other team.
    Bold,
    /// Would rather stand and hold.
    Careful,
    /// Wanted to be beside a teammate it likes.
    Friendly { ally: Slot },
    /// Kept away from a teammate it does not get on with.
    Avoiding { ally: Slot },
    /// Couldn't leave something lying on the ground alone.
    Curious,
    /// On a whim.
    Whim,
    /// The only one free for a job meant for two.
    Alone,
}

/// What made a creature think again partway through a turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "because", rename_all = "snake_case")]
pub enum Because {
    /// What it went for was picked up by someone else, or is gone.
    ItemGone,
    /// It picked up what it went for.
    GotIt,
    /// The one it went after was knocked out.
    TargetDown,
    /// The one it went after started holding before it got there, and a contest alone would be
    /// turned away.
    DugIn { target: Slot },
    /// The one of theirs on its square stepped off it.
    SquareCleared,
    /// Someone got to its square first: one of theirs, or a teammate now holding it.
    SquareTaken { by: Slot },
    /// Its ground was marked for something about to land.
    Warned,
    /// A teammate was knocked out and left its job undone.
    AllyDown { ally: Slot },
    /// Something turned up nearby.
    Spotted,
}

/// What a square named in one order is worth to the team, against `STEP` for each step a
/// creature walks to it.
const NEED: i32 = 1000;
/// The second side of a flank: a flank needs both, so the pair is worth two squares.
const SECOND: i32 = NEED;
/// One flanker without the other: still worth having there.
const LONE: i32 = 450;
/// Each guard beside a square named more than once.
const EXTRA: i32 = 600;
/// Guarding or heading off for a square named once, by whoever has nothing better to do: more
/// for DEFEND, which asks for it, than for TAKE.
const BACKUP_DEFEND: i32 = 400;
const BACKUP_TAKE: i32 = 250;
/// Heading off one of theirs coming at a square to defend: less than guarding it, since one that
/// starts holding before it is reached turns the contest away.
const INTERCEPT: i32 = 250;
/// Something lying near, picked up by whoever has nothing better to do.
const FIND: i32 = 150;
/// A plain use for what a creature carries, by one with nothing better to do.
const USE: i32 = 350;
/// The lever or the tap is resting: waiting by it is worth less than pulling it.
const WAIT: i32 = 500;
/// One step's walk.
const STEP: i32 = 90;
/// Walks longer than this count no more: the job is a turn or two off either way.
const MAX_STEPS: i32 = 12;
/// A job it can get to this turn.
const ON_TIME: i32 = 200;
/// Already doing this job, or standing on its square.
const STICK: i32 = 80;
/// The most who a creature is can count for, either way: about two steps.
pub const LEANING_CAP: i32 = 180;
/// How hard leanings pull on who takes which job, in percent of what `leaning` gives.
const LEAN_PERCENT: i32 = 100;
/// Something lying no further than this from a creature with nothing better to do is worth
/// fetching.
const FIND_REACH: u8 = 4;
/// One of theirs no further than this from a square to defend is worth heading off.
const INTERCEPT_REACH: u8 = 3;
/// The most jobs one creature weighs, best first, when the team shares out the plan.
const SHORTLIST: usize = 8;

/// Where a job sends a creature.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Goal {
    HoldAt(Tile),
    /// Hold on whichever of these it can reach soonest.
    HoldNear(Vec<Tile>),
    Contest(Slot),
    /// Go at a holder of theirs that cannot be flanked: alone, a contest is turned away.
    Press(Slot),
    /// A flank's side `at`, against the holder `of`.
    Flank {
        at: Tile,
        of: Slot,
    },
    Pull(Tile),
    Fetch(Tile),
    Use(Tile),
}

/// What kind of job, for the leanings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lean {
    /// Meets the other team.
    Meet,
    /// Stands and holds.
    Stand,
    /// Stands beside a teammate's square.
    Guard,
    /// Goes for something lying there.
    Find,
    /// Works the lever, the tap or something carried.
    Work,
}

#[derive(Clone, Debug)]
struct Seat {
    task: Task,
    goal: Goal,
    value: i32,
    /// Only this creature can take it: the one already standing on the square, or carrying.
    only: Option<Slot>,
    /// The other side of a flank, by place in the seats.
    pair: Option<usize>,
    lean: Lean,
}

impl Seat {
    /// The one tile it stands on, if it is set: two jobs that stand on one tile are not both
    /// taken.
    fn place(&self) -> Option<Tile> {
        match self.goal {
            Goal::HoldAt(t) | Goal::Flank { at: t, .. } => Some(t),
            _ => None,
        }
    }

    fn contests(&self) -> bool {
        matches!(
            self.goal,
            Goal::Contest(_) | Goal::Press(_) | Goal::Flank { .. } | Goal::Pull(_)
        )
    }
}

/// A team's plan as it will be carried out: every order that can be carried out at all, the
/// first `PLAN_SIZE` of them, each with its place in the plan as given; and the places of those
/// dropped.
pub fn checked(m: &Match, side: Side, plan: &TeamPlan) -> (Vec<(u8, Directive)>, Vec<u8>) {
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    for (i, d) in plan.iter().enumerate() {
        let i = i as u8;
        if usize::from(i) >= PLAN_SIZE {
            dropped.push(i);
            continue;
        }
        match tidy(m, side, d) {
            Some(d) => kept.push((i, d)),
            None => dropped.push(i),
        }
    }
    (kept, dropped)
}

/// An order as it can be carried out: its squares on the board and on ground (or a lever or a
/// tap), each once, no more than `MAX_SQUARES`; or `None` if nothing is left.
fn tidy(m: &Match, side: Side, d: &Directive) -> Option<Directive> {
    let fits = |t: &Tile| {
        m.arena.contains(*t) && matches!(m.arena.terrain(*t), Terrain::Open | Terrain::Gate)
            || m.arena.levers.contains(t)
            || m.arena.switches.contains(t)
    };
    let clean = |squares: &[Tile]| {
        let mut out: Vec<Tile> = Vec::new();
        for t in squares {
            if fits(t) && !out.contains(t) && out.len() < MAX_SQUARES {
                out.push(*t);
            }
        }
        (!out.is_empty()).then_some(out)
    };
    match d {
        Directive::Take { squares } => clean(squares).map(|squares| Directive::Take { squares }),
        Directive::Defend { squares } => {
            clean(squares).map(|squares| Directive::Defend { squares })
        }
        Directive::Interact { at } => {
            let at = *at;
            let thing = m.board.ground.iter().any(|(g, _)| *g == at)
                || m.board.glint.iter().any(|(g, _)| *g == at)
                || m.arena.levers.contains(&at)
                || m.arena.switches.contains(&at)
                || m.occupant(at).is_some_and(|o| {
                    m.fighter(o).carrying.is_some()
                        && (o.side != side || use_for(m, o, &[], true).is_some())
                });
            thing.then_some(Directive::Interact { at })
        }
    }
}

/// What a team's plan comes to for each of its creatures: one order and one job each, and the
/// events that say so. Everyone is placed from the same picture of the board, before anyone
/// moves. An empty plan places nobody: each falls back on its own.
pub fn muster(
    m: &Match,
    side: Side,
    plan: &TeamPlan,
    turn: u8,
) -> (Orders, [Option<Task>; TEAM_SIZE], Vec<Event>) {
    let mut orders: Orders = [None; TEAM_SIZE];
    let mut tasks: [Option<Task>; TEAM_SIZE] = [None; TEAM_SIZE];
    let mut events = Vec::new();
    let (kept, dropped) = checked(m, side, plan);
    for index in dropped {
        events.push(Event {
            turn,
            tick: 0,
            kind: EventKind::OrderDropped { side, index },
        });
    }
    if kept.is_empty() {
        return (orders, tasks, events);
    }
    let team: Vec<Slot> = m.team(side).collect();
    let seats = seats(m, side, &kept);
    let walks: Vec<Vec<Option<u8>>> = team.iter().map(|s| ai::walks(m, *s)).collect();
    let chosen = solve(m, &team, &seats, &walks, true, turn);
    let plain = solve(m, &team, &seats, &walks, false, turn);
    let mut taken: Vec<Tile> = Vec::new();
    // Squares to hold are spoken for first, so a guard never stands where a claim is going.
    for (k, seat) in chosen.iter().enumerate() {
        if let Some(i) = seat
            && let Goal::HoldAt(t) = seats[*i].goal
        {
            let _ = k;
            taken.push(t);
        }
    }
    for (k, slot) in team.iter().enumerate() {
        let (task, order, why) = match chosen[k] {
            None => {
                let order = spare(m, *slot, &taken);
                let at = match order {
                    Order::Hold { at } | Order::Move { to: at } | Order::Use { at } => at,
                    Order::Contest { .. } => m.fighter(*slot).at,
                };
                (
                    Task {
                        directive: None,
                        role: Role::Spare,
                        at,
                    },
                    order,
                    Why::Nearest,
                )
            }
            Some(i) => {
                let seat = &seats[i];
                let alone = seat.pair.is_some_and(|p| !chosen.contains(&Some(p)))
                    || matches!(seat.goal, Goal::Press(_));
                let (role, order) = order_for(
                    m, *slot, seat, &chosen, &team, &seats, &walks[k], &mut taken, alone,
                );
                let why = if alone {
                    Why::Alone
                } else {
                    explain(
                        m,
                        *slot,
                        seat,
                        plain[k].map(|p| &seats[p]),
                        plain[k] == chosen[k],
                        &walks[k],
                        turn,
                    )
                };
                (
                    Task {
                        directive: seat.task.directive,
                        role,
                        at: seat.task.at,
                    },
                    order,
                    why,
                )
            }
        };
        orders[slot.index as usize] = Some(order);
        tasks[slot.index as usize] = Some(task);
        events.push(Event {
            turn,
            tick: 0,
            kind: EventKind::Assigned {
                slot: *slot,
                task,
                order,
                why,
            },
        });
    }
    (orders, tasks, events)
}

/// Every job the plan makes, in the order the plan gives its squares; then the jobs anyone with
/// nothing better to do might take, nearest the team's own side of the board first.
fn seats(m: &Match, side: Side, plan: &[(u8, Directive)]) -> Vec<Seat> {
    let mut seats: Vec<Seat> = Vec::new();
    // How many orders name each square, and which order named it first.
    let mut named: Vec<(Tile, u8, i32, bool)> = Vec::new();
    for (i, d) in plan {
        for t in d.squares() {
            match named.iter_mut().find(|(s, ..)| s == t) {
                Some(entry) => {
                    entry.2 += 1;
                    entry.3 |= matches!(d, Directive::Defend { .. });
                }
                None => named.push((*t, *i, 1, matches!(d, Directive::Defend { .. }))),
            }
        }
    }
    let kind_of = |i: u8| plan.iter().find(|(j, _)| *j == i).map(|(_, d)| d.clone());
    for (t, i, weight, defended) in named.clone() {
        let task = |role| Task {
            directive: Some(i),
            role,
            at: t,
        };
        let interact = matches!(kind_of(i), Some(Directive::Interact { .. }));
        let lever = m.arena.levers.contains(&t);
        let switch = m.arena.switches.contains(&t);
        if lever || switch {
            let ready = if lever {
                m.lever_ready()
            } else {
                m.switch_ready()
            };
            let beside = open_steps(m, t);
            if defended && !interact {
                // Keep the other team off it: holding beside it shoos off whoever pulls it.
                seats.push(Seat {
                    task: task(Role::Guard),
                    goal: Goal::HoldNear(beside),
                    value: NEED * weight,
                    only: None,
                    pair: None,
                    lean: Lean::Guard,
                });
            } else if ready {
                seats.push(Seat {
                    task: task(Role::Pull),
                    goal: Goal::Pull(t),
                    value: NEED * weight,
                    only: None,
                    pair: None,
                    lean: Lean::Work,
                });
            } else {
                seats.push(Seat {
                    task: task(Role::Wait),
                    goal: Goal::HoldNear(beside),
                    value: WAIT * weight,
                    only: None,
                    pair: None,
                    lean: Lean::Guard,
                });
            }
            continue;
        }
        let occupant = m.occupant(t);
        if interact {
            if m.board.ground.iter().any(|(g, _)| *g == t)
                || m.board.glint.iter().any(|(g, _)| *g == t)
            {
                seats.push(Seat {
                    task: task(Role::Fetch),
                    goal: Goal::Fetch(t),
                    value: NEED * weight,
                    only: None,
                    pair: None,
                    lean: Lean::Find,
                });
            } else if let Some(o) = occupant {
                if o.side == side {
                    if let Some(on) = use_for(m, o, &squares_of(plan), true) {
                        seats.push(Seat {
                            task: task(Role::Use),
                            goal: Goal::Use(on),
                            value: NEED * weight,
                            only: Some(o),
                            pair: None,
                            lean: Lean::Work,
                        });
                    }
                } else {
                    seats.push(Seat {
                        task: task(Role::Oust),
                        goal: Goal::Contest(o),
                        value: NEED * weight,
                        only: None,
                        pair: None,
                        lean: Lean::Meet,
                    });
                }
            }
            continue;
        }
        match occupant {
            // One of ours is on it already. Holding there, it keeps it: swapping a holder for
            // another leaves the square empty while one walks off and the other walks on. Only
            // passing through, it steps off for whoever comes if it is needed elsewhere more.
            Some(o) if o.side == side => seats.push(Seat {
                task: task(Role::Claim),
                goal: Goal::HoldAt(t),
                value: NEED * weight,
                only: holds(m, o).then_some(o),
                pair: None,
                lean: Lean::Stand,
            }),
            Some(e) => {
                let digs_in = holds(m, e);
                let places = digs_in.then(|| ai::flank_places(m, e, side)).flatten();
                match places {
                    Some([a, b]) => {
                        let first = seats.len();
                        for (k, place) in [a, b].into_iter().enumerate() {
                            seats.push(Seat {
                                task: task(Role::Flank),
                                goal: Goal::Flank { at: place, of: e },
                                value: if k == 0 {
                                    NEED * weight
                                } else {
                                    SECOND * weight
                                },
                                only: None,
                                pair: Some(if k == 0 { first + 1 } else { first }),
                                lean: Lean::Meet,
                            });
                        }
                    }
                    None => seats.push(Seat {
                        task: task(Role::Oust),
                        goal: if digs_in {
                            Goal::Press(e)
                        } else {
                            Goal::Contest(e)
                        },
                        value: NEED * weight,
                        only: None,
                        pair: None,
                        lean: Lean::Meet,
                    }),
                }
            }
            None => seats.push(Seat {
                task: task(Role::Claim),
                goal: Goal::HoldAt(t),
                value: NEED * weight,
                only: None,
                pair: None,
                lean: Lean::Stand,
            }),
        }
        // A square named more than once brings guards beside it; one named once, a guard for
        // whoever has nothing better to do.
        let guard_tiles = guard_places(m, side, t);
        if !guard_tiles.is_empty() {
            let mut values: Vec<i32> = (1..weight.min(3)).map(|_| EXTRA).collect();
            values.push(if defended { BACKUP_DEFEND } else { BACKUP_TAKE });
            for value in values {
                seats.push(Seat {
                    task: task(Role::Guard),
                    goal: Goal::HoldNear(guard_tiles.clone()),
                    value,
                    only: None,
                    pair: None,
                    lean: Lean::Guard,
                });
            }
        }
        if defended {
            let mut coming: Vec<Slot> = m
                .team(side.other())
                .filter(|e| {
                    let f = m.fighter(*e);
                    !holds(m, *e) && f.at != t && f.at.distance(t) <= INTERCEPT_REACH
                })
                .collect();
            coming.sort_by_key(|e| (m.fighter(*e).at.distance(t), *e));
            for e in coming {
                seats.push(Seat {
                    task: task(Role::Intercept),
                    goal: Goal::Contest(e),
                    value: INTERCEPT,
                    only: None,
                    pair: None,
                    lean: Lean::Meet,
                });
            }
        }
    }

    // Jobs nobody was given: something lying near, and a plain use for what one carries.
    let named_tiles: Vec<Tile> = named.iter().map(|(t, ..)| *t).collect();
    let mut finds: Vec<Tile> = m
        .board
        .ground
        .iter()
        .map(|(t, _)| *t)
        .filter(|t| !named_tiles.contains(t))
        .filter(|t| {
            m.team(side)
                .any(|s| m.fighter(s).at.distance(*t) <= FIND_REACH)
        })
        .collect();
    finds.sort_by_key(|t| (t.y, m.arena.across(side, *t)));
    for t in finds {
        seats.push(Seat {
            task: Task {
                directive: None,
                role: Role::Fetch,
                at: t,
            },
            goal: Goal::Fetch(t),
            value: FIND,
            only: None,
            pair: None,
            lean: Lean::Find,
        });
    }
    let squares = squares_of(plan);
    for s in m.team(side) {
        let named_use = seats
            .iter()
            .any(|seat| seat.only == Some(s) && seat.task.role == Role::Use);
        if named_use {
            continue;
        }
        if let Some(on) = use_for(m, s, &squares, false) {
            seats.push(Seat {
                task: Task {
                    directive: None,
                    role: Role::Use,
                    at: m.fighter(s).at,
                },
                goal: Goal::Use(on),
                value: USE,
                only: Some(s),
                pair: None,
                lean: Lean::Work,
            });
        }
    }
    seats
}

/// Whether one of theirs holds where it stands, or will as soon as the turn starts: it was
/// holding, or was told to hold just there and was knocked off balance before it could.
pub(crate) fn holds(m: &Match, e: Slot) -> bool {
    let f = m.fighter(e);
    f.holding || f.order == Some(Order::Hold { at: f.at })
}

/// The squares a plan names to take or defend.
fn squares_of(plan: &[(u8, Directive)]) -> Vec<Tile> {
    plan.iter()
        .filter(|(_, d)| !matches!(d, Directive::Interact { .. }))
        .flat_map(|(_, d)| d.squares().to_vec())
        .collect()
}

/// Ground next to a tile, open to stand on.
fn open_steps(m: &Match, t: Tile) -> Vec<Tile> {
    t.steps()
        .into_iter()
        .filter(|n| m.arena.contains(*n) && m.walkable(*n))
        .collect()
}

/// Where to guard a square from: corner to corner with it, where a flank on either is turned
/// away by both; next to it if there is no such place. The zone first, then nearest the team's
/// own side.
fn guard_places(m: &Match, side: Side, t: Tile) -> Vec<Tile> {
    let free = |c: &Tile| {
        m.arena.contains(*c) && m.walkable(*c) && m.occupant(*c).is_none_or(|o| o.side == side)
    };
    let mut corners: Vec<Tile> = t
        .around()
        .into_iter()
        .filter(|c| !c.touches(t))
        .filter(free)
        .collect();
    if corners.is_empty() {
        corners = t.steps().into_iter().filter(free).collect();
    }
    corners.sort_by_key(|c| (!m.arena.in_zone(*c), m.arena.across(side, *c), c.y));
    corners
}

/// A use for what this creature carries that serves the plan: a puff on one of theirs on or next
/// to a square of the plan, a twig on the way the other team would come at one, a leaf over one
/// of ours about to be soaked. Asked for in so many words (`asked`), it looks further: a puff on
/// any of theirs in reach, a leaf over whichever of ours the other team is nearest.
fn use_for(m: &Match, slot: Slot, squares: &[Tile], asked: bool) -> Option<Tile> {
    let f = m.fighter(slot);
    let item = f.carrying?;
    let at = f.at;
    let side = slot.side;
    let squares: Vec<Tile> = if squares.is_empty() {
        m.arena.zone.clone()
    } else {
        squares.to_vec()
    };
    let near = |t: Tile| at.distance(t) <= item.reach() + 2;
    let by = |t: &Tile| (at.distance(*t), m.arena.across(side, *t), t.y);
    match item {
        Item::Puff => m
            .team(side.other())
            .map(|e| m.fighter(e).at)
            .filter(|t| *t != at && near(*t))
            .filter(|t| asked || squares.iter().any(|s| s.distance(*t) <= 1))
            .min_by_key(by),
        Item::Twig => squares
            .iter()
            .flat_map(|s| s.steps())
            .filter(|t| {
                near(*t)
                    && m.arena.contains(*t)
                    && m.walkable(*t)
                    && m.occupant(*t).is_none()
                    && !squares.contains(t)
                    && !m.board.ground.iter().any(|(g, _)| g == t)
                    && m.team(side.other())
                        .any(|e| m.fighter(e).at.distance(*t) <= 3)
            })
            .min_by_key(by),
        Item::Leaf => {
            let soaking = m
                .team(side)
                .map(|x| m.fighter(x).at)
                .filter(|t| {
                    at.distance(*t) <= item.reach()
                        && m.board.event_at.is_some()
                        && m.board.event_tiles.contains(t)
                })
                .min_by_key(by);
            soaking.or_else(|| {
                asked
                    .then(|| {
                        m.team(side)
                            .filter(|x| at.distance(m.fighter(*x).at) <= item.reach())
                            .min_by_key(|x| {
                                let t = m.fighter(*x).at;
                                (
                                    m.team(side.other())
                                        .map(|e| m.fighter(e).at.distance(t))
                                        .min()
                                        .unwrap_or(u8::MAX),
                                    by(&t),
                                )
                            })
                            .map(|x| m.fighter(x).at)
                    })
                    .flatten()
            })
        }
        // Rung where it stands: the other team's contests nearby come to nothing.
        Item::Bell => m
            .team(side.other())
            .any(|e| {
                let there = m.fighter(e).at;
                !m.fighter(e).holding
                    && there.distance(at) <= items::BELL_REACH
                    && (asked || m.team(side).any(|x| m.fighter(x).at.touches(there)))
            })
            .then_some(at),
        // Honey where one of theirs not holding walks in: near a square of the plan, or anywhere
        // in reach when asked.
        Item::Honey => m
            .team(side.other())
            .filter(|e| !m.fighter(*e).holding)
            .map(|e| m.fighter(e).at)
            .filter(|t| near(*t) && (asked || squares.iter().any(|s| s.distance(*t) <= 2)))
            .min_by_key(by),
        // A hop onto a square of the plan two tiles off.
        Item::Seed => squares
            .iter()
            .copied()
            .filter(|t| {
                at.distance(*t) == 2
                    && m.walkable(*t)
                    && m.occupant(*t).is_none()
                    && !crate::interpret::warned(m, *t)
            })
            .min_by_key(by),
    }
}

/// How many steps this creature is from doing a job, and the tile it would do it from.
fn reach(m: &Match, slot: Slot, seat: &Seat, walk: &[Option<u8>]) -> Option<(u8, Tile)> {
    let w = m.arena.width as usize;
    let steps = |t: Tile| -> Option<u8> {
        if !m.arena.contains(t) {
            return None;
        }
        walk[t.y as usize * w + t.x as usize]
    };
    let here = m.fighter(slot).at;
    let side = slot.side;
    let nearest = |tiles: Vec<Tile>| {
        tiles
            .into_iter()
            .filter_map(|t| steps(t).map(|d| (d, t)))
            .min_by_key(|(d, t)| (*d, m.arena.across(side, *t), t.y))
    };
    match &seat.goal {
        Goal::HoldAt(t) | Goal::Fetch(t) | Goal::Flank { at: t, .. } => steps(*t).map(|d| (d, *t)),
        Goal::HoldNear(tiles) => nearest(tiles.clone()),
        Goal::Contest(e) | Goal::Press(e) => {
            let there = m.fighter(*e).at;
            if here.touches(there) {
                return Some((0, here));
            }
            nearest(open_steps(m, there))
        }
        Goal::Pull(t) => {
            if here.touches(*t) {
                return Some((0, here));
            }
            nearest(open_steps(m, *t))
        }
        Goal::Use(t) => {
            let item = m.fighter(slot).carrying?;
            if here.distance(*t) <= item.reach() {
                return Some((0, here));
            }
            nearest(
                m.arena
                    .tiles()
                    .filter(|x| x.distance(*t) <= item.reach() && m.walkable(*x))
                    .collect(),
            )
        }
    }
}

/// The parts of how a creature leans toward a job, so its choice can be explained.
#[derive(Clone, Copy, Debug, Default)]
struct Leaning {
    meet: i32,
    stand: i32,
    together: i32,
    apart: i32,
    find: i32,
    whim: i32,
    friend: Option<Slot>,
    shunned: Option<Slot>,
}

impl Leaning {
    fn total(&self) -> i32 {
        let sum = self.meet + self.stand + self.together + self.apart + self.find + self.whim;
        (sum * LEAN_PERCENT / 100).clamp(-LEANING_CAP, LEANING_CAP)
    }
}

/// How this creature leans toward a job done from `from`. Every part is bounded, and the whole
/// is held to `LEANING_CAP`.
fn leaning(m: &Match, slot: Slot, seat: &Seat, from: Tile, turn: u8) -> Leaning {
    let b = m.behavior(slot);
    let caution = i32::from(b.caution);
    let opportunism = i32::from(b.opportunism);
    let patience = i32::from(b.patience);
    let together = i32::from(b.togetherness);
    let wander = i32::from(b.wanderlust);
    let mut l = Leaning::default();
    match seat.lean {
        Lean::Meet => l.meet = (opportunism - caution) / 4,
        Lean::Stand => l.stand = (caution + patience) / 5,
        Lean::Guard => {
            l.together = together / 4;
            l.stand = caution / 8;
        }
        Lean::Find => l.find = (wander + opportunism / 2) / 4,
        Lean::Work => l.find = opportunism / 6,
    }
    let me = m.profile(slot);
    let mut best = 0;
    let mut worst = 0;
    for ally in m.team(slot.side).filter(|a| *a != slot) {
        if !m.fighter(ally).at.beside(from) {
            continue;
        }
        let bond = me.bond_with(m.profile(ally).creature_id);
        let liking = match bond.map(|b| b.affinity) {
            Some(crate::profile::Strength::High) => 60,
            Some(crate::profile::Strength::Medium) => 30,
            _ => 0,
        };
        let dislike = match bond.map(|b| b.avoidance) {
            Some(crate::profile::Strength::High) => -120,
            Some(crate::profile::Strength::Medium) => -60,
            _ => 0,
        };
        l.together += liking;
        l.apart += dislike;
        if liking > best {
            best = liking;
            l.friend = Some(ally);
        }
        if dislike < worst {
            worst = dislike;
            l.shunned = Some(ally);
        }
    }
    let spread = u64::from(b.variability) + 1;
    l.whim = (rng::roll(
        m.seed,
        Purpose::Muster,
        &[
            u64::from(turn),
            slot.key(),
            seat.task.at.x as u64,
            seat.task.at.y as u64,
            seat.task.role as u64,
        ],
    ) % spread) as i32;
    l
}

/// What a job is worth to the team in this creature's paws, from `from`, `steps` away. With
/// `leanings` off, what it is worth from a creature that leans no way at all.
fn worth(
    m: &Match,
    slot: Slot,
    seat: &Seat,
    steps: u8,
    from: Tile,
    leanings: bool,
    turn: u8,
) -> i32 {
    let f = m.fighter(slot);
    let b = m.behavior(slot);
    let mut v = seat.value - STEP * i32::from(steps).min(MAX_STEPS);
    // Getting there this turn: a square held now is worth more than one held next turn.
    if steps <= m.ruleset.ticks_per_turn {
        v += ON_TIME;
    }
    if f.task
        .is_some_and(|t| t.role == seat.task.role && t.at == seat.task.at)
        || steps == 0
    {
        v += STICK;
        if leanings {
            v += i32::from(b.patience).max(0) / 5;
        }
    }
    // Ground marked for something about to land: worse to stand on, and worse the warier.
    if matches!(seat.lean, Lean::Stand | Lean::Guard)
        && crate::interpret::warned(m, from)
        && !f.sheltered(m.clock(m.board.turn, 0))
    {
        v -= 150;
        if leanings {
            v -= i32::from(b.caution).max(0) / 2;
        }
    }
    if leanings {
        v += leaning(m, slot, seat, from, turn).total();
    }
    v
}

/// Whether this creature can take this job at all; partway through a turn (`midturn`), not a
/// contest it has already spent.
fn can_take(m: &Match, slot: Slot, seat: &Seat, midturn: bool) -> bool {
    let f = m.fighter(slot);
    if seat.only.is_some_and(|o| o != slot) {
        return false;
    }
    if seat.only.is_none()
        && let Goal::HoldAt(t) = seat.goal
        && m.occupant(t).is_some_and(|o| {
            o != slot && (o.side != slot.side || (midturn && m.fighter(o).holding))
        })
    {
        return false;
    }
    if f.carrying.is_some() && matches!(seat.goal, Goal::Fetch(_)) {
        return false;
    }
    if midturn && f.spent && seat.contests() {
        return false;
    }
    true
}

/// Who takes which job, by place in `team`: the sharing out that is worth the most to the team
/// in all, each creature weighing its best few. A flank with only one of its sides taken is
/// worth `LONE` for that side.
fn solve(
    m: &Match,
    team: &[Slot],
    seats: &[Seat],
    walks: &[Vec<Option<u8>>],
    leanings: bool,
    turn: u8,
) -> Vec<Option<usize>> {
    let lists: Vec<Vec<(usize, i32)>> = team
        .iter()
        .enumerate()
        .map(|(k, slot)| {
            let mut list: Vec<(usize, i32)> = seats
                .iter()
                .enumerate()
                .filter(|(_, seat)| can_take(m, *slot, seat, false))
                .filter_map(|(i, seat)| {
                    reach(m, *slot, seat, &walks[k]).map(|(steps, from)| {
                        (i, worth(m, *slot, seat, steps, from, leanings, turn))
                    })
                })
                .filter(|(_, v)| *v > 0)
                .collect();
            list.sort_by_key(|(i, v)| (std::cmp::Reverse(*v), *i));
            list.truncate(SHORTLIST);
            list
        })
        .collect();
    let mut best: (i32, Vec<Option<usize>>) = (i32::MIN, vec![None; team.len()]);
    let mut current: Vec<Option<usize>> = Vec::with_capacity(team.len());
    search(&lists, seats, 0, &mut current, 0, &mut best);
    best.1
}

fn search(
    lists: &[Vec<(usize, i32)>],
    seats: &[Seat],
    k: usize,
    current: &mut Vec<Option<usize>>,
    sum: i32,
    best: &mut (i32, Vec<Option<usize>>),
) {
    if k == lists.len() {
        let mut total = sum;
        for (i, seat) in current.iter().enumerate() {
            if let Some(s) = seat
                && let Some(p) = seats[*s].pair
                && !current.contains(&Some(p))
            {
                let _ = i;
                total -= seats[*s].value - LONE;
            }
        }
        if total > best.0 {
            *best = (total, current.clone());
        }
        return;
    }
    for (i, v) in &lists[k] {
        if current.contains(&Some(*i)) {
            continue;
        }
        if let Some(p) = seats[*i].place()
            && current
                .iter()
                .flatten()
                .any(|c| seats[*c].place() == Some(p))
        {
            continue;
        }
        current.push(Some(*i));
        search(lists, seats, k + 1, current, sum + v, best);
        current.pop();
    }
    current.push(None);
    search(lists, seats, k + 1, current, sum, best);
    current.pop();
}

/// The order a job comes to for the creature that took it, and the role it ends up playing.
#[allow(clippy::too_many_arguments)]
fn order_for(
    m: &Match,
    slot: Slot,
    seat: &Seat,
    chosen: &[Option<usize>],
    team: &[Slot],
    seats: &[Seat],
    walk: &[Option<u8>],
    taken: &mut Vec<Tile>,
    alone: bool,
) -> (Role, Order) {
    let role = seat.task.role;
    match &seat.goal {
        Goal::HoldAt(t) => (role, Order::Hold { at: *t }),
        Goal::HoldNear(tiles) => {
            let w = m.arena.width as usize;
            let at = tiles
                .iter()
                .copied()
                .filter(|t| !taken.contains(t))
                .filter_map(|t| walk[t.y as usize * w + t.x as usize].map(|d| (d, t)))
                .min_by_key(|(d, t)| (*d, m.arena.across(slot.side, *t), t.y))
                .map_or(m.fighter(slot).at, |(_, t)| t);
            taken.push(at);
            (role, Order::Hold { at })
        }
        Goal::Contest(e) => (
            role,
            Order::Contest {
                target: Target::Creature { slot: *e },
            },
        ),
        Goal::Press(e) => press(m, slot, *e, walk, taken),
        Goal::Pull(t) => (
            role,
            Order::Contest {
                target: Target::Lever { at: *t },
            },
        ),
        Goal::Fetch(t) => (role, Order::Move { to: *t }),
        Goal::Use(t) => (role, Order::Use { at: *t }),
        Goal::Flank { at, of } => {
            let contest = Order::Contest {
                target: Target::Creature { slot: *of },
            };
            if alone {
                return press(m, slot, *of, walk, taken);
            }
            // Both in place: they contest together. Otherwise each walks into place and holds
            // there, ready: contests already in reach land before anyone steps, so walking up
            // first is what makes a flank work.
            let partner = seat.pair.and_then(|p| {
                chosen
                    .iter()
                    .position(|c| *c == Some(p))
                    .map(|k| (team[k], &seats[p]))
            });
            let in_place = |s: Slot, place: Tile| m.fighter(s).at == place;
            let both = in_place(slot, *at)
                && partner.is_some_and(|(s, seat)| match seat.goal {
                    Goal::Flank { at, .. } => in_place(s, at),
                    _ => false,
                });
            if both {
                (Role::Flank, contest)
            } else {
                taken.push(*at);
                (Role::Flank, Order::Hold { at: *at })
            }
        }
    }
}

/// How much keener to meet the other team than wary a creature must be to go at a holder alone.
const RECKLESS: i16 = 150;

/// Alone against a holder, where two were needed: a creature much keener than it is wary goes at
/// it anyway and is likely turned away; any other stands beside it, keeping the square contested,
/// until help comes.
fn press(
    m: &Match,
    slot: Slot,
    holder: Slot,
    walk: &[Option<u8>],
    taken: &mut Vec<Tile>,
) -> (Role, Order) {
    let b = m.behavior(slot);
    if b.opportunism - b.caution > RECKLESS {
        return (
            Role::Oust,
            Order::Contest {
                target: Target::Creature { slot: holder },
            },
        );
    }
    let w = m.arena.width as usize;
    let here = m.fighter(slot).at;
    let beside = open_steps(m, m.fighter(holder).at);
    let at = if beside.contains(&here) {
        here
    } else {
        beside
            .into_iter()
            .filter(|t| !taken.contains(t) && m.occupant(*t).is_none())
            .filter_map(|t| walk[t.y as usize * w + t.x as usize].map(|d| (d, t)))
            .min_by_key(|(d, t)| (!m.arena.in_zone(*t), *d, m.arena.across(slot.side, *t), t.y))
            .map_or(here, |(_, t)| t)
    };
    taken.push(at);
    (Role::Pin, Order::Hold { at })
}

/// With nothing in the plan for it: what it would do on its own, keeping off squares a teammate
/// is going to.
fn spare(m: &Match, slot: Slot, taken: &[Tile]) -> Order {
    match ai::fallback(m, slot) {
        Order::Hold { at } if taken.contains(&at) && at != m.fighter(slot).at => Order::Hold {
            at: ai::nearest_free_zone_tile(m, slot, taken).unwrap_or(m.fighter(slot).at),
        },
        order => order,
    }
}

/// Why it took this job: the same as a plain creature would have (`same`), or the leaning that
/// made the most difference against the job a plain creature would have taken in its place
/// (`plain`).
fn explain(
    m: &Match,
    slot: Slot,
    seat: &Seat,
    plain: Option<&Seat>,
    same: bool,
    walk: &[Option<u8>],
    turn: u8,
) -> Why {
    let (steps, from) = reach(m, slot, seat, walk).unwrap_or((0, m.fighter(slot).at));
    if same {
        let kept = steps == 0
            || m.fighter(slot)
                .task
                .is_some_and(|t| t.role == seat.task.role && t.at == seat.task.at);
        return if kept { Why::KeptAtIt } else { Why::Nearest };
    }
    let l = leaning(m, slot, seat, from, turn);
    let o = plain
        .and_then(|p| reach(m, slot, p, walk).map(|(_, f)| leaning(m, slot, p, f, turn)))
        .unwrap_or_default();
    let parts = [
        (l.meet - o.meet, 0),
        (l.stand - o.stand, 1),
        (l.together - o.together, 2),
        (o.apart - l.apart, 3),
        (l.find - o.find, 4),
        (l.whim - o.whim, 5),
    ];
    let (gain, part) = parts
        .into_iter()
        .max_by_key(|(gain, part)| (*gain, -part))
        .expect("there are parts");
    if gain <= 0 {
        return Why::Nearest;
    }
    match part {
        0 => Why::Bold,
        1 => Why::Careful,
        2 => l.friend.map_or(Why::Careful, |ally| Why::Friendly { ally }),
        3 => o.shunned.map_or(Why::Whim, |ally| Why::Avoiding { ally }),
        4 => Why::Curious,
        _ => Why::Whim,
    }
}

impl Match {
    /// Partway through a turn, before anyone steps: each creature whose job has just come to
    /// nothing thinks again, once a turn, from its team's plan. It keeps to jobs nobody else of
    /// its team is on, and a contest it has already spent this turn is not taken again.
    pub(crate) fn rethink(&mut self, turn: u8, tick: u8, events: &mut Vec<Event>) {
        let news: Vec<EventKind> = events
            .iter()
            .filter(|e| e.turn == turn && (e.tick == tick || e.tick + 1 == tick))
            .map(|e| e.kind.clone())
            .collect();
        let warned_now = news.iter().any(|k| {
            matches!(
                k,
                EventKind::SceneryWarned { .. }
                    | EventKind::ItemUsed {
                        item: Item::Puff,
                        ..
                    }
            )
        });
        let appeared = news
            .iter()
            .any(|k| matches!(k, EventKind::ItemAppeared { .. }));
        let mut rethinking: Vec<(Slot, Because)> = Vec::new();
        for slot in self.standing_slots().collect::<Vec<_>>() {
            let f = self.fighter(slot);
            let Some(task) = f.task else { continue };
            if f.rethought || f.interrupted || self.plans[slot.side.index()].is_empty() {
                continue;
            }
            let because = self.why_rethink(slot, task, &news, warned_now, appeared);
            if let Some(because) = because {
                rethinking.push((slot, because));
            }
        }
        for (slot, because) in rethinking {
            let side = slot.side;
            let (kept, _) = checked(self, side, &self.plans[side.index()].clone());
            if kept.is_empty() {
                continue;
            }
            let seats = seats(self, side, &kept);
            let walk = ai::walks(self, slot);
            let others: Vec<Task> = self
                .team(side)
                .filter(|s| *s != slot)
                .filter_map(|s| self.fighter(s).task)
                .collect();
            let best = seats
                .iter()
                .enumerate()
                .filter(|(_, seat)| can_take(self, slot, seat, true))
                .filter(|(_, seat)| {
                    !others.iter().any(|t| {
                        t.role == seat.task.role
                            && t.at == seat.task.at
                            && seat.task.role != Role::Guard
                    })
                })
                .filter_map(|(i, seat)| {
                    reach(self, slot, seat, &walk)
                        .map(|(steps, from)| (i, worth(self, slot, seat, steps, from, true, turn)))
                })
                .filter(|(_, v)| *v > 0)
                .max_by_key(|(i, v)| (*v, std::cmp::Reverse(*i)));
            let taken: Vec<Tile> = self
                .team(side)
                .filter(|s| *s != slot)
                .filter_map(|s| match self.fighter(s).order {
                    Some(Order::Hold { at }) => Some(at),
                    _ => None,
                })
                .collect();
            let (task, order) = match best {
                Some((i, _)) => {
                    let seat = &seats[i];
                    let mut taken = taken;
                    let (role, order) = match &seat.goal {
                        // Partway through a turn there is no flanking together: it walks into
                        // place and holds there.
                        Goal::Flank { at, .. } => (Role::Flank, Order::Hold { at: *at }),
                        _ => {
                            order_for(self, slot, seat, &[], &[], &seats, &walk, &mut taken, false)
                        }
                    };
                    (
                        Task {
                            directive: seat.task.directive,
                            role,
                            at: seat.task.at,
                        },
                        order,
                    )
                }
                None => {
                    let order = spare(self, slot, &taken);
                    let at = match order {
                        Order::Hold { at } | Order::Move { to: at } | Order::Use { at } => at,
                        Order::Contest { .. } => self.fighter(slot).at,
                    };
                    (
                        Task {
                            directive: None,
                            role: Role::Spare,
                            at,
                        },
                        order,
                    )
                }
            };
            let f = self.fighter_mut(slot);
            f.rethought = true;
            if f.task == Some(task) && f.order == Some(order) {
                continue;
            }
            f.task = Some(task);
            f.order = Some(order);
            f.arrived = false;
            f.hung_back = false;
            f.holding = matches!(order, Order::Hold { at } if at == f.at) && f.holding;
            events.push(Event {
                turn,
                tick,
                kind: EventKind::Rethought {
                    slot,
                    task,
                    order,
                    because,
                },
            });
        }
    }

    fn why_rethink(
        &self,
        slot: Slot,
        task: Task,
        news: &[EventKind],
        warned_now: bool,
        appeared: bool,
    ) -> Option<Because> {
        let f = self.fighter(slot);
        let order = f.order?;
        // A teammate knocked out just now leaves its job undone: whoever is only helping out
        // goes to it.
        let helping = task.directive.is_none() || task.role == Role::Guard;
        for kind in news {
            if let EventKind::KnockedOut { slot: ally } = kind
                && ally.side == slot.side
                && helping
                && self
                    .fighter(*ally)
                    .task
                    .is_some_and(|t| t.directive.is_some() && t.role != Role::Guard)
            {
                return Some(Because::AllyDown { ally: *ally });
            }
        }
        match task.role {
            Role::Fetch => {
                if news
                    .iter()
                    .any(|k| matches!(k, EventKind::PickedUp { slot: s, .. } if *s == slot))
                {
                    return Some(Because::GotIt);
                }
                let there = self.board.ground.iter().any(|(g, _)| *g == task.at)
                    || self.board.glint.iter().any(|(g, _)| *g == task.at);
                if !there && f.carrying.is_none() {
                    return Some(Because::ItemGone);
                }
            }
            Role::Oust | Role::Intercept | Role::Flank => {
                if let Order::Contest {
                    target: Target::Creature { slot: target },
                } = order
                {
                    if !self.standing(target) {
                        return Some(Because::TargetDown);
                    }
                    if task.role != Role::Flank
                        && !f.spent
                        && self.fighter(target).holding
                        && !f.at.touches(self.fighter(target).at)
                    {
                        return Some(Because::DugIn { target });
                    }
                    let square = task.directive.is_some_and(|d| {
                        self.plans[slot.side.index()]
                            .get(usize::from(d))
                            .is_some_and(|d| !matches!(d, Directive::Interact { .. }))
                    });
                    if task.role == Role::Oust
                        && square
                        && self.fighter(target).at != task.at
                        && self.occupant(task.at).is_none()
                        && !f.spent
                    {
                        return Some(Because::SquareCleared);
                    }
                }
            }
            _ => {}
        }
        if let Order::Hold { at } = order {
            if !f.holding
                && let Some(by) = self.occupant(at).filter(|o| *o != slot)
                && (self.fighter(by).holding
                    || (by.side != slot.side && self.fighter(by).order == Some(Order::Hold { at })))
            {
                return Some(Because::SquareTaken { by });
            }
            if warned_now
                && crate::interpret::warned(self, at)
                && !f.sheltered(self.clock(self.board.turn, self.ruleset.ticks_per_turn))
            {
                return Some(Because::Warned);
            }
        }
        if appeared && task.directive.is_none() && f.carrying.is_none() {
            return Some(Because::Spotted);
        }
        None
    }
}
