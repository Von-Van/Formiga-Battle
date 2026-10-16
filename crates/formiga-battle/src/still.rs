//! A picture of the whole window without opening one: the app is run frame by frame through egui
//! alone, as the tests drive it, and its last frame is painted here on the processor, triangle by
//! triangle, with the textures it uploaded. Nothing appears on the screen and nothing takes the
//! keyboard, so review pictures of every page can be made while someone is using the computer.
//!
//! It blends as the window does, premultiplied in the colours as they are stored, so the picture
//! matches a `--snap` of the same moment closely enough to judge the art by; it is for looking,
//! not for comparing pixel by pixel.

use crate::app::BattleApp;
use eframe::egui::{
    self, Color32, ColorImage, Rect, TextureFilter, TextureId, TextureOptions, epaint, pos2, vec2,
};
use formiga_art::{Canvas, Rgba};
use std::collections::HashMap;

/// How often the app is asked for a frame while the moment comes round, as a window at 30
/// frames a second would ask.
const STEP: f64 = 1.0 / 30.0;

/// The window `size` points across and down, `at` seconds after it opened, at `scale` pixels to
/// the point (2 for a Retina screen).
pub fn picture(app: &mut BattleApp, size: (f32, f32), at: f32, scale: f32) -> Canvas {
    let ctx = egui::Context::default();
    let mut textures: HashMap<TextureId, (ColorImage, TextureOptions)> = HashMap::new();
    let mut time = 0.0;
    loop {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(size.0, size.1))),
            time: Some(time),
            viewports: std::iter::once((
                egui::ViewportId::ROOT,
                egui::ViewportInfo {
                    native_pixels_per_point: Some(scale),
                    ..Default::default()
                },
            ))
            .collect(),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.frame(ui));
        let set = std::mem::take(&mut output.textures_delta.set);
        let free = std::mem::take(&mut output.textures_delta.free);
        for (id, deltas) in set {
            for delta in deltas {
                upload(&mut textures, id, delta);
            }
        }
        if time >= f64::from(at) {
            let shapes = std::mem::take(&mut output.shapes);
            let primitives = ctx.tessellate(shapes, output.pixels_per_point);
            return paint(
                &primitives,
                &textures,
                output.pixels_per_point,
                (size.0 * output.pixels_per_point) as u32,
                (size.1 * output.pixels_per_point) as u32,
            );
        }
        for id in free {
            textures.remove(&id);
        }
        time += STEP;
    }
}

fn upload(
    textures: &mut HashMap<TextureId, (ColorImage, TextureOptions)>,
    id: TextureId,
    delta: epaint::ImageDelta,
) {
    let epaint::ImageData::Color(image) = delta.image;
    match delta.pos {
        None => {
            textures.insert(id, ((*image).clone(), delta.options));
        }
        Some([x0, y0]) => {
            let Some((whole, _)) = textures.get_mut(&id) else {
                return;
            };
            let [w, h] = image.size;
            for y in 0..h {
                for x in 0..w {
                    let (tx, ty) = (x0 + x, y0 + y);
                    if tx < whole.size[0] && ty < whole.size[1] {
                        whole.pixels[ty * whole.size[0] + tx] = image.pixels[y * w + x];
                    }
                }
            }
        }
    }
}

/// Every triangle, back to front, onto the notebook's cover colour.
fn paint(
    primitives: &[epaint::ClippedPrimitive],
    textures: &HashMap<TextureId, (ColorImage, TextureOptions)>,
    ppp: f32,
    width: u32,
    height: u32,
) -> Canvas {
    let cover = crate::chrome::COVER;
    let clear = [
        f32::from(cover.r) / 255.0,
        f32::from(cover.g) / 255.0,
        f32::from(cover.b) / 255.0,
        1.0,
    ];
    let mut pixels = vec![clear; (width * height) as usize];
    for primitive in primitives {
        let epaint::Primitive::Mesh(mesh) = &primitive.primitive else {
            continue;
        };
        let texture = textures.get(&mesh.texture_id);
        let clip = primitive.clip_rect;
        let clip = (
            (clip.min.x * ppp).round().max(0.0) as i32,
            (clip.min.y * ppp).round().max(0.0) as i32,
            ((clip.max.x * ppp).round() as i32).min(width as i32),
            ((clip.max.y * ppp).round() as i32).min(height as i32),
        );
        for corners in mesh.indices.chunks_exact(3) {
            let v = [
                &mesh.vertices[corners[0] as usize],
                &mesh.vertices[corners[1] as usize],
                &mesh.vertices[corners[2] as usize],
            ];
            triangle(&mut pixels, width, clip, ppp, v, texture);
        }
    }
    let mut canvas = Canvas::new(width, height);
    for (i, p) in pixels.iter().enumerate() {
        let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        // Opaque throughout, since the cover is painted under everything.
        let a = p[3].max(1e-6);
        canvas.set(
            (i as u32 % width) as i32,
            (i as u32 / width) as i32,
            Rgba::new(byte(p[0] / a), byte(p[1] / a), byte(p[2] / a), 255),
        );
    }
    canvas
}

