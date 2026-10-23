//! Every outcome pinned: the board each seeded match ends on, by arena and coach, and the
//! ruleset's own words and hash. A change here without a `RULES_VERSION` bump is a rule changed
//! by accident, or a hash drifting with a dependency; a change with one is regenerated in the
//! same change, from the table this test prints when it fails.

use formiga_battle_core::ai::Coach;
use formiga_battle_core::tournament::{drawn_creature, play};
use formiga_battle_core::*;

/// Arena, seed, coaches, turns played, and the board's hash at the end.
const GOLDEN: &[(&str, u64, &str, u8, u64)] = &[
    ("backyard", 1, "exhibition", 12, 0xd669e1c51e7d628d),
    ("backyard", 1, "familiar", 5, 0xedcbad6d84e743f5),
    ("backyard", 1, "mixed", 9, 0x7b2039d3585572ba),
    ("backyard", 2, "exhibition", 12, 0x8fe402c56c0a78ea),
    ("backyard", 2, "familiar", 11, 0xddfea27f80f623d5),
    ("backyard", 2, "mixed", 12, 0x9aa9790ce8a03a69),
    ("backyard", 3, "exhibition", 12, 0x800eb30ce6f56284),
    ("backyard", 3, "familiar", 12, 0xafbdc80912f41edf),
    ("backyard", 3, "mixed", 8, 0x5afa162081b18d81),
    ("woodland", 1, "exhibition", 12, 0xec68c3e4e7062180),
    ("woodland", 1, "familiar", 11, 0x54d11ffbf6f5801d),
    ("woodland", 1, "mixed", 11, 0xe6e28b67006dc1ae),
    ("woodland", 2, "exhibition", 10, 0xbef3c998fcd8b92c),
    ("woodland", 2, "familiar", 10, 0x79e94934e8ab6c65),
    ("woodland", 2, "mixed", 12, 0x4418a8d643eaf1e0),
    ("woodland", 3, "exhibition", 10, 0x195bca0392138987),
    ("woodland", 3, "familiar", 9, 0x11d1e57e53ef79a8),
    ("woodland", 3, "mixed", 10, 0x070fc9b6b0a7f29d),
    ("millpond", 1, "exhibition", 12, 0x28c2b241a474464a),
    ("millpond", 1, "familiar", 12, 0x126fe781c2cfd962),
    ("millpond", 1, "mixed", 9, 0xf20e28cb54ffedbe),
    ("millpond", 2, "exhibition", 12, 0xf719c04c2bcbc0f7),
    ("millpond", 2, "familiar", 6, 0x4a4b506249793b6a),
    ("millpond", 2, "mixed", 12, 0x8d27ec3c79cd64e3),
    ("millpond", 3, "exhibition", 12, 0x4256d480bd0f7a5f),
    ("millpond", 3, "familiar", 12, 0x7b90def83cb6cef0),
    ("millpond", 3, "mixed", 9, 0x4a1462e5171ab254),
    ("cove", 1, "exhibition", 2, 0xb12b50f283328362),
    ("cove", 1, "familiar", 4, 0x202b1df37eaa56ba),
    ("cove", 1, "mixed", 6, 0xa5bd806d36403663),
    ("cove", 2, "exhibition", 12, 0x6a59d2581fbd95dd),
    ("cove", 2, "familiar", 3, 0xce9edd6c2a532ca3),
    ("cove", 2, "mixed", 3, 0xeee6efa152ab2e7c),
    ("cove", 3, "exhibition", 2, 0x19c1ab3ee45af65d),
    ("cove", 3, "familiar", 6, 0x1c75ed4d9c0a24ad),
    ("cove", 3, "mixed", 8, 0xc3153940bf66f23a),
];

const RULESET_JSON: &str = r#"{"composure":3,"ticks_per_turn":3,"capture_ticks":3,"capture_rule":"uncontested","capture_decays":false,"turn_cap":12,"damage_cap":2,"detour_budget":2,"turn_seconds":40}"#;
const RULESET_HASH: u64 = 0xe362c52d531c2d08;

fn coaches(named: &str) -> [Coach; 2] {
    match named {
        "exhibition" => [Coach::Exhibition; 2],
        "familiar" => [Coach::Familiar; 2],
        "mixed" => [Coach::Generic, Coach::Exhibition],
        other => panic!("no coaches called {other}"),
    }
}

fn played(arena: &str, seed: u64, named: &str) -> (u8, u64) {
    let arena = Arena::by_id(arena).expect("a built-in arena");
    let rosters = [
        (0..3).map(|i| drawn_creature(seed, i)).collect(),
        (3..6).map(|i| drawn_creature(seed, i)).collect(),
    ];
    let (m, reports) = play(&arena, Ruleset::default(), seed, rosters, coaches(named));
    let outcome = m.outcome().expect("played to the end");
    (outcome.turn, reports.last().expect("a turn").hash)
}

#[test]
fn every_golden_match_still_ends_on_the_same_board() {
    let mut rows = Vec::new();
    let mut drift = Vec::new();
    for (arena, seed, named, turns, hash) in GOLDEN {
        let (now_turns, now_hash) = played(arena, *seed, named);
        rows.push(format!(
            "    (\"{arena}\", {seed}, \"{named}\", {now_turns}, 0x{now_hash:016x}),"
        ));
        if (now_turns, now_hash) != (*turns, *hash) {
            drift.push(format!("{arena} seed {seed} {named}"));
        }
    }
    assert!(
        drift.is_empty(),
        "{} matches changed ({}) under RULES_VERSION {RULES_VERSION}; if the rules changed on \
         purpose, bump it and paste this table:\n{}",
        drift.len(),
        drift.join(", "),
        rows.join("\n")
    );
}

#[test]
fn the_default_ruleset_reads_and_hashes_as_it_always_has() {
    let json = serde_json::to_string(&Ruleset::default()).unwrap();
    assert_eq!(json, RULESET_JSON, "the ruleset's own words changed");
    assert_eq!(
        Ruleset::default().hash(),
        RULESET_HASH,
        "the ruleset's hash moved: 0x{:016x}",
        Ruleset::default().hash()
    );
}

#[test]
fn a_ruleset_with_a_zero_in_it_cannot_set_up_a_match() {
    let rules = Ruleset {
        ticks_per_turn: 0,
        ..Ruleset::default()
    };
    assert_eq!(rules.validate(), Err(RulesetError::Zero("ticks_per_turn")));
    let rosters = [
        (0..3).map(|i| drawn_creature(1, i)).collect(),
        (3..6).map(|i| drawn_creature(1, i)).collect(),
    ];
    assert!(matches!(
        Match::new(Arena::backyard(), rules, 1, rosters),
        Err(game::MatchError::Rules(RulesetError::Zero(
            "ticks_per_turn"
        )))
    ));
    assert_eq!(Ruleset::default().validate(), Ok(()));
}
