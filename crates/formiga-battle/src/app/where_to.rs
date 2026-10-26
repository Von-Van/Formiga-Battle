//! The Circuit, as the first page of an outing close to home lays it out: what the Handler is in
//! (the chosen place's cup and how far along it they are), who they face next (the rival Handler
//! coming, with their pennant and the three they bring), what is at stake, and the four cups as
//! stations on one line. The right page keeps what is secondary: the last few matches, the shelf
//! of cups, and the friends played. Everything said is read from the record and the career.

use super::{BattleApp, Step, kept_picture, portrait_mat, rosette, texture};
use crate::chrome::{self, Kind, hand_job, ink};
use crate::keepsake::{self, Keepsake};
use crate::rewards::{self, CUP_WINS, CUPS};
use eframe::egui::{self, Color32, Id, Rect, RichText, Sense, pos2, vec2};
use formiga_battle_contract::Mode;

/// What the Handler calls each cup's place on the shelf.
const SHELF_NAMES: [&str; 4] = ["Backyard", "Woodland", "Millpond", "Cove"];

/// "the Millpond", for the place the cup after `cup` is played on, if there is one.
fn next_place(cup: usize) -> Option<String> {
    let next = CUPS.get(cup + 1)?;
    let arena = formiga_battle_core::Arena::by_id(next.arena)?;
    Some(format!("the {}", arena.name))
}

/// The line under the cup's name: how far along it the Handler is, as they would jot it.
fn progress_line(cup: usize, wins: u8) -> String {
    let name = CUPS[cup].name;
    match wins {
        0 => format!("Three wins here and the {name} is mine!"),
        1 => "One down, two to go!".to_owned(),
        2 => match next_place(cup) {
            Some(next) => {
                format!("Two down, one to go! One more and it's ours, and {next} opens up.")
            }
            None => "Two down, one to go! One more and it's ours.".to_owned(),
        },
        _ => format!("Won it! The {name} is on my shelf, and I can play here any time."),
    }
}

/// What a win here would bring.
fn stake_line(cup: usize, wins: u8) -> String {
    match wins {
        0 | 1 => format!("a win toward the {}", CUPS[cup].name),
        2 => match next_place(cup) {
            Some(next) => format!("the trophy and medal, and {next} opens up!"),
            None => "the trophy and medal!".to_owned(),
        },
        _ => "nothing but bragging rights!".to_owned(),
    }
}

/// How the Handler stands against a rival so far, in a few words.
fn against_line(played: u32, won: u32, lost: u32, short: &str) -> String {
    if played == 0 {
        return "Haven't met them yet!".to_owned();
    }
    if won > lost {
        format!("I lead {won} to {lost}.")
    } else if lost > won {
        format!("{short} leads {lost} to {won}.")
    } else {
        format!("Level at {won} each.")
    }
}

impl BattleApp {
    /// The cup the chosen place plays for, by its place in `CUPS`, and the wins toward it.
    fn chosen_cup(&self) -> Option<(usize, u8)> {
        let cup = CUPS.iter().position(|c| c.arena == self.table.arena.id)?;
        let wins = self.table.record.cup_wins(CUPS[cup].id).min(CUP_WINS);
        Some((cup, wins))
    }

    /// The footer's line on the Circuit page.
    pub(super) fn circuit_hint(&self) -> String {
        match self.chosen_cup() {
            Some((cup, wins)) if wins < CUP_WINS => {
                let nth = ["first", "second", "third"][usize::from(wins)];
                format!("Who's going for the {}'s {nth} win?", CUPS[cup].name)
            }
            _ => "Pick a cup on the line, then turn the page to choose who goes.".to_owned(),
        }
    }

    /// The left page of the Circuit.
    pub(super) fn circuit_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        let Some((cup, wins)) = self.chosen_cup() else {
            return;
        };
        let full = ui.available_rect_before_wrap();
        let gap = u * 10.0;

