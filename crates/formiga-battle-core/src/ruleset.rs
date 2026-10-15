//! The numbers a match is played by. Every one is provisional until playtests settle it, so they
//! live together here, travel with every recorded match, and are named by a hash that an online
//! room checks before two clients play.

use serde::{Deserialize, Serialize};

/// The version of the rules themselves: what the code does with a ruleset. A change to how
/// anything resolves bumps it, so a recorded match from an older build is refused rather than
/// replayed differently.
pub const RULES_VERSION: u32 = 3;

/// When holding the zone counts toward the beacon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureRule {
    /// A team holding in the zone with nobody at all from the other team in it: the handoff's
    /// first reading. Under AI play both teams nearly always have someone in the zone, and well
    /// under one match in a hundred is won by capture.
    Uncontested,
    /// More of a team holding in the zone than the other team has in it, holding or not. The
    /// default: about three matches in five are won by capture, in about ten turns.
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
    /// The handoff's opening proposal, with two settings of its open questions decided by
    /// tournament: capture is counted over two full turns of holding (at three ticks a turn, its
    /// suggested three ticks would go to whoever reached the yard first), and a team controls the
    /// zone by outnumbering the other team in it, since with nobody allowed in it at all almost
    /// no match was ever won by capture.
    fn default() -> Self {
        Self {
            composure: 3,
            ticks_per_turn: 3,
            capture_ticks: 6,
            capture_rule: CaptureRule::Majority,
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
