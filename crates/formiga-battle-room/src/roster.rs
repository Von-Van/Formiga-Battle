//! Desktop's roster read as match profiles. The contract carries who each companion is; how that
//! becomes a profile for the rules is Battle's own, so the contract knows nothing of the rules.

use formiga_battle_contract::{BattleSnapshot, Contender, SessionId};
use formiga_battle_core::{Bond, Profile, Strength, TEAM_SIZE};
use formiga_travel::{Band, TravelRelationship};
use std::collections::BTreeSet;

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

/// A team as a roster of its own, for reading and checking: the three, and their bonds to each
/// other.
fn as_roster(team: &[Contender], bonds: &[TravelRelationship]) -> BattleSnapshot {
    BattleSnapshot {
        format: formiga_battle_contract::SNAPSHOT_FORMAT.to_owned(),
        version: formiga_battle_contract::BATTLE_FORMAT_VERSION,
        min_reader_version: 1,
        session_id: SessionId::parse("00000000000000000000000000000000").expect("a session id"),
        colony_id: "0000000000000000".to_owned(),
        created_at_utc: time::OffsetDateTime::UNIX_EPOCH,
        desktop_version: "battle".to_owned(),
        contenders: team.to_vec(),
        relationships: bonds.to_vec(),
        presentation: Default::default(),
        record: None,
    }
}

/// A team's profiles, in team order, with their bonds to each other: the same on every machine
/// for the same team.
pub fn team_profiles(team: &[Contender], bonds: &[TravelRelationship]) -> Vec<Profile> {
    profiles(&as_roster(team, bonds))
}

/// The match two teams play, as the room and both clients build it: the same seed, rules, arena
/// and profiles give the same match on every machine. The arena is set up in the variant the seed
/// draws, so both windows open on the same starts, finds and conditions without a word more
/// through the relay: both play the same Battle, which every room checks.
pub fn room_match(
    arena: formiga_battle_core::Arena,
    ruleset: formiga_battle_core::Ruleset,
    seed: u64,
    teams: [&[Contender]; 2],
    bonds: [&[TravelRelationship]; 2],
) -> Result<formiga_battle_core::Match, formiga_battle_core::game::MatchError> {
    let arena = formiga_battle_core::Variant::drawn(&arena, seed).apply(&arena)?;
    formiga_battle_core::Match::new(
        arena,
        ruleset,
        seed,
        [
            team_profiles(teams[0], bonds[0]),
            team_profiles(teams[1], bonds[1]),
        ],
    )
}

/// The bonds among a chosen few, from their whole roster.
pub fn bonds_among(roster: &BattleSnapshot, team: &[Contender]) -> Vec<TravelRelationship> {
    let ids: BTreeSet<u64> = team.iter().map(|c| c.traveler.id.0).collect();
    roster
        .relationships
        .iter()
        .filter(|r| ids.contains(&r.a.0) && ids.contains(&r.b.0))
        .copied()
        .collect()
}

/// Whether a team sent by someone else can be played and drawn: three different companions,
/// every field within Desktop's bounds, every look one the art crate can draw, every bond among
/// them.
pub fn check_team(team: &[Contender], bonds: &[TravelRelationship]) -> Result<(), String> {
    if team.len() != TEAM_SIZE {
        return Err("a team is three companions".to_owned());
    }
    as_roster(team, bonds)
        .validate()
        .map_err(|error| error.to_string())?;
    for contender in team {
        contender
            .traveler
            .to_creature()
            .map_err(|_| format!("{}'s look cannot be drawn", contender.traveler.name))?;
    }
    Ok(())
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
    fn a_team_of_three_from_desktop_checks_out_and_one_of_two_does_not() {
        let roster = sample::snapshot();
        let team: Vec<Contender> = roster.contenders[..3].to_vec();
        let bonds = bonds_among(&roster, &team);
        assert!(check_team(&team, &bonds).is_ok());
        assert!(check_team(&team[..2], &[]).is_err());
        let mut twice = team.clone();
        twice[1] = twice[0].clone();
        assert!(check_team(&twice, &[]).is_err());
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