fn triangle(
    pixels: &mut [[f32; 4]],
    width: u32,
    clip: (i32, i32, i32, i32),
    ppp: f32,
    v: [&epaint::Vertex; 3],
    texture: Option<&(ColorImage, TextureOptions)>,
) {
    let p = v.map(|v| (v.pos.x * ppp, v.pos.y * ppp));
    let area = edge(p[0], p[1], p[2]);
    if area.abs() < 1e-6 {
        return;
    }
    let min_x = (p.iter().map(|p| p.0).fold(f32::MAX, f32::min).floor() as i32).max(clip.0);
    let min_y = (p.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor() as i32).max(clip.1);
    let max_x = (p.iter().map(|p| p.0).fold(f32::MIN, f32::max).ceil() as i32).min(clip.2);
    let max_y = (p.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil() as i32).min(clip.3);
    let colours = v.map(|v| unit(v.color));
    for y in min_y..max_y {
        for x in min_x..max_x {
            let c = (x as f32 + 0.5, y as f32 + 0.5);
            let w = [
                edge(p[1], p[2], c) / area,
                edge(p[2], p[0], c) / area,
                edge(p[0], p[1], c) / area,
            ];
            if w.iter().any(|w| *w < 0.0) {
                continue;
            }
            let mut src = [0.0; 4];
            for (k, s) in src.iter_mut().enumerate() {
                *s = w[0] * colours[0][k] + w[1] * colours[1][k] + w[2] * colours[2][k];
            }
            if let Some((image, options)) = texture {
                let u = w[0] * v[0].uv.x + w[1] * v[1].uv.x + w[2] * v[2].uv.x;
                let t = w[0] * v[0].uv.y + w[1] * v[1].uv.y + w[2] * v[2].uv.y;
                let texel = sample(image, options.magnification, u, t);
                for k in 0..4 {
                    src[k] *= texel[k];
                }
            }
            let dst = &mut pixels[(y as u32 * width + x as u32) as usize];
            let keep = 1.0 - src[3];
            for k in 0..4 {
                dst[k] = src[k] + dst[k] * keep;
            }
        }
    }
}

fn edge(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn unit(c: Color32) -> [f32; 4] {
    c.to_array().map(|b| f32::from(b) / 255.0)
}

fn sample(image: &ColorImage, filter: TextureFilter, u: f32, v: f32) -> [f32; 4] {
    let [w, h] = image.size;
    let texel = |x: i32, y: i32| {
        let x = x.clamp(0, w as i32 - 1) as usize;
        let y = y.clamp(0, h as i32 - 1) as usize;
        unit(image.pixels[y * w + x])
    };
    let (x, y) = (u * w as f32, v * h as f32);
    match filter {
        TextureFilter::Nearest => texel(x.floor() as i32, y.floor() as i32),
        TextureFilter::Linear => {
            let (x, y) = (x - 0.5, y - 0.5);
            let (x0, y0) = (x.floor(), y.floor());
            let (fx, fy) = (x - x0, y - y0);
            let (x0, y0) = (x0 as i32, y0 as i32);
            let (a, b, c, d) = (
                texel(x0, y0),
                texel(x0 + 1, y0),
                texel(x0, y0 + 1),
                texel(x0 + 1, y0 + 1),
            );
            let mut out = [0.0; 4];
            for k in 0..4 {
                let top = a[k] + (b[k] - a[k]) * fx;
                let foot = c[k] + (d[k] - c[k]) * fx;
                out[k] = top + (foot - top) * fy;
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::Host;
    use crate::sound::Sound;
    use crate::table::Table;
    use formiga_battle_contract::sample;

    #[test]
    fn a_picture_of_the_window_is_made_without_one_at_its_own_size() {
        let table = Table::new(sample::snapshot(), sample::visitors(9), 5);
        let host = Host::Rehearsal {
            from: "a test".to_owned(),
        };
        let mut app = BattleApp::new(
            host,
            table,
            Some(crate::app::Review::Where),
            None,
            Sound::silent(),
        );
        let picture = picture(&mut app, (1024.0, 640.0), 0.5, 1.0);
        assert_eq!((picture.width(), picture.height()), (1024, 640));
        // The cover's leather at the corner, and the notebook's pages and pictures within.
        let corner = picture.get(2, 2);
        let middle = picture.get(512, 320);
        assert_ne!(corner, middle);
        let mut colours = std::collections::HashSet::new();
        for y in (0..640).step_by(7) {
            for x in (0..1024).step_by(7) {
                let p = picture.get(x, y);
                colours.insert((p.r, p.g, p.b));
            }
        }
        assert!(colours.len() > 200, "only {} colours", colours.len());
    }
}
