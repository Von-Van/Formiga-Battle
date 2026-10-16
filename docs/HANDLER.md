# The Handler, their trios and their legacy

A plan, written before building (2026-10-16), with what Jake decided and what Phase 1 became. Desktop makes each Formiga unique; this is how
Battle makes their competitive lives, and the player's own part as their Handler, unique too,
without a progression system, a currency or anything that changes how a match is played.

## What is there today

| Area | What Battle has | What that means for identity |
|---|---|---|
| Saving | Battle keeps no files. Desktop keeps the colony's `BattleRecord` (points, cups, unlocks, the last 20 friend matches, notebook pages, applied match ids), hands it over in each session's snapshot, and applies Battle's `result.json` when Battle closes. The record crosses through `formiga-battle-contract` and is mirrored field for field in `formiga-core`'s `BattleRecord` (save v31) | Anything that should last (a Handler card, trios, their records) must be added to the contract **and** to Desktop's copy, or Desktop silently drops it. This is a Desktop change as well as a Battle one |
| The Handler | A name typed on the Friend page each sitting, sent to the relay, cleaned to 24 characters. Nothing else | Nothing persists yet. The name is the one identity field a friend already sees |
| The roster | Desktop's contenders, by permanent creature id; names and looks come live from the snapshot | Keying everything by creature id handles renames for free. A creature that has left the colony is simply absent from the roster |
| Notebook | One page per companion (24 kept): friend matches, wins, what was seen (`Sign`s) and moments ("Flanked Biscuit with a friend", "Was holding the beacon when it lit") | The per-companion layer of legacy already exists and is evidence-based |
| Rewards | Friend: points, a history line, notebook, participation. Visitors: cup wins, unlocks, journal lines, keepsakes. A visitor match leaves no history line | A trio's record needs a new, separate tally, or visitor matches would never count for a team |
| Friend matches | The relay's `Entrant` carries the Handler name, the three companions and the arena. Protocol 3 (team plans) is not released yet. The site relay rebuilds each entrant field by field, so it would drop anything new | New shared fields need the matching site relay change, and must be listed in the "what is shared" notice (`SHARED` in `app.rs`) |
| Screens | Station masthead already has two companions with team pennants; History & settings pages; the match's top bar; the result card with "Earned" lines | Natural places for a pennant, a trio's name and a line of recognition, with no new windows |
| Observable events | Every turn's events: captures, flanks (with who flanked), counters, knockouts, levers and switches, items, the tally at each turn | Enough for honest distinctions (cups, the beacon, flanking together, comebacks on the tally). Not enough to claim traits like "patient" or "reads the opponent" |

## The shape of it

One record, kept by Desktop, extended in place:

- **Handler card**: name (24 characters), optional club name (32), and a pennant.
- **Pennant**: four small choices, each from a fixed list Battle draws: shape (pennon,
  swallowtail, banner), field colour (8 of the notebook's inks and cloths), a symbol (about 12:
  acorn, leaf, sun, moon, star, shell, mushroom, flower, wave, key, feather, bell) and a trim
  (plain, stitched, fringed). Stored as short ids; an id Battle no longer knows is drawn as the
  default rather than refused.
- **Trios**: up to 12, each with its own id, a name (32 characters), its three creature ids, an
  optional pennant of its own (otherwise the Handler's), when it was formed, and its record:
  played, won and drawn against friends and against the visitors, and which cups it won.
- **Recognition is automatic.** When the three picked are exactly a registered trio's members,
  the match is played as that trio. Nothing is restricted: any three still play normally, and a
  trio gives no advantage.
- **Changing a trio's members does not rewrite its past** (Jake chose swaps over fixed
  members). Picking two of a trio's three with someone new offers to put the new one in; the
  trio keeps its name and one continuing record, and the three it had before are kept as earlier
  lineups with the day each ended, so a past match is never credited to someone who was not
  there. Renaming is free, because the record belongs to the trio's id, not its name.
