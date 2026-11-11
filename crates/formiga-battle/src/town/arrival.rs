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
pub const CASE: (i32, i32, i32, i32) = (312, 140, 208, 117);

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

/// The desk the habitat stands on, a stand-in painted here until its picture is drawn: the
/// desk's top in warm lamplight with the glass case in the middle, an open notebook of team
/// plans, the lamp, a mug, pencils, a little cup and a plant. Light from the upper left, every
/// material a ramp, edges in a darker shade of their own colour.
pub fn desk_stand_in() -> Canvas {
    let rgb = |hex: u32| Rgba::new((hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255);
    let wood = [rgb(0x4a2d1d), rgb(0x6b4128), rgb(0x8a5634), rgb(0xa86d43)];
    let wall = [rgb(0x3a2a22), rgb(0x4d382d)];
    let paper = [rgb(0xc9b88f), rgb(0xe9dcb8), rgb(0xf8efd6)];
    let pencil = rgb(0x6e776c);
    let brass = [rgb(0x6a4e22), rgb(0x9c7a35), rgb(0xd1ac55), rgb(0xf0d58a)];
    let (w, h) = DESK;
    let mut c = Canvas::new(w as u32, h as u32);
    // The wall behind, and the desk's top running back to it.
    let back = 96;
    for y in 0..h {
        for x in 0..w {
            let n = paint::noise(x, y, 701) % 17;
            let tone = if y < back {
                wall[usize::from(n == 0)]
            } else {
                // Planks running across, a seam every 46 pixels, the grain in streaks.
                let seam = (y - back) % 46 == 0;
                let streak = paint::noise(x / 7, y, 703).is_multiple_of(5);
                if seam {
                    wood[0]
                } else if streak || n == 0 {
                    wood[1]
                } else {
                    wood[2]
                }
            };
            c.set(x, y, tone);
        }
    }
    paint::rect(&mut c, 0, back, w, 2, wood[3]);
    // The edge of a window at the top right, evening beyond it.
    paint::rect(&mut c, w - 150, 0, 120, back - 12, rgb(0x2c3b52));
    paint::rect(&mut c, w - 150, 0, 120, 4, rgb(0x5b3a27));
    paint::rect(&mut c, w - 92, 0, 4, back - 12, rgb(0x5b3a27));
    // The lamp's light: a warm pool, laid over the desk in rings.
    for (r, a) in [(300, 18), (220, 22), (150, 26)] {
        for y in back..h {
            for x in 0..w {
                let (dx, dy) = (x - 150, (y - 210) * 3 / 2);
                if dx * dx + dy * dy < r * r {
                    paint::put(&mut c, x, y, Rgba::new(0xff, 0xd8, 0x8a, a));
                }
            }
        }
    }
    // The open notebook, lower left: two pages with team plans pencilled on them.
    let (nx, ny, nw, nh) = (60, 290, 230, 150);
    paint::rect(&mut c, nx + 4, ny + 4, nw, nh, Rgba::new(0, 0, 0, 70));
    paint::rect(&mut c, nx, ny, nw, nh, paper[1]);
    paint::rect(&mut c, nx, ny, nw, 2, paper[2]);
    paint::rect(&mut c, nx + nw / 2 - 1, ny, 2, nh, paper[0]);
    for row in (ny + 16..ny + nh - 8).step_by(12) {
        paint::rect(&mut c, nx + 8, row, nw / 2 - 18, 1, paper[0]);
    }
    for (cx, cy) in [
        (nx + 160, ny + 50),
        (nx + 195, ny + 80),
        (nx + 150, ny + 105),
    ] {
        for a in 0..24 {
            let t = a as f32 / 24.0 * std::f32::consts::TAU;
            let (px, py) = (cx + (t.cos() * 9.0) as i32, cy + (t.sin() * 9.0) as i32);
            c.set(px, py, pencil);
        }
    }
    for i in 0..40 {
        c.set(nx + 168 + i, ny + 56 + i / 2, pencil);
    }
    // Pencils beside it.
    paint::rect(&mut c, nx + nw + 14, ny + 30, 6, 90, rgb(0xd8a23a));
    paint::rect(&mut c, nx + nw + 14, ny + 120, 6, 8, paper[1]);
    paint::rect(&mut c, nx + nw + 24, ny + 44, 6, 80, rgb(0x3d6a55));
    // The lamp, upper left: its foot, its arm and its shade.
    paint::rect(&mut c, 120, 236, 64, 12, brass[0]);
    paint::rect(&mut c, 122, 234, 60, 4, brass[2]);
    for i in 0..110 {
        paint::rect(&mut c, 150 + i / 3, 236 - i, 4, 1, brass[1]);
    }
    paint::rect(&mut c, 160, 110, 70, 30, rgb(0x285a4b));
    paint::rect(&mut c, 160, 110, 70, 4, rgb(0x3b7563));
    paint::rect(&mut c, 166, 140, 58, 3, rgb(0xfff0bd));
    // A mug, right of the case, bigger than the station inside it.
    let (mx, my) = (560, 230);
    paint::rect(&mut c, mx + 3, my + 3, 44, 56, Rgba::new(0, 0, 0, 70));
    paint::rect(&mut c, mx, my, 44, 56, rgb(0xb5543f));
    paint::rect(&mut c, mx, my, 44, 6, rgb(0x7a2c33));
    paint::rect(&mut c, mx + 2, my + 8, 6, 44, rgb(0xd07a5f));
    paint::rect(&mut c, mx + 44, my + 14, 10, 4, rgb(0x8a3f2f));
    paint::rect(&mut c, mx + 50, my + 14, 4, 24, rgb(0x8a3f2f));
    paint::rect(&mut c, mx + 44, my + 34, 10, 4, rgb(0x8a3f2f));
    // A little cup on the right, and a plant in a pot.
    paint::rect(&mut c, 680, 250, 30, 8, brass[0]);
    paint::rect(&mut c, 688, 228, 14, 22, brass[2]);
    paint::rect(&mut c, 680, 210, 30, 20, brass[3]);
    paint::rect(&mut c, 682, 212, 4, 14, brass[1]);
    paint::rect(&mut c, 740, 320, 50, 44, rgb(0xa8584a));
    paint::rect(&mut c, 740, 320, 50, 5, rgb(0xc4775a));
    for (lx, ly, r) in [(752, 300, 18), (776, 294, 16), (764, 280, 14)] {
        c.fill_circle(lx, ly, r, rgb(0x3f6a45));
        c.fill_circle(lx - 4, ly - 4, r / 2, rgb(0x578656));
    }
    // The glass case: its brass frame round the habitat, the inside left for the halt.
    let (gx, gy, gw, gh) = CASE;
    paint::rect(&mut c, gx - 6, gy - 6, gw + 12, gh + 12, brass[0]);
    paint::rect(&mut c, gx - 6, gy - 6, gw + 12, 2, brass[3]);
    paint::rect(&mut c, gx - 6, gy - 6, 2, gh + 12, brass[2]);
    paint::rect(&mut c, gx - 10, gy + gh + 6, gw + 20, 8, wood[0]);
    c
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
        let desk = desk_stand_in();
        let mut halt = Canvas::new(VIEW.0 as u32, VIEW.1 as u32);
        halt.fill_rect(0, 0, VIEW.0, VIEW.1, Rgba::new(1, 2, 3, 255));
        let start = pull_back(&desk, &halt, 0.0);
        assert_eq!(start.get(200, 100), Rgba::new(1, 2, 3, 255));
        let end = pull_back(&desk, &halt, 1.0);
        assert_eq!(end.get(5, 5), desk.get(11, 11));
    }
}
