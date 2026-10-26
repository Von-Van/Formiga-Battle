//! The balance report: how the rivals' styles and tiers, the playbooks and the old coaches fare
//! against each other, played by the same loop as the fairness tournament.
//!
//! ```text
//! cargo run --release -p formiga-battle-core --example balance -- [pairings]
//! ```

use formiga_battle_core::ai::Coach;
use formiga_battle_core::playbook::{self, Pilot, Playbook};
use formiga_battle_core::tournament::run;
use formiga_battle_core::{Archetype, Arena, Ruleset};

const STYLES: [Archetype; 5] = [
    Archetype::Aggressor,
    Archetype::Defender,
    Archetype::Tactician,
    Archetype::Opportunist,
    Archetype::Adaptive,
];

fn main() {
    let pairings: u32 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(20);
    let rules = Ruleset::default();
    let backyard = Arena::backyard();
    println!("Scores are the row's: wins plus half of draws, each pairing played from both sides.");
    println!("{pairings} pairings each.\n");

    println!("Book rivals (cups and Opens), each against each, on every arena:");
    for arena in Arena::built_in() {
        println!("  {}", arena.name);
        for a in STYLES {
            let row: Vec<String> = STYLES
                .iter()
                .map(|b| {
                    let s = run(
                        &arena,
                        rules,
                        5,
                        pairings,
                        [Coach::Styled(a), Coach::Styled(*b)],
                    );
                    format!("{:.2}", s.left.score())
                })
                .collect();
            println!("    {:<12} {}", a.label(), row.join("  "));
        }
    }

    println!("\nEach tier against the old coaches, on the Backyard:");
    for a in STYLES {
        for (tier, coach) in [
            ("book", Coach::Styled(a)),
            ("thinking", Coach::Thinking(a)),
            ("scouted", Coach::Scouted(a)),
        ] {
            let ex = run(&backyard, rules, 7, pairings, [coach, Coach::Exhibition])
                .left
                .score();
            let generic = run(&backyard, rules, 7, pairings, [coach, Coach::Generic])
                .left
                .score();
            println!(
                "  {:<12} {:<9} v Exhibition {ex:.2}   v generic {generic:.2}",
                a.label(),
                tier
            );
        }
    }

    println!("\nThinking rivals (Regionals), each against each, on the Backyard:");
    for a in STYLES {
        let row: Vec<String> = STYLES
            .iter()
            .map(|b| {
                let s = run(
                    &backyard,
                    rules,
                    9,
                    pairings,
                    [Coach::Thinking(a), Coach::Thinking(*b)],
                );
                format!("{:.2}", s.left.score())
            })
            .collect();
        println!("  {:<12} {}", a.label(), row.join("  "));
    }

    println!("\nSample playbooks against each tier of each style, on the Backyard:");
    for book in Playbook::samples() {
        let pilot = Pilot::Playbook(book.clone());
        let mut cells = Vec::new();
        for a in STYLES {
            for coach in [Coach::Styled(a), Coach::Thinking(a)] {
                let f = playbook::face_off(
                    &backyard,
                    rules,
                    11,
                    pairings,
                    [&pilot, &Pilot::Coach(coach)],
                );
                cells.push(format!("{:.2}", f.record.score()));
            }
        }
        println!("  {:<16} {}", book.name, cells.join(" "));
    }
    println!("  (pairs of columns: book, thinking; styles in the order Aggressor, Defender,");
    println!("   Tactician, Opportunist, Adaptive)");
}
