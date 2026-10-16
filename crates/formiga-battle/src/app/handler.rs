//! The Handler's own pages, opened from the station: the colony's history of matches with its
//! cups and points beside it, and how Battle looks and sounds for this sitting. Desktop keeps the
//! record; Battle keeps no files, so the settings start fresh each time, with reduced motion as
//! Desktop has it.

use super::{BattleApp, card, padlock, portrait_mat, rosette};
use crate::chrome::{self, ink, label_job};
use eframe::egui::{self, Id, RichText};
use formiga_battle_contract::{MatchSummary, Mode};

/// The keys Battle answers to, for the settings page.
const KEYS: [(&str, &str); 9] = [
    (
        "Space, Enter or a click",
        "hurries the train in at the station",
    ),
    (
        "P, H, T, I",
        "picks up a Press, Hold, Trick or Interact order in a match",
    ),
    (
        "U",
        "picks up a Use Item for one of yours carrying something, a bonus beside the three",
    ),
    (
        "R, or the mouse wheel",
        "turns a Hold a quarter, to face another way (Shift+R turns it back)",
    ),
    ("1, 2, 3", "chooses one of your three on the board"),
    ("Enter", "locks in your orders"),
    ("L", "opens the whole match, turn by turn"),
    ("Escape", "puts an order back, or closes the whole match"),
    ("Escape, in the notebook", "turns back a page"),
];

impl BattleApp {
    /// Every match the colony has played, newest first.
    pub(super) fn history(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        let history: Vec<MatchSummary> = self.table.record.history.iter().rev().cloned().collect();
        if history.is_empty() {
            ui.label(
                RichText::new("No matches yet. Take the train out to play one.")
                    .size(14.0)
                    .italics()
                    .color(ink::MUTED),
            );
            return;
        }
        let since = self.turned;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (i, summary) in history.iter().enumerate() {
                    let team: Vec<_> = summary
                        .team
                        .iter()
                        .filter_map(|id| {
                            self.table
                                .home
                                .iter()
                                .find(|(c, _)| c.traveler.id.0 == *id)
                                .map(|(c, _)| c.clone())
                        })
                        .collect();
                    let faces: Vec<egui::TextureHandle> =
                        team.iter().map(|c| self.portrait(ctx, c)).collect();
                    crate::feel::dropping(ui, since, i.min(8), u, |ui| {
                        card(ui, Id::new(("history", i)), false, true, false, u, |ui| {
                            ui.horizontal(|ui| {
                                stamp(ui, summary.won, u);
                                ui.add_space(u * 3.0);
                                ui.vertical(|ui| {
                                    let mut against = match (&summary.mode, &summary.opponent) {
                                        (Mode::Friend, Some(name)) => format!("Against {name}"),
                                        (Mode::Friend, None) => "Against a friend".to_owned(),
                                        (Mode::Ai, _) => "Against the visiting team".to_owned(),
                                    };
                                    if let Some(theirs) = &summary.opponent_trio {
                                        against = format!("{against} and {theirs}");
                                    }
                                    let ours = summary.trio.as_ref().and_then(|id| {
                                        self.table.record.trios.iter().find(|t| &t.id == id)
                                    });
                                    ui.label(
                                        RichText::new(against).size(15.0).strong().color(ink::PAGE),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {}, after {} turns",
                                            day(summary),
                                            ended(&summary.ended_by),
                                            summary.turns
                                        ))
                                        .size(12.5)
                                        .color(ink::MUTED),
                                    );
                                    if let Some(trio) = ours {
                                        ui.label(
                                            RichText::new(format!("Played as {}", trio.name))
                                                .size(12.0)
                                                .color(ink::FOREST),
                                        );
                                    }
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        for face in faces.iter().rev() {
                                            portrait_mat(ui, face, 36.0, u, true);
                                        }
                                    },
                                );
                            });
                        });
                    });
                    ui.add_space(u * 3.0);
                }
            });
    }

    /// The colony's totals beside its history: matches, points and cups.
    pub(super) fn totals(&mut self, ui: &mut egui::Ui, u: f32) {
        let record = &self.table.record;
        let played = record.history.len();
        let won = record
            .history
            .iter()
            .filter(|m| m.won == Some(true))
            .count();
        let drawn = record.history.iter().filter(|m| m.won.is_none()).count();
        chrome::kicker(ui, "Totals");
        ui.add_space(u * 2.0);
        let line = |ui: &mut egui::Ui, label: &str, value: String| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).size(13.0).color(ink::MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(label_job(&value, 12.0, ink::FOREST));
                });
            });
        };
        line(ui, "Matches played", played.to_string());
        line(ui, "Won", won.to_string());
        line(ui, "Drawn", drawn.to_string());
        line(ui, "Open Circuit points", record.circuit_points.to_string());
        line(
            ui,
            "Companions in your notebook",
            record.notebook.len().to_string(),
        );
        ui.add_space(u * 6.0);
        chrome::kicker(ui, "The Championship Circuit");
        ui.add_space(u * 2.0);
        let open = crate::rewards::open_to_play(record);
        for cup in &crate::rewards::CUPS {
            let wins = record.cup_wins(cup.id).min(crate::rewards::CUP_WINS);
            ui.horizontal(|ui| {
                ui.label(RichText::new(cup.name).size(13.0).color(ink::PAGE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = u;
                    if open.contains(&cup.arena) {
                        for k in (0..crate::rewards::CUP_WINS).rev() {
                            rosette(ui, crate::rewards::cup_index(cup.id), k < wins, u);
                        }
                    } else {
                        padlock(ui, "Win the cup before it to open this one", u);
                    }
                });
            });
            ui.add_space(u);
        }
        ui.add_space(u * 4.0);
        ui.label(
            RichText::new("Formiga Desktop keeps all of this with your colony.")
                .size(11.5)
                .italics()
                .color(ink::MUTED),
        );
    }

    /// How Battle looks and sounds, for this sitting.
    pub(super) fn settings(&mut self, ui: &mut egui::Ui, u: f32) {
        let since = self.turned;
        let mut reduce = self.table.reduce_motion;
        crate::feel::dropping(ui, since, 0, u, |ui| {
            card(
                ui,
                Id::new("settings-motion"),
                false,
                true,
                false,
                u,
                |ui| {
                    switch_row(ui, "Reduce motion", MOTION, &mut reduce, u);
                },
            );
        });
        if reduce != self.table.reduce_motion {
            self.table.reduce_motion = reduce;
            // The station is painted again the way motion now is.
            self.station = None;
            self.station_picture = None;
            self.masthead = None;
            self.masthead_picture = None;
            self.turn = None;
        }
        ui.add_space(u * 4.0);
        let mut levels = self.sound.levels();
        crate::feel::dropping(ui, since, 1, u, |ui| {
            card(ui, Id::new("settings-sound"), false, true, false, u, |ui| {
                let mut on = !levels.muted;
                switch_row(ui, "Sound", SOUND, &mut on, u);
                levels.muted = !on;
                ui.add_space(u * 3.0);
                let width = (ui.available_width() - 110.0).max(120.0);
                let muted = levels.muted;
                for (label, value) in [("Music", &mut levels.music), ("Sounds", &mut levels.sounds)]
                {
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [100.0, u * 12.0],
                            egui::Label::new(RichText::new(label).size(13.0).color(ink::PAGE)),
                        );
                        chrome::dial(ui, value, width, !muted, u);
                    });
                }
            });
        });
        if levels != self.sound.levels() {
            self.sound.set_levels(levels);
        }
    }

    /// Beside the settings: the keys, and which Battle this is.
    pub(super) fn keys_and_about(&mut self, ui: &mut egui::Ui, u: f32) {
        chrome::kicker(ui, "Keys");
        ui.add_space(u * 2.0);
        for (keys, does) in KEYS {
            ui.label(label_job(keys, 9.5, ink::FOREST));
            ui.label(RichText::new(does).size(12.5).color(ink::PAGE));
            ui.add_space(u);
        }
        ui.add_space(u * 6.0);
        chrome::kicker(ui, "This Battle");
        ui.add_space(u * 2.0);
        ui.label(
            RichText::new(format!(
                "Formiga Battle {}, rules version {}",
                env!("CARGO_PKG_VERSION"),
                formiga_battle_core::RULES_VERSION
            ))
            .size(12.5),
        );
        ui.label(
            RichText::new(self.host.describe())
                .size(12.5)
                .color(ink::MUTED),
        );
    }
}

