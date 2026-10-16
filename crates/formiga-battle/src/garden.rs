//! The Backyard as a place: the lawn, the paved yard round the beacon, the hedges and the garden
//! wall, and the garden the board sits in. Painted to Formiga Hill's and Home's rules: every
//! material a ramp, light from the upper left, shadows falling down and to the right, edges in a
//! darker shade of their own colour (never black, so the creatures' near-black outlines read
//! first), grain from `paint::noise`.
//!
//! Everything that never changes is painted once, into `Garden::ground` and the standing pieces'
//! sprites; only what moves (the beacon's flame, the bunting, the robin, the butterflies) is
//! painted every frame. Nothing here decides where anyone can walk: the board's tiles keep their
//! centres clear of clutter, so whoever stands on one is never lost in the grass.

use crate::iso::{FAR_BORDER, NEAR_BORDER, SLAB, View};
use formiga_art::paint::{self, Ramp, rgb, rgba};
use formiga_art::{Canvas, Rgba};
use formiga_battle_core::arena::Terrain;
use formiga_battle_core::{Arena, Tile};
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

mod cove;
mod millpond;
mod woodland;

/// Hill's grass, and the two stripes its lawns are mown in.
pub const GRASS: Ramp = Ramp::new(0x2f5a2e, 0x447a3c, 0x5e9450, 0x7db36c, 0x9fd08a);
const LAWN_LIGHT: Rgba = rgb(0x8cc178);
const LAWN_DARK: Rgba = rgb(0x7fb46d);
pub const LEAF: Ramp = Ramp::new(0x2c5233, 0x3d6e45, 0x518a55, 0x6fa866, 0x93c67e);
pub const STONE: Ramp = Ramp::new(0x605a5d, 0x857d7c, 0xa69d94, 0xc2baae, 0xdcd5c8);
pub const PLANK: Ramp = Ramp::new(0x6f4f37, 0x9b734d, 0xbb9061, 0xd0a874, 0xe2c08c);
const BARK: Ramp = Ramp::new(0x36261b, 0x553b2a, 0x70503a, 0x8b6a50, 0xa88a6c);
const BRICK: Ramp = Ramp::new(0x6a3328, 0x8f4636, 0xab5a42, 0xc4775a, 0xdc9c7c);
const MORTAR: Ramp = Ramp::new(0x8a7c6a, 0xb0a28c, 0xcabca4, 0xddd1ba, 0xeee6d2);
const SOIL: Ramp = Ramp::new(0x3e2818, 0x5c3c26, 0x7a5236, 0x966a48, 0xb48a64);
const ROOF: Ramp = Ramp::new(0x6e2a22, 0x96392c, 0xb84e38, 0xd06a4c, 0xe48e6c);
/// The shed door's paint: the green of Home's front door.
const PAINT: Ramp = Ramp::new(0x2f4a3c, 0x46705a, 0x5c8f74, 0x7aae92, 0xa2cdb4);
/// The gate's paint: Hill's white pickets.
const PICKET: Ramp = Ramp::new(0x7a7466, 0xb8b0a0, 0xe2dccc, 0xf4f0e4, 0xffffff);
pub const BRASS: Ramp = Ramp::new(0x6e4f1c, 0xa47a2c, 0xcf9f3e, 0xe8c25a, 0xfbe6a2);
const IRON: Ramp = Ramp::new(0x2e2a30, 0x48434a, 0x67616a, 0x8a848c, 0xb0aab2);
const WATER: Ramp = Ramp::new(0x3c6f86, 0x5a94aa, 0x7fb8c8, 0xa8d6df, 0xdff3f2);
/// Hill's flowers.
const BLOSSOMS: [Rgba; 5] = [
    rgb(0xe0605a),
    rgb(0xf19bb0),
    rgb(0xf5d25e),
    rgb(0xfbf6ee),
    rgb(0xb79be0),
];
/// A shadow on grass: cool and a little green, as Hill casts them.
const SHADOW: Rgba = rgba(0x24452b, 60);

/// Which place an arena is drawn as. It is the picture's alone, chosen by the arena's id, so an
/// arena file stays the rules' business and says nothing about how it looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dressing {
    /// The Backyard: a garden of lawn, hedges, a brick wall and a potting shed.
    Garden,
    /// A clearing in the woods: moss, stumps and boulders, ferns, and a ring of stones.
    Woodland,
    /// A meadow by a watermill: the pond on the board, a footbridge, and a jetty.
    Millpond,
    /// A beach at low tide: sand, sea-worn rocks, a sandbar, the sea and a lighthouse.
    Cove,
}

impl Dressing {
    /// What the place is like, in a line, for choosing where to play.
    pub const fn blurb(self) -> &'static str {
        match self {
            Self::Garden => "Hedges, a garden wall, and a gate the lever swings.",
            Self::Woodland => "Old stumps and boulders cut the glade into lanes. No gate.",
            Self::Millpond => "A jetty between the pools, and a footbridge the winch lowers.",
            Self::Cove => "The beacon on a long sandbar between the rocks, open on both sides.",
        }
    }

    pub fn of(arena: &Arena) -> Self {
        match arena.id.as_str() {
            "woodland" => Self::Woodland,
            "millpond" => Self::Millpond,
            "cove" => Self::Cove,
            _ => Self::Garden,
        }
    }
}

/// How tall each standing thing is, in pixels.
const HEDGE_HEIGHT: f32 = 17.0;
const WALL_HEIGHT: f32 = 14.0;
const HEDGEROW_HEIGHT: f32 = 21.0;
const SHED_WALL: f32 = 25.0;
const SHED_RIDGE: f32 = 37.0;

/// A picture and where its top-left corner goes on the board's canvas.
#[derive(Clone, Debug)]
pub struct Sprite {
    pub pic: Canvas,
    pub x: i32,
    pub y: i32,
}

impl Sprite {
    /// The painted part of a whole-board canvas, cut out.
    fn cut(full: &Canvas) -> Self {
        let Some((x0, y0, x1, y1)) = full.alpha_bounds() else {
            return Self {
                pic: Canvas::new(1, 1),
                x: 0,
                y: 0,
            };
        };
        let mut pic = Canvas::new(x1 - x0 + 1, y1 - y0 + 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                pic.set(
                    (x - x0) as i32,
                    (y - y0) as i32,
                    full.get(x as i32, y as i32),
                );
            }
        }
        Self {
            pic,
            x: x0 as i32,
            y: y0 as i32,
        }
    }

    /// Its box on the canvas: left, top, right, bottom.
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        (
            self.x,
            self.y,
            self.x + self.pic.width() as i32,
            self.y + self.pic.height() as i32,
        )
    }
}

