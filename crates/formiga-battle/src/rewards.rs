//! What a finished match earns, as Jake set it (2026-10-15): a match with a friend earns Open
//! Circuit points, a line in the match history, notebook entries and a match played for each of
//! the three; a match against the AI earns Championship progress, journal entries and keepsakes.
//! Nothing else earns anything. Every reward is handed to Desktop, which keeps it.

use formiga_battle_contract::{
    BattleRecord, MatchResult, MatchSummary, Mode, NotebookAddition, NotebookPage, Rewards, Showing,
};
use formiga_battle_core::notebook::{self, Page, Sign, observe};
use formiga_battle_core::{Event, Match, Side, Victory};
use time::OffsetDateTime;

/// The cup AI matches in the Backyard count toward.
pub const BACKYARD_CUP: &str = "cup.backyard";
/// What winning the Backyard Cup unlocks.
pub const WOODLAND_CUP: &str = "cup.woodland";
/// Wins against the AI that win a cup.
pub const CUP_WINS: u8 = 3;

/// A cup of the Championship Circuit, played against the visitors on its own arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cup {
    /// Its id in Desktop's record, which is also what unlocks it.
    pub id: &'static str,
    /// The arena it is played on.
    pub arena: &'static str,
    pub name: &'static str,
    /// Its trophy and medal, for Home's shelf.
    pub keepsakes: [&'static str; 2],
}

/// The cups in the order they open: the Backyard Cup from the start, and each one after it once
/// the one before is won.
pub const CUPS: [Cup; 4] = [
    Cup {
        id: BACKYARD_CUP,
        arena: "backyard",
        name: "Backyard Cup",
        keepsakes: ["battle.trophy.backyard_cup", "battle.medal.backyard_cup"],
    },
    Cup {
        id: WOODLAND_CUP,
        arena: "woodland",
        name: "Woodland Cup",
        keepsakes: ["battle.trophy.woodland_cup", "battle.medal.woodland_cup"],
    },
    Cup {
        id: "cup.millpond",
        arena: "millpond",
        name: "Millpond Cup",
        keepsakes: ["battle.trophy.millpond_cup", "battle.medal.millpond_cup"],
    },
    Cup {
        id: "cup.cove",
        arena: "cove",
        name: "Sandy Cove Cup",
        keepsakes: ["battle.trophy.cove_cup", "battle.medal.cove_cup"],
    },
];

/// The cup played on an arena, if any.
pub fn cup_for(arena: &str) -> Option<&'static Cup> {
    CUPS.iter().find(|c| c.arena == arena)
}

/// Whether a cup is open to play: the first always, any other once it is unlocked.
pub fn is_open(cup: &Cup, record: &BattleRecord) -> bool {
    cup.id == CUPS[0].id || record.unlocked.iter().any(|u| u == cup.id)
}

/// The arenas open for a match against the visitors, in the order their cups open. Friend
/// matches may be played on any arena that comes with Battle.
pub fn open_to_play(record: &BattleRecord) -> Vec<&'static str> {
    CUPS.iter()
        .filter(|c| is_open(c, record))
        .map(|c| c.arena)
        .collect()
}

/// Points for finishing a match with a friend, and more for winning or drawing it.
const FINISHED_POINTS: u32 = 10;
const WIN_POINTS: u32 = 5;
const DRAW_POINTS: u32 = 2;
/// Friend matches in one day before each earns only a token: playing the same friend over and
/// over is never the way to points.
const FULL_MATCHES_A_DAY: usize = 5;
const TOKEN_POINTS: u32 = 2;

fn names(m: &Match, side: Side) -> String {
    let names: Vec<&str> = m.rosters[side.index()]
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    match names.as_slice() {
        [a, b, c] => format!("{a}, {b} and {c}"),
        other => other.join(", "),
    }
}

