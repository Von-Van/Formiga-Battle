//! The Season: the Handler's place at the Battleground, in the notebook's pencil. On the left, the
//! season's four events as stations on a line and the event under way as a bracket of pennanted
//! cards, with the way to the next fixture; on the right, the Handler's standing, the season's
//! points as bars, the Hall of Champions on brass plates, what the Battleground noticed, and how
//! they stand against each rival. What it shows is the career Battle keeps
//! (`career.rs`); none of it changes how a match is played.

use super::{BattleApp, Outing, Step, kept_picture, shrunk, texture};
use crate::career;
use crate::chrome::{self, Kind as Button, hand_job, ink};
use crate::identity::Style;
use crate::rival::{self, RIVALS};
use eframe::egui::{self, Color32, Id, Rect, RichText, Sense, pos2, vec2};
use formiga_battle_world::{Bracket, Decided, Kind, Season};
use formiga_pencil::ruled;

/// A Handler's name by their id in an event: the colony is "You".
fn who(id: &str) -> String {
    if id == "you" {
        "You".to_owned()
    } else {
        rival::by_id(id).map_or_else(|| id.to_owned(), |r| r.name.to_owned())
    }
}

/// What the Battleground noticed, in words.
fn noticed_words(n: &career::Noticed) -> String {
    use career::Noticed::*;
    match n {
        FirstWin { rival } => format!("First win over {}", who(rival)),
        Upset { rival, event, .. } => {
            format!("Beat {}, seeded above you, in {event}", who(rival))
        }
        Rivalry { rival } => format!("A rivalry: five meetings with {}", who(rival)),
        EventWon { event, season } => format!("Won {event} in Season {season}"),
        SeasonChampion { season } => format!("Season {season} Champion"),
        TitleDefended { season } => format!("Defended the title in Season {season}"),
        TitleLost { season, to } => format!("Lost the title to {} in Season {season}", who(to)),
        TitleReclaimed { season } => format!("Won the title back in Season {season}"),
    }
}

/// How a fixture was settled, after the winner's name, as the Handler would jot it.
fn settled_short(decided: Decided) -> &'static str {
    match decided {
        Decided::Capture => "lit the beacon",
        Decided::Knockout => "made them dizzy",
        Decided::Cap => "won on the tally",
        Decided::Forfeit => "won by forfeit",
        Decided::Toss => "won the toss",
    }
}

/// A rival's or the colony's short name, for a card.
fn short(id: &str) -> String {
    if id == "you" {
        "Me".to_owned()
    } else {
        rival::by_id(id).map_or_else(|| id.to_owned(), |r| r.short.to_owned())
    }
}

/// How the Handler stands against a rival, in a few words.
fn against_line(played: u32, won: u32, lost: u32, short: &str) -> String {
    if played == 0 {
        return "Haven't met them yet!".to_owned();
    }
    if won > lost {
        format!("I lead {won} to {lost}!")
    } else if lost > won {
        format!("{short} leads {lost} to {won}.")
    } else {
        format!("Level at {won} each.")
    }
}

/// What a season's event is, in a few words.
fn size_words(kind: Kind) -> &'static str {
    match kind {
        Kind::Open => "Four of us",
        Kind::Regional => "Eight of us",
        Kind::Championship => "The best four",
    }
}

impl BattleApp {
    /// The pennant a Handler in an event flies: the colony's own, or a rival's.
    fn event_flag(&mut self, ctx: &egui::Context, id: &str) -> egui::TextureHandle {
        let style = if id == "you" {
            self.handler_style()
        } else {
            rival::by_id(id).map_or_else(
                || Style::of(&crate::identity::first_pennant()),
                |r| Style::of(&r.pennant()),
            )
        };
        self.flag(ctx, style)
    }

