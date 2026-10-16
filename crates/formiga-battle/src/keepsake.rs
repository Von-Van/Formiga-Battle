//! The little things a match is told with in place of words, painted at the notebook's pixel as
//! its other pictures are: lit from the upper left, edges a darker shade of their own colour,
//! grain from `paint::noise`. What a match earned, each beside its own keepsake (a brass medal for
//! points, a journal, a notebook page and pencil, a cup's rosette, a trophy, a key); a padlock on
//! a place still to open; the brass studs along the leather counting the turns; the rubber stamp
//! that comes down on the board when the match ends; and the rosette pinned beside the result.

use crate::garden::{BRASS, PLANK};
use crate::identity::FIELDS;
use formiga_art::paint::{self, Ramp, rgb, rgba};
use formiga_art::{Canvas, Rgba};

const SHADOW: Rgba = rgba(0x000000, 60);
const CLEAR: Rgba = rgba(0x000000, 0);
const LEATHER: Ramp = Ramp::new(0x2f1c12, 0x4a2e1e, 0x6b4630, 0x8a6145, 0xa98060);
const PAPER: Ramp = Ramp::new(0xb8a682, 0xe2d3ae, 0xf6ecd0, 0xfcf5e2, 0xfffcf2);
const IRON: Ramp = Ramp::new(0x2c2a30, 0x48434a, 0x67616a, 0x8a848c, 0xb3aeb5);
const RULE: Rgba = rgb(0x9cb8cf);
const MARGIN: Rgba = rgb(0xd98b84);
const PENCIL: Rgba = rgba(0x6e5a44, 120);
const PENCIL_SMUDGE: Rgba = rgba(0x6e5a44, 28);
/// The rubber stamp's red, a little thin in places as ink is.
const STAMP: Rgba = rgb(0xb5543f);

/// What a line of what a match earned is about, for its keepsake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keepsake {
    /// Open Circuit points, from a friend match.
    Points,
    /// A match played for the three, with a friend.
    Played,
    /// A win toward a cup, by its place in `rewards::CUPS`.
    Cup(usize),
    /// A cup won outright: a trophy for the shelf.
    Trophy(usize),
    /// The next place opened.
    Unlocked,
    /// A journal entry for the colony.
    Journal,
    /// A line in the Handler's Notebook.
    Notebook,
    /// How a named trio stands now.
    Trio,
    /// Anything else: a note, such as a match that earned nothing.
    Note,
}

/// How large a keepsake is, its shadow included.
pub const ICON_W: u32 = 16;
pub const ICON_H: u32 = 16;

/// A keepsake's picture, `ICON_W` by `ICON_H`.
pub fn icon(keepsake: Keepsake) -> Canvas {
    let mut c = Canvas::new(ICON_W, ICON_H);
    match keepsake {
        Keepsake::Points => medal(&mut c),
        Keepsake::Played => ticket(&mut c),
        Keepsake::Cup(cup) => {
            let r = small_rosette(cup, true);
            paint::blit(&mut c, &r, 2, 1);
        }
        Keepsake::Trophy(_) => trophy(&mut c),
        Keepsake::Unlocked => key(&mut c),
        Keepsake::Journal => journal(&mut c),
        Keepsake::Notebook => page(&mut c),
        Keepsake::Trio => trio_pennant(&mut c),
        Keepsake::Note => slip(&mut c),
    }
    c
}

