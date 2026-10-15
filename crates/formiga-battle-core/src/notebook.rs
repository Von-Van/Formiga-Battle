//! The Handler's Notebook: what each creature was seen to do, read from a match's events and
//! nothing else, and the few plain sentences that can be said about it.
//!
//! A note is only ever a pattern in what actually happened: never a number, a rating, or a guess
//! about what a creature is like inside. It is kept over a creature's last few matches, so as
//! Desktop's leanings drift and the creature walks differently, its notes change with it.

use crate::event::{Event, EventKind, Reason, Victory};
use crate::game::Match;
use crate::order::Slot;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The matches a creature's notes are drawn from, most recent last.
pub const MATCHES_KEPT: usize = 10;
/// The moments kept for a creature, most recent last.
pub const MOMENTS_KEPT: usize = 8;

/// Something a creature was seen to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sign {
    /// Went round so as to stay out of reach.
    KeptOutOfReach,
    /// Went round the yard while the other team was in it.
    WentRoundTheYard,
    /// Kept beside a teammate.
    KeptNearFriend,
    /// Kept away from a teammate.
    KeptApart,
    /// Took an outside lane.
    TookTheEdge,
    /// Kept to cover.
    KeptToCover,
    /// Changed its mind on the way.
    ChangedItsMind,
    /// Walked straight past the other team.
    WalkedRightPast,
    /// With time to spare, shuffled over to a friend.
    ShuffledToFriend,
    /// With time to spare, tucked itself away.
    TuckedAway,
    /// Stepped back after a contest.
    SteppedBack,
    /// Took a contest against someone wandering past, instead of the one it was sent for.
    TookAChance,
    /// Turned a contest away while holding.
    HeldFirm,
    /// Caught someone on the move.
    CaughtSomeone,
    /// Waited rather than walk in among the other team.
    HungBack,
    /// Came at a holder from another side with a friend and broke its hold.
    Flanked,
}

impl Sign {
    pub const ALL: [Self; 16] = [
        Self::KeptOutOfReach,
        Self::WentRoundTheYard,
        Self::KeptNearFriend,
        Self::KeptApart,
        Self::TookTheEdge,
        Self::KeptToCover,
        Self::ChangedItsMind,
        Self::WalkedRightPast,
        Self::ShuffledToFriend,
        Self::TuckedAway,
        Self::SteppedBack,
        Self::TookAChance,
        Self::HeldFirm,
        Self::CaughtSomeone,
        Self::HungBack,
        Self::Flanked,
    ];

    /// Its name in Desktop's keeping, the same as it is written in JSON. Never changed: a page
    /// Desktop keeps names signs this way.
    pub const fn id(self) -> &'static str {
        match self {
            Self::KeptOutOfReach => "kept_out_of_reach",
            Self::WentRoundTheYard => "went_round_the_yard",
            Self::KeptNearFriend => "kept_near_friend",
            Self::KeptApart => "kept_apart",
            Self::TookTheEdge => "took_the_edge",
            Self::KeptToCover => "kept_to_cover",
            Self::ChangedItsMind => "changed_its_mind",
            Self::WalkedRightPast => "walked_right_past",
            Self::ShuffledToFriend => "shuffled_to_friend",
            Self::TuckedAway => "tucked_away",
            Self::SteppedBack => "stepped_back",
            Self::TookAChance => "took_a_chance",
            Self::HeldFirm => "held_firm",
            Self::CaughtSomeone => "caught_someone",
            Self::HungBack => "hung_back",
            Self::Flanked => "flanked",
        }
    }

    /// A sign by that name; `None` for one a newer Battle wrote that this one does not know.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.id() == id)
    }
}

/// What one match showed of one creature.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Showing {
    /// Steps it took, every one a choice of way.
    pub steps: u16,
    pub signs: BTreeMap<Sign, u16>,
    /// The teammates it kept beside, by Desktop's creature id.
    pub friends: BTreeMap<u64, u16>,
}

impl Showing {
    fn mark(&mut self, sign: Sign) {
        *self.signs.entry(sign).or_default() += 1;
    }

    fn add(&mut self, other: &Self) {
        self.steps = self.steps.saturating_add(other.steps);
        for (sign, n) in &other.signs {
            let entry = self.signs.entry(*sign).or_default();
            *entry = entry.saturating_add(*n);
        }
        for (id, n) in &other.friends {
            let entry = self.friends.entry(*id).or_default();
            *entry = entry.saturating_add(*n);
        }
    }
}

