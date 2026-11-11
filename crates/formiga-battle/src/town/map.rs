//! The town's places as data: each a grid of 32-pixel tiles saying where the Handler can stand,
//! the things there are to use, and the doors between places. They are read from the `.place`
//! files in `town/` at the top of the repository, as the arenas are from theirs, so a room's
//! layout is changed by editing a text file, never the code.
//!
//! A place file is made of lines. `#` starts a comment. `name`, `size` and `ground` come first;
//! `ground` is followed by exactly `size` rows of tile characters. After the ground:
//!
//! - `thing <letter> <id> <words…>`: every tile marked with that letter is one thing, used from
//!   any open tile beside it while facing it; the words are what its prompt says.
//! - `door <digit> <place> <digit> <inward>`: the tile marked with that digit leads to the door
//!   of that digit in the other place; arriving here through it, the Handler steps `inward`.
//! - `person <id> <column> <row> <facing>`: someone who stands on that tile (Battle draws them).
//!
//! Ground characters: `.` `,` `=` `:` are open (floor, rug, lane, grass); `#` `~` `T` `^` are
//! closed (wall, water, tree, ledge); `@` is open and is where the Handler stands after the
//! arrival. Letters are things and digits are doors.

use std::collections::{HashMap, VecDeque};
use std::fmt;

/// One side of a tile, or the way someone faces. In the order of a character sheet's rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Facing {
    Down,
    Left,
    Right,
    Up,
}

impl Facing {
    pub const ALL: [Facing; 4] = [Facing::Down, Facing::Left, Facing::Right, Facing::Up];

    /// The step one tile this way takes, in columns and rows.
    pub fn step(self) -> (i32, i32) {
        match self {
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
            Self::Up => (0, -1),
        }
    }

    /// The row of a character sheet that faces this way.
    pub fn row(self) -> usize {
        match self {
            Self::Down => 0,
            Self::Left => 1,
            Self::Right => 2,
            Self::Up => 3,
        }
    }

    fn parse(word: &str) -> Option<Self> {
        match word {
            "down" => Some(Self::Down),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "up" => Some(Self::Up),
            _ => None,
        }
    }
}

/// The places in town.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlaceId {
    Office,
    Campus,
    Arena,
    Clubhouse,
    GuildHall,
    Studio,
}

impl PlaceId {
    pub const ALL: [PlaceId; 6] = [
        PlaceId::Office,
        PlaceId::Campus,
        PlaceId::Arena,
        PlaceId::Clubhouse,
        PlaceId::GuildHall,
        PlaceId::Studio,
    ];

    /// How the place files and door lines name it.
    pub fn key(self) -> &'static str {
        match self {
            Self::Office => "office",
            Self::Campus => "campus",
            Self::Arena => "arena",
            Self::Clubhouse => "clubhouse",
            Self::GuildHall => "guildhall",
            Self::Studio => "studio",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.key() == word)
    }

    /// The place file, built in.
    fn source(self) -> &'static str {
        match self {
            Self::Office => include_str!("../../../../town/office.place"),
            Self::Campus => include_str!("../../../../town/campus.place"),
            Self::Arena => include_str!("../../../../town/arena.place"),
            Self::Clubhouse => include_str!("../../../../town/clubhouse.place"),
            Self::GuildHall => include_str!("../../../../town/guildhall.place"),
            Self::Studio => include_str!("../../../../town/studio.place"),
        }
    }

    /// The place as built in. The files are checked by the tests, so this cannot fail in a
    /// build that passed them.
    pub fn place(self) -> Place {
        Place::parse(self, self.source())
            .unwrap_or_else(|e| panic!("the built-in {} is broken: {e}", self.key()))
    }
}

/// What a tile is, for walking and for the stand-in picture drawn until a place has its art.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ground {
    Floor,
    Rug,
    Lane,
    Grass,
    Wall,
    Water,
    Tree,
    Ledge,
    /// Part of a thing.
    Thing(usize),
    /// A door: the index of its line.
    Door(usize),
}

impl Ground {
    /// Whether the Handler can stand here.
    pub fn open(self) -> bool {
        matches!(
            self,
            Self::Floor | Self::Rug | Self::Lane | Self::Grass | Self::Door(_)
        )
    }
}

/// A thing to use: everything marked with its letter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thing {
    pub id: String,
    pub prompt: String,
    pub tiles: Vec<(i32, i32)>,
}

