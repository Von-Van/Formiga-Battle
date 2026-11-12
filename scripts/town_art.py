#!/usr/bin/env python3
"""Brings an overworld art pack into Battle: the pictures the town uses, and the place files
that say where everything stands.

The pack itself stays out of git (`art-source/formiga-battle-assets-v*`, Jake's art). This reads
its layout guide and writes, inside the repository:

- `crates/formiga-battle/assets/town/*.png`: every picture the town draws, copied as it is;
- `town/<place>.place`: each place's tiles, things, doors and people (the format is in
  `crates/formiga-battle/src/town/map.rs`), built from the guide's walk grids and the town's own
  uses below;
- `town/<place>.props`: where each piece of furniture, building and tree stands and how it is
  drawn (the format is in `crates/formiga-battle/src/town/picture.rs`);
- `crates/formiga-battle/src/town/art.rs`: the pictures built into Battle, by name.

Run it from the repository's top with the pack's folder: python3 scripts/town_art.py PACK
It needs Pillow, for the one picture it cuts (the office habitat's train, standing).
"""

import json
import shutil
import sys
from pathlib import Path

from PIL import Image

ASSETS = Path("crates/formiga-battle/assets/town")
TOWN = Path("town")
ART_RS = Path("crates/formiga-battle/src/town/art.rs")
# A dialogue portrait's cell, in pixels.
TALK_CELL = 256
# Pictures this script makes rather than copies.
made = {"prop-habitat-train-still.png"}

# The town's own uses of each place: what each thing is (by the prop instance, or wall tiles it
# is read from), where the doors lead and who stands where. The guide gives the shapes; this
# gives the meaning.
INTERIORS = {
    "office": {
        "name": "Research Office",
        "things": [
            ("s", "sideboard", "Look at the trophies", {"instance": "trophy-sideboard-1-2"}),
            ("h", "habitat", "Look in the habitat", {"instance": "desk-5-3"}),
            ("b", "bookcase", "Browse the bookcase", {"instance": "bookcase-10-2"}),
        ],
        "door": ("campus", "0"),
        "spawn": "habitat",
        # The back wall spaced out (Jake, 12 November): the windows either side of the pennant,
        # which hangs over the desk, and the framed note over the trophy sideboard. The windows
        # are moved in the background picture: first and last column, and how far.
        "windows": [(105, 152, 5), (162, 209, 96)],
        "wall": {"pennant": (182, 12), "framed-note": (64, 12)},
        "people": [],
    },
    "arena": {
        "name": "Arena",
        "things": [
            ("m", "visitors", "Play the visitors", {"instance": "arena-table-4-3"}),
            ("f", "friends", "Play a friend", {"wall": [(5, 1), (6, 1), (7, 1)]}),
            ("l", "lessons", "Take a Field Lesson", {"instance": "trophy-sideboard-9-6"}),
        ],
        "door": ("campus", "1"),
        "people": [("marlow", "marlow", "down")],
    },
    "clubhouse": {
        "name": "Clubhouse",
        "things": [
            ("r", "register", "Look at your trios", {"instance": "planning-table-4-3"}),
            ("c", "chalkboard", "Open the playbooks", {"wall": [(4, 1), (5, 1), (6, 1), (7, 1), (8, 1)]}),
            ("h", "shelves", "Read the history", {"instance": ["bookcase-1-2", "bookcase-10-2"]}),
        ],
        "door": ("campus", "2"),
        # Beside the guide's spot for her, so she never stands between the Handler and the
        # chalkboard.
        "people": [("iris", (3, 2), "down")],
    },
    "guildhall": {
        "name": "Guild Hall",
        "things": [
            ("n", "board", "Read the season board", {"wall": [(4, 1), (5, 1), (6, 1), (7, 1), (8, 1)]}),
            ("t", "champions", "Look at the Hall of Champions",
             {"instance": ["trophy-sideboard-1-3", "trophy-sideboard-9-3"]}),
        ],
        "door": ("campus", "4"),
        "people": [("odile", "odile", "down"), ("wren", "wren", "down")],
    },
    "studio": {
        "name": "Handler Studio",
        "things": [
            ("m", "mirror", "Try a new look", {"instance": "mirror-6-2"}),
            ("c", "counter", "Talk at the counter", {"instance": "counter-9-4"}),
        ],
        "door": ("campus", "3"),
        "people": [("tailor", "tailor", "down")],
    },
}

