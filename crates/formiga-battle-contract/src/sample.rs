//! The sample colony a trip to Hill rehearses with, as Battle's roster: one of everything the
//! format carries, made through Desktop's own `World`, the same every time. Its companions have
//! lived a little, so their leanings are not all where they rest.

use crate::BattleSnapshot;
use formiga_core::{LearnedTendencies, SaveFile};
use formiga_expansion_rulebook::SessionId;

pub fn colony() -> SaveFile {
    let mut save = formiga_travel::sample::colony(3);
    for (n, creature) in save.creatures.iter_mut().enumerate() {
        let t = &mut creature.tendencies;
        let k = n as f32;
        LearnedTendencies::adjust(&mut t.exploration, 18.0 * k - 30.0);
        LearnedTendencies::adjust(&mut t.sociability, 25.0 - 11.0 * k);
        LearnedTendencies::adjust(&mut t.play, 9.0 * k);
        LearnedTendencies::adjust(&mut t.routine, -12.0 * k + 20.0);
    }
    save
}

pub fn snapshot() -> BattleSnapshot {
    crate::project_roster(
        &colony(),
        SessionId::parse(formiga_travel::sample::SESSION).expect("the sample session"),
        formiga_travel::sample::MADE,
        "0.67.6",
    )
    .expect("the sample colony projects")
}
