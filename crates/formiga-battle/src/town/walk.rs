//! The Handler on foot: one tile a step, in four directions, as the older handheld games walk.
//! A tap on a new direction turns on the spot; holding it walks. A click asks for a walk to a
//! tile, which is taken a step at a time and given up as soon as a key is pressed.
//!
//! Everything here depends only on the time it is given, so walking is checked without a window.

use super::map::{Facing, Ground, Place};

/// How long one step takes, in seconds.
pub const STEP: f32 = 0.22;
/// How long a new direction must be held before turning becomes walking, in seconds.
pub const TURN: f32 = 0.09;

/// A step under way.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Stepping {
    from: (i32, i32),
    started: f32,
}

/// What a frame of walking came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moved {
    /// Nothing new.
    Still,
    /// A step began.
    Stepped,
    /// A step ended on a door: the place should change.
    Door(char),
    /// The way was closed: a bump against a wall or someone.
    Bumped,
}

#[derive(Clone, Debug)]
pub struct Walker {
    pub tile: (i32, i32),
    pub facing: Facing,
    stepping: Option<Stepping>,
    /// The direction held, and since when.
    held: Option<(Facing, f32)>,
    /// A walk asked for by a click, as the steps left.
    route: Vec<Facing>,
    /// Steps taken since standing still, for the walk cycle.
    strides: u32,
    /// When the Handler last stood still, for breathing.
    rested: f32,
    /// The last wall bumped into, so a held key bumps only once.
    bumped: Option<((i32, i32), Facing)>,
}

impl Walker {
    pub fn new(tile: (i32, i32), facing: Facing, now: f32) -> Self {
        Self {
            tile,
            facing,
            stepping: None,
            held: None,
            route: Vec::new(),
            strides: 0,
            rested: now,
            bumped: None,
        }
    }

    /// Whether a step is under way.
    pub fn moving(&self) -> bool {
        self.stepping.is_some()
    }

    /// Follows a walk to one of the goals, given up by the next key.
    pub fn walk_to(&mut self, steps: Vec<Facing>) {
        self.route = steps;
        self.route.reverse();
    }

    /// Whether a click's walk is under way.
    pub fn routed(&self) -> bool {
        !self.route.is_empty()
    }

    /// Turns on the spot, as reaching for something faces it.
    pub fn face(&mut self, facing: Facing) {
        if self.stepping.is_none() {
            self.facing = facing;
        }
    }

    /// One frame: the direction held now, if any.
    pub fn tick(&mut self, held: Option<Facing>, place: &Place, now: f32) -> Moved {
        if held.is_some() {
            self.route.clear();
        }
        // The direction held, and for how long.
        self.held = match (held, self.held) {
            (Some(f), Some((g, since))) if f == g => Some((f, since)),
            (Some(f), _) => Some((f, now)),
            (None, _) => None,
        };
        let mut moved = Moved::Still;
        if let Some(step) = self.stepping {
            if now - step.started < STEP {
                return Moved::Still;
            }
            self.stepping = None;
            if let Ground::Door(i) = place.ground(self.tile) {
                self.route.clear();
                return Moved::Door(place.doors[i].mark);
            }
            moved = Moved::Still;
        }
        let want = if let Some((facing, since)) = self.held {
            // A tap on a new direction only turns; once turned, or held long enough, it walks.
            if facing != self.facing && self.strides == 0 && now - since < TURN {
                self.facing = facing;
                return moved;
            }
            Some(facing)
        } else {
            self.route.pop()
        };
        let Some(facing) = want else {
            if self.strides > 0 {
                self.strides = 0;
                self.rested = now;
            }
            self.bumped = None;
            return moved;
        };
        self.facing = facing;
        let (dx, dy) = facing.step();
        let next = (self.tile.0 + dx, self.tile.1 + dy);
        if !place.can_step(self.tile, facing) {
            self.route.clear();
            if self.strides > 0 {
                self.strides = 0;
                self.rested = now;
            }
            if self.bumped == Some((self.tile, facing)) {
                return moved;
            }
            self.bumped = Some((self.tile, facing));
            return Moved::Bumped;
        }
        self.bumped = None;
        self.stepping = Some(Stepping {
            from: self.tile,
            started: now,
        });
        self.tile = next;
        self.strides += 1;
        Moved::Stepped
    }

    /// Where the Handler's feet are, in tiles, partway through a step.
    pub fn at(&self, now: f32) -> (f32, f32) {
        match self.stepping {
            Some(step) => {
                let t = ((now - step.started) / STEP).clamp(0.0, 1.0);
                let lerp = |a: i32, b: i32| a as f32 + (b - a) as f32 * t;
                (
                    lerp(step.from.0, self.tile.0),
                    lerp(step.from.1, self.tile.1),
                )
            }
            None => (self.tile.0 as f32, self.tile.1 as f32),
        }
    }

