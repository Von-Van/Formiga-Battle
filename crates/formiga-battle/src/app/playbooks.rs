//! Playbooks: a trio's plans for a match, written in stages, kept with the career (`career.rs`).
//! On the left, the trio and its playbooks; on the right, the one open, stage by stage. Against
//! the visitors, the playbook a trio plays by fills in each turn's orders, and the Handler changes
//! any of them before locking in (`table.rs`). See docs/PLAYBOOK.md.

use super::{BattleApp, Step};
use crate::career::{PLAYBOOKS_KEPT, Shelf};
use crate::chrome::{self, Kind, ink};
use eframe::egui::{self, RichText};
use formiga_battle_core::playbook::{
    Aim, FacingHint, Fallback, Intent, MAX_STAGE_TURNS, MAX_STAGES, NAME_LEN, Playbook, Posture,
    Stage, Trigger,
};

/// One way of setting a stage, as a Handler reads it.
fn intent_word(i: Intent) -> &'static str {
    i.label()
}

fn aim_word(a: Aim) -> &'static str {
    match a {
        Aim::Centre => "The middle",
        Aim::Near => "Our side",
        Aim::Far => "Their side",
    }
}

fn posture_word(p: Posture) -> &'static str {
    match p {
        Posture::AllIn => "Nobody",
        Posture::Balanced => "One",
        Posture::Guarded => "Two",
    }
}

fn facing_word(f: FacingHint) -> &'static str {
    match f {
        FacingHint::Them => "Toward them",
        FacingHint::Across => "Across the board",
    }
}

fn fallback_word(f: Fallback) -> &'static str {
    match f {
        Fallback::Next => "On to the next",
        Fallback::Again => "Try it again",
        Fallback::First => "Back to the first",
    }
}

/// A stage in a sentence or two.
pub fn stage_words(stage: &Stage) -> String {
    let kept = match stage.posture {
        Posture::AllIn => "sends everyone",
        Posture::Balanced => "keeps one on the beacon",
        Posture::Guarded => "keeps two on the beacon",
    };
    let looks = match stage.aim {
        Aim::Centre => "looks to the middle first",
        Aim::Near => "looks to our side first",
        Aim::Far => "looks to their side first",
    };
    let turns = match stage.turns {
        1 => "One turn".to_owned(),
        n => format!("{n} turns"),
    };
    let then = match stage.fallback {
        Fallback::Next => "then on to the next stage",
        Fallback::Again => "then it tries again",
        Fallback::First => "then back to the first stage",
    };
    let mut words = format!(
        "{}: {kept}, {looks}. It is done with {}. {turns} to get it, {then}.",
        stage.intent.label(),
        stage.intent.goal()
    );
    if let Some(t) = stage.when {
        words.push_str(&format!(" It comes in whenever {}.", t.label()));
    }
    words
}

/// A playbook's stages in one line: "Secure, then Draw out, then Counter when they hold the
/// beacon".
fn outline(book: &Playbook) -> String {
    book.stages
        .iter()
        .map(|s| match s.when {
            Some(t) => format!("{} when {}", s.intent.label(), t.label()),
            None => s.intent.label().to_owned(),
        })
        .collect::<Vec<_>>()
        .join(", then ")
}

/// Buttons side by side along their tops, wrapping onto the next line.
fn row() -> egui::Layout {
    egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(true)
}

/// A row of choices, the one set marked; true if another was chosen.
fn choices<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    options: &[(T, &str)],
    value: &mut T,
    u: f32,
) -> bool {
    ui.label(RichText::new(label).size(15.0).color(ink::MUTED));
    let mut changed = false;
    ui.with_layout(row(), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(u * 2.0, u * 2.0);
        for (option, word) in options {
            let on = *value == *option;
            if chrome::button(ui, word, Kind::Plain, on, true, None, u).clicked() && !on {
                *value = *option;
                changed = true;
            }
        }
    });
    ui.add_space(u * 2.0);
    changed
}

