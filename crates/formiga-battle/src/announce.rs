//! A turn told as it plays, over the board rather than in a list: for each moment worth seeing, a
//! short headline on a slip at the top of the board, a cartoon word where it happened ("POW!",
//! "CLANG!", "BOOM!") and the sound it makes. Worked out once when the turn is shown, from the same
//! timed events the playback moves everyone by, so the words land on the beat they belong to.
//!
//! The recap's full sentences (`formiga_battle_core::recap`) are still kept, in the menu that
//! tells the whole match; these are the quick version, for the eye.

use crate::playback::Playback;
use crate::sound::Cue;
use formiga_battle_core::{EventKind, Match, Side, Slot, Victory};

/// One moment of a turn, as it is announced.
#[derive(Clone, Debug, PartialEq)]
pub struct Callout {
    /// Seconds into the turn's playback.
    pub at: f32,
    /// Where on the floor it happened, in tiles, for the word.
    pub place: Option<(f32, f32)>,
    /// The cartoon word, if it gets one.
    pub word: Option<&'static str>,
    /// The headline, if it gets one.
    pub headline: Option<String>,
    /// Whose good news it is, for the headline's colour.
    pub good_for: Option<Side>,
    pub cue: Option<Cue>,
    pub style: Style,
}

/// How a callout's word is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// A cartoon word over whoever it happened to.
    Word,
    /// A big moment: a knockout, the beacon lit. Its word is drawn larger, and stays longer.
    Big,
    /// Composure lost, as a number: "-1" by whoever lost it, rising and fading.
    Damage,
}

impl Callout {
    fn new(at: f32) -> Self {
        Self {
            at,
            place: None,
            word: None,
            headline: None,
            good_for: None,
            cue: None,
            style: Style::Word,
        }
    }
}

/// Words for each kind of moment; which one is used goes round with the turn, so a match is not
/// all "POW!".
const HIT: [&str; 4] = ["POW!", "BONK!", "WHAP!", "BIFF!"];
const TURNED: [&str; 3] = ["CLANG!", "TONK!", "DONG!"];
const SHOVE: [&str; 2] = ["WHAM!", "SHOVE!"];
const OUT: [&str; 3] = ["BOOM!", "KER-FLOP!", "BWOMP!"];
const WHIFF: [&str; 2] = ["MISS!", "WHIFF!"];
const BUMP: [&str; 2] = ["BOINK", "BOING"];

/// Composure lost, as it is shown.
const DAMAGE: [&str; 3] = ["-1", "-2", "-3"];

fn pick(words: &'static [&'static str], turn: u8, k: usize) -> &'static str {
    words[(usize::from(turn) + k) % words.len()]
}

