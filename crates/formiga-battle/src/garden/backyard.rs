//! The Backyard: a garden of lawn, with the paved yard round the beacon, hedges and a stretch of
//! brick wall either side of the gate, flower beds along the near edges, and at the back a tree
//! in the far corner, hedgerows, a potting shed and a bird bath, with bunting strung from the
//! tree to the shed. What every place shares (the ramps, the board's slab, the gate, the lever,
//! the robin and the butterflies) is in `garden`.

use super::*;

/// The two stripes the Backyard's lawn is mown in.
const LAWN_LIGHT: Rgba = rgb(0x8cc178);
const LAWN_DARK: Rgba = rgb(0x7fb46d);
const BRICK: Ramp = Ramp::new(0x6a3328, 0x8f4636, 0xab5a42, 0xc4775a, 0xdc9c7c);
const ROOF: Ramp = Ramp::new(0x6e2a22, 0x96392c, 0xb84e38, 0xd06a4c, 0xe48e6c);
/// Hill's flowers.
const BLOSSOMS: [Rgba; 5] = [
    rgb(0xe0605a),
    rgb(0xf19bb0),
    rgb(0xf5d25e),
    rgb(0xfbf6ee),
    rgb(0xb79be0),
];

/// How tall each standing thing is, in pixels.
pub(super) const HEDGE_HEIGHT: f32 = 17.0;
const WALL_HEIGHT: f32 = 14.0;
const HEDGEROW_HEIGHT: f32 = 21.0;
const SHED_WALL: f32 = 25.0;
const SHED_RIDGE: f32 = 37.0;

/// One pixel of mown lawn: stripes a tile wide, broad patches of lighter and darker grass,
/// speckle, and a faint seam round every tile so the grid reads.
pub(super) fn lawn(px: i32, py: i32, fx: f32, fy: f32, tile: Tile) -> Rgba {
    let stripe = if tile.x % 2 == 0 {
        LAWN_LIGHT
    } else {
        LAWN_DARK
    };
    let p = patches(px, py, (26, 14), 31);
    let mut c = if p > 0.62 {
        paint::mix(stripe, GRASS.shine, (p - 0.62) * 0.7)
    } else if p < 0.34 {
        paint::mix(stripe, GRASS.base, (0.34 - p) * 0.9)
    } else {
        stripe
    };
    if paint::chance(px, py, 41, 18) {
        c = paint::mix(c, GRASS.shine, 0.5);
    } else if paint::chance(px, py, 43, 16) {
        c = paint::mix(c, GRASS.base, 0.55);
    }
    let (ux, uy) = (fx - fx.floor(), fy - fy.floor());
    if ux < 0.035 || uy < 0.035 {
        c = paint::mix(c, GRASS.shadow, 0.55);
    } else if ux > 0.965 || uy > 0.965 {
        c = paint::mix(c, GRASS.shine, 0.25);
    }
    c
}

/// One pixel of the garden's longer grass.
pub(super) fn rough(px: i32, py: i32) -> Rgba {
    let p = patches(px, py, (18, 10), 37);
    let mut c = paint::mix(GRASS.base, GRASS.light, p * 0.55);
    if paint::chance(px, py, 47, 30) {
        c = GRASS.shadow;
    } else if paint::chance(px, py, 53, 24) {
        c = GRASS.light;
    }
    c
}

