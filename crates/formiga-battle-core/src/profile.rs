//! Who a creature is, as Battle reads it: Desktop's own temperament and learned leanings, never a
//! taxonomy of Battle's own, turned into whole numbers once on the way in so every machine
//! resolves a match identically.
//!
//! A profile changes how a creature carries out an order (which way it walks, whether it keeps
//! close to a friend, what it does with a moment to spare) and nothing else. Every creature has
//! the same composure, the same pace and one order a turn, and no profile changes what MOVE, HOLD
//! and CONTEST do to each other. `docs/PERSONALITY.md` maps every Desktop field to what Battle
//! does with it.

use formiga_core::{LearnedTendencies, Temperament, TemperamentKind, Tension};
use serde::{Deserialize, Serialize};

/// Desktop's nine temperament axes, each in thousandths from 0 to 1000 with 500 in the middle.
/// The name of each field is the end that 1000 stands for, as in `formiga_core::Axes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nature {
    pub social: u16,
    pub energy: u16,
    pub boldness: u16,
    pub playfulness: u16,
    pub curiosity: u16,
    pub feistiness: u16,
    pub impulsiveness: u16,
    pub suspicion: u16,
    pub affection: u16,
}

impl Nature {
    pub const MIDDLING: Self = Self {
        social: 500,
        energy: 500,
        boldness: 500,
        playfulness: 500,
        curiosity: 500,
        feistiness: 500,
        impulsiveness: 500,
        suspicion: 500,
        affection: 500,
    };

    fn from_axes(axes: formiga_core::Axes) -> Self {
        let a = axes.bounded();
        let q = |v: f32| (v * 1000.0).round().clamp(0.0, 1000.0) as u16;
        Self {
            social: q(a.social),
            energy: q(a.energy),
            boldness: q(a.boldness),
            playfulness: q(a.playfulness),
            curiosity: q(a.curiosity),
            feistiness: q(a.feistiness),
            impulsiveness: q(a.impulsiveness),
            suspicion: q(a.suspicion),
            affection: q(a.affection),
        }
    }
}

/// Desktop's learned leanings, the part of a creature that drifts with how it lives, each a
/// whole number from -100 to 100, as in `formiga_core::LearnedTendencies`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Leanings {
    pub cursor_trust: i8,
    pub sociability: i8,
    pub climbing: i8,
    pub sleep_security: i8,
    pub exploration: i8,
    pub play: i8,
    pub home_affinity: i8,
    pub routine: i8,
}

impl From<&LearnedTendencies> for Leanings {
    fn from(t: &LearnedTendencies) -> Self {
        let q = |v: f32| {
            if v.is_finite() {
                v.round().clamp(-100.0, 100.0) as i8
            } else {
                0
            }
        };
        Self {
            cursor_trust: q(t.cursor_trust),
            sociability: q(t.sociability),
            climbing: q(t.climbing),
            sleep_security: q(t.sleep_security),
            exploration: q(t.exploration),
            play: q(t.play),
            home_affinity: q(t.home_affinity),
            routine: q(t.routine),
        }
    }
}

/// How strongly something is so between two creatures, in the bands Desktop hands its companion
/// apps (`formiga_travel::Band`), so retuning how bonds grow never changes a match.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Strength {
    #[default]
    None,
    Low,
    Medium,
    High,
}

/// How a creature gets on with another of its colony.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bond {
    pub with: u64,
    pub affinity: Strength,
    pub avoidance: Strength,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Desktop's stable creature id.
    pub creature_id: u64,
    pub name: String,
    pub kind: TemperamentKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tension: Option<Tension>,
    pub nature: Nature,
    pub leanings: Leanings,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bonds: Vec<Bond>,
}

impl Profile {
    /// A profile from Desktop's own record of a creature: its temperament (read, for a creature
    /// older than temperaments, the way Desktop reads it) and its learned leanings.
    pub fn from_creature(creature: &formiga_core::Creature) -> Self {
        Self::from_parts(
            creature.id,
            &creature.name,
            &creature.temperament(),
            &creature.tendencies,
        )
    }

    pub fn from_parts(
        creature_id: u64,
        name: &str,
        temperament: &Temperament,
        tendencies: &LearnedTendencies,
    ) -> Self {
        Self {
            creature_id,
            name: name.to_owned(),
            kind: temperament.kind,
            tension: temperament.tension,
            nature: Nature::from_axes(temperament.axes),
            leanings: tendencies.into(),
            bonds: Vec::new(),
        }
    }

    pub fn bond_with(&self, creature_id: u64) -> Option<Bond> {
        self.bonds.iter().copied().find(|b| b.with == creature_id)
    }

    /// The handful of leanings the behaviour interpreter reads, worked out once.
    pub fn behavior(&self) -> Behavior {
        Behavior::of(self)
    }
}

