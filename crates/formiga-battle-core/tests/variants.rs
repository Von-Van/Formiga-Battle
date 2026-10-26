//! Match variants: every part fits the arenas it names, every variant is the same for both teams
//! and fair in play, the seed draws every part sooner or later, and a match played in one is
//! recorded and replayed exactly. A variant that fails its sweep is not offered.

use formiga_battle_core::ai::Coach;
use formiga_battle_core::items::Finds;
use formiga_battle_core::tournament::{drawn_creature, play, run};
use formiga_battle_core::variant::{self, CONDITIONS, FINDS, STARTS, Variant, steps_to_zone};
use formiga_battle_core::*;
use std::collections::BTreeSet;

fn rosters(seed: u64) -> [Vec<Profile>; 2] {
    [
        (0..3).map(|i| drawn_creature(seed, i)).collect(),
        (3..6).map(|i| drawn_creature(seed, i)).collect(),
    ]
}

fn with(start: &str, finds: &str, condition: Option<&str>) -> Variant {
    Variant {
        start: start.to_owned(),
        finds: finds.to_owned(),
        condition: condition.map(str::to_owned),
    }
}

#[test]
fn every_part_has_its_own_id_and_names_only_arenas_that_exist() {
    let arenas: BTreeSet<String> = Arena::built_in().into_iter().map(|a| a.id).collect();
    let starts: BTreeSet<_> = STARTS.iter().map(|s| s.id).collect();
    let finds: BTreeSet<_> = FINDS.iter().map(|f| f.id).collect();
    let conditions: BTreeSet<_> = CONDITIONS.iter().map(|c| c.id).collect();
    assert_eq!(starts.len(), STARTS.len());
    assert_eq!(finds.len(), FINDS.len());
    assert_eq!(conditions.len(), CONDITIONS.len());
    assert!(!conditions.contains(variant::STANDARD));
    for start in STARTS {
        for (arena, _) in start.places {
            assert!(arenas.contains(*arena), "{} names {arena}", start.id);
        }
    }
    for set in FINDS {
        for (arena, ..) in set.spots.unwrap_or_default() {
            assert!(arenas.contains(*arena), "{} names {arena}", set.id);
        }
    }
    for condition in CONDITIONS {
        for (arena, _) in condition.names {
            assert!(arenas.contains(*arena), "{} names {arena}", condition.id);
        }
    }
    // The standard parts fit every arena, and are listed first.
    for arena in &arenas {
        assert_eq!(Variant::starts(arena)[0].id, variant::STANDARD);
        assert_eq!(Variant::finds(arena)[0].id, variant::STANDARD);
    }
    let standard = FINDS[0];
    assert_eq!(standard.rhythm(), Finds::default());
}

#[test]
fn every_variant_an_arena_names_sets_up_and_says_what_it_does() {
    for arena in Arena::built_in() {
        let singles = Variant::singles(&arena.id);
        assert!(
            singles.len() >= 8,
            "{} has {} variants",
            arena.id,
            singles.len()
        );
        for v in singles {
            let varied = v
                .apply(&arena)
                .unwrap_or_else(|e| panic!("{} {v:?}: {e}", arena.id));
            assert_eq!(varied.variant.as_ref(), Some(&v));
            let told = v.told(&arena);
            assert!(told.len() >= 2);
            assert!(
                told.iter()
                    .all(|t| !t.name.is_empty() && !t.line.is_empty())
            );
            assert_eq!(told, v.told(&varied), "told the same from either arena");
            // A part that is not the standard changes something.
            if !v.is_standard() {
                assert_ne!(
                    (
                        varied.starts,
                        &varied.item_spots,
                        varied.finds,
                        &varied.scenery,
                        &varied.gates_open
                    ),
                    (
                        arena.starts,
                        &arena.item_spots,
                        arena.finds,
                        &arena.scenery,
                        &arena.gates_open
                    ),
                    "{} {v:?} changes nothing",
                    arena.id
                );
            }
        }
    }
}