/// A brass medal on a striped ribbon, a star struck in it.
fn medal(c: &mut Canvas) {
    let (red, blue) = (FIELDS[1].2, FIELDS[2].2);
    for (x0, ramp, lean) in [(4, red, 1), (8, blue, -1)] {
        for k in 0..6 {
            let x = x0 + (k * lean) / 3;
            paint::hline(c, x, k, 3, ramp.base);
            paint::put(c, x, k, ramp.light);
            paint::put(c, x + 3, k, ramp.edge);
        }
    }
    let (cx, cy) = (7, 10);
    paint::ellipse(c, cx + 1, cy + 1, 4, 4, SHADOW);
    paint::ellipse(c, cx, cy, 4, 4, BRASS.edge);
    paint::ellipse(c, cx, cy, 3, 3, BRASS.base);
    paint::hline(c, cx - 2, cy - 3, 3, BRASS.light);
    paint::put(c, cx - 2, cy - 2, BRASS.shine);
    // The star, struck in.
    for (dx, dy) in [
        (0, -2),
        (0, -1),
        (-2, 0),
        (-1, 0),
        (0, 0),
        (1, 0),
        (2, 0),
        (-1, 1),
        (1, 1),
    ] {
        paint::put(c, cx + dx, cy + dy, BRASS.shine);
    }
    paint::put(c, cx - 1, cy + 2, BRASS.shadow);
    paint::put(c, cx + 1, cy + 2, BRASS.shadow);
}

/// A railway ticket stub, punched: a match played away with a friend.
fn ticket(c: &mut Canvas) {
    paint::rect(c, 2, 5, 13, 9, SHADOW);
    paint::bevel(c, 1, 4, 13, 9, PAPER);
    let band = FIELDS[2].2;
    paint::rect(c, 2, 5, 4, 7, band.base);
    paint::vline(c, 2, 5, 7, band.light);
    // The perforation, then two lines of print.
    for y in (5..12).step_by(2) {
        paint::put(c, 6, y, PAPER.shadow);
    }
    paint::hline(c, 8, 7, 4, PAPER.edge);
    paint::hline(c, 8, 9, 3, PAPER.shadow);
    // Punched through.
    c.set(3, 8, CLEAR);
    c.set(4, 8, CLEAR);
    paint::put(c, 3, 9, band.edge);
}

/// A small brass cup on a wooden plinth.
fn trophy(c: &mut Canvas) {
    paint::rect(c, 4, 13, 9, 3, SHADOW);
    // The bowl, narrowing to its foot: each row's left end and width.
    for (row, (x, w)) in [(3, 11), (3, 11), (3, 11), (4, 9), (4, 9), (5, 7), (6, 5)]
        .into_iter()
        .enumerate()
    {
        let y = row as i32 + 2;
        paint::hline(c, x, y, w, BRASS.base);
        paint::put(c, x, y, BRASS.edge);
        paint::put(c, x + w - 1, y, BRASS.edge);
        paint::put(c, x + 1, y, BRASS.light);
        paint::put(c, x + w - 2, y, BRASS.shadow);
    }
    paint::hline(c, 3, 2, 11, BRASS.shine);
    paint::put(c, 5, 4, BRASS.shine);
    // The handles.
    for (x, dx) in [(2, -1), (14, 1)] {
        paint::put(c, x, 3, BRASS.edge);
        paint::put(c, x + dx, 4, BRASS.edge);
        paint::put(c, x, 5, BRASS.edge);
    }
    paint::vline(c, 8, 9, 2, BRASS.shadow);
    paint::hline(c, 6, 11, 5, BRASS.base);
    paint::hline(c, 6, 11, 2, BRASS.light);
    paint::bevel(c, 4, 12, 9, 3, PLANK);
}

/// A brass key: an open bow, its shaft, two teeth.
fn key(c: &mut Canvas) {
    paint::ellipse(c, 5, 8, 4, 4, SHADOW);
    paint::ellipse(c, 4, 7, 4, 4, BRASS.edge);
    paint::ellipse(c, 4, 7, 3, 3, BRASS.base);
    for (x, y) in [(4, 6), (4, 7), (4, 8), (3, 7), (5, 7)] {
        c.set(x, y, CLEAR);
    }
    paint::put(c, 2, 5, BRASS.shine);
    paint::put(c, 3, 4, BRASS.light);
    paint::hline(c, 8, 8, 7, SHADOW);
    paint::hline(c, 8, 6, 7, BRASS.light);
    paint::hline(c, 8, 7, 7, BRASS.shadow);
    paint::vline(c, 12, 8, 3, BRASS.base);
    paint::vline(c, 14, 8, 2, BRASS.base);
    paint::put(c, 12, 10, BRASS.edge);
    paint::put(c, 14, 9, BRASS.edge);
}

