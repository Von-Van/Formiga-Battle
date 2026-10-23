//! Playbooks: a Handler's plan for a match, written before it starts, in one to three stages of
//! what they want (secure the beacon, put pressure on, draw the other team out, counter its
//! holders, fetch what turns up, work the board) rather than where each order goes.
//!
//! A playbook is not a new kind of order. Each turn it compiles to the same team plan a Handler
//! could have given by hand: at most three PRESS, HOLD, TRICK and INTERACT orders naming squares,
//! and a USE ITEM for anyone carrying something. The team then shares that plan out by itself
//! exactly as it would any other (`muster`), and the turn resolves under the same rules. Nothing
//! here changes who takes which job, what a job is worth, or what an order does.
//!
//! What a playbook knows that an order does not is the Handler's own trio. Where an intent can be
//! worded more than one way (one PRESS naming two squares, or two PRESSes naming one each; a HOLD
//! facing the other team or facing across), the compiler asks the team's own share-out which
//! wording these three creatures would carry out as meant, and gives that one. It reads only what
//! a Handler can see: the board, the arena and its own creatures. The other team's leanings never
//! come into it.
//!
//! Every turn a playbook plays says so in a [`Compiled`] record: which stage ran, what it became,
//! whether it moved on from the stage before and why, and why it played differently from what
//! the stage asked for, when it did. Compiling is pure: the same board and the same playbook give
//! the same plan, so a match played from a playbook replays from its recorded plans like any
//! other.

use crate::ai::{self, Coach};
use crate::arena::Arena;
use crate::event::Victory;
use crate::game::Match;
use crate::grid::Tile;
use crate::muster::{self, Role, Why};
use crate::order::{Directive, Facing, PLAN_SIZE, Side, Slot, TeamPlan};
use crate::rng::{self, Purpose};
use crate::ruleset::Ruleset;
use crate::tournament::{self, Record, drawn_creature};
use serde::{Deserialize, Serialize};

/// The most stages a playbook has.
pub const MAX_STAGES: usize = 3;
/// The most turns one stage can be given before it is taken to have failed.
pub const MAX_STAGE_TURNS: u8 = 4;
/// The longest name a playbook can have, as long as a trio's.
pub const NAME_LEN: usize = 32;
/// One of theirs further than this from the beacon's middle is not worth going after under
/// PRESSURE.
const PRESSURE_REACH: u8 = 5;
/// Something lying further than this from every one of ours with empty paws is not worth fetching.
const FETCH_REACH: u8 = 5;

/// What a stage wants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    /// Get onto the beacon and keep it: PRESS onto its free squares, HOLD the ones already ours.
    Secure,
    /// Go at the other team: PRESS whoever of theirs is in or near the beacon, two at a holder so
    /// it is flanked.
    Pressure,
    /// Make them come to us: HOLD squares of the beacon facing them, so a PRESS at us is turned
    /// away and costs them composure; turn the tap on them when it soaks them.
    DrawOut,
    /// Answer their holders: TRICK them from the side or behind, and HOLD facing anyone coming.
    Counter,
    /// Go for found things: INTERACT with what lies or glints nearby.
    Fetch,
    /// Work the board: turn the tap, pull the lever.
    Work,
}

impl Intent {
    pub const ALL: [Self; 6] = [
        Self::Secure,
        Self::Pressure,
        Self::DrawOut,
        Self::Counter,
        Self::Fetch,
        Self::Work,
    ];

    /// The name a Handler reads.
    pub fn label(self) -> &'static str {
        match self {
            Self::Secure => "Secure",
            Self::Pressure => "Pressure",
            Self::DrawOut => "Draw out",
            Self::Counter => "Counter",
            Self::Fetch => "Fetch",
            Self::Work => "Work the board",
        }
    }

    /// What it counts as having got done, as the recap would say it.
    pub fn goal(self) -> &'static str {
        match self {
            Self::Secure => "two of ours holding the beacon and none of theirs on it",
            Self::Pressure => "one of theirs shaken or knocked out",
            Self::DrawOut => "one of theirs shaken, or drawn off the beacon",
            Self::Counter => "a hold of theirs on the beacon broken",
            Self::Fetch => "one more of ours carrying something",
            Self::Work => "the tap turned or the lever pulled",
        }
    }

    /// The jobs a creature does when it is carrying out this intent.
    fn roles(self) -> &'static [Role] {
        match self {
            Self::Secure | Self::DrawOut => &[Role::Claim, Role::Guard],
            Self::Pressure => &[Role::Oust, Role::Flank, Role::Pin, Role::Intercept],
            Self::Counter => &[Role::Trick, Role::Claim, Role::Guard],
            Self::Fetch => &[Role::Fetch],
            Self::Work => &[Role::Pull, Role::Wait, Role::Guard],
        }
    }
}

/// Which way across the board a stage looks first, among squares or creatures otherwise as good
/// as each other. Measured from each team's own edge, so it plays the same from both sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Aim {
    /// Nearest the beacon's middle.
    Centre,
    /// Nearest our own side.
    Near,
    /// Nearest their side.
    Far,
}