/// The flower beds along the two near edges: a row of edging stones against the lawn, then
/// turned soil and flowers, all low enough never to stand in front of anyone.
pub(super) fn beds(
    canvas: &mut Canvas,
    view: &View,
    w: f32,
    d: f32,
    flowers: &mut Vec<(i32, i32)>,
) {
    let n = NEAR_BORDER;
    let strips = [
        (w + 0.06, -FAR_BORDER + 0.5, w + n - 0.06, d + n - 0.06),
        (-FAR_BORDER + 0.5, d + 0.06, w + n - 0.06, d + n - 0.06),
    ];
    for r in strips {
        paint::fill_polygon(&quad(view, r, 0.0), |px, py| {
            let c = if paint::chance(px, py, 19, 40) {
                SOIL.light
            } else if paint::chance(px, py, 23, 30) {
                SOIL.shadow
            } else {
                SOIL.base
            };
            paint::put(canvas, px, py, c);
        });
    }
    // Edging stones, a row of them along each bed.
    let mut stones = Vec::new();
    let mut t = -FAR_BORDER + 0.55;
    while t < d + n - 0.1 {
        stones.push((w + 0.04, t));
        t += 0.17;
    }
    let mut t = -FAR_BORDER + 0.55;
    while t < w + n - 0.1 {
        stones.push((t, d + 0.04));
        t += 0.17;
    }
    for (k, (x, y)) in stones.into_iter().enumerate() {
        let (sx, sy) = view.pixel(x, y);
        let lift = (k % 2) as i32;
        paint::ellipse(canvas, sx, sy, 2, 1, STONE.shadow);
        paint::put(canvas, sx - 1, sy - 1 - lift, STONE.light);
        paint::put(canvas, sx, sy - 1 - lift, STONE.base);
        paint::put(canvas, sx + 1, sy - lift, STONE.base);
        paint::put(canvas, sx - 1, sy - 2 - lift, STONE.shine);
    }
    // Flowers along the middle of each bed.
    let mut k = 0u32;
    let mut place = |canvas: &mut Canvas, x: f32, y: f32| {
        let m = paint::noise(k as i32, 3, 97);
        let (sx, sy) = view.pixel(x, y);
        let bloom = BLOSSOMS[(m % 5) as usize];
        let h = 2 + (m >> 3) as i32 % 3;
        paint::vline(canvas, sx, sy - h, h, LEAF.base);
        paint::put(canvas, sx - 1, sy - 1, LEAF.light);
        paint::put(canvas, sx + 1, sy - 2, LEAF.shadow);
        paint::put(canvas, sx, sy - h - 1, bloom);
        paint::put(canvas, sx - 1, sy - h, bloom);
        paint::put(canvas, sx + 1, sy - h, paint::darker(bloom, 0.25));
        paint::put(canvas, sx, sy - h + 1, paint::darker(bloom, 0.35));
        paint::put(canvas, sx, sy - h, rgb(0xf5c94e));
        flowers.push((sx, sy - h));
        k += 1;
    };
    let mut t = -FAR_BORDER + 0.7;
    while t < d + n - 0.2 {
        let j = (paint::noise(t.to_bits() as i32, 1, 5) % 10) as f32 / 100.0;
        place(canvas, w + 0.18 + j, t);
        t += 0.31;
    }
    let mut t = -FAR_BORDER + 0.7;
    while t < w + n - 0.2 {
        let j = (paint::noise(t.to_bits() as i32, 2, 5) % 10) as f32 / 100.0;
        place(canvas, t, d + 0.18 + j);
        t += 0.31;
    }
}

/// One tile of the beacon's yard: flagstones, two by two with a jog in the joints, moss in the
/// joints, and a raised kerb wherever the yard meets the lawn.
pub(super) fn paving(canvas: &mut Canvas, arena: &Arena, view: &View, tile: Tile) {
    let (tx, ty) = (i32::from(tile.x), i32::from(tile.y));
    let n = paint::noise(tx, ty, 101);
    let su = 0.5 + ((n % 21) as f32 - 10.0) / 100.0;
    let sv = 0.5 + (((n >> 5) % 21) as f32 - 10.0) / 100.0;
    let open = |dx: i8, dy: i8| !arena.in_zone(Tile::new(tile.x + dx, tile.y + dy));
    let (west, north, east, south) = (open(-1, 0), open(0, -1), open(1, 0), open(0, 1));
    let span = f32::from(arena.width) + f32::from(arena.height);
    paint::fill_polygon(&view.diamond(tile, 0.0), |px, py| {
        let (fx, fy) = view.floor_at(px as f32 + 0.5, py as f32 + 0.5);
        let (ux, uy) = (fx - fx.floor(), fy - fy.floor());
        // The kerb.
        let kerb = [(west, ux), (north, uy), (east, 1.0 - ux), (south, 1.0 - uy)]
            .iter()
            .filter(|(edge, _)| *edge)
            .map(|(_, at)| *at)
            .fold(1.0f32, f32::min);
        let c = if kerb < 0.05 {
            STONE.shadow
        } else if kerb < 0.09 {
            if paint::chance(px, py, 3, 50) {
                STONE.base
            } else {
                STONE.light
            }
        } else {
            let right = ux > su;
            let low = uy > sv;
            let (u0, u1) = if right { (su, 1.0) } else { (0.0, su) };
            let (v0, v1) = if low { (sv, 1.0) } else { (0.0, sv) };
            let (du0, du1, dv0, dv1) = (ux - u0, u1 - ux, uy - v0, v1 - uy);
            if du0.min(du1) < 0.035 || dv0.min(dv1) < 0.04 {
                // Moss in the joints.
                if paint::chance(px, py, 7, 90) {
                    GRASS.light
                } else {
                    paint::mix(GRASS.shadow, SOIL.base, 0.35)
                }
            } else {
                let stone = paint::noise(tx * 2 + i32::from(right), ty * 2 + i32::from(low), 107);
                let mut c = paint::mix(STONE.base, STONE.light, (stone % 100) as f32 / 160.0);
                if stone.is_multiple_of(3) {
                    c = paint::mix(c, rgb(0xd8c49c), 0.22);
                }
                if du0 < 0.08 || dv0 < 0.07 {
                    c = paint::mix(c, STONE.shine, 0.55);
                } else if du1 < 0.08 || dv1 < 0.07 {
                    c = paint::mix(c, STONE.shadow, 0.45);
                }
                if paint::chance(px, py, 11, 20) {
                    c = paint::mix(c, STONE.shadow, 0.3);
                }
                c
            }
        };
        paint::put(canvas, px, py, lit(c, STONE, ((fy - fx) / span) * 0.16));
    });
}

