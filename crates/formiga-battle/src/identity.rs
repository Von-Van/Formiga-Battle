//! Who the Handler is, and the trios they name: the choices a pennant is made from, the standing
//! a Handler's record gives them, and which trio three picked companions are. None of it changes
//! how a match is played. Desktop keeps the Handler card and the trios with the rest of the
//! colony's Battle record (`formiga-battle-contract`'s `handler` module); this is what Battle
//! makes of them.

use crate::rewards::{CUP_WINS, CUPS, Cup};
use formiga_art::paint::Ramp;
use formiga_battle_contract::{
    BattleRecord, HandlerCard, Pennant, Registry, Trio, TrioCard, handler::handler_limits,
    lineup_of,
};
use formiga_expansion_rulebook::sanitize_text;

/// The longest a Handler's name may be.
pub const NAME_CHARS: usize = handler_limits::MAX_HANDLER_CHARS;
pub const CLUB_CHARS: usize = handler_limits::MAX_CLUB_CHARS;
pub const TRIO_CHARS: usize = handler_limits::MAX_TRIO_CHARS;
/// Trios that can be flown at once: the rest of the record keeps room for retired ones.
pub const MAX_ACTIVE_TRIOS: usize = 8;

/// A pennant's cut.
pub const SHAPES: [(&str, &str); 3] = [
    ("pennon", "Pennon"),
    ("swallowtail", "Swallowtail"),
    ("banner", "Banner"),
];

/// A pennant's cloth: the notebook's own inks and cloths, each a ramp to Formiga's rules.
pub const FIELDS: [(&str, &str, Ramp); 8] = [
    (
        "forest",
        "Forest",
        Ramp::new(0x10281f, 0x1c4137, 0x285a4b, 0x3b7563, 0x5a9480),
    ),
    (
        "russet",
        "Russet",
        Ramp::new(0x4a1d12, 0x7a3226, 0xa84a32, 0xcc6a4c, 0xe8977a),
    ),
    (
        "sky",
        "Sky",
        Ramp::new(0x1d3a52, 0x2f5f82, 0x4a86b0, 0x74abd0, 0xa9d6ea),
    ),
    (
        "rose",
        "Rose",
        Ramp::new(0x5a2034, 0x8a3654, 0xb8577a, 0xd6809e, 0xf0b4c6),
    ),
    (
        "gold",
        "Mustard",
        Ramp::new(0x6a4a12, 0x9a7020, 0xc89a34, 0xe8c25a, 0xf8e3a0),
    ),
    (
        "plum",
        "Plum",
        Ramp::new(0x2a1838, 0x46295a, 0x664080, 0x8a62a4, 0xb498c8),
    ),
    (
        "slate",
        "Slate",
        Ramp::new(0x22282e, 0x3a444c, 0x56626c, 0x7a8790, 0xa8b4bc),
    ),
    (
        "cream",
        "Cream",
        Ramp::new(0x9a8a62, 0xc8b88e, 0xe6d8b2, 0xf6ecd0, 0xfffcf2),
    ),
];

/// What the pennant carries. Each is drawn from a little picture of its own (`pennant::GLYPHS`).
pub const SYMBOLS: [(&str, &str); 12] = [
    ("acorn", "Acorn"),
    ("leaf", "Leaf"),
    ("sun", "Sun"),
    ("moon", "Moon"),
    ("star", "Star"),
    ("shell", "Shell"),
    ("mushroom", "Mushroom"),
    ("flower", "Flower"),
    ("wave", "Wave"),
    ("key", "Key"),
    ("feather", "Feather"),
    ("bell", "Bell"),
];

/// How its edge is finished.
pub const TRIMS: [(&str, &str); 3] = [
    ("plain", "Plain"),
    ("stitched", "Stitched"),
    ("brass", "Brass edge"),
];

/// A pennant as places in the lists above. A name the lists no longer have is read as their first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Style {
    pub shape: usize,
    pub field: usize,
    pub symbol: usize,
    pub trim: usize,
}

fn place<T>(list: &[(&str, T)], id: &str) -> usize {
    list.iter().position(|(i, _)| *i == id).unwrap_or(0)
}

