//! A turn told as it plays, over the board rather than in a list: for each moment worth seeing, a
//! short headline on a slip at the top of the board, a cartoon word where it happened ("POW!",
//! "CLANG!", "BOOM!") and the sound it makes. Worked out once when the turn is shown, from the same
//! timed events the playback moves everyone by, so the words land on the beat they belong to.
//!
//! The recap's full sentences (`formiga_battle_core::recap`) are still kept, in the menu that
//! tells the whole match; these are the quick version, for the eye.

use crate::playback::Playback;
use crate::sound::Cue;
use formiga_battle_core::recap::recap_naming;
use formiga_battle_core::{Because, EventKind, Match, Role, Side, Slot, Victory, Why};

/// One moment of a turn, as it is announced.
#[derive(Clone, Debug, PartialEq)]
pub struct Callout {
    /// Seconds into the turn's playback.
    pub at: f32,
    /// Where on the floor it happened, in tiles, for the word.
    pub place: Option<(f32, f32)>,
    /// Whoever it happened to, if anyone: its word and line go over its head.
    pub about: Option<Slot>,
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
            about: None,
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

/// How an arena's map event is told: the sprinkler, the wave, the pond, the oak.
struct Map {
    /// The event, as a line names it.
    event: &'static str,
    warning: &'static str,
    warn_word: &'static str,
    warn_cue: Cue,
    /// Its switch, and what turning it is said as.
    switch: &'static str,
    turned: &'static str,
    switch_word: &'static str,
    switch_cue: Cue,
    land_word: &'static str,
    land_cue: Cue,
    /// What it did to whoever it caught, and the line when it caught nobody.
    did: &'static str,
    missed: &'static str,
}

impl Map {
    fn of(m: &Match) -> Self {
        let name = m.arena.scenery.as_ref().map_or("", |s| s.name.as_str());
        match name {
            "wave" => Self {
                event: "wave",
                warning: "A wave is rolling in!",
                warn_word: "SWASH...",
                warn_cue: Cue::Wave,
                switch: "big shell",
                turned: "blew the big shell",
                switch_word: "TOOT!",
                switch_cue: Cue::Toot,
                land_word: "KER-SPLASH!",
                land_cue: Cue::Wave,
                did: "washed along",
                missed: "The wave washed over the sand!",
            },
            "pond" => Self {
                event: "pond",
                warning: "The water is rising!",
                warn_word: "GLUG...",
                warn_cue: Cue::Gurgle,
                switch: "sluice",
                turned: "opened the sluice",
                switch_word: "CLUNK!",
                switch_cue: Cue::Clunk,
                land_word: "SPLASH!",
                land_cue: Cue::Splash,
                did: "washed off",
                missed: "The pond rose over the banks by the jetty!",
            },
            "tree" => Self {
                event: "oak",
                warning: "An oak is about to shake!",
                warn_word: "RUSTLE...",
                warn_cue: Cue::Rustle,
                switch: "oak",
                turned: "shook the oak",
                switch_word: "SHAKE!",
                switch_cue: Cue::Rustle,
                land_word: "CLONK!",
                land_cue: Cue::Acorns,
                did: "dropped acorns on",
                missed: "Acorns rained down round the oak!",
            },
            _ => Self {
                event: "sprinkler",
                warning: "The sprinkler is about to come round!",
                warn_word: "DRIP...",
                warn_cue: Cue::Gurgle,
                switch: "tap",
                turned: "turned the tap",
                switch_word: "SQUEAK!",
                switch_cue: Cue::Gurgle,
                land_word: "SPLOOSH!",
                land_cue: Cue::Splash,
                did: "soaked",
                missed: "The sprinkler soaked the yard!",
            },
        }
    }
}

/// Composure lost, as it is shown.
const DAMAGE: [&str; 3] = ["-1", "-2", "-3"];

fn pick(words: &'static [&'static str], turn: u8, k: usize) -> &'static str {
    words[(usize::from(turn) + k) % words.len()]
}

