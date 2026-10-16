//! The Handler's registry, the first of their own pages: who they are (name, club and pennant)
//! and how they stand, beside the trios they have named. The three picked on the Who page can be
//! named as a trio there, or take a place in a trio that has the other two. Everything here is
//! kept by Desktop with the colony's Battle record, and none of it changes how a match is played.

use super::{BattleApp, Kind, card, dashed_rule, portrait_mat, rosette, sticky, texture};
use crate::chrome::{self, ink, label_job};
use crate::identity::{self, FIELDS, SHAPES, SYMBOLS, Standing, Style, TRIMS};
use eframe::egui::{self, FontId, Id, Rect, RichText, Sense, pos2, vec2};
use formiga_battle_contract::{HandlerCard, Trio, TrioCard, lineup_of};

/// A trio being renamed: its id, and the name as typed so far.
pub struct Renaming {
    pub id: String,
    pub name: String,
}

impl BattleApp {
    /// A pennant's picture, painted once for each style.
    pub(super) fn flag(&mut self, ctx: &egui::Context, style: Style) -> egui::TextureHandle {
        self.flags
            .entry(style)
            .or_insert_with(|| {
                let key = format!(
                    "flag-{}-{}-{}-{}",
                    style.shape, style.field, style.symbol, style.trim
                );
                texture(ctx, &key, &crate::pennant::flag(style))
            })
            .clone()
    }

    /// The pennant the Handler flies: their card's, or the one being chosen before they sign.
    pub(super) fn handler_style(&self) -> Style {
        self.table
            .record
            .handler
            .as_ref()
            .map_or(self.draft_style, |h| Style::of(&h.pennant))
    }

    /// The pennant a trio flies: its own, or the Handler's.
    pub(super) fn trio_style(&self, trio: &Trio) -> Style {
        trio.pennant
            .as_ref()
            .map_or_else(|| self.handler_style(), Style::of)
    }

    /// The Handler card as it stands, with the drafts of its name and club.
    fn keep_card(&mut self) {
        let Some(mut card) = identity::card_with_name(&self.table.record, &self.handler) else {
            return;
        };
        let club = formiga_expansion_rulebook::sanitize_text(&self.club, identity::CLUB_CHARS);
        card.club = (!club.is_empty()).then_some(club);
        if self.table.record.handler.is_none() {
            card.pennant = self.draft_style.pennant();
        }
        if self.table.record.handler.as_ref() != Some(&card) {
            self.table.set_handler(card);
        }
    }

