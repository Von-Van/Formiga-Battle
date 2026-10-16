//! The halt Battle opens at: a platform in the country where the colony's train comes in with the
//! Handler's companions already riding it, since they got aboard on Desktop when Battle was
//! chosen. It stands puffing, a face at every window, while the Handler chooses where they are
//! off to; choosing sends it on its way, as Hill's train takes a colony home.
//!
//! The train is `formiga-art`'s, the one Desktop sends across the desktop and Hill meets at its
//! station, given a coach more for every three companions past six so all of them ride, and each
//! face at a window is framed as Hill frames it. The halt round it is painted here to Hill's and
//! Home's rules: every material a ramp, light from the upper left, edges in a darker shade of
//! their own colour, grain from `paint::noise`.
//!
//! Everything here depends only on the time it is given, so the whole arrival can be checked
//! without a window. With reduced motion there is none of it: the train stands at the platform
//! with everyone aboard, and goes without being seen to.

use crate::garden::{GRASS, LEAF, STONE};
use crate::look::{Look, Pose};
use formiga_art::paint::{self, Ramp, rgb, rgba};
use formiga_art::{
    Canvas, FRAME_SIZE, RUNNING_FRAMES, Rgba, TRAIN_FRAMES, TRAIN_GROUND, TRAIN_HEIGHT,
    TRAIN_WIDTH, TrainLook, TrainRenderer,
};
use std::f32::consts::TAU;

// `formiga-art`'s train, measured as Hill measures it.
/// A coach and the coupling at its front: what one more coach adds to the train.
const COACH: i32 = 70;
const AXLE: i32 = 59;
/// Where the train's own top row is in `formiga-art`'s frames, below the room left for steam.
const HEADROOM: i32 = TRAIN_GROUND as i32 - (AXLE + 3);
const WINDOW_SIZE: (i32, i32) = (14, 13);
const WINDOW_TOP: i32 = 18;
const WINDOW_XS: [i32; 3] = [8, 26, 44];
/// The driving wheels' radius: a full turn of them is this much travel, times τ.
const DRIVING_WHEEL: f32 = 7.0;
const PUFFS_PER_SECOND: f32 = 1.3;

// The train's comings and goings, in seconds: Hill's own pace.
const PULL_IN: f32 = 3.2;
const PULL_OUT: f32 = 3.0;
/// How long the train is watched pulling away before the notebook opens.
pub const SEEN_OFF: f32 = 1.1;

// The halt, in rows up from the near rail.
/// The near rail, up from the foot of the scene.
const RAIL: i32 = 14;
const PLATFORM_BACK: i32 = 53;
const PLATFORM_EDGE: i32 = 26;
const PLATFORM_FOOT: i32 = 19;
const HORIZON: i32 = 67;

const SKY_HIGH: Rgba = rgb(0x8ec9e4);
const SKY_LOW: Rgba = rgb(0xdaefe8);
const CLOUD: Ramp = Ramp::new(0xb7cfd9, 0xd3e4ea, 0xf3f8f7, 0xfbfdfc, 0xffffff);
const FAR_HILL: Ramp = Ramp::new(0x5f8d86, 0x79a49a, 0x91b9ac, 0xa8cbbd, 0xc2dccf);
const BRICK: Ramp = Ramp::new(0x6a3328, 0x8f4636, 0xab5a42, 0xc4775a, 0xdc9c7c);
const BALLAST: Ramp = Ramp::new(0x4e4640, 0x6b625a, 0x857b70, 0x9f958a, 0xbab1a6);
const SLEEPER: Ramp = Ramp::new(0x3e2a1d, 0x55392a, 0x6c4a36, 0x83604a, 0x9c7a62);
const STEEL: Ramp = Ramp::new(0x3f4249, 0x5d626b, 0x80858e, 0xb2b7bf, 0xe3e7ec);
const IRON: Ramp = Ramp::new(0x2e2a30, 0x48434a, 0x67616a, 0x8a848c, 0xb0aab2);
const PICKET: Ramp = Ramp::new(0x7a7466, 0xb8b0a0, 0xe2dccc, 0xf4f0e4, 0xffffff);
/// The station board: the train's own cream, edged in its maroon.
const BOARD: Ramp = Ramp::new(0x3a1418, 0xd8c9a8, 0xf0e4c6, 0xfaf3e1, 0xffffff);
const MAROON: Rgba = rgb(0x7a2c33);
const LAMPLIGHT: Rgba = rgb(0xfff0bd);