    /// What the Handler would jot under the season's heading: what is next, or how it went.
    fn season_line(&self) -> String {
        let career = &self.table.career;
        if let Some((_, round, rival)) = self.table.next_event_fixture() {
            return format!(
                "My {round} is next, against {}! {} {}.",
                rival.short,
                rival.short,
                rival.archetype.tendency()
            );
        }
        let Some(season) = &career.season else {
            return "A new season! Four events, and the best four meet at the end.".to_owned();
        };
        let last = career.event.as_ref().and_then(|e| {
            e.bracket
                .champion()
                .map(|c| (e.name.clone(), e.bracket.entrants[c].clone()))
        });
        let then = match (season.finished(), season.next_stage()) {
            (true, _) => format!(
                " Season {} is done{}.",
                season.number,
                season
                    .champion
                    .as_ref()
                    .map_or(String::new(), |c| format!(": {} is the champion", short(c)))
            ),
            (false, Some(next)) => {
                let place = formiga_battle_core::Arena::by_id(&next.arena)
                    .map_or_else(|| next.arena.clone(), |a| a.name);
                format!(" On to {}!", next.name(&place, season.number))
            }
            (false, None) => String::new(),
        };
        match last {
            Some((name, c)) if c == "you" => format!("Won {name}!{then}"),
            Some((name, c)) => format!("{} took {name}.{then}", short(&c)),
            None => then.trim().to_owned(),
        }
    }

    /// The left page of the Season: its heading, the four events on a line, the bracket of the
    /// one under way, and the way on.
    pub(super) fn event_board(&mut self, ui: &mut egui::Ui, u: f32) {
        let ctx = ui.ctx().clone();
        let career = self.table.career.clone();
        let line = match &career.season {
            Some(s) => format!("Close to home · Season {}", s.number),
            None => "Close to home · the season".to_owned(),
        };
        let title = career
            .event
            .as_ref()
            .map_or_else(|| "The Season".to_owned(), |e| e.name.clone());
        chrome::page_heading(ui, &line, &title, &self.season_line());
        ui.add_space(u * 6.0);
        let full = ui.available_rect_before_wrap();
        // The four events as stations along the foot of the page; the bracket above them.
        let rail_h = u * 84.0;
        let button_h = u * 22.0;
        let rail = Rect::from_min_max(
            pos2(full.left(), full.bottom() - rail_h - button_h - u * 8.0),
            pos2(full.right(), full.bottom() - button_h - u * 8.0),
        );
        let board = Rect::from_min_max(full.min, pos2(full.right(), rail.top() - u * 10.0));
        if let Some(event) = &career.event {
            self.bracket_board(ui, &ctx, board, &event.bracket, u);
        } else {
            let mut note = ui.new_child(egui::UiBuilder::new().max_rect(board));
            super::sticky(
                &mut note,
                "No season yet! A season is an Open of four, a Spring and a Summer Regional of eight, and a Championship of the four with the most points. In my first Open I'm the newcomer, drawn against the first seed.",
                u,
            );
        }
        if let Some(season) = &career.season {
            self.season_rail(ui, &ctx, rail, season, u);
        }
        // The way on.
        let way = Rect::from_min_max(pos2(full.left(), full.bottom() - button_h), full.max);
        let mut foot = ui.new_child(egui::UiBuilder::new().max_rect(way));
        let width = way.width();
        match self.table.next_event_fixture() {
            Some((index, round, rival)) => {
                let label = format!("Play my {round} vs. {}! ›", rival.short);
                if chrome::button(
                    &mut foot,
                    &label,
                    Button::Primary,
                    false,
                    true,
                    Some(width),
                    u,
                )
                .on_hover_text(rival.tag())
                .clicked()
                {
                    self.table.event_fixture = Some(index);
                    if let Some(arena) = career
                        .event
                        .as_ref()
                        .and_then(|e| formiga_battle_core::Arena::by_id(&e.arena))
                    {
                        self.choose_arena(arena);
                    }
                    self.open_journal(Outing::Home, Step::Who);
                }
            }
            None => {
                let label = match &career.season {
                    Some(s) if !s.finished() => "On to the next event! ›".to_owned(),
                    Some(s) => format!("Start Season {}! ›", s.number + 1),
                    None => "Start Season 1! ›".to_owned(),
                };
                if chrome::button(
                    &mut foot,
                    &label,
                    Button::Primary,
                    false,
                    true,
                    Some(width),
                    u,
                )
                .on_hover_text("An event I didn't make it into is played out at once")
                .clicked()
                {
                    self.table.begin_next_event();
                }
            }
        }
        ui.allocate_rect(full, Sense::hover());
    }

