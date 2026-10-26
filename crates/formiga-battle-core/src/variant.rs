//! Match variants: the same arena set up a little differently, so one match's opening is not the
//! last one's. A variant has three parts, each a small piece of data compiled in beside the arenas:
//!
//! - a start: where each team's three stand when the match begins;
//! - finds: which item spots are live, which found things turn up and how often;
//! - at most one condition: the map event's timing, how long a switch rests, a gate's starting
//!   state.
//!
//! Every part names the arenas it fits, and is set out for the left team only: the right team's
//! half is always its mirror image, so a variant can never favour a side by how it was written.
//! Applying one gives the arena as this match plays it, with the variant's ids written into it,
//! and a match recorded on that arena replays exactly. Nothing about how a turn resolves changes,
//! so `RULES_VERSION` does not move; the standard variant is the arena as its file has it.
//!
//! Which variant a match gets is drawn from its seed, so both windows of a friend match, and a
//! replay, draw the same one. It is shown before the first order: on the Where page and in the
//! match introduction.

use crate::arena::{Arena, TEAM_SIZE, Terrain};
use crate::grid::Tile;
use crate::items::{Finds, Item};
use crate::rng::{self, Purpose};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

/// The id of every part that leaves the arena as its file has it.
pub const STANDARD: &str = "standard";

/// A match's variant, by the ids of its parts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant {
    pub start: String,
    pub finds: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

impl Default for Variant {
    fn default() -> Self {
        Self::standard()
    }
}

/// A variant that cannot be set up on an arena.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum VariantError {
    #[error("the {arena} has no start called {id}")]
    Start { arena: String, id: String },
    #[error("the {arena} has no finds called {id}")]
    Finds { arena: String, id: String },
    #[error("the {arena} has no condition called {id}")]
    Condition { arena: String, id: String },
    #[error("the {0} is already set up in a variant")]
    AlreadyVaried(String),
    #[error("the {arena}'s {part} cannot be played: {why}")]
    Unplayable {
        arena: String,
        part: String,
        why: &'static str,
    },
}

/// Where the left team starts, on each arena it fits; the right team starts on the mirror image.
#[derive(Clone, Copy, Debug)]
pub struct Start {
    pub id: &'static str,
    pub name: &'static str,
    pub line: &'static str,
    /// By arena id, the left team's three tiles, in the order its creatures take them.
    pub places: &'static [(&'static str, [(i8, i8); TEAM_SIZE])],
}

