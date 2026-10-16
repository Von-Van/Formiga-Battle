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
use crate::order::{Directive, Side, Slot, TeamPlan};
use crate::profile::{Leanings, Nature, Profile};
use crate::ruleset::Ruleset;

/// The side the Handler plays in a lesson.
pub const YOU: Side = Side::Left;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lesson {
    /// TAKE a square of the yard and light the beacon.
    Beacon,
    /// A holder turns a contest away; a walker caught on its way is interrupted; three lost is
    /// dizzy.
    Composure,
    /// Come at a lone holder from two sides and break its hold.
    Flank,
    /// Two jobs in one plan: a flank, and the tap kept.
    Orders,
    /// The sprinkler is warned a turn ahead; a leaf umbrella keeps a holder dry.
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
/// The garden tap on the visitors' side of the near edge.
const TAP: Tile = Tile::new(7, 6);
/// The squares beside the yard the visitors come for in the lesson on composure.
const COMPOSURE_SQUARES: [Tile; 2] = [Tile::new(2, 3), Tile::new(2, 5)];

impl Lesson {
    pub const ALL: [Self; 6] = [
        Self::Beacon,
        Self::Composure,
        Self::Flank,
        Self::Orders,
        Self::Weather,
        Self::Personality,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Beacon => "beacon",
            Self::Composure => "composure",
            Self::Flank => "flank",
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
            Self::Composure => "Hold and contest",
            Self::Flank => "Flanks",
            Self::Orders => "Three orders, one team",
            Self::Weather => "The sprinkler and found things",
            Self::Personality => "Who they are",
        }
    }

    /// What the lesson asks, in a line.
    pub fn goal(self) -> &'static str {
        match self {
            Self::Beacon => "TAKE a square of the yard and light all three lamps.",
            Self::Composure => "Make both visitors who come at you dizzy, this turn.",
            Self::Flank => "Break the lone holder's hold by coming at it from two sides.",
            Self::Orders => "Flank the holder, and keep their runner off the tap.",
            Self::Weather => "Light your last two lamps on turn 4, through the sprinkler.",
            Self::Personality => "Light the beacon with your three as they really are.",
        }
    }

    /// What the lesson says before the first turn, a few short paragraphs.
    pub fn notes(self) -> &'static [&'static str] {
        match self {
            Self::Beacon => &[
                "The yard in the middle is the beacon. Someone of yours holding in it, with none of theirs there, lights a lamp every tick. Light three and you win.",
                "Pick TAKE (T, or a left click), click a square of the yard, then lock in your orders (Enter). Each turn is three ticks, and every creature takes one step a tick.",
                "The visitors are staying at home for this one.",
            ],
            Self::Composure => &[
                "Everyone has three composure. Lose it all and a Formiga is dizzy: it sits out the rest of the match. The visitors are tired from the trip, so one each.",
                "A contest against someone holding is turned away, and the contester loses composure. A contest against someone walking stops it where it is, and it loses composure instead.",
                "Two visitors are about to come at two of yours, beside the yard. DEFEND both squares (drag to name more than one): standing on their own squares, your two hold, and turn the visitors away.",
            ],
            Self::Flank => &[
                "One holder alone turns any single contest away. Come at it from two sides at once and its hold breaks: it is pushed out of the yard and stops for the turn.",
                "TAKE the square the visitor holds. Your team splits the job by itself: two walk to its sides this turn, and both come at it together next turn.",
                "Breaking a hold in the yard lights a lamp for you, and anyone of yours left standing in the yard starts holding at once.",
            ],
            Self::Orders => &[
                "You give up to three orders a turn, and your team shares them out. You never say who goes where: the nearest, or whoever is already at it, usually takes a job.",
                "While you give orders, the board shows who will probably go where. Name a square in two orders and more of the team goes to it.",
                "A visitor holds the bottom of the yard, and their runner is heading for the tap by your near edge, to set the sprinkler off. TAKE the holder's square, and DEFEND the tap: a tap named in DEFEND is guarded from beside it.",
            ],
            Self::Weather => &[
                "Every arena has one map event, and it is always warned a whole turn before. On the Backyard it is the sprinkler, on turns 4, 8 and 12: everyone in the yard is soaked off balance and stops counting, whoever they are.",
                "Your holder in the yard is carrying a leaf umbrella. INTERACT on it this turn and it opens the umbrella over itself, for this turn and the next. Then hold the yard on turn 4 while the sprinkler soaks theirs.",
                "Found things glint a turn before they turn up, and anyone who steps on one with empty paws picks it up. The taps on the near edge set the sprinkler off early: INTERACT on one.",
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
            Self::Composure => COMPOSURE_SQUARES.to_vec(),
            Self::Flank => vec![TOP],
            Self::Orders => vec![BOTTOM, TAP],
            Self::Weather => vec![MIDDLE],
        }
    }

    /// The turns the Handler has to do it in.
    pub fn turns(self) -> u8 {
        match self {
            Self::Beacon | Self::Flank | Self::Orders | Self::Personality => 3,
            Self::Composure => 1,
            Self::Weather => 2,
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
        match self {
            Self::Beacon => {
                place(&mut m, YOU, [(1, 2), (1, 3), (1, 4)]);
                place(&mut m, YOU.other(), [(8, 1), (8, 3), (8, 5)]);
            }
            Self::Composure => {
                place(&mut m, YOU, [(2, 3), (1, 1), (2, 5)]);
                place(&mut m, YOU.other(), [(3, 3), (3, 5), (8, 5)]);
                for f in &mut m.board.fighters[YOU.other().index()] {
                    f.composure = 1;
                }
            }
            Self::Flank => {
                place(&mut m, YOU, [(2, 1), (1, 3), (2, 5)]);
                place(&mut m, YOU.other(), [(4, 2), (8, 3), (8, 5)]);
                m.board.fighters[YOU.other().index()][0].holding = true;
            }
            Self::Orders => {
                place(&mut m, YOU, [(2, 3), (2, 5), (5, 5)]);
                place(&mut m, YOU.other(), [(4, 4), (8, 3), (8, 1)]);
                m.board.fighters[YOU.other().index()][0].holding = true;
            }
            Self::Weather => {
                place(&mut m, YOU, [(4, 3), (1, 1), (1, 5)]);
                place(&mut m, YOU.other(), [(4, 2), (8, 3), (8, 5)]);
                // Turn 3 is next; the sprinkler comes round by itself on turn 4, warned at the end
                // of turn 3.
                m.board.turn = 2;
                m.board.capture = [1, 1];
                m.board.fighters[YOU.index()][0].holding = true;
                m.board.fighters[YOU.index()][0].carrying = Some(Item::Leaf);
                m.board.fighters[YOU.other().index()][0].holding = true;
            }
            Self::Personality => {}
        }
        Ok(m)
    }

    /// The visitors' orders this turn.
    pub fn visitors(self, m: &Match) -> TeamPlan {
        let at = |i: usize| m.board.fighters[YOU.other().index()][i].at;
        let defend = |squares: Vec<Tile>| Directive::Defend { squares };
        match self {
            Self::Beacon | Self::Personality => {
                vec![defend(m.arena.starts[YOU.other().index()].to_vec())]
            }
            Self::Composure => vec![
                Directive::Take {
                    squares: vec![COMPOSURE_SQUARES[0]],
                },
                Directive::Take {
                    squares: vec![COMPOSURE_SQUARES[1]],
                },
                defend(vec![at(2)]),
            ],
            Self::Flank => vec![defend(vec![TOP, Tile::new(8, 3), Tile::new(8, 5)])],
            Self::Orders => vec![
                defend(vec![BOTTOM]),
                Directive::Interact { at: TAP },
                defend(vec![Tile::new(8, 1)]),
            ],
            Self::Weather => vec![defend(vec![TOP, Tile::new(8, 3), Tile::new(8, 5)])],
        }
    }

    /// How the lesson stands, from every event since it began and every plan the Handler gave.
    pub fn judge(self, m: &Match, events: &[Event], given: &[TeamPlan]) -> Verdict {
        let played = given.len() as u8;
        let out_of_turns = played >= self.turns();
        let won_by_capture = m
            .outcome()
            .is_some_and(|o| o.winner == Some(YOU) && o.victory == Some(Victory::Capture));
        let theirs = |slot: &Slot| slot.side != YOU;
        let ours = |slot: &Slot| slot.side == YOU;
        let try_again = |hint: &str| Verdict::TryAgain(hint.to_owned());
        match self {
            Self::Beacon => {
                let named_yard = given.iter().flatten().any(|d| {
                    !matches!(d, Directive::Interact { .. })
                        && d.squares().iter().any(|t| m.arena.in_zone(*t))
                });
                if won_by_capture && named_yard {
                    Verdict::Learned(
                        "All three lamps lit. Holding the yard alone is how most matches are won, and why the other team always comes for it.".to_owned(),
                    )
                } else if won_by_capture {
                    try_again(
                        "Your team found the yard on its own: with no orders, each falls back on the nearest square of it. Give it a TAKE this time.",
                    )
                } else if out_of_turns || m.outcome().is_some() {
                    try_again(
                        "Pick TAKE, click a square of the yard, and lock in. Someone has to be holding there, with none of theirs in it.",
                    )
                } else {
                    Verdict::Going
                }
            }
            Self::Composure => {
                let dizzy = m
                    .board
                    .fighters
                    .get(YOU.other().index())
                    .map_or(0, |team| team.iter().filter(|f| f.out).count());
                if dizzy >= 2 {
                    Verdict::Learned(
                        "Two dizzy. A holder wins a contest; a walker loses one. That is why a team holds where it can and walks where nobody can reach it.".to_owned(),
                    )
                } else if out_of_turns || m.outcome().is_some() {
                    try_again(
                        "DEFEND the two squares the visitors are coming for, so the two of yours on them are holding when the visitors arrive.",
                    )
                } else {
                    Verdict::Going
                }
            }
            Self::Flank => {
                let flanked = events.iter().any(|e| {
                    matches!(&e.kind, EventKind::Flanked { target, by, .. }
                        if theirs(target) && by.iter().all(ours))
                });
                if flanked {
                    Verdict::Learned(
                        "Hold broken. Two holders side by side turn a flank away together, so it takes three to break one of them: the answer to a flank is a friend beside you.".to_owned(),
                    )
                } else if out_of_turns || m.outcome().is_some() {
                    try_again(
                        "TAKE the square the visitor is holding, and let your team walk to its sides. Once two are in place at the start of a turn, they come at it together.",
                    )
                } else {
                    Verdict::Going
                }
            }
            Self::Orders => {
                let tap_turned = events.iter().any(
                    |e| matches!(&e.kind, EventKind::SwitchTurned { slot, .. } if theirs(slot)),
                );
                let flanked = events.iter().any(|e| {
                    matches!(&e.kind, EventKind::Flanked { target, by, .. }
                        if theirs(target) && by.iter().all(ours))
                });
                if tap_turned {
                    try_again(
                        "Their runner got to the tap. Give a DEFEND on the tap alongside your TAKE: the one nearest it guards it from beside it.",
                    )
                } else if flanked {
                    Verdict::Learned(
                        "Both jobs done from one plan. Three orders is never three creatures each told what to do: it is what you want, and they work out who.".to_owned(),
                    )
                } else if out_of_turns || m.outcome().is_some() {
                    try_again(
                        "TAKE the holder's square so two of yours flank it, and DEFEND the tap with your other order.",
                    )
                } else {
                    Verdict::Going
                }
            }
            Self::Weather => {
                if won_by_capture {
                    Verdict::Learned(
                        "Dry under the leaf, alone in the yard while theirs was soaked. The sprinkler clears the yard for both teams alike: what it rewards is being ready for it.".to_owned(),
                    )
                } else if out_of_turns || m.outcome().is_some() {
                    try_again(
                        "INTERACT on your holder in the yard on turn 3, to open its leaf umbrella, then DEFEND its square on turn 4.",
                    )
                } else {
                    Verdict::Going
                }
            }
            Self::Personality => {
                if won_by_capture {
                    Verdict::Learned(
                        "Lit, by your three as they are. Knowing who wanders, who hangs back and who keeps beside a friend is what makes a Handler better than a plan.".to_owned(),
                    )
                } else if out_of_turns || m.outcome().is_some() {
                    try_again(
                        "Read who will probably go where before you lock in, and name a square more than once if someone keeps going their own way.",
                    )
                } else {
                    Verdict::Going
                }
            }
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
