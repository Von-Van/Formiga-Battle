//! What Battle keeps of its own, in one small file in its own data folder: how loud it plays,
//! whether it is muted, whether it moves, and how large its window was, so the next sitting opens
//! the way the last one closed. Only how Battle looks and sounds is kept here. The colony's record
//! is Desktop's to keep (`docs/REWARDS.md`), and nothing about a match ever comes here.
//!
//! A window opened only for its picture, and a picture made without one, read and keep nothing,
//! so a review looks the same on every machine.

use crate::sound::Levels;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FILE: &str = "settings.json";

/// The file's shape. A newer Battle may add to it; an older one reads what it knows and keeps the
/// rest at its defaults.
const VERSION: u32 = 1;

/// How long the settings stay put before they are written, so a dial dragged across, or a window
/// dragged larger, is written once rather than every frame.
const SETTLE_SECS: f32 = 0.5;

/// The smallest and largest window remembered, in points. The smallest is the window's own least
/// size; anything outside these is forgotten and the window opens at its usual size.
const SMALLEST: [f32; 2] = [1024.0, 640.0];
const LARGEST: f32 = 8000.0;

/// Battle's data folder: `FORMIGA_BATTLE_DATA_DIR` if it is set, for development and tests, as
/// Hill and Home have theirs; otherwise the platform's usual place (Application Support on macOS,
/// `%APPDATA%` on Windows).
pub fn folder() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("FORMIGA_BATTLE_DATA_DIR").filter(|dir| !dir.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    directories::ProjectDirs::from("com", "Formiga", "Formiga Battle")
        .map(|dirs| dirs.data_dir().to_owned())
}

/// A window size worth opening at again; a minimised window's, say, is not.
fn sane(&[w, h]: &[f32; 2]) -> bool {
    w.is_finite()
        && h.is_finite()
        && (SMALLEST[0]..=LARGEST).contains(&w)
        && (SMALLEST[1]..=LARGEST).contains(&h)
}

/// How Battle looks and sounds, as last left.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub music: f32,
    pub sounds: f32,
    pub muted: bool,
    /// Reduced motion as the Handler last set it in Battle. None if they never have, and then
    /// Desktop's setting stands, whatever it is that day.
    pub reduce_motion: Option<bool>,
    /// The window's size inside its frame, in points.
    pub window: Option<[f32; 2]>,
}

impl Default for Settings {
    fn default() -> Self {
        let levels = Levels::default();
        Self {
            version: VERSION,
            music: levels.music,
            sounds: levels.sounds,
            muted: levels.muted,
            reduce_motion: None,
            window: None,
        }
    }
}

impl Settings {
    /// The settings kept in `folder`. A missing or unreadable file, or a value that makes no
    /// sense, is the usual setting instead; nothing is refused for it.
    pub fn load(folder: &Path) -> Self {
        let read = std::fs::read(folder.join(FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok());
        let Some(read) = read else {
            return Self::default();
        };
        let usual = Self::default();
        let level = |value: f32, usual: f32| {
            if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                usual
            }
        };
        Self {
            version: VERSION,
            music: level(read.music, usual.music),
            sounds: level(read.sounds, usual.sounds),
            muted: read.muted,
            reduce_motion: read.reduce_motion,
            window: read.window.filter(sane),
        }
    }

    pub fn save(&self, folder: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(folder)?;
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        formiga_travel::write_atomically(&folder.join(FILE), &bytes)
    }

    pub fn levels(&self) -> Levels {
        Levels {
            music: self.music,
            sounds: self.sounds,
            muted: self.muted,
        }
    }
}

/// Keeps the settings as the window changes them: written once they have settled, and at once
/// when the window closes.
pub struct Keeper {
    folder: PathBuf,
    /// As the file has them.
    written: Settings,
    /// A change not yet written, and when it was last changed.
    pending: Option<(Settings, f32)>,
    /// Reduced motion, if the Handler has chosen it in Battle.
    motion: Option<bool>,
}

impl Keeper {
    /// The settings kept in `folder`, ready to be kept again.
    pub fn open(folder: PathBuf) -> Self {
        let written = Settings::load(&folder);
        Self {
            folder,
            written,
            pending: None,
            motion: written.reduce_motion,
        }
    }

    /// The settings as Battle opened them.
    pub fn settings(&self) -> Settings {
        self.written
    }