/// On one arena, the left-hand spot of each live pair (its mirror is live too) and words for where
/// they are.
pub type LiveSpots = (&'static str, &'static [(i8, i8)], &'static str);

/// Which item spots are live, which found things turn up, and when.
#[derive(Clone, Copy, Debug)]
pub struct FindSet {
    pub id: &'static str,
    pub name: &'static str,
    pub line: &'static str,
    pub first: u8,
    pub every: u8,
    /// In `Item::ALL`'s order.
    pub weights: [u8; 6],
    /// The arenas it fits. `None`: every arena, with all its spots live. Otherwise, by arena id,
    /// the left-hand spot of each live pair (its mirror is live too) and words for where they are.
    pub spots: Option<&'static [LiveSpots]>,
}

/// One change to how the board behaves by itself.
#[derive(Clone, Copy, Debug, Default)]
pub struct Change {
    /// The map event's first turn by itself.
    pub first: Option<u8>,
    /// Turns between its coming round by itself; 0 for only when a switch sets it off.
    pub every: Option<u8>,
    /// Whole turns a switch rests after setting it off.
    pub rest: Option<u8>,
    /// Every gate starts open.
    pub gates_open: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Condition {
    pub id: &'static str,
    pub change: Change,
    /// By arena id, what it is called there.
    pub names: &'static [(&'static str, &'static str)],
}

/// How each arena's map event and switches are spoken of, for a condition's line.
const WORDS: &[(&str, &str, &str, &str)] = &[
    // Arena, the event as it happens, the switch as it is used, the switch.
    (
        "backyard",
        "The sprinkler sweeps the yard",
        "someone turns a tap",
        "tap",
    ),
    (
        "woodland",
        "A breeze shakes two trees",
        "someone shakes one",
        "tree",
    ),
    (
        "millpond",
        "The pond rises round the jetty",
        "someone opens a sluice",
        "sluice",
    ),
    (
        "cove",
        "A wave rolls over the sandbar",
        "someone blows a shell",
        "shell",
    ),
];

const STANDARD_WEIGHTS: [u8; 6] = [26, 22, 18, 16, 12, 6];

pub const STARTS: &[Start] = &[
    Start {
        id: STANDARD,
        name: "Usual starts",
        line: "Each team lines up along its own edge.",
        places: &[
            ("backyard", [(0, 1), (0, 3), (0, 5)]),
            ("woodland", [(0, 1), (0, 3), (0, 5)]),
            ("millpond", [(0, 1), (0, 3), (0, 5)]),
            ("cove", [(0, 1), (0, 3), (0, 5)]),
        ],
    },
    Start {
        id: "forward",
        name: "A step in",
        line: "Each team lines up a step in from its edge, closer to the beacon.",
        places: &[
            ("backyard", [(1, 1), (1, 3), (1, 5)]),
            ("woodland", [(1, 1), (1, 3), (1, 5)]),
            ("millpond", [(1, 1), (1, 3), (1, 5)]),
            // Not the Cove: a step in from its edge puts the teams in reach of each other round
            // the rocks by turn 2, and Wallflowers fell to 0.368 over 3,000 matches (0.432 from
            // the usual starts).
        ],
    },
    Start {
        id: "far",
        name: "Bunched far",
        line: "Each team starts bunched in its far corner, a long way round to the near side.",
        places: &[
            ("backyard", [(1, 0), (0, 1), (0, 2)]),
            ("woodland", [(0, 0), (1, 0), (0, 1)]),
            ("millpond", [(1, 0), (0, 1), (0, 2)]),
            ("cove", [(0, 0), (1, 0), (0, 1)]),
        ],
    },
    Start {
        id: "wide",
        name: "Spread wide",
        line: "Each team starts spread from corner to corner along its edge.",
        places: &[
            ("backyard", [(1, 0), (0, 3), (0, 6)]),
            ("woodland", [(0, 0), (0, 3), (1, 6)]),
            ("millpond", [(1, 0), (0, 3), (0, 6)]),
            ("cove", [(0, 0), (0, 3), (0, 6)]),
        ],
    },
];

pub const FINDS: &[FindSet] = &[
    FindSet {
        id: STANDARD,
        name: "Usual finds",
        line: "Found things turn up from turn 2, every third turn.",
        first: 2,
        every: 3,
        weights: STANDARD_WEIGHTS,
        spots: None,
    },
    FindSet {
        id: "few",
        name: "Few finds",
        line: "Found things turn up from turn 3, every fourth turn.",
        first: 3,
        every: 4,
        weights: STANDARD_WEIGHTS,
        spots: None,
    },
    FindSet {
        id: "plenty",
        name: "Plenty of finds",
        line: "Found things turn up from turn 2, every other turn.",
        first: 2,
        every: 2,
        weights: STANDARD_WEIGHTS,
        spots: None,
    },
    FindSet {
        id: "light",
        name: "Light finds",
        line: "Only dandelion puffs, twigs and leaf umbrellas turn up.",
        first: 2,
        every: 3,
        weights: [36, 34, 30, 0, 0, 0],
        spots: None,
    },
    FindSet {
        id: "one_pair",
        name: "One pair of spots",
        line: "Found things turn up in only one pair of spots.",
        first: 2,
        every: 3,
        weights: STANDARD_WEIGHTS,
        spots: Some(&[
            ("backyard", &[(0, 0)], "the far corners"),
            ("woodland", &[(0, 6)], "the near corners"),
            ("millpond", &[(0, 0)], "the far corners"),
            ("cove", &[(1, 2)], "the rocks by the sandbar"),
        ]),
    },
];

pub const CONDITIONS: &[Condition] = &[
    Condition {
        id: "early",
        change: Change {
            first: Some(2),
            every: None,
            rest: None,
            gates_open: false,
        },
        names: &[
            ("backyard", "Early sprinkler"),
            ("woodland", "Early breeze"),
            ("millpond", "Early flood"),
            ("cove", "Early tide"),
        ],
    },
    Condition {
        id: "restless",
        change: Change {
            first: Some(3),
            every: Some(3),
            rest: None,
            gates_open: false,
        },
        names: &[
            ("backyard", "Restless sprinkler"),
            ("woodland", "Gusty"),
            ("millpond", "High water"),
        ],
    },
    Condition {
        id: "calm",
        change: Change {
            first: None,
            every: Some(0),
            rest: None,
            gates_open: false,
        },
        names: &[
            ("backyard", "Sprinkler off"),
            ("woodland", "Still air"),
            ("millpond", "Still pond"),
            ("cove", "Slack tide"),
        ],
    },
    Condition {
        id: "slow_switches",
        change: Change {
            first: None,
            every: None,
            rest: Some(4),
            gates_open: false,
        },
        names: &[
            ("backyard", "Stiff taps"),
            ("woodland", "Settled trees"),
            ("millpond", "Stiff sluices"),
            ("cove", "Quiet shells"),
        ],
    },
    Condition {
        id: "gate_open",
        change: Change {
            first: None,
            every: None,
            rest: None,
            gates_open: true,
        },
        names: &[
            ("backyard", "Gate left open"),
            ("millpond", "Footbridge down"),
        ],
    },
];

/// One part of a variant as the Handler is told it: its name and a line saying what it does here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Told {
    pub name: String,
    pub line: String,
    /// It leaves the arena as its file has it.
    pub standard: bool,
}

impl Start {
    pub fn on(&self, arena: &str) -> Option<[(i8, i8); TEAM_SIZE]> {
        self.places
            .iter()
            .find(|(a, _)| *a == arena)
            .map(|(_, tiles)| *tiles)
    }
}

impl FindSet {
    pub fn fits(&self, arena: &str) -> bool {
        self.spots
            .is_none_or(|s| s.iter().any(|(a, _, _)| *a == arena))
    }

    fn live(&self, arena: &str) -> Option<(&'static [(i8, i8)], &'static str)> {
        self.spots?
            .iter()
            .find(|(a, _, _)| *a == arena)
            .map(|(_, spots, words)| (*spots, *words))
    }

    pub fn rhythm(&self) -> Finds {
        Finds {
            first: self.first,
            every: self.every,
            weights: self.weights,
        }
    }
}

impl Condition {
    pub fn name(&self, arena: &str) -> Option<&'static str> {
        self.names
            .iter()
            .find(|(a, _)| *a == arena)
            .map(|(_, n)| *n)
    }
}

