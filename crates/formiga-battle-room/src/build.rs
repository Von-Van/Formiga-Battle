//! Which Battle and which Desktop a side plays with, so that only the same, unaltered copies play
//! each other.
//!
//! Every side says three things when it opens or joins a room: Battle's version, the version of
//! the Formiga Desktop that opened it (none in a rehearsal), and the SHA-256 of Battle's own
//! program file. A joiner must say the same versions as the friend who opened the room, and the
//! same digest, unless the relay knows both digests as builds of that version as it was
//! released (one per platform). Once the relay knows a version's released builds, a copy of that
//! version that is none of them cannot open or join a room at all.
//!
//! None of this can stop a copy changed on purpose to say what a released one says: whatever a
//! program reports about itself, a changed program can report too. It keeps out the honest
//! mistakes (an old Battle, another Desktop, a build from somewhere else) and makes changing
//! Battle and still playing a deliberate effort. What keeps a match honest when someone does make
//! that effort is the board both windows report after every turn: a copy that works a turn out
//! differently is caught at once.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

/// The most characters a version may have.
pub const MAX_VERSION_CHARS: usize = 24;

/// What a side plays with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Build {
    /// Battle's version.
    pub battle: String,
    /// The version of the Formiga Desktop that opened Battle; `None` in a rehearsal, when no
    /// Desktop did.
    #[serde(default)]
    pub desktop: Option<String>,
    /// The SHA-256 of Battle's program file, as 64 lowercase hex digits. The same for everyone
    /// with the same copy, so it says nothing about the computer.
    pub digest: String,
}

impl Build {
    /// This Battle, opened by the Desktop of `desktop`'s version (or none). Its digest is left
    /// empty: reading the program file takes a moment, so [`Build::digested`] fills it in off the
    /// window's thread.
    pub fn this(desktop: Option<String>) -> Self {
        Self {
            battle: env!("CARGO_PKG_VERSION").to_owned(),
            desktop,
            digest: String::new(),
        }
    }

    /// The same, with the digest of the program that is running.
    pub fn digested(self) -> std::io::Result<Self> {
        let digest = program_digest()?;
        Ok(Self { digest, ..self })
    }

    /// Whether it has the shape the relay reads: versions of letters, digits and `.+-` only, and
    /// a digest of 64 lowercase hex digits.
    pub fn well_formed(&self) -> bool {
        let version = |v: &str| {
            !v.is_empty()
                && v.chars().count() <= MAX_VERSION_CHARS
                && v.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-'))
        };
        version(&self.battle)
            && self.desktop.as_deref().is_none_or(version)
            && self.digest.len() == 64
            && self
                .digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
}

/// The SHA-256 of the program that is running.
pub fn program_digest() -> std::io::Result<String> {
    let mut file = std::fs::File::open(std::env::current_exe()?)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// The builds of each Battle version as it was released, by version: the digests the relay
/// knows. Empty until a release publishes its digests; a version with no entry is a development
/// build, and plays only a copy with the very same digest.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Released(pub BTreeMap<String, BTreeSet<String>>);

impl Released {
    /// Whether a build may enter a room at all: once its version's release is known, only as one
    /// of that release's builds.
    pub fn admits(&self, build: &Build) -> bool {
        self.0
            .get(&build.battle)
            .is_none_or(|digests| digests.contains(&build.digest))
    }

    /// Why `joiner` may not play `opener`, if it may not. Both are taken to be admitted already.
    pub fn mismatch(&self, opener: &Build, joiner: &Build) -> Option<Mismatch> {
        if joiner.battle != opener.battle || joiner.desktop != opener.desktop {
            return Some(Mismatch::Versions);
        }
        let both_released = self.0.contains_key(&opener.battle);
        (joiner.digest != opener.digest && !both_released).then_some(Mismatch::Build)
    }
}

/// How two sides' builds differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mismatch {
    /// Another Battle or another Desktop.
    Versions,
    /// The same versions, built differently, and not both builds of one release.
    Build,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(battle: &str, desktop: Option<&str>, digest: char) -> Build {
        Build {
            battle: battle.to_owned(),
            desktop: desktop.map(str::to_owned),
            digest: digest.to_string().repeat(64),
        }
    }

    #[test]
    fn the_running_program_has_a_digest_of_the_right_shape() {
        let this = Build::this(Some("0.67.7".to_owned())).digested().unwrap();
        assert!(this.well_formed(), "{this:?}");
        assert_eq!(this.battle, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn a_version_or_digest_of_another_shape_is_not_read() {
        assert!(build("0.1.0", Some("0.67.7"), 'a').well_formed());
        assert!(build("0.1.0", None, 'a').well_formed());
        assert!(!build("", None, 'a').well_formed());
        assert!(!build("0.1.0 beta", None, 'a').well_formed());
        assert!(!build("0.1.0", Some(&"9".repeat(25)), 'a').well_formed());
        assert!(!build("0.1.0", None, 'A').well_formed());
        assert!(!build("0.1.0", None, 'g').well_formed());
    }

    #[test]
    fn a_development_build_plays_only_the_very_same_copy() {
        let known = Released::default();
        let mine = build("0.1.0", Some("0.67.7"), 'a');
        assert!(known.admits(&mine));
        assert_eq!(known.mismatch(&mine, &mine.clone()), None);
        assert_eq!(
            known.mismatch(&mine, &build("0.1.0", Some("0.67.7"), 'b')),
            Some(Mismatch::Build)
        );
    }

    #[test]
    fn another_battle_or_desktop_or_a_rehearsal_is_another_version() {
        let known = Released::default();
        let mine = build("0.1.0", Some("0.67.7"), 'a');
        for other in [
            build("0.1.1", Some("0.67.7"), 'a'),
            build("0.1.0", Some("0.67.6"), 'a'),
            build("0.1.0", None, 'a'),
        ] {
            assert_eq!(known.mismatch(&mine, &other), Some(Mismatch::Versions));
        }
    }

    #[test]
    fn a_released_version_admits_only_its_own_builds_and_they_play_each_other() {
        let mut known = Released::default();
        known.0.insert(
            "0.2.0".to_owned(),
            ["a".repeat(64), "b".repeat(64)].into_iter().collect(),
        );
        let mac = build("0.2.0", Some("0.68.0"), 'a');
        let windows = build("0.2.0", Some("0.68.0"), 'b');
        assert!(known.admits(&mac) && known.admits(&windows));
        assert!(!known.admits(&build("0.2.0", Some("0.68.0"), 'c')));
        assert_eq!(known.mismatch(&mac, &windows), None);
        // Another version's release says nothing about a development build.
        assert!(known.admits(&build("0.3.0", None, 'c')));
    }
}