/// A rectangle of the scene, in its pixels: left, top, width, height.
pub type Area = (i32, i32, i32, i32);

/// How the train is moving, which decides the frame it is drawn in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Going {
    /// Rolling, this far so far in pixels, for how far round the wheels are.
    Rolling(f32),
    /// Stood at the platform, puffing.
    Standing,
}

/// When the whistle goes as the train comes into hearing, and how long before it stops it lets
/// off steam: Hill's.
const WHISTLE_IN: f32 = 0.15;
const BRAKES_BEFORE_STOP: f32 = 0.9;

/// Something heard at the halt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Heard {
    /// The whistle, as the train comes in.
    Whistle,
    /// Steam let off as it stops.
    Brakes,
    /// The whistle again, as it pulls away.
    Away,
}

/// Where one companion rides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    /// At this window.
    Window(usize),
    /// With no window of its own: only a train too long for the halt has these.
    Unseen,
}

/// Everything that moves, at one moment.
#[derive(Clone, Debug, PartialEq)]
pub struct Moment {
    /// The train's left end, if it is in sight.
    pub train_x: Option<f32>,
    pub going: Going,
    pub places: Vec<Place>,
}

/// One companion at the halt.
struct Rider {
    look: Look,
    facing_right: bool,
    face: (i32, i32),
}

pub struct Station {
    size: (i32, i32),
    backdrop: Canvas,
    /// Every frame of the train, as long as the colony needs, in `formiga-art`'s order.
    train: Vec<Canvas>,
    coaches: i32,
    riders: Vec<Rider>,
    reduce_motion: bool,
    /// When the train set off for the halt.
    began: f32,
    /// When it set off again, with everyone aboard.
    leaving: Option<f32>,
}

impl Station {
    /// The halt, `size` scene pixels across and down, with the train and everyone on it setting
    /// off for it `now`.
    pub fn new(looks: Vec<Look>, size: (i32, i32), now: f32, reduce_motion: bool) -> Self {
        let riders = looks
            .into_iter()
            .enumerate()
            .map(|(i, look)| {
                // Some look up the line and some down it, as people on a train do.
                let facing_right = i % 3 != 1;
                let face = look.face(facing_right);
                Rider {
                    look,
                    facing_right,
                    face,
                }
            })
            .collect();
        let mut station = Self {
            size: (0, 0),
            backdrop: Canvas::new(1, 1),
            train: Vec::new(),
            coaches: 0,
            riders,
            reduce_motion,
            began: if reduce_motion { f32::MIN / 2.0 } else { now },
            leaving: None,
        };
        station.resize(size);
        station
    }

    /// Repaints the halt for a new size of window. The train keeps its place in the pickup.
    pub fn resize(&mut self, size: (i32, i32)) {
        let size = (size.0.max(320), size.1.max(160));
        if size == self.size {
            return;
        }
        self.size = size;
        self.backdrop = backdrop(size);
        let coaches = coaches_for(self.riders.len(), size.0);
        if coaches != self.coaches {
            self.coaches = coaches;
            self.train = (0..TRAIN_FRAMES)
                .map(|frame| {
                    lengthened(
                        &TrainRenderer::render(&TrainLook { lit: false }, frame),
                        coaches,
                    )
                })
                .collect();
        }
    }

    fn length(&self) -> i32 {
        train_length(self.coaches)
    }

    fn windows(&self) -> usize {
        self.coaches as usize * 3
    }

    fn stop_x(&self) -> f32 {
        ((self.size.0 - self.length()) / 2) as f32
    }

