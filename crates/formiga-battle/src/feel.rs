//! How Battle answers a touch: the interaction kit every button, card and panel shares, so the
//! whole game speaks one language. A control lifts when pointed at, sinks when pressed and springs
//! back with a little overshoot when let go; a confirming one (the page's way on) gives a brass
//! sparkle as it springs, a choosing one a smaller bounce, and a cancelling one (back, close, take
//! out) a quick shake with no sparkle, so going on and going back never feel alike. Cards and
//! panels drop onto the page one after another as it shows, a chosen companion hops, and markers
//! breathe while they wait.
//!
//! Everything is quick and small, and under reduced motion none of it moves: a pointed-at control
//! still changes colour, and nothing else. Whatever is felt is also told, as a [`Feedback`], for
//! whatever plays sounds to hear: drain them each frame with [`take`].

use eframe::egui::{self, Color32, Id, Rect, emath::TSTransform, vec2};

/// Something felt, for the sounds to answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feedback {
    /// The pointer came onto a control.
    Hover,
    /// A choice changed: a card picked, a tab turned to.
    Select,
    /// The way on: a page's own button, a destination, a room opened.
    Confirm,
    /// The way back, or something undone: back, close, take out.
    Cancel,
    /// A page of the notebook turned.
    PageTurn,
    /// A companion hopped, chosen for the team.
    Hop,
    /// A companion taken out of the team.
    Unpick,
    /// The train whistled as it came in for the companions.
    TrainIn,
    /// The train let off steam as it stopped at the platform.
    TrainStops,
    /// The train whistled and pulled away.
    TrainOut,
}

/// How a control answers being pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feel {
    /// A choice among several.
    Select,
    /// The way on.
    Confirm,
    /// The way back.
    Cancel,
}

impl Feel {
    fn feedback(self) -> Feedback {
        match self {
            Self::Select => Feedback::Select,
            Self::Confirm => Feedback::Confirm,
            Self::Cancel => Feedback::Cancel,
        }
    }
}

/// How long a press springs back for, in seconds.
const SPRING: f32 = 0.42;
/// How long a card takes to drop onto the page, and how far apart each one after it starts.
const DROP: f32 = 0.24;
const STAGGER: f32 = 0.045;
/// How long a hop lasts.
const HOP: f32 = 0.36;

fn now(ctx: &egui::Context) -> f32 {
    ctx.input(|i| i.time) as f32
}

/// Whether motion is reduced, for everything the kit does.
pub fn still(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(Id::new("feel-still")))
        .unwrap_or(false)
}

pub fn set_still(ctx: &egui::Context, still: bool) {
    ctx.data_mut(|d| d.insert_temp(Id::new("feel-still"), still));
}

/// Tells whatever plays sounds that something was felt.
pub fn emit(ctx: &egui::Context, feedback: Feedback) {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Vec<Feedback>>(Id::new("feel-told"))
            .push(feedback)
    });
}

/// Everything felt since the last call.
pub fn take(ctx: &egui::Context) -> Vec<Feedback> {
    ctx.data_mut(|d| {
        std::mem::take(d.get_temp_mut_or_default::<Vec<Feedback>>(Id::new("feel-told")))
    })
}

/// A control's state of being pressed this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Press {
    /// How far it has lifted towards the pointer, 0 to 1.
    pub hover: f32,
    /// How large it is drawn: a little under 1 while held down, a little over as it springs.
    pub scale: f32,
    /// How far it is shaken sideways, in units: a cancelling press only.
    pub shake: f32,
    /// How far through its sparkle a confirming press is, 0 to 1, if it is sparkling.
    pub sparkle: Option<f32>,
}

impl Press {
    /// Draw it at rest.
    pub const REST: Self = Self {
        hover: 0.0,
        scale: 1.0,
        shake: 0.0,
        sparkle: None,
    };

    /// The transform that draws a control this way round its centre.
    pub fn transform(&self, rect: Rect, unit: f32) -> TSTransform {
        let c = rect.center();
        // A big card gives only as much as a button would, so nothing on it jumps far.
        let give = (140.0 / rect.width().max(rect.height()).max(1.0)).min(1.0);
        let scale = 1.0 + (self.scale - 1.0) * give;
        TSTransform::from_translation(vec2(c.x + self.shake * unit, c.y))
            * TSTransform::from_scaling(scale)
            * TSTransform::from_translation(-c.to_vec2())
    }
}

