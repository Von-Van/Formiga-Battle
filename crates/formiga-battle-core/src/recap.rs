//! A turn told in sentences, from its events: what each creature did and why, so nothing that
//! happened rests on dice nobody can see.

use crate::event::{Event, EventKind, Fill, Reason, Victory};
use crate::game::Match;
use crate::muster::{Because, Role, Task, Why};
use crate::order::{Directive, Order, Side, Slot, Target};
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
        Order::Hold { at } => format!("HOLD at {at}"),
        Order::Contest {
            target: Target::Creature { slot },
        } => format!("CONTEST {}", m.profile(*slot).name),
        Order::Contest {
            target: Target::Lever { at },
        } if m.arena.switches.contains(at) => "CONTEST the tap".to_owned(),
        Order::Contest {
            target: Target::Lever { .. },
        } => "CONTEST the lever".to_owned(),
        Order::Use { at } => format!("USE what it carries at {at}"),
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
        Directive::Take { squares: ts } => format!("TAKE {}", squares(ts)),
        Directive::Defend { squares: ts } => format!("DEFEND {}", squares(ts)),
        Directive::Interact { at } if m.arena.switches.contains(at) => {
            "INTERACT with the tap".to_owned()
        }
        Directive::Interact { at } if m.arena.levers.contains(at) => {
            "INTERACT with the lever".to_owned()
        }
        Directive::Interact { at } => format!("INTERACT at {at}"),
    }
}

/// A creature's job, as something to do: "hold (4, 3)", "flank Pip", "fetch the twig".
pub fn job(m: &Match, slot: Slot, task: &Task, order: &Order) -> String {
    let target = match order {
        Order::Contest {
            target: Target::Creature { slot },
        } => Some(m.profile(*slot).name.clone()),
        _ => None,
    };
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
            Some(t) if m.occupant(at).is_some_and(|o| m.fighter(o).carrying.is_some()) => {
                format!("go after what {t} carries")
            }
            Some(t) => format!("move {t} off {at}"),
            None => format!("clear {at}"),
        },
        Role::Flank => match target.or_else(|| m.occupant(at).map(|o| m.profile(o).name.clone())) {
            Some(t) => format!("flank {t}"),
            None => format!("flank whoever holds {at}"),
        },
        Role::Pin => format!("stand beside whoever holds {at}"),
        Role::Guard if tap => "guard the tap".to_owned(),
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
        Role::Pull if tap => "turn the tap".to_owned(),
        Role::Pull => "pull the lever".to_owned(),
        Role::Wait if tap => "wait by the tap".to_owned(),
        Role::Wait => "wait by the lever".to_owned(),
        Role::Use => match m.fighter(slot).carrying {
            Some(item) => format!("use its {}", item.name()),
            None => "use what it carried".to_owned(),
        },
        Role::Spare => "do as it saw fit".to_owned(),
    }
}

/// What the arena's map event is called in the recap.
fn scenery_name(m: &Match) -> &'static str {
    match m.arena.scenery.as_ref().map(|s| s.effect) {
        Some(crate::arena::Effect::Soak) | None => "sprinkler",
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
                    Why::Avoiding { ally } => {
                        Some(format!("{n} chose to {j}, to keep away from {}.", name(ally)))
                    }
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
                        Reason::SteppedBack => format!("{n} hopped back after the contest."),
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
            EventKind::Interrupted { target, by } => Some(format!(
                "{} caught {} on the move.",
                list(m, by),
                name(target)
            )),
            EventKind::Countered {
                target,
                holders,
                contesters,
            } => Some(if holders.len() > 1 {
                format!(
                    "{} held firm side by side and turned {} away.",
                    list(m, holders),
                    list(m, contesters)
                )
            } else {
                format!(
                    "{} held firm and turned {} away.",
                    name(target),
                    list(m, contesters)
                )
            }),
            EventKind::Flanked {
                target,
                by,
                pushed_to,
                ..
            } => Some(match pushed_to {
                Some(_) => format!(
                    "{} came at {} from two sides and pushed it off its spot.",
                    list(m, by),
                    name(target)
                ),
                None => format!(
                    "{} came at {} from two sides; boxed in, it lost its footing.",
                    list(m, by),
                    name(target)
                ),
            }),
            EventKind::Scuffle {
                contester, target, ..
            } => Some(format!(
                "{} and {} got tangled up in a scuffle; nobody minded.",
                name(contester),
                name(target)
            )),
            EventKind::TookOpportunity { slot, meant, took } => Some(format!(
                "{} spotted {} wandering past and went for them instead of {}.",
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
                Some(format!("The {} is about to come round.", scenery_name(m)))
            }
            EventKind::SwitchTurned { slot, .. } => Some(format!(
                "{} turned the tap: the {} is coming.",
                name(slot),
                scenery_name(m)
            )),
            EventKind::SwitchDefended { slot, holders, .. } => Some(format!(
                "{} guarded the tap and shooed {} off.",
                list(m, holders),
                name(slot)
            )),
            EventKind::SwitchResting { slot, .. } => Some(format!(
                "{} turned the tap, but nothing more came of it just yet.",
                name(slot)
            )),
            EventKind::SceneryLanded { hit, sheltered, .. } => Some({
                let mut line = format!("The {} swept the yard", scenery_name(m));
                if !hit.is_empty() {
                    line.push_str(&format!(" and soaked {}", list(m, hit)));
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
            EventKind::GateWedged { .. } => {
                Some("The gate couldn't close with a twig wedged in it.".to_owned())
            }
            EventKind::Sheltering { slots, .. } => {
                Some(format!("{} is under a leaf umbrella.", list(m, slots)))
            }
            EventKind::Sheltered { target, by } => Some(format!(
                "{} bounced off {}'s leaf umbrella.",
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
                    format!("Time's up: {} wins on the beacon tally.", team_name(side))
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