/// The colony's journal, bound in leather with a brass clasp.
fn journal(c: &mut Canvas) {
    paint::rect(c, 4, 3, 11, 13, SHADOW);
    // The pages' edge, peeping out at the fore-edge.
    paint::rect(c, 4, 2, 10, 12, PAPER.light);
    for y in (3..13).step_by(2) {
        paint::put(c, 13, y, PAPER.shadow);
    }
    paint::bevel(c, 2, 1, 11, 13, LEATHER);
    paint::vline(c, 4, 2, 11, LEATHER.shadow);
    // A tooled panel on the cover.
    paint::hline(c, 6, 4, 5, BRASS.shadow);
    paint::hline(c, 6, 10, 5, BRASS.shadow);
    paint::vline(c, 6, 4, 7, BRASS.shadow);
    paint::vline(c, 10, 4, 7, BRASS.shadow);
    paint::put(c, 8, 7, BRASS.light);
    // The clasp.
    paint::rect(c, 12, 6, 3, 3, BRASS.base);
    paint::put(c, 12, 6, BRASS.shine);
    paint::put(c, 14, 8, BRASS.edge);
}

/// A ruled page from the Handler's Notebook with a pencil across it.
fn page(c: &mut Canvas) {
    paint::rect(c, 3, 2, 11, 14, SHADOW);
    paint::bevel(c, 2, 1, 11, 13, PAPER);
    for y in (4..13).step_by(2) {
        paint::hline(c, 3, y, 9, RULE);
    }
    paint::vline(c, 4, 2, 11, MARGIN);
    // The pencil, lying from the lower left to the upper right.
    let yellow = Ramp::new(0x8a5a12, 0xc78a1e, 0xe8b33a, 0xf5d36b, 0xfbe9a6);
    for k in 0..9 {
        let (x, y) = (6 + k, 13 - k);
        paint::put(c, x, y, yellow.base);
        paint::put(c, x - 1, y, yellow.light);
        paint::put(c, x, y + 1, yellow.edge);
    }
    paint::put(c, 5, 14, rgb(0xe9c49c));
    paint::put(c, 4, 15, rgb(0x3b2b3a));
    paint::put(c, 15, 4, rgb(0xd9878f));
    paint::put(c, 14, 4, rgb(0xd9878f));
    paint::put(c, 15, 5, rgb(0x9b5560));
}

/// A little green pennant on its cane: a named trio.
fn trio_pennant(c: &mut Canvas) {
    let green = FIELDS[0].2;
    paint::vline(c, 3, 1, 14, PLANK.shadow);
    paint::vline(c, 4, 1, 14, PLANK.light);
    paint::rect(c, 2, 0, 4, 2, BRASS.base);
    paint::polygon(c, &[(5.0, 2.0), (14.0, 6.0), (5.0, 10.0)], green.base);
    paint::line(c, (5, 2), (13, 6), green.light);
    paint::line(c, (5, 10), (13, 6), green.edge);
    paint::put(c, 8, 6, green.shine);
    paint::hline(c, 1, 15, 6, rgb(0x447a3c));
}

/// A folded paper slip.
fn slip(c: &mut Canvas) {
    paint::rect(c, 3, 4, 12, 9, SHADOW);
    paint::bevel(c, 2, 3, 12, 9, PAPER);
    paint::hline(c, 4, 6, 7, PAPER.edge);
    paint::hline(c, 4, 8, 5, PAPER.shadow);
    paint::put(c, 13, 3, PAPER.shadow);
    paint::put(c, 12, 4, PAPER.shadow);
}

