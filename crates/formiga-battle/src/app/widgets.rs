//! The little painted pieces the window's pages are made of, free of the app: a card round
//! whatever is written on it, a sticky note, a dashed rule, a rosette, a ribbon slip, the
//! composure pips, the beacon's lamps, the turn's watch, a name tag over the board, the order
//! tokens, and the controls card. Each draws into the `Ui` or `Painter` it is handed and keeps
//! nothing of its own, so any page can use it.

use super::*;

/// The width a row of these little buttons takes at the right of a card, with the gap before
/// them: nothing for none.
pub(super) fn beside(ui: &egui::Ui, buttons: &[&str], u: f32) -> f32 {
    let gap = ui.spacing().item_spacing.x;
    buttons
        .iter()
        .map(|b| chrome::button_size(ui, b, Kind::Back, u).x + gap)
        .sum()
}

/// A card on the page round whatever `add` writes on it, painted under the writing: card
/// coloured on a hard shadow with a stepped edge, mint when chosen, faded for someone sitting
/// out. A `lively` card can be clicked, and lifts a unit when pointed at. The card senses its
/// click beneath what is written on it, so a button on the card takes its own click.
pub(super) fn card<R>(
    ui: &mut egui::Ui,
    id: Id,
    chosen: bool,
    up: bool,
    lively: bool,
    u: f32,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> (egui::Response, R) {
    let mark = crate::feel::mark(ui);
    let under = ui.painter().add(egui::Shape::Noop);
    let sense = if lively {
        Sense::click()
    } else {
        Sense::hover()
    };
    let inner = ui.scope_builder(egui::UiBuilder::new().id_salt(id).sense(sense), |ui| {
        egui::Frame::new()
            .inner_margin(egui::Margin::same((u * 5.0).round() as i8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if !up {
                    ui.multiply_opacity(0.62);
                }
                add(ui)
            })
            .inner
    });
    let response = inner.response;
    let rect = response.rect;
    let pointed = lively && up && response.hovered();
    let press = if lively && up {
        crate::feel::press(ui.ctx(), id, &response, crate::feel::Feel::Select)
    } else {
        crate::feel::Press::REST
    };
    let lift = press.hover;
    let fill = if chosen {
        ink::MINT
    } else if pointed {
        Color32::from_rgb(255, 252, 240)
    } else {
        ink::CARD
    };
    let edge = if chosen {
        ink::FOREST
    } else {
        ink::LINE.gamma_multiply(0.55)
    };
    let face = rect.translate(vec2(0.0, -(lift * u).round()));
    let shadow = rect.translate(vec2(u, u));
    let mut shapes = vec![egui::Shape::rect_filled(shadow.shrink(u), 0.0, ink::SHADOW)];
    shapes.push(egui::Shape::rect_filled(face.shrink(u), 0.0, fill));
    let (l, r, t, b) = (face.left(), face.right(), face.top(), face.bottom());
    for strip in [
        Rect::from_min_max(pos2(l + u, t), pos2(r - u, t + u)),
        Rect::from_min_max(pos2(l + u, b - u), pos2(r - u, b)),
        Rect::from_min_max(pos2(l, t + u), pos2(l + u, b - u)),
        Rect::from_min_max(pos2(r - u, t + u), pos2(r, b - u)),
    ] {
        shapes.push(egui::Shape::rect_filled(strip, 0.0, edge));
    }
    ui.painter().set(under, egui::Shape::Vec(shapes));
    crate::feel::transform_since(ui, mark, press.transform(rect, u));
    (response, inner.inner)
}

/// A portrait on a little mat of paper, so it sits on the card like a pasted-in photo.
pub(super) fn portrait_mat(
    ui: &mut egui::Ui,
    texture: &egui::TextureHandle,
    size: f32,
    u: f32,
    up: bool,
) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect.shrink(u), 0.0, Color32::from_rgb(0xf3, 0xe9, 0xcc));
    chrome::stepped(painter, rect, u, ink::LINE.gamma_multiply(0.3));
    let tint = if up {
        Color32::WHITE
    } else {
        Color32::from_gray(170)
    };
    painter.image(
        texture.id(),
        rect.shrink(u * 3.0),
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        tint,
    );
}