    /// The train is standing at the platform: the time to ask where they are off to.
    pub fn stopped(&self, now: f32) -> bool {
        self.leaving.is_none() && now - self.began >= PULL_IN
    }

    /// Skips to the train standing at the platform, as a click or a key does while it comes in.
    pub fn skip(&mut self, now: f32) {
        if !self.stopped(now) && self.leaving.is_none() {
            self.began = now - PULL_IN;
        }
    }

    /// Sends the train on its way.
    pub fn set_off(&mut self, now: f32) {
        self.skip(now);
        self.leaving.get_or_insert(now);
    }

    pub fn leaving(&self) -> bool {
        self.leaving.is_some()
    }

    /// The train has been seen off: the notebook can open. At once under reduced motion.
    pub fn seen_off(&self, now: f32) -> bool {
        self.leaving
            .is_some_and(|at| self.reduce_motion || now - at >= SEEN_OFF)
    }

    /// Brings the train back to the platform with everyone aboard, for whoever comes back to the
    /// station from the notebook.
    pub fn wait_at_platform(&mut self, now: f32) {
        self.leaving = None;
        self.began = now - PULL_IN;
    }

    /// Brings the train in again with everyone aboard, as it came the first time: back from a
    /// match. Under reduced motion it is simply in.
    pub fn come_in(&mut self, now: f32) {
        self.leaving = None;
        if !self.reduce_motion {
            self.began = now;
        }
    }

    /// When the train came to a stop at the platform, or will.
    pub fn stopped_at(&self) -> f32 {
        self.began + PULL_IN
    }

    /// What is heard of the train after `from` and up to `to`: its whistle as it comes, steam as
    /// it stops, and its whistle as it goes.
    pub fn heard(&self, from: f32, to: f32) -> Vec<Heard> {
        if self.reduce_motion || to <= from {
            return Vec::new();
        }
        let mut sounds = vec![
            (self.began + WHISTLE_IN, Heard::Whistle),
            (self.began + PULL_IN - BRAKES_BEFORE_STOP, Heard::Brakes),
        ];
        if let Some(at) = self.leaving {
            sounds.push((at, Heard::Away));
        }
        sounds
            .into_iter()
            .filter(|(at, _)| *at > from && *at <= to)
            .map(|(_, h)| h)
            .collect()
    }

    /// Which window a companion rides at: spread along the train, as Hill seats them.
    fn window_for(&self, index: usize) -> Option<usize> {
        let (n, windows) = (self.riders.len(), self.windows());
        if n <= windows {
            Some(index * windows / n.max(1))
        } else {
            (index < windows).then_some(index)
        }
    }

    /// Where the train and everyone are, `now`.
    pub fn moment(&self, now: f32) -> Moment {
        let stop = self.stop_x();
        let start = -(self.length() as f32) - 8.0;
        let end = self.size.0 as f32 + 8.0;
        let t = now - self.began;
        let places = (0..self.riders.len())
            .map(|i| self.window_for(i).map_or(Place::Unseen, Place::Window))
            .collect();
        let (train_x, going) = if let Some(at) = self.leaving {
            let u = ((now - at) / PULL_OUT).clamp(0.0, 1.0);
            let x = stop + (end - stop) * u * u;
            if u >= 1.0 {
                (None, Going::Standing)
            } else if self.reduce_motion {
                (Some(stop), Going::Standing)
            } else {
                (Some(x), Going::Rolling(x - start))
            }
        } else if t < PULL_IN {
            let u = (t / PULL_IN).clamp(0.0, 1.0);
            let x = start + (stop - start) * (1.0 - (1.0 - u).powi(3));
            (Some(x), Going::Rolling(x - start))
        } else {
            (Some(stop), Going::Standing)
        };
        Moment {
            train_x,
            going,
            places,
        }
    }

    /// The scene's size, in its own pixels, after any resizing to fit the halt.
    pub fn size(&self) -> (i32, i32) {
        self.size
    }

    /// The station board, where the window writes the halt's name.
    pub fn sign(&self) -> Area {
        sign_area(self.size)
    }

