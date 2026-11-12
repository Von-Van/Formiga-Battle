//! The town round the Battleground: a small research campus the Handler walks, a tile at a time,
//! from their office past a courtyard to the Arena, the Clubhouse, the Guild Hall and the Handler
//! Studio. It is a way to the pages Battle already has, never a game of its own: nothing here
//! changes how a match plays or what the record keeps, and the companions never walk it. They
//! stay in the habitat on the office desk, at their own size.
//!
//! The places are data (`map`), the Handler walks them (`walk`) and is drawn from a character
//! sheet (`sheet`) over each place's picture (`picture`). Everything here depends only on the time
//! it is given, so the town is checked without a window; the window's side is `app/town.rs`.

pub mod arrival;
pub mod look;
pub mod map;
pub mod picture;
pub mod sheet;
pub mod walk;

use formiga_art::{Canvas, Rgba};
use map::{Facing, Ground, Place, PlaceId, Target};
use sheet::Sheet;
use walk::{Moved, Walker};

/// How long reaching for something shows, in seconds.
const REACH: f32 = 0.35;
/// How long a place takes to come up after a door, in seconds.
pub const ENTER: f32 = 0.25;
/// The colour round a place smaller than the window.
const SURROUND: Rgba = Rgba::new(0x2f, 0x1c, 0x12, 255);

/// What happened in the town this frame that the window acts on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Happened {
    Nothing,
    /// A step was taken.
    Stepped,
    /// The Handler walked into a wall.
    Bumped,
    /// Through a door into another place.
    Entered(PlaceId),
    /// A thing was used, by its id in the place file.
    Used(PlaceId, String),
    /// Someone was spoken to, by their id.
    Spoke(PlaceId, String),
}

pub struct Town {
    pub place: Place,
    pub walker: Walker,
    /// The target Tab moved to, shown with its prompt until the Handler walks away.
    pub picked: Option<Target>,
    /// A target being walked to, used on arrival.
    heading_for: Option<Target>,
    ground: Canvas,
    sheet: Sheet,
    /// When the place was entered.
    pub entered: f32,
    /// When the Handler last reached for something.
    reached: f32,
    /// Just through a door: the key still held from the other side is ignored until it is let
    /// go, so walking out of a door never walks straight back in.
    let_go: bool,
}

impl Town {
    /// The Handler at their office desk, as the arrival leaves them, facing the room.
    pub fn new(now: f32) -> Self {
        let place = PlaceId::Office.place();
        let at = place.spawn.expect("the office has a spawn");
        Self::standing(place, at, Facing::Down, now)
    }

    /// The Handler just inside a place, having come through one of its doors.
    pub fn through(id: PlaceId, door: char, now: f32) -> Self {
        let place = id.place();
        let (at, facing) = place.arrival(door).unwrap_or_else(|| {
            let d = &place.doors[0];
            ((d.tile.0, d.tile.1), d.inward)
        });
        Self::standing(place, at, facing, now)
    }

    fn standing(place: Place, at: (i32, i32), facing: Facing, now: f32) -> Self {
        let ground = picture::stand_in(&place);
        Self {
            place,
            walker: Walker::new(at, facing, now),
            picked: None,
            heading_for: None,
            ground,
            sheet: Sheet::stand_in(),
            entered: now,
            reached: f32::NEG_INFINITY,
            let_go: false,
        }
    }

    /// Goes through a door: the place on its other side, just inside.
    fn go_through(&mut self, mark: char, now: f32) -> Happened {
        let Some(door) = self.place.door(mark).cloned() else {
            return Happened::Nothing;
        };
        *self = Self::through(door.to, door.to_mark, now);
        self.let_go = true;
        Happened::Entered(door.to)
    }

    /// One frame of walking, with the direction held now, if any.
    pub fn tick(&mut self, held: Option<Facing>, now: f32) -> Happened {
        if held.is_none() {
            self.let_go = false;
        }
        let held = held.filter(|_| !self.let_go);
        if held.is_some() {
            self.heading_for = None;
        }
        let moved = self.walker.tick(held, &self.place, now);
        if held.is_some() || moved == Moved::Stepped {
            self.picked = self.picked.filter(|_| moved != Moved::Stepped);
        }
        match moved {
            Moved::Door(mark) => return self.go_through(mark, now),
            Moved::Bumped => return Happened::Bumped,
            Moved::Stepped => return Happened::Stepped,
            Moved::Still => {}
        }
        // Arrived where a click or Tab was taking them: face it and use it.
        if let Some(target) = self.heading_for
            && !self.walker.moving()
            && !self.walker.routed()
        {
            self.heading_for = None;
            if let Some((_, facing)) = self
                .place
                .uses_of(target)
                .into_iter()
                .find(|(tile, _)| *tile == self.walker.tile)
            {
                self.walker.face(facing);
                return self.use_target(target, now);
            }
        }
        Happened::Nothing
    }