/// What Battle has to say right now, on a sticky note.
pub(super) fn sticky(ui: &mut egui::Ui, text: &str, u: f32) {
    let width = ui.available_width();
    let galley = ui.painter().layout(
        text.to_owned(),
        FontId::proportional(15.0),
        ink::PAGE,
        width - u * 12.0,
    );
    let size = vec2(width, galley.size().y + u * 10.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    chrome::card(
        ui.painter(),
        rect,
        u,
        ink::NOTE,
        Color32::from_rgb(0xb8, 0x9a, 0x52),
    );
    // A strip of tape across its top.
    let tape = Rect::from_center_size(
        pos2(rect.center().x, rect.min.y + u),
        vec2(u * 18.0, u * 4.0),
    );
    ui.painter().rect_filled(
        tape,
        0.0,
        Color32::from_rgba_unmultiplied(255, 255, 255, 110),
    );
    ui.painter()
        .galley(rect.min + vec2(u * 6.0, u * 5.0), galley, ink::PAGE);
}

/// A dashed rule across the page, three on and two off, as Hill rules its cards.
pub(super) fn dashed_rule(ui: &mut egui::Ui, u: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), u * 3.0), Sense::hover());
    let mut x = rect.min.x;
    while x < rect.max.x {
        let w = (u * 3.0).min(rect.max.x - x);
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(x, rect.center().y), vec2(w, u)),
            0.0,
            Color32::from_rgb(0xd8, 0xc8, 0xa0),
        );
        x += u * 5.0;
    }
}

/// A rosette for a cup win: brass when won, an empty ring when still to come.
pub(super) fn rosette(ui: &mut egui::Ui, cup: usize, won: bool, u: f32) {
    let picture = kept_picture(ui.ctx(), &format!("small-rosette-{cup}-{won}"), || {
        crate::keepsake::small_rosette(cup, won)
    });
    registry::place(ui, &picture, 1.0, u);
}

