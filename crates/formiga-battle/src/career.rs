//! The colony's career at the Battleground: what Battle keeps of its own about the matches a
//! colony has played against the named rivals (`rival.rs`), in one small file in Battle's data
//! folder, one per colony. It is the one record Battle keeps itself (Jake, 2026-10-23: the
//! career lives in Battle, keyed by the colony, until a Desktop contract carries it): cups,
//! points, the notebook and the trios stay with Desktop, as `docs/REWARDS.md` says. A match is
//! applied once, by the same id Desktop applies it by, so a crash between the two writes never
//! counts a match twice. A rehearsal, with no Desktop behind it, keeps no career.

use formiga_battle_core::playbook::{MAX_STAGES, Playbook};
use formiga_battle_world::{Bracket, Season};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The file's shape. A newer Battle may add to it; an older one reads what it knows.
pub const VERSION: u32 = 1;

/// Match ids remembered, so a match is never counted twice; the oldest are let go.
const APPLIED_KEPT: usize = 512;

/// How a colony has fared against one rival.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HeadToHead {
    pub played: u32,
    pub won: u32,
    pub drawn: u32,
    pub lost: u32,
    /// The last meeting: when, where, and how it went for the colony.
    pub last: Option<Meeting>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meeting {
    pub match_id: String,
    pub when_utc: String,
    pub arena: String,
    pub won: Option<bool>,
}

/// An event the colony has entered: its bracket, where it is played, and what it is called.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entered {
    pub name: String,
    pub arena: String,
    pub bracket: Bracket,
}

/// An event the colony won.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Title {
    pub event: String,
    pub arena: String,
    /// The final's match.
    pub match_id: String,
    pub when_utc: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Career {
    pub version: u32,
    pub colony_id: String,
    /// By rival id.
    pub rivals: BTreeMap<String, HeadToHead>,
    /// Match ids applied, oldest first.
    pub applied: Vec<String>,
    /// The event being played, or the last one finished.
    pub event: Option<Entered>,
    /// Events entered, in all.
    pub events_entered: u32,
    /// Events won, oldest first.
    pub titles: Vec<Title>,
    /// The season under way, or the last finished.
    pub season: Option<Season>,
    /// Every season's champion, oldest first: rivals and the colony alike.
    pub hall: Vec<Champion>,
    /// What the Battleground has noticed about the colony, oldest first, each with the matches
    /// that show it.
    pub recognition: Vec<Recognition>,
    /// Each trio's playbooks, by trio id.
    pub playbooks: BTreeMap<String, Shelf>,
}

/// The most playbooks one trio keeps.
pub const PLAYBOOKS_KEPT: usize = 3;

/// A trio's playbooks, and the one it plays by, if any.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shelf {
    pub books: Vec<Playbook>,
    /// Which of `books` its orders are filled in from when it plays the visitors.
    pub playing: Option<usize>,
}

impl Shelf {
    /// The playbook it plays by, if one is chosen and can be played.
    pub fn in_play(&self) -> Option<&Playbook> {
        self.playing
            .and_then(|i| self.books.get(i))
            .filter(|p| p.validate().is_ok())
    }

    /// Keeps it to what Battle can play: no more than `PLAYBOOKS_KEPT`, each with no more than
    /// `MAX_STAGES` stages, and the one in play one of them.
    pub fn tidy(&mut self) {
        self.books.truncate(PLAYBOOKS_KEPT);
        for book in &mut self.books {
            book.stages.truncate(MAX_STAGES);
            book.name = book
                .name
                .chars()
                .take(formiga_battle_core::playbook::NAME_LEN)
                .collect();
        }
        if self.playing.is_some_and(|i| i >= self.books.len()) {
            self.playing = None;
        }
    }
}

/// A season's champion, for the Hall of Champions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Champion {
    pub season: u32,
    /// "you" for the colony, or a rival's id.
    pub who: String,
    pub event: String,
}

