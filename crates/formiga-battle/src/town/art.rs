//! The town's pictures, built into Battle by name: Jake's overworld art, copied from his
//! art pack by `scripts/town_art.py`, which writes this file. Do not edit it by hand.

/// A picture's PNG bytes, by its file name.
pub fn bytes(name: &str) -> Option<&'static [u8]> {
    Some(match name {
        "anim-beacon.png" => include_bytes!("../../assets/town/anim-beacon.png"),
        "anim-flag.png" => include_bytes!("../../assets/town/anim-flag.png"),
        "anim-lamp.png" => include_bytes!("../../assets/town/anim-lamp.png"),
        "anim-water.png" => include_bytes!("../../assets/town/anim-water.png"),
        "arrival-desk.png" => include_bytes!("../../assets/town/arrival-desk.png"),
        "arrival-habitat-fg.png" => include_bytes!("../../assets/town/arrival-habitat-fg.png"),
        "arrival-habitat.png" => include_bytes!("../../assets/town/arrival-habitat.png"),
        "banner-cups.png" => include_bytes!("../../assets/town/banner-cups.png"),
        "frames-notebook.png" => include_bytes!("../../assets/town/frames-notebook.png"),
        "handler-acc-field-cap.png" => {
            include_bytes!("../../assets/town/handler-acc-field-cap.png")
        }
        "handler-acc-glasses.png" => include_bytes!("../../assets/town/handler-acc-glasses.png"),
        "handler-acc-magnifier.png" => {
            include_bytes!("../../assets/town/handler-acc-magnifier.png")
        }
        "handler-acc-satchel-back.png" => {
            include_bytes!("../../assets/town/handler-acc-satchel-back.png")
        }
        "handler-acc-satchel-front.png" => {
            include_bytes!("../../assets/town/handler-acc-satchel-front.png")
        }
        "handler-acc-scarf-back.png" => {
            include_bytes!("../../assets/town/handler-acc-scarf-back.png")
        }
        "handler-acc-scarf-front.png" => {
            include_bytes!("../../assets/town/handler-acc-scarf-front.png")
        }
        "handler-body-a.png" => include_bytes!("../../assets/town/handler-body-a.png"),
        "handler-body-b.png" => include_bytes!("../../assets/town/handler-body-b.png"),
        "handler-hair-braids-back.png" => {
            include_bytes!("../../assets/town/handler-hair-braids-back.png")
        }
        "handler-hair-braids-front.png" => {
            include_bytes!("../../assets/town/handler-hair-braids-front.png")
        }
        "handler-hair-bun-back.png" => {
            include_bytes!("../../assets/town/handler-hair-bun-back.png")
        }
        "handler-hair-bun-front.png" => {
            include_bytes!("../../assets/town/handler-hair-bun-front.png")
        }
        "handler-hair-curls-back.png" => {
            include_bytes!("../../assets/town/handler-hair-curls-back.png")
        }
        "handler-hair-curls-front.png" => {
            include_bytes!("../../assets/town/handler-hair-curls-front.png")
        }
        "handler-hair-long-back.png" => {
            include_bytes!("../../assets/town/handler-hair-long-back.png")
        }
        "handler-hair-long-front.png" => {
            include_bytes!("../../assets/town/handler-hair-long-front.png")
        }
        "handler-hair-short-crop-back.png" => {
            include_bytes!("../../assets/town/handler-hair-short-crop-back.png")
        }
        "handler-hair-short-crop-front.png" => {
            include_bytes!("../../assets/town/handler-hair-short-crop-front.png")
        }
        "handler-hair-tousled-back.png" => {
            include_bytes!("../../assets/town/handler-hair-tousled-back.png")
        }
        "handler-hair-tousled-front.png" => {
            include_bytes!("../../assets/town/handler-hair-tousled-front.png")
        }
        "handler-outfit-club-jersey-a.png" => {
            include_bytes!("../../assets/town/handler-outfit-club-jersey-a.png")
        }
        "handler-outfit-club-jersey-b.png" => {
            include_bytes!("../../assets/town/handler-outfit-club-jersey-b.png")
        }
        "handler-outfit-field-jacket-a.png" => {
            include_bytes!("../../assets/town/handler-outfit-field-jacket-a.png")
        }
        "handler-outfit-field-jacket-b.png" => {
            include_bytes!("../../assets/town/handler-outfit-field-jacket-b.png")
        }
        "handler-outfit-jumper-a.png" => {
            include_bytes!("../../assets/town/handler-outfit-jumper-a.png")
        }
        "handler-outfit-jumper-b.png" => {
            include_bytes!("../../assets/town/handler-outfit-jumper-b.png")
        }
        "handler-outfit-lab-coat-a.png" => {
            include_bytes!("../../assets/town/handler-outfit-lab-coat-a.png")
        }
        "handler-outfit-lab-coat-b.png" => {
            include_bytes!("../../assets/town/handler-outfit-lab-coat-b.png")
        }
        "handler-outfit-overalls-a.png" => {
            include_bytes!("../../assets/town/handler-outfit-overalls-a.png")
        }
        "handler-outfit-overalls-b.png" => {
            include_bytes!("../../assets/town/handler-outfit-overalls-b.png")
        }
        "handler-outfit-waistcoat-a.png" => {
            include_bytes!("../../assets/town/handler-outfit-waistcoat-a.png")
        }
        "handler-outfit-waistcoat-b.png" => {
            include_bytes!("../../assets/town/handler-outfit-waistcoat-b.png")
        }
        "icon-notice.png" => include_bytes!("../../assets/town/icon-notice.png"),
        "npc-angler.png" => include_bytes!("../../assets/town/npc-angler.png"),
        "npc-gardener.png" => include_bytes!("../../assets/town/npc-gardener.png"),
        "npc-iris.png" => include_bytes!("../../assets/town/npc-iris.png"),
        "npc-marlow.png" => include_bytes!("../../assets/town/npc-marlow.png"),
        "npc-odile.png" => include_bytes!("../../assets/town/npc-odile.png"),
        "npc-tailor.png" => include_bytes!("../../assets/town/npc-tailor.png"),
        "npc-wren.png" => include_bytes!("../../assets/town/npc-wren.png"),
        "place-arena-bg.png" => include_bytes!("../../assets/town/place-arena-bg.png"),
        "place-arena-fg.png" => include_bytes!("../../assets/town/place-arena-fg.png"),
        "place-campus-bg.png" => include_bytes!("../../assets/town/place-campus-bg.png"),
        "place-campus-fg.png" => include_bytes!("../../assets/town/place-campus-fg.png"),
        "place-clubhouse-bg.png" => include_bytes!("../../assets/town/place-clubhouse-bg.png"),
        "place-clubhouse-fg.png" => include_bytes!("../../assets/town/place-clubhouse-fg.png"),
        "place-guildhall-bg.png" => include_bytes!("../../assets/town/place-guildhall-bg.png"),
        "place-guildhall-fg.png" => include_bytes!("../../assets/town/place-guildhall-fg.png"),
        "place-office-bg.png" => include_bytes!("../../assets/town/place-office-bg.png"),
        "place-office-fg.png" => include_bytes!("../../assets/town/place-office-fg.png"),
        "place-studio-bg.png" => include_bytes!("../../assets/town/place-studio-bg.png"),
        "place-studio-fg.png" => include_bytes!("../../assets/town/place-studio-fg.png"),
        "portrait-angler-talk.png" => include_bytes!("../../assets/town/portrait-angler-talk.png"),
        "portrait-gardener-talk.png" => {
            include_bytes!("../../assets/town/portrait-gardener-talk.png")
        }
        "portrait-iris-talk.png" => include_bytes!("../../assets/town/portrait-iris-talk.png"),
        "portrait-marlow-talk.png" => include_bytes!("../../assets/town/portrait-marlow-talk.png"),
        "portrait-odile-talk.png" => include_bytes!("../../assets/town/portrait-odile-talk.png"),
        "portrait-tailor-talk.png" => include_bytes!("../../assets/town/portrait-tailor-talk.png"),
        "portrait-wren-talk.png" => include_bytes!("../../assets/town/portrait-wren-talk.png"),
        "prop-arena-model.png" => include_bytes!("../../assets/town/prop-arena-model.png"),
        "prop-arena-table-shadow.png" => {
            include_bytes!("../../assets/town/prop-arena-table-shadow.png")
        }
        "prop-arena-table.png" => include_bytes!("../../assets/town/prop-arena-table.png"),
        "prop-arena-wall-flag.png" => include_bytes!("../../assets/town/prop-arena-wall-flag.png"),
        "prop-armchair-shadow.png" => include_bytes!("../../assets/town/prop-armchair-shadow.png"),
        "prop-armchair-wine-shadow.png" => {
            include_bytes!("../../assets/town/prop-armchair-wine-shadow.png")
        }
        "prop-armchair-wine.png" => include_bytes!("../../assets/town/prop-armchair-wine.png"),
        "prop-armchair.png" => include_bytes!("../../assets/town/prop-armchair.png"),
        "prop-bench-shadow.png" => include_bytes!("../../assets/town/prop-bench-shadow.png"),
        "prop-bench.png" => include_bytes!("../../assets/town/prop-bench.png"),
        "prop-bookcase-fg.png" => include_bytes!("../../assets/town/prop-bookcase-fg.png"),
        "prop-bookcase-shadow.png" => include_bytes!("../../assets/town/prop-bookcase-shadow.png"),
        "prop-bookcase.png" => include_bytes!("../../assets/town/prop-bookcase.png"),
        "prop-campus-arena.png" => include_bytes!("../../assets/town/prop-campus-arena.png"),
        "prop-campus-beacon-lamp-base.png" => {
            include_bytes!("../../assets/town/prop-campus-beacon-lamp-base.png")
        }
        "prop-campus-beacon-lamp-fg.png" => {
            include_bytes!("../../assets/town/prop-campus-beacon-lamp-fg.png")
        }
        "prop-campus-bench.png" => include_bytes!("../../assets/town/prop-campus-bench.png"),
        "prop-campus-clubhouse.png" => {
            include_bytes!("../../assets/town/prop-campus-clubhouse.png")
        }
        "prop-campus-cottage.png" => include_bytes!("../../assets/town/prop-campus-cottage.png"),
        "prop-campus-fence-north-edge-of-row-6.png" => {
            include_bytes!("../../assets/town/prop-campus-fence-north-edge-of-row-6.png")
        }
        "prop-campus-flower-patch-1.png" => {
            include_bytes!("../../assets/town/prop-campus-flower-patch-1.png")
        }
        "prop-campus-flower-patch-2.png" => {
            include_bytes!("../../assets/town/prop-campus-flower-patch-2.png")
        }
        "prop-campus-flower-patch.png" => {
            include_bytes!("../../assets/town/prop-campus-flower-patch.png")
        }
        "prop-campus-greenhouse.png" => {
            include_bytes!("../../assets/town/prop-campus-greenhouse.png")
        }
        "prop-campus-guildhall.png" => {
            include_bytes!("../../assets/town/prop-campus-guildhall.png")
        }
        "prop-campus-lamp-post-base.png" => {
            include_bytes!("../../assets/town/prop-campus-lamp-post-base.png")
        }
        "prop-campus-lamp-post-fg.png" => {
            include_bytes!("../../assets/town/prop-campus-lamp-post-fg.png")
        }
        "prop-campus-notice-board.png" => {
            include_bytes!("../../assets/town/prop-campus-notice-board.png")
        }
        "prop-campus-rock-1.png" => include_bytes!("../../assets/town/prop-campus-rock-1.png"),
        "prop-campus-rock-2.png" => include_bytes!("../../assets/town/prop-campus-rock-2.png"),
        "prop-campus-rock.png" => include_bytes!("../../assets/town/prop-campus-rock.png"),
        "prop-campus-studio.png" => include_bytes!("../../assets/town/prop-campus-studio.png"),
        "prop-campus-tree-0.png" => include_bytes!("../../assets/town/prop-campus-tree-0.png"),
        "prop-campus-tree-1.png" => include_bytes!("../../assets/town/prop-campus-tree-1.png"),
        "prop-campus-tree-2.png" => include_bytes!("../../assets/town/prop-campus-tree-2.png"),
        "prop-campus-tree-3.png" => include_bytes!("../../assets/town/prop-campus-tree-3.png"),
        "prop-campus-tree-4.png" => include_bytes!("../../assets/town/prop-campus-tree-4.png"),
        "prop-campus-tree-5.png" => include_bytes!("../../assets/town/prop-campus-tree-5.png"),
        "prop-campus-tree-6.png" => include_bytes!("../../assets/town/prop-campus-tree-6.png"),
        "prop-campus-tree-7.png" => include_bytes!("../../assets/town/prop-campus-tree-7.png"),
        "prop-campus-tree-8.png" => include_bytes!("../../assets/town/prop-campus-tree-8.png"),
        "prop-chalkboard.png" => include_bytes!("../../assets/town/prop-chalkboard.png"),
        "prop-clothes-rail-shadow.png" => {
            include_bytes!("../../assets/town/prop-clothes-rail-shadow.png")
        }
        "prop-clothes-rail.png" => include_bytes!("../../assets/town/prop-clothes-rail.png"),
        "prop-counter-shadow.png" => include_bytes!("../../assets/town/prop-counter-shadow.png"),
        "prop-counter.png" => include_bytes!("../../assets/town/prop-counter.png"),
        "prop-desk-fg.png" => include_bytes!("../../assets/town/prop-desk-fg.png"),
        "prop-desk-mug.png" => include_bytes!("../../assets/town/prop-desk-mug.png"),
        "prop-desk-pencil.png" => include_bytes!("../../assets/town/prop-desk-pencil.png"),
        "prop-desk-shadow.png" => include_bytes!("../../assets/town/prop-desk-shadow.png"),
        "prop-desk.png" => include_bytes!("../../assets/town/prop-desk.png"),
        "prop-doormat.png" => include_bytes!("../../assets/town/prop-doormat.png"),
        "prop-field-notebook.png" => include_bytes!("../../assets/town/prop-field-notebook.png"),
        "prop-fitting-rug.png" => include_bytes!("../../assets/town/prop-fitting-rug.png"),
        "prop-floor-rug.png" => include_bytes!("../../assets/town/prop-floor-rug.png"),
        "prop-framed-note.png" => include_bytes!("../../assets/town/prop-framed-note.png"),
        "prop-habitat-base.png" => include_bytes!("../../assets/town/prop-habitat-base.png"),
        "prop-habitat-glass.png" => include_bytes!("../../assets/town/prop-habitat-glass.png"),
        "prop-habitat-station.png" => include_bytes!("../../assets/town/prop-habitat-station.png"),
        "prop-habitat-track.png" => include_bytes!("../../assets/town/prop-habitat-track.png"),
        "prop-habitat-train-still.png" => {
            include_bytes!("../../assets/town/prop-habitat-train-still.png")
        }
        "prop-habitat-train-vertical.png" => {
            include_bytes!("../../assets/town/prop-habitat-train-vertical.png")
        }
        "prop-habitat-train.png" => include_bytes!("../../assets/town/prop-habitat-train.png"),
        "prop-hat-shelf-shadow.png" => {
            include_bytes!("../../assets/town/prop-hat-shelf-shadow.png")
        }
        "prop-hat-shelf.png" => include_bytes!("../../assets/town/prop-hat-shelf.png"),
        "prop-lamp.png" => include_bytes!("../../assets/town/prop-lamp.png"),
        "prop-mirror-shadow.png" => include_bytes!("../../assets/town/prop-mirror-shadow.png"),
        "prop-mirror.png" => include_bytes!("../../assets/town/prop-mirror.png"),
        "prop-notice-board.png" => include_bytes!("../../assets/town/prop-notice-board.png"),
        "prop-pennant.png" => include_bytes!("../../assets/town/prop-pennant.png"),
        "prop-planning-table-shadow.png" => {
            include_bytes!("../../assets/town/prop-planning-table-shadow.png")
        }
        "prop-planning-table.png" => include_bytes!("../../assets/town/prop-planning-table.png"),
        "prop-planter-fg.png" => include_bytes!("../../assets/town/prop-planter-fg.png"),
        "prop-planter-shadow.png" => include_bytes!("../../assets/town/prop-planter-shadow.png"),
        "prop-planter.png" => include_bytes!("../../assets/town/prop-planter.png"),
        "prop-rug-guild.png" => include_bytes!("../../assets/town/prop-rug-guild.png"),
        "prop-rug-wine.png" => include_bytes!("../../assets/town/prop-rug-wine.png"),
        "prop-scoreboard.png" => include_bytes!("../../assets/town/prop-scoreboard.png"),
        "prop-side-table-shadow.png" => {
            include_bytes!("../../assets/town/prop-side-table-shadow.png")
        }
        "prop-side-table.png" => include_bytes!("../../assets/town/prop-side-table.png"),
        "prop-specimen-crates-shadow.png" => {
            include_bytes!("../../assets/town/prop-specimen-crates-shadow.png")
        }
        "prop-specimen-crates.png" => include_bytes!("../../assets/town/prop-specimen-crates.png"),
        "prop-trophy-empty-stand.png" => {
            include_bytes!("../../assets/town/prop-trophy-empty-stand.png")
        }
        "prop-trophy-sideboard-shadow.png" => {
            include_bytes!("../../assets/town/prop-trophy-sideboard-shadow.png")
        }
        "prop-trophy-sideboard.png" => {
            include_bytes!("../../assets/town/prop-trophy-sideboard.png")
        }
        "trophy-cups.png" => include_bytes!("../../assets/town/trophy-cups.png"),
        "trophy-season.png" => include_bytes!("../../assets/town/trophy-season.png"),
        _ => return None,
    })
}

