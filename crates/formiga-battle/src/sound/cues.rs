//! Every effect in Battle, as a cue: what happened, and the sound it makes. Each is a cheerful
//! placeholder made in code from the synthesiser's parts, cartoon rather than real (a hit is a
//! thump with a spring in it, a knockout a boom and a few tweeting birds), until real audio
//! replaces it.
//!
//! A cue played again and again (a step, a pow, the clock's tock) comes out a little different
//! each time, but the `n`th time is always the same, so every sound can be listened to in a test.

use super::synth::{
    BAR, BLOCK, CHIME, DcBlock, METAL, Noise, Phase, Smooth, Struck, Svf, ease, hz as note_hz,
    reed, sine, strike, swell, triangle,
};

/// Something that makes a sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cue {
    // The window's own controls, used sparingly; the menus' too.
    /// The pointer coming onto a button or a card: the softest tick.
    Hover,
    /// A button pressed.
    Click,
    /// A choice confirmed: up and bright, unlike `Back`.
    Confirm,
    /// Going back, or a choice put away: down and soft.
    Back,
    /// A page of the journal turned.
    PageTurn,
    /// The train pulling in: its whistle, its chuffs slowing, the brakes.
    TrainArrive,
    /// The train pulling out: the whistle, and its chuffs gathering pace.
    TrainDepart,
    /// A companion saying it is ready: a little chirp up.
    Ready,
    /// A companion chosen for the team, or a page opened.
    Pick,
    /// A companion let go from the team, or a page closed.
    Unpick,
    /// One of the Handler's three picked out on the board.
    Select,
    /// An order given.
    Order,
    /// An order or a choice let go.
    Cancel,
    /// The orders locked in.
    Lock,
    /// The turn begins to play out.
    Reveal,
    /// The timer's last seconds.
    Tick,
    /// The referee's whistle: go!
    Whistle,
    /// A match about to begin: a quick run up the marimba, before the ready-up.
    Begin,

    // The board, as the turn plays out.
    /// A soft footstep.
    Step,
    /// Pressed with nowhere to go: knocked off its feet where it stands.
    Pow,
    /// Pressed and pushed a tile on: a thump and a whoosh away.
    Shove,
    /// A PRESS turned away by someone holding firm.
    Clang,
    /// Two PRESSes meeting: two heads knocking.
    Bonk,
    /// A hold broken from two sides.
    Wham,
    /// A TRICK round a holder's side or back: a tiptoe and a swish.
    Sneak,
    /// A TRICK seen coming from the front: "uh-uh".
    Read,
    /// Something snatched by a presser: a quick fwip and a pluck.
    Snatch,
    /// Knocked out, dizzy.
    Boom,
    /// Something that came to nothing: a whiff of air.
    Miss,
    /// Two wanting the same tile.
    Boink,
    /// Hanging back.
    Hmm,
    /// The lever yanked.
    Clunk,
    /// A gate swinging.
    Creak,
    /// The lever guarded: shooed off.
    Shoo,
    /// The beacon warming.
    Glow,
    /// The beacon held by both.
    Contested,
    /// The beacon lit.
    Fwoosh,

    // The map's own events, and things found on the ground.
    /// The pipes gurgling: the sprinkler is coming, or a tap was turned.
    Gurgle,
    /// The sprinkler going off: a sploosh and a patter of drops.
    Splash,
    /// Something glinting, or turning up, on the ground.
    Glint,
    /// Something picked up: a bright little pluck up.
    Pickup,
    /// Something dropped, or a twig falling over: a soft plop.
    Plop,
    /// A dandelion puff blown: a soft, airy poof.
    Poof,
    /// A twig wedged into the ground: a woody knock.
    Snap,
    /// A leaf umbrella opening: a quick fwip.
    Fwip,
    /// Something used that came to nothing: a deflating sputter.
    Fizzle,
    /// A wave rolling in and washing over: a low rush of surf.
    Wave,
    /// The big shell blown: a low, hollow toot.
    Toot,
    /// An oak shaken: a rustle of leaves.
    Rustle,
    /// Acorns coming down: a clatter of little wooden knocks.
    Acorns,
    /// The pebble bell rung.
    Ding,
    /// Honey: a sticky squelch.
    Squelch,

    // The end of a match.
    /// A win.
    Fanfare,
    /// A loss: the sad trombone, kindly.
    Womp,
    /// A draw: a shrug.
    Shrug,
}

impl Cue {
    pub const ALL: [Cue; 55] = [
        Cue::Hover,
        Cue::Click,
        Cue::Confirm,
        Cue::Back,
        Cue::PageTurn,
        Cue::TrainArrive,
        Cue::TrainDepart,
        Cue::Ready,
        Cue::Pick,
        Cue::Unpick,
        Cue::Select,
        Cue::Order,
        Cue::Cancel,
        Cue::Lock,
        Cue::Reveal,
        Cue::Tick,
        Cue::Whistle,
        Cue::Begin,
        Cue::Step,
        Cue::Pow,
        Cue::Shove,
        Cue::Clang,
        Cue::Bonk,
        Cue::Wham,
        Cue::Sneak,
        Cue::Read,
        Cue::Snatch,
        Cue::Boom,
        Cue::Miss,
        Cue::Boink,
        Cue::Hmm,
        Cue::Clunk,
        Cue::Creak,
        Cue::Shoo,
        Cue::Glow,
        Cue::Contested,
        Cue::Fwoosh,
        Cue::Gurgle,
        Cue::Splash,
        Cue::Glint,
        Cue::Pickup,
        Cue::Plop,
        Cue::Poof,
        Cue::Snap,
        Cue::Fwip,
        Cue::Fizzle,
        Cue::Wave,
        Cue::Toot,
        Cue::Rustle,
        Cue::Acorns,
        Cue::Ding,
        Cue::Squelch,
        Cue::Fanfare,
        Cue::Womp,
        Cue::Shrug,
    ];

