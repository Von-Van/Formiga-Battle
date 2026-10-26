//! The card a match opens under, painted as the notebook's other pictures are: for each side a
//! slanted panel in its team's colours with speed lines through it, its pennant flown large on a
//! cane, a green ribbon for its name, a brass plate for its Handler, a cloth tab for its club, and
//! a stitched badge saying how it plays; between the two, a red burst with VS on it. The window
//! slides the two panels in from the sides and letters the words on top, as it does on every
//! other painted thing.

use crate::garden::{BRASS, PLANK};
use crate::heraldry::{self, Area};
use crate::identity::Style;
use crate::lettering::{self, Paint};
use crate::pennant::{CLOTH_H, CLOTH_W, cloth};
use formiga_art::paint::{self, Ramp, rgba};
use formiga_art::{Canvas, Rgba};

/// How tall a side's panel is, its shadow included.
pub const CREST_H: i32 = 64;
const TOP: i32 = 4;
const BOTTOM: i32 = CREST_H - 6;
/// How far the inner edge leans over its height.
const LEAN: i32 = 14;
/// The pennant is flown at twice its cloth.
const FLAG_SCALE: i32 = 2;
const SHADOW: Rgba = rgba(0x000000, 70);
const BADGE_CLOTH: Ramp = Ramp::new(0x8a7650, 0xc8b88e, 0xe6d8b2, 0xf6ecd0, 0xfffcf2);
const BURST: Ramp = Ramp::new(0x5a1e14, 0x9c3324, 0xd9573f, 0xf08a5c, 0xffc49a);

/// What a side's badge shows: how it plays, or who it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Badge {
    /// Presses first and sends everyone: two chevrons.
    Aggressor,
    /// Holds the yard: a shield.
    Defender,
    /// Goes round the sides: an arrow bending round.
    Tactician,
    /// Plays for what turns up: a glint.
    Opportunist,
    /// A friend's team: a heart.
    Friend,
    /// The colony's own: a star.
    Own,
}

impl Badge {
    pub fn of(archetype: formiga_battle_core::Archetype) -> Self {
        use formiga_battle_core::Archetype::*;
        match archetype {
            Aggressor => Self::Aggressor,
            Defender => Self::Defender,
            Tactician => Self::Tactician,
            Opportunist => Self::Opportunist,
        }
    }

    fn icon(self) -> [&'static str; 9] {
        match self {
            Self::Aggressor => [
                ".........",
                "#...#....",
                ".#...#...",
                "..#...#..",
                "...#...#.",
                "..#...#..",
                ".#...#...",
                "#...#....",
                ".........",
            ],
            Self::Defender => [
                "#########",
                "#.......#",
                "#.#####.#",
                "#.#####.#",
                "#.#####.#",
                ".#.###.#.",
                "..#.#.#..",
                "...#.#...",
                "....#....",
            ],
            Self::Tactician => [
                "..####...",
                ".#....#..",
                "#......#.",
                "#.......#",
                "#.....###",
                "#......#.",
                ".#.......",
                "..####...",
                ".........",
            ],
            Self::Opportunist => [
                "....#....",
                "....#....",
                "...###...",
                "..#####..",
                "#########",
                "..#####..",
                "...###...",
                "....#....",
                "....#....",
            ],
            Self::Friend => [
                ".........",
                ".##...##.",
                "####.####",
                "#########",
                "#########",
                ".#######.",
                "..#####..",
                "...###...",
                "....#....",
            ],
            Self::Own => [
                "....#....",
                "...###...",
                "...###...",
                "#########",
                ".#######.",
                "..#####..",
                "..##.##..",
                ".##...##.",
                "#.......#",
            ],
        }
    }
}

/// A stitched round badge, 17 pixels across, with its icon in the team's darkest shade.
pub const BADGE_W: i32 = 17;

pub fn badge(kind: Badge, team: Ramp) -> Canvas {
    let mut canvas = Canvas::new(BADGE_W as u32 + 1, BADGE_W as u32 + 1);
    let (c, r) = (8, 8);
    for y in 0..BADGE_W {
        for x in 0..BADGE_W {
            let (dx, dy) = ((x - c) as f32, (y - c) as f32);
            let d = (dx * dx + dy * dy).sqrt();
            if d > r as f32 + 0.4 {
                continue;
            }
            let colour = if d > r as f32 - 0.6 {
                BADGE_CLOTH.edge
            } else if d > r as f32 - 1.6 && (x + y) % 2 == 0 {
                // The stitching round its rim.
                team.base
            } else if dx + dy < -6.0 {
                BADGE_CLOTH.shine
            } else {
                BADGE_CLOTH.base
            };
            paint::put(&mut canvas, x + 1, y + 1, SHADOW);
            paint::put(&mut canvas, x, y, colour);
        }
    }
    for (row, line) in kind.icon().iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if ch == '#' {
                paint::put(&mut canvas, 4 + col as i32, 4 + row as i32, team.edge);
            }
        }
    }
    canvas
}

