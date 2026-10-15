//! Coaches that think a turn ahead: for each creature they try each order it could sensibly be
//! given, play the turn out in their head against what they expect the other team to do, and keep
//! whatever leaves the board best for them.
//!
//! The two differ only in how well they know their own team. The familiar coach imagines its
//! creatures as they are. The generic coach imagines every one of them as a creature that leans no
//! way at all, so it plans the same way and is wrong about which way each will walk, who will
//! step back, and who will take a chance. Whatever the familiar coach wins beyond the generic one
//! is what knowing your creatures is worth, and the handoff asks that it be worth something while
//! no creature or trio is worth more than another.

use crate::ai::{Coach, nearest_free_zone_tile};
use crate::game::Match;
use crate::order::{Order, Orders, Side, Slot, Target};
use crate::profile::{Leanings, Nature, Profile};

/// Each creature's orders are revisited this many times, each time with its teammates' best so
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

/// The orders worth trying for one creature.
fn options(m: &Match, slot: Slot) -> Vec<Order> {
    let f = m.fighter(slot);
    let mut options = vec![Order::Hold { at: f.at }];
    let mut taken = Vec::new();
    for _ in 0..2 {
        if let Some(tile) = nearest_free_zone_tile(m, slot, &taken) {
            options.push(Order::Hold { at: tile });
            taken.push(tile);
        }
    }
    for enemy in m.team(slot.side.other()) {
        options.push(Order::Contest {
            target: Target::Creature { slot: enemy },
        });
    }
    if m.lever_ready() && m.board.gates_open.iter().any(|open| !open) {
        for lever in &m.arena.levers {
            options.push(Order::Contest {
                target: Target::Lever { at: *lever },
            });
        }
    }
    options.dedup();
    options
}

/// How good the board is for `side`: the beacon most of all, then who is still standing and how
/// steady they are, then holding in the zone.
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
    300 * (capture(s) - capture(o))
        + 80 * (composure(s) - composure(o))
        + 200 * (standing(s) - standing(o))
        + 40 * (holding(s) - holding(o))
}

/// One turn's orders for `side`, thought through on `model`.
pub fn think_ahead(m: &Match, side: Side, familiar: bool) -> Orders {
    let model = if familiar {
        m.clone()
    } else {
        imagined_plainly(m, side)
    };
    let theirs = Coach::Exhibition.orders(&model, side.other());
    let mut ours = Coach::Exhibition.orders(&model, side);
    let play = |ours: Orders| {
        let mut sim = model.clone();
        let orders = match side {
            Side::Left => [ours, theirs],
            Side::Right => [theirs, ours],
        };
        sim.resolve_turn(orders);
        score(&sim, side)
    };
    let mut best = play(ours);
    let team: Vec<Slot> = m.team(side).collect();
    for _ in 0..PASSES {
        for slot in &team {
            for option in options(&model, *slot) {
                let mut trial = ours;
                trial[slot.index as usize] = Some(option);
                if trial == ours {
                    continue;
                }
                let value = play(trial);
                if value > best {
                    best = value;
                    ours = trial;
                }
            }
        }
    }
    ours
}