/// A ribbon slip filling `rect`, its folded ends showing either side: the notebook's green, or a
/// team's cloth.
pub(super) fn slip_ribbon(
    painter: &egui::Painter,
    rect: Rect,
    cloth: Option<formiga_art::paint::Ramp>,
    u: f32,
) {
    let w = (rect.width() / u).round() as i32;
    let h = (rect.height() / u).round() as i32;
    let key = format!("slip-{w}-{h}-{:?}", cloth.map(|c| c.base));
    let picture = kept_picture(painter.ctx(), &key, || {
        let mut canvas = Canvas::new((w + 14) as u32, (h + 2) as u32);
        let area = (7, 0, w, h);
        match cloth {
            Some(cloth) => crate::heraldry::ribbon_in(&mut canvas, area, cloth),
            None => crate::heraldry::ribbon(&mut canvas, area),
        }
        canvas
    });
    let min = rect.min - vec2(7.0 * u, 0.0);
    painter.image(
        picture.id(),
        Rect::from_min_size(min, picture.size_vec2() * u),
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

/// A line of what a match earned, beside its keepsake.
pub(super) fn earned_line(ui: &mut egui::Ui, line: &str, u: f32) {
    let (keepsake, words) = crate::rewards::keepsake_of(line);
    let picture = kept_picture(ui.ctx(), &format!("keepsake-{keepsake:?}"), || {
        crate::keepsake::icon(keepsake)
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = u * 3.0;
        registry::place(ui, &picture, 1.0, u);
        ui.add(egui::Label::new(RichText::new(words).size(15.0)).wrap());
    });
}

/// A padlock on a place still to open, saying when pointed at what opens it.
pub(super) fn padlock(ui: &mut egui::Ui, why: &str, u: f32) {
    let picture = kept_picture(ui.ctx(), "padlock", crate::keepsake::padlock);
    registry::place(ui, &picture, 1.0, u).1.on_hover_text(why);
}

/// Composure as three pips: filled in the team's colour, empty as rings.
pub(super) fn pips(ui: &mut egui::Ui, composure: u8, team: formiga_art::paint::Ramp, u: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(u * 22.0, u * 8.0), Sense::hover());
    for k in 0..3u8 {
        let centre = pos2(
            rect.min.x + u * 4.0 + f32::from(k) * u * 7.0,
            rect.center().y,
        );
        if k < composure {
            ui.painter()
                .circle_filled(centre, u * 2.8, color(team.base));
            ui.painter()
                .circle_stroke(centre, u * 2.8, Stroke::new(u * 0.7, color(team.edge)));
            ui.painter()
                .circle_filled(centre - vec2(u, u), u * 0.8, color(team.shine));
        } else {
            ui.painter()
                .circle_stroke(centre, u * 2.6, Stroke::new(u * 0.7, ink::MUTED));
        }
    }
}

/// A team's progress toward lighting the beacon: a row of little lamps on the leather, lit one by
/// one; while both teams hold the yard, the row is crossed with a hatch and gains nothing.
pub(super) fn beacon_meter(
    ui: &mut egui::Ui,
    progress: u8,
    of: u8,
    team: formiga_art::paint::Ramp,
    paused: bool,
    u: f32,
    now: f32,
) {
    let lamp = vec2(u * 5.0, u * 8.0);
    let gap = u * 1.5;
    let count = of.max(1);
    let width = f32::from(count) * (lamp.x + gap) - gap + u * 4.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, lamp.y + u * 4.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect.shrink(u), 0.0, Color32::from_rgb(0x2f, 0x1c, 0x12));
    chrome::stepped(painter, rect, u, Color32::from_rgb(0x87, 0x60, 0x4a));
    for k in 0..count {
        let at = pos2(
            rect.min.x + u * 2.0 + f32::from(k) * (lamp.x + gap),
            rect.min.y + u * 2.0,
        );
        let r = Rect::from_min_size(at, lamp);
        if k < progress {
            painter.rect_filled(r, 0.0, color(team.base));
            painter.rect_filled(
                Rect::from_min_size(r.min, vec2(r.width(), u)),
                0.0,
                color(team.shine),
            );
            // The newest lamp glows a little brighter, on and off.
            if k + 1 == progress && !paused && (now * 3.0).sin() > 0.0 {
                painter.rect_filled(r.shrink(u), 0.0, color(team.light));
            }
        } else {
            painter.rect_filled(r, 0.0, Color32::from_rgb(0x4a, 0x2e, 0x1e));
        }
    }
    if paused {
        let clip = painter.clone().with_clip_rect(rect.shrink(u));
        let mut x = rect.min.x - rect.height();
        while x < rect.max.x {
            clip.line_segment(
                [pos2(x, rect.max.y), pos2(x + rect.height(), rect.min.y)],
                Stroke::new(u, Color32::from_rgba_unmultiplied(243, 230, 196, 110)),
            );
            x += u * 4.0;
        }
    }
    response.on_hover_text(if paused {
        format!("Beacon {progress} of {of}: held by both teams, so nobody gains")
    } else {
        format!("Beacon {progress} of {of}")
    });
}

/// The turn's timer, as a little brass watch whose face empties as the time runs down.
pub(super) fn watch(ui: &mut egui::Ui, left: f32, of: f32, u: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(u * 13.0, u * 13.0), Sense::hover());
    let c = rect.center() + vec2(0.0, u * 0.5);
    let r = u * 5.5;
    let painter = ui.painter();
    painter.circle_filled(c + vec2(u, u), r, ink::SHADOW);
    painter.rect_filled(
        Rect::from_center_size(c - vec2(0.0, r + u * 0.5), vec2(u * 2.0, u * 1.5)),
        0.0,
        ink::BRASS_LIGHT,
    );
    painter.circle_filled(c, r, ink::BRASS);
    painter.circle_stroke(c, r, Stroke::new(u * 0.7, ink::BRASS_EDGE));
    painter.circle_filled(c, r - u * 1.5, ink::CARD);
    let share = (left / of.max(1.0)).clamp(0.0, 1.0);
    let fill = if left <= 10.0 {
        ink::STAMP
    } else {
        ink::FOREST
    };
    if share > 0.0 {
        let steps = 24;
        let mut points = vec![c];
        for k in 0..=steps {
            let a = -std::f32::consts::FRAC_PI_2
                + share * std::f32::consts::TAU * k as f32 / steps as f32;
            points.push(c + vec2(a.cos(), a.sin()) * (r - u * 2.0));
        }
        // A pie is not convex past half way, so it is drawn as a fan of slices.
        for pair in points[1..].windows(2) {
            painter.add(egui::Shape::convex_polygon(
                vec![c, pair[0], pair[1]],
                fill.gamma_multiply(0.85),
                Stroke::NONE,
            ));
        }
    }
    painter.circle_filled(c, u * 0.8, ink::LINE);
}