/// What a profile means on the board, each bounded to -500..=500 unless it says otherwise. These
/// are preferences between routes and moments, weighed against the time a detour costs; none is
/// a measure of how good a creature is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Behavior {
    /// Keeps out of reach of the other team and close to cover; at the other end, walks right
    /// past them. From boldness, suspicion and how securely it sleeps.
    pub caution: i16,
    /// Keeps beside its teammates. From how social it is and how sociable it has become.
    pub togetherness: i16,
    /// Takes the outside lanes rather than the middle. From curiosity and how much it has come to
    /// explore, less how much it has come to love home.
    pub wanderlust: i16,
    /// On the way to one contest, takes another that walks into reach. From feistiness,
    /// impulsiveness, playfulness and how much it has come to play.
    pub opportunism: i16,
    /// With a moment to spare, stays where it was told rather than shifting. From patience, calm
    /// and how much it has come to keep a routine.
    pub patience: i16,
    /// In thousandths, 40 to 140: how often it takes its second-best step instead of its best.
    /// From impulsiveness, the same side Desktop's `decision_temperature` comes from.
    pub variability: u16,
    /// Steps back after a contest. Brave-but-nervous and dramatic-but-cowardly creatures always
    /// do; anyone else only if it is very cautious.
    pub steps_back: bool,
}

impl Behavior {
    fn of(p: &Profile) -> Self {
        let n = p.nature;
        let l = p.leanings;
        let lean = |v: u16| i32::from(v) - 500;
        let bound = |v: i32| v.clamp(-500, 500) as i16;
        let caution = bound(
            (-lean(n.boldness) * 6 + lean(n.suspicion) * 4) / 10 - i32::from(l.sleep_security),
        );
        let togetherness = bound(lean(n.social) * 8 / 10 + i32::from(l.sociability) * 2);
        let wanderlust = bound(
            lean(n.curiosity) * 6 / 10 + i32::from(l.exploration) * 2 - i32::from(l.home_affinity),
        );
        let opportunism = bound(
            (lean(n.feistiness) * 5 + lean(n.impulsiveness) * 3 + lean(n.playfulness) * 2) / 10
                + i32::from(l.play),
        );
        let patience = bound(
            (-lean(n.impulsiveness) * 7 - lean(n.energy) * 3) / 10 + i32::from(l.routine) * 2,
        );
        let variability = 40 + n.impulsiveness / 10;
        let steps_back = matches!(
            p.tension,
            Some(Tension::BraveButNervous | Tension::DramaticButCowardly)
        ) || caution > 250;
        Self {
            caution,
            togetherness,
            wanderlust,
            opportunism,
            patience,
            variability,
            steps_back,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_axes(f: impl Fn(&mut formiga_core::Axes)) -> Profile {
        let mut axes = formiga_core::Axes::MIDDLING;
        f(&mut axes);
        let temperament = Temperament {
            kind: Temperament::closest_kind(axes),
            axes,
            tension: None,
        };
        Profile::from_parts(1, "Pip", &temperament, &LearnedTendencies::default())
    }

    #[test]
    fn a_middling_creature_leans_no_way_at_all() {
        let b = with_axes(|_| {}).behavior();
        assert_eq!(
            (
                b.caution,
                b.togetherness,
                b.wanderlust,
                b.opportunism,
                b.patience
            ),
            (0, 0, 0, 0, 0)
        );
        assert_eq!(b.variability, 90);
        assert!(!b.steps_back);
    }

    #[test]
    fn every_leaning_stays_inside_its_bounds_at_the_ends_of_every_scale() {
        for end in [0.0, 1.0] {
            let mut p = with_axes(|a| {
                *a = formiga_core::Axes {
                    social: end,
                    energy: end,
                    boldness: 1.0 - end,
                    playfulness: end,
                    curiosity: end,
                    feistiness: end,
                    impulsiveness: end,
                    suspicion: end,
                    affection: end,
                }
            });
            for v in [-100, 100] {
                p.leanings = Leanings {
                    cursor_trust: v,
                    sociability: v,
                    climbing: v,
                    sleep_security: -v,
                    exploration: v,
                    play: v,
                    home_affinity: -v,
                    routine: v,
                };
                let b = p.behavior();
                for value in [
                    b.caution,
                    b.togetherness,
                    b.wanderlust,
                    b.opportunism,
                    b.patience,
                ] {
                    assert!((-500..=500).contains(&value));
                }
                assert!((40..=140).contains(&b.variability));
            }
        }
    }

    #[test]
    fn learned_leanings_move_behaviour_the_way_their_names_say() {
        let mut p = with_axes(|_| {});
        p.leanings.exploration = 80;
        p.leanings.sociability = 60;
        let b = p.behavior();
        assert!(b.wanderlust > 100);
        assert!(b.togetherness > 100);
    }

    #[test]
    fn a_creature_older_than_temperaments_reads_as_desktop_reads_it() {
        let genome = Temperament {
            kind: TemperamentKind::Explorer,
            axes: formiga_core::Axes {
                curiosity: 0.9,
                ..formiga_core::Axes::MIDDLING
            },
            tension: None,
        }
        .genome(0.5);
        let temperament = Temperament::read(&genome);
        let p = Profile::from_parts(9, "Moss", &temperament, &LearnedTendencies::default());
        assert_eq!(p.kind, temperament.kind);
        assert_eq!(p.nature.feistiness, 500);
    }
}
