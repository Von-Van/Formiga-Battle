//! Woodland Glade: a clearing in the trees. The board is moss with leaf litter blown across it,
//! the beacon burns in a ring of stones on a patch of packed earth, old stumps and mossy boulders
//! stand where nobody can walk, and pines and an oak close the glade in at the back.

use super::*;

pub const MOSS: Ramp = Ramp::new(0x2c4220, 0x3e5a2a, 0x557634, 0x6f9442, 0x93b45e);
/// Fallen leaves, in autumn's browns and golds.
const LITTER: [Rgba; 4] = [rgb(0xb5703a), rgb(0xd09048), rgb(0x8a5a30), rgb(0xc8a050)];
/// A toadstool's cap.
const CAP: Ramp = Ramp::new(0x6a1e18, 0x9a2e22, 0xc8463a, 0xe0705a, 0xf4a08a);
/// Pine needles, a cooler green than the oak's leaves.
const PINE: Ramp = Ramp::new(0x1e3a2c, 0x2a5040, 0x3a6a52, 0x528a68, 0x78ae88);

/// One pixel of the glade's moss: soft two-pixel clumps, a faint check so the tiles read, leaf
/// litter, and the seam round every tile.
pub(super) fn moss(px: i32, py: i32, fx: f32, fy: f32, tile: Tile) -> Rgba {
    let base = if (tile.x + tile.y) % 2 == 0 {
        MOSS.light
    } else {
        paint::mix(MOSS.light, MOSS.base, 0.4)
    };
    let n = paint::noise(px.div_euclid(2), py.div_euclid(2), 131) % 100;
    let mut c = match n {
        0..=11 => paint::mix(base, MOSS.shine, 0.45),
        12..=22 => paint::mix(base, MOSS.base, 0.6),
        _ => base,
    };
    let p = patches(px, py, (22, 12), 137);
    if p < 0.3 {
        c = paint::mix(c, MOSS.base, 0.3 - p);
    }
    if paint::chance(px, py, 139, 5) {
        c = LITTER[(paint::noise(px, py, 141) % 4) as usize];
    }
    let (ux, uy) = (fx - fx.floor(), fy - fy.floor());
    if ux < 0.035 || uy < 0.035 {
        c = paint::mix(c, MOSS.shadow, 0.6);
    } else if ux > 0.965 || uy > 0.965 {
        c = paint::mix(c, MOSS.shine, 0.2);
    }
    c
}

/// One pixel of the woodland floor round the glade: darker, and deep in leaves.
pub(super) fn litter(px: i32, py: i32) -> Rgba {
    let p = patches(px, py, (16, 9), 143);
    let mut c = paint::mix(MOSS.shadow, SOIL.base, 0.3 + p * 0.3);
    if paint::chance(px, py, 147, 30) {
        c = LITTER[(paint::noise(px, py, 149) % 4) as usize];
    } else if paint::chance(px, py, 151, 34) {
        c = MOSS.base;
    }
    c
}

/// Something small in a tile's corner: a toadstool, a fern's sprig or a few fallen leaves.
pub(super) fn corner(canvas: &mut Canvas, x: i32, y: i32, m: u32) {
    if m.is_multiple_of(5) {
        toadstool(canvas, x, y);
    } else if m.is_multiple_of(2) {
        sprig(canvas, x, y, m as i32);
    } else {
        for (dx, dy) in [(0, 0), (2, 1), (-1, 1)] {
            paint::put(
                canvas,
                x + dx,
                y + dy,
                LITTER[((m >> 4) as i32 + dx).rem_euclid(4) as usize],
            );
        }
    }
}

fn toadstool(canvas: &mut Canvas, x: i32, y: i32) {
    paint::vline(canvas, x, y - 2, 2, rgb(0xf0e6d0));
    paint::hline(canvas, x - 1, y - 3, 3, CAP.base);
    paint::put(canvas, x - 1, y - 3, CAP.light);
    paint::put(canvas, x, y - 4, CAP.base);
    paint::put(canvas, x, y - 3, rgb(0xfbf6ee));
    paint::put(canvas, x + 1, y - 2, CAP.shadow);
}

/// A fern's sprig: a stalk with a leaflet either side.
pub(super) fn sprig(canvas: &mut Canvas, x: i32, y: i32, k: i32) {
    let lean = (k % 3) - 1;
    paint::put(canvas, x, y, LEAF.shadow);
    paint::put(canvas, x, y - 1, LEAF.base);
    paint::put(canvas, x + lean, y - 2, LEAF.light);
    paint::put(canvas, x - 1, y - 1, LEAF.light);
    paint::put(canvas, x + 1, y - 2, LEAF.base);
    paint::put(canvas, x + lean, y - 3, LEAF.shine);
}