    /// The thing or person in front of the Handler now.
    pub fn faced(&self) -> Option<Target> {
        if self.walker.moving() {
            return None;
        }
        self.place.faced(self.walker.tile, self.walker.facing)
    }

    /// What the prompt offers: the target in front, or else the one Tab picked.
    pub fn offered(&self) -> Option<Target> {
        self.faced().or(self.picked)
    }

    /// The words of a target's prompt.
    pub fn prompt(&self, target: Target) -> String {
        match target {
            Target::Thing(i) => self.place.things[i].prompt.clone(),
            Target::Person(i) => format!("Talk to {}", person_name(&self.place.people[i].id)),
        }
    }

    /// The action key: uses what is in front, or walks to what Tab picked and uses it there.
    pub fn act(&mut self, now: f32) -> Happened {
        if let Some(target) = self.faced() {
            return self.use_target(target, now);
        }
        if let Some(target) = self.picked {
            return self.head_for(target, now);
        }
        Happened::Nothing
    }

    /// Tab: the next target in reading order, after the one picked or faced.
    pub fn pick_next(&mut self, backwards: bool) {
        let all = self.place.targets();
        if all.is_empty() {
            return;
        }
        let now_at = self
            .offered()
            .and_then(|t| all.iter().position(|a| *a == t));
        let next = match (now_at, backwards) {
            (None, false) => 0,
            (None, true) => all.len() - 1,
            (Some(i), false) => (i + 1) % all.len(),
            (Some(i), true) => (i + all.len() - 1) % all.len(),
        };
        self.picked = Some(all[next]);
    }

    /// A click on a tile of the place: walk there, or to what stands there and use it.
    pub fn click(&mut self, tile: (i32, i32), now: f32) -> Happened {
        let target = self
            .place
            .targets()
            .into_iter()
            .find(|t| self.place.tiles_of(*t).contains(&tile));
        if let Some(target) = target {
            return self.head_for(target, now);
        }
        if let Ground::Door(_) = self.place.ground(tile)
            && let Some(steps) = self.place.path(self.walker.tile, &[tile])
        {
            self.heading_for = None;
            self.walker.walk_to(steps);
            return Happened::Nothing;
        }
        if self.place.walkable(tile)
            && let Some(steps) = self.place.path(self.walker.tile, &[tile])
        {
            self.heading_for = None;
            self.walker.walk_to(steps);
        }
        Happened::Nothing
    }

    fn head_for(&mut self, target: Target, now: f32) -> Happened {
        let uses = self.place.uses_of(target);
        if let Some((_, facing)) = uses.iter().find(|(tile, _)| *tile == self.walker.tile) {
            self.walker.face(*facing);
            return self.use_target(target, now);
        }
        let goals: Vec<(i32, i32)> = uses.iter().map(|(t, _)| *t).collect();
        if let Some(steps) = self.place.path(self.walker.tile, &goals) {
            self.walker.walk_to(steps);
            self.heading_for = Some(target);
            self.picked = Some(target);
        }
        Happened::Nothing
    }

    fn use_target(&mut self, target: Target, now: f32) -> Happened {
        self.reached = now;
        self.picked = None;
        match target {
            Target::Thing(i) => Happened::Used(self.place.id, self.place.things[i].id.clone()),
            Target::Person(i) => Happened::Spoke(self.place.id, self.place.people[i].id.clone()),
        }
    }

    /// The window's view of the place now: the ground, then everyone from the back row forward.
    pub fn compose(&self, now: f32, reduce_motion: bool) -> Canvas {
        let feet = self.walker.at(now);
        let (cx, cy) = picture::camera(&self.place, feet);
        let mut world = self.ground.clone();
        // Everyone, drawn back to front by where their feet are.
        let mut figures: Vec<((f32, f32), Canvas)> = self
            .place
            .people
            .iter()
            .map(|p| {
                let frame = self.sheet.frame(p.facing, 4);
                (
                    (p.tile.0 as f32, p.tile.1 as f32),
                    picture::tinted(&frame, person_tint(&p.id)),
                )
            })
            .collect();
        let column = if now - self.reached < REACH && !self.walker.moving() {
            if now - self.reached < REACH / 2.0 {
                6
            } else {
                7
            }
        } else {
            self.walker.frame(now, reduce_motion)
        };
        figures.push((feet, self.sheet.frame(self.walker.facing, column)));
        figures.sort_by(|a, b| a.0.1.total_cmp(&b.0.1));
        for (at, frame) in &figures {
            picture::stand(&mut world, frame, *at);
        }
        let mut view = Canvas::new(picture::VIEW.0 as u32, picture::VIEW.1 as u32);
        for y in 0..picture::VIEW.1 {
            for x in 0..picture::VIEW.0 {
                let (wx, wy) = (cx + x, cy + y);
                let inside =
                    wx >= 0 && wy >= 0 && wx < world.width() as i32 && wy < world.height() as i32;
                view.set(x, y, if inside { world.get(wx, wy) } else { SURROUND });
            }
        }
        view
    }

