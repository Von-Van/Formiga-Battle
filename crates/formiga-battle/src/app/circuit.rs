//! The Circuit: the Handler's place at the Battleground. On the left, the event under way as a
//! bracket, round by round, and the way to play the next fixture; on the right, the titles won
//! and how the colony stands against each rival. What it shows is the career Battle keeps
//! (`career.rs`); none of it changes how a match is played.

use super::{BattleApp, Kind, Outing, Step};
use crate::chrome::{self, ink};
use crate::rival::{self, RIVALS};
use eframe::egui::{self, RichText};
use formiga_battle_world::Decided;

/// How a fixture was settled, in words after the winner's name.
fn settled(decided: Decided) -> &'static str {
    match decided {
        Decided::Capture => "lit the beacon",
        Decided::Knockout => "won with all three of theirs dizzy",
        Decided::Cap => "won on the tally",
        Decided::Forfeit => "won when the other side gave up",
        Decided::Toss => "went through on the toss after a draw",
    }
}

impl BattleApp {
    /// The event under way, round by round, and the way to the next fixture or a new event.
    pub(super) fn event_board(&mut self, ui: &mut egui::Ui, u: f32) {
        let width = ui.available_width();
        let entered = self.table.career.event.clone();
        match &entered {
            Some(event) => {
                let b = &event.bracket;
                let name = |i: Option<usize>| match i {
                    Some(i) if b.entrants[i] == "you" => "You".to_owned(),
                    Some(i) => rival::by_id(&b.entrants[i])
                        .map_or_else(|| b.entrants[i].clone(), |r| r.name.to_owned()),
                    None => "To be decided".to_owned(),
                };
                chrome::kicker(ui, &event.name);
                ui.add_space(u * 2.0);
                for round in 0..b.rounds() {
                    ui.label(
                        RichText::new(b.round_name(round))
                            .size(13.0)
                            .strong()
                            .color(ink::PAGE),
                    );
                    for f in b.fixtures.iter().filter(|f| f.round == round) {
                        let line = format!("{} v {}", name(f.sides[0]), name(f.sides[1]));
                        let after = match &f.result {
                            Some(r) => format!(
                                "{} {}, in {} turns",
                                name(f.winner()),
                                settled(r.decided),
                                r.turns
                            ),
                            None if f.ready() => "To play".to_owned(),
                            None => "Waiting on the round before".to_owned(),
                        };
                        ui.label(RichText::new(line).size(14.0).color(ink::PAGE));
                        ui.label(RichText::new(after).size(12.5).color(ink::MUTED));
                        ui.add_space(u * 2.0);
                    }
                    ui.add_space(u * 3.0);
                }
                if let Some(champion) = b.champion() {
                    ui.label(
                        RichText::new(format!("Champion: {}", name(Some(champion))))
                            .size(14.0)
                            .strong()
                            .color(ink::FOREST),
                    );
                    ui.add_space(u * 4.0);
                }
            }
            None => {
                ui.label(
                    RichText::new(
                        "No event entered yet. An event is four Handlers on one place: the three who come to it most, and you, new to it, drawn against the first seed.",
                    )
                    .size(13.0)
                    .italics()
                    .color(ink::MUTED),
                );
                ui.add_space(u * 4.0);
            }
        }
        match self.table.next_event_fixture() {
            Some((index, round, rival)) => {
                let label = format!("Play your {round} against {}", rival.name);
                if chrome::button(ui, &label, Kind::Primary, false, true, Some(width), u)
                    .on_hover_text(rival.tag())
                    .clicked()
                {
                    self.table.event_fixture = Some(index);
                    if let Some(arena) = entered
                        .as_ref()
                        .and_then(|e| formiga_battle_core::Arena::by_id(&e.arena))
                    {
                        self.choose_arena(arena);
                    }
                    self.open_journal(Outing::Home, Step::Who);
                }
            }
            None => {
                if !self.arena_open_here() {
                    self.choose_arena(formiga_battle_core::Arena::backyard());
                }
                let label = format!("Enter the {} Open", self.table.arena.name);
                if chrome::button(ui, &label, Kind::Primary, false, true, Some(width), u)
                    .on_hover_text("Choose the place on the Where page of Close to Home first")
                    .clicked()
                {
                    self.table.enter_event();
                }
            }
        }
    }

    fn arena_open_here(&self) -> bool {
        crate::rewards::open_to_play(&self.table.record).contains(&self.table.arena.id.as_str())
    }

    /// Titles won, and how the colony stands against each rival.
    pub(super) fn standing_against_rivals(&mut self, ui: &mut egui::Ui, u: f32) {
        let career = &self.table.career;
        chrome::kicker(ui, "Titles");
        ui.add_space(u * 2.0);
        if career.titles.is_empty() {
            ui.label(
                RichText::new("None yet.")
                    .size(13.0)
                    .italics()
                    .color(ink::MUTED),
            );
        }
        for title in career.titles.iter().rev() {
            ui.label(RichText::new(&title.event).size(13.5).color(ink::FOREST));
        }
        ui.add_space(u * 6.0);
        chrome::kicker(ui, "Against the rivals");
        ui.add_space(u * 2.0);
        for r in &RIVALS {
            let h = career.against(r.id);
            ui.label(
                RichText::new(format!("{} \u{b7} {}", r.name, r.club))
                    .size(13.5)
                    .color(ink::PAGE),
            );
            let record = if h.played == 0 {
                format!("Not met yet. {}", capitalised(r.archetype.tendency()))
            } else {
                format!(
                    "Played {}, won {}, drawn {}, lost {}. {}",
                    h.played,
                    h.won,
                    h.drawn,
                    h.lost,
                    capitalised(r.archetype.tendency())
                )
            };
            ui.label(RichText::new(record).size(12.5).color(ink::MUTED));
            ui.add_space(u * 2.0);
        }
    }
}

fn capitalised(words: &str) -> String {
    let mut chars = words.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect::<String>() + ".",
        None => String::new(),
    }
}