/// A hedge on a board tile: a box of leaves whose edges are broken with lumps, a scatter of one
/// colour of flower, and a darker band at its foot.
pub(super) fn hedge(view: &View, tile: Tile) -> Sprite {
    let mut full = Canvas::new(view.size.0, view.size.1);
    let (x, y) = (f32::from(tile.x), f32::from(tile.y));
    let salt = 300 + (tile.x as u32) * 7 + (tile.y as u32) * 13;
    let r = (x + 0.06, y + 0.06, x + 0.94, y + 0.94);
    let low = quad(view, r, 0.0);
    let high = quad(view, r, HEDGE_HEIGHT);
    let foot = |face: Face, px: i32, py: i32| -> bool {
        let (a, b) = match face {
            Face::Left => (low[3], low[2]),
            Face::Right => (low[2], low[1]),
            Face::Top => return false,
        };
        let bottom = a.1 + (px as f32 + 0.5 - a.0) * (b.1 - a.1) / (b.0 - a.0);
        bottom - (py as f32 + 0.5) < 3.0
    };
    upright(&mut full, view, r, HEDGE_HEIGHT, |face, px, py| {
        let level = match face {
            Face::Top => 3,
            Face::Left => 2,
            Face::Right => 1,
        } - i32::from(foot(face, px, py));
        Some(foliage(px, py, salt, level))
    });
    // Lumps along its top edges and down its corners.
    for i in 0..4 {
        let (a, b) = (high[i], high[(i + 1) % 4]);
        let steps = ((b.0 - a.0).abs() / 5.0).ceil().max(1.0) as i32;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            let n = paint::noise(i as i32, s, salt);
            if n.is_multiple_of(4) {
                continue;
            }
            lump(
                &mut full,
                (p.0.round() as i32, p.1.round() as i32 - 1),
                2,
                salt,
                3,
            );
        }
    }
    for (corner, level) in [(3usize, 2), (2, 2), (1, 1)] {
        for up in [5.0, 11.0] {
            let p = (high[corner].0, high[corner].1 + HEDGE_HEIGHT - up);
            lump(
                &mut full,
                (p.0.round() as i32, p.1.round() as i32),
                2,
                salt + 1,
                level,
            );
        }
    }
    // Flowers on what faces the light.
    let bloom = BLOSSOMS[(salt % 5) as usize];
    let snapshot = full.clone();
    for py in 0..full.height() as i32 {
        for px in 0..full.width() as i32 {
            let c = snapshot.get(px, py);
            if c.a > 0 && (c == LEAF.light || c == LEAF.shine) && paint::chance(px, py, salt, 9) {
                full.set(px, py, bloom);
                paint::put(&mut full, px + 1, py + 1, paint::darker(bloom, 0.35));
            }
        }
    }
    paint::outline_inside(&mut full, |c| paint::mix(c, LEAF.edge, 0.7));
    Sprite::cut(&full)
}