    /// The frame of the walk or stand, as a column of the character sheet: 0 to 3 walking
    /// (step, pass, step, pass, a stride of two steps), 4 or 5 standing (breathing in and out).
    pub fn frame(&self, now: f32, reduce_motion: bool) -> usize {
        match self.stepping {
            Some(step) => {
                let half = ((now - step.started) / STEP).clamp(0.0, 0.999) >= 0.5;
                // Each step is a contact then a passing frame; alternate steps lead with the
                // other foot.
                let lead = if self.strides % 2 == 1 { 0 } else { 2 };
                lead + usize::from(half)
            }
            None if reduce_motion => 4,
            None => {
                // A slow breath: out for a little longer than in.
                let t = (now - self.rested).rem_euclid(2.4);
                if t < 1.4 { 4 } else { 5 }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::town::map::PlaceId;

    fn room() -> Place {
        let text = "name Test\nsize 5 3\nground\n.....\n.#...\n..1..\ndoor 1 campus 0 up\n";
        Place::parse(PlaceId::Office, text).unwrap()
    }

    /// Ticks at 60 frames a second for `seconds`, holding a direction.
    fn hold(
        w: &mut Walker,
        place: &Place,
        facing: Option<Facing>,
        from: f32,
        seconds: f32,
    ) -> Vec<Moved> {
        let mut out = Vec::new();
        let mut t = from;
        while t < from + seconds {
            out.push(w.tick(facing, place, t));
            t += 1.0 / 60.0;
        }
        out
    }

    #[test]
    fn a_tap_on_a_new_direction_turns_without_stepping() {
        let place = room();
        let mut w = Walker::new((0, 0), Facing::Down, 0.0);
        w.tick(Some(Facing::Right), &place, 0.0);
        w.tick(None, &place, 0.03);
        assert_eq!((w.tile, w.facing), ((0, 0), Facing::Right));
    }

    #[test]
    fn holding_a_direction_walks_a_tile_a_step() {
        let place = room();
        let mut w = Walker::new((0, 0), Facing::Right, 0.0);
        hold(&mut w, &place, Some(Facing::Right), 0.0, STEP * 4.0 + 0.1);
        hold(&mut w, &place, None, 1.0, 0.5);
        assert_eq!(w.tile, (4, 0));
    }

    #[test]
    fn a_wall_stops_the_walk_and_is_bumped_once() {
        let place = room();
        let mut w = Walker::new((0, 1), Facing::Right, 0.0);
        let moved = hold(&mut w, &place, Some(Facing::Right), 0.0, 1.0);
        assert_eq!(w.tile, (0, 1));
        assert_eq!(moved.iter().filter(|m| **m == Moved::Bumped).count(), 1);
    }

    #[test]
    fn a_step_onto_a_door_says_so_when_it_ends() {
        let place = room();
        let mut w = Walker::new((2, 0), Facing::Down, 0.0);
        let moved = hold(&mut w, &place, Some(Facing::Down), 0.0, STEP * 2.0 + 0.05);
        assert!(moved.contains(&Moved::Door('1')), "{moved:?}");
    }

    #[test]
    fn a_clicked_walk_goes_round_the_wall_and_a_key_gives_it_up() {
        let place = room();
        let mut w = Walker::new((0, 2), Facing::Up, 0.0);
        let steps = place.path((0, 2), &[(2, 1)]).unwrap();
        w.walk_to(steps);
        hold(&mut w, &place, None, 0.0, 2.0);
        assert_eq!(w.tile, (2, 1));

        let mut w = Walker::new((0, 0), Facing::Right, 0.0);
        w.walk_to(place.path((0, 0), &[(4, 0)]).unwrap());
        hold(&mut w, &place, None, 0.0, STEP + 0.02);
        w.tick(Some(Facing::Down), &place, 0.5);
        assert!(!w.routed());
    }

    #[test]
    fn the_walk_cycle_takes_all_four_frames_over_two_steps() {
        let place = room();
        let mut w = Walker::new((0, 0), Facing::Right, 0.0);
        let mut frames = std::collections::BTreeSet::new();
        let mut t = 0.0;
        while t < STEP * 2.0 {
            w.tick(Some(Facing::Right), &place, t);
            frames.insert(w.frame(t, false));
            t += 1.0 / 60.0;
        }
        assert_eq!(frames.into_iter().collect::<Vec<_>>(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn standing_still_breathes_unless_motion_is_reduced() {
        let place = room();
        let mut w = Walker::new((0, 0), Facing::Down, 0.0);
        w.tick(None, &place, 0.0);
        let frames: std::collections::BTreeSet<usize> =
            (0..240).map(|i| w.frame(i as f32 / 60.0, false)).collect();
        assert_eq!(frames.into_iter().collect::<Vec<_>>(), vec![4, 5]);
        assert_eq!(w.frame(2.0, true), 4);
    }
}
