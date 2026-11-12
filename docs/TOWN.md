# The town

The town round the Battleground is a small research campus the Handler walks between pages: their
Research Office, a courtyard, and four buildings off it, the Arena, the Clubhouse, the Guild Hall
and the Handler Studio. It is a way to the pages Battle already has, never a game of its own:
nothing in it changes how a match plays or what the record keeps, and the companions never walk
it. The code is `crates/formiga-battle/src/town/` (places, walking, sheets, pictures) and
`crates/formiga-battle/src/app/town.rs` (the window's side).

Battle opens in town: the train arrives on the Handler's desk, and the Handler walks from there.
The country station is kept for its review picture (`--review station`).

## Arriving

The reveal is scale (`town/arrival.rs`, on screen in `app/arrival.rs`):

| Part | First time | What shows |
| --- | --- | --- |
| Close-up | 4 s | The train pulls in at the little halt inside the habitat, every companion at a window: the station's own train, behind the glass among moss and ferns |
| Pull back | 3 s | One easing zoom out from the glass case to the whole desk, the halt still running inside it, the station smaller than the mug |
| Handoff | 2 s | A cut to the office from above, its 32-pixel tile grid showing and fading over half a second |

After the first time only the train's last second and a half is seen, then the office. Escape,
Space, Enter or a click skips whatever is left, and with reduced motion there is none of it.
Whether the whole arrival has been seen is kept with Battle's settings (`arrival_seen`), and the
Settings page can show it again. Skipping or replaying never touches the career, the trios or the
town. The habitat and the desk are Jake's pictures (`arrival-habitat`, its foreground and
`arrival-desk`), the train standing on the habitat's rails on row 191 as his arrival layout
has it, and the glass case where the close-up is laid into the desk while the camera draws
back.

## Places

Each place is a `.place` file in `town/`, read like the arenas: a grid of 32-pixel tiles saying
where the Handler can stand, the things there are to use, the doors between places and who stands
where. The file's own header (`town/map.rs`) gives the format. The tests check that every place
reads, that every door leads to a door that leads back, and that everything in every place can be
reached and used.

| Place | Size in tiles | Things to use | Who stands there |
| --- | --- | --- | --- |
| Research Office | 13 × 8 | the habitat, the trophies, the bookcase | |
| Campus | 20 × 12 | the notice board, the beacon lamp, the greenhouse | the gardener, the angler |
| Arena | 13 × 8 | the visitors' table, the friends' board, the lessons shelf | Marlow |
| Clubhouse | 13 × 8 | the trio register, the chalkboard, the history shelves | Iris |
| Guild Hall | 13 × 8 | the season board, the Hall of Champions | Odile, Wren |
| Handler Studio | 13 × 8 | the mirror, the counter | Bram, the tailor |

## The art

Every picture in town is Jake's, from his overworld art pack. The pack itself stays out of git
(`.dev/design/formiga-battle-assets-v*`); `scripts/town_art.py` reads its layout guide and writes
what Battle uses, all of it committed:

- the pictures the town draws, in `crates/formiga-battle/assets/town/`, and the list of them
  built into Battle (`town/art.rs`, written by the script);
- each place's `.place` file: its walk grid from the guide, with the town's own things, doors,
  people and the terrace's ledge laid over it;
- each place's `.props` file: where every piece of furniture, building, tree and lamp stands, its
  depth among the people, its shadow, and what of it stands in front of them. The format is in
  `town/picture.rs`.

A place is drawn as the guide says: its background with shadows, rugs and wall pieces laid on it,
then every piece and everyone in the place back to front by their feet, then its foreground and
the tops of trees, lamps, desks and bookcases over them. Light moves on the campus water. The cups
won stand on the office and Arena sideboards in place of their empty stands.

Two things are adjusted from the pack as it came. What stands on a desk or table whose near edge is
drawn in front of people is drawn in front of that edge too, so the habitat, the lamp and the mug
stand on the office desk rather than sinking into it. And the office's back wall is spaced out
(Jake, 12 November): the script moves the two windows in the background picture to either side of
the pennant, which hangs over the desk, and hangs the framed note over the trophy sideboard. A room
a few pixels taller than the window keeps its back wall in sight until the Handler walks the last
row to the door.