impl Style {
    pub fn of(pennant: &Pennant) -> Self {
        Self {
            shape: place(&SHAPES, &pennant.shape),
            field: FIELDS
                .iter()
                .position(|(i, _, _)| *i == pennant.field)
                .unwrap_or(0),
            symbol: place(&SYMBOLS, &pennant.symbol),
            trim: place(&TRIMS, &pennant.trim),
        }
    }

    pub fn pennant(self) -> Pennant {
        Pennant {
            shape: SHAPES[self.shape % SHAPES.len()].0.to_owned(),
            field: FIELDS[self.field % FIELDS.len()].0.to_owned(),
            symbol: SYMBOLS[self.symbol % SYMBOLS.len()].0.to_owned(),
            trim: TRIMS[self.trim % TRIMS.len()].0.to_owned(),
        }
    }

    pub fn ramp(self) -> Ramp {
        FIELDS[self.field % FIELDS.len()].2
    }

    /// The pennant in words: "a forest swallowtail with an acorn, stitched".
    pub fn words(self) -> String {
        let trim = match self.trim % TRIMS.len() {
            0 => String::new(),
            1 => ", stitched".to_owned(),
            _ => ", with a brass edge".to_owned(),
        };
        format!(
            "a {} {} with {} {}{trim}",
            FIELDS[self.field % FIELDS.len()].1.to_lowercase(),
            SHAPES[self.shape % SHAPES.len()].1.to_lowercase(),
            article(SYMBOLS[self.symbol % SYMBOLS.len()].1),
            SYMBOLS[self.symbol % SYMBOLS.len()].1.to_lowercase(),
        )
    }
}

fn article(word: &str) -> &'static str {
    if word.starts_with(['A', 'E', 'I', 'O', 'U']) {
        "an"
    } else {
        "a"
    }
}

/// The pennant a Handler flies before choosing one.
pub fn first_pennant() -> Pennant {
    Style::default().pennant()
}

/// The pennant a trio flies: the Handler's, in the trio's own colour if it has one. Only the
/// colour is the trio's, so it follows the Handler's cut, symbol and trim when they change.
pub fn trio_style(handler: Style, trio: &Trio) -> Style {
    match &trio.pennant {
        Some(own) => Style {
            field: Style::of(own).field,
            ..handler
        },
        None => handler,
    }
}

/// The Handler card as it stands, or one made from the name signed on the Friend page, flying the
/// first pennant. `None` until there is a name.
pub fn card_with_name(record: &BattleRecord, name: &str) -> Option<HandlerCard> {
    let name = sanitize_text(name, NAME_CHARS);
    if name.is_empty() {
        return record.handler.clone();
    }
    Some(match &record.handler {
        Some(card) => HandlerCard {
            name,
            ..card.clone()
        },
        None => HandlerCard {
            name,
            club: None,
            pennant: first_pennant(),
        },
    })
}

/// A Handler's standing on the Open Circuit, from the points they have: a title, and what the
/// next one needs. The numbers are provisional, like the rules'.
pub const STANDINGS: [(u32, &str); 6] = [
    (0, "New Handler"),
    (30, "Handler"),
    (100, "Seasoned Handler"),
    (250, "Circuit Regular"),
    (500, "Circuit Veteran"),
    (1000, "Circuit Legend"),
];

/// What a Handler's record says of them, in plain sight: their Open Circuit standing, and how far
/// along the Championship Circuit against the visitors they are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    pub title: &'static str,
    pub points: u32,
    /// The next title, and the points it needs.
    pub next: Option<(&'static str, u32)>,
    /// Every cup won, in the order they open.
    pub cups: Vec<&'static Cup>,
}

impl Standing {
    pub fn of(record: &BattleRecord) -> Self {
        let points = record.circuit_points;
        let at = STANDINGS
            .iter()
            .rposition(|(need, _)| points >= *need)
            .unwrap_or(0);
        Self {
            title: STANDINGS[at].1,
            points,
            next: STANDINGS.get(at + 1).map(|(need, title)| (*title, *need)),
            cups: CUPS
                .iter()
                .filter(|c| record.cup_wins(c.id) >= CUP_WINS)
                .collect(),
        }
    }