impl BattleApp {
    /// The trios still flying, by id, with their names.
    fn flying_trios(&self) -> Vec<(String, String)> {
        self.table
            .record
            .trios
            .iter()
            .filter(|t| !t.retired)
            .map(|t| (t.id.clone(), t.name.clone()))
            .collect()
    }

    /// The trio the page is open at: the one chosen, if it still flies, else the three picked,
    /// else the first.
    fn shelf_trio_id(&self) -> Option<String> {
        let flying = self.flying_trios();
        self.shelf_trio
            .clone()
            .filter(|id| flying.iter().any(|(t, _)| t == id))
            .or_else(|| self.table.picked_trio().map(|t| t.id.clone()))
            .filter(|id| flying.iter().any(|(t, _)| t == id))
            .or_else(|| flying.first().map(|(id, _)| id.clone()))
    }

    fn shelf(&self, trio: &str) -> Shelf {
        self.table
            .career
            .playbooks
            .get(trio)
            .cloned()
            .unwrap_or_default()
    }

    /// Keeps a trio's shelf as changed, in the career and on this computer.
    fn keep_shelf(&mut self, trio: &str, mut shelf: Shelf) {
        shelf.tidy();
        self.table.career.playbooks.insert(trio.to_owned(), shelf);
        self.table.keep_career();
    }