/// Turns as a Handler says them: "3, 7 and 11".
fn turns(list: &[u8]) -> String {
    match list {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => {
            let rest: Vec<String> = rest.iter().map(u8::to_string).collect();
            format!("{} and {last}", rest.join(", "))
        }
    }
}

impl Variant {
    /// The arena as its file has it.
    pub fn standard() -> Self {
        Self {
            start: STANDARD.to_owned(),
            finds: STANDARD.to_owned(),
            condition: None,
        }
    }

    pub fn is_standard(&self) -> bool {
        *self == Self::standard()
    }

    /// The starts that fit this arena, the standard first.
    pub fn starts(arena: &str) -> Vec<&'static Start> {
        STARTS.iter().filter(|s| s.on(arena).is_some()).collect()
    }

    /// The finds that fit this arena, the standard first.
    pub fn finds(arena: &str) -> Vec<&'static FindSet> {
        FINDS.iter().filter(|f| f.fits(arena)).collect()
    }

    /// The conditions that fit this arena.
    pub fn conditions(arena: &str) -> Vec<&'static Condition> {
        CONDITIONS
            .iter()
            .filter(|c| c.name(arena).is_some())
            .collect()
    }

    /// Every variant this arena can be set up in that changes one part only, the standard
    /// first: what the fairness sweep plays.
    pub fn singles(arena: &str) -> Vec<Self> {
        let mut all = vec![Self::standard()];
        for start in Self::starts(arena).iter().skip(1) {
            all.push(Self {
                start: start.id.to_owned(),
                ..Self::standard()
            });
        }
        for finds in Self::finds(arena).iter().skip(1) {
            all.push(Self {
                finds: finds.id.to_owned(),
                ..Self::standard()
            });
        }
        for condition in Self::conditions(arena) {
            all.push(Self {
                condition: Some(condition.id.to_owned()),
                ..Self::standard()
            });
        }
        all
    }

    /// The variant a match on this arena with this seed is played in: each part drawn on its own,
    /// every start and every finds equally likely, and no condition as likely as each one.
    pub fn drawn(arena: &Arena, seed: u64) -> Self {
        let id = arena.id.as_str();
        let roll = |part: u64, of: usize| {
            (rng::roll(seed, Purpose::Variant, &[part]) % of as u64) as usize
        };
        let starts = Self::starts(id);
        let finds = Self::finds(id);
        let conditions = Self::conditions(id);
        if starts.is_empty() || finds.is_empty() {
            return Self::standard();
        }
        let condition = match roll(3, conditions.len() + 1) {
            0 => None,
            k => Some(conditions[k - 1].id.to_owned()),
        };
        Self {
            start: starts[roll(1, starts.len())].id.to_owned(),
            finds: finds[roll(2, finds.len())].id.to_owned(),
            condition,
        }
    }

    /// The arena as a match in this variant plays it: the arena's own, from its file, with this
    /// variant's starts, finds and condition in place and the variant named in it. Refused for a
    /// part the arena has no such thing for, or one that would leave the board unplayable.
    pub fn apply(&self, arena: &Arena) -> Result<Arena, VariantError> {
        if arena.variant.is_some() {
            return Err(VariantError::AlreadyVaried(arena.name.clone()));
        }
        let id = arena.id.as_str();
        let unplayable = |part: &str, why| VariantError::Unplayable {
            arena: arena.name.clone(),
            part: part.to_owned(),
            why,
        };
        let mut varied = arena.clone();
        let start = STARTS
            .iter()
            .find(|s| s.id == self.start)
            .and_then(|s| s.on(id))
            .ok_or_else(|| VariantError::Start {
                arena: arena.name.clone(),
                id: self.start.clone(),
            })?;
        let left = start.map(|(x, y)| Tile::new(x, y));
        varied.starts = [left, left.map(|t| arena.mirror(t))];
        let finds = FINDS
            .iter()
            .find(|f| f.id == self.finds && f.fits(id))
            .ok_or_else(|| VariantError::Finds {
                arena: arena.name.clone(),
                id: self.finds.clone(),
            })?;
        varied.finds = finds.rhythm();
        if let Some((spots, _)) = finds.live(id) {
            let live: BTreeSet<Tile> = spots
                .iter()
                .flat_map(|(x, y)| {
                    let t = Tile::new(*x, *y);
                    [t, arena.mirror(t)]
                })
                .collect();
            if !live.iter().all(|t| arena.item_spots.contains(t)) {
                return Err(unplayable("finds", "a live spot that is not an item spot"));
            }
            varied.item_spots.retain(|t| live.contains(t));
        }
        if let Some(condition) = &self.condition {
            let found = CONDITIONS
                .iter()
                .find(|c| c.id == condition && c.name(id).is_some())
                .ok_or_else(|| VariantError::Condition {
                    arena: arena.name.clone(),
                    id: condition.clone(),
                })?;
            let change = found.change;
            if change.gates_open {
                if varied.gates.is_empty() {
                    return Err(unplayable("condition", "no gate to leave open"));
                }
                varied.gates_open = vec![true; varied.gates.len()];
            }
            if change.first.is_some() || change.every.is_some() || change.rest.is_some() {
                let scenery = varied
                    .scenery
                    .as_mut()
                    .ok_or_else(|| unplayable("condition", "no map event to change"))?;
                if let Some(first) = change.first {
                    scenery.first = first.max(2);
                }
                if let Some(every) = change.every {
                    scenery.every = every;
                }
                if let Some(rest) = change.rest {
                    scenery.rest = rest;
                }
            }
        }
        if let Err(why) = check(&varied) {
            return Err(unplayable("set-up", why));
        }
        varied.variant = Some(self.clone());
        Ok(varied)
    }

    /// What the Handler is told about each part, start first; a condition only when there is one.
    pub fn told(&self, arena: &Arena) -> Vec<Told> {
        let id = arena.id.as_str();
        let mut told = Vec::new();
        if let Some(start) = STARTS.iter().find(|s| s.id == self.start) {
            told.push(Told {
                name: start.name.to_owned(),
                line: start.line.to_owned(),
                standard: start.id == STANDARD,
            });
        }
        if let Some(finds) = FINDS.iter().find(|f| f.id == self.finds) {
            let line = match finds.live(id) {
                Some((_, words)) => format!("Found things turn up only in {words}."),
                None => finds.line.to_owned(),
            };
            told.push(Told {
                name: finds.name.to_owned(),
                line,
                standard: finds.id == STANDARD,
            });
        }
        if let Some(condition) = self
            .condition
            .as_ref()
            .and_then(|c| CONDITIONS.iter().find(|k| k.id == *c))
            && let Some(name) = condition.name(id)
        {
            told.push(Told {
                name: name.to_owned(),
                line: condition_line(condition, arena),
                standard: false,
            });
        }
        told
    }

    /// The parts that are not the arena's own, by name, for a single line: "A step in · Few
    /// finds · Early sprinkler". Empty for the standard variant.
    pub fn headline(&self, arena: &Arena) -> String {
        self.told(arena)
            .into_iter()
            .filter(|t| !t.standard)
            .map(|t| t.name)
            .collect::<Vec<_>>()
            .join(" \u{b7} ")
    }
}

