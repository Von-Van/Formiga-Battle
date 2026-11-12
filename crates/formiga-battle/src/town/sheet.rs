//! Character sheets: the Handler's and the townspeople's frames, as drawn by hand and read from
//! PNG files built into Battle. A sheet is cells of 48 × 48 pixels, a row for each way of facing
//! (down, left, right, up) and a column for each frame: four of walking, two of standing and two
//! of reaching. The feet rest on the same pixel row in every cell, so frames and layers line up
//! with no shifting.
//!
//! Until the layered sheets are drawn, the Handler is the first sheet Jake made: four walking
//! frames and nothing else, standing on its first walking frame.

use super::map::Facing;
use formiga_art::{Canvas, Rgba};

/// A cell's size, in pixels.
pub const CELL: i32 = 48;
/// The pixel row the feet rest on, counted from the top of a cell.
pub const FEET: i32 = 43;
/// The frames a full sheet has across.
pub const FRAMES: usize = 8;

/// The Handler's stand-in sheet: Jake's first, walking only.
const STAND_IN: &[u8] = include_bytes!("../../assets/town/handler-standin.png");

/// A character sheet, read.
#[derive(Clone, Debug)]
pub struct Sheet {
    canvas: Canvas,
    /// How many frames it has across: 8 for a full sheet, 4 for one that only walks.
    across: usize,
}

impl Sheet {
    /// The Handler as the stand-in sheet draws them.
    pub fn stand_in() -> Self {
        Self::read(STAND_IN).expect("the stand-in sheet is a 4 × 4 sheet of 48-pixel cells")
    }

    /// A sheet from a PNG: 4 rows of 48-pixel cells, 4 or 8 across.
    pub fn read(bytes: &[u8]) -> Option<Self> {
        let canvas = decode(bytes)?;
        let across = (canvas.width() / CELL as u32) as usize;
        let fits = canvas.width() % CELL as u32 == 0 && canvas.height() == 4 * CELL as u32;
        (fits && (across == 4 || across == FRAMES)).then_some(Self { canvas, across })
    }

    /// One frame: a column of a full sheet (walk 0 to 3, stand 4 and 5, reach 6 and 7). A sheet
    /// that only walks stands and reaches on its first walking frame.
    pub fn frame(&self, facing: Facing, column: usize) -> Canvas {
        let column = if column < self.across { column } else { 0 };
        let mut out = Canvas::new(CELL as u32, CELL as u32);
        let (ox, oy) = (column as i32 * CELL, facing.row() as i32 * CELL);
        for y in 0..CELL {
            for x in 0..CELL {
                out.set(x, y, self.canvas.get(ox + x, oy + y));
            }
        }
        out
    }
}

impl Sheet {
    /// Every pixel of the sheet run through `f`, given its row.
    pub(super) fn map_pixels(&mut self, f: impl Fn(i32, Rgba) -> Rgba) {
        for y in 0..self.canvas.height() as i32 {
            for x in 0..self.canvas.width() as i32 {
                let p = self.canvas.get(x, y);
                self.canvas.set(x, y, f(y, p));
            }
        }
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
    fn the_stand_in_reads_as_a_walking_sheet() {
        let sheet = Sheet::stand_in();
        assert_eq!(sheet.across, 4);
    }

    #[test]
    fn every_stand_in_frame_has_its_feet_on_the_feet_row() {
        let sheet = Sheet::stand_in();
        for facing in Facing::ALL {
            for column in 0..4 {
                let frame = sheet.frame(facing, column);
                let (_, _, _, bottom) = frame.alpha_bounds().expect("every frame is drawn");
                assert!(
                    (FEET - 3..=FEET).contains(&(bottom as i32)),
                    "{facing:?} {column}: feet on row {bottom}"
                );
            }
        }
    }

    #[test]
    fn standing_on_a_walking_sheet_uses_its_first_frame() {
        let sheet = Sheet::stand_in();
        assert_eq!(sheet.frame(Facing::Left, 5), sheet.frame(Facing::Left, 0));
    }

    #[test]
    fn a_picture_that_is_not_a_sheet_is_refused() {
        assert!(Sheet::read(b"not a png").is_none());
    }
}
