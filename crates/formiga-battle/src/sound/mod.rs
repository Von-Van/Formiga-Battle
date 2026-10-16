//! Battle's sound: cheerful placeholder effects and music, all made in code (`synth`), as
//! Formiga Hill makes its own. The window and the menus hold one `Sound` and only ever say what
//! happened (`play`) or what should be playing (`music`); the speaker thread does the rest.
//!
//! Nothing here keeps a file: Desktop keeps Battle's record and has no sound setting to send, so
//! the setting lasts as long as the window.

mod cues;
mod mixer;
mod music;
mod speaker;
mod synth;

pub use cues::Cue;
pub use music::Track;

use anyhow::{Context, Result};
use mixer::{Command, Mixer};
use std::path::Path;
use std::sync::mpsc::{Sender, channel};

/// What is heard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    /// Music and sounds.
    All,
    /// The sounds, without the music.
    Sounds,
    /// Nothing.
    Off,
}

impl Setting {
    /// The next, for a button that goes round them.
    pub fn next(self) -> Self {
        match self {
            Setting::All => Setting::Sounds,
            Setting::Sounds => Setting::Off,
            Setting::Off => Setting::All,
        }
    }

    /// In a word or two, for the button.
    pub fn words(self) -> &'static str {
        match self {
            Setting::All => "Music on",
            Setting::Sounds => "Sounds only",
            Setting::Off => "Quiet",
        }
    }

    /// How loud the music and the sounds are, from 0 to 1.
    fn levels(self) -> (f32, f32) {
        match self {
            // The music sits under the sounds, so a pow is never lost in the band.
            Setting::All => (0.45, 1.0),
            Setting::Sounds => (0.0, 1.0),
            Setting::Off => (0.0, 0.0),
        }
    }
}

/// The window's handle on the speaker. Quiet, and costs nothing, if there is no speaker to play
/// on, or none was wanted (a review picture).
pub struct Sound {
    commands: Option<Sender<Command>>,
    /// Dropped with the window, which lets the speaker thread close the device.
    _hush: Option<Sender<()>>,
    setting: Setting,
    track: Option<Track>,
}

impl Sound {
    /// Opens the speaker on a thread of its own.
    pub fn open() -> Self {
        let (commands, heard) = channel();
        let (hush, hushed) = channel();
        let spawned = std::thread::Builder::new()
            .name("formiga-battle-sound".to_owned())
            .spawn(move || {
                if let Err(why) = speaker::speak(heard, hushed) {
                    eprintln!("formiga-battle: playing without sound: {why}");
                }
            });
        let mut sound = Self {
            commands: spawned.ok().map(|_| commands),
            _hush: Some(hush),
            setting: Setting::All,
            track: None,
        };
        sound.set(Setting::All);
        sound
    }

    /// No sound at all.
    pub fn silent() -> Self {
        Self {
            commands: None,
            _hush: None,
            setting: Setting::Off,
            track: None,
        }
    }

    fn send(&self, command: Command) {
        if let Some(commands) = &self.commands {
            // A speaker thread that has gone takes nothing more; the window plays on in quiet.
            let _ = commands.send(command);
        }
    }

    /// A cue, as loud as it is made.
    pub fn play(&self, cue: Cue) {
        self.play_at(cue, 1.0);
    }

    /// A cue, `loudness` as loud as it is made.
    pub fn play_at(&self, cue: Cue, loudness: f32) {
        if self.setting != Setting::Off {
            self.send(Command::Play { cue, loudness });
        }
    }

    /// The piece that should be playing now, or quiet. Asking for the one already playing does
    /// nothing, so a window can say it every frame.
    pub fn music(&mut self, track: Option<Track>) {
        if self.track != track {
            self.track = track;
            self.send(Command::Music(track));
        }
    }

    pub fn setting(&self) -> Setting {
        self.setting
    }

    pub fn set(&mut self, setting: Setting) {
        self.setting = setting;
        let (music, sounds) = setting.levels();
        self.send(Command::Levels { music, sounds });
    }
}

/// How much of each piece of music is written.
const MUSIC_SECS: f32 = 40.0;

/// Writes every cue and some of each piece of music into `folder` as WAV files, for listening to
/// them without the window, and says what it wrote.
pub fn render(folder: &Path) -> Result<Vec<String>> {
    std::fs::create_dir_all(folder)
        .with_context(|| format!("could not make the folder {}", folder.display()))?;
    let mut written = Vec::new();
    for cue in Cue::ALL {
        let mut mixer = Mixer::new(synth::RATE);
        mixer.command(Command::Levels {
            music: 1.0,
            sounds: 1.0,
        });
        // The mixer eases its levels in; a moment's start lets it reach them first.
        for _ in 0..(synth::RATE * 0.05) as usize {
            mixer.next();
        }
        mixer.command(Command::Play { cue, loudness: 1.0 });
        let length = cue.voice(0, synth::RATE).count();
        let samples: Vec<f32> = (0..length).map(|_| mixer.next()).collect();
        let name = format!("cue-{}.wav", cue.name());
        save(&folder.join(&name), &samples)?;
        written.push(format!(
            "{name}: {:.2}s",
            samples.len() as f32 / synth::RATE
        ));
    }
    for track in Track::ALL {
        let mut player = music::Player::new(track, synth::RATE);
        let length = (MUSIC_SECS * synth::RATE) as usize;
        let fade = (0.05 * synth::RATE) as usize;
        let samples: Vec<f32> = (0..length)
            .map(|at| synth::soften(player.next()) * ((length - at) as f32 / fade as f32).min(1.0))
            .collect();
        let name = format!("music-{}.wav", track.name());
        save(&folder.join(&name), &samples)?;
        written.push(format!("{name}: {MUSIC_SECS:.0}s"));
    }
    Ok(written)
}

fn save(path: &Path, samples: &[f32]) -> Result<()> {
    std::fs::write(path, encode(samples, synth::RATE as u32))
        .with_context(|| format!("could not write {}", path.display()))
}

/// A sixteen-bit mono WAV of `samples` at `rate`, each clamped to full scale. The header is
/// written by hand: sixteen-bit mono PCM needs nothing more.
fn encode(samples: &[f32], rate: u32) -> Vec<u8> {
    let data = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data as usize);
    bytes.extend(b"RIFF");
    bytes.extend((36 + data).to_le_bytes());
    bytes.extend(b"WAVE");
    bytes.extend(b"fmt ");
    bytes.extend(16u32.to_le_bytes());
    // Plain PCM, one channel, two bytes a sample.
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(rate.to_le_bytes());
    bytes.extend((rate * 2).to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(data.to_le_bytes());
    for sample in samples {
        let level = (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        bytes.extend(level.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_setting_goes_round_all_three() {
        let mut setting = Setting::All;
        let mut seen = Vec::new();
        for _ in 0..3 {
            seen.push(setting);
            setting = setting.next();
        }
        assert_eq!(setting, Setting::All);
        assert_eq!(seen, [Setting::All, Setting::Sounds, Setting::Off]);
    }

    #[test]
    fn a_silent_sound_takes_every_ask_and_plays_nothing() {
        let mut sound = Sound::silent();
        sound.play(Cue::Pow);
        sound.music(Some(Track::Match));
        sound.set(Setting::All);
        assert_eq!(sound.setting(), Setting::All);
    }

    #[test]
    fn a_wav_says_how_long_it_is_in_its_header() {
        let bytes = encode(&[0.0, 0.5, -0.5, 1.0], 44_100);
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(bytes.len(), 44 + 8);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 8);
    }
}