/// A cup's rosette small enough to sit in a row: its ribbon in the cup's colour round a brass
/// button, with two short tails. One still to win is pencilled on the page.
pub const ROSETTE_W: u32 = 11;
pub const ROSETTE_H: u32 = 14;

pub fn small_rosette(cup: usize, won: bool) -> Canvas {
    let mut c = Canvas::new(ROSETTE_W, ROSETTE_H);
    let (cx, cy) = (5, 5);
    let ramp = crate::heraldry::cup_ribbon(cup);
    if !won {
        for py in 0..11 {
            for px in 0..11 {
                let (dx, dy) = ((px - cx) as f32, (py - cy) as f32);
                let r = (dx * dx + dy * dy).sqrt();
                if (3.9..=4.9).contains(&r) {
                    paint::put(&mut c, px, py, PENCIL);
                } else if r < 3.9 {
                    paint::put(&mut c, px, py, PENCIL_SMUDGE);
                }
            }
        }
        paint::line(&mut c, (cx - 1, cy + 5), (cx - 2, cy + 8), PENCIL_SMUDGE);
        paint::line(&mut c, (cx + 1, cy + 5), (cx + 2, cy + 8), PENCIL_SMUDGE);
        return c;
    }
    for (dx, lean) in [(-2, -1), (1, 1)] {
        for k in 0..7 {
            let x0 = cx + dx + (k * lean) / 4;
            let y = cy + 3 + k;
            let notch = k == 6;
            paint::put(&mut c, x0, y, ramp.light);
            if !notch {
                paint::put(&mut c, x0 + 1, y, ramp.base);
            }
            paint::put(&mut c, x0 + 2, y, ramp.edge);
        }
    }
    for py in 0..11 {
        for px in 0..11 {
            let (dx, dy) = ((px - cx) as f32, (py - cy) as f32);
            let r = (dx * dx + dy * dy).sqrt();
            if r > 5.0 {
                continue;
            }
            let a = dy.atan2(dx);
            let pleat = ((a / std::f32::consts::TAU * 12.0).floor() as i32).rem_euclid(2) == 0;
            let colour = if r > 4.2 {
                ramp.edge
            } else if pleat {
                ramp.light
            } else {
                ramp.base
            };
            paint::put(&mut c, px, py, colour);
        }
    }
    paint::ellipse(&mut c, cx, cy, 2, 2, BRASS.edge);
    paint::rect(&mut c, cx - 1, cy - 1, 3, 3, BRASS.base);
    paint::put(&mut c, cx - 1, cy - 1, BRASS.shine);
    c
}

/// A padlock, hung on a place still to open.
pub const PADLOCK_W: u32 = 11;
pub const PADLOCK_H: u32 = 14;

pub fn padlock() -> Canvas {
    let mut c = Canvas::new(PADLOCK_W, PADLOCK_H);
    // The shackle, an iron hoop.
    for (x, y) in [
        (3, 5),
        (3, 4),
        (3, 3),
        (4, 2),
        (4, 1),
        (5, 1),
        (6, 1),
        (6, 2),
        (7, 3),
        (7, 4),
        (7, 5),
    ] {
        paint::put(&mut c, x, y, IRON.base);
    }
    paint::put(&mut c, 4, 1, IRON.light);
    paint::put(&mut c, 5, 0, IRON.edge);
    paint::put(&mut c, 2, 3, IRON.edge);
    paint::put(&mut c, 8, 3, IRON.edge);
    paint::rect(&mut c, 2, 7, 9, 7, SHADOW);
    paint::bevel(&mut c, 1, 6, 9, 7, BRASS);
    // The keyhole.
    paint::put(&mut c, 5, 8, BRASS.edge);
    paint::vline(&mut c, 5, 9, 2, BRASS.edge);
    paint::put(&mut c, 4, 9, BRASS.shadow);
    c
}