/// What a condition does on this arena, in a line worked out from the arena it gives, so the
/// words always say what the board will do.
fn condition_line(condition: &Condition, arena: &Arena) -> String {
    let words = WORDS.iter().find(|(a, ..)| *a == arena.id);
    let change = condition.change;
    if change.gates_open {
        return match arena.id.as_str() {
            "millpond" => "The footbridge starts down, so the far lane is open.".to_owned(),
            _ => "The gate across the far lane starts open.".to_owned(),
        };
    }
    let Some((_, happening, set_off, switch)) = words else {
        return String::new();
    };
    if let Some(rest) = change.rest {
        return format!("A {switch} rests {rest} whole turns after it is used.");
    }
    // The arena may be the one this condition gave already; setting the same timing again
    // changes nothing.
    let Some(mut scenery) = arena.scenery.clone() else {
        return String::new();
    };
    if let Some(first) = change.first {
        scenery.first = first.max(2);
    }
    if let Some(every) = change.every {
        scenery.every = every;
    }
    let due: Vec<u8> = (1..=crate::ruleset::Ruleset::default().turn_cap)
        .filter(|t| scenery.due(*t))
        .collect();
    if due.is_empty() {
        format!("{happening} only when {set_off}.")
    } else {
        format!("{happening} by itself on turns {}.", turns(&due))
    }
}

