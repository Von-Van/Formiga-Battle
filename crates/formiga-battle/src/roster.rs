//! Desktop's roster read as match profiles. The contract carries who each companion is; how that
//! becomes a profile for the rules is Battle's own, so the contract knows nothing of the rules.

use formiga_battle_contract::{BattleSnapshot, Contender};
use formiga_battle_core::{Bond, Profile, Strength};
use formiga_travel::Band;

fn strength(band: Band) -> Strength {
    match band {
        Band::None => Strength::None,
        Band::Low => Strength::Low,
        Band::Medium => Strength::Medium,
        Band::High => Strength::High,
    }
}

/// One contender as a match profile, with its bonds to everyone else on the roster.
pub fn profile(snapshot: &BattleSnapshot, contender: &Contender) -> Profile {
    let t = &contender.traveler;
    let temperament = formiga_core::Temperament {
        kind: t.character.temperament.into(),
        axes: t.character.axes.into(),
        tension: t.character.tension.map(Into::into),
    };
    let l = contender.leanings;
    let tendencies = formiga_core::LearnedTendencies {
        cursor_trust: f32::from(l.cursor_trust),
        sociability: f32::from(l.sociability),
        climbing: f32::from(l.climbing),
        sleep_security: f32::from(l.sleep_security),
        exploration: f32::from(l.exploration),
        play: f32::from(l.play),
        home_affinity: f32::from(l.home_affinity),
        routine: f32::from(l.routine),
    };
    let mut profile = Profile::from_parts(t.id.0, &t.name, &temperament, &tendencies);
    profile.bonds = snapshot
        .relationships
        .iter()
        .filter_map(|r| {
            let with = if r.a == t.id {
                r.b
            } else if r.b == t.id {
                r.a
            } else {
                return None;
            };
            Some(Bond {
                with: with.0,
                affinity: strength(r.affinity),
                avoidance: strength(r.avoidance),
            })
        })
        .collect();
    profile
}

/// Every contender as a match profile, in roster order.
pub fn profiles(snapshot: &BattleSnapshot) -> Vec<Profile> {
    snapshot
        .contenders
        .iter()
        .map(|c| profile(snapshot, c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_battle_contract::sample;

    #[test]
    fn every_companion_reads_as_desktop_reads_it() {
        let save = sample::colony();
        let profiles = profiles(&sample::snapshot());
        assert_eq!(profiles.len(), save.creatures.len());
        for (profile, creature) in profiles.iter().zip(&save.creatures) {
            let direct = Profile::from_creature(creature);
            assert_eq!(profile.creature_id, direct.creature_id);
            assert_eq!(profile.kind, direct.kind);
            assert_eq!(profile.tension, direct.tension);
            assert_eq!(profile.nature, direct.nature);
            assert_eq!(profile.leanings, direct.leanings);
        }
    }

    #[test]
    fn bonds_come_in_desktops_bands_both_ways_round() {
        let profiles = profiles(&sample::snapshot());
        let (a, b) = (&profiles[0], &profiles[1]);
        let ab = a.bond_with(b.creature_id).expect("a bond");
        let ba = b.bond_with(a.creature_id).expect("the same bond");
        assert_eq!(ab.affinity, ba.affinity);
        assert_eq!(ab.avoidance, ba.avoidance);
    }

    #[test]
    fn a_visiting_colony_brings_companions_of_its_own() {
        let home = profiles(&sample::snapshot());
        let visitors = profiles(&sample::visitors(9));
        assert!(visitors.len() >= 3);
        assert!(
            visitors
                .iter()
                .any(|v| !home.iter().any(|h| h.nature == v.nature))
        );
    }
}