    /// Dresses the Handler in a look from the Studio.
    pub fn dress(&mut self, look: &look::HandlerLook) {
        self.sheet = look.sheet();
    }

    /// Where the window's view starts in the place, in its pixels, for placing prompts over it
    /// and reading clicks.
    pub fn view_origin(&self, now: f32) -> (i32, i32) {
        picture::camera(&self.place, self.walker.at(now))
    }
}

/// How the townspeople are called.
pub fn person_name(id: &str) -> String {
    match id {
        "marlow" => "Marlow".to_owned(),
        "iris" => "Iris".to_owned(),
        "odile" => "Odile".to_owned(),
        "tailor" => "the tailor".to_owned(),
        "wren" => "Wren".to_owned(),
        other => other.to_owned(),
    }
}

/// The colour a townsperson is tinted until they have a sheet of their own.
fn person_tint(id: &str) -> Rgba {
    match id {
        "marlow" => Rgba::new(0x7a, 0x2c, 0x33, 255),
        "iris" => Rgba::new(0x28, 0x5a, 0x4b, 255),
        "odile" => Rgba::new(0x4a, 0x62, 0x88, 255),
        "tailor" => Rgba::new(0xa4, 0x7a, 0x2c, 255),
        _ => Rgba::new(0xb5, 0x54, 0x3f, 255),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ticks at 60 frames a second, holding a direction.
    fn hold(town: &mut Town, facing: Option<Facing>, from: f32, seconds: f32) -> Vec<Happened> {
        let mut out = Vec::new();
        let mut t = from;
        while t < from + seconds {
            out.push(town.tick(facing, t));
            t += 1.0 / 60.0;
        }
        out
    }

    #[test]
    fn the_handler_starts_at_the_desk_facing_the_room() {
        let town = Town::new(0.0);
        assert_eq!(town.place.id, PlaceId::Office);
        assert_eq!(town.walker.facing, Facing::Down);
    }

    #[test]
    fn turning_to_the_desk_offers_the_habitat() {
        let mut town = Town::new(0.0);
        town.tick(Some(Facing::Up), 0.0);
        town.tick(None, 0.05);
        let offered = town.offered().expect("the habitat is in front");
        assert_eq!(town.prompt(offered), "Look in the habitat");
        assert_eq!(
            town.act(0.1),
            Happened::Used(PlaceId::Office, "habitat".to_owned())
        );
    }

    #[test]
    fn walking_down_and_out_of_the_office_reaches_the_courtyard() {
        let mut town = Town::new(0.0);
        let happened = hold(&mut town, Some(Facing::Down), 0.0, 3.0);
        assert!(happened.contains(&Happened::Entered(PlaceId::Campus)));
        // Still holding down, they stay out: the key held through the door is ignored.
        assert_eq!(town.place.id, PlaceId::Campus);
    }

    #[test]
    fn tab_then_the_action_key_walks_to_a_thing_and_uses_it() {
        let mut town = Town::new(0.0);
        town.pick_next(false);
        let picked = town.picked.expect("something is picked");
        town.act(0.0);
        let happened = hold(&mut town, None, 0.0, 6.0);
        let expected = match picked {
            Target::Thing(i) => Happened::Used(PlaceId::Office, town.place.things[i].id.clone()),
            Target::Person(_) => unreachable!("nobody stands in the office"),
        };
        assert!(happened.contains(&expected), "{happened:?}");
    }

    #[test]
    fn a_click_on_someone_walks_over_and_speaks_to_them() {
        let mut town = Town::through(PlaceId::Arena, '1', 0.0);
        let marlow = town.place.people[0].tile;
        town.click(marlow, 0.0);
        let happened = hold(&mut town, None, 0.0, 6.0);
        assert!(happened.contains(&Happened::Spoke(PlaceId::Arena, "marlow".to_owned())));
    }

    #[test]
    fn every_door_of_the_campus_goes_in_and_comes_back_out_where_it_went_in() {
        for door in PlaceId::Campus.place().doors {
            let inside = Town::through(door.to, door.to_mark, 0.0);
            let mut town = inside;
            let way_out = town.place.door(door.to_mark).unwrap().tile;
            town.click(way_out, 0.0);
            hold(&mut town, None, 0.0, 6.0);
            assert_eq!(town.place.id, PlaceId::Campus, "out of {}", door.to.key());
            let (expected, _) = PlaceId::Campus.place().arrival(door.mark).unwrap();
            assert_eq!(town.walker.tile, expected, "out of {}", door.to.key());
        }
    }

    #[test]
    fn the_view_is_the_window_size_in_every_place() {
        for id in PlaceId::ALL {
            let town = Town::through(id, id.place().doors[0].mark, 0.0);
            let view = town.compose(0.0, false);
            assert_eq!((view.width() as i32, view.height() as i32), picture::VIEW);
        }
    }
}
