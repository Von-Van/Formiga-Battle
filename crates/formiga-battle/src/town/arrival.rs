//! Arriving in town, where the reveal is scale. The train pulls in at the little halt inside the
//! habitat, the companions at its windows (`station`, in the habitat); the camera pulls back
//! until the habitat is a glass case on the Handler's desk, the station smaller than the mug;
//! then the view settles on the office from above, its tile grid showing for a moment, and the
//! Handler is there to walk.
//!
//! The first time it is the whole of it, about nine seconds. After that it is the train coming
//! in, a second and a half, and straight to the office. Escape, Space, Enter or a click skips
//! whatever is left; with reduced motion there is none of it. Nothing here keeps or changes
//! anything: skipping never resets the career, the trio or the town.

use super::picture::{TILE, VIEW};
use formiga_art::{Canvas, Rgba, paint};

/// How long each part lasts the first time, in seconds.
pub const CLOSE_UP: f32 = 4.0;
pub const PULLBACK: f32 = 3.0;
pub const HANDOFF: f32 = 2.0;
/// The train coming in, every time after the first.
pub const LATER: f32 = 1.5;
/// How long the office's tile grid takes to fade, in seconds.
const GRID_FADE: f32 = 0.5;
/// How long the cut from the desk to the office takes, in seconds.
const CUT: f32 = 0.3;

/// The desk, in its own pixels, and the glass case on it: left, top, width, height. Both are
/// the window's shape, so the pull back is one even zoom.
pub const DESK: (i32, i32) = (832, 468);
pub const CASE: (i32, i32, i32, i32) = (208, 107, 416, 234);

/// Which part of the arrival is showing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Beat {
    /// The train coming in at the halt in the habitat; how far through, 0 to 1.
    CloseUp(f32),
    /// The camera drawing back to the whole desk.
    PullBack(f32),
    /// The office from above, settling.
    Handoff(f32),
    /// Over: the Handler walks.
    Play,
}

#[derive(Clone, Copy, Debug)]
pub struct Arrival {
    began: f32,
    first: bool,
    skipped: bool,
}

impl Arrival {
    /// Arriving `now`: the whole of it if `first`, the train alone otherwise. With reduced
    /// motion it is over before it starts.
    pub fn new(now: f32, first: bool, reduce_motion: bool) -> Self {
        Self {
            began: now,
            first,
            skipped: reduce_motion,
        }
    }

    pub fn first(&self) -> bool {
        self.first
    }

    /// Skips whatever is left.
    pub fn skip(&mut self) {
        self.skipped = true;
    }

    pub fn beat(&self, now: f32) -> Beat {
        if self.skipped {
            return Beat::Play;
        }
        let t = now - self.began;
        if !self.first {
            return if t < LATER {
                Beat::CloseUp(t / LATER)
            } else {
                Beat::Play
            };
        }
        if t < CLOSE_UP {
            Beat::CloseUp(t / CLOSE_UP)
        } else if t < CLOSE_UP + PULLBACK {
            Beat::PullBack((t - CLOSE_UP) / PULLBACK)
        } else if t < CLOSE_UP + PULLBACK + HANDOFF {
            Beat::Handoff((t - CLOSE_UP - PULLBACK) / HANDOFF)
        } else {
            Beat::Play
        }
    }
}

/// The desk the habitat stands on: Jake's picture, the glass case exactly where `CASE` says.
pub fn desk() -> Canvas {
    super::art::bytes("arrival-desk.png")
        .and_then(super::sheet::decode)
        .unwrap_or_else(|| Canvas::new(DESK.0 as u32, DESK.1 as u32))
}

/// What stands in front of the train in the habitat: the near ferns and the glass.
pub fn habitat_front() -> Option<Canvas> {
    super::art::bytes("arrival-habitat-fg.png").and_then(super::sheet::decode)
}

/// A frame of the pull back, `t` from 0 to 1: the desk with the halt as it is now laid into the
/// glass case, seen through a window that grows, easing out, from the case to the whole desk.
/// For the first moment the halt itself shows over it, so the cut into the case is not seen.
pub fn pull_back(desk: &Canvas, halt: &Canvas, t: f32) -> Canvas {
    let mut scene = desk.clone();
    let (gx, gy, gw, gh) = CASE;
    // The halt, at half its size, inside the case.
    for y in 0..gh {
        for x in 0..gw {
            let sx = x * halt.width() as i32 / gw;
            let sy = y * halt.height() as i32 / gh;
            scene.set(gx + x, gy + y, halt.get(sx, sy));
        }
    }
    let e = 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3);
    let lerp = |a: f32, b: f32| a + (b - a) * e;
    let (rx, ry) = (lerp(gx as f32, 0.0), lerp(gy as f32, 0.0));
    let (rw, rh) = (
        lerp(gw as f32, DESK.0 as f32),
        lerp(gh as f32, DESK.1 as f32),
    );
    let mut view = sample(&scene, (rx, ry, rw, rh));
    let early = 0.15;
    if t < early {
        let a = (255.0 * (1.0 - t / early)) as u8;
        let close = sample(halt, (0.0, 0.0, halt.width() as f32, halt.height() as f32));
        blend(&mut view, &close, a);
    }
    view
}