    /// The left page: which trio, its playbooks, and new ones started from a sample.
    pub(super) fn shelf_left(&mut self, ui: &mut egui::Ui, u: f32) {
        let flying = self.flying_trios();
        let Some(trio) = self.shelf_trio_id() else {
            ui.label(
                RichText::new(
                    "A playbook is written for three who play together. Name a trio on the Handler page first.",
                )
                .size(15.0)
                .color(ink::PAGE),
            );
            ui.add_space(u * 3.0);
            if chrome::button(ui, "Your Handler", Kind::Plain, false, true, None, u).clicked() {
                let now = self.clock;
                self.turn_to(Step::Handler, now);
            }
            return;
        };
        chrome::kicker(ui, "Trio");
        ui.add_space(u * 2.0);
        ui.with_layout(row(), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(u * 2.0, u * 2.0);
            for (id, name) in &flying {
                let on = *id == trio;
                if chrome::button(ui, name, Kind::Plain, on, true, None, u).clicked() && !on {
                    self.shelf_trio = Some(id.clone());
                    self.shelf_book = 0;
                    self.shelf_stage = 0;
                    self.shelf_removing = false;
                }
            }
        });
        ui.add_space(u * 5.0);
        let mut shelf = self.shelf(&trio);
        chrome::kicker(ui, "Their playbooks");
        ui.add_space(u * 2.0);
        if shelf.books.is_empty() {
            ui.label(
                RichText::new("No playbooks yet. Start one from a sample below.")
                    .size(15.0)
                    .color(ink::MUTED),
            );
        }
        for (i, book) in shelf.books.iter().enumerate() {
            ui.horizontal(|ui| {
                let on = i == self.shelf_book;
                if chrome::button(ui, &book.name, Kind::Plain, on, true, None, u).clicked() && !on {
                    self.shelf_book = i;
                    self.shelf_stage = 0;
                    self.shelf_removing = false;
                }
                if shelf.playing == Some(i) {
                    ui.label(
                        RichText::new("Plays when they go")
                            .size(15.0)
                            .color(ink::FOREST),
                    );
                }
            });
            ui.label(RichText::new(outline(book)).size(15.0).color(ink::MUTED));
            ui.add_space(u * 3.0);
        }
        ui.add_space(u * 3.0);
        chrome::kicker(ui, "Start one from");
        ui.add_space(u * 2.0);
        let room = shelf.books.len() < PLAYBOOKS_KEPT;
        let mut started = None;
        ui.with_layout(row(), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(u * 2.0, u * 2.0);
            for sample in Playbook::samples() {
                let response = chrome::button(ui, &sample.name, Kind::Plain, false, room, None, u)
                    .on_hover_text(outline(&sample))
                    .on_disabled_hover_text("A trio keeps three playbooks at most.");
                if response.clicked() {
                    started = Some(sample);
                }
            }
        });
        if let Some(sample) = started {
            shelf.books.push(sample);
            self.shelf_book = shelf.books.len() - 1;
            self.shelf_stage = 0;
            self.shelf_removing = false;
            self.keep_shelf(&trio, shelf);
        }
    }

    /// The right page: the open playbook, stage by stage.
    pub(super) fn shelf_right(&mut self, ui: &mut egui::Ui, u: f32) {
        let Some(trio) = self.shelf_trio_id() else {
            return;
        };
        let mut shelf = self.shelf(&trio);
        let Some(mut book) = shelf.books.get(self.shelf_book).cloned() else {
            chrome::kicker(ui, "Choose or start a playbook");
            return;
        };
        let index = self.shelf_book;
        let mut changed = false;
        chrome::kicker(ui, "Name");
        ui.add_space(u);
        let name = ui.add(
            egui::TextEdit::singleline(&mut book.name)
                .char_limit(NAME_LEN)
                .id_salt(("playbook-name", &trio, index)),
        );
        // Kept as typed, and written down when the field is left.
        if name.changed() && !book.name.trim().is_empty() {
            shelf.books[index].name = book.name.clone();
            self.table
                .career
                .playbooks
                .insert(trio.clone(), shelf.clone());
        }
        if name.lost_focus() {
            changed = true;
        }
        ui.add_space(u * 3.0);
        let playing = shelf.playing == Some(index);
        if playing {
            if chrome::button(ui, "Stop playing by it", Kind::Back, false, true, None, u).clicked()
            {
                shelf.playing = None;
                changed = true;
            }
        } else if chrome::button(ui, "Play by this one", Kind::Primary, false, true, None, u)
            .on_hover_text(
                "Against the visitors, its orders are filled in each turn for you to change.",
            )
            .clicked()
        {
            shelf.playing = Some(index);
            changed = true;
        }
        ui.add_space(u * 4.0);
        // Which stage is open, and room for another.
        self.shelf_stage = self.shelf_stage.min(book.stages.len().saturating_sub(1));
        ui.with_layout(row(), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(u * 2.0, u * 2.0);
            for i in 0..book.stages.len() {
                let on = i == self.shelf_stage;
                if chrome::button(
                    ui,
                    &format!("Stage {}", i + 1),
                    Kind::Plain,
                    on,
                    true,
                    None,
                    u,
                )
                .clicked()
                {
                    self.shelf_stage = i;
                }
            }
            if book.stages.len() < MAX_STAGES
                && chrome::button(ui, "Add a stage", Kind::Quiet, false, true, None, u).clicked()
            {
                book.stages.push(Stage::new(Intent::Secure));
                self.shelf_stage = book.stages.len() - 1;
                changed = true;
            }
        });
        ui.add_space(u * 3.0);
        let k = self.shelf_stage;
        if let Some(stage) = book.stages.get_mut(k) {
            ui.label(
                RichText::new(stage_words(stage))
                    .size(15.0)
                    .color(ink::PAGE),
            );
            ui.add_space(u * 3.0);
            let intents = Intent::ALL.map(|i| (i, intent_word(i)));
            changed |= choices(ui, "What it wants", &intents, &mut stage.intent, u);
            let aims = [Aim::Centre, Aim::Near, Aim::Far].map(|a| (a, aim_word(a)));
            changed |= choices(ui, "Where it looks first", &aims, &mut stage.aim, u);
            let postures =
                [Posture::AllIn, Posture::Balanced, Posture::Guarded].map(|p| (p, posture_word(p)));
            changed |= choices(ui, "Kept on the beacon", &postures, &mut stage.posture, u);
            let facings = [FacingHint::Them, FacingHint::Across].map(|f| (f, facing_word(f)));
            changed |= choices(ui, "Holds facing", &facings, &mut stage.facing, u);
            let turns: Vec<(u8, String)> =
                (1..=MAX_STAGE_TURNS).map(|n| (n, n.to_string())).collect();
            let turns: Vec<(u8, &str)> = turns.iter().map(|(n, w)| (*n, w.as_str())).collect();
            changed |= choices(ui, "Turns to get it", &turns, &mut stage.turns, u);
            let fallbacks =
                [Fallback::Next, Fallback::Again, Fallback::First].map(|f| (f, fallback_word(f)));
            changed |= choices(
                ui,
                "If the turns run out",
                &fallbacks,
                &mut stage.fallback,
                u,
            );
            let mut when: Option<Trigger> = stage.when;
            let mut triggers: Vec<(Option<Trigger>, &str)> = vec![(None, "Only in its turn")];
            triggers.extend(Trigger::ALL.map(|t| (Some(t), t.label())));
            if choices(ui, "Comes in whenever", &triggers, &mut when, u) {
                stage.when = when;
                changed = true;
            }
        }
        ui.add_space(u * 3.0);
        let mut remove = false;
        ui.with_layout(row(), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(u * 2.0, u * 2.0);
            if book.stages.len() > 1
                && chrome::button(ui, "Remove this stage", Kind::Back, false, true, None, u)
                    .clicked()
            {
                book.stages.remove(k);
                self.shelf_stage = k.saturating_sub(1);
                changed = true;
            }
            let word = if self.shelf_removing {
                "Remove the playbook for good"
            } else {
                "Remove this playbook"
            };
            if chrome::button(ui, word, Kind::Back, false, true, None, u).clicked() {
                if self.shelf_removing {
                    remove = true;
                }
                self.shelf_removing = true;
            }
        });
        if remove {
            shelf.books.remove(index);
            shelf.playing = match shelf.playing {
                Some(p) if p == index => None,
                Some(p) if p > index => Some(p - 1),
                other => other,
            };
            self.shelf_book = index.saturating_sub(1);
            self.shelf_stage = 0;
            self.shelf_removing = false;
            self.keep_shelf(&trio, shelf);
            return;
        }
        if changed {
            if book.name.trim().is_empty() {
                book.name = format!("Playbook {}", index + 1);
            }
            shelf.books[index] = book;
            self.keep_shelf(&trio, shelf);
        }
    }

    /// For review pictures: the three picked, or the first trio, given the first samples and
    /// playing by the first.
    pub(super) fn sample_playbooks(&mut self) {
        let trio = self
            .table
            .picked_trio()
            .map(|t| t.id.clone())
            .or_else(|| self.flying_trios().first().map(|(id, _)| id.clone()));
        if let Some(id) = trio {
            let shelf = Shelf {
                books: Playbook::samples()
                    .into_iter()
                    .take(PLAYBOOKS_KEPT)
                    .collect(),
                playing: Some(0),
            };
            self.table.career.playbooks.insert(id, shelf);
        }
    }

    /// On the Who page, beside a trio: which playbook it plays by, and the way to write them.
    pub(super) fn playbook_chooser(&mut self, ui: &mut egui::Ui, trio: &str, u: f32) {
        let mut shelf = self.shelf(trio);
        chrome::kicker(ui, "Playbook");
        ui.add_space(u);
        if shelf.books.is_empty() {
            ui.label(
                RichText::new("No playbooks for them yet.")
                    .size(15.0)
                    .color(ink::MUTED),
            );
            ui.add_space(u);
        }
        let mut playing = shelf.playing;
        let mut options: Vec<(Option<usize>, &str)> = vec![(None, "None")];
        options.extend(
            shelf
                .books
                .iter()
                .enumerate()
                .map(|(i, b)| (Some(i), b.name.as_str())),
        );
        let changed = !shelf.books.is_empty()
            && choices(
                ui,
                "Fills in their orders each turn",
                &options,
                &mut playing,
                u,
            );
        if changed {
            shelf.playing = playing;
            self.keep_shelf(trio, shelf);
        }
        if chrome::button(ui, "Write playbooks", Kind::Quiet, false, true, None, u).clicked() {
            self.shelf_trio = Some(trio.to_owned());
            let outing = self.outing;
            self.open_journal(outing, Step::Playbooks);
        }
    }
}