/// A name tag over someone on the board: Home's hover tag, card on a hard shadow with a stepped
/// edge, with a word for how they stand under the name.
pub(super) fn name_tag(
    painter: &egui::Painter,
    at: egui::Pos2,
    name: &str,
    state: Option<&str>,
    note: Option<&str>,
    team: formiga_art::paint::Ramp,
    u: f32,
) {
    let galley = painter.layout_no_wrap(name.to_owned(), FontId::proportional(15.0), ink::PAGE);
    let state = state.map(|s| painter.layout_job(hand_job(s, 9.0, ink::MUTED)));
    // A note from the notebook, in the notebook's own hand, wrapped narrow.
    let note = note.map(|n| {
        painter.layout(
            format!("\u{201c}{n}\u{201d}"),
            FontId::proportional(15.0),
            ink::FOREST,
            u * 70.0,
        )
    });
    let w = galley
        .size()
        .x
        .max(state.as_ref().map_or(0.0, |s| s.size().x))
        .max(note.as_ref().map_or(0.0, |n| n.size().x))
        + u * 11.0;
    let h = galley.size().y
        + state.as_ref().map_or(0.0, |s| s.size().y)
        + note.as_ref().map_or(0.0, |n| n.size().y + u)
        + u * 4.0;
    let rect = Rect::from_min_size(
        pos2(
            ((at.x - w / 2.0) / u).round() * u,
            ((at.y - h - u * 3.0) / u).round() * u,
        ),
        vec2(w, h),
    );
    chrome::card(painter, rect, u, ink::CARD, ink::LINE);
    // A tick of the team's colour down its left side.
    painter.rect_filled(
        Rect::from_min_size(
            rect.min + vec2(u * 2.0, u * 2.0),
            vec2(u, rect.height() - u * 4.0),
        ),
        0.0,
        color(team.base),
    );
    // A little point down towards whoever it names.
    let tip = pos2(rect.center().x, rect.max.y + u * 2.0);
    painter.add(egui::Shape::convex_polygon(
        vec![
            pos2(tip.x - u * 2.0, rect.max.y - u),
            pos2(tip.x + u * 2.0, rect.max.y - u),
            tip,
        ],
        ink::LINE,
        Stroke::NONE,
    ));
    let top = rect.min + vec2(u * 6.0, u * 2.0);
    let line = galley.size().y;
    painter.galley(top, galley, ink::PAGE);
    let mut y = line;
    if let Some(state) = state {
        let h = state.size().y;
        painter.galley(top + vec2(0.0, y), state, ink::MUTED);
        y += h;
    }
    if let Some(note) = note {
        painter.galley(top + vec2(0.0, y + u), note, ink::FOREST);
    }
}

/// A little speaker, a box and a cone, with `waves` coming off it, or a cross when there are none.
pub(super) fn speaker(painter: &egui::Painter, c: egui::Pos2, waves: u8, colour: Color32, u: f32) {
    painter.rect_filled(
        Rect::from_center_size(c - vec2(u * 1.5, 0.0), vec2(u * 2.0, u * 3.0)),
        0.0,
        colour,
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            c + vec2(-u * 0.5, -u * 1.5),
            c + vec2(u * 2.0, -u * 3.5),
            c + vec2(u * 2.0, u * 3.5),
            c + vec2(-u * 0.5, u * 1.5),
        ],
        colour,
        Stroke::NONE,
    ));
    for k in 0..waves {
        let r = u * (4.0 + 2.0 * f32::from(k));
        let arc: Vec<egui::Pos2> = (0..=8)
            .map(|i| {
                let a = -0.8 + 1.6 * i as f32 / 8.0;
                c + vec2(u * 1.5 + a.cos() * r, a.sin() * r)
            })
            .collect();
        painter.add(egui::Shape::line(arc, Stroke::new(u * 0.8, colour)));
    }
    if waves == 0 {
        let x = c + vec2(u * 5.0, 0.0);
        for d in [vec2(u * 1.6, u * 1.6), vec2(u * 1.6, -u * 1.6)] {
            painter.line_segment([x - d, x + d], Stroke::new(u * 0.8, colour));
        }
    }
}

