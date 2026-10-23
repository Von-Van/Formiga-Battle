//! A season at the Battleground: a fixed run of events, the same shape every season so a colony
//! knows what it is playing for. An Open of four to begin, two Regionals of eight, and a
//! Championship of the four with the most season points, where last season's champion defends
//! their title. Every Handler earns season points from how far they went in the Open and the
//! Regionals, the colony and the rivals alike, so the four in the Championship are the four who
//! did best, and a colony that did not make it watches it played out under the same rules.
//!
//! Points are recognition of results, not a currency: they decide only who is seeded where and
//! who reaches the Championship, and a new season starts everyone from nothing.

use crate::bracket::Bracket;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What an event is in its season.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Four: the three Handlers who come to the place most, and the colony.
    Open,
    /// Eight: every Handler in the season. A season has two, in spring and in summer.
    Regional,
    /// Four: the season's best, last season's champion defending.
    Championship,
}

/// An event of the season, before it is played.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage {
    pub kind: Kind,
    pub arena: String,
    /// Which of the season's events it is, from 0.
    #[serde(default)]
    pub nth: u8,
}

impl Stage {
    pub fn size(&self) -> usize {
        match self.kind {
            Kind::Open | Kind::Championship => 4,
            Kind::Regional => 8,
        }
    }

    /// Its name, given the place's name and the season's number.
    pub fn name(&self, place: &str, season: u32) -> String {
        match self.kind {
            Kind::Open => format!("The {place} Open"),
            Kind::Regional if self.nth <= 1 => format!("The {place} Spring Regional"),
            Kind::Regional => format!("The {place} Summer Regional"),
            Kind::Championship => format!("The Season {season} Championship"),
        }
    }
}

/// Season points for going out in each round, counted back from the final: the champion, the
/// finalist, the semi-finalists and the quarter-finalists.
pub const POINTS: [u32; 4] = [8, 5, 3, 1];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Season {
    pub number: u32,
    pub stages: Vec<Stage>,
    /// Stages finished.
    pub played: usize,
    /// Season points by entrant id.
    pub points: BTreeMap<String, u32>,
    /// Last season's champion, defending the title.
    pub defending: Option<String>,
    /// This season's champion, once the Championship is played.
    pub champion: Option<String>,
}

impl Season {
    /// Season `number`, its events on the places open (`arenas`, never empty), turning a place
    /// further each season so the events move round.
    pub fn new(number: u32, arenas: &[String], defending: Option<String>) -> Self {
        let at = |k: usize| arenas[(number as usize - 1 + k) % arenas.len()].clone();
        let stages = vec![
            Stage {
                kind: Kind::Open,
                arena: at(0),
                nth: 0,
            },
            Stage {
                kind: Kind::Regional,
                arena: at(1),
                nth: 1,
            },
            Stage {
                kind: Kind::Regional,
                arena: at(2),
                nth: 2,
            },
            Stage {
                kind: Kind::Championship,
                arena: at(3),
                nth: 3,
            },
        ];
        Self {
            number,
            stages,
            played: 0,
            points: BTreeMap::new(),
            defending,
            champion: None,
        }
    }

    pub fn next_stage(&self) -> Option<&Stage> {
        self.stages.get(self.played)
    }

    pub fn finished(&self) -> bool {
        self.played >= self.stages.len()
    }

    pub fn points_of(&self, id: &str) -> u32 {
        self.points.get(id).copied().unwrap_or(0)
    }

    /// Who is in `stage`, first seed first. `pool` is every rival in the season, the place's own
    /// Handler first, then each in turn; `you` is the colony. An Open takes the first three of
    /// the pool and the colony, new to it, as fourth seed. A Regional takes everyone, seeded by
    /// season points, the defending champion first among equals, then the pool's order, the
    /// colony after the rivals it is level with. A Championship takes the four with most points
    /// the same way.
    pub fn entrants(&self, stage: &Stage, pool: &[String], you: &str) -> Vec<String> {
        if stage.kind == Kind::Open {
            let mut e: Vec<String> = pool.iter().take(3).cloned().collect();
            e.push(you.to_owned());
            return e;
        }
        let mut everyone: Vec<String> = pool.to_vec();
        everyone.push(you.to_owned());
        let place = |id: &String| everyone.iter().position(|e| e == id).unwrap_or(usize::MAX);
        let mut ranked = everyone.clone();
        ranked.sort_by_key(|id| {
            (
                std::cmp::Reverse(self.points_of(id)),
                self.defending.as_deref() != Some(id.as_str()),
                place(id),
            )
        });
        ranked.truncate(stage.size());
        ranked
    }

    /// Counts a finished event: season points for everyone in it, and the champion if it was
    /// the Championship. A bracket not finished counts nothing.
    pub fn finish(&mut self, bracket: &Bracket) {
        let Some(champion) = bracket.champion() else {
            return;
        };
        let kind = self.next_stage().map(|s| s.kind);
        if kind != Some(Kind::Championship) {
            let rounds = bracket.rounds();
            for (i, id) in bracket.entrants.iter().enumerate() {
                let (reached, _) = bracket.reached(i);
                let back = if i == champion {
                    0
                } else {
                    usize::from(rounds - reached)
                };
                let points = POINTS.get(back).copied().unwrap_or(0);
                *self.points.entry(id.clone()).or_default() += points;
            }
        } else {
            self.champion = Some(bracket.entrants[champion].clone());
        }
        self.played += 1;
    }

    /// Everyone in the season with points, most first.
    pub fn table(&self) -> Vec<(String, u32)> {
        let mut rows: Vec<(String, u32)> =
            self.points.iter().map(|(k, v)| (k.clone(), *v)).collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        rows
    }
}