/// How many of the team a stage leaves on the beacon while the rest carry out its intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Posture {
    /// Nobody kept back: everyone on the intent.
    AllIn,
    /// One kept on the beacon.
    Balanced,
    /// Two kept on the beacon, one on the intent.
    Guarded,
}

impl Posture {
    const fn kept(self) -> usize {
        match self {
            Self::AllIn => 0,
            Self::Balanced => 1,
            Self::Guarded => 2,
        }
    }
}

/// Which way a stage's HOLDs face.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FacingHint {
    /// Toward the nearest of the other team.
    Them,
    /// Toward the other team's side of the board.
    Across,
}

/// What happens when a stage runs out of turns without getting its goal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fallback {
    /// Go on to the next stage.
    Next,
    /// Give the same stage its turns again.
    Again,
    /// Go back to the first stage.
    First,
}

/// One stage of a playbook.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stage {
    pub intent: Intent,
    pub aim: Aim,
    pub posture: Posture,
    pub facing: FacingHint,
    /// How many turns it is given to get its goal, 1 to `MAX_STAGE_TURNS`.
    pub turns: u8,
    pub fallback: Fallback,
}

impl Stage {
    /// A stage with the usual settings: aimed at the middle, one kept on the beacon, facing them,
    /// two turns, then on to the next.
    pub const fn new(intent: Intent) -> Self {
        Self {
            intent,
            aim: Aim::Centre,
            posture: Posture::Balanced,
            facing: FacingHint::Them,
            turns: 2,
            fallback: Fallback::Next,
        }
    }

    pub const fn aim(mut self, aim: Aim) -> Self {
        self.aim = aim;
        self
    }

    pub const fn posture(mut self, posture: Posture) -> Self {
        self.posture = posture;
        self
    }

    pub const fn facing(mut self, facing: FacingHint) -> Self {
        self.facing = facing;
        self
    }

    pub const fn turns(mut self, turns: u8) -> Self {
        self.turns = turns;
        self
    }

    pub const fn fallback(mut self, fallback: Fallback) -> Self {
        self.fallback = fallback;
        self
    }
}

/// How a playbook's stages share out the match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Timing {
    /// One stage at a time: stage one from the first turn, each holding until its goal is got or
    /// its turns run out.
    Successive,
    /// Every stage every turn, each given a share of the team's three orders, the first stage
    /// first.
    Bundled,
}

/// A Handler's plan for a match.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Playbook {
    pub name: String,
    pub stages: Vec<Stage>,
    pub timing: Timing,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PlaybookError {
    #[error("a playbook needs a name of 1 to {NAME_LEN} characters")]
    Name,
    #[error("a playbook has 1 to {MAX_STAGES} stages")]
    Stages,
    #[error("stage {0} must be given 1 to {MAX_STAGE_TURNS} turns")]
    Turns(usize),
}

impl Playbook {
    pub fn new(name: &str, stages: &[Stage]) -> Self {
        Self {
            name: name.to_owned(),
            stages: stages.to_vec(),
            timing: Timing::Successive,
        }
    }

    /// The same playbook with its stages played together each turn.
    pub fn bundled(mut self) -> Self {
        self.timing = Timing::Bundled;
        self
    }

    pub fn validate(&self) -> Result<(), PlaybookError> {
        let len = self.name.trim().chars().count();
        if len == 0 || len > NAME_LEN {
            return Err(PlaybookError::Name);
        }
        if self.stages.is_empty() || self.stages.len() > MAX_STAGES {
            return Err(PlaybookError::Stages);
        }
        if let Some(i) = self
            .stages
            .iter()
            .position(|s| s.turns == 0 || s.turns > MAX_STAGE_TURNS)
        {
            return Err(PlaybookError::Turns(i + 1));
        }
        Ok(())
    }

    /// A few playbooks of the kind a Handler might write, for trying the compiler out.
    pub fn samples() -> Vec<Self> {
        use Intent::*;
        vec![
            Self::new(
                "Take and keep",
                &[
                    Stage::new(Secure).turns(3),
                    Stage::new(DrawOut).posture(Posture::Guarded).turns(3),
                    Stage::new(Counter).fallback(Fallback::First),
                ],
            ),
            Self::new(
                "Early pressure",
                &[
                    Stage::new(Pressure).aim(Aim::Near).posture(Posture::AllIn),
                    Stage::new(Secure).turns(3).fallback(Fallback::First),
                ],
            ),
            Self::new(
                "Finders keepers",
                &[
                    Stage::new(Fetch).aim(Aim::Near),
                    Stage::new(Pressure),
                    Stage::new(Secure).turns(3).fallback(Fallback::Again),
                ],
            ),
            Self::new(
                "Tap and take",
                &[
                    Stage::new(Work).aim(Aim::Far),
                    Stage::new(Secure).posture(Posture::AllIn).turns(3),
                    Stage::new(Counter).fallback(Fallback::Again),
                ],
            ),
            Self::new(
                "Sit and sting",
                &[
                    Stage::new(DrawOut).posture(Posture::Guarded).turns(3),
                    Stage::new(Counter),
                    Stage::new(Secure).turns(3).fallback(Fallback::First),
                ],
            ),
        ]
    }
}

