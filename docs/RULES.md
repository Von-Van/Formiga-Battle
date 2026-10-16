# Formiga Battle rules

How a match of Formiga Battle resolves, as `formiga-battle-core` plays it. Every number here lives
in `Ruleset` (`crates/formiga-battle-core/src/ruleset.rs`), travels with every recorded match, and
is provisional until playtests settle it. `RULES_VERSION` is 6.

## A match

Two teams of three companions on an arena of flat tiles. Each turn, each Handler gives each of
their creatures one order; when the timer runs out, any order not given (or one that cannot be
carried out at all) is filled by the fallback, the same for both teams. Both teams' orders are
locked together and the turn resolves in `ticks_per_turn` ticks.

| Setting | Value | What it means |
|---|---|---|
| `composure` | 3 | The same for everyone. Zero is a cartoon knockout: off the board for this match only |
| `ticks_per_turn` | 3 | Every creature takes at most one step a tick |
| `capture_ticks` | 3 | Ticks of controlling the beacon, in total, that win: one whole turn's worth |
| `capture_rule` | `uncontested` | See [The beacon](#the-beacon) |
| `capture_decays` | false | Whether one team's progress falls while the other controls the beacon |
| `turn_cap` | 12 | The last turn |
| `damage_cap` | 2 | The most composure one creature can lose in one tick |
| `detour_budget` | 2 | Steps a creature may take in a turn that bring it no closer |
| `turn_seconds` | 40 | For clients and the room service; the rules never look at a clock |

## The orders

| Order | Intent | Target |
|---|---|---|
| MOVE | Reach a tile | A tile. Walks round anyone standing still. Open to a contest the whole turn, arrived or not |
| HOLD | Defend a tile | A tile. Walks there, then stands. If someone else is standing on it, holds beside it instead |
| CONTEST | Disrupt | A creature of the other team, a lever or a tap. Walks up next to it (one of the four tiles touching it) and settles once a turn |
| USE | Use what it carries | A tile. Only for a creature carrying something it found (see [Found things](#found-things)). Walks until the tile is within the item's reach, uses it there once a turn, and the item is gone |

There are no move sets, abilities, power ratings or Battle stats. Every creature has the same
composure, the same pace and one order a turn. USE comes from something lying on the board,
which any creature can pick up, never from the creature.

## A tick

Every tick works from one shared picture of the board. Nobody moves first.

1. A gate whose swing was telegraphed last tick swings. Then whatever was marked to land this
   tick lands: a leaf umbrella or a twig whose time is up folds or falls, things glinting since
   last turn turn up (on a turn's first tick), items used last tick take effect, and the map event
   comes.
2. Contests already next to their target are settled, so a creature walking away from a contester
   beside it is caught before it can step.
3. Each creature still carrying out its order picks one step (see [How a creature walks](#how-a-creature-walks)).
4. The steps happen together. Anyone with empty paws who stepped onto something lying there
   picks it up.
5. Anyone who has reached the tile it was told to hold starts holding.
6. Contests that have just come into reach are settled. A USE, a pull or a turn of a tap in reach
   is settled with them.
7. The beacon counts.
8. The match ends by knockout, then by capture.

Composure is taken as each contest is settled, never more than `damage_cap` from one creature in a
tick. A creature at zero is knocked out and leaves the board at once.

## Encounters

| Encounter | Rule |
|---|---|
| CONTEST vs anyone not holding | Interrupted: it stops where it is for the rest of the turn and loses one composure for each contester (capped) |
| CONTEST vs HOLD | Countered: the holder, and each holder of its team beside it corner to corner that a contester has reached, costs one contester one composure, at most once a tick, spread so no contester takes a second before every other has taken one. The contesters are turned away |
| CONTEST vs HOLD from two sides | Flanked, when the contesters outnumber the holders turning them away: see [Encirclement](#encirclement) |
| CONTEST vs CONTEST | A scuffle: a contest that reaches someone busy with a contest of their own that tick (a creature or a lever) hurts nobody. The busy one's own contest still settles |
| MOVE vs HOLD | The mover walks round, if there is a way round. Nobody is hurt |
| Two wanting one tile | A fair draw, seeded and the same for both teams, lets one in; the other is bumped |
| Two wanting to swap | Neither moves |
| A step into a tile its creature did not leave | The step fails, and so does any step into that one's tile |
| Bumped | Until it steps again, others plan round it as if it were standing still, and it plans round them, so two meeting in a narrow way stop trying rather than bumping every tick |

A contest is settled once a turn, whatever comes of it. On the way to a target three or more tiles
off, a creature may instead take a contest against someone not holding who walks into reach (see
`opportunism` below).

## Encirclement

A holder can be broken without being worn down: by contesters coming at it from two or more of
its four sides at once.

- **Flanked.** When two or more contesters settle on one holder in the same tick and they
  outnumber the holders turning them away (the holder itself, and any holder of its team beside
  it, corner to corner, that a contester has reached), the hold breaks. The holder loses one
  composure, however many came at it, and nobody is turned away.
- **Pushed.** It is pushed one tile straight away from one of its contesters, onto a free tile out
  of the zone if any of those ways leads out of it; ties go to the upper tile, then the one
  nearer its own side of the board. If every way out is blocked (two contesters on opposite
  sides block each other's), it is knocked off balance where it stands.
- **Off balance.** Either way it stops for the rest of the turn, is not holding, and does not count
  in the zone. Next turn it can hold again.
- **Side by side.** Two holders corner to corner share the two tiles they both touch: a
  contester on either is turned away by both, so it takes three to break one of them.
- **Already in place.** Contests already beside their target settle before anyone steps, so
  flankers who were in place when the turn began break the hold before the holder can step
  away. Getting into place is the work: it takes MOVE (or HOLD on the flanking tile), and on the
  way a flanker that is not yet holding can be caught by a single contest.

The counterplay is the defender's: hold beside a friend, contest a flanker on its way, or move
before the flankers are in place. Being flanked costs a little composure; what it really costs is
the zone.

`ai::flank_threatened` says whether a holder would be flanked if everyone of the other team beside
it contested it now, and `ai::flank_places` finds two sides to flank it from.

## The beacon

The zone (`Z` tiles) is the beacon. At the end of every tick:

- `uncontested` (the default): a team gains a tick of progress when it has someone holding in the
  zone and nobody of the other team stands in it unbroken. Progress pauses as soon as the other
  team gets someone into the zone and keeps them there; a creature knocked off balance, flanked or
  caught on the move, no longer counts.
- `majority`: a team gains a tick of progress when more of its creatures are holding in the zone
  than the other team has in it unbroken, holding or not.

Walking through the zone is not holding it. Reaching `capture_ticks` wins at once.

Before rules version 4 the default was `majority`, because under `uncontested` both teams nearly
always had someone in the zone and almost nothing was won by capture. Flanking gives a team a way
to clear the zone, and `majority` lets a team that only ever holds win before anyone can flank it,
so version 4 counts it the handoff's way.

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
| Beacon (`Z`) | HOLD in the zone | Capture progress | Progress shown to both teams | Stand in the zone unbroken to pause it; flank its holders to clear it |
| Gate lever (`L`, gates `G`/`g`) | CONTEST from a tile touching the lever | Every gate swings `swing_ticks` later (at least 1), then the lever rests `cooldown_turns` whole turns | The pull, and the tick the gates will swing, are an event both teams see | HOLD on a tile touching the lever: a pull is turned away like a contest, one composure for each such holder. A gate cannot close on someone standing in it, or on a twig |
| Map event (`event:` and its tiles) | Comes round by itself on `event_first` and every `event_every` turns after | Its effect on everyone standing on its tiles | Its tiles are marked a whole turn before, from the end of the turn before it comes; it lands on that turn's second tick | Stand elsewhere when it comes, or under a leaf umbrella |
| Tap (`S`) | CONTEST from a tile touching it | The map event lands a tick later; then every tap rests `switch_rest` whole turns | The turn, and the tiles, are an event both teams see | HOLD on a tile touching the tap, as at the lever. A tap does nothing while the event is already coming |
| Item spot (`*`) | Step onto what turns up there | See [Found things](#found-things) | It glints a whole turn before | Get there first, or knock it out of the carrier's paws |

## The map event

An arena may have one map event. Its effects are ones a match already has, and none costs
composure.

| Effect | What happens to anyone on its tiles when it lands |
|---|---|
| Soak | Off balance, as when boxed in by a flank: it stops for the rest of the turn, stops holding, does not count in the zone, and drops what it carries. Under a leaf umbrella it stays dry |

Backyard's is a sprinkler that soaks the whole yard on turns 4, 8 and 12, with a tap on each side
of the near edge. It clears the yard for both teams alike: what it rewards is having someone
waiting just outside to walk in, or a leaf umbrella over a holder. While it swept only the yard's
four arms, whoever held the middle when it came gained two ticks for nothing, and 98% of 10,000
Exhibition matches were won at the beacon in about 7 turns.

## Found things

Things turn up on an arena's item spots, which come in pairs mirrored across the board, on turn 2
and every three turns after. At the end of the turn before, a seeded draw picks a pair whose
spots are clear and one item, and the same item glints on both spots for both teams to see.
Puffs turn up 40% of the time, twigs 35% and leaf umbrellas 25%.

A creature that steps onto something with empty paws picks it up, whatever its order; one thing
at a time, and both teams can see who carries what. Caught on the move, flanked, soaked, blown
over or knocked out, it drops what it carries where it ends up (or loses it, if something already
lies there). A USE is marked on its tiles the tick it is used and lands at the start of the next.
Nothing found costs composure, nothing outlasts the match, and the fallback never uses anything.

| Item | Reach | What USE does | Lasts |
|---|---|---|---|
| Dandelion puff | 2 | Blows across the tile and the two either side of it, across the way from its user to the tile (along whichever way is further; across the board when level). Everyone there is pushed one tile on and knocked off balance, or knocked off balance where it stands if the way on is blocked. A holder with a holder of its team beside it, corner to corner, braces and is not moved. Twigs there blow over | Lands and is gone |
| Twig | 2 | Wedges into the tile, if nobody and nothing is there and anyone could stand there: nobody can stand there, and a gate cannot close on it | To the end of the turn two after it lands |
| Leaf umbrella | 1 | Opens over whoever of its user's team stands on the tile, itself included: the map event cannot soak it, and a contest against it while it is not holding bounces off, hurting nobody. A puff catches it like a sail and carries it two tiles | To the end of the next turn |

Things meet each other and the board in a few fixed ways: a leaf keeps off the sprinkler, a puff
blows twigs over and carries a leaf two tiles, a twig wedged in an open gate keeps it open, and a
puff can blow someone onto ground marked for the sprinkler just before it lands.

## Arenas

Arenas are text files in `arenas/` (format in `crates/formiga-battle-core/src/arena.rs`). The
first, Backyard, is 9 by 7 and the same for both teams, mirrored left to right. Its yard is a
cross: an arm has open ground on three sides, so a holder on it can be flanked and pushed out of
the yard; the middle can be flanked only from the arms.

```text
* . . # G # . . *
1 . . . . . . . 2
. . # . Z . # . .
1 . . Z Z Z . . 2
. . # . Z . # . .
1 . * . . . * . 2
. S . . L . . S .
```

Before rules version 4 the yard was the whole 3 by 3 square. With all six creatures fitting in it,
holders could always be found to keep it contested, and a flank could rarely push anyone out:
in 3,000 Exhibition matches on the square, 14% were won by capture, and Wallflowers scored 0.413.

## How a creature walks

A creature never chooses whether to obey: its order sets where it is going and what it does there.
Its profile picks the way, and what it does with a moment to spare.

Each tick it looks ahead over the steps left in the turn (at most three), at every way that
spends at most its remaining detour budget, and scores each whole way: what every tile on it is
like to this creature, counted four times over, less 220 for each step it would arrive later
than the quickest way. Danger
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
| Marked for something about to land (the map event, or a puff used last tick) | −caution / 2, for a cautious creature; nothing for a bold one, which walks straight on and takes its chances |
| Something lying there, with empty paws | +(wanderlust + opportunism) / 2, counting only leanings toward it: a curious or playful creature goes out of its way for it, one that keeps to its way does not |

Counting a way's tiles four times over is the pull (`PULL` in `interpret.rs`, 400%), set so that
each turn about one creature of a team's three does something of its own: in 200 Exhibition
matches, 27% of team-turns had none, 46% one and 26% two, and 39% of steps were not the plain
way. Only about one step in ten has two ways there that arrive equally soon, so beyond that every
way a profile chooses costs time. At 100%, under rules version 4, 79% of team-turns had none. It weighs only the way there: what a creature does with
a moment to spare is weighed as the table says.

Ties are broken by a seeded draw. Now and then (`variability`, 1% to 6%) it takes its second-best
choice instead, if both are on time. A wary creature (caution 150 or more) sent to walk waits one
tick, once a turn, rather than take a step that puts it within reach of more of the other team
than it is now; told to hold or contest, it goes anyway. Once arrived with ticks to spare it may
take one follow-up step beside a friend or out of reach, if that beats its patience; when its
contest is turned away, a creature that `steps back` takes one step out of reach.

Both of those were wider before rules version 4: a wary creature hung back when sent to hold too,
and one that steps back did so after every contest. With flanking, both meant walking away from
the place it had been sent to take, and Wallflowers scored 0.39 to 0.43. Every choice that is not the plain way has a reason the
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
| Left side | 0.497 |
| Kinds, highest to lowest | Guardian 0.545, Lazybones 0.542, Grump 0.537, Troublemaker 0.523, Sweetheart 0.518, Explorer 0.481, Showoff 0.474, Oddball 0.461, Wallflower 0.448, Scholar 0.443 |
| Spread between kinds | 0.102, each kind over 3,600 to 7,000 matches |
| Won by capture, knockout, at the cap, drawn | 25%, 0%, 63%, 12%, in 10.8 turns |

With both teams coached by `familiar` (`--coach familiar,familiar --pairings 1500 --seed 7`),
kinds score within 0.078 of each other, Wallflowers lowest; 52% of those matches are won by
capture and 1% by knockout, in about 9.5 turns.

Rules version 6 added Backyard's sprinkler and found things. Before it, under version 5, 13% of
Exhibition matches were won at the beacon and the kinds were 0.098 apart; with familiar coaching,
24% were won at the beacon. Most of the change is the sprinkler: with found things alone, 21% of
1,000 pairings were won at the beacon, the kinds 0.096 apart. A sprinkler that clears the yard is
a moment a team can plan for, and coaches that think a turn ahead plan for it best.

Rules version 5 made the pull four times as strong, and the spread grew with it (0.036 under
version 4, when Wallflowers scored 0.480, 21% of matches were won by capture and familiar
coaching kept kinds within 0.037): every way a profile chooses past the first one in ten costs
time, and the kinds that lean hardest pay most for it. Jake chose this on 2026-10-16, for about
one creature in three doing something of its own each turn.

Most matches now go to the turn cap. With good holding on both sides the zone stays contested,
and the cap's order (beacon progress, then composure, then creatures standing) decides them;
flanks are what move composure. Every tie between equal tiles, in the rules and in the coaches,
is broken from each team's own side of the board, so the board plays the same from both sides.

Before rules version 2, under strong coaching, a bold creature went out of its way to walk into
the other team's reach, and a social one detoured to stay beside its teammates because company
counted on every tile of a long way round: Grumps scored 0.554 and Explorers 0.459. Both now
have to be paid for like any other preference.

## Holding still

The `turtle` coach is the strategy encirclement was made against: every creature walks to a tile
of the zone and holds it for the rest of the match. Over 2,000 matches each (`--coach
<coach>,turtle --pairings 1000 --seed 3`):

| Against turtle | Score | How |
|---|---|---|
| exhibition | 0.608 | 67% won or lost at the cap, 22% drawn |
| familiar | 0.800 | 50% at the cap, 48% by capture |
| generic | 0.662 | 48% at the cap, 51% by capture |

Three holders in a five-tile cross keep it contested for as long as they stand, so they are
almost never beaten by capture; they are beaten on the cap, because a team that only holds never
stops anyone from standing in the zone too and is flanked turn after turn. Under rules version 3
(`majority`, no flanking) the turtle beat the Exhibition coach 99.9% of the time and the familiar
coach 83%.

## Knowing your creatures

The `familiar` and `generic` coaches (`plan.rs`) think each turn through the same way: for each
creature, try each sensible order, play the turn out in their head against what they expect the
other team to do, and keep whatever leaves the board best. The familiar coach imagines its
creatures as they are; the generic one imagines every creature as one that leans no way at all.

| Coaches | Familiar's score |
|---|---|
| familiar against generic, 6,000 matches | 0.621 |
| familiar against Exhibition, 2,000 matches | 0.966 |
| generic against Exhibition, 2,000 matches | 0.857 |

Knowing your creatures is worth more under rules version 5 than it was: the coach who knows them
wins 61.6% against one who plans as well without knowing them (59.0% under version 4, 52.4%
under version 3), because so many more of its creatures' steps are their own. A flank
lands only if both flankers are in place at the start of a turn, so knowing who will wander, who
hangs back and who goes the long way round decides whether a flank lands. It shows most with the
creatures that differ most from a plain one: under version 4 the generic coach scored 0.426 with its Wallflowers
against Exhibition, while the familiar coach scored 0.503 with them.

Under version 3, most of it came from hanging back: a coach who knows which of its creatures are
wary plans for the tick they lose, or sends them to contest instead. It was 51.1% in rules
version 2, before wary creatures hung back, and 53.1% in version 1, when most of it came from
bold creatures walking into reach, which also made some kinds stronger than others.