/// The mirrored partner: the right team's half of every variant is the left's mirror image, the
/// beacon is the same number of steps from each start and its partner, and every live item spot
/// has its partner live too.
#[test]
fn every_variant_is_the_mirror_image_for_the_other_team() {
    for arena in Arena::built_in() {
        for v in Variant::singles(&arena.id) {
            let varied = v.apply(&arena).unwrap();
            assert_eq!(variant::check(&varied), Ok(()), "{} {v:?}", arena.id);
            for (left, right) in varied.starts[0].iter().zip(varied.starts[1]) {
                assert_eq!(varied.mirror(*left), right, "{} {v:?}", arena.id);
                let steps = steps_to_zone(&varied, *left);
                assert!(steps.is_some(), "{} {v:?}: {left} cannot reach", arena.id);
                assert_eq!(steps, steps_to_zone(&varied, right), "{} {v:?}", arena.id);
            }
            for spot in &varied.item_spots {
                assert!(varied.item_spots.contains(&varied.mirror(*spot)));
            }
            for tile in varied.tiles() {
                assert_eq!(
                    varied.in_scenery(tile),
                    varied.in_scenery(varied.mirror(tile))
                );
            }
        }
    }
}

/// The same fairness the arenas themselves are held to, for every variant that changes one part:
/// 300 pairings played from both sides, the left side within 0.42 to 0.58 and every temperament
/// kind within 0.35 to 0.65. Twice the arenas' own check, since a kind is then near 300 matches
/// and its score within about 0.03 of the truth.
fn sweep(arena_id: &str) {
    let arena = Arena::by_id(arena_id).unwrap();
    let mut failed = Vec::new();
    for v in Variant::singles(arena_id).into_iter().skip(1) {
        let varied = v.apply(&arena).unwrap();
        let s = run(&varied, Ruleset::default(), 3, 300, [Coach::Exhibition; 2]);
        assert_eq!(s.matches, 600);
        let name = v.headline(&arena);
        if !(0.42..=0.58).contains(&s.left.score()) {
            failed.push(format!("{name}: left {:.3}", s.left.score()));
        }
        for (kind, record) in &s.by_kind {
            if !(0.35..=0.65).contains(&record.score()) {
                failed.push(format!(
                    "{name}: {kind} {:.3} over {}",
                    record.score(),
                    record.played
                ));
            }
        }
    }
    assert!(failed.is_empty(), "{arena_id}: {}", failed.join("; "));
}

#[test]
fn every_backyard_variant_is_fair_to_both_sides_and_every_kind() {
    sweep("backyard");
}

#[test]
fn every_woodland_variant_is_fair_to_both_sides_and_every_kind() {
    sweep("woodland");
}

#[test]
fn every_millpond_variant_is_fair_to_both_sides_and_every_kind() {
    sweep("millpond");
}

#[test]
fn every_cove_variant_is_fair_to_both_sides_and_every_kind() {
    sweep("cove");
}

#[test]
fn the_seed_draws_the_same_variant_every_time_and_every_part_sooner_or_later() {
    for arena in Arena::built_in() {
        let mut starts = BTreeSet::new();
        let mut finds = BTreeSet::new();
        let mut conditions = BTreeSet::new();
        for seed in 0..400 {
            let v = Variant::drawn(&arena, seed);
            assert_eq!(v, Variant::drawn(&arena, seed));
            v.apply(&arena).unwrap();
            starts.insert(v.start);
            finds.insert(v.finds);
            conditions.insert(v.condition);
        }
        assert_eq!(
            starts.len(),
            Variant::starts(&arena.id).len(),
            "{}",
            arena.id
        );
        assert_eq!(finds.len(), Variant::finds(&arena.id).len(), "{}", arena.id);
        assert_eq!(
            conditions.len(),
            Variant::conditions(&arena.id).len() + 1,
            "{} draws every condition and none",
            arena.id
        );
    }
}

#[test]
fn a_match_played_in_a_variant_records_it_and_replays_exactly() {
    for arena in Arena::built_in() {
        for v in Variant::singles(&arena.id) {
            let varied = v.apply(&arena).unwrap();
            let (m, _) = play(
                &varied,
                Ruleset::default(),
                5,
                rosters(5),
                [Coach::Exhibition; 2],
            );
            let record = m.record();
            assert_eq!(record.arena.variant.as_ref(), Some(&v));
            assert_eq!(
                record.staged, None,
                "a variant's starts are the arena's own"
            );
            let json = serde_json::to_string(&record).unwrap();
            let read: MatchRecord = serde_json::from_str(&json).unwrap();
            let (again, _) = read
                .replay()
                .unwrap_or_else(|e| panic!("{} {v:?}: {e}", arena.id));
            assert_eq!(again.board, m.board);
        }
    }
}

