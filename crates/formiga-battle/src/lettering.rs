//! Cartoon words in Formiga's pixel art: "POW!", "CLANG!", "READY?", "-1". Each is drawn from a
//! small bold pixel alphabet, at a whole number of board pixels to each of its own, coloured by
//! the art's rules (lighter along the top, as the light comes from above; edged in a darker shade
//! of its own colour) with a hard shadow under it, and laid on the board's own canvas, so it is as
//! crisp as everything else there.

use formiga_art::paint::{self, rgb};
use formiga_art::{Canvas, Rgba};

/// How tall a letter is, in its own pixels.
const HEIGHT: i32 = 7;

/// A letter's pixels, a row to a string, `#` drawn. Anything not here is drawn as a space.
fn glyph(letter: char) -> &'static [&'static str] {
    match letter {
        'A' => &[
            ".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'B' => &[
            "####.", "#...#", "#...#", "####.", "#...#", "#...#", "####.",
        ],
        'C' => &[
            ".####", "#....", "#....", "#....", "#....", "#....", ".####",
        ],
        'D' => &[
            "####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####.",
        ],
        'E' => &[
            "#####", "#....", "#....", "####.", "#....", "#....", "#####",
        ],
        'F' => &[
            "#####", "#....", "#....", "####.", "#....", "#....", "#....",
        ],
        'G' => &[
            ".####", "#....", "#....", "#..##", "#...#", "#...#", ".####",
        ],
        'H' => &[
            "#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'I' => &["###", ".#.", ".#.", ".#.", ".#.", ".#.", "###"],
        'K' => &[
            "#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#",
        ],
        'L' => &[
            "#....", "#....", "#....", "#....", "#....", "#....", "#####",
        ],
        'M' => &[
            "#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#",
        ],
        'N' => &[
            "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#",
        ],
        'O' => &[
            ".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'P' => &[
            "####.", "#...#", "#...#", "####.", "#....", "#....", "#....",
        ],
        'Q' => &[
            ".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#",
        ],
        'R' => &[
            "####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#",
        ],
        'S' => &[
            ".####", "#....", "#....", ".###.", "....#", "....#", "####.",
        ],
        'T' => &[
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#..",
        ],
        'U' => &[
            "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'V' => &[
            "#...#", "#...#", "#...#", "#...#", ".#.#.", ".#.#.", "..#..",
        ],
        'W' => &[
            "#...#", "#...#", "#...#", "#.#.#", "#.#.#", "##.##", "#...#",
        ],
        'Y' => &[
            "#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..",
        ],
        'Z' => &[
            "#####", "....#", "...#.", "..#..", ".#...", "#....", "#####",
        ],
        '!' => &["#", "#", "#", "#", "#", ".", "#"],
        '?' => &[
            ".###.", "#...#", "....#", "..##.", "..#..", ".....", "..#..",
        ],
        '-' => &["...", "...", "...", "###", "...", "...", "..."],
        '.' => &[".", ".", ".", ".", ".", ".", "#"],
        '1' => &[".#.", "##.", ".#.", ".#.", ".#.", ".#.", "###"],
        '2' => &[
            ".###.", "#...#", "....#", "..##.", ".#...", "#....", "#####",
        ],
        '3' => &[
            "####.", "....#", "....#", ".###.", "....#", "....#", "####.",
        ],
        _ => &["..", "..", "..", "..", "..", "..", ".."],
    }
}

/// The colours a word is painted in: its face, lighter along the top, and its edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Paint {
    pub top: Rgba,
    pub face: Rgba,
    pub edge: Rgba,
}

impl Paint {
    /// Cream with a gold foot, for news that is nobody's in particular.
    pub const CREAM: Self = Self {
        top: rgb(0xfffbe6),
        face: rgb(0xffe9a0),
        edge: rgb(0x6e4f1c),
    };
    /// Gold, for a big moment.
    pub const GOLD: Self = Self {
        top: rgb(0xfff0a0),
        face: rgb(0xf5c542),
        edge: rgb(0x6e4f1c),
    };
    /// A warm red, for composure lost.
    pub const HURT: Self = Self {
        top: rgb(0xffb49c),
        face: rgb(0xd9573f),
        edge: rgb(0x5a1e14),
    };

    /// A team's own colours.
    pub fn team(ramp: paint::Ramp) -> Self {
        Self {
            top: ramp.shine,
            face: ramp.light,
            edge: ramp.edge,
        }
    }
}

