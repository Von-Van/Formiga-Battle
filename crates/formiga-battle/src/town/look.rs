//! How the Handler looks in town, chosen in the Handler Studio and kept with the colony's career
//! (`career.rs`): build, skin, hairstyle and hair colour, outfit and its colour, trousers, shoes,
//! and one accessory and its colour. Purely how they look: nothing here touches a match, the record or the companions.
//!
//! The Handler is drawn from Jake's layered sheets, stacked in his draw order: hair behind, the
//! accessory behind, body, outfit, the accessory in front, hair in front, then glasses or a cap.
//! A colour is a lookup, never a redraw: each recolourable material is painted in four shades of
//! one hue (the ramps below, from the art pack's character layout), and a chosen colour swaps
//! them for the same steps of the new one. "As drawn" keeps the art's own colour. The Handler
//! has no portrait: the Studio shows them walking (Jake, 12 November).

use super::art;
use super::sheet::{CELL, Sheet, decode};
use formiga_art::{Canvas, Rgba};
use serde::{Deserialize, Serialize};

/// A choice's name and the colour it shows; for colour choices the first is "As drawn", whose
/// colour is the art's own and changes with the hairstyle or outfit.
pub type Swatch = (&'static str, u32);

pub const HAIR_COLOURS: [Swatch; 8] = [
    ("As drawn", 0),
    ("Brown", 0x6b3c37),
    ("Black", 0x2a1e22),
    ("Auburn", 0x923c22),
    ("Blonde", 0xc9a040),
    ("Silver", 0x8c8892),
    ("Ginger", 0xb5562a),
    ("Ash", 0x7a7466),
];
pub const OUTFIT_COLOURS: [Swatch; 8] = [
    ("As drawn", 0),
    ("Navy", 0x4c4262),
    ("Forest", 0x285a4b),
    ("Maroon", 0x7a2c33),
    ("Mustard", 0xa47a2c),
    ("Slate", 0x4a6288),
    ("Plum", 0x5e3a62),
    ("Teal", 0x2f6a6e),
];
pub const TROUSER_COLOURS: [Swatch; 6] = [
    ("As drawn", 0x676273),
    ("Charcoal", 0x45424a),
    ("Denim", 0x4f6a8c),
    ("Khaki", 0x9a8a5c),
    ("Olive", 0x5f6b3c),
    ("Brown", 0x6e5040),
];
pub const SHOE_COLOURS: [Swatch; 6] = [
    ("As drawn", 0x805842),
    ("Black", 0x3a3036),
    ("Tan", 0xb07a48),
    ("Oxblood", 0x7a2e32),
    ("Grey", 0x7a7880),
    ("Green", 0x4e6a46),
];
/// The satchel's, scarf's or field cap's colour; glasses and the magnifier keep their own.
pub const ACCESSORY_COLOURS: [Swatch; 7] = [
    ("As drawn", 0),
    ("Russet", 0xa0522d),
    ("Navy", 0x4c4262),
    ("Forest", 0x285a4b),
    ("Crimson", 0x9a2c3a),
    ("Mustard", 0xa47a2c),
    ("Plum", 0x6a3a6a),
];
pub const SKIN_TONES: [Swatch; 6] = [
    ("As drawn", 0xdc986b),
    ("Fair", 0xf4d9c0),
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

/// Each material's four shades as the art draws them, darkest first.
type Ramp = [u32; 4];
const SKIN: Ramp = [0x96583f, 0xbb7752, 0xdc986b, 0xf0b883];
const HAIR: [Ramp; 6] = [
    [0x553038, 0x70423d, 0x965d4b, 0xb97c60],
    [0x3c3545, 0x51495d, 0x6b6077, 0x897e95],
    [0x632f34, 0x8b4541, 0xa95a49, 0xc77d56],
    [0x543553, 0x714466, 0x905579, 0xb77292],
    [0x765130, 0x9c703a, 0xbf934c, 0xe0bb68],
    [0x464254, 0x656078, 0x817c91, 0xaaa4b4],
];
const OUTFIT: [Ramp; 6] = [
    [0x403950, 0x534a68, 0x675d82, 0x8a7fa1],
    [0x777184, 0xa39bac, 0xc9c0d0, 0xe7dfeb],
    [0x394638, 0x4e6149, 0x657b56, 0x8b9d6f],
    [0x35434c, 0x475c66, 0x5e7880, 0x81a098],
    [0x633947, 0x884d62, 0xae6580, 0xd98ea2],
    [0x584335, 0x7d5b43, 0xa17850, 0xc99e66],
];
const TROUSERS: Ramp = [0x393344, 0x50495b, 0x676273, 0x888391];
const SHOES: Ramp = [0x3c3032, 0x614437, 0x805842, 0xa47250];
/// The satchel's leather. The scarf and the cap are drawn in three shades, so their darkest
/// stands twice to keep the matched shade third.
const SATCHEL: Ramp = [0x6e5439, 0xa68050, 0xc19c64, 0xd9b781];
const SCARF: Ramp = [0x804455, 0x804455, 0xaa5563, 0xc98b85];
const FIELD_CAP: Ramp = [0x505e4d, 0x505e4d, 0x647b58, 0x9ca37e];
/// The shade a chosen colour is matched to: the third, the material's middle light.
const MATCHED: usize = 2;

/// The Handler's look: each choice an index into its list above.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HandlerLook {
    pub build: u8,
    pub skin: u8,
    pub hairstyle: u8,
    pub hair: u8,
    pub outfit: u8,
    pub outfit_colour: u8,
    pub accessory: u8,
    pub trousers: u8,
    pub shoes: u8,
    pub accessory_colour: u8,
}

/// The file names used for each choice.
fn slug(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

impl HandlerLook {
    /// The same look with any choice past the end of its list put back to the usual one, as a
    /// file from a newer Battle or a hand-edited one might hold.
    pub fn sane(self) -> Self {
        let keep = |v: u8, len: usize| if (v as usize) < len { v } else { 0 };
        Self {
            build: keep(self.build, BUILDS.len()),
            skin: keep(self.skin, SKIN_TONES.len()),
            hairstyle: keep(self.hairstyle, HAIRSTYLES.len()),
            hair: keep(self.hair, HAIR_COLOURS.len()),
            outfit: keep(self.outfit, OUTFITS.len()),
            outfit_colour: keep(self.outfit_colour, OUTFIT_COLOURS.len()),
            accessory: keep(self.accessory, ACCESSORIES.len()),
            trousers: keep(self.trousers, TROUSER_COLOURS.len()),
            shoes: keep(self.shoes, SHOE_COLOURS.len()),
            accessory_colour: keep(self.accessory_colour, ACCESSORY_COLOURS.len()),
        }
    }

    /// The colour "As drawn" stands for now, for hair and outfit: the art's own.
    pub fn own_colours(&self) -> (u32, u32) {
        let look = self.sane();
        (
            HAIR[look.hairstyle as usize][MATCHED],
            OUTFIT[look.outfit as usize][MATCHED],
        )
    }

    /// The colours of the accessory chosen, if it takes another: its own colour is what
    /// "As drawn" stands for.
    fn accessory_ramp(&self) -> Option<&'static Ramp> {
        match ACCESSORIES[self.sane().accessory as usize] {
            "Satchel" => Some(&SATCHEL),
            "Scarf" => Some(&SCARF),
            "Field cap" => Some(&FIELD_CAP),
            _ => None,
        }
    }

    /// The colour "As drawn" stands for the accessory chosen, or `None` when it keeps its own
    /// colour whatever is chosen (glasses, the magnifier, nothing).
    pub fn own_accessory_colour(&self) -> Option<u32> {
        self.accessory_ramp().map(|ramp| ramp[MATCHED])
    }

    /// The overworld sheets in draw order, by file name.
    fn layers(&self) -> Vec<String> {
        let look = self.sane();
        let build = if look.build == 0 { "a" } else { "b" };
        let hair = slug(HAIRSTYLES[look.hairstyle as usize]);
        let outfit = slug(OUTFITS[look.outfit as usize]);
        let accessory = slug(ACCESSORIES[look.accessory as usize]);
        let (back, front, top): (Option<String>, Option<String>, Option<String>) = match accessory
            .as_str()
        {
            "satchel" | "scarf" => (
                Some(format!("handler-acc-{accessory}-back.png")),
                Some(format!("handler-acc-{accessory}-front.png")),
                None,
            ),
            "magnifier" => (None, Some("handler-acc-magnifier.png".to_owned()), None),
            "glasses" | "field-cap" => (None, None, Some(format!("handler-acc-{accessory}.png"))),
            _ => (None, None, None),
        };
        [
            Some(format!("handler-hair-{hair}-back.png")),
            back,
            Some(format!("handler-body-{build}.png")),
            Some(format!("handler-outfit-{outfit}-{build}.png")),
            front,
            Some(format!("handler-hair-{hair}-front.png")),
            top,
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// Swaps the art's own colours for the chosen ones, all at once, so a colour one material
    /// is dyed to is never dyed again as another's.
    fn dye(&self, canvas: &mut Canvas) {
        let look = self.sane();
        let mut swaps = Vec::new();
        let mut dye = |ramp: &Ramp, list: &[Swatch], chosen: u8| {
            if chosen != 0 {
                swaps.extend(shades(ramp, list[chosen as usize].1));
            }
        };
        dye(&HAIR[look.hairstyle as usize], &HAIR_COLOURS, look.hair);
        dye(
            &OUTFIT[look.outfit as usize],
            &OUTFIT_COLOURS,
            look.outfit_colour,
        );
        dye(&SKIN, &SKIN_TONES, look.skin);
        dye(&TROUSERS, &TROUSER_COLOURS, look.trousers);
        dye(&SHOES, &SHOE_COLOURS, look.shoes);
        if let Some(ramp) = look.accessory_ramp() {
            dye(ramp, &ACCESSORY_COLOURS, look.accessory_colour);
        }
        swap(canvas, &swaps);
    }

    /// The Handler's overworld sheet in this look.
    pub fn sheet(&self) -> Sheet {
        let mut canvas = Canvas::new(8 * CELL as u32, 4 * CELL as u32);
        for name in self.layers() {
            if let Some(layer) = art::bytes(&name).and_then(decode) {
                stack(&mut canvas, &layer, 0, 0);
            }
        }
        self.dye(&mut canvas);
        Sheet::from_canvas(canvas, 8)
    }

    /// A few words for the look, as the Studio sums it up.
    pub fn summary(&self) -> String {
        let look = self.sane();
        let hair = match look.hair {
            0 => String::new(),
            i => format!("{} ", HAIR_COLOURS[i as usize].0.to_lowercase()),
        };
        let outfit = match look.outfit_colour {
            0 => String::new(),
            i => format!("{} ", OUTFIT_COLOURS[i as usize].0.to_lowercase()),
        };
        let words = format!(
            "{hair}{} hair, a {outfit}{}, {}",
            HAIRSTYLES[look.hairstyle as usize].to_lowercase(),
            OUTFITS[look.outfit as usize].to_lowercase(),
            match ACCESSORIES[look.accessory as usize] {
                "None" => "nothing carried".to_owned(),
                a if look.accessory_ramp().is_some() && look.accessory_colour != 0 => format!(
                    "the {} {}",
                    ACCESSORY_COLOURS[look.accessory_colour as usize]
                        .0
                        .to_lowercase(),
                    a.to_lowercase()
                ),
                a => format!("the {}", a.to_lowercase()),
            }
        );
        let mut chars = words.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => words,
        }
    }
}

/// `layer` laid over `canvas`, its top-left corner at `(x, y)`; the art's alpha is all or nothing.
fn stack(canvas: &mut Canvas, layer: &Canvas, x: i32, y: i32) {
    for ly in 0..layer.height() as i32 {
        for lx in 0..layer.width() as i32 {
            let p = layer.get(lx, ly);
            if p.a > 0 {
                canvas.set(x + lx, y + ly, p);
            }
        }
    }
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

fn from_hsv(h: f32, s: f32, v: f32) -> Rgba {
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
    Rgba::new(byte(r), byte(g), byte(b), 255)
}

/// A material's four shades, each beside the same step of `to`: each keeps how much darker or
/// paler it was than the matched shade, in the new colour's hue.
fn shades(ramp: &Ramp, to: u32) -> Vec<(Rgba, Rgba)> {
    let (_, base_s, base_v) = hsv(rgb(ramp[MATCHED]));
    let (to_h, to_s, to_v) = hsv(rgb(to));
    ramp.iter()
        .map(|&from| {
            let (_, s, v) = hsv(rgb(from));
            let s_ratio = if base_s > 0.0 { s / base_s } else { 1.0 };
            let v_ratio = if base_v > 0.0 { v / base_v } else { 1.0 };
            (rgb(from), from_hsv(to_h, to_s * s_ratio, to_v * v_ratio))
        })
        .collect()
}

/// Each pixel of a `from` colour painted its `to`, in one pass.
fn swap(canvas: &mut Canvas, swaps: &[(Rgba, Rgba)]) {
    if swaps.is_empty() {
        return;
    }
    for y in 0..canvas.height() as i32 {
        for x in 0..canvas.width() as i32 {
            let p = canvas.get(x, y);
            if p.a == 0 {
                continue;
            }
            let opaque = Rgba::new(p.r, p.g, p.b, 255);
            if let Some((_, to)) = swaps.iter().find(|(from, _)| *from == opaque) {
                canvas.set(x, y, Rgba::new(to.r, to.g, to.b, p.a));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::town::map::Facing;

    #[test]
    fn every_choice_has_its_pictures() {
        for build in 0..BUILDS.len() as u8 {
            for hairstyle in 0..HAIRSTYLES.len() as u8 {
                for outfit in 0..OUTFITS.len() as u8 {
                    for accessory in 0..ACCESSORIES.len() as u8 {
                        let look = HandlerLook {
                            build,
                            hairstyle,
                            outfit,
                            accessory,
                            ..HandlerLook::default()
                        };
                        for name in look.layers() {
                            assert!(art::bytes(&name).is_some(), "no {name}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_usual_look_stacks_jakes_default_and_keeps_its_colours() {
        let sheet = HandlerLook::default().sheet();
        let frame = sheet.frame(Facing::Down, 4);
        let (_, _, _, bottom) = frame.alpha_bounds().expect("drawn");
        assert_eq!(bottom as i32, crate::town::sheet::FEET);
        let hair = rgb(HAIR[0][MATCHED]);
        assert!(frame.pixels().contains(&hair), "the tousled hair as drawn");
    }

    #[test]
    fn a_new_jacket_colour_changes_the_jacket_and_nothing_else() {
        let look = HandlerLook {
            outfit_colour: 2,
            ..HandlerLook::default()
        };
        let a = look.sheet().frame(Facing::Down, 4);
        let b = HandlerLook::default().sheet().frame(Facing::Down, 4);
        let jacket: Vec<Rgba> = OUTFIT[0].iter().map(|&h| rgb(h)).collect();
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

    /// Dyeing one material changes only that material's pixels, somewhere in the sheet.
    fn only_dyes(look: HandlerLook, ramp: &Ramp) {
        let a = look.sheet();
        let mut plain = look;
        (plain.trousers, plain.shoes, plain.accessory_colour) = (0, 0, 0);
        let b = plain.sheet();
        let material: Vec<Rgba> = ramp.iter().map(|&h| rgb(h)).collect();
        let mut changed = 0;
        for facing in Facing::ALL {
            let (fa, fb) = (a.frame(facing, 0), b.frame(facing, 0));
            for y in 0..CELL {
                for x in 0..CELL {
                    if fa.get(x, y) != fb.get(x, y) {
                        changed += 1;
                        assert!(material.contains(&fb.get(x, y)), "{facing:?} ({x}, {y})");
                    }
                }
            }
        }
        assert!(changed > 20, "only {changed} pixels changed");
    }

    #[test]
    fn trousers_shoes_and_the_accessory_take_their_own_colours() {
        let usual = HandlerLook::default();
        only_dyes(
            HandlerLook {
                trousers: 2,
                ..usual
            },
            &TROUSERS,
        );
        only_dyes(HandlerLook { shoes: 1, ..usual }, &SHOES);
        only_dyes(
            HandlerLook {
                accessory_colour: 2,
                ..usual
            },
            &SATCHEL,
        );
        only_dyes(
            HandlerLook {
                accessory: 2,
                accessory_colour: 4,
                ..usual
            },
            &SCARF,
        );
        only_dyes(
            HandlerLook {
                accessory: 3,
                accessory_colour: 3,
                ..usual
            },
            &FIELD_CAP,
        );
    }

    #[test]
    fn glasses_and_the_magnifier_keep_their_own_colours() {
        for accessory in [1, 4, 5] {
            let look = HandlerLook {
                accessory,
                accessory_colour: 3,
                ..HandlerLook::default()
            };
            assert_eq!(look.own_accessory_colour(), None);
            let plain = HandlerLook {
                accessory_colour: 0,
                ..look
            };
            assert_eq!(
                look.sheet().frame(Facing::Left, 6),
                plain.sheet().frame(Facing::Left, 6)
            );
        }
    }

    #[test]
    fn a_look_reads_from_a_file_missing_its_newer_choices() {
        let look: HandlerLook = serde_json::from_str(r#"{"hair": 2}"#).unwrap();
        assert_eq!(look.hair, 2);
        assert_eq!(look.skin, 0);
    }

    #[test]
    fn the_summary_names_every_choice() {
        assert_eq!(
            HandlerLook::default().summary(),
            "Tousled hair, a field jacket, the satchel"
        );
        let dyed = HandlerLook {
            hair: 3,
            outfit_colour: 2,
            ..HandlerLook::default()
        };
        assert_eq!(
            dyed.summary(),
            "Auburn tousled hair, a forest field jacket, the satchel"
        );
    }
}