/// How a control with this response answers this frame, and what it tells: a hover as the
/// pointer comes onto it, its feel as it is clicked.
pub fn press(ctx: &egui::Context, id: Id, response: &egui::Response, feel: Feel) -> Press {
    let t = now(ctx);
    let enabled = response.sense.senses_click();
    let hovered = enabled && response.hovered();
    let was = ctx.data(|d| d.get_temp::<bool>(id.with("feel-hovered")).unwrap_or(false));
    if hovered && !was {
        emit(ctx, Feedback::Hover);
    }
    ctx.data_mut(|d| d.insert_temp(id.with("feel-hovered"), hovered));
    if response.clicked() {
        emit(ctx, feel.feedback());
        ctx.data_mut(|d| d.insert_temp(id.with("feel-clicked"), t));
    }
    if still(ctx) {
        return Press {
            hover: f32::from(u8::from(hovered)),
            ..Press::REST
        };
    }
    let hover = ctx.animate_bool_with_time(id.with("feel-lift"), hovered, 0.1);
    let held = enabled && response.is_pointer_button_down_on();
    let since = ctx
        .data(|d| d.get_temp::<f32>(id.with("feel-clicked")))
        .map(|at| t - at)
        .filter(|s| *s < SPRING);
    let (depth, wobble) = match feel {
        Feel::Select => (0.05, 0.05),
        Feel::Confirm => (0.06, 0.08),
        Feel::Cancel => (0.04, 0.0),
    };
    let mut scale = if held { 1.0 - depth } else { 1.0 };
    let mut shake = 0.0;
    let mut sparkle = None;
    if let Some(s) = since.filter(|_| !held) {
        let damp = (-s * 11.0).exp();
        scale = 1.0 + wobble * damp * (s * 26.0).sin();
        if feel == Feel::Cancel {
            shake = 2.0 * damp * (s * 46.0).sin();
        }
        if feel == Feel::Confirm {
            sparkle = Some((s / SPRING).clamp(0.0, 1.0));
        }
    }
    Press {
        hover,
        scale,
        shake,
        sparkle,
    }
}

/// Brass glints thrown out from a confirming control as it springs back.
pub fn sparkle(painter: &egui::Painter, rect: Rect, through: f32, unit: f32) {
    let fade = 1.0 - through;
    let reach = unit * 3.0 + through * unit * 12.0;
    let points: [(f32, f32); 8] = [
        (0.0, -1.0),
        (0.7, -0.7),
        (1.0, 0.0),
        (0.7, 0.7),
        (0.0, 1.0),
        (-0.7, 0.7),
        (-1.0, 0.0),
        (-0.7, -0.7),
    ];
    let half = rect.size() / 2.0;
    for (k, (dx, dy)) in points.iter().enumerate() {
        let at = rect.center() + vec2(dx * (half.x + reach), dy * (half.y + reach));
        let size = unit * if k % 2 == 0 { 2.0 } else { 1.0 } * (0.5 + fade);
        let colour = Color32::from_rgba_unmultiplied(0xfb, 0xe6, 0xa2, (fade * 230.0) as u8);
        // A little four-pointed star, in whole units.
        painter.rect_filled(Rect::from_center_size(at, vec2(size, unit)), 0.0, colour);
        painter.rect_filled(Rect::from_center_size(at, vec2(unit, size)), 0.0, colour);
    }
}

/// How far a card or panel has dropped onto a page shown since `since`, the `index`th to drop:
/// how high above its place it still is, in units, and how much of it shows.
pub fn entrance(ctx: &egui::Context, since: f32, index: usize) -> (f32, f32) {
    if still(ctx) {
        return (0.0, 1.0);
    }
    let t = ((now(ctx) - since - STAGGER * index as f32) / DROP).clamp(0.0, 1.0);
    // Out and back past its place a hair, as a card set down settles.
    let c = 1.6;
    let eased = 1.0 + (c + 1.0) * (t - 1.0).powi(3) + c * (t - 1.0).powi(2);
    ((1.0 - eased) * 6.0, (t * 2.0).min(1.0))
}