#[test]
fn an_arena_without_a_variant_is_written_as_it_always_was() {
    let json = serde_json::to_string(&Arena::backyard()).unwrap();
    assert!(
        !json.contains("finds") && !json.contains("variant"),
        "{json}"
    );
    // A record kept before variants reads with the standard rhythm.
    let back: Arena = serde_json::from_str(&json).unwrap();
    assert_eq!(back.finds, Finds::default());
    assert_eq!(back.variant, None);
}

/// The turns found things glint for, and what they are, over a whole match.
fn finds_seen(arena: &Arena, seed: u64) -> Vec<(u8, Item, Vec<Tile>)> {
    let (_, reports) = play(
        arena,
        Ruleset::default(),
        seed,
        rosters(seed),
        [Coach::Exhibition; 2],
    );
    reports
        .iter()
        .flat_map(|r| &r.events)
        .filter_map(|e| match &e.kind {
            EventKind::ItemsGlint {
                spots,
                item,
                appears_turn,
            } => Some((*appears_turn, *item, spots.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn finds_turn_up_when_where_and_as_their_set_says() {
    let backyard = Arena::backyard();
    let plenty = with("standard", "plenty", None).apply(&backyard).unwrap();
    let few = with("standard", "few", None).apply(&backyard).unwrap();
    let light = with("standard", "light", None).apply(&backyard).unwrap();
    let one_pair = with("standard", "one_pair", None).apply(&backyard).unwrap();
    for seed in 0..20 {
        for (turn, ..) in finds_seen(&plenty, seed) {
            assert!(turn >= 2 && turn % 2 == 0, "plenty: turn {turn}");
        }
        for (turn, ..) in finds_seen(&few, seed) {
            assert!(turn >= 3 && (turn - 3) % 4 == 0, "few: turn {turn}");
        }
        for (_, item, _) in finds_seen(&light, seed) {
            assert!(
                matches!(item, Item::Puff | Item::Twig | Item::Leaf),
                "{item:?}"
            );
        }
        for (_, _, spots) in finds_seen(&one_pair, seed) {
            assert_eq!(spots, vec![Tile::new(0, 0), Tile::new(8, 0)]);
        }
    }
}

#[test]
fn a_condition_changes_the_board_as_its_line_says() {
    let backyard = Arena::backyard();
    let rules = Ruleset::default();
    // The gate left open.
    let open = with("standard", "standard", Some("gate_open"))
        .apply(&backyard)
        .unwrap();
    assert!(
        Match::starting_board(&open, &rules)
            .gates_open
            .iter()
            .all(|g| *g)
    );
    // The sprinkler early: warned at the end of turn 1 for turn 2.
    let early = with("standard", "standard", Some("early"))
        .apply(&backyard)
        .unwrap();
    let mut m = Match::new(early, rules, 1, rosters(1)).unwrap();
    let report = m.resolve_turn([Vec::new(), Vec::new()]);
    assert!(
        report
            .events
            .iter()
            .any(|e| matches!(e.kind, EventKind::SceneryWarned { lands_turn: 2, .. }))
    );
    // Off: it never comes round by itself.
    let calm = with("standard", "standard", Some("calm"))
        .apply(&backyard)
        .unwrap();
    let scenery = calm.scenery.clone().unwrap();
    assert!((1..=rules.turn_cap).all(|t| !scenery.due(t)));
    // Stiff taps rest longer.
    let stiff = with("standard", "standard", Some("slow_switches"))
        .apply(&backyard)
        .unwrap();
    assert_eq!(stiff.scenery.unwrap().rest, 4);
}

#[test]
fn the_starts_a_variant_names_are_where_the_teams_begin() {
    let backyard = Arena::backyard();
    let forward = with("forward", "standard", None).apply(&backyard).unwrap();
    let m = Match::new(forward, Ruleset::default(), 1, rosters(1)).unwrap();
    for slot in Slot::all() {
        let x = m.fighter(slot).at.x;
        assert_eq!(x, if slot.side == Side::Left { 1 } else { 7 });
    }
}
