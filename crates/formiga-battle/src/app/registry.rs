//! The Handler's registry, the first of their own pages: who they are (name, club and pennant)
//! and how they stand, beside the trios they have named. The three picked on the Who page can be
//! named as a trio there, or take a place in a trio that has the other two. Everything here is
//! kept by Desktop with the colony's Battle record, and none of it changes how a match is played.

use super::{BattleApp, Kind, sticky, texture};
use crate::chrome::{self, ink, label_job};
use crate::heraldry::{self, Part};
use crate::identity::{self, FIELDS, SHAPES, SYMBOLS, Standing, Style, TRIMS};
use crate::look::Look;
use eframe::egui::{self, Color32, FontId, Id, Rect, RichText, Sense, pos2, vec2};
use formiga_art::Canvas;
use formiga_battle_contract::{HandlerCard, Trio, TrioCard, lineup_of};

/// Thread cream, for words lettered on the notebook's green ribbon.
const ON_RIBBON: Color32 = Color32::from_rgb(0xf3, 0xe4, 0xbc);
/// The engraver's brown, for words on brass.
const ON_BRASS: Color32 = Color32::from_rgb(0x4a, 0x34, 0x12);

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

    /// A painted picture, painted once under its key and kept.
    fn painted(
        &mut self,
        ctx: &egui::Context,
        key: String,
        paint: impl FnOnce(&Self) -> Canvas,
    ) -> egui::TextureHandle {
        if let Some(t) = self.art.get(&key) {
            return t.clone();
        }
        let t = texture(ctx, &key, &paint(self));
        self.art.insert(key, t.clone());
        t
    }

    /// The looks of a trio's three, where the colony still has them.
    fn looks_of(&self, members: &[u64]) -> Vec<Option<Look>> {
        let reduce = self.table.reduce_motion;
        members
            .iter()
            .map(|id| {
                self.table
                    .home
                    .iter()
                    .find(|(c, _)| c.traveler.id.0 == *id)
                    .and_then(|(c, _)| Look::of(c, reduce).ok())
            })
            .collect()
    }

    /// A trio's team picture, as wide as there is room for, with its name lettered on its
    /// ribbon.
    fn team_picture(&mut self, ui: &mut egui::Ui, trio: &Trio, height: i32, u: f32) {
        let ctx = ui.ctx().clone();
        let w = ((ui.available_width() / u).floor() as i32).clamp(100, 240);
        let style = self.trio_style(trio);
        let key = format!(
            "team-{}-{:?}-{style:?}-{}-{w}-{height}",
            trio.id, trio.members, trio.retired
        );
        let picture = self.painted(&ctx, key, |app| {
            let mut looks = app.looks_of(&trio.members);
            heraldry::team_picture(w, height, style, &mut looks, trio.retired)
        });
        let rect = place(ui, &picture, 1.0, u).0;
        let (rx, ry, rw, rh) = heraldry::team_ribbon(w, height);
        let ribbon = Rect::from_min_size(
            rect.min + vec2(rx as f32, ry as f32) * u,
            vec2(rw as f32, rh as f32) * u,
        );
        letter(ui.painter(), ribbon, &trio.name, ON_RIBBON, 0.55);
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
        identity::trio_style(self.handler_style(), trio)
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

    /// The left page of the registry: the Handler's card as it stands, painted, and under it
    /// what it is made of: their name and club, and the samples their pennant is chosen from.
    pub(super) fn handler_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        let since = self.turned;
        let style = self.handler_style();
        crate::feel::dropping(ui, since, 0, u, |ui| self.handler_card(ui, ctx, style, u));
        ui.add_space(u * 8.0);
        let mut changed = false;
        crate::feel::dropping(ui, since, 1, u, |ui| {
            chrome::kicker(ui, "Your name and club");
            ui.add_space(u * 2.0);
            let half = ((ui.available_width() - u * 8.0) / 2.0).min(300.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = u * 8.0;
                for (hint, limit, club) in [
                    (
                        "The name your friends know you by",
                        identity::NAME_CHARS,
                        false,
                    ),
                    ("A club or team, if you like", identity::CLUB_CHARS, true),
                ] {
                    let text = if club {
                        &mut self.club
                    } else {
                        &mut self.handler
                    };
                    let field = ui.add(
                        egui::TextEdit::singleline(text)
                            .font(FontId::proportional(17.0))
                            .char_limit(limit)
                            .frame(egui::Frame::NONE)
                            .hint_text(hint)
                            .desired_width(half),
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
                }
            });
        });
        if changed {
            self.keep_card();
        }
        ui.add_space(u * 10.0);
        let mut tiles: Vec<(Part, usize, egui::TextureHandle, egui::TextureHandle, &str)> =
            Vec::new();
        for (part, names) in [
            (Part::Shape, SHAPES.map(|s| s.1).to_vec()),
            (Part::Field, FIELDS.map(|f| f.1).to_vec()),
            (Part::Symbol, SYMBOLS.map(|s| s.1).to_vec()),
            (Part::Trim, TRIMS.map(|t| t.1).to_vec()),
        ] {
            for (i, name) in names.into_iter().enumerate() {
                let chosen = match part {
                    Part::Shape => style.shape == i,
                    Part::Field => style.field == i,
                    Part::Symbol => style.symbol == i,
                    Part::Trim => style.trim == i,
                };
                let mut faces = [None, None];
                for (lit, face) in faces.iter_mut().enumerate() {
                    let lit = lit == 1;
                    *face = Some(self.painted(
                        ctx,
                        format!("tile-{style:?}-{part:?}-{i}-{chosen}-{lit}"),
                        |_| heraldry::tile(style, part, i, chosen, lit),
                    ));
                }
                let [plain, lit] = faces.map(Option::unwrap);
                tiles.push((part, i, plain, lit, name));
            }
        }
        let mut chosen = style;
        crate::feel::dropping(ui, since, 2, u, |ui| {
            chrome::kicker(ui, "Your pennant");
            ui.add_space(u * 3.0);
            for (part, label) in [
                (Part::Shape, "Cut"),
                (Part::Field, "Cloth"),
                (Part::Symbol, "Symbol"),
                (Part::Trim, "Edge"),
            ] {
                ui.horizontal_top(|ui| {
                    let (slot, _) =
                        ui.allocate_exact_size(vec2(u * 34.0, TILE_ROW * u), Sense::hover());
                    let galley =
                        ui.painter()
                            .layout_job(label_job(label, chrome::LABEL - 0.5, ink::MUTED));
                    ui.painter().galley(
                        pos2(slot.min.x, slot.center().y - galley.size().y / 2.0 - u),
                        galley,
                        ink::MUTED,
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(u * 2.0, u * 2.0);
                        for (p, i, plain, lit, name) in &tiles {
                            if *p != part {
                                continue;
                            }
                            if pick_tile(ui, plain, lit, name, u) {
                                match part {
                                    Part::Shape => chosen.shape = *i,
                                    Part::Field => chosen.field = *i,
                                    Part::Symbol => chosen.symbol = *i,
                                    Part::Trim => chosen.trim = *i,
                                }
                            }
                        }
                    });
                });
                ui.add_space(u * 3.0);
            }
            ui.label(
                RichText::new(format!(
                    "{}. It flies beside your team in every match, and your friends see it.",
                    capitalised(&style.words())
                ))
                .size(12.0)
                .italics()
                .color(ink::MUTED),
            );
        });
        if chosen != style || changed {
            // The station's picture flies the pennant as it now is.
            self.masthead = None;
            self.masthead_picture = None;
        }
        if chosen != style {
            self.draft_style = chosen;
            self.felt.push(crate::feel::Feedback::Select);
            match self.table.record.handler.clone() {
                Some(card) => self.table.set_handler(HandlerCard {
                    pennant: chosen.pennant(),
                    ..card
                }),
                None => self.keep_card(),
            }
        }
        if let Some(notice) = self.table.notice.clone() {
            ui.add_space(u * 3.0);
            sticky(ui, &notice, u);
        }
    }

    /// The Handler's card, painted, with their name, club and title lettered on it.
    fn handler_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, style: Style, u: f32) {
        let standing = Standing::of(&self.table.record);
        let won: [bool; 4] = std::array::from_fn(|i| {
            standing
                .cups
                .iter()
                .any(|c| c.id == crate::rewards::CUPS[i].id)
        });
        let w = ((ui.available_width() / u).floor() as i32).clamp(220, 380);
        let k = if self.table.reduce_motion {
            None
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(110));
            Some((self.clock * 9.0) as usize % heraldry::STIR_FRAMES)
        };
        let picture = self.painted(ctx, format!("card-{w}-{style:?}-{k:?}-{won:?}"), |_| {
            heraldry::handler_card(w, style, k.map_or(0.0, heraldry::stir_at), won)
        });
        let rect = place(ui, &picture, 1.0, u).0;
        let at = |(x, y, w, h): heraldry::Area| {
            Rect::from_min_size(
                rect.min + vec2(x as f32, y as f32) * u,
                vec2(w as f32, h as f32) * u,
            )
        };
        let places = heraldry::card_places(w);
        let painter = ui.painter();
        let card = self.table.record.handler.clone();
        match &card {
            Some(card) => {
                letter_left(painter, at(places.name), &card.name, 22.0, ink::PAGE, false);
                if let Some(club) = &card.club {
                    letter_left(painter, at(places.club), club, 13.0, ink::MUTED, true);
                }
            }
            None => letter_left(
                painter,
                at(places.name),
                "Sign your name below",
                17.0,
                ink::MUTED,
                true,
            ),
        }
        letter(painter, at(places.title), standing.title, ON_BRASS, 0.9);
        for (i, cup) in crate::rewards::CUPS.iter().enumerate() {
            let hint = if won[i] {
                format!("{}: won", cup.name)
            } else {
                format!(
                    "{}: win it with three wins against the visitors there",
                    cup.name
                )
            };
            ui.interact(
                at(places.rosettes[i]),
                Id::new(("rosette", i)),
                Sense::hover(),
            )
            .on_hover_text(hint);
        }
    }

    /// The right page of the registry: how the Handler stands, and their trios.
    pub(super) fn standing_and_trios(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        let standing = Standing::of(&self.table.record);
        let _ = ctx;
        let rules = self.rules;
        on_rules(ui, rules, label_job("Standing", chrome::LABEL, ink::FOREST));
        on_rules(
            ui,
            rules,
            hand(
                &format!(
                    "{} · {} Open Circuit points · {}",
                    standing.title,
                    standing.points,
                    standing.circuit()
                ),
                13.0,
                ink::PAGE,
                false,
            ),
        );
        on_rules(ui, rules, hand(&standing.how(), 12.0, ink::MUTED, false));
        on_rules(ui, rules, hand("", 12.0, ink::MUTED, false));
        on_rules(
            ui,
            rules,
            label_job("Your trios", chrome::LABEL, ink::FOREST),
        );
        let trios: Vec<Trio> = self.table.record.trios.iter().rev().cloned().collect();
        if trios.is_empty() {
            on_rules(
                ui,
                rules,
                hand(
                    "None yet. Pick three on the Who page and name them: a trio keeps its own record, and its name and pennant go with it into every match.",
                    12.5,
                    ink::MUTED,
                    true,
                ),
            );
            return;
        }
        for trio in &trios {
            self.trio_card(ui, ctx, trio, u);
            on_rules(ui, rules, hand("", 12.0, ink::MUTED, false));
        }
    }

    /// One trio on the registry's page: its pennant, name, three and record, and what can be done
    /// with it.
    fn trio_card(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, trio: &Trio, u: f32) {
        let missing = trio
            .members
            .iter()
            .any(|id| !self.table.home.iter().any(|(c, _)| c.traveler.id.0 == *id));
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
        let rules = self.rules;
        // The picture sits on the rules: its card from one rule down to another.
        to_rule(ui, rules);
        self.team_picture(ui, trio, 84, u);
        ui.horizontal(|ui| {
            ui.add_space(u * 6.0);
            ui.vertical(|ui| {
                if let Some(name) = &mut draft {
                    ui.horizontal(|ui| {
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
                    });
                }
                let record = if trio.retired {
                    format!("{} · retired", identity::record_words(trio))
                } else {
                    identity::record_words(trio)
                };
                on_rules(ui, rules, hand(&record, 13.0, ink::PAGE, false));
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
                on_rules(ui, rules, hand(&split, 12.0, ink::MUTED, false));
                if missing {
                    on_rules(
                        ui,
                        rules,
                        hand(
                            "Someone in this trio is no longer in your colony. Its record stays.",
                            12.0,
                            ink::MUTED,
                            true,
                        ),
                    );
                }
                for line in &earlier {
                    on_rules(ui, rules, hand(line, 12.0, ink::MUTED, false));
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
                    on_rules(
                        ui,
                        rules,
                        hand(
                            &format!("Cup wins together: {}", cups.join(", ")),
                            12.0,
                            ink::FOREST,
                            false,
                        ),
                    );
                }
                // The trio's actions sit on the next rule, as the words do.
                let y = ui.cursor().min.y;
                let rule = rule_below(rules, y + rules.1);
                ui.add_space((rule - rules.1 * 0.85 - y).max(0.0));
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
            });
        });
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
            let _ = ctx;
            chrome::kicker(ui, "Playing as");
            ui.add_space(u);
            self.team_picture(ui, &trio, 84, u);
            on_rules(
                ui,
                self.rules,
                hand(&identity::record_words(&trio), 12.5, ink::MUTED, false),
            );
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

    /// Over the board as a match begins: a ribbon strung between two canes, each flying its
    /// side's pennant, with each side's trio (or Handler) lettered on its half, and a tag under
    /// each cane naming the Handler and their standing, while "Ready?" and "Go!" are said.
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
        let mut flags: [Option<Style>; 2] = [None, None];
        let mut names: [String; 2] = Default::default();
        let mut tags: [String; 2] = Default::default();
        let ours = self.our_team_words();
        flags[you.index()] = Some(ours.0);
        names[you.index()] = ours.1;
        tags[you.index()] = ours.2;
        match &self.table.friend {
            Some(f) => {
                let card = f.cards[you.other().index()].clone().unwrap_or_default();
                flags[you.other().index()] = card
                    .trio_pennant
                    .as_ref()
                    .or(card.pennant.as_ref())
                    .map(Style::of);
                let name = f.handlers[you.other().index()].clone();
                names[you.other().index()] = card
                    .trio
                    .clone()
                    .unwrap_or_else(|| format!("{name}'s three"));
                tags[you.other().index()] = match (&card.club, &card.standing) {
                    (Some(club), Some(s)) => format!("{name} · {club} · {s}"),
                    (Some(club), None) => format!("{name} · {club}"),
                    (None, Some(s)) => format!("{name} · {s}"),
                    (None, None) => name,
                };
            }
            None => {
                names[you.other().index()] = "The visitors".to_owned();
                tags[you.other().index()] = "From next door".to_owned();
            }
        }
        // As many of the board's own pixels across as most of the board is wide.
        let scale = self.board_scale.max(1.0);
        let w = ((board.width() * 0.78) / scale).floor() as i32;
        let key = format!("banner-{w}-{flags:?}");
        let picture = self.painted(&ctx, key, |_| {
            heraldry::banner(w, flags, crate::scene::TEAM)
        });
        let alpha = 1.0 - ((age - (SHOWN - 0.4)) / 0.4).clamp(0.0, 1.0);
        let drop = if self.table.reduce_motion {
            0.0
        } else {
            ((0.25 - age).max(0.0) / 0.25) * u * 12.0
        };
        let mut painter = painter.clone();
        painter.set_opacity(alpha);
        let size = picture.size_vec2() * scale;
        let min = pos2(
            (board.center().x - size.x / 2.0).round(),
            (board.min.y + u * 8.0 - drop).round(),
        );
        let rect = Rect::from_min_size(min, size);
        painter.image(
            picture.id(),
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let (rx, ry, rw, rh) = heraldry::banner_ribbon(w);
        let ribbon = Rect::from_min_size(
            rect.min + vec2(rx as f32, ry as f32) * scale,
            vec2(rw as f32, rh as f32) * scale,
        );
        let gap = ribbon.width() * 0.08;
        let halves = [
            Rect::from_min_max(ribbon.min, pos2(ribbon.center().x - gap, ribbon.max.y)),
            Rect::from_min_max(pos2(ribbon.center().x + gap, ribbon.min.y), ribbon.max),
        ];
        for (i, half) in halves.iter().enumerate() {
            letter(&painter, *half, &names[i], ON_RIBBON, 0.55);
        }
        letter(
            &painter,
            Rect::from_center_size(ribbon.center(), vec2(gap * 2.0, ribbon.height())),
            "vs",
            Color32::from_rgb(0xf5, 0xc5, 0x42),
            0.55,
        );
        // A paper tag hung under each cane.
        for (i, tag) in tags.iter().enumerate() {
            let galley = painter.layout_no_wrap(tag.clone(), FontId::proportional(11.0), ink::PAGE);
            let pad = vec2(u * 4.0, u * 2.0);
            let tag_size = galley.size() + pad * 2.0;
            let x = if i == 0 {
                rect.min.x + u * 2.0
            } else {
                rect.max.x - u * 2.0 - tag_size.x
            };
            let tag_rect = Rect::from_min_size(pos2(x, rect.max.y + u * 2.0), tag_size);
            chrome::card(&painter, tag_rect, u, ink::CARD, ink::LINE);
            painter.galley(tag_rect.min + pad, galley, ink::PAGE);
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

/// A painted picture placed on the page, `scale` notebook pixels to each of its own, its corner on
/// a whole screen pixel so it stays crisp.
fn place(
    ui: &mut egui::Ui,
    picture: &egui::TextureHandle,
    scale: f32,
    u: f32,
) -> (Rect, egui::Response) {
    let size = picture.size_vec2() * u * scale;
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let ppp = ui.ctx().pixels_per_point();
    let rect = Rect::from_min_size(
        pos2(
            (rect.min.x * ppp).round() / ppp,
            (rect.min.y * ppp).round() / ppp,
        ),
        size,
    );
    ui.painter().image(
        picture.id(),
        rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    (rect, response)
}

/// A sample tile to choose a pennant's part with: lifted when pointed at, answering a touch as
/// every control does. True if it was chosen.
fn pick_tile(
    ui: &mut egui::Ui,
    plain: &egui::TextureHandle,
    lit: &egui::TextureHandle,
    name: &str,
    u: f32,
) -> bool {
    let size = plain.size_vec2() * u;
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let id = response.id;
    let press = crate::feel::press(ui.ctx(), id, &response, crate::feel::Feel::Select);
    let picture = if press.hover > 0.5 { lit } else { plain };
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter().image(
        picture.id(),
        rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    response.on_hover_text(name).clicked()
}

/// The first rule at or below `y`, on a page ruled from `first`, `pitch` apart.
fn rule_below((first, pitch): (f32, f32), y: f32) -> f32 {
    if y <= first {
        first
    } else {
        first + ((y - first) / pitch - 0.001).ceil() * pitch
    }
}

/// Writing on the ruled page, as a hand writes in a ruled notebook: each line of `job` with its
/// foot on a rule, the first on the next rule there is room above, wrapped to the page.
fn on_rules(ui: &mut egui::Ui, rules: (f32, f32), mut job: egui::text::LayoutJob) {
    let pitch = rules.1;
    for section in &mut job.sections {
        section.format.line_height = Some(pitch);
        section.format.valign = egui::Align::BOTTOM;
    }
    job.wrap.max_width = ui.available_width();
    let galley = ui.painter().layout_job(job);
    let baseline = galley
        .rows
        .first()
        .and_then(|r| r.row.glyphs.first().map(|g| r.pos.y + g.pos.y))
        .unwrap_or(pitch * 0.8);
    let at = ui.cursor().min;
    let rule = rule_below(rules, at.y + baseline);
    let top = rule - baseline;
    let colour = galley
        .job
        .sections
        .first()
        .map_or(ink::PAGE, |s| s.format.color);
    // The page goes on from the last line's rule, so the next words take the rule after it.
    let last = rule + (galley.rows.len().max(1) - 1) as f32 * pitch;
    let spacing = ui.spacing().item_spacing.y;
    let height = (last + 1.0 - spacing - at.y).max(0.0);
    ui.painter().galley(pos2(at.x, top), galley, colour);
    ui.allocate_space(vec2(ui.available_width(), height));
}

/// Words in the page's hand, for writing on its rules.
fn hand(text: &str, size: f32, colour: Color32, italic: bool) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: FontId::proportional(size),
            color: colour,
            italics: italic,
            ..Default::default()
        },
    );
    job
}

/// Brings the page down to the next rule, so what comes next starts on it.
fn to_rule(ui: &mut egui::Ui, rules: (f32, f32)) {
    let y = ui.cursor().min.y;
    let rule = rule_below(rules, y);
    if rule > y {
        ui.add_space(rule - y);
    }
}

/// How tall a row of sample tiles is, in notebook pixels.
const TILE_ROW: f32 = heraldry::TILE as f32;

/// Words written at the left of `area`, centred on it top to bottom, as large as `size` while
/// they fit: in the page's own hand, or its italic.
fn letter_left(
    painter: &egui::Painter,
    area: Rect,
    text: &str,
    size: f32,
    colour: Color32,
    italic: bool,
) {
    let mut size = size;
    let layout = |size: f32| {
        let mut rich = RichText::new(text).size(size).color(colour);
        if italic {
            rich = rich.italics();
        }
        let job = egui::WidgetText::from(rich).into_layout_job(
            &egui::Style::default(),
            egui::FontSelection::Default,
            egui::Align::Min,
        );
        painter.layout_job((*job).clone())
    };
    let mut galley = layout(size);
    while galley.size().x > area.width() && size > 9.0 {
        size -= 0.5;
        galley = layout(size);
    }
    painter.galley(
        pos2(area.min.x, area.center().y - galley.size().y / 2.0),
        galley,
        colour,
    );
}

/// Words lettered in the notebook's printed capitals, as large as `area` holds, centred on it.
fn letter(painter: &egui::Painter, area: Rect, text: &str, colour: Color32, fill: f32) {
    let mut size = (area.height() * fill).clamp(8.0, 16.0);
    let mut galley = painter.layout_job(label_job(text, size, colour));
    while galley.size().x > area.width() * 0.92 && size > 7.0 {
        size -= 0.5;
        galley = painter.layout_job(label_job(text, size, colour));
    }
    painter.galley(area.center() - galley.size() / 2.0, galley, colour);
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