impl Thing {
    /// The tile its prompt stands over: the middle of its top row.
    pub fn head(&self) -> (i32, i32) {
        let top = self.tiles.iter().map(|t| t.1).min().unwrap_or(0);
        let row: Vec<i32> = self
            .tiles
            .iter()
            .filter(|t| t.1 == top)
            .map(|t| t.0)
            .collect();
        let (lo, hi) = (
            row.iter().copied().min().unwrap_or(0),
            row.iter().copied().max().unwrap_or(0),
        );
        ((lo + hi) / 2, top)
    }
}

/// A door to another place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Door {
    pub mark: char,
    pub tile: (i32, i32),
    pub to: PlaceId,
    pub to_mark: char,
    /// The way into this place from the door tile.
    pub inward: Facing,
}

/// Someone who stands in a place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Person {
    pub id: String,
    pub tile: (i32, i32),
    pub facing: Facing,
}

/// A place in town.
#[derive(Clone, Debug)]
pub struct Place {
    pub id: PlaceId,
    pub name: String,
    pub width: i32,
    pub height: i32,
    ground: Vec<Ground>,
    pub things: Vec<Thing>,
    pub doors: Vec<Door>,
    pub people: Vec<Person>,
    /// Where the Handler stands after the arrival, in the place that has one.
    pub spawn: Option<(i32, i32)>,
}

/// What is wrong with a place file, and on which line.
#[derive(Debug, PartialEq, Eq)]
pub struct Broken {
    pub line: usize,
    pub why: String,
}

impl fmt::Display for Broken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.why)
    }
}