/// The turns of a match as a row of brass studs along the leather: those played pressed home,
/// the one being given shining, those to come still dark. The last turn's stud has a brass ring
/// round it all along, so the end can be seen coming.
pub const STUD: i32 = 9;
pub const STUD_H: u32 = 11;

pub fn studs(turn: u8, cap: u8, glow: bool) -> Canvas {
    let n = i32::from(cap.max(1));
    let mut c = Canvas::new((n * STUD + 1) as u32, STUD_H);
    for k in 0..n {
        let (x, y) = (k * STUD + 4, 5);
        let number = k + 1;
        if number == n {
            // The last turn's ring, set round its stud.
            paint::ellipse(&mut c, x, y, 5, 5, BRASS.edge);
            paint::ellipse(&mut c, x, y, 4, 4, BRASS.shadow);
            paint::put(&mut c, x - 3, y - 2, BRASS.light);
            paint::put(&mut c, x - 2, y - 3, BRASS.light);
        }
        if number < i32::from(turn) {
            // Played: pressed home, dull brass.
            paint::ellipse(&mut c, x, y, 3, 3, BRASS.edge);
            paint::ellipse(&mut c, x, y, 2, 2, BRASS.shadow);
            paint::put(&mut c, x - 1, y - 1, BRASS.base);
        } else if number == i32::from(turn) {
            // Now: bright, and glinting.
            if glow {
                paint::ellipse(&mut c, x, y, 5, 5, rgba(0xfbe6a2, 90));
            }
            paint::ellipse(&mut c, x, y, 3, 3, BRASS.edge);
            paint::ellipse(&mut c, x, y, 2, 2, BRASS.base);
            paint::rect(&mut c, x - 2, y - 2, 2, 2, BRASS.light);
            paint::put(&mut c, x - 1, y - 1, BRASS.shine);
            paint::put(&mut c, x + 1, y + 1, BRASS.shadow);
        } else {
            // To come: an empty socket in the leather, its lower lip catching the light.
            paint::ellipse(&mut c, x, y, 3, 3, LEATHER.edge);
            paint::ellipse(&mut c, x, y, 2, 2, LEATHER.shadow);
            paint::hline(&mut c, x - 1, y + 3, 3, LEATHER.light);
            paint::put(&mut c, x + 2, y + 2, LEATHER.light);
        }
    }
    c
}

/// A garden sign for a place's name, `width` pixels wide: a painted plank nailed across two
/// stakes driven into a tuft of grass. The name is lettered on the plank, at `sign_board`.
pub const SIGN_H: i32 = 24;

pub fn signboard(width: i32) -> Canvas {
    let w = width.max(40);
    let mut c = Canvas::new(w as u32, SIGN_H as u32);
    let (x, y, bw, bh) = sign_board(w);
    // The stakes, behind the plank, down into the grass.
    for sx in [x + 6, x + bw - 8] {
        paint::rect(&mut c, sx + 1, y + 2, 2, SIGN_H - y - 3, SHADOW);
        paint::vline(&mut c, sx, y + 2, SIGN_H - y - 3, PLANK.light);
        paint::vline(&mut c, sx + 1, y + 2, SIGN_H - y - 3, PLANK.shadow);
    }
    let grass = crate::garden::GRASS;
    for gx in x + 2..x + bw - 2 {
        let tall = (paint::noise(gx, 0, 991) % 3) as i32;
        paint::vline(&mut c, gx, SIGN_H - 2 - tall, tall + 1, grass.light);
        paint::put(&mut c, gx, SIGN_H - 1, grass.shadow);
    }
    // The plank: grained along its length, bevelled, nailed at each end.
    paint::rect(&mut c, x + 2, y + 2, bw, bh, SHADOW);
    paint::bevel(&mut c, x, y, bw, bh, PLANK);
    for py in y + 2..y + bh - 2 {
        for px in x + 2..x + bw - 2 {
            if paint::noise(px / 3, py, 2203).is_multiple_of(5) {
                paint::put(&mut c, px, py, paint::faded(PLANK.shadow, 110));
            }
        }
    }
    for nx in [x + 3, x + bw - 5] {
        paint::put(&mut c, nx, y + 3, IRON.base);
        paint::put(&mut c, nx, y + bh - 4, IRON.base);
        paint::put(&mut c, nx - 1, y + 2, IRON.light);
    }
    c
}

