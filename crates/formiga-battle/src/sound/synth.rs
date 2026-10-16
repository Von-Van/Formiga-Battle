//! The little synthesiser every sound in Battle is made with: oscillators, envelopes, noise and a
//! filter, after Formiga Hill's. Nothing is recorded or downloaded, so there is nothing to
//! license; and nothing here reads a clock or a random device, so a sound is made the same way
//! every time it is asked for, which is what lets the tests listen to it.

use std::f32::consts::{PI, TAU};

/// The rate renders and tests are made at. A device plays at its own rate, and every sound is
/// made for whichever rate it is asked for.
pub const RATE: f32 = 44_100.0;

/// The sine of a phase measured in turns.
pub fn sine(turns: f32) -> f32 {
    (turns * TAU).sin()
}

/// A triangle wave of a phase in turns: a sine's softer, hollower cousin.
pub fn triangle(turns: f32) -> f32 {
    let t = turns - turns.floor();
    4.0 * (t - 0.5).abs() - 1.0
}

/// A square wave with its corners rounded off by keeping only its first few odd harmonics: a
/// reed's buzz, not a buzzer's.
pub fn reed(turns: f32) -> f32 {
    sine(turns) + sine(turns * 3.0) / 3.0 * 0.8 + sine(turns * 5.0) / 5.0 * 0.5
}

/// The frequency of a MIDI note: 69 is the A at 440 Hz, and each step is a semitone.
pub fn hz(note: f32) -> f32 {
    440.0 * 2f32.powf((note - 69.0) / 12.0)
}

/// A phase going round, in turns.
#[derive(Clone, Copy, Debug, Default)]
pub struct Phase(f32);

impl Phase {
    /// Where it is, then on by one sample at `hz`.
    pub fn tick(&mut self, hz: f32, rate: f32) -> f32 {
        let now = self.0;
        self.0 += hz / rate;
        if self.0 >= 1.0 {
            self.0 -= self.0.floor();
        }
        now
    }
}

/// White noise from a seed: the same hiss every time for the same seed.
#[derive(Clone, Debug)]
pub struct Noise(u32);

impl Noise {
    pub fn new(seed: u32) -> Self {
        // Xorshift never leaves zero, so a seed is never allowed to put it there.
        let mixed = seed.wrapping_mul(0x9e37_79b9) ^ 0x6d2b_79f5;
        Self(if mixed == 0 { 1 } else { mixed })
    }

    /// The next sample, from -1 up to 1.
    pub fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x >> 8) as f32 / 8_388_608.0 - 1.0
    }

    /// A number from 0 up to 1, for choosing.
    pub fn unit(&mut self) -> f32 {
        (self.next() + 1.0) * 0.5
    }
}

/// A one-pole low-pass: the gentlest smoothing, for taking the edge off a level as it changes.
#[derive(Clone, Debug)]
pub struct Smooth {
    coefficient: f32,
    held: f32,
}

impl Smooth {
    pub fn new(hz: f32, rate: f32) -> Self {
        Self {
            coefficient: 1.0 - (-TAU * hz / rate).exp(),
            held: 0.0,
        }
    }

    pub fn run(&mut self, input: f32) -> f32 {
        self.held += self.coefficient * (input - self.held);
        self.held
    }
}

/// Takes out any steady offset below hearing, so every sound settles back to silence at zero.
#[derive(Clone, Debug)]
pub struct DcBlock {
    pole: f32,
    last_in: f32,
    last_out: f32,
}

impl DcBlock {
    pub fn new(rate: f32) -> Self {
        Self {
            pole: (-TAU * 16.0 / rate).exp(),
            last_in: 0.0,
            last_out: 0.0,
        }
    }

    pub fn run(&mut self, input: f32) -> f32 {
        let out = input - self.last_in + self.pole * self.last_out;
        self.last_in = input;
        self.last_out = out;
        out
    }
}

/// What a state-variable filter gives at once.
#[derive(Clone, Copy, Debug)]
pub struct Bands {
    pub low: f32,
    /// Peaks at the input's own level, whatever the resonance.
    pub band: f32,
}

/// A state-variable filter (Zavalishin's form) that stays stable however fast its frequency is
/// swept, which most of the effects do: a whoosh, a creak, a rustle.
#[derive(Clone, Debug)]
pub struct Svf {
    rate: f32,
    k: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    ic1: f32,
    ic2: f32,
}

impl Svf {
    pub fn new(hz: f32, q: f32, rate: f32) -> Self {
        let mut filter = Self {
            rate,
            k: 1.0,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            ic1: 0.0,
            ic2: 0.0,
        };
        filter.tune(hz, q);
        filter
    }