/// A fern, five fronds fanning out from its root.
fn fern(canvas: &mut Canvas, x: i32, y: i32, salt: u32) {
    for (k, (dx, h)) in [(-3, 3), (-2, 4), (0, 5), (2, 4), (3, 3)]
        .iter()
        .enumerate()
    {
        for up in 0..*h {
            let px = x + dx * up / h.max(&1);
            let c = if up == h - 1 {
                LEAF.shine
            } else if (k + up as usize).is_multiple_of(2) {
                LEAF.light
            } else {
                LEAF.base
            };
            paint::put(canvas, px, y - up, c);
        }
    }
    let _ = salt;
    paint::put(canvas, x, y, LEAF.shadow);
}

/// Along the two near edges: a mossy bank with ferns, toadstools and the odd stone.
pub(super) fn beds(canvas: &mut Canvas, view: &View, w: f32, d: f32, homes: &mut Vec<(i32, i32)>) {
    let n = NEAR_BORDER;
    let strips = [
        (w + 0.05, -FAR_BORDER + 0.5, w + n - 0.05, d + n - 0.05),
        (-FAR_BORDER + 0.5, d + 0.05, w + n - 0.05, d + n - 0.05),
    ];
    for r in strips {
        paint::fill_polygon(&quad(view, r, 0.0), |px, py| {
            let c = if paint::chance(px, py, 19, 40) {
                MOSS.base
            } else if paint::chance(px, py, 23, 36) {
                LITTER[(paint::noise(px, py, 29) % 4) as usize]
            } else {
                paint::mix(MOSS.shadow, SOIL.shadow, 0.4)
            };
            paint::put(canvas, px, py, c);
        });
    }
    let mut k = 0u32;
    let mut place = |canvas: &mut Canvas, x: f32, y: f32| {
        let m = paint::noise(k as i32, 7, 97);
        let (sx, sy) = view.pixel(x, y);
        match m % 4 {
            0 => toadstool(canvas, sx, sy),
            1 => {
                paint::ellipse(canvas, sx, sy, 3, 2, STONE.shadow);
                paint::ellipse(canvas, sx, sy - 1, 3, 2, STONE.base);
                paint::put(canvas, sx - 1, sy - 2, STONE.light);
                paint::put(canvas, sx + 1, sy - 2, MOSS.light);
            }
            _ => {
                fern(canvas, sx, sy, m);
                homes.push((sx, sy - 5));
            }
        }
        k += 1;
    };
    let mut t = -FAR_BORDER + 0.7;
    while t < d + n - 0.2 {
        place(canvas, w + 0.2, t);
        t += 0.29;
    }
    let mut t = -FAR_BORDER + 0.7;
    while t < w + n - 0.2 {
        place(canvas, t, d + 0.2);
        t += 0.29;
    }
}

