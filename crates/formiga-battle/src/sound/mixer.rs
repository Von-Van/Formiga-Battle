//! Everything playing at once, mixed into one stream: the music, crossfading from one piece to
//! the next, and whatever cues are sounding over it, each at the level asked for. Pure, so it is
//! the same whether it feeds a speaker or a file; the window only ever sends it commands.

use super::cues::{Cue, Voice};
use super::music::{Player, Track};
use super::synth::{Smooth, soften};

/// What the window asks of the mixer.
pub enum Command {
    /// A cue, `loudness` as loud as it is made.
    Play { cue: Cue, loudness: f32 },
    /// The piece wanted now, or quiet.
    Music(Option<Track>),
    /// How loud the music and the sounds are to be, each from 0 to 1.
    Levels { music: f32, sounds: f32 },
}

/// How long the music takes to cross from one piece to the next.
const CROSSFADE_SECS: f32 = 1.2;
/// The most cues sounding at once; past that, a new one is let go unheard.
const MOST_VOICES: usize = 24;

struct Playing {
    track: Track,
    player: Player,
    /// From 0, silent, to 1, full.
    fade: f32,
    /// How far it moves each sample: up while it comes in, down while it goes.
    step: f32,
}

pub struct Mixer {
    rate: f32,
    current: Option<Playing>,
    fading: Vec<Playing>,
    voices: Vec<Voice>,
    /// How many times each cue has been played, for its next variation, and when it last was.
    plays: Vec<u32>,
    last: Vec<Option<u64>>,
    clock: u64,
    music: f32,
    sounds: f32,
    music_now: Smooth,
    sounds_now: Smooth,
}

impl Mixer {
    /// A mixer at `rate`, everything silent until levels are given.
    pub fn new(rate: f32) -> Self {
        Self {
            rate,
            current: None,
            fading: Vec::new(),
            voices: Vec::with_capacity(MOST_VOICES),
            plays: vec![0; Cue::ALL.len()],
            last: vec![None; Cue::ALL.len()],
            clock: 0,
            music: 0.0,
            sounds: 0.0,
            music_now: Smooth::new(12.0, rate),
            sounds_now: Smooth::new(12.0, rate),
        }
    }

    pub fn command(&mut self, command: Command) {
        match command {
            Command::Play { cue, loudness } => self.play(cue, loudness),
            Command::Music(track) => self.change_music(track),
            Command::Levels { music, sounds } => {
                self.music = music.clamp(0.0, 1.0);
                self.sounds = sounds.clamp(0.0, 1.0);
            }
        }
    }

    fn play(&mut self, cue: Cue, loudness: f32) {
        let index = cue.index();
        // A cue too soon after the last of the same is let go: a patter, not a roar.
        let spacing = (cue.spacing() * self.rate) as u64;
        if self.last[index].is_some_and(|last| self.clock < last + spacing)
            || self.voices.len() >= MOST_VOICES
        {
            return;
        }
        self.last[index] = Some(self.clock);
        let variation = self.plays[index];
        self.plays[index] = variation.wrapping_add(1);
        self.voices
            .push(cue.voice(variation, self.rate).louder(loudness));
    }

    fn change_music(&mut self, wanted: Option<Track>) {
        if self.current.as_ref().map(|playing| playing.track) == wanted {
            return;
        }
        let step = 1.0 / (CROSSFADE_SECS * self.rate);
        if let Some(mut leaving) = self.current.take() {
            leaving.step = -step;
            self.fading.push(leaving);
        }
        let Some(track) = wanted else {
            return;
        };
        // A piece still fading out comes back from where it was, rather than starting over.
        let playing = match self.fading.iter().position(|p| p.track == track) {
            Some(index) => {
                let mut returning = self.fading.remove(index);
                returning.step = step;
                returning
            }
            None => Playing {
                track,
                player: Player::new(track, self.rate),
                fade: 0.0,
                step,
            },
        };
        self.current = Some(playing);
    }

    /// The next sample of everything together.
    pub fn next(&mut self) -> f32 {
        self.clock += 1;
        let music_level = self.music_now.run(self.music);
        let sounds_level = self.sounds_now.run(self.sounds);
        let mut music = 0.0;
        for playing in self.current.iter_mut().chain(self.fading.iter_mut()) {
            playing.fade = (playing.fade + playing.step).clamp(0.0, 1.0);
            // Equal power: as one piece goes and another comes, the two are as loud as one.
            let gain = (playing.fade * std::f32::consts::FRAC_PI_2).sin();
            music += playing.player.next() * gain;
        }
        self.fading.retain(|playing| playing.fade > 0.0);
        let mut sounds = 0.0;
        self.voices.retain_mut(|voice| match voice.next() {
            Some(sample) => {
                sounds += sample;
                true
            }
            None => false,
        });
        soften(music * music_level + sounds * sounds_level)
    }

    /// How many pieces are playing, or fading out.
    #[cfg(test)]
    fn pieces(&self) -> usize {
        usize::from(self.current.is_some()) + self.fading.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sound::synth::RATE;

    fn run(mixer: &mut Mixer, seconds: f32) -> Vec<f32> {
        (0..(seconds * RATE) as usize)
            .map(|_| mixer.next())
            .collect()
    }

    fn loudest(samples: &[f32]) -> f32 {
        samples.iter().fold(0.0f32, |most, x| most.max(x.abs()))
    }

    #[test]
    fn muted_not_a_sample_is_heard() {
        let mut mixer = Mixer::new(RATE);
        mixer.command(Command::Levels {
            music: 0.0,
            sounds: 0.0,
        });
        mixer.command(Command::Music(Some(Track::Match)));
        for cue in Cue::ALL {
            mixer.command(Command::Play { cue, loudness: 1.0 });
        }
        assert!(run(&mut mixer, 1.0).iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn everything_at_once_at_full_volume_stays_within_full_scale() {
        let mut mixer = Mixer::new(RATE);
        mixer.command(Command::Levels {
            music: 1.0,
            sounds: 1.0,
        });
        mixer.command(Command::Music(Some(Track::Match)));
        for cue in Cue::ALL {
            mixer.command(Command::Play { cue, loudness: 3.0 });
        }
        let samples = run(&mut mixer, 1.0);
        assert!(samples.iter().all(|sample| sample.abs() < 1.0));
        assert!(loudest(&samples) > 0.5);
    }

    #[test]
    fn the_music_crosses_over_to_the_next_piece_and_lets_the_last_go() {
        let mut mixer = Mixer::new(RATE);
        mixer.command(Command::Levels {
            music: 1.0,
            sounds: 1.0,
        });
        mixer.command(Command::Music(Some(Track::Lobby)));
        run(&mut mixer, 2.0);
        mixer.command(Command::Music(Some(Track::Match)));
        run(&mut mixer, CROSSFADE_SECS / 2.0);
        assert_eq!(mixer.pieces(), 2, "both are heard while they cross");
        run(&mut mixer, CROSSFADE_SECS);
        assert_eq!(mixer.pieces(), 1);
        mixer.command(Command::Music(None));
        run(&mut mixer, CROSSFADE_SECS + 0.2);
        assert_eq!(mixer.pieces(), 0);
    }

    #[test]
    fn a_crowd_of_the_same_cue_is_thinned_to_a_patter() {
        let mut mixer = Mixer::new(RATE);
        for _ in 0..10 {
            mixer.command(Command::Play {
                cue: Cue::Step,
                loudness: 1.0,
            });
        }
        assert_eq!(mixer.voices.len(), 1);
    }
}
