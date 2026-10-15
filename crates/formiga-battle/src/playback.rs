//! A resolved turn played back over time, from the board as it stood and the turn's events.
//!
//! The rules hand back where everyone ended up and what happened on the way; this works out what
//! the board looks like at any moment in between, so the window can animate it and a review
//! picture can stop at any point. Each tick is shown in three beats, the same order the rules
//! settle it in: contests already in reach, then everyone's steps together, then whatever those
//! steps brought into reach, the beacon and the end.

use crate::look::Pose;
use formiga_battle_core::game::Board;
use formiga_battle_core::{Event, EventKind, Side, Slot, TEAM_SIZE, Tile};

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
    pub pose: Pose,
    /// Seconds into its pose, for its animation.
    pub posed_for: f32,
    pub composure: u8,
    pub out: bool,
    /// How much of it shows, from 0 to 1: a knocked-out creature fades as it flops.
    pub opacity: f32,
    pub holding: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Burst {
    /// A contest landing on someone not holding.
    Hit,
    /// A contest turned away by someone holding.
    Shield,
    /// Two busy with contests of their own.
    Scuffle,
    /// Two wanting the same tile.
    Bump,
    /// A cartoon knockout.
    Stars,
    /// The lever pulled.
    Spark,
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
}

/// How long effects stay on the board.
const EFFECT_SECS: f32 = 0.7;

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
                    pose: if f.holding { Pose::Hold } else { Pose::Idle },
                    posed_for: 0.0,
                    composure: f.composure,
                    out: f.out,
                    opacity: if f.out { 0.0 } else { 1.0 },
                    holding: f.holding,
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
        // Whoever holds where it stands holds from the moment orders lock.
        for event in events.iter().filter(|e| e.tick == 0) {
            if let EventKind::Holding { slot, .. } = event.kind {
                let f = start.figure_mut(slot);
                f.holding = true;
                f.pose = Pose::Hold;
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
            gates: gates.to_vec(),
            timed,
            ticks: played,
            winners,
        }
    }

    /// How long the whole turn takes to show.
    pub fn duration(&self) -> f32 {
        REVEAL_SECS + f32::from(self.ticks) * TICK_SECS + HOLD_SECS
    }

    /// The board `t` seconds into the turn.
    pub fn at(&self, t: f32) -> Frame {
        let mut frame = self.start.clone();
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
                EventKind::HungBack { slot, .. } => {
                    if age < 0.6 {
                        set_pose(&mut frame, &mut since, *slot, Pose::Worry, *when);
                    }
                }
                EventKind::Holding { slot, .. } => {
                    frame.figure_mut(*slot).holding = true;
                    set_pose(&mut frame, &mut since, *slot, Pose::Hold, *when);
                }
                EventKind::Interrupted { target, by } => {
                    let tile = frame.figure(*target).tile;
                    effect(&mut frame, Burst::Hit, tile);
                    let short = age < 0.6;
                    set_pose(
                        &mut frame,
                        &mut since,
                        *target,
                        if short { Pose::Gasp } else { Pose::Idle },
                        *when,
                    );
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
                EventKind::Countered {
                    target,
                    contesters,
                    pressed,
                    ..
                } => {
                    let tile = frame.figure(*target).tile;
                    effect(&mut frame, Burst::Shield, tile);
                    let short = age < 0.6;
                    let held = if short && *pressed {
                        Pose::Wobble
                    } else {
                        Pose::Hold
                    };
                    set_pose(&mut frame, &mut since, *target, held, *when);
                    for c in contesters {
                        set_pose(
                            &mut frame,
                            &mut since,
                            *c,
                            if short { Pose::Reach } else { Pose::Idle },
                            *when,
                        );
                    }
                }
                EventKind::Scuffle {
                    contester, target, ..
                } => {
                    let tile = frame.figure(*target).tile;
                    effect(&mut frame, Burst::Scuffle, tile);
                    if age < 0.6 {
                        set_pose(&mut frame, &mut since, *contester, Pose::Worry, *when);
                    }
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
                EventKind::GateJammed { gate, .. } => {
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
                }
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
                EventKind::OrderFilled { .. }
                | EventKind::TookOpportunity { .. }
                | EventKind::LeverResting { .. }
                | EventKind::CaptureContested => {}
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
    use formiga_battle_core::{Arena, Match, Ruleset};

    fn turn() -> (Board, formiga_battle_core::TurnReport, Match) {
        let rosters = [
            (0..3).map(|i| drawn_creature(2, i)).collect(),
            (3..6).map(|i| drawn_creature(2, i)).collect(),
        ];
        let mut m = Match::new(Arena::backyard(), Ruleset::default(), 2, rosters).unwrap();
        let before = m.board.clone();
        let orders = [
            Coach::Exhibition.orders(&m, Side::Left),
            Coach::Exhibition.orders(&m, Side::Right),
        ];
        let report = m.resolve_turn(orders);
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
}