/// What one match gives the notebook for one creature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seen {
    pub creature_id: u64,
    pub showing: Showing,
    /// Moments worth remembering, in words.
    pub moments: Vec<String>,
}

/// Everything a match showed of each creature on one side, from all its turns' events.
pub fn observe(m: &Match, side: crate::order::Side, events: &[Event]) -> Vec<Seen> {
    let name = |s: &Slot| m.profile(*s).name.clone();
    let mut seen: BTreeMap<Slot, Seen> = m
        .team_all(side)
        .map(|slot| {
            (
                slot,
                Seen {
                    creature_id: m.profile(slot).creature_id,
                    showing: Showing::default(),
                    moments: Vec::new(),
                },
            )
        })
        .collect();
    let mut moment = |slot: &Slot, text: String| {
        if let Some(entry) = seen.get_mut(slot)
            && !entry.moments.contains(&text)
        {
            entry.moments.push(text);
        }
    };
    // Who caught whom, so a knockout that follows can be told as theirs.
    let mut last_caught_by: BTreeMap<Slot, Vec<Slot>> = BTreeMap::new();
    let mut signs: Vec<(Slot, Sign)> = Vec::new();
    let mut friends: Vec<(Slot, u64)> = Vec::new();
    let mut steps: Vec<Slot> = Vec::new();
    for event in events {
        match &event.kind {
            EventKind::Stepped { slot, reason, .. } => {
                steps.push(*slot);
                let sign = match reason {
                    Reason::Direct => None,
                    Reason::AvoidedExposure => Some(Sign::KeptOutOfReach),
                    Reason::AvoidedContestedCentre => Some(Sign::WentRoundTheYard),
                    Reason::KeptNearAlly { ally } => {
                        friends.push((*slot, m.profile(*ally).creature_id));
                        Some(Sign::KeptNearFriend)
                    }
                    Reason::KeptApartFrom { .. } => Some(Sign::KeptApart),
                    Reason::SideRoute => Some(Sign::TookTheEdge),
                    Reason::TookCover => Some(Sign::KeptToCover),
                    Reason::OnImpulse => Some(Sign::ChangedItsMind),
                    Reason::Bold => Some(Sign::WalkedRightPast),
                    Reason::FollowedUpToAlly { ally } => {
                        friends.push((*slot, m.profile(*ally).creature_id));
                        Some(Sign::ShuffledToFriend)
                    }
                    Reason::FollowedUpToCover => Some(Sign::TuckedAway),
                    Reason::SteppedBack => Some(Sign::SteppedBack),
                    Reason::HungBack => Some(Sign::HungBack),
                };
                if let Some(sign) = sign {
                    signs.push((*slot, sign));
                }
            }
            EventKind::HungBack { slot, .. } => signs.push((*slot, Sign::HungBack)),
            EventKind::TookOpportunity { slot, took, .. } => {
                signs.push((*slot, Sign::TookAChance));
                moment(
                    slot,
                    format!("Spotted {} wandering past and went for them", name(took)),
                );
            }
            EventKind::Countered {
                target,
                holders,
                contesters,
                pressed,
            } => {
                for h in holders {
                    signs.push((*h, Sign::HeldFirm));
                }
                if *pressed && contesters.len() >= 2 {
                    moment(target, "Held firm against two at once".to_owned());
                } else if holders.len() >= 2 {
                    for h in holders {
                        moment(h, "Held the line side by side with a friend".to_owned());
                    }
                }
            }
            EventKind::Flanked { target, by, .. } => {
                for c in by {
                    signs.push((*c, Sign::Flanked));
                }
                moment(
                    target,
                    "Was caught from two sides and lost its spot".to_owned(),
                );
                if by.len() >= 2 {
                    for c in by {
                        moment(c, format!("Flanked {} with a friend", name(target)));
                    }
                }
            }
            EventKind::Interrupted { target, by } => {
                for c in by {
                    signs.push((*c, Sign::CaughtSomeone));
                }
                last_caught_by.insert(*target, by.clone());
            }
            EventKind::KnockedOut { slot } => {
                if let Some(by) = last_caught_by.get(slot) {
                    for c in by {
                        moment(c, format!("Sent {} into a dizzy tumble", name(slot)));
                    }
                }
                moment(slot, "Took a dizzy tumble, and was fine".to_owned());
            }
            EventKind::LeverPulled { slot, .. } => {
                moment(slot, "Pulled the lever and swung the gate".to_owned());
            }
            EventKind::Finished { outcome }
                if outcome.victory == Some(Victory::Capture) && outcome.winner == Some(side) =>
            {
                for slot in m.team(side) {
                    if m.fighter(slot).holding && m.arena.in_zone(m.fighter(slot).at) {
                        moment(&slot, "Was holding the beacon when it lit".to_owned());
                    }
                }
            }
            _ => {}
        }
    }
    for slot in steps {
        if let Some(entry) = seen.get_mut(&slot) {
            entry.showing.steps = entry.showing.steps.saturating_add(1);
        }
    }
    for (slot, sign) in signs {
        if let Some(entry) = seen.get_mut(&slot) {
            entry.showing.mark(sign);
        }
    }
    for (slot, id) in friends {
        if let Some(entry) = seen.get_mut(&slot) {
            *entry.showing.friends.entry(id).or_default() += 1;
        }
    }
    seen.into_values().collect()
}