    /// The halt as it is `now`.
    pub fn compose(&mut self, now: f32) -> Canvas {
        let mut scene = self.backdrop.clone();
        let moment = self.moment(now);
        let rail = self.size.1 - RAIL;
        let still = self.reduce_motion;
        let windows: Vec<(i32, i32)> = (0..self.windows()).map(window_origin).collect();
        let Some(train_x) = moment.train_x else {
            return scene;
        };
        let frame = match moment.going {
            Going::Rolling(rolled) => {
                let turn = TAU * DRIVING_WHEEL;
                let eighths = (rolled.max(0.0) / turn * f32::from(RUNNING_FRAMES)) as usize;
                eighths % usize::from(RUNNING_FRAMES)
            }
            Going::Standing if still => usize::from(RUNNING_FRAMES),
            Going::Standing => usize::from(RUNNING_FRAMES) + (now * PUFFS_PER_SECOND) as usize % 2,
        };
        let x = train_x.round() as i32;
        let top = rail - TRAIN_GROUND as i32;
        paint::blit(&mut scene, &self.train[frame], x, top);
        let own_top = top + HEADROOM;
        for (i, place) in moment.places.iter().enumerate() {
            let Place::Window(window) = *place else {
                continue;
            };
            let (wx, wy) = windows[window];
            let (left, wtop) = (x + wx, own_top + wy);
            let (width, height) = WINDOW_SIZE;
            let rider = &mut self.riders[i];
            let face = rider.face;
            let frame = rider.look.frame(
                Pose::Idle,
                if still { 0.0 } else { now + i as f32 * 0.37 },
                rider.facing_right,
            );
            // The face in the middle of the window; whatever does not fit is out of sight.
            let (fx0, fy0) = (left + width / 2 - face.0, wtop + height / 2 - face.1);
            for fy in 0..FRAME_SIZE as i32 {
                for fx in 0..FRAME_SIZE as i32 {
                    let (sx, sy) = (fx0 + fx, fy0 + fy);
                    if sx < left || sy < wtop || sx >= left + width || sy >= wtop + height {
                        continue;
                    }
                    let pixel = frame.get(fx, fy);
                    if pixel.a > 0 {
                        paint::put(&mut scene, sx, sy, pixel);
                    }
                }
            }
            glass(&mut scene, left, wtop);
        }
        scene
    }
}

/// How many coaches the train needs for `riders`, three to a coach, at least the two it has and
/// no more than the halt is long enough for.
fn coaches_for(riders: usize, width: i32) -> i32 {
    let wanted = (riders as i32 + 2) / 3;
    let room = 2 + (width - 24 - TRAIN_WIDTH as i32).max(0) / COACH;
    wanted.clamp(2, room.max(2))
}

fn train_length(coaches: i32) -> i32 {
    TRAIN_WIDTH as i32 + (coaches - 2) * COACH
}

/// A frame of the train with `coaches` coaches: the last coach, coupling and all, put on again at
/// the back as many times as it takes.
fn lengthened(frame: &Canvas, coaches: i32) -> Canvas {
    let extra = (coaches - 2).max(0);
    if extra == 0 {
        return frame.clone();
    }
    let mut out = Canvas::new(train_length(coaches) as u32, TRAIN_HEIGHT);
    for k in 0..extra {
        for y in 0..TRAIN_HEIGHT as i32 {
            for x in 0..COACH {
                out.set(k * COACH + x, y, frame.get(x, y));
            }
        }
    }
    for y in 0..TRAIN_HEIGHT as i32 {
        for x in 0..TRAIN_WIDTH as i32 {
            let pixel = frame.get(x, y);
            if pixel.a > 0 {
                out.set(extra * COACH + x, y, pixel);
            }
        }
    }
    out
}

/// A window's top left, in the train's own pixels below its headroom, counted from the back.
fn window_origin(window: usize) -> (i32, i32) {
    let coach = (window / 3) as i32;
    (coach * COACH + WINDOW_XS[window % 3], WINDOW_TOP)
}

