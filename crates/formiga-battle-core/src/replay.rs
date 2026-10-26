//! A whole match as data: the rules, the arena, the seed, both rosters and every turn's plans as
//! they were sent, with the board's hash after each turn, and the board it began from when that
//! was not the arena's own start (a lesson, a test). Replaying it runs the same match again and
//! stops at the first turn whose board differs, which is how a desync is found.

use crate::arena::Arena;
use crate::event::Outcome;
use crate::game::{Board, Match, MatchError, TurnRecord, TurnReport};
use crate::profile::Profile;
use crate::ruleset::{RULES_VERSION, Ruleset};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchRecord {
    pub rules_version: u32,
    pub ruleset: Ruleset,
    pub arena: Arena,
    pub seed: u64,
    pub rosters: [Vec<Profile>; 2],
    pub turns: Vec<TurnRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Outcome>,
    /// The board the first turn began from, only when it was not the arena's own start: a
    /// record without one, as every record before it was kept, begins where `Match::new` does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staged: Option<Board>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ReplayError {
    #[error("the match was played by rules version {0}; this build plays version {RULES_VERSION}")]
    RulesVersion(u32),
    #[error("the match cannot be set up: {0}")]
    Setup(#[from] MatchError),
    #[error(
        "turn {turn} came out differently (recorded {recorded:016x}, replayed {replayed:016x})"
    )]
    Desync {
        turn: u8,
        recorded: u64,
        replayed: u64,
    },
    #[error("the replay finished differently from the record")]
    Outcome,
}

impl Match {
    pub fn record(&self) -> MatchRecord {
        MatchRecord {
            rules_version: RULES_VERSION,
            ruleset: self.ruleset,
            arena: self.arena.clone(),
            seed: self.seed,
            rosters: self.rosters.clone().map(Vec::from),
            turns: self.history.clone(),
            outcome: self.outcome(),
            staged: self.staged(),
        }
    }

    /// The board the first turn began from, if it was set up by hand rather than by the arena.
    /// Before the first turn, the board as it stands, if it has been moved.
    pub fn staged(&self) -> Option<Board> {
        let began = self.start.as_ref().unwrap_or(&self.board);
        (*began != Self::starting_board(&self.arena, &self.ruleset)).then(|| began.clone())
    }
}

impl MatchRecord {
    /// Plays the match again, turn by turn, checking every board against the record. Gives the
    /// replayed match and every turn's report.
    pub fn replay(&self) -> Result<(Match, Vec<TurnReport>), ReplayError> {
        if self.rules_version != RULES_VERSION {
            return Err(ReplayError::RulesVersion(self.rules_version));
        }
        let mut m = Match::new(
            self.arena.clone(),
            self.ruleset,
            self.seed,
            self.rosters.clone(),
        )?;
        if let Some(staged) = &self.staged {
            m.board = staged.clone();
        }
        let mut reports = Vec::with_capacity(self.turns.len());
        for record in &self.turns {
            let report = match record.orders {
                Some(orders) => m.resolve_orders(orders),
                None => m.resolve_turn(record.submitted.clone()),
            };
            if report.hash != record.hash {
                return Err(ReplayError::Desync {
                    turn: report.turn,
                    recorded: record.hash,
                    replayed: report.hash,
                });
            }
            reports.push(report);
        }
        if let Some(outcome) = self.outcome
            && outcome.victory == Some(crate::event::Victory::Forfeit)
            && let Some(winner) = outcome.winner
        {
            m.forfeit(winner.other());
        }
        if m.outcome() != self.outcome {
            return Err(ReplayError::Outcome);
        }
        Ok((m, reports))
    }
}
