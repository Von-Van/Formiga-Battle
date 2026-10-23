//! Coaching styles: the ways a Handler who is not a person goes about a match. A style is the
//! opening book the Exhibition coach has always played from, with its chances set to make a
//! different Handler of it: one who commits everyone to clearing the yard, one who holds it and
//! shuts the board, one who goes round rather than through, and one who plays for what turns up.
//! Every number is a whole number and every chance is rolled from the match seed, so a styled
//! coach's match replays like any other. A style changes which orders a coach gives, never what
//! an order does, and the team shares the plan out by itself exactly as it would a Handler's.

use serde::{Deserialize, Serialize};

/// The version of the styles' numbers: a change bumps it, and a recorded match says which it was
/// played under, so a replay is read against the right book.
pub const STYLE_VERSION: u32 = 1;

/// The four ways of playing a coach can be told apart by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Archetype {
    /// Clears the yard: presses whoever is in it, chases whoever comes near, and sends everyone.
    Aggressor,
    /// Holds the yard with two and faces the other team, works the lever and the tap, and goes
    /// out only when it is safe to.
    Defender,
    /// Goes round rather than through: flanks and tricks holders, keeps its anchor, and leaves
    /// loose creatures alone.
    Tactician,
    /// Plays for what turns up: found things first, a tap when it soaks the other team, a steal
    /// when one wanders close.
    Opportunist,
}

impl Archetype {
    pub const ALL: [Self; 4] = [
        Self::Aggressor,
        Self::Defender,
        Self::Tactician,
        Self::Opportunist,
    ];

    /// The name a Handler reads.
    pub fn label(self) -> &'static str {
        match self {
            Self::Aggressor => "Aggressor",
            Self::Defender => "Defender",
            Self::Tactician => "Tactician",
            Self::Opportunist => "Opportunist",
        }
    }

    /// One line on how it plays, as the introduction says it.
    pub fn tendency(self) -> &'static str {
        match self {
            Self::Aggressor => "presses first and sends everyone",
            Self::Defender => "holds the yard and shuts the board",
            Self::Tactician => "goes round the sides and behind",
            Self::Opportunist => "plays for whatever turns up",
        }
    }

    pub fn book(self) -> Book {
        match self {
            Self::Aggressor => Book {
                flank: 90,
                trick: 85,
                press_loose: 95,
                second_home: 20,
                tap_from: 85,
                find_below: 15,
                chase_below: 75,
                lever_from: 95,
                item_use: 90,
                keep_home: 0,
                prefers_trick: false,
            },
            Self::Defender => Book {
                flank: 25,
                trick: 35,
                press_loose: 40,
                second_home: 95,
                tap_from: 35,
                find_below: 20,
                chase_below: 5,
                lever_from: 55,
                item_use: 60,
                keep_home: 1,
                prefers_trick: false,
            },
            Self::Tactician => Book {
                flank: 55,
                trick: 55,
                press_loose: 15,
                second_home: 60,
                tap_from: 65,
                find_below: 30,
                chase_below: 10,
                lever_from: 80,
                item_use: 70,
                keep_home: 0,
                prefers_trick: true,
            },
            Self::Opportunist => Book {
                flank: 35,
                trick: 55,
                press_loose: 90,
                second_home: 40,
                tap_from: 25,
                find_below: 85,
                chase_below: 90,
                lever_from: 70,
                item_use: 100,
                keep_home: 0,
                prefers_trick: false,
            },
        }
    }
}

/// A style's numbers: the opening book with each chance a percentage against a roll of 0 to 99.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Book {
    /// Flank a holder of theirs in the yard, when two hands are free.
    pub flank: u8,
    /// Otherwise trick a holder of theirs in the yard.
    pub trick: u8,
    /// Otherwise press one of theirs in the yard who is not holding.
    pub press_loose: u8,
    /// Keep a second of ours in the yard.
    pub second_home: u8,
    /// With what is left, in order: the tap when the roll is at least this,
    pub tap_from: u8,
    /// something lying near when the roll is below this,
    pub find_below: u8,
    /// one of theirs wandering close when the roll is below this,
    pub chase_below: u8,
    /// the lever when the roll is at least this; otherwise a second hand at the anchor.
    pub lever_from: u8,
    /// Use something carried, for each creature carrying something.
    pub item_use: u8,
    /// Hands never sent away from the anchor, beyond the one that holds it.
    pub keep_home: u8,
    /// Tricks a holder of theirs before thinking of flanking it, rather than after.
    pub prefers_trick: bool,
}

impl Book {
    /// The Exhibition coach's book, as it has always been: the numbers every style is a
    /// variation on, and the measure the others are told apart from.
    pub const EXHIBITION: Self = Self {
        flank: 60,
        trick: 75,
        press_loose: 80,
        second_home: 70,
        tap_from: 60,
        find_below: 35,
        chase_below: 45,
        lever_from: 85,
        item_use: 75,
        keep_home: 0,
        prefers_trick: false,
    };
}
