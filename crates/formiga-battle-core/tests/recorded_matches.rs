//! Matches recorded once and kept as a file, replayed against the rules as they are now.
//!
//! The other determinism tests play their matches in the same build they check, so a change to
//! how something resolves passes them: both sides of the comparison moved together. These
//! matches were played by an earlier build and written down, every turn's plans and the board's
//! hash after each, so a silent rules change shows up as a desync here, and the fix is to move
//! `RULES_VERSION` (which refuses the old record and asks for a new one), never to make the
//! test agree.
//!
//! The fixture is only ever rewritten on purpose: run this test with
//! `FORMIGA_BATTLE_WRITE_FIXTURE=1` and commit what it writes, in the same change as the
//! `RULES_VERSION` bump that made it necessary. Nothing rewrites it otherwise.

use formiga_battle_core::ai::Coach;
use formiga_battle_core::game::Board;
use formiga_battle_core::replay::ReplayError;
use formiga_battle_core::tournament::{drawn_creature, play};
use formiga_battle_core::{Arena, MatchRecord, Profile, RULES_VERSION, Ruleset, TurnReport, rng};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Once;

/// Where the recorded matches live, beside this test.
const PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/recorded_matches.json"
);

/// Set to `1`, this rewrites the fixture from the current build before checking it. It is an
/// environment variable rather than a feature or a flag so that no ordinary `cargo test` can
/// ever do it by accident.
const WRITE_VAR: &str = "FORMIGA_BATTLE_WRITE_FIXTURE";

/// The one command that rewrites the fixture, spelled out in every message that asks for it.
const REGENERATE: &str =
    "FORMIGA_BATTLE_WRITE_FIXTURE=1 cargo test -p formiga-battle-core --test recorded_matches";

/// The first seed; each built-in arena's match takes the next one.
const FIRST_SEED: u64 = 23;

/// The whole file: one recorded match for each arena that comes with Battle.
#[derive(Debug, Serialize, Deserialize)]
struct Fixture {
    /// Why the file exists, for whoever opens it without reading this test.
    about: String,
    /// The rules the matches were played by, checked before anything is replayed.
    rules_version: u32,
    matches: Vec<Recorded>,
}

/// One match as the core records it, with what it ended on and what every turn reported.
#[derive(Debug, Serialize, Deserialize)]
struct Recorded {
    /// The arena, ruleset, seed, both rosters, every turn's plans and the board's hash after
    /// each: everything `MatchRecord::replay` needs, and the per-turn hashes it checks.
    record: MatchRecord,
    /// FNV-1a over the JSON of every turn's report, as sixteen hex digits. The reports carry the
    /// events a client animates and the Notebook reads, which the board's hash does not.
    reports_digest: String,
    /// The board as the match ended, kept whole so a desync is readable as a diff, not only as
    /// two hashes.
    final_board: Board,
}

fn digest(reports: &[TurnReport]) -> String {
    let bytes = serde_json::to_vec(reports).expect("turn reports serialize");
    format!("{:016x}", rng::fnv1a(&bytes))
}

/// Six creatures drawn as Desktop draws new companions, three a side.
fn rosters(seed: u64) -> [Vec<Profile>; 2] {
    [
        (0..3).map(|n| drawn_creature(seed, n)).collect(),
        (3..6).map(|n| drawn_creature(seed, n)).collect(),
    ]
}

/// Plays one match on every built-in arena with the current build, both sides coached, so every
/// arena's map event and found things are in the record.
fn play_them_all() -> Vec<Recorded> {
    (FIRST_SEED..)
        .zip(Arena::built_in())
        .map(|(seed, arena)| {
            let (m, reports) = play(
                &arena,
                Ruleset::default(),
                seed,
                rosters(seed),
                [Coach::Exhibition; 2],
            );
            Recorded {
                record: m.record(),
                reports_digest: digest(&reports),
                final_board: m.board,
            }
        })
        .collect()
}

fn write_fixture(path: &Path) {
    let fixture = Fixture {
        about: format!(
            "Matches recorded by formiga-battle-core at rules version {RULES_VERSION}, replayed \
             by tests/recorded_matches.rs. Never edit by hand; rewrite on purpose with \
             `{REGENERATE}` and commit the result with the RULES_VERSION bump that needed it."
        ),
        rules_version: RULES_VERSION,
        matches: play_them_all(),
    };
    let mut text = serde_json::to_string_pretty(&fixture).expect("the fixture serializes");
    text.push('\n');
    std::fs::write(path, text).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    eprintln!(
        "wrote {} from rules version {RULES_VERSION}",
        path.display()
    );
}

