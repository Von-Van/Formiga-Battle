//! A single-elimination bracket of two, four, eight or sixteen entrants, seeded so the first two
//! seeds can meet only in the final. Each fixture is recorded once, by the id of the match that
//! settled it, so a bracket resumed after a crash never plays a fixture twice or credits a match
//! twice. A drawn fixture is settled by a toss rolled from the bracket's seed, the same on every
//! machine, and says so.

use formiga_battle_core::rng::{self, Purpose};
use serde::{Deserialize, Serialize};

/// The file's shape: a bracket kept by an older Battle reads as it was.
pub const VERSION: u32 = 1;

/// The most entrants a bracket takes.
pub const MAX_ENTRANTS: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum BracketError {
    #[error("a bracket takes 2, 4, 8 or 16 entrants, not {0}")]
    Size(usize),
    #[error("{0} is entered twice")]
    Twice(String),
    #[error("there is no fixture {0}")]
    NoFixture(usize),
    #[error("fixture {0} does not have both its entrants yet")]
    NotReady(usize),
    #[error("fixture {0} has already been played")]
    Played(usize),
}

/// How a fixture was settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decided {
    /// The beacon lit.
    Capture,
    /// All three of the other side dizzy.
    Knockout,
    /// The turn cap, on the tally.
    Cap,
    /// The other side gave it up.
    Forfeit,
    /// Drawn, and settled by the toss.
    Toss,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureResult {
    /// The match that settled it.
    pub match_id: String,
    /// 0 if the fixture's first entrant went through, 1 if its second.
    pub winner: u8,
    pub decided: Decided,
    pub turns: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fixture {
    /// 0 for the first round; the last round is the final.
    pub round: u8,
    /// The two entrants, by place in the bracket's list, once known.
    pub sides: [Option<usize>; 2],
    pub result: Option<FixtureResult>,
}

impl Fixture {
    /// The entrant who went through, once it is played.
    pub fn winner(&self) -> Option<usize> {
        self.result
            .as_ref()
            .and_then(|r| self.sides[usize::from(r.winner)])
    }

    /// Ready to be played: both entrants known, and not yet played.
    pub fn ready(&self) -> bool {
        self.result.is_none() && self.sides.iter().all(Option::is_some)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bracket {
    pub version: u32,
    pub id: String,
    pub seed: u64,
    /// The entrants' ids, first seed first.
    pub entrants: Vec<String>,
    /// Round by round, each round's fixtures top to bottom.
    pub fixtures: Vec<Fixture>,
}

/// The seeds in bracket order: for four, 1 v 4 above 2 v 3, so 1 and 2 meet only in the final.
fn seeding(n: usize) -> Vec<usize> {
    let mut order = vec![0];
    while order.len() < n {
        let size = order.len() * 2;
        order = order.iter().flat_map(|&s| [s, size - 1 - s]).collect();
    }
    order
}

impl Bracket {
    pub fn new(id: &str, seed: u64, entrants: Vec<String>) -> Result<Self, BracketError> {
        let n = entrants.len();
        if !(2..=MAX_ENTRANTS).contains(&n) || !n.is_power_of_two() {
            return Err(BracketError::Size(n));
        }
        for (i, e) in entrants.iter().enumerate() {
            if entrants[..i].contains(e) {
                return Err(BracketError::Twice(e.clone()));
            }
        }
        let order = seeding(n);
        let mut fixtures: Vec<Fixture> = order
            .chunks(2)
            .map(|pair| Fixture {
                round: 0,
                sides: [Some(pair[0]), Some(pair[1])],
                result: None,
            })
            .collect();
        let mut in_round = n / 2;
        let mut round = 1;
        while in_round > 1 {
            in_round /= 2;
            fixtures.extend((0..in_round).map(|_| Fixture {
                round,
                sides: [None, None],
                result: None,
            }));
            round += 1;
        }
        Ok(Self {
            version: VERSION,
            id: id.to_owned(),
            seed,
            entrants,
            fixtures,
        })
    }

    /// How many rounds, the final included.
    pub fn rounds(&self) -> u8 {
        self.entrants.len().trailing_zeros() as u8
    }

    /// The round's name, as a Handler reads it.
    pub fn round_name(&self, round: u8) -> &'static str {
        match self.rounds() - round {
            1 => "Final",
            2 => "Semi-finals",
            3 => "Quarter-finals",
            _ => "First round",
        }
    }

    /// One fixture of the round, as a Handler says it: "your semi-final".
    pub fn fixture_name(&self, round: u8) -> &'static str {
        match self.rounds() - round {
            1 => "final",
            2 => "semi-final",
            3 => "quarter-final",
            _ => "first-round match",
        }
    }

    /// Where the winner of fixture `index` goes: the next round's fixture and side.
    fn feeds(&self, index: usize) -> Option<(usize, usize)> {
        let round = self.fixtures.get(index)?.round;
        if round + 1 >= self.rounds() {
            return None;
        }
        let start = self.round_start(round);
        let next_start = self.round_start(round + 1);
        let place = index - start;
        Some((next_start + place / 2, place % 2))
    }

    fn round_start(&self, round: u8) -> usize {
        self.fixtures
            .iter()
            .position(|f| f.round == round)
            .unwrap_or(self.fixtures.len())
    }

    /// The first fixture ready to be played, top of the earliest round first.
    pub fn next(&self) -> Option<usize> {
        self.fixtures.iter().position(Fixture::ready)
    }

    /// The fixture ready to be played that has `entrant` in it.
    pub fn ready_for(&self, entrant: usize) -> Option<usize> {
        self.fixtures
            .iter()
            .position(|f| f.ready() && f.sides.contains(&Some(entrant)))
    }

    /// Records how fixture `index` went, once: `winner` is 0 or 1 for its first or second
    /// entrant, `None` for a draw, settled by the toss. Gives false, changing nothing, if this
    /// match has already been recorded.
    pub fn record(
        &mut self,
        index: usize,
        match_id: &str,
        winner: Option<u8>,
        decided: Decided,
        turns: u8,
    ) -> Result<bool, BracketError> {
        if self
            .fixtures
            .iter()
            .any(|f| f.result.as_ref().is_some_and(|r| r.match_id == match_id))
        {
            return Ok(false);
        }
        let fixture = self
            .fixtures
            .get(index)
            .ok_or(BracketError::NoFixture(index))?;
        if fixture.result.is_some() {
            return Err(BracketError::Played(index));
        }
        if !fixture.ready() {
            return Err(BracketError::NotReady(index));
        }
        let (winner, decided) = match winner {
            Some(w) => (w.min(1), decided),
            None => (self.toss(index), Decided::Toss),
        };
        self.fixtures[index].result = Some(FixtureResult {
            match_id: match_id.to_owned(),
            winner,
            decided,
            turns,
        });
        let through = self.fixtures[index].winner();
        if let Some((next, side)) = self.feeds(index) {
            self.fixtures[next].sides[side] = through;
        }
        Ok(true)
    }

    /// The toss that settles a drawn fixture: the same on every machine.
    pub fn toss(&self, index: usize) -> u8 {
        (rng::roll(self.seed, Purpose::TieBreak, &[1000, index as u64]) % 2) as u8
    }

    /// Who won the final, once it is played.
    pub fn champion(&self) -> Option<usize> {
        self.fixtures.last().and_then(Fixture::winner)
    }

    pub fn finished(&self) -> bool {
        self.champion().is_some()
    }

    /// The last round `entrant` played in, and whether they are still in.
    pub fn reached(&self, entrant: usize) -> (u8, bool) {
        let mut reached = 0;
        let mut still_in = true;
        for f in self
            .fixtures
            .iter()
            .filter(|f| f.sides.contains(&Some(entrant)))
        {
            reached = reached.max(f.round);
            if f.result.is_some() && f.winner() != Some(entrant) {
                still_in = false;
            }
        }
        if self.champion() == Some(entrant) {
            still_in = false;
        }
        (reached, still_in)
    }
}