/// One tile of the glade's yard: packed earth, scuffed bare round the fire, with a border of
/// small stones wherever it meets the moss.
pub(super) fn earth(canvas: &mut Canvas, arena: &Arena, view: &View, tile: Tile) {
    let open = |dx: i8, dy: i8| !arena.in_zone(Tile::new(tile.x + dx, tile.y + dy));
    let (west, north, east, south) = (open(-1, 0), open(0, -1), open(1, 0), open(0, 1));
    let span = f32::from(arena.width) + f32::from(arena.height);
    paint::fill_polygon(&view.diamond(tile, 0.0), |px, py| {
        let (fx, fy) = view.floor_at(px as f32 + 0.5, py as f32 + 0.5);
        let (ux, uy) = (fx - fx.floor(), fy - fy.floor());
        let p = patches(px, py, (10, 6), 157);
        let mut c = paint::mix(SOIL.light, rgb(0xc8a87a), 0.3 + p * 0.4);
        if paint::chance(px, py, 159, 30) {
            c = SOIL.base;
        } else if paint::chance(px, py, 161, 14) {
            c = STONE.light;
        } else if paint::chance(px, py, 163, 10) {
            c = MOSS.base;
        }
        if ux < 0.03 || uy < 0.03 {
            c = paint::mix(c, SOIL.shadow, 0.45);
        }
        paint::put(canvas, px, py, lit(c, SOIL, ((fy - fx) / span) * 0.16));
    });
    // The ring of stones round the edge of the yard.
    let (x, y) = (f32::from(tile.x), f32::from(tile.y));
    let mut edges = Vec::new();
    if west {
        edges.push(((x + 0.06, y), (x + 0.06, y + 1.0)));
    }
    if east {
        edges.push(((x + 0.94, y), (x + 0.94, y + 1.0)));
    }
    if north {
        edges.push(((x, y + 0.06), (x + 1.0, y + 0.06)));
    }
    if south {
        edges.push(((x, y + 0.94), (x + 1.0, y + 0.94)));
    }
    for (a, b) in edges {
        for k in 0..6 {
            let t = (k as f32 + 0.5) / 6.0;
            let (sx, sy) = view.pixel(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            let n = paint::noise(sx, sy, 167);
            let r = 2 + (n % 2) as i32;
            paint::ellipse(canvas, sx, sy, r, r - 1, STONE.shadow);
            paint::ellipse(canvas, sx, sy - 1, r, r - 1, STONE.base);
            paint::put(canvas, sx - 1, sy - 2, STONE.shine);
            if n.is_multiple_of(3) {
                paint::put(canvas, sx + 1, sy - 2, MOSS.light);
            }
        }
    }
}

/// The fire's own ring of stones, round the beacon's bowl.
pub(super) fn fire_ring(canvas: &mut Canvas, view: &View, centre: Tile) {
    let (cx, cy) = view.centre(centre);
    for k in 0..9 {
        let a = k as f32 / 9.0 * TAU + 0.2;
        let (x, y) = (
            cx + (a.cos() * 13.0).round() as i32,
            cy + (a.sin() * 6.0).round() as i32,
        );
        paint::ellipse(canvas, x, y + 1, 3, 2, STONE.edge);
        paint::ellipse(canvas, x, y, 3, 2, STONE.base);
        paint::put(canvas, x - 1, y - 1, STONE.light);
        paint::put(canvas, x - 2, y, STONE.shine);
    }
}

/// What stands on a board tile in the glade: an old stump, or a mossy boulder.
pub(super) fn piece(view: &View, tile: Tile) -> Sprite {
    let mut full = Canvas::new(view.size.0, view.size.1);
    let (cx, cy) = view.centre(tile);
    let n = paint::noise(i32::from(tile.x), i32::from(tile.y), 171);
    if n.is_multiple_of(2) {
        stump(&mut full, (cx, cy), n);
    } else {
        boulder(&mut full, (cx, cy), n);
    }
    edged(&mut full, 0.5);
    Sprite::cut(&full)
}

/// A sawn-off stump: bark round its sides, rings on its top, roots spreading at its foot.
fn stump(canvas: &mut Canvas, (cx, cy): (i32, i32), salt: u32) {
    let (rx, ry, h) = (11, 5, 11);
    // Roots, out under the bark.
    for (dx, dy, len) in [(-11, 0, 4), (9, 2, 4), (-4, 4, 3), (5, -3, 3)] {
        for k in 0..len {
            paint::put(
                canvas,
                cx + dx + k * dx.signum(),
                cy + dy + k / 2,
                BARK.shadow,
            );
            paint::put(
                canvas,
                cx + dx + k * dx.signum(),
                cy + dy + k / 2 - 1,
                BARK.base,
            );
        }
    }
    // The sides: every column from the top's rim down to the foot's rim.
    for dx in -rx..=rx {
        let along = dx as f32 / rx as f32;
        let rim = (ry as f32 * (1.0 - along * along).max(0.0).sqrt()).round() as i32;
        let level = if along < -0.4 {
            3
        } else if along < 0.3 {
            2
        } else {
            1
        };
        for y in cy - h + rim..=cy + rim {
            let ridge = i32::from(paint::noise(cx + dx, y / 3, salt).is_multiple_of(4));
            let mut c = tone(BARK, level - ridge);
            if along < -0.5 && y > cy - 4 && paint::chance(cx + dx, y, salt + 1, 90) {
                c = MOSS.light;
            }
            paint::put(canvas, cx + dx, y, c);
        }
    }
    // Its top: pale wood, with rings.
    let top = cy - h;
    for oy in -ry..=ry {
        for ox in -rx..=rx {
            let d = (ox * ox) as f32 / (rx * rx) as f32 + (oy * oy) as f32 / (ry * ry) as f32;
            if d > 1.0 {
                continue;
            }
            let ring = (d.sqrt() * 4.0) as i32;
            let c = if d > 0.82 {
                BARK.light
            } else if ring % 2 == 0 {
                PLANK.light
            } else {
                PLANK.base
            };
            paint::put(canvas, cx + ox, top + oy, c);
        }
    }
    paint::put(canvas, cx, top, PLANK.shadow);
    // A bracket fungus on its shaded side.
    paint::hline(canvas, cx + 6, cy - 5, 3, rgb(0xd8b878));
    paint::hline(canvas, cx + 7, cy - 4, 2, rgb(0xa88848));
}

/// A boulder: a lumpy round stone lit on its upper left, with moss on its top.
fn boulder(canvas: &mut Canvas, (cx, cy): (i32, i32), salt: u32) {
    let (rx, ry) = (13.0f32, 10.0f32);
    let centre = (cx as f32, cy as f32 - 7.0);
    for py in (cy - 19)..=(cy + 2) {
        for px in (cx - 15)..=(cx + 15) {
            let u = (px as f32 + 0.5 - centre.0) / rx;
            let v = (py as f32 + 0.5 - centre.1) / ry;
            let lump = (paint::noise(px / 3, py / 3, salt) % 100) as f32 / 900.0;
            // Rounder than an ellipse, as a stone settles: a squarish curve.
            let d = u.abs().powf(2.4) + v.abs().powf(2.4);
            if d > 1.0 - lump || py as f32 > cy as f32 + 1.0 {
                continue;
            }
            let lit = -0.6 * u - 0.8 * v;
            let mut level = if lit > 0.45 {
                4
            } else if lit > 0.05 {
                3
            } else if lit > -0.4 {
                2
            } else {
                1
            };
            level += i32::from(paint::noise(px, py, salt + 1).is_multiple_of(9))
                - i32::from(paint::noise(px, py, salt + 2).is_multiple_of(9));
            let mut c = paint::mix(tone(STONE, level), STONE.shadow, 0.25);
            if v < -0.45 && !paint::noise(px / 2, py / 2, salt + 3).is_multiple_of(3) {
                c = tone(MOSS, level);
            }
            paint::put(canvas, px, py, c);
        }
    }
}

/// The back of the glade: an oak in the far corner, pines along both far edges, a fallen log,
/// brambles between, and a signpost by the left corner. Returns where light moves in the
/// leaves, where the robin perches, and where leaves fall from.
pub(super) fn back(
    canvas: &mut Canvas,
    view: &View,
    w: f32,
    d: f32,
) -> (Points, Vec<Perch>, Points) {
    let mut layer = Canvas::new(canvas.width(), canvas.height());
    // Brambles along the back first, low and dark.
    for (k, (x, y)) in [
        (-0.4, 1.2),
        (-0.4, 3.0),
        (-0.4, 5.2),
        (1.6, -0.4),
        (4.5, -0.4),
        (w - 1.3, -0.4),
    ]
    .iter()
    .enumerate()
    {
        let (sx, sy) = view.pixel(*x, *y);
        cast(canvas, view, (x - 0.3, y - 0.3, x + 0.3, y + 0.3), 10.0);
        for (dx, dy, r) in [(-6, -4, 5), (5, -5, 5), (0, -8, 6), (-2, -2, 4), (4, -1, 4)] {
            lump(&mut layer, (sx + dx, sy + dy), r, 600 + k as u32, 2);
        }
        for j in 0..4 {
            let n = paint::noise(k as i32, j, 607);
            paint::put(
                &mut layer,
                sx - 6 + (n % 12) as i32,
                sy - 10 + ((n >> 5) % 8) as i32,
                rgb(0x5a2a4a),
            );
        }
    }
    // The oak in the far corner.
    let (tx, ty) = view.pixel(-0.42, -0.42);
    dappled(canvas, (tx + 14, ty + 6), 26, 10);
    let mut leaves = oak(&mut layer, (tx, ty + 2), &OAK_CROWN, 613);
    // Pines.
    for (x, y) in [
        (-0.42, 1.6),
        (-0.5, 3.4),
        (-0.42, 5.4),
        (1.8, -0.5),
        (3.9, -0.42),
        (6.0, -0.5),
    ] {
        let (sx, sy) = view.pixel(x, y);
        cast(canvas, view, (x - 0.25, y - 0.25, x + 0.25, y + 0.25), 30.0);
        leaves.extend(pine(&mut layer, (sx, sy), 50));
    }
    // A fallen log along the back right, before the right-hand corner.
    let (a, b) = (view.pixel(w - 2.4, -0.45), view.pixel(w - 0.2, -0.45));
    log(&mut layer, a, b);
    let perches = vec![
        (a.0 + (b.0 - a.0) / 4, a.1 + (b.1 - a.1) / 4 - 7),
        (a.0 + (b.0 - a.0) * 3 / 4, a.1 + (b.1 - a.1) * 3 / 4 - 7),
    ];
    // A signpost by the left corner.
    let (px, py) = view.pixel(-0.5, d - 0.4);
    paint::ellipse(canvas, px + 2, py + 1, 4, 2, SHADOW);
    paint::rect(&mut layer, px - 1, py - 16, 2, 16, PLANK.shadow);
    paint::vline(&mut layer, px - 1, py - 16, 16, PLANK.light);
    paint::rect(&mut layer, px - 7, py - 15, 13, 5, PLANK.base);
    paint::hline(&mut layer, px - 7, py - 15, 13, PLANK.shine);
    paint::hline(&mut layer, px - 5, py - 13, 8, PLANK.edge);
    paint::put(&mut layer, px + 6, py - 13, PLANK.base);
    edged(&mut layer, 0.45);
    paint::blit(canvas, &layer, 0, 0);
    let drops = leaves.iter().step_by(5).copied().collect();
    (leaves, perches, drops)
}

/// A pine: tiers of needles, each lit on its left and shaded on its right, over a short trunk.
fn pine(canvas: &mut Canvas, (x, y): (i32, i32), height: i32) -> Vec<(i32, i32)> {
    let mut tree = Canvas::new(canvas.width(), canvas.height());
    paint::rect(&mut tree, x - 2, y - 9, 4, 9, BARK.base);
    paint::vline(&mut tree, x - 2, y - 9, 9, BARK.light);
    paint::vline(&mut tree, x + 1, y - 9, 9, BARK.shadow);
    let tiers = 4;
    let mut glints = Vec::new();
    for k in 0..tiers {
        let bottom = y - 7 - k * (height - 10) / tiers;
        let tall = (height - 10) / tiers + 6;
        let half = 13 - k * 3;
        for up in 0..tall {
            let span = half * (tall - up) / tall;
            for dx in -span..=span {
                let side = dx as f32 / half.max(1) as f32;
                let level = if side < -0.3 {
                    3
                } else if side < 0.25 {
                    2
                } else {
                    1
                };
                let c = if up == 0 && paint::noise(x + dx, bottom, 617).is_multiple_of(2) {
                    PINE.shadow
                } else {
                    tone(
                        PINE,
                        level + i32::from(paint::noise(x + dx, bottom - up, 619).is_multiple_of(7)),
                    )
                };
                paint::put(&mut tree, x + dx, bottom - up, c);
            }
        }
        glints.push((x - half / 2, bottom - 2));
    }
    edged(&mut tree, 0.5);
    paint::blit(canvas, &tree, 0, 0);
    glints
}

/// A fallen log lying from `a` to `b`, mossy along its top, its cut end towards the viewer.
fn log(canvas: &mut Canvas, a: (i32, i32), b: (i32, i32)) {
    let steps = (b.0 - a.0).abs().max(1);
    for i in 0..=steps {
        let x = a.0 + (b.0 - a.0) * i / steps;
        let y = a.1 + (b.1 - a.1) * i / steps;
        for up in 0..7 {
            let c = match up {
                0 => BARK.shadow,
                1 | 2 => BARK.base,
                3 | 4 => BARK.light,
                5 => {
                    if paint::noise(x, y, 623).is_multiple_of(3) {
                        MOSS.light
                    } else {
                        BARK.light
                    }
                }
                _ => MOSS.base,
            };
            paint::put(canvas, x, y - up, c);
        }
    }
    // Its cut end.
    paint::ellipse(canvas, b.0, b.1 - 3, 3, 4, BARK.edge);
    paint::ellipse(canvas, b.0, b.1 - 3, 2, 3, PLANK.light);
    paint::put(canvas, b.0, b.1 - 3, PLANK.shadow);
}

/// Leaves drifting down out of the trees, swaying as they fall.
pub(super) fn falling(canvas: &mut Canvas, drops: &[(i32, i32)], t: f32, still: bool) {
    if still {
        return;
    }
    for (k, &(x, y)) in drops.iter().enumerate().take(6) {
        let cycle = 6.0 + (k % 3) as f32;
        let age = ((t + k as f32 * 1.7) / cycle).fract();
        let fall = age * 34.0;
        let sway = ((t * 2.2 + k as f32).sin() * 4.0).round() as i32;
        let alpha = (255.0 * (1.0 - age).min(age * 6.0).min(1.0)) as u8;
        let c = paint::faded(LITTER[k % 4], alpha);
        let (px, py) = (x + sway, y + fall as i32);
        paint::put(canvas, px, py, c);
        paint::put(
            canvas,
            px + 1,
            py,
            paint::faded(paint::darker(LITTER[k % 4], 0.3), alpha),
        );
    }
}
