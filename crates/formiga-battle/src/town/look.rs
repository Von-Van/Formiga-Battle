//! How the Handler looks in town, chosen in the Handler Studio and kept with the colony's career
//! (`career.rs`): build, skin, hairstyle and hair colour, outfit and its colour, and one
//! accessory. Purely how they look: nothing here touches a match, the record or the companions.
//!
//! A colour choice is a lookup, never a redraw: each recolourable material is painted in shades
//! of one hue, and choosing a colour swaps those shades for the same steps of the new one. Until
//! the layered sheets are drawn the Handler is the stand-in sheet, on which hair, jacket and skin
//! can already be recoloured; the other choices are kept, and drawn once their layers exist.

use super::sheet::{CELL, Sheet};
use formiga_art::{Canvas, Rgba};
use serde::{Deserialize, Serialize};

/// A choice's name and the colour it shows, for colour choices.
pub type Swatch = (&'static str, u32);

pub const HAIR_COLOURS: [Swatch; 5] = [
    ("Brown", 0x6b3c37),
    ("Black", 0x2a1e22),
    ("Auburn", 0x923c22),
    ("Blonde", 0xc9a040),
    ("Silver", 0x8c8892),
];
pub const OUTFIT_COLOURS: [Swatch; 5] = [
    ("Navy", 0x4c4262),
    ("Forest", 0x285a4b),
    ("Maroon", 0x7a2c33),
    ("Mustard", 0xa47a2c),
    ("Slate", 0x4a6288),
];
pub const SKIN_TONES: [Swatch; 6] = [
    ("Fair", 0xf4d9c0),
    ("Light", 0xedaa76),
    ("Warm", 0xc7936b),
    ("Tan", 0xa86b4a),
    ("Brown", 0x7a4a32),
    ("Deep", 0x5a3424),
];
pub const BUILDS: [&str; 2] = ["Build A", "Build B"];
pub const HAIRSTYLES: [&str; 6] = ["Tousled", "Short crop", "Long", "Bun", "Curls", "Braids"];
pub const OUTFITS: [&str; 6] = [
    "Field jacket",
    "Lab coat",
    "Jumper",
    "Overalls",
    "Club jersey",
    "Waistcoat",
];
pub const ACCESSORIES: [&str; 6] = [
    "Satchel",
    "Glasses",
    "Scarf",
    "Field cap",
    "Magnifier",
    "None",
];

/// The stand-in sheet's own shades of each recolourable material, darkest first; the last is
/// the one a chosen colour is matched to.
const STAND_IN_JACKET: [u32; 4] = [0x2f1f30, 0x382a3d, 0x453850, 0x4c4262];
const STAND_IN_HAIR: [u32; 2] = [0x6b3c37, 0x703f3a];
/// The hair's highlight shares its colour with the satchel, so it counts as hair only on the
/// head, above this row of a cell.
const STAND_IN_HAIR_LIGHT: (u32, i32) = (0xae7154, 17);
const STAND_IN_SKIN: [u32; 2] = [0xc7936b, 0xedaa76];

/// The Handler's look: each choice an index into its list above.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HandlerLook {
    pub build: u8,
    pub skin: u8,
    pub hairstyle: u8,
    pub hair: u8,
    pub outfit: u8,
    pub outfit_colour: u8,
    pub accessory: u8,
}

impl Default for HandlerLook {
    /// Jake's first sheet: build A, light skin, tousled brown hair, a navy field jacket and the
    /// satchel.
    fn default() -> Self {
        Self {
            build: 0,
            skin: 1,
            hairstyle: 0,
            hair: 0,
            outfit: 0,
            outfit_colour: 0,
            accessory: 0,
        }
    }
}

impl HandlerLook {
    /// The same look with any choice past the end of its list put back to the usual one, as a
    /// file from a newer Battle or a hand-edited one might hold.
    pub fn sane(self) -> Self {
        let usual = Self::default();
        let keep = |v: u8, len: usize, usual: u8| if (v as usize) < len { v } else { usual };
        Self {
            build: keep(self.build, BUILDS.len(), usual.build),
            skin: keep(self.skin, SKIN_TONES.len(), usual.skin),
            hairstyle: keep(self.hairstyle, HAIRSTYLES.len(), usual.hairstyle),
            hair: keep(self.hair, HAIR_COLOURS.len(), usual.hair),
            outfit: keep(self.outfit, OUTFITS.len(), usual.outfit),
            outfit_colour: keep(
                self.outfit_colour,
                OUTFIT_COLOURS.len(),
                usual.outfit_colour,
            ),
            accessory: keep(self.accessory, ACCESSORIES.len(), usual.accessory),
        }
    }

    /// The Handler's sheet in this look.
    pub fn sheet(&self) -> Sheet {
        let look = self.sane();
        let usual = Self::default();
        let mut sheet = Sheet::stand_in();
        if look.outfit_colour != usual.outfit_colour {
            let to = OUTFIT_COLOURS[look.outfit_colour as usize].1;
            sheet.recolour(&STAND_IN_JACKET, to, |_| true);
        }
        if look.hair != usual.hair {
            let to = HAIR_COLOURS[look.hair as usize].1;
            sheet.recolour(&STAND_IN_HAIR, to, |_| true);
            let (light, below) = STAND_IN_HAIR_LIGHT;
            let ramp = [light];
            // The highlight follows the hair, a step lighter than its chosen colour.
            sheet.recolour(&ramp, lighter(to), |y| y % CELL < below);
        }
        if look.skin != usual.skin {
            let to = SKIN_TONES[look.skin as usize].1;
            sheet.recolour(&STAND_IN_SKIN, to, |_| true);
        }
        sheet
    }

