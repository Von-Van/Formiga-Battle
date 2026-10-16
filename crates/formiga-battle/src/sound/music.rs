//! The music: two whimsical placeholder loops, made as they play rather than stored. A gentle
//! waltz for choosing a team and between matches, and a bouncy little march for the match itself,
//! played by a small band (marimba, pizzicato, a reedy clarinet, a tuba, a shaker and a wood
//! block). Each note rings on from where it was struck, over the end of the loop and into the
//! next time round, so the loop joins without a seam.

use super::synth::{BAR, BLOCK, CHIME, Noise, Phase, Svf, hz, reed, sine, strike, swell, triangle};

/// A piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Track {
    /// Choosing a team, and the notes page between matches.
    Lobby,
    /// A match on the board.
    Match,
}

impl Track {
    pub const ALL: [Track; 2] = [Track::Lobby, Track::Match];

    pub fn name(self) -> &'static str {
        match self {
            Track::Lobby => "lobby",
            Track::Match => "match",
        }
    }

    fn score(self) -> Score {
        match self {
            Track::Lobby => lobby(),
            Track::Match => march(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Instrument {
    Marimba,
    Glock,
    Pluck,
    Tuba,
    Clarinet,
    Shaker,
    Block,
}

/// A note on the loop: when, in beats; its MIDI pitch; how long it is held, in beats; which
/// instrument; how hard.
#[derive(Clone, Copy, Debug)]
struct Note {
    beat: f32,
    pitch: f32,
    beats: f32,
    instrument: Instrument,
    loud: f32,
}

struct Score {
    tempo: f32,
    beats: f32,
    notes: Vec<Note>,
    gain: f32,
}

impl Score {
    fn new(tempo: f32, beats: f32, gain: f32) -> Self {
        Self {
            tempo,
            beats,
            notes: Vec::new(),
            gain,
        }
    }

    fn add(&mut self, beat: f32, pitch: f32, beats: f32, instrument: Instrument, loud: f32) {
        self.notes.push(Note {
            beat,
            pitch,
            beats,
            instrument,
            loud,
        });
    }

    /// A line of (pitch, beats) from `start`, a pitch of 0 being a rest.
    fn line(&mut self, start: f32, line: &[(f32, f32)], instrument: Instrument, loud: f32) {
        let mut beat = start;
        for &(pitch, beats) in line {
            if pitch > 0.0 {
                self.add(beat, pitch, beats * 0.9, instrument, loud);
            }
            beat += beats;
        }
    }
}

/// A line moved up or down by `by` semitones, its rests left as rests.
fn octave(line: &[(f32, f32)], by: f32) -> Vec<(f32, f32)> {
    line.iter()
        .map(|&(pitch, beats)| (if pitch > 0.0 { pitch + by } else { 0.0 }, beats))
        .collect()
}

/// The waltz for choosing: C major, three in a bar, the tune on the marimba and then on the
/// glockenspiel, the tuba on the one and soft plucked chords on the two and three.
fn lobby() -> Score {
    let mut score = Score::new(100.0, 48.0, 0.42);
    let tune = [
        (76.0, 1.0),
        (79.0, 1.0),
        (76.0, 1.0),
        (74.0, 0.5),
        (76.0, 0.5),
        (74.0, 1.0),
        (72.0, 1.0),
        (71.0, 1.0),
        (74.0, 1.0),
        (79.0, 1.0),
        (77.0, 2.0),
        (0.0, 1.0),
        (81.0, 1.0),
        (77.0, 1.0),
        (81.0, 1.0),
        (79.0, 1.5),
        (76.0, 0.5),
        (72.0, 1.0),
        (74.0, 1.0),
        (77.0, 0.5),
        (76.0, 0.5),
        (74.0, 1.0),
        (72.0, 2.0),
        (0.0, 1.0),
    ];
    score.line(0.0, &tune, Instrument::Marimba, 0.9);
    score.line(24.0, &tune, Instrument::Glock, 0.55);
    score.line(24.0, &octave(&tune, -12.0), Instrument::Marimba, 0.45);
    // C, C, G7, G7, F, C, G7, C; twice.
    let chords: [(f32, [f32; 3]); 8] = [
        (48.0, [60.0, 64.0, 67.0]),
        (43.0, [60.0, 64.0, 67.0]),
        (43.0, [59.0, 62.0, 65.0]),
        (50.0, [59.0, 62.0, 65.0]),
        (41.0, [57.0, 60.0, 65.0]),
        (48.0, [60.0, 64.0, 67.0]),
        (43.0, [59.0, 62.0, 65.0]),
        (48.0, [60.0, 64.0, 67.0]),
    ];
    for half in 0..2 {
        for (bar, (bass, chord)) in chords.iter().enumerate() {
            let at = (half * 8 + bar) as f32 * 3.0;
            score.add(at, *bass, 0.8, Instrument::Tuba, 0.7);
            for beat in [1.0, 2.0] {
                for (k, pitch) in chord.iter().enumerate() {
                    score.add(
                        at + beat + k as f32 * 0.012,
                        *pitch,
                        0.5,
                        Instrument::Pluck,
                        0.28,
                    );
                }
            }
        }
    }
    score
}

/// The march for the match: F major, four in a bar, the tune on a reedy clarinet and then on a
/// xylophone-bright marimba, the tuba's oom on one and three and plucked pahs on two and four,
/// a shaker on every half beat and a wood block at the end of every other bar.
fn march() -> Score {
    let mut score = Score::new(116.0, 64.0, 0.34);
    let tune = [
        // F
        (72.0, 0.5),
        (69.0, 0.5),
        (72.0, 0.5),
        (77.0, 0.5),
        (76.0, 0.5),
        (77.0, 0.5),
        (79.0, 1.0),
        // C7
        (76.0, 0.5),
        (72.0, 0.5),
        (67.0, 0.5),
        (72.0, 0.5),
        (70.0, 1.0),
        (69.0, 0.5),
        (67.0, 0.5),
        // F
        (69.0, 0.5),
        (72.0, 0.5),
        (77.0, 0.5),
        (81.0, 0.5),
        (79.0, 0.5),
        (77.0, 0.5),
        (76.0, 0.5),
        (77.0, 0.5),
        // B flat
        (74.0, 1.0),
        (70.0, 0.5),
        (74.0, 0.5),
        (77.0, 1.0),
        (0.0, 1.0),
        // F
        (72.0, 0.5),
        (69.0, 0.5),
        (72.0, 0.5),
        (77.0, 0.5),
        (81.0, 0.5),
        (79.0, 0.5),
        (77.0, 0.5),
        (74.0, 0.5),
        // C7
        (72.0, 0.5),
        (76.0, 0.5),
        (79.0, 0.5),
        (82.0, 0.5),
        (81.0, 0.5),
        (79.0, 0.5),
        (76.0, 0.5),
        (72.0, 0.5),
        // C7
        (74.0, 0.5),
        (76.0, 0.5),
        (77.0, 0.5),
        (79.0, 0.5),
        (76.0, 1.0),
        (72.0, 1.0),
        // F
        (77.0, 1.0),
        (72.0, 0.5),
        (69.0, 0.5),
    ];
    score.line(0.0, &tune, Instrument::Clarinet, 0.55);
    // The F the tune lands on, and a bar's rest before it starts again.
    score.add(30.0, 65.0, 0.9, Instrument::Clarinet, 0.55);
    score.line(32.0, &tune, Instrument::Marimba, 0.8);
    score.add(62.0, 65.0, 0.9, Instrument::Marimba, 0.8);
    score.line(32.0, &octave(&tune, 12.0), Instrument::Glock, 0.18);
    // F, C7, F, B flat, F, C7, C7, F; twice. The tuba's oom, root then fifth, and the chord's pah.
    let chords: [(f32, f32, [f32; 3]); 8] = [
        (41.0, 48.0, [57.0, 60.0, 65.0]),
        (36.0, 43.0, [55.0, 58.0, 64.0]),
        (41.0, 48.0, [57.0, 60.0, 65.0]),
        (46.0, 41.0, [58.0, 62.0, 65.0]),
        (41.0, 48.0, [57.0, 60.0, 65.0]),
        (36.0, 43.0, [55.0, 58.0, 64.0]),
        (36.0, 43.0, [55.0, 58.0, 64.0]),
        (41.0, 48.0, [57.0, 60.0, 65.0]),
    ];
    for half in 0..2 {
        for (bar, (root, fifth, chord)) in chords.iter().enumerate() {
            let at = (half * 8 + bar) as f32 * 4.0;
            score.add(at, *root, 0.7, Instrument::Tuba, 0.8);
            score.add(at + 2.0, *fifth, 0.7, Instrument::Tuba, 0.7);
            for beat in [1.0, 3.0] {
                for (k, pitch) in chord.iter().enumerate() {
                    score.add(
                        at + beat + k as f32 * 0.01,
                        *pitch,
                        0.4,
                        Instrument::Pluck,
                        0.3,
                    );
                }
            }
            for eighth in 0..8 {
                let loud = if eighth % 2 == 1 { 0.5 } else { 0.28 };
                score.add(at + eighth as f32 * 0.5, 0.0, 0.1, Instrument::Shaker, loud);
            }
            if bar % 2 == 1 {
                score.add(at + 3.0, 0.0, 0.1, Instrument::Block, 0.6);
                score.add(at + 3.5, 0.0, 0.1, Instrument::Block, 0.45);
            }
        }
    }
    score
}

/// One note sounding.
struct Sounding {
    instrument: Instrument,
    hz: f32,
    held: f32,
    loud: f32,
    t: f32,
    phases: [Phase; 3],
    noise: Noise,
    filter: Svf,
}

impl Sounding {
    fn new(note: &Note, seconds_per_beat: f32, seed: u32, rate: f32) -> Self {
        // The percussion has no tune: the block is tapped at its own pitch.
        let pitch_hz = match note.instrument {
            Instrument::Block => 880.0,
            _ => hz(note.pitch),
        };
        let filter = match note.instrument {
            Instrument::Clarinet => Svf::new(2400.0, 0.7, rate),
            Instrument::Shaker => Svf::new(7000.0, 1.2, rate),
            _ => Svf::new(1000.0, 0.7, rate),
        };
        Self {
            instrument: note.instrument,
            hz: pitch_hz,
            held: note.beats * seconds_per_beat,
            loud: note.loud,
            t: 0.0,
            phases: [Phase::default(); 3],
            noise: Noise::new(seed),
            filter,
        }
    }

    /// How long it rings after it is let go of.
    fn tail(&self) -> f32 {
        match self.instrument {
            Instrument::Marimba => 1.2,
            Instrument::Glock => 2.0,
            Instrument::Pluck => 0.6,
            Instrument::Tuba | Instrument::Clarinet => 0.1,
            Instrument::Shaker | Instrument::Block => 0.15,
        }
    }

    fn partials(&mut self, partials: super::synth::Partials, rate: f32) -> f32 {
        let mut out = 0.0;
        for (phase, &(ratio, amp, decay)) in self.phases.iter_mut().zip(partials) {
            out += sine(phase.tick(self.hz * ratio, rate)) * amp * strike(self.t, 0.0015, decay);
        }
        out
    }

    /// The next sample, or `None` once it has died away.
    fn next(&mut self, rate: f32) -> Option<f32> {
        if self.t > self.held + self.tail() {
            return None;
        }
        let t = self.t;
        let out = match self.instrument {
            Instrument::Marimba => self.partials(BAR, rate),
            Instrument::Glock => self.partials(CHIME, rate),
            Instrument::Pluck => {
                let p = self.phases[0].tick(self.hz, rate);
                (sine(p) + sine(p * 2.0) * 0.4 + sine(p * 3.0) * 0.15) * strike(t, 0.003, 0.16)
            }
            Instrument::Tuba => {
                let p = self.phases[0].tick(self.hz, rate);
                (triangle(p) * 0.7 + sine(p) * 0.6 + sine(p * 2.0) * 0.2)
                    * swell(t, 0.025, self.held, 0.08)
            }
            Instrument::Clarinet => {
                let vibrato = 1.0 + 0.008 * sine(t * 5.0) * (t / 0.3).min(1.0);
                let p = self.phases[0].tick(self.hz * vibrato, rate);
                let buzz = reed(p) * swell(t, 0.02, self.held, 0.06);
                self.filter.run(buzz).low
            }
            Instrument::Shaker => {
                let hiss = self.noise.next();
                self.filter.run(hiss).band * strike(t, 0.004, 0.035)
            }
            Instrument::Block => self.partials(BLOCK, rate),
        };
        self.t += 1.0 / rate;
        Some(out * self.loud)
    }
}

/// A piece playing, a sample at a time, round and round.
pub struct Player {
    rate: f32,
    score: Score,
    /// Each note's start, in samples into the loop, in order.
    starts: Vec<usize>,
    order: Vec<usize>,
    length: usize,
    at: usize,
    next: usize,
    sounding: Vec<Sounding>,
    struck: u32,
}

impl Player {
    pub fn new(track: Track, rate: f32) -> Self {
        let score = track.score();
        let per_beat = 60.0 / score.tempo;
        let mut order: Vec<usize> = (0..score.notes.len()).collect();
        order.sort_by(|a, b| score.notes[*a].beat.total_cmp(&score.notes[*b].beat));
        let starts = order
            .iter()
            .map(|i| (score.notes[*i].beat * per_beat * rate) as usize)
            .collect();
        let length = (score.beats * per_beat * rate) as usize;
        Self {
            rate,
            score,
            starts,
            order,
            length,
            at: 0,
            next: 0,
            sounding: Vec::new(),
            struck: 0,
        }
    }

    pub fn next(&mut self) -> f32 {
        let per_beat = 60.0 / self.score.tempo;
        while self.next < self.starts.len() && self.starts[self.next] <= self.at {
            let note = self.score.notes[self.order[self.next]];
            self.struck = self.struck.wrapping_add(1);
            self.sounding
                .push(Sounding::new(&note, per_beat, self.struck, self.rate));
            self.next += 1;
        }
        let rate = self.rate;
        let mut out = 0.0;
        self.sounding.retain_mut(|s| match s.next(rate) {
            Some(sample) => {
                out += sample;
                true
            }
            None => false,
        });
        self.at += 1;
        if self.at >= self.length {
            self.at = 0;
            self.next = 0;
        }
        out * self.score.gain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sound::synth::RATE;

    #[test]
    fn every_piece_is_heard_and_stays_within_full_scale() {
        for track in Track::ALL {
            let mut player = Player::new(track, RATE);
            let samples: Vec<f32> = (0..(RATE as usize * 8)).map(|_| player.next()).collect();
            let peak = samples.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.05, "{track:?} is silent");
            assert!(peak < 0.9, "{track:?} is too loud: {peak}");
        }
    }

    #[test]
    fn a_piece_goes_round_again_and_its_notes_die_away() {
        let mut player = Player::new(Track::Lobby, RATE);
        for _ in 0..player.length + 10 {
            player.next();
        }
        assert!(player.at < 20, "it starts again");
        assert!(player.sounding.len() < 24, "notes are let go");
    }
}
