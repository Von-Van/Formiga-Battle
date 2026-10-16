//! Boards set up by hand, for testing one rule at a time.
#![allow(dead_code)]

use formiga_battle_core::*;
use formiga_core::{TemperamentKind, Tension};

pub const L0: Slot = Slot::new(Side::Left, 0);
pub const L1: Slot = Slot::new(Side::Left, 1);
pub const L2: Slot = Slot::new(Side::Left, 2);
pub const R0: Slot = Slot::new(Side::Right, 0);
pub const R1: Slot = Slot::new(Side::Right, 1);
pub const R2: Slot = Slot::new(Side::Right, 2);

/// A creature that leans no way at all and never acts on a whim.
pub fn plain(id: u64) -> Profile {
    Profile {
        creature_id: id,
        name: format!("C{id}"),
        kind: TemperamentKind::Oddball,
        tension: None,
        nature: Nature {
            impulsiveness: 0,
            ..Nature::MIDDLING
        },
        leanings: Leanings::default(),
        bonds: Vec::new(),
    }
}

pub fn with_tension(mut p: Profile, tension: Tension) -> Profile {
    p.tension = Some(tension);
    p
}

/// An open field, 7 by 5, with the zone in its top-right corner where nobody goes unless a test
/// sends them.
pub fn field() -> Arena {
    Arena::parse(
        "id: field\nname: Field\n\
         1 1 1 . . . Z\n\
         . . . . . . .\n\
         . . . . . . .\n\
         . . . . . . .\n\
         2 2 2 . . . .\n",
    )
    .unwrap()
}

pub fn rosters() -> [Vec<Profile>; 2] {
    [
        vec![plain(1), plain(2), plain(3)],
        vec![plain(4), plain(5), plain(6)],
    ]
}

/// A match on `arena` with only the listed creatures on the board, where the test puts them.
pub fn board_on(arena: Arena, placed: &[(Slot, (i8, i8))]) -> Match {
    board_with(arena, rosters(), placed)
}

/// The same, with rosters of the test's own.
pub fn board_with(arena: Arena, rosters: [Vec<Profile>; 2], placed: &[(Slot, (i8, i8))]) -> Match {
    let mut m = Match::new(arena, Ruleset::default(), 7, rosters).unwrap();
    for slot in Slot::all() {
        let f = &mut m.board.fighters[slot.side.index()][slot.index as usize];
        match placed.iter().find(|(s, _)| *s == slot) {
            Some((_, (x, y))) => f.at = Tile::new(*x, *y),
            None => f.out = true,
        }
    }
    m
}

pub fn board(placed: &[(Slot, (i8, i8))]) -> Match {
    board_on(field(), placed)
}

pub fn orders(given: &[(Slot, Order)]) -> [Orders; 2] {
    let mut all: [Orders; 2] = [[None; TEAM_SIZE]; 2];
    for (slot, order) in given {
        all[slot.side.index()][slot.index as usize] = Some(*order);
    }
    all
}

pub fn press(slot: Slot) -> Order {
    Order::Press { target: slot }
}

pub fn trick(slot: Slot) -> Order {
    Order::Trick { target: slot }
}

/// Holding facing north, toward the far edge: side-on to anyone coming from the left or right.
pub fn hold(x: i8, y: i8) -> Order {
    facing(x, y, Facing::North)
}

pub fn facing(x: i8, y: i8, facing: Facing) -> Order {
    Order::Hold {
        at: Tile::new(x, y),
        facing,
    }
}

pub fn interact(x: i8, y: i8) -> Order {
    Order::Interact {
        at: Tile::new(x, y),
    }
}

/// A plan of USE ITEMs only, for `resolve_orders_using`.
pub fn using(side: Side, given: &[(u8, (i8, i8))]) -> [TeamPlan; 2] {
    let mut plans: [TeamPlan; 2] = [Vec::new(), Vec::new()];
    plans[side.index()] = given
        .iter()
        .map(|(by, (x, y))| Directive::UseItem {
            by: *by,
            at: Tile::new(*x, *y),
        })
        .collect();
    plans
}

pub fn walk(x: i8, y: i8) -> Order {
    Order::Move {
        to: Tile::new(x, y),
    }
}

pub fn composure(m: &Match, slot: Slot) -> u8 {
    m.fighter(slot).composure
}

pub fn at(m: &Match, slot: Slot) -> (i8, i8) {
    let t = m.fighter(slot).at;
    (t.x, t.y)
}

pub fn kinds(report: &TurnReport) -> Vec<&EventKind> {
    report.events.iter().map(|e| &e.kind).collect()
}