impl Place {
    pub fn parse(id: PlaceId, text: &str) -> Result<Self, Broken> {
        let broken = |line: usize, why: String| Broken { line, why };
        let mut lines = text
            .lines()
            .enumerate()
            .map(|(i, l)| (i + 1, l.split('#').next().unwrap_or("").trim_end()))
            .filter(|(_, l)| !l.trim().is_empty());
        // `#` is also the wall: comments start only at a `#` with a space or the line's start
        // before it on header lines, so the ground rows are read raw below.
        let raw: Vec<&str> = text.lines().collect();

        let (n, name_line) = lines
            .next()
            .ok_or_else(|| broken(0, "the file is empty".into()))?;
        let name = name_line
            .strip_prefix("name ")
            .ok_or_else(|| broken(n, "the first line must be `name …`".into()))?
            .trim()
            .to_owned();
        let (n, size_line) = lines
            .next()
            .ok_or_else(|| broken(n, "no `size` line".into()))?;
        let sizes: Vec<i32> = size_line
            .strip_prefix("size ")
            .ok_or_else(|| broken(n, "the second line must be `size <columns> <rows>`".into()))?
            .split_whitespace()
            .map(|w| {
                w.parse()
                    .map_err(|_| broken(n, format!("{w:?} is not a size")))
            })
            .collect::<Result<_, _>>()?;
        let [width, height] = sizes[..] else {
            return Err(broken(n, "`size` takes two numbers".into()));
        };
        if !(1..=64).contains(&width) || !(1..=64).contains(&height) {
            return Err(broken(n, "a place is 1 to 64 tiles each way".into()));
        }
        let (n, ground_line) = lines
            .next()
            .ok_or_else(|| broken(n, "no `ground` line".into()))?;
        if ground_line.trim() != "ground" {
            return Err(broken(n, "the third line must be `ground`".into()));
        }

        // The ground rows, read raw since `#` is a wall in them.
        let mut marks = Vec::with_capacity((width * height) as usize);
        for row in 0..height as usize {
            let index = n + row; // `n` is 1-based, so this is the 0-based index of the next line.
            let line = raw
                .get(index)
                .ok_or_else(|| broken(index + 1, format!("only {row} of {height} ground rows")))?;
            let chars: Vec<char> = line.trim_end().chars().collect();
            if chars.len() != width as usize {
                return Err(broken(
                    index + 1,
                    format!("a ground row has {} tiles, not {width}", chars.len()),
                ));
            }
            marks.extend(chars);
        }
        let after = n + height as usize;
        let rest = raw
            .iter()
            .enumerate()
            .skip(after)
            .map(|(i, l)| (i + 1, l.split('#').next().unwrap_or("").trim()))
            .filter(|(_, l)| !l.is_empty());

        let mut things: Vec<Thing> = Vec::new();
        let mut letters: HashMap<char, usize> = HashMap::new();
        let mut doors: Vec<Door> = Vec::new();
        let mut digits: HashMap<char, usize> = HashMap::new();
        let mut people = Vec::new();
        for (n, line) in rest {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.as_slice() {
                ["thing", letter, id, prompt @ ..] if !prompt.is_empty() => {
                    let letter = single(letter)
                        .filter(char::is_ascii_alphabetic)
                        .filter(|c| *c != 'T')
                        .ok_or_else(|| broken(n, format!("{letter:?} is not a thing's letter")))?;
                    if letters.insert(letter, things.len()).is_some() {
                        return Err(broken(n, format!("{letter:?} is two things")));
                    }
                    things.push(Thing {
                        id: (*id).to_owned(),
                        prompt: prompt.join(" "),
                        tiles: Vec::new(),
                    });
                }
                ["door", digit, to, to_digit, inward] => {
                    let digit = single(digit)
                        .filter(char::is_ascii_digit)
                        .ok_or_else(|| broken(n, format!("{digit:?} is not a door's digit")))?;
                    let to_mark = single(to_digit)
                        .filter(char::is_ascii_digit)
                        .ok_or_else(|| broken(n, format!("{to_digit:?} is not a door's digit")))?;
                    let to = PlaceId::parse(to)
                        .ok_or_else(|| broken(n, format!("{to:?} is not a place")))?;
                    let inward = Facing::parse(inward)
                        .ok_or_else(|| broken(n, format!("{inward:?} is not a direction")))?;
                    if digits.insert(digit, doors.len()).is_some() {
                        return Err(broken(n, format!("{digit:?} is two doors")));
                    }
                    doors.push(Door {
                        mark: digit,
                        tile: (-1, -1),
                        to,
                        to_mark,
                        inward,
                    });
                }
                ["person", id, column, row, facing] => {
                    let at = |w: &str| {
                        w.parse::<i32>()
                            .map_err(|_| broken(n, format!("{w:?} is not a tile")))
                    };
                    let facing = Facing::parse(facing)
                        .ok_or_else(|| broken(n, format!("{facing:?} is not a direction")))?;
                    people.push(Person {
                        id: (*id).to_owned(),
                        tile: (at(column)?, at(row)?),
                        facing,
                    });
                }
                _ => {
                    return Err(broken(
                        n,
                        format!("{line:?} is not a thing, door or person"),
                    ));
                }
            }
        }

        let mut ground = Vec::with_capacity(marks.len());
        let mut spawn = None;
        for (i, mark) in marks.into_iter().enumerate() {
            let tile = (i as i32 % width, i as i32 / width);
            let row = n + 1 + tile.1 as usize;
            ground.push(match mark {
                '.' => Ground::Floor,
                ',' => Ground::Rug,
                '=' => Ground::Lane,
                ':' => Ground::Grass,
                '#' => Ground::Wall,
                '~' => Ground::Water,
                'T' => Ground::Tree,
                '^' => Ground::Ledge,
                '@' => {
                    if spawn.replace(tile).is_some() {
                        return Err(broken(row, "two `@` tiles".into()));
                    }
                    Ground::Floor
                }
                c if c.is_ascii_alphabetic() => {
                    let index = *letters
                        .get(&c)
                        .ok_or_else(|| broken(row, format!("{c:?} has no `thing` line")))?;
                    things[index].tiles.push(tile);
                    Ground::Thing(index)
                }
                c if c.is_ascii_digit() => {
                    let index = *digits
                        .get(&c)
                        .ok_or_else(|| broken(row, format!("{c:?} has no `door` line")))?;
                    if doors[index].tile != (-1, -1) {
                        return Err(broken(row, format!("door {c:?} is on two tiles")));
                    }
                    doors[index].tile = tile;
                    Ground::Door(index)
                }
                c => return Err(broken(row, format!("{c:?} is not a ground tile"))),
            });
        }
        if let Some(t) = things.iter().find(|t| t.tiles.is_empty()) {
            return Err(broken(0, format!("the thing {:?} is on no tile", t.id)));
        }
        if let Some(d) = doors.iter().find(|d| d.tile == (-1, -1)) {
            return Err(broken(0, format!("door {:?} is on no tile", d.mark)));
        }
        let place = Self {
            id,
            name,
            width,
            height,
            ground,
            things,
            doors,
            people,
            spawn,
        };
        for p in &place.people {
            if !place.ground(p.tile).open() {
                return Err(broken(
                    0,
                    format!("{} does not stand on an open tile", p.id),
                ));
            }
        }
        Ok(place)
    }

