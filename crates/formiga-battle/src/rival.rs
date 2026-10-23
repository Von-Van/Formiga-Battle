//! The Handlers a colony meets at the Battleground when it takes the train close to home: four
//! named rivals, each with a club, a pennant, a way of playing (`formiga_battle_core::style`) and
//! a trio of their own, so the visitors are people a Handler can learn to beat rather than three
//! strangers drawn afresh each match. A rival is content: their record against this colony is the
//! career's (`career.rs`), and nothing about a rival changes how a match is played.

use formiga_battle_contract::Pennant;
use formiga_battle_core::Archetype;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rival {
    pub id: &'static str,
    pub name: &'static str,
    /// The name they are called by on a card.
    pub short: &'static str,
    pub club: &'static str,
    /// Shape, field, symbol and trim, by the ids `identity.rs` draws.
    pennant: (&'static str, &'static str, &'static str, &'static str),
    pub archetype: Archetype,
    /// The arena whose cup is theirs to defend first.
    pub home: &'static str,
    /// Draws the same three of the visiting colony every time, so their trio is known.
    pub trio_seed: u64,
}

pub const RIVALS: [Rival; 4] = [
    Rival {
        id: "hazel",
        name: "Hazel Thorne",
        short: "Hazel",
        club: "Bramble Club",
        pennant: ("pennon", "russet", "acorn", "stitched"),
        archetype: Archetype::Aggressor,
        home: "backyard",
        trio_seed: 0x4861_7a65_6c01,
    },
    Rival {
        id: "tam",
        name: "Tam Rook",
        short: "Tam",
        club: "Sluice Gate",
        pennant: ("banner", "sky", "wave", "plain"),
        archetype: Archetype::Defender,
        home: "millpond",
        trio_seed: 0x5461_6d52_6f02,
    },
    Rival {
        id: "ivy",
        name: "Ivy Larkspur",
        short: "Ivy",
        club: "Glade Walkers",
        pennant: ("swallowtail", "forest", "leaf", "brass"),
        archetype: Archetype::Tactician,
        home: "woodland",
        trio_seed: 0x4976_794c_6103,
    },
    Rival {
        id: "bram",
        name: "Bram Driftwood",
        short: "Bram",
        club: "Driftwood Cove",
        pennant: ("pennon", "gold", "shell", "stitched"),
        archetype: Archetype::Opportunist,
        home: "cove",
        trio_seed: 0x4272_616d_4404,
    },
];

impl Rival {
    pub fn pennant(&self) -> Pennant {
        let (shape, field, symbol, trim) = self.pennant;
        Pennant {
            shape: shape.to_owned(),
            field: field.to_owned(),
            symbol: symbol.to_owned(),
            trim: trim.to_owned(),
        }
    }

    /// The paper tag under their cane as a match begins: who, whose club, how they play.
    pub fn tag(&self) -> String {
        format!(
            "{} \u{b7} {} \u{b7} {}",
            self.name,
            self.club,
            self.archetype.tendency()
        )
    }
}

/// Who comes to `arena` for the colony's `meetings`-th match against the visitors: the arena's
/// own Handler first, then each of the others in turn, so every rival is met on every arena.
pub fn visiting(arena: &str, meetings: u32) -> &'static Rival {
    let home = RIVALS.iter().position(|r| r.home == arena).unwrap_or(0);
    &RIVALS[(home + meetings as usize) % RIVALS.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rival_flies_a_pennant_the_lists_know() {
        for r in &RIVALS {
            let p = r.pennant();
            let style = crate::identity::Style::of(&p);
            assert_eq!(
                style.pennant(),
                p,
                "{} flies a pennant drawn as given",
                r.name
            );
        }
    }

    #[test]
    fn each_arena_is_met_by_its_own_handler_first_and_then_every_other() {
        for arena in ["backyard", "woodland", "millpond", "cove"] {
            assert_eq!(visiting(arena, 0).home, arena);
            let mut seen: Vec<&str> = (0..4).map(|n| visiting(arena, n).id).collect();
            seen.sort();
            assert_eq!(seen, ["bram", "hazel", "ivy", "tam"]);
            assert_eq!(visiting(arena, 4), visiting(arena, 0));
        }
    }

    #[test]
    fn rivals_are_told_apart_by_name_club_and_way_of_playing() {
        let ids: std::collections::BTreeSet<_> = RIVALS.iter().map(|r| r.id).collect();
        assert_eq!(ids.len(), 4);
        for (i, a) in RIVALS.iter().enumerate() {
            assert!(
                RIVALS
                    .iter()
                    .skip(i + 1)
                    .all(|b| b.archetype != a.archetype),
                "{} and another rival play the same way",
                a.name
            );
        }
        assert_eq!(
            RIVALS[0].tag(),
            "Hazel Thorne \u{b7} Bramble Club \u{b7} presses first and sends everyone"
        );
    }
}