/// Why a stage played differently this turn from what it asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "deviation", rename_all = "snake_case")]
pub enum Deviation {
    /// Nothing to aim it at (nothing to fetch, nothing ready to work, no holder of theirs to
    /// trick, nobody of theirs near enough to press): its hands went to the beacon instead.
    NoTarget,
    /// Too few of ours standing to keep the posture's hands on the beacon and still carry out the
    /// intent: one was taken off the beacon for it.
    Shorthanded,
    /// Every stage has had its turn: the team plays for the beacon.
    Finished,
}

/// Why the playbook is on a different stage from last turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "advance", rename_all = "snake_case")]
pub enum Advance {
    /// The stage got its goal.
    Met { stage: u8 },
    /// The stage ran out of turns without it, and its fallback was taken.
    RanOut { stage: u8, fallback: Fallback },
}

/// One stage's part in a turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ran {
    /// By place in the playbook, from 0; `None` once every stage has had its turn.
    pub stage: Option<u8>,
    pub intent: Intent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deviation: Option<Deviation>,
}

/// The wording the compiler gave rather than the plainest one, because of who is on the team.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Reworded {
    /// How many of the team the plainest wording would have put on the stage's intent.
    pub plain: u8,
    /// How many the wording given does.
    pub given: u8,
}

/// What a playbook came to on one turn: which stage ran, what it became, and why it played
/// otherwise than asked, when it did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compiled {
    pub turn: u8,
    pub side: Side,
    pub timing: Timing,
    pub ran: Vec<Ran>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub advanced: Vec<Advance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reworded: Option<Reworded>,
    /// The team plan it compiled to: what the match was sent.
    pub plan: TeamPlan,
}

impl Compiled {
    /// The turn in one plain line, as a whole-match log or a notebook would say it.
    pub fn line(&self, playbook: &Playbook) -> String {
        let mut parts: Vec<String> = Vec::new();
        for a in &self.advanced {
            parts.push(match a {
                Advance::Met { stage } => format!("stage {} got its goal", stage + 1),
                Advance::RanOut { stage, fallback } => format!(
                    "stage {} ran out of turns, so {}",
                    stage + 1,
                    match fallback {
                        Fallback::Next => "on to the next",
                        Fallback::Again => "it goes again",
                        Fallback::First => "back to the first",
                    }
                ),
            });
        }
        let stages: Vec<String> = self
            .ran
            .iter()
            .map(|r| {
                let which = match r.stage {
                    Some(s) => format!("stage {}, {}", s + 1, r.intent.label()),
                    None => format!("after the last stage, {}", r.intent.label()),
                };
                match r.deviation {
                    Some(Deviation::NoTarget) => format!("{which}, with nothing to aim it at"),
                    Some(Deviation::Shorthanded) => format!("{which}, short of hands"),
                    Some(Deviation::Finished) | None => which,
                }
            })
            .collect();
        let mut line = format!(
            "Turn {}: {} played {}",
            self.turn,
            playbook.name,
            stages.join(" and ")
        );
        if !parts.is_empty() {
            line.push_str(&format!(" ({})", parts.join("; ")));
        }
        if let Some(r) = self.reworded {
            line.push_str(&format!(
                "; worded for this trio, {} of them on it rather than {}",
                r.given, r.plain
            ));
        }
        line.push('.');
        line
    }
}

/// What the board was when a stage began, to tell whether it got its goal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mark {
    /// The turn it began.
    turn: u8,
    their_composure: u32,
    their_standing: usize,
    their_in_zone: usize,
    their_holding_in_zone: usize,
    our_carrying: usize,
    worked: bool,
}

impl Mark {
    fn of(m: &Match, side: Side, turn: u8) -> Self {
        let o = side.other();
        Self {
            turn,
            their_composure: m.team(o).map(|x| u32::from(m.fighter(x).composure)).sum(),
            their_standing: m.team(o).count(),
            their_in_zone: in_zone(m, o).len(),
            their_holding_in_zone: in_zone(m, o)
                .into_iter()
                .filter(|e| muster::holds(m, *e))
                .count(),
            our_carrying: m
                .team(side)
                .filter(|x| m.fighter(*x).carrying.is_some())
                .count(),
            worked: worked(m),
        }
    }
}

fn in_zone(m: &Match, side: Side) -> Vec<Slot> {
    m.team(side)
        .filter(|s| m.arena.in_zone(m.fighter(*s).at))
        .collect()
}

/// Whether the tap has been turned or the lever pulled and has not yet come back round.
fn worked(m: &Match) -> bool {
    m.board.event_at.is_some() || m.board.swing_at.is_some()
}