    /// Its name, for a file of it.
    pub fn name(self) -> &'static str {
        match self {
            Cue::Hover => "hover",
            Cue::Click => "click",
            Cue::Confirm => "confirm",
            Cue::Back => "back",
            Cue::PageTurn => "page-turn",
            Cue::TrainArrive => "train-arrive",
            Cue::TrainDepart => "train-depart",
            Cue::Ready => "ready",
            Cue::Pick => "pick",
            Cue::Unpick => "unpick",
            Cue::Select => "select",
            Cue::Order => "order",
            Cue::Cancel => "cancel",
            Cue::Lock => "lock",
            Cue::Reveal => "reveal",
            Cue::Tick => "tick",
            Cue::Whistle => "whistle",
            Cue::Begin => "begin",
            Cue::Step => "step",
            Cue::Pow => "pow",
            Cue::Shove => "shove",
            Cue::Clang => "clang",
            Cue::Bonk => "bonk",
            Cue::Wham => "wham",
            Cue::Sneak => "sneak",
            Cue::Read => "read",
            Cue::Snatch => "snatch",
            Cue::Boom => "boom",
            Cue::Miss => "miss",
            Cue::Boink => "boink",
            Cue::Hmm => "hmm",
            Cue::Clunk => "clunk",
            Cue::Creak => "creak",
            Cue::Shoo => "shoo",
            Cue::Glow => "glow",
            Cue::Contested => "contested",
            Cue::Fwoosh => "fwoosh",
            Cue::Gurgle => "gurgle",
            Cue::Splash => "splash",
            Cue::Glint => "glint",
            Cue::Pickup => "pickup",
            Cue::Plop => "plop",
            Cue::Poof => "poof",
            Cue::Snap => "snap",
            Cue::Fwip => "fwip",
            Cue::Fizzle => "fizzle",
            Cue::Wave => "wave",
            Cue::Toot => "toot",
            Cue::Rustle => "rustle",
            Cue::Acorns => "acorns",
            Cue::Ding => "ding",
            Cue::Squelch => "squelch",
            Cue::Fanfare => "fanfare",
            Cue::Womp => "womp",
            Cue::Shrug => "shrug",
        }
    }

    /// Its place in `ALL`.
    pub fn index(self) -> usize {
        Cue::ALL
            .iter()
            .position(|cue| *cue == self)
            .unwrap_or_default()
    }

    /// The least time between two of it, so a crowd of them is heard as a patter and not a roar.
    pub fn spacing(self) -> f32 {
        match self {
            Cue::Step => 0.09,
            Cue::Hover => 0.04,
            Cue::Click | Cue::Tick => 0.05,
            Cue::Pow | Cue::Shove | Cue::Clang | Cue::Bonk | Cue::Boink => 0.08,
            Cue::Glow | Cue::Hmm | Cue::Glint | Cue::Plop => 0.15,
            _ => 0.0,
        }
    }

    /// How loud it is beside the others: the controls and the footsteps are kept small.
    fn level(self) -> f32 {
        match self {
            Cue::Hover => 0.12,
            Cue::Step => 0.18,
            Cue::Click | Cue::Tick => 0.3,
            Cue::Select
            | Cue::Order
            | Cue::Cancel
            | Cue::Pick
            | Cue::Unpick
            | Cue::Back
            | Cue::Ready
            | Cue::PageTurn => 0.38,
            Cue::Confirm => 0.45,
            Cue::Hmm | Cue::Glow => 0.4,
            _ => 0.6,
        }
    }

    /// The sound, the `variation`th time it is played, made at `rate` samples a second.
    pub fn voice(self, variation: u32, rate: f32) -> Voice {
        let seed = variation
            .wrapping_mul(0x9e37_79b9)
            .wrapping_add(self.index() as u32 * 7919);
        // A little higher or lower each time, in five steps either side of the note.
        let vary = 1.0 + (seed % 9) as f32 * 0.012 - 0.048;
        let voice = match self {
            Cue::Hover => struck(rate, BLOCK, &[(0.0, 2600.0 * vary, 1.0)], 0.05),
            Cue::Click => struck(rate, BLOCK, &[(0.0, 1900.0 * vary, 1.0)], 0.08),
            Cue::Confirm => struck(
                rate,
                BAR,
                &[
                    (0.0, n(79.0), 0.7),
                    (0.06, n(84.0), 0.8),
                    (0.12, n(91.0), 1.0),
                ],
                0.7,
            ),
            Cue::Back => struck(rate, BAR, &[(0.0, n(76.0), 0.8), (0.07, n(69.0), 0.7)], 0.5),
            Cue::PageTurn => page_turn(rate, seed),
            Cue::TrainArrive => train(rate, seed, true),
            Cue::TrainDepart => train(rate, seed, false),
            Cue::Ready => glide(rate, 660.0 * vary, 990.0 * vary, 0.05, 0.09, 0.22),
            Cue::Pick => struck(rate, BAR, &[(0.0, n(84.0), 0.8), (0.07, n(91.0), 1.0)], 0.5),
            Cue::Unpick => struck(rate, BAR, &[(0.0, n(79.0), 0.9), (0.07, n(72.0), 0.8)], 0.5),
            Cue::Select => glide(rate, 520.0, 780.0, 0.07, 0.1, 0.3),
            Cue::Order => struck(
                rate,
                CHIME,
                &[(0.0, n(79.0), 0.8), (0.05, n(86.0), 0.6)],
                0.6,
            ),
            Cue::Cancel => glide(rate, 700.0, 330.0, 0.12, 0.12, 0.3),
            Cue::Lock => lock(rate, seed),
            Cue::Reveal => slide_whistle(rate, seed, 620.0, 1500.0, 0.36),
            Cue::Tick => struck(rate, BLOCK, &[(0.0, 1250.0 * vary, 1.0)], 0.1),
            Cue::Whistle => whistle(rate, seed),
            Cue::Begin => struck(
                rate,
                BAR,
                &[
                    (0.0, n(72.0), 0.7),
                    (0.06, n(76.0), 0.75),
                    (0.12, n(79.0), 0.8),
                    (0.18, n(84.0), 0.9),
                    (0.3, n(88.0), 0.85),
                ],
                1.0,
            )
            .louder(0.85),
            Cue::Step => step(rate, seed, vary),
            Cue::Pow => pow(rate, seed, vary),
            Cue::Shove => shove(rate, seed, vary),
            Cue::Clang => clang(rate, seed, vary),
            Cue::Bonk => bonk(rate, vary),
            Cue::Wham => wham(rate, seed),
            Cue::Sneak => sneak(rate, seed, vary),
            Cue::Read => horn(rate, &[(0.0, 0.09, 74.0), (0.13, 0.16, 69.0)], 2400.0, 0.6),
            Cue::Snatch => snatch(rate, seed, vary),
            Cue::Boom => boom(rate, seed),
            Cue::Miss => miss(rate, seed),
            Cue::Boink => boink(rate, vary),
            Cue::Hmm => horn(rate, &[(0.0, 0.16, 64.0), (0.2, 0.28, 67.0)], 1500.0, 0.5),
            Cue::Clunk => clunk(rate),
            Cue::Creak => creak(rate, seed),
            Cue::Shoo => whoosh(rate, seed, 0.32, 3200.0),
            Cue::Glow => struck(
                rate,
                CHIME,
                &[
                    (0.0, n(88.0) * vary, 0.6),
                    (0.06, n(91.0) * vary, 0.5),
                    (0.12, n(95.0) * vary, 0.45),
                ],
                0.9,
            ),
            Cue::Contested => horn(rate, &[(0.0, 0.14, 69.0), (0.19, 0.3, 65.0)], 1800.0, 0.6),
            Cue::Fwoosh => fwoosh(rate, seed),
            Cue::Gurgle => gurgle(rate, seed),
            Cue::Splash => splash(rate, seed),
            Cue::Glint => struck(
                rate,
                CHIME,
                &[(0.0, n(96.0) * vary, 0.6), (0.07, n(100.0) * vary, 0.5)],
                0.7,
            )
            .louder(0.7),
            Cue::Pickup => struck(
                rate,
                BAR,
                &[
                    (0.0, n(79.0), 0.7),
                    (0.05, n(83.0), 0.75),
                    (0.1, n(86.0), 0.8),
                    (0.15, n(91.0), 0.9),
                ],
                0.6,
            ),
            Cue::Plop => glide(rate, 420.0 * vary, 160.0 * vary, 0.06, 0.07, 0.2),
            Cue::Poof => whoosh(rate, seed, 0.4, 1800.0),
            Cue::Snap => struck(
                rate,
                BLOCK,
                &[(0.0, 520.0 * vary, 1.0), (0.05, 700.0 * vary, 0.6)],
                0.25,
            ),
            Cue::Fwip => fwip(rate, seed),
            Cue::Fizzle => fizzle(rate, seed),
            Cue::Wave => surf(rate, seed),
            Cue::Toot => horn(rate, &[(0.0, 0.5, 50.0), (0.0, 0.5, 57.0)], 900.0, 0.55),
            Cue::Rustle => rustle(rate, seed),
            Cue::Acorns => struck(
                rate,
                BLOCK,
                &[
                    (0.0, 760.0 * vary, 1.0),
                    (0.07, 900.0 * vary, 0.8),
                    (0.12, 680.0 * vary, 0.9),
                    (0.2, 820.0 * vary, 0.7),
                    (0.26, 720.0 * vary, 0.6),
                ],
                0.45,
            ),
            Cue::Ding => {
                struck(rate, super::synth::METAL, &[(0.0, 1320.0 * vary, 1.0)], 1.0).louder(0.6)
            }
            Cue::Squelch => squelch(rate, seed),
            Cue::Fanfare => fanfare(rate),
            Cue::Womp => womp(rate),
            Cue::Shrug => struck(rate, BAR, &[(0.0, n(76.0), 0.9), (0.18, n(72.0), 0.8)], 0.7),
        };
        voice.louder(self.level())
    }
}