    /// Every cup of the Championship Circuit is won: the game against the visitors is complete.
    pub fn champion(&self) -> bool {
        self.cups.len() == CUPS.len()
    }

    /// How far along the circuit, in a few words.
    pub fn circuit(&self) -> String {
        match self.cups.len() {
            0 => "No cups yet".to_owned(),
            n if n == CUPS.len() => "Champion of the Circuit".to_owned(),
            n => format!("{n} of {} cups won", CUPS.len()),
        }
    }

    /// The standing in one line, as a friend sees it: "Seasoned Handler · Champion of the Circuit".
    pub fn line(&self) -> String {
        if self.champion() {
            format!("{} · Champion of the Circuit", self.title)
        } else {
            self.title.to_owned()
        }
    }

    /// How it is earned, said beside it.
    pub fn how(&self) -> String {
        let next = match self.next {
            Some((title, need)) => format!(
                " {} more Open Circuit points make you {} {title}.",
                need - self.points,
                article(title),
            ),
            None => String::new(),
        };
        format!(
            "Your title comes from Open Circuit points, earned playing friends.{next} The cups come \
             from the visitors: three wins on a place win its cup."
        )
    }
}

/// The trio these three are, if they are one still flying.
pub fn trio_of<'a>(record: &'a BattleRecord, members: &[u64]) -> Option<&'a Trio> {
    if members.len() != 3 {
        return None;
    }
    record.trio_of(members)
}

/// Trios still flying that have two of these three: one of them could take the third in place of
/// the one it has.
pub fn swaps<'a>(record: &'a BattleRecord, members: &[u64]) -> Vec<&'a Trio> {
    if members.len() != 3 || trio_of(record, members).is_some() {
        return Vec::new();
    }
    record
        .trios
        .iter()
        .filter(|t| !t.retired && t.members.iter().filter(|m| members.contains(m)).count() == 2)
        .collect()
}

/// Trios still flying.
pub fn active(record: &BattleRecord) -> usize {
    record.trios.iter().filter(|t| !t.retired).count()
}

/// Whether another trio can be named now, and if not, why.
pub fn can_name(record: &BattleRecord) -> Result<(), &'static str> {
    if active(record) >= MAX_ACTIVE_TRIOS {
        Err("Retire a trio first: eight can fly at once.")
    } else if record.trios.len() >= handler_limits::MAX_TRIOS
        && !record.trios.iter().any(|t| t.retired)
    {
        Err("Your notebook has no room for another trio.")
    } else {
        Ok(())
    }
}

/// A trio's name as it is kept: plain, and never empty.
pub fn trio_name(raw: &str) -> Option<String> {
    Some(sanitize_text(raw, TRIO_CHARS)).filter(|n| !n.is_empty())
}

