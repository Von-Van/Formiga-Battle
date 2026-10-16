//! A resolved turn played back over time, from the board as it stood and the turn's events.
//!
//! The rules hand back where everyone ended up and what happened on the way; this works out what
//! the board looks like at any moment in between, so the window can animate it and a review
//! picture can stop at any point. Each tick is shown in three beats, the same order the rules
//! settle it in: presses and tricks already in reach, then everyone's steps together, then
//! whatever those steps brought into reach, the beacon and the end.

use crate::look::Pose;
use formiga_battle_core::game::Board;
use formiga_battle_core::{Event, EventKind, Facing, Item, Order, Side, Slot, TEAM_SIZE, Tile};

/// How long one tick takes to show, in seconds.
pub const TICK_SECS: f32 = 0.9;
/// The pause before the first tick, while both teams' orders are shown.
pub const REVEAL_SECS: f32 = 0.8;
/// How long the result is held after the last tick before the turn counts as shown.
pub const HOLD_SECS: f32 = 0.6;

/// The three beats of a tick, as shares of it.
const STEPS_FROM: f32 = 0.3;
const STEPS_TO: f32 = 0.7;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Figure {
    /// Where its feet are, in tiles.
    pub at: (f32, f32),
    pub tile: Tile,
    pub facing_right: bool,
    /// The way it faces while it holds, fixed from when it starts holding to the end of the turn.
    pub facing: Facing,
    pub pose: Pose,
    /// Seconds into its pose, for its animation.
    pub posed_for: f32,
    pub composure: u8,
    pub out: bool,
    /// How much of it shows, from 0 to 1: a knocked-out creature fades as it flops.
    pub opacity: f32,
    pub holding: bool,
    /// Knocked off balance this turn: it holds nothing and counts for nothing in the yard until
    /// the turn is over.
    pub off_balance: bool,
    /// What it carries, if anything.
    pub carrying: Option<Item>,
    /// A leaf umbrella is open over it.
    pub sheltered: bool,
    /// The sprinkler soaked it this turn.
    pub soaked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Burst {
    /// A PRESS landing on someone not holding, with nowhere to push it: knocked off balance where
    /// it stands.
    Hit,
    /// A PRESS pushing someone not holding one tile on, toward `to`: dust kicked up where it was.
    Shove { to: Tile },
    /// A PRESS turned away by someone holding.
    Shield,
    /// A holder at `from` facing a presser head on, which costs the presser composure: a jab out
    /// of the holder's front into it.
    Counter { from: Tile },
    /// Two PRESSes meeting, the other at `partner`: a bonk between them.
    Collide { partner: Tile },
    /// A TRICK coming round from `from` to the side or back of a hold, and breaking it.
    Sneak { from: Tile },
    /// A trick seen coming: an alert over whoever saw through it.
    Read,
    /// What a pressed creature carried, snatched and flying across to the presser at `to`.
    Snatch { to: Tile, item: Item },
    /// Two wanting the same tile.
    Bump,
    /// A cartoon knockout.
    Stars,
    /// The lever pulled.
    Spark,
    /// Holders shoulder to shoulder turning a flank away together, drawn from one to the other.
    Braced { partner: Tile },
    /// A hold broken from two sides at once.
    Broken,
    /// A wary creature waiting a tick rather than walk in among the other team.
    Hesitate,
    /// The sprinkler's spray landing on a tile.
    Splash,
    /// A dandelion puff's gust landing on a tile.
    Gust,
    /// Something found turning up, or picked up.
    Twinkle,
    /// A pebble bell ringing.
    Ring,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effect {
    pub burst: Burst,
    pub tile: Tile,
    /// Seconds since it began.
    pub age: f32,
}

/// The board at one moment of the turn.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub figures: [[Figure; TEAM_SIZE]; 2],
    pub gates_open: Vec<bool>,
    pub capture: [u8; 2],
    /// A lever has been pulled and the gates have not swung yet.
    pub swing_pending: bool,
    pub effects: Vec<Effect>,
    /// The tick being shown, 0 before the first.
    pub tick: u8,
    /// Both teams have someone steady in the yard, so neither gains on the beacon.
    pub paused: bool,
    /// Things lying on the ground.
    pub ground: Vec<(Tile, Item)>,
    /// Spots glinting with something about to turn up.
    pub glint: Vec<Tile>,
    /// Twigs wedged upright.
    pub twigs: Vec<Tile>,
    /// The map event is warned and has not landed yet.
    pub sprinkler_coming: bool,
    /// A tap can be turned.
    pub tap_ready: bool,
    /// Tiles a puff is about to blow across.
    pub gusty: Vec<Tile>,
    /// Where the map event will land, while it is coming.
    pub event_tiles: Vec<Tile>,
    /// Ground gone under.
    pub closed: Vec<Tile>,
    /// Twigs floating as rafts.
    pub rafts: Vec<Tile>,
    /// Sticky ground.
    pub honey: Vec<Tile>,
}

/// How long effects stay on the board.
const EFFECT_SECS: f32 = 0.7;
/// How long something snatched takes to fly across to its new carrier.
pub const SNATCH_SECS: f32 = 0.45;

/// The way a HOLD faces, if the order is one.
fn hold_facing(order: &Order) -> Option<Facing> {
    match order {
        Order::Hold { facing, .. } => Some(*facing),
        _ => None,
    }
}

/// Whether a creature facing this way is drawn looking right across the screen: east and north
/// are to the right, as the board is drawn.
fn looks_right(facing: Facing) -> bool {
    matches!(facing, Facing::East | Facing::North)
}

/// A slide one tile on, quickly, from `from` to `to`, `age` seconds in.
fn slid(f: &mut Figure, from: Tile, to: Tile, age: f32) {
    let share = (age / 0.35).clamp(0.0, 1.0);
    let (fx, fy) = (f32::from(from.x) + 0.5, f32::from(from.y) + 0.5);
    let (tx, ty) = (f32::from(to.x) + 0.5, f32::from(to.y) + 0.5);
    f.at = (fx + (tx - fx) * share, fy + (ty - fy) * share);
    f.tile = to;
}

impl Frame {
    /// The board as it stands, with nothing happening.
    pub fn still(board: &Board, facing: &[[bool; TEAM_SIZE]; 2]) -> Self {
        let figures = [0, 1].map(|s| {
            [0, 1, 2].map(|i| {
                let f = &board.fighters[s][i];
                Figure {
                    at: (f32::from(f.at.x) + 0.5, f32::from(f.at.y) + 0.5),
                    tile: f.at,
                    facing_right: facing[s][i],
                    facing: f.facing,
                    pose: if f.holding { Pose::Hold } else { Pose::Idle },
                    posed_for: 0.0,
                    composure: f.composure,
                    out: f.out,
                    opacity: if f.out { 0.0 } else { 1.0 },
                    holding: f.holding,
                    off_balance: false,
                    carrying: f.carrying,
                    sheltered: f.shelter_until > 0,
                    soaked: false,
                }
            })
        });
        Self {
            figures,
            gates_open: board.gates_open.clone(),
            capture: board.capture,
            swing_pending: board.swing_at.is_some(),
            effects: Vec::new(),
            tick: 0,
            paused: false,
            ground: board.ground.clone(),
            glint: board.glint.iter().map(|(t, _)| *t).collect(),
            twigs: board.twigs.iter().map(|(t, _)| *t).collect(),
            sprinkler_coming: board.event_at.is_some(),
            tap_ready: board.event_at.is_none()
                && board.turn.saturating_add(1) >= board.switch_ready_turn,
            event_tiles: board.event_tiles.clone(),
            closed: board.closed.iter().map(|(t, _)| *t).collect(),
            rafts: board.rafts.iter().map(|(t, _)| *t).collect(),
            honey: board.honey.iter().map(|(t, _)| *t).collect(),
            gusty: board
                .pending
                .iter()
                .filter(|p| p.item == Item::Puff)
                .flat_map(|p| formiga_battle_core::items::puff_tiles(p.from, p.target))
                .collect(),
        }
    }

    pub fn figure(&self, slot: Slot) -> &Figure {
        &self.figures[slot.side.index()][slot.index as usize]
    }

    fn figure_mut(&mut self, slot: Slot) -> &mut Figure {
        &mut self.figures[slot.side.index()][slot.index as usize]
    }
}

/// One turn, ready to be shown.
pub struct Playback {
    start: Frame,
    /// Each creature's order as the turn began, for which way a holder faces.
    orders: [[Option<Order>; TEAM_SIZE]; 2],
    /// The arena's gates, in its own order.
    gates: Vec<Tile>,
    /// Each event and when it happens, in seconds from the start.
    timed: Vec<(f32, Event)>,
    ticks: u8,
    winners: Option<Side>,
}

impl Playback {
    pub fn new(
        before: &Board,
        gates: &[Tile],
        facing: &[[bool; TEAM_SIZE]; 2],
        events: &[Event],
        ticks: u8,
    ) -> Self {
        let mut start = Frame::still(before, facing);
        let mut orders = [[None; TEAM_SIZE]; 2];
        // Whoever holds where it stands holds from the moment orders lock, facing the way its
        // HOLD says.
        for event in events.iter().filter(|e| e.tick == 0) {
            match &event.kind {
                EventKind::Assigned { slot, order, .. }
                | EventKind::OrderFilled { slot, order, .. } => {
                    orders[slot.side.index()][slot.index as usize] = Some(*order);
                }
                EventKind::Holding { slot, .. } => {
                    let held = orders[slot.side.index()][slot.index as usize]
                        .as_ref()
                        .and_then(hold_facing);
                    let f = start.figure_mut(*slot);
                    f.holding = true;
                    f.pose = Pose::Hold;
                    if let Some(facing) = held {
                        f.facing = facing;
                        f.facing_right = looks_right(facing);
                    }
                }
                _ => {}
            }
        }
        let played = events
            .iter()
            .filter_map(|e| match e.kind {
                EventKind::Finished { .. } => Some(e.tick),
                _ => None,
            })
            .next()
            .unwrap_or(ticks)
            .max(1);
        let mut timed = Vec::new();
        for tick in 1..=played {
            let this: Vec<&Event> = events.iter().filter(|e| e.tick == tick).collect();
            let stepping =
                |e: &&Event| matches!(e.kind, EventKind::Stepped { .. } | EventKind::Bumped { .. });
            let first_step = this.iter().position(stepping);
            let last_step = this.iter().rposition(stepping);
            let begin = REVEAL_SECS + f32::from(tick - 1) * TICK_SECS;
            let n = this.len().max(1) as f32;
            for (i, event) in this.iter().enumerate() {
                let share = match (first_step, last_step) {
                    (Some(first), _) if i < first => {
                        STEPS_FROM * (i as f32 + 0.5) / (first as f32 + 1.0)
                    }
                    (Some(_), Some(last)) if i <= last => STEPS_FROM,
                    (_, Some(last)) => {
                        let after = (this.len() - last - 1).max(1) as f32;
                        STEPS_TO + (1.0 - STEPS_TO) * ((i - last) as f32 - 0.5) / after
                    }
                    _ => (i as f32 + 0.5) / n,
                };
                timed.push((begin + share * TICK_SECS, (*event).clone()));
            }
        }
        let winners = events.iter().find_map(|e| match e.kind {
            EventKind::Finished { outcome } => outcome.winner,
            _ => None,
        });
        Self {
            start,
            orders,
            gates: gates.to_vec(),
            timed,
            ticks: played,
            winners,
        }
    }

    /// Each event and when it happens, in seconds from the start: what the announcer reads.
    pub fn moments(&self) -> &[(f32, Event)] {
        &self.timed
    }

    /// How long the whole turn takes to show.
    pub fn duration(&self) -> f32 {
        REVEAL_SECS + f32::from(self.ticks) * TICK_SECS + HOLD_SECS
    }

    /// The board `t` seconds into the turn.
    pub fn at(&self, t: f32) -> Frame {
        let mut frame = self.start.clone();
        let mut orders = self.orders;
        // Flankers in the yard who take the ground, and the tile they face: where the holder they
        // broke stood.
        let mut took: Vec<(Slot, Tile)> = Vec::new();
        let mut since: [[f32; TEAM_SIZE]; 2] = [[0.0; TEAM_SIZE]; 2];
        let step_secs = (STEPS_TO - STEPS_FROM) * TICK_SECS;
        let set_pose =
            |frame: &mut Frame, since: &mut [[f32; TEAM_SIZE]; 2], slot: Slot, pose, at| {
                frame.figure_mut(slot).pose = pose;
                since[slot.side.index()][slot.index as usize] = at;
            };
        for (when, event) in &self.timed {
            if *when > t {
                break;
            }
            frame.tick = event.tick;
            let age = t - when;
            let effect = |frame: &mut Frame, burst, tile| {
                if age < EFFECT_SECS {
                    frame.effects.push(Effect { burst, tile, age });
                }
            };
            match &event.kind {
                EventKind::Stepped { slot, from, to, .. } => {
                    let share = (age / step_secs).clamp(0.0, 1.0);
                    let ease = share * share * (3.0 - 2.0 * share);
                    let f = frame.figure_mut(*slot);
                    let (fx, fy) = (f32::from(from.x) + 0.5, f32::from(from.y) + 0.5);
                    let (tx, ty) = (f32::from(to.x) + 0.5, f32::from(to.y) + 0.5);
                    f.at = (fx + (tx - fx) * ease, fy + (ty - fy) * ease);
                    f.tile = *to;
                    // Facing follows which way it goes across the screen.
                    let across = (tx - ty) - (fx - fy);
                    if across != 0.0 {
                        f.facing_right = across > 0.0;
                    }
                    let pose = if share < 1.0 {
                        Pose::Walk
                    } else if f.holding {
                        Pose::Hold
                    } else {
                        Pose::Idle
                    };
                    let started = if share < 1.0 {
                        *when
                    } else {
                        *when + step_secs
                    };
                    set_pose(&mut frame, &mut since, *slot, pose, started);
                }
                EventKind::Bumped { slot, wanted } => {
                    effect(&mut frame, Burst::Bump, *wanted);
                    if age < 0.5 {
                        set_pose(&mut frame, &mut since, *slot, Pose::Worry, *when);
                    }
                }
                EventKind::HungBack { slot, at } => {
                    effect(&mut frame, Burst::Hesitate, *at);
                    if age < 0.6 {
                        set_pose(&mut frame, &mut since, *slot, Pose::Worry, *when);
                    }
                }
                EventKind::Holding { slot, at } => {
                    let facing = match took.iter().find(|(s, _)| s == slot) {
                        Some((_, there)) => Facing::toward(*at, *there),
                        None => orders[slot.side.index()][slot.index as usize]
                            .as_ref()
                            .and_then(hold_facing),
                    };
                    let f = frame.figure_mut(*slot);
                    f.holding = true;
                    if let Some(facing) = facing {
                        f.facing = facing;
                        f.facing_right = looks_right(facing);
                    }
                    set_pose(&mut frame, &mut since, *slot, Pose::Hold, *when);
                }
                EventKind::Assigned { slot, order, .. }
                | EventKind::OrderFilled { slot, order, .. }
                | EventKind::Rethought { slot, order, .. } => {
                    orders[slot.side.index()][slot.index as usize] = Some(*order);
                }
                EventKind::Pressed {
                    target,
                    by,
                    pushed_to,
                    tricking,
                } => {
                    let short = age < 0.6;
                    let from = frame.figure(*target).tile;
                    match pushed_to {
                        // Shoved one tile on, quickly, dust kicked up where it stood.
                        Some(to) => effect(&mut frame, Burst::Shove { to: *to }, from),
                        // Nowhere to go: knocked off balance where it stands.
                        None => effect(&mut frame, Burst::Hit, from),
                    }
                    // Caught in the middle of a TRICK: whoever pressed it saw it coming.
                    if *tricking && let Some(first) = by.first() {
                        let tile = frame.figure(*first).tile;
                        effect(&mut frame, Burst::Read, tile);
                    }
                    let f = frame.figure_mut(*target);
                    f.holding = false;
                    f.off_balance = true;
                    if let Some(to) = pushed_to {
                        slid(f, from, *to, age);
                    }
                    let pose = match (short, pushed_to) {
                        (true, Some(_)) => Pose::Wobble,
                        (true, None) => Pose::Gasp,
                        (false, _) => Pose::Idle,
                    };
                    set_pose(&mut frame, &mut since, *target, pose, *when);
                    for c in by {
                        let back = if frame.figure(*c).holding {
                            Pose::Hold
                        } else {
                            Pose::Idle
                        };
                        set_pose(
                            &mut frame,
                            &mut since,
                            *c,
                            if short { Pose::Reach } else { back },
                            *when,
                        );
                    }
                }
                EventKind::Stole { by, from, item } => {
                    // It leaves the one pressed at once and flies across; the presser has it
                    // once it lands.
                    let (a, b) = (frame.figure(*from).tile, frame.figure(*by).tile);
                    frame.figure_mut(*from).carrying = None;
                    if age < SNATCH_SECS {
                        frame.effects.push(Effect {
                            burst: Burst::Snatch { to: b, item: *item },
                            tile: a,
                            age,
                        });
                    } else {
                        frame.figure_mut(*by).carrying = Some(*item);
                    }
                }
                EventKind::Collided { target, by } => {
                    // Press met press: both stop where they are, off balance for the turn.
                    let short = age < 0.6;
                    let tile = frame.figure(*target).tile;
                    if let Some(first) = by.first() {
                        let partner = frame.figure(*first).tile;
                        effect(&mut frame, Burst::Collide { partner }, tile);
                    }
                    for c in by.iter().chain(std::iter::once(target)) {
                        frame.figure_mut(*c).off_balance = true;
                        set_pose(
                            &mut frame,
                            &mut since,
                            *c,
                            if short { Pose::Gasp } else { Pose::Idle },
                            *when,
                        );
                    }
                }
                EventKind::Repelled {
                    target,
                    holders,
                    pressers,
                    faced,
                } => {
                    let tile = frame.figure(*target).tile;
                    effect(&mut frame, Burst::Shield, tile);
                    // Shoulder to shoulder: neighbouring holders turned it away together.
                    if let [first, second, ..] = holders[..] {
                        let (from, partner) = (frame.figure(first).tile, frame.figure(second).tile);
                        effect(&mut frame, Burst::Braced { partner }, from);
                    }
                    // A presser in front of a holder facing it: a jab out of the holder's front.
                    for p in faced {
                        let at = frame.figure(*p).tile;
                        let from = holders
                            .iter()
                            .map(|h| frame.figure(*h))
                            .find(|h| h.facing.ahead(h.tile) == at)
                            .map_or(tile, |h| h.tile);
                        effect(&mut frame, Burst::Counter { from }, at);
                    }
                    let short = age < 0.6;
                    set_pose(&mut frame, &mut since, *target, Pose::Hold, *when);
                    for c in pressers {
                        let pose = match (short, faced.contains(c)) {
                            (true, true) => Pose::Gasp,
                            (true, false) => Pose::Reach,
                            (false, _) => Pose::Idle,
                        };
                        set_pose(&mut frame, &mut since, *c, pose, *when);
                    }
                }
                EventKind::Flanked {
                    target,
                    by,
                    from,
                    pushed_to,
                } => {
                    let short = age < 0.6;
                    // Its hold breaks: the shield flies apart where it stood, and it is off
                    // balance for the rest of the turn.
                    effect(&mut frame, Burst::Broken, *from);
                    took.extend(by.iter().map(|c| (*c, *from)));
                    let f = frame.figure_mut(*target);
                    f.holding = false;
                    f.off_balance = true;
                    if let Some(to) = pushed_to {
                        // Shoved one tile, quickly.
                        slid(f, *from, *to, age);
                    }
                    set_pose(
                        &mut frame,
                        &mut since,
                        *target,
                        if short { Pose::Wobble } else { Pose::Idle },
                        *when,
                    );
                    for c in by {
                        set_pose(
                            &mut frame,
                            &mut since,
                            *c,
                            if short { Pose::Reach } else { Pose::Idle },
                            *when,
                        );
                    }
                }
                EventKind::Tricked { target, by, from } => {
                    // Round its side or back: its hold breaks where it stands. It stays on its
                    // feet, and in the yard, but holds nothing more this turn: its ring and front
                    // go, and it looks round, caught out.
                    let short = age < 0.6;
                    effect(&mut frame, Burst::Broken, *from);
                    if let Some(first) = by.first() {
                        let came = frame.figure(*first).tile;
                        effect(&mut frame, Burst::Sneak { from: came }, *from);
                    }
                    frame.figure_mut(*target).holding = false;
                    set_pose(
                        &mut frame,
                        &mut since,
                        *target,
                        if short { Pose::Worry } else { Pose::Idle },
                        *when,
                    );
                    for c in by {
                        set_pose(
                            &mut frame,
                            &mut since,
                            *c,
                            if short { Pose::Reach } else { Pose::Idle },
                            *when,
                        );
                    }
                }
                EventKind::TrickRead { target, by } => {
                    // Tried from the front of a holder (the one aimed at, or one beside it): it
                    // saw it coming and keeps holding.
                    let tile = frame.figure(*target).tile;
                    effect(&mut frame, Burst::Read, tile);
                    if age < 0.6 {
                        set_pose(&mut frame, &mut since, *by, Pose::Worry, *when);
                    }
                    set_pose(&mut frame, &mut since, *target, Pose::Hold, *when);
                }
                EventKind::LeverPulled { slot, lever, .. } => {
                    frame.swing_pending = true;
                    effect(&mut frame, Burst::Spark, *lever);
                    if age < 0.6 {
                        set_pose(&mut frame, &mut since, *slot, Pose::Reach, *when);
                    }
                }
                EventKind::LeverDefended { slot, lever, .. } => {
                    effect(&mut frame, Burst::Shield, *lever);
                    if age < 0.6 {
                        set_pose(&mut frame, &mut since, *slot, Pose::Worry, *when);
                    }
                }
                EventKind::GateSwung { gate, open } => {
                    frame.swing_pending = false;
                    effect(&mut frame, Burst::Spark, *gate);
                    if let Some(i) = self.gates.iter().position(|g| g == gate) {
                        frame.gates_open[i] = *open;
                    }
                }
                EventKind::GateJammed { gate, .. } | EventKind::GateWedged { gate } => {
                    frame.swing_pending = false;
                    effect(&mut frame, Burst::Bump, *gate);
                }
                EventKind::Composure { slot, left, .. } => {
                    frame.figure_mut(*slot).composure = *left;
                }
                EventKind::KnockedOut { slot } => {
                    let tile = frame.figure(*slot).tile;
                    effect(&mut frame, Burst::Stars, tile);
                    let f = frame.figure_mut(*slot);
                    f.out = true;
                    f.holding = false;
                    f.opacity = (1.0 - age / 1.2).clamp(0.0, 1.0);
                    set_pose(&mut frame, &mut since, *slot, Pose::Dizzy, *when);
                }
                EventKind::CaptureProgress { side, progress } => {
                    frame.capture[side.index()] = *progress;
                    frame.paused = false;
                }
                EventKind::CaptureContested => frame.paused = true,
                EventKind::BrokeThrough { .. } => frame.paused = false,
                EventKind::Finished { .. } => {
                    if let Some(side) = self.winners {
                        for i in 0..TEAM_SIZE as u8 {
                            let slot = Slot::new(side, i);
                            if !frame.figure(slot).out {
                                set_pose(&mut frame, &mut since, slot, Pose::Cheer, *when);
                            }
                        }
                    }
                }
                // Where the map event and found things move or topple anyone; how they look and
                // sound is the match screen's to draw.
                EventKind::SceneryLanded {
                    hit,
                    tiles,
                    effect: what,
                    moved,
                    ..
                } => {
                    frame.sprinkler_coming = false;
                    frame.event_tiles.clear();
                    for tile in tiles {
                        effect(&mut frame, Burst::Splash, *tile);
                    }
                    if *what == formiga_battle_core::Effect::Close {
                        frame.ground.retain(|(t, _)| !tiles.contains(t));
                        frame.closed.extend(tiles.iter().copied());
                    }
                    for b in moved {
                        if let Some(to) = b.to {
                            let f = frame.figure_mut(b.slot);
                            let share = (age / 0.35).clamp(0.0, 1.0);
                            let (fx, fy) = (f32::from(b.from.x) + 0.5, f32::from(b.from.y) + 0.5);
                            let (tx, ty) = (f32::from(to.x) + 0.5, f32::from(to.y) + 0.5);
                            f.at = (fx + (tx - fx) * share, fy + (ty - fy) * share);
                            f.tile = to;
                        }
                    }
                    for slot in hit {
                        let f = frame.figure_mut(*slot);
                        f.holding = false;
                        f.off_balance = true;
                        f.soaked = true;
                        set_pose(&mut frame, &mut since, *slot, Pose::Wobble, *when);
                    }
                }
                EventKind::Puffed { blown, twigs, .. } => {
                    frame.gusty.clear();
                    frame.twigs.retain(|t| !twigs.contains(t));
                    for b in blown {
                        effect(&mut frame, Burst::Gust, b.from);
                        let f = frame.figure_mut(b.slot);
                        f.holding = false;
                        f.off_balance = true;
                        if let Some(to) = b.to {
                            let share = (age / 0.35).clamp(0.0, 1.0);
                            let (fx, fy) = (f32::from(b.from.x) + 0.5, f32::from(b.from.y) + 0.5);
                            let (tx, ty) = (f32::from(to.x) + 0.5, f32::from(to.y) + 0.5);
                            f.at = (fx + (tx - fx) * share, fy + (ty - fy) * share);
                            f.tile = to;
                        }
                        set_pose(&mut frame, &mut since, b.slot, Pose::Wobble, *when);
                    }
                }
                EventKind::SceneryWarned { tiles, .. } => {
                    frame.sprinkler_coming = true;
                    frame.event_tiles = tiles.clone();
                }
                EventKind::GroundReopened { tiles } => frame.closed.retain(|t| !tiles.contains(t)),
                EventKind::RaftFloated { at, .. } => frame.rafts.push(*at),
                EventKind::RaftSank { at } => frame.rafts.retain(|t| t != at),
                EventKind::Rung { at, .. } => effect(&mut frame, Burst::Ring, *at),
                EventKind::HoneySpilled { tiles, .. } => {
                    frame.gusty.retain(|t| !tiles.contains(t));
                    frame.honey.extend(tiles.iter().copied());
                }
                EventKind::HoneyDried { tiles } => frame.honey.retain(|t| !tiles.contains(t)),
                EventKind::Stuck { slot, .. } => {
                    set_pose(&mut frame, &mut since, *slot, Pose::Wobble, *when);
                }
                EventKind::Hopped { slot, from, to } => {
                    // Up and over, quickly.
                    let share = (age / 0.4).clamp(0.0, 1.0);
                    let f = frame.figure_mut(*slot);
                    let (fx, fy) = (f32::from(from.x) + 0.5, f32::from(from.y) + 0.5);
                    let (tx, ty) = (f32::from(to.x) + 0.5, f32::from(to.y) + 0.5);
                    f.at = (fx + (tx - fx) * share, fy + (ty - fy) * share);
                    f.tile = *to;
                    effect(&mut frame, Burst::Twinkle, *from);
                }
                EventKind::SwitchTurned { .. } => {
                    frame.tap_ready = false;
                    frame.sprinkler_coming = true;
                }
                EventKind::ItemsGlint { spots, .. } => frame.glint = spots.clone(),
                EventKind::ItemAppeared { at, item } => {
                    frame.glint.retain(|t| t != at);
                    frame.ground.push((*at, *item));
                    effect(&mut frame, Burst::Twinkle, *at);
                }
                EventKind::PickedUp { slot, item, at } => {
                    frame.ground.retain(|(t, _)| t != at);
                    frame.figure_mut(*slot).carrying = Some(*item);
                    effect(&mut frame, Burst::Twinkle, *at);
                }
                EventKind::Dropped { slot, item, at } => {
                    frame.figure_mut(*slot).carrying = None;
                    if let Some(at) = at {
                        frame.ground.push((*at, *item));
                    }
                }
                EventKind::ItemUsed {
                    slot, item, tiles, ..
                } => {
                    frame.figure_mut(*slot).carrying = None;
                    set_pose(&mut frame, &mut since, *slot, Pose::Reach, *when);
                    if matches!(item, Item::Puff | Item::Honey) {
                        frame.gusty = tiles.clone();
                    }
                }
                EventKind::TwigWedged { at, .. } => frame.twigs.push(*at),
                EventKind::TwigFell { at } => frame.twigs.retain(|t| t != at),
                EventKind::Sheltering { slots, .. } => {
                    for slot in slots {
                        frame.figure_mut(*slot).sheltered = true;
                    }
                }
                EventKind::ShelterFolded { slot } => frame.figure_mut(*slot).sheltered = false,
                EventKind::Sheltered { target, .. } => {
                    let tile = frame.figure(*target).tile;
                    effect(&mut frame, Burst::Shield, tile);
                }
                EventKind::OrderDropped { .. }
                | EventKind::TookOpportunity { .. }
                | EventKind::LeverResting { .. }
                | EventKind::SwitchDefended { .. }
                | EventKind::SwitchResting { .. }
                | EventKind::ItemFizzled { .. } => {}
            }
        }
        for (figures, began) in frame.figures.iter_mut().zip(since) {
            for (f, began) in figures.iter_mut().zip(began) {
                f.posed_for = (t - began).max(0.0);
            }
        }
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_battle_core::ai::Coach;
    use formiga_battle_core::tournament::drawn_creature;
    use formiga_battle_core::{Arena, Event, Match, Ruleset};

    fn turn() -> (Board, formiga_battle_core::TurnReport, Match) {
        let rosters = [
            (0..3).map(|i| drawn_creature(2, i)).collect(),
            (3..6).map(|i| drawn_creature(2, i)).collect(),
        ];
        let mut m = Match::new(Arena::backyard(), Ruleset::default(), 2, rosters).unwrap();
        let before = m.board.clone();
        let plans = [
            Coach::Exhibition.plan(&m, Side::Left),
            Coach::Exhibition.plan(&m, Side::Right),
        ];
        let report = m.resolve_turn(plans);
        (before, report, m)
    }

    #[test]
    fn a_turn_played_to_its_end_shows_the_board_the_rules_left() {
        let (before, report, m) = turn();
        let facing = [[true; TEAM_SIZE], [false; TEAM_SIZE]];
        let playback = Playback::new(
            &before,
            &m.arena.gates,
            &facing,
            &report.events,
            m.ruleset.ticks_per_turn,
        );
        let end = playback.at(playback.duration());
        for slot in Slot::all() {
            let f = end.figure(slot);
            assert_eq!(f.tile, m.fighter(slot).at, "{slot:?}");
            let (x, y) = f.at;
            assert_eq!(
                (x - 0.5, y - 0.5),
                (f32::from(f.tile.x), f32::from(f.tile.y))
            );
            assert_eq!(f.composure, m.fighter(slot).composure);
            assert_eq!(f.out, m.fighter(slot).out);
        }
        assert_eq!(end.capture, m.board.capture);
    }

    #[test]
    fn before_the_first_tick_everyone_is_where_the_turn_began() {
        let (before, report, m) = turn();
        let facing = [[true; TEAM_SIZE], [false; TEAM_SIZE]];
        let playback = Playback::new(
            &before,
            &m.arena.gates,
            &facing,
            &report.events,
            m.ruleset.ticks_per_turn,
        );
        let start = playback.at(0.0);
        for slot in Slot::all() {
            assert_eq!(
                start.figure(slot).tile,
                before.fighters[slot.side.index()][slot.index as usize].at
            );
        }
    }

    fn at(tick: u8, kind: EventKind) -> Event {
        Event {
            turn: 1,
            tick,
            kind,
        }
    }

    #[test]
    fn a_flanked_holder_stays_off_balance_until_the_turn_is_over() {
        let (before, _, m) = turn();
        let facing = [[true; TEAM_SIZE], [false; TEAM_SIZE]];
        let target = Slot::new(Side::Left, 0);
        let from = before.fighters[0][0].at;
        let events = vec![
            at(
                0,
                EventKind::Holding {
                    slot: target,
                    at: from,
                },
            ),
            at(
                1,
                EventKind::Flanked {
                    target,
                    by: vec![Slot::new(Side::Right, 0), Slot::new(Side::Right, 1)],
                    from,
                    pushed_to: None,
                },
            ),
        ];
        let playback = Playback::new(&before, &m.arena.gates, &facing, &events, 3);
        assert!(playback.at(0.1).figure(target).holding);
        let end = playback.at(playback.duration());
        assert!(end.figure(target).off_balance);
        assert!(!end.figure(target).holding);
    }

    #[test]
    fn the_beacon_pauses_while_both_hold_the_yard_and_resumes_when_someone_gains() {
        let (before, _, m) = turn();
        let facing = [[true; TEAM_SIZE], [false; TEAM_SIZE]];
        let events = vec![
            at(1, EventKind::CaptureContested),
            at(
                2,
                EventKind::CaptureProgress {
                    side: Side::Left,
                    progress: 1,
                },
            ),
        ];
        let playback = Playback::new(&before, &m.arena.gates, &facing, &events, 3);
        let tick = |n: f32| REVEAL_SECS + (n - 0.2) * TICK_SECS;
        assert!(!playback.at(0.0).paused);
        assert!(playback.at(tick(1.0)).paused);
        assert!(!playback.at(tick(2.0)).paused);
    }

    #[test]
    fn a_gate_wedged_by_a_twig_is_no_longer_waiting_to_swing() {
        let (mut before, _, m) = turn();
        before.swing_at = Some(4);
        let facing = [[true; TEAM_SIZE], [false; TEAM_SIZE]];
        let gate = m.arena.gates[0];
        let events = vec![at(1, EventKind::GateWedged { gate })];
        let playback = Playback::new(&before, &m.arena.gates, &facing, &events, 3);
        assert!(playback.at(0.0).swing_pending);
        assert!(!playback.at(playback.duration()).swing_pending);
    }

    #[test]
    fn halfway_through_a_step_a_creature_is_between_its_tiles() {
        let (before, report, m) = turn();
        let facing = [[true; TEAM_SIZE], [false; TEAM_SIZE]];
        let playback = Playback::new(
            &before,
            &m.arena.gates,
            &facing,
            &report.events,
            m.ruleset.ticks_per_turn,
        );
        let (when, slot, from, to) = playback
            .timed
            .iter()
            .find_map(|(when, e)| match e.kind {
                EventKind::Stepped { slot, from, to, .. } => Some((*when, slot, from, to)),
                _ => None,
            })
            .expect("someone steps on the first turn");
        let mid = playback.at(when + (STEPS_TO - STEPS_FROM) * TICK_SECS / 2.0);
        let f = mid.figure(slot);
        let along = |a: i8, b: i8, v: f32| {
            let (lo, hi) = (f32::from(a.min(b)) + 0.5, f32::from(a.max(b)) + 0.5);
            lo <= v && v <= hi
        };
        assert!(along(from.x, to.x, f.at.0) && along(from.y, to.y, f.at.1));
        assert_eq!(f.pose, Pose::Walk);
    }

    /// When the first event of this kind is shown, in seconds from the start.
    fn when(playback: &Playback, kind: fn(&EventKind) -> bool) -> f32 {
        playback
            .timed
            .iter()
            .find(|(_, e)| kind(&e.kind))
            .map(|(when, _)| *when)
            .expect("the event is shown")
    }

    fn hold(slot: Slot, at: Tile, facing: Facing) -> EventKind {
        EventKind::Assigned {
            slot,
            task: formiga_battle_core::Task {
                directive: Some(0),
                role: formiga_battle_core::Role::Claim,
                at,
            },
            order: Order::Hold { at, facing },
            why: formiga_battle_core::Why::Nearest,
        }
    }

    #[test]
    fn a_holder_faces_the_way_its_hold_says_and_a_flanker_faces_where_it_broke_one() {
        let (before, _, m) = turn();
        let facing = [[true; TEAM_SIZE], [false; TEAM_SIZE]];
        let (a, b) = (Slot::new(Side::Left, 0), Slot::new(Side::Left, 1));
        let (at_a, at_b) = (before.fighters[0][0].at, before.fighters[0][1].at);
        let them = Slot::new(Side::Right, 0);
        let there = before.fighters[1][0].at;
        let events = vec![
            at(0, hold(a, at_a, Facing::South)),
            at(0, EventKind::Holding { slot: a, at: at_a }),
            at(0, hold(b, at_b, Facing::North)),
            at(1, EventKind::Holding { slot: b, at: at_b }),
            at(
                2,
                EventKind::Flanked {
                    target: them,
                    by: vec![a, b],
                    from: there,
                    pushed_to: None,
                },
            ),
            at(2, EventKind::Holding { slot: a, at: at_a }),
        ];
        let playback = Playback::new(&before, &m.arena.gates, &facing, &events, 3);
        let start = playback.at(0.1);
        assert_eq!(start.figure(a).facing, Facing::South);
        assert!(
            !start.figure(a).facing_right,
            "south is down and to the left"
        );
        let end = playback.at(playback.duration());
        assert_eq!(end.figure(b).facing, Facing::North);
        assert!(end.figure(b).facing_right);
        assert_eq!(
            Some(end.figure(a).facing),
            Facing::toward(at_a, there),
            "taking the ground, it faces where the holder stood"
        );
    }

    #[test]
    fn a_press_pushes_its_target_on_and_snatches_what_it_carries() {
        let (mut before, _, m) = turn();
        let facing = [[true; TEAM_SIZE], [false; TEAM_SIZE]];
        let (by, target) = (Slot::new(Side::Left, 0), Slot::new(Side::Right, 0));
        before.fighters[1][0].carrying = Some(Item::Twig);
        let from = before.fighters[1][0].at;
        let to = Tile::new(from.x + 1, from.y);
        let events = vec![
            at(
                1,
                EventKind::Pressed {
                    target,
                    by: vec![by],
                    pushed_to: Some(to),
                    tricking: false,
                },
            ),
            at(
                1,
                EventKind::Stole {
                    by,
                    from: target,
                    item: Item::Twig,
                },
            ),
        ];
        let playback = Playback::new(&before, &m.arena.gates, &facing, &events, 3);
        let pressed = when(&playback, |k| matches!(k, EventKind::Pressed { .. }));
        let pushing = playback.at(pressed + 0.1);
        assert!(
            pushing
                .effects
                .iter()
                .any(|e| e.burst == Burst::Shove { to } && e.tile == from)
        );
        assert!(pushing.figure(target).off_balance);
        let stole = when(&playback, |k| matches!(k, EventKind::Stole { .. }));
        let flying = playback.at(stole + SNATCH_SECS / 2.0);
        assert_eq!(flying.figure(target).carrying, None);
        assert_eq!(flying.figure(by).carrying, None, "still in the air");
        assert!(flying.effects.iter().any(|e| matches!(
            e.burst,
            Burst::Snatch {
                item: Item::Twig,
                ..
            }
        )));
        let end = playback.at(playback.duration());
        assert_eq!(end.figure(target).tile, to);
        assert_eq!(
            end.figure(target).at,
            (f32::from(to.x) + 0.5, f32::from(to.y) + 0.5)
        );
        assert_eq!(end.figure(by).carrying, Some(Item::Twig));
    }

    #[test]
    fn a_hold_facing_a_presser_jabs_it_and_one_from_the_side_breaks_to_a_trick() {
        let (mut before, _, m) = turn();
        let facing = [[true; TEAM_SIZE], [false; TEAM_SIZE]];
        let holder = Slot::new(Side::Right, 0);
        let (presser, trickster) = (Slot::new(Side::Left, 0), Slot::new(Side::Left, 1));
        let there = before.fighters[1][0].at;
        let ahead = Facing::West.ahead(there);
        before.fighters[0][0].at = ahead;
        before.fighters[0][1].at = Facing::North.ahead(there);
        let events = vec![
            at(0, hold(holder, there, Facing::West)),
            at(
                0,
                EventKind::Holding {
                    slot: holder,
                    at: there,
                },
            ),
            at(
                1,
                EventKind::Repelled {
                    target: holder,
                    holders: vec![holder],
                    pressers: vec![presser],
                    faced: vec![presser],
                },
            ),
            at(
                2,
                EventKind::Tricked {
                    target: holder,
                    by: vec![trickster],
                    from: there,
                },
            ),
        ];
        let playback = Playback::new(&before, &m.arena.gates, &facing, &events, 3);
        let repelled = when(&playback, |k| matches!(k, EventKind::Repelled { .. }));
        let jab = playback.at(repelled + 0.1);
        assert!(
            jab.effects
                .iter()
                .any(|e| e.burst == Burst::Counter { from: there } && e.tile == ahead)
        );
        assert!(jab.figure(holder).holding);
        let tricked = when(&playback, |k| matches!(k, EventKind::Tricked { .. }));
        let broken = playback.at(tricked + 0.1);
        assert!(
            broken
                .effects
                .iter()
                .any(|e| matches!(e.burst, Burst::Sneak { .. }))
        );
        let end = playback.at(playback.duration());
        assert!(!end.figure(holder).holding);
        assert!(
            !end.figure(holder).off_balance,
            "tricked, it is still on its feet"
        );
        assert_eq!(end.figure(holder).tile, there, "a trick moves nobody");
    }

    #[test]
    fn two_presses_meeting_both_stop_off_balance() {
        let (before, _, m) = turn();
        let facing = [[true; TEAM_SIZE], [false; TEAM_SIZE]];
        let (a, b) = (Slot::new(Side::Left, 0), Slot::new(Side::Right, 0));
        let events = vec![at(
            1,
            EventKind::Collided {
                target: b,
                by: vec![a],
            },
        )];
        let playback = Playback::new(&before, &m.arena.gates, &facing, &events, 3);
        let met = when(&playback, |k| matches!(k, EventKind::Collided { .. }));
        assert!(
            playback
                .at(met + 0.1)
                .effects
                .iter()
                .any(|e| matches!(e.burst, Burst::Collide { .. }))
        );
        let end = playback.at(playback.duration());
        assert!(end.figure(a).off_balance && end.figure(b).off_balance);
    }
}
