//! The welcome card, for Battle opened by itself rather than by Formiga: a card over the
//! Battleground saying where a player's own companions come from, with the way to look around
//! with the sample colony meanwhile, or to close.

use super::{BattleApp, Kind, dashed_rule, registry, slip_ribbon};
use crate::chrome::{self, hand_job, ink};
use eframe::egui::{self, Color32, Id, LayerId, Order, Rect, RichText, Sense, pos2, vec2};

/// What the ribbon across the card's top says.
const RIBBON: &str = "Welcome to Formiga Battle";
pub(super) const TITLE: &str = "Your companions come from Formiga";
pub(super) const LOOK_AROUND: &str = "Look around with the sample colony";
pub(super) const CLOSE: &str = "Close";
const WEBSITE: &str = "https://www.formigaworld.com";
/// How wide the card is, in points, where the window has the room.
const WIDTH: f32 = 470.0;

/// Where the Formiga icon is kept, as Formiga's own tour of itself says it.
fn where_the_icon_is() -> &'static str {
    if cfg!(target_os = "windows") {
        "in the notification area"
    } else {
        "in the menu bar"
    }
}

/// How a player's own colony comes to Battle, in Formiga's own words for its menu.
pub(super) fn how_to_open() -> String {
    format!(
        "Formiga Battle plays with your own colony, when it comes by train from Formiga. Open \
         the Formiga icon {} and choose “Play Formiga Battle…”.",
        where_the_icon_is()
    )
}

impl BattleApp {
    /// The welcome card over the whole window, which takes every click until it is answered.
    /// Looking around puts it away and leaves the rehearsal as it was; closing closes Battle.
    pub(super) fn welcome_card(&mut self, ctx: &egui::Context, full: Rect, u: f32) {
        let mut look = !ctx.egui_wants_keyboard_input()
            && ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape));
        let mut close = false;
        let pad = u * 12.0;
        let width = WIDTH.min(full.width() - u * 32.0);
        // As tall as it was last frame, so it sits in the middle once it knows its height.
        let tall = Id::new("welcome-height");
        let height = ctx.data(|d| d.get_temp::<f32>(tall)).unwrap_or(300.0);
        let card = Rect::from_center_size(full.center(), vec2(width, height));
        let card = Rect::from_min_size(
            pos2((card.min.x / u).round() * u, (card.min.y / u).round() * u),
            card.size(),
        );
        let painter = ctx.layer_painter(LayerId::new(Order::Middle, Id::new("welcome-under")));
        let lettering = painter.layout_job(hand_job(RIBBON, 13.0, registry::ON_RIBBON));
        let ribbon_size = lettering.size() + vec2(u * 22.0, u * 9.0);
        let ribbon = Rect::from_min_size(
            pos2(
                ((card.center().x - ribbon_size.x / 2.0) / u).round() * u,
                ((card.min.y - ribbon_size.y / 2.0) / u).round() * u,
            ),
            ribbon_size,
        );
        let mut bottom = card.max.y;
        egui::Area::new(Id::new("welcome"))
            .order(Order::Foreground)
            .fixed_pos(full.min)
            .show(ctx, |ui| {
                // The whole window, so nothing under the card answers while it is up.
                ui.allocate_exact_size(full.size(), Sense::click());
                let words = Rect::from_min_size(
                    pos2(card.min.x + pad, ribbon.max.y + u * 6.0),
                    vec2(width - pad * 2.0, full.height()),
                );
                let shown = ui.scope_builder(egui::UiBuilder::new().max_rect(words), |ui| {
                    ui.set_width(words.width());
                    ui.label(RichText::new(TITLE).size(22.0).color(ink::PAGE).strong());
                    ui.add_space(u * 3.0);
                    ui.label(RichText::new(how_to_open()).size(16.0).color(ink::PAGE));
                    ui.add_space(u * 2.0);
                    ui.label(
                        RichText::new(
                            "Meanwhile the sample colony is here to look around with. A look \
                             around keeps nothing.",
                        )
                        .size(15.0)
                        .color(ink::MUTED),
                    );
                    ui.add_space(u * 4.0);
                    dashed_rule(ui, u);
                    ui.add_space(u * 3.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label(
                            RichText::new("Don't have Formiga yet? It's at")
                                .size(15.0)
                                .color(ink::MUTED),
                        );
                        ui.hyperlink_to(
                            RichText::new("formigaworld.com")
                                .size(15.0)
                                .color(ink::FOREST),
                            WEBSITE,
                        );
                    });
                    ui.add_space(u * 8.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = u * 4.0;
                        if chrome::button(ui, LOOK_AROUND, Kind::Primary, false, true, None, u)
                            .clicked()
                        {
                            look = true;
                        }
                        if chrome::button(ui, CLOSE, Kind::Back, false, true, None, u).clicked() {
                            close = true;
                        }
                    });
                });
                bottom = shown.response.rect.max.y + pad;
            });
        ctx.data_mut(|d| d.insert_temp(tall, (bottom - card.min.y).round()));
        // Under the words: the window dimmed, the card, and its ribbon.
        painter.rect_filled(full, 0.0, Color32::from_black_alpha(120));
        let card = Rect::from_min_max(card.min, pos2(card.max.x, bottom));
        chrome::card(&painter, card, u, ink::CARD, ink::LINE.gamma_multiply(0.55));
        slip_ribbon(&painter, ribbon, None, u);
        painter.galley(
            ribbon.center() - lettering.size() / 2.0,
            lettering,
            registry::ON_RIBBON,
        );
        if close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if look {
            self.welcome = false;
        }
    }
}