/// Where a garden sign's plank is, for lettering the name on.
pub fn sign_board(width: i32) -> (i32, i32, i32, i32) {
    (0, 0, width.max(40) - 2, 15)
}

/// The rubber stamp that comes down on the board: `text` in the board's pixel letters, `scale`
/// pixels to each of theirs, inside a double rule, inked unevenly as a stamp is, and pressed on
/// a little askew, rising to the right a pixel at a time.
pub fn stamp(text: &str, scale: i32) -> Canvas {
    let scale = scale.max(1);
    let (width, rows) = crate::lettering::mask(text);
    let pad = 6;
    let w = width * scale + pad * 2;
    let h = crate::lettering::HEIGHT * scale + pad * 2;
    let mut flat = Canvas::new(w as u32, h as u32);
    for (y, row) in rows.iter().enumerate() {
        for (x, on) in row.iter().enumerate() {
            if *on {
                paint::rect(
                    &mut flat,
                    pad + x as i32 * scale,
                    pad + y as i32 * scale,
                    scale,
                    scale,
                    STAMP,
                );
            }
        }
    }
    // The double rule: a heavy one outside, a fine one within.
    for inset in [0, 1] {
        paint::hline(&mut flat, inset, inset, w - inset * 2, STAMP);
        paint::hline(&mut flat, inset, h - 1 - inset, w - inset * 2, STAMP);
        paint::vline(&mut flat, inset, inset, h - inset * 2, STAMP);
        paint::vline(&mut flat, w - 1 - inset, inset, h - inset * 2, STAMP);
    }
    paint::hline(&mut flat, 3, 3, w - 6, STAMP);
    paint::hline(&mut flat, 3, h - 4, w - 6, STAMP);
    paint::vline(&mut flat, 3, 3, h - 6, STAMP);
    paint::vline(&mut flat, w - 4, 3, h - 6, STAMP);
    // Inked unevenly: some pixels thin, a few missed, as rubber on paper leaves it.
    for y in 0..h {
        for x in 0..w {
            if flat.get(x, y).a == 0 {
                continue;
            }
            let n = paint::noise(x, y, 7717) % 40;
            let ink = match n {
                0 => CLEAR,
                1..=3 => paint::faded(STAMP, 170),
                _ => paint::faded(STAMP, 240),
            };
            flat.set(x, y, ink);
        }
    }
    // Askew: each column lifted a pixel more every few across.
    let rise = w / 11 + 1;
    let mut out = Canvas::new(w as u32, (h + rise) as u32);
    for x in 0..w {
        let lift = x / 11;
        for y in 0..h {
            let p = flat.get(x, y);
            if p.a > 0 {
                out.set(x, y + rise - lift, p);
            }
        }
    }
    out
}

/// How a match ended for the rosette pinned beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Award {
    /// Won, toward the cup by its place in `rewards::CUPS` where there is one.
    Won(Option<usize>),
    /// Lost: a green rosette for playing well.
    Lost,
    Drawn,
    /// A Field Lesson learned: a brass star.
    Learned,
    /// A Field Lesson not learned yet: a pencilled star, to come back for.
    NotYet,
}

/// The rosette pinned beside a result, `AWARD_W` by `AWARD_H`.
pub const AWARD_W: u32 = 25;
pub const AWARD_H: u32 = 34;