/// Everything announced in a turn, in order. `teams` names each side for a headline, as the
/// Handler would say it: "you", "the visitors", a friend's name.
pub fn callouts(
    m: &Match,
    playback: &Playback,
    events: &[formiga_battle_core::Event],
    teams: [&str; 2],
    you: Side,
) -> Vec<Callout> {
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
    let middle = |tiles: &[formiga_battle_core::Tile]| -> Option<(f32, f32)> {
        let n = tiles.len() as f32;
        (n > 0.0).then(|| {
            let (x, y) = tiles.iter().fold((0.0, 0.0), |(x, y), t| {
                (x + f32::from(t.x) + 0.5, y + f32::from(t.y) + 0.5)
            });
            (x / n, y / n)
        })
    };
    let mut out = Vec::new();
    let mut contested_told = false;
    let mut appeared_told = false;
    let map = Map::of(m);
    // A moment as the full account tells it, for the line over the board.
    let told = |kind: &EventKind| -> Option<String> {
        let event = formiga_battle_core::Event {
            turn: 0,
            tick: 1,
            kind: kind.clone(),
        };
        recap_naming(m, &[event], teams).into_iter().next()
    };
    let mut progress_told = [false; 2];
    // How the team shared out the plan, as the orders are revealed: only where a companion's
    // own leanings chose its job, and only for the Handler's team.
    let mut leaning = 0;
    for event in events {
        let EventKind::Assigned {
            slot, why, task, ..
        } = &event.kind
        else {
            continue;
        };
        if slot.side != you || matches!(why, Why::Nearest | Why::KeptAtIt) {
            continue;
        }
        let when = crate::playback::REVEAL_SECS * (0.15 + 0.3 * leaning as f32);
        leaning += 1;
        let mut c = Callout::new(when);
        c.place = Some(playback.at(when).figure(*slot).at);
        c.about = Some(*slot);
        c.word = Some(match why {
            Why::Bold => "CHARGE!",
            Why::Careful => "STEADY!",
            Why::Friendly { .. } => "PALS!",
            Why::Avoiding { .. } => "HMPH!",
            Why::Curious => "OOH!",
            Why::Whim => "WHIM!",
            Why::Alone if task.role == Role::Pin => "WAIT!",
            Why::Alone | Why::Nearest | Why::KeptAtIt => "SOLO!",
        });
        c.headline = told(&event.kind);
        c.good_for = Some(slot.side);
        c.cue = Some(Cue::Ready);
        out.push(c);
    }
    for (k, (when, event)) in playback.moments().iter().enumerate() {
        let turn = event.turn;
        let mut c = Callout::new(*when);
        let at = |slot: Slot| playback.at(*when).figure(slot).at;
        match &event.kind {
            EventKind::Stepped { .. } => c.cue = Some(Cue::Step),
            EventKind::Bumped { slot, wanted } => {
                c.place = Some(centre(*wanted));
                c.about = Some(*slot);
                c.word = Some(pick(&BUMP, turn, k));
                c.cue = Some(Cue::Boink);
            }
            EventKind::HungBack { slot, at: tile } => {
                c.place = Some(centre(*tile));
                c.about = Some(*slot);
                c.word = Some("HMM?");
                c.cue = Some(Cue::Hmm);
            }
            EventKind::Interrupted { target, by } => {
                c.place = Some(at(*target));
                c.about = Some(*target);
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
                c.about = Some(*target);
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
                c.about = Some(*slot);
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
                c.about = Some(*target);
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
                c.about = Some(*slot);
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
                c.about = Some(*slot);
                c.word = Some(DAMAGE[usize::from(*lost).min(DAMAGE.len()) - 1]);
                c.style = Style::Damage;
            }
            EventKind::SceneryWarned { tiles, .. } => {
                c.place = Some(middle(tiles).unwrap_or(beacon));
                c.word = Some(map.warn_word);
                c.headline = Some(map.warning.to_owned());
                c.cue = Some(map.warn_cue);
            }
            EventKind::SwitchTurned { slot, switch } => {
                c.place = Some(centre(*switch));
                c.word = Some(map.switch_word);
                c.headline = Some(format!("{} {}!", name(slot), map.turned));
                c.good_for = Some(slot.side);
                c.cue = Some(map.switch_cue);
            }
            EventKind::SwitchDefended {
                switch, holders, ..
            } => {
                c.place = Some(centre(*switch));
                c.word = Some("SHOO!");
                c.headline = Some(format!("{} guarded the {}!", list(holders), map.switch));
                c.good_for = holders.first().map(|s| s.side);
                c.cue = Some(Cue::Shoo);
            }
            EventKind::SwitchResting { switch, .. } => {
                c.place = Some(centre(*switch));
                c.headline = Some(format!("The {} is still resting.", map.switch));
                c.cue = Some(Cue::Hmm);
            }
            EventKind::SceneryLanded {
                tiles,
                hit,
                sheltered,
                moved,
                ..
            } => {
                c.place = Some(middle(tiles).unwrap_or(beacon));
                c.word = Some(map.land_word);
                c.style = Style::Big;
                let moved: Vec<Slot> = moved.iter().map(|b| b.slot).collect();
                let caught: Vec<Slot> = if moved.is_empty() { hit.clone() } else { moved };
                let what = map.event;
                c.headline = Some(match (caught.is_empty(), sheltered.is_empty()) {
                    (false, true) => format!("The {what} {} {}!", map.did, list(&caught)),
                    (false, false) => format!(
                        "The {what} {} {}; {} stayed dry under the leaf!",
                        map.did,
                        list(&caught),
                        list(sheltered)
                    ),
                    (true, false) => format!("{} stayed dry under the leaf!", list(sheltered)),
                    (true, true) => map.missed.to_owned(),
                });
                c.cue = Some(map.land_cue);
            }
            EventKind::GroundReopened { tiles } => {
                c.place = middle(tiles);
                c.word = Some("GLUG");
                c.headline = Some("The water went down again.".to_owned());
                c.cue = Some(Cue::Plop);
            }
            EventKind::RaftFloated { by, at: tile, .. } => {
                c.place = Some(centre(*tile));
                c.word = Some("BOB!");
                c.headline = Some(format!("{}'s twig floats: a raft!", name(by)));
                c.good_for = Some(by.side);
                c.cue = Some(Cue::Plop);
            }
            EventKind::RaftSank { at: tile } => {
                c.place = Some(centre(*tile));
                c.word = Some("GLUB");
                c.cue = Some(Cue::Plop);
            }
            EventKind::Rung { by, hushed, .. } => {
                c.place = Some(at(*by));
                c.about = Some(*by);
                c.word = Some("DING!");
                c.headline = Some(if hushed.is_empty() {
                    format!("{} rang the pebble bell!", name(by))
                } else {
                    format!(
                        "{} rang the bell; {} lost their nerve!",
                        name(by),
                        list(hushed)
                    )
                });
                c.good_for = Some(by.side);
                c.cue = Some(Cue::Ding);
            }
            EventKind::HoneySpilled { by, tiles, .. } => {
                c.place = middle(tiles);
                c.word = Some("SPLAT!");
                c.headline = Some(format!("{} spilled honey!", name(by)));
                c.good_for = Some(by.side);
                c.cue = Some(Cue::Squelch);
            }
            EventKind::Stuck { slot, .. } => {
                c.place = Some(at(*slot));
                c.about = Some(*slot);
                c.word = Some("STUCK!");
                c.headline = Some(format!("{} is stuck in the honey!", name(slot)));
                c.good_for = Some(slot.side.other());
                c.cue = Some(Cue::Squelch);
            }
            EventKind::Hopped { slot, to, .. } => {
                c.place = Some(centre(*to));
                c.about = Some(*slot);
                c.word = Some("BOING!");
                c.headline = Some(format!("{} hopped on a springy seed!", name(slot)));
                c.good_for = Some(slot.side);
                c.cue = Some(Cue::Boink);
            }
            EventKind::ItemsGlint { spots, item, .. } => {
                c.place = spots.first().map(|t| centre(*t));
                c.word = Some("TING!");
                c.headline = Some(format!("Something glints: a {} next turn!", item.name()));
                c.cue = Some(Cue::Glint);
            }
            EventKind::ItemAppeared { at: tile, item } => {
                c.place = Some(centre(*tile));
                c.word = Some("POP!");
                if !appeared_told {
                    appeared_told = true;
                    c.headline = Some(format!("A {} turned up!", item.name()));
                }
                c.cue = Some(Cue::Glint);
            }
            EventKind::PickedUp {
                slot,
                item,
                at: tile,
            } => {
                c.place = Some(centre(*tile));
                c.about = Some(*slot);
                c.word = Some("GOT IT!");
                c.headline = Some(format!("{} picked up the {}!", name(slot), item.name()));
                c.good_for = Some(slot.side);
                c.cue = Some(Cue::Pickup);
            }
            EventKind::Dropped { slot, item, .. } => {
                c.place = Some(at(*slot));
                c.about = Some(*slot);
                c.word = Some("PLOP");
                c.headline = Some(format!("{} dropped the {}!", name(slot), item.name()));
                c.good_for = Some(slot.side.other());
                c.cue = Some(Cue::Plop);
            }
            EventKind::ItemUsed { slot, item, .. } => {
                c.place = Some(at(*slot));
                c.about = Some(*slot);
                c.headline = Some(format!("{} used the {}!", name(slot), item.name()));
                c.good_for = Some(slot.side);
            }
            EventKind::Puffed {
                by, blown, braced, ..
            } => {
                let moved: Vec<Slot> = blown.iter().map(|b| b.slot).collect();
                c.place = blown
                    .first()
                    .map(|b| centre(b.from))
                    .or_else(|| Some(at(*by)));
                c.word = Some("WHOOSH!");
                c.headline = Some(match (moved.is_empty(), braced.is_empty()) {
                    (false, _) => format!("{}'s puff blew {} away!", name(by), list(&moved)),
                    (true, false) => format!("{} braced together against the puff!", list(braced)),
                    (true, true) => format!("{}'s puff blew across the yard!", name(by)),
                });
                c.good_for = Some(by.side);
                c.cue = Some(Cue::Poof);
            }
            EventKind::TwigWedged { by, at: tile, .. } => {
                c.place = Some(centre(*tile));
                c.word = Some("WEDGE!");
                c.headline = Some(format!("{} wedged a twig in the way!", name(by)));
                c.good_for = Some(by.side);
                c.cue = Some(Cue::Snap);
            }
            EventKind::TwigFell { at: tile } => {
                c.place = Some(centre(*tile));
                c.word = Some("PLOP");
                c.cue = Some(Cue::Plop);
            }
            EventKind::GateWedged { gate } => {
                c.place = Some(centre(*gate));
                c.word = Some("STUCK!");
                c.headline = Some("A twig keeps the gate from closing!".to_owned());
                c.cue = Some(Cue::Clunk);
            }
            EventKind::Sheltering { by, slots, .. } => {
                c.place = Some(at(*by));
                c.about = Some(*by);
                c.word = Some("FWIP!");
                c.headline = Some(if slots.iter().any(|s| s != by) {
                    format!("{} opened a leaf over {}!", name(by), list(slots))
                } else {
                    format!("{} opened a leaf umbrella!", name(by))
                });
                c.good_for = Some(by.side);
                c.cue = Some(Cue::Fwip);
            }
            EventKind::Sheltered { target, by } => {
                c.place = Some(at(*target));
                c.about = Some(*target);
                c.word = Some("BOING");
                c.headline = Some(format!("{} bounced off {}'s leaf!", list(by), name(target)));
                c.good_for = Some(target.side);
                c.cue = Some(Cue::Boink);
            }
            EventKind::ItemFizzled { slot, item, .. } => {
                c.place = Some(at(*slot));
                c.about = Some(*slot);
                c.word = Some("FIZZLE");
                c.headline = Some(format!("{}'s {} came to nothing.", name(slot), item.name()));
                c.cue = Some(Cue::Fizzle);
            }
            // Partway through, a creature whose job came to nothing takes another.
            EventKind::Rethought { slot, because, .. } => {
                c.place = Some(at(*slot));
                c.about = Some(*slot);
                c.word = Some(match because {
                    Because::ItemGone => "AW!",
                    Because::GotIt => "GOT IT!",
                    Because::TargetDown => "NEXT!",
                    Because::DugIn { .. } => "UH-OH",
                    Because::SquareCleared => "GO GO!",
                    Because::SquareTaken { .. } => "TOO LATE!",
                    Because::Warned => "EEK!",
                    Because::AllyDown { .. } => "ON IT!",
                    Because::Spotted => "OOH!",
                });
                c.headline = told(&event.kind);
                c.good_for = Some(slot.side);
                c.cue = Some(Cue::Hmm);
            }
            // Orders filled and the plan shared out come before the first tick, and are told in
            // the full account.
            EventKind::OrderFilled { .. }
            | EventKind::Assigned { .. }
            | EventKind::OrderDropped { .. }
            | EventKind::Holding { .. }
            | EventKind::Composure { .. }
            | EventKind::ShelterFolded { .. }
            | EventKind::CaptureContested => {}
            EventKind::HoneyDried { .. } => {}
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
            let plans = [
                Coach::Exhibition.plan(&m, Side::Left),
                Coach::Exhibition.plan(&m, Side::Right),
            ];
            let report = m.resolve_turn(plans);
            let playback = Playback::new(
                &before,
                &m.arena.gates,
                &facing,
                &report.events,
                m.ruleset.ticks_per_turn,
            );
            all.extend(callouts(
                &m,
                &playback,
                &report.events,
                ["you", "the visitors"],
                Side::Left,
            ));
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