    /// The event under way as a bracket: a column of cards for each round, each card the two
    /// Handlers with their pennants, the one who went through in bold with how, lines joining
    /// each pair to the card they feed, and the champion stamped.
    fn bracket_board(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        area: Rect,
        bracket: &Bracket,
        u: f32,
    ) {
        let painter = ui.painter().clone();
        let rounds = bracket.rounds() as usize;
        let gap = u * 14.0;
        let col_w = ((area.width() - gap * (rounds as f32 - 1.0)) / rounds as f32).floor();
        // A card no wider than a name and its pennant need, centred in its round's column.
        let card_w = col_w.min(u * 170.0);
        let card_h = u * 56.0;
        let mut centres: Vec<Vec<egui::Pos2>> = Vec::new();
        for round in 0..rounds {
            let fixtures: Vec<(usize, &formiga_battle_world::Fixture)> = bracket
                .fixtures
                .iter()
                .enumerate()
                .filter(|(_, f)| f.round as usize == round)
                .collect();
            let x = area.left() + round as f32 * (col_w + gap);
            let head = painter.layout_job(hand_job(
                bracket.round_name(round as u8),
                chrome::HAND,
                ink::FOREST,
            ));
            let head_h = head.size().y;
            painter.galley(
                pos2(x + (col_w - card_w) / 2.0, area.top()),
                head,
                ink::FOREST,
            );
            let top = area.top() + head_h + u * 4.0;
            let slot = (area.bottom() - top) / fixtures.len().max(1) as f32;
            let mut col = Vec::new();
            for (k, (index, fixture)) in fixtures.iter().enumerate() {
                let cy = top + slot * (k as f32 + 0.5);
                let rect = Rect::from_center_size(pos2(x + col_w / 2.0, cy), vec2(card_w, card_h));
                let ready = fixture.ready();
                let mine = fixture
                    .sides
                    .iter()
                    .flatten()
                    .any(|e| bracket.entrants[*e] == "you");
                let fill = if ready && mine { ink::MINT } else { ink::CARD };
                chrome::card(&painter, rect, u, fill, ink::LINE.gamma_multiply(0.55));
                let winner = fixture.winner();
                for (row, entrant) in fixture.sides.iter().enumerate() {
                    let ry = rect.top() + u * 5.0 + row as f32 * u * 17.0;
                    match entrant {
                        Some(e) => {
                            let id = bracket.entrants[*e].clone();
                            let flag = self.event_flag(ctx, &id);
                            let fh = u * 13.0;
                            let fw = flag.size_vec2().x * fh / flag.size_vec2().y;
                            painter.image(
                                flag.id(),
                                Rect::from_min_size(pos2(rect.left() + u * 4.0, ry), vec2(fw, fh)),
                                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                            let won = winner == Some(*e);
                            let out = winner.is_some() && !won;
                            let colour = if out { ink::MUTED } else { ink::PAGE };
                            let font = if won {
                                formiga_pencil::hand_bold(19.0)
                            } else {
                                formiga_pencil::hand(19.0)
                            };
                            let name = painter.layout_no_wrap(short(&id), font, colour);
                            painter.galley(pos2(rect.left() + u * 7.0 + fw, ry - u), name, colour);
                        }
                        None => {
                            // Whoever comes through from the card that feeds this place.
                            let fed = bracket
                                .fixtures
                                .iter()
                                .filter(|f| round > 0 && f.round as usize == round - 1)
                                .nth(k * 2 + row)
                                .map(|f| {
                                    f.sides
                                        .iter()
                                        .flatten()
                                        .map(|e| short(&bracket.entrants[*e]))
                                        .collect::<Vec<_>>()
                                        .join(" or ")
                                })
                                .filter(|w| !w.is_empty())
                                .unwrap_or_else(|| "?".to_owned());
                            let name =
                                painter.layout_no_wrap(fed, formiga_pencil::hand(17.0), ink::MUTED);
                            painter.galley(pos2(rect.left() + u * 6.0, ry - u), name, ink::MUTED);
                        }
                    }
                }
                let foot = match &fixture.result {
                    Some(r) => format!(
                        "{} {} · {} turns",
                        winner.map_or_else(String::new, |w| short(&bracket.entrants[w])),
                        settled_short(r.decided),
                        r.turns
                    ),
                    None if ready && mine => "Up next: me!".to_owned(),
                    None if ready => "To play".to_owned(),
                    None => "Waiting".to_owned(),
                };
                let foot = painter.layout_no_wrap(foot, formiga_pencil::hand(15.0), ink::MUTED);
                painter.galley(
                    pos2(
                        rect.left() + u * 4.0,
                        rect.bottom() - foot.size().y - u * 2.0,
                    ),
                    foot,
                    ink::MUTED,
                );
                if *index == bracket.fixtures.len() - 1
                    && let Some(c) = bracket.champion()
                {
                    let word = if bracket.entrants[c] == "you" {
                        "Champion!"
                    } else {
                        "Champion"
                    };
                    chrome::stamp(
                        &painter,
                        pos2(rect.right() - u * 22.0, rect.top() - u * 2.0),
                        word,
                        chrome::HAND,
                        if bracket.entrants[c] == "you" {
                            ink::FOREST
                        } else {
                            ink::STAMP
                        },
                        Some(ink::CARD.gamma_multiply(0.9)),
                        (-6f32).to_radians(),
                        u,
                    );
                }
                col.push(rect.right_center());
                let _ = index;
            }
            // Lines from each pair in the round before to the card they feed.
            if let Some(before) = centres.last() {
                for (k, into) in col.iter().enumerate() {
                    let left_edge = pos2(into.x - card_w, into.y);
                    for from in before.iter().skip(k * 2).take(2) {
                        let mid = from.x + gap / 2.0;
                        let stroke = egui::Stroke::new(u, ink::LINE.gamma_multiply(0.5));
                        painter.line_segment([*from, pos2(mid, from.y)], stroke);
                        painter.line_segment([pos2(mid, from.y), pos2(mid, left_edge.y)], stroke);
                        painter.line_segment([pos2(mid, left_edge.y), left_edge], stroke);
                    }
                }
            }
            centres.push(col);
        }
    }

    /// The season's four events as stations on one line: played (stamped with who won it), up
    /// next, or still to come.
    fn season_rail(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        area: Rect,
        season: &Season,
        u: f32,
    ) {
        let painter = ui.painter().clone();
        let gap = u * 8.0;
        let n = season.stages.len().max(1) as f32;
        let width = ((area.width() - gap * (n - 1.0)) / n).floor();
        // The rails the stations stand on.
        let rail_y = area.top() + u * 20.0;
        for y in [rail_y, rail_y + u * 5.0] {
            painter.hline(
                (area.left() - u * 4.0)..=(area.right() + u * 4.0),
                y,
                egui::Stroke::new(u, Color32::from_rgb(0xb2, 0x95, 0x70)),
            );
        }
        let hall: Vec<(u32, String)> = self
            .table
            .career
            .hall
            .iter()
            .map(|c| (c.season, c.who.clone()))
            .collect();
        for (i, stage) in season.stages.iter().enumerate() {
            let rect = Rect::from_min_size(
                pos2(area.left() + i as f32 * (width + gap), area.top()),
                vec2(width, area.height()),
            );
            let played = i < season.played;
            let next = i == season.played;
            let fade = if played || next { 1.0 } else { 0.62 };
            chrome::card(
                &painter,
                rect,
                u,
                if next { ink::MINT } else { ink::CARD }.gamma_multiply(fade),
                if next {
                    ink::FOREST
                } else {
                    ink::LINE.gamma_multiply(0.55 * fade)
                },
            );
            let inner = rect.shrink(u * 4.0);
            let thumb_h = u * 34.0;
            let thumb = self
                .arenas
                .iter_mut()
                .find(|(a, _)| a.id == stage.arena)
                .map(|(arena, thumb)| {
                    thumb
                        .get_or_insert_with(|| {
                            let view = crate::iso::View::new(arena.width, arena.height);
                            let garden = crate::garden::Garden::new(arena, &view);
                            texture(
                                ctx,
                                &format!("arena-{}", arena.id),
                                &shrunk(&garden.picture(), 3),
                            )
                        })
                        .clone()
                });
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
            let place = formiga_battle_core::Arena::by_id(&stage.arena)
                .map_or_else(|| stage.arena.clone(), |a| a.name);
            let name = stage.name(&place, season.number);
            let name = name.trim_start_matches("The ").to_owned();
            let galley = painter.layout(
                name,
                formiga_pencil::hand_bold(15.0),
                ink::PAGE.gamma_multiply(fade),
                inner.width(),
            );
            let name_y = inner.top() + thumb_h + u * 2.0;
            let name_h = galley.size().y;
            painter.galley(pos2(inner.left(), name_y), galley, ink::PAGE);
            let note = painter.layout_no_wrap(
                size_words(stage.kind).to_owned(),
                formiga_pencil::hand(15.0),
                ink::MUTED.gamma_multiply(fade),
            );
            painter.galley(pos2(inner.left(), name_y + name_h), note, ink::MUTED);
            if played {
                // Who won it: the Hall for the Championship, the event's bracket for the last one.
                let champion =
                    if stage.kind == Kind::Championship {
                        hall.iter()
                            .find(|(s, _)| *s == season.number)
                            .map(|(_, w)| w.clone())
                    } else if i + 1 == season.played {
                        self.table.career.event.as_ref().and_then(|e| {
                            e.bracket.champion().map(|c| e.bracket.entrants[c].clone())
                        })
                    } else {
                        None
                    };
                let (word, colour) = match champion.as_deref() {
                    Some("you") => ("Won!".to_owned(), ink::FOREST),
                    Some(id) => (short(id), ink::STAMP),
                    None => ("Played".to_owned(), ink::MUTED),
                };
                chrome::stamp(
                    &painter,
                    pos2(rect.right() - u * 16.0, rect.top() + u * 10.0),
                    &word,
                    chrome::HAND,
                    colour,
                    Some(ink::CARD.gamma_multiply(0.9)),
                    (-7f32).to_radians(),
                    u,
                );
            } else if next {
                let tab = painter.layout_job(hand_job("Up next", chrome::HAND, ink::COVER));
                let tab_rect = Rect::from_center_size(
                    pos2(rect.center().x, rect.top()),
                    tab.size() + vec2(u * 8.0, u * 2.0),
                );
                painter.rect_filled(tab_rect, 0.0, ink::BRASS);
                painter.rect_filled(tab_rect.shrink(u), 0.0, ink::FOREST);
                painter.galley(tab_rect.center() - tab.size() / 2.0, tab, ink::COVER);
            }
            ui.interact(rect, Id::new(("season-station", i)), Sense::hover())
                .on_hover_text(format!(
                    "{}: {}",
                    stage.name(&place, season.number),
                    size_words(stage.kind).to_lowercase()
                ));
        }
    }

    /// The ruled right page of the Season: the Handler's standing, the season's points, the Hall
    /// of Champions, what the Battleground noticed, and how they stand against each rival.
    pub(super) fn standing_against_rivals(&mut self, ui: &mut egui::Ui, u: f32) {
        let ctx = ui.ctx().clone();
        let career = self.table.career.clone();
        let (title, how) = career::standing(&career);
        // My standing, on a card with my pennant.
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(width, u * 40.0), Sense::hover());
        let painter = ui.painter().clone();
        chrome::card(&painter, rect, u, ink::CARD, ink::LINE.gamma_multiply(0.55));
        let flag = self.event_flag(&ctx, "you");
        let fh = u * 26.0;
        let fw = flag.size_vec2().x * fh / flag.size_vec2().y;
        painter.image(
            flag.id(),
            Rect::from_min_size(rect.min + vec2(u * 5.0, u * 6.0), vec2(fw, fh)),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let x = rect.left() + u * 10.0 + fw;
        let mut y = rect.top() + u * 4.0;
        for (text, font, colour) in [
            (
                "Me, so far".to_owned(),
                formiga_pencil::hand_bold(15.0),
                ink::MUTED,
            ),
            (title.to_owned(), formiga_pencil::hand_bold(20.0), ink::PAGE),
            (format!("{how}!"), formiga_pencil::hand(15.0), ink::FOREST),
        ] {
            let galley = painter.layout(text, font, colour, rect.right() - x - u * 4.0);
            let h = galley.size().y;
            painter.galley(pos2(x, y), galley, colour);
            y += h;
        }
        ui.add_space(u * 6.0);

        // The season's points, as bars.
        if let Some(season) = &career.season
            && !season.points.is_empty()
        {
            ruled::write(
                ui,
                hand_job(
                    &format!("Season {} points", season.number),
                    chrome::HAND,
                    ink::FOREST,
                ),
            );
            let table = season.table();
            let most = table.first().map_or(1, |(_, p)| (*p).max(1));
            for (id, points) in table {
                let flag = self.event_flag(&ctx, &id);
                // A rule to each: the name and the points rest on it, the bar beside them.
                let row = ruled::band(ui, 1.0, u * 13.0);
                let painter = ui.painter().clone();
                let fh = u * 9.0;
                let fw = flag.size_vec2().x * fh / flag.size_vec2().y;
                painter.image(
                    flag.id(),
                    Rect::from_min_size(pos2(row.left(), row.top() + u * 2.0), vec2(fw, fh)),
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                let me = id == "you";
                let colour = if me { ink::FOREST } else { ink::PAGE };
                let name = painter.layout_no_wrap(
                    short(&id),
                    if me {
                        formiga_pencil::hand_bold(15.0)
                    } else {
                        formiga_pencil::hand(15.0)
                    },
                    colour,
                );
                let rests = ruled::on_rule(&name, row.bottom());
                painter.galley(pos2(row.left() + fw + u * 3.0, rests), name, colour);
                let bar_x = row.left() + width * 0.42;
                let bar_w = (row.right() - bar_x - u * 14.0) * points as f32 / most as f32;
                let bar = Rect::from_min_size(
                    pos2(bar_x, row.top() + u * 3.0),
                    vec2(bar_w.max(u), u * 6.0),
                );
                painter.rect_filled(bar, 0.0, if me { ink::FOREST } else { ink::BRASS });
                let number = painter.layout_no_wrap(
                    points.to_string(),
                    formiga_pencil::hand_bold(15.0),
                    colour,
                );
                let rests = ruled::on_rule(&number, row.bottom());
                painter.galley(pos2(bar.right() + u * 2.0, rests), number, colour);
            }
            ui.add_space(u * 6.0);
        }

        // The Hall of Champions, a brass plate a season.
        if !career.hall.is_empty() {
            ruled::write(ui, hand_job("Hall of Champions", chrome::HAND, ink::FOREST));
            let plate_w = (width / u).floor() as i32;
            let plate = kept_picture(&ctx, &format!("hall-plate-{plate_w}"), || {
                crate::heraldry::plate(plate_w)
            });
            for champion in career.hall.iter().rev() {
                let size = plate.size_vec2() * u;
                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                let painter = ui.painter();
                painter.image(
                    plate.id(),
                    rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                let words = format!("Season {} · {}", champion.season, who(&champion.who));
                let galley =
                    painter.layout_no_wrap(words, formiga_pencil::hand_bold(15.0), ink::BRASS_EDGE);
                painter.galley(rect.center() - galley.size() / 2.0, galley, ink::BRASS_EDGE);
                ui.add_space(u * 2.0);
            }
            ui.add_space(u * 4.0);
        }

        // What the Battleground noticed: the newest on a sticky note, the rest under it.
        if let Some((newest, rest)) = career.recognition.split_last() {
            ruled::write(ui, hand_job("Noticed!", chrome::HAND, ink::FOREST));
            super::sticky(ui, &format!("{}!", noticed_words(&newest.noticed)), u);
            for r in rest.iter().rev().take(4) {
                ruled::write(
                    ui,
                    RichText::new(noticed_words(&r.noticed))
                        .font(formiga_pencil::hand(15.0))
                        .color(ink::MUTED),
                );
            }
            ui.add_space(u * 6.0);
        }

        // How I stand against each rival.
        ruled::write(ui, hand_job("The rivals", chrome::HAND, ink::FOREST));
        for r in &RIVALS {
            let h = career.against(r.id);
            let flag = self.event_flag(&ctx, r.id);
            ruled::ruled_row(ui, |ui| {
                let fh = u * 11.0;
                let size = vec2(flag.size_vec2().x * fh / flag.size_vec2().y, fh);
                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                ui.painter().image(
                    flag.id(),
                    rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                ui.vertical(|ui| {
                    ruled::write(
                        ui,
                        RichText::new(format!("{} · {}", r.name, r.club))
                            .font(formiga_pencil::hand_bold(15.0))
                            .color(ink::PAGE),
                    )
                    .on_hover_text(r.tag());
                    ruled::write(
                        ui,
                        RichText::new(format!(
                            "{} {}",
                            against_line(h.played, h.won, h.lost, r.short),
                            capitalised(r.archetype.tendency())
                        ))
                        .font(formiga_pencil::hand(15.0))
                        .color(ink::MUTED),
                    );
                });
            });
        }
        if !career.titles.is_empty() {
            ui.add_space(u * 6.0);
            ruled::write(ui, hand_job("My titles", chrome::HAND, ink::FOREST));
            for title in career.titles.iter().rev() {
                ruled::write(
                    ui,
                    RichText::new(&title.event)
                        .font(formiga_pencil::hand_bold(15.0))
                        .color(ink::FOREST),
                );
            }
        }
    }
}

fn capitalised(words: &str) -> String {
    let mut chars = words.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect::<String>() + ".",
        None => String::new(),
    }
}