/// One spot on the hedgerow the robin hops between.
type Perch = (i32, i32);
/// Points on the canvas: where light moves in leaves, where leaves fall from.
type Points = Vec<(i32, i32)>;

/// The garden, painted once for an arena.
pub struct Garden {
    /// The ground, the yard's paving, the back of the garden and its shadows: everything behind
    /// whoever stands on the board.
    pub ground: Canvas,
    /// Each hedge and length of wall on the board, to be drawn in turn with the creatures.
    pub pieces: HashMap<Tile, Sprite>,
    /// Each gate, shut and then open.
    pub gates: HashMap<Tile, [Sprite; 2]>,
    dressing: Dressing,
    perches: Vec<Perch>,
    leaves: Vec<(i32, i32)>,
    /// Where butterflies (or dragonflies, by the pond) come back to.
    flowers: Vec<(i32, i32)>,
    bunting: Option<[(f32, f32); 2]>,
    bath: Option<(i32, i32)>,
    /// Woodland: where leaves fall from.
    drops: Vec<(i32, i32)>,
    /// Millpond: the tiles under water, the run the ducks paddle along, and the mill's wheel.
    water: Vec<Tile>,
    paddle: Option<((f32, f32), (f32, f32))>,
    wheel: Option<(i32, i32)>,
    /// Sandy Cove: the lighthouse's lamp, and the strip of sea the waves roll along.
    lamp: Option<(i32, i32)>,
    sea: Option<[(f32, f32); 2]>,
    /// The board's size, in tiles.
    size: (f32, f32),
}

impl Garden {
    pub fn new(arena: &Arena, view: &View) -> Self {
        let dressing = Dressing::of(arena);
        let mut ground = Canvas::new(view.size.0, view.size.1);
        let (w, d) = (f32::from(arena.width), f32::from(arena.height));
        // By the pond, what stands in the way is water, and the gate a footbridge over it.
        let water: Vec<Tile> = if dressing == Dressing::Millpond {
            arena
                .tiles()
                .filter(|t| matches!(arena.terrain(*t), Terrain::Obstacle | Terrain::Gate))
                .collect()
        } else {
            Vec::new()
        };
        let mut flowers = Vec::new();
        slab(&mut ground, view, w, d);
        floor(&mut ground, arena, view, dressing, &water);
        match dressing {
            Dressing::Garden => beds(&mut ground, view, w, d, &mut flowers),
            Dressing::Woodland => woodland::beds(&mut ground, view, w, d, &mut flowers),
            Dressing::Millpond => millpond::beds(&mut ground, view, w, d, &mut flowers),
            Dressing::Cove => cove::beds(&mut ground, view, w, d, &mut flowers),
        }
        if dressing == Dressing::Millpond {
            millpond::water(&mut ground, arena, view, &water);
        }
        for tile in arena.tiles().filter(|t| arena.in_zone(*t)) {
            match dressing {
                Dressing::Garden => paving(&mut ground, arena, view, tile),
                Dressing::Woodland => woodland::earth(&mut ground, arena, view, tile),
                Dressing::Millpond => millpond::deck(&mut ground, arena, view, tile, &water),
                Dressing::Cove => cove::bar(&mut ground, arena, view, tile),
            }
        }
        beacon_bowl(&mut ground, view, arena.zone_centre());
        if matches!(dressing, Dressing::Woodland | Dressing::Cove) {
            woodland::fire_ring(&mut ground, view, arena.zone_centre());
        }
        // Shadows, before anything that casts them.
        for tile in arena.tiles() {
            let height = match (dressing, arena.terrain(tile)) {
                (Dressing::Millpond, _) => continue,
                (Dressing::Garden, Terrain::Obstacle) => HEDGE_HEIGHT,
                (Dressing::Woodland, Terrain::Obstacle) => 12.0,
                (Dressing::Cove, Terrain::Obstacle) => 14.0,
                (_, Terrain::Gate) => 10.0,
                _ => continue,
            };
            let (x, y) = (f32::from(tile.x), f32::from(tile.y));
            cast(
                &mut ground,
                view,
                (x + 0.05, y + 0.05, x + 0.95, y + 0.95),
                height,
            );
        }

        // The back of the place, furthest first.
        let (mut bunting, mut bath, mut drops, mut wheel) = (None, None, Vec::new(), None);
        let (mut lamp, mut sea) = (None, None);
        let (leaves, perches) = match dressing {
            Dressing::Garden => {
                let gaps = arena
                    .gates
                    .iter()
                    .filter(|g| g.y == 0)
                    .map(|g| f32::from(g.x))
                    .collect::<Vec<_>>();
                let back = back(&mut ground, view, w, d, &gaps);
                bunting = Some([(-0.3, -0.3), (w - 2.0, -FAR_BORDER + 0.05)]);
                bath = Some(bird_bath(&mut ground, view, d));
                back
            }
            Dressing::Woodland => {
                let (leaves, perches, falling) = woodland::back(&mut ground, view, w, d);
                drops = falling;
                (leaves, perches)
            }
            Dressing::Millpond => {
                let (leaves, perches, turning) = millpond::back(&mut ground, view, w, d);
                wheel = Some(turning);
                (leaves, perches)
            }
            Dressing::Cove => {
                let (light, perches, strip) = cove::back(&mut ground, view, w, d);
                lamp = light.first().copied();
                sea = Some(strip);
                (Vec::new(), perches)
            }
        };

        let mut pieces = HashMap::new();
        let mut gates = HashMap::new();
        for tile in arena.tiles() {
            match (dressing, arena.terrain(tile)) {
                (Dressing::Garden, Terrain::Obstacle) => {
                    let beside_gate = arena.gates.iter().any(|g| {
                        (i32::from(g.x) - i32::from(tile.x)).abs()
                            + (i32::from(g.y) - i32::from(tile.y)).abs()
                            == 1
                    });
                    let sprite = if beside_gate {
                        let along_x = arena.gates.iter().any(|g| g.y == tile.y);
                        wall(view, tile, along_x)
                    } else {
                        hedge(view, tile)
                    };
                    pieces.insert(tile, sprite);
                }
                (Dressing::Woodland, Terrain::Obstacle) => {
                    pieces.insert(tile, woodland::piece(view, tile));
                }
                (Dressing::Cove, Terrain::Obstacle) => {
                    pieces.insert(tile, cove::rock(view, tile));
                }
                (Dressing::Millpond, Terrain::Gate) => {
                    gates.insert(
                        tile,
                        [
                            millpond::bridge(view, tile, false),
                            millpond::bridge(view, tile, true),
                        ],
                    );
                }
                (_, Terrain::Gate) => {
                    let along_x = [-1i8, 1].iter().any(|dx| {
                        arena.terrain(Tile::new(tile.x + dx, tile.y)) == Terrain::Obstacle
                    });
                    gates.insert(
                        tile,
                        [
                            gate(view, tile, along_x, false),
                            gate(view, tile, along_x, true),
                        ],
                    );
                }
                _ => {}
            }
        }
        let paddle = millpond::paddle(&water);
        Self {
            ground,
            pieces,
            gates,
            dressing,
            perches,
            leaves,
            flowers,
            bunting,
            bath,
            drops,
            water,
            paddle,
            wheel,
            lamp,
            sea,
            size: (w, d),
        }
    }