/// A length of old brick garden wall with a stone coping, either side of the gate.
pub(super) fn wall(view: &View, tile: Tile, along_x: bool) -> Sprite {
    let (x, y) = (f32::from(tile.x), f32::from(tile.y));
    let r = if along_x {
        (x, y + 0.3, x + 1.0, y + 0.7)
    } else {
        (x + 0.3, y, x + 0.7, y + 1.0)
    };
    let low = quad(view, r, 0.0);
    let high = quad(view, r, WALL_HEIGHT);
    let mut brick = Canvas::new(view.size.0, view.size.1);
    upright(&mut brick, view, r, WALL_HEIGHT, |face, px, py| {
        // Courses three pixels deep, following the face's slope, each brick six across.
        let (a, slope, level) = match face {
            Face::Left => (high[3], 0.5, 2),
            Face::Right => (high[2], -0.5, 1),
            Face::Top => return Some(STONE.base),
        };
        let along = px - a.0.round() as i32;
        let down = py - (a.1 + along as f32 * slope).round() as i32;
        let row = down.div_euclid(3);
        let shift = if row % 2 == 0 { 0 } else { 3 };
        let b = (along + shift).div_euclid(6);
        let mortar = down.rem_euclid(3) == 0 || (along + shift).rem_euclid(6) == 0;
        let bottom = (low[3].1 - high[3].1) as i32;
        Some(if mortar {
            tone(MORTAR, level)
        } else if down > bottom - 3 && paint::chance(px, py, 5, 70) {
            LEAF.base
        } else {
            let n = paint::noise(b, row, 151 + level as u32);
            tone(
                BRICK,
                level + i32::from(n.is_multiple_of(5)) - i32::from(n.is_multiple_of(7)),
            )
        })
    });
    // The coping, a little proud of the brick, painted on the floor and lifted onto the wall.
    let pad = 0.04;
    let top = (r.0 - pad, r.1 - pad, r.2 + pad, r.3 + pad);
    let mut coping = Canvas::new(view.size.0, view.size.1);
    upright(&mut coping, view, top, 2.0, |face, px, py| {
        Some(match face {
            Face::Top => {
                if paint::chance(px, py, 17, 40) {
                    STONE.shine
                } else {
                    STONE.light
                }
            }
            Face::Left => STONE.base,
            Face::Right => STONE.shadow,
        })
    });
    paint::blit(&mut brick, &coping, 0, -(WALL_HEIGHT as i32));
    // A trail of ivy down one face.
    let start = high[3];
    let mut p = (start.0.round() as i32 + 4, start.1.round() as i32 + 3);
    for k in 0..9 {
        paint::put(
            &mut brick,
            p.0,
            p.1,
            if k % 3 == 0 { LEAF.light } else { LEAF.base },
        );
        if k % 2 == 0 {
            paint::put(&mut brick, p.0 - 1, p.1, LEAF.shadow);
        }
        p = (p.0 + i32::from(k % 3 == 1), p.1 + 1);
    }
    edged(&mut brick, 0.5);
    Sprite::cut(&brick)
}