CAMPUS = {
    "name": "Campus",
    "doors": {
        # door digit: (tile, place, its door digit, inward)
        "0": ((4, 11), "office", "1", "up"),
        "1": ((3, 3), "arena", "1", "down"),
        "2": ((17, 3), "clubhouse", "1", "down"),
        "3": ((8, 10), "studio", "1", "down"),
        "4": ((16, 9), "guildhall", "1", "down"),
    },
    "things": [
        ("n", "notices", "Read the notice board", [(9, 5)]),
        ("l", "beacon", "Look at the beacon lamp", [(8, 8)]),
        ("g", "greenhouse", "Look in the greenhouse", [(0, 8), (1, 8), (2, 8), (0, 9), (1, 9), (2, 9)]),
    ],
    "people": [("gardener", (1, 10), "up"), ("angler", (11, 2), "right")],
}

WALKABLE_CAMPUS = {"grass", "tall-grass", "terrace", "lane", "steps", "bridge", "exit"}
CAMPUS_CHAR = {
    "grass": ":", "tall-grass": ":", "terrace": ",", "lane": "=", "steps": "=", "bridge": "=",
    "exit": "=", "tree": "T", "water": "~", "building": "#",
}

used = set()


def use(pack, name):
    """Marks a pack file as used and gives its name in Battle's assets."""
    src = pack / name
    if not src.exists():
        raise SystemExit(f"the pack has no {name}")
    flat = Path(name).name
    used.add((src, flat))
    return flat


def interior(pack, guide, key, spec):
    place = guide["places"][key]
    w, h = place["grid"]
    terrain = place["terrain"]
    grid = [["#" if c in ("wall", "prop") else "." for c in row] for row in terrain]
    instances = {}
    props = []
    for p in place["props"]:
        if p.get("sample_only"):
            continue
        defn = guide["props"][p["id"]]
        if "instance" in p:
            tx, ty = p["tile"]
            fw, fh = defn["footprint_tiles"]
            instances[p["instance"]] = [(tx + i, ty + j) for j in range(fh) for i in range(fw)]
        props.append((p, defn))
    letters = []
    for letter, tid, words, where in spec["things"]:
        tiles = []
        names = where.get("instance", [])
        for name in [names] if isinstance(names, str) else names:
            tiles += instances[name]
        tiles += where.get("wall", [])
        for x, y in tiles:
            grid[y][x] = letter
        letters.append(f"thing {letter} {tid} {words}")
    ex, ey = place["exit"]
    grid[ey][ex] = "1"
    spawn = spec.get("spawn")
    if spawn:
        sx, sy = place["hotspots"][spawn]
        grid[sy][sx] = "@"
    lines = [f"name {spec['name']}", f"size {w} {h}", "ground"]
    lines += ["".join(r) for r in grid]
    lines.append(f"# Built by scripts/town_art.py from the art pack's layout guide.")
    lines += letters
    to, mark = spec["door"]
    lines.append(f"door 1 {to} {mark} up")
    for pid, hotspot, facing in spec["people"]:
        x, y = hotspot if isinstance(hotspot, tuple) else place["hotspots"][hotspot]
        lines.append(f"person {pid} {x} {y} {facing}")
    (TOWN / f"{key}.place").write_text("\n".join(lines) + "\n")
    write_props(pack, guide, key, props, interior=True, spec=spec)


def campus(pack, guide):
    place = guide["places"]["campus"]
    w, h = place["grid"]
    terrain = place["terrain"]
    grid = [[CAMPUS_CHAR[c] for c in row] for row in terrain]
    props = []
    for p in place["props"]:
        defn = guide["props"][p["id"]]
        props.append((p, defn))
        cells = defn.get("footprint_cells")
        if cells is None:
            fw, fh = defn["footprint_tiles"]
            tx, ty = p["tile"]
            cells = [[tx + i, ty + j] for j in range(fh) for i in range(fw)]
        for x, y in cells:
            if grid[y][x] in ":,=":
                grid[y][x] = "#"
    things = []
    for letter, tid, words, tiles in CAMPUS["things"]:
        for x, y in tiles:
            grid[y][x] = letter
        things.append(f"thing {letter} {tid} {words}")
    doors = []
    for digit, ((x, y), to, mark, inward) in CAMPUS["doors"].items():
        grid[y][x] = digit
        doors.append(f"door {digit} {to} {mark} {inward}")
    lines = [f"name {CAMPUS['name']}", f"size {w} {h}", "ground"]
    lines += ["".join(r) for r in grid]
    lines.append("# Built by scripts/town_art.py from the art pack's layout guide.")
    lines += things + doors
    boundary = place["terrace_boundary"]
    top_row = boundary["between_rows"][0]
    lo, hi = boundary["columns"]
    crossing = tuple(boundary["crossing_tile"])
    for x in range(lo, hi + 1):
        if (x, top_row + 1) != crossing:
            lines.append(f"ledge {x} {top_row} down")
    for pid, (x, y), facing in CAMPUS["people"]:
        lines.append(f"person {pid} {x} {y} {facing}")
    (TOWN / "campus.place").write_text("\n".join(lines) + "\n")
    write_props(pack, guide, "campus", props, interior=False)