        // The head of the page, with how the Handler stands pinned beside it.
        let tag_w = u * 150.0;
        let head = ui.allocate_ui_with_layout(
            vec2(full.width() - tag_w - gap, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                chrome::page_heading(
                    ui,
                    &format!("{} · the championship circuit", self.outing.name()),
                    &format!("The {}", CUPS[cup].name),
                    &progress_line(cup, wins),
                );
            },
        );
        let tag = Rect::from_min_size(
            pos2(full.right() - tag_w, full.top() + u * 3.0),
            vec2(tag_w, u * 56.0),
        );
        self.standing_tag(ui, ctx, tag, u);

        // The line of cups along the foot of the page.
        let thumb_w = ((full.width() - gap * 3.0) / 4.0).floor();
        let thumb_h = (thumb_w * 0.6).min(u * 46.0);
        let card_h = thumb_h + u * 46.0;
        let rail_top = full.bottom() - card_h - u * 16.0;
        let middle = Rect::from_min_max(
            pos2(full.left(), head.response.rect.bottom() + gap),
            pos2(full.right(), rail_top - gap),
        );
        let left_w = ((middle.width() - gap) * 1.45 / 2.45).floor();
        let left = Rect::from_min_max(middle.min, pos2(middle.left() + left_w, middle.bottom()));
        let right = Rect::from_min_max(pos2(left.right() + gap, middle.top()), middle.max);
        self.pasted_place(ui, ctx, left, cup, wins, u);
        self.visitors_card(ui, ctx, right, cup, wins, u);
        self.cup_line(
            ui,
            Rect::from_min_max(pos2(full.left(), rail_top), full.max),
            thumb_h,
            u,
        );
        ui.allocate_rect(full, Sense::hover());
    }

    /// The Handler's standing on a card pinned to the corner of the page: their pennant, their
    /// title, how far to the next one, and the cups and points so far.
    fn standing_tag(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, rect: Rect, u: f32) {
        let standing = crate::identity::Standing::of(&self.table.record);
        let painter = ui.painter().clone();
        chrome::card(&painter, rect, u, ink::CARD, ink::LINE.gamma_multiply(0.55));
        let style = self.table.record.handler.as_ref().map_or_else(
            || crate::identity::Style::of(&crate::identity::first_pennant()),
            |h| crate::identity::Style::of(&h.pennant),
        );
        let flag = self.flag(ctx, style);
        let flag_h = (rect.height() - u * 10.0).min(u * 34.0);
        let flag_size = flag.size_vec2() * (flag_h / flag.size_vec2().y);
        painter.image(
            flag.id(),
            Rect::from_min_size(rect.min + vec2(u * 5.0, u * 5.0), flag_size),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let x = rect.left() + u * 10.0 + flag_size.x;
        let mut y = rect.top() + u * 4.0;
        for (job, gap) in [
            (hand_job("Me, so far", chrome::HAND, ink::MUTED), u),
            (
                {
                    let mut job = egui::text::LayoutJob::default();
                    job.append(
                        standing.title,
                        0.0,
                        egui::TextFormat::simple(formiga_pencil::hand_bold(18.0), ink::PAGE),
                    );
                    job
                },
                u * 2.0,
            ),
        ] {
            let galley = painter.layout_job(job);
            let h = galley.size().y;
            painter.galley(pos2(x, y), galley, ink::PAGE);
            y += h + gap;
        }
        // How far to the next title.
        let bar = Rect::from_min_size(pos2(x + u, y), vec2(rect.right() - x - u * 7.0, u * 3.0));
        let at = crate::identity::STANDINGS
            .iter()
            .rev()
            .find(|(need, _)| standing.points >= *need)
            .map_or(0, |(need, _)| *need);
        let filled = standing.next.map_or(1.0, |(_, need)| {
            (standing.points - at) as f32 / (need - at).max(1) as f32
        });
        painter.rect_filled(bar, 0.0, ink::CARD);
        chrome::stepped(&painter, bar.expand(u), u, ink::LINE.gamma_multiply(0.6));
        painter.rect_filled(
            Rect::from_min_size(
                bar.min,
                vec2(bar.width() * filled.clamp(0.0, 1.0), bar.height()),
            ),
            0.0,
            ink::FOREST,
        );
        let line = painter.layout_no_wrap(
            format!(
                "{} of {} cups · {} pts",
                standing.cups.len(),
                CUPS.len(),
                standing.points
            ),
            formiga_pencil::hand_bold(15.0),
            ink::FOREST,
        );
        painter.galley(pos2(x, bar.bottom() + u * 3.0), line, ink::FOREST);
        let hint = match standing.next {
            Some((title, need)) => format!(
                "{} more points and I'm a {title}! Points come from playing friends.",
                need - standing.points
            ),
            None => "As high as the Open Circuit goes!".to_owned(),
        };
        ui.interact(rect, Id::new("standing-tag"), Sense::hover())
            .on_hover_text(hint);
    }

    /// The chosen place's picture pasted onto the page, with its name and what it is like under
    /// it, and the cup's trophy on its plinth in the corner.
    fn pasted_place(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        area: Rect,
        cup: usize,
        wins: u8,
        u: f32,
    ) {
        let garden = super::garden_for(&mut self.garden, &mut self.view, &self.table.arena);
        let key = format!("preview-{}", self.table.arena.id);
        let fresh = !matches!(&self.preview, Some((k, _)) if *k == key);
        let picture = if fresh { Some(garden.picture()) } else { None };
        let (w, h) = (garden.ground.width() as f32, garden.ground.height() as f32);
        let blurb = crate::garden::Dressing::of(&self.table.arena).blurb();
        let texture = match (picture, &self.preview) {
            (Some(picture), _) => {
                let t = texture(ctx, &key, &picture);
                self.preview = Some((key, t.clone()));
                t
            }
            (None, Some((_, t))) => t.clone(),
            (None, None) => return,
        };
        let painter = ui.painter().clone();
        let caption = painter.layout(
            String::new(),
            egui::FontId::proportional(15.0),
            ink::PAGE,
            area.width(),
        );
        let caption_h = caption.size().y * 2.0 + u * 8.0;
        let pad = u * 8.0;
        let room = Rect::from_min_max(
            area.min + vec2(pad, pad),
            pos2(area.right() - pad, area.bottom() - caption_h - pad),
        );
        let ppp = ctx.pixels_per_point();
        let pixels = ((room.width() * ppp) / w)
            .min((room.height() * ppp) / h)
            .floor()
            .min(4.0);
        let size = if pixels >= 1.0 {
            vec2(w, h) * pixels / ppp
        } else {
            vec2(w, h) * (room.width() / w).min(room.height() / h).max(0.1)
        };
        let min = pos2(room.center().x - size.x / 2.0, room.top());
        let photo = Rect::from_min_size(
            pos2((min.x * ppp).round() / ppp, (min.y * ppp).round() / ppp),
            size,
        );
        chrome::taped_mat(&painter, photo, u);
        painter.image(
            texture.id(),
            photo,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        // The trophy on its plinth, in the corner: what a win here earns, or what was won.
        let plinth = Rect::from_min_size(
            pos2(area.right() - u * 62.0, photo.bottom() - u * 40.0),
            vec2(u * 60.0, u * 70.0),
        );
        let trophy = kept_picture(ctx, &format!("trophy-{cup}"), || {
            keepsake::icon(Keepsake::Trophy(cup))
        });
        let won = wins >= CUP_WINS;
        chrome::plinth(
            &painter,
            plinth,
            &trophy,
            if won { "On my shelf" } else { "The prize!" },
            u,
        );
        let mut job = egui::text::LayoutJob::default();
        job.append(
            &self.table.arena.name,
            0.0,
            egui::TextFormat::simple(formiga_pencil::hand_bold(15.0), ink::PAGE),
        );
        job.append(
            &format!(" · {blurb}"),
            0.0,
            egui::TextFormat::simple(egui::FontId::proportional(15.0), ink::PAGE),
        );
        job.wrap.max_width = (plinth.left() - photo.left() - u * 6.0).max(u * 60.0);
        let galley = painter.layout_job(job);
        painter.galley(
            pos2(photo.left() - u * 4.0, photo.bottom() + u * 9.0),
            galley,
            ink::PAGE,
        );
    }

    /// Who comes next: the rival Handler, their pennant and the three they bring, what a win
    /// here would bring, and the way on.
    fn visitors_card(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        area: Rect,
        cup: usize,
        wins: u8,
        u: f32,
    ) {
        let painter = ui.painter().clone();
        chrome::card(&painter, area, u, ink::CARD, ink::LINE.gamma_multiply(0.55));
        let (rival, three) = self.table.next_visitors();
        let before = self.table.career.against(rival.id);
        let faces: Vec<egui::TextureHandle> = three
            .iter()
            .filter_map(|i| self.table.away.get(*i).map(|(c, _)| c.clone()))
            .collect::<Vec<_>>()
            .iter()
            .map(|c| self.portrait(ctx, c))
            .collect();
        let flag = self.flag(ctx, crate::identity::Style::of(&rival.pennant()));
        let inner = area.shrink2(vec2(u * 9.0, u * 7.0));
        let mut card = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let ui = &mut card;
        ui.spacing_mut().item_spacing.y = u * 2.0;
        let kicker = if wins < CUP_WINS {
            format!("Next up · win {} of {CUP_WINS}", wins + 1)
        } else {
            "Next up · just for fun".to_owned()
        };
        ui.label(hand_job(&kicker, chrome::HAND, ink::FOREST));
        ui.label(
            RichText::new(format!("vs. {}!", rival.short))
                .font(formiga_pencil::hand_bold(23.0))
                .color(ink::PAGE),
        );
        ui.label(
            RichText::new(format!(
                "{} of the {}. {}",
                rival.name,
                rival.club,
                against_line(before.played, before.won, before.lost, rival.short)
            ))
            .size(15.0)
            .color(ink::MUTED),
        )
        .on_hover_text(rival.tag());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = u * 4.0;
            let size = flag.size_vec2() * u;
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            ui.painter().image(
                flag.id(),
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            let mat = ((ui.available_width() - u * 8.0) / 3.0).min(u * 30.0);
            for face in &faces {
                portrait_mat(ui, face, mat, u, true);
            }
        });
        dotted(ui, u);
        ui.label(hand_job("At stake", chrome::HAND, ink::MUTED));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = u;
            for k in 0..CUP_WINS {
                rosette(ui, cup, k < wins, u);
            }
            ui.add_space(u * 3.0);
            ui.label(
                RichText::new(stake_line(cup, wins))
                    .size(15.0)
                    .color(ink::PAGE),
            );
        });
        // The way on, at the foot of the card.
        let width = ui.available_width();
        let button_h = chrome::button_size(ui, "Who's going? ›", Kind::Primary, u).y;
        let space = ui.available_rect_before_wrap().bottom() - ui.cursor().top() - button_h;
        ui.add_space(space.max(u * 2.0));
        let now = self.clock;
        if chrome::button(
            ui,
            "Who's going? ›",
            Kind::Primary,
            false,
            true,
            Some(width),
            u,
        )
        .on_hover_text("Next: I pick my three (Enter)")
        .clicked()
        {
            self.turn_to(Step::Who, now);
        }
    }

    /// The four cups as stations on one line: won, in progress, open, or still locked behind the
    /// one before. A station that is open can be chosen.
    fn cup_line(&mut self, ui: &mut egui::Ui, area: Rect, thumb_h: f32, u: f32) {
        let painter = ui.painter().clone();
        let won = CUPS
            .iter()
            .filter(|c| self.table.record.cup_wins(c.id) >= CUP_WINS)
            .count();
        let head = painter.layout_job(hand_job("The Circuit", chrome::HAND, ink::FOREST));
        let head_h = head.size().y;
        painter.galley(area.min, head, ink::FOREST);
        let count = painter.layout_job(hand_job(
            &format!("{won} of {} cups won", CUPS.len()),
            chrome::HAND,
            ink::MUTED,
        ));
        painter.galley(
            pos2(area.right() - count.size().x, area.top()),
            count,
            ink::MUTED,
        );
        let top = area.top() + head_h + u * 8.0;
        let gap = u * 10.0;
        let width = ((area.width() - gap * 3.0) / 4.0).floor();
        let height = area.bottom() - top - u * 2.0;
        // The rails the stations stand on.
        let rail = Rect::from_min_max(
            pos2(area.left() - u * 4.0, top + thumb_h * 0.45),
            pos2(area.right() + u * 4.0, top + thumb_h * 0.45 + u * 5.0),
        );
        painter.hline(
            rail.x_range(),
            rail.top(),
            egui::Stroke::new(u, Color32::from_rgb(0xb2, 0x95, 0x70)),
        );
        painter.hline(
            rail.x_range(),
            rail.bottom(),
            egui::Stroke::new(u, Color32::from_rgb(0xb2, 0x95, 0x70)),
        );
        let mut x = rail.left();
        while x < rail.right() {
            painter.vline(
                x,
                rail.y_range(),
                egui::Stroke::new(u, Color32::from_rgb(0xc9, 0xb7, 0x8e)),
            );
            x += u * 6.0;
        }
        let ctx = ui.ctx().clone();
        let mut chosen = None;
        for (i, cup) in CUPS.iter().enumerate() {
            let rect = Rect::from_min_size(
                pos2(area.left() + i as f32 * (width + gap), top),
                vec2(width, height),
            );
            let open = rewards::is_open(cup, &self.table.record);
            let wins = self.table.record.cup_wins(cup.id).min(CUP_WINS);
            let here = self.table.arena.id == cup.arena;
            let response = ui.interact(rect, Id::new(("cup-station", i)), Sense::click());
            let lift = if open && !here && response.hovered() && !crate::feel::still(&ctx) {
                u
            } else {
                0.0
            };
            let face = rect.translate(vec2(0.0, -lift));
            let fill = if here {
                ink::MINT
            } else if response.hovered() && open {
                Color32::from_rgb(214, 238, 218)
            } else {
                ink::CARD
            };
            let fade = if open { 1.0 } else { 0.62 };
            chrome::card(
                &painter,
                face,
                u,
                fill.gamma_multiply(fade),
                if here {
                    ink::FOREST
                } else {
                    ink::LINE.gamma_multiply(0.55 * fade)
                },
            );
            let thumb =
                self.arenas
                    .iter_mut()
                    .find(|(a, _)| a.id == cup.arena)
                    .map(|(arena, thumb)| {
                        thumb
                            .get_or_insert_with(|| {
                                let view = crate::iso::View::new(arena.width, arena.height);
                                let garden = crate::garden::Garden::new(arena, &view);
                                texture(
                                    &ctx,
                                    &format!("arena-{}", arena.id),
                                    &super::shrunk(&garden.picture(), 3),
                                )
                            })
                            .clone()
                    });
            let inner = face.shrink(u * 4.0);
            if let Some(thumb) = thumb {
                let size = thumb.size_vec2();
                let fit = (inner.width() / size.x).min(thumb_h / size.y);
                let drawn = size * fit;
                painter.image(
                    thumb.id(),
                    Rect::from_min_size(pos2(inner.center().x - drawn.x / 2.0, inner.top()), drawn),
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE.gamma_multiply(fade),
                );
            }
            let name = painter.layout_no_wrap(
                cup.name.to_owned(),
                formiga_pencil::hand_bold(15.0),
                ink::PAGE.gamma_multiply(fade),
            );
            let name_y = inner.top() + thumb_h + u * 3.0;
            painter.galley(pos2(inner.left(), name_y), name, ink::PAGE);
            let note_y = name_y + u * 13.0;
            if open {
                let mut rx = inner.left();
                for k in 0..CUP_WINS {
                    let picture =
                        kept_picture(&ctx, &format!("small-rosette-{i}-{}", k < wins), || {
                            keepsake::small_rosette(i, k < wins)
                        });
                    let size = picture.size_vec2() * u;
                    painter.image(
                        picture.id(),
                        Rect::from_min_size(pos2(rx, note_y), size),
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    rx += size.x + u;
                }
                if (1..CUP_WINS).contains(&wins) {
                    let note = painter.layout_job(hand_job(
                        &format!("{wins} of {CUP_WINS}"),
                        chrome::HAND,
                        ink::MUTED,
                    ));
                    painter.galley(pos2(rx + u * 3.0, note_y), note, ink::MUTED);
                }
            } else {
                let note = painter.layout_job(hand_job("Locked", chrome::HAND, ink::MUTED));
                painter.galley(pos2(inner.left(), note_y), note, ink::MUTED);
            }
            if wins >= CUP_WINS {
                chrome::stamp(
                    &painter,
                    pos2(face.right() - u * 18.0, face.top() + u * 12.0),
                    "Won",
                    chrome::HAND,
                    ink::FOREST,
                    Some(ink::CARD.gamma_multiply(0.9)),
                    (-7f32).to_radians(),
                    u,
                );
            } else if here && wins > 0 {
                let tab = painter.layout_job(hand_job("In progress", chrome::HAND, ink::COVER));
                let tab_rect = Rect::from_center_size(
                    pos2(face.center().x, face.top()),
                    tab.size() + vec2(u * 8.0, u * 2.0),
                );
                painter.rect_filled(tab_rect, 0.0, ink::BRASS);
                painter.rect_filled(tab_rect.shrink(u), 0.0, ink::FOREST);
                painter.galley(tab_rect.center() - tab.size() / 2.0, tab, ink::COVER);
            }
            let response = if open {
                response.on_hover_text(if here {
                    format!("The {}: where I'm playing next", cup.name)
                } else {
                    format!("Play for the {} instead", cup.name)
                })
            } else {
                let before = CUPS[i.saturating_sub(1)].name;
                response.on_hover_text(format!("Gotta win the {before} first!"))
            };
            if open && !here {
                if response.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if response.clicked() {
                    chosen = formiga_battle_core::Arena::by_id(cup.arena);
                }
            } else if !open && response.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::NotAllowed);
            }
        }
        if let Some(arena) = chosen {
            self.choose_arena(arena);
        }
    }

    /// The ruled right page of the Circuit: the last few matches, the shelf of cups, and the
    /// friends played, folded away until opened.
    pub(super) fn circuit_notes(&mut self, ui: &mut egui::Ui, u: f32) {
        let ctx = ui.ctx().clone();
        chrome::kicker(ui, "Lately");
        let lately: Vec<_> = self
            .table
            .record
            .history
            .iter()
            .rev()
            .take(3)
            .cloned()
            .collect();
        if lately.is_empty() {
            ui.label(
                RichText::new("No matches yet! Let's hop on the train and play one.")
                    .size(15.0)
                    .color(ink::MUTED),
            );
        }
        for summary in &lately {
            ui.horizontal(|ui| {
                let (word, colour) = match summary.won {
                    Some(true) => ("Won", ink::FOREST),
                    Some(false) => ("Lost", ink::STAMP),
                    None => ("Drawn", ink::MUTED),
                };
                let (rect, _) = ui.allocate_exact_size(vec2(u * 32.0, u * 22.0), Sense::hover());
                chrome::stamp(
                    ui.painter(),
                    rect.center(),
                    word,
                    chrome::HAND,
                    colour,
                    None,
                    (-3f32).to_radians(),
                    u,
                );
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let against = match (&summary.mode, &summary.opponent, summary.won) {
                        (Mode::Friend, Some(name), Some(true)) => format!("Beat {name}!"),
                        (Mode::Friend, Some(name), Some(false)) => format!("Lost to {name}"),
                        (Mode::Friend, Some(name), None) => format!("Drew with {name}"),
                        (Mode::Friend, None, _) => "A friend match".to_owned(),
                        (Mode::Ai, _, Some(true)) => "Beat the visitors!".to_owned(),
                        (Mode::Ai, _, Some(false)) => "Lost to the visitors".to_owned(),
                        (Mode::Ai, _, None) => "Drew with the visitors".to_owned(),
                    };
                    ui.add(
                        egui::Label::new(RichText::new(against).size(15.0).color(ink::PAGE))
                            .truncate(),
                    );
                    let how = match summary.ended_by.as_str() {
                        "capture" => format!("lit the beacon in {} turns", summary.turns),
                        "knockout" => format!("everyone sat out by turn {}", summary.turns),
                        "turn_cap" => format!("on the tally, {} turns", summary.turns),
                        "forfeit" => "by forfeit".to_owned(),
                        _ => format!("{} turns", summary.turns),
                    };
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!(
                                "{} · {how}",
                                super::handler::date(summary.finished_at_utc)
                            ))
                            .size(15.0)
                            .color(ink::MUTED),
                        )
                        .truncate(),
                    );
                });
            });
        }
        ui.add_space(u * 8.0);
        chrome::kicker(ui, "My shelf");
        ui.horizontal(|ui| {
            let gap = u * 4.0;
            ui.spacing_mut().item_spacing.x = gap;
            let slot_w = ((ui.available_width() - gap * 3.0) / 4.0).floor();
            for (i, cup) in CUPS.iter().enumerate() {
                let won = self.table.record.cup_wins(cup.id) >= CUP_WINS;
                ui.allocate_ui_with_layout(
                    vec2(slot_w, u * 56.0),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        let (slot, _) =
                            ui.allocate_exact_size(vec2(u * 30.0, u * 36.0), Sense::hover());
                        let painter = ui.painter();
                        if won {
                            painter.rect_filled(slot, 0.0, ink::BRASS_EDGE);
                            painter.rect_filled(slot.shrink(u), 0.0, ink::BRASS);
                            painter.rect_filled(
                                slot.shrink(u * 2.0),
                                0.0,
                                Color32::from_rgb(0xf6, 0xe7, 0xbd),
                            );
                            let trophy = kept_picture(&ctx, &format!("trophy-{i}"), || {
                                keepsake::icon(Keepsake::Trophy(i))
                            });
                            let size = trophy.size_vec2() * u;
                            painter.image(
                                trophy.id(),
                                Rect::from_center_size(slot.center(), size),
                                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        } else {
                            dashed_box(painter, slot, u);
                            painter.text(
                                slot.center(),
                                egui::Align2::CENTER_CENTER,
                                "?",
                                formiga_pencil::hand_bold(20.0),
                                ink::MUTED,
                            );
                        }
                        ui.label(
                            RichText::new(SHELF_NAMES[i])
                                .font(formiga_pencil::hand_bold(15.0))
                                .color(ink::MUTED),
                        );
                    },
                )
                .response
                .on_hover_text(if won {
                    format!("The {}, won!", cup.name)
                } else {
                    format!("A space for the {}", cup.name)
                });
            }
        });
        ui.add_space(u * 8.0);
        chrome::kicker(ui, "Out and about");
        let mut friends: Vec<(String, u32, u32)> = Vec::new();
        for summary in &self.table.record.history {
            if let (Mode::Friend, Some(name)) = (&summary.mode, &summary.opponent) {
                let at = match friends.iter().position(|(n, _, _)| n == name) {
                    Some(at) => at,
                    None => {
                        friends.push((name.clone(), 0, 0));
                        friends.len() - 1
                    }
                };
                friends[at].1 += 1;
                friends[at].2 += u32::from(summary.won == Some(true));
            }
        }
        let open = self.friends_open;
        let row = ui
            .horizontal(|ui| {
                ui.label(
                    RichText::new("Friends I've played")
                        .size(15.0)
                        .color(ink::PAGE),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(hand_job(
                        &format!("{} {}", friends.len(), if open { "‹" } else { "›" }),
                        chrome::HAND,
                        ink::FOREST,
                    ));
                });
            })
            .response
            .interact(Sense::click());
        if row.hovered() && !friends.is_empty() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if row.clicked() {
            self.friends_open = !open;
        }
        if open {
            for (name, played, won) in &friends {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).size(15.0).color(ink::PAGE));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(crate::identity::tally_words(*played, *won, 0))
                                .size(15.0)
                                .color(ink::MUTED),
                        );
                    });
                });
            }
        }
        let points = self.table.record.circuit_points;
        ui.label(
            RichText::new(if points == 0 {
                "No Open Circuit points yet. Friend matches are where they come from!".to_owned()
            } else {
                format!("{points} Open Circuit points so far, all from friend matches.")
            })
            .size(15.0)
            .color(ink::MUTED),
        );
    }
}