fn n(note: f32) -> f32 {
    note_hz(note)
}

/// How long the last part of every sound takes to fade to nothing, so none ends on a click.
const FADE_SECS: f32 = 0.006;

/// One sound playing: made a sample at a time until it is over.
pub struct Voice {
    rate: f32,
    made: usize,
    length: usize,
    loudness: f32,
    dc: DcBlock,
    make: Box<dyn FnMut(f32) -> f32 + Send>,
}

impl Voice {
    fn new(rate: f32, seconds: f32, make: impl FnMut(f32) -> f32 + Send + 'static) -> Self {
        Self {
            rate,
            made: 0,
            length: (seconds * rate) as usize,
            loudness: 1.0,
            dc: DcBlock::new(rate),
            make: Box::new(make),
        }
    }

    /// The same sound, `by` as loud.
    pub fn louder(mut self, by: f32) -> Self {
        self.loudness *= by.max(0.0);
        self
    }

    /// How long it lasts, in seconds.
    #[cfg(test)]
    pub fn seconds(&self) -> f32 {
        self.length as f32 / self.rate
    }
}

impl Iterator for Voice {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.made >= self.length {
            return None;
        }
        let t = self.made as f32 / self.rate;
        let left = (self.length - self.made) as f32 / self.rate;
        self.made += 1;
        let fade = (left / FADE_SECS).min(1.0);
        Some(self.dc.run((self.make)(t)) * fade * self.loudness)
    }
}