/// A picture made `k` times larger, pixel for pixel.
fn scaled(canvas: &Canvas, k: i32) -> Canvas {
    let (w, h) = (canvas.width() as i32, canvas.height() as i32);
    let mut out = Canvas::new((w * k) as u32, (h * k) as u32);
    for y in 0..h * k {
        for x in 0..w * k {
            out.set(x, y, canvas.get(x / k, y / k));
        }
    }
    out
}

/// The inner edge's column at row `y`, for a panel `w` wide.
fn inner(w: i32, y: i32) -> i32 {
    w - 1 - (y - TOP) * LEAN / (BOTTOM - TOP)
}

/// Where the words go on a side's panel, for the window to letter on: its name's ribbon, its
/// Handler's plate (the sunk panel), its club's tab, and the badge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrestPlaces {
    pub name: Area,
    pub plate: Area,
    pub club: Area,
    pub badge: Area,
}

fn left_places(w: i32) -> CrestPlaces {
    let x0 = 8 + CLOTH_W * FLAG_SCALE + 8;
    let room = inner(w, BOTTOM) - x0 - 6;
    let plate_w = (room - BADGE_W - 4).max(40);
    CrestPlaces {
        name: (x0, 9, room.max(40), 18),
        plate: (x0, 29, plate_w, heraldry::PLATE_H),
        club: (x0, 48, (room * 2 / 3).max(40), 9),
        badge: (x0 + plate_w + 4, 30, BADGE_W, BADGE_W),
    }
}

/// The places on a side's panel; the right side's are the left side's turned about.
pub fn crest_places(width: i32, side: usize) -> CrestPlaces {
    let w = width.max(160);
    let p = left_places(w);
    if side == 0 {
        return p;
    }
    let flip = |(x, y, aw, ah): Area| (w - x - aw, y, aw, ah);
    CrestPlaces {
        name: flip(p.name),
        plate: flip(p.plate),
        club: flip(p.club),
        badge: flip(p.badge),
    }
}

/// A side's panel, `width` wide, for `side` 0 (left) or 1 (right): the right side's is the left
/// side's turned about, so its pennant flies at the outer edge too.
pub fn crest(
    width: i32,
    side: usize,
    team: Ramp,
    flag: Option<Style>,
    mark: Option<Badge>,
) -> Canvas {
    let w = width.max(160);
    let mut canvas = Canvas::new(w as u32, CREST_H as u32);
    // The panel's shadow, then the panel: speed lines leaning the way it slides in, grain.
    for y in TOP..BOTTOM {
        for x in 0..=inner(w, y) {
            paint::put(&mut canvas, x + 2, y + 2, SHADOW);
        }
    }
    for y in TOP..BOTTOM {
        let edge_x = inner(w, y);
        for x in 0..=edge_x {
            let stripe = ((x + (BOTTOM - y) * 2) / 7).rem_euclid(3);
            let mut colour = match stripe {
                0 => team.shadow,
                _ => team.base,
            };
            if paint::noise(x, y, 811) % 100 < 6 {
                colour = team.light;
            }
            if y == TOP || x == 0 || x == edge_x || y == BOTTOM - 1 {
                colour = team.edge;
            } else if y == TOP + 1 {
                colour = team.shine;
            }
            canvas.set(x, y, colour);
        }
    }
    // The pennant, flown large on a cane at the outer edge.
    let cane = 5;
    paint::vline(&mut canvas, cane, 1, CREST_H - 4, PLANK.shadow);
    paint::vline(&mut canvas, cane + 1, 1, CREST_H - 4, PLANK.light);
    paint::rect(&mut canvas, cane - 1, 0, 4, 2, BRASS.base);
    paint::hline(&mut canvas, cane - 1, 0, 4, BRASS.shine);
    let flown = match flag {
        Some(style) => scaled(&cloth(style), FLAG_SCALE),
        None => {
            // A plain pennant in the team's colours, for a side with none of its own.
            let mut plain =
                Canvas::new((CLOTH_W * FLAG_SCALE) as u32, (CLOTH_H * FLAG_SCALE) as u32);
            let (fw, fh) = ((CLOTH_W * FLAG_SCALE) as f32, (CLOTH_H * FLAG_SCALE) as f32);
            paint::polygon(
                &mut plain,
                &[(0.0, 0.0), (fw, fh / 2.0), (0.0, fh)],
                team.light,
            );
            paint::line(&mut plain, (0, 0), (fw as i32, fh as i32 / 2), team.shine);
            paint::line(
                &mut plain,
                (0, fh as i32 - 1),
                (fw as i32, fh as i32 / 2),
                team.edge,
            );
            plain
        }
    };
    paint::blit(&mut canvas, &flown, cane + 2, 6);
    // The name's ribbon, the Handler's plate, the club's tab, the badge.
    let p = left_places(w);
    heraldry::ribbon(&mut canvas, p.name);
    paint::blit(
        &mut canvas,
        &heraldry::plate(p.plate.2),
        p.plate.0,
        p.plate.1,
    );
    tab(&mut canvas, p.club, team);
    if let Some(kind) = mark {
        paint::blit(&mut canvas, &badge(kind, team), p.badge.0, p.badge.1);
    }
    if side == 1 { mirrored(&canvas) } else { canvas }
}

