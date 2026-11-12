//! What the townspeople say, and the card it is said on. Everything said about the Handler or
//! their companions comes from what is recorded (the cups, the trios, the friend matches' points,
//! the season, the meetings with each rival in the career) and nothing else: with nothing recorded
//! yet, a plain greeting. Staff say only what their page already shows; the gardener and the
//! angler have one line of their own each, nothing about anyone. Nothing said changes anything.
//!
//! The card sits over the foot of the town's picture with the speaker's portrait: E, Space, Enter
//! or a click goes on to the next line, and after the last opens the page the speaker keeps, if
//! any. Escape closes it.

use super::*;
use crate::town::map::PlaceId;
use crate::town::sheet::decode;

/// A conversation under way.
pub(super) struct Speech {
    /// Who is speaking, as their card names them.
    who: String,
    /// Their portrait sheet, `portrait-<id>.png`: calm, talking, pleased, their own.
    portrait: Option<Canvas>,
    lines: Vec<String>,
    at: usize,
    /// The page opened after the last line, if any.
    then: Option<(Outing, Step)>,
    /// When the line showing began, for the mouth moving while it is new.
    since: f32,
}

impl Speech {
    #[cfg(test)]
    pub(super) fn lines_for_test(&self) -> Vec<String> {
        self.lines.clone()
    }

    fn new(who: &str, id: Option<&str>, lines: Vec<String>, then: Option<(Outing, Step)>) -> Self {
        let portrait = id
            .and_then(|id| crate::town::art::bytes(&format!("portrait-{id}.png")))
            .and_then(decode);
        Self {
            who: who.to_owned(),
            portrait,
            lines,
            at: 0,
            then,
            since: f32::NEG_INFINITY,
        }
    }
}

/// How long a new line's speaker is seen talking, in seconds.
const TALKING: f32 = 0.9;