# The background's plaster rows; the wainscot below them has panels 32 pixels wide.
PLASTER_ROWS = 39


def respace(bg, moves):
    """Moves windows baked into an office wall, filling where they were with the plain wall a
    whole number of panels away. `moves`: each window's first and last column and its shift."""
    plaster = {bg.getpixel((x, y)) for y in range(PLASTER_ROWS) for x in range(222, 318)}
    out = bg.copy()

    def clean(x):
        return x - 96 if x < 158 else x + 128

    def is_window(x, y):
        if y < PLASTER_ROWS:
            return bg.getpixel((x, y)) not in plaster
        return bg.getpixel((x, y)) != bg.getpixel((clean(x), y))

    pixels = []
    for x0, x1, dx in moves:
        for y in range(48):
            for x in range(x0 - 2, x1 + 3):
                if is_window(x, y):
                    pixels.append((x + dx, y, bg.getpixel((x, y))))
        for y in range(48):
            for x in range(x0 - 2, x1 + 3):
                out.putpixel((x, y), bg.getpixel((clean(x), y)))
    for x, y, p in pixels:
        out.putpixel((x, y), p)
    return out


edited = {}


def write_props(pack, guide, key, props, interior, spec=None):
    shade = guide["shadows"]
    spec = spec or {}
    background = use(pack, f"place-{key}-bg.png")
    if spec.get("windows"):
        edited[background] = respace(
            Image.open(pack / f"place-{key}-bg.png").convert("RGBA"), spec["windows"]
        )
    lines = [
        "# Built by scripts/town_art.py from the art pack's layout guide.",
        f"background {background}",
        f"foreground {use(pack, f'place-{key}-fg.png')}",
    ]
    parents = {}
    # Pieces whose near edge stands in front of people: what stands on them has to stand in
    # front too, or the near edge would cover it.
    fronted = set()
    for p, d in props:
        x, y = p["at_px"]
        layer = d.get("layer", "depth-sort")
        if p["id"] == "habitat-train":
            # The office's little train, standing at the platform: one frame of its strip.
            strip = Image.open(pack / d["file"])
            fw, fh = d["frame_size"]
            frame = p.get("preview_frame", 0)
            name = "prop-habitat-train-still.png"
            strip.crop((fw * frame, 0, fw * (frame + 1), fh)).save(ASSETS / name)
            parent = p.get("attached_to")
            sort = parents.get(parent, y)
            ax, ay = d["anchor_px"]
            front = 1 if parent in fronted else 0
            # The train runs round its loop in the habitat, standing at the platform only when
            # motion is reduced: the habitat's corner, the loop's points from it, and the still.
            hx, hy = next(q["at_px"] for q, e in props if q["id"] == "habitat-base")
            loop = guide["habitat"]["office"]["rail_loop"]
            use(pack, d["file"])
            use(pack, guide["props"]["habitat-train-vertical"]["file"])
            points = ";".join(f"{lx},{ly}" for lx, ly in loop)
            lines.append(f"train {hx} {hy} {sort} {front} {points} {name} {x - ax} {y - ay}")
            continue
        if p["id"] == "trophy-empty-stand":
            parent = p["attached_to"]
            sort = parents[parent]
            slot = p["slot"]
            stand = use(pack, d["file"])
            # Where a won cup stands instead: bottom-centre on the slot.
            lines.append(f"cup {slot} {x + 6} {y + 6} {sort} {stand} {x} {y}")
            continue
        file = use(pack, d.get("base_file", d["file"]))
        if layer in ("floor", "wall"):
            x, y = spec.get("wall", {}).get(p["id"], (x, y))
            lines.append(f"{layer} {file} {x} {y}")
            continue
        if "attached_to" in p or layer.startswith("habitat") or layer == "surface":
            parent = p.get("attached_to")
            sort = parents.get(parent, y)
            over = file if parent in fronted else "-"
            lines.append(f"sort {file} {x} {y} {sort} - 0 {over} 0")
            continue
        sort = p.get("sort_y", y + d.get("sort_baseline_y", 0))
        if "instance" in p:
            parents[p["instance"]] = sort
        shadow = d.get("shadow_file")
        shadow_name = use(pack, shadow) if shadow else "-"
        opacity = int(round(255 * d.get("shadow_opacity", shade["opacity"]))) if shadow else 0
        over, rows = "-", 0
        if "foreground_file" in d:
            over = use(pack, d["foreground_file"])
        elif "foreground_rows" in d:
            rows = d["foreground_rows"][1]
        if (over != "-" or rows) and "instance" in p:
            fronted.add(p["instance"])
        lines.append(f"sort {file} {x} {y} {sort} {shadow_name} {opacity} {over} {rows}")
    (TOWN / f"{key}.props").write_text("\n".join(lines) + "\n")


