//! Character sheets: the Handler's and the townspeople's frames, Jake's hand-drawn art read from
//! the PNG files built into Battle (`art`). A sheet is cells of 48 × 48 pixels, a row for each
//! way of facing (down, left, right, up) and a column for each frame. The feet rest on the same
//! pixel row in every cell, so frames and layers line up with no shifting.
//!
//! A full sheet is 8 frames across: four of walking, two of standing (breathing in and out) and
//! two of its own (the Handler reaching, Wren pointing, the gardener's can, the angler's rod).
//! A staff sheet is 4 across: two of standing and two of talking.

use super::art;
use super::map::Facing;
use formiga_art::{Canvas, Rgba};

/// A cell's size, in pixels.
pub const CELL: i32 = 48;
/// The pixel row the feet rest on, counted from the top of a cell.
pub const FEET: i32 = 43;
/// The frames a full sheet has across.
pub const FRAMES: usize = 8;

/// A character sheet, read.
#[derive(Clone, Debug)]
pub struct Sheet {
    canvas: Canvas,
    /// How many frames it has across: 8 for a full sheet, 4 for a staff sheet.
    across: usize,
}

impl Sheet {
    /// A sheet already stacked and dyed.
    pub fn from_canvas(canvas: Canvas, across: usize) -> Self {
        Self { canvas, across }
    }

    /// A sheet from a PNG: 4 rows of 48-pixel cells, 4 or 8 across.
    pub fn read(bytes: &[u8]) -> Option<Self> {
        let canvas = decode(bytes)?;
        let across = (canvas.width() / CELL as u32) as usize;
        let fits = canvas.width() % CELL as u32 == 0 && canvas.height() == 4 * CELL as u32;
        (fits && (across == 4 || across == FRAMES)).then_some(Self { canvas, across })
    }

    /// A townsperson's sheet, by their id: `npc-<id>.png`.
    pub fn person(id: &str) -> Option<Self> {
        art::bytes(&format!("npc-{id}.png")).and_then(Self::read)
    }

    /// One frame by its column.
    pub fn frame(&self, facing: Facing, column: usize) -> Canvas {
        let column = column.min(self.across - 1);
        let mut out = Canvas::new(CELL as u32, CELL as u32);
        let (ox, oy) = (column as i32 * CELL, facing.row() as i32 * CELL);
        for y in 0..CELL {
            for x in 0..CELL {
                out.set(x, y, self.canvas.get(ox + x, oy + y));
            }
        }
        out
    }

    /// Standing, breathing in or out.
    pub fn standing(&self, facing: Facing, out: bool) -> Canvas {
        let first = if self.across == FRAMES { 4 } else { 0 };
        self.frame(facing, first + usize::from(out))
    }

    /// Talking (or, on a full sheet, its own two frames: pointing, the can, the rod).
    pub fn talking(&self, facing: Facing, second: bool) -> Canvas {
        let first = if self.across == FRAMES { 6 } else { 2 };
        self.frame(facing, first + usize::from(second))
    }
}

/// A PNG's pixels, whatever colour type it was saved in; none if it is not a PNG.
pub fn decode(bytes: &[u8]) -> Option<Canvas> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let channels = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return None,
    };
    let mut canvas = Canvas::new(info.width, info.height);
    for y in 0..info.height {
        for x in 0..info.width {
            let i = (y as usize * info.line_size) + x as usize * channels;
            let px = &buf[i..i + channels];
            let color = match channels {
                4 => Rgba::new(px[0], px[1], px[2], px[3]),
                3 => Rgba::new(px[0], px[1], px[2], 255),
                2 => Rgba::new(px[0], px[0], px[0], px[1]),
                _ => Rgba::new(px[0], px[0], px[0], 255),
            };
            canvas.set(x as i32, y as i32, color);
        }
    }
    Some(canvas)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_picture_reads() {
        for name in art::ALL {
            let bytes = art::bytes(name).expect("listed");
            assert!(decode(bytes).is_some(), "{name} does not read");
        }
    }

    #[test]
    fn every_townsperson_has_a_sheet_with_feet_on_the_feet_row() {
        for id in [
            "wren", "marlow", "iris", "tailor", "odile", "gardener", "angler",
        ] {
            let sheet = Sheet::person(id).unwrap_or_else(|| panic!("no sheet for {id}"));
            for facing in Facing::ALL {
                let frame = sheet.standing(facing, false);
                let (_, _, _, bottom) = frame.alpha_bounds().expect("drawn");
                assert!(
                    (FEET - 1..=FEET).contains(&(bottom as i32)),
                    "{id} {facing:?}: feet on row {bottom}"
                );
            }
        }
    }

    #[test]
    fn a_picture_that_is_not_a_sheet_is_refused() {
        assert!(Sheet::read(b"not a png").is_none());
    }
}
