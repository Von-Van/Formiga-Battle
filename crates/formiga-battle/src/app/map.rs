//! The campus map, on M from anywhere in town: the whole courtyard at a whole number of screen
//! pixels to its own, each building named at its door and the Handler marked where they are (at
//! the door of the building they are in, when indoors). M or Escape closes it.

use super::*;
use crate::town::map::PlaceId;
use crate::town::picture::TILE;

/// Each building's name on the map, by the place its door leads to.
fn label(place: PlaceId) -> &'static str {
    match place {
        PlaceId::Office => "Your office",
        PlaceId::Campus => "The courtyard",
        PlaceId::Arena => "Arena",
        PlaceId::Clubhouse => "Clubhouse",
        PlaceId::GuildHall => "Guild Hall",
        PlaceId::Studio => "Handler Studio",
    }
}

impl BattleApp {
    /// The whole courtyard as it looks, without anyone in it.
    fn campus_picture(&self) -> Canvas {
        let scenery = PlaceId::Campus.scenery();
        let mut world = scenery.ground.clone();
        let cups = self.cups_won();
        for piece in scenery.pieces(cups, 0.0, true) {
            crate::town::picture::over(&mut world, &piece.picture, piece.x, piece.y);
        }
        scenery.lay_front(&mut world, 0.0, true);
        world
    }

    pub(super) fn map_frame(&mut self, ctx: &egui::Context, full: Rect) {
        let close = !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::M)
                    | i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
            });
        if close {
            self.map = None;
            return;
        }
        if self.map.is_none() {
            let picture = self.campus_picture();
            self.map = Some(texture(ctx, "campus-map", &picture));
        }
        let Some(map) = &self.map else {
            return;
        };
        let campus = PlaceId::Campus.place();
        let (w, h) = (campus.width * TILE, campus.height * TILE);
        let ppp = ctx.pixels_per_point();
        let room = full.shrink(48.0);
        let scale = ((room.width() * ppp) / w as f32)
            .min((room.height() * ppp) / h as f32)
            .floor()
            .max(1.0);
        let size = vec2(w as f32, h as f32) * scale / ppp;
        let rect = Rect::from_center_size(full.center(), size);
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            Id::new("campus-map"),
        ));
        painter.rect_filled(full, 0.0, Color32::from_black_alpha(190));
        painter.rect_filled(rect.expand(4.0), 0.0, Color32::from_rgb(0x2f, 0x1c, 0x12));
        painter.image(
            map.id(),
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let at = |(x, y): (i32, i32)| {
            rect.min
                + vec2(
                    ((x * TILE + TILE / 2) as f32) * scale / ppp,
                    ((y * TILE + TILE / 2) as f32) * scale / ppp,
                )
        };
        let tag = |p: egui::Pos2, words: &str, strong: bool| {
            let fill = if strong {
                Color32::from_rgb(0x1c, 0x41, 0x37)
            } else {
                Color32::from_rgb(0xff, 0xf8, 0xe4)
            };
            let ink = if strong {
                Color32::from_rgb(0xf3, 0xe6, 0xc4)
            } else {
                Color32::from_rgb(0x23, 0x36, 0x2f)
            };
            let galley = painter.layout_no_wrap(words.to_owned(), FontId::monospace(13.0), ink);
            let r = Rect::from_center_size(p, galley.size() + vec2(12.0, 6.0));
            painter.rect_filled(r, 0.0, fill);
            painter.rect_stroke(
                r,
                0.0,
                Stroke::new(1.5, Color32::from_rgb(0x3b, 0x2b, 0x3a)),
                egui::StrokeKind::Inside,
            );
            painter.galley(r.min + vec2(6.0, 3.0), galley, ink);
        };
        for door in &campus.doors {
            tag(at(door.tile), label(door.to), false);
        }
        // The Handler: where they stand in the courtyard, or at the door of where they are.
        let here = self.town.as_ref().and_then(|town| {
            if town.place.id == PlaceId::Campus {
                Some(town.walker.tile)
            } else {
                campus
                    .doors
                    .iter()
                    .find(|d| d.to == town.place.id)
                    .map(|d| d.tile)
            }
        });
        if let Some(tile) = here {
            tag(at(tile) - vec2(0.0, 28.0), "YOU", true);
        }
        painter.text(
            pos2(rect.min.x, rect.max.y + 22.0),
            egui::Align2::LEFT_CENTER,
            "THE CAMPUS   ·   M or Esc: close",
            FontId::monospace(13.0),
            Color32::from_rgb(0xf3, 0xe6, 0xc4),
        );
    }
}