/// The word's letters as a mask, one pixel to each of the alphabet's, each stroke doubled
/// sideways so the letters are bold.
fn mask(text: &str) -> (i32, Vec<Vec<bool>>) {
    let mut rows = vec![Vec::new(); HEIGHT as usize];
    for (k, letter) in text.chars().enumerate() {
        if k > 0 {
            for row in &mut rows {
                row.push(false);
            }
        }
        let glyph = glyph(letter.to_ascii_uppercase());
        let width = glyph[0].len();
        for (row, line) in rows.iter_mut().zip(glyph) {
            let start = row.len();
            row.extend(line.chars().map(|c| c == '#'));
            // Bold: each pixel also fills the one to its right.
            row.push(false);
            for x in (start..start + width).rev() {
                if row[x] {
                    row[x + 1] = true;
                }
            }
        }
    }
    let width = rows.first().map_or(0, Vec::len) as i32;
    (width, rows)
}

/// A word as a picture, `scale` board pixels to each of its own, with its edge and its shadow.
pub fn word(text: &str, scale: i32, colours: Paint) -> Canvas {
    let scale = scale.max(1);
    let (width, rows) = mask(text);
    // Room for the edge all round and the shadow below and to the right.
    let pad = 1;
    let shadow = scale.max(2) / 2 + 1;
    let w = width * scale + pad * 2 + shadow;
    let h = HEIGHT * scale + pad * 2 + shadow;
    let mut face = Canvas::new(w.max(1) as u32, h.max(1) as u32);
    for (y, row) in rows.iter().enumerate() {
        for (x, on) in row.iter().enumerate() {
            if !on {
                continue;
            }
            for dy in 0..scale {
                for dx in 0..scale {
                    let py = y as i32 * scale + dy;
                    // Lighter in the top rows, as the light falls from above.
                    let colour = if (py as f32) < HEIGHT as f32 * scale as f32 * 0.38 {
                        colours.top
                    } else {
                        colours.face
                    };
                    face.set(pad + x as i32 * scale + dx, pad + py, colour);
                }
            }
        }
    }
    let mut out = Canvas::new(face.width(), face.height());
    // The hard shadow first, then the edge round the letters, then the letters.
    let dark = paint::faded(rgb(0x1e1410), 120);
    for y in 0..face.height() as i32 {
        for x in 0..face.width() as i32 {
            if face.get(x, y).a > 0 {
                for d in 1..=shadow {
                    paint::put(&mut out, x + d / 2 + 1, y + d, dark);
                }
            }
        }
    }
    paint::ring(&mut out, &face, 0, 0, colours.edge);
    paint::blit(&mut out, &face, 0, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_word_the_board_says_has_its_letters() {
        for text in [
            "POW!",
            "BONK!",
            "WHAP!",
            "BIFF!",
            "CLANG!",
            "TONK!",
            "DONG!",
            "WHAM!",
            "SHOVE!",
            "BOOM!",
            "KER-FLOP!",
            "BWOMP!",
            "MISS!",
            "WHIFF!",
            "BOINK",
            "BOING",
            "CLUNK!",
            "SHOO!",
            "CREEEAK",
            "THUNK",
            "FWOOSH!",
            "HMM?",
            "READY?",
            "GO!",
            "-1",
            "-2",
            "-3",
            "DRIP...",
            "SQUEAK!",
            "SPLOOSH!",
            "TING!",
            "POP!",
            "GOT IT!",
            "PLOP",
            "WHOOSH!",
            "WEDGE!",
            "STUCK!",
            "FWIP!",
            "FIZZLE",
            "SWASH...",
            "TOOT!",
            "KER-SPLASH!",
            "GLUG...",
            "SPLASH!",
            "RUSTLE...",
            "SHAKE!",
            "CLONK!",
            "GLUB",
            "BOB!",
            "DING!",
            "SPLAT!",
            "BOING!",
            "HOORAY!",
            "SO CLOSE!",
            "A TIE!",
        ] {
            for letter in text.chars().filter(|c| *c != ' ') {
                assert!(
                    glyph(letter).iter().any(|row| row.contains('#')),
                    "{letter:?} in {text}"
                );
            }
        }
    }

    #[test]
    fn every_letter_is_as_tall_as_the_rest_and_square_in_its_rows() {
        for letter in "ABCDEFGHIKLMNOPQRSTUVWYZ!?-.123".chars() {
            let rows = glyph(letter);
            assert_eq!(rows.len(), HEIGHT as usize, "{letter:?}");
            assert!(rows.iter().all(|r| r.len() == rows[0].len()), "{letter:?}");
        }
    }

    #[test]
    fn a_word_grows_with_its_scale_and_is_edged_in_its_own_dark() {
        let small = word("POW!", 1, Paint::CREAM);
        let big = word("POW!", 3, Paint::CREAM);
        assert!(big.width() > small.width() * 2);
        let mut edged = false;
        for y in 0..big.height() as i32 {
            for x in 0..big.width() as i32 {
                edged |= big.get(x, y) == Paint::CREAM.edge;
            }
        }
        assert!(edged);
    }
}