/// One creature's page of the notebook.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub creature_id: u64,
    pub name: String,
    pub matches: u32,
    pub wins: u32,
    /// What each of its last few matches showed, most recent last.
    pub recent: Vec<Showing>,
    /// Moments worth remembering, most recent last.
    pub moments: Vec<String>,
    /// Teammates it has played beside, by creature id, with their names as last seen.
    pub teammates: BTreeMap<u64, String>,
}

impl Page {
    /// Adds one match.
    pub fn record(&mut self, seen: &Seen, won: bool, name: &str, teammates: &[(u64, String)]) {
        self.creature_id = seen.creature_id;
        self.name = name.to_owned();
        self.matches += 1;
        self.wins += u32::from(won);
        self.recent.push(seen.showing.clone());
        if self.recent.len() > MATCHES_KEPT {
            self.recent.remove(0);
        }
        for text in &seen.moments {
            self.moments.retain(|m| m != text);
            self.moments.push(text.clone());
        }
        while self.moments.len() > MOMENTS_KEPT {
            self.moments.remove(0);
        }
        for (id, mate) in teammates {
            if *id != seen.creature_id {
                self.teammates.insert(*id, mate.clone());
            }
        }
    }

    /// The patterns its recent matches show, most telling first, in plain words.
    pub fn notes(&self) -> Vec<String> {
        let mut all = Showing::default();
        for showing in &self.recent {
            all.add(showing);
        }
        let mut notes: Vec<(u32, String)> = Vec::new();
        let steps = u32::from(all.steps.max(1));
        let count = |sign| u32::from(all.signs.get(&sign).copied().unwrap_or(0));
        let way = |sign, words: &str| {
            let n = count(sign);
            // A way of walking is told once it is a habit, not an accident.
            if n >= 3 && n * 5 >= steps {
                let often = if n * 3 >= steps { "Often" } else { "Sometimes" };
                Some((n * 100 / steps, format!("{often} {words}")))
            } else {
                None
            }
        };
        notes.extend(way(
            Sign::KeptOutOfReach,
            "goes the long way round to stay out of reach",
        ));
        notes.extend(way(
            Sign::WentRoundTheYard,
            "goes round the yard while the other team is in it",
        ));
        notes.extend(way(Sign::TookTheEdge, "wanders out along the edges"));
        notes.extend(way(Sign::KeptToCover, "keeps close to walls and hedges"));
        notes.extend(way(
            Sign::WalkedRightPast,
            "walks straight past the other team",
        ));
        notes.extend(way(Sign::ChangedItsMind, "changes its mind halfway"));
        notes.extend(way(Sign::KeptApart, "keeps its distance from a teammate"));
        let moment_sign = |sign, at_least: u32, words: &str| {
            let n = count(sign);
            (n >= at_least).then(|| (50 + n, words.to_owned()))
        };
        notes.extend(moment_sign(
            Sign::TookAChance,
            2,
            "Takes a chance on anyone wandering past, even when sent for someone else",
        ));
        notes.extend(moment_sign(
            Sign::SteppedBack,
            2,
            "Hops back out of reach after a contest",
        ));
        notes.extend(moment_sign(
            Sign::HeldFirm,
            3,
            "Holds firm when someone comes for it",
        ));
        notes.extend(moment_sign(
            Sign::CaughtSomeone,
            3,
            "Good at catching others on the move",
        ));
        notes.extend(moment_sign(
            Sign::ShuffledToFriend,
            2,
            "With time to spare, shuffles over beside a friend",
        ));
        notes.extend(moment_sign(
            Sign::TuckedAway,
            2,
            "With time to spare, tucks itself out of reach",
        ));
        if let Some((id, n)) = all
            .friends
            .iter()
            .max_by_key(|(id, n)| (**n, std::cmp::Reverse(**id)))
            && *n >= 3
            && let Some(friend) = self.teammates.get(id)
        {
            notes.push((60 + u32::from(*n), format!("Likes to stay near {friend}")));
        }
        notes.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        notes.into_iter().map(|(_, text)| text).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Coach;
    use crate::arena::Arena;
    use crate::order::Side;
    use crate::ruleset::Ruleset;
    use crate::tournament::drawn_creature;

    fn played(seed: u64) -> (Match, Vec<Event>) {
        let rosters = [
            (0..3).map(|i| drawn_creature(seed, i)).collect(),
            (3..6).map(|i| drawn_creature(seed, i)).collect(),
        ];
        let mut m = Match::new(Arena::backyard(), Ruleset::default(), seed, rosters).unwrap();
        let mut events = Vec::new();
        while m.outcome().is_none() {
            let orders = [
                Coach::Exhibition.orders(&m, Side::Left),
                Coach::Exhibition.orders(&m, Side::Right),
            ];
            events.extend(m.resolve_turn(orders).events);
        }
        (m, events)
    }

    #[test]
    fn every_creature_on_a_side_gets_a_page_of_what_it_did() {
        let (m, events) = played(4);
        let seen = observe(&m, Side::Left, &events);
        assert_eq!(seen.len(), 3);
        let stepped: u16 = seen.iter().map(|s| s.showing.steps).sum();
        let told = events
            .iter()
            .filter(
                |e| matches!(e.kind, EventKind::Stepped { slot, .. } if slot.side == Side::Left),
            )
            .count();
        assert_eq!(usize::from(stepped), told);
    }

    #[test]
    fn a_habit_is_only_noted_once_it_shows_again_and_again() {
        let mut page = Page::default();
        let mut once = Showing {
            steps: 10,
            ..Showing::default()
        };
        once.signs.insert(Sign::TookTheEdge, 1);
        page.record(
            &Seen {
                creature_id: 1,
                showing: once.clone(),
                moments: Vec::new(),
            },
            false,
            "Pip",
            &[],
        );
        assert!(page.notes().is_empty());
        let mut habit = once;
        habit.signs.insert(Sign::TookTheEdge, 5);
        page.record(
            &Seen {
                creature_id: 1,
                showing: habit,
                moments: Vec::new(),
            },
            true,
            "Pip",
            &[],
        );
        let notes = page.notes();
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].ends_with("wanders out along the edges"));
    }

    #[test]
    fn notes_come_from_recent_matches_so_they_change_as_a_creature_does() {
        let mut page = Page::default();
        let mut edge = Showing {
            steps: 6,
            ..Showing::default()
        };
        edge.signs.insert(Sign::TookTheEdge, 4);
        let plain = Showing {
            steps: 6,
            ..Showing::default()
        };
        let seen = |showing: Showing| Seen {
            creature_id: 1,
            showing,
            moments: Vec::new(),
        };
        page.record(&seen(edge), false, "Pip", &[]);
        assert!(!page.notes().is_empty());
        for _ in 0..MATCHES_KEPT {
            page.record(&seen(plain.clone()), false, "Pip", &[]);
        }
        assert!(page.notes().is_empty());
        assert_eq!(page.matches, 1 + MATCHES_KEPT as u32);
    }

    #[test]
    fn every_sign_is_named_as_it_is_written() {
        for sign in Sign::ALL {
            assert_eq!(serde_json::to_value(sign).unwrap(), sign.id());
            assert_eq!(Sign::from_id(sign.id()), Some(sign));
        }
    }

    #[test]
    fn no_note_ever_carries_a_number() {
        for seed in 0..6 {
            let (m, events) = played(seed);
            for seen in observe(&m, Side::Left, &events) {
                let mut page = Page::default();
                for _ in 0..4 {
                    page.record(&seen, true, "Pip", &[(9, "Mallow".to_owned())]);
                }
                for note in page.notes().iter().chain(&page.moments) {
                    assert!(!note.chars().any(|c| c.is_ascii_digit()), "{note}");
                }
            }
        }
    }
}
