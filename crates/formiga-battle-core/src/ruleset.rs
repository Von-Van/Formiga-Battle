//! The numbers a match is played by. Every one is provisional until playtests settle it, so they
//! live together here, travel with every recorded match, and are named by a hash that an online
//! room checks before two clients play.

use serde::{Deserialize, Serialize};

/// The version of the rules themselves: what the code does with a ruleset. A change to how
/// anything resolves bumps it, so a recorded match from an older build is refused rather than
/// replayed differently.
pub const RULES_VERSION: u32 = 8;

/// When holding the zone counts toward the beacon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureRule {
    /// A team holding in the zone while nobody of the other team stands in it unbroken: the
    /// handoff's reading, and Jake's, that progress pauses when the other team gets in. The
    /// default since rules version 4, when flanking gave a team a way to break holders and so
    /// to clear the zone.
    Uncontested,
    /// More of a team holding in the zone than the other team has in it unbroken, holding or
    /// not. The default before rules version 4; a team that only ever holds wins by it before
    /// anyone can flank it.
    Majority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ruleset {
    /// Composure every creature starts with, the same for everyone. Zero is a cartoon knockout.
    pub composure: u8,
    /// Ticks a turn resolves in. Every creature takes at most one step a tick.
    pub ticks_per_turn: u8,
    /// Ticks of controlling the beacon, in total, that win by capture.
    pub capture_ticks: u8,
    /// When holding the zone counts toward the beacon.
    pub capture_rule: CaptureRule,
    /// Whether a team's capture progress falls while the other team holds the beacon.
    pub capture_decays: bool,
    /// The last turn. A match still going after it is decided by `Outcome::TurnCap` rules.
    pub turn_cap: u8,
    /// The most composure one creature can lose in one tick, however many are pressing it.
    pub damage_cap: u8,
    /// Sideways steps a creature may take in one turn to keep to a route it prefers. A detour
    /// costs time, so a route preference is a tradeoff and never a free advantage.
    pub detour_budget: u8,
    /// Seconds a player has to give orders, for clients and the room service; the rules
    /// themselves never look at a clock.
    pub turn_seconds: u16,
}

impl Default for Ruleset {
    /// The handoff's opening proposal, with its open questions decided by tournament: capture is
    /// the handoff's three ticks of holding the zone with nobody of the other team in it, which
    /// with three ticks a turn is one whole turn of having it to yourselves.
    fn default() -> Self {
        Self {
            composure: 3,
            ticks_per_turn: 3,
            capture_ticks: 3,
            capture_rule: CaptureRule::Uncontested,
            capture_decays: false,
            turn_cap: 12,
            damage_cap: 2,
            detour_budget: 2,
            turn_seconds: 40,
        }
    }
}

impl Ruleset {
    /// Names this ruleset and the rules version exactly, for a room to compare before two clients
    /// play and for a replay to check before it runs.
    pub fn hash(&self) -> u64 {
        let bytes = serde_json::to_vec(&(RULES_VERSION, self)).expect("a ruleset serializes");
        crate::rng::fnv1a(&bytes)
    }
}
