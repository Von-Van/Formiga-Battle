//! A turn told in sentences, from its events: what each creature did and why, so nothing that
//! happened rests on dice nobody can see.

use crate::event::{Event, EventKind, Fill, Reason, Victory};
use crate::game::Match;
use crate::grid::Tile;
use crate::muster::{Because, Role, Task, Why};
use crate::order::{Directive, Order, Side, Slot};
use std::collections::BTreeSet;

pub fn team_name(side: Side) -> &'static str {
    match side {
        Side::Left => "The left team",
        Side::Right => "The right team",
    }
}

fn list(m: &Match, slots: &[Slot]) -> String {
    let names: Vec<&str> = slots.iter().map(|s| m.profile(*s).name.as_str()).collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// What it was told, in a few words.
pub fn describe(m: &Match, order: &Order) -> String {
    match order {
        Order::Move { to } => format!("MOVE to {to}"),
        Order::Hold { at, facing } => format!("HOLD at {at} facing {}", facing.name()),
        Order::Press { target } => format!("PRESS {}", m.profile(*target).name),
        Order::Trick { target } => format!("TRICK {}", m.profile(*target).name),
        Order::Interact { at } if m.arena.switches.contains(at) => {
            format!("INTERACT with {}", words(m).switch)
        }
        Order::Interact { .. } => "INTERACT with the lever".to_owned(),
    }
}

/// A team order, in a few words.
pub fn describe_directive(m: &Match, d: &Directive) -> String {
    let squares = |ts: &[crate::grid::Tile]| {
        ts.iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    match d {
        Directive::Press { squares: ts } => format!("PRESS {}", squares(ts)),
        Directive::Hold {
            squares: ts,
            facing,
        } => {
            format!("HOLD {} facing {}", squares(ts), facing.name())
        }
        Directive::Trick { squares: ts } => format!("TRICK {}", squares(ts)),
        Directive::Interact { at } if m.arena.switches.contains(at) => {
            format!("INTERACT with {}", words(m).switch)
        }
        Directive::Interact { at } if m.arena.levers.contains(at) => {
            "INTERACT with the lever".to_owned()
        }
        Directive::Interact { at } => format!("INTERACT at {at}"),
        Directive::UseItem { at, .. } => format!("USE ITEM at {at}"),
    }
}

/// A creature's job, as something to do: "hold (4, 3)", "flank Pip", "fetch the twig".
pub fn job(m: &Match, slot: Slot, task: &Task, order: &Order) -> String {
    let _ = slot;
    let target = order.target().map(|t| m.profile(t).name.clone());
    let thing = |at| {
        m.board
            .ground
            .iter()
            .chain(m.board.glint.iter())
            .find(|(g, _)| *g == at)
            .map(|(_, i)| i.name())
    };
    let tap = m.arena.switches.contains(&task.at);
    let at = task.at;
    match task.role {
        Role::Claim => format!("hold {at}"),
        Role::Oust => match target {
            Some(t) => format!("push {t} off {at}"),
            None => format!("clear {at}"),
        },
        Role::Trick => match target.or_else(|| m.occupant(at).map(|o| m.profile(o).name.clone())) {
            Some(t) => format!("trick {t}"),
            None => format!("trick whoever holds {at}"),
        },
        Role::Flank => match target.or_else(|| m.occupant(at).map(|o| m.profile(o).name.clone())) {
            Some(t) => format!("flank {t}"),
            None => format!("flank whoever holds {at}"),
        },
        Role::Pin => format!("stand beside whoever holds {at}"),
        Role::Guard if tap => format!("guard {}", words(m).switch),
        Role::Guard if m.arena.levers.contains(&at) => "guard the lever".to_owned(),
        Role::Guard => format!("guard {at}"),
        Role::Intercept => match target {
            Some(t) => format!("head off {t}"),
            None => format!("head off whoever comes at {at}"),
        },
        Role::Fetch => match thing(at) {
            Some(item) => format!("fetch the {item}"),
            None => format!("fetch what lay at {at}"),
        },
        Role::Pull if tap => format!("set off {}", words(m).switch),
        Role::Pull => "pull the lever".to_owned(),
        Role::Wait if tap => format!("wait by {}", words(m).switch),
        Role::Wait => "wait by the lever".to_owned(),
        Role::Spare => "do as it saw fit".to_owned(),
    }
}

/// How the recap tells an arena's map event: what its switch is, what setting it off is, what the
/// event does when it lands and what it does to whoever it catches.
struct Words {
    name: &'static str,
    switch: &'static str,
    turned: &'static str,
    lands: &'static str,
    hits: &'static str,
}

fn words(m: &Match) -> Words {
    let name = m.arena.scenery.as_ref().map_or("", |s| s.name.as_str());
    match name {
        "wave" => Words {
            name: "wave",
            switch: "the shell",
            turned: "blew",
            lands: "rolled over the ends of the sandbar",
            hits: "washed",
        },
        "pond" => Words {
            name: "pond",
            switch: "the sluice",
            turned: "opened",
            lands: "rose over the banks by the jetty",
            hits: "washed off",
        },
        "tree" => Words {
            name: "tree",
            switch: "a tree",
            turned: "shook",
            lands: "shed its acorns",
            hits: "bonked",
        },
        _ => Words {
            name: "sprinkler",
            switch: "the tap",
            turned: "turned",
            lands: "swept the yard",
            hits: "soaked",
        },
    }
}

/// The turn's events, told. Each creature's route is told once a turn, by the first choice it
/// made that was not the plain way.
pub fn recap(m: &Match, events: &[Event]) -> Vec<String> {
    recap_naming(m, events, [team_name(Side::Left), team_name(Side::Right)])
}

/// The same, with each team called what the teller calls it: "Your team", "The visiting team".
pub fn recap_naming(m: &Match, events: &[Event], teams: [&str; 2]) -> Vec<String> {
    let team_name = |side: Side| teams[side.index()];
    let name = |s: &Slot| m.profile(*s).name.clone();
    let mut lines = Vec::new();
    let mut routed: BTreeSet<Slot> = BTreeSet::new();
    let mut contested_told = false;
    let mut progress_told: BTreeSet<Side> = BTreeSet::new();
    for event in events {
        let line = match &event.kind {
            EventKind::Assigned {
                slot,
                task,
                order,
                why,
            } => {
                let n = name(slot);
                let j = job(m, *slot, task, order);
                let held = || {
                    m.occupant(task.at)
                        .map_or_else(|| "the holder".to_owned(), |o| name(&o))
                };
                match why {
                    Why::Nearest | Why::KeptAtIt => None,
                    Why::Bold => Some(format!("{n} went to {j}, keen to meet the other team.")),
                    Why::Careful => Some(format!("{n} chose to {j}, happier standing its ground.")),
                    Why::Friendly { ally } => {
                        Some(format!("{n} chose to {j}, to be beside {}.", name(ally)))
                    }
                    Why::Avoiding { ally } => Some(format!(
                        "{n} chose to {j}, to keep away from {}.",
                        name(ally)
                    )),
                    Why::Curious => Some(format!("{n} couldn't resist: it went to {j}.")),
                    Why::Whim => Some(format!("{n} took it into its head to {j}.")),
                    Why::Alone if task.role == Role::Pin => Some(format!(
                        "{n} was the only one free to flank {}, so it waited beside it for help.",
                        held()
                    )),
                    Why::Alone => Some(format!(
                        "{n} was the only one free to flank {} and went at it alone anyway.",
                        held()
                    )),
                }
            }
            EventKind::Rethought {
                slot,
                task,
                order,
                because,
            } => {
                let n = name(slot);
                let j = job(m, *slot, task, order);
                Some(match because {
                    Because::ItemGone => {
                        format!("{n} found what it went for gone, so it went to {j} instead.")
                    }
                    Because::GotIt => format!("{n} got what it went for and went on to {j}."),
                    Because::TargetDown => {
                        format!("{n}'s target was out of it, so it went to {j} instead.")
                    }
                    Because::DugIn { target } => format!(
                        "{} dug in before {n} got there, so {n} went to {j} instead.",
                        name(target)
                    ),
                    Because::SquareCleared => format!("The square came free, so {n} went to {j}."),
                    Because::SquareTaken { by } => {
                        format!("{} got there first, so {n} went to {j} instead.", name(by))
                    }
                    Because::Warned => format!("{n} saw what was coming and went to {j} instead."),
                    Because::AllyDown { ally } => {
                        format!("With {} out, {n} stepped in to {j}.", name(ally))
                    }
                    Because::Spotted => {
                        format!("{n} spotted something turning up and went to {j}.")
                    }
                })
            }
            EventKind::OrderDropped { side, .. } => Some(format!(
                "One of {}'s orders couldn't be carried out and was left out.",
                team_name(*side).to_lowercase()
            )),
            EventKind::OrderFilled { slot, order, fill } => Some(format!(
                "{} {}, so it chose for itself: {}.",
                name(slot),
                match fill {
                    Fill::Missing => "had no order in time",
                    Fill::Invalid => "was given an order it could not carry out",
                },
                describe(m, order)
            )),
            EventKind::Stepped { slot, reason, .. } => {
                if *reason == Reason::Direct || !routed.insert(*slot) {
                    None
                } else {
                    let n = name(slot);
                    Some(match reason {
                        Reason::Direct => unreachable!("skipped above"),
                        Reason::AvoidedExposure => {
                            format!("{n} went the long way round to stay out of reach.")
                        }
                        Reason::AvoidedContestedCentre => {
                            format!("{n} took the side route to avoid the contested centre.")
                        }
                        Reason::KeptNearAlly { ally } => {
                            format!("{n} kept close to {}.", name(ally))
                        }
                        Reason::KeptApartFrom { ally } => {
                            format!("{n} kept its distance from {}.", name(ally))
                        }
                        Reason::SideRoute => format!("{n} wandered out along the edge."),
                        Reason::TookCover => format!("{n} hugged the wall for cover."),
                        Reason::OnImpulse => {
                            format!("{n} changed its mind halfway and went another way.")
                        }
                        Reason::Bold => format!("{n} marched straight past the other team."),
                        Reason::FollowedUpToAlly { ally } => format!(
                            "With a moment to spare, {n} shuffled over beside {}.",
                            name(ally)
                        ),
                        Reason::FollowedUpToCover => {
                            format!("With a moment to spare, {n} tucked itself out of reach.")
                        }
                        Reason::SteppedBack => {
                            format!("{n} hopped back after its press was turned away.")
                        }
                        Reason::HungBack => {
                            format!("{n} hung back rather than walk in among the other team.")
                        }
                        Reason::SkirtedWarning => {
                            format!("{n} kept clear of the ground about to get it.")
                        }
                        Reason::WentForItem => {
                            format!("{n} couldn't resist something lying on the ground.")
                        }
                    })
                }
            }
            EventKind::HungBack { slot, .. } => routed.insert(*slot).then(|| {
                format!(
                    "{} hung back rather than walk in among the other team.",
                    name(slot)
                )
            }),
            EventKind::Pressed {
                target,
                by,
                pushed_to,
                tricking,
            } => Some(match (tricking, pushed_to) {
                (true, Some(_)) => format!(
                    "{} saw through {}'s trick and pushed it back.",
                    list(m, by),
                    name(target)
                ),
                (true, None) => format!(
                    "{} saw through {}'s trick; with nowhere to go, it lost its footing.",
                    list(m, by),
                    name(target)
                ),
                (false, Some(_)) => format!(
                    "{} pressed {} on the move and pushed it back.",
                    list(m, by),
                    name(target)
                ),
                (false, None) => format!(
                    "{} pressed {}; with nowhere to go, it lost its footing.",
                    list(m, by),
                    name(target)
                ),
            }),
            EventKind::Stole { by, from, item } => Some(format!(
                "{} snatched {}'s {}.",
                name(by),
                name(from),
                item.name()
            )),
            EventKind::Collided { target, by } => Some(format!(
                "{} and {} pressed into each other: both lost composure and stopped.",
                list(m, by),
                name(target)
            )),
            EventKind::Repelled {
                target,
                holders,
                pressers,
                faced,
            } => Some({
                let who = if holders.len() > 1 {
                    format!("{} held firm side by side", list(m, holders))
                } else {
                    format!("{} held firm", name(target))
                };
                if faced.is_empty() {
                    format!("{who} and turned {} away.", list(m, pressers))
                } else {
                    format!(
                        "{who} and turned {} away; {} came at it head on and lost composure.",
                        list(m, pressers),
                        list(m, faced)
                    )
                }
            }),
            EventKind::Flanked {
                target,
                by,
                pushed_to,
                ..
            } => Some(match pushed_to {
                Some(_) => format!(
                    "{} pressed {} from two sides and pushed it off its spot.",
                    list(m, by),
                    name(target)
                ),
                None => format!(
                    "{} pressed {} from two sides; boxed in, it lost its footing.",
                    list(m, by),
                    name(target)
                ),
            }),
            EventKind::Tricked { target, by, .. } => Some(format!(
                "{} slipped round {} where it wasn't looking and broke its hold; it lost composure.",
                list(m, by),
                name(target)
            )),
            EventKind::TrickRead { target, by } => Some(format!(
                "{} tried to trick {} from the front; it saw it coming and held.",
                name(by),
                name(target)
            )),
            EventKind::TookOpportunity { slot, meant, took } => Some(format!(
                "{} spotted {} wandering past and pressed them instead of {}.",
                name(slot),
                name(took),
                name(meant)
            )),
            EventKind::LeverPulled { slot, .. } => Some(format!(
                "{} yanked the lever; the gate is about to swing.",
                name(slot)
            )),
            EventKind::LeverDefended { slot, holders, .. } => Some(format!(
                "{} guarded the lever and shooed {} off.",
                list(m, holders),
                name(slot)
            )),
            EventKind::LeverResting { slot, .. } => Some(format!(
                "{} tugged the lever, but it was still resting.",
                name(slot)
            )),
            EventKind::GateSwung { open, .. } => Some(
                if *open {
                    "The gate swung open."
                } else {
                    "The gate swung shut."
                }
                .to_owned(),
            ),
            EventKind::GateJammed { by, .. } => Some(format!(
                "The gate couldn't close with {} standing in it.",
                name(by)
            )),
            EventKind::SceneryWarned { .. } if event.tick == m.ruleset.ticks_per_turn => {
                Some(format!("The {} is about to come round.", words(m).name))
            }
            EventKind::SwitchTurned { slot, .. } => {
                let w = words(m);
                Some(format!(
                    "{} {} {}: the {} is coming.",
                    name(slot),
                    w.turned,
                    w.switch,
                    w.name
                ))
            }
            EventKind::SwitchDefended { slot, holders, .. } => Some(format!(
                "{} guarded {} and shooed {} off.",
                list(m, holders),
                words(m).switch,
                name(slot)
            )),
            EventKind::SwitchResting { slot, .. } => {
                let w = words(m);
                Some(format!(
                    "{} {} {}, but nothing more came of it just yet.",
                    name(slot),
                    w.turned,
                    w.switch
                ))
            }
            EventKind::SceneryLanded { hit, sheltered, .. } => Some({
                let w = words(m);
                let mut line = format!("The {} {}", w.name, w.lands);
                if !hit.is_empty() {
                    line.push_str(&format!(" and {} {}", w.hits, list(m, hit)));
                }
                if !sheltered.is_empty() {
                    line.push_str(&format!("; {} stayed dry under a leaf", list(m, sheltered)));
                }
                line.push('.');
                line
            }),
            EventKind::ItemsGlint { item, .. } => Some(format!(
                "Something is glinting: a {} is about to turn up.",
                item.name()
            )),
            EventKind::PickedUp { slot, item, .. } => {
                Some(format!("{} picked up a {}.", name(slot), item.name()))
            }
            EventKind::Dropped { slot, item, .. } => {
                Some(format!("{} dropped its {}.", name(slot), item.name()))
            }
            EventKind::ItemUsed { slot, item, .. } => Some(format!(
                "{} {}.",
                name(slot),
                match item {
                    crate::items::Item::Puff => "blew a dandelion puff",
                    crate::items::Item::Twig => "threw a twig",
                    crate::items::Item::Leaf => "opened a leaf umbrella",
                    crate::items::Item::Bell => "rang a pebble bell",
                    crate::items::Item::Honey => "dropped some honey",
                    crate::items::Item::Seed => "bit into a springy seed",
                }
            )),
            EventKind::Puffed { blown, braced, .. } => {
                let moved: Vec<Slot> = blown.iter().map(|b| b.slot).collect();
                match (moved.is_empty(), braced.is_empty()) {
                    (true, true) => Some("The puff blew across empty ground.".to_owned()),
                    (false, true) => Some(format!("The puff bowled {} over.", list(m, &moved))),
                    (true, false) => Some(format!(
                        "{} braced side by side and the puff blew past.",
                        list(m, braced)
                    )),
                    (false, false) => Some(format!(
                        "The puff bowled {} over; {} braced and stood firm.",
                        list(m, &moved),
                        list(m, braced)
                    )),
                }
            }
            EventKind::TwigWedged { by, .. } => {
                Some(format!("{}'s twig stuck fast in the ground.", name(by)))
            }
            EventKind::TwigFell { .. } => Some("A twig toppled over.".to_owned()),
            EventKind::RaftFloated { by, .. } => Some(format!(
                "{}'s twig floated on the water as a raft.",
                name(by)
            )),
            EventKind::RaftSank { .. } => Some("A twig raft drifted off.".to_owned()),
            EventKind::Rung { by, hushed, .. } => Some(if hushed.is_empty() {
                format!("{}'s bell rang out.", name(by))
            } else {
                format!(
                    "{}'s bell rang out, and {} forgot what they were doing.",
                    name(by),
                    list(m, hushed)
                )
            }),
            EventKind::HoneySpilled { by, .. } => {
                Some(format!("{}'s honey spread over the ground.", name(by)))
            }
            EventKind::HoneyDried { .. } => Some("The honey dried up.".to_owned()),
            EventKind::Stuck { slot, .. } => {
                Some(format!("{} got stuck in the honey.", name(slot)))
            }
            EventKind::Hopped { slot, .. } => {
                Some(format!("{} sprang two tiles in one go.", name(slot)))
            }
            EventKind::GroundReopened { .. } => {
                Some(format!("The {} went down again.", words(m).name))
            }
            EventKind::GateWedged { .. } => {
                Some("The gate couldn't close with a twig wedged in it.".to_owned())
            }
            EventKind::Sheltering { slots, .. } => {
                Some(format!("{} is under a leaf umbrella.", list(m, slots)))
            }
            EventKind::Sheltered { target, by } => Some(format!(
                "{} pressed, and bounced off {}'s leaf umbrella.",
                list(m, by),
                name(target)
            )),
            EventKind::ItemFizzled { slot, item, .. } => {
                Some(format!("{}'s {} came to nothing.", name(slot), item.name()))
            }
            EventKind::KnockedOut { slot } => Some(format!(
                "{} flopped over, dizzy, and sits out the rest of the match.",
                name(slot)
            )),
            EventKind::BrokeThrough { side, target } => Some(format!(
                "{} broke through: with {}'s hold broken, their beacon jumped a tick.",
                team_name(*side),
                name(target)
            )),
            EventKind::CaptureContested if !contested_told => {
                contested_told = true;
                Some("The beacon is contested.".to_owned())
            }
            EventKind::CaptureProgress { side, .. } => {
                progress_told.insert(*side);
                None
            }
            EventKind::Finished { outcome } => Some(match (outcome.winner, outcome.victory) {
                (Some(side), Some(Victory::Capture)) => {
                    format!("{} lit the beacon and wins!", team_name(side))
                }
                (Some(side), Some(Victory::Knockout)) => {
                    format!("{} is the last team standing and wins!", team_name(side))
                }
                (Some(side), _) => {
                    // Says which of the cap's three counts decided it, and by how much.
                    let (won, lost) = (m.cap_tally(side), m.cap_tally(side.other()));
                    let winner = team_name(side);
                    if won.0 != lost.0 {
                        format!(
                            "Time's up: {winner} wins, ahead on the beacon {} to {}.",
                            won.0, lost.0
                        )
                    } else if won.1 != lost.1 {
                        format!(
                            "Time's up, level on the beacon: {winner} wins with more composure \
                             left, {} to {}.",
                            won.1, lost.1
                        )
                    } else {
                        format!(
                            "Time's up, level on the beacon and composure: {winner} wins with \
                             more still standing, {} to {}.",
                            won.2, lost.2
                        )
                    }
                }
                (None, _) => "It's a draw.".to_owned(),
            }),
            _ => None,
        };
        lines.extend(line);
    }
    for side in progress_told {
        // Placed before the result, if there is one.
        let line = format!(
            "{} holds the beacon ({}/{}).",
            team_name(side),
            m.board.capture[side.index()],
            m.ruleset.capture_ticks
        );
        let at = lines
            .iter()
            .position(|l| l.ends_with("wins!") || l.starts_with("Time's up") || l == "It's a draw.")
            .unwrap_or(lines.len());
        lines.insert(at, line);
    }
    lines
}

/// Where a team order points, in words a player would use: the board shows no coordinates.
fn place(m: &Match, squares: &[Tile]) -> String {
    let yard = squares.iter().filter(|t| m.arena.in_zone(**t)).count();
    match (squares, yard) {
        ([t], 1) if *t == m.arena.zone_centre() => "the middle of the yard".to_owned(),
        ([_], 1) => "a square of the yard".to_owned(),
        ([_], _) => "a square outside the yard".to_owned(),
        (ts, y) if y == ts.len() => format!("{} squares of the yard", ts.len()),
        (ts, _) => format!("{} squares", ts.len()),
    }
}

/// How a creature went its own way this turn, in a few words, if it did: the first time it hung
/// back, went a way of its own, or took the job it did for who it is.
fn own_way(m: &Match, slot: Slot, events: &[Event]) -> Option<String> {
    let n = &m.profile(slot).name;
    events.iter().find_map(|e| match &e.kind {
        EventKind::HungBack { slot: s, .. } if *s == slot => Some(format!("{n} hung back")),
        EventKind::Stepped {
            slot: s, reason, ..
        } if *s == slot => match reason {
            Reason::AvoidedExposure | Reason::AvoidedContestedCentre => {
                Some(format!("{n} went the long way round"))
            }
            Reason::KeptNearAlly { ally } => {
                Some(format!("{n} kept close to {}", m.profile(*ally).name))
            }
            Reason::KeptApartFrom { ally } => {
                Some(format!("{n} kept away from {}", m.profile(*ally).name))
            }
            Reason::SideRoute => Some(format!("{n} wandered along the edge")),
            Reason::TookCover => Some(format!("{n} hugged the wall")),
            Reason::OnImpulse => Some(format!("{n} changed its mind halfway")),
            Reason::Bold => Some(format!("{n} marched straight past them")),
            Reason::SkirtedWarning => Some(format!("{n} kept clear of what was coming")),
            Reason::WentForItem => Some(format!("{n} went for something on the ground")),
            Reason::HungBack => Some(format!("{n} hung back")),
            _ => None,
        },
        EventKind::Assigned { slot: s, why, .. } if *s == slot => match why {
            Why::Bold => Some(format!("{n} was keen to go")),
            Why::Careful => Some(format!("{n} would rather stand its ground")),
            Why::Friendly { ally } => {
                Some(format!("{n} wanted to be by {}", m.profile(*ally).name))
            }
            Why::Avoiding { ally } => Some(format!("{n} kept away from {}", m.profile(*ally).name)),
            Why::Curious => Some(format!("{n} was curious")),
            Why::Whim => Some(format!("{n} went on a whim")),
            _ => None,
        },
        _ => None,
    })
}

/// What came of each of a team's orders this turn, one line each, in the order they were given:
/// who took it on, what came of it, and whether any of them went their own way doing it. Call it
/// after the turn has resolved, with that turn's events.
pub fn orders_told(m: &Match, side: Side, events: &[Event]) -> Vec<String> {
    let Some(record) = m.history.last() else {
        return Vec::new();
    };
    let plan = &record.submitted[side.index()];
    let names = |slots: &[Slot]| list(m, slots);
    // Orders naming the same squares (or the same thing) are told once, under all their
    // numbers: naming a square again only made it more pressing.
    let about = |d: &Directive| (d.verb() == "TRICK", d.squares().to_vec());
    let tactical: Vec<Directive> = plan.iter().filter(|d| d.tactical()).cloned().collect();
    let plan = &tactical;
    let bonus = record.submitted[side.index()]
        .iter()
        .filter_map(|d| match d {
            Directive::UseItem { by, .. } => Some(Slot::new(side, *by)),
            _ => None,
        })
        .map(|slot| {
            let n = &m.profile(slot).name;
            let came = events.iter().find_map(|e| match &e.kind {
                EventKind::ItemUsed { slot: s, item, .. } if *s == slot => {
                    Some(format!("used the {}", item.name()))
                }
                EventKind::ItemFizzled { slot: s, item, .. } if *s == slot => {
                    Some(format!("the {} came to nothing", item.name()))
                }
                _ => None,
            });
            format!(
                "Bonus. USE ITEM, {n}: {}.",
                came.unwrap_or_else(|| "nothing was left to use".to_owned())
            )
        })
        .collect::<Vec<_>>();
    let mut told: Vec<String> = plan
        .iter()
        .enumerate()
        .filter(|(i, d)| !plan[..*i].iter().any(|e| about(e) == about(d)))
        .map(|(_, d)| {
            let given: Vec<usize> = (0..plan.len())
                .filter(|j| about(&plan[*j]) == about(d))
                .collect();
            // Who took it on, first job or a later one, and as what.
            let mut on: Vec<Slot> = Vec::new();
            let mut flanking = false;
            for e in events {
                if let EventKind::Assigned { slot, task, .. }
                | EventKind::Rethought { slot, task, .. } = &e.kind
                    && slot.side == side
                    && task
                        .directive
                        .is_some_and(|k| given.contains(&usize::from(k)))
                {
                    flanking |= task.role == Role::Flank;
                    if !on.contains(slot) {
                        on.push(*slot);
                    }
                }
            }
            let said = match d {
                Directive::Press { squares } => format!("PRESS {}", place(m, squares)),
                Directive::Hold { squares, facing } => {
                    format!("HOLD {} facing {}", place(m, squares), facing.name())
                }
                Directive::Trick { squares } => format!("TRICK {}", place(m, squares)),
                Directive::UseItem { .. } => "USE ITEM".to_owned(),
                Directive::Interact { at } => {
                    let found = events.iter().find_map(|e| match &e.kind {
                        EventKind::PickedUp { item, at: t, .. } if t == at => Some(*item),
                        _ => None,
                    });
                    let lying = m
                        .board
                        .ground
                        .iter()
                        .chain(m.board.glint.iter())
                        .find(|(t, _)| t == at)
                        .map(|(_, i)| *i);
                    match found.or(lying) {
                        Some(item) => format!("INTERACT with the {}", item.name()),
                        None if m.arena.levers.contains(at) || m.arena.switches.contains(at) => {
                            describe_directive(m, d)
                        }
                        None => "INTERACT".to_owned(),
                    }
                }
            };
            let n = given
                .iter()
                .map(|j| (j + 1).to_string())
                .collect::<Vec<_>>()
                .join(" and ");
            if on.is_empty() {
                return format!("{n}. {said}: nobody was free for it.");
            }
            let ours = |s: &Slot| on.contains(s);
            let mut came: Vec<String> = Vec::new();
            for e in events {
                match &e.kind {
                    EventKind::Flanked { target, by, .. } if by.iter().any(ours) => {
                        let broke = events.iter().any(|b| {
                            matches!(&b.kind, EventKind::BrokeThrough { side: s, target: t }
                                if *s == side && t == target)
                        });
                        came.push(if broke {
                            format!("flanked {} and broke through", m.profile(*target).name)
                        } else {
                            format!("flanked {}", m.profile(*target).name)
                        });
                    }
                    EventKind::Repelled {
                        holders,
                        pressers,
                        faced,
                        ..
                    } if pressers.iter().any(ours) => {
                        came.push(if faced.iter().any(ours) {
                            format!("turned away head on by {}", names(holders))
                        } else {
                            format!("turned away by {}", names(holders))
                        });
                    }
                    EventKind::Pressed { target, .. } if ours(target) => {
                        came.push(format!(
                            "{} was pressed on the way",
                            m.profile(*target).name
                        ));
                    }
                    EventKind::Pressed { target, by, .. } if by.iter().any(ours) => {
                        came.push(format!("pushed {} back", m.profile(*target).name));
                    }
                    EventKind::Stole { by, item, .. } if ours(by) => {
                        came.push(format!("took the {}", item.name()));
                    }
                    EventKind::Collided { target, by } if ours(target) || by.iter().any(ours) => {
                        came.push("ran into a press of theirs".to_owned());
                    }
                    EventKind::Tricked { target, by, .. } if by.iter().any(ours) => {
                        came.push(format!("tricked {}", m.profile(*target).name));
                    }
                    EventKind::TrickRead { target, by } if ours(by) => {
                        came.push(format!("{} saw it coming", m.profile(*target).name));
                    }
                    EventKind::LeverPulled { slot, .. } | EventKind::SwitchTurned { slot, .. }
                        if ours(slot) =>
                    {
                        came.push("set it off".to_owned());
                    }
                    EventKind::LeverDefended { slot, .. }
                    | EventKind::SwitchDefended { slot, .. }
                        if ours(slot) =>
                    {
                        came.push("shooed off".to_owned());
                    }
                    EventKind::PickedUp { slot, item, .. } if ours(slot) => {
                        came.push(format!(
                            "{} picked up the {}",
                            m.profile(*slot).name,
                            item.name()
                        ));
                    }
                    EventKind::ItemUsed { slot, item, .. } if ours(slot) => {
                        came.push(format!(
                            "{} used the {}",
                            m.profile(*slot).name,
                            item.name()
                        ));
                    }
                    _ => {}
                }
            }
            if flanking && !came.iter().any(|c| c.starts_with("flanked")) {
                came.push("getting into place to flank".to_owned());
            }
            if let Directive::Press { squares } | Directive::Hold { squares, .. } = d {
                let held = squares
                    .iter()
                    .filter(|t| {
                        m.occupant(**t)
                            .is_some_and(|o| o.side == side && m.fighter(o).holding)
                    })
                    .count();
                let theirs = squares
                    .iter()
                    .any(|t| m.occupant(*t).is_some_and(|o| o.side != side));
                came.push(match (held, squares.len()) {
                    (0, _) if theirs => "they still have it".to_owned(),
                    (0, _) => "not held yet".to_owned(),
                    (h, n) if h == n => "held".to_owned(),
                    (h, n) => format!("{h} of {n} held"),
                });
            }
            came.dedup();
            let mut line = format!("{n}. {said}, {}: {}.", names(&on), came.join(", "));
            if let Some(aside) = on.iter().find_map(|s| own_way(m, *s, events)) {
                line.pop();
                line.push_str(&format!("; {aside}."));
            }
            line
        })
        .collect();
    told.extend(bonus);
    told
}
