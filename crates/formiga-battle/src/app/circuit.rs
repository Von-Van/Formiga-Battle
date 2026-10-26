//! The Circuit: the Handler's place at the Battleground. On the left, the event under way as a
//! bracket, round by round, and the way to play the next fixture; on the right, the titles won
//! and how the colony stands against each rival. What it shows is the career Battle keeps
//! (`career.rs`); none of it changes how a match is played.

use super::{BattleApp, Outing, Step};
use crate::career;
use crate::chrome::{self, ink};
use crate::rival::{self, RIVALS};
use eframe::egui::{self, RichText};
use formiga_battle_world::{Decided, Kind};

/// A Handler's name by their id in an event: the colony is "You".
fn who(id: &str) -> String {
    if id == "you" {
        "You".to_owned()
    } else {
        rival::by_id(id).map_or_else(|| id.to_owned(), |r| r.name.to_owned())
    }
}

/// What the Battleground noticed, in words.
fn noticed_words(n: &career::Noticed) -> String {
    use career::Noticed::*;
    match n {
        FirstWin { rival } => format!("First win over {}", who(rival)),
        Upset { rival, event, .. } => {
            format!("Beat {}, seeded above you, in {event}", who(rival))
        }
        Rivalry { rival } => format!("A rivalry: five meetings with {}", who(rival)),
        EventWon { event, season } => format!("Won {event} in Season {season}"),
        SeasonChampion { season } => format!("Season {season} Champion"),
        TitleDefended { season } => format!("Defended the title in Season {season}"),
        TitleLost { season, to } => format!("Lost the title to {} in Season {season}", who(to)),
        TitleReclaimed { season } => format!("Won the title back in Season {season}"),
    }
}

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
        if let Some(season) = self.table.career.season.clone() {
            chrome::kicker(ui, &format!("Season {}", season.number));
            ui.add_space(u * 2.0);
            for (i, stage) in season.stages.iter().enumerate() {
                let place = formiga_battle_core::Arena::by_id(&stage.arena)
                    .map_or_else(|| stage.arena.clone(), |a| a.name);
                let state = if i < season.played {
                    "Played"
                } else if i == season.played {
                    "Next"
                } else {
                    "To come"
                };
                let size = match stage.kind {
                    Kind::Regional => "eight Handlers",
                    Kind::Open => "four Handlers",
                    Kind::Championship => "the season's best four",
                };
                ui.label(
                    RichText::new(format!(
                        "{} \u{b7} {size} \u{b7} {state}",
                        stage.name(&place, season.number)
                    ))
                    .size(15.0)
                    .color(if i == season.played {
                        ink::PAGE
                    } else {
                        ink::MUTED
                    }),
                );
            }
            if let Some(defending) = &season.defending {
                ui.label(
                    RichText::new(format!("Defending champion: {}", who(defending)))
                        .size(15.0)
                        .color(ink::FOREST),
                );
            }
            ui.add_space(u * 5.0);
        }
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
                            .size(15.0)
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
                        ui.label(RichText::new(line).size(16.0).color(ink::PAGE));
                        ui.label(RichText::new(after).size(15.0).color(ink::MUTED));
                        ui.add_space(u * 2.0);
                    }
                    ui.add_space(u * 3.0);
                }
                if let Some(champion) = b.champion() {
                    ui.label(
                        RichText::new(format!("Champion: {}", name(Some(champion))))
                            .size(16.0)
                            .strong()
                            .color(ink::FOREST),
                    );
                    ui.add_space(u * 4.0);
                }
            }
            None => {
                ui.label(
                    RichText::new(
                        "No season begun yet. A season is an Open of four, two Regionals of eight, and a Championship of the four with most season points. Your first event is an Open: the three Handlers who come to its place most, and you, drawn against the first seed.",
                    )
                    .size(15.0)
                    .italics()
                    .color(ink::MUTED),
                );
                ui.add_space(u * 4.0);
            }
        }
        match self.table.next_event_fixture() {
            Some((index, round, rival)) => {
                let label = format!("Play your {round} against {}", rival.name);
                if chrome::button(
                    ui,
                    &label,
                    chrome::Kind::Primary,
                    false,
                    true,
                    Some(width),
                    u,
                )
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
                let label = match &self.table.career.season {
                    Some(s) if !s.finished() => "Begin the next event".to_owned(),
                    Some(s) => format!("Begin Season {}", s.number + 1),
                    None => "Begin Season 1".to_owned(),
                };
                if chrome::button(
                    ui,
                    &label,
                    chrome::Kind::Primary,
                    false,
                    true,
                    Some(width),
                    u,
                )
                .on_hover_text("An event you do not reach is played out at once")
                .clicked()
                {
                    self.table.begin_next_event();
                }
            }
        }
    }

    /// Titles won, and how the colony stands against each rival.
    pub(super) fn standing_against_rivals(&mut self, ui: &mut egui::Ui, u: f32) {
        let career = &self.table.career;
        let (title, how) = career::standing(career);
        chrome::kicker(ui, "Your standing");
        ui.add_space(u * 2.0);
        ui.label(RichText::new(title).size(15.0).strong().color(ink::FOREST));
        ui.label(RichText::new(how).size(15.0).italics().color(ink::MUTED));
        ui.add_space(u * 6.0);
        if let Some(season) = &career.season
            && !season.points.is_empty()
        {
            chrome::kicker(ui, &format!("Season {} points", season.number));
            ui.add_space(u * 2.0);
            for (id, points) in season.table() {
                ui.horizontal(|ui| {
                    let colour = if id == "you" { ink::FOREST } else { ink::PAGE };
                    ui.label(RichText::new(who(&id)).size(15.0).color(colour));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(points.to_string()).size(15.0).color(colour));
                    });
                });
            }
            ui.add_space(u * 6.0);
        }
        if !career.hall.is_empty() {
            chrome::kicker(ui, "Hall of Champions");
            ui.add_space(u * 2.0);
            for c in career.hall.iter().rev() {
                ui.label(
                    RichText::new(format!("Season {}: {}", c.season, who(&c.who)))
                        .size(15.0)
                        .color(if c.who == "you" {
                            ink::FOREST
                        } else {
                            ink::PAGE
                        }),
                );
            }
            ui.add_space(u * 6.0);
        }
        if !career.recognition.is_empty() {
            chrome::kicker(ui, "Noticed");
            ui.add_space(u * 2.0);
            for r in career.recognition.iter().rev().take(6) {
                ui.label(
                    RichText::new(noticed_words(&r.noticed))
                        .size(15.0)
                        .color(ink::PAGE),
                );
            }
            ui.add_space(u * 6.0);
        }
        chrome::kicker(ui, "Titles");
        ui.add_space(u * 2.0);
        if career.titles.is_empty() {
            ui.label(
                RichText::new("None yet.")
                    .size(15.0)
                    .italics()
                    .color(ink::MUTED),
            );
        }
        for title in career.titles.iter().rev() {
            ui.label(RichText::new(&title.event).size(16.0).color(ink::FOREST));
        }
        ui.add_space(u * 6.0);
        chrome::kicker(ui, "Against the rivals");
        ui.add_space(u * 2.0);
        for r in &RIVALS {
            let h = career.against(r.id);
            ui.label(
                RichText::new(format!("{} \u{b7} {}", r.name, r.club))
                    .size(16.0)
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
            ui.label(RichText::new(record).size(15.0).color(ink::MUTED));
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