/// Whether a stage begun at `mark` has got its goal on the board as it stands.
fn met(intent: Intent, m: &Match, side: Side, mark: &Mark) -> bool {
    let now = Mark::of(m, side, mark.turn);
    match intent {
        Intent::Secure => {
            let ours = in_zone(m, side)
                .into_iter()
                .filter(|s| m.fighter(*s).holding)
                .count();
            ours >= m.team(side).count().min(2) && now.their_in_zone == 0
        }
        Intent::Pressure => {
            now.their_composure < mark.their_composure || now.their_standing < mark.their_standing
        }
        Intent::DrawOut => {
            now.their_composure < mark.their_composure || now.their_in_zone < mark.their_in_zone
        }
        Intent::Counter => now.their_holding_in_zone < mark.their_holding_in_zone,
        Intent::Fetch => now.our_carrying > mark.our_carrying,
        Intent::Work => now.worked && !mark.worked,
    }
}

/// Plays a playbook for one team through a match: keeps which stage it is on, and compiles each
/// turn's plan.
#[derive(Clone, Debug)]
pub struct Runner {
    pub playbook: Playbook,
    pub side: Side,
    stage: usize,
    mark: Option<Mark>,
    /// Every turn's record, in order.
    pub log: Vec<Compiled>,
}

impl Runner {
    pub fn new(playbook: Playbook, side: Side) -> Self {
        Self {
            playbook,
            side,
            stage: 0,
            mark: None,
            log: Vec::new(),
        }
    }

    /// The stage it is on, by place from 0; `None` once every stage has had its turn.
    pub fn stage(&self) -> Option<usize> {
        (self.stage < self.playbook.stages.len()).then_some(self.stage)
    }

    /// This turn's plan, recorded in `log`.
    pub fn plan(&mut self, m: &Match) -> TeamPlan {
        let compiled = match self.playbook.timing {
            Timing::Successive => self.successive(m),
            Timing::Bundled => bundled(&self.playbook, m, self.side),
        };
        let plan = compiled.plan.clone();
        self.log.push(compiled);
        plan
    }

    fn successive(&mut self, m: &Match) -> Compiled {
        let side = self.side;
        let turn = m.board.turn + 1;
        let stages = &self.playbook.stages;
        let mut advanced = Vec::new();
        // Each stage plays at least the turn it began; from then on it is checked for its goal and
        // its turns before each turn, and a stage taken up plays that turn whatever the board says.
        if let Some(stage) = stages.get(self.stage) {
            let mark = *self.mark.get_or_insert_with(|| Mark::of(m, side, turn));
            let index = self.stage as u8;
            let moved = if turn == mark.turn {
                false
            } else if met(stage.intent, m, side, &mark) {
                advanced.push(Advance::Met { stage: index });
                self.stage += 1;
                true
            } else if turn - mark.turn >= stage.turns {
                advanced.push(Advance::RanOut {
                    stage: index,
                    fallback: stage.fallback,
                });
                match stage.fallback {
                    Fallback::Next => self.stage += 1,
                    Fallback::Again => {}
                    Fallback::First => self.stage = 0,
                }
                true
            } else {
                false
            };
            if moved {
                self.mark = Some(Mark::of(m, side, turn));
            }
        }
        let (stage, index, finished) = match stages.get(self.stage) {
            Some(s) => (*s, Some(self.stage as u8), false),
            None => (CLOSING, None, true),
        };
        let mut built = Build::new(m, side, stage.posture.kept(), stage.facing);
        let hands = built.free_hands();
        let deviation = built.add(stage, hands);
        built.anchor();
        let (plan, reworded) = built.finish();
        Compiled {
            turn,
            side,
            timing: Timing::Successive,
            ran: vec![Ran {
                stage: index,
                intent: stage.intent,
                deviation: if finished {
                    Some(Deviation::Finished)
                } else {
                    deviation
                },
            }],
            advanced,
            reworded,
            plan,
        }
    }
}

/// What a team plays once every stage of a playbook has had its turn.
const CLOSING: Stage = Stage::new(Intent::Secure);

/// Every stage at once: the first stage's posture keeps hands on the beacon, and the rest are
/// shared among the stages in order, the earlier ones first.
fn bundled(playbook: &Playbook, m: &Match, side: Side) -> Compiled {
    let turn = m.board.turn + 1;
    let first = playbook.stages.first().copied().unwrap_or(CLOSING);
    let mut built = Build::new(m, side, first.posture.kept(), first.facing);
    let hands = built.free_hands();
    let n = playbook.stages.len().max(1);
    let mut ran = Vec::new();
    for (i, stage) in playbook.stages.iter().enumerate() {
        let share = hands / n + usize::from(i < hands % n);
        if share == 0 {
            continue;
        }
        let mut deviation = built.add(*stage, share);
        if i > 0 && deviation == Some(Deviation::Shorthanded) {
            deviation = None;
        }
        ran.push(Ran {
            stage: Some(i as u8),
            intent: stage.intent,
            deviation,
        });
    }
    built.anchor();
    let (plan, reworded) = built.finish();
    Compiled {
        turn,
        side,
        timing: Timing::Bundled,
        ran,
        advanced: Vec::new(),
        reworded,
        plan,
    }
}

/// One part of a plan, and the ways it can be worded: the first is the plainest.
struct Part {
    intent: Intent,
    wordings: Vec<Vec<Directive>>,
}