    /// What moves at the back of the place: light in the leaves, the bunting, the robin, the
    /// water and its ducks, the mill's wheel, falling leaves. Still pictures and reduced motion
    /// hold it all where it rests.
    pub fn behind(&self, canvas: &mut Canvas, view: &View, now: f32, still: bool) {
        let t = if still { 0.0 } else { now };
        let beat = (t * 1.5).floor() as i32;
        for (k, &(x, y)) in self.leaves.iter().enumerate() {
            if paint::noise(k as i32, beat, 71).is_multiple_of(3) {
                paint::put(canvas, x, y, LEAF.shine);
            }
        }
        if let Some(bath) = self.bath {
            let glint = if still {
                0
            } else {
                ((t * 0.8).sin() * 2.0).round() as i32
            };
            paint::put(canvas, bath.0 + glint, bath.1, WATER.shine);
            paint::put(canvas, bath.0 + glint + 1, bath.1, rgba(0xffffff, 160));
        }
        if let Some(ends) = self.bunting {
            bunting(canvas, view, ends, t, still);
        }
        match self.dressing {
            Dressing::Woodland => woodland::falling(canvas, &self.drops, t, still),
            Dressing::Millpond => {
                millpond::shimmer(canvas, view, &self.water, t, still);
                if let Some(path) = self.paddle {
                    millpond::ducks(canvas, view, path, t, still);
                }
                if let Some(wheel) = self.wheel {
                    millpond::wheel(canvas, wheel, t, still);
                }
            }
            Dressing::Cove => {
                if let Some(strip) = self.sea {
                    cove::waves(canvas, view, strip, t, still);
                }
                // The lighthouse's lamp, brightening as its beam comes round.
                if let Some((x, y)) = self.lamp {
                    let beam = if still { 0.5 } else { (t * 1.2).sin().max(0.0) };
                    let glow = (beam * 150.0) as u8;
                    paint::ellipse(canvas, x, y, 6, 4, rgba(0xffe9a0, glow / 2));
                    paint::ellipse(canvas, x, y, 3, 2, rgba(0xffffff, glow));
                }
                cove::gulls(canvas, view, t, still);
            }
            Dressing::Garden => {}
        }
        robin(canvas, &self.perches, t, still);
    }

    /// What moves in front of everything: butterflies over the beds, or dragonflies by the pond.
    pub fn over(&self, canvas: &mut Canvas, view: &View, now: f32, still: bool) {
        if self.dressing == Dressing::Cove {
            cove::crab(canvas, view, self.size.0, self.size.1, now, still);
            return;
        }
        if self.flowers.is_empty() {
            return;
        }
        let colours = match self.dressing {
            Dressing::Garden => [
                (rgb(0xf6e08a), rgb(0xb89a3a)),
                (rgb(0xfbf6ee), rgb(0xa8a2b0)),
                (rgb(0xa9c8f0), rgb(0x5a78a8)),
            ],
            Dressing::Woodland => [
                (rgb(0xe8b860), rgb(0x9a6a2a)),
                (rgb(0xfbf6ee), rgb(0xa8a2b0)),
                (rgb(0xd8a0c8), rgb(0x8a5a80)),
            ],
            Dressing::Millpond | Dressing::Cove => [
                (rgb(0x9fe0f0), rgb(0x2a7a9a)),
                (rgb(0xb8f0c0), rgb(0x3a8a5a)),
                (rgb(0xf6e08a), rgb(0xb89a3a)),
            ],
        };
        for (k, (wing, edge)) in colours.iter().enumerate() {
            let home = self.flowers[(k * 7 + 3) % self.flowers.len()];
            let (x, y, open) = if still {
                (home.0, home.1 - 4, false)
            } else {
                let s = now * 0.55 + k as f32 * 2.1;
                let x = home.0 as f32 + (s.sin() * 16.0) + (s * 2.3).sin() * 4.0;
                let y = home.1 as f32 - 9.0 + (s * 1.3).cos() * 5.0 + (now * 5.0 + k as f32).sin();
                (
                    x.round() as i32,
                    y.round() as i32,
                    ((now * 9.0 + k as f32) as i32) % 2 == 0,
                )
            };
            if self.dressing == Dressing::Millpond && k < 2 {
                millpond::dragonfly(canvas, (x, y), *wing, *edge, open);
            } else {
                butterfly(canvas, (x, y), *wing, *edge, open);
            }
        }
    }

    /// The lever, as this place has it: on a brick plinth in the garden, a winch post by the pond.
    pub fn lever(
        &self,
        canvas: &mut Canvas,
        view: &View,
        tile: Tile,
        swung: bool,
        pending: bool,
        now: f32,
    ) {
        lever(
            canvas,
            view,
            tile,
            swung,
            pending,
            now,
            self.dressing == Dressing::Millpond,
        );
    }
}

fn tone(ramp: Ramp, level: i32) -> Rgba {
    match level.clamp(0, 4) {
        0 => ramp.edge,
        1 => ramp.shadow,
        2 => ramp.base,
        3 => ramp.light,
        _ => ramp.shine,
    }
}

/// Smooth value noise in 0..1, over cells `cell` pixels across: broad patches, not speckle.
fn patches(x: i32, y: i32, cell: (i32, i32), salt: u32) -> f32 {
    let (cx, cy) = (x.div_euclid(cell.0), y.div_euclid(cell.1));
    let (fx, fy) = (
        x.rem_euclid(cell.0) as f32 / cell.0 as f32,
        y.rem_euclid(cell.1) as f32 / cell.1 as f32,
    );
    let n = |i: i32, j: i32| (paint::noise(i, j, salt) % 1000) as f32 / 999.0;
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(fx), s(fy));
    let top = n(cx, cy) + (n(cx + 1, cy) - n(cx, cy)) * sx;
    let bottom = n(cx, cy + 1) + (n(cx + 1, cy + 1) - n(cx, cy + 1)) * sx;
    top + (bottom - top) * sy
}

/// Lit from the upper left, across the whole garden.
fn lit(color: Rgba, ramp: Ramp, light: f32) -> Rgba {
    if light > 0.0 {
        paint::mix(color, ramp.shine, light)
    } else {
        paint::mix(color, ramp.shadow, -light)
    }
}