/// A frame of the handoff, `t` from 0 to 1: the whole desk cut to the office from above, its tile
/// grid showing and fading. `origin` is where the office's view starts, for lining the grid up
/// with its tiles.
pub fn handoff(desk: &Canvas, office: &Canvas, origin: (i32, i32), t: f32) -> Canvas {
    let mut view = office.clone();
    let seconds = t * HANDOFF;
    if seconds < CUT {
        let whole = sample(desk, (0.0, 0.0, DESK.0 as f32, DESK.1 as f32));
        let mut faded = whole;
        blend(&mut faded, office, (255.0 * seconds / CUT) as u8);
        view = faded;
    }
    let grid = 1.0 - ((seconds - CUT) / GRID_FADE).clamp(0.0, 1.0);
    if grid > 0.0 {
        let line = Rgba::new(0xff, 0xf8, 0xe4, (grid * 90.0) as u8);
        for y in 0..VIEW.1 {
            for x in 0..VIEW.0 {
                let (wx, wy) = (x + origin.0, y + origin.1);
                if wx.rem_euclid(TILE) == 0 || wy.rem_euclid(TILE) == 0 {
                    paint::put(&mut view, x, y, line);
                }
            }
        }
    }
    view
}

/// Part of a picture, seen at the window's size: nearest pixel, so pixels stay pixels.
fn sample(from: &Canvas, (rx, ry, rw, rh): (f32, f32, f32, f32)) -> Canvas {
    let mut out = Canvas::new(VIEW.0 as u32, VIEW.1 as u32);
    for y in 0..VIEW.1 {
        for x in 0..VIEW.0 {
            let sx = rx + (x as f32 + 0.5) / VIEW.0 as f32 * rw;
            let sy = ry + (y as f32 + 0.5) / VIEW.1 as f32 * rh;
            out.set(x, y, from.get(sx as i32, sy as i32));
        }
    }
    out
}

/// `top` laid over `under` at `alpha` of 255.
fn blend(under: &mut Canvas, top: &Canvas, alpha: u8) {
    for y in 0..under.height() as i32 {
        for x in 0..under.width() as i32 {
            let t = top.get(x, y);
            paint::put(under, x, y, Rgba::new(t.r, t.g, t.b, alpha));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_arrival_runs_close_up_pull_back_handoff_then_play() {
        let a = Arrival::new(10.0, true, false);
        assert!(matches!(a.beat(10.5), Beat::CloseUp(_)));
        assert!(matches!(a.beat(10.0 + CLOSE_UP + 0.1), Beat::PullBack(_)));
        assert!(matches!(
            a.beat(10.0 + CLOSE_UP + PULLBACK + 0.1),
            Beat::Handoff(_)
        ));
        assert_eq!(
            a.beat(10.0 + CLOSE_UP + PULLBACK + HANDOFF + 0.1),
            Beat::Play
        );
    }

    #[test]
    fn a_later_arrival_is_the_train_alone() {
        let a = Arrival::new(0.0, false, false);
        assert!(matches!(a.beat(1.0), Beat::CloseUp(_)));
        assert_eq!(a.beat(LATER + 0.01), Beat::Play);
    }

    #[test]
    fn skipping_or_reduced_motion_goes_straight_to_play() {
        let mut a = Arrival::new(0.0, true, false);
        a.skip();
        assert_eq!(a.beat(0.1), Beat::Play);
        assert_eq!(Arrival::new(0.0, true, true).beat(0.0), Beat::Play);
    }

    #[test]
    fn the_case_and_the_desk_are_the_windows_shape() {
        let (_, _, w, h) = CASE;
        assert_eq!(w * VIEW.1, h * VIEW.0, "case {w}x{h}");
        assert_eq!(DESK.0 * h, DESK.1 * w);
    }

    #[test]
    fn the_pull_back_starts_on_the_halt_and_ends_on_the_whole_desk() {
        let desk = desk();
        let mut halt = Canvas::new(VIEW.0 as u32, VIEW.1 as u32);
        halt.fill_rect(0, 0, VIEW.0, VIEW.1, Rgba::new(1, 2, 3, 255));
        let start = pull_back(&desk, &halt, 0.0);
        assert_eq!(start.get(200, 100), Rgba::new(1, 2, 3, 255));
        let end = pull_back(&desk, &halt, 1.0);
        assert_eq!(end.get(5, 5), desk.get(11, 11));
        assert_eq!((desk.width() as i32, desk.height() as i32), DESK);
    }
}