fn read_fixture(path: &Path) -> Fixture {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {} ({e}): run `{REGENERATE}`", path.display()));
    let value: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{} is not JSON ({e}): run `{REGENERATE}`", path.display()));
    assert!(
        value.get("placeholder").is_none(),
        "no fixture yet: {} is the placeholder. Run `{REGENERATE}` and commit what it writes.",
        path.display()
    );
    serde_json::from_value(value).unwrap_or_else(|e| {
        panic!(
            "{} does not parse as recorded matches ({e}). If the record's shape changed on \
             purpose, run `{REGENERATE}` and commit the result.",
            path.display()
        )
    })
}

/// Every test reads the fixture through here, so with the variable set the file is written once,
/// before any test reads it, and never while another test is reading it.
fn fixture() -> Fixture {
    static WRITTEN: Once = Once::new();
    let path = Path::new(PATH);
    if std::env::var_os(WRITE_VAR).is_some_and(|v| v == "1") {
        WRITTEN.call_once(|| write_fixture(path));
    }
    read_fixture(path)
}

#[test]
fn the_recorded_matches_replay_to_their_recorded_outcomes_under_the_current_rules() {
    let fixture = fixture();
    assert_eq!(
        fixture.rules_version, RULES_VERSION,
        "RULES_VERSION moved from {} to {RULES_VERSION}, so the recorded matches are from the \
         old rules. Regenerate them on purpose with `{REGENERATE}` and commit the file with the \
         bump.",
        fixture.rules_version
    );
    assert!(
        !fixture.matches.is_empty(),
        "the fixture records no match at all"
    );
    for recorded in &fixture.matches {
        let id = &recorded.record.arena.id;
        let (m, reports) = match recorded.record.replay() {
            Ok(replayed) => replayed,
            Err(ReplayError::Desync {
                turn,
                recorded: was,
                replayed: now,
            }) => panic!(
                "the rules changed: on {id}, turn {turn} comes out differently from the record \
                 (recorded {was:016x}, replayed {now:016x}) under the same \
                 RULES_VERSION {RULES_VERSION}. A change to how anything resolves must bump \
                 RULES_VERSION in crates/formiga-battle-core/src/ruleset.rs; then regenerate \
                 the fixture on purpose with `{REGENERATE}`."
            ),
            Err(e) => panic!(
                "the rules changed: on {id} the recorded match no longer replays ({e}) under \
                 the same RULES_VERSION {RULES_VERSION}. Bump RULES_VERSION, then regenerate the \
                 fixture on purpose with `{REGENERATE}`."
            ),
        };
        assert_eq!(
            m.outcome(),
            recorded.record.outcome,
            "the rules changed: on {id} the match ends differently. Bump RULES_VERSION, then \
             regenerate the fixture on purpose with `{REGENERATE}`."
        );
        assert_eq!(
            m.board, recorded.final_board,
            "the rules changed: on {id} the board ends up different from the record. Bump \
             RULES_VERSION, then regenerate the fixture on purpose with `{REGENERATE}`."
        );
        assert_eq!(
            digest(&reports),
            recorded.reports_digest,
            "the turn reports changed on {id} while the board came out the same. If what a \
             turn resolves to is unchanged and only what it reports moved, regenerate the \
             fixture on purpose with `{REGENERATE}`; otherwise bump RULES_VERSION first."
        );
    }
}

#[test]
fn the_recorded_matches_cover_every_built_in_arena_as_it_ships() {
    let fixture = fixture();
    for arena in Arena::built_in() {
        let recorded = fixture
            .matches
            .iter()
            .find(|r| r.record.arena.id == arena.id)
            .unwrap_or_else(|| {
                panic!(
                    "no recorded match on {}. Regenerate the fixture on purpose with \
                     `{REGENERATE}` so every built-in arena is in it.",
                    arena.id
                )
            });
        assert_eq!(
            recorded.record.arena, arena,
            "{} changed since the fixture was recorded, so the recorded match plays on a board \
             that no longer ships. Regenerate the fixture on purpose with `{REGENERATE}`.",
            arena.id
        );
        assert_eq!(
            recorded.record.ruleset,
            Ruleset::default(),
            "the default ruleset changed since the fixture was recorded. Regenerate the fixture \
             on purpose with `{REGENERATE}`."
        );
    }
}
