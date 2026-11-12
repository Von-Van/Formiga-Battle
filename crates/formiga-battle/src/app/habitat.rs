//! Looking into the habitat on the office desk: the companions at their own size, standing on the
//! little platform behind the glass, drawn by `formiga-art` from Desktop's look as everywhere else.
//! One is picked at a time (the arrow keys, A and D, or a click), and the card under them says
//! who it is as Desktop's record has it: its name, its temperament, its traits and what the
//! notebook has seen it do. Battle only shows these; nothing here changes them. E, Enter or
//! Escape goes back to the office.

use super::town::show_picture;
use super::*;
use crate::look::Pose;
use crate::town::picture::VIEW;

/// The row the companions' feet rest on: the top of the habitat's platform.
const PLATFORM: i32 = 152;
/// The platform's ends, where the first and last stand.
const ENDS: (i32, i32) = (96, 330);

/// Looking into the habitat.
pub(super) struct Habitat {
    looks: Vec<Look>,
    picked: usize,
    backdrop: Canvas,
    front: Option<Canvas>,
    /// When it was opened, for the companions' idling.
    since: f32,
}

impl Habitat {
    /// The companion picked, by its place.
    pub(super) fn picked(&self) -> usize {
        self.picked
    }

    /// Picks a companion by its place, if there is one there.
    pub(super) fn pick(&mut self, index: usize) {
        if index < self.looks.len() {
            self.picked = index;
        }
    }
}

impl BattleApp {
    /// Opens the habitat, the first companion picked.
    pub(super) fn open_habitat(&mut self) {
        let reduce = self.table.reduce_motion;
        let looks: Vec<Look> = self
            .table
            .home
            .iter()
            .filter_map(|(c, _)| Look::of(c, reduce).ok())
            .collect();
        let backdrop = crate::town::art::bytes("arrival-habitat.png")
            .and_then(crate::town::sheet::decode)
            .unwrap_or_else(|| Canvas::new(VIEW.0 as u32, VIEW.1 as u32));
        self.habitat = Some(Habitat {
            looks,
            picked: 0,
            backdrop,
            front: crate::town::arrival::habitat_front(),
            since: self.clock,
        });
    }

    /// Where each companion stands along the platform, by the middle of its feet.
    fn standing_places(count: usize) -> Vec<i32> {
        match count {
            0 => Vec::new(),
            1 => vec![(ENDS.0 + ENDS.1) / 2],
            n => (0..n)
                .map(|i| ENDS.0 + (ENDS.1 - ENDS.0) * i as i32 / (n as i32 - 1))
                .collect(),
        }
    }

    pub(super) fn habitat_frame(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, now: f32) {
        let typing = ctx.egui_wants_keyboard_input();
        let (left, right, back) = ctx.input_mut(|i| {
            if typing {
                return (false, false, false);
            }
            let m = egui::Modifiers::NONE;
            let left = i.consume_key(m, egui::Key::ArrowLeft) | i.consume_key(m, egui::Key::A);
            let right = i.consume_key(m, egui::Key::ArrowRight) | i.consume_key(m, egui::Key::D);
            let back = i.consume_key(m, egui::Key::Escape)
                | i.consume_key(m, egui::Key::E)
                | i.consume_key(m, egui::Key::Enter);
            (left, right, back)
        });
        if back {
            self.habitat = None;
            return;
        }
        let reduce = self.table.reduce_motion;
        let Some(habitat) = &mut self.habitat else {
            return;
        };
        let count = habitat.looks.len();
        if count > 0 {
            if left {
                habitat.picked = (habitat.picked + count - 1) % count;
            }
            if right {
                habitat.picked = (habitat.picked + 1) % count;
            }
        }
        let places = Self::standing_places(count);
        let mut view = habitat.backdrop.clone();
        let elapsed = if reduce { 0.0 } else { now - habitat.since };
        for (i, look) in habitat.looks.iter_mut().enumerate() {
            let foot = look.foot_row;
            // Some look one way and some the other, as on the train.
            let frame = look.frame(Pose::Idle, elapsed + i as f32 * 0.41, i % 2 == 0);
            let size = frame.width() as i32;
            crate::town::picture::over(&mut view, frame, places[i] - size / 2, PLATFORM - foot);
        }
        if let Some(front) = &habitat.front {
            crate::town::picture::over(&mut view, front, 0, 0);
        }
        // A little arrow over the one picked, bobbing unless motion is reduced.
        if let Some(&x) = places.get(habitat.picked) {
            let bob = if reduce {
                0
            } else {
                ((now * 3.0).sin() * 1.5).round() as i32
            };
            let y = PLATFORM - 62 + bob;
            for row in 0..5 {
                for dx in -(4 - row)..=(4 - row) {
                    view.set(
                        x + dx,
                        y + row,
                        formiga_art::Rgba::new(0xff, 0xf8, 0xe4, 255),
                    );
                }
            }
        }
        let (_painter, _full, picture_rect, scale) =
            show_picture(ui, ctx, &mut self.town_picture, &view);
        // A click on a companion picks it.
        let response = ui.interact(picture_rect, Id::new("habitat-picture"), Sense::click());
        if let Some(pos) = response
            .interact_pointer_pos()
            .filter(|_| response.clicked())
        {
            let px = (pos.x - picture_rect.min.x) * ctx.pixels_per_point() / scale;
            if let Some((i, _)) = places
                .iter()
                .enumerate()
                .min_by_key(|(_, x)| (**x - px as i32).abs())
            {
                habitat.picked = i;
            }
        }
    }
}