/// The back of the garden: a tree in the far corner, a hedgerow along both far edges (broken
/// behind a gate in the board's far lane, where stepping stones lead out), and a potting shed at
/// the right-hand end. Returns where light can move in the tree's leaves, and where the robin
/// can perch.
pub(super) fn back(
    canvas: &mut Canvas,
    view: &View,
    w: f32,
    d: f32,
    gaps: &[f32],
) -> (Vec<(i32, i32)>, Vec<Perch>) {
    let f = FAR_BORDER;
    let mut layer = Canvas::new(canvas.width(), canvas.height());
    // The tree stands in the crook of the far corner, where the two hedgerows meet, and its
    // shade falls dappled on the grass.
    let crook = -f + 0.12 + 0.55;
    let (tx, ty) = view.pixel(crook, crook);
    dappled(canvas, (tx + 14, ty + 6), 26, 10);
    // Stepping stones out through the gap behind a far gate.
    for &g in gaps {
        for (k, t) in [-0.2f32, -0.46, -0.7].iter().enumerate() {
            let (sx, sy) = view.pixel(g + 0.5 + (k as f32 - 1.0) * 0.06, *t);
            paint::ellipse(canvas, sx, sy + 1, 6, 3, STONE.shadow);
            paint::ellipse(canvas, sx, sy, 6, 3, STONE.base);
            paint::ellipse(canvas, sx - 1, sy - 1, 4, 2, STONE.light);
        }
    }

    // The hedgerow along both far edges.
    let thick = crook - (-f + 0.12);
    // The left run takes the whole corner and the right one starts where it ends, so neither is
    // painted over the other at the join.
    let mut runs = vec![(Face::Left, -f + 0.12, d - 1.4)];
    let mut from = crook;
    let mut sorted: Vec<f32> = gaps.to_vec();
    sorted.sort_by(f32::total_cmp);
    for g in sorted {
        runs.push((Face::Right, from, g - 0.15));
        from = g + 1.15;
    }
    runs.push((Face::Right, from, w - 2.1));
    let mut perches = Vec::new();
    for (k, (side, a, b)) in runs.into_iter().enumerate() {
        if b - a < 0.3 {
            continue;
        }
        let r = match side {
            Face::Left => (-f + 0.12, a, -f + 0.12 + thick, b),
            _ => (a, -f + 0.12, b, -f + 0.12 + thick),
        };
        let mut row = Canvas::new(canvas.width(), canvas.height());
        cast(canvas, view, r, HEDGEROW_HEIGHT);
        let salt = 500 + k as u32 * 17;
        let high = quad(view, r, HEDGEROW_HEIGHT);
        upright(&mut row, view, r, HEDGEROW_HEIGHT, |face, px, py| {
            let level = match face {
                Face::Top => 3,
                Face::Left => 2,
                Face::Right => 1,
            };
            Some(foliage(px, py, salt, level))
        });
        for i in 0..4 {
            let (p, q) = (high[i], high[(i + 1) % 4]);
            let steps = ((q.0 - p.0).abs() / 5.0).ceil().max(1.0) as i32;
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let pt = (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t);
                if paint::noise(i as i32, s, salt).is_multiple_of(3) {
                    continue;
                }
                lump(
                    &mut row,
                    (pt.0.round() as i32, pt.1.round() as i32 - 1),
                    3,
                    salt,
                    3,
                );
            }
        }
        let snapshot = row.clone();
        for py in 0..row.height() as i32 {
            for px in 0..row.width() as i32 {
                let c = snapshot.get(px, py);
                if c.a > 0
                    && (c == LEAF.light || c == LEAF.shine)
                    && paint::chance(px, py, salt, 12)
                {
                    let bloom = BLOSSOMS[(paint::noise(px / 9, py / 7, salt) % 5) as usize];
                    row.set(px, py, bloom);
                }
            }
        }
        edged(&mut row, 0.6);
        paint::blit(&mut layer, &row, 0, 0);
        if side == Face::Right {
            for t in [0.3f32, 0.65] {
                let along = a + (b - a) * t;
                let (sx, sy) = view.pixel(along, -f + 0.12 + thick * 0.5);
                perches.push((sx, sy - HEDGEROW_HEIGHT as i32 - 2));
            }
        }
    }

    // Over the hedgerows, so its crown covers the corner where they join.
    let leaves = oak(&mut layer, (tx, ty + 2), &CORNER_CROWN, 41);

    // The potting shed.
    let shed = (w - 2.0, -f + 0.05, w + NEAR_BORDER - 0.15, -0.2);
    cast(canvas, view, shed, SHED_WALL);
    shed_at(&mut layer, view, shed);
    paint::blit(canvas, &layer, 0, 0);
    (leaves, perches)
}

/// The Backyard's tree, its crown hanging low either side over the hedgerows' tops so the corner
/// where they join is hidden in its leaves.
const CORNER_CROWN: [(i32, i32, i32); 11] = [
    (-15, -34, 13),
    (14, -35, 13),
    (0, -45, 15),
    (-6, -27, 11),
    (9, -26, 11),
    (-20, -44, 10),
    (18, -46, 10),
    (-9, -55, 11),
    (8, -57, 11),
    (-19, -22, 8),
    (18, -21, 8),
];

