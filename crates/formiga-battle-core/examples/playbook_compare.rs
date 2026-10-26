//! The playbook prototype's headless comparison: each sample playbook, one stage at a time and
//! all stages at once, against the coaches that give orders turn by turn, beside those coaches
//! against each other; then the playbooks against each other. Every match is played under the
//! real rules by the same loop as the fairness tournament, with random trios kept with their
//! pilot and sides swapped.
//!
//!     cargo run --release -p formiga-battle-core --example playbook_compare -- [pairings] [arena]
//!
//! `pairings` defaults to 50 (100 matches a row); `arena` to every built-in arena.

use formiga_battle_core::ai::Coach;
use formiga_battle_core::playbook::{FaceOff, Pilot, Playbook, face_off};
use formiga_battle_core::{Arena, Ruleset};

fn main() {
    let mut args = std::env::args().skip(1);
    let pairings: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(50);
    let arenas: Vec<Arena> = match args.next() {
        Some(id) => vec![Arena::by_id(&id).expect("a built-in arena id")],
        None => Arena::built_in(),
    };
    let seed = 20_261_023;
    let rules = Ruleset::default();

    let coaches = [Coach::Exhibition, Coach::Generic, Coach::Familiar];
    let mut pilots: Vec<(String, Pilot)> = coaches
        .iter()
        .map(|c| (format!("{c:?}"), Pilot::Coach(*c)))
        .collect();
    for p in Playbook::samples() {
        pilots.push((p.name.clone(), Pilot::Playbook(p.clone())));
        pilots.push((
            format!("{} (turns only)", p.name),
            Pilot::Playbook(p.clone().untriggered()),
        ));
        pilots.push((
            format!("{} (bundled)", p.name),
            Pilot::Playbook(p.bundled()),
        ));
    }
    let opponents = [
        Pilot::Coach(Coach::Exhibition),
        Pilot::Coach(Coach::Familiar),
    ];

    for arena in &arenas {
        println!("\n## {} ({} matches a row)\n", arena.name, pairings * 2);
        println!(
            "| Pilot | vs Exhibition | vs Familiar | turns | as asked | past last stage | reworded | met | ran out | brought in |"
        );
        println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");
        for (label, pilot) in &pilots {
            let rows: Vec<FaceOff> = opponents
                .iter()
                .map(|o| face_off(arena, rules, seed, pairings, [pilot, o]))
                .collect();
            let all = |f: fn(&FaceOff) -> u32| rows.iter().map(f).sum::<u32>();
            let compiled = all(|r| r.compiled);
            let share = |n: u32| {
                if compiled == 0 {
                    "".to_owned()
                } else {
                    format!("{:.0}%", 100.0 * f64::from(n) / f64::from(compiled))
                }
            };
            let played = all(|r| r.stages_played);
            let as_asked = if played == 0 {
                String::new()
            } else {
                let off = all(|r| r.no_target + r.shorthanded);
                format!(
                    "{:.0}%",
                    100.0 * f64::from(played - off) / f64::from(played)
                )
            };
            println!(
                "| {} | {:.1}% | {:.1}% | {:.1} | {} | {} | {} | {} | {} | {} |",
                label,
                100.0 * rows[0].record.score(),
                100.0 * rows[1].record.score(),
                f64::from(all(|r| r.turns)) / f64::from(all(|r| r.record.played).max(1)),
                as_asked,
                share(all(|r| r.finished)),
                share(all(|r| r.reworded)),
                if compiled == 0 {
                    String::new()
                } else {
                    all(|r| r.met).to_string()
                },
                if compiled == 0 {
                    String::new()
                } else {
                    all(|r| r.ran_out).to_string()
                },
                if compiled == 0 {
                    String::new()
                } else {
                    all(|r| r.triggered).to_string()
                },
            );
        }

        println!("\n### Playbooks against each other (row's score)\n");
        let books: Vec<Pilot> = Playbook::samples()
            .into_iter()
            .map(Pilot::Playbook)
            .collect();
        print!("| |");
        for b in &books {
            print!(" {} |", b.label());
        }
        println!();
        println!("|{}", " --- |".repeat(books.len() + 1));
        for a in &books {
            print!("| {} |", a.label());
            for b in &books {
                if a == b {
                    print!(" |");
                } else {
                    let f = face_off(arena, rules, seed, pairings, [a, b]);
                    print!(" {:.1}% |", 100.0 * f.record.score());
                }
            }
            println!();
        }
    }
}
