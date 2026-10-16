//! Field Lessons: short situations on the Backyard, played under the real rules, that teach how a
//! match works beyond simply playing one.
//!
//! A lesson sets up a board part way through a match (`stage`), has the visitors give orders of
//! its own (`visitors`) and, after every turn, says whether the Handler has done what it asks
//! (`judge`). Nothing here changes a rule: a lesson is a match like any other, begun from a board
//! set up by hand, with a goal and a few turns to reach it. A lesson keeps nothing and earns
//! nothing.
//!
//! The Handler plays the left side, as against the visitors. In every lesson but the last, the
//! Handler's companions are taken to lean no way at all, so the board does what the lesson says
//! it will; the last lesson gives them back who they are.

use crate::arena::{Arena, TEAM_SIZE};
use crate::event::{Event, EventKind, Victory};
use crate::game::{Match, MatchError};
use crate::grid::Tile;
use crate::items::Item;
use crate::order::{Directive, Facing, Side, Slot, TeamPlan};
use crate::profile::{Leanings, Nature, Profile};
use crate::ruleset::Ruleset;

/// The side the Handler plays in a lesson.
pub const YOU: Side = Side::Left;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lesson {
    /// PRESS a square of the yard and light the beacon.
    Beacon,
    /// A HOLD turns a PRESS away, and costs the presser composure when it faces it.
    Facing,
    /// Press a lone holder from two sides and break its hold.
    Flank,
    /// Go round a holder and TRICK it from the side or the back.
    Trick,
    /// Two jobs in one plan: a flank, and the tap kept.
    Orders,
    /// The sprinkler is warned a turn ahead; a leaf umbrella, used as a bonus, keeps a holder dry.
    Weather,
    /// The Handler's own three, as they are.
    Personality,
}

/// How a lesson stands after a turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Still going: more turns to play.
    Going,
    /// Done what it asked, and what that showed.
    Learned(String),
    /// Out of turns, or past saving, and a word on what to try.
    TryAgain(String),
}

impl Verdict {
    pub fn decided(&self) -> bool {
        !matches!(self, Self::Going)
    }
}

/// Backyard's yard: its middle and four arms.
const TOP: Tile = Tile::new(4, 2);
const MIDDLE: Tile = Tile::new(4, 3);
const BOTTOM: Tile = Tile::new(4, 4);
/// The garden tap on the visitors' side of the near edge, and the tile above it.
const TAP: Tile = Tile::new(7, 6);
const BY_TAP: Tile = Tile::new(7, 5);
/// The squares beside the yard the visitors press in the lesson on facing.
const FACING_SQUARES: [Tile; 2] = [Tile::new(2, 3), Tile::new(2, 5)];