/// A little potting shed of weathered boards under a tiled roof, its door and window towards the
/// board.
fn shed_at(canvas: &mut Canvas, view: &View, r: (f32, f32, f32, f32)) {
    let mut full = Canvas::new(canvas.width(), canvas.height());
    let (x0, y0, x1, y1) = r;
    upright(&mut full, view, r, SHED_WALL, |face, px, _| {
        let (ramp_level, board) = match face {
            Face::Left => (2, px.rem_euclid(4) == 0),
            Face::Right => (1, px.rem_euclid(4) == 0),
            Face::Top => return None,
        };
        let n = paint::noise(px / 4, 0, 61);
        let l = ramp_level + i32::from(n.is_multiple_of(4)) - i32::from(board);
        Some(paint::mix(tone(PLANK, l), rgb(0xa8a090), 0.18))
    });
    // The gable on its right-hand end, boarded like its walls.
    let ridge_y = (y0 + y1) / 2.0;
    let gable = [
        lifted(view, x1, y1, SHED_WALL),
        lifted(view, x1, y0, SHED_WALL),
        lifted(view, x1, ridge_y, SHED_RIDGE),
    ];
    paint::fill_polygon(&gable, |px, py| {
        let c = if px.rem_euclid(4) == 0 {
            PLANK.edge
        } else {
            PLANK.shadow
        };
        paint::put(&mut full, px, py, paint::mix(c, rgb(0xa8a090), 0.18));
    });
    // The roof, overhanging on every side.
    let o = 0.12;
    let roof = [
        lifted(view, x0 - o, ridge_y, SHED_RIDGE),
        lifted(view, x1 + o, ridge_y, SHED_RIDGE),
        lifted(view, x1 + o, y1 + o, SHED_WALL - 3.0),
        lifted(view, x0 - o, y1 + o, SHED_WALL - 3.0),
    ];
    let top = roof[0].1.min(roof[1].1);
    paint::fill_polygon(&roof, |px, py| {
        let (fx, _) = view.floor_at(px as f32 + 0.5, py as f32 + 0.5 + 0.0);
        let _ = fx;
        let row = ((py as f32 - top) / 3.0).floor() as i32;
        let shift = if row % 2 == 0 { 0 } else { 2 };
        let seam = (py as f32 - top).rem_euclid(3.0) < 1.0 || (px + shift).rem_euclid(5) == 0;
        let n = paint::noise((px + shift) / 5, row, 67);
        let c = if seam {
            ROOF.shadow
        } else if n.is_multiple_of(6) {
            ROOF.light
        } else {
            ROOF.base
        };
        paint::put(&mut full, px, py, c);
    });
    paint::line(
        &mut full,
        (roof[0].0.round() as i32, roof[0].1.round() as i32),
        (roof[1].0.round() as i32, roof[1].1.round() as i32),
        ROOF.shine,
    );
    // Door and window on the face towards the board.
    let door = [
        lifted(view, x0 + 0.35, y1, 17.0),
        lifted(view, x0 + 0.75, y1, 17.0),
        lifted(view, x0 + 0.75, y1, 0.0),
        lifted(view, x0 + 0.35, y1, 0.0),
    ];
    paint::polygon(&mut full, &door, PAINT.base);
    let knob = lifted(view, x0 + 0.68, y1, 8.0);
    paint::put(
        &mut full,
        knob.0.round() as i32,
        knob.1.round() as i32,
        BRASS.light,
    );
    let window = [
        lifted(view, x0 + 1.15, y1, 18.0),
        lifted(view, x0 + 1.75, y1, 18.0),
        lifted(view, x0 + 1.75, y1, 10.0),
        lifted(view, x0 + 1.15, y1, 10.0),
    ];
    paint::polygon(&mut full, &window, PLANK.light);
    let pane = [
        lifted(view, x0 + 1.2, y1, 17.0),
        lifted(view, x0 + 1.7, y1, 17.0),
        lifted(view, x0 + 1.7, y1, 11.0),
        lifted(view, x0 + 1.2, y1, 11.0),
    ];
    paint::fill_polygon(&pane, |px, py| {
        let c = if (px + py) % 7 == 0 {
            rgb(0xe9f6f7)
        } else {
            rgb(0x9cc8d4)
        };
        paint::put(&mut full, px, py, c);
    });
    let mid = lifted(view, x0 + 1.45, y1, 17.0);
    paint::vline(
        &mut full,
        mid.0.round() as i32,
        mid.1.round() as i32,
        7,
        PLANK.light,
    );
    // A flower box under the window.
    for k in 0..6 {
        let p = lifted(view, x0 + 1.18 + k as f32 * 0.1, y1 + 0.02, 10.0);
        let (px, py) = (p.0.round() as i32, p.1.round() as i32);
        paint::put(&mut full, px, py, PLANK.shadow);
        paint::put(&mut full, px, py - 1, LEAF.base);
        paint::put(&mut full, px, py - 2, BLOSSOMS[k % 5]);
    }
    edged(&mut full, 0.5);
    paint::blit(canvas, &full, 0, 0);
}

