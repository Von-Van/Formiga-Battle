//! Formiga Battle's events: the competitions a colony enters at the Battleground. A bracket is
//! data, resumable from a file between matches; a fixture between two Handlers nobody is playing
//! is played out under the same rules as any other match (`formiga_battle_core::tournament::play`)
//! and recorded the same way. Nothing here draws, keeps time or talks to a network, and nothing
//! here changes how a match is played: an event only decides who meets whom.

pub mod bracket;
pub mod fixture;
pub mod season;

pub use bracket::{Bracket, BracketError, Decided, Fixture, FixtureResult};
pub use season::{Kind, Season, Stage};
