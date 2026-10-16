//! Battle's own icon: the Backyard's brass beacon on its paved yard, in a mown lawn, with a
//! pennant for each team either side of it. It is drawn once as pixel art, 32 pixels square, and
//! every size is made from that drawing by whole pixels, so each stays crisp. The same picture is
//! the window's icon, the macOS bundle's `.icns` and the Windows installer's `.ico`; the packaging
//! scripts ask the binary for them, so there is one source.

use crate::garden::{BRASS, GRASS, PLANK, STONE};
use crate::scene::TEAM;
use anyhow::Result;
use formiga_art::paint::{self, Ramp, rgb, rgba};
use formiga_art::{Canvas, Rgba};

/// How big the drawing is, in pixels each way.
pub const SIZE: u32 = 32;

const LAWN_LIGHT: Rgba = rgb(0x8cc178);
const LAWN_DARK: Rgba = rgb(0x7fb46d);
const FLAME: Rgba = rgb(0xf2a03c);
const FLAME_HEART: Rgba = rgb(0xfde9b0);
const SHADOW: Rgba = rgba(0x24452b, 60);
const RIM: Rgba = rgb(0x2c3e33);

/// Whether a pixel is inside the rounded square, its corners cut on a radius of six.
fn inside(x: i32, y: i32) -> bool {
    let size = SIZE as i32;
    let corner = |a: i32| {
        if a < 6 {
            6 - a
        } else if a > size - 7 {
            a - (size - 7)
        } else {
            0
        }
    };
    let (cx, cy) = (corner(x), corner(y));
    (0..size).contains(&x) && (0..size).contains(&y) && cx * cx + cy * cy <= 36
}

/// The icon as drawn.
pub fn draw() -> Canvas {
    let size = SIZE as i32;
    let mut icon = Canvas::new(SIZE, SIZE);
    // The lawn, mown in diagonal stripes as the Backyard's is, with a little grain.
    for y in 0..size {
        for x in 0..size {
            if !inside(x, y) {
                continue;
            }
            let mut color = if ((x + y) / 4) % 2 == 0 {
                LAWN_LIGHT
            } else {
                LAWN_DARK
            };
            if paint::chance(x, y, 71, 28) {
                color = paint::mix(color, GRASS.base, 0.5);
            }
            icon.set(x, y, color);
        }
    }
    // The paved yard: a tile seen from above and to the side, lit from the upper left.
    let (yard_x, yard_y, half_w, half_h) = (16.0f32, 21.0f32, 10.0f32, 5.0f32);
    let yard = [
        (yard_x, yard_y - half_h),
        (yard_x + half_w, yard_y),
        (yard_x, yard_y + half_h),
        (yard_x - half_w, yard_y),
    ];
    paint::fill_polygon(&yard, |x, y| {
        let color = if y < yard_y as i32 {
            STONE.light
        } else {
            STONE.base
        };
        let color = if paint::chance(x, y, 73, 40) {
            paint::mix(color, STONE.shadow, 0.4)
        } else {
            color
        };
        icon.set(x, y, color);
    });
    for (from, to, tone) in [
        (yard[3], yard[0], STONE.shine),
        (yard[0], yard[1], STONE.light),
        (yard[1], yard[2], STONE.edge),
        (yard[2], yard[3], STONE.edge),
    ] {
        let point = |(x, y): (f32, f32)| (x as i32, y as i32);
        paint::line(&mut icon, point(from), point(to), tone);
    }
    // The beacon: a brass post with a bowl on top and its flame, its shadow down to the right.
    paint::ellipse(&mut icon, 18, 21, 3, 1, SHADOW);
    paint::vline(&mut icon, 15, 11, 10, BRASS.light);
    paint::vline(&mut icon, 16, 11, 10, BRASS.base);
    paint::vline(&mut icon, 17, 11, 10, BRASS.shadow);
    paint::hline(&mut icon, 14, 20, 5, BRASS.edge);
    paint::hline(&mut icon, 13, 10, 7, BRASS.base);
    paint::hline(&mut icon, 13, 9, 7, BRASS.light);
    paint::put(&mut icon, 13, 10, BRASS.edge);
    paint::put(&mut icon, 19, 10, BRASS.edge);
    paint::put(&mut icon, 14, 9, BRASS.shine);
    paint::ellipse(&mut icon, 16, 6, 2, 3, FLAME);
    paint::vline(&mut icon, 16, 5, 3, FLAME_HEART);
    paint::put(&mut icon, 16, 2, FLAME);
    // A pennant for each team on a garden cane, both flying towards the beacon.
    pennant(&mut icon, 5, 1, TEAM[0]);
    pennant(&mut icon, 26, -1, TEAM[1]);
    // A soft rim, so the square reads on a light dock or a dark one.
    for y in 0..size {
        for x in 0..size {
            if inside(x, y)
                && [(-1, 0), (1, 0), (0, -1), (0, 1)]
                    .iter()
                    .any(|(dx, dy)| !inside(x + dx, y + dy))
            {
                let pixel = icon.get(x, y);
                icon.set(x, y, paint::mix(pixel, RIM, 0.45));
            }
        }
    }
    icon
}

/// A pennant on a cane standing at `x`, flying towards `toward` (1 is right, -1 left).
fn pennant(icon: &mut Canvas, x: i32, toward: i32, team: Ramp) {
    let foot = 25;
    paint::ellipse(icon, x + 1, foot, 2, 1, SHADOW);
    paint::vline(icon, x, foot - 17, 17, PLANK.shadow);
    paint::vline(icon, x + toward.max(0), foot - 16, 16, PLANK.light);
    let base = (x + toward) as f32;
    let flag = [
        (base, (foot - 17) as f32),
        (base + 6.0 * toward as f32, (foot - 13) as f32),
        (base, (foot - 9) as f32),
    ];
    paint::polygon(icon, &flag, team.base);
    paint::line(
        icon,
        (x + toward, foot - 17),
        (x + 6 * toward, foot - 13),
        team.light,
    );
    paint::line(
        icon,
        (x + toward, foot - 9),
        (x + 6 * toward, foot - 13),
        team.shadow,
    );
    paint::put(icon, x, foot - 18, team.shine);
}