/// The glass over a window, as `formiga-art` lays it: a faint sheen and a glint, so a face reads
/// as behind it.
fn glass(scene: &mut Canvas, x: i32, y: i32) {
    let (width, height) = WINDOW_SIZE;
    paint::rect(scene, x, y, width, height, rgba(0xa6c8d2, 30));
    paint::hline(scene, x, y, width, rgba(0x2e2a33, 110));
    for (dx, dy) in [(1, 4), (2, 3), (3, 2), (4, 1)] {
        paint::put(scene, x + width - 6 + dx, y + dy, rgba(0xffffff, 150));
    }
    paint::put(scene, x + 1, y + height - 2, rgba(0xffffff, 60));
}

/// The station board, on its posts at the back of the platform towards the engine's end.
fn sign_area((width, height): (i32, i32)) -> Area {
    let rail = height - RAIL;
    let w = 96;
    let x = (width * 4 / 5 - w / 2).min(width - w - 6);
    (x, rail - PLATFORM_BACK - 30, w, 13)
}

/// Everything that never moves: sky, hills, the meadow, the platform and the line.
fn backdrop((width, height): (i32, i32)) -> Canvas {
    let mut canvas = Canvas::new(width as u32, height as u32);
    let rail = height - RAIL;
    // However tall the picture, the country takes up its lower half or so, not just a strip
    // under a tall sky.
    let horizon = (rail - HORIZON).min(height * 11 / 20);
    sky(&mut canvas, width, horizon);
    clouds(&mut canvas, width, horizon);
    hills(&mut canvas, width, horizon);
    meadow(&mut canvas, width, horizon, rail - PLATFORM_BACK);
    fence(&mut canvas, width, rail - PLATFORM_BACK);
    platform(&mut canvas, width, rail);
    line(&mut canvas, width, height, rail);
    lamp(&mut canvas, width / 8, rail - PLATFORM_BACK + 6);
    board(&mut canvas, sign_area((width, height)));
    canvas
}

/// The sky, paler towards the hills, in bands dithered into each other.
fn sky(canvas: &mut Canvas, width: i32, horizon: i32) {
    const BANDS: f32 = 7.0;
    for y in 0..horizon {
        for x in 0..width {
            let t = y as f32 / horizon.max(1) as f32;
            let jitter = (paint::noise(x, y, 401) % 100) as f32 / 100.0 - 0.5;
            let band = ((t * BANDS + jitter * 0.7).floor() / BANDS).clamp(0.0, 1.0);
            canvas.set(x, y, paint::mix(SKY_HIGH, SKY_LOW, band));
        }
    }
}

/// A few fair-weather clouds, lit from the upper left and flat underneath.
fn clouds(canvas: &mut Canvas, width: i32, horizon: i32) {
    let count = (width / 110).max(2);
    for k in 0..count {
        let n = |salt| paint::noise(k, salt, 409);
        let cx = (k * width / count) + (n(1) % 60) as i32;
        let cy = 10 + (n(2) % (horizon / 3).max(1) as u32) as i32;
        let r = 5 + (n(3) % 5) as i32;
        let puffs = [
            (cx, cy, r * 2, r),
            (cx - r, cy + 1, r + 1, r - 1),
            (cx + r + 1, cy + 1, r, r - 2),
            (cx - r / 2, cy - r / 2, r, r - 1),
        ];
        let inside = |x: i32, y: i32| {
            y <= cy + r / 2
                && puffs.iter().any(|&(px, py, rx, ry)| {
                    let (dx, dy) = ((x - px) as f32 / rx as f32, (y - py) as f32 / ry as f32);
                    dx * dx + dy * dy <= 1.0
                })
        };
        for y in cy - r * 2..=cy + r {
            for x in cx - r * 3..=cx + r * 3 {
                if !inside(x, y) {
                    continue;
                }
                let tone = if !inside(x, y + 1) {
                    CLOUD.edge
                } else if !inside(x + 1, y + 1) || !inside(x, y + 2) {
                    CLOUD.shadow
                } else if !inside(x - 1, y - 1) {
                    CLOUD.shine
                } else {
                    CLOUD.base
                };
                paint::put(canvas, x, y, tone);
            }
        }
    }
}