/// Notes struck on a bar, a chime or a block.
fn struck(
    rate: f32,
    partials: super::synth::Partials,
    notes: &[(f32, f32, f32)],
    seconds: f32,
) -> Voice {
    let mut bar = Struck::new(partials, notes);
    Voice::new(rate, seconds, move |t| bar.run(t, rate) * 0.6)
}

/// A pure tone sliding from one pitch to another: a boop up, or a bloop down.
fn glide(rate: f32, from: f32, to: f32, over: f32, decay: f32, seconds: f32) -> Voice {
    let mut phase = Phase::default();
    Voice::new(rate, seconds, move |t| {
        let hz = from + (to - from) * ease(t / over);
        sine(phase.tick(hz, rate)) * strike(t, 0.004, decay) * 0.8
    })
}

/// A soft thump: a low sine dropping in pitch, like something landing.
struct Thump {
    phase: Phase,
}

impl Thump {
    fn new() -> Self {
        Self {
            phase: Phase::default(),
        }
    }

    fn run(&mut self, t: f32, from: f32, to: f32, rate: f32) -> f32 {
        let hz = to + (from - to) * (-t / 0.03).exp();
        sine(self.phase.tick(hz, rate))
    }
}

/// Noise through a band-pass, swept wherever each sound wants it.
struct Hiss {
    noise: Noise,
    filter: Svf,
}

impl Hiss {
    fn new(seed: u32, rate: f32) -> Self {
        Self {
            noise: Noise::new(seed),
            filter: Svf::new(1000.0, 1.0, rate),
        }
    }

    fn band(&mut self, hz: f32, q: f32) -> f32 {
        self.filter.tune(hz, q);
        self.filter.run(self.noise.next()).band
    }

    fn low(&mut self, hz: f32) -> f32 {
        self.filter.tune(hz, 0.7);
        self.filter.run(self.noise.next()).low
    }
}

/// A slide whistle: up (or down) in one breath, with a wobble in it.
fn slide_whistle(rate: f32, seed: u32, from: f32, to: f32, over: f32) -> Voice {
    let mut phase = Phase::default();
    let mut breath = Hiss::new(seed, rate);
    Voice::new(rate, over + 0.12, move |t| {
        let hz = from + (to - from) * ease(t / over);
        let wobble = 1.0 + 0.012 * sine(t * 9.0);
        let tone = sine(phase.tick(hz * wobble, rate));
        let air = breath.band(hz, 2.0) * 0.25;
        (tone + air) * swell(t, 0.03, over, 0.1) * 0.5
    })
}

fn lock(rate: f32, seed: u32) -> Voice {
    let mut roll = Hiss::new(seed, rate);
    let mut bar = Struck::new(BAR, &[(0.26, n(72.0), 0.9), (0.34, n(84.0), 1.0)]);
    Voice::new(rate, 0.9, move |t| {
        // A short drum roll: a snare's rattle, beaten thirty times a second, then the ta-da.
        let beat = (t * 30.0).fract();
        let snare = roll.band(3200.0, 0.8) * strike(beat, 0.002, 0.012) * swell(t, 0.1, 0.22, 0.03);
        snare * 0.7 + bar.run(t, rate) * 0.6
    })
}

/// The referee's whistle: a trill, the pea rattling in it.
fn whistle(rate: f32, seed: u32) -> Voice {
    let mut phase = Phase::default();
    let mut breath = Hiss::new(seed, rate);
    Voice::new(rate, 0.55, move |t| {
        let trill = 1.0 + 0.06 * sine(t * 38.0);
        let hz = 2700.0 * trill;
        let tone = sine(phase.tick(hz, rate)) * 0.7 + breath.band(hz, 4.0) * 0.4;
        tone * swell(t, 0.02, 0.45, 0.08) * 0.45
    })
}

fn step(rate: f32, seed: u32, vary: f32) -> Voice {
    let mut thump = Thump::new();
    let mut scuff = Hiss::new(seed, rate);
    Voice::new(rate, 0.12, move |t| {
        thump.run(t, 220.0 * vary, 95.0 * vary, rate) * strike(t, 0.002, 0.035)
            + scuff.low(1800.0) * strike(t, 0.001, 0.02) * 0.6
    })
}

/// Caught on the move: a punch of a thump, a crack, and a spring that boings down after it.
fn pow(rate: f32, seed: u32, vary: f32) -> Voice {
    let mut thump = Thump::new();
    let mut crack = Hiss::new(seed, rate);
    let mut spring = Phase::default();
    Voice::new(rate, 0.5, move |t| {
        let body = thump.run(t, 170.0 * vary, 55.0, rate) * strike(t, 0.002, 0.11);
        let snap = crack.low(2600.0) * strike(t, 0.001, 0.04) * 1.2;
        let since = t - 0.03;
        let boing = if since > 0.0 {
            let hz = (320.0 + 700.0 * (-since / 0.07).exp()) * vary;
            sine(spring.tick(hz * (1.0 + 0.05 * sine(since * 22.0)), rate))
                * strike(since, 0.005, 0.16)
                * 0.35
        } else {
            0.0
        };
        (body + snap + boing) * 0.85
    })
}