impl BattleApp {
    /// What a person or a thing in town says, if it speaks at all.
    pub(super) fn speech_for(&self, place: PlaceId, what: &str) -> Option<Speech> {
        let record = &self.table.record;
        Some(match (place, what) {
            (PlaceId::Arena, "marlow") => {
                let next = crate::rewards::CUPS
                    .iter()
                    .find(|c| record.cup_wins(c.id) < crate::rewards::CUP_WINS);
                let line = match next {
                    None => "Every cup is on your shelf. Champion of the Circuit!".to_owned(),
                    Some(cup) => match record.cup_wins(cup.id) {
                        0 => format!(
                            "The {} is next for you: win three against the visitors and it's yours.",
                            cup.name
                        ),
                        wins => format!(
                            "The {}: {wins} of {} won. {} more and it's yours.",
                            cup.name,
                            crate::rewards::CUP_WINS,
                            crate::rewards::CUP_WINS - wins
                        ),
                    },
                };
                Speech::new(
                    "Marlow",
                    Some("marlow"),
                    vec![line, "Shall we set up a match?".to_owned()],
                    Some((Outing::Home, Step::Where)),
                )
            }
            (PlaceId::Clubhouse, "iris") => {
                let trios: Vec<&str> = record
                    .trios
                    .iter()
                    .filter(|t| !t.retired)
                    .map(|t| t.name.as_str())
                    .collect();
                let line = match trios.as_slice() {
                    [] => "No trios in the register yet. Pick three and name them, and I'll write them in.".to_owned(),
                    [one] => format!("One trio in the register: {one}."),
                    many => format!("{} trios in the register: {}.", many.len(), many.join(", ")),
                };
                Speech::new(
                    "Iris",
                    Some("iris"),
                    vec![line],
                    Some((self.outing, Step::Handler)),
                )
            }
            (PlaceId::GuildHall, "odile") => {
                let points = match record.circuit_points {
                    0 => "No Open Circuit points yet. Friend matches earn them.".to_owned(),
                    1 => "One Open Circuit point, from friend matches.".to_owned(),
                    p => format!("{p} Open Circuit points, from friend matches."),
                };
                let next = match self.table.next_event_fixture() {
                    Some((_, round, rival)) => {
                        format!("Your {round} against {} is next.", rival.name)
                    }
                    None => "No event under way. The board has the next one.".to_owned(),
                };
                Speech::new(
                    "Odile",
                    Some("odile"),
                    vec![points, next],
                    Some((self.outing, Step::Season)),
                )
            }
            (PlaceId::GuildHall, "wren") => {
                Speech::new("Wren", Some("wren"), self.wren_lines(), None)
            }
            (PlaceId::Studio, "tailor" | "counter") => Speech::new(
                "Bram",
                Some("tailor"),
                vec![
                    "Come in! Try a look in the mirror. Nothing here changes how your three play."
                        .to_owned(),
                ],
                Some((self.outing, Step::Studio)),
            ),
            (PlaceId::Campus, "gardener") => Speech::new(
                "The gardener",
                Some("gardener"),
                vec!["The tomatoes are coming along nicely this year.".to_owned()],
                None,
            ),
            (PlaceId::Campus, "angler") => Speech::new(
                "The angler",
                Some("angler"),
                vec!["Nothing's biting. Lovely water, though.".to_owned()],
                None,
            ),
            (PlaceId::Campus, "notices") => {
                let mut lines = Vec::new();
                if let Some(cup) = crate::rewards::CUPS
                    .iter()
                    .find(|c| record.cup_wins(c.id) < crate::rewards::CUP_WINS)
                {
                    lines.push(format!(
                        "Close to Home: the {}, {} of {} won.",
                        cup.name,
                        record.cup_wins(cup.id),
                        crate::rewards::CUP_WINS
                    ));
                }
                lines.push(match self.table.next_event_fixture() {
                    Some((_, round, rival)) => {
                        format!("This season: your {round} against {}.", rival.name)
                    }
                    None => "This season: entries are open at the Guild Hall.".to_owned(),
                });
                Speech::new("The notice board", None, lines, None)
            }
            (PlaceId::Campus, "beacon") => Speech::new(
                "The beacon lamp",
                None,
                vec![
                    "The same beacon every arena is played round, burning on the green.".to_owned(),
                ],
                None,
            ),
            (PlaceId::Campus, "greenhouse") => Speech::new(
                "The greenhouse",
                None,
                vec!["Warm glass and tomato vines. The gardener keeps it.".to_owned()],
                None,
            ),
            _ => return None,
        })
    }

    /// What Wren says: who she is, and how the meetings with her stand as the career has them.
    fn wren_lines(&self) -> Vec<String> {
        let met = self.table.career.against("wren");
        let mut lines = vec![];
        if met.played == 0 {
            lines.push(
                "I'm Wren Ashby, of Thistledown. We haven't met on the field yet.".to_owned(),
            );
            lines.push("When the visitors come, it might be me. I'll see you there.".to_owned());
            return lines;
        }
        let times = if met.played == 1 {
            "once".to_owned()
        } else {
            format!("{} times", met.played)
        };
        let mut tally = format!(
            "We've met {times}. You've won {}, I've won {}",
            met.won, met.lost
        );
        if met.drawn > 0 {
            tally.push_str(&format!(", and {} ended level", met.drawn));
        }
        tally.push('.');
        lines.push(tally);
        if let Some(last) = &met.last {
            let arena = formiga_battle_core::Arena::by_id(&last.arena)
                .map_or_else(|| last.arena.clone(), |a| a.name);
            lines.push(match last.won {
                Some(true) => {
                    format!("You took the last one, at {arena}. I've been thinking about it.")
                }
                Some(false) => format!("I took the last one, at {arena}. Ready for another?"),
                None => format!("The last one, at {arena}, ended level. Not for long."),
            });
        }
        lines
    }

    /// Back in town from a match: a card with what it earned, in the words the match's own page
    /// used, so nothing is said that was not recorded.
    pub(super) fn came_back(&mut self) {
        let lines: Vec<String> = self
            .table
            .earned
            .iter()
            .filter(|l| !l.trim().is_empty())
            .cloned()
            .collect();
        if lines.is_empty() {
            return;
        }
        let who = format!("Back from {}", self.table.arena.name);
        let mut speech = Speech::new(&who, None, lines, None);
        speech.since = self.clock;
        self.speech = Some(speech);
    }