/// Far blue-green hills, then the near green one with a few trees along its top.
fn hills(canvas: &mut Canvas, width: i32, horizon: i32) {
    let far = |x: f32| {
        horizon as f32 - 20.0 - 10.0 * (x * 0.021 + 1.3).sin() - 5.0 * (x * 0.057 + 0.4).sin()
    };
    let near =
        |x: f32| horizon as f32 - 7.0 - 6.0 * (x * 0.033 + 2.1).sin() - 2.5 * (x * 0.11).sin();
    for (ridge, ramp, salt) in [
        (&far as &dyn Fn(f32) -> f32, FAR_HILL, 421),
        (&near as &dyn Fn(f32) -> f32, GRASS, 431),
    ] {
        for x in 0..width {
            let top = ridge(x as f32).round() as i32;
            // Lit where the ground rises to the right, facing the light; shaded where it falls.
            let rising = ridge(x as f32 + 1.0) < ridge(x as f32 - 1.0);
            for y in top..horizon {
                let tone = if y == top {
                    ramp.edge
                } else if y == top + 1 {
                    if rising { ramp.light } else { ramp.shadow }
                } else if paint::chance(x, y, salt, 22) {
                    ramp.shadow
                } else if paint::chance(x, y, salt + 1, 12) {
                    ramp.light
                } else {
                    ramp.base
                };
                canvas.set(x, y, tone);
            }
        }
    }
    let trees = (width / 70).max(3);
    for k in 0..trees {
        let x = k * width / trees + 12 + (paint::noise(k, 3, 439) % 30) as i32;
        let foot = near(x as f32).round() as i32 + 1;
        tree(canvas, x, foot, 5 + (paint::noise(k, 4, 439) % 3) as i32);
    }
}

/// A round-headed tree, its trunk under a canopy lit on its upper left.
fn tree(canvas: &mut Canvas, x: i32, foot: i32, r: i32) {
    paint::rect(canvas, x, foot - r - 2, 2, r + 2, rgb(0x553b2a));
    paint::put(canvas, x, foot - r - 2, rgb(0x70503a));
    let (cx, cy) = (x + 1, foot - r * 2);
    for dy in -r..=r {
        for dx in -r - 1..=r + 1 {
            let (fx, fy) = (dx as f32 / (r + 1) as f32, dy as f32 / r as f32);
            let d = fx * fx + fy * fy;
            if d > 1.0 {
                continue;
            }
            let lit = fx + fy;
            let tone = if d > 0.78 && lit > 0.2 {
                LEAF.edge
            } else if lit < -0.7 {
                LEAF.shine
            } else if lit < -0.25 {
                LEAF.light
            } else if lit > 0.5 || paint::chance(cx + dx, cy + dy, 443, 30) {
                LEAF.shadow
            } else {
                LEAF.base
            };
            canvas.set(cx + dx, cy + dy, tone);
        }
    }
}

/// The meadow between the hills and the platform, with a scatter of flowers.
fn meadow(canvas: &mut Canvas, width: i32, horizon: i32, back: i32) {
    const BLOSSOMS: [Rgba; 4] = [rgb(0xf5d25e), rgb(0xfbf6ee), rgb(0xf19bb0), rgb(0xb79be0)];
    for y in horizon..back {
        for x in 0..width {
            let n = paint::noise(x, y, 449) % 100;
            let tone = match n {
                0..=9 => GRASS.shadow,
                10..=15 => GRASS.shine,
                16..=17 if y > horizon + 2 => BLOSSOMS[(paint::noise(x, y, 451) % 4) as usize],
                _ => GRASS.light,
            };
            canvas.set(x, y, tone);
        }
    }
}

/// A white picket fence along the back of the platform.
fn fence(canvas: &mut Canvas, width: i32, back: i32) {
    paint::hline(canvas, 0, back - 4, width, PICKET.shadow);
    paint::hline(canvas, 0, back - 5, width, PICKET.base);
    let mut x = 1;
    while x < width {
        paint::vline(canvas, x, back - 7, 7, PICKET.base);
        paint::put(canvas, x, back - 7, PICKET.shine);
        paint::vline(canvas, x + 1, back - 6, 6, PICKET.edge);
        x += 4;
    }
}