    /// A few words for the look, as the Studio sums it up.
    pub fn summary(&self) -> String {
        let look = self.sane();
        format!(
            "{} {} hair, a {} {}, {}",
            HAIR_COLOURS[look.hair as usize].0,
            HAIRSTYLES[look.hairstyle as usize].to_lowercase(),
            OUTFIT_COLOURS[look.outfit_colour as usize].0.to_lowercase(),
            OUTFITS[look.outfit as usize].to_lowercase(),
            match ACCESSORIES[look.accessory as usize] {
                "None" => "nothing carried".to_owned(),
                a => format!("the {}", a.to_lowercase()),
            }
        )
    }
}

/// A head-and-shoulders portrait from the sheet's first frame facing down, twice its size: the
/// stand-in until the portraits are drawn.
pub fn portrait(sheet: &Sheet) -> Canvas {
    let frame = sheet.frame(super::map::Facing::Down, 4);
    let (top, size) = (2, 32);
    let left = (CELL - size) / 2;
    let mut out = Canvas::new((size * 2) as u32, (size * 2) as u32);
    for y in 0..size * 2 {
        for x in 0..size * 2 {
            out.set(x, y, frame.get(left + x / 2, top + y / 2));
        }
    }
    out
}

fn rgb(hex: u32) -> Rgba {
    Rgba::new((hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255)
}

/// A colour's hue, saturation and value, each 0 to 1.
fn hsv(c: Rgba) -> (f32, f32, f32) {
    let (r, g, b) = (
        f32::from(c.r) / 255.0,
        f32::from(c.g) / 255.0,
        f32::from(c.b) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    let s = if max == 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

fn from_hsv(h: f32, s: f32, v: f32, a: u8) -> Rgba {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - f * s), v * (1.0 - (1.0 - f) * s));
    let (r, g, b) = match (i as i32).rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    let byte = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    Rgba::new(byte(r), byte(g), byte(b), a)
}

/// A step lighter: the value raised by a fifth of the way to white.
fn lighter(hex: u32) -> u32 {
    let (h, s, v) = hsv(rgb(hex));
    let c = from_hsv(h, s * 0.85, v + (1.0 - v) * 0.35, 255);
    (u32::from(c.r) << 16) | (u32::from(c.g) << 8) | u32::from(c.b)
}

impl Sheet {
    /// Swaps the shades of `ramp` (darkest first, the last matched to the chosen colour) for the
    /// same steps of `to`: each keeps how much darker or paler it was than the ramp's last, in
    /// the new colour's hue. `rows` says which pixel rows of the sheet it applies to.
    pub fn recolour(&mut self, ramp: &[u32], to: u32, rows: impl Fn(i32) -> bool) {
        let Some(&base) = ramp.last() else {
            return;
        };
        let (_, base_s, base_v) = hsv(rgb(base));
        let (to_h, to_s, to_v) = hsv(rgb(to));
        let swaps: Vec<(Rgba, Rgba)> = ramp
            .iter()
            .map(|&from| {
                let (_, s, v) = hsv(rgb(from));
                let s_ratio = if base_s > 0.0 { s / base_s } else { 1.0 };
                let v_ratio = if base_v > 0.0 { v / base_v } else { 1.0 };
                (
                    rgb(from),
                    from_hsv(to_h, to_s * s_ratio, to_v * v_ratio, 255),
                )
            })
            .collect();
        self.map_pixels(|y, p| {
            if p.a == 0 || !rows(y) {
                return p;
            }
            let opaque = Rgba::new(p.r, p.g, p.b, 255);
            match swaps.iter().find(|(from, _)| *from == opaque) {
                Some((_, to)) => Rgba::new(to.r, to.g, to.b, p.a),
                None => p,
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::town::map::Facing;

    #[test]
    fn the_usual_look_is_the_stand_in_sheet_exactly() {
        let dressed = HandlerLook::default().sheet();
        let plain = Sheet::stand_in();
        for facing in Facing::ALL {
            assert_eq!(dressed.frame(facing, 0), plain.frame(facing, 0));
        }
    }

    #[test]
    fn a_new_jacket_colour_changes_the_jacket_and_nothing_else() {
        let look = HandlerLook {
            outfit_colour: 1,
            ..HandlerLook::default()
        };
        let (dressed, plain) = (look.sheet(), Sheet::stand_in());
        let a = dressed.frame(Facing::Down, 0);
        let b = plain.frame(Facing::Down, 0);
        let jacket: Vec<Rgba> = STAND_IN_JACKET.iter().map(|&h| rgb(h)).collect();
        let mut changed = 0;
        for y in 0..CELL {
            for x in 0..CELL {
                if a.get(x, y) != b.get(x, y) {
                    changed += 1;
                    assert!(
                        jacket.contains(&b.get(x, y)),
                        "({x}, {y}) is not the jacket"
                    );
                }
            }
        }
        assert!(changed > 20, "only {changed} pixels changed");
    }

    #[test]
    fn a_choice_past_the_end_of_its_list_goes_back_to_the_usual() {
        let odd = HandlerLook {
            hair: 99,
            accessory: 7,
            ..HandlerLook::default()
        };
        assert_eq!(odd.sane(), HandlerLook::default());
    }

    #[test]
    fn a_look_reads_from_a_file_missing_its_newer_choices() {
        let look: HandlerLook = serde_json::from_str(r#"{"hair": 2}"#).unwrap();
        assert_eq!(look.hair, 2);
        assert_eq!(look.skin, HandlerLook::default().skin);
    }

    #[test]
    fn the_summary_names_every_choice() {
        let words = HandlerLook::default().summary();
        assert_eq!(
            words,
            "Brown tousled hair, a navy field jacket, the satchel"
        );
    }
}