impl Lesson {
    pub const ALL: [Self; 7] = [
        Self::Beacon,
        Self::Facing,
        Self::Flank,
        Self::Trick,
        Self::Orders,
        Self::Weather,
        Self::Personality,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Beacon => "beacon",
            Self::Facing => "facing",
            Self::Flank => "flank",
            Self::Trick => "trick",
            Self::Orders => "orders",
            Self::Weather => "weather",
            Self::Personality => "personality",
        }
    }

    pub fn number(self) -> usize {
        Self::ALL.iter().position(|l| *l == self).unwrap_or(0) + 1
    }

    pub fn next(self) -> Option<Self> {
        Self::ALL.get(self.number()).copied()
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Beacon => "Light the beacon",
            Self::Facing => "Hold, and face them",
            Self::Flank => "Flanks",
            Self::Trick => "Tricks",
            Self::Orders => "Three orders, one team",
            Self::Weather => "The sprinkler and found things",
            Self::Personality => "Who they are",
        }
    }

    /// What the lesson asks, in a line.
    pub fn goal(self) -> &'static str {
        match self {
            Self::Beacon => "PRESS a square of the yard and light all three lamps.",
            Self::Facing => "Make both pressers dizzy, without losing any composure yourself.",
            Self::Flank => "Break the lone holder's hold by pressing it from two sides.",
            Self::Trick => "Break the holder's hold with a TRICK from its side or back.",
            Self::Orders => "Flank the holder, and keep their runner off the tap.",
            Self::Weather => "Light your last two lamps this turn, through the sprinkler.",
            Self::Personality => "Light the beacon with your three as they really are.",
        }
    }

    /// What the lesson says before the first turn, a few short paragraphs.
    pub fn notes(self) -> &'static [&'static str] {
        match self {
            Self::Beacon => &[
                "The yard in the middle is the beacon. Someone of yours holding in it, with none of theirs there, lights a lamp every tick. Light three and you win.",
                "Pick PRESS (P, or a left click), click a square of the yard, then lock in your orders (Enter). Pressing an empty square means walking onto it and holding it. Each turn is three ticks, and every creature takes one step a tick.",
                "The visitors are staying at home for this one.",
            ],
            Self::Facing => &[
                "Everyone has three composure. Lose it all and a Formiga is dizzy: it sits out the rest of the match. The visitors are tired from the trip, so one each.",
                "A HOLD turns any single PRESS away. Facing the presser, it costs the presser a composure; pressed from the side or behind, nobody is hurt. You choose which way a HOLD faces as you give it: R, or the mouse wheel, turns it.",
                "Two visitors are about to press two of yours beside the yard, from the east. HOLD both squares (drag to name more than one) facing east. With no orders, yours would press back, and two presses that meet cost both sides a composure.",
            ],
            Self::Flank => &[
                "One holder alone turns any single PRESS away. Press it from two sides at once and its hold breaks: it loses a composure, it is pushed out of the yard, and it stops for the turn.",
                "PRESS the square the visitor holds. Your team splits the job by itself: two walk to its sides this turn, and both press together next turn.",
                "Breaking a hold in the yard lights a lamp for you, and anyone of yours left standing in the yard starts holding at once.",
            ],
            Self::Trick => &[
                "A holder sees what comes at it from the front. A TRICK goes round to its side or back instead, and breaks its hold for the turn: it loses a composure, and though it stays on its feet and still counts in the yard, a PRESS can now push it off.",
                "TRICK the square the visitor holds. It faces the middle of the yard, so whoever goes will come at it from the side or behind.",
                "A TRICK from the front is read and does nothing, and so is one from a square another of their holders faces. Anyone caught by a PRESS while tricking is pushed, stopped and robbed.",
            ],
            Self::Orders => &[
                "You give up to three orders a turn, and your team shares them out. You never say who goes where: the nearest, or whoever is already at it, usually takes a job.",
                "While you give orders, the board shows who will probably go where. Name a square in two orders and more of the team goes to it.",
                "A visitor holds the bottom of the yard, and their runner is heading for the tap by your near edge, to set the sprinkler off. PRESS the holder's square, and HOLD the square above the tap: a holder beside a tap turns a pull away.",
            ],
            Self::Weather => &[
                "Every arena has one map event, warned a whole turn before. Here the sprinkler is coming this turn, on its second tick: everyone in the yard is soaked off balance and stops counting.",
                "Your holder in the yard is carrying a leaf umbrella. Anyone carrying something can USE ITEM as a bonus beyond your three orders: press USE ITEM (U), then click the square to use it on, green within its reach. Using takes its whole first tick, then it does its job. Use the umbrella on your holder's own square and HOLD that square: the sprinkler cannot soak it.",
                "Things glint a turn before they turn up, and anyone who steps on one with empty paws picks it up; INTERACT on one to send someone for it. INTERACT on a tap by the near edge to set the sprinkler off early.",
            ],
            Self::Personality => &[
                "Out in a real match, your companions are themselves. Each turn about one of your three does something of its own: a wary one hangs back from the other team, a curious one wanders the outside lanes or after something glinting, a friendly one keeps beside a friend.",
                "Nobody disobeys: who takes which job, and the way each walks there, is who they are. While you give orders, the board shows who will probably go where, and after each turn the recap says who went their own way, and why.",
                "These are your own three, as they are. Light the beacon before the sprinkler comes on turn 4.",
            ],
        }
    }

    /// Squares the lesson points at on the board.
    pub fn marked(self) -> Vec<Tile> {
        match self {
            Self::Beacon | Self::Personality => Arena::backyard().zone,
            Self::Facing => FACING_SQUARES.to_vec(),
            Self::Flank | Self::Trick => vec![TOP],
            Self::Orders => vec![BOTTOM, BY_TAP],
            Self::Weather => vec![MIDDLE],
        }
    }

    /// The turns the Handler has to do it in.
    pub fn turns(self) -> u8 {
        match self {
            Self::Beacon | Self::Flank | Self::Orders | Self::Personality => 3,
            Self::Trick => 2,
            Self::Facing | Self::Weather => 1,
        }
    }

    /// Whether the Handler's companions are taken to lean no way at all.
    pub fn plain(self) -> bool {
        self != Self::Personality
    }

    /// The board the lesson begins from. `rosters` are the Handler's three and the visitors'
    /// three; the visitors always lean no way at all, and the Handler's do too unless the lesson
    /// is about who they are.
    pub fn stage(
        self,
        ruleset: Ruleset,
        seed: u64,
        rosters: [Vec<Profile>; 2],
    ) -> Result<Match, MatchError> {
        let [ours, theirs] = rosters;
        let ours = if self.plain() {
            ours.into_iter().map(plainly).collect()
        } else {
            ours
        };
        let theirs = theirs.into_iter().map(plainly).collect();
        let mut m = Match::new(Arena::backyard(), ruleset, seed, [ours, theirs])?;
        let place = |m: &mut Match, side: Side, at: [(i8, i8); TEAM_SIZE]| {
            for (i, (x, y)) in at.into_iter().enumerate() {
                m.board.fighters[side.index()][i].at = Tile::new(x, y);
            }
        };
        // One of theirs already holding where the lesson puts it, facing this way.
        let holding = |m: &mut Match, side: Side, i: usize, facing: Facing| {
            let f = &mut m.board.fighters[side.index()][i];
            f.holding = true;
            f.facing = facing;
        };
        match self {
            Self::Beacon => {
                place(&mut m, YOU, [(1, 2), (1, 3), (1, 4)]);
                place(&mut m, YOU.other(), [(8, 1), (8, 3), (8, 5)]);
            }
            Self::Facing => {
                place(&mut m, YOU, [(2, 3), (1, 1), (2, 5)]);
                place(&mut m, YOU.other(), [(3, 3), (3, 5), (8, 5)]);
                for f in &mut m.board.fighters[YOU.other().index()] {
                    f.composure = 1;
                }
            }
            Self::Flank => {
                place(&mut m, YOU, [(2, 1), (1, 3), (2, 5)]);
                place(&mut m, YOU.other(), [(4, 2), (8, 3), (8, 5)]);
                holding(&mut m, YOU.other(), 0, Facing::South);
            }
            Self::Trick => {
                place(&mut m, YOU, [(2, 1), (1, 3), (2, 5)]);
                place(&mut m, YOU.other(), [(4, 2), (8, 3), (8, 5)]);
                holding(&mut m, YOU.other(), 0, Facing::South);
            }
            Self::Orders => {
                place(&mut m, YOU, [(2, 3), (2, 5), (5, 5)]);
                place(&mut m, YOU.other(), [(4, 4), (8, 3), (8, 1)]);
                holding(&mut m, YOU.other(), 0, Facing::North);
            }
            Self::Weather => {
                place(&mut m, YOU, [(4, 3), (1, 1), (1, 5)]);
                place(&mut m, YOU.other(), [(4, 2), (8, 3), (8, 5)]);
                // Turn 4 is next, and the sprinkler was warned at the end of turn 3, as it always
                // is: it lands on turn 4's second tick.
                m.board.turn = 3;
                m.board.capture = [1, 1];
                holding(&mut m, YOU, 0, Facing::North);
                m.board.fighters[YOU.index()][0].carrying = Some(Item::Leaf);
                holding(&mut m, YOU.other(), 0, Facing::South);
                m.board.event_at = Some(4 * u16::from(m.ruleset.ticks_per_turn) + 2);
                m.board.event_tiles = m.arena.scenery_tiles(&[]);
            }
            Self::Personality => {}
        }
        Ok(m)
    }

    /// The visitors' orders this turn.
    pub fn visitors(self, m: &Match) -> TeamPlan {
        let at = |i: usize| m.board.fighters[YOU.other().index()][i].at;
        let hold = |squares: Vec<Tile>, facing: Facing| Directive::Hold { squares, facing };
        let home = |squares: Vec<Tile>| hold(squares, Facing::West);
        match self {
            Self::Beacon | Self::Personality => {
                vec![home(m.arena.starts[YOU.other().index()].to_vec())]
            }
            Self::Facing => vec![
                Directive::Press {
                    squares: vec![FACING_SQUARES[0]],
                },
                Directive::Press {
                    squares: vec![FACING_SQUARES[1]],
                },
                home(vec![at(2)]),
            ],
            Self::Flank | Self::Trick | Self::Weather => vec![
                hold(vec![TOP], Facing::South),
                home(vec![Tile::new(8, 3), Tile::new(8, 5)]),
            ],
            Self::Orders => vec![
                hold(vec![BOTTOM], Facing::North),
                Directive::Interact { at: TAP },
                home(vec![Tile::new(8, 1)]),
            ],
        }
    }

    /// How the lesson stands, from every event since it began and every plan the Handler gave.
    pub fn judge(self, m: &Match, events: &[Event], given: &[TeamPlan]) -> Verdict {
        let played = given.len() as u8;
        let over = played >= self.turns() || m.outcome().is_some();
        let won_by_capture = m
            .outcome()
            .is_some_and(|o| o.winner == Some(YOU) && o.victory == Some(Victory::Capture));
        let theirs = |slot: &Slot| slot.side != YOU;
        let ours = |slot: &Slot| slot.side == YOU;
        let flanked = events.iter().any(|e| {
            matches!(&e.kind, EventKind::Flanked { target, by, .. }
                if theirs(target) && by.iter().all(ours))
        });
        let learned = |words: &str| Verdict::Learned(words.to_owned());
        let try_again = |hint: &str| Verdict::TryAgain(hint.to_owned());
        let decide = |done: bool, words: &str, hint: &str| {
            if done {
                learned(words)
            } else if over {
                try_again(hint)
            } else {
                Verdict::Going
            }
        };
        match self {
            Self::Beacon => {
                let named_yard = given.iter().flatten().any(|d| {
                    matches!(d, Directive::Press { .. } | Directive::Hold { .. })
                        && d.squares().iter().any(|t| m.arena.in_zone(*t))
                });
                if won_by_capture && !named_yard {
                    try_again(
                        "Your team found the yard on its own: with no orders, each falls back on the nearest square of it. Give it a PRESS this time.",
                    )
                } else {
                    decide(
                        won_by_capture,
                        "All three lamps lit. Holding the yard alone is how most matches are won, and why the other team always comes for it.",
                        "Pick PRESS, click a square of the yard, and lock in. Someone has to be holding there, with none of theirs in it.",
                    )
                }
            }
            Self::Facing => {
                let dizzy = m.board.fighters[YOU.other().index()]
                    .iter()
                    .filter(|f| f.out)
                    .count();
                let lost = events
                    .iter()
                    .any(|e| matches!(&e.kind, EventKind::Composure { slot, .. } if ours(slot)));
                let unfaced = events.iter().any(|e| {
                    matches!(&e.kind, EventKind::Repelled { holders, faced, .. }
                        if holders.iter().any(ours) && faced.is_empty())
                });
                let hint = if lost {
                    "Yours pressed back, and two presses that meet cost both sides. HOLD the two squares instead, facing east."
                } else if unfaced {
                    "Your holders turned the presses away, but facing the wrong way they cost the pressers nothing. HOLD facing east, toward them."
                } else {
                    "HOLD the two squares beside the yard, facing east, toward the pressers."
                };
                decide(
                    dizzy >= 2 && !lost,
                    "Both dizzy, and yours untouched. A HOLD facing a PRESS wins it; from the side or behind, it only turns it away. Which way your holders face is what the other team will try to get round.",
                    hint,
                )
            }
            Self::Flank => decide(
                flanked,
                "Hold broken. Two holders side by side turn a flank away together, so it takes three to break one of them: the answer to a flank is a friend beside you.",
                "PRESS the square the visitor is holding, and let your team walk to its sides. Once two are in place at the start of a turn, they press together.",
            ),
            Self::Trick => {
                let tricked = events.iter().any(|e| {
                    matches!(&e.kind, EventKind::Tricked { target, by, .. }
                        if theirs(target) && by.iter().all(ours))
                });
                let read = events
                    .iter()
                    .any(|e| matches!(&e.kind, EventKind::TrickRead { by, .. } if ours(by)));
                decide(
                    tricked,
                    "Tricked. A holder is strong to its front and weak behind: hold where it faces the danger, and keep a friend beside it to watch its sides.",
                    if read {
                        "That TRICK came from the front, and was read. TRICK the holder's square and let someone go round to its side or back."
                    } else {
                        "TRICK the square the visitor holds: someone of yours goes round to its side or back."
                    },
                )
            }
            Self::Orders => {
                let tap_turned = events.iter().any(
                    |e| matches!(&e.kind, EventKind::SwitchTurned { slot, .. } if theirs(slot)),
                );
                if tap_turned {
                    try_again(
                        "Their runner got to the tap. HOLD the square above the tap alongside your PRESS: a holder beside it turns the pull away.",
                    )
                } else {
                    decide(
                        flanked,
                        "Both jobs done from one plan. Three orders is never three creatures each told what to do: it is what you want, and they work out who.",
                        "PRESS the holder's square so two of yours flank it, and HOLD the square above the tap with another order.",
                    )
                }
            }
            Self::Weather => decide(
                won_by_capture,
                "Dry under the leaf, alone in the yard while theirs was soaked. The sprinkler clears the yard for both teams alike: what it rewards is being ready for it.",
                "USE ITEM: your holder's leaf umbrella, on its own square, and HOLD that square as well. The umbrella opens before the sprinkler lands.",
            ),
            Self::Personality => decide(
                won_by_capture,
                "Lit, by your three as they are. Knowing who wanders, who hangs back and who keeps beside a friend is what makes a Handler better than a plan.",
                "Read who will probably go where before you lock in, and name a square more than once if someone keeps going their own way.",
            ),
        }
    }
}

/// A creature taken to lean no way at all and never act on a whim, keeping its name and look.
fn plainly(p: Profile) -> Profile {
    Profile {
        tension: None,
        nature: Nature {
            impulsiveness: 0,
            ..Nature::MIDDLING
        },
        leanings: Leanings::default(),
        bonds: Vec::new(),
        ..p
    }
}