When the pack changes, run the script again with its folder and check the pictures with
`--render-window --review town` (the office), `--review campus` and `--review arrival`.

## Walking

The Handler walks one tile a step (0.22 s) in four directions. A tap on a new direction turns on
the spot; holding it walks. Walking onto a door goes through it, and a key held through a door is
ignored until it is let go, so nobody walks straight back in. A click walks to a tile, or to
whatever stands there and uses it; any key gives the walk up.

| Key | Does |
| --- | --- |
| WASD or the arrow keys | walk |
| E, Enter or Space | use what is in front |
| Tab, Shift-Tab | point to the next or previous thing; E then walks there and uses it |
| a click | walk there, or to the thing or person clicked and use it |

The thing in front, or the one Tab points to, shows its prompt over it.

## What each thing opens

| Where | Thing or person | Opens |
| --- | --- | --- |
| Office | habitat | the Handler's page |
| Office | trophies | History |
| Office | bookcase | Settings |
| Campus | notice board | Season |
| Arena | visitors' table, Marlow | Where, against the visitors |
| Arena | friends' board | Where, for a friend |
| Arena | lessons shelf | Field Lessons |
| Clubhouse | register, Iris | the Handler's page and trios |
| Clubhouse | chalkboard | Playbooks |
| Clubhouse | history shelves | History |
| Guild Hall | board, Hall of Champions, Odile | Season |
| Studio | mirror, counter, Bram | the Handler Studio |

## Who says what

The townspeople speak on a card over the foot of the picture, with their portrait; E, Space,
Enter or a click goes on, and after the last line opens the page they keep, if any. Escape
closes it. Everything said about the Handler or their companions is what the record shows, and
with nothing recorded yet it is a plain greeting (`app/speech.rs`). Nothing said changes anything.

| Who or what | Says | Then opens |
| --- | --- | --- |
| Marlow | the next cup and how many of its three are won, or that every cup is won | Where, against the visitors |
| Iris | the trios in the register, by name | the Handler's page |
| Odile | the Open Circuit points from friend matches, and the next event fixture | Season |
| Wren | who she is, or how the meetings with her stand and how the last one went, from the career | nothing |
| Bram, the tailor (also the Studio counter) | a welcome to try a look | the Handler Studio |
| the gardener, the angler | one line of their own, nothing about anyone | nothing |
| the notice board | the next cup and this season's fixture | nothing |
| the beacon lamp, the greenhouse | what they are | nothing |
| the habitat | the companions' names | the Handler's page |

Closing the page, or finishing a match begun from it, comes back to the town where the Handler
stood, and the way back is named after the place: "‹ Back to the Arena".

## The Handler Studio

The Studio's page (also a tab among the Handler's own pages) chooses how the Handler looks around
the Battleground: hair colour, jacket colour (the club's), skin tone, build, hairstyle, outfit and
accessory, each with Jake's own colours as the first choice ("As drawn"). A look is tried on first and kept only by "Keep this look", with the colony's career
in Battle's own file (`career.rs`, `look`), where a rehearsal keeps nothing. Only how they look:
nothing in it touches a match or the record.

A colour is a lookup, never a redraw: each recolourable material is painted in shades of one hue,
and a chosen colour swaps them for the same steps of the new one (`town/look.rs`, with the ramps
from the pack's character layout). The portrait is stacked the same way from its own layers.

## Character sheets

A character sheet is 4 rows of 48 × 48 cells, facing down, left, right and up, with feet on the
44th pixel row of every cell. A full sheet is 8 frames across: 4 walking, 2 standing, 2 reaching.
The Handler is stacked from Jake's layered sheets in his draw order: hair behind, the accessory
behind, body, outfit, the accessory in front, hair in front, then glasses or a cap. Wren, the
gardener and the angler have full sheets (their last two frames are pointing, the watering can and
the rod); Marlow, Iris, Odile and Bram have staff sheets of 4 frames: two standing, two
talking. Spoken to, a townsperson turns to the Handler and talks for a moment.