/// A little arrow pointing down at whoever is chosen, in its team's colour on a dark edge.
pub(super) fn pointer_arrow(painter: &egui::Painter, tip: egui::Pos2, fill: Color32, u: f32) {
    let shape = |grow: f32| {
        vec![
            tip + vec2(-u * 5.0 - grow, -u * 6.0 - grow),
            tip + vec2(u * 5.0 + grow, -u * 6.0 - grow),
            tip + vec2(0.0, grow),
        ]
    };
    painter.add(egui::Shape::convex_polygon(
        shape(u * 1.2),
        ink::LINE,
        Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(shape(0.0), fill, Stroke::NONE));
}

/// One of the three orders as a token: a disc in its colour with its number, pale until it is
/// given. A HOLD's has an arrowhead on its rim, pointing the way it faces as the board is drawn.
pub(super) fn token(ui: &mut egui::Ui, order: u8, given: bool, facing: Option<Facing>, u: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(u * 14.0, u * 14.0), Sense::hover());
    let ramp = scene::ORDER_COLOURS[usize::from(order) % scene::ORDER_COLOURS.len()];
    let painter = ui.painter();
    let c = rect.center();
    let r = u * 6.0;
    painter.circle_filled(c + vec2(u, u), r, ink::SHADOW);
    let (fill, edge) = if given {
        (color(ramp.base), color(ramp.edge))
    } else {
        (color(ramp.shine).gamma_multiply(0.6), color(ramp.light))
    };
    painter.circle_filled(c, r, fill);
    painter.circle_stroke(c, r, Stroke::new(u * 0.8, edge));
    if given {
        painter.circle_filled(c - vec2(u * 2.0, u * 2.0), u * 1.2, color(ramp.shine));
    }
    let galley = painter.layout_job(hand_job(&format!("{}", order + 1), 11.0, edge));
    painter.galley(c - galley.size() / 2.0 + vec2(u * 0.3, 0.0), galley, edge);
    if let Some(facing) = facing {
        let (x, y) = scene::screen_way(facing);
        let way = vec2(x, y);
        let across = vec2(-y, x);
        let tip = c + way * (r + u * 1.5);
        let base = c + way * (r - u * 2.0);
        painter.add(egui::Shape::convex_polygon(
            vec![tip, base + across * u * 2.6, base - across * u * 2.6],
            color(ramp.shine),
            Stroke::new(u * 0.6, edge),
        ));
    }
}

/// A USE ITEM's token: a leaf-green disc with a plus, for a bonus beside the three.
pub(super) fn bonus_token(ui: &mut egui::Ui, given: bool, u: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(u * 14.0, u * 14.0), Sense::hover());
    let ramp = scene::USE_COLOUR;
    let painter = ui.painter();
    let c = rect.center();
    let r = u * 6.0;
    painter.circle_filled(c + vec2(u, u), r, ink::SHADOW);
    let (fill, edge) = if given {
        (color(ramp.base), color(ramp.edge))
    } else {
        (color(ramp.shine).gamma_multiply(0.6), color(ramp.light))
    };
    painter.circle_filled(c, r, fill);
    painter.circle_stroke(c, r, Stroke::new(u * 0.8, edge));
    let arm = u * 3.0;
    let stroke = Stroke::new(u * 1.4, edge);
    painter.line_segment([c - vec2(arm, 0.0), c + vec2(arm, 0.0)], stroke);
    painter.line_segment([c - vec2(0.0, arm), c + vec2(0.0, arm)], stroke);
}

/// A five-pointed star, for the dizzy.
pub(super) fn star(painter: &egui::Painter, c: egui::Pos2, r: f32, fill: Color32, edge: Color32) {
    let points: Vec<egui::Pos2> = (0..10)
        .map(|k| {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
            let reach = if k % 2 == 0 { r } else { r * 0.45 };
            c + vec2(a.cos(), a.sin()) * reach
        })
        .collect();
    // A star is not convex: drawn as five points round a middle.
    for k in 0..5 {
        let tri = vec![
            c,
            points[(2 * k + 9) % 10],
            points[2 * k],
            points[2 * k + 1],
        ];
        painter.add(egui::Shape::convex_polygon(tri, fill, Stroke::NONE));
    }
    painter.add(egui::Shape::closed_line(
        points,
        Stroke::new(r * 0.18, edge),
    ));
}

