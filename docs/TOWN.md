# The town

The town round the Battleground is a small research campus the Handler walks between pages: their
Research Office, a courtyard, and four buildings off it, the Arena, the Clubhouse, the Guild Hall
and the Handler Studio. It is a way to the pages Battle already has, never a game of its own:
nothing in it changes how a match plays or what the record keeps, and the companions never walk
it. The code is `crates/formiga-battle/src/town/` (places, walking, sheets, pictures) and
`crates/formiga-battle/src/app/town.rs` (the window's side).

For now the town opens only for its review picture (`--review town`); Battle still opens at the
station.

## Places

Each place is a `.place` file in `town/`, read like the arenas: a grid of 32-pixel tiles saying
where the Handler can stand, the things there are to use, the doors between places and who stands
where. The file's own header (`town/map.rs`) gives the format. The tests check that every place
reads, that every door leads to a door that leads back, and that everything in every place can be
reached and used.

| Place | Size in tiles | Things to use | Who stands there |
| --- | --- | --- | --- |
| Research Office | 13 × 8 | the habitat, the trophies, the bookcase | |
| Campus | 20 × 12 | the notice board, the beacon lamp, the greenhouse | |
| Arena | 13 × 8 | the visitors' table, the friends' board, the lessons shelf | Marlow |
| Clubhouse | 13 × 8 | the trio register, the chalkboard, the history shelves | Iris |
| Guild Hall | 13 × 8 | the season board, the Hall of Champions | Odile |
| Handler Studio | 13 × 8 | the mirror, the counter | the tailor |

Until a place has its hand-drawn picture, its ground is a plain stand-in painted from its tiles.

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
| Studio | mirror, counter, the tailor | the Handler's page |

Closing the page, or finishing a match begun from it, comes back to the town where the Handler
stood.

## Character sheets

A character sheet is 4 rows of 48 × 48 cells, facing down, left, right and up, with feet on the
44th pixel row of every cell. A full sheet is 8 frames across: 4 walking, 2 standing, 2 reaching.
The Handler is drawn from Jake's first sheet (`crates/formiga-battle/assets/town/`), which walks
only, so it stands and reaches on its first walking frame. The townspeople are the same sheet
tinted, until they have their own.