/// A stone bird bath by the board's left corner, with a pot of flowers beside it. Returns where
/// its water catches the light.
pub(super) fn bird_bath(canvas: &mut Canvas, view: &View, d: f32) -> (i32, i32) {
    let mut full = Canvas::new(canvas.width(), canvas.height());
    let (cx, cy) = view.pixel(-0.5, d - 0.75);
    paint::ellipse(canvas, cx + 3, cy + 1, 7, 3, SHADOW);
    paint::ellipse(&mut full, cx, cy, 4, 2, STONE.shadow);
    paint::rect(&mut full, cx - 2, cy - 11, 5, 11, STONE.base);
    paint::vline(&mut full, cx - 2, cy - 11, 11, STONE.light);
    paint::vline(&mut full, cx + 2, cy - 11, 11, STONE.shadow);
    paint::ellipse(&mut full, cx, cy - 13, 8, 3, STONE.base);
    paint::ellipse(&mut full, cx, cy - 14, 8, 3, STONE.light);
    paint::ellipse(&mut full, cx, cy - 14, 6, 2, WATER.base);
    paint::hline(&mut full, cx - 4, cy - 15, 3, WATER.light);
    // A pot of flowers.
    let (px, py) = view.pixel(-0.62, d - 0.1);
    paint::ellipse(canvas, px + 2, py + 1, 5, 2, SHADOW);
    paint::rect(&mut full, px - 3, py - 6, 7, 6, rgb(0xb5543f));
    paint::vline(&mut full, px - 3, py - 6, 6, rgb(0xd07a5c));
    paint::hline(&mut full, px - 4, py - 7, 9, rgb(0xd88a68));
    for (k, (dx, dy)) in [(-2, -9), (0, -11), (2, -9), (-1, -12), (1, -8)]
        .iter()
        .enumerate()
    {
        paint::put(&mut full, px + dx, py + dy + 2, LEAF.base);
        paint::put(&mut full, px + dx, py + dy, BLOSSOMS[(k + 1) % 5]);
    }
    edged(&mut full, 0.5);
    paint::blit(canvas, &full, 0, 0);
    (cx - 1, cy - 14)
}

/// A string of bunting from the tree to the shed, its flags stirring.
pub(super) fn bunting(
    canvas: &mut Canvas,
    view: &View,
    ends: [(f32, f32); 2],
    t: f32,
    still: bool,
) {
    let a = lifted(view, ends[0].0, ends[0].1, 40.0);
    let b = lifted(view, ends[1].0, ends[1].1, SHED_WALL + 2.0);
    let length = (b.0 - a.0).abs().max(1.0);
    let steps = length as i32;
    let sag = 9.0;
    let at = |s: f32| {
        (
            a.0 + (b.0 - a.0) * s,
            a.1 + (b.1 - a.1) * s + sag * (s * PI).sin(),
        )
    };
    for i in 0..=steps {
        let p = at(i as f32 / steps as f32);
        paint::put(canvas, p.0.round() as i32, p.1.round() as i32, PLANK.shadow);
    }
    let flags = (length / 8.0) as i32;
    for k in 1..flags {
        let s = k as f32 / flags as f32;
        let p = at(s);
        let (x, y) = (p.0.round() as i32, p.1.round() as i32 + 1);
        let stir = if still {
            0
        } else {
            ((t * 2.6 + k as f32 * 0.9).sin() * 1.2).round() as i32
        };
        let colour = BLOSSOMS[(k as usize) % 5];
        for row in 0..5 {
            let half = 2 - row / 2;
            let shift = if row >= 3 { stir } else { 0 };
            paint::hline(canvas, x - half + shift, y + row, half * 2 + 1, colour);
        }
        paint::put(canvas, x - 2, y, paint::mix(colour, rgb(0xffffff), 0.4));
        paint::put(canvas, x + 2, y + 1, paint::darker(colour, 0.3));
    }
}