    /// The tile at a column and row; outside the place it is wall.
    pub fn ground(&self, (x, y): (i32, i32)) -> Ground {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return Ground::Wall;
        }
        self.ground[(y * self.width + x) as usize]
    }

    /// Whether the Handler can step onto a tile: open, and nobody standing there.
    pub fn walkable(&self, tile: (i32, i32)) -> bool {
        self.ground(tile).open() && !self.people.iter().any(|p| p.tile == tile)
    }

    /// The door with this mark.
    pub fn door(&self, mark: char) -> Option<&Door> {
        self.doors.iter().find(|d| d.mark == mark)
    }

    /// Where someone coming in through a door stands, and the way they face.
    pub fn arrival(&self, mark: char) -> Option<((i32, i32), Facing)> {
        let door = self.door(mark)?;
        let (dx, dy) = door.inward.step();
        Some(((door.tile.0 + dx, door.tile.1 + dy), door.inward))
    }

    /// The thing, or person, faced from a tile looking one way.
    pub fn faced(&self, from: (i32, i32), facing: Facing) -> Option<Target> {
        let (dx, dy) = facing.step();
        let tile = (from.0 + dx, from.1 + dy);
        if let Some(i) = self.people.iter().position(|p| p.tile == tile) {
            return Some(Target::Person(i));
        }
        match self.ground(tile) {
            Ground::Thing(i) => Some(Target::Thing(i)),
            _ => None,
        }
    }

    /// Every target in reading order: top to bottom, then left to right, by where its prompt
    /// stands. Tab goes through them in this order.
    pub fn targets(&self) -> Vec<Target> {
        let mut all: Vec<(Target, (i32, i32))> = self
            .things
            .iter()
            .enumerate()
            .map(|(i, t)| (Target::Thing(i), t.head()))
            .chain(
                self.people
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (Target::Person(i), p.tile)),
            )
            .collect();
        all.sort_by_key(|(_, (x, y))| (*y, *x));
        all.into_iter().map(|(t, _)| t).collect()
    }

    /// The tiles a target occupies.
    pub fn tiles_of(&self, target: Target) -> Vec<(i32, i32)> {
        match target {
            Target::Thing(i) => self.things[i].tiles.clone(),
            Target::Person(i) => vec![self.people[i].tile],
        }
    }

    /// The tile a target's prompt stands over.
    pub fn head_of(&self, target: Target) -> (i32, i32) {
        match target {
            Target::Thing(i) => self.things[i].head(),
            Target::Person(i) => self.people[i].tile,
        }
    }

    /// The shortest way on foot from one tile to any of the goals, as the steps to take; none
    /// when no goal can be reached. A door on the way is not walked through.
    pub fn path(&self, from: (i32, i32), goals: &[(i32, i32)]) -> Option<Vec<Facing>> {
        if goals.contains(&from) {
            return Some(Vec::new());
        }
        let index = |(x, y): (i32, i32)| (y * self.width + x) as usize;
        let mut came: Vec<Option<(usize, Facing)>> = vec![None; self.ground.len()];
        let mut seen = vec![false; self.ground.len()];
        let mut queue = VecDeque::from([from]);
        if self.ground(from) == Ground::Wall {
            return None;
        }
        seen[index(from)] = true;
        while let Some(at) = queue.pop_front() {
            for facing in Facing::ALL {
                let (dx, dy) = facing.step();
                let next = (at.0 + dx, at.1 + dy);
                if !self.walkable(next) || seen[index(next)] {
                    continue;
                }
                let goal = goals.contains(&next);
                if matches!(self.ground(next), Ground::Door(_)) && !goal {
                    continue;
                }
                seen[index(next)] = true;
                came[index(next)] = Some((index(at), facing));
                if goal {
                    let mut steps = Vec::new();
                    let mut i = index(next);
                    while let Some((before, facing)) = came[i] {
                        steps.push(facing);
                        i = before;
                    }
                    steps.reverse();
                    return Some(steps);
                }
                queue.push_back(next);
            }
        }
        None
    }

    /// The open tiles a target can be used from, each with the way to face it.
    pub fn uses_of(&self, target: Target) -> Vec<((i32, i32), Facing)> {
        let tiles = self.tiles_of(target);
        let mut out = Vec::new();
        for &(x, y) in &tiles {
            for facing in Facing::ALL {
                let (dx, dy) = facing.step();
                let from = (x - dx, y - dy);
                if self.walkable(from)
                    && !matches!(self.ground(from), Ground::Door(_))
                    && !tiles.contains(&from)
                    && !out.iter().any(|(t, _)| *t == from)
                {
                    out.push((from, facing));
                }
            }
        }
        out
    }
}