pub fn award(award: Award) -> Canvas {
    let mut c = Canvas::new(AWARD_W, AWARD_H);
    let (cx, cy) = (12, 12);
    let gold = Ramp::new(0x6e4f1c, 0xa47a2c, 0xcf9f3e, 0xe8c25a, 0xfbe6a2);
    let rings: [Ramp; 2] = match award {
        Award::Won(cup) => [gold, cup.map_or(FIELDS[1].2, crate::heraldry::cup_ribbon)],
        Award::Lost => [FIELDS[0].2, FIELDS[0].2],
        Award::Drawn => [FIELDS[2].2, FIELDS[1].2],
        Award::Learned | Award::NotYet => [gold, gold],
    };
    if matches!(award, Award::Learned | Award::NotYet) {
        star(&mut c, cx, cy + 2, award == Award::Learned);
        return c;
    }
    // Two tails, notched, behind.
    for (dx, lean, ramp) in [(-5, -1, rings[1]), (1, 1, rings[1])] {
        for k in 0..18 {
            let x0 = cx + dx + (k * lean) / 5;
            let y = cy + 6 + k;
            let notch = k >= 16;
            for px in 0..5 {
                if notch && (1..4).contains(&px) && (k == 17 || px == 2) {
                    continue;
                }
                let colour = match px {
                    0 => ramp.light,
                    4 => ramp.edge,
                    _ => ramp.base,
                };
                paint::put(&mut c, x0 + px, y, colour);
            }
        }
    }
    // Two pleated rings, the outer in the first colour, the inner in the second.
    paint::ellipse(&mut c, cx + 1, cy + 1, 12, 12, SHADOW);
    for py in 0..25 {
        for px in 0..25 {
            let (dx, dy) = ((px - cx) as f32, (py - cy) as f32);
            let r = (dx * dx + dy * dy).sqrt();
            if r > 11.9 {
                continue;
            }
            let a = dy.atan2(dx);
            let (ramp, pleats, edge_at) = if r > 7.0 {
                (rings[0], 24.0, 11.0)
            } else {
                (rings[1], 16.0, 6.3)
            };
            let pleat = ((a / std::f32::consts::TAU * pleats).floor() as i32).rem_euclid(2) == 0;
            // Lit from the upper left.
            let lit = dx + dy < -4.0;
            let colour = if r > edge_at {
                ramp.edge
            } else if pleat {
                if lit { ramp.shine } else { ramp.light }
            } else if lit {
                ramp.light
            } else {
                ramp.base
            };
            paint::put(&mut c, px, py, colour);
        }
    }
    // The brass button.
    paint::ellipse(&mut c, cx, cy, 4, 4, BRASS.edge);
    paint::ellipse(&mut c, cx, cy, 3, 3, BRASS.base);
    paint::rect(&mut c, cx - 2, cy - 2, 2, 2, BRASS.light);
    paint::put(&mut c, cx - 2, cy - 2, BRASS.shine);
    paint::put(&mut c, cx + 2, cy + 2, BRASS.shadow);
    c
}