/// A plain cloth tab in the team's darkest shade, stitched along both edges, for the club's
/// name: plain, so the small words on it read clearly.
fn tab(canvas: &mut Canvas, (x, y, w, h): Area, team: Ramp) {
    paint::rect(canvas, x + 1, y + 1, w, h, SHADOW);
    paint::rect(canvas, x, y, w, h, team.edge);
    paint::hline(canvas, x, y, w, team.shadow);
    let mut along = x + 2;
    while along + 2 < x + w - 1 {
        paint::hline(canvas, along, y + 1, 2, BADGE_CLOTH.light);
        paint::hline(canvas, along, y + h - 2, 2, BADGE_CLOTH.shadow);
        along += 4;
    }
}

/// The burst between the two panels, with VS on it.
pub const VERSUS_W: i32 = 44;

pub fn versus() -> Canvas {
    let mut canvas = Canvas::new(VERSUS_W as u32, VERSUS_W as u32);
    let c = VERSUS_W as f32 / 2.0;
    let points: Vec<(f32, f32)> = (0..24)
        .map(|k| {
            let a = k as f32 / 24.0 * std::f32::consts::TAU;
            let r = if k % 2 == 0 { c - 1.0 } else { c * 0.62 };
            (c + a.cos() * r, c + a.sin() * r)
        })
        .collect();
    let shadow: Vec<(f32, f32)> = points.iter().map(|(x, y)| (x + 1.5, y + 2.0)).collect();
    paint::polygon(&mut canvas, &shadow, SHADOW);
    paint::polygon(&mut canvas, &points, BURST.edge);
    let inner: Vec<(f32, f32)> = points
        .iter()
        .map(|(x, y)| (c + (x - c) * 0.88, c + (y - c) * 0.88))
        .collect();
    paint::polygon(&mut canvas, &inner, BURST.base);
    let core: Vec<(f32, f32)> = points
        .iter()
        .map(|(x, y)| (c + (x - c) * 0.5, c + (y - c) * 0.5 - 1.0))
        .collect();
    paint::polygon(&mut canvas, &core, BURST.light);
    let word = lettering::word("VS", 2, Paint::GOLD);
    let (ww, wh) = (word.width() as i32, word.height() as i32);
    paint::blit(
        &mut canvas,
        &word,
        (VERSUS_W - ww) / 2 + 1,
        (VERSUS_W - wh) / 2 + 1,
    );
    canvas
}

fn mirrored(canvas: &Canvas) -> Canvas {
    let (w, h) = (canvas.width() as i32, canvas.height() as i32);
    let mut out = Canvas::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            out.set(w - 1 - x, y, canvas.get(x, y));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::TEAM;

    #[test]
    fn each_side_has_its_ribbon_plate_and_badge_where_the_window_letters_them() {
        for (side, team) in TEAM.into_iter().enumerate() {
            let card = crest(
                220,
                side,
                team,
                Some(Style::default()),
                Some(Badge::Defender),
            );
            let p = crest_places(220, side);
            for (x, y, w, h) in [p.name, p.plate, p.club, p.badge] {
                assert_eq!(
                    card.get(x + w / 2, y + h / 2).a,
                    255,
                    "painted under {x},{y}"
                );
            }
        }
        assert_ne!(
            crest(220, 0, TEAM[0], None, Some(Badge::Aggressor)),
            crest(220, 0, TEAM[0], None, Some(Badge::Opportunist)),
            "a badge says how a side plays"
        );
    }

    #[test]
    fn every_badge_is_its_own_picture() {
        let all = [
            Badge::Aggressor,
            Badge::Defender,
            Badge::Tactician,
            Badge::Opportunist,
            Badge::Friend,
            Badge::Own,
        ];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(badge(*a, TEAM[0]), badge(*b, TEAM[0]), "{a:?} and {b:?}");
            }
        }
        assert_eq!(versus().get(VERSUS_W / 2, VERSUS_W / 2).a, 255);
    }
}