/// Everything announced in a turn, in order. `teams` names each side for a headline, as the
/// Handler would say it: "you", "the visitors", a friend's name.
pub fn callouts(m: &Match, playback: &Playback, teams: [&str; 2]) -> Vec<Callout> {
    let name = |s: &Slot| m.profile(*s).name.clone();
    let list = |slots: &[Slot]| -> String {
        let names: Vec<String> = slots.iter().map(name).collect();
        match names.as_slice() {
            [] => String::new(),
            [one] => one.clone(),
            [init @ .., last] => format!("{} and {last}", init.join(", ")),
        }
    };
    let beacon = {
        let zone = &m.arena.zone;
        let n = zone.len().max(1) as f32;
        let (x, y) = zone.iter().fold((0.0, 0.0), |(x, y), t| {
            (x + f32::from(t.x) + 0.5, y + f32::from(t.y) + 0.5)
        });
        (x / n, y / n)
    };
    let centre = |t: formiga_battle_core::Tile| (f32::from(t.x) + 0.5, f32::from(t.y) + 0.5);
    let mut out = Vec::new();
    let mut contested_told = false;
    let mut progress_told = [false; 2];
    for (k, (when, event)) in playback.moments().iter().enumerate() {
        let turn = event.turn;
        let mut c = Callout::new(*when);
        let at = |slot: Slot| playback.at(*when).figure(slot).at;
        match &event.kind {
            EventKind::Stepped { .. } => c.cue = Some(Cue::Step),
            EventKind::Bumped { wanted, .. } => {
                c.place = Some(centre(*wanted));
                c.word = Some(pick(&BUMP, turn, k));
                c.cue = Some(Cue::Boink);
            }
            EventKind::HungBack { at: tile, .. } => {
                c.place = Some(centre(*tile));
                c.word = Some("HMM?");
                c.cue = Some(Cue::Hmm);
            }
            EventKind::Interrupted { target, by } => {
                c.place = Some(at(*target));
                c.word = Some(pick(&HIT, turn, k));
                c.headline = Some(format!("{} caught {} on the move!", list(by), name(target)));
                c.good_for = by.first().map(|s| s.side);
                c.cue = Some(Cue::Pow);
            }
            EventKind::Countered {
                target,
                holders,
                contesters,
            } => {
                c.place = Some(at(*target));
                c.word = Some(pick(&TURNED, turn, k));
                c.headline = Some(if holders.len() > 1 {
                    format!("{} held firm together!", list(holders))
                } else {
                    format!("{} held firm against {}!", name(target), list(contesters))
                });
                c.good_for = Some(target.side);
                c.cue = Some(Cue::Clang);
            }
            EventKind::Flanked {
                target,
                by,
                from,
                pushed_to,
            } => {
                c.place = Some(centre(*from));
                c.word = Some(pick(&SHOVE, turn, k));
                c.headline = Some(match pushed_to {
                    Some(_) => format!("{} pushed {} off its spot!", list(by), name(target)),
                    None => format!("{} boxed {} in!", list(by), name(target)),
                });
                c.good_for = by.first().map(|s| s.side);
                c.cue = Some(Cue::Wham);
            }
            EventKind::KnockedOut { slot } => {
                c.place = Some(at(*slot));
                c.word = Some(pick(&OUT, turn, k));
                c.headline = Some(format!("{} is dizzy and sits out!", name(slot)));
                c.good_for = Some(slot.side.other());
                c.cue = Some(Cue::Boom);
                c.style = Style::Big;
            }
            EventKind::Scuffle {
                contester, target, ..
            } => {
                c.place = Some(at(*target));
                c.word = Some(pick(&WHIFF, turn, k));
                c.headline = Some(format!(
                    "{} missed: {} was busy with a contest of its own!",
                    name(contester),
                    name(target)
                ));
                c.cue = Some(Cue::Miss);
            }
            EventKind::TookOpportunity { slot, took, .. } => {
                c.place = Some(at(*slot));
                c.headline = Some(format!("{} went for {} instead!", name(slot), name(took)));
                c.good_for = Some(slot.side);
            }
            EventKind::LeverPulled { slot, lever, .. } => {
                c.place = Some(centre(*lever));
                c.word = Some("CLUNK!");
                c.headline = Some(format!("{} yanked the lever!", name(slot)));
                c.good_for = Some(slot.side);
                c.cue = Some(Cue::Clunk);
            }
            EventKind::LeverDefended { holders, lever, .. } => {
                c.place = Some(centre(*lever));
                c.word = Some("SHOO!");
                c.headline = Some(format!("{} guarded the lever!", list(holders)));
                c.good_for = holders.first().map(|s| s.side);
                c.cue = Some(Cue::Shoo);
            }
            EventKind::LeverResting { lever, .. } => {
                c.place = Some(centre(*lever));
                c.headline = Some("The lever is still resting.".to_owned());
                c.cue = Some(Cue::Hmm);
            }
            EventKind::GateSwung { gate, open } => {
                c.place = Some(centre(*gate));
                c.word = Some("CREEEAK");
                c.headline = Some(if *open {
                    "The gate swung open!".to_owned()
                } else {
                    "The gate swung shut!".to_owned()
                });
                c.cue = Some(Cue::Creak);
            }
            EventKind::GateJammed { gate, by } => {
                c.place = Some(centre(*gate));
                c.word = Some("THUNK");
                c.headline = Some(format!("{} is in the way: the gate is jammed!", name(by)));
                c.cue = Some(Cue::Clunk);
            }
            EventKind::CaptureContested if !contested_told => {
                contested_told = true;
                c.headline = Some("The beacon is held by both!".to_owned());
                c.cue = Some(Cue::Contested);
            }
            EventKind::CaptureProgress { side, progress } => {
                c.cue = Some(Cue::Glow);
                if !progress_told[side.index()] {
                    progress_told[side.index()] = true;
                    c.headline = Some(format!(
                        "Beacon {progress} of {} for {}",
                        m.ruleset.capture_ticks,
                        teams[side.index()]
                    ));
                    c.good_for = Some(*side);
                }
            }
            EventKind::Finished { outcome } => match (outcome.winner, outcome.victory) {
                (Some(side), Some(Victory::Capture)) => {
                    c.place = Some(beacon);
                    c.word = Some("FWOOSH!");
                    c.headline = Some(format!("Beacon lit for {}!", teams[side.index()]));
                    c.good_for = Some(side);
                    c.cue = Some(Cue::Fwoosh);
                    c.style = Style::Big;
                }
                (Some(side), Some(Victory::Knockout)) => {
                    c.headline = Some(format!(
                        "Everyone else is dizzy: a win for {}!",
                        teams[side.index()]
                    ));
                    c.good_for = Some(side);
                }
                _ => {
                    c.headline = Some("Time's up!".to_owned());
                    c.cue = Some(Cue::Whistle);
                }
            },
            EventKind::Composure { slot, lost, .. } if *lost > 0 => {
                c.place = Some(at(*slot));
                c.word = Some(DAMAGE[usize::from(*lost).min(DAMAGE.len()) - 1]);
                c.style = Style::Damage;
            }
            // Orders filled come before the first tick, and are told in the full account.
            EventKind::OrderFilled { .. }
            | EventKind::Holding { .. }
            | EventKind::Composure { .. }
            | EventKind::CaptureContested => {}
            // The map event and found things: their words and sounds are the match screen's to
            // add.
            EventKind::SceneryWarned { .. }
            | EventKind::SwitchTurned { .. }
            | EventKind::SwitchDefended { .. }
            | EventKind::SwitchResting { .. }
            | EventKind::SceneryLanded { .. }
            | EventKind::ItemsGlint { .. }
            | EventKind::ItemAppeared { .. }
            | EventKind::PickedUp { .. }
            | EventKind::Dropped { .. }
            | EventKind::ItemUsed { .. }
            | EventKind::Puffed { .. }
            | EventKind::TwigWedged { .. }
            | EventKind::TwigFell { .. }
            | EventKind::GateWedged { .. }
            | EventKind::Sheltering { .. }
            | EventKind::Sheltered { .. }
            | EventKind::ShelterFolded { .. }
            | EventKind::ItemFizzled { .. } => {}
        }
        // A headline is shown over whoever it is about; one about the beacon, over the beacon.
        if c.headline.is_some() && c.place.is_none() {
            c.place = Some(beacon);
        }
        if c.cue.is_some() || c.word.is_some() || c.headline.is_some() {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_battle_core::ai::Coach;
    use formiga_battle_core::tournament::drawn_creature;
    use formiga_battle_core::{Arena, Ruleset, TEAM_SIZE};

    fn played(seed: u64, turns: usize) -> Vec<Callout> {
        let rosters = [
            (0..3).map(|i| drawn_creature(seed, i)).collect(),
            (3..6).map(|i| drawn_creature(seed, i)).collect(),
        ];
        let mut m = Match::new(Arena::backyard(), Ruleset::default(), seed, rosters).unwrap();
        let facing = [[true; TEAM_SIZE], [false; TEAM_SIZE]];
        let mut all = Vec::new();
        for _ in 0..turns {
            if m.outcome().is_some() {
                break;
            }
            let before = m.board.clone();
            let orders = [
                Coach::Exhibition.orders(&m, Side::Left),
                Coach::Exhibition.orders(&m, Side::Right),
            ];
            let report = m.resolve_turn(orders);
            let playback = Playback::new(
                &before,
                &m.arena.gates,
                &facing,
                &report.events,
                m.ruleset.ticks_per_turn,
            );
            all.extend(callouts(&m, &playback, ["you", "the visitors"]));
        }
        all
    }

    #[test]
    fn a_match_is_announced_with_headlines_words_and_sounds() {
        let all: Vec<Callout> = (1..6).flat_map(|seed| played(seed, 12)).collect();
        assert!(all.iter().any(|c| c.headline.is_some()));
        assert!(all.iter().any(|c| c.word.is_some()));
        assert!(all.iter().any(|c| c.cue == Some(Cue::Step)));
        // Every word and every headline has somewhere on the board to appear.
        assert!(
            all.iter()
                .filter(|c| c.word.is_some() || c.headline.is_some())
                .all(|c| c.place.is_some())
        );
    }

    #[test]
    fn composure_lost_is_shown_as_a_number_by_whoever_lost_it() {
        let all: Vec<Callout> = (1..8).flat_map(|seed| played(seed, 12)).collect();
        let damage: Vec<&Callout> = all.iter().filter(|c| c.style == Style::Damage).collect();
        assert!(
            !damage.is_empty(),
            "somebody loses composure in seven matches"
        );
        assert!(
            damage
                .iter()
                .all(|c| c.place.is_some() && c.word.is_some_and(|w| w.starts_with('-')))
        );
    }

    #[test]
    fn a_turns_callouts_come_in_the_order_they_happen() {
        let all = played(3, 1);
        assert!(all.windows(2).all(|w| w[0].at <= w[1].at));
    }

    #[test]
    fn the_cartoon_words_go_round_rather_than_repeat() {
        let mut hits: Vec<&str> = (0..4).map(|k| pick(&HIT, 1, k)).collect();
        hits.sort_unstable();
        hits.dedup();
        assert_eq!(hits.len(), HIT.len());
    }
}