/// Something in a place the Handler can use or talk to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Thing(usize),
    Person(usize),
}

fn single(word: &str) -> Option<char> {
    let mut chars = word.chars();
    let c = chars.next()?;
    chars.next().is_none().then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_place_reads() {
        for id in PlaceId::ALL {
            let place = Place::parse(id, id.source());
            assert!(place.is_ok(), "{}: {}", id.key(), place.unwrap_err());
        }
    }

    #[test]
    fn every_door_leads_to_a_door_that_leads_back() {
        for id in PlaceId::ALL {
            let place = id.place();
            for door in &place.doors {
                let there = door.to.place();
                let back = there.door(door.to_mark).unwrap_or_else(|| {
                    panic!(
                        "{} door {} leads to {} door {}, which is not there",
                        id.key(),
                        door.mark,
                        door.to.key(),
                        door.to_mark
                    )
                });
                assert_eq!(
                    (back.to, back.to_mark),
                    (id, door.mark),
                    "{} door {} does not lead back",
                    id.key(),
                    door.mark
                );
                let (tile, _) = there.arrival(door.to_mark).expect("the door is there");
                assert!(
                    there.walkable(tile),
                    "coming into {} through door {} lands on a closed tile",
                    door.to.key(),
                    door.to_mark
                );
            }
        }
    }

    #[test]
    fn everything_in_every_place_can_be_reached_and_used() {
        for id in PlaceId::ALL {
            let place = id.place();
            let start = place.spawn.unwrap_or_else(|| {
                place
                    .arrival(place.doors[0].mark)
                    .expect("every place without a spawn has a door")
                    .0
            });
            for target in place.targets() {
                let uses: Vec<(i32, i32)> =
                    place.uses_of(target).into_iter().map(|(t, _)| t).collect();
                assert!(
                    place.path(start, &uses).is_some(),
                    "{}: {:?} cannot be reached",
                    id.key(),
                    target
                );
            }
            for door in &place.doors {
                assert!(
                    place.path(start, &[door.tile]).is_some(),
                    "{}: door {} cannot be reached",
                    id.key(),
                    door.mark
                );
            }
        }
    }

    #[test]
    fn only_the_office_has_where_the_handler_starts() {
        for id in PlaceId::ALL {
            assert_eq!(
                id.place().spawn.is_some(),
                id == PlaceId::Office,
                "{}",
                id.key()
            );
        }
    }

    #[test]
    fn the_campus_is_twenty_tiles_by_twelve() {
        let campus = PlaceId::Campus.place();
        assert_eq!((campus.width, campus.height), (20, 12));
    }

    #[test]
    fn a_broken_file_says_which_line() {
        let text = "name Test\nsize 3 2\nground\n#.#\n#?#\n";
        let err = Place::parse(PlaceId::Office, text).unwrap_err();
        assert_eq!(err.line, 5);
    }

    #[test]
    fn a_row_of_the_wrong_length_is_refused() {
        let text = "name Test\nsize 3 2\nground\n#.#\n#.\n";
        assert!(Place::parse(PlaceId::Office, text).is_err());
    }

    #[test]
    fn the_way_round_a_wall_is_found_and_a_way_through_it_is_not() {
        let text = "name Test\nsize 5 3\nground\n.....\n.###.\n.....\n";
        let place = Place::parse(PlaceId::Office, text).unwrap();
        let steps = place.path((0, 1), &[(4, 1)]).unwrap();
        assert_eq!(steps.len(), 6);
        let walled = "name Test\nsize 3 1\nground\n.#.\n";
        let place = Place::parse(PlaceId::Office, walled).unwrap();
        assert!(place.path((0, 0), &[(2, 0)]).is_none());
    }

    #[test]
    fn a_thing_is_used_from_beside_it_facing_it() {
        let text = "name Test\nsize 3 3\nground\n...\n.a.\n...\nthing a lamp Light the lamp\n";
        let place = Place::parse(PlaceId::Office, text).unwrap();
        assert_eq!(place.uses_of(Target::Thing(0)).len(), 4);
        assert_eq!(place.faced((1, 0), Facing::Down), Some(Target::Thing(0)));
        assert_eq!(place.faced((1, 0), Facing::Up), None);
    }
}