/// Whether a board set up this way can be played fairly: three starts a side on open ground off
/// the zone, the item spots and the map event's own tiles, all different, each team in its own
/// half and the right's the mirror of the left's, and every start able to walk to the zone.
pub fn check(arena: &Arena) -> Result<(), &'static str> {
    let middle = (i16::from(arena.width) - 1) / 2;
    let mut seen = BTreeSet::new();
    for (side, tiles) in arena.starts.iter().enumerate() {
        for (k, t) in tiles.iter().enumerate() {
            if !arena.contains(*t) || arena.terrain(*t) != Terrain::Open {
                return Err("a start off open ground");
            }
            if arena.in_zone(*t) || arena.item_spots.contains(t) {
                return Err("a start in the zone or on an item spot");
            }
            if arena.scenery.as_ref().is_some_and(|s| s.tiles.contains(t)) {
                return Err("a start under the map event");
            }
            if !seen.insert(*t) {
                return Err("two starting on one tile");
            }
            let own_half = if side == 0 {
                i16::from(t.x) < middle
            } else {
                i16::from(t.x) > middle
            };
            if !own_half {
                return Err("a start in the other team's half");
            }
            if side == 1 && *t != arena.mirror(arena.starts[0][k]) {
                return Err("the right team's starts are not the left's mirror");
            }
            if steps_to_zone(arena, *t).is_none() {
                return Err("a start with no way to the beacon");
            }
        }
    }
    let finds = arena.finds;
    if !finds.playable() {
        return Err("found things that never turn up");
    }
    if arena
        .item_spots
        .iter()
        .any(|t| !arena.item_spots.contains(&arena.mirror(*t)))
    {
        return Err("an item spot without its mirror");
    }
    if let Some(s) = &arena.scenery
        && s.first < 2
    {
        return Err("a map event with no turn to be warned in");
    }
    Ok(())
}