/// Pushed a tile on: a thump, and a whoosh of air as it goes.
fn shove(rate: f32, seed: u32, vary: f32) -> Voice {
    let mut thump = Thump::new();
    let mut air = Hiss::new(seed, rate);
    Voice::new(rate, 0.45, move |t| {
        let body = thump.run(t, 160.0 * vary, 60.0, rate) * strike(t, 0.002, 0.08);
        let since = t - 0.03;
        let rush = if since > 0.0 {
            let hz = 2600.0 - 1600.0 * ease(since / 0.3);
            air.band(hz, 1.4) * swell(since, 0.02, 0.18, 0.12) * 1.1
        } else {
            0.0
        };
        (body + rush) * 0.85
    })
}

/// Two PRESSes meeting: two wooden knocks close together, and a little spring after them.
fn bonk(rate: f32, vary: f32) -> Voice {
    let mut heads = Struck::new(
        BLOCK,
        &[(0.0, 360.0 * vary, 1.0), (0.06, 300.0 * vary, 0.9)],
    );
    let mut thump = Thump::new();
    let mut spring = Phase::default();
    Voice::new(rate, 0.5, move |t| {
        let knock = heads.run(t, rate) * 0.7;
        let body = thump.run(t, 200.0 * vary, 80.0, rate) * strike(t, 0.002, 0.05) * 0.6;
        let since = t - 0.1;
        let boing = if since > 0.0 {
            let hz = 520.0 * vary * (1.0 + 0.4 * (-since / 0.05).exp());
            triangle(spring.tick(hz * (1.0 + 0.1 * sine(since * 26.0)), rate))
                * strike(since, 0.004, 0.12)
                * 0.3
        } else {
            0.0
        };
        knock + body + boing
    })
}

/// A TRICK: three tiptoes, high and light, then a swish round the side.
fn sneak(rate: f32, seed: u32, vary: f32) -> Voice {
    let mut toes = Struck::new(
        BLOCK,
        &[
            (0.0, 1500.0 * vary, 0.5),
            (0.07, 1300.0 * vary, 0.5),
            (0.14, 1700.0 * vary, 0.6),
        ],
    );
    let mut swish = whoosh(rate, seed, 0.22, 3600.0);
    Voice::new(rate, 0.5, move |t| {
        toes.run(t, rate) * 0.6
            + if t > 0.2 {
                swish.next().unwrap_or(0.0) * 0.9
            } else {
                0.0
            }
    })
}

/// Something snatched: a quick fwip of air up, and a pluck as it lands with the presser.
fn snatch(rate: f32, seed: u32, vary: f32) -> Voice {
    let mut air = Hiss::new(seed, rate);
    let mut pluck = Struck::new(BAR, &[(0.12, n(88.0) * vary, 0.9)]);
    Voice::new(rate, 0.5, move |t| {
        let hz = 1200.0 + 3000.0 * ease(t / 0.12);
        air.band(hz, 2.0) * swell(t, 0.01, 0.08, 0.04) * 0.9 + pluck.run(t, rate) * 0.6
    })
}

/// Turned away: a pan lid struck, ringing.
fn clang(rate: f32, seed: u32, vary: f32) -> Voice {
    let mut lid = Struck::new(METAL, &[(0.0, 540.0 * vary, 1.0)]);
    let mut tap = Hiss::new(seed, rate);
    Voice::new(rate, 1.1, move |t| {
        lid.run(t, rate) * 0.45 + tap.band(4000.0, 1.0) * strike(t, 0.001, 0.015) * 0.6
    })
}

/// A hold broken from two sides: a bigger thump, and a slide whistle down as it goes.
fn wham(rate: f32, seed: u32) -> Voice {
    let mut thump = Thump::new();
    let mut crack = Hiss::new(seed, rate);
    let mut whistle = Phase::default();
    Voice::new(rate, 0.75, move |t| {
        let body = thump.run(t, 140.0, 40.0, rate) * strike(t, 0.002, 0.2);
        let snap = crack.low(1800.0) * strike(t, 0.001, 0.06);
        let since = t - 0.06;
        let slide = if since > 0.0 {
            let hz = 1300.0 - 950.0 * ease(since / 0.45);
            sine(whistle.tick(hz, rate)) * swell(since, 0.03, 0.45, 0.15) * 0.3
        } else {
            0.0
        };
        body + snap + slide
    })
}

/// Knocked out: a soft boom, a wobble, and birds tweeting round its head.
fn boom(rate: f32, seed: u32) -> Voice {
    let mut thump = Thump::new();
    let mut rumble = Hiss::new(seed, rate);
    let mut birds = [Phase::default(); 3];
    let tweets = [0.38, 0.58, 0.78];
    Voice::new(rate, 1.25, move |t| {
        let body = thump.run(t, 110.0, 32.0, rate) * strike(t, 0.004, 0.45);
        let roar = rumble.low(420.0) * strike(t, 0.002, 0.25) * 1.4;
        let mut tweet = 0.0;
        for (bird, at) in birds.iter_mut().zip(tweets) {
            let since = t - at;
            if (0.0..0.12).contains(&since) {
                let hz = 2300.0 + 1200.0 * (since / 0.12);
                tweet += sine(bird.tick(hz, rate)) * swell(since, 0.01, 0.08, 0.04) * 0.22;
            }
        }
        body + roar + tweet
    })
}

