//! The Handler Studio's page: how the Handler looks around the Battleground, tried on before it
//! is kept. On the left, each choice. On the right, the look as it stands: a portrait and the
//! Handler walking in each of the four directions, and the button that keeps the look with the
//! colony's career. Only how they look: nothing here touches a match.

use super::*;
use crate::town::look::{
    ACCESSORIES, BUILDS, HAIR_COLOURS, HAIRSTYLES, HandlerLook, OUTFIT_COLOURS, OUTFITS,
    SKIN_TONES, Swatch, portrait,
};
use crate::town::map::Facing;

/// How large the preview's pixels are drawn, in points.
const PREVIEW: f32 = 3.0;
/// The walking preview's pace, in frames a second.
const STRIDE: f32 = 6.0;

impl BattleApp {
    /// The look being tried on: the kept one until something is chosen.
    fn draft(&mut self) -> HandlerLook {
        *self.studio_draft.get_or_insert(self.table.career.look)
    }

    /// The preview's pictures for the look being tried on, painted again only when it changes:
    /// the portrait, then every walking frame, a row of four for each direction.
    fn studio_pictures(&mut self, ctx: &egui::Context) -> Vec<egui::TextureHandle> {
        let look = self.draft();
        if let Some((kept, pictures)) = &self.studio_pictures
            && *kept == look
        {
            return pictures.clone();
        }
        let sheet = look.sheet();
        let mut pictures = vec![texture(ctx, "studio-portrait", &portrait(&sheet))];
        for facing in Facing::ALL {
            for column in 0..4 {
                pictures.push(texture(
                    ctx,
                    &format!("studio-{facing:?}-{column}"),
                    &sheet.frame(facing, column),
                ));
            }
        }
        self.studio_pictures = Some((look, pictures.clone()));
        pictures
    }

    pub(super) fn studio_preview(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        let pictures = self.studio_pictures(ctx);
        let look = self.draft();
        let walking = if self.table.reduce_motion {
            0
        } else {
            (self.clock * STRIDE) as usize % 4
        };
        ui.vertical_centered(|ui| {
            if let Some(face) = pictures.first() {
                ui.add(egui::Image::new(face).fit_to_exact_size(vec2(64.0, 64.0) * PREVIEW * 0.75));
            }
            ui.add_space(u * 3.0);
            ruled::write(
                ui,
                RichText::new(self.handler.trim().to_owned())
                    .size(20.0)
                    .color(ink::PAGE),
            );
            ui.add_space(u * 4.0);
            ui.horizontal(|ui| {
                let each = (ui.available_width() / 4.0).min(48.0 * PREVIEW * 0.7);
                let spare = (ui.available_width() - each * 4.0).max(0.0) / 2.0;
                ui.add_space(spare);
                for (row, facing) in Facing::ALL.iter().enumerate() {
                    if let Some(frame) = pictures.get(1 + row * 4 + walking) {
                        ui.add(egui::Image::new(frame).fit_to_exact_size(vec2(each, each)))
                            .on_hover_text(format!("Facing {facing:?}").to_lowercase());
                    }
                }
            });
            ui.add_space(u * 4.0);
            ui.add(
                egui::Label::new(RichText::new(look.summary()).size(15.0).color(ink::MUTED)).wrap(),
            );
        });
        ui.add_space(u * 4.0);
        let look = self.draft();
        let changed = look != self.table.career.look;
        if chrome::button(ui, "Keep this look", Kind::Primary, false, changed, None, u)
            .on_disabled_hover_text("This is the look you have now")
            .clicked()
        {
            self.keep_look(look);
        }
    }

    pub(super) fn studio_choices(&mut self, ui: &mut egui::Ui, u: f32) {
        let mut look = self.draft();
        swatches(ui, "Hair colour", &HAIR_COLOURS, &mut look.hair, u);
        swatches(
            ui,
            "Jacket: your club colour",
            &OUTFIT_COLOURS,
            &mut look.outfit_colour,
            u,
        );
        swatches(ui, "Skin tone", &SKIN_TONES, &mut look.skin, u);
        choices(ui, "Build", &BUILDS, &mut look.build, u);
        choices(ui, "Hairstyle", &HAIRSTYLES, &mut look.hairstyle, u);
        choices(ui, "Outfit", &OUTFITS, &mut look.outfit, u);
        choices(ui, "Accessory", &ACCESSORIES, &mut look.accessory, u);
        ruled::write(
            ui,
            RichText::new(
                "Hair, jacket and skin show now. Build, hairstyle, outfit and accessory are kept, \
                 and drawn once their pictures are made.",
            )
            .size(14.0)
            .color(ink::MUTED),
        );
        self.studio_draft = Some(look);
    }

    /// Keeps a look with the colony's career, and dresses the Handler in it.
    pub(super) fn keep_look(&mut self, look: HandlerLook) {
        self.table.career.look = look.sane();
        self.table.keep_career();
        if let Some(town) = &mut self.town {
            town.dress(&self.table.career.look);
        }
    }
}

/// A row of colour choices: each a swatch of its colour beside its name, the chosen one marked.
fn swatches(ui: &mut egui::Ui, title: &str, list: &[Swatch], chosen: &mut u8, u: f32) {
    ruled::write(ui, RichText::new(title).size(16.0).color(ink::PAGE));
    // Top-aligned rows that wrap, so every button in a row stands on the same line.
    ui.with_layout(
        egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
        |ui| {
            ui.spacing_mut().item_spacing = vec2(u * 2.0, u * 3.0);
            for (i, (name, hex)) in list.iter().enumerate() {
                let (rect, _) = ui.allocate_exact_size(vec2(u * 6.0, u * 6.0), Sense::hover());
                let colour = Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, *hex as u8);
                ui.painter().rect_filled(rect, 1.0, colour);
                ui.painter().rect_stroke(
                    rect,
                    1.0,
                    Stroke::new(1.0, ink::PAGE),
                    egui::StrokeKind::Inside,
                );
                let picked = usize::from(*chosen) == i;
                if chrome::button(ui, name, Kind::Plain, picked, true, None, u).clicked() {
                    *chosen = i as u8;
                }
                ui.add_space(u * 2.0);
            }
        },
    );
    ui.add_space(u * 3.0);
}

/// A row of word choices, the chosen one marked.
fn choices(ui: &mut egui::Ui, title: &str, list: &[&str], chosen: &mut u8, u: f32) {
    ruled::write(ui, RichText::new(title).size(16.0).color(ink::PAGE));
    // Top-aligned rows that wrap, so every button in a row stands on the same line.
    ui.with_layout(
        egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
        |ui| {
            ui.spacing_mut().item_spacing = vec2(u * 2.0, u * 3.0);
            for (i, name) in list.iter().enumerate() {
                let picked = usize::from(*chosen) == i;
                if chrome::button(ui, name, Kind::Plain, picked, true, None, u).clicked() {
                    *chosen = i as u8;
                }
            }
        },
    );
    ui.add_space(u * 3.0);
}