- **Missing companions**: a trio with someone no longer in the colony stays in the list with that
  place shown empty; it cannot be fielded, and its record stays.

How it travels: `Rewards` gains the trio a match was played as, and `BattleRecord::apply` adds
the match to that trio's record (both kinds of match; it is a record, not a reward, so Open
Circuit points stay friend-only and cups stay visitor-only). Edits to the Handler card and trios
travel in `BattleResult` as the card and trio definitions as they stand, applied before the
session's matches; Desktop keeps the records itself, so an edit can never change a tally. Contract
version 3; Desktop's save goes to v32 with nothing to migrate (absent means "not set up yet").

## Decisions (Jake, 2026-10-16)

Phase 1 goes into 0.67.9 with the Desktop change; visitor matches count on a trio's record,
kept apart from friend matches; a trio's members may be swapped, with one continuing record.

## Phase 1 for 0.67.9: the foundation

Built on `battle-handler-identity`, with Desktop's `desktop-battle-handler` (contract version 3,
save version 32) and the site relay's protocol 3 card. One difference from the table: the trio
pennant is the Handler's pennant in a colour of the trio's own, not a separate design.

| Piece | Where it shows |
|---|---|
| A Handler page (name, club, pennant, standing) | A new first page in History & settings, so the station gains no button |
| Trios: name the three picked, re-form, retire, rename | The Who page: three picked that are not a trio get "Name these three"; a registered three shows its pennant and name |
| Handler standing (Jake's reputation idea in its simplest form): a title that rises with Open Circuit points, and a mark for each cup won, with "Champion of the Circuit" once all four are | The Handler page, the masthead ribbon and a friend's introduction. It says how it is earned, beside it |
| Trio records | The Handler page's trio list, and each trio's line in History |
| Identity in a match | Your pennant and trio name in the top bar; a friend's name, pennant and trio name across from yours |
| Identity in the results | "The Thistle Three: 7 played, 5 won" under the result, and journal lines that name the trio ("The Thistle Three won a Backyard Cup match against the visitors.") |
| Friend matches | The entrant carries club, pennant, standing and trio name and pennant, all bounded and cleaned by the relay and by each window, and listed in the shared notice |
| Tests | Contract round trips and bounds, apply-once for trio records, old records and old results still read, a trio whose member is gone, re-forming keeps the old record, relay entrants with and without the new fields, every new field cleaned |
| Docs | This file, REWARDS.md, FRIENDS.md, RELAY.md and the changelog |

What 0.67.9 needs from elsewhere: the Desktop contract and save change (one Desktop PR, in the
0.67.9 release), and the site relay passing the new optional entrant fields through (the
"Battle friend-match relay" thread, before protocol 3 ships). Phase 1 touches pages the menus
thread and the match screen thread own; new code goes in new files (`pennant.rs`,
`app/registry.rs`, `identity.rs`), with small hooks in theirs, agreed through the coordinator.

## Later phases

- **Phase 2, recognition.** Distinctions earned from what the match record shows: for a trio,
  a cup won together, the beacon lit together, a comeback (behind on the tally at the halfway
  turn and won), flanking together often, many matches together; for the Handler, wins with
  several different trios, every arena won on. A handful, each once, shown as small stitched
  badges, two pinned to the profile. Individual companions keep their notebook moments. Needs
  one small contract addition (a trio's distinctions).
- **Phase 3, emergent identity.** Reputations from observed play over the last matches (lights
  the beacon, flanks, holds the line, pulls levers), each shown with its evidence and allowed to
  fade. Rematches and rivalries need a way to know a friend again; the honest options are their
  typed name and pennant (easily changed) or a new anonymous Handler key, which would be a new
  shared field and a privacy decision for Jake.
- **Phase 4, beyond Battle.** Desktop's journal naming trios, and Home showing a pennant beside
  the trophies. Both are small once Phase 1's record exists, but each is a Desktop or Home change.