def main():
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    pack = Path(sys.argv[1])
    guide = json.loads((pack / "kit/layout-guide.json").read_text())
    ASSETS.mkdir(parents=True, exist_ok=True)
    for key, spec in INTERIORS.items():
        interior(pack, guide, key, spec)
    campus(pack, guide)
    chars = json.loads((pack / "character-layout.json").read_text())
    # Every layered Handler sheet and portrait piece, the townspeople and their portraits, the
    # arrival and the small pieces.
    for f in sorted(pack.glob("handler-*.png")):
        use(pack, f.name)
    for npc in chars["npcs"]:
        use(pack, npc["sprite"])
    # The dialogue portraits are Jake's illustrated sheets: four expressions (calm, talking,
    # pleased, their own) each lined up by its draw offset, and brought down to 256-pixel cells
    # so Battle carries a quarter of their weight. They are drawn smoothed, not as pixel art.
    talk = chars["portrait_layout"].get("dialogue_sheets", {}).get("sheets", {})
    for key, sheet in talk.items():
        src = Image.open(pack / sheet["file"]).convert("RGBA")
        out = Image.new("RGBA", (2 * TALK_CELL, 2 * TALK_CELL), (0, 0, 0, 0))
        for i, frame in enumerate(sheet["frames"]):
            x, y, w, h = frame["rect_px"]
            ox, oy = frame.get("draw_offset_px", [0, 0])
            cell = Image.new("RGBA", (w, h), (0, 0, 0, 0))
            cell.alpha_composite(src.crop((x, y, x + w, y + h)), (ox, oy))
            cell = cell.resize((TALK_CELL, TALK_CELL), Image.LANCZOS)
            out.alpha_composite(cell, ((i % 2) * TALK_CELL, (i // 2) * TALK_CELL))
        name = f"portrait-{key}-talk.png"
        out.save(ASSETS / name, optimize=True)
        made.add(name)
    for name in [
        "arrival-habitat.png", "arrival-habitat-fg.png", "arrival-desk.png",
        "anim-water.png", "anim-lamp.png", "anim-beacon.png", "anim-flag.png", "icon-notice.png",
        "trophy-cups.png", "trophy-season.png", "banner-cups.png", "frames-notebook.png",
    ]:
        use(pack, name)
    for src, flat in sorted(used):
        if flat in edited:
            edited[flat].save(ASSETS / flat)
        else:
            shutil.copyfile(src, ASSETS / flat)
    names = sorted({flat for _, flat in used} | made)
    arms = "\n".join(
        f'        "{n}" => include_bytes!("../../assets/town/{n}"),' for n in names
    )
    ART_RS.write_text(
        "//! The town's pictures, built into Battle by name: Jake's overworld art, copied from his\n"
        "//! art pack by `scripts/town_art.py`, which writes this file. Do not edit it by hand.\n"
        "\n"
        "/// A picture's PNG bytes, by its file name.\n"
        "pub fn bytes(name: &str) -> Option<&'static [u8]> {\n"
        "    Some(match name {\n"
        f"{arms}\n"
        "        _ => return None,\n"
        "    })\n"
        "}\n"
        "\n"
        "/// Every picture there is, for the tests.\n"
        "#[cfg(test)]\n"
        f"pub const ALL: [&str; {len(names)}] = [\n"
        + "".join(f'    "{n}",\n' for n in names)
        + "];\n"
    )
    print(f"{len(names)} pictures, {len(INTERIORS) + 1} places")


if __name__ == "__main__":
    main()
