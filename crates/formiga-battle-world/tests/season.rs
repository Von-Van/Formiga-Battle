//! A season runs its four events in order, points go to how far each Handler went, the four with
//! most reach the Championship, and its winner is the champion the next season defends.

use formiga_battle_world::*;

fn pool() -> Vec<String> {
    ["a", "b", "c", "d", "e", "f", "g"]
        .map(String::from)
        .to_vec()
}

/// Plays a bracket out with the higher seed always winning.
fn by_seed(b: &mut Bracket) {
    let mut n = 0;
    while let Some(i) = b.next() {
        let [x, y] = b.fixtures[i].sides.map(|s| s.unwrap());
        let winner = u8::from(y < x);
        b.record(i, &format!("m{n}"), Some(winner), Decided::Capture, 6)
            .unwrap();
        n += 1;
    }
}

#[test]
fn a_season_runs_an_open_two_regionals_and_a_championship_of_the_best_four() {
    let arenas = ["backyard", "woodland"].map(String::from).to_vec();
    let mut s = Season::new(1, &arenas, None);
    let kinds: Vec<Kind> = s.stages.iter().map(|st| st.kind).collect();
    assert_eq!(
        kinds,
        [
            Kind::Open,
            Kind::Regional,
            Kind::Regional,
            Kind::Championship
        ]
    );
    assert_eq!(s.stages[0].arena, "backyard");
    assert_eq!(s.stages[1].arena, "woodland");
    assert_eq!(s.stages[0].name("Backyard", 1), "The Backyard Open");
    assert_eq!(
        s.stages[1].name("Backyard", 1),
        "The Backyard Spring Regional"
    );
    assert_eq!(
        s.stages[2].name("Backyard", 1),
        "The Backyard Summer Regional"
    );
    assert_eq!(
        s.stages[3].name("Woodland Glade", 1),
        "The Season 1 Championship"
    );

    let open = s.entrants(&s.stages[0].clone(), &pool(), "you");
    assert_eq!(open, ["a", "b", "c", "you"]);
    let mut b = Bracket::new("o", 1, open).unwrap();
    by_seed(&mut b);
    s.finish(&b);
    assert_eq!(s.points_of("a"), 8, "the champion");
    assert_eq!(s.points_of("b"), 5, "the finalist");
    assert_eq!(s.points_of("you"), 3, "a semi-finalist");
    assert_eq!(s.played, 1);

    for _ in 0..2 {
        let stage = s.next_stage().unwrap().clone();
        let e = s.entrants(&stage, &pool(), "you");
        assert_eq!(e.len(), 8);
        assert_eq!(e[0], "a", "seeded by points");
        let mut b = Bracket::new("r", 2, e).unwrap();
        by_seed(&mut b);
        s.finish(&b);
    }
    let stage = s.next_stage().unwrap().clone();
    assert_eq!(stage.kind, Kind::Championship);
    let best = s.entrants(&stage, &pool(), "you");
    assert_eq!(best.len(), 4);
    let table: Vec<String> = s.table().into_iter().take(4).map(|(id, _)| id).collect();
    let mut sorted_best = best.clone();
    sorted_best.sort();
    let mut sorted_table = table.clone();
    sorted_table.sort();
    assert_eq!(sorted_best, sorted_table, "the four with most points");
    let points_before = s.points.clone();
    let mut b = Bracket::new("c", 3, best).unwrap();
    by_seed(&mut b);
    s.finish(&b);
    assert_eq!(s.points, points_before, "the Championship gives no points");
    assert_eq!(s.champion.as_deref(), Some("a"));
    assert!(s.finished());

    // Next season the champion defends, first seed among equals, and everyone starts at nothing.
    let next = Season::new(2, &arenas, s.champion.clone());
    assert_eq!(
        next.stages[0].arena, "woodland",
        "the events move round a place"
    );
    let e = next.entrants(
        &next.stages[1].clone(),
        &["b", "a", "c", "d", "e", "f", "g"].map(String::from),
        "you",
    );
    assert_eq!(e[0], "a", "the defending champion is first among equals");
    assert_eq!(next.points_of("a"), 0);
}

#[test]
fn an_unfinished_bracket_counts_nothing() {
    let mut s = Season::new(1, &["backyard".to_owned()], None);
    let b = Bracket::new("o", 1, ["a", "b", "c", "you"].map(String::from).to_vec()).unwrap();
    s.finish(&b);
    assert_eq!(s.played, 0);
    assert!(s.points.is_empty());
}