/// A tick of whose good news it is down a slip's left side: the team's colour, or the ink.
pub(super) fn tick(painter: &egui::Painter, rect: Rect, side: Option<Side>, u: f32) {
    painter.rect_filled(
        Rect::from_min_size(
            rect.min + vec2(u * 4.0, u * 3.0),
            vec2(u * 2.0, rect.height() - u * 6.0),
        ),
        0.0,
        side.map_or(ink::LINE.gamma_multiply(0.4), |s| {
            color(TEAM[s.index()].base)
        }),
    );
}

/// A little mouse, its left button lit for a click or its right one for a right click.
pub(super) fn mouse(painter: &egui::Painter, rect: Rect, right: bool, u: f32) {
    painter.rect_filled(rect, u * 2.5, ink::LINE);
    painter.rect_filled(rect.shrink(u * 0.8), u * 2.0, ink::CARD);
    let split = rect.min.y + rect.height() * 0.42;
    let half = Rect::from_min_max(
        pos2(
            if right {
                rect.center().x
            } else {
                rect.min.x + u * 0.8
            },
            rect.min.y + u * 0.8,
        ),
        pos2(
            if right {
                rect.max.x - u * 0.8
            } else {
                rect.center().x
            },
            split,
        ),
    );
    painter.rect_filled(half, u, ink::STAMP);
    painter.hline(rect.x_range(), split, Stroke::new(u * 0.6, ink::LINE));
    painter.vline(
        rect.center().x,
        rect.min.y..=split,
        Stroke::new(u * 0.6, ink::LINE),
    );
}

/// The controls, on a card in the board page's lower left while orders are given: what a click
/// and a right click do, and the keys.
pub(super) fn controls(painter: &egui::Painter, page: Rect, u: f32) {
    let rows: [(Option<bool>, &str); 4] = [
        (Some(false), "Press a square, or work what is there"),
        (Some(true), "Hold a square"),
        (
            None,
            "Drag to name more squares  ·  R or wheel turns a HOLD",
        ),
        (None, "P H T I U pick  ·  Enter lock in  ·  L whole match"),
    ];
    let galleys: Vec<_> = rows
        .iter()
        .map(|(_, text)| {
            painter.layout_no_wrap((*text).to_owned(), FontId::proportional(15.0), ink::PAGE)
        })
        .collect();
    let icon = vec2(u * 6.0, u * 9.0);
    let row = galleys.iter().map(|g| g.size().y).fold(icon.y, f32::max) + u * 1.5;
    let title = painter.layout_job(hand_job("Controls", 9.5, ink::FOREST));
    let width = galleys
        .iter()
        .map(|g| g.size().x)
        .fold(title.size().x, f32::max)
        + icon.x
        + u * 16.0;
    let height = u * 10.0 + title.size().y + row * rows.len() as f32;
    let corner = pos2(page.min.x + u * 8.0, page.max.y - u * 6.0 - height);
    let rect = Rect::from_min_size(
        pos2((corner.x / u).round() * u, (corner.y / u).round() * u),
        vec2(width, height),
    );
    let mut painter = painter.clone();
    painter.set_opacity(0.92);
    chrome::card(&painter, rect, u, ink::CARD, ink::LINE.gamma_multiply(0.45));
    painter.galley(
        rect.min + vec2(u * 5.0, u * 4.0),
        title.clone(),
        ink::FOREST,
    );
    let mut y = rect.min.y + u * 7.0 + title.size().y;
    for ((button, _), galley) in rows.iter().zip(galleys) {
        let x = rect.min.x + u * 5.0;
        if let Some(right) = button {
            mouse(
                &painter,
                Rect::from_min_size(pos2(x, y + (row - icon.y) / 2.0 - u * 0.5), icon),
                *right,
                u,
            );
        }
        let h = galley.size().y;
        painter.galley(
            pos2(x + icon.x + u * 4.0, y + (row - h) / 2.0 - u * 0.5),
            galley,
            ink::PAGE,
        );
        y += row;
    }
}