/// 64 bits from the system's randomness, as 16 hex digits: a new trio's id.
pub fn new_trio_id() -> String {
    let mut bytes = [0u8; 8];
    if getrandom::fill(&mut bytes).is_err() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        bytes = (nanos as u64).to_le_bytes();
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A trio named now for these three.
pub fn named(name: &str, members: &[u64]) -> Option<TrioCard> {
    Some(TrioCard {
        id: new_trio_id(),
        name: trio_name(name)?,
        members: lineup_of(members),
        pennant: None,
        retired: false,
    })
}

/// Everything the Handler has said of themselves and their trios, for Desktop to keep.
pub fn registry(record: &BattleRecord) -> Registry {
    Registry {
        handler: record.handler.clone(),
        trios: record.trios.iter().map(Trio::card).collect(),
    }
}

/// Played, won and drawn, in words: "7 played, 5 won".
pub fn tally_words(played: u32, won: u32, drawn: u32) -> String {
    match (played, drawn) {
        (0, _) => "Not played yet".to_owned(),
        (_, 0) => format!("{played} played, {won} won"),
        _ => format!("{played} played, {won} won, {drawn} drawn"),
    }
}

/// A trio's whole record in words.
pub fn record_words(trio: &Trio) -> String {
    let all = trio.total();
    tally_words(all.played, all.won, all.drawn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_battle_contract::sample;

    #[test]
    fn every_pennant_reads_back_as_itself_and_an_unknown_part_as_the_first() {
        for shape in 0..SHAPES.len() {
            for field in 0..FIELDS.len() {
                for symbol in 0..SYMBOLS.len() {
                    for trim in 0..TRIMS.len() {
                        let style = Style {
                            shape,
                            field,
                            symbol,
                            trim,
                        };
                        let pennant = style.pennant();
                        assert!(pennant.holds(), "{pennant:?}");
                        assert_eq!(Style::of(&pennant), style);
                    }
                }
            }
        }
        let mut odd = first_pennant();
        odd.symbol = "dragon".to_owned();
        assert_eq!(Style::of(&odd), Style::default());
        assert_eq!(Style::default().words(), "a forest pennon with an acorn",);
    }

    #[test]
    fn standing_rises_with_points_and_says_what_comes_next() {
        let mut record = BattleRecord::default();
        let new = Standing::of(&record);
        assert_eq!(
            (new.title, new.next),
            ("New Handler", Some(("Handler", 30)))
        );
        assert_eq!(new.circuit(), "No cups yet");
        record.circuit_points = 260;
        let regular = Standing::of(&record);
        assert_eq!(regular.title, "Circuit Regular");
        assert!(
            regular
                .how()
                .contains("240 more Open Circuit points make you a Circuit Veteran")
        );
        record.circuit_points = 5000;
        assert_eq!(Standing::of(&record).next, None);
        for cup in &CUPS {
            record.cups.push(formiga_battle_contract::CupWins {
                cup: cup.id.to_owned(),
                wins: CUP_WINS,
            });
        }
        let champion = Standing::of(&record);
        assert!(champion.champion());
        assert_eq!(champion.line(), "Circuit Legend · Champion of the Circuit");
    }

    #[test]
    fn three_picked_in_any_order_are_their_trio_and_two_of_them_offer_a_swap() {
        let record = sample::record();
        let thistle = &record.trios[0];
        let mut picked = thistle.members.clone();
        picked.rotate_left(1);
        assert_eq!(trio_of(&record, &picked).map(|t| &t.id), Some(&thistle.id));
        assert!(swaps(&record, &picked).is_empty());
        picked[0] = 424_242;
        assert!(trio_of(&record, &picked).is_none());
        assert_eq!(swaps(&record, &picked).len(), 1);
        picked[1] = 434_343;
        assert!(swaps(&record, &picked).is_empty());
    }

    #[test]
    fn a_trio_is_named_plainly_with_a_fresh_id() {
        let card = named("  The  Thistle\u{202E} Three ", &[9, 3, 5]).unwrap();
        assert_eq!(card.name, "The Thistle Three");
        assert_eq!(card.members, vec![3, 5, 9]);
        assert!(card.holds());
        assert_ne!(card.id, named("x", &[1, 2, 3]).unwrap().id);
        assert!(named("\u{200b}", &[1, 2, 3]).is_none());
    }

    #[test]
    fn a_trio_of_its_own_colour_keeps_the_handlers_pennant_as_it_changes() {
        let mut trio = sample::record().trios[0].clone();
        let handler = Style {
            shape: 1,
            field: 0,
            symbol: 4,
            trim: 2,
        };
        trio.pennant = None;
        assert_eq!(trio_style(handler, &trio), handler);
        // Given its own colour under an older pennant, it takes only the colour from it.
        trio.pennant = Some(
            Style {
                field: 3,
                ..Style::default()
            }
            .pennant(),
        );
        assert_eq!(
            trio_style(handler, &trio),
            Style {
                field: 3,
                ..handler
            }
        );
    }

    #[test]
    fn a_name_signed_for_a_friend_becomes_the_handlers_and_keeps_their_pennant() {
        let mut record = BattleRecord::default();
        assert_eq!(card_with_name(&record, "  "), None);
        let card = card_with_name(&record, "Sam").unwrap();
        assert_eq!(
            (card.name.as_str(), card.pennant.clone()),
            ("Sam", first_pennant())
        );
        record.handler = sample::record().handler;
        let renamed = card_with_name(&record, "Samwise").unwrap();
        assert_eq!(renamed.pennant, record.handler.as_ref().unwrap().pennant);
        assert_eq!(renamed.club.as_deref(), Some("Hedge End Club"));
    }
}