/// The fewest steps from a tile to the nearest zone tile over the ground as a match begins, gates
/// as they start: what "the beacon reachable in the same steps from both sides" is checked by.
pub fn steps_to_zone(arena: &Arena, from: Tile) -> Option<u16> {
    let open = |t: Tile| match arena.terrain(t) {
        Terrain::Open => true,
        Terrain::Gate => arena
            .gates
            .iter()
            .position(|g| *g == t)
            .is_some_and(|i| arena.gates_open[i]),
        Terrain::Obstacle | Terrain::Lever => false,
    };
    let mut seen = BTreeSet::from([from]);
    let mut queue = VecDeque::from([(from, 0u16)]);
    while let Some((at, n)) = queue.pop_front() {
        if arena.in_zone(at) {
            return Some(n);
        }
        for next in at.steps() {
            if arena.contains(next) && open(next) && seen.insert(next) {
                queue.push_back((next, n + 1));
            }
        }
    }
    None
}

/// Every found thing a set can bring, by name, for a line in the docs and the CLI.
pub fn possible(finds: &Finds) -> Vec<&'static str> {
    Item::ALL
        .into_iter()
        .filter(|i| finds.weight(*i) > 0)
        .map(Item::name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_variant_leaves_every_arena_as_its_file_has_it_but_named() {
        for arena in Arena::built_in() {
            let varied = Variant::standard().apply(&arena).unwrap();
            assert_eq!(varied.starts, arena.starts, "{}", arena.id);
            assert_eq!(varied.item_spots, arena.item_spots);
            assert_eq!(varied.finds, arena.finds);
            assert_eq!(varied.scenery, arena.scenery);
            assert_eq!(varied.gates_open, arena.gates_open);
            assert_eq!(varied.variant, Some(Variant::standard()));
        }
    }

    #[test]
    fn an_arena_already_set_up_in_a_variant_is_not_set_up_again() {
        let once = Variant::standard().apply(&Arena::backyard()).unwrap();
        assert!(matches!(
            Variant::standard().apply(&once),
            Err(VariantError::AlreadyVaried(_))
        ));
    }

    #[test]
    fn a_part_an_arena_has_no_such_thing_for_is_refused() {
        let woodland = Arena::by_id("woodland").unwrap();
        let gate = Variant {
            condition: Some("gate_open".to_owned()),
            ..Variant::standard()
        };
        assert!(matches!(
            gate.apply(&woodland),
            Err(VariantError::Condition { .. })
        ));
        let nowhere = Variant {
            start: "on_the_roof".to_owned(),
            ..Variant::standard()
        };
        assert!(matches!(
            nowhere.apply(&woodland),
            Err(VariantError::Start { .. })
        ));
    }

    #[test]
    fn turns_are_said_the_way_a_handler_says_them() {
        assert_eq!(turns(&[3, 7, 11]), "3, 7 and 11");
        assert_eq!(turns(&[5]), "5");
        assert_eq!(turns(&[]), "");
    }

    #[test]
    fn a_condition_says_what_the_board_will_do() {
        let backyard = Arena::backyard();
        let early = Variant {
            condition: Some("early".to_owned()),
            ..Variant::standard()
        };
        let told = early.told(&backyard);
        assert_eq!(told[2].name, "Early sprinkler");
        assert_eq!(
            told[2].line,
            "The sprinkler sweeps the yard by itself on turns 2, 6 and 10."
        );
        let calm = Variant {
            condition: Some("calm".to_owned()),
            ..Variant::standard()
        };
        assert_eq!(
            calm.told(&backyard)[2].line,
            "The sprinkler sweeps the yard only when someone turns a tap."
        );
        assert_eq!(early.headline(&backyard), "Early sprinkler");
        assert_eq!(Variant::standard().headline(&backyard), "");
    }
}
