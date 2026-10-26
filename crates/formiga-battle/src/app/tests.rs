//! The window's tests: the app driven frame by frame through egui alone, with no window.

use super::*;
use formiga_battle_contract::sample;

/// The app driven frame by frame through egui alone, with no window: what each frame drew,
/// and clicks on what it drew.
struct Headless {
    ctx: egui::Context,
    app: BattleApp,
    time: f64,
    drawn: Vec<(String, Rect)>,
    /// The app asked for the window to close.
    closed: bool,
}

impl Headless {
    fn new() -> Self {
        let table = Table::new(sample::snapshot(), sample::visitors(9), 5);
        let host = Host::Rehearsal {
            from: "a test".to_owned(),
        };
        Self {
            ctx: egui::Context::default(),
            app: BattleApp::new(host, table, None, None, Sound::silent()),
            time: 0.0,
            drawn: Vec::new(),
            closed: false,
        }
    }

    fn frame(&mut self, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 780.0))),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let app = &mut self.app;
        let mut output = self.ctx.run_ui(input, |ui| app.frame(ui));
        output.textures_delta.clear();
        self.closed |= output.viewport_output.values().any(|v| {
            v.commands
                .iter()
                .any(|c| matches!(c, egui::ViewportCommand::Close))
        });
        self.drawn.clear();
        for clipped in &output.shapes {
            words(&clipped.shape, &mut self.drawn);
        }
        self.time += 0.05;
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame(Vec::new());
        }
    }

    /// A click on the `nth` thing drawn with these words.
    fn click(&mut self, label: &str, nth: usize) {
        let label = label.to_uppercase();
        let Some(at) = self
            .drawn
            .iter()
            .filter(|(w, _)| w.to_uppercase() == label)
            .nth(nth)
            .map(|(_, r)| r.center())
        else {
            panic!("nothing says {label:?}");
        };
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        self.frame(vec![egui::Event::PointerMoved(at)]);
        self.frame(vec![button(true)]);
        self.frame(vec![button(false)]);
    }
}

fn words(shape: &egui::epaint::Shape, out: &mut Vec<(String, Rect)>) {
    match shape {
        egui::epaint::Shape::Text(text) => {
            out.push((text.galley.text().to_owned(), text.visual_bounding_rect()));
        }
        egui::epaint::Shape::Vec(shapes) => shapes.iter().for_each(|s| words(s, out)),
        _ => {}
    }
}

fn says(h: &Headless, words: &str) -> bool {
    h.drawn.iter().any(|(w, _)| w.eq_ignore_ascii_case(words))
}

#[test]
fn battle_opened_by_itself_welcomes_the_player_until_they_look_around() {
    let mut h = Headless::new();
    h.app = BattleApp::new(
        Host::Rehearsal {
            from: "the sample colony".to_owned(),
        },
        Table::new(sample::snapshot(), sample::visitors(9), 5),
        None,
        None,
        Sound::silent(),
    )
    .welcoming();
    h.frames(4);
    assert!(says(&h, welcome::TITLE));
    assert!(says(&h, &welcome::how_to_open()));
    assert!(says(&h, "formigaworld.com"));
    h.click(welcome::LOOK_AROUND, 0);
    h.frames(2);
    assert!(!says(&h, welcome::TITLE));
    assert!(!h.closed);
    // The rehearsal carries on at the Battleground, as it would have without the card.
    assert_eq!(h.app.stage, Stage::Station);
}

#[test]
fn the_welcome_takes_the_clicks_meant_for_the_station_under_it() {
    let mut h = Headless::new();
    h.app.welcome = true;
    h.frames(2);
    // The train in, and the way on from the station dropped onto its page, under the card.
    let now = h.app.clock;
    h.app.station.as_mut().expect("the station").skip(now);
    h.frames(40);
    h.click("Field Lessons", 0);
    h.frames(2);
    assert_eq!(h.app.stage, Stage::Station, "the card took the click");
    assert!(says(&h, welcome::TITLE));
}

#[test]
fn closing_the_welcome_closes_battle() {
    let mut h = Headless::new();
    h.app.welcome = true;
    h.frames(4);
    h.click(welcome::CLOSE, 0);
    assert!(h.closed);
}

#[test]
fn battle_opened_with_a_colony_has_no_welcome() {
    let mut h = Headless::new();
    h.frames(4);
    assert!(!says(&h, welcome::TITLE));
}