/// Every picture there is, for the tests.
#[cfg(test)]
pub const ALL: [&str; 156] = [
    "anim-beacon.png",
    "anim-flag.png",
    "anim-lamp.png",
    "anim-water.png",
    "arrival-desk.png",
    "arrival-habitat-fg.png",
    "arrival-habitat.png",
    "banner-cups.png",
    "frames-notebook.png",
    "handler-acc-field-cap.png",
    "handler-acc-glasses.png",
    "handler-acc-magnifier.png",
    "handler-acc-satchel-back.png",
    "handler-acc-satchel-front.png",
    "handler-acc-scarf-back.png",
    "handler-acc-scarf-front.png",
    "handler-body-a.png",
    "handler-body-b.png",
    "handler-hair-braids-back.png",
    "handler-hair-braids-front.png",
    "handler-hair-bun-back.png",
    "handler-hair-bun-front.png",
    "handler-hair-curls-back.png",
    "handler-hair-curls-front.png",
    "handler-hair-long-back.png",
    "handler-hair-long-front.png",
    "handler-hair-short-crop-back.png",
    "handler-hair-short-crop-front.png",
    "handler-hair-tousled-back.png",
    "handler-hair-tousled-front.png",
    "handler-outfit-club-jersey-a.png",
    "handler-outfit-club-jersey-b.png",
    "handler-outfit-field-jacket-a.png",
    "handler-outfit-field-jacket-b.png",
    "handler-outfit-jumper-a.png",
    "handler-outfit-jumper-b.png",
    "handler-outfit-lab-coat-a.png",
    "handler-outfit-lab-coat-b.png",
    "handler-outfit-overalls-a.png",
    "handler-outfit-overalls-b.png",
    "handler-outfit-waistcoat-a.png",
    "handler-outfit-waistcoat-b.png",
    "icon-notice.png",
    "npc-angler.png",
    "npc-gardener.png",
    "npc-iris.png",
    "npc-marlow.png",
    "npc-odile.png",
    "npc-tailor.png",
    "npc-wren.png",
    "place-arena-bg.png",
    "place-arena-fg.png",
    "place-campus-bg.png",
    "place-campus-fg.png",
    "place-clubhouse-bg.png",
    "place-clubhouse-fg.png",
    "place-guildhall-bg.png",
    "place-guildhall-fg.png",
    "place-office-bg.png",
    "place-office-fg.png",
    "place-studio-bg.png",
    "place-studio-fg.png",
    "portrait-angler-talk.png",
    "portrait-gardener-talk.png",
    "portrait-iris-talk.png",
    "portrait-marlow-talk.png",
    "portrait-odile-talk.png",
    "portrait-tailor-talk.png",
    "portrait-wren-talk.png",
    "prop-arena-model.png",
    "prop-arena-table-shadow.png",
    "prop-arena-table.png",
    "prop-arena-wall-flag.png",
    "prop-armchair-shadow.png",
    "prop-armchair-wine-shadow.png",
    "prop-armchair-wine.png",
    "prop-armchair.png",
    "prop-bench-shadow.png",
    "prop-bench.png",
    "prop-bookcase-fg.png",
    "prop-bookcase-shadow.png",
    "prop-bookcase.png",
    "prop-campus-arena.png",
    "prop-campus-beacon-lamp-base.png",
    "prop-campus-beacon-lamp-fg.png",
    "prop-campus-bench.png",
    "prop-campus-clubhouse.png",
    "prop-campus-cottage.png",
    "prop-campus-fence-north-edge-of-row-6.png",
    "prop-campus-flower-patch-1.png",
    "prop-campus-flower-patch-2.png",
    "prop-campus-flower-patch.png",
    "prop-campus-greenhouse.png",
    "prop-campus-guildhall.png",
    "prop-campus-lamp-post-base.png",
    "prop-campus-lamp-post-fg.png",
    "prop-campus-notice-board.png",
    "prop-campus-rock-1.png",
    "prop-campus-rock-2.png",
    "prop-campus-rock.png",
    "prop-campus-studio.png",
    "prop-campus-tree-0.png",
    "prop-campus-tree-1.png",
    "prop-campus-tree-2.png",
    "prop-campus-tree-3.png",
    "prop-campus-tree-4.png",
    "prop-campus-tree-5.png",
    "prop-campus-tree-6.png",
    "prop-campus-tree-7.png",
    "prop-campus-tree-8.png",
    "prop-chalkboard.png",
    "prop-clothes-rail-shadow.png",
    "prop-clothes-rail.png",
    "prop-counter-shadow.png",
    "prop-counter.png",
    "prop-desk-fg.png",
    "prop-desk-mug.png",
    "prop-desk-pencil.png",
    "prop-desk-shadow.png",
    "prop-desk.png",
    "prop-doormat.png",
    "prop-field-notebook.png",
    "prop-fitting-rug.png",
    "prop-floor-rug.png",
    "prop-framed-note.png",
    "prop-habitat-base.png",
    "prop-habitat-glass.png",
    "prop-habitat-station.png",
    "prop-habitat-track.png",
    "prop-habitat-train-still.png",
    "prop-habitat-train-vertical.png",
    "prop-habitat-train.png",
    "prop-hat-shelf-shadow.png",
    "prop-hat-shelf.png",
    "prop-lamp.png",
    "prop-mirror-shadow.png",
    "prop-mirror.png",
    "prop-notice-board.png",
    "prop-pennant.png",
    "prop-planning-table-shadow.png",
    "prop-planning-table.png",
    "prop-planter-fg.png",
    "prop-planter-shadow.png",
    "prop-planter.png",
    "prop-rug-guild.png",
    "prop-rug-wine.png",
    "prop-scoreboard.png",
    "prop-side-table-shadow.png",
    "prop-side-table.png",
    "prop-specimen-crates-shadow.png",
    "prop-specimen-crates.png",
    "prop-trophy-empty-stand.png",
    "prop-trophy-sideboard-shadow.png",
    "prop-trophy-sideboard.png",
    "trophy-cups.png",
    "trophy-season.png",
];