/// Draws whatever `add` draws as a card dropping onto the page: risen and faded at first, then in
/// its place. Only the picture moves; it answers the pointer where it lands.
pub fn dropping<R>(
    ui: &mut egui::Ui,
    since: f32,
    index: usize,
    unit: f32,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let (rise, shown) = entrance(ui.ctx(), since, index);
    if rise == 0.0 && shown >= 1.0 {
        return add(ui);
    }
    ui.with_visual_transform(
        TSTransform::from_translation(vec2(0.0, -(rise * unit).round())),
        |ui| {
            ui.multiply_opacity(shown);
            add(ui)
        },
    )
    .inner
}

/// Where the next shape painted on `ui`'s layer will go: for [`transform_since`], when how
/// something is drawn depends on a response that only comes after it is drawn.
pub fn mark(ui: &egui::Ui) -> egui::layers::ShapeIdx {
    ui.ctx().graphics(|g| {
        g.get(ui.layer_id())
            .map_or(egui::layers::ShapeIdx(0), |l| l.next_idx())
    })
}

/// Transforms everything painted on `ui`'s layer since `mark`.
pub fn transform_since(ui: &egui::Ui, mark: egui::layers::ShapeIdx, transform: TSTransform) {
    if transform == TSTransform::IDENTITY {
        return;
    }
    ui.ctx().graphics_mut(|g| {
        let list = g.entry(ui.layer_id());
        let end = list.next_idx();
        list.transform_range(mark, end, transform);
    });
}

/// Starts a hop for whatever `id` names: a companion just chosen, or one getting aboard.
pub fn start_hop(ctx: &egui::Context, id: Id) {
    let t = now(ctx);
    ctx.data_mut(|d| d.insert_temp(id.with("feel-hop"), t));
    emit(ctx, Feedback::Hop);
}

/// How high whatever `id` names is off the ground in its hop, in units: a hop, then a little one.
pub fn hop(ctx: &egui::Context, id: Id) -> f32 {
    if still(ctx) {
        return 0.0;
    }
    let Some(at) = ctx.data(|d| d.get_temp::<f32>(id.with("feel-hop"))) else {
        return 0.0;
    };
    let s = now(ctx) - at;
    let first = HOP * 0.7;
    if s < first {
        (std::f32::consts::PI * s / first).sin() * 5.0
    } else if s < HOP {
        (std::f32::consts::PI * (s - first) / (HOP - first)).sin() * 1.5
    } else {
        0.0
    }
}

/// A slow breath, 0 to 1 and back every `period` seconds, for a marker waiting to be noticed;
/// held at the middle under reduced motion.
pub fn breathe(ctx: &egui::Context, period: f32) -> f32 {
    if still(ctx) {
        return 0.5;
    }
    0.5 - 0.5 * (now(ctx) * std::f32::consts::TAU / period).cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_card_lands_in_its_place_and_shows_whole() {
        let ctx = egui::Context::default();
        let (rise, shown) = entrance(&ctx, -10.0, 3);
        assert!(rise.abs() < 1e-4);
        assert_eq!(shown, 1.0);
    }

    #[test]
    fn under_reduced_motion_nothing_moves() {
        let ctx = egui::Context::default();
        set_still(&ctx, true);
        start_hop(&ctx, Id::new("pip"));
        assert_eq!(hop(&ctx, Id::new("pip")), 0.0);
        assert_eq!(entrance(&ctx, 0.0, 0), (0.0, 1.0));
        assert_eq!(breathe(&ctx, 2.0), 0.5);
    }

    #[test]
    fn what_is_felt_is_told_once() {
        let ctx = egui::Context::default();
        emit(&ctx, Feedback::Confirm);
        emit(&ctx, Feedback::PageTurn);
        assert_eq!(take(&ctx), vec![Feedback::Confirm, Feedback::PageTurn]);
        assert!(take(&ctx).is_empty());
    }
}
