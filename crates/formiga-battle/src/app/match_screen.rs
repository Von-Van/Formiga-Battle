//! The match on screen: the leather strips along the top and the foot (which every stage
//! shows) with the sound control on them, the board on the left page with what is laid over it
//! while a turn plays out (the headlines, the turn's banner, the result's stamp, the card of
//! recent headlines and the whole match), and the right page: the orders, the field guide, a
//! Field Lesson's notes and result, and a match's result.

use super::*;

impl BattleApp {
    /// The tabs and what is written on the leather along the top: the arena, the turn, how near
    /// each team is to lighting the beacon, and the timer.
    pub(super) fn top_bar(&mut self, ui: &mut egui::Ui, layout: &Layout, now: f32) {
        let u = layout.unit;
        let words = self.tab_words();
        // The tabs were laid out at the start of the frame, and a page drawn since may have left
        // for somewhere with other tabs, or none: the station's button on a first page, or after
        // a match. Only the tabs there still are get drawn; the next frame lays out the rest.
        for (i, (rect, word)) in layout.tabs.iter().zip(&words).enumerate() {
            let open = i == layout.open_tab;
            let reachable = match self.stage {
                Stage::Journal => self.reachable(self.steps()[i]).is_ok(),
                _ => true,
            };
            let mut response = ui.interact(
                *rect,
                Id::new(("tab", i)),
                if reachable {
                    Sense::click()
                } else {
                    Sense::hover()
                },
            );
            if self.stage == Stage::Playing && !open {
                crate::feel::press(
                    ui.ctx(),
                    Id::new(("tab", i)),
                    &response,
                    crate::feel::Feel::Select,
                );
            }
            if response.hovered() && !open && reachable {
                chrome::stepped(ui.painter(), *rect, u, Color32::from_rgb(0x87, 0x60, 0x4a));
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let colour = if open {
                ink::PAGE
            } else if reachable {
                ink::DEBOSS
            } else {
                ink::DEBOSS.gamma_multiply(0.45)
            };
            let galley = ui
                .painter()
                .layout_job(label_job(word, chrome::LABEL, colour));
            ui.painter()
                .galley(rect.center() - galley.size() / 2.0, galley, colour);
            if self.stage == Stage::Journal
                && let Err(why) = self.reachable(self.steps()[i])
            {
                response = response.on_hover_text(why);
            }
            if response.clicked() && !open {
                match self.stage {
                    Stage::Journal => self.turn_to(self.steps()[i], now),
                    _ => {
                        self.notebook_open = i == 1;
                        self.turned = now;
                    }
                }
            }
        }
        self.sound_button(ui, layout);
        let area = Rect::from_min_max(
            pos2(layout.patch.max.x + u * 8.0, layout.patch.min.y),
            pos2(layout.left.max.x - u * 4.0, layout.patch.max.y),
        );
        if self.stage == Stage::Journal {
            self.journal_strip(ui, area, u, now);
            return;
        }
        if self.stage == Stage::Station {
            ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(hand_job("The Battleground", chrome::LABEL, ink::COVER));
                });
            });
            return;
        }
        let paused = self.paused;
        // Each side's pennant beside its name: the Handler's (or their trio's), and a friend's.
        let ctx = ui.ctx().clone();
        let mut flags: [Option<egui::TextureHandle>; 2] = [None, None];
        let ours = self.our_team_words().0;
        flags[self.table.you.index()] = Some(self.flag(&ctx, ours));
        let theirs = self.table.friend.as_ref().and_then(|f| {
            f.cards[self.table.you.other().index()]
                .as_ref()
                .and_then(|c| c.trio_pennant.as_ref().or(c.pennant.as_ref()))
                .map(crate::identity::Style::of)
        });
        if let Some(style) = theirs {
            flags[self.table.you.other().index()] = Some(self.flag(&ctx, style));
        }
        ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = u * 4.0;
                let lesson = self
                    .table
                    .lesson
                    .as_ref()
                    .map(|run| (run.lesson, run.given.len() as u8));
                ui.label(hand_job(
                    &match lesson {
                        Some((lesson, _)) => format!("Lesson {}", lesson.number()),
                        None => self.table.arena.name.clone(),
                    },
                    chrome::LABEL,
                    ink::DEBOSS,
                ));
                let Some(m) = &self.table.m else {
                    return;
                };
                ui.add_space(u * 4.0);
                let giving = self.table.shown.is_none() && m.outcome().is_none();
                // A lesson counts its own few turns.
                let (turn, cap) = match lesson {
                    Some((lesson, given)) => (given + u8::from(giving), lesson.turns()),
                    None => (m.board.turn + u8::from(giving), m.ruleset.turn_cap),
                };
                let turn = turn.min(cap);
                // The last turn says so, in brass: whatever the tally says after it decides.
                let last = turn >= cap;
                // A brass stud for each turn along the leather; the last glints while it is
                // being given, since whatever the tally says after it decides.
                let words = if last && cap > 1 {
                    format!("Last turn, {turn} of {cap}")
                } else {
                    format!("Turn {} of {cap}", turn.max(1))
                };
                let glow = last && (self.table.reduce_motion || (now * 3.0).sin() > 0.0);
                let shown_turn = turn.max(1);
                let studs = kept_picture(
                    ui.ctx(),
                    &format!("studs-{shown_turn}-{cap}-{glow}"),
                    || crate::keepsake::studs(shown_turn, cap, glow),
                );
                registry::place(ui, &studs, 1.0, u).1.on_hover_text(words);
                ui.add_space(u * 4.0);
                let shown = self.table.frame_capture(now);
                for side in Side::BOTH {
                    let team = TEAM[side.index()];
                    let name = if side == self.table.you {
                        "You".to_owned()
                    } else {
                        self.table.other_short()
                    };
                    if let Some(flag) = &flags[side.index()] {
                        let h = (u * 9.0).round();
                        let w = h * crate::pennant::WIDTH as f32 / crate::pennant::HEIGHT as f32;
                        ui.add(egui::Image::new(flag).fit_to_exact_size(vec2(w, h)));
                    }
                    ui.label(hand_job(&name, chrome::LABEL, color(team.light)));
                    beacon_meter(
                        ui,
                        shown[side.index()],
                        m.ruleset.capture_ticks,
                        team,
                        paused,
                        u,
                        now,
                    );
                    if shown[side.index()] + 1 == m.ruleset.capture_ticks && m.outcome().is_none() {
                        ui.label(hand_job("One tick to go", chrome::LABEL, ink::BRASS_LIGHT));
                    }
                    ui.add_space(u * 2.0);
                }
                if paused && (self.table.reduce_motion || (now * 2.0).sin() > -0.4) {
                    ui.label(hand_job("Held by both", chrome::LABEL, ink::COVER));
                }
                if self.table.shown.is_none() && m.outcome().is_none() && lesson.is_none() {
                    ui.add_space(u * 3.0);
                    let left = self.table.time_left(now);
                    let of = f32::from(self.table.ruleset.turn_seconds).max(left);
                    watch(ui, left, of, u);
                    let warn = left <= 10.0;
                    ui.label(hand_job(
                        &format!("{}s", left.ceil() as u32),
                        12.0,
                        if warn {
                            Color32::from_rgb(0xff, 0xb0, 0x90)
                        } else {
                            ink::COVER
                        },
                    ));
                    let other = color(TEAM[self.table.you.other().index()].light);
                    let (word, colour) = match &self.table.friend {
                        None => (format!("{} ready", self.table.other_short()), other),
                        Some(f) if f.opponent_away => {
                            (format!("{} is away", self.table.other_short()), ink::DEBOSS)
                        }
                        Some(f) if f.opponent_ready => {
                            (format!("{} is ready", self.table.other_short()), other)
                        }
                        Some(_) => (
                            format!("{} is choosing", self.table.other_short()),
                            ink::DEBOSS,
                        ),
                    };
                    ui.add_space(u * 2.0);
                    ui.label(hand_job(&word, chrome::LABEL, colour));
                }
            });
        });
    }

    /// The sound, on the leather at the top right: a little speaker that opens a card with the
    /// music's and the sounds' volumes and a mute.
    fn sound_button(&mut self, ui: &mut egui::Ui, layout: &Layout) {
        let u = layout.unit;
        let levels = self.sound.levels();
        let word = if levels.muted { "Muted" } else { "Sound" };
        let galley = ui
            .painter()
            .layout_job(label_job(word, chrome::LABEL, ink::DEBOSS));
        let size = vec2(galley.size().x + u * 23.0, layout.patch.height());
        let rect = Rect::from_min_size(
            pos2(layout.right.max.x - size.x - u * 2.0, layout.patch.min.y),
            size,
        );
        let response = ui
            .interact(rect, Id::new("sound"), Sense::click())
            .on_hover_text("Music and sound volume");
        crate::feel::press(
            ui.ctx(),
            Id::new("sound"),
            &response,
            if self.sound_open {
                crate::feel::Feel::Cancel
            } else {
                crate::feel::Feel::Select
            },
        );
        let painter = ui.painter();
        let colour = if response.hovered() || self.sound_open {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            chrome::stepped(painter, rect, u, Color32::from_rgb(0x87, 0x60, 0x4a));
            ink::COVER
        } else {
            ink::DEBOSS
        };
        let waves = if levels.muted || levels.music + levels.sounds == 0.0 {
            0
        } else if levels.music.max(levels.sounds) < 0.5 {
            1
        } else {
            2
        };
        speaker(
            painter,
            pos2(rect.min.x + u * 7.0, rect.center().y),
            waves,
            colour,
            u,
        );
        let galley = painter.layout_job(hand_job(word, chrome::LABEL, colour));
        painter.galley(
            pos2(
                rect.min.x + u * 18.0,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            colour,
        );
        if response.clicked() {
            self.sound_open = !self.sound_open;
        }
        if self.sound_open {
            self.sound_card(ui.ctx(), rect, u);
        }
    }

    /// The card of volumes under the speaker: music, sounds, and a mute for everything.
    fn sound_card(&mut self, ctx: &egui::Context, under: Rect, u: f32) {
        let mut levels = self.sound.levels();
        let before = levels;
        let width = 230.0;
        let mut close = false;
        egui::Area::new(Id::new("sound-card"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(under.max.x - width, under.max.y + u * 3.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(ink::CARD)
                    .stroke(Stroke::new(u, ink::LINE))
                    .shadow(egui::Shadow {
                        offset: [2, 3],
                        blur: 0,
                        spread: 0,
                        color: Color32::from_black_alpha(60),
                    })
                    .inner_margin(egui::Margin::same((u * 6.0).round() as i8))
                    .show(ui, |ui| {
                        ui.set_width(width - u * 12.0);
                        ui.horizontal(|ui| {
                            chrome::kicker(ui, "Sound");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if chrome::button(ui, "close", Kind::Back, false, true, None, u)
                                        .clicked()
                                    {
                                        close = true;
                                    }
                                },
                            );
                        });
                        ui.spacing_mut().slider_width = width - u * 12.0 - 70.0;
                        for (word, level) in
                            [("Music", &mut levels.music), ("Sounds", &mut levels.sounds)]
                        {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    vec2(56.0, 18.0),
                                    egui::Label::new(RichText::new(word).size(15.0)),
                                );
                                ui.add(egui::Slider::new(level, 0.0..=1.0).show_value(false));
                            });
                        }
                        ui.checkbox(&mut levels.muted, "Mute everything");
                    });
            });
        if levels != before {
            let louder = levels.sounds > before.sounds || (before.muted && !levels.muted);
            self.sound.set_levels(levels);
            // A sound to hear how loud the sounds now are.
            if louder || levels.sounds < before.sounds {
                self.sound.play(Cue::Tick);
            }
        }
        if close {
            self.sound_open = false;
        }
    }

    /// What to do next, along the leather at the foot; and who Battle is open for.
    pub(super) fn foot_bar(&self, ui: &mut egui::Ui, layout: &Layout, now: f32) {
        let u = layout.unit;
        let (left, right) = (
            layout.foot.min.x + RIM * u + u * 4.0,
            layout.foot.max.x - RIM * u - u * 4.0,
        );
        let line = match self.stage {
            Stage::Station => match &self.station {
                Some(s) if s.leaving() => "Off they go…".to_owned(),
                Some(s) if s.stopped(now) => {
                    "Choose where they are off to, or look back over your history.".to_owned()
                }
                _ => "Click the picture, or press Space, to hurry the train in.".to_owned(),
            },
            Stage::Journal => self.journal_hint(),
            Stage::Playing if self.table.finished(now) && self.table.lesson.is_some() => {
                "The lesson is over. Try it again, or go on, from the notes page.".to_owned()
            }
            Stage::Playing if self.table.finished(now) => {
                "The match is over. Play again from the notes page.".to_owned()
            }
            Stage::Playing if self.table.shown.is_some() => "The turn plays out…".to_owned(),
            Stage::Playing => match (self.table.selected, self.table.verb) {
                (_, Some(Verb::UseItem)) => match self.table.user {
                    Some(_) => "USE ITEM: click a green square in reach to use it there, at the start of the turn. A bonus: it takes none of the three. Escape is done.".to_owned(),
                    None => "USE ITEM: click one of yours carrying something, or press U to go round them.".to_owned(),
                },
                (Some(_), Some(Verb::Press)) => {
                    "PRESS: click squares to push theirs off and take them. Right-click or Escape is done.".to_owned()
                }
                (Some(_), Some(Verb::Hold)) => {
                    "HOLD: click squares to keep, facing the arrow. R or the mouse wheel turns it. Right-click or Escape is done."
                        .to_owned()
                }
                (Some(_), Some(Verb::Trick)) => {
                    "TRICK: click squares where theirs hold, to go round their side or back. Right-click or Escape is done."
                        .to_owned()
                }
                (Some(_), Some(Verb::Interact)) => {
                    "INTERACT: click a lever, a tap or switch, or a found thing to fetch.".to_owned()
                }
                (Some(i), _) => format!(
                    "Order {}: click a square to PRESS it, something to INTERACT with; right-click to HOLD it. R turns a HOLD. Enter locks in.",
                    i + 1
                ),
                (None, _) => "Choose one of your three orders.".to_owned(),
            },
        };
        let painter = ui.painter();
        let baseline = foot_baseline(layout, FOOT_WRITING);
        for (text, colour, at_right) in [
            (line, ink::COVER, false),
            (self.host.describe(), ink::DEBOSS, true),
        ] {
            let galley = painter.layout_no_wrap(text, FontId::proportional(FOOT_WRITING), colour);
            let rests = galley
                .rows
                .first()
                .and_then(|row| row.glyphs.first())
                .map_or(galley.size().y * 0.75, |glyph| glyph.pos.y);
            let x = if at_right {
                right - galley.size().x
            } else {
                left
            };
            painter.galley(pos2(x, baseline - rests), galley, colour);
        }
    }

    pub(super) fn board_view(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        area: Rect,
        now: f32,
        u: f32,
    ) {
        let Some(frame) = self.table.frame(now) else {
            return;
        };
        self.paused = frame.paused;
        // The match's own place: a friend's room plays the opener's, whatever was chosen here.
        let m = self.table.m.as_ref().expect("a match is on");
        garden_for(&mut self.garden, &mut self.view, &m.arena);
        let mut marks = self.table.marks(self.hover, now);
        // The board's scale is known before it is painted: it is drawn as large as the page
        // allows, by whole screen pixels.
        let scale = {
            let ppp = ctx.pixels_per_point();
            let (w, h) = (self.view.size.0 as f32, self.view.size.1 as f32);
            ((area.width() * ppp) / w)
                .min((area.height() * ppp) / h)
                .floor()
                .max(1.0)
                / ppp
        };
        let (lettering, caption) = self.overlays(&frame, ctx, scale, u, now);
        marks.lettering = lettering;
        // While orders are given, who will probably go where: faint footprints and a word on
        // the name tag. Played out, the turn's own.
        let likely = self.table.likely();
        if likely.is_empty() {
            marks.trails = self.trails(now);
        } else {
            marks.trails_likely = true;
            marks.trails = likely
                .iter()
                .map(|(slot, task, _)| {
                    let to = (f32::from(task.at.x) + 0.5, f32::from(task.at.y) + 0.5);
                    (frame.figure(*slot).at, to, task.directive)
                })
                .collect();
        }
        marks.pokes = self.pokes.iter().map(|(t, at)| (*t, now - at)).collect();
        let m = self.table.m.as_ref().expect("a match is on");
        let garden = garden_for(&mut self.garden, &mut self.view, &m.arena);
        let canvas = scene::paint(
            &m.arena,
            garden,
            &frame,
            &mut self.table.looks,
            &marks,
            m.ruleset.capture_ticks,
            now,
            self.table.reduce_motion,
            &self.view,
        );
        // Scaled by whole screen pixels, as large as the page allows.
        let ppp = ctx.pixels_per_point();
        let (w, h) = (canvas.width() as f32, canvas.height() as f32);
        let pixels = ((area.width() * ppp) / w)
            .min((area.height() * ppp) / h)
            .floor()
            .max(1.0);
        let scale = pixels / ppp;
        let size = vec2(w * scale, h * scale);
        let min = area.center() - size / 2.0;
        let min = pos2((min.x * ppp).round() / ppp, (min.y * ppp).round() / ppp);
        let rect = Rect::from_min_size(min, size);
        into_image(&canvas, std::sync::Arc::make_mut(&mut self.board_pixels));
        let image = std::sync::Arc::clone(&self.board_pixels);
        match &mut self.board {
            Some(texture) => texture.set(image, egui::TextureOptions::NEAREST),
            None => {
                self.board = Some(ctx.load_texture("board", image, egui::TextureOptions::NEAREST))
            }
        }
        let texture = self.board.as_ref().expect("just made").id();
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        ui.painter().image(
            texture,
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        // A copy of the view, so a square can be found while the Handler's choice changes the app.
        let view = self.view;
        let to_tile = move |pos: egui::Pos2| {
            let local = (pos - rect.min) / scale;
            view.tile_at(local.x, local.y)
        };
        // The mouse over the board puts the arrow keys' cursor away; otherwise it shows where
        // the keys have got to.
        let pointed = response.hover_pos().and_then(to_tile);
        if pointed.is_some() && ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
            self.cursor = None;
        }
        self.hover = pointed.or(self.cursor);
        let giving = self.table.shown.is_none() && self.table.outcome().is_none();
        let aims = self
            .hover
            .filter(|_| giving)
            .map(|tile| (self.table.aim(tile, false), self.table.aim(tile, true)));
        if aims
            .as_ref()
            .is_some_and(|(plain, right)| *plain != Aim::Nothing || *right != Aim::Nothing)
        {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let pressed = if response.clicked() {
            Some(false)
        } else if response.secondary_clicked() {
            Some(true)
        } else {
            None
        };
        if giving
            && let Some(hold) = pressed
            && let Some(tile) = response.interact_pointer_pos().and_then(to_tile)
        {
            self.press_tile(tile, hold, now);
        }
        // The mouse wheel over the board turns the HOLD in hand, or the one last given: a notch
        // a quarter turn, up clockwise and down back.
        if giving && response.hovered() {
            self.wheel += ctx.input(|i| i.smooth_scroll_delta.y);
            if self.wheel.abs() >= WHEEL_NOTCH {
                let back = self.wheel < 0.0;
                self.wheel = 0.0;
                if self.table.turn_hold(back) {
                    self.sound.play(Cue::Select);
                }
            }
        } else {
            self.wheel = 0.0;
        }
        // Dragging across the board with PRESS, HOLD or TRICK in hand adds every square it passes.
        if giving
            && response.dragged_by(egui::PointerButton::Primary)
            && let Some(tile) = ctx.pointer_latest_pos().and_then(to_tile)
            && self.table.add_square(tile)
        {
            self.pokes.push((tile, now));
            self.sound.play(Cue::Order);
        }
        // A name tag over whoever is pointed at or chosen, saying how they stand.
        let painter = ui.painter().clone();
        for slot in Slot::all() {
            let f = frame.figure(slot);
            if f.out {
                continue;
            }
            let pointed = self.hover == Some(f.tile) || marks.selected == Some(slot);
            if !pointed {
                continue;
            }
            let look = &self.table.looks[slot.side.index()][slot.index as usize];
            let (hx, hy) = scene::head(&self.view, f, look);
            let at = rect.min + vec2(hx as f32, hy as f32 - 7.0) * scale;
            let threatened = self.table.m.as_ref().is_some_and(|m| {
                self.table.shown.is_none() && formiga_battle_core::ai::flank_threatened(m, slot)
            });
            let state = if f.off_balance {
                Some("off balance".to_owned())
            } else if let Some((_, task, why)) = likely.iter().find(|(s, _, _)| *s == slot) {
                Some(likely_words(&self.table, task, why))
            } else if threatened {
                Some("holding, could be flanked".to_owned())
            } else if f.holding {
                Some("holding".to_owned())
            } else {
                None
            };
            // While orders are given, the first thing the notebook has noted of one of the
            // Handler's own, to plan round.
            let note = (slot.side == self.table.you && !likely.is_empty())
                .then(|| {
                    let id = self.table.board_teams[slot.side.index()]
                        .get(slot.index as usize)?
                        .traveler
                        .id
                        .0;
                    self.table.page(id)?.notes().into_iter().next()
                })
                .flatten();
            name_tag(
                &painter,
                at,
                &look.name,
                state.as_deref(),
                note.as_deref(),
                TEAM[slot.side.index()],
                u,
            );
        }
        self.board_scale = scale;
        if self.table.shown.is_none()
            && self.table.outcome().is_none()
            && let Some(slot) = marks.selected
        {
            let f = frame.figure(slot);
            let look = &self.table.looks[slot.side.index()][slot.index as usize];
            let (hx, hy) = scene::head(&self.view, f, look);
            let bob = if self.table.reduce_motion {
                0.0
            } else {
                (now * 5.0).sin().abs() * 3.0
            };
            let tip =
                rect.min + vec2(hx as f32, hy as f32 - 7.0) * scale - vec2(0.0, u * 22.0 + bob * u);
            pointer_arrow(&painter, tip, color(TEAM[slot.side.index()].light), u);
        }
        if let Some(caption) = &caption {
            self.caption(&painter, rect, scale, caption, u);
        }
        self.confetti(&painter, rect, now, u);
        self.result_stamp(&painter, rect, now);
        self.introduction(&painter, rect, now, u);
        if let (Some(pointer), Some((plain, right))) = (response.hover_pos(), aims) {
            self.cursor_hint(&painter, pointer, plain, right, u);
        }
    }

    /// Lays out this frame's words and line over the board, in board pixels, so none covers a
    /// face or another: the line over the head of whoever it is about, each cartoon word above
    /// whoever it happened to (or beside them, if that is taken), composure lost beside the head,
    /// and "READY?" then "GO!" as a match begins. The words are Formiga pixel art for the board
    /// to paint (`lettering`); the line is drawn on the notebook's own card after it.
    fn overlays(
        &self,
        frame: &crate::playback::Frame,
        ctx: &egui::Context,
        scale: f32,
        u: f32,
        now: f32,
    ) -> (Vec<scene::Lettered>, Option<Caption>) {
        use crate::lettering::{Paint, word};
        let reduce = self.table.reduce_motion;
        let (w, h) = (self.view.size.0 as f32, self.view.size.1 as f32);
        // Every face on the board, with the pips over it: kept clear.
        let heads: Vec<(Slot, f32, f32)> = Slot::all()
            .filter(|s| !frame.figure(*s).out)
            .map(|s| {
                let look = &self.table.looks[s.side.index()][s.index as usize];
                let (hx, hy) = scene::head(&self.view, frame.figure(s), look);
                (s, hx as f32, hy as f32)
            })
            .collect();
        let mut taken: Vec<Rect> = heads
            .iter()
            .map(|(_, x, y)| Rect::from_min_max(pos2(x - 14.0, y - 10.0), pos2(x + 14.0, y + 24.0)))
            .collect();
        let anchor = |c: &Callout| -> Option<(f32, f32, Option<Rect>)> {
            if let Some(slot) = c.about
                && let Some((_, x, y)) = heads.iter().find(|(s, _, _)| *s == slot)
            {
                let own = Rect::from_min_max(pos2(x - 14.0, y - 10.0), pos2(x + 14.0, y + 24.0));
                return Some((*x, y - 10.0, Some(own)));
            }
            let (x, y) = c.place?;
            let (sx, sy) = self.view.screen(x, y);
            Some((sx, sy - 26.0, None))
        };
        let clear = |r: Rect, taken: &[Rect]| {
            r.min.x >= 1.0
                && r.max.x <= w - 1.0
                && r.min.y >= 1.0
                && !taken.iter().any(|t| t.intersects(r))
        };

        // The line: one at a time, each given a moment of its own before the next. The lines
        // that matter most are placed first; a lesser one takes whatever room is left near its
        // moment. A line that would come too long after its moment is left to the card of the
        // last two, so the board never falls behind the turn.
        let mut told: Vec<(&Callout, f32)> = Vec::new();
        for rank in (1..=3).rev() {
            for (c, born) in &self.live {
                if c.rank != rank || c.headline.is_none() || anchor(c).is_none() {
                    continue;
                }
                let placed: Vec<f32> = told.iter().map(|(_, at)| *at).collect();
                let shown = headline_moment(*born, &placed);
                if shown - born <= 1.0 {
                    told.push((c, shown));
                }
            }
        }
        told.sort_by(|a, b| a.1.total_cmp(&b.1));
        let mut caption = None;
        // Once the match is over, its stamp says so; no line over it.
        if self.finished_at.is_some() {
            told.clear();
        }
        if let Some(k) = told.iter().rposition(|(_, shown)| *shown <= now) {
            let (c, shown) = told[k];
            let age = now - shown;
            let fade_in = if reduce { 1.0 } else { (age / 0.15).min(1.0) };
            let alpha = fade_in * ((2.2 - age) / 0.4).clamp(0.0, 1.0);
            if let (Some(text), Some((ax, ay, _)), true) = (&c.headline, anchor(c), alpha > 0.0) {
                let size = ctx
                    .fonts_mut(|f| {
                        f.layout_no_wrap(text.clone(), FontId::proportional(15.0), ink::PAGE)
                    })
                    .size();
                let size = (size + vec2(u * 16.0, u * 6.0)) / scale;
                let x = (ax - size.x / 2.0).clamp(2.0, (w - size.x - 2.0).max(2.0));
                // Room under it for its own word, which goes just over the head.
                let room = c.word.map_or(0.0, |text| {
                    let big = i32::from(c.style == Style::Big) + 1;
                    word(text, big, Paint::CREAM).height() as f32 + 4.0
                });
                let mut r = Rect::from_min_size(pos2(x, ay - 4.0 - room - size.y), size);
                // Up, clear of anyone else's face.
                for _ in 0..12 {
                    if clear(r, &taken) || r.min.y < 4.0 {
                        break;
                    }
                    r = r.translate(vec2(0.0, -4.0));
                }
                taken.push(r);
                caption = Some(Caption {
                    text: text.clone(),
                    good_for: c.good_for,
                    alpha,
                    rect: r,
                    tip: pos2(ax.clamp(r.min.x + 4.0, r.max.x - 4.0), ay),
                });
            }
        }

        // The words, newest first, each in the first clear spot round whoever it is about.
        let mut out = Vec::new();
        let mut words = 0;
        let mut shown_damage = false;
        for (k, (c, born)) in self.live.iter().enumerate().rev() {
            let Some(text) = c.word else {
                continue;
            };
            let Some((ax, ay, _)) = anchor(c) else {
                continue;
            };
            let age = now - born;
            let life = match c.style {
                Style::Big => 1.5,
                Style::Word => 1.0,
                Style::Damage => 1.1,
            };
            if age > life {
                continue;
            }
            if c.style != Style::Damage {
                words += 1;
                if words > 3 {
                    continue;
                }
            }
            let alpha = 1.0 - ((age - life * 0.65) / (life * 0.35)).clamp(0.0, 1.0);
            let popping = !reduce && age < 0.1;
            let (picture, rise) = match c.style {
                Style::Damage => (
                    word(text, 1, Paint::HURT),
                    if reduce { 0.0 } else { age * 14.0 },
                ),
                style => {
                    let colours = match c.good_for {
                        _ if style == Style::Big => Paint::GOLD,
                        Some(side) => Paint::team(TEAM[side.index()]),
                        None => Paint::CREAM,
                    };
                    let scale = if style == Style::Big { 2 } else { 1 } + i32::from(popping);
                    (
                        word(text, scale, colours),
                        if reduce { 0.0 } else { age * 8.0 },
                    )
                }
            };
            let (pw, ph) = (picture.width() as f32, picture.height() as f32);
            let above = pos2(ax - pw / 2.0, ay - ph - 2.0);
            let left = pos2(ax - 16.0 - pw, ay + 4.0);
            let right = pos2(ax + 16.0, ay + 4.0);
            let higher = pos2(ax - pw / 2.0, ay - ph * 2.0 - 4.0);
            let above_left = pos2(ax - pw - 4.0, ay - ph - 2.0);
            let above_right = pos2(ax + 4.0, ay - ph - 2.0);
            let lean = if k % 2 == 0 {
                [right, left]
            } else {
                [left, right]
            };
            let tries = if c.style == Style::Damage {
                [lean[0], lean[1], above, above_right, above_left, higher]
            } else {
                [above, above_right, above_left, lean[0], lean[1], higher]
            };
            let found = tries
                .into_iter()
                .map(|p| Rect::from_min_size(p, vec2(pw, ph)))
                .find(|r| clear(*r, &taken));
            // An older word with nowhere clear to go makes way; the newest word, and the newest
            // number, always show.
            let newest = if c.style == Style::Damage {
                !shown_damage
            } else {
                words == 1
            };
            shown_damage |= c.style == Style::Damage;
            let Some(spot) =
                found.or_else(|| newest.then(|| Rect::from_min_size(tries[0], vec2(pw, ph))))
            else {
                continue;
            };
            taken.push(spot);
            out.push((
                picture,
                spot.min.x.round() as i32,
                (spot.min.y - rise).round() as i32,
                (alpha.clamp(0.0, 1.0) * 255.0) as u8,
            ));
        }

        // The end of a match, said big in the middle of the board under the stamp: a cheer, a
        // sigh, or a shrug.
        if let (Some(at), Some(outcome)) = (self.finished_at, self.table.outcome()) {
            let age = now - at;
            if age <= 2.4 {
                let (text, colours) = match outcome.winner {
                    Some(side) if side == self.table.you => ("HOORAY!", Paint::GOLD),
                    Some(_) => ("SO CLOSE!", Paint::CREAM),
                    None => ("A TIE!", Paint::CREAM),
                };
                let since = age - 0.35;
                if since >= 0.0 {
                    let size = 3 + i32::from(!reduce && since < 0.1);
                    let alpha = 1.0 - ((age - 2.0) / 0.4).clamp(0.0, 1.0);
                    let picture = word(text, size, colours);
                    let x = (w - picture.width() as f32) / 2.0;
                    let y = h * 0.58 - picture.height() as f32 / 2.0;
                    out.push((
                        picture,
                        x.round() as i32,
                        y.round() as i32,
                        (alpha * 255.0) as u8,
                    ));
                }
            }
        }
        if let Some((at, _)) = self.ready {
            let age = now - at;
            if age <= READY_SECS {
                let (text, since, colours) = if age < GO_AT {
                    ("READY?", age, Paint::CREAM)
                } else {
                    ("GO!", age - GO_AT, Paint::GOLD)
                };
                let size = 3 + i32::from(!reduce && since < 0.1);
                let alpha = if age < GO_AT {
                    1.0
                } else {
                    1.0 - ((age - READY_SECS + 0.25) / 0.25).clamp(0.0, 1.0)
                };
                let picture = word(text, size, colours);
                let x = (w - picture.width() as f32) / 2.0;
                let y = h * 0.42 - picture.height() as f32 / 2.0;
                out.push((
                    picture,
                    x.round() as i32,
                    y.round() as i32,
                    (alpha * 255.0) as u8,
                ));
            }
        }
        (out, caption)
    }

    /// The line over the board, on a little card pointing down at whoever it is about.
    fn caption(&self, painter: &egui::Painter, board: Rect, scale: f32, caption: &Caption, u: f32) {
        let rect = Rect::from_min_max(
            board.min + caption.rect.min.to_vec2() * scale,
            board.min + caption.rect.max.to_vec2() * scale,
        );
        let rect = Rect::from_min_size(
            pos2((rect.min.x / u).round() * u, (rect.min.y / u).round() * u),
            rect.size(),
        );
        let tip = board.min + caption.tip.to_vec2() * scale;
        let mut painter = painter.clone();
        painter.set_opacity(caption.alpha);
        chrome::card(&painter, rect, u, ink::CARD, ink::LINE);
        tick(&painter, rect, caption.good_for, u);
        let px = tip.x.clamp(rect.min.x + u * 4.0, rect.max.x - u * 4.0);
        if tip.y > rect.max.y + u * 2.0 {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(px - u * 2.5, rect.max.y - u),
                    pos2(px + u * 2.5, rect.max.y - u),
                    pos2(px, rect.max.y + u * 2.5),
                ],
                ink::LINE,
                Stroke::NONE,
            ));
        }
        let galley =
            painter.layout_no_wrap(caption.text.clone(), FontId::proportional(15.0), ink::PAGE);
        painter.galley(
            pos2(
                rect.min.x + u * 9.0,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            ink::PAGE,
        );
    }

    /// Footprints from each of the Handler's team to the job it took on, as the turn's orders are
    /// revealed and into its first tick, in the colour of the order it serves.
    fn trails(&self, now: f32) -> Vec<scene::Trail> {
        let Some(shown) = &self.table.shown else {
            return Vec::new();
        };
        if now - shown.started > crate::playback::REVEAL_SECS + crate::playback::TICK_SECS {
            return Vec::new();
        }
        let start = shown.playback.at(0.0);
        shown
            .report
            .events
            .iter()
            .filter_map(|e| match &e.kind {
                formiga_battle_core::EventKind::Assigned { slot, task, .. }
                    if slot.side == self.table.you =>
                {
                    let to = (f32::from(task.at.x) + 0.5, f32::from(task.at.y) + 0.5);
                    Some((start.figure(*slot).at, to, task.directive))
                }
                _ => None,
            })
            .collect()
    }

    /// Confetti over the board when the Handler's team wins: a short shower, the same every time.
    fn confetti(&self, painter: &egui::Painter, board: Rect, now: f32, u: f32) {
        let (Some(at), Some(outcome)) = (self.finished_at, self.table.outcome()) else {
            return;
        };
        if outcome.winner != Some(self.table.you) || self.table.reduce_motion {
            return;
        }
        let age = now - at;
        if age > 3.0 {
            return;
        }
        let team = TEAM[self.table.you.index()];
        let colours = [
            color(team.light),
            color(team.shine),
            ink::BRASS_LIGHT,
            ink::MINT,
            Color32::from_rgb(0xff, 0xf3, 0xc4),
        ];
        let fade = 1.0 - ((age - 2.2) / 0.8).clamp(0.0, 1.0);
        for k in 0..70u32 {
            // A scrap's own numbers, from its index alone.
            let h = k.wrapping_mul(2_654_435_761).rotate_left(7) ^ 0x5bd1_e995;
            let unit = |shift: u32| ((h >> shift) & 0xff) as f32 / 255.0;
            let start = unit(0) * 0.6;
            let t = age - start;
            if t < 0.0 {
                continue;
            }
            let x = board.min.x
                + board.width() * (0.1 + 0.8 * unit(8))
                + (t * (2.0 + 3.0 * unit(16))).sin() * u * 6.0;
            let y =
                board.min.y + board.height() * 0.05 + t * board.height() * (0.25 + 0.15 * unit(24));
            if y > board.max.y {
                continue;
            }
            let spin = (t * (4.0 + 6.0 * unit(4))).sin();
            let size = vec2(u * 3.0 * spin.abs().max(0.25), u * 2.0);
            painter.rect_filled(
                Rect::from_center_size(pos2(x, y), size),
                0.0,
                colours[(h % colours.len() as u32) as usize].gamma_multiply(fade),
            );
        }
    }

    /// A little card by the pointer saying what a click there would do, and a right click.
    fn cursor_hint(
        &self,
        painter: &egui::Painter,
        pointer: egui::Pos2,
        plain: Aim,
        right: Aim,
        u: f32,
    ) {
        let Some(m) = &self.table.m else {
            return;
        };
        let say = |aim: &Aim| match aim {
            Aim::Give(order @ Directive::UseItem { .. }) => {
                Some(format!("Bonus. {}", speak_order(m, self.table.you, order)))
            }
            Aim::Give(order) => Some(match self.table.selected {
                Some(i) => format!("{}. {}", i + 1, speak_order(m, self.table.you, order)),
                None => speak_order(m, self.table.you, order),
            }),
            Aim::Carrier(by) => Some(format!(
                "USE ITEM: {}",
                carried(m, Slot::new(self.table.you, *by))
            )),
            Aim::Nothing => None,
        };
        let mut lines = Vec::new();
        if let Some(text) = say(&plain) {
            lines.push((false, text));
        }
        if self.table.verb.is_some() {
            lines.push((true, "done".to_owned()));
        } else if let Some(text) = say(&right).filter(|_| right != plain) {
            lines.push((true, text));
        }
        if lines.is_empty() {
            return;
        }
        let galleys: Vec<_> = lines
            .into_iter()
            .map(|(right, text)| {
                (
                    right,
                    painter.layout_no_wrap(text, FontId::proportional(15.0), ink::PAGE),
                )
            })
            .collect();
        let icon = vec2(u * 6.0, u * 9.0);
        let row = galleys
            .iter()
            .map(|(_, g)| g.size().y)
            .fold(icon.y, f32::max)
            + u * 2.0;
        let width = galleys.iter().map(|(_, g)| g.size().x).fold(0.0, f32::max) + icon.x + u * 14.0;
        let height = row * galleys.len() as f32 + u * 6.0;
        let min = pointer + vec2(u * 10.0, u * 12.0);
        let rect = Rect::from_min_size(
            pos2((min.x / u).round() * u, (min.y / u).round() * u),
            vec2(width, height),
        );
        chrome::card(painter, rect, u, ink::CARD, ink::LINE);
        for (k, (right, galley)) in galleys.into_iter().enumerate() {
            let top = rect.min.y + u * 3.0 + k as f32 * row;
            let at = Rect::from_min_size(
                pos2(rect.min.x + u * 5.0, top + (row - icon.y) / 2.0 - u),
                icon,
            );
            mouse(painter, at, right, u);
            let h = galley.size().y;
            painter.galley(
                pos2(at.max.x + u * 4.0, top + (row - h) / 2.0 - u),
                galley,
                ink::PAGE,
            );
        }
    }

    /// What is written on the board's page round it: the turn's slip and its headlines across
    /// the top, the last two headlines on a card in the upper right (the whole match opens from
    /// there), and the controls in the lower left while orders are being given.
    pub(super) fn board_page(&mut self, ui: &mut egui::Ui, page: Rect, now: f32, u: f32) {
        let painter = ui.painter().clone();
        self.turn_banner(&painter, page, now, u);
        self.orders_banner(&painter, page, now, u);
        let card = self.recent_card(ui, page, u);
        if self.account_open {
            self.account(ui.ctx(), page, card, u);
        }
        if self.table.shown.is_none() && self.table.outcome().is_none() {
            controls(&painter, page, u);
        }
    }

    /// The last two headlines, kept on a card in the board's upper right as a reminder, newest
    /// first; and under them, the way into the whole match told in full. Returns the card.
    fn recent_card(&mut self, ui: &mut egui::Ui, page: Rect, u: f32) -> Rect {
        let painter = ui.painter().clone();
        let width = (page.width() * 0.3).clamp(220.0, 300.0);
        let inner = width - u * 14.0;
        let lines: Vec<(std::sync::Arc<egui::Galley>, Option<Side>)> =
            if self.recent.is_empty() && !self.table.told.is_empty() {
                Vec::new()
            } else if self.recent.is_empty() {
                vec![(
                    painter.layout(
                        "Nothing yet: the first turn is yours.".to_owned(),
                        FontId::proportional(15.0),
                        ink::MUTED,
                        inner,
                    ),
                    None,
                )]
            } else {
                self.recent
                    .iter()
                    .rev()
                    .enumerate()
                    .map(|(k, (text, side))| {
                        let colour = if k == 0 { ink::PAGE } else { ink::MUTED };
                        (
                            painter.layout(text.clone(), FontId::proportional(15.0), colour, inner),
                            *side,
                        )
                    })
                    .collect()
            };
        let title = painter.layout_job(hand_job("Just now", 9.5, ink::FOREST));
        // While orders are given, what came of each of last turn's, under the headlines.
        let planning = self.table.shown.is_none() && self.table.outcome().is_none();
        let orders_title = painter.layout_job(hand_job("Your orders last turn", 9.0, ink::FOREST));
        let told: Vec<std::sync::Arc<egui::Galley>> = if planning {
            self.table
                .told
                .iter()
                .map(|line| {
                    painter.layout(line.clone(), FontId::proportional(15.0), ink::PAGE, inner)
                })
                .collect()
        } else {
            Vec::new()
        };
        let told_height = if told.is_empty() {
            0.0
        } else {
            orders_title.size().y + u * 5.0 + told.iter().map(|g| g.size().y + u * 1.5).sum::<f32>()
        };
        let link_text = if self.account_open {
            "Close the whole match (L)"
        } else {
            "The whole match (L)"
        };
        let link = painter.layout_job(hand_job(link_text, 9.0, ink::FOREST));
        let body: f32 = lines.iter().map(|(g, _)| g.size().y + u * 2.0).sum();
        let height = u * 9.0 + title.size().y + body + told_height + link.size().y + u * 6.0;
        let corner = pos2(page.max.x - u * 8.0 - width, page.min.y + u * 6.0);
        let rect = Rect::from_min_size(
            pos2((corner.x / u).round() * u, (corner.y / u).round() * u),
            vec2(width, height),
        );
        chrome::card(&painter, rect, u, ink::CARD, ink::LINE.gamma_multiply(0.45));
        let mut y = rect.min.y + u * 4.0;
        painter.galley(pos2(rect.min.x + u * 5.0, y), title.clone(), ink::FOREST);
        y += title.size().y + u * 3.0;
        for (galley, side) in lines {
            let h = galley.size().y;
            if side.is_some() || !self.recent.is_empty() {
                let mark = Rect::from_min_size(
                    pos2(rect.min.x + u * 5.0, y + u),
                    vec2(u * 1.5, h - u * 2.0),
                );
                painter.rect_filled(
                    mark,
                    0.0,
                    side.map_or(ink::LINE.gamma_multiply(0.4), |s| {
                        color(TEAM[s.index()].base)
                    }),
                );
            }
            painter.galley(pos2(rect.min.x + u * 9.0, y), galley, ink::PAGE);
            y += h + u * 2.0;
        }
        if !told.is_empty() {
            y += u * 2.0;
            painter.hline(
                (rect.min.x + u * 5.0)..=(rect.max.x - u * 5.0),
                y,
                Stroke::new(u * 0.5, ink::LINE.gamma_multiply(0.35)),
            );
            y += u * 2.0;
            let h = orders_title.size().y;
            painter.galley(pos2(rect.min.x + u * 5.0, y), orders_title, ink::FOREST);
            y += h + u * 3.0;
            for galley in told {
                let h = galley.size().y;
                painter.galley(pos2(rect.min.x + u * 5.0, y), galley, ink::PAGE);
                y += h + u * 1.5;
            }
        }
        y += u;
        let link_rect =
            Rect::from_min_size(pos2(rect.min.x + u * 5.0, y), link.size() + vec2(0.0, u));
        let response = ui.interact(link_rect, Id::new("whole-match"), Sense::click());
        crate::feel::press(
            ui.ctx(),
            Id::new("whole-match"),
            &response,
            if self.account_open {
                crate::feel::Feel::Cancel
            } else {
                crate::feel::Feel::Select
            },
        );
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            painter.hline(
                link_rect.x_range(),
                link_rect.bottom(),
                Stroke::new(u * 0.7, ink::FOREST),
            );
        }
        painter.galley(link_rect.min, link, ink::FOREST);
        if response.clicked() {
            self.toggle_account();
        }
        rect
    }

    /// The whole match told in full sentences, every turn, in a little menu that opens under the
    /// card of the last two: for checking what happened.
    fn account(&mut self, ctx: &egui::Context, page: Rect, card: Rect, u: f32) {
        let width = card.width().max(320.0);
        let top = card.max.y + u * 3.0;
        let mut close = false;
        egui::Area::new(Id::new("account"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(card.max.x - width, top))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(ink::CARD)
                    .stroke(Stroke::new(u, ink::LINE))
                    .shadow(egui::Shadow {
                        offset: [2, 3],
                        blur: 0,
                        spread: 0,
                        color: Color32::from_black_alpha(60),
                    })
                    .inner_margin(egui::Margin::same((u * 6.0).round() as i8))
                    .show(ui, |ui| {
                        ui.set_width(width - u * 12.0);
                        ui.horizontal(|ui| {
                            chrome::kicker(ui, "The whole match");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if chrome::button(ui, "close", Kind::Back, false, true, None, u)
                                        .clicked()
                                    {
                                        close = true;
                                    }
                                },
                            );
                        });
                        egui::ScrollArea::vertical()
                            .max_height((page.max.y - top - u * 16.0).max(120.0))
                            .min_scrolled_height((page.max.y - top - u * 16.0).clamp(120.0, 420.0))
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                if self.table.log.is_empty() {
                                    ui.label(
                                        RichText::new("Nothing yet: the first turn is yours.")
                                            .size(15.0)
                                            .italics()
                                            .color(ink::MUTED),
                                    );
                                }
                                // The turn still playing out is told once it has been seen.
                                let playing = self
                                    .table
                                    .shown
                                    .as_ref()
                                    .filter(|_| self.finished_at.is_none())
                                    .map(|shown| shown.report.turn);
                                for (turn, lines) in &self.table.log {
                                    if Some(*turn) == playing {
                                        continue;
                                    }
                                    ui.add_space(u);
                                    ui.label(hand_job(
                                        &format!("Turn {turn}"),
                                        chrome::LABEL,
                                        ink::MUTED,
                                    ));
                                    if lines.is_empty() {
                                        ui.label(
                                            RichText::new("· Everyone went where they were told.")
                                                .size(15.0)
                                                .color(ink::MUTED),
                                        );
                                    }
                                    for line in lines {
                                        ui.label(RichText::new(format!("· {line}")).size(15.0));
                                    }
                                }
                            });
                    });
            });
        if close {
            self.toggle_account();
        }
    }

    /// While a turn's orders are revealed, a slip drops onto the top of the board naming it.
    fn turn_banner(&self, painter: &egui::Painter, board: Rect, now: f32, u: f32) {
        let (Some(shown), Some(m)) = (&self.table.shown, &self.table.m) else {
            return;
        };
        let age = now - shown.started;
        // Gone by the first tick, when the turn's own headlines begin.
        let life = crate::playback::REVEAL_SECS;
        if age > life || self.table.finished(now) {
            return;
        }
        let (drop, alpha) = if self.table.reduce_motion {
            (0.0, 1.0)
        } else {
            let t = (age / 0.25).min(1.0);
            let ease = 1.0 - (1.0 - t).powi(3);
            let out = ((life - age) / 0.2).clamp(0.0, 1.0);
            ((1.0 - ease) * -14.0 * u, out)
        };
        let galley = painter.layout_job(hand_job(
            &format!("Turn {} plays out", m.board.turn),
            13.0,
            registry::ON_RIBBON,
        ));
        let size = galley.size() + vec2(u * 16.0, u * 9.0);
        let rect = Rect::from_center_size(
            pos2(
                board.center().x,
                board.min.y + u * 8.0 + size.y / 2.0 + drop,
            ),
            size,
        );
        let rect = Rect::from_min_size(
            pos2((rect.min.x / u).round() * u, (rect.min.y / u).round() * u),
            rect.size(),
        );
        let mut painter = painter.clone();
        painter.set_opacity(alpha);
        slip_ribbon(&painter, rect, None, u);
        painter.galley(
            rect.center() - galley.size() / 2.0,
            galley,
            registry::ON_RIBBON,
        );
    }

    /// As each turn's orders begin, a slip in the Handler's colour says so: whose move it is.
    fn orders_banner(&self, painter: &egui::Painter, page: Rect, now: f32, u: f32) {
        let Some(m) = &self.table.m else {
            return;
        };
        let age = now - self.table.turn_began;
        let life = 1.4;
        if self.table.shown.is_some() || m.outcome().is_some() || !(0.0..life).contains(&age) {
            return;
        }
        let (drop, alpha) = if self.table.reduce_motion {
            (0.0, 1.0)
        } else {
            let t = (age / 0.22).min(1.0);
            let out = ((life - age) / 0.3).clamp(0.0, 1.0);
            ((1.0 - (1.0 - t).powi(3)) * 14.0 * u - 14.0 * u, out)
        };
        let galley = painter.layout_job(hand_job(
            &format!("Turn {}: your orders", m.board.turn + 1),
            13.0,
            registry::ON_RIBBON,
        ));
        let size = galley.size() + vec2(u * 22.0, u * 9.0);
        let rect = Rect::from_min_size(
            pos2(page.center().x - size.x / 2.0, page.min.y + u * 8.0 + drop),
            size,
        );
        let rect = Rect::from_min_size(
            pos2((rect.min.x / u).round() * u, (rect.min.y / u).round() * u),
            rect.size(),
        );
        let mut painter = painter.clone();
        painter.set_opacity(alpha);
        // In the Handler's own team colour: their move.
        slip_ribbon(&painter, rect, Some(TEAM[self.table.you.index()]), u);
        painter.galley(
            rect.center() - galley.size() / 2.0,
            galley,
            registry::ON_RIBBON,
        );
    }

    /// When the match is over, a rubber stamp comes down on the board saying how it ended.
    fn result_stamp(&self, painter: &egui::Painter, board: Rect, now: f32) {
        let (Some(at), Some(outcome)) = (self.finished_at, self.table.outcome()) else {
            return;
        };
        let you = self.table.you;
        let word = match (self.table.lesson.as_ref().map(|r| &r.verdict), outcome) {
            (Some(Verdict::Learned(_)), _) => "Learned!",
            (Some(_), _) => "Not yet",
            (None, outcome) => match (outcome.winner, outcome.victory) {
                (Some(s), Some(Victory::Capture)) if s == you => "Beacon lit!",
                (Some(s), _) if s == you => "We win!",
                (Some(_), _) => "Well played",
                (None, _) => "A draw",
            },
        };
        let t = if self.table.reduce_motion {
            1.0
        } else {
            ((now - at) / 0.32).clamp(0.0, 1.0)
        };
        let ease = 1.0 - (1.0 - t).powi(3);
        // It comes down from a little above the page, large, and lands at its own size.
        let grow = 1.0 + (1.0 - ease) * 0.8;
        let picture = kept_picture(painter.ctx(), &format!("stamp-{word}"), || {
            crate::keepsake::stamp(&word.to_uppercase(), 2)
        });
        let scale = self.board_scale.max(1.0);
        let size = picture.size_vec2() * scale * grow;
        let centre = pos2(board.center().x, board.min.y + board.height() * 0.2);
        let rect = Rect::from_center_size(centre, size);
        let rect = Rect::from_min_size(rect.min.round(), rect.size());
        let mut painter = painter.clone();
        painter.set_opacity(ease);
        painter.image(
            picture.id(),
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    pub(super) fn side_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, now: f32, u: f32) {
        if self.notebook_open {
            chrome::kicker(ui, "What you have seen them do");
            ui.add_space(u * 2.0);
            self.notebook(ui, ctx, u);
            return;
        }
        if self.table.finished(now) {
            self.result(ui, u);
            return;
        }
        if self.table.room_gone() {
            self.room_gone(ui, u);
            return;
        }
        let you = self.table.you;
        let sent = self.table.friend.as_ref().is_some_and(|f| {
            self.table
                .m
                .as_ref()
                .is_some_and(|m| f.sent(m.board.turn + 1))
        });
        let giving = self.table.shown.is_none() && !sent;
        let Some(m) = &self.table.m else {
            return;
        };
        let standing: Vec<bool> = (0..TEAM_SIZE as u8)
            .map(|i| m.standing(Slot::new(you, i)))
            .collect();
        let composure: Vec<u8> = (0..TEAM_SIZE as u8)
            .map(|i| m.fighter(Slot::new(you, i)).composure)
            .collect();
        if let Some(run) = &self.table.lesson {
            let lesson = run.lesson;
            let turn =
                (run.given.len() + usize::from(giving)).clamp(1, usize::from(lesson.turns()));
            sticky(
                ui,
                &format!("{}\nTurn {turn} of {}", lesson.goal(), lesson.turns()),
                u,
            );
            ui.add_space(u * 3.0);
        }
        if let Some((name, words)) = self.table.playbook_words() {
            sticky(ui, &format!("Playbook: {name}\n{words}"), u);
            ui.add_space(u * 3.0);
        }
        chrome::kicker(
            ui,
            if giving {
                "My orders"
            } else {
                "The turn plays out"
            },
        );
        ui.add_space(u * 2.0);
        let team = TEAM[you.index()];
        // Who is on the team, and how steady each is: the plan is the team's, not one creature's.
        ui.horizontal_wrapped(|ui| {
            for i in 0..TEAM_SIZE as u8 {
                let contender = &self.table.board_teams[you.index()][i as usize];
                let up = standing[i as usize];
                ui.label(
                    RichText::new(&contender.traveler.name)
                        .size(15.0)
                        .strong()
                        .color(if up { ink::PAGE } else { ink::MUTED }),
                );
                pips(ui, composure[i as usize], team, u);
                ui.add_space(u * 3.0);
            }
        });
        ui.add_space(u * 2.0);
        // The three orders in hand: pick one, then the squares or the thing on the board.
        if giving {
            let left = self.table.orders.iter().filter(|o| o.is_none()).count();
            ruled_line(
                ui,
                hand_job("Give an order", chrome::LABEL, ink::FOREST),
                |ui| {
                    ruled::write(
                        ui,
                        hand_job(
                            &format!("{left} of {PLAN_SIZE} left!"),
                            chrome::HAND,
                            ink::STAMP,
                        ),
                    );
                },
            );
            ui.add_space(u);
            let row = egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(true);
            let mut picked = None;
            let carriers = self.table.carriers();
            ui.with_layout(row, |ui| {
                ui.spacing_mut().item_spacing = vec2(u * 2.0, u * 2.0);
                for v in Verb::ALL {
                    let on = self.table.verb == Some(v);
                    let can = left > 0
                        || self
                            .table
                            .selected
                            .and_then(|i| self.table.orders[usize::from(i)].as_ref())
                            .is_some_and(|d| d.verb() == v.word());
                    if chrome::button(ui, v.word(), Kind::Plain, on, can, None, u)
                        .on_hover_text(format!("{} ({})", v.hint(), v.key()))
                        .clicked()
                    {
                        picked = Some(v);
                    }
                }
                // USE ITEM, a bonus beside the three: it can be picked while someone of yours
                // carries something.
                let v = Verb::UseItem;
                let hover = if carriers.is_empty() {
                    format!("{} Nobody of yours carries anything just now.", v.hint())
                } else {
                    format!("{} ({})", v.hint(), v.key())
                };
                if chrome::button(
                    ui,
                    v.word(),
                    Kind::Plain,
                    self.table.verb == Some(v),
                    !carriers.is_empty(),
                    None,
                    u,
                )
                .on_hover_text(hover)
                .clicked()
                {
                    picked = Some(v);
                }
            });
            if let Some(v) = picked {
                self.table.take_token(v);
            }
            ui.add_space(u * 3.0);
        }
        for i in 0..PLAN_SIZE as u8 {
            let chosen = giving && self.table.selected == Some(i) && self.table.verb.is_some();
            let order = self.table.orders[i as usize].clone();
            let (title, line) = match &order {
                Some(order) => (
                    order.verb().to_owned(),
                    speak_order(self.table.m.as_ref().expect("on"), you, order),
                ),
                None if chosen => ("Now!".to_owned(), "Click the board to place it.".to_owned()),
                None => ("Empty".to_owned(), "Not given yet.".to_owned()),
            };
            let has_order = order.is_some();
            let facing = match &order {
                Some(Directive::Hold { facing, .. }) => Some(*facing),
                _ => None,
            };
            let (response, (cleared, turned)) = card(
                ui,
                Id::new(("order-card", i)),
                chosen,
                true,
                giving && has_order,
                u,
                |ui| {
                    ui.horizontal(|ui| {
                        token(ui, i, has_order || chosen, facing, u);
                        // Room kept on the right for the buttons, so the words wrap short of them.
                        let mut buttons = Vec::new();
                        if giving && has_order {
                            buttons.push("clear");
                            if facing.is_some() {
                                buttons.push("turn");
                            }
                        }
                        let room = ui.available_width() - beside(ui, &buttons, u);
                        ui.vertical(|ui| {
                            ui.set_max_width(room);
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(hand_job(
                                &title,
                                chrome::LABEL,
                                if has_order { ink::PAGE } else { ink::MUTED },
                            ));
                            let words: String = line
                                .split_once(": ")
                                .map_or(line.clone(), |(_, rest)| rest.to_owned());
                            let words = words
                                .strip_prefix(&format!("{title} "))
                                .map_or(words.clone(), str::to_owned);
                            ui.label(RichText::new(words).size(15.0).color(if has_order {
                                ink::PAGE
                            } else {
                                ink::MUTED
                            }));
                        });
                        let (mut cleared, mut turned) = (false, false);
                        if giving && has_order {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if chrome::button(ui, "clear", Kind::Back, false, true, None, u)
                                        .on_hover_text("Take this order back")
                                        .clicked()
                                    {
                                        cleared = true;
                                    }
                                    // A HOLD turns a quarter clockwise each click, as R does.
                                    if facing.is_some()
                                        && chrome::button(ui, "turn", Kind::Plain, false, true, None, u)
                                            .on_hover_text(
                                                "Turn which way it faces, a quarter clockwise (R, or the mouse wheel over the board)",
                                            )
                                            .clicked()
                                    {
                                        turned = true;
                                    }
                                },
                            );
                        }
                        (cleared, turned)
                    })
                    .inner
                },
            );
            if cleared {
                self.table.clear_order(i);
                if self.table.selected == Some(i) {
                    self.table.put_down();
                }
            } else if turned {
                if self.table.turn_order(i, false) {
                    self.sound.play(Cue::Select);
                }
            } else if giving && response.clicked() {
                // A placed order picked up again, to add squares or move it.
                self.table.select(i);
                if let Some(v) = order.as_ref().map(Verb::of) {
                    self.table.choose_verb(v);
                }
            }
            ui.add_space(u * 2.0);
        }
        // The bonus USE ITEMs: one for each of the team carrying something, beside the three.
        for by in self.table.carriers() {
            let Some(m) = self.table.m.as_ref() else {
                break;
            };
            let slot = Slot::new(you, by);
            let given = self.table.uses[usize::from(by)];
            let chosen =
                giving && self.table.verb == Some(Verb::UseItem) && self.table.user == Some(by);
            let line = match given {
                Some(at) => speak_order(m, you, &Directive::UseItem { by, at }),
                None if chosen => "Click a green square in reach.".to_owned(),
                None => format!("{}: not used yet.", carried(m, slot)),
            };
            let words = line
                .strip_prefix("USE ITEM: ")
                .map_or(line.clone(), str::to_owned);
            let (response, cleared) = card(
                ui,
                Id::new(("use-card", by)),
                chosen,
                true,
                giving,
                u,
                |ui| {
                    ui.horizontal(|ui| {
                        bonus_token(ui, given.is_some() || chosen, u);
                        let buttons: &[&str] = if giving && given.is_some() {
                            &["clear"]
                        } else {
                            &[]
                        };
                        let room = ui.available_width() - beside(ui, buttons, u);
                        ui.vertical(|ui| {
                            ui.set_max_width(room);
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(hand_job(
                                "USE ITEM · bonus",
                                chrome::LABEL,
                                if given.is_some() {
                                    ink::PAGE
                                } else {
                                    ink::MUTED
                                },
                            ));
                            ui.label(RichText::new(words).size(15.0).color(if given.is_some() {
                                ink::PAGE
                            } else {
                                ink::MUTED
                            }));
                        });
                        let mut cleared = false;
                        if giving && given.is_some() {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if chrome::button(ui, "clear", Kind::Back, false, true, None, u)
                                        .on_hover_text("Take this USE ITEM back")
                                        .clicked()
                                    {
                                        cleared = true;
                                    }
                                },
                            );
                        }
                        cleared
                    })
                    .inner
                },
            );
            if cleared {
                self.table.clear_use(by);
            } else if giving && response.clicked() && self.table.pick_user(by) {
                self.sound.play(Cue::Select);
            }
            ui.add_space(u * 2.0);
        }
        if giving {
            ui.add_space(u * 2.0);
            let width = ui.available_width();
            if chrome::button(
                ui,
                "Lock in my orders ›",
                Kind::Primary,
                false,
                true,
                Some(width),
                u,
            )
            .on_hover_text(if self.table.lesson.is_some() {
                "Enter. With no orders at all, each of the team chooses for itself."
            } else {
                "Enter. With no orders at all, each of the team chooses for itself when the timer runs out."
            })
            .clicked()
            {
                self.lock_quietly(now);
            }
        }
        if sent && self.table.shown.is_none() {
            ui.add_space(u * 3.0);
            sticky(
                ui,
                &format!("Orders sent. Waiting for {}…", self.table.other_short()),
                u,
            );
        }
        if let Some(notice) = self.table.notice.clone() {
            ui.add_space(u * 3.0);
            sticky(ui, &notice, u);
        }
        ui.add_space(u * 4.0);
        dashed_rule(ui, u);
        if self.table.lesson.is_some() {
            self.lesson_notes(ui, u);
        } else {
            self.field_guide(ui, ctx, u);
        }
    }

    /// A match with a friend whose room closed partway through: it cannot go on, earns nothing,
    /// and the way out is back to the station.
    fn room_gone(&mut self, ui: &mut egui::Ui, u: f32) {
        chrome::kicker(ui, "The room has closed");
        ui.add_space(u * 2.0);
        if let Some(notice) = self.table.notice.clone() {
            sticky(ui, &notice, u);
            ui.add_space(u * 3.0);
        }
        ruled::write(
            ui,
            RichText::new(format!(
                "The match with {} cannot go on without it, so it ended here and earned nothing.",
                self.table.other_short()
            ))
            .size(15.0),
        );
        ui.add_space(u * 5.0);
        if chrome::button(
            ui,
            "‹ Back to the station",
            Kind::Back,
            false,
            true,
            None,
            u,
        )
        .on_hover_text("Choose again where your companions are off to")
        .clicked()
        {
            self.table.leave_room();
            self.table.shown = None;
            self.portraits.clear();
            self.fresh_match();
            self.notebook_open = false;
            self.back_to_station();
        }
    }

    /// In a Field Lesson, its notes where the field guide would be: what it teaches, and how
    /// to do it.
    fn lesson_notes(&mut self, ui: &mut egui::Ui, u: f32) {
        let Some(run) = &self.table.lesson else {
            return;
        };
        let lesson = run.lesson;
        chrome::kicker(
            ui,
            &format!(
                "Lesson {} of {} \u{b7} {}",
                lesson.number(),
                Lesson::ALL.len(),
                lesson.title()
            ),
        );
        ui.add_space(u * 2.0);
        egui::ScrollArea::vertical()
            .id_salt("lesson-notes")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for note in lesson.notes() {
                    ruled::write(ui, RichText::new(*note).size(15.0).color(ink::PAGE));
                    ui.add_space(u * 2.0);
                }
            });
    }

    /// A Field Lesson's end: what it showed, or what to try, and where next.
    /// How it ended, pinned to the page: a rosette, with the result lettered on a ribbon beside
    /// it, as wide as the page has room for.
    fn award(&mut self, ui: &mut egui::Ui, award: crate::keepsake::Award, title: &str, u: f32) {
        use crate::keepsake::{AWARD_H, AWARD_W};
        let ctx = ui.ctx().clone();
        let w = ((ui.available_width() / u).floor() as i32).clamp(120, 280);
        let ribbon = (AWARD_W as i32 + 8, 8, w - AWARD_W as i32 - 15, 18);
        let picture = self.painted(&ctx, format!("award-{award:?}-{w}"), |_| {
            let mut canvas = Canvas::new(w as u32, AWARD_H);
            crate::heraldry::ribbon(&mut canvas, ribbon);
            formiga_art::paint::blit(&mut canvas, &crate::keepsake::award(award), 0, 0);
            canvas
        });
        let rect = registry::place(ui, &picture, 1.0, u).0;
        let (rx, ry, rw, rh) = ribbon;
        let area = Rect::from_min_size(
            rect.min + vec2(rx as f32, ry as f32) * u,
            vec2(rw as f32, rh as f32) * u,
        );
        registry::letter(ui.painter(), area, title, registry::ON_RIBBON, 0.6);
    }

    fn lesson_result(&mut self, ui: &mut egui::Ui, u: f32) {
        let Some(run) = &self.table.lesson else {
            return;
        };
        let lesson = run.lesson;
        let (title, words, learned) = match &run.verdict {
            Verdict::Learned(words) => ("Learned!", words.clone(), true),
            Verdict::TryAgain(words) => ("Not yet", words.clone(), false),
            Verdict::Going => ("Not yet", String::new(), false),
        };
        chrome::kicker(
            ui,
            &format!("Lesson {} \u{b7} {}", lesson.number(), lesson.title()),
        );
        let award = if learned {
            crate::keepsake::Award::Learned
        } else {
            crate::keepsake::Award::NotYet
        };
        self.award(ui, award, title, u);
        ruled::write(ui, RichText::new(words).size(15.0));
        ui.add_space(u * 5.0);
        let width = ui.available_width();
        let next = lesson.next().filter(|_| learned);
        let mut go = None;
        let mut to_list = false;
        let mut to_station = false;
        if let Some(next) = next
            && chrome::button(
                ui,
                &format!("Next: {}", next.title()),
                Kind::Primary,
                false,
                true,
                Some(width),
                u,
            )
            .clicked()
        {
            go = Some(next);
        }
        let again = if learned {
            "Play it again"
        } else {
            "Try again"
        };
        let kind = if learned { Kind::Plain } else { Kind::Primary };
        if chrome::button(ui, again, kind, false, true, Some(width), u).clicked() {
            go = Some(lesson);
        }
        if chrome::button(
            ui,
            "All the lessons",
            Kind::Plain,
            false,
            true,
            Some(width),
            u,
        )
        .clicked()
        {
            to_list = true;
        }
        if chrome::button(
            ui,
            "‹ Back to the station",
            Kind::Back,
            false,
            true,
            None,
            u,
        )
        .clicked()
        {
            to_station = true;
        }
        if learned && next.is_none() {
            ui.add_space(u * 5.0);
            chrome::kicker(ui, "After the lessons");
            for line in AFTER_LESSONS {
                ruled::write(ui, RichText::new(format!("\u{b7} {line}")).size(15.0));
            }
        }
        ui.add_space(u * 3.0);
        ruled::write(
            ui,
            RichText::new("A lesson: nothing is kept.")
                .size(15.0)
                .italics()
                .color(ink::MUTED),
        );
        ui.add_space(u * 4.0);
        let read = if self.account_open {
            "Close the whole match"
        } else {
            "Read the whole match, turn by turn"
        };
        if chrome::button(ui, read, Kind::Plain, false, true, Some(width), u).clicked() {
            self.toggle_account();
        }
        if go.is_some() || to_list || to_station {
            self.table.shown = None;
            self.portraits.clear();
            self.fresh_match();
            self.notebook_open = false;
        }
        if let Some(lesson) = go {
            // Your own three walk a little differently each time; the others are the same.
            self.table.seed = self.table.seed.wrapping_add(1);
            self.lesson = lesson;
            self.begin_lesson();
        } else if to_list {
            self.open_journal(Outing::Lessons, Step::Lessons);
            self.leaf(false);
        } else if to_station {
            self.back_to_station();
        }
    }

    /// A glance at the arena and the rules under the orders: one topic at a time (how to win,
    /// the orders, the board's marks), each a few short lines with a picture, the detail on
    /// hover. The controls are on the board itself.
    fn field_guide(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        if self.keys.is_empty() {
            self.keys = scene::Key::ALL
                .iter()
                .enumerate()
                .map(|(i, key)| texture(ctx, &format!("key-{i}"), &key.picture()))
                .collect();
        }
        let arena = &self.table.arena;
        ruled::ruled_row(ui, |ui| {
            chrome::kicker(ui, &format!("The {}", arena.name));
        })
        .response
        .on_hover_text(crate::garden::Dressing::of(arena).blurb());
        ui.add_space(u * 2.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = u * 2.0;
            for topic in Guide::ALL {
                if chrome::button(
                    ui,
                    topic.word(),
                    Kind::Plain,
                    self.guide == topic,
                    true,
                    None,
                    u,
                )
                .clicked()
                {
                    self.guide = topic;
                }
            }
        });
        ui.add_space(u * 3.0);
        let ruleset = self.table.ruleset;
        let key =
            |k: scene::Key| Glyph::Key(scene::Key::ALL.iter().position(|x| *x == k).unwrap_or(0));
        let rows: Vec<(Glyph, String, String, String)> = match self.guide {
            Guide::Win => vec![
                (
                    key(scene::Key::Beacon),
                    "Light the beacon".to_owned(),
                    format!("hold the yard alone, {} ticks", ruleset.capture_ticks),
                    "Keep someone steady in the yard while none of theirs is there. Each tick of it lights a lamp; light them all and you win.".to_owned(),
                ),
                (
                    Glyph::Stars,
                    "Or make them dizzy".to_owned(),
                    "all three of theirs".to_owned(),
                    "A Formiga out of composure sits out, dizzy, for the rest of the match. Nobody is hurt.".to_owned(),
                ),
                (
                    Glyph::Watch,
                    format!("Turn {}", ruleset.turn_cap),
                    "nearest the beacon wins".to_owned(),
                    "If time runs out: the most beacon lamps, then the most composure left, then the most still standing.".to_owned(),
                ),
            ],
            Guide::Orders => vec![
                (
                    key(scene::Key::Press),
                    "PRESS".to_owned(),
                    Verb::Press.gloss().to_owned(),
                    Verb::Press.hint().to_owned(),
                ),
                (
                    key(scene::Key::Holding),
                    "HOLD".to_owned(),
                    Verb::Hold.gloss().to_owned(),
                    Verb::Hold.hint().to_owned(),
                ),
                (
                    key(scene::Key::Trick),
                    "TRICK".to_owned(),
                    Verb::Trick.gloss().to_owned(),
                    Verb::Trick.hint().to_owned(),
                ),
                (
                    Glyph::Stars,
                    "INTERACT".to_owned(),
                    Verb::Interact.gloss().to_owned(),
                    Verb::Interact.hint().to_owned(),
                ),
                (
                    Glyph::Stars,
                    "USE ITEM".to_owned(),
                    Verb::UseItem.gloss().to_owned(),
                    Verb::UseItem.hint().to_owned(),
                ),
                (
                    key(scene::Key::Move),
                    "Up to three".to_owned(),
                    "PRESS beats TRICK, HOLD beats PRESS, TRICK beats HOLD".to_owned(),
                    "Name a square in two orders and more of the team goes to it. Who goes where is up to them: who is nearest, what each is already doing, and a little of who each one is. USE ITEMs come on top of the three.".to_owned(),
                ),
            ],
            Guide::Marks => scene::Key::ALL
                .iter()
                .map(|k| (key(*k), k.words().to_owned(), String::new(), String::new()))
                .collect(),
        };
        // Scrolls when the window is too short for it all.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (glyph, label, gloss, hover) in rows {
                    let response = ui
                        .horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = u * 3.0;
                            self.glyph(ui, ctx, glyph, u);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.label(
                                    RichText::new(&label).size(15.0).strong().color(ink::PAGE),
                                );
                                if !gloss.is_empty() {
                                    ui.label(RichText::new(&gloss).size(15.0).color(ink::MUTED));
                                }
                            });
                        })
                        .response;
                    if !hover.is_empty() {
                        response.on_hover_text(hover);
                    }
                    ui.add_space(u * 1.5);
                }
            });
    }

    /// A row's picture in the field guide: one of the board's marks, stars, or a little watch.
    fn glyph(&self, ui: &mut egui::Ui, ctx: &egui::Context, glyph: Glyph, u: f32) {
        let ppp = ctx.pixels_per_point();
        let px = ppp.round().max(1.0) / ppp;
        let size = vec2(34.0, 24.0) * px;
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter();
        match glyph {
            Glyph::Key(i) => {
                painter.image(
                    self.keys[i].id(),
                    rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            Glyph::Stars => {
                for (k, dx) in [-1.0f32, 0.0, 1.0].into_iter().enumerate() {
                    let c = rect.center() + vec2(dx * u * 6.0, if k == 1 { -u * 2.0 } else { u });
                    star(painter, c, u * 3.2, ink::BRASS_LIGHT, ink::BRASS_EDGE);
                }
            }
            Glyph::Watch => {
                let c = rect.center();
                let r = u * 6.0;
                painter.circle_filled(c, r, ink::BRASS);
                painter.circle_stroke(c, r, Stroke::new(u * 0.7, ink::BRASS_EDGE));
                painter.circle_filled(c, r - u * 1.5, ink::CARD);
                painter.line_segment(
                    [c, c + vec2(0.0, -r + u * 2.5)],
                    Stroke::new(u * 0.8, ink::LINE),
                );
                painter.line_segment(
                    [c, c + vec2(r - u * 3.0, 0.0)],
                    Stroke::new(u * 0.8, ink::LINE),
                );
            }
        }
    }

    fn result(&mut self, ui: &mut egui::Ui, u: f32) {
        if self.table.lesson.is_some() {
            self.lesson_result(ui, u);
            return;
        }
        let you = self.table.you;
        let outcome = self.table.outcome().expect("finished");
        let tally = self.table.m.as_ref().map_or(Tally::Beacon, tally);
        let ahead_you = match tally {
            Tally::Beacon => "Time ran out with your team ahead on the beacon.",
            Tally::Composure => {
                "Time ran out level on the beacon, with more composure left across your team."
            }
            Tally::Standing => {
                "Time ran out level on the beacon and on composure, with more of your team standing."
            }
        };
        let ahead_them = match tally {
            Tally::Beacon => "Time ran out with them ahead on the beacon.",
            Tally::Composure => {
                "Time ran out level on the beacon, with more composure left across their team."
            }
            Tally::Standing => {
                "Time ran out level on the beacon and on composure, with more of them standing."
            }
        };
        // The other team is a rival's, by name, a friend's, named, or the visitors.
        let (them, wins, dizzy_them) = match (self.table.rival, self.table.other_handler.clone()) {
            (Some(rival), _) => (
                rival.short.to_owned(),
                "wins",
                format!("{}'s three are all sitting out, dizzy.", rival.short),
            ),
            (None, Some(name)) => {
                let dizzy = format!("{name}'s three are all sitting out, dizzy.");
                (name, "wins", dizzy)
            }
            (None, None) => (
                "The visitors".to_owned(),
                "win",
                "The visitors are all sitting out, dizzy.".to_owned(),
            ),
        };
        let beacon_them = format!("{them} lit the beacon");
        let knockout_them = format!("{them} {wins}");
        let forfeit_them = format!("{them} {wins} by forfeit");
        let tally_them = format!("{them} {wins} on the tally");
        let (title, why): (&str, &str) = match (outcome.winner, outcome.victory) {
            (Some(side), Some(Victory::Capture)) if side == you => {
                ("We lit the beacon!", "My team held the yard long enough!")
            }
            (Some(side), Some(Victory::Knockout)) if side == you => ("We win!", &dizzy_them),
            (Some(side), Some(Victory::Forfeit)) if side == you => (
                "We win by forfeit!",
                "My friend left the match, or was away from it too long.",
            ),
            (Some(side), _) if side == you => ("We win on the tally!", ahead_you),
            (Some(_), Some(Victory::Capture)) => (&beacon_them, "They held the yard long enough."),
            (Some(_), Some(Victory::Knockout)) => (
                &knockout_them,
                "My three are all sitting out, dizzy, and none the worse for it.",
            ),
            (Some(_), Some(Victory::Forfeit)) => {
                (&forfeit_them, "The relay lost touch with me for too long.")
            }
            (Some(_), _) => (&tally_them, ahead_them),
            (None, _) => ("A draw!", "Nobody could be split."),
        };
        let award = match outcome.winner {
            Some(side) if side == you => crate::keepsake::Award::Won(
                crate::rewards::cup_for(&self.table.arena.id)
                    .map(|c| crate::rewards::cup_index(c.id)),
            ),
            Some(_) => crate::keepsake::Award::Lost,
            None => crate::keepsake::Award::Drawn,
        };
        self.award(ui, award, title, u);
        // At the cap, the counts that decided it, yours first.
        let counts = self
            .table
            .m
            .as_ref()
            .filter(|_| outcome.victory == Some(Victory::TurnCap) || outcome.winner.is_none())
            .filter(|m| m.board.turn >= m.ruleset.turn_cap)
            .map(|m| {
                let (a, b) = (m.cap_tally(you), m.cap_tally(you.other()));
                format!(
                    "Beacon {} to {} \u{b7} composure {} to {} \u{b7} standing {} to {}",
                    a.0, b.0, a.1, b.1, a.2, b.2
                )
            });
        ruled::write(ui, RichText::new(why).size(15.0));
        if let Some(counts) = counts {
            ruled::write(ui, RichText::new(counts).size(15.0).color(ink::FOREST));
        }
        if let Some(reason) = &self.table.ended_because {
            ruled::write(ui, RichText::new(reason).size(15.0).color(ink::MUTED));
        }
        // The rival met, by their pennant, and how the meetings stand.
        if let (Some(rival), Some(meeting)) = (self.table.rival, self.table.meeting.clone()) {
            let ctx = ui.ctx().clone();
            let flag = self.flag(&ctx, crate::identity::Style::of(&rival.pennant()));
            ui.horizontal(|ui| {
                let fh = u * 12.0;
                let size = vec2(flag.size_vec2().x * fh / flag.size_vec2().y, fh);
                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                ui.painter().image(
                    flag.id(),
                    rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                ui.label(
                    RichText::new(meeting)
                        .font(formiga_pencil::hand(15.0))
                        .color(ink::FOREST),
                );
            });
        }
        // How the event stands, stamped.
        if let Some(event) = self.table.event_line.clone() {
            let (word, colour) = if event.starts_with("You won")
                || event.starts_with("You defended")
                || event.starts_with("You are the")
            {
                ("Champion!", ink::FOREST)
            } else if event.starts_with("Through") {
                ("Through!", ink::FOREST)
            } else if event.starts_with("Out") || event.contains("took your title") {
                ("Out", ink::STAMP)
            } else {
                ("Event", ink::MUTED)
            };
            let width = ui.available_width();
            let (row, _) = ui.allocate_exact_size(vec2(width, u * 22.0), Sense::hover());
            let painter = ui.painter().clone();
            let stamped = chrome::stamp(
                &painter,
                pos2(row.left() + u * 26.0, row.center().y),
                word,
                chrome::HAND,
                colour,
                Some(ink::CARD.gamma_multiply(0.9)),
                (-5f32).to_radians(),
                u,
            );
            let text = painter.layout(
                event,
                formiga_pencil::hand_bold(15.0),
                ink::PAGE,
                row.right() - stamped.right() - u * 6.0,
            );
            painter.galley(
                pos2(
                    stamped.right() + u * 6.0,
                    row.center().y - text.size().y / 2.0,
                ),
                text,
                ink::PAGE,
            );
        }
        ruled::write(
            ui,
            RichText::new(format!("After {} turns", outcome.turn))
                .size(15.0)
                .color(ink::MUTED),
        );
        ui.add_space(u * 5.0);
        let width = ui.available_width();
        if chrome::button(
            ui,
            "Play again! ›",
            Kind::Primary,
            false,
            true,
            Some(width),
            u,
        )
        .clicked()
        {
            self.table.seed = self
                .table
                .seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.table.leave_when_settled();
            self.table.shown = None;
            self.portraits.clear();
            self.fresh_match();
            self.notebook_open = false;
            let outing = self.outing;
            self.open_journal(outing, Step::Who);
            // Back through the notebook to choosing who goes.
            self.leaf(false);
        }
        if chrome::button(
            ui,
            "‹ Back to the station",
            Kind::Back,
            false,
            true,
            None,
            u,
        )
        .on_hover_text("Choose again where your companions are off to")
        .clicked()
        {
            self.table.leave_when_settled();
            self.table.shown = None;
            self.portraits.clear();
            self.fresh_match();
            self.notebook_open = false;
            self.back_to_station();
        }
        if !self.table.earned.is_empty() {
            ui.add_space(u * 5.0);
            chrome::kicker(ui, "Earned!");
            for line in self.table.earned.clone() {
                earned_line(ui, &line, u);
            }
            let kept = if self.table.session.is_some() {
                "Kept in your colony when Battle closes."
            } else {
                "A rehearsal: nothing is kept."
            };
            ruled::write(
                ui,
                RichText::new(kept).size(15.0).italics().color(ink::MUTED),
            );
        }
        ui.add_space(u * 4.0);
        let width = ui.available_width();
        let read = if self.account_open {
            "Close the whole match"
        } else {
            "Read the whole match, turn by turn"
        };
        if chrome::button(ui, read, Kind::Plain, false, true, Some(width), u).clicked() {
            self.toggle_account();
        }
    }

    /// The Handler's Notebook for the three on the board: what each has been seen to do.
    fn notebook(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, u: f32) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for i in 0..TEAM_SIZE {
                    let contender = self.table.board_teams[self.table.you.index()][i].clone();
                    let texture = self.portrait(ctx, &contender);
                    let page = self.table.page(contender.traveler.id.0);
                    card(
                        ui,
                        Id::new(("notebook-card", i)),
                        false,
                        true,
                        false,
                        u,
                        |ui| {
                            ui.horizontal_top(|ui| {
                                portrait_mat(ui, &texture, 44.0, u, true);
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(&contender.traveler.name)
                                            .size(15.0)
                                            .strong()
                                            .color(ink::PAGE),
                                    );
                                    let Some(page) = page else {
                                        ui.label(
                                            RichText::new(
                                                "Nothing noted yet: play a match together.",
                                            )
                                            .size(15.0)
                                            .color(ink::MUTED),
                                        );
                                        return;
                                    };
                                    let played = if page.matches == 1 {
                                        "Played once".to_owned()
                                    } else {
                                        format!("Played {} times, won {}", page.matches, page.wins)
                                    };
                                    ui.label(hand_job(&played, chrome::LABEL, ink::MUTED));
                                    let notes = page.notes();
                                    if notes.is_empty() {
                                        ui.label(
                                            RichText::new("No habits noted yet.")
                                                .size(15.0)
                                                .italics()
                                                .color(ink::MUTED),
                                        );
                                    }
                                    for note in notes.iter().take(4) {
                                        ui.label(RichText::new(format!("· {note}")).size(15.0));
                                    }
                                    for moment in page.moments.iter().rev().take(2) {
                                        ui.label(
                                            RichText::new(moment.as_str())
                                                .size(15.0)
                                                .italics()
                                                .color(ink::MUTED),
                                        );
                                    }
                                });
                            });
                        },
                    );
                    ui.add_space(u * 3.0);
                }
            });
    }
}

/// How large the words along the foot are written, in points.
const FOOT_WRITING: f32 = 15.0;

/// Where the words along the foot rest: between the pages' bottom edge, which runs on two pixels
/// below them, and the stitching five pixels up from the cover's edge. Their tallest letters (an
/// apostrophe reaches 0.85 of their size above where they rest, a tail 0.25 below) are kept a
/// pixel clear of the pages, and the letters are centred between the two as far as they fit.
fn foot_baseline(layout: &Layout, size: f32) -> f32 {
    let u = layout.unit;
    let (above, below) = (size * 0.85, size * 0.25);
    let clear = layout.foot.min.y + u * 3.0;
    let rows = (layout.full.height() / u).ceil();
    let stitching = layout.full.min.y + (rows - 5.0) * u;
    ((clear + stitching + above - below) / 2.0).max(clear + above)
}
