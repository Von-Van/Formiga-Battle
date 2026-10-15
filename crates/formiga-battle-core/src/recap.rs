//! A turn told in sentences, from its events: what each creature did and why, so nothing that
//! happened rests on dice nobody can see.

use crate::event::{Event, EventKind, Fill, Reason, Victory};
use crate::game::Match;
use crate::order::{Order, Side, Slot, Target};
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
            target: Target::Lever { .. },
        } => "CONTEST the lever".to_owned(),
    }
}

/// The turn's events, told. Each creature's route is told once a turn, by the first choice it
/// made that was not the plain way.
pub fn recap(m: &Match, events: &[Event]) -> Vec<String> {
    let name = |s: &Slot| m.profile(*s).name.clone();
    let mut lines = Vec::new();
    let mut routed: BTreeSet<Slot> = BTreeSet::new();
    let mut contested_told = false;
    let mut progress_told: BTreeSet<Side> = BTreeSet::new();
    for event in events {
        let line = match &event.kind {
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
                    })
                }
            }
            EventKind::Interrupted { target, by } => Some(format!(
                "{} caught {} on the move.",
                list(m, by),
                name(target)
            )),
            EventKind::Countered {
                target,
                holders,
                contesters,
                pressed,
            } => Some(if *pressed {
                format!(
                    "{} pressed {} together; {} held on, wobbling.",
                    list(m, contesters),
                    name(target),
                    name(target)
                )
            } else if holders.len() > 1 {
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