/// A plan being put together from stages.
struct Build<'a> {
    m: &'a Match,
    side: Side,
    facing: FacingHint,
    standing: usize,
    kept: usize,
    used: usize,
    /// Orders given so far.
    orders: usize,
    parts: Vec<Part>,
    /// Squares already named by a part, so the next looks elsewhere.
    named: Vec<Tile>,
    /// The beacon's order, for the hands kept on it.
    beacon: Option<Directive>,
    /// Hands given to the beacon because a stage had nothing to aim at.
    spare: usize,
}

impl<'a> Build<'a> {
    fn new(m: &'a Match, side: Side, kept: usize, facing: FacingHint) -> Self {
        let standing = m.team(side).count();
        let mut built = Self {
            m,
            side,
            facing,
            standing,
            kept: kept.min(standing),
            used: 0,
            orders: 0,
            parts: Vec::new(),
            named: Vec::new(),
            beacon: None,
            spare: 0,
        };
        built.beacon = built.beacon_order();
        if built.kept > 0
            && let Some(d) = &built.beacon
        {
            built.named.extend(d.squares().iter().copied());
        }
        built
    }

    /// Hands left for intents once the posture's are kept on the beacon; at least one while
    /// anyone is standing.
    fn free_hands(&mut self) -> usize {
        if self.standing > 0 && self.kept >= self.standing {
            self.kept = self.standing - 1;
        }
        self.standing - self.kept - self.used
    }

    /// Orders still free for intents, one kept back for the beacon if anyone is kept on it.
    fn room(&self) -> usize {
        PLAN_SIZE.saturating_sub(self.orders + usize::from(self.kept > 0))
    }

    fn face(&self, at: Tile) -> Facing {
        match self.facing {
            FacingHint::Them => ai::face(self.m, self.side, at),
            FacingHint::Across => Facing::across(self.side),
        }
    }

    /// Orders for `stage`'s intent, with `hands` of the team. Says why it played differently from
    /// what was asked, when it did.
    fn add(&mut self, stage: Stage, hands: usize) -> Option<Deviation> {
        let shorthanded = self.kept < stage.posture.kept().min(self.standing);
        let room = self.room();
        if hands == 0 || room == 0 {
            return shorthanded.then_some(Deviation::Shorthanded);
        }
        let wordings: Vec<Vec<Directive>> = self
            .wordings(stage, hands)
            .unwrap_or_default()
            .into_iter()
            .map(|mut w| {
                w.truncate(room);
                w
            })
            .filter(|w| !w.is_empty())
            .collect();
        self.used += hands;
        if wordings.is_empty() {
            self.spare += hands;
            return Some(Deviation::NoTarget);
        }
        for d in &wordings[0] {
            self.named.extend(d.squares().iter().copied());
        }
        self.orders += wordings.iter().map(Vec::len).max().unwrap_or(0);
        self.parts.push(Part {
            intent: stage.intent,
            wordings,
        });
        shorthanded.then_some(Deviation::Shorthanded)
    }