fn ended_by(m: &Match) -> &'static str {
    match m.outcome().and_then(|o| o.victory) {
        Some(Victory::Capture) => "capture",
        Some(Victory::Knockout) => "knockout",
        Some(Victory::TurnCap) => "turn_cap",
        Some(Victory::Forfeit) => "forfeit",
        None => "draw",
    }
}

/// 128 bits from the system's randomness, as 32 hex digits: the match's id for Desktop.
fn match_id() -> String {
    let mut bytes = [0u8; 16];
    if getrandom::fill(&mut bytes).is_err() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        bytes = nanos.to_le_bytes();
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn to_contract(showing: &notebook::Showing) -> Showing {
    Showing {
        steps: showing.steps,
        signs: showing
            .signs
            .iter()
            .map(|(sign, n)| (sign.id().to_owned(), *n))
            .collect(),
        friends: showing.friends.clone(),
    }
}

/// A page Desktop keeps, read back for showing: its notes come from the same rules as ever.
pub fn page_for_showing(page: &NotebookPage, teammates: &[(u64, String)]) -> Page {
    Page {
        creature_id: page.creature_id,
        name: String::new(),
        matches: page.matches,
        wins: page.wins,
        recent: page
            .recent
            .iter()
            .map(|s| notebook::Showing {
                steps: s.steps,
                signs: s
                    .signs
                    .iter()
                    .filter_map(|(id, n)| Sign::from_id(id).map(|sign| (sign, *n)))
                    .collect(),
                friends: s.friends.clone(),
            })
            .collect(),
        moments: page.moments.clone(),
        teammates: teammates.iter().cloned().collect(),
    }
}

/// What a finished match earned, for the Handler's side.
pub struct Earned {
    pub result: MatchResult,
    /// The same, in words, for the window to show.
    pub lines: Vec<String>,
}

/// What a match with a friend earns.
pub fn friend_match(
    m: &Match,
    you: Side,
    events: &[Event],
    record: &BattleRecord,
    opponent: &str,
    now: OffsetDateTime,
) -> Earned {
    let outcome = m.outcome();
    let won = outcome.and_then(|o| o.winner).map(|w| w == you);
    let today = now.date();
    let played_today = record
        .history
        .iter()
        .filter(|h| h.mode == Mode::Friend && h.finished_at_utc.date() == today)
        .count();
    let forfeited_by_you = outcome
        .is_some_and(|o| o.victory == Some(Victory::Forfeit) && o.winner == Some(you.other()));
    let points = if forfeited_by_you {
        0
    } else if played_today >= FULL_MATCHES_A_DAY {
        TOKEN_POINTS
    } else {
        FINISHED_POINTS
            + match won {
                Some(true) => WIN_POINTS,
                None => DRAW_POINTS,
                Some(false) => 0,
            }
    };
    let team: Vec<u64> = m.rosters[you.index()]
        .iter()
        .map(|p| p.creature_id)
        .collect();
    let additions: Vec<NotebookAddition> = observe(m, you, events)
        .into_iter()
        .map(|seen| NotebookAddition {
            creature_id: seen.creature_id,
            won: won == Some(true),
            showing: to_contract(&seen.showing),
            moments: seen.moments,
        })
        .collect();
    let mut lines = vec![format!("+{points} Open Circuit points")];
    lines.push(format!("A friend match played for {}", names(m, you)));
    for a in &additions {
        let name = m.rosters[you.index()]
            .iter()
            .find(|p| p.creature_id == a.creature_id)
            .map_or("", |p| p.name.as_str());
        for moment in &a.moments {
            lines.push(format!("In the notebook, {name}: {moment}"));
        }
    }
    Earned {
        result: MatchResult {
            id: match_id(),
            mode: Mode::Friend,
            rewards: Rewards {
                circuit_points: points,
                participation: team.clone(),
                notebook: additions,
                history: Some(MatchSummary {
                    finished_at_utc: now,
                    mode: Mode::Friend,
                    won,
                    ended_by: ended_by(m).to_owned(),
                    turns: outcome.map_or(m.board.turn, |o| o.turn),
                    team,
                    opponent: Some(formiga_expansion_rulebook::sanitize_text(opponent, 24))
                        .filter(|o| !o.is_empty()),
                }),
                ..Rewards::default()
            },
        },
        lines,
    }
}

/// What a match against the AI earns. A win counts toward the cup played on the match's arena,
/// if that cup is open; the third wins it, with its trophy and medal, and opens the next.
pub fn ai_match(m: &Match, you: Side, record: &BattleRecord) -> Earned {
    let won = m.outcome().and_then(|o| o.winner) == Some(you);
    let team = names(m, you);
    let cup = cup_for(&m.arena.id).filter(|c| is_open(c, record));
    let place = cup.map_or("Backyard", |c| c.name.trim_end_matches(" Cup"));
    let mut rewards = Rewards::default();
    let mut lines = Vec::new();
    match (won, cup) {
        (true, Some(cup)) => {
            rewards.cup_win = Some(cup.id.to_owned());
            let wins = record.cup_wins(cup.id).saturating_add(1);
            rewards.journal.push(format!(
                "{team} won a {} match against the visitors.",
                cup.name
            ));
            lines.push(format!(
                "{}: {} of {CUP_WINS} wins",
                cup.name,
                wins.min(CUP_WINS)
            ));
            if wins == CUP_WINS {
                rewards
                    .journal
                    .push(format!("{team} won the {}!", cup.name));
                rewards
                    .keepsakes
                    .extend(cup.keepsakes.iter().map(|k| (*k).to_owned()));
                lines.push(format!(
                    "Won the {}: a trophy and a medal for the shelf",
                    cup.name
                ));
                let next = CUPS
                    .iter()
                    .skip_while(|c| c.id != cup.id)
                    .nth(1)
                    .filter(|next| !record.unlocked.iter().any(|u| u == next.id));
                if let Some(next) = next {
                    rewards.unlocks.push(next.id.to_owned());
                    lines.push(format!("Unlocked the {}", next.name));
                }
            }
        }
        (true, None) => rewards.journal.push(format!(
            "{team} won a match against the visitors in the {place}."
        )),
        (false, _) => rewards.journal.push(format!(
            "{team} played the visitors in the {place}{}",
            if m.outcome().and_then(|o| o.winner).is_none() {
                " and drew."
            } else {
                ", and had fun losing."
            }
        )),
    }
    lines.push("A journal entry for your colony".to_owned());
    Earned {
        result: MatchResult {
            id: match_id(),
            mode: Mode::Ai,
            rewards,
        },
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_battle_contract::BattleResult;
    use formiga_battle_core::ai::Coach;
    use formiga_battle_core::tournament::drawn_creature;
    use formiga_battle_core::{Arena, Ruleset};

    fn played(seed: u64) -> (Match, Vec<Event>) {
        let rosters = [
            (0..3).map(|i| drawn_creature(seed, i)).collect(),
            (3..6).map(|i| drawn_creature(seed, i)).collect(),
        ];
        let mut m = Match::new(Arena::backyard(), Ruleset::default(), seed, rosters).unwrap();
        let mut events = Vec::new();
        while m.outcome().is_none() {
            let orders = [
                Coach::Exhibition.orders(&m, Side::Left),
                Coach::Exhibition.orders(&m, Side::Right),
            ];
            events.extend(m.resolve_turn(orders).events);
        }
        (m, events)
    }

    fn checks_out(result: MatchResult) {
        let mut doc = BattleResult::new(
            formiga_battle_contract::SessionId::parse("00000000000000000000000000000000").unwrap(),
            "0".repeat(64),
            "0.1.0",
        );
        doc.matches.push(result);
        doc.validate().unwrap();
    }

    #[test]
    fn a_friend_match_earns_points_history_notebook_and_credit_and_nothing_else() {
        let (m, events) = played(3);
        let earned = friend_match(
            &m,
            Side::Left,
            &events,
            &BattleRecord::default(),
            "Sam",
            OffsetDateTime::UNIX_EPOCH,
        );
        let r = &earned.result.rewards;
        assert!(r.circuit_points >= FINISHED_POINTS);
        assert_eq!(r.participation.len(), 3);
        assert_eq!(r.notebook.len(), 3);
        assert!(r.history.is_some());
        assert!(
            r.journal.is_empty()
                && r.keepsakes.is_empty()
                && r.unlocks.is_empty()
                && r.cup_win.is_none()
        );
        checks_out(earned.result);
    }

    #[test]
    fn many_friend_matches_in_one_day_earn_only_a_token() {
        let (m, events) = played(3);
        let mut record = BattleRecord::default();
        for _ in 0..FULL_MATCHES_A_DAY {
            let earned = friend_match(
                &m,
                Side::Left,
                &events,
                &record,
                "Sam",
                OffsetDateTime::UNIX_EPOCH,
            );
            record.apply(&earned.result);
        }
        let earned = friend_match(
            &m,
            Side::Left,
            &events,
            &record,
            "Sam",
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(earned.result.rewards.circuit_points, TOKEN_POINTS);
    }

    #[test]
    fn an_ai_match_earns_cup_progress_journal_and_keepsakes_and_nothing_else() {
        let mut record = BattleRecord::default();
        let mut cup_won = false;
        for seed in 0..40 {
            let (m, _) = played(seed);
            let earned = ai_match(&m, Side::Left, &record);
            let r = &earned.result.rewards;
            assert_eq!(r.circuit_points, 0);
            assert!(r.participation.is_empty() && r.notebook.is_empty() && r.history.is_none());
            assert!(!r.journal.is_empty());
            cup_won |= r.unlocks.iter().any(|u| u == WOODLAND_CUP);
            checks_out(earned.result.clone());
            record.apply(&earned.result);
        }
        assert!(cup_won, "three wins win the cup");
        assert!(record.unlocked.iter().any(|u| u == WOODLAND_CUP));
        assert_eq!(record.circuit_points, 0, "the AI never gives points");
        assert!(record.notebook.is_empty(), "nor notebook entries");
    }

    /// A match won by the left team on an arena, standing in for one played there.
    fn won_on(arena: &str, seed: u64) -> Match {
        let (mut m, _) = played(seed);
        m.arena.id = arena.to_owned();
        m.board.outcome = Some(formiga_battle_core::Outcome {
            winner: Some(Side::Left),
            victory: Some(Victory::Capture),
            turn: 5,
        });
        m
    }

    #[test]
    fn each_cup_opens_the_next_and_a_locked_one_counts_for_nothing_yet() {
        let mut record = BattleRecord::default();
        assert_eq!(open_to_play(&record), vec!["backyard"]);
        // The Woodland Cup is not open: a win there earns a journal line and nothing toward it.
        let earned = ai_match(&won_on("woodland", 1), Side::Left, &record);
        assert!(earned.result.rewards.cup_win.is_none());
        assert!(!earned.result.rewards.journal.is_empty());
        for (i, cup) in CUPS.iter().enumerate() {
            for n in 0..CUP_WINS {
                let earned = ai_match(&won_on(cup.arena, u64::from(n)), Side::Left, &record);
                assert_eq!(earned.result.rewards.cup_win.as_deref(), Some(cup.id));
                checks_out(earned.result.clone());
                record.apply(&earned.result);
            }
            assert_eq!(record.cup_wins(cup.id), CUP_WINS);
            assert_eq!(open_to_play(&record).len(), (i + 2).min(CUPS.len()));
        }
        // A fourth win in a cup already won adds a win and nothing else.
        let earned = ai_match(&won_on("backyard", 9), Side::Left, &record);
        assert!(
            earned.result.rewards.unlocks.is_empty() && earned.result.rewards.keepsakes.is_empty()
        );
    }
}
