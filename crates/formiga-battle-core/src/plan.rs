//! Coaches that think a turn ahead: for each of the team's three orders they try each order it
//! could sensibly be, play the turn out in their head against what they expect the other team to
//! do, and keep whatever leaves the board best for them.
//!
//! The two differ only in how well they know their own team. The familiar coach imagines its
//! creatures as they are. The generic coach imagines every one of them as a creature that leans no
//! way at all, so it plans the same way and is wrong about who will take which job, which way
//! each will walk, who will step back, and who will take a chance. Whatever the familiar coach wins beyond the generic one
//! is what knowing your creatures is worth, and the handoff asks that it be worth something while
//! no creature or trio is worth more than another.

use crate::ai::Coach;
use crate::game::Match;
use crate::order::{Directive, Facing, PLAN_SIZE, Side, TeamPlan};
use crate::profile::{Leanings, Nature, Profile};

/// Each of the plan's orders is revisited this many times, each time with the others' best so
/// far.
const PASSES: usize = 2;

/// The same match, with one team's creatures imagined as leaning no way at all.
fn imagined_plainly(m: &Match, side: Side) -> Match {
    let plain = m.rosters[side.index()].clone().map(|p| Profile {
        tension: None,
        nature: Nature::MIDDLING,
        leanings: Leanings::default(),
        bonds: Vec::new(),
        ..p
    });
    m.imagining(side, plain)
}

/// The orders worth trying: a square of the zone to press onto, the square of any of theirs to
/// press, the square of any of theirs holding to trick, a square of ours to hold facing the other
/// team or its side of the board, and anything to interact with.
fn options(m: &Match, side: Side) -> Vec<Directive> {
    let mut options = Vec::new();
    for t in &m.arena.zone {
        options.push(Directive::Press { squares: vec![*t] });
    }
    for e in m.team(side.other()) {
        let at = m.fighter(e).at;
        if !m.arena.in_zone(at) {
            options.push(Directive::Press { squares: vec![at] });
        }
        if crate::muster::holds(m, e) {
            options.push(Directive::Trick { squares: vec![at] });
        }
    }
    for s in m.team(side) {
        let at = m.fighter(s).at;
        if m.arena.in_zone(at) {
            for facing in [crate::ai::face(m, side, at), Facing::across(side)] {
                let d = Directive::Hold {
                    squares: vec![at],
                    facing,
                };
                if !options.contains(&d) {
                    options.push(d);
                }
            }
        }
    }
    let things = m
        .board
        .ground
        .iter()
        .chain(m.board.glint.iter())
        .map(|(t, _)| *t)
        .chain(m.arena.levers.iter().copied().filter(|_| m.lever_ready()))
        .chain(
            m.arena
                .switches
                .iter()
                .copied()
                .filter(|_| m.switch_ready()),
        );
    for at in things {
        let d = Directive::Interact { at };
        if crate::muster::checked(m, side, &vec![d.clone()]).0.len() == 1 {
            options.push(d);
        }
    }
    options.dedup();
    options
}

/// How good the board is for `side`: the beacon most of all, then who is still standing and how
/// steady they are, then holding in the zone, then having something in hand.
fn score(m: &Match, side: Side) -> i32 {
    if let Some(outcome) = m.outcome() {
        return match outcome.winner {
            Some(w) if w == side => 1_000_000,
            Some(_) => -1_000_000,
            None => 0,
        };
    }
    let (s, o) = (side, side.other());
    let composure = |sd: Side| {
        m.team(sd)
            .map(|x| i32::from(m.fighter(x).composure))
            .sum::<i32>()
    };
    let standing = |sd: Side| m.team(sd).count() as i32;
    let holding = |sd: Side| {
        m.team(sd)
            .filter(|x| m.fighter(*x).holding && m.arena.in_zone(m.fighter(*x).at))
            .count() as i32
    };
    let capture = |sd: Side| i32::from(m.board.capture[sd.index()]);
    let carrying = |sd: Side| {
        m.team(sd)
            .filter(|x| m.fighter(*x).carrying.is_some())
            .count() as i32
    };
    300 * (capture(s) - capture(o))
        + 80 * (composure(s) - composure(o))
        + 200 * (standing(s) - standing(o))
        + 40 * (holding(s) - holding(o))
        + 25 * (carrying(s) - carrying(o))
}

/// One turn's plan for `side`, thought through on `model`.
pub fn think_ahead(m: &Match, side: Side, familiar: bool) -> TeamPlan {
    let model = if familiar {
        m.clone()
    } else {
        imagined_plainly(m, side)
    };
    // What the other team will do: what the Exhibition coach would.
    let theirs = Coach::Exhibition.plan(&model, side.other());
    // Its USE ITEMs are the Exhibition coach's; only the three orders are thought through.
    let (mut ours, uses): (TeamPlan, TeamPlan) = Coach::Exhibition
        .plan(&model, side)
        .into_iter()
        .partition(Directive::tactical);
    let play = |ours: &TeamPlan| {
        let mut sim = model.clone();
        let mut ours = ours.clone();
        ours.extend(uses.iter().cloned());
        let plans = match side {
            Side::Left => [ours, theirs.clone()],
            Side::Right => [theirs.clone(), ours],
        };
        sim.resolve_turn(plans);
        score(&sim, side)
    };
    let mut best = play(&ours);
    let options = options(&model, side);
    for _ in 0..PASSES {
        for place in 0..PLAN_SIZE {
            for option in options.iter() {
                let mut trial = ours.clone();
                if place < trial.len() {
                    trial[place] = option.clone();
                } else {
                    trial.push(option.clone());
                }
                if trial == ours {
                    continue;
                }
                let value = play(&trial);
                if value > best {
                    best = value;
                    ours = trial;
                }
            }
        }
    }
    ours.extend(uses);
    ours
}