    /// The ways to word an intent for `hands` of the team, plainest first; `None` when there is
    /// nothing to aim it at.
    fn wordings(&self, stage: Stage, hands: usize) -> Option<Vec<Vec<Directive>>> {
        let (m, side) = (self.m, self.side);
        let aim = |t: Tile| aim_key(m, side, stage.aim, t);
        let ours_on = |t: Tile| m.occupant(t).is_some_and(|o| o.side == side);
        match stage.intent {
            Intent::Secure | Intent::DrawOut => {
                let mut squares: Vec<Tile> = m
                    .arena
                    .zone
                    .iter()
                    .copied()
                    .filter(|t| m.walkable(*t) || ours_on(*t))
                    .filter(|t| !self.named.contains(t))
                    .filter(|t| m.occupant(*t).is_none_or(|o| o.side == side))
                    .collect();
                // Squares already ours first, then the rest the way the stage aims.
                squares.sort_by_key(|t| (!ours_on(*t), aim(*t)));
                squares.truncate(hands.max(1));
                if squares.is_empty() {
                    return None;
                }
                let facing = self.face(squares[0]);
                Some(if stage.intent == Intent::Secure {
                    vec![
                        squares
                            .iter()
                            .map(|t| Directive::Press { squares: vec![*t] })
                            .collect(),
                        vec![Directive::Press {
                            squares: squares.clone(),
                        }],
                        vec![Directive::Hold {
                            squares: squares.clone(),
                            facing,
                        }],
                    ]
                } else {
                    let mut wordings = vec![
                        vec![Directive::Hold {
                            squares: squares.clone(),
                            facing,
                        }],
                        squares
                            .iter()
                            .map(|t| Directive::Hold {
                                squares: vec![*t],
                                facing: self.face(*t),
                            })
                            .collect(),
                    ];
                    if let Some(tap) = tap_on_them(m, side) {
                        let mut with_tap = vec![Directive::Interact { at: tap }];
                        if hands > 1 {
                            with_tap.push(Directive::Hold {
                                squares: squares.clone(),
                                facing,
                            });
                        }
                        wordings.push(with_tap);
                    }
                    wordings
                })
            }
            Intent::Pressure => {
                let centre = m.arena.zone_centre();
                let mut targets: Vec<Slot> = m
                    .team(side.other())
                    .filter(|e| {
                        let at = m.fighter(*e).at;
                        m.arena.in_zone(at) || at.distance(centre) <= PRESSURE_REACH
                    })
                    .filter(|e| !self.named.contains(&m.fighter(*e).at))
                    .collect();
                targets.sort_by_key(|e| {
                    let at = m.fighter(*e).at;
                    (!m.arena.in_zone(at), aim(at), *e)
                });
                if targets.is_empty() {
                    return None;
                }
                // Each target in turn as the one gone at first, the rest after it.
                let wordings = (0..targets.len().min(3))
                    .map(|first| {
                        let mut order: Vec<Slot> = vec![targets[first]];
                        order.extend(targets.iter().copied().filter(|e| *e != targets[first]));
                        let mut left = hands;
                        let mut plan = Vec::new();
                        for e in order {
                            if left == 0 {
                                break;
                            }
                            let at = m.fighter(e).at;
                            // A holder takes two, flanking it; alone, a press at it is turned away.
                            let holder = muster::holds(m, e);
                            if holder && (left < 2 || ai::flank_places(m, e, side).is_none()) {
                                continue;
                            }
                            left -= if holder { 2 } else { 1 };
                            plan.push(Directive::Press { squares: vec![at] });
                        }
                        plan
                    })
                    .filter(|p| !p.is_empty())
                    .collect::<Vec<_>>();
                (!wordings.is_empty()).then_some(wordings)
            }
            Intent::Counter => {
                let mut holders: Vec<Slot> = m
                    .team(side.other())
                    .filter(|e| muster::holds(m, *e))
                    .filter(|e| !self.named.contains(&m.fighter(*e).at))
                    .collect();
                holders.sort_by_key(|e| {
                    let at = m.fighter(*e).at;
                    (!m.arena.in_zone(at), aim(at), *e)
                });
                holders.truncate(hands);
                if holders.is_empty() {
                    return None;
                }
                let squares: Vec<Tile> = holders.iter().map(|e| m.fighter(*e).at).collect();
                Some(vec![
                    squares
                        .iter()
                        .map(|t| Directive::Trick { squares: vec![*t] })
                        .collect(),
                    vec![Directive::Trick { squares }],
                ])
            }
            Intent::Fetch => {
                let empty: Vec<Slot> = m
                    .team(side)
                    .filter(|s| m.fighter(*s).carrying.is_none())
                    .collect();
                let walk: Vec<(Slot, Vec<Option<u8>>)> =
                    empty.iter().map(|s| (*s, ai::walks(m, *s))).collect();
                let w = m.arena.width as usize;
                let near = |t: Tile| {
                    walk.iter()
                        .filter_map(|(_, d)| d[t.y as usize * w + t.x as usize])
                        .min()
                };
                let mut things: Vec<(Tile, u8)> = m
                    .board
                    .ground
                    .iter()
                    .chain(m.board.glint.iter())
                    .map(|(t, _)| *t)
                    .filter(|t| !self.named.contains(t))
                    .filter_map(|t| near(t).filter(|d| *d <= FETCH_REACH).map(|d| (t, d)))
                    .collect();
                things.sort_by_key(|(t, d)| (*d, aim(*t)));
                things.dedup();
                things.truncate(hands.min(empty.len()));
                if things.is_empty() {
                    return None;
                }
                Some(vec![
                    things
                        .iter()
                        .map(|(t, _)| Directive::Interact { at: *t })
                        .collect(),
                ])
            }
            Intent::Work => {
                let mut works: Vec<Tile> = Vec::new();
                if let Some(tap) = tap_on_them(m, side) {
                    works.push(tap);
                } else if m.switch_ready() {
                    let mut taps = m.arena.switches.clone();
                    taps.sort_by_key(|t| aim(*t));
                    works.extend(taps.into_iter().take(1));
                }
                if m.lever_ready() && m.board.gates_open.iter().any(|open| !open) {
                    works.extend(m.arena.levers.first().copied());
                }
                works.retain(|t| !self.named.contains(t));
                works.truncate(hands);
                if works.is_empty() {
                    return None;
                }
                let mut wordings = vec![
                    works
                        .iter()
                        .map(|t| Directive::Interact { at: *t })
                        .collect::<Vec<_>>(),
                ];
                if works.len() > 1 {
                    wordings.push(vec![Directive::Interact { at: works[0] }]);
                }
                Some(wordings)
            }
        }
    }

    /// The beacon's order: hold the square of it one of ours is on (facing as the stage says), or
    /// press onto the free one the team can walk to soonest.
    fn beacon_order(&self) -> Option<Directive> {
        let (m, side) = (self.m, self.side);
        let centre = m.arena.zone_centre();
        let team: Vec<Slot> = m.team(side).collect();
        if team.is_empty() {
            return None;
        }
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
        Some(match held {
            Some(s) => {
                let at = m.fighter(s).at;
                Directive::Hold {
                    squares: vec![at],
                    facing: self.face(at),
                }
            }
            None => {
                let tile = team
                    .iter()
                    .filter_map(|s| ai::nearest_free_zone_tile(m, *s, &[]).map(|t| (*s, t)))
                    .min_by_key(|(s, t)| (m.fighter(*s).at.distance(*t), t.distance(centre), *s))
                    .map_or(centre, |(_, t)| t);
                Directive::Press {
                    squares: vec![tile],
                }
            }
        })
    }

