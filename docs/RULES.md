# Formiga Battle rules

How a match of Formiga Battle resolves, as `formiga-battle-core` plays it. Every number here lives
in `Ruleset` (`crates/formiga-battle-core/src/ruleset.rs`), travels with every recorded match, and
is provisional until playtests settle it. `RULES_VERSION` is 7.

## A match

Two teams of three companions on an arena of flat tiles. Each turn, each Handler gives their
whole team up to three orders (see [Team orders](#team-orders)), and the team shares them out
among itself: nobody is told who goes where. Both teams' plans are locked together and the turn
resolves in `ticks_per_turn` ticks. A Handler who gives no orders before the timer runs out
leaves each creature to the fallback, the same for both teams.

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

## Team orders

A Handler's plan is up to `PLAN_SIZE` (three) orders, in any mix, repeats allowed:

| Order | Aimed at | What the team does |
|---|---|---|
| TAKE | One to five squares | Gets them and holds them: someone walks onto an empty one and holds it; one of theirs standing on one is contested, and one of theirs holding there is flanked by two |
| DEFEND | One to five squares | Keeps them: someone holds each, and whoever is free guards beside one or heads off one of theirs coming at it. A lever or a switch named is guarded from beside it |
| INTERACT | One thing | A found thing on the ground or glinting is fetched; the lever or the arena's switch is pulled (or, while it rests, waited by); one of theirs carrying something is contested, so it drops it; one of ours carrying something uses it |

An order with nothing it can be aimed at (a square off the board or in a wall, an INTERACT with
nothing there) is left out, as is a fourth order and beyond (`OrderDropped`), so a room never has
to trust what a client sends.

### Sharing out the plan

At the start of the turn, from one picture of the board, each team turns its plan into jobs
(`muster`). A square named in one order is worth `NEED` (1000) to the team; named in two orders,
twice that, and it brings a guard beside it, a second guard if it is named in three. Taking a
square one of theirs holds is two jobs, a side each, worth a square apiece; with only one of them
taken it is worth `LONE` (450). Whoever is left over guards a square of the plan (more for a
DEFEND than a TAKE), heads off one of theirs coming at a square to defend, picks up something
lying within four steps, or uses what it carries where that plainly serves the plan.

Each creature weighs the jobs it could do: the job's worth, less `STEP` (90) for every step it
would walk, plus `ON_TIME` (200) if it can be there this turn and `STICK` (80) if it is already
there or already doing that job; a job on ground marked for the map event or a puff is worth 150
less to stand on. Then, last, who it is (see [PERSONALITY.md](PERSONALITY.md)), never more than
`LEANING_CAP` (180, about two steps) either way. The team takes the sharing out worth the most in
all, each creature weighing its best eight. Two jobs that stand on one tile are never both taken,
and one of ours already holding a square of the plan keeps it: swapping holders would leave it
empty while one walks off and the other walks on.

Each job becomes one of the four orders below for the creature that took it. A flank's two sides
walk into place and hold there; once both stand in place at the start of a turn, both contest
together. Alone where two were needed, a creature much keener to meet the other team than it is
wary (`RECKLESS`, 150) goes at the holder anyway and is likely turned away; any other stands beside
it, keeping the square contested, until help comes.

Every creature's job is an event (`Assigned`): the job, the order it comes to, and why it was this
creature rather than the one a creature with no leanings would have been (nearest, already at it,
bold, careful, beside a friend, away from one it avoids, curious, on a whim, alone).

### Thinking again

After whatever lands at the start of each tick, a creature whose job has just come to nothing
thinks again from the same plan, once a turn, keeping to jobs nobody else of its team is on
(`Rethought`): what it went for was picked up or is gone; it picked up what it went for; the one it
went after was knocked out, or started holding before it got there; the one of theirs on its
square stepped off; someone got to its square first; its ground was marked for something about to
land; a teammate was knocked out and left its job undone; something turned up nearby while it had
nothing to do. A contest already spent this turn is not taken again. Thinking again about ground
about to be hit, a wary creature finds another job and a bold one often stays.

## The orders each creature carries out

| Order | Intent | Target |
|---|---|---|
| MOVE | Reach a tile | A tile. Walks round anyone standing still. Open to a contest the whole turn, arrived or not |
| HOLD | Defend a tile | A tile. Walks there, then stands. If someone else is standing on it, holds beside it instead |
| CONTEST | Disrupt | A creature of the other team, a lever or a tap. Walks up next to it (one of the four tiles touching it) and settles once a turn |
| USE | Use what it carries | A tile. Only for a creature carrying something it found (see [Found things](#found-things)). Walks until the tile is within the item's reach, uses it there once a turn, and the item is gone |

There are no move sets, abilities, power ratings or Battle stats. Every creature has the same
composure, the same pace and one job a turn. USE comes from something lying on the board,
which any creature can pick up, never from the creature.

## A tick

Every tick works from one shared picture of the board. Nobody moves first.

1. A gate whose swing was telegraphed last tick swings. Then whatever was marked to land this
   tick lands: a leaf umbrella or a twig whose time is up folds or falls, things glinting since
   last turn turn up (on a turn's first tick), items used last tick take effect, and the map event
   comes. Anyone whose job has just come to nothing thinks again (see [Thinking again](#thinking-again)).
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
| Switch (`S`): a tap, a shell, a sluice, a tree | CONTEST from a tile touching it, or a pebble bell rung beside it | The map event lands a tick later; then every switch rests `switch_rest` whole turns | The turn, and the tiles, are an event both teams see | HOLD on a tile touching the switch, as at the lever. A switch does nothing while the event is already coming |
| Item spot (`*`) | Step onto what turns up there | See [Found things](#found-things) | It glints a whole turn before | Get there first, or knock it out of the carrier's paws |

## The map event

An arena may have one map event. Its effects are ones a match already has, and none costs
composure.

| Effect | What happens to anyone on its tiles when it lands |
|---|---|
| Soak | Off balance, as when boxed in by a flank: it stops for the rest of the turn, stops holding, does not count in the zone, and drops what it carries. Under a leaf umbrella it stays dry |
| Wash | Pushed one tile onto the free open ground beside it nearest the board's middle row, off the event's tiles (upper first among equals, then nearer its own side), then off balance as for a soak; off balance where it stands if there is nowhere to go. Under a leaf umbrella it stays put |
| Close | The tiles go under for the rest of the turn: nobody can stand there, and whatever lay or stood wedged there floats off. Anyone on them is washed off first, never onto the yard; a leaf umbrella does not help |

| Arena | Event | Effect | Tiles | By itself | Switches |
|---|---|---|---|---|---|
| Backyard | The sprinkler | Soak | The whole yard | Turns 4, 8 and 12 | A garden tap on each side of the near edge |
| Sandy Cove | A wave | Wash | The sandbar's two ends and the sand either side of them | Turns 3, 6, 9 and 12 | A big shell on each side of the near edge |
| Millpond | The pond rising | Close | The four banks either side of the jetty, from which its sides are come at | Turns 5 and 10 | A sluice on each bank |
| Woodland Glade | A tree shaken | Soak, and a twig falls beside the tree | The open ground touching the tree | Turns 4, 8 and 12, on a pair of trees across from each other drawn at random | Each of the four old trees by the lanes (`event_tiles: around`) |

Backyard's is a sprinkler that soaks the whole yard on turns 4, 8 and 12, with a tap on each side
of the near edge. It clears the yard for both teams alike: what it rewards is having someone
waiting just outside to walk in, or a leaf umbrella over a holder. While it swept only the yard's
four arms, whoever held the middle when it came gained two ticks for nothing, and 98% of 10,000
Exhibition matches were won at the beacon in about 7 turns.

## Found things

Things turn up on an arena's item spots, which come in pairs mirrored across the board, on turn 2
and every three turns after. At the end of the turn before, a seeded draw picks a pair whose
spots are clear and one item, and the same item glints on both spots for both teams to see.
Puffs turn up 26% of the time, twigs 22%, leaf umbrellas 18%, pebble bells 16%, honey 12% and
springy seeds, the one that only moves its user, 6%.

A creature that steps onto something with empty paws picks it up, whatever its order; one thing
at a time, and both teams can see who carries what. Caught on the move, flanked, soaked, blown
over or knocked out, it drops what it carries where it ends up (or loses it, if something already
lies there). A USE is marked on its tiles the tick it is used and lands at the start of the next.
Nothing found costs composure, nothing outlasts the match, and the fallback never uses anything.

| Item | Reach | What USE does | Lasts |
|---|---|---|---|
| Dandelion puff | 2 | Blows across the tile and the two either side of it, across the way from its user to the tile (along whichever way is further; across the board when level). Everyone there is pushed one tile on and knocked off balance, or knocked off balance where it stands if the way on is blocked. A holder with a holder of its team beside it, corner to corner, braces and is not moved. Twigs there blow over | Lands and is gone |
| Twig | 2 | Wedges into the tile, if nobody and nothing is there and anyone could stand there: nobody can stand there, and a gate cannot close on it | To the end of the turn two after it lands |
| Leaf umbrella | 1 | Opens over whoever of its user's team stands on the tile, itself included: the map event cannot soak or wash it, and a contest against it while it is not holding bounces off, hurting nobody. A puff catches it like a sail and carries it two tiles | To the end of the next turn |
| Pebble bell | 2 | Rings where its user stands: everyone of the other team within two tiles who has not settled a contest (or a pull, or a turn of a switch) this turn loses it. Rung beside a switch that is ready, it sets the map event off | Lands and is gone |
| Honey drop | 2 | Spills over the tile and the four round it, wherever anyone could stand: whoever steps in, or is already there, is stuck where it stands for the rest of the turn, and can still hold | To the end of the turn two after it lands |
| Springy seed | 2 | Hops its user onto the tile, if it is free ground: over anyone or a twig in between, never over a wall, an obstacle or water | Lands and is gone |

Things meet each other and the board in a few fixed ways: a leaf keeps off the sprinkler, the
acorns and the wave; a puff blows twigs over and carries a leaf two tiles; a twig wedged in an open
gate keeps it open, and thrown into the millpond floats as a raft anyone can stand on until the
end of the next turn; a pebble bell rung by a switch sets the event off; honey stops a creature
pushed onto it by a puff or a wave, and a twig will not stand in it; a puff can blow someone onto
ground marked for the event just before it lands.

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
like to this creature, counted twice over, less 220 for each step it would arrive later
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
| Marked for something about to land (the map event, or a puff or honey used last tick), or sticky with honey | −caution / 2, for a cautious creature; nothing for a bold one, which walks straight on and takes its chances |
| Something lying there, with empty paws | +(wanderlust + opportunism) / 2, counting only leanings toward it: a curious or playful creature goes out of its way for it, one that keeps to its way does not |

Counting a way's tiles twice over is the pull (`PULL` in `interpret.rs`, 200% since rules version
7), set so that each turn about one creature of a team's three does something of its own: in 300
Exhibition matches, 28% of the creatures that walked somewhere took a way of their own or hung
back, about 0.8 a team-turn. Under rules versions 5 and 6 it was 400%, which gave the same with an
order for each creature; with team orders, creatures walk further and more often, and at 400% it
became nearly one in two (1.5 a team-turn), and the kinds fell 0.143 apart under thinking coaches
(0.057 at 200%). Only about one step in ten has two ways there that arrive equally soon, so beyond that every
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
| Left side | 0.495 |
| Kinds, highest to lowest | Lazybones 0.542, Guardian 0.531, Grump 0.529, Scholar 0.527, Explorer 0.504, Wallflower 0.497, Sweetheart 0.480, Troublemaker 0.475, Oddball 0.457, Showoff 0.437 |
| Spread between kinds | 0.105 (0.099 with seed 8), each kind over 3,600 to 7,000 matches |
| Won by capture, knockout, at the cap, drawn | 28%, 0%, 50%, 22%, in 10.3 turns |

With both teams coached by `familiar` (`--coach familiar,familiar --pairings 1500 --seed 7`),
kinds score within 0.155 of each other, Guardians highest (0.557) and Wallflowers lowest (0.402);
55% of those matches are won by capture, in about 9.1 turns.

On every arena, over 4,000 Exhibition-coached matches each (`--pairings 2000 --seed 7 --file
arenas/<arena>.arena`), with seed 8 in brackets:

| Arena | Left side | Spread between kinds | Won by capture |
|---|---|---|---|
| Backyard | 0.496 | 0.089 | 26% |
| Sandy Cove | 0.498 | 0.143 (0.151) | 66% |
| Millpond | 0.494 | 0.135 (0.136) | 40% |
| Woodland Glade | 0.495 | 0.143 (0.097) | 39% |

Knowing your creatures (`--coach familiar,generic --pairings 1000 --seed 7`) is worth 0.642 on
Backyard, 0.627 on Sandy Cove, 0.636 at the Millpond and 0.558 in the glade.

Rules version 7 replaced an order for each creature with three orders for the team. Under
version 6, with an order each, Exhibition matches put the kinds 0.108 apart on Backyard, 0.107 on
Sandy Cove, 0.133 to 0.141 at the Millpond and 0.123 to 0.152 in the glade; familiar coaching kept
them within 0.078, and knowing your creatures was worth 0.55 to 0.62. With team orders, the
Exhibition spread is about where it was, a little closer on Backyard and a little wider on Sandy
Cove. Under thinking coaches it doubled: a coach can no longer send a creature that wanders, hangs
back or keeps to its friends exactly where its way of walking costs least, so who is in the team
counts for more. The leanings that pick who takes which job are not what does it: with them taken
out of the sharing out (`LEAN_PERCENT` at 0), familiar self-play over 1,600 matches puts the kinds
0.170 apart, against 0.143 with them in. What is left is how each creature walks (rules version 5),
which now shows more because nobody can be steered round it, and why knowing your creatures is
worth a little more than it was. More matches are also drawn: 22% of Exhibition matches on
Backyard, against 12%.

The numbers above are with the pull at 400%. With it at 200% (this branch), so that again about
one creature in three goes its own way each turn: over 10,000 Exhibition matches on Backyard the
kinds are 0.069 apart (Scholars highest at 0.538, Troublemakers and Showoffs lowest at 0.469); on
the other arenas 0.113 on Sandy Cove, 0.069 at the Millpond and 0.117 in the glade; under familiar
coaching 0.057 (1,600 matches, seed 9). Knowing your creatures is worth 0.629 and Exhibition beats
the turtle 0.661.

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

The `turtle` coach is the strategy encirclement was made against: it DEFENDs a tile of the zone
for every creature, so each walks to one and holds it for the rest of the match. A creature whose
tile was taken first goes after whoever took it, which is what DEFEND asks; under rules version 6,
when it gave each creature its own HOLD, the turtle never contested at all and Exhibition scored
0.608 against it. Over 2,000 matches each (`--coach
<coach>,turtle --pairings 1000 --seed 3`):

| Against turtle | Score | How |
|---|---|---|
| exhibition | 0.549 | 52% won or lost at the cap, 38% drawn, 10% by capture |
| familiar | 0.743 | 54% at the cap, 31% by capture, 15% drawn |
| generic | 0.624 | 53% at the cap, 32% by capture, 15% drawn |

Three holders in a five-tile cross keep it contested for as long as they stand, so they are
almost never beaten by capture; they are beaten on the cap, because a team that only holds never
stops anyone from standing in the zone too and is flanked turn after turn. Under rules version 3
(`majority`, no flanking) the turtle beat the Exhibition coach 99.9% of the time and the familiar
coach 83%.

## Knowing your creatures

The `familiar` and `generic` coaches (`plan.rs`) think each turn through the same way: for each of
the team's three orders, try each sensible order (each square of the zone or of theirs to TAKE,
each square of ours in it to DEFEND, each thing to INTERACT with), play the turn out in their head
against what they expect the other team to do, and keep whatever leaves the board best. The
familiar coach imagines its creatures as they are, so it knows who will take which job; the
generic one imagines every creature as one that leans no way at all.

| Coaches | Familiar's score |
|---|---|
| familiar against generic, 2,000 matches | 0.642 |
| familiar against Exhibition, 2,000 matches | 0.952 |
| generic against Exhibition, 2,000 matches | 0.834 |

Under rules version 7 the coach who knows its creatures wins 64.2% on Backyard (62.1% under
version 6, with an order for each creature): it cannot say who goes where, but it knows who will
take which job and how each will walk there. Knowing your creatures was worth more under rules
version 5 than before it: the coach who knows them
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