/// A rectangle of floor, `(x0, y0, x1, y1)` in tiles, on screen: far corner first and clockwise.
fn quad(view: &View, r: (f32, f32, f32, f32), lift: f32) -> [(f32, f32); 4] {
    let (x0, y0, x1, y1) = r;
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| {
        let (sx, sy) = view.screen(x, y);
        (sx, sy - lift)
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Face {
    Top,
    Left,
    Right,
}

/// An upright box on the floor, `height` pixels tall, each visible face's pixels coloured by
/// `color`: the left face is the one half in the light, the right one in shade.
fn upright(
    canvas: &mut Canvas,
    view: &View,
    r: (f32, f32, f32, f32),
    height: f32,
    mut color: impl FnMut(Face, i32, i32) -> Option<Rgba>,
) {
    let low = quad(view, r, 0.0);
    let high = quad(view, r, height);
    let left = [high[3], high[2], low[2], low[3]];
    let right = [high[2], high[1], low[1], low[2]];
    for (face, points) in [
        (Face::Left, &left[..]),
        (Face::Right, &right[..]),
        (Face::Top, &high[..]),
    ] {
        paint::fill_polygon(points, |px, py| {
            if let Some(c) = color(face, px, py) {
                paint::put(canvas, px, py, c);
            }
        });
    }
}

/// Leaves in clumps on a staggered lattice, each clump lit on its upper left and shaded under:
/// Hill's way with a hedge or a crown. `level` is the ramp step the clumps sit around.
fn foliage(px: i32, py: i32, salt: u32, level: i32) -> Rgba {
    let row = py.div_euclid(4);
    let shift = if row % 2 == 0 { 0 } else { 2 };
    let col = (px + shift).div_euclid(5);
    let n = paint::noise(col, row, salt);
    let cx = col * 5 - shift + 2 + (n % 3) as i32 - 1;
    let cy = row * 4 + 2 + ((n >> 4) % 2) as i32;
    let (dx, dy) = (px - cx, py - cy);
    let mut l = level - i32::from((n >> 8).is_multiple_of(5));
    if dx + dy <= -2 {
        l += 1;
    } else if dx + dy >= 3 {
        l -= 1;
    }
    tone(LEAF, l)
}

/// A soft lump of leaves for breaking a hedge's straight edges.
fn lump(canvas: &mut Canvas, (cx, cy): (i32, i32), r: i32, salt: u32, level: i32) {
    for dy in -r..=r {
        for dx in -r - 1..=r + 1 {
            if dx * dx * 2 + dy * dy * 3 > r * r * 3 {
                continue;
            }
            paint::put(
                canvas,
                cx + dx,
                cy + dy,
                foliage(cx + dx, cy + dy, salt, level),
            );
        }
    }
}

/// Gives a sprite painted on a whole-board canvas its edge in a darker shade of its own colours.
fn edged(full: &mut Canvas, amount: f32) {
    paint::outline_inside(full, |c| paint::darker(c, amount));
}

/// A shadow on the ground under a standing thing, falling down and to the right.
fn cast(canvas: &mut Canvas, view: &View, r: (f32, f32, f32, f32), height: f32) {
    let reach = height / 24.0 * 0.55;
    let (x0, y0, x1, y1) = r;
    let points = [
        view.screen(x0, y0),
        view.screen(x1 + reach, y0),
        view.screen(x1 + reach, y1),
        view.screen(x0, y1),
    ];
    paint::fill_polygon(&points, |px, py| {
        // Lightest at its far end, as a shadow softens away from what casts it.
        let (fx, _) = view.floor_at(px as f32 + 0.5, py as f32 + 0.5);
        let fade = ((fx - x1) / reach.max(0.01)).clamp(0.0, 1.0);
        if fade > 0.6 && paint::chance(px, py, 5, 128) {
            return;
        }
        paint::put(
            canvas,
            px,
            py,
            paint::faded(SHADOW, (60.0 * (1.0 - fade * 0.5)) as u8),
        );
    });
}

/// The garden's cross-section along its two near edges, and its shadow on the page: topsoil,
/// then subsoil with stones and roots in it, under a lip of grass.
fn slab(canvas: &mut Canvas, view: &View, w: f32, d: f32) {
    let (f, n) = (FAR_BORDER, NEAR_BORDER);
    let left = view.screen(-f, d + n);
    let near = view.screen(w + n, d + n);
    let right = view.screen(w + n, -f);
    let deep = SLAB as f32;
    let down = |p: (f32, f32), by: f32| (p.0, p.1 + by);
    // Its shadow on the page.
    let lift = |p: (f32, f32)| (p.0 + 3.0, p.1 + deep + 3.0);
    paint::polygon(
        canvas,
        &[
            down(left, deep),
            down(near, deep),
            down(right, deep),
            lift(right),
            lift(near),
            lift(left),
        ],
        rgba(0x5a4630, 46),
    );
    for (a, b, lit_face) in [(left, near, true), (near, right, false)] {
        let face = [a, b, down(b, deep), down(a, deep)];
        paint::fill_polygon(&face, |px, py| {
            let top = a.1 + (px as f32 + 0.5 - a.0) * (b.1 - a.1) / (b.0 - a.0);
            let depth = py as f32 + 0.5 - top;
            let c = if depth < 2.0 || (depth < 3.0 && paint::chance(px, 0, 33, 110)) {
                if lit_face { GRASS.base } else { GRASS.shadow }
            } else if depth < 5.0 {
                if paint::chance(px, py, 7, 30) {
                    SOIL.light
                } else {
                    SOIL.base
                }
            } else if paint::chance(px, py, 9, 22) {
                STONE.base
            } else if paint::chance(px, py, 13, 26) {
                SOIL.edge
            } else {
                SOIL.shadow
            };
            let c = if lit_face {
                c
            } else {
                paint::mix(c, SOIL.edge, 0.22)
            };
            paint::put(canvas, px, py, c);
        });
        paint::line(
            canvas,
            (a.0.round() as i32, (a.1 + deep).round() as i32 - 1),
            (b.0.round() as i32, (b.1 + deep).round() as i32 - 1),
            SOIL.edge,
        );
    }
}

/// The ground: the board's own ground (a mown lawn, moss, a meadow), and the rougher ground of
/// the place round it. Tiles under the yard or under water are left for their own painters.
fn floor(canvas: &mut Canvas, arena: &Arena, view: &View, dressing: Dressing, water: &[Tile]) {
    let (w, d) = (f32::from(arena.width), f32::from(arena.height));
    let ring = quad(
        view,
        (-FAR_BORDER, -FAR_BORDER, w + NEAR_BORDER, d + NEAR_BORDER),
        0.0,
    );
    let span = w + d;
    let bare = |tile: Tile| arena.in_zone(tile) || water.contains(&tile);
    paint::fill_polygon(&ring, |px, py| {
        let (fx, fy) = view.floor_at(px as f32 + 0.5, py as f32 + 0.5);
        let light = ((fy - fx) / span) * 0.2;
        let on_board = fx >= 0.0 && fy >= 0.0 && fx < w && fy < d;
        let c = if on_board {
            let tile = Tile::new(fx.floor() as i8, fy.floor() as i8);
            if bare(tile) {
                return;
            }
            match dressing {
                Dressing::Garden => lawn(px, py, fx, fy, tile),
                Dressing::Woodland => woodland::moss(px, py, fx, fy, tile),
                Dressing::Millpond => millpond::meadow(px, py, fx, fy, tile),
                Dressing::Cove => cove::sand(px, py, fx, fy, tile),
            }
        } else {
            match dressing {
                Dressing::Garden => rough(px, py),
                Dressing::Woodland => woodland::litter(px, py),
                Dressing::Millpond => millpond::long_grass(px, py),
                Dressing::Cove => cove::dunes(px, py),
            }
        };
        paint::put(canvas, px, py, lit(c, GRASS, light));
    });
    // Small things in the corners of the board's tiles only: nobody stands there.
    for tile in arena.tiles().filter(|t| !bare(*t)) {
        let n = paint::noise(i32::from(tile.x), i32::from(tile.y), 61);
        for k in 0..2u32 {
            let m = paint::noise(i32::from(tile.x) + k as i32 * 31, i32::from(tile.y), 67 + k);
            if m.is_multiple_of(3) {
                continue;
            }
            // A corner of the tile, pulled a little in.
            let corner = ((n >> (k * 2)) % 4) as usize;
            let (cu, cv) = [(0.18, 0.18), (0.82, 0.18), (0.82, 0.82), (0.18, 0.82)][corner];
            let jitter = |s: u32| ((m >> s) % 9) as f32 / 100.0 - 0.04;
            let (x, y) = view.pixel(
                f32::from(tile.x) + cu + jitter(3),
                f32::from(tile.y) + cv + jitter(7),
            );
            match dressing {
                Dressing::Garden if m.is_multiple_of(7) => daisy(canvas, x, y),
                Dressing::Garden => clover(canvas, x, y),
                Dressing::Woodland => woodland::corner(canvas, x, y, m),
                Dressing::Millpond => millpond::corner(canvas, x, y, m),
                Dressing::Cove => cove::corner(canvas, x, y, m),
            }
        }
    }
    // Tufts all through the rough.
    let mut k = 0;
    paint::fill_polygon(&ring, |px, py| {
        let (fx, fy) = view.floor_at(px as f32 + 0.5, py as f32 + 0.5);
        let on_board = fx >= -0.08 && fy >= -0.08 && fx < w + 0.08 && fy < d + 0.08;
        if !on_board && paint::chance(px, py, 89, 3) {
            k += 1;
            match dressing {
                Dressing::Woodland => woodland::sprig(canvas, px, py, k),
                Dressing::Cove => cove::marram(canvas, px, py, k),
                _ => tuft(canvas, px, py, k),
            }
        }
    });
    // The board's edge, cut clean against the rough.
    let edge = quad(view, (0.0, 0.0, w, d), 0.0);
    for i in 0..4 {
        let (a, b) = (edge[i], edge[(i + 1) % 4]);
        paint::line(
            canvas,
            (a.0.round() as i32, a.1.round() as i32),
            (b.0.round() as i32, b.1.round() as i32),
            paint::faded(GRASS.edge, 120),
        );
    }
}

/// One pixel of mown lawn: stripes a tile wide, broad patches of lighter and darker grass,
/// speckle, and a faint seam round every tile so the grid reads.
fn lawn(px: i32, py: i32, fx: f32, fy: f32, tile: Tile) -> Rgba {
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
fn rough(px: i32, py: i32) -> Rgba {
    let p = patches(px, py, (18, 10), 37);
    let mut c = paint::mix(GRASS.base, GRASS.light, p * 0.55);
    if paint::chance(px, py, 47, 30) {
        c = GRASS.shadow;
    } else if paint::chance(px, py, 53, 24) {
        c = GRASS.light;
    }
    c
}

fn tuft(canvas: &mut Canvas, x: i32, y: i32, k: i32) {
    let lean = (k % 3) - 1;
    for (dx, h) in [(-1, 2), (0, 3), (1, 2)] {
        paint::put(canvas, x + dx, y, GRASS.shadow);
        for up in 1..h {
            paint::put(
                canvas,
                x + dx + if up == h - 1 { lean } else { 0 },
                y - up,
                GRASS.light,
            );
        }
    }
    paint::put(canvas, x + lean, y - 3, GRASS.shine);
}

fn clover(canvas: &mut Canvas, x: i32, y: i32) {
    for (dx, dy) in [(0, -1), (-1, 0), (1, 0)] {
        paint::put(canvas, x + dx, y + dy, LEAF.light);
    }
    paint::put(canvas, x, y, LEAF.base);
    paint::put(canvas, x, y + 1, LEAF.shadow);
}

fn daisy(canvas: &mut Canvas, x: i32, y: i32) {
    paint::put(canvas, x, y, rgb(0xf5c94e));
    paint::put(canvas, x - 1, y, rgb(0xfbf6ee));
    paint::put(canvas, x, y - 1, rgb(0xfbf6ee));
    paint::put(canvas, x + 1, y, rgb(0xd9dbe0));
    paint::put(canvas, x, y + 1, rgb(0xd9dbe0));
}

/// The flower beds along the two near edges: a row of edging stones against the lawn, then
/// turned soil and flowers, all low enough never to stand in front of anyone.
fn beds(canvas: &mut Canvas, view: &View, w: f32, d: f32, flowers: &mut Vec<(i32, i32)>) {
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
fn paving(canvas: &mut Canvas, arena: &Arena, view: &View, tile: Tile) {
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

/// The beacon itself, set into the middle of its yard: a stone ring round a brass bowl of coals.
fn beacon_bowl(canvas: &mut Canvas, view: &View, centre: Tile) {
    let (cx, cy) = view.centre(centre);
    paint::ellipse(canvas, cx + 1, cy + 1, 11, 5, paint::faded(STONE.edge, 140));
    paint::ellipse(canvas, cx, cy, 11, 5, STONE.edge);
    paint::ellipse(canvas, cx, cy, 10, 4, STONE.light);
    for (dx, dy) in [(-8, -1), (-6, -2), (-3, -3), (0, -3)] {
        paint::put(canvas, cx + dx, cy + dy, STONE.shine);
    }
    for (dx, dy) in [(8, 1), (6, 2), (3, 3)] {
        paint::put(canvas, cx + dx, cy + dy, STONE.shadow);
    }
    paint::ellipse(canvas, cx, cy, 7, 3, BRASS.edge);
    paint::ellipse(canvas, cx, cy, 6, 2, BRASS.base);
    paint::hline(canvas, cx - 5, cy - 2, 4, BRASS.shine);
    paint::ellipse(canvas, cx, cy, 5, 2, rgb(0x3a2a24));
    for (dx, dy, c) in [
        (-3, 0, 0x6a3a2a),
        (-1, -1, 0x8a4a30),
        (1, 0, 0x5a3426),
        (3, -1, 0x7a4430),
        (0, 1, 0x4a2c22),
        (2, 1, 0x6a3a2a),
    ] {
        paint::put(canvas, cx + dx, cy + dy, rgb(c));
    }
}

/// The beacon's light: the yard warmed in the leading team's colour and a flame in its bowl, both
/// growing as they near lighting it. While the yard is held by both, the flame gutters between
/// the two colours under a wisp of smoke: nobody gains.
pub fn beacon_light(
    canvas: &mut Canvas,
    arena: &Arena,
    view: &View,
    lead: Option<(Ramp, f32)>,
    paused: Option<[Ramp; 2]>,
    now: f32,
    still: bool,
) {
    let (cx, cy) = view.centre(arena.zone_centre());
    let t = if still { 0.0 } else { now };
    if let Some((team, share)) = lead {
        let warmth = (18.0 + 52.0 * share) as u8;
        for tile in arena.tiles().filter(|t| arena.in_zone(*t)) {
            paint::polygon(
                canvas,
                &view.diamond(tile, 0.0),
                paint::faded(team.light, warmth / 2),
            );
        }
        let r = 12 + (share * 10.0) as i32;
        paint::ellipse(
            canvas,
            cx,
            cy,
            r,
            r / 2,
            paint::faded(team.shine, warmth / 2),
        );
    }
    let flame_of = |team: Ramp, size: f32, canvas: &mut Canvas| {
        let flicker = (t * 11.0).floor() as i32;
        for dx in -2i32..=2 {
            let n = paint::noise(dx, flicker, 211);
            let h = ((size * (3.0 - dx.abs() as f32 * 0.9)).max(0.0) + (n % 3) as f32 - 1.0).round()
                as i32;
            for up in 0..h.max(0) {
                let heat = up as f32 / h.max(1) as f32;
                let c = if heat < 0.35 {
                    rgb(0xfff1c4)
                } else if heat < 0.7 {
                    team.shine
                } else {
                    team.light
                };
                paint::put(canvas, cx + dx, cy - 1 - up, c);
            }
        }
    };
    if let Some([a, b]) = paused {
        let which = if still || (t * 3.0).sin() > 0.0 { a } else { b };
        flame_of(which, 0.9, canvas);
        // Smoke, curling up out of the bowl.
        for k in 0..3 {
            let age = if still {
                k as f32 / 3.0
            } else {
                (t * 0.5 + k as f32 / 3.0).fract()
            };
            let x = cx + ((age * TAU + k as f32).sin() * 2.0).round() as i32;
            let y = cy - 6 - (age * 14.0) as i32;
            let r = 1 + (age * 2.0) as i32;
            paint::ellipse(
                canvas,
                x,
                y,
                r,
                r,
                rgba(0xd8d4cc, (150.0 * (1.0 - age)) as u8),
            );
        }
    } else if let Some((team, share)) = lead {
        flame_of(team, 0.6 + share * 2.2, canvas);
        // Sparks rising, more of them the nearer it is.
        let count = (share * 4.0).ceil() as i32;
        for k in 0..count {
            let age = if still {
                0.5
            } else {
                (t * 0.7 + k as f32 * 0.37).fract()
            };
            let x = cx + ((k as f32 * 2.3 + age * 3.0).sin() * 4.0).round() as i32;
            let y = cy - 8 - (age * 16.0) as i32;
            paint::put(
                canvas,
                x,
                y,
                paint::faded(team.shine, (255.0 * (1.0 - age)) as u8),
            );
        }
    } else {
        // Banked coals, glowing faintly.
        if (t * 1.3).sin() > 0.2 || still {
            paint::put(canvas, cx - 1, cy - 1, rgb(0xd06a3a));
            paint::put(canvas, cx + 2, cy, rgb(0xb85a30));
        }
    }
}

/// A hedge on a board tile: a box of leaves whose edges are broken with lumps, a scatter of one
/// colour of flower, and a darker band at its foot.
fn hedge(view: &View, tile: Tile) -> Sprite {
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
fn wall(view: &View, tile: Tile, along_x: bool) -> Sprite {
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

/// The garden gate, between its two posts: shut, a row of pickets on two rails; open, swung back
/// towards the viewer against its hinge post.
fn gate(view: &View, tile: Tile, along_x: bool, open: bool) -> Sprite {
    let mut full = Canvas::new(view.size.0, view.size.1);
    let (x, y) = (f32::from(tile.x), f32::from(tile.y));
    let at = |a: f32, s: f32| {
        if along_x {
            view.pixel(x + a, y + s)
        } else {
            view.pixel(x + s, y + a)
        }
    };
    let hinge = at(0.08, 0.5);
    let latch = at(0.92, 0.5);
    let end = if open {
        if along_x {
            view.pixel(x + 0.08, y + 1.05)
        } else {
            view.pixel(x + 1.05, y + 0.08)
        }
    } else {
        latch
    };
    // Rails first, pickets over them.
    for lift in [4, 10] {
        paint::line(
            &mut full,
            (hinge.0, hinge.1 - lift),
            (end.0, end.1 - lift),
            PICKET.shadow,
        );
        paint::line(
            &mut full,
            (hinge.0, hinge.1 - lift - 1),
            (end.0, end.1 - lift - 1),
            PICKET.base,
        );
    }
    let count = 5;
    for k in 1..=count {
        let t = k as f32 / (count + 1) as f32;
        let px = (hinge.0 as f32 + (end.0 - hinge.0) as f32 * t).round() as i32;
        let py = (hinge.1 as f32 + (end.1 - hinge.1) as f32 * t).round() as i32;
        paint::vline(&mut full, px, py - 13, 13, PICKET.light);
        paint::vline(&mut full, px + 1, py - 13, 13, PICKET.base);
        paint::put(&mut full, px, py - 14, PICKET.shine);
        paint::put(&mut full, px, py - 13, PICKET.shine);
    }
    let post = |canvas: &mut Canvas, (px, py): (i32, i32)| {
        paint::rect(canvas, px - 1, py - 18, 4, 18, PLANK.base);
        paint::vline(canvas, px - 1, py - 18, 18, PLANK.light);
        paint::vline(canvas, px + 2, py - 18, 18, PLANK.shadow);
        paint::rect(canvas, px - 2, py - 20, 6, 2, PLANK.light);
        paint::hline(canvas, px - 2, py - 20, 6, PLANK.shine);
    };
    post(&mut full, hinge);
    post(&mut full, latch);
    // The latch, in iron.
    if !open {
        paint::rect(&mut full, latch.0 - 3, latch.1 - 9, 2, 2, IRON.light);
    }
    edged(&mut full, 0.5);
    Sprite::cut(&full)
}

/// The lever on its brick plinth: leaning one way while the gates are as they started, the other
/// once swung, its knob glowing while a swing is coming.
fn lever(
    canvas: &mut Canvas,
    view: &View,
    tile: Tile,
    swung: bool,
    pending: bool,
    now: f32,
    winch: bool,
) {
    let (x, y) = (f32::from(tile.x), f32::from(tile.y));
    let r = (x + 0.28, y + 0.28, x + 0.72, y + 0.72);
    cast(canvas, view, r, 6.0);
    let block = if winch { PLANK } else { STONE };
    upright(canvas, view, r, 6.0, |face, px, py| {
        let grain = paint::chance(px, py, 3, 50);
        Some(match face {
            Face::Top => {
                if grain {
                    block.shine
                } else {
                    block.light
                }
            }
            Face::Left => {
                if grain {
                    block.light
                } else {
                    block.base
                }
            }
            Face::Right => {
                if grain {
                    block.base
                } else {
                    block.shadow
                }
            }
        })
    });
    let low = quad(view, r, 6.0);
    for i in 0..4 {
        let (a, b) = (low[i], low[(i + 1) % 4]);
        paint::line(
            canvas,
            (a.0.round() as i32, a.1.round() as i32),
            (b.0.round() as i32, b.1.round() as i32),
            STONE.shadow,
        );
    }
    let (cx, cy) = view.pixel(x + 0.5, y + 0.5);
    let base = (cx, cy - 6);
    paint::ellipse(canvas, base.0, base.1, 3, 1, IRON.shadow);
    let tip = if swung {
        (cx + 7, cy - 21)
    } else {
        (cx - 7, cy - 21)
    };
    paint::line(canvas, base, tip, IRON.shadow);
    paint::line(canvas, (base.0 + 1, base.1), (tip.0 + 1, tip.1), IRON.light);
    let glowing = pending && (now * 6.0).sin() > -0.3;
    if glowing {
        paint::ellipse(canvas, tip.0, tip.1, 6, 6, rgba(0xffe9a0, 60));
    }
    paint::ellipse(canvas, tip.0, tip.1, 3, 3, rgb(0x6a2218));
    paint::ellipse(
        canvas,
        tip.0,
        tip.1,
        2,
        2,
        if glowing {
            rgb(0xffe9a0)
        } else {
            rgb(0xc8463a)
        },
    );
    paint::put(canvas, tip.0 - 1, tip.1 - 1, rgb(0xfff4d6));
}

/// The back of the garden: a tree in the far corner, a hedgerow along both far edges (broken
/// behind a gate in the board's far lane, where stepping stones lead out), and a potting shed at
/// the right-hand end. Returns where light can move in the tree's leaves, and where the robin
/// can perch.
fn back(
    canvas: &mut Canvas,
    view: &View,
    w: f32,
    d: f32,
    gaps: &[f32],
) -> (Vec<(i32, i32)>, Vec<Perch>) {
    let f = FAR_BORDER;
    let mut layer = Canvas::new(canvas.width(), canvas.height());
    // The tree's shade, dappled, on the grass.
    let (tx, ty) = view.pixel(-0.45, -0.45);
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
    let leaves = oak(&mut layer, (tx, ty + 2), &OAK_CROWN, 41);

    // The hedgerow along both far edges.
    let thick = 0.55;
    let mut runs = vec![(Face::Left, -f + 0.25, d - 1.4)];
    let mut from = -f + 0.3;
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

    // The potting shed.
    let shed = (w - 2.0, -f + 0.05, w + NEAR_BORDER - 0.15, -0.2);
    cast(canvas, view, shed, SHED_WALL);
    shed_at(&mut layer, view, shed);
    paint::blit(canvas, &layer, 0, 0);
    (leaves, perches)
}

/// A little potting shed of weathered boards under a tiled roof, its door and window towards the
/// board.
/// A broad tree's crown, as lobes: each one's offset from the root and its radius.
const OAK_CROWN: [(i32, i32, i32); 9] = [
    (-15, -34, 13),
    (14, -35, 13),
    (0, -45, 15),
    (-6, -27, 11),
    (9, -26, 11),
    (-20, -44, 10),
    (18, -46, 10),
    (-9, -55, 11),
    (8, -57, 11),
];

/// A tree's shade on the ground, dappled where light comes through its leaves.
fn dappled(canvas: &mut Canvas, (cx, cy): (i32, i32), rx: i32, ry: i32) {
    for dy in -ry..=ry {
        for dx in -rx..=rx {
            if dx * dx * ry * ry + dy * dy * rx * rx > rx * rx * ry * ry {
                continue;
            }
            let (px, py) = (cx + dx, cy + dy);
            if patches(px, py, (6, 4), 19) > 0.42 {
                paint::put(canvas, px, py, rgba(0x24452b, 48));
            }
        }
    }
}

/// A broad-leaved tree standing at `root`: a ridged trunk under a crown of lobes, each banded lit
/// on its upper left and shaded on its lower right. Returns points in its leaves where the light
/// can move.
fn oak(
    layer: &mut Canvas,
    root: (i32, i32),
    crown: &[(i32, i32, i32)],
    salt: u32,
) -> Vec<(i32, i32)> {
    let mut tree = Canvas::new(layer.width(), layer.height());
    for dx in -4i32..=4 {
        let h = 26 - dx.abs() * 2;
        for up in 0..h {
            let l = if dx < -1 {
                3
            } else if dx < 2 {
                2
            } else {
                1
            };
            if dx.abs() > 2 + (4 - up.min(4)) / 2 && up >= 4 {
                continue;
            }
            let ridge = i32::from(paint::noise(dx, up / 3, 23).is_multiple_of(4));
            paint::put(&mut tree, root.0 + dx, root.1 - up, tone(BARK, l - ridge));
        }
    }
    for (k, &(dx, dy, r)) in crown.iter().enumerate() {
        let (cx, cy) = (root.0 + dx, root.1 + dy);
        for oy in -r..=r {
            for ox in -r - 2..=r + 2 {
                let edge = (ox * ox) as f32 / ((r + 2) * (r + 2)) as f32
                    + (oy * oy) as f32 / (r * r) as f32;
                let bump = (paint::noise(cx + ox, cy + oy, salt + k as u32) % 100) as f32 / 400.0;
                if edge > 1.0 - bump {
                    continue;
                }
                let band = (ox + oy) as f32 / (2.0 * r as f32);
                let level = if band < -0.45 {
                    4
                } else if band < -0.05 {
                    3
                } else if band < 0.4 {
                    2
                } else {
                    1
                };
                let level = if oy > r / 2 { level - 1 } else { level };
                paint::put(
                    &mut tree,
                    cx + ox,
                    cy + oy,
                    foliage(cx + ox, cy + oy, salt, level),
                );
            }
        }
    }
    edged(&mut tree, 0.55);
    let mut leaves = Vec::new();
    for (k, &(dx, dy, r)) in crown.iter().enumerate() {
        for j in 0..4 {
            let n = paint::noise(k as i32, j, salt + 2);
            let ox = (n % (r as u32)) as i32 - r / 2 - 3;
            let oy = ((n >> 8) % (r as u32)) as i32 - r / 2 - 2;
            leaves.push((root.0 + dx + ox, root.1 + dy + oy));
        }
    }
    paint::blit(layer, &tree, 0, 0);
    leaves
}

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

fn lifted(view: &View, x: f32, y: f32, lift: f32) -> (f32, f32) {
    let (sx, sy) = view.screen(x, y);
    (sx, sy - lift)
}

/// A stone bird bath by the board's left corner, with a pot of flowers beside it. Returns where
/// its water catches the light.
fn bird_bath(canvas: &mut Canvas, view: &View, d: f32) -> (i32, i32) {
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
fn bunting(canvas: &mut Canvas, view: &View, ends: [(f32, f32); 2], t: f32, still: bool) {
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

/// A robin that hops along the hedgerow from perch to perch, and pecks.
fn robin(canvas: &mut Canvas, perches: &[Perch], t: f32, still: bool) {
    if perches.is_empty() {
        return;
    }
    let stay = 3.5;
    let leg = (t / stay).floor() as usize;
    let into = t - leg as f32 * stay;
    let here = perches[leg % perches.len()];
    let next = perches[(leg + 1) % perches.len()];
    let hop = 0.45;
    let (x, y, facing) = if !still && into > stay - hop {
        let s = (into - (stay - hop)) / hop;
        let x = here.0 as f32 + (next.0 - here.0) as f32 * s;
        let y = here.1 as f32 + (next.1 - here.1) as f32 * s - (s * PI).sin() * 7.0;
        (x.round() as i32, y.round() as i32, next.0 >= here.0)
    } else {
        (here.0, here.1, next.0 >= here.0)
    };
    let peck = !still && into < stay - hop && (into * 2.0).fract() < 0.15;
    let f = if facing { 1 } else { -1 };
    let brown = rgb(0x7a5236);
    let dark = rgb(0x4e3424);
    paint::rect(canvas, x - 2, y - 3, 5, 3, brown);
    paint::put(canvas, x - 2 * f, y - 3, dark);
    paint::put(canvas, x - 3 * f, y - 2, dark);
    paint::put(canvas, x + f, y - 2, rgb(0xe0703a));
    paint::put(canvas, x + 2 * f, y - 2, rgb(0xe0703a));
    paint::put(canvas, x + f, y - 1, rgb(0xf2e2c8));
    let head = if peck {
        (x + 2 * f, y - 2)
    } else {
        (x + 2 * f, y - 4)
    };
    paint::rect(canvas, head.0 - 1, head.1, 2, 2, brown);
    paint::put(canvas, head.0 + f, head.1, rgb(0x1e1a18));
    paint::put(canvas, head.0 + 2 * f, head.1 + 1, rgb(0xd8a040));
    paint::put(canvas, x, y, dark);
}

fn butterfly(canvas: &mut Canvas, (x, y): (i32, i32), wing: Rgba, edge: Rgba, open: bool) {
    if open {
        for (dx, dy) in [(-2, -1), (-1, -1), (-2, 0), (1, -1), (2, -1), (2, 0)] {
            paint::put(canvas, x + dx, y + dy, wing);
        }
        paint::put(canvas, x - 2, y - 1, paint::mix(wing, rgb(0xffffff), 0.4));
        paint::put(canvas, x - 1, y, edge);
        paint::put(canvas, x + 1, y, edge);
    } else {
        paint::put(canvas, x - 1, y - 2, wing);
        paint::put(canvas, x - 1, y - 1, wing);
        paint::put(canvas, x + 1, y - 2, edge);
    }
    paint::put(canvas, x, y - 1, rgb(0x3a2a24));
    paint::put(canvas, x, y, rgb(0x3a2a24));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_arena_is_dressed_with_a_piece_for_everything_standing_on_it() {
        for arena in Arena::built_in() {
            let view = View::new(arena.width, arena.height);
            let garden = Garden::new(&arena, &view);
            let dressing = Dressing::of(&arena);
            for tile in arena.tiles() {
                match arena.terrain(tile) {
                    // By the pond, what stands in the way is water, drawn into the ground.
                    Terrain::Obstacle if dressing != Dressing::Millpond => {
                        assert!(garden.pieces.contains_key(&tile), "{} {tile:?}", arena.id);
                    }
                    Terrain::Gate => {
                        assert!(garden.gates.contains_key(&tile), "{} {tile:?}", arena.id);
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn nothing_is_painted_on_the_middle_of_a_tile_someone_can_stand_on() {
        // The corners of the board's tiles take clover and toadstools; their middles stay the
        // ground's own, so whoever stands there is never lost in the decoration.
        for arena in Arena::built_in() {
            let view = View::new(arena.width, arena.height);
            let garden = Garden::new(&arena, &view);
            for tile in arena.tiles() {
                if arena.terrain(tile) != Terrain::Open || arena.in_zone(tile) {
                    continue;
                }
                // Round its middle is the ground's own colour, give or take a fleck of grain: no
                // flower, stone or toadstool.
                let (x, y) = view.centre(tile);
                let mut green = 0;
                for dy in -1..=1 {
                    for dx in -2..=2 {
                        let c = garden.ground.get(x + dx, y + dy);
                        assert_eq!(c.a, 255, "{} {tile:?}", arena.id);
                        // Sand is warm, every other ground green.
                        let ground = if Dressing::of(&arena) == Dressing::Cove {
                            c.r >= c.g && c.g >= c.b
                        } else {
                            c.g >= c.r && c.g >= c.b
                        };
                        green += i32::from(ground);
                    }
                }
                assert!(green >= 12, "{} {tile:?}: {green} of 15", arena.id);
            }
        }
    }
}