/// The icon at `size` pixels each way: scaled up by whole pixels, or down by averaging.
pub fn at(size: u32) -> Canvas {
    let drawn = draw();
    let mut out = Canvas::new(size, size);
    if size >= SIZE {
        let scale = size / SIZE;
        let offset = ((size - SIZE * scale) / 2) as i32;
        for y in 0..(SIZE * scale) as i32 {
            for x in 0..(SIZE * scale) as i32 {
                let pixel = drawn.get(x / scale as i32, y / scale as i32);
                out.set(x + offset, y + offset, pixel);
            }
        }
    } else {
        let block = SIZE / size;
        for y in 0..size as i32 {
            for x in 0..size as i32 {
                let mut sum = [0u32; 4];
                for dy in 0..block as i32 {
                    for dx in 0..block as i32 {
                        let p = drawn.get(x * block as i32 + dx, y * block as i32 + dy);
                        let a = u32::from(p.a);
                        sum[0] += u32::from(p.r) * a;
                        sum[1] += u32::from(p.g) * a;
                        sum[2] += u32::from(p.b) * a;
                        sum[3] += a;
                    }
                }
                let n = block * block;
                // Colours weighted by how much of each pixel is there; nothing where none is.
                let average = |channel: u32| channel.checked_div(sum[3]).map(|v| v as u8);
                if let (Some(r), Some(g), Some(b)) =
                    (average(sum[0]), average(sum[1]), average(sum[2]))
                {
                    out.set(x, y, Rgba::new(r, g, b, (sum[3] / n) as u8));
                }
            }
        }
    }
    out
}

/// A picture as PNG bytes.
fn png(picture: &Canvas) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, picture.width(), picture.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(&picture.rgba_bytes()))
        .expect("a PNG written to memory");
    out
}

/// A macOS `.icns`: each size as a PNG, under the type codes macOS reads PNGs from.
pub fn icns() -> Vec<u8> {
    // Each size at 1x and 2x, as an `.iconset` makes them.
    let entries: [(&[u8; 4], u32); 10] = [
        (b"icp4", 16),
        (b"ic11", 32),
        (b"icp5", 32),
        (b"ic12", 64),
        (b"ic07", 128),
        (b"ic13", 256),
        (b"ic08", 256),
        (b"ic14", 512),
        (b"ic09", 512),
        (b"ic10", 1024),
    ];
    let mut body = Vec::new();
    for (code, size) in entries {
        let data = png(&at(size));
        body.extend_from_slice(code);
        body.extend_from_slice(&(data.len() as u32 + 8).to_be_bytes());
        body.extend_from_slice(&data);
    }
    let mut out = Vec::with_capacity(body.len() + 8);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&(body.len() as u32 + 8).to_be_bytes());
    out.extend_from_slice(&body);
    out
}

/// A Windows `.ico`: each size as a PNG, which every Windows since Vista reads.
pub fn ico() -> Vec<u8> {
    // Only sizes made from the drawing by whole pixels; Windows scales between them.
    let sizes = [16u32, 32, 64, 128, 256];
    let images: Vec<Vec<u8>> = sizes.iter().map(|size| png(&at(*size))).collect();
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (size, image) in sizes.iter().zip(&images) {
        // A side of 256 is written as 0.
        let side = if *size >= 256 { 0 } else { *size as u8 };
        out.extend_from_slice(&[side, side, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(image.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += image.len() as u32;
    }
    for image in images {
        out.extend_from_slice(&image);
    }
    out
}

/// The window's icon, at the size the title bar and the dock ask for.
pub fn window() -> eframe::egui::IconData {
    let picture = at(256);
    eframe::egui::IconData {
        rgba: picture.rgba_bytes(),
        width: picture.width(),
        height: picture.height(),
    }
}

/// Writes the icon files the packaging scripts put in the bundle and the installer.
pub fn write(folder: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(folder)?;
    std::fs::write(folder.join("FormigaBattle.icns"), icns())?;
    std::fs::write(folder.join("FormigaBattle.ico"), ico())?;
    std::fs::write(folder.join("FormigaBattle.png"), png(&at(1024)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_size_is_the_same_picture_made_crisp() {
        let drawn = draw();
        let big = at(256);
        for y in 0..SIZE as i32 {
            for x in 0..SIZE as i32 {
                assert_eq!(big.get(x * 8 + 3, y * 8 + 5), drawn.get(x, y));
            }
        }
        // The corners are clear, so the rounded square reads as one.
        assert_eq!(drawn.get(0, 0).a, 0);
        assert_eq!(at(16).get(0, 0).a, 0);
    }

    #[test]
    fn the_icon_files_say_what_they_hold() {
        let icns = icns();
        assert_eq!(&icns[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize,
            icns.len()
        );
        let ico = ico();
        assert_eq!(&ico[..4], &[0, 0, 1, 0]);
        let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
        let last = 6 + 16 * (count - 1);
        let size = u32::from_le_bytes(ico[last + 8..last + 12].try_into().unwrap()) as usize;
        let offset = u32::from_le_bytes(ico[last + 12..last + 16].try_into().unwrap()) as usize;
        assert_eq!(offset + size, ico.len(), "the last image runs to the end");
        assert_eq!(&ico[offset + 1..offset + 4], b"PNG");
    }
}