    /// Moves the filter to `hz`, with resonance `q` (0.5 is gentle, 8 rings), keeping its state.
    pub fn tune(&mut self, hz: f32, q: f32) {
        let hz = hz.clamp(10.0, self.rate * 0.45);
        let g = (PI * hz / self.rate).tan();
        self.k = 1.0 / q.max(0.1);
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    pub fn run(&mut self, input: f32) -> Bands {
        let v3 = input - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        Bands {
            low: v2,
            band: self.k * v1,
        }
    }
}

/// Smoothly from 0 to 1 as `x` goes from 0 to 1.
pub fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A struck sound's loudness `t` seconds in: up over `attack`, then dying away, by a factor of e
/// every `decay` seconds.
pub fn strike(t: f32, attack: f32, decay: f32) -> f32 {
    if t < 0.0 {
        0.0
    } else if t < attack {
        t / attack
    } else {
        (-(t - attack) / decay).exp()
    }
}

/// A held sound's loudness `t` seconds in: up over `attack`, held until `held` seconds are up,
/// then down over `release`, smooth at each end.
pub fn swell(t: f32, attack: f32, held: f32, release: f32) -> f32 {
    if t < 0.0 {
        0.0
    } else if t < held {
        ease(t / attack.max(1e-4))
    } else {
        ease(held / attack.max(1e-4)) * (1.0 - ease((t - held) / release.max(1e-4)))
    }
}

/// A gentle limit on the loudest moments: untouched below 0.7, then bending smoothly towards a
/// ceiling a little under full scale, so nothing that reaches the speaker can clip.
pub fn soften(sample: f32) -> f32 {
    const KNEE: f32 = 0.7;
    const CEILING: f32 = 0.98;
    let size = sample.abs();
    if size <= KNEE {
        sample
    } else {
        let room = CEILING - KNEE;
        (KNEE + room * ((size - KNEE) / room).tanh()).copysign(sample)
    }
}

/// Partials of a struck thing: how far above the note each is, how loud, and how many seconds it
/// takes to die away by a factor of e.
pub type Partials = &'static [(f32, f32, f32)];

/// A marimba's wooden bar.
pub const BAR: Partials = &[(1.0, 1.0, 0.32), (3.93, 0.12, 0.05), (9.2, 0.04, 0.02)];
/// A soft chime: nearly a pure tone, with a bell's faint overtones.
pub const CHIME: Partials = &[(1.0, 1.0, 0.9), (2.0, 0.22, 0.45), (3.0, 0.08, 0.25)];
/// A hollow wooden block, tapped.
pub const BLOCK: Partials = &[(1.0, 1.0, 0.035), (2.71, 0.35, 0.018), (4.4, 0.1, 0.01)];
/// A pan lid or a bucket: metal, its overtones out of tune with one another.
pub const METAL: Partials = &[
    (1.0, 1.0, 0.5),
    (2.41, 0.7, 0.35),
    (3.83, 0.5, 0.25),
    (5.27, 0.35, 0.18),
    (6.9, 0.2, 0.12),
];

/// Notes struck one after another, each ringing on with the same partials.
pub struct Struck {
    partials: Partials,
    /// When each note is struck, its pitch, and how loud.
    notes: Vec<(f32, f32, f32)>,
    phases: Vec<Phase>,
}

impl Struck {
    pub fn new(partials: Partials, notes: &[(f32, f32, f32)]) -> Self {
        Self {
            partials,
            notes: notes.to_vec(),
            phases: vec![Phase::default(); notes.len() * partials.len()],
        }
    }

    pub fn run(&mut self, t: f32, rate: f32) -> f32 {
        let mut out = 0.0;
        for (index, &(at, hz, loud)) in self.notes.iter().enumerate() {
            let since = t - at;
            if since < 0.0 {
                continue;
            }
            for (part, &(ratio, amp, decay)) in self.partials.iter().enumerate() {
                let phase = &mut self.phases[index * self.partials.len() + part];
                let turn = phase.tick(hz * ratio, rate);
                out += sine(turn) * amp * loud * strike(since, 0.0015, decay);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_are_tuned_to_concert_pitch() {
        assert!((hz(69.0) - 440.0).abs() < 1e-3);
        assert!((hz(81.0) - 880.0).abs() < 1e-2);
        assert!((hz(60.0) - 261.63).abs() < 1e-2);
    }

    #[test]
    fn noise_is_the_same_for_the_same_seed_and_stays_in_range() {
        let (mut a, mut b) = (Noise::new(7), Noise::new(7));
        let heard: Vec<f32> = (0..5000).map(|_| a.next()).collect();
        assert!(heard.iter().all(|sample| (-1.0..1.0).contains(sample)));
        assert!(heard.iter().all(|x| *x == b.next()));
        assert_ne!(Noise::new(0).next(), 0.0, "a zero seed still hisses");
    }

    #[test]
    fn envelopes_rise_hold_and_fall_to_nothing() {
        assert_eq!(strike(-0.1, 0.01, 0.2), 0.0);
        assert!((strike(0.01, 0.01, 0.2) - 1.0).abs() < 1e-6);
        assert!(strike(2.0, 0.01, 0.2) < 1e-4);
        assert!((swell(0.5, 0.1, 1.0, 0.3) - 1.0).abs() < 1e-6);
        assert_eq!(swell(1.3, 0.1, 1.0, 0.3), 0.0);
    }

    #[test]
    fn softening_never_lets_a_sample_past_full_scale() {
        for step in -400..=400 {
            let sample = step as f32 / 40.0;
            let soft = soften(sample);
            assert!(soft.abs() < 1.0);
            if sample.abs() <= 0.7 {
                assert_eq!(soft, sample, "quiet sounds are left alone");
            }
        }
    }
}