    /// The beacon's orders for the hands kept on it and any a stage with nothing to aim at gave
    /// back, named twice for a second hand so it stands guard beside the first.
    fn anchor(&mut self) {
        let hands = self.kept + self.spare;
        let room = PLAN_SIZE.saturating_sub(self.orders);
        let Some(beacon) = self.beacon.clone() else {
            return;
        };
        if hands == 0 || room == 0 {
            return;
        }
        let times = hands.min(2).min(room);
        self.orders += times;
        self.parts.push(Part {
            intent: Intent::Secure,
            wordings: vec![vec![beacon; times]],
        });
    }

    /// The plan: every part's wording, the one this trio carries out most as meant, and a USE ITEM
    /// for anyone carrying something with a plain use for it.
    fn finish(self) -> (TeamPlan, Option<Reworded>) {
        let (m, side) = (self.m, self.side);
        // Every way of putting the parts' wordings together, the plainest first, a few at most.
        let mut choices: Vec<Vec<usize>> = vec![vec![]];
        for part in &self.parts {
            let mut next = Vec::new();
            for c in &choices {
                for w in 0..part.wordings.len().min(3) {
                    let mut c = c.clone();
                    c.push(w);
                    next.push(c);
                }
            }
            choices = next;
            choices.truncate(12);
        }
        let assemble = |choice: &[usize]| {
            let mut plan: TeamPlan = Vec::new();
            let mut owners: Vec<Intent> = Vec::new();
            for (part, w) in self.parts.iter().zip(choice) {
                for d in &part.wordings[*w] {
                    if plan.len() < PLAN_SIZE {
                        plan.push(d.clone());
                        owners.push(part.intent);
                    }
                }
            }
            (plan, owners)
        };
        let turn = m.board.turn + 1;
        let on_intent = |plan: &TeamPlan, owners: &[Intent]| -> (i32, u8) {
            let (_, tasks, events) = muster::muster(m, side, plan, turn);
            let alone: Vec<Slot> = events
                .iter()
                .filter_map(|e| match &e.kind {
                    crate::event::EventKind::Assigned {
                        slot,
                        why: Why::Alone,
                        ..
                    } => Some(*slot),
                    _ => None,
                })
                .collect();
            let mut score = 0;
            let mut on = 0u8;
            for s in m.team(side) {
                let Some(task) = tasks[usize::from(s.index)] else {
                    continue;
                };
                let meant = task
                    .directive
                    .and_then(|i| owners.get(usize::from(i)))
                    .is_some_and(|intent| intent.roles().contains(&task.role));
                if meant {
                    score += 3;
                    on += 1;
                }
                if task.role == Role::Spare {
                    score -= 2;
                }
                if alone.contains(&s) {
                    score -= 1;
                }
            }
            (score, on)
        };
        let mut best: Option<(i32, usize, TeamPlan, u8)> = None;
        let mut plain_on = 0;
        for (k, choice) in choices.iter().enumerate() {
            let (plan, owners) = assemble(choice);
            let (score, on) = on_intent(&plan, &owners);
            if k == 0 {
                plain_on = on;
            }
            if best.as_ref().is_none_or(|(b, ..)| score > *b) {
                best = Some((score, k, plan, on));
            }
        }
        let (_, k, mut plan, given) = best.unwrap_or((0, 0, Vec::new(), 0));
        let reworded = (k != 0).then_some(Reworded {
            plain: plain_on,
            given,
        });
        let squares: Vec<Tile> = plan.iter().flat_map(|d| d.squares().to_vec()).collect();
        plan.extend(ai::item_uses(m, side, &squares, |_| true));
        (plan, reworded)
    }
}

/// A key to sort tiles by the way a stage aims, ties going to the upper row.
fn aim_key(m: &Match, side: Side, aim: Aim, t: Tile) -> (i32, i32, i8) {
    let across = i32::from(m.arena.across(side, t));
    let centre = i32::from(t.distance(m.arena.zone_centre()));
    match aim {
        Aim::Centre => (centre, across, t.y),
        Aim::Near => (across, centre, t.y),
        Aim::Far => (-across, centre, t.y),
    }
}

/// A switch ready to turn whose map event would catch more of theirs than of ours.
fn tap_on_them(m: &Match, side: Side) -> Option<Tile> {
    if !m.switch_ready() {
        return None;
    }
    let caught = |sd: Side| {
        m.team(sd)
            .filter(|x| m.arena.in_scenery(m.fighter(*x).at))
            .count()
    };
    (caught(side.other()) > caught(side))
        .then(|| {
            m.arena
                .switches
                .iter()
                .copied()
                .min_by_key(|t| (m.arena.across(side, *t), t.y))
        })
        .flatten()
}

/// Who gives a team its plan in a comparison: one of the coaches, or a playbook.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pilot {
    Coach(Coach),
    Playbook(Playbook),
}