/// A miss: a quick whiff of air, and a little rustle of the two tangled up.
fn miss(rate: f32, seed: u32) -> Voice {
    let mut whiff = whoosh(rate, seed, 0.18, 4200.0);
    let mut rustle = scuffle(rate, seed ^ 0x77);
    Voice::new(rate, 0.6, move |t| {
        whiff.next().unwrap_or(0.0) * 0.9
            + if t > 0.12 {
                rustle.next().unwrap_or(0.0) * 0.6
            } else {
                0.0
            }
    })
}

/// A page of the journal turned: a papery flick, rising as it lifts and settling as it lands.
fn page_turn(rate: f32, seed: u32) -> Voice {
    let mut paper = Hiss::new(seed, rate);
    let mut grain = Noise::new(seed ^ 0x33);
    Voice::new(rate, 0.42, move |t| {
        let hz = 1500.0 + 2500.0 * sine((t / 0.32).min(1.0) * 0.5);
        let crackle = if grain.unit() > 0.985 { 2.0 } else { 1.0 };
        let lift = swell(t, 0.08, 0.2, 0.12);
        let land = strike(t - 0.3, 0.003, 0.03) * 0.8;
        paper.band(hz, 0.8) * (lift + land) * crackle * 0.9
    })
}

/// The train: two notes of its whistle together, then its chuffs, slowing as it pulls in with a
/// hiss of brakes, or gathering pace as it pulls out.
fn train(rate: f32, seed: u32, arriving: bool) -> Voice {
    let mut whistle = horn(rate, &[(0.0, 0.5, 74.0)], 3200.0, 0.4);
    let mut whistle_low = horn(rate, &[(0.0, 0.5, 70.0)], 3200.0, 0.35);
    let mut steam = Hiss::new(seed, rate);
    let mut brakes = Hiss::new(seed ^ 0x99, rate);
    let chuffs: Vec<f32> = (0..8)
        .scan(0.55f32, |at, k| {
            let gap = if arriving {
                0.12 + 0.04 * k as f32
            } else {
                0.36 - 0.035 * k as f32
            };
            *at += gap;
            Some(*at)
        })
        .collect();
    let end = chuffs.last().copied().unwrap_or(1.0) + 0.6;
    Voice::new(rate, end + if arriving { 0.6 } else { 0.0 }, move |t| {
        let mut out = whistle.next().unwrap_or(0.0) + whistle_low.next().unwrap_or(0.0);
        let puff: f32 = chuffs.iter().map(|at| strike(t - at, 0.01, 0.06)).sum();
        out += steam.band(900.0, 0.9) * puff.min(1.0) * 0.8;
        if arriving {
            out += brakes.band(5200.0, 1.5) * swell(t - end + 0.2, 0.05, 0.45, 0.3) * 0.5;
        }
        out
    })
}

/// A scuffle: a quick rustle of bursts, like two rolling about in leaves.
fn scuffle(rate: f32, seed: u32) -> Voice {
    let mut rustle = Hiss::new(seed, rate);
    let mut pick = Noise::new(seed ^ 0x55);
    let bursts: Vec<(f32, f32)> = (0..6)
        .map(|k| {
            (
                k as f32 * 0.075 + pick.unit() * 0.03,
                900.0 + pick.unit() * 1800.0,
            )
        })
        .collect();
    Voice::new(rate, 0.6, move |t| {
        let mut out = 0.0;
        let mut centre = 1200.0;
        for &(at, hz) in &bursts {
            let since = t - at;
            if since >= 0.0 {
                out += strike(since, 0.004, 0.03);
                if since < 0.08 {
                    centre = hz;
                }
            }
        }
        rustle.band(centre, 1.2) * out.min(1.2) * 1.8
    })
}

/// A bump: a spring going boing.
fn boink(rate: f32, vary: f32) -> Voice {
    let mut phase = Phase::default();
    Voice::new(rate, 0.42, move |t| {
        let hz = 240.0 * vary * (1.0 + 0.6 * (-t / 0.06).exp());
        let wobble = 1.0 + 0.12 * sine(t * 24.0) * (-t / 0.2).exp();
        triangle(phase.tick(hz * wobble, rate)) * strike(t, 0.004, 0.14) * 0.7
    })
}

/// Notes on a soft reed, each a held note with a little vibrato: a hum, a horn, a trombone.
/// `notes` are (start, length, MIDI note); `bright` is how far the reed's buzz is let through.
fn horn(rate: f32, notes: &[(f32, f32, f32)], bright: f32, gain: f32) -> Voice {
    let notes = notes.to_vec();
    let seconds = notes
        .iter()
        .map(|(at, held, _)| at + held)
        .fold(0.0, f32::max)
        + 0.15;
    let mut phases = vec![Phase::default(); notes.len()];
    let mut tone = Svf::new(bright, 0.7, rate);
    Voice::new(rate, seconds, move |t| {
        let mut out = 0.0;
        for (phase, &(at, held, note)) in phases.iter_mut().zip(&notes) {
            let since = t - at;
            if since < 0.0 || since > held + 0.12 {
                continue;
            }
            let vibrato = 1.0 + 0.01 * sine(since * 5.5) * ease((since - 0.12) / 0.2);
            out += reed(phase.tick(n(note) * vibrato, rate)) * swell(since, 0.025, held, 0.1);
        }
        tone.run(out).low * gain
    })
}