    /// Starts a conversation, or opens the page straight away for a thing that does not speak.
    pub(super) fn talk_or_open(&mut self, place: PlaceId, what: &str) -> bool {
        match self.speech_for(place, what) {
            Some(mut speech) => {
                speech.since = self.clock;
                self.speech = Some(speech);
                true
            }
            None => false,
        }
    }

    /// On to the next line; after the last, the page the speaker keeps.
    pub(super) fn speech_on(&mut self) {
        let Some(speech) = &mut self.speech else {
            return;
        };
        speech.at += 1;
        speech.since = self.clock;
        if speech.at >= speech.lines.len() {
            let then = speech.then;
            self.speech = None;
            if let Some((outing, step)) = then {
                if step == Step::Studio {
                    self.studio_draft = None;
                }
                self.open_journal(outing, step);
            }
        }
    }

    /// The card over the foot of the picture.
    pub(super) fn speech_card(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        picture: Rect,
    ) {
        let now = self.clock;
        let Some(speech) = &self.speech else {
            return;
        };
        let line = speech.lines.get(speech.at).cloned().unwrap_or_default();
        let card = Rect::from_min_max(
            pos2(picture.min.x + 12.0, picture.max.y - 120.0),
            pos2(picture.max.x - 12.0, picture.max.y - 12.0),
        );
        painter.rect_filled(
            card.translate(vec2(3.0, 3.0)),
            0.0,
            Color32::from_black_alpha(70),
        );
        painter.rect_filled(card, 0.0, Color32::from_rgb(0xff, 0xf8, 0xe4));
        painter.rect_stroke(
            card,
            0.0,
            Stroke::new(2.0, Color32::from_rgb(0x3b, 0x2b, 0x3a)),
            egui::StrokeKind::Inside,
        );
        let mut text_left = card.min.x + 16.0;
        if let Some(sheet) = &speech.portrait {
            // Talking while the line is new, then calm.
            let talking = now - speech.since < TALKING;
            let cell = if talking && ((now - speech.since) / 0.15) as i32 % 2 == 0 {
                1
            } else {
                0
            };
            let mut face = Canvas::new(64, 64);
            crate::town::picture::over(&mut face, sheet, -64 * cell, 0);
            let tex = match &mut self.speech_face {
                Some(t) => {
                    let mut image = egui::ColorImage::filled([1, 1], Color32::TRANSPARENT);
                    into_image(&face, &mut image);
                    t.set(image, egui::TextureOptions::NEAREST);
                    t.id()
                }
                None => {
                    let t = texture(ctx, "speech-face", &face);
                    let id = t.id();
                    self.speech_face = Some(t);
                    id
                }
            };
            let side = card.height() - 16.0;
            let face_rect = Rect::from_min_size(card.min + vec2(8.0, 8.0), vec2(side, side));
            painter.image(
                tex,
                face_rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            text_left = face_rect.max.x + 12.0;
        }
        painter.text(
            pos2(text_left, card.min.y + 12.0),
            egui::Align2::LEFT_TOP,
            speech.who.to_uppercase(),
            FontId::monospace(13.0),
            Color32::from_rgb(0x1c, 0x41, 0x37),
        );
        let galley = painter.layout(
            line,
            FontId::proportional(19.0),
            Color32::from_rgb(0x23, 0x36, 0x2f),
            card.max.x - text_left - 16.0,
        );
        painter.galley(pos2(text_left, card.min.y + 34.0), galley, Color32::WHITE);
        let more = if speech.at + 1 < speech.lines.len() {
            "E: next"
        } else {
            "E: done"
        };
        painter.text(
            card.max - vec2(12.0, 10.0),
            egui::Align2::RIGHT_BOTTOM,
            more,
            FontId::monospace(13.0),
            Color32::from_rgb(0x3b, 0x2b, 0x3a),
        );
    }
}