#[test]
fn pictures_left_behind_while_resizing_are_let_go() {
    let ctx = egui::Context::default();
    let picture = |w| texture(&ctx, &format!("sign-{w}"), &Canvas::new(w, 4));
    let mut kept = Kept::default();
    // Dragged through three widths, then left at the last.
    for (frame, w) in [(1, 100), (2, 101), (3, 102)] {
        kept.keep(format!("sign-{w}"), picture(w), frame);
    }
    for frame in 4..=Kept::UNSHOWN_FRAMES + 1 {
        assert!(kept.get("sign-102", frame).is_some());
        assert!(kept.sweep(frame).is_empty(), "none goes too soon");
    }
    assert_eq!(kept.sweep(Kept::UNSHOWN_FRAMES + 3).len(), 2);
    assert_eq!(kept.len(), 1);
    assert!(kept.get("sign-102", Kept::UNSHOWN_FRAMES + 5).is_some());
}

#[test]
fn the_board_goes_to_its_texture_exactly_as_egui_would_convert_it() {
    let arena = formiga_battle_core::Arena::backyard();
    let view = View::new(arena.width, arena.height);
    let garden = Garden::new(&arena, &view);
    // Filled in place over a smaller picture's pixels, as when the window changes place.
    let mut image = egui::ColorImage::filled([3, 2], Color32::RED);
    for canvas in [garden.picture(), garden.ground.clone()] {
        into_image(&canvas, &mut image);
        let expected = egui::ColorImage::from_rgba_unmultiplied(
            [canvas.width() as usize, canvas.height() as usize],
            &canvas.rgba_bytes(),
        );
        assert!(image == expected);
    }
}

#[test]
fn the_board_is_painted_for_the_matchs_own_place_not_the_one_last_shown() {
    // A friend's room plays the opener's place, though the joiner last looked at another.
    let backyard = formiga_battle_core::Arena::backyard();
    let other = formiga_battle_core::Arena::built_in()
        .into_iter()
        .find(|a| a.id != backyard.id)
        .expect("more than one place");
    let mut view = View::new(backyard.width, backyard.height);
    let mut garden = Some(Garden::new(&backyard, &view));
    let painted = garden_for(&mut garden, &mut view, &other);
    assert_eq!(painted.arena, other.id);
    let fresh = Garden::new(&other, &View::new(other.width, other.height));
    assert_eq!(painted.pieces.len(), fresh.pieces.len());
    assert!(painted.ground.rgba_bytes() == fresh.ground.rgba_bytes());
    assert_eq!(view, View::new(other.width, other.height));
}

#[test]
fn the_station_button_on_the_handlers_first_page_goes_back_to_the_station() {
    let mut h = Headless::new();
    h.frames(2);
    h.app.open_journal(Outing::Home, Step::Handler);
    h.frames(10);
    // The way back along the top, then the button under the way on.
    for nth in [0, 1] {
        h.click("‹ The station", nth);
        h.frames(2);
        assert_eq!(h.app.stage, Stage::Station);
        h.app.open_journal(Outing::Home, Step::Handler);
        h.frames(10);
    }
}

#[test]
fn a_name_typed_on_the_handlers_page_is_kept_before_the_field_is_left() {
    let mut h = Headless::new();
    h.frames(2);
    h.app.open_journal(Outing::Home, Step::Handler);
    h.frames(10);
    h.click("The name your friends know you by", 0);
    h.frame(vec![egui::Event::Text("Jo".to_owned())]);
    h.frames(2);
    let card = h.app.table.record.handler.as_ref();
    assert_eq!(card.map(|c| c.name.as_str()), Some("Jo"));
}

#[test]
fn back_to_the_station_after_a_match_goes_back_to_the_station() {
    let mut h = Headless::new();
    h.frames(2);
    h.app.begin();
    let mut now = h.time as f32 + READY_SECS;
    while !h.app.table.finished(now) {
        h.app.table.coach_orders();
        h.app.table.lock(now);
        now += 20.0;
        h.app.table.frame(now);
    }
    h.time = f64::from(now) + 1.0;
    h.frames(4);
    h.click("‹ Back to the station", 0);
    h.frames(2);
    assert_eq!(h.app.stage, Stage::Station);
}

#[test]
fn orders_can_be_given_from_the_keyboard_alone() {
    let mut h = Headless::new();
    h.frames(2);
    h.app.begin();
    // Past the ready-up, with orders to give.
    h.time += f64::from(READY_SECS) + 1.0;
    h.frames(2);
    let key = |key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    };
    let centre = h.app.table.m.as_ref().unwrap().arena.zone_centre();
    // The first arrow puts the cursor in the middle of the yard; the next moves it.
    h.frame(vec![key(egui::Key::ArrowUp)]);
    assert_eq!(h.app.cursor, Some(centre));
    h.frame(vec![key(egui::Key::ArrowDown)]);
    h.frame(vec![key(egui::Key::ArrowUp)]);
    assert_eq!(h.app.cursor, Some(centre));
    // P picks up a PRESS; Space gives it at the cursor.
    h.frame(vec![key(egui::Key::P)]);
    h.frame(vec![key(egui::Key::Space)]);
    let given = h
        .app
        .table
        .orders
        .iter()
        .flatten()
        .any(|o| matches!(o, Directive::Press { squares } if squares.contains(&centre)));
    assert!(
        given,
        "a PRESS on the middle of the yard: {:?}",
        h.app.table.orders
    );
    // The cursor never leaves the board.
    for _ in 0..20 {
        h.frame(vec![key(egui::Key::ArrowLeft)]);
    }
    assert_eq!(h.app.cursor.map(|t| t.x), Some(0));
}

