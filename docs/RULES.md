# Formiga Battle rules

How a match of Formiga Battle resolves, as `formiga-battle-core` plays it. Every number here lives
in `Ruleset` (`crates/formiga-battle-core/src/ruleset.rs`), travels with every recorded match, and
is provisional until playtests settle it. `RULES_VERSION` is 3.

## A match

Two teams of three companions on an arena of flat tiles. Each turn, each Handler gives each of
their creatures one order; when the timer runs out, any order not given (or one that cannot be
carried out at all) is filled by the fallback, the same for both teams. Both teams' orders are
locked together and the turn resolves in `ticks_per_turn` ticks.

| Setting | Value | What it means |
|---|---|---|
| `composure` | 3 | The same for everyone. Zero is a cartoon knockout: off the board for this match only |
| `ticks_per_turn` | 3 | Every creature takes at most one step a tick |
| `capture_ticks` | 6 | Ticks of controlling the beacon, in total, that win |
| `capture_rule` | `majority` | See [The beacon](#the-beacon) |
| `capture_decays` | false | Whether one team's progress falls while the other controls the beacon |
| `turn_cap` | 12 | The last turn |
| `damage_cap` | 2 | The most composure one creature can lose in one tick |
| `detour_budget` | 2 | Steps a creature may take in a turn that bring it no closer |
| `turn_seconds` | 40 | For clients and the room service; the rules never look at a clock |

## The three orders

| Order | Intent | Target |
|---|---|---|
| MOVE | Reach a tile | A tile. Walks round anyone standing still. Open to a contest the whole turn, arrived or not |
| HOLD | Defend a tile | A tile. Walks there, then stands. If someone else is standing on it, holds beside it instead |
| CONTEST | Disrupt | A creature of the other team, or a lever. Walks up next to it (one of the four tiles touching it) and settles once a turn |

There are no move sets, abilities, power ratings or Battle stats. Every creature has the same
composure, the same pace and one order a turn.

## A tick

Every tick works from one shared picture of the board. Nobody moves first.

1. A gate whose swing was telegraphed last tick swings.
2. Contests already next to their target are settled, so a creature walking away from a contester
   beside it is caught before it can step.
3. Each creature still carrying out its order picks one step (see [How a creature walks](#how-a-creature-walks)).
4. The steps happen together.
5. Anyone who has reached the tile it was told to hold starts holding.
6. Contests that have just come into reach are settled.
7. The beacon counts.
8. The match ends by knockout, then by capture.

Composure is taken as each contest is settled, never more than `damage_cap` from one creature in a
tick. A creature at zero is knocked out and leaves the board at once.

## Encounters

| Encounter | Rule |
|---|---|
| CONTEST vs anyone not holding | Interrupted: it stops where it is for the rest of the turn and loses one composure for each contester (capped) |
| CONTEST vs HOLD | Countered: each holder touching a contester costs one contester one composure, at most once a tick, spread so no contester takes a second before every other has taken one. The contesters are turned away |
| Group pressure | When the contesters on one holder outnumber the holders countering them, that holder also loses one composure, once, however many more there are. Two contesters against one holder: the holder loses one and one contester loses one. Two against two holders side by side: each contester loses one and neither holder does |
| CONTEST vs CONTEST | A scuffle: a contest that reaches someone busy with a contest of their own that tick (a creature or a lever) hurts nobody. The busy one's own contest still settles |
| MOVE vs HOLD | The mover walks round, if there is a way round. Nobody is hurt |
| Two wanting one tile | A fair draw, seeded and the same for both teams, lets one in; the other is bumped |
| Two wanting to swap | Neither moves |
| A step into a tile its creature did not leave | The step fails, and so does any step into that one's tile |
| Bumped | Until it steps again, others plan round it as if it were standing still, and it plans round them, so two meeting in a narrow way stop trying rather than bumping every tick |

A contest is settled once a turn, whatever comes of it. On the way to a target three or more tiles
off, a creature may instead take a contest against someone not holding who walks into reach (see
`opportunism` below).

## The beacon

The zone (`Z` tiles) is the beacon. At the end of every tick:

- `majority` (the default): a team gains a tick of progress when more of its creatures are
  holding in the zone than the other team has in the zone, holding or not.
- `uncontested`: a team gains a tick of progress when it has someone holding in the zone and the
  other team has nobody in it at all.

Walking through the zone is not holding it. Reaching `capture_ticks` wins at once.

The handoff left the exact uncontested condition open. Under `uncontested`, both teams nearly
always have someone in the zone: in 10,000 Exhibition-coached matches, 1% were won by capture and
88% went to the turn cap. Under `majority`, 59% were won by capture, in 10.4 turns on average.

## The end

In this order, at the end of a tick:

1. Both teams knocked out together: a draw.
2. One team knocked out: the other wins by knockout.
3. A team at `capture_ticks`: it wins by capture. Only one team can gain progress in a tick.

After the last tick of `turn_cap` with no winner: more beacon progress wins, then more composure
left across the team, then more creatures standing. Anything level after that is a draw.

## Props

Props use the three orders and grant nothing permanent. Each is data in the arena file.

| Prop | Interaction | Effect | Telegraph | Counterplay |
|---|---|---|---|---|
| Beacon (`Z`) | HOLD in the zone | Capture progress | Progress shown to both teams | Outnumber the holders in the zone; group pressure |
| Gate lever (`L`, gates `G`/`g`) | CONTEST from a tile touching the lever | Every gate swings `swing_ticks` later (at least 1), then the lever rests `cooldown_turns` whole turns | The pull, and the tick the gates will swing, are an event both teams see | HOLD on a tile touching the lever: a pull is turned away like a contest, one composure for each such holder. A gate cannot close on someone standing in it |

## Arenas

Arenas are text files in `arenas/` (format in `crates/formiga-battle-core/src/arena.rs`). The
first, Backyard, is 9 by 7 and the same for both teams, mirrored left to right:

```text
. . . # G # . . .
1 . . . . . . . 2
. . # Z Z Z # . .
1 . . Z Z Z . . 2
. . # Z Z Z # . .
1 . . . . . . . 2
. . . . L . . . .
```

## How a creature walks

A creature never chooses whether to obey: its order sets where it is going and what it does there.
Its profile picks the way, and what it does with a moment to spare.

Each tick it looks ahead over the steps left in the turn (at most three), at every way that
spends at most its remaining detour budget, and scores each whole way: what every tile on it is
like to this creature, less 220 for each step it would arrive later than the quickest way. Danger
counts on every tile the way passes; company and the lane it keeps to count as the way's
average, so a long way round is never worth more company for being long. It takes the first
step of the best and looks again next tick.

| Part of a tile | Score |
|---|---|
| Within reach of the other team (not counting whom it is walking up to) | −caution for each creature of theirs, for a cautious creature; nothing for a bold one, which does not avoid them but does not seek them out either |
| In the zone while the other team is, when it is only passing through | −caution / 2, likewise |
| Beside a teammate | +togetherness / 2, +100 for a high bond (+50 medium), −200 for high avoidance (−100 medium) |
| How far out from the middle row | +wanderlust × rows out / half the board's height |
| Against a wall or obstacle | +caution / 4 for each, likewise |

Ties are broken by a seeded draw. Now and then (`variability`, 1% to 6%) it takes its second-best
choice instead, if both are on time. A wary creature (caution 150 or more) sent to walk or hold
waits one tick, once a turn, rather than take a step that puts it within reach of more of the
other team than it is now; told to contest, it goes anyway. Once arrived with ticks to spare it may take one follow-up
step beside a friend or out of reach, if that beats its patience; after its contest a creature that
`steps back` takes one step out of reach. Every choice that is not the plain way has a reason the
recap says in words.

`docs/PERSONALITY.md` says where caution, togetherness and the rest come from.

## Replay and desync

A `MatchRecord` is the rules version, ruleset, arena, seed, both rosters and every turn's orders as
sent, with the board's hash after each turn. Replaying it plays the match again and stops at the
first turn whose board differs. Every chance the rules take is a hash of the seed and where in the
match it is asked, and every value a profile contributes is a whole number, so the same record
replays the same on every machine.

## Fairness

`formiga-battle tournament` draws creatures the way Desktop draws a new companion, drifts their
leanings, puts them in random trios and plays each pairing from both sides of the board. Score is
wins plus half of draws, over matches played; with a few thousand matches a kind, about 0.01 of a
score either way is chance.

Over 10,000 Exhibition-coached matches with the default rules (`--pairings 5000 --seed 7`):

| | |
|---|---|
| Left side | 0.496 |
| Kinds, highest to lowest | Troublemaker 0.518, Grump 0.505, Showoff 0.503, Lazybones 0.502, Sweetheart 0.501, Scholar 0.497, Explorer 0.496, Guardian 0.494, Oddball 0.494, Wallflower 0.488 |
| Spread between kinds | 0.029, each kind over 3,600 to 7,000 matches |
| Won by capture, knockout, at the cap, drawn | 63%, 3%, 31%, 3%, in 10.2 turns |

With both teams coached by `familiar` (`--coach familiar,familiar --pairings 1500`, seeds 7 and
8), kinds score within 0.046 and 0.039 of each other. Wallflowers, the most cautious kind, are
lowest on both (0.471 and 0.478): about two points under even, as they were before hanging back
was added, and the one place to watch in playtests.

Before rules version 2, under strong coaching, a bold creature went out of its way to walk into
the other team's reach, and a social one detoured to stay beside its teammates because company
counted on every tile of a long way round: Grumps scored 0.554 and Explorers 0.459. Both now
have to be paid for like any other preference.

## Knowing your creatures

The `familiar` and `generic` coaches (`plan.rs`) think each turn through the same way: for each
creature, try each sensible order, play the turn out in their head against what they expect the
other team to do, and keep whatever leaves the board best. The familiar coach imagines its
creatures as they are; the generic one imagines every creature as one that leans no way at all.

| Coaches | Familiar's score |
|---|---|
| familiar against generic, 6,000 matches | 0.524 |
| familiar or generic against Exhibition | 0.997 and above |

Knowing your creatures is worth about five points between two equally careful coaches: the one
who knows them wins 52.4% and the one who does not 47.6%. Most of it comes from hanging back: a
coach who knows which of its creatures are wary plans for the tick they lose, or sends them to
contest instead. It was 51.1% in rules version 2, before wary creatures hung back, and 53.1% in
version 1, when most of it came from bold creatures walking into reach, which also made some
kinds stronger than others. Hanging back every tick rather than once a turn was worth 52.5% too,
but cost Wallflowers four points.
