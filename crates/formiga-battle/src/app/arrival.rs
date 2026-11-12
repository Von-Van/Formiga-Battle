//! Arriving in town, on screen: the train into the habitat, the pull back to the desk, the office
//! settling on its grid (`crate::town::arrival`), in the town's own frame. Escape, Space, Enter
//! or a click skips the rest. It ends with the Handler at the desk in the office, ready to walk.

use super::town::show_picture;
use super::*;
use crate::station::{Heard, Station};
use crate::town::Town;
use crate::town::arrival::{self, Arrival, Beat};
use crate::town::picture::VIEW;

impl BattleApp {
    /// Arrives in town again from the start, as the Settings page's button asks: the whole of it,
    /// whether or not it has been seen. Nothing else changes.
    pub(super) fn replay_arrival(&mut self) {
        self.arrival = None;
        self.replaying = true;
        self.stage = Stage::Arrival;
    }

    /// The companions' names, as a line says them: "Mochi, Biscuit and Fig".
    pub(super) fn companion_names(&self) -> String {
        let names: Vec<&str> = self
            .table
            .home
            .iter()
            .map(|(c, _)| c.traveler.name.as_str())
            .collect();
        match names.as_slice() {
            [] => String::new(),
            [one] => (*one).to_owned(),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        }
    }

    pub(super) fn arrival_frame(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, now: f32) {
        let reduce = self.table.reduce_motion;
        if self.arrival.is_none() {
            let first = self.replaying
                || self
                    .kept
                    .as_ref()
                    .is_none_or(|k| !k.settings().arrival_seen);
            self.replaying = false;
            let journey = Arrival::new(now, first, reduce);
            // After the first time only the end of the train's run is seen.
            let set_off = if first {
                now
            } else {
                now - (crate::station::PULL_IN - arrival::LATER)
            };
            let looks = self
                .table
                .home
                .iter()
                .filter_map(|(c, _)| Look::of(c, reduce).ok())
                .collect();
            let halt = Station::in_habitat(looks, VIEW, set_off, reduce);
            self.arrival = Some((journey, halt));
            self.habitat_front = arrival::habitat_front();
            self.arrival_heard = now;
            if self.town.is_none() {
                let mut town = Town::new(now);
                town.dress(&self.table.career.look);
                town.cups = self.cups_won();
                town.news = self.table.next_event_fixture().is_some();
                town.titled = !self.table.career.titles.is_empty();
                self.town = Some(town);
            }
        }
        let skip = !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                    | i.consume_key(egui::Modifiers::NONE, egui::Key::Space)
                    | i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
            });
        let Some((journey, halt)) = &mut self.arrival else {
            return;
        };
        if skip {
            journey.skip();
        }
        // The train's whistle and brakes, as at the station.
        for heard in halt.heard(self.arrival_heard, now) {
            crate::feel::emit(
                ctx,
                match heard {
                    Heard::Whistle => crate::feel::Feedback::TrainIn,
                    Heard::Brakes => crate::feel::Feedback::TrainStops,
                    Heard::Away => crate::feel::Feedback::TrainOut,
                },
            );
        }
        self.arrival_heard = now;
        let beat = journey.beat(now);
        let first = journey.first();
        let Some(town) = &mut self.town else {
            return;
        };
        // The halt with what stands in front of the train laid over it.
        let mut close = halt.compose(now);
        if let Some(front) = &self.habitat_front {
            crate::town::picture::over(&mut close, front, 0, 0);
        }
        let view = match beat {
            Beat::CloseUp(_) => close,
            Beat::PullBack(t) => {
                let desk = self.desk.get_or_insert_with(arrival::desk);
                arrival::pull_back(desk, &close, t)
            }
            Beat::Handoff(t) => {
                let desk = self.desk.get_or_insert_with(arrival::desk);
                let office = town.compose(now, reduce);
                arrival::handoff(desk, &office, town.view_origin(now), t)
            }
            Beat::Play => {
                // Into the office: the Handler walks from this frame on. A short arrival comes
                // up out of the dark like a door; the whole one has already settled.
                town.entered = if first { f32::NEG_INFINITY } else { now };
                if let Some(keeper) = &mut self.kept {
                    keeper.arrived();
                }
                self.arrival = None;
                self.stage = Stage::Town;
                return;
            }
        };
        let (_painter, _full, picture_rect, _scale) =
            show_picture(ui, ctx, &mut self.town_picture, &view);
        let response = ui.interact(picture_rect, Id::new("arrival-picture"), Sense::click());
        if response.clicked()
            && let Some((journey, _)) = &mut self.arrival
        {
            journey.skip();
        }
    }
}