/// Frames until something says `label`, or panics.
fn until(h: &mut Headless, label: &str) {
    let label = label.to_uppercase();
    for _ in 0..600 {
        if h.drawn.iter().any(|(w, _)| w.to_uppercase() == label) {
            return;
        }
        h.frame(Vec::new());
    }
    panic!("nothing ever said {label:?}");
}

/// Plays the lesson on the board to its end with these orders each turn.
fn play_lesson(h: &mut Headless, plan: impl Fn(&Match) -> formiga_battle_core::TeamPlan) {
    let mut now = h.time as f32 + READY_SECS;
    while !h.app.table.finished(now) {
        let orders = plan(h.app.table.m.as_ref().expect("on the board"));
        h.app.table.orders = [0, 1, 2].map(|i| orders.get(i).cloned());
        h.app.table.lock(now);
        now += 20.0;
        h.app.table.frame(now);
    }
    h.time = f64::from(now) + 1.0;
    h.frames(4);
}

#[test]
fn the_circuit_names_the_rival_coming_next_and_turns_to_who_goes() {
    let mut h = Headless::new();
    h.frames(2);
    h.app.open_journal(Outing::Home, Step::Where);
    h.frames(20);
    let (rival, _) = h.app.table.next_visitors();
    assert!(says(&h, &format!("vs. {}!", rival.short)));
    assert!(says(&h, "The Backyard Cup"));
    h.click("Who's going? ›", 0);
    h.frames(20);
    assert_eq!(h.app.step, Step::Who);
}

#[test]
fn field_lessons_open_from_the_station_and_begin_with_the_three_chosen() {
    let mut h = Headless::new();
    until(&mut h, "Field Lessons");
    h.click("Field Lessons", 0);
    h.frames(20);
    assert_eq!(h.app.stage, Stage::Journal);
    assert_eq!((h.app.outing, h.app.step), (Outing::Lessons, Step::Lessons));
    h.click("Flanks", 0);
    h.frames(2);
    assert_eq!(h.app.lesson, Lesson::Flank);
    h.click("Turn the page: who goes", 0);
    h.frames(20);
    assert_eq!(h.app.step, Step::Who);
    until(&mut h, "Begin the lesson");
    h.click("Begin the lesson", 0);
    h.frames(4);
    assert_eq!(h.app.stage, Stage::Playing);
    let run = h.app.table.lesson.as_ref().expect("a lesson on the board");
    assert_eq!(run.lesson, Lesson::Flank);
    assert_eq!(h.app.table.arena.id, "backyard");
    assert!(
        h.drawn
            .iter()
            .any(|(w, _)| w.starts_with(Lesson::Flank.goal())),
        "{:?}",
        h.drawn.iter().map(|d| &d.0).collect::<Vec<_>>()
    );
}

#[test]
fn a_lesson_waits_for_the_handler_and_keeps_nothing() {
    let mut h = Headless::new();
    h.frames(2);
    let record = h.app.table.record.clone();
    h.app.lesson = Lesson::Beacon;
    h.app.outing = Outing::Lessons;
    h.app.begin_lesson();
    // Minutes go by with no orders given: no timer runs out on a lesson.
    for _ in 0..40 {
        h.time += 10.0;
        h.frame(Vec::new());
    }
    assert!(h.app.table.shown.is_none());
    assert!(
        h.app
            .table
            .lesson
            .as_ref()
            .is_some_and(|r| r.given.is_empty())
    );
    play_lesson(&mut h, |m| {
        vec![formiga_battle_core::Directive::Press {
            squares: vec![m.arena.zone_centre()],
        }]
    });
    let run = h.app.table.lesson.as_ref().expect("still the lesson");
    assert!(
        matches!(run.verdict, Verdict::Learned(_)),
        "{:?}",
        run.verdict
    );
    assert!(h.app.table.earned.is_empty());
    assert_eq!(h.app.table.record, record);
    assert!(h.app.learned.contains(&Lesson::Beacon));
    until(&mut h, "Next: Hold, and face them");
    h.click("Next: Hold, and face them", 0);
    h.frames(2);
    assert_eq!(
        h.app.table.lesson.as_ref().map(|r| r.lesson),
        Some(Lesson::Facing)
    );
}