/// Something the Battleground noticed, backed by the matches that show it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Noticed {
    /// The first win over a rival.
    FirstWin { rival: String },
    /// A win over a rival seeded above the colony in an event.
    Upset {
        rival: String,
        event: String,
        season: u32,
    },
    /// A fifth meeting with the same rival.
    Rivalry { rival: String },
    /// An event won.
    EventWon { event: String, season: u32 },
    /// The season's Championship won for the first time.
    SeasonChampion { season: u32 },
    /// The Championship won again by its holder.
    TitleDefended { season: u32 },
    /// The Championship lost by its holder.
    TitleLost { season: u32, to: String },
    /// The Championship won back after losing it.
    TitleReclaimed { season: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recognition {
    #[serde(flatten)]
    pub noticed: Noticed,
    /// The matches that show it.
    pub evidence: Vec<String>,
    pub when_utc: String,
}

/// Recognition kept; the oldest are let go, the Hall and titles keep the rest.
const NOTICED_KEPT: usize = 64;

/// The colony's standing at the Battleground, from what it has done, never from how much it has
/// played, with how the next is earned.
pub fn standing(career: &Career) -> (&'static str, &'static str) {
    let championships = career.hall.iter().filter(|c| c.who == "you").count();
    let reached_final = !career.titles.is_empty()
        || career
            .recognition
            .iter()
            .any(|r| matches!(r.noticed, Noticed::TitleLost { .. }))
        || career.event.as_ref().is_some_and(|e| {
            let b = &e.bracket;
            b.entrants
                .iter()
                .position(|x| x == "you")
                .is_some_and(|you| b.reached(you).0 + 1 == b.rounds())
        });
    if championships >= 3 {
        ("Legend of the Battleground", "Three Championships won")
    } else if championships >= 1 {
        ("Season Champion", "Win three Championships to be a Legend")
    } else if !career.titles.is_empty() {
        (
            "Champion",
            "Win a season's Championship to be Season Champion",
        )
    } else if reached_final {
        ("Contender", "Win an event to be a Champion")
    } else {
        ("Newcomer", "Reach a final to be a Contender")
    }
}

impl Career {
    /// Notes something the Battleground noticed, once: the same notice twice is kept once.
    pub fn notice(&mut self, noticed: Noticed, evidence: Vec<String>, when_utc: &str) {
        if self.recognition.iter().any(|r| r.noticed == noticed) {
            return;
        }
        self.recognition.push(Recognition {
            noticed,
            evidence,
            when_utc: when_utc.to_owned(),
        });
        if self.recognition.len() > NOTICED_KEPT {
            self.recognition.remove(0);
        }
    }
}

impl Default for Career {
    fn default() -> Self {
        Self {
            version: VERSION,
            colony_id: String::new(),
            rivals: BTreeMap::new(),
            applied: Vec::new(),
            event: None,
            events_entered: 0,
            titles: Vec::new(),
            season: None,
            hall: Vec::new(),
            recognition: Vec::new(),
            playbooks: BTreeMap::new(),
        }
    }
}

/// A career as read, and whether it may be written back where it was read from.
#[derive(Debug)]
pub struct Loaded {
    pub career: Career,
    /// False when what is there could not be read and was left as it was: writing would lose it.
    pub keep: bool,
    /// What the Handler is told about how the career was read, when it was not simply read.
    pub notice: Option<String>,
}

/// Moves a career file that cannot be read out of the way, to a name no colony's file has, so a
/// fresh career never writes over it.
fn set_aside(path: &Path) -> std::io::Result<PathBuf> {
    let stem = path
        .file_stem()
        .map_or_else(|| "colony".to_owned(), |s| s.to_string_lossy().into_owned());
    let mut n = 1u32;
    let aside = loop {
        let aside = path.with_file_name(format!("{stem}.unreadable-{n}.json"));
        if !aside.exists() {
            break aside;
        }
        n += 1;
    };
    std::fs::rename(path, &aside)?;
    Ok(aside)
}

/// Where careers are kept, under Battle's data folder.
pub fn folder(data: &Path) -> PathBuf {
    data.join("careers")
}

/// The file for one colony: its id, kept to the characters a file name is safe with.
fn file(folder: &Path, colony_id: &str) -> PathBuf {
    let safe: String = colony_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect();
    folder.join(format!(
        "{}.json",
        if safe.is_empty() { "colony" } else { &safe }
    ))
}

impl Career {
    /// A colony's career, as kept in `folder`. Only a career that is not there yet starts
    /// fresh. One that cannot be read is never written over: a file that is not a career is set
    /// aside under another name first, and one that cannot be read at all, or is another
    /// colony's, is not written this session. Either way the match goes on.
    pub fn load(folder: &Path, colony_id: &str) -> Loaded {
        let fresh = || Self {
            colony_id: colony_id.to_owned(),
            ..Self::default()
        };
        let path = file(folder, colony_id);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Loaded {
                    career: fresh(),
                    keep: true,
                    notice: None,
                };
            }
            Err(error) => {
                return Loaded {
                    career: fresh(),
                    keep: false,
                    notice: Some(format!(
                        "Your record against the rivals could not be read, so this session will not \
                         keep it: {error}"
                    )),
                };
            }
        };
        match serde_json::from_slice::<Self>(&bytes) {
            Ok(mut career) if career.colony_id == colony_id => {
                career.playbooks.values_mut().for_each(Shelf::tidy);
                Loaded {
                    career,
                    keep: true,
                    notice: None,
                }
            }
            // Another colony's, under the same safe file name: theirs to keep, so never written
            // over from here.
            Ok(_) => Loaded {
                career: fresh(),
                keep: false,
                notice: None,
            },
            Err(_) => match set_aside(&path) {
                Ok(aside) => Loaded {
                    career: fresh(),
                    keep: true,
                    notice: Some(format!(
                        "Your record against the rivals could not be read, so it starts afresh; the \
                         old file is kept as {}.",
                        aside.file_name().map_or_else(
                            || aside.display().to_string(),
                            |n| n.to_string_lossy().into_owned()
                        )
                    )),
                },
                Err(error) => Loaded {
                    career: fresh(),
                    keep: false,
                    notice: Some(format!(
                        "Your record against the rivals could not be read, so this session will not \
                         keep it: {error}"
                    )),
                },
            },
        }
    }

    pub fn save(&self, folder: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(folder)?;
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        formiga_travel::write_atomically(&file(folder, &self.colony_id), &bytes)
    }

    /// Matches against the rivals, in all.
    pub fn meetings(&self) -> u32 {
        self.rivals.values().map(|h| h.played).sum()
    }

    pub fn against(&self, rival: &str) -> HeadToHead {
        self.rivals.get(rival).cloned().unwrap_or_default()
    }

    /// Counts a match against `rival`, once: a second call with the same id changes nothing.
    /// True if it counted.
    pub fn apply(
        &mut self,
        match_id: &str,
        rival: &str,
        arena: &str,
        won: Option<bool>,
        when_utc: &str,
    ) -> bool {
        if self.applied.iter().any(|id| id == match_id) {
            return false;
        }
        self.applied.push(match_id.to_owned());
        if self.applied.len() > APPLIED_KEPT {
            self.applied.drain(..self.applied.len() - APPLIED_KEPT);
        }
        let h = self.rivals.entry(rival.to_owned()).or_default();
        h.played += 1;
        match won {
            Some(true) => h.won += 1,
            Some(false) => h.lost += 1,
            None => h.drawn += 1,
        }
        h.last = Some(Meeting {
            match_id: match_id.to_owned(),
            when_utc: when_utc.to_owned(),
            arena: arena.to_owned(),
            won,
        });
        true
    }
}