/// A dotted line across the room left, between parts of a card.
fn dotted(ui: &mut egui::Ui, u: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), u * 3.0), Sense::hover());
    let mut x = rect.left();
    while x < rect.right() {
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(x, rect.center().y), vec2(u * 3.0, u)),
            0.0,
            Color32::from_rgb(0xd8, 0xc8, 0xa0),
        );
        x += u * 5.0;
    }
}

/// A dashed box: the space on the shelf for a cup not yet won.
fn dashed_box(painter: &egui::Painter, rect: Rect, u: f32) {
    let colour = ink::LINE.gamma_multiply(0.55);
    let dash = |from: egui::Pos2, to: egui::Pos2| {
        let length = from.distance(to);
        let along = (to - from) / length;
        let mut at = 0.0;
        while at < length {
            let end = (at + u * 3.0).min(length);
            painter.line_segment(
                [from + along * at, from + along * end],
                egui::Stroke::new(u, colour),
            );
            at += u * 5.0;
        }
    };
    dash(rect.left_top(), rect.right_top());
    dash(rect.left_bottom(), rect.right_bottom());
    dash(rect.left_top(), rect.left_bottom());
    dash(rect.right_top(), rect.right_bottom());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cup_line_says_how_far_along_it_the_handler_is() {
        assert_eq!(
            progress_line(0, 0),
            "Three wins here and the Backyard Cup is mine!"
        );
        assert_eq!(
            progress_line(1, 2),
            "Two down, one to go! One more and it's ours, and the Millpond opens up."
        );
        assert_eq!(
            progress_line(3, 2),
            "Two down, one to go! One more and it's ours."
        );
        assert!(progress_line(2, 3).starts_with("Won it!"));
        assert_eq!(
            stake_line(1, 2),
            "the trophy and medal, and the Millpond opens up!"
        );
    }

    #[test]
    fn standing_against_a_rival_is_said_from_their_record() {
        assert_eq!(against_line(0, 0, 0, "Hazel"), "Haven't met them yet!");
        assert_eq!(against_line(3, 2, 1, "Hazel"), "I lead 2 to 1.");
        assert_eq!(against_line(3, 1, 2, "Hazel"), "Hazel leads 2 to 1.");
        assert_eq!(against_line(2, 1, 1, "Hazel"), "Level at 1 each.");
    }
}