#[test]
fn a_lesson_not_learned_says_what_to_try_and_can_be_tried_again() {
    let mut h = Headless::new();
    h.frames(2);
    h.app.lesson = Lesson::Flank;
    h.app.outing = Outing::Lessons;
    h.app.begin_lesson();
    play_lesson(&mut h, |_| Vec::new());
    let run = h.app.table.lesson.as_ref().expect("still the lesson");
    assert!(
        matches!(run.verdict, Verdict::TryAgain(_)),
        "{:?}",
        run.verdict
    );
    until(&mut h, "Try again");
    h.click("Try again", 0);
    h.frames(2);
    let run = h.app.table.lesson.as_ref().expect("the lesson again");
    assert_eq!(run.lesson, Lesson::Flank);
    assert!(run.given.is_empty() && h.app.table.outcome().is_none());
    play_lesson(&mut h, |_| Vec::new());
    until(&mut h, "All the lessons");
    h.click("All the lessons", 0);
    h.frames(2);
    assert_eq!((h.app.stage, h.app.step), (Stage::Journal, Step::Lessons));
}

/// The Facing lesson on the board with its orders open: a HOLD and a PRESS given, the third
/// left empty. The HOLD names several squares, for the longest words a card says.
fn hold_and_press_given() -> Headless {
    use formiga_battle_core::{Directive, Facing};
    let mut h = Headless::new();
    h.frames(2);
    h.app.lesson = Lesson::Facing;
    h.app.outing = Outing::Lessons;
    h.app.begin_lesson();
    h.time += f64::from(READY_SECS) + 1.0;
    h.frames(4);
    let at = h
        .app
        .table
        .m
        .as_ref()
        .expect("on the board")
        .arena
        .zone_centre();
    let beside = Tile { x: at.x + 1, ..at };
    h.app.table.orders[0] = Some(Directive::Hold {
        squares: vec![at, beside],
        facing: Facing::North,
    });
    h.app.table.orders[1] = Some(Directive::Press { squares: vec![at] });
    h.frames(4);
    h
}

#[test]
fn the_turn_and_clear_buttons_on_an_order_card_do_what_they_say() {
    use formiga_battle_core::{Directive, Facing};
    let mut h = hold_and_press_given();
    let in_hand = (h.app.table.selected, h.app.table.verb);
    h.click("turn", 0);
    h.frames(2);
    assert!(matches!(
        h.app.table.orders[0],
        Some(Directive::Hold {
            facing: Facing::East,
            ..
        })
    ));
    // The button took the click, not the card under it: the order was not picked up again.
    assert_eq!((h.app.table.selected, h.app.table.verb), in_hand);
    // The PRESS's clear is the second on the page.
    h.click("clear", 1);
    h.frames(2);
    assert!(h.app.table.orders[0].is_some() && h.app.table.orders[1].is_none());
    h.click("clear", 0);
    h.frames(2);
    assert!(h.app.table.orders.iter().all(Option::is_none));
    // The card round the buttons is still clicked to pick its order up again.
    let mut h = hold_and_press_given();
    h.click("PRESS", 1);
    h.frames(2);
    assert_eq!(
        (h.app.table.selected, h.app.table.verb),
        (Some(1), Some(Verb::Press))
    );
}

#[test]
fn the_buttons_on_an_order_card_cover_none_of_its_words() {
    let h = hold_and_press_given();
    let u = chrome::unit(1.0);
    let buttons: Vec<Rect> = h
        .drawn
        .iter()
        .filter(|(w, _)| w == "CLEAR" || w == "TURN")
        .map(|(_, r)| r.expand2(vec2(u * 5.0, u * 3.0)))
        .collect();
    assert_eq!(buttons.len(), 3, "a clear each and the HOLD's turn");
    for (words, rect) in &h.drawn {
        if words == "CLEAR" || words == "TURN" {
            continue;
        }
        for button in &buttons {
            assert!(
                !button.intersects(*rect),
                "a button at {button:?} covers {words:?} at {rect:?}"
            );
        }
    }
}

#[test]
fn a_line_over_the_board_always_finds_its_moment() {
    // Two lines born in the same frame: half of all moments are ones where stepping past
    // the first lands a hair too close to it, which once stepped to the same moment for ever.
    for k in 0..2000 {
        let born = k as f32 * 0.05;
        let shown = headline_moment(born, &[born]);
        assert!(
            shown > born && shown - born <= HEADLINE_SECS + 0.001,
            "{born}"
        );
        let third = headline_moment(born, &[born, shown]);
        assert!(
            third > shown && third - shown <= HEADLINE_SECS + 0.001,
            "{born}"
        );
    }
    // Clear of every line already placed, it is shown when it is born.
    assert_eq!(headline_moment(5.0, &[1.0, 9.0]), 5.0);
    assert_eq!(headline_moment(5.0, &[5.5, 4.6]), 5.5 + HEADLINE_SECS);
}