/// The platform: flagstones on top, a lit lip along its edge, brick down its face.
fn platform(canvas: &mut Canvas, width: i32, rail: i32) {
    let (back, edge, foot) = (
        rail - PLATFORM_BACK,
        rail - PLATFORM_EDGE,
        rail - PLATFORM_FOOT,
    );
    for y in back..edge {
        let row = (y - back) / 7;
        for x in 0..width {
            let (sx, sy) = ((x + row * 9).rem_euclid(18), (y - back) % 7);
            let tone = if sx == 0 || sy == 6 {
                STONE.shadow
            } else if sx == 1 || sy == 0 {
                STONE.light
            } else if paint::chance(x, y, 457, 20) {
                STONE.shadow
            } else {
                STONE.base
            };
            canvas.set(x, y, tone);
        }
    }
    paint::hline(canvas, 0, back, width, STONE.edge);
    paint::hline(canvas, 0, edge, width, STONE.shine);
    paint::hline(canvas, 0, edge + 1, width, STONE.light);
    for y in edge + 2..foot {
        let course = (y - edge - 2) / 3;
        for x in 0..width {
            let (bx, by) = ((x + course * 4).rem_euclid(8), (y - edge - 2) % 3);
            let tone = if by == 2 || bx == 0 {
                BRICK.shadow
            } else if paint::chance(x, y, 461, 30) {
                BRICK.light
            } else {
                BRICK.base
            };
            canvas.set(x, y, tone);
        }
    }
    // The lip's shadow on the brick below it.
    paint::hline(canvas, 0, edge + 2, width, BRICK.edge);
}

/// The ballast, the sleepers and the two rails.
fn line(canvas: &mut Canvas, width: i32, height: i32, rail: i32) {
    let foot = rail - PLATFORM_FOOT;
    for y in foot..height {
        for x in 0..width {
            let n = paint::noise(x, y, 467) % 100;
            let tone = match n {
                0..=17 => BALLAST.shadow,
                18..=27 => BALLAST.light,
                28..=30 => BALLAST.shine,
                _ => BALLAST.base,
            };
            canvas.set(x, y, tone);
        }
    }
    paint::hline(canvas, 0, foot, width, BALLAST.edge);
    let mut x = 2;
    while x < width {
        paint::rect(canvas, x, rail - 9, 5, 12, SLEEPER.base);
        paint::hline(canvas, x, rail - 9, 5, SLEEPER.light);
        paint::vline(canvas, x + 4, rail - 8, 11, SLEEPER.shadow);
        paint::hline(canvas, x, rail + 3, 5, SLEEPER.edge);
        x += 9;
    }
    for y in [rail - 8, rail] {
        paint::hline(canvas, 0, y, width, STEEL.shine);
        paint::hline(canvas, 0, y + 1, width, STEEL.base);
        paint::hline(canvas, 0, y + 2, width, STEEL.edge);
    }
}

/// An iron lamp post, its lamp lit faintly even by day.
fn lamp(canvas: &mut Canvas, x: i32, foot: i32) {
    let top = foot - 36;
    paint::rect(canvas, x, top + 6, 2, foot - top - 6, IRON.base);
    paint::vline(canvas, x, top + 6, foot - top - 6, IRON.light);
    paint::rect(canvas, x - 1, foot - 2, 4, 2, IRON.shadow);
    paint::bevel(canvas, x - 3, top, 8, 7, IRON);
    paint::rect(canvas, x - 2, top + 1, 6, 5, LAMPLIGHT);
    paint::put(canvas, x - 2, top + 1, rgb(0xffffff));
    paint::hline(canvas, x - 4, top - 1, 10, IRON.edge);
    paint::put(canvas, x + 1, top - 2, IRON.edge);
}