/// A five-pointed star, brass where it was earned and pencilled where it is still to earn.
fn star(c: &mut Canvas, cx: i32, cy: i32, earned: bool) {
    let point = |k: i32, r: f32| {
        let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
        (cx as f32 + a.cos() * r, cy as f32 + a.sin() * r)
    };
    let shape: Vec<(f32, f32)> = (0..10)
        .map(|k| point(k, if k % 2 == 0 { 12.0 } else { 5.0 }))
        .collect();
    if !earned {
        let mut fill = Canvas::new(c.width(), c.height());
        paint::polygon(&mut fill, &shape, PENCIL_SMUDGE);
        paint::blit(c, &fill, 0, 0);
        paint::ring(c, &fill, 0, 0, PENCIL);
        return;
    }
    let mut fill = Canvas::new(c.width(), c.height());
    paint::fill_polygon(&shape, |x, y| {
        let lit = (x - cx) + (y - cy) < -2;
        fill.set(x, y, if lit { BRASS.light } else { BRASS.base });
    });
    let shadowed = fill.clone();
    for y in 0..c.height() as i32 {
        for x in 0..c.width() as i32 {
            if shadowed.get(x, y).a > 0 {
                paint::put(c, x + 1, y + 1, SHADOW);
            }
        }
    }
    paint::ring(c, &fill, 0, 0, BRASS.edge);
    paint::blit(c, &fill, 0, 0);
    paint::put(c, cx - 1, cy - 3, BRASS.shine);
    paint::put(c, cx - 2, cy - 1, BRASS.shine);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_keepsake_is_its_own_picture() {
        let all = [
            Keepsake::Points,
            Keepsake::Played,
            Keepsake::Cup(0),
            Keepsake::Trophy(0),
            Keepsake::Unlocked,
            Keepsake::Journal,
            Keepsake::Notebook,
            Keepsake::Trio,
            Keepsake::Note,
        ];
        let pictures: Vec<Canvas> = all.iter().map(|k| icon(*k)).collect();
        for i in 0..all.len() {
            assert!(
                (0..ICON_H as i32).any(|y| (0..ICON_W as i32).any(|x| pictures[i].get(x, y).a > 0)),
                "{:?} is drawn",
                all[i]
            );
            for j in i + 1..all.len() {
                assert_ne!(pictures[i], pictures[j], "{:?} and {:?}", all[i], all[j]);
            }
        }
    }

    #[test]
    fn a_cups_rosette_is_in_its_colour_and_pencilled_until_won() {
        assert_ne!(small_rosette(0, true), small_rosette(1, true));
        assert_ne!(small_rosette(0, true), small_rosette(0, false));
        assert_eq!(small_rosette(2, true).get(5, 5).a, 255);
        assert!(small_rosette(2, false).get(5, 5).a < 255);
    }

    #[test]
    fn the_studs_count_the_turns_and_light_the_one_being_given() {
        let early = studs(2, 12, false);
        assert_eq!(early.width(), (12 * STUD + 1) as u32);
        assert_ne!(early, studs(3, 12, false));
        assert_ne!(studs(3, 12, true), studs(3, 12, false));
        // The current stud is brighter than one played and one to come.
        let at = |k: i32| early.get(k * STUD + 3, 4);
        assert_eq!(at(1), BRASS.shine);
        assert_ne!(at(0), BRASS.shine);
        assert_ne!(at(5), BRASS.shine);
    }

    #[test]
    fn a_stamp_says_its_word_inside_its_rule_and_leans() {
        let stamp = stamp("YOU WIN!", 2);
        let inked = |x: i32, y: i32| stamp.get(x, y).a > 0;
        let (w, h) = (stamp.width() as i32, stamp.height() as i32);
        // Askew: the left end sits lower than the right.
        let lowest = |x: i32| (0..h).rev().find(|y| inked(x, *y)).unwrap_or(0);
        assert!(lowest(1) > lowest(w - 2));
        assert_ne!(stamp, super::stamp("A DRAW", 2));
    }

    #[test]
    fn each_way_a_match_ends_has_its_own_rosette() {
        let all = [
            Award::Won(Some(0)),
            Award::Won(Some(1)),
            Award::Lost,
            Award::Drawn,
            Award::Learned,
            Award::NotYet,
        ];
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                assert_ne!(
                    award(all[i]),
                    award(all[j]),
                    "{:?} and {:?}",
                    all[i],
                    all[j]
                );
            }
        }
        assert_ne!(padlock().get(5, 9).a, 0);
        let sign = signboard(80);
        let (x, y, w, h) = sign_board(80);
        assert_eq!(
            sign.get(x + w / 2, y + h / 2).a,
            255,
            "the name has its plank"
        );
    }
}
