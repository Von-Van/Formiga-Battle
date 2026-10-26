//! Brackets advance as they should under wins, draws and forfeits, resume from their file without
//! playing or crediting anything twice, and a whole event between coaches is the same event on
//! every run.

use formiga_battle_core::ai::Coach;
use formiga_battle_core::tournament::drawn_creature;
use formiga_battle_core::{Archetype, Arena, Ruleset};
use formiga_battle_world::fixture::{match_id, play_out};
use formiga_battle_world::*;

fn four() -> Bracket {
    Bracket::new(
        "newcomers-1",
        42,
        ["you", "hazel", "tam", "ivy"].map(String::from).to_vec(),
    )
    .unwrap()
}

#[test]
fn the_first_two_seeds_can_meet_only_in_the_final() {
    let b = four();
    assert_eq!(b.rounds(), 2);
    assert_eq!(b.fixtures.len(), 3);
    assert_eq!(b.fixtures[0].sides, [Some(0), Some(3)]);
    assert_eq!(b.fixtures[1].sides, [Some(1), Some(2)]);
    assert_eq!(b.round_name(0), "Semi-finals");
    assert_eq!(b.round_name(1), "Final");
    let eight = Bracket::new("e", 1, (0..8).map(|i| format!("h{i}")).collect()).unwrap();
    let firsts: Vec<_> = eight
        .fixtures
        .iter()
        .filter(|f| f.round == 0)
        .map(|f| f.sides)
        .collect();
    assert_eq!(
        firsts,
        [
            [Some(0), Some(7)],
            [Some(3), Some(4)],
            [Some(1), Some(6)],
            [Some(2), Some(5)]
        ]
    );
    assert_eq!(eight.fixtures.len(), 7);
    assert_eq!(eight.round_name(0), "Quarter-finals");
}

#[test]
fn winners_go_through_until_there_is_a_champion() {
    let mut b = four();
    assert_eq!(b.next(), Some(0));
    assert_eq!(b.ready_for(0), Some(0));
    assert_eq!(
        b.record(2, "m0", Some(0), Decided::Capture, 6),
        Err(BracketError::NotReady(2))
    );
    assert_eq!(b.record(0, "m1", Some(0), Decided::Capture, 6), Ok(true));
    assert_eq!(b.fixtures[2].sides, [Some(0), None]);
    assert_eq!(b.record(1, "m2", Some(1), Decided::Forfeit, 4), Ok(true));
    assert_eq!(b.fixtures[2].sides, [Some(0), Some(2)]);
    assert_eq!(b.reached(3), (0, false));
    assert_eq!(b.reached(0), (1, true));
    assert!(!b.finished());
    assert_eq!(b.record(2, "m3", Some(1), Decided::Cap, 12), Ok(true));
    assert_eq!(b.champion(), Some(2));
    assert_eq!(b.reached(2), (1, false));
    assert_eq!(b.next(), None);
}

#[test]
fn a_match_is_recorded_once_and_a_fixture_is_played_once() {
    let mut b = four();
    assert_eq!(b.record(0, "m1", Some(1), Decided::Knockout, 5), Ok(true));
    // The same match again, as after a crash between writing it and moving on: nothing changes.
    let before = b.clone();
    assert_eq!(b.record(0, "m1", Some(0), Decided::Capture, 5), Ok(false));
    assert_eq!(b, before);
    // Another match for a fixture already played is refused.
    assert_eq!(
        b.record(0, "m9", Some(0), Decided::Capture, 5),
        Err(BracketError::Played(0))
    );
}

#[test]
fn a_drawn_fixture_is_settled_by_the_same_toss_everywhere() {
    let mut a = four();
    let mut b = four();
    a.record(0, "m1", None, Decided::Cap, 12).unwrap();
    b.record(0, "m1", None, Decided::Cap, 12).unwrap();
    assert_eq!(a, b);
    let r = a.fixtures[0].result.as_ref().unwrap();
    assert_eq!(r.decided, Decided::Toss);
    assert_eq!(r.winner, a.toss(0));
}

#[test]
fn a_bracket_resumes_from_its_file_as_it_was() {
    let mut b = four();
    b.record(0, "m1", Some(0), Decided::Capture, 6).unwrap();
    let json = serde_json::to_string(&b).unwrap();
    let mut read: Bracket = serde_json::from_str(&json).unwrap();
    assert_eq!(read, b);
    assert_eq!(read.next(), Some(1));
    assert_eq!(
        read.record(0, "m1", Some(1), Decided::Capture, 6),
        Ok(false)
    );
}

#[test]
fn a_bracket_of_the_wrong_size_or_with_someone_twice_is_refused() {
    assert_eq!(
        Bracket::new("x", 1, ["a", "b", "c"].map(String::from).to_vec()),
        Err(BracketError::Size(3))
    );
    assert_eq!(
        Bracket::new("x", 1, ["a", "b", "a", "c"].map(String::from).to_vec()),
        Err(BracketError::Twice("a".to_owned()))
    );
}

#[test]
fn an_event_between_coaches_plays_the_same_on_every_run() {
    let run = || {
        let mut b = Bracket::new(
            "rivals",
            7,
            ["hazel", "tam", "ivy", "bram"].map(String::from).to_vec(),
        )
        .unwrap();
        let styles = [
            Archetype::Aggressor,
            Archetype::Defender,
            Archetype::Tactician,
            Archetype::Opportunist,
        ];
        let team = |e: usize| (0..3).map(|i| drawn_creature(100 + e as u64, i)).collect();
        while let Some(index) = b.next() {
            let [a, c] = b.fixtures[index].sides.map(|s| s.unwrap());
            let played = play_out(
                &Arena::backyard(),
                Ruleset::default(),
                b.seed,
                index,
                [team(a), team(c)],
                [Coach::Styled(styles[a]), Coach::Styled(styles[c])],
            );
            assert_eq!(played.match_id, match_id(b.seed, index));
            assert!(
                played.record.replay().is_ok(),
                "the fixture's match replays"
            );
            b.record(
                index,
                &played.match_id,
                played.winner,
                played.decided,
                played.turns,
            )
            .unwrap();
        }
        b
    };
    let (a, b) = (run(), run());
    assert!(a.finished());
    assert_eq!(a, b);
}