    /// The Handler turned reduced motion on or off, here in Battle.
    pub fn choose_motion(&mut self, reduce: bool) {
        self.motion = Some(reduce);
    }

    /// The settings as they are this frame, at `now` seconds. `window` is the window's size if
    /// it is known; `closing`, that the window is about to go, writes any change at once.
    pub fn note(&mut self, levels: Levels, window: Option<[f32; 2]>, now: f32, closing: bool) {
        let live = Settings {
            music: levels.music,
            sounds: levels.sounds,
            muted: levels.muted,
            reduce_motion: self.motion,
            window: window.filter(sane).or(self.written.window),
            ..self.written
        };
        if live == self.written {
            self.pending = None;
            return;
        }
        let since = match self.pending {
            Some((pending, at)) if pending == live => at,
            _ => now,
        };
        self.pending = Some((live, since));
        if closing || now - since >= SETTLE_SECS {
            if let Err(error) = live.save(&self.folder) {
                eprintln!("formiga-battle: could not keep the settings: {error}");
            }
            // Written or not, this change is not tried again every frame; the next one is.
            self.written = live;
            self.pending = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("formiga-battle-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn quiet() -> Levels {
        Levels {
            music: 0.2,
            sounds: 0.4,
            muted: true,
        }
    }

    #[test]
    fn a_fresh_battle_starts_at_the_usual_settings_and_writes_nothing() {
        let folder = scratch("fresh");
        let mut keeper = Keeper::open(folder.clone());
        assert_eq!(keeper.settings(), Settings::default());
        keeper.note(Levels::default(), None, 10.0, true);
        assert!(!folder.exists(), "nothing changed, so nothing is written");
    }

    #[test]
    fn the_sound_motion_and_window_are_there_next_time() {
        let folder = scratch("round-trip");
        let mut keeper = Keeper::open(folder.clone());
        keeper.choose_motion(true);
        keeper.note(quiet(), Some([1400.0, 900.0]), 1.0, true);
        let next = Keeper::open(folder.clone()).settings();
        assert_eq!(next.levels(), quiet());
        assert_eq!(next.reduce_motion, Some(true));
        assert_eq!(next.window, Some([1400.0, 900.0]));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_dial_dragged_across_is_written_once_it_settles() {
        let folder = scratch("settle");
        let mut keeper = Keeper::open(folder.clone());
        for (i, music) in [0.6, 0.5, 0.4, 0.3].into_iter().enumerate() {
            let levels = Levels {
                music,
                ..Levels::default()
            };
            keeper.note(levels, None, i as f32 * 0.1, false);
        }
        assert!(
            !folder.join(FILE).exists(),
            "still moving, so not yet written"
        );
        let settled = Levels {
            music: 0.3,
            ..Levels::default()
        };
        keeper.note(settled, None, 0.3 + SETTLE_SECS, false);
        assert_eq!(Settings::load(&folder).music, 0.3);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn reduced_motion_is_only_kept_once_the_handler_chooses_it() {
        let folder = scratch("motion");
        let mut keeper = Keeper::open(folder.clone());
        keeper.note(quiet(), None, 1.0, true);
        assert_eq!(Settings::load(&folder).reduce_motion, None);
        keeper.choose_motion(false);
        keeper.note(quiet(), None, 2.0, true);
        assert_eq!(Settings::load(&folder).reduce_motion, Some(false));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_minimised_window_leaves_the_size_it_had() {
        let folder = scratch("minimised");
        let mut keeper = Keeper::open(folder.clone());
        keeper.note(Levels::default(), Some([1300.0, 800.0]), 1.0, true);
        keeper.note(Levels::default(), Some([0.0, 0.0]), 2.0, true);
        assert_eq!(Settings::load(&folder).window, Some([1300.0, 800.0]));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn nonsense_in_the_file_is_forgotten_not_refused() {
        let folder = scratch("nonsense");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(
            folder.join(FILE),
            br#"{"version": 9, "music": 7.0, "sounds": -1.0, "window": [2, 2], "later": true}"#,
        )
        .unwrap();
        let read = Settings::load(&folder);
        assert_eq!((read.music, read.sounds), (1.0, 0.0));
        assert_eq!(read.window, None);
        assert_eq!(read.version, VERSION);
        std::fs::write(folder.join(FILE), b"not json").unwrap();
        assert_eq!(Settings::load(&folder), Settings::default());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
