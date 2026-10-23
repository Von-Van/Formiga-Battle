//! The colony's career at the Battleground: what Battle keeps of its own about the matches a
//! colony has played against the named rivals (`rival.rs`), in one small file in Battle's data
//! folder, one per colony. It is the one record Battle keeps itself (Jake, 2026-10-23: the
//! career lives in Battle, keyed by the colony, until a Desktop contract carries it): cups,
//! points, the notebook and the trios stay with Desktop, as `docs/REWARDS.md` says. A match is
//! applied once, by the same id Desktop applies it by, so a crash between the two writes never
//! counts a match twice. A rehearsal, with no Desktop behind it, keeps no career.

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
        }
    }
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
    /// A colony's career, as kept in `folder`; a fresh one if there is none, or it cannot be
    /// read, so a damaged file costs the record and never the match.
    pub fn load(folder: &Path, colony_id: &str) -> Self {
        let read = std::fs::read(file(folder, colony_id))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(|c| c.colony_id == colony_id);
        read.unwrap_or_else(|| Self {
            colony_id: colony_id.to_owned(),
            ..Self::default()
        })
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
    fn a_match_counts_once_by_its_id_and_the_file_round_trips() {
        let dir = std::env::temp_dir().join(format!("battle-career-{}", std::process::id()));
        let folder = folder(&dir);
        let mut career = Career::load(&folder, "colony-one");
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
        let read = Career::load(&folder, "colony-one");
        assert_eq!(read, career);
        let h = read.against("hazel");
        assert_eq!((h.played, h.won, h.drawn, h.lost), (2, 1, 1, 0));
        assert_eq!(h.last.as_ref().map(|m| m.match_id.as_str()), Some("a2"));
        assert_eq!(read.meetings(), 3);
        // Another colony's career is its own file, and starts fresh.
        assert_eq!(Career::load(&folder, "colony-two").meetings(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_for_another_colony_is_not_read_as_this_one() {
        let dir = std::env::temp_dir().join(format!("battle-career-id-{}", std::process::id()));
        let folder = folder(&dir);
        let mut other = Career::load(&folder, "a/b");
        other.apply("x", "ivy", "woodland", Some(true), "2026-10-23T00:00:00Z");
        other.save(&folder).unwrap();
        // The same safe file name, a different colony: read as fresh rather than as theirs.
        assert_eq!(Career::load(&folder, "ab").meetings(), 0);
        assert_eq!(Career::load(&folder, "a/b").meetings(), 1);
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
}