/// The lever yanked: a wooden clunk and its ratchet clicking.
fn clunk(rate: f32) -> Voice {
    let mut thump = Thump::new();
    let mut knock = Struck::new(BLOCK, &[(0.0, 320.0, 1.0)]);
    let mut ratchet = Struck::new(
        BLOCK,
        &[
            (0.08, 2100.0, 0.5),
            (0.11, 2000.0, 0.45),
            (0.14, 2150.0, 0.4),
            (0.17, 2050.0, 0.35),
        ],
    );
    Voice::new(rate, 0.5, move |t| {
        (thump.run(t, 200.0, 80.0, rate) * strike(t, 0.002, 0.07)
            + knock.run(t, rate) * 0.9
            + ratchet.run(t, rate) * 0.5)
            * 0.55
    })
}

/// A gate swinging: a squeaky hinge, its pitch wandering as it goes.
fn creak(rate: f32, seed: u32) -> Voice {
    let mut phase = Phase::default();
    let mut wander = Noise::new(seed);
    let mut drift = Smooth::new(6.0, rate);
    let mut body = Svf::new(900.0, 6.0, rate);
    Voice::new(rate, 0.85, move |t| {
        let hz = 105.0 + 35.0 * sine(t * 1.6) + 120.0 * drift.run(wander.next());
        let rasp = phase.tick(hz.max(40.0), rate) * 2.0 - 1.0;
        body.tune(850.0 + 300.0 * sine(t * 2.3), 6.0);
        body.run(rasp).band * swell(t, 0.08, 0.68, 0.12) * 1.8
    })
}

/// A whoosh: air through a sweep, up and back.
fn whoosh(rate: f32, seed: u32, seconds: f32, top: f32) -> Voice {
    let mut air = Hiss::new(seed, rate);
    Voice::new(rate, seconds + 0.05, move |t| {
        let share = (t / seconds).clamp(0.0, 1.0);
        let hz = 500.0 + (top - 500.0) * sine(share * 0.5);
        air.band(hz, 1.4) * swell(t, seconds * 0.3, seconds * 0.6, seconds * 0.4) * 1.1
    })
}

/// The pipes gurgling: bubbles of water through a narrow pipe, rising and popping.
fn gurgle(rate: f32, seed: u32) -> Voice {
    let mut pick = Noise::new(seed);
    let bubbles: Vec<(f32, f32)> = (0..9)
        .map(|k| {
            (
                k as f32 * 0.06 + pick.unit() * 0.03,
                260.0 + pick.unit() * 340.0,
            )
        })
        .collect();
    let mut phases = vec![Phase::default(); bubbles.len()];
    Voice::new(rate, 0.75, move |t| {
        let mut out = 0.0;
        for (phase, &(at, hz)) in phases.iter_mut().zip(&bubbles) {
            let since = t - at;
            if (0.0..0.09).contains(&since) {
                // Each bubble rises in pitch as it pops.
                let rising = hz * (1.0 + since * 9.0);
                out += sine(phase.tick(rising, rate)) * strike(since, 0.004, 0.03);
            }
        }
        out * 0.6
    })
}

/// The sprinkler: a sploosh of water, then a patter of drops coming down.
fn splash(rate: f32, seed: u32) -> Voice {
    let mut water = Hiss::new(seed, rate);
    let mut drops = Hiss::new(seed ^ 0x51, rate);
    let mut pick = Noise::new(seed ^ 0x2a);
    let patter: Vec<f32> = (0..16)
        .map(|k| 0.2 + k as f32 * 0.045 + pick.unit() * 0.03)
        .collect();
    Voice::new(rate, 1.2, move |t| {
        let hz = 600.0 + 2400.0 * (1.0 - ease(t / 0.4));
        let body = water.band(hz, 0.7) * strike(t, 0.01, 0.22) * 1.6;
        let tap: f32 = patter.iter().map(|at| strike(t - at, 0.001, 0.012)).sum();
        body + drops.band(4200.0, 2.0) * tap.min(1.0) * 0.7
    })
}

/// A leaf umbrella opening: a quick upward flick of air and a soft pluck.
fn fwip(rate: f32, seed: u32) -> Voice {
    let mut air = whoosh(rate, seed, 0.14, 5000.0);
    let mut leaf = Struck::new(BAR, &[(0.08, n(84.0), 0.6)]);
    Voice::new(rate, 0.5, move |t| {
        air.next().unwrap_or(0.0) + leaf.run(t, rate) * 0.4
    })
}

/// Surf: a low rush of noise that swells and draws back, with a fizz of foam on top.
fn surf(rate: f32, seed: u32) -> Voice {
    let mut body = Hiss::new(seed, rate);
    let mut foam = Hiss::new(seed ^ 0x3c, rate);
    Voice::new(rate, 1.4, move |t| {
        let rush =
            body.low(300.0 + 900.0 * swell(t, 0.4, 0.5, 0.6)) * swell(t, 0.35, 0.55, 0.7) * 1.6;
        let fizz = foam.band(5500.0, 1.0) * swell(t - 0.3, 0.2, 0.4, 0.6) * 0.35;
        rush + fizz
    })
}

/// Leaves rustling as a branch is shaken: quick bursts of high, bright noise.
fn rustle(rate: f32, seed: u32) -> Voice {
    let mut leaves = Hiss::new(seed, rate);
    let mut pick = Noise::new(seed ^ 0x19);
    let shakes: Vec<f32> = (0..8)
        .map(|k| k as f32 * 0.07 + pick.unit() * 0.03)
        .collect();
    Voice::new(rate, 0.75, move |t| {
        let amount: f32 = shakes.iter().map(|at| strike(t - at, 0.01, 0.05)).sum();
        leaves.band(3600.0 + 1200.0 * sine(t * 7.0), 0.9) * amount.min(1.2) * 0.9
    })
}

