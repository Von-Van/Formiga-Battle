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
/// Wins against the AI that win the Backyard Cup.
pub const CUP_WINS: u8 = 3;

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

/// What a match against the AI earns.
pub fn ai_match(m: &Match, you: Side, record: &BattleRecord) -> Earned {
    let won = m.outcome().and_then(|o| o.winner) == Some(you);
    let team = names(m, you);
    let mut rewards = Rewards::default();
    let mut lines = Vec::new();
    if won {
        rewards.cup_win = Some(BACKYARD_CUP.to_owned());
        let wins = record.cup_wins(BACKYARD_CUP).saturating_add(1);
        rewards.journal.push(format!(
            "{team} won a Backyard Cup match against the visitors."
        ));
        lines.push(format!(
            "Backyard Cup: {} of {CUP_WINS} wins",
            wins.min(CUP_WINS)
        ));
        if wins == CUP_WINS && !record.unlocked.iter().any(|u| u == WOODLAND_CUP) {
            rewards.unlocks.push(WOODLAND_CUP.to_owned());
            rewards
                .journal
                .push(format!("{team} won the Backyard Cup!"));
            rewards
                .keepsakes
                .push("battle.trophy.backyard_cup".to_owned());
            rewards
                .keepsakes
                .push("battle.medal.backyard_cup".to_owned());
            lines.push("Won the Backyard Cup: a trophy and a medal for the shelf".to_owned());
            lines.push("Unlocked the Woodland Cup".to_owned());
        }
    } else {
        rewards.journal.push(format!(
            "{team} played the visitors in the Backyard{}",
            if m.outcome().and_then(|o| o.winner).is_none() {
                " and drew."
            } else {
                ", and had fun losing."
            }
        ));
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
}