    /// The left page of the registry: the Handler's name and club, and their pennant.
    pub(super) fn handler_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        let since = self.turned;
        let mut changed = false;
        crate::feel::dropping(ui, since, 0, u, |ui| {
            card(ui, Id::new("handler-card"), false, true, false, u, |ui| {
                for (label, hint, limit, club) in [
                    (
                        "Your name",
                        "The name your friends know you by",
                        identity::NAME_CHARS,
                        false,
                    ),
                    (
                        "Your club",
                        "A club or team name, if you like",
                        identity::CLUB_CHARS,
                        true,
                    ),
                ] {
                    ui.label(label_job(label, chrome::LABEL, ink::FOREST));
                    let text = if club {
                        &mut self.club
                    } else {
                        &mut self.handler
                    };
                    let field = ui.add(
                        egui::TextEdit::singleline(text)
                            .font(FontId::proportional(18.0))
                            .char_limit(limit)
                            .frame(egui::Frame::NONE)
                            .hint_text(hint)
                            .desired_width(ui.available_width().min(360.0)),
                    );
                    let r = field.rect;
                    ui.painter().rect_filled(
                        Rect::from_min_size(pos2(r.left(), r.bottom() + u), vec2(r.width(), u)),
                        0.0,
                        if field.has_focus() {
                            ink::FOREST
                        } else {
                            ink::LINE.gamma_multiply(0.5)
                        },
                    );
                    changed |= field.lost_focus();
                    ui.add_space(u * 4.0);
                }
            });
        });
        if changed {
            self.keep_card();
        }
        ui.add_space(u * 4.0);
        let style = self.handler_style();
        let flag = self.flag(ctx, style);
        let mut chosen = style;
        crate::feel::dropping(ui, since, 1, u, |ui| {
            card(
                ui,
                Id::new("handler-pennant"),
                false,
                true,
                false,
                u,
                |ui| {
                    ui.horizontal_top(|ui| {
                        big_flag(ui, &flag, u);
                        ui.add_space(u * 6.0);
                        ui.vertical(|ui| {
                            ui.label(label_job("Your pennant", chrome::LABEL, ink::FOREST));
                            ui.add_space(u * 2.0);
                            stepper(ui, "Cut", &mut chosen.shape, &SHAPES.map(|s| s.1), u);
                            stepper(ui, "Colour", &mut chosen.field, &FIELDS.map(|f| f.1), u);
                            stepper(ui, "Symbol", &mut chosen.symbol, &SYMBOLS.map(|s| s.1), u);
                            stepper(ui, "Trim", &mut chosen.trim, &TRIMS.map(|t| t.1), u);
                        });
                    });
                    ui.add_space(u * 2.0);
                    ui.label(
                    RichText::new(format!(
                        "It flies beside your team in every match, and your friends see it. {}.",
                        capitalised(&style.words())
                    ))
                    .size(12.0)
                    .color(ink::MUTED),
                );
                },
            );
        });
        if chosen != style || changed {
            // The station's picture flies the pennant as it now is.
            self.masthead = None;
            self.masthead_picture = None;
        }
        if chosen != style {
            self.draft_style = chosen;
            match self.table.record.handler.clone() {
                Some(card) => self.table.set_handler(HandlerCard {
                    pennant: chosen.pennant(),
                    ..card
                }),
                None => self.keep_card(),
            }
        }
        if self.table.record.handler.is_none() {
            ui.add_space(u * 3.0);
            ui.label(
                RichText::new(
                    "Sign your name and your notebook keeps all of this with your colony.",
                )
                .size(12.5)
                .italics()
                .color(ink::MUTED),
            );
        }
        if let Some(notice) = self.table.notice.clone() {
            ui.add_space(u * 3.0);
            sticky(ui, &notice, u);
        }
    }

    /// The right page of the registry: how the Handler stands, and their trios.
    pub(super) fn standing_and_trios(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        let standing = Standing::of(&self.table.record);
        chrome::kicker(ui, "Standing");
        ui.add_space(u);
        ui.label(
            RichText::new(standing.title)
                .size(20.0)
                .strong()
                .color(ink::FOREST),
        );
        ui.label(
            RichText::new(format!(
                "{} Open Circuit points · {}",
                standing.points,
                standing.circuit()
            ))
            .size(12.5)
            .color(ink::PAGE),
        );
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = u;
            for cup in &crate::rewards::CUPS {
                rosette(ui, standing.cups.iter().any(|c| c.id == cup.id), u);
            }
        });
        ui.label(RichText::new(standing.how()).size(11.5).color(ink::MUTED));
        ui.add_space(u * 4.0);
        dashed_rule(ui, u);
        ui.add_space(u * 2.0);
        chrome::kicker(ui, "Your trios");
        ui.add_space(u * 2.0);
        let trios: Vec<Trio> = self.table.record.trios.iter().rev().cloned().collect();
        if trios.is_empty() {
            ui.label(
                RichText::new(
                    "None yet. Pick three on the Who page and name them: a trio keeps its own record, and its name and pennant go with it into every match.",
                )
                .size(12.5)
                .italics()
                .color(ink::MUTED),
            );
            return;
        }
        let since = self.turned;
        for (i, trio) in trios.iter().enumerate() {
            crate::feel::dropping(ui, since, (i + 2).min(9), u, |ui| {
                self.trio_card(ui, ctx, trio, u);
            });
            ui.add_space(u * 3.0);
        }
    }

    /// One trio on the registry's page: its pennant, name, three and record, and what can be done
    /// with it.
    fn trio_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, trio: &Trio, u: f32) {
        let flag = self.flag(ctx, self.trio_style(trio));
        let faces: Vec<Option<egui::TextureHandle>> = trio
            .members
            .iter()
            .map(|id| {
                let found = self
                    .table
                    .home
                    .iter()
                    .find(|(c, _)| c.traveler.id.0 == *id)
                    .map(|(c, _)| c.clone());
                found.map(|c| self.portrait(ctx, &c))
            })
            .collect();
        let earlier = self.lineup_words(trio);
        let mut draft = self
            .renaming
            .as_ref()
            .filter(|r| r.id == trio.id)
            .map(|r| r.name.clone());
        let renaming = draft.is_some();
        let handler_style = self.handler_style();
        let may_return = identity::can_name(&self.table.record).is_ok();
        let mut update: Option<TrioCard> = None;
        let mut rename_to: Option<Option<Renaming>> = None;
        card(
            ui,
            Id::new(("trio", &trio.id)),
            false,
            !trio.retired,
            false,
            u,
            |ui| {
                ui.horizontal(|ui| {
                    small_flag(ui, &flag, u);
                    ui.add_space(u * 2.0);
                    if let Some(name) = &mut draft {
                        let edit = ui.add(
                            egui::TextEdit::singleline(name)
                                .font(FontId::proportional(15.0))
                                .char_limit(identity::TRIO_CHARS)
                                .desired_width(170.0),
                        );
                        let enter =
                            edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if chrome::button(ui, "keep", Kind::Quiet, false, true, None, u).clicked()
                            || enter
                        {
                            if let Some(name) = identity::trio_name(name) {
                                update = Some(TrioCard {
                                    name,
                                    ..trio.card()
                                });
                            }
                            rename_to = Some(None);
                        }
                    } else {
                        ui.label(
                            RichText::new(&trio.name)
                                .size(15.0)
                                .strong()
                                .color(ink::PAGE),
                        );
                        if trio.retired {
                            ui.label(label_job("Retired", 9.0, ink::MUTED));
                        }
                    }
                });
                ui.horizontal(|ui| {
                    for face in &faces {
                        match face {
                            Some(face) => portrait_mat(ui, face, 32.0, u, !trio.retired),
                            None => {
                                let (rect, _) =
                                    ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                                chrome::stepped(
                                    ui.painter(),
                                    rect,
                                    u,
                                    ink::LINE.gamma_multiply(0.3),
                                );
                            }
                        }
                    }
                    ui.add_space(u * 2.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(identity::record_words(trio))
                                .size(12.5)
                                .color(ink::PAGE),
                        );
                        let split = format!(
                            "Friends {} · Visitors {}",
                            identity::tally_words(
                                trio.friends.played,
                                trio.friends.won,
                                trio.friends.drawn
                            )
                            .to_lowercase(),
                            identity::tally_words(
                                trio.visitors.played,
                                trio.visitors.won,
                                trio.visitors.drawn
                            )
                            .to_lowercase(),
                        );
                        ui.label(RichText::new(split).size(11.0).color(ink::MUTED));
                    });
                });
                if faces.iter().any(Option::is_none) {
                    ui.label(
                        RichText::new(
                            "Someone in this trio is no longer in your colony. Its record stays.",
                        )
                        .size(11.0)
                        .italics()
                        .color(ink::MUTED),
                    );
                }
                for line in &earlier {
                    ui.label(RichText::new(line).size(11.0).color(ink::MUTED));
                }
                let cups: Vec<String> = trio
                    .cups
                    .iter()
                    .filter_map(|c| {
                        crate::rewards::CUPS
                            .iter()
                            .find(|cup| cup.id == c.cup)
                            .map(|cup| format!("{} {}", c.wins, cup.name))
                    })
                    .collect();
                if !cups.is_empty() {
                    ui.label(
                        RichText::new(format!("Cup wins together: {}", cups.join(", ")))
                            .size(11.0)
                            .color(ink::FOREST),
                    );
                }
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = u * 3.0;
                    if !renaming
                        && chrome::button(ui, "rename", Kind::Quiet, false, true, None, u).clicked()
                    {
                        rename_to = Some(Some(Renaming {
                            id: trio.id.clone(),
                            name: trio.name.clone(),
                        }));
                    }
                    // Its own colours, or the Handler's: a step through the cloths.
                    let own = trio.pennant.as_ref().map(|p| Style::of(p).field);
                    let word = match own {
                        Some(f) => format!("colours: {}", FIELDS[f].1.to_lowercase()),
                        None => "colours: yours".to_owned(),
                    };
                    if chrome::button(ui, &word, Kind::Quiet, false, true, None, u)
                        .on_hover_text("Fly your pennant in a colour of the trio's own")
                        .clicked()
                    {
                        let next = match own {
                            None => Some(0),
                            Some(f) if f + 1 < FIELDS.len() => Some(f + 1),
                            Some(_) => None,
                        };
                        update = Some(TrioCard {
                            pennant: next.map(|field| {
                                Style {
                                    field,
                                    ..handler_style
                                }
                                .pennant()
                            }),
                            ..trio.card()
                        });
                    }
                    let (word, hint) = if trio.retired {
                        (
                            "bring back",
                            "Recognise this trio again when its three are picked",
                        )
                    } else {
                        ("retire", "Keep its record, but stop recognising it")
                    };
                    let may = !trio.retired || may_return;
                    if chrome::button(ui, word, Kind::Quiet, false, may, None, u)
                        .on_hover_text(hint)
                        .clicked()
                    {
                        update = Some(TrioCard {
                            retired: !trio.retired,
                            ..trio.card()
                        });
                    }
                });
            },
        );
        match (rename_to, draft) {
            (Some(r), _) => self.renaming = r,
            (None, Some(name)) => {
                self.renaming = Some(Renaming {
                    id: trio.id.clone(),
                    name,
                });
            }
            (None, None) => {}
        }
        if let Some(card) = update {
            self.table.set_trio(card);
        }
    }

    /// The three a trio had before, by name where the colony still has them, and when they
    /// stopped.
    fn lineup_words(&self, trio: &Trio) -> Vec<String> {
        trio.lineups
            .iter()
            .rev()
            .take(2)
            .map(|lineup| {
                let names: Vec<String> = lineup
                    .members
                    .iter()
                    .filter(|id| !trio.members.contains(id))
                    .map(|id| {
                        self.table
                            .home
                            .iter()
                            .find(|(c, _)| c.traveler.id.0 == *id)
                            .map_or("someone no longer here".to_owned(), |(c, _)| {
                                c.traveler.name.clone()
                            })
                    })
                    .collect();
                format!(
                    "Played with {} until {}",
                    and_list(&names),
                    super::handler::date(lineup.until_utc)
                )
            })
            .collect()
    }

    /// On the Who page, under the three picked: the trio they are, or naming them as one, or
    /// taking the third into a trio that has the other two.
    pub(super) fn trio_corner(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        let ids: Vec<u64> = self
            .table
            .picked
            .iter()
            .filter_map(|&i| self.table.home.get(i))
            .map(|(c, _)| c.traveler.id.0)
            .collect();
        if ids.len() != 3 {
            return;
        }
        if let Some(trio) = identity::trio_of(&self.table.record, &ids).cloned() {
            let flag = self.flag(ctx, self.trio_style(&trio));
            ui.horizontal(|ui| {
                small_flag(ui, &flag, u);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("Playing as {}", trio.name))
                            .size(14.0)
                            .strong()
                            .color(ink::FOREST),
                    );
                    ui.label(
                        RichText::new(identity::record_words(&trio))
                            .size(12.0)
                            .color(ink::MUTED),
                    );
                });
            });
            return;
        }
        let swaps: Vec<Trio> = identity::swaps(&self.table.record, &ids)
            .into_iter()
            .cloned()
            .collect();
        let mut keep: Option<TrioCard> = None;
        for trio in &swaps {
            let leaving = trio.members.iter().find(|m| !ids.contains(m)).copied();
            let joining = ids.iter().find(|m| !trio.members.contains(m)).copied();
            let name_of = |id: Option<u64>| {
                id.and_then(|id| {
                    self.table
                        .home
                        .iter()
                        .find(|(c, _)| c.traveler.id.0 == id)
                        .map(|(c, _)| c.traveler.name.clone())
                })
                .unwrap_or_else(|| "someone".to_owned())
            };
            let (out, inn) = (name_of(leaving), name_of(joining));
            let word = format!("{inn} in for {out}: {}", trio.name);
            if chrome::button(ui, &word, Kind::Plain, false, true, Some(ui.available_width()), u)
                .on_hover_text(format!(
                    "{} keeps its name and record; {out} is remembered as having played with it before.",
                    trio.name
                ))
                .clicked()
            {
                keep = Some(TrioCard {
                    members: lineup_of(&ids),
                    ..trio.card()
                });
            }
            ui.add_space(u * 2.0);
        }
        if let Some(card) = keep {
            self.table.set_trio(card);
            self.felt.push(crate::feel::Feedback::Confirm);
            return;
        }
        let can = identity::can_name(&self.table.record);
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.trio_draft)
                    .font(FontId::proportional(14.0))
                    .char_limit(identity::TRIO_CHARS)
                    .hint_text("Name these three")
                    .desired_width((ui.available_width() - 90.0).max(120.0)),
            );
            let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let named = identity::trio_name(&self.trio_draft).is_some();
            if (chrome::button(
                ui,
                "Name",
                Kind::Plain,
                false,
                named && can.is_ok(),
                None,
                u,
            )
            .on_hover_text("Keep these three as a trio, with its own record")
            .clicked()
                || (enter && named))
                && can.is_ok()
                && let Some(card) = identity::named(&self.trio_draft, &ids)
            {
                self.table.set_trio(card);
                self.trio_draft.clear();
                self.felt.push(crate::feel::Feedback::Confirm);
            }
        });
        if let Err(why) = can {
            ui.label(RichText::new(why).size(11.5).italics().color(ink::MUTED));
        }
    }

    /// Over the board as a match begins: the two teams as their Handlers named them, with their
    /// pennants and standing, while "Ready?" and "Go!" are said.
    pub(super) fn introduction(&mut self, painter: &egui::Painter, board: Rect, now: f32, u: f32) {
        const SHOWN: f32 = 2.6;
        let Some((at, _)) = self.ready else {
            return;
        };
        let age = now - at;
        if !(0.0..SHOWN).contains(&age) {
            return;
        }
        let ctx = painter.ctx().clone();
        let you = self.table.you;
        let mut sides: [(Option<egui::TextureHandle>, String, String); 2] = Default::default();
        let ours = self.our_team_words();
        let mine = self.flag(&ctx, ours.0);
        sides[you.index()] = (Some(mine), ours.1, ours.2);
        let theirs = match &self.table.friend {
            Some(f) => {
                let card = f.cards[you.other().index()].clone().unwrap_or_default();
                let style = card
                    .trio_pennant
                    .as_ref()
                    .or(card.pennant.as_ref())
                    .map(Style::of);
                let name = f.handlers[you.other().index()].clone();
                let top = card
                    .trio
                    .clone()
                    .unwrap_or_else(|| format!("{name}'s three"));
                let under = match (&card.club, &card.standing) {
                    (Some(club), Some(s)) => format!("{name} · {club} · {s}"),
                    (Some(club), None) => format!("{name} · {club}"),
                    (None, Some(s)) => format!("{name} · {s}"),
                    (None, None) => name,
                };
                (style.map(|s| self.flag(&ctx, s)), top, under)
            }
            None => (None, "The visitors".to_owned(), "From next door".to_owned()),
        };
        sides[you.other().index()] = theirs;
        let alpha = 1.0 - ((age - (SHOWN - 0.4)) / 0.4).clamp(0.0, 1.0);
        let mut painter = painter.clone();
        painter.set_opacity(alpha);
        let width = (board.width() * 0.8).min(u * 260.0);
        let rect = Rect::from_center_size(
            pos2(board.center().x, board.min.y + u * 24.0),
            vec2(width, u * 30.0),
        );
        chrome::card(&painter, rect, u, ink::CARD, ink::LINE);
        let vs = painter.layout_no_wrap("vs".to_owned(), FontId::proportional(13.0), ink::MUTED);
        painter.galley(rect.center() - vs.size() / 2.0, vs, ink::MUTED);
        for (i, (flag, top, under)) in sides.iter().enumerate() {
            let left = i == 0;
            let fh = u * 18.0;
            let fw = fh * crate::pennant::WIDTH as f32 / crate::pennant::HEIGHT as f32;
            let fx = if left {
                rect.min.x + u * 6.0
            } else {
                rect.max.x - u * 6.0 - fw
            };
            if let Some(flag) = flag {
                let r = Rect::from_min_size(pos2(fx, rect.center().y - fh / 2.0), vec2(fw, fh));
                painter.image(
                    flag.id(),
                    r,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            let colour = super::color(crate::scene::TEAM[i].shadow);
            let top = painter.layout_no_wrap(top.clone(), FontId::proportional(14.0), colour);
            let under =
                painter.layout_no_wrap(under.clone(), FontId::proportional(10.5), ink::MUTED);
            let tx = if left {
                fx + fw + u * 4.0
            } else {
                fx - u * 4.0 - top.size().x.max(under.size().x)
            };
            let ux = if left {
                tx
            } else {
                fx - u * 4.0 - under.size().x
            };
            let ttx = if left {
                tx
            } else {
                fx - u * 4.0 - top.size().x
            };
            painter.galley(pos2(ttx, rect.center().y - top.size().y), top, colour);
            painter.galley(pos2(ux, rect.center().y + u), under, ink::MUTED);
        }
    }

    /// The Handler's own side, for the introduction and the top bar: the pennant it flies, its
    /// trio's name (or the Handler's), and a line under it.
    pub(super) fn our_team_words(&self) -> (Style, String, String) {
        let ids: Vec<u64> = self.table.board_teams[self.table.you.index()]
            .iter()
            .map(|c| c.traveler.id.0)
            .collect();
        let standing = Standing::of(&self.table.record).line();
        let handler = self.table.record.handler.as_ref();
        let under = match handler {
            Some(h) => match &h.club {
                Some(club) => format!("{} · {club} · {standing}", h.name),
                None => format!("{} · {standing}", h.name),
            },
            None => standing,
        };
        let top = match (identity::trio_of(&self.table.record, &ids), handler) {
            (Some(trio), _) => trio.name.clone(),
            (None, Some(h)) => format!("{}'s three", h.name),
            (None, None) => "Your three".to_owned(),
        };
        let style = identity::trio_of(&self.table.record, &ids)
            .map_or_else(|| self.handler_style(), |trio| self.trio_style(trio));
        (style, top, under)
    }
}

/// The pennant large, at a whole number of screen pixels to each of its own.
fn big_flag(ui: &mut egui::Ui, flag: &egui::TextureHandle, u: f32) {
    let ppp = ui.ctx().pixels_per_point();
    let scale = ((u * 4.0 * ppp).round() / ppp).max(1.0);
    let size = vec2(
        crate::pennant::WIDTH as f32 * scale,
        crate::pennant::HEIGHT as f32 * scale,
    );
    ui.add(egui::Image::new(flag).fit_to_exact_size(size));
}

/// The pennant small, about as tall as a line of the page's writing.
pub(super) fn small_flag(ui: &mut egui::Ui, flag: &egui::TextureHandle, u: f32) {
    let h = (u * 12.0).round();
    let w = h * crate::pennant::WIDTH as f32 / crate::pennant::HEIGHT as f32;
    ui.add(egui::Image::new(flag).fit_to_exact_size(vec2(w, h)));
}

/// One of a pennant's choices, stepped through with arrows either side of its name.
fn stepper(ui: &mut egui::Ui, label: &str, at: &mut usize, names: &[&str], u: f32) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [64.0, u * 10.0],
            egui::Label::new(RichText::new(label).size(12.5).color(ink::MUTED)),
        );
        let n = names.len();
        if chrome::button(ui, "‹", Kind::Quiet, false, true, None, u).clicked() {
            *at = (*at + n - 1) % n;
        }
        ui.add_sized(
            [92.0, u * 10.0],
            egui::Label::new(RichText::new(names[*at % n]).size(13.0).color(ink::PAGE)),
        );
        if chrome::button(ui, "›", Kind::Quiet, false, true, None, u).clicked() {
            *at = (*at + 1) % n;
        }
    });
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn and_list(names: &[String]) -> String {
    match names {
        [] => "others".to_owned(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