/// Honey: a low, wet squelch sliding down.
fn squelch(rate: f32, seed: u32) -> Voice {
    let mut phase = Phase::default();
    let mut wet = Hiss::new(seed, rate);
    Voice::new(rate, 0.35, move |t| {
        let hz = 220.0 - 120.0 * ease(t / 0.25);
        let body = triangle(phase.tick(hz * (1.0 + 0.3 * sine(t * 40.0)), rate));
        (body * 0.6 + wet.band(hz * 4.0, 2.0) * 0.5) * swell(t, 0.02, 0.2, 0.1) * 0.7
    })
}

/// Something that came to nothing: a sputter running down, like air out of a balloon.
fn fizzle(rate: f32, seed: u32) -> Voice {
    let mut phase = Phase::default();
    let mut rasp = Hiss::new(seed, rate);
    Voice::new(rate, 0.6, move |t| {
        let hz = 520.0 - 360.0 * ease(t / 0.5);
        let flutter = 0.5 + 0.5 * sine(t * 34.0);
        (triangle(phase.tick(hz, rate)) * 0.6 + rasp.band(hz * 3.0, 3.0) * 0.4)
            * flutter
            * swell(t, 0.02, 0.45, 0.1)
            * 0.6
    })
}

/// The beacon lit: a rush of flame and a sparkle over it.
fn fwoosh(rate: f32, seed: u32) -> Voice {
    let mut flame = Hiss::new(seed, rate);
    let mut sparkle = Struck::new(
        CHIME,
        &[
            (0.3, n(84.0), 0.5),
            (0.38, n(88.0), 0.45),
            (0.46, n(91.0), 0.45),
            (0.54, n(96.0), 0.5),
        ],
    );
    Voice::new(rate, 1.4, move |t| {
        let hz = 300.0 + 3200.0 * ease(t / 0.5);
        flame.low(hz) * swell(t, 0.25, 0.45, 0.5) * 1.3 + sparkle.run(t, rate) * 0.45
    })
}

/// A win: a little brass fanfare, with a marimba doubling it.
fn fanfare(rate: f32) -> Voice {
    let notes = [
        (0.0, 0.1, 72.0),
        (0.13, 0.1, 76.0),
        (0.26, 0.1, 79.0),
        (0.4, 0.55, 84.0),
    ];
    let mut brass = horn(rate, &notes, 2600.0, 0.5);
    let mut bar = Struck::new(BAR, &notes.map(|(at, _, note)| (at, n(note), 0.8)));
    Voice::new(rate, 1.2, move |t| {
        brass.next().unwrap_or(0.0) + bar.run(t, rate) * 0.35
    })
}

/// A loss: wah, wah, wah, wahhh, with the last one wobbling, kindly.
fn womp(rate: f32) -> Voice {
    let notes = [
        (0.0, 0.26, 67.0),
        (0.32, 0.26, 66.0),
        (0.64, 0.26, 65.0),
        (0.96, 0.8, 64.0),
    ];
    let mut phases = [Phase::default(); 4];
    let mut tone = Svf::new(1200.0, 0.9, rate);
    Voice::new(rate, 1.9, move |t| {
        let mut out = 0.0;
        for (phase, &(at, held, note)) in phases.iter_mut().zip(&notes) {
            let since = t - at;
            if since < 0.0 || since > held + 0.12 {
                continue;
            }
            // The last note sags and wobbles; each opens like a hand off a mute.
            let wobble = if note == 64.0 {
                1.0 + 0.025 * sine(since * 6.0) * ease(since / 0.3)
            } else {
                1.0
            };
            let mute = 0.35 + 0.65 * ease(since / 0.12);
            out += reed(phase.tick(n(note) * wobble, rate)) * swell(since, 0.03, held, 0.1) * mute;
        }
        tone.run(out).low * 0.55
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sound::synth::RATE;

    fn loudest(cue: Cue, variation: u32) -> f32 {
        cue.voice(variation, RATE)
            .fold(0.0f32, |most, x| most.max(x.abs()))
    }

    #[test]
    fn every_cue_is_heard_and_none_is_too_loud_or_runs_on() {
        for cue in Cue::ALL {
            let voice = cue.voice(0, RATE);
            // The train takes its time pulling in and out; nothing else does.
            let most = if matches!(cue, Cue::TrainArrive | Cue::TrainDepart) {
                4.5
            } else {
                2.5
            };
            assert!(voice.seconds() < most, "{cue:?} runs on");
            let peak = loudest(cue, 0);
            assert!(peak > 0.02, "{cue:?} is silent: {peak}");
            assert!(peak < 0.9, "{cue:?} is too loud: {peak}");
        }
    }

    #[test]
    fn a_cue_played_again_is_the_same_the_nth_time() {
        for cue in [Cue::Pow, Cue::Step, Cue::Miss, Cue::PageTurn] {
            let a: Vec<f32> = cue.voice(3, RATE).collect();
            let b: Vec<f32> = cue.voice(3, RATE).collect();
            assert_eq!(a, b);
        }
    }

    #[test]
    fn every_cue_ends_in_silence() {
        for cue in Cue::ALL {
            let samples: Vec<f32> = cue.voice(1, RATE).collect();
            let last = samples.last().copied().unwrap_or(0.0);
            assert!(last.abs() < 1e-3, "{cue:?} ends on {last}");
        }
    }

    #[test]
    fn every_cue_has_its_own_name() {
        let mut names: Vec<&str> = Cue::ALL.iter().map(|c| c.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Cue::ALL.len());
    }
}