const MOTION: &str = "The train is already in, pages change without turning, and nothing hops or \
     springs. It starts as Formiga Desktop has it.";
const SOUND: &str = "Music and sounds, or none at all: the same as the speaker on the cover.";

/// A setting that is on or off: its name and what it does, with its switch at the right. The
/// switch takes its room first, so the words wrap beside it rather than run under it.
fn switch_row(ui: &mut egui::Ui, name: &str, does: &str, on: &mut bool, u: f32) {
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            chrome::switch(ui, on, u);
            ui.add_space(u * 6.0);
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.label(label_job(name, chrome::LABEL + 1.0, ink::FOREST));
                ui.label(RichText::new(does).size(12.5));
            });
        });
    });
}

/// A match's result as a rubber stamp: won in green, lost in red, drawn in grey.
fn stamp(ui: &mut egui::Ui, won: Option<bool>, u: f32) {
    let (word, colour) = match won {
        Some(true) => ("Won", ink::FOREST),
        Some(false) => ("Lost", ink::STAMP),
        None => ("Drawn", ink::MUTED),
    };
    let galley = ui.painter().layout_job(label_job(word, 11.0, colour));
    let size = egui::vec2(u * 32.0, galley.size().y + u * 6.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter();
    chrome::stepped(painter, rect, u, colour);
    chrome::stepped(painter, rect.shrink(u * 2.0), u, colour.gamma_multiply(0.5));
    painter.galley(rect.center() - galley.size() / 2.0, galley, colour);
}

/// The day a match finished, as a notebook dates it: 15 Oct 2026.
fn day(summary: &MatchSummary) -> String {
    date(summary.finished_at_utc)
}

/// A day as the notebook dates it: 15 Oct 2026.
pub(super) fn date(at: time::OffsetDateTime) -> String {
    let date = at.date();
    let month = format!("{:?}", date.month());
    format!("{} {} {}", date.day(), &month[..3], date.year())
}

/// How a match ended, in a few words.
fn ended(by: &str) -> &'static str {
    match by {
        "capture" => "the beacon lit",
        "knockout" => "everyone sitting out",
        "turn_cap" => "on the tally",
        "forfeit" => "by forfeit",
        _ => "finished",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_match_is_dated_as_the_notebook_dates_it() {
        let summary = formiga_battle_contract::sample::record().history[0].clone();
        let dated = day(&summary);
        assert!(dated.split(' ').count() == 3, "{dated}");
        assert_eq!(ended("capture"), "the beacon lit");
    }
}