/// "First meeting with Hazel Thorne", or "Third meeting with Hazel Thorne: you lead 2 to 0";
/// `before` is the record as it stood when the match began.
pub fn meeting_words(before: &HeadToHead, name: &str, short: &str) -> String {
    let nth = match before.played + 1 {
        1 => "First".to_owned(),
        2 => "Second".to_owned(),
        3 => "Third".to_owned(),
        4 => "Fourth".to_owned(),
        5 => "Fifth".to_owned(),
        n => format!("{n}th"),
    };
    if before.played == 0 {
        return format!("{nth} meeting with {name}");
    }
    let standing = if before.won > before.lost {
        format!("you lead {} to {}", before.won, before.lost)
    } else if before.lost > before.won {
        format!("{short} leads {} to {}", before.lost, before.won)
    } else {
        format!("level at {} each", before.won)
    };
    let drawn = match before.drawn {
        0 => String::new(),
        1 => ", 1 drawn".to_owned(),
        n => format!(", {n} drawn"),
    };
    format!("{nth} meeting with {name}: {standing}{drawn}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trios_playbooks_are_kept_and_read_back_within_bounds() {
        let dir = std::env::temp_dir().join(format!("battle-career-books-{}", std::process::id()));
        let folder = folder(&dir);
        let mut career = Career::load(&folder, "colony-books").career;
        let samples = Playbook::samples();
        career.playbooks.insert(
            "0123456789abcdef".to_owned(),
            Shelf {
                books: samples.clone(),
                playing: Some(4),
            },
        );
        career.save(&folder).unwrap();
        let read = Career::load(&folder, "colony-books").career;
        let shelf = &read.playbooks["0123456789abcdef"];
        assert_eq!(shelf.books.len(), PLAYBOOKS_KEPT, "three kept at most");
        assert_eq!(shelf.books[..], samples[..PLAYBOOKS_KEPT]);
        assert_eq!(shelf.playing, None, "the one in play was not kept");
        assert_eq!(shelf.in_play(), None);
        // A career written before playbooks reads with none.
        let older = serde_json::json!({"version": 1, "colony_id": "colony-old"});
        std::fs::write(
            file(&folder, "colony-old"),
            serde_json::to_vec(&older).unwrap(),
        )
        .unwrap();
        assert!(
            Career::load(&folder, "colony-old")
                .career
                .playbooks
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_match_counts_once_by_its_id_and_the_file_round_trips() {
        let dir = std::env::temp_dir().join(format!("battle-career-{}", std::process::id()));
        let folder = folder(&dir);
        let mut career = Career::load(&folder, "colony-one").career;
        assert_eq!(career.meetings(), 0);
        assert!(career.apply(
            "a1",
            "hazel",
            "backyard",
            Some(true),
            "2026-10-23T00:00:00Z"
        ));
        assert!(!career.apply(
            "a1",
            "hazel",
            "backyard",
            Some(true),
            "2026-10-23T00:00:00Z"
        ));
        assert!(career.apply("a2", "hazel", "backyard", None, "2026-10-23T00:01:00Z"));
        assert!(career.apply("a3", "tam", "millpond", Some(false), "2026-10-23T00:02:00Z"));
        career.save(&folder).unwrap();
        let read = Career::load(&folder, "colony-one").career;
        assert_eq!(read, career);
        let h = read.against("hazel");
        assert_eq!((h.played, h.won, h.drawn, h.lost), (2, 1, 1, 0));
        assert_eq!(h.last.as_ref().map(|m| m.match_id.as_str()), Some("a2"));
        assert_eq!(read.meetings(), 3);
        // Another colony's career is its own file, and starts fresh.
        assert_eq!(Career::load(&folder, "colony-two").career.meetings(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_for_another_colony_is_not_read_as_this_one() {
        let dir = std::env::temp_dir().join(format!("battle-career-id-{}", std::process::id()));
        let folder = folder(&dir);
        let mut other = Career::load(&folder, "a/b").career;
        other.apply("x", "ivy", "woodland", Some(true), "2026-10-23T00:00:00Z");
        other.save(&folder).unwrap();
        // The same safe file name, a different colony: read as fresh rather than as theirs.
        let ab = Career::load(&folder, "ab");
        assert_eq!(ab.career.meetings(), 0);
        assert!(!ab.keep, "and never written over");
        assert_eq!(Career::load(&folder, "a/b").career.meetings(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_meeting_line_says_how_it_stands() {
        let mut h = HeadToHead::default();
        assert_eq!(
            meeting_words(&h, "Hazel Thorne", "Hazel"),
            "First meeting with Hazel Thorne"
        );
        h.played = 3;
        h.won = 2;
        h.lost = 1;
        assert_eq!(
            meeting_words(&h, "Hazel Thorne", "Hazel"),
            "Fourth meeting with Hazel Thorne: you lead 2 to 1"
        );
        h.lost = 2;
        h.drawn = 1;
        h.played = 5;
        assert_eq!(
            meeting_words(&h, "Hazel Thorne", "Hazel"),
            "6th meeting with Hazel Thorne: level at 2 each, 1 drawn"
        );
        h.lost = 3;
        assert_eq!(
            meeting_words(&h, "Hazel Thorne", "Hazel"),
            "6th meeting with Hazel Thorne: Hazel leads 3 to 2, 1 drawn"
        );
    }

    #[test]
    fn only_a_career_not_there_yet_starts_fresh_and_an_unreadable_one_is_never_written_over() {
        let dir =
            std::env::temp_dir().join(format!("battle-career-unreadable-{}", std::process::id()));
        let folder = folder(&dir);
        let missing = Career::load(&folder, "colony-new");
        assert!(missing.keep && missing.notice.is_none());
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(file(&folder, "colony-bad"), b"{ not a career").unwrap();
        let damaged = Career::load(&folder, "colony-bad");
        assert!(
            damaged.keep,
            "a fresh one may be kept once the old is out of the way"
        );
        assert!(damaged.notice.is_some());
        assert_eq!(damaged.career.meetings(), 0);
        let aside = folder.join("colony-bad.unreadable-1.json");
        assert_eq!(std::fs::read(&aside).unwrap(), b"{ not a career");
        assert!(!file(&folder, "colony-bad").exists());
        damaged.career.save(&folder).unwrap();
        assert_eq!(
            std::fs::read(&aside).unwrap(),
            b"{ not a career",
            "still there"
        );
        // A file that cannot be read at all (here a folder in its place) is left as it is, and
        // nothing is written this session.
        std::fs::create_dir_all(file(&folder, "colony-locked")).unwrap();
        let locked = Career::load(&folder, "colony-locked");
        assert!(!locked.keep);
        assert!(locked.notice.is_some());
        assert!(file(&folder, "colony-locked").is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