impl Pilot {
    pub fn label(&self) -> String {
        match self {
            Self::Coach(c) => format!("{c:?}"),
            Self::Playbook(p) => match p.timing {
                Timing::Successive => p.name.clone(),
                Timing::Bundled => format!("{} (bundled)", p.name),
            },
        }
    }

    /// Ready to give `side` its plan, turn after turn, through one match.
    pub fn take(&self, side: Side) -> Planner {
        match self {
            Self::Coach(c) => Planner::Coach(*c),
            Self::Playbook(p) => Planner::Playbook(Box::new(Runner::new(p.clone(), side))),
        }
    }
}

/// A pilot partway through a match.
#[derive(Clone, Debug)]
pub enum Planner {
    Coach(Coach),
    Playbook(Box<Runner>),
}

impl Planner {
    pub fn plan(&mut self, m: &Match, side: Side) -> TeamPlan {
        match self {
            Self::Coach(c) => c.plan(m, side),
            Self::Playbook(r) => r.plan(m),
        }
    }

    /// The playbook's record of the match so far, for a playbook.
    pub fn log(&self) -> &[Compiled] {
        match self {
            Self::Coach(_) => &[],
            Self::Playbook(r) => &r.log,
        }
    }
}

/// How one pilot did against another over many matches, from its own point of view whichever
/// side of the board it was on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FaceOff {
    pub record: Record,
    /// How its wins came: by the beacon, by knocking the other team out, at the turn cap.
    pub won_by_capture: u32,
    pub won_by_knockout: u32,
    pub won_at_cap: u32,
    pub turns: u32,
    /// For a playbook: turns compiled, and of them those where a stage had nothing to aim at,
    /// was short of hands, or was past the last stage; turns worded for the trio rather than
    /// plainly; and stages that got their goal or ran out of turns.
    pub compiled: u32,
    /// Stages played, summed over turns: one a turn one stage at a time, up to three bundled;
    /// turns past the last stage are not counted.
    pub stages_played: u32,
    pub no_target: u32,
    pub shorthanded: u32,
    pub finished: u32,
    pub reworded: u32,
    pub met: u32,
    pub ran_out: u32,
}

impl FaceOff {
    /// Of the stages a playbook played, turn by turn, the share that played as asked.
    pub fn as_asked(&self) -> f64 {
        if self.stages_played == 0 {
            return 1.0;
        }
        1.0 - f64::from(self.no_target + self.shorthanded) / f64::from(self.stages_played)
    }
}

/// `pilots[0]` against `pilots[1]`: `pairings` pairs of random trios drawn as the fairness
/// tournament draws them, each played twice with the sides swapped and the trios kept with their
/// pilot, so neither side of the board and neither trio is in it.
pub fn face_off(
    arena: &Arena,
    ruleset: Ruleset,
    seed: u64,
    pairings: u32,
    pilots: [&Pilot; 2],
) -> FaceOff {
    let mut result = FaceOff::default();
    for pairing in 0..u64::from(pairings) {
        let base = pairing * 6;
        let ours: Vec<_> = (0..3).map(|i| drawn_creature(seed, base + i)).collect();
        let theirs: Vec<_> = (3..6).map(|i| drawn_creature(seed, base + i)).collect();
        let match_seed = rng::roll(seed, Purpose::Roster, &[pairing, 999]);
        for our_side in Side::BOTH {
            let (rosters, by_side) = match our_side {
                Side::Left => ([ours.clone(), theirs.clone()], [pilots[0], pilots[1]]),
                Side::Right => ([theirs.clone(), ours.clone()], [pilots[1], pilots[0]]),
            };
            let mut planners = [by_side[0].take(Side::Left), by_side[1].take(Side::Right)];
            let (m, _) = tournament::play_by(arena, ruleset, match_seed, rosters, |m, side| {
                planners[side.index()].plan(m, side)
            });
            let outcome = m.outcome().expect("played to the end");
            result.turns += u32::from(outcome.turn);
            result.record.played += 1;
            match outcome.winner {
                Some(w) if w == our_side => {
                    result.record.won += 1;
                    match outcome.victory {
                        Some(Victory::Capture) => result.won_by_capture += 1,
                        Some(Victory::Knockout) => result.won_by_knockout += 1,
                        _ => result.won_at_cap += 1,
                    }
                }
                Some(_) => {}
                None => result.record.drawn += 1,
            }
            for c in planners[our_side.index()].log() {
                result.compiled += 1;
                for r in &c.ran {
                    result.stages_played += u32::from(r.stage.is_some());
                    match r.deviation {
                        Some(Deviation::NoTarget) => result.no_target += 1,
                        Some(Deviation::Shorthanded) => result.shorthanded += 1,
                        Some(Deviation::Finished) => result.finished += 1,
                        None => {}
                    }
                }
                result.reworded += u32::from(c.reworded.is_some());
                for a in &c.advanced {
                    match a {
                        Advance::Met { .. } => result.met += 1,
                        Advance::RanOut { .. } => result.ran_out += 1,
                    }
                }
            }
        }
    }
    result
}