/// The station board on its two posts, left blank for the window to letter.
fn board(canvas: &mut Canvas, (x, y, w, h): Area) {
    for post in [x + 6, x + w - 8] {
        paint::rect(canvas, post, y + h, 2, 22, IRON.base);
        paint::vline(canvas, post, y + h, 22, IRON.light);
    }
    paint::bevel(canvas, x, y, w, h, BOARD);
    paint::rect(canvas, x + 2, y + 2, w - 4, h - 4, BOARD.base);
    paint::hline(canvas, x + 2, y + 2, w - 4, MAROON);
    paint::hline(canvas, x + 2, y + h - 3, w - 4, MAROON);
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_battle_contract::sample;

    fn looks(n: usize, reduce_motion: bool) -> Vec<Look> {
        let snapshot = sample::snapshot();
        snapshot
            .contenders
            .iter()
            .cycle()
            .take(n)
            .map(|c| Look::of(c, reduce_motion).expect("a look"))
            .collect()
    }

    #[test]
    fn everyone_rides_in_from_the_start_and_the_train_stops_at_the_platform() {
        let station = Station::new(looks(5, false), (384, 216), 0.0, false);
        assert!(
            station
                .moment(0.5)
                .places
                .iter()
                .all(|p| matches!(p, Place::Window(_)))
        );
        assert!(!station.stopped(1.0));
        assert!(station.stopped(PULL_IN));
        assert_eq!(station.moment(30.0).going, Going::Standing);
    }

    #[test]
    fn nobody_shares_a_window() {
        for n in 1..=12 {
            let station = Station::new(looks(n, true), (384, 216), 0.0, true);
            let mut seen: Vec<usize> = station
                .moment(0.0)
                .places
                .iter()
                .filter_map(|p| match p {
                    Place::Window(w) => Some(*w),
                    _ => None,
                })
                .collect();
            let count = seen.len();
            seen.dedup();
            assert_eq!(seen.len(), count, "{n} riders");
            assert!(seen.iter().all(|w| *w < station.windows()));
        }
    }

    #[test]
    fn a_coach_more_comes_for_every_three_past_six_while_the_halt_has_room() {
        assert_eq!(coaches_for(4, 400), 2);
        assert_eq!(coaches_for(7, 400), 3);
        assert_eq!(coaches_for(12, 600), 4);
        assert_eq!(coaches_for(30, 400), 4);
        let frame = TrainRenderer::render(&TrainLook { lit: false }, 0);
        assert_eq!(lengthened(&frame, 3).width(), TRAIN_WIDTH + COACH as u32);
    }

    #[test]
    fn with_reduced_motion_the_train_is_already_in_and_goes_at_once() {
        let mut station = Station::new(looks(3, true), (384, 216), 5.0, true);
        assert!(station.stopped(5.0));
        station.set_off(6.0);
        assert!(station.seen_off(6.0));
    }

    #[test]
    fn a_click_while_it_comes_in_brings_it_to_the_platform() {
        let mut station = Station::new(looks(4, false), (384, 216), 0.0, false);
        station.skip(1.0);
        assert!(station.stopped(1.0));
        station.set_off(2.0);
        assert!(!station.seen_off(2.5));
        assert!(station.seen_off(2.0 + SEEN_OFF + 0.01));
    }

    #[test]
    fn the_train_is_heard_coming_stopping_and_going() {
        let mut station = Station::new(looks(3, false), (384, 216), 0.0, false);
        let all = station.heard(-1.0, 30.0);
        assert_eq!(all, vec![Heard::Whistle, Heard::Brakes]);
        station.set_off(30.0);
        assert_eq!(station.heard(29.5, 30.5), vec![Heard::Away]);
        assert!(station.heard(31.0, 32.0).is_empty());
    }

    #[test]
    fn the_train_moves_the_picture_and_stands_still_once_in() {
        let mut station = Station::new(looks(3, false), (384, 216), 0.0, false);
        assert_ne!(station.compose(1.0), station.compose(1.5));
        station.skip(2.0);
        assert!(station.moment(9.0).train_x.is_some());
    }
}
