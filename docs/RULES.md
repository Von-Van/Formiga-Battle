# Formiga Battle rules

How a match of Formiga Battle resolves, as `formiga-battle-core` plays it. Every number here lives
in `Ruleset` (`crates/formiga-battle-core/src/ruleset.rs`), travels with every recorded match, and
is provisional until playtests settle it. `RULES_VERSION` is 9.

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

A Handler's plan is up to `PLAN_SIZE` (three) orders, in any mix, repeats allowed, and a USE ITEM
for each creature carrying something besides:

| Order | Aimed at | What the team does |
|---|---|---|
| PRESS | One to five squares | Gets them: someone walks onto an empty one and holds it; one of theirs standing on one is pressed off it; one of theirs holding there is flanked by two |
| HOLD | One to five squares, and a facing | Keeps them: someone holds each, facing north, east, south or west as the Handler chose, and whoever is free guards beside one or heads off one of theirs coming at it. A lever or a switch named is guarded from beside it |
| TRICK | One to five squares | Outfoxes whoever of theirs is on them: someone goes round to its side or its back and, once it is holding, breaks its hold |
| INTERACT | One thing | Works the board: a found thing on the ground or glinting is fetched; the lever or the arena's switch is pulled (or, while it rests, waited by) |
| USE ITEM | One of ours carrying something, and a tile | A bonus beyond the three: see [USE ITEM](#use-item) |

PRESS, HOLD and TRICK go round in a circle: a PRESS pushes a TRICK off its feet, a HOLD turns a
PRESS away, and a TRICK from the side or behind breaks a HOLD (see [Encounters](#encounters)).
INTERACT is outside the circle.

An order with nothing it can be aimed at (a square off the board or in a wall, an INTERACT with
nothing there) is left out, as is a fourth order and beyond (`OrderDropped`), so a room never has
to trust what a client sends. Before rules version 9 the three orders were TAKE, DEFEND and
INTERACT, and an INTERACT could also go after one of theirs carrying something or have one of
ours use what it carried.

### USE ITEM

A creature that picks something up (by stepping onto it, as ever) can be told to USE ITEM, on any
tile within the item's reach of where it stands when the turn begins. It is the one order a
Handler gives a creature by name, and it never counts toward the three. One USE ITEM a creature a
turn; one for a creature not carrying anything when the turn began, a second for the same
creature, or a tile out of reach is left out.

Every USE ITEM is used at the very start of the turn's first tick, before anything else settles,
in roster order, left team first, and it takes the user's step that tick: it stands still while
it uses it, then carries on with its job from the second tick. A springy seed's hop, landing at
the start of the second tick, is its step that tick too. So using something never adds a step or
a press to a creature's turn. What a creature takes from one of theirs during a turn (see
[Encounters](#encounters)) can be used from the next turn, when its Handler can tell it to.

### Sharing out the plan

At the start of the turn, from one picture of the board, each team turns its plan into jobs
(`muster`). A square named in one order is worth `NEED` (1000) to the team; named in two orders,
twice that, and it brings a guard beside it, a second guard if it is named in three. Taking a
square one of theirs holds is two jobs, a side each, worth a square apiece; with only one of them
taken it is worth `LONE` (450). Whoever is left over guards a square of the plan (more for a
HOLD than a PRESS), heads off one of theirs coming at a square to hold, or picks up something
lying within four steps. A square to TRICK with one of theirs on it is a job worth `NEED` from a
tile beside it that neither it nor a holder of its team beside it faces; one standing in front
of it walks round first.

Each creature weighs the jobs it could do: the job's worth, less `STEP` (90) for every step it
would walk, plus `ON_TIME` (200) if it can be there this turn and `STICK` (80) if it is already
there or already doing that job; a job on ground marked for the map event or a puff is worth 150
less to stand on. Then, last, who it is (see [PERSONALITY.md](PERSONALITY.md)), never more than
`LEANING_CAP` (180, about two steps) either way. The team takes the sharing out worth the most in
all, each creature weighing its best eight. Two jobs that stand on one tile are never both taken,
and one of ours already holding a square of the plan keeps it: swapping holders would leave it
empty while one walks off and the other walks on.

Each job becomes one of the five orders below for the creature that took it. A HOLD from a PRESS,
a guard or anyone with nothing better to do faces the nearest of the other team; a flank's sides,
and one waiting beside a holder, face that holder; one waiting by the lever or a tap faces it. A
flank's two sides walk into place and hold there; once both stand in place at the start of a
turn, both press together. Alone where two were needed, a creature much keener to meet the other
team than it is wary (`RECKLESS`, 150) presses the holder anyway and is likely turned away; any
other stands beside it, keeping the square contested, until help comes.

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
nothing to do. A press, trick or pull already spent this turn is not taken again. Thinking again about ground
about to be hit, a wary creature finds another job and a bold one often stays.

## The orders each creature carries out

| Order | Intent | Target |
|---|---|---|
| MOVE | Reach a tile | A tile. Walks round anyone standing still. Open to a PRESS the whole turn, arrived or not |
| HOLD | Defend a tile, facing one way | A tile and a facing. Walks there, then stands, facing that way for the rest of the turn. If someone else is standing on it, holds beside it instead |
| PRESS | Displace and interrupt | A creature of the other team. Walks up next to it (one of the four tiles touching it) and pushes, once a turn |
| TRICK | Outmaneuver and disrupt | A creature of the other team. Walks round to a tile beside it that it does not face, never across its front, and tricks it once it is holding, once a turn |
| INTERACT | Work the board | A lever or a switch. Walks up next to it and works it, once a turn |

A holder's facing is shown on the board while it holds, and nothing in a turn turns it: flankers
who take the ground (see [Encirclement](#encirclement)) face where the holder they broke stood.
There are no move sets, abilities, power ratings or Battle stats. Every creature has the same
composure, the same pace and one job a turn. A USE ITEM comes from something lying on the board,
which any creature can pick up, never from the creature.

## A tick

Every tick works from one shared picture of the board. Nobody moves first.

1. A gate whose swing was telegraphed last tick swings. Then whatever was marked to land this
   tick lands: a leaf umbrella or a twig whose time is up folds or falls, things glinting since
   last turn turn up (on a turn's first tick), items used last tick take effect, and the map event
   comes. Anyone whose job has just come to nothing thinks again (see [Thinking again](#thinking-again)).
   On the first tick, every USE ITEM is used.
2. Presses, tricks and interactions already next to what they are aimed at are settled, so a
   creature walking away from a presser beside it is caught before it can step.
3. Each creature still carrying out its order picks one step (see [How a creature walks](#how-a-creature-walks)).
4. The steps happen together. Anyone with empty paws who stepped onto something lying there
   picks it up.
5. Anyone who has reached the tile it was told to hold starts holding, facing the way it was
   told.
6. Presses, tricks and interactions that have just come into reach are settled.
7. The beacon counts, a breakthrough included (see [The beacon](#the-beacon)).
8. The match ends by knockout, then by capture.

Within a settling, presses settle first, then tricks, then interactions, so a creature pressed
in a tick gets neither its trick nor its interaction that tick. Composure is taken as each is
settled, never more than `damage_cap` from one creature in a tick. A creature at zero is knocked
out and leaves the board at once.

## Encounters

| PRESS against | Rule |
|---|---|
| Anyone not holding (moving, tricking, interacting, done) | Pushed one tile straight on from a presser, onto free open ground, out of the zone if any of those ways leads out of it (ties to the upper tile, then the one nearer its own side); knocked off balance where it stands if every way on is blocked. Either way it stops for the rest of the turn and does not count in the zone. What it carries goes to the first presser with empty paws, or onto the ground if none has. Nobody loses composure |
| PRESS (pressing, on its way or now) | A collision: the one pressed and each presser lose one composure, and all stop for the rest of the turn. Nobody is pushed. Two pressing each other collide once |
| HOLD | Repelled: the holder, and each holder of its team beside it corner to corner that a presser has reached, turns every presser away. A holder facing a presser (it stands on the tile in front) costs it one composure, at most once a tick, spread so no presser takes a second before every other has taken one; from the side or behind nobody is hurt |
| HOLD from two sides | Flanked, when the pressers outnumber the holders turning them away: see [Encirclement](#encirclement) |
| Anyone under a leaf umbrella, not holding | Bounces off: nobody is hurt, and the one under it goes on |

| TRICK against | Rule |
|---|---|
| HOLD, from the side or behind | The hold breaks: the holder loses one composure and holds nothing more this turn. It is not moved and stays on its feet, so it still counts in the zone, and a PRESS now pushes it like anyone not holding. However many trick it in a tick, it loses one |
| HOLD, from the front | Read: nothing happens, and the trick is spent. So is a trick from a tile that another holder of the target's team faces: holding side by side, each watches the other's flank |
| PRESS (pressing, on its way or now) | Caught: the trickster is pushed one tile on away from the one it tried to trick, stopped for the turn, and robbed by it, as if pressed |
| Anyone else | Nothing yet: the trickster waits beside it, and tricks it if it starts holding |

| Others | Rule |
|---|---|
| MOVE vs HOLD | The mover walks round, if there is a way round. Nobody is hurt |
| Two wanting one tile | A fair draw, seeded and the same for both teams, lets one in; the other is bumped |
| Two wanting to swap | Neither moves |
| A step into a tile its creature did not leave | The step fails, and so does any step into that one's tile |
| Bumped | Until it steps again, others plan round it as if it were standing still, and it plans round them, so two meeting in a narrow way stop trying rather than bumping every tick |
| INTERACT, pressed | A press that reaches a creature working a lever or a switch, before it does or in the same tick, stops it: no pull. A holder beside the lever or the switch turns the pull away (see [Props](#props)), and a pebble bell rung nearby hushes it |

A press, a trick and an interaction are each settled once a turn, whatever comes of them. On the
way to a target three or more tiles off, a creature may instead press someone not holding who
walks into reach (see `opportunism` below).

Since rules version 9, PRESS costs composure only when two presses collide, or when a holder
facing it turns it away, and in a flank. Before it, a CONTEST cost a creature caught on the move
one composure for each contester, two contests meeting were a harmless scuffle, and a holder
countered a contest from any side.

### Cases rules version 9 had to settle

PRESS, HOLD and TRICK came as a spec that left some cases open. These are the choices made, each
the plainest that kept the circle a circle; any can change.

| Case | Choice |
|---|---|
| PRESS against someone moving, working the board or done | As against a TRICK: pushed, stopped and robbed, no composure. Only a HOLD stands up to a PRESS |
| A press meeting one pressing someone else | A collision too: both lose one and stop, whoever each was going for |
| Several pressing one not holding | One push, away from one of them as for a flank, and one composure-free stop; the item goes to the first with empty paws |
| A push with nowhere to go (wall, edge, someone standing there) | Off balance where it stands, as when boxed in by a flank |
| Item theft with full paws | The item drops where the one caught ends up |
| A trick that lands | The hold breaks for the turn and costs one composure, and the holder keeps its feet: it is not off balance, does not drop what it carries, and still counts in the zone. Breaking its hold is half the job; a PRESS then pushes it off. While tricked holders were knocked off balance as a flank knocks them, tricks alone cleared the yard and 92% of matches between thinking coaches were won at the beacon in 6.6 turns |
| Several tricking one holder | One composure and one broken hold, however many |
| Holders side by side against a trick | A trickster standing where another holder of the target's team faces is read, so two holders facing each other's flanks cover them |
| A trickster in front of a holder | It walks round, never across the front; the team never sends a creature standing in front to trick, but walks it round first. A trick that settles from the front is read |
| A trick on someone not holding | It waits beside it, and lands if it starts holding, or is caught if it is pressing |
| Order within a tick | USE ITEMs first (first tick only), then presses, then tricks, then interactions, each from the same picture of the board; a creature pressed in a tick gets neither its trick nor its interaction |
| Interrupting INTERACT | A press reaching the one working a lever or a switch stops it, as it stops anyone not holding; holders beside the lever or the switch turn a pull away as before, one composure each, whichever way they face; a pebble bell hushes it |
| Facings the team picks for itself | Toward the nearest of the other team for claims, guards and the fallback; toward the holder for flankers and pinners; toward the lever or the tap for one waiting by it |
| USE ITEM and the three orders | Never counts toward them; one per creature carrying something when the turn began, aimed within its item's reach of where it stands, used at the start of the turn in roster order, costing the user its whole first tick |

## Encirclement

A holder can be broken without being worn down: by pressers coming at it from two or more of its
four sides at once, as well as by a TRICK from the side or behind.

- **Flanked.** When two or more pressers settle on one holder in the same tick and they
  outnumber the holders turning them away (the holder itself, and any holder of its team beside
  it, corner to corner, that a presser has reached), the hold breaks, whichever way it faces. The holder loses one
  composure, however many came at it, and nobody is turned away.
- **Pushed.** It is pushed one tile straight away from one of its pressers, onto a free tile out
  of the zone if any of those ways leads out of it; ties go to the upper tile, then the one
  nearer its own side of the board. If every way out is blocked (two pressers on opposite
  sides block each other's), it is knocked off balance where it stands.
- **Off balance.** Either way it stops for the rest of the turn, is not holding, and does not count
  in the zone. Next turn it can hold again.
- **Taking the ground.** Flankers standing in the zone when the hold breaks start holding where
  they stand, facing where the holder stood, so the ground they won counts for them at once. A
  flanker outside the zone does not.
- **Breakthrough.** A hold in the zone broken by a flank is worth a tick of beacon progress to
  the flankers (see [The beacon](#the-beacon)).
- **Side by side.** Two holders corner to corner share the two tiles they both touch: a
  presser on either is turned away by both, so it takes three to break one of them.
- **Already in place.** Presses already beside their target settle before anyone steps, so
  flankers who were in place when the turn began break the hold before the holder can step
  away. Getting into place is the work: it takes MOVE (or HOLD on the flanking tile), and on the
  way a flanker that is not yet holding can be pushed back by a single press.

The counterplay is the defender's: hold beside a friend, press a flanker on its way, or move
before the flankers are in place. Being flanked costs a little composure; what it really costs is
the zone, and since rules version 8 a tick of the beacon besides.

`ai::flank_threatened` says whether a holder would be flanked if everyone of the other team beside
it pressed it now, and `ai::flank_places` finds two sides to flank it from.

## The beacon

The zone (`Z` tiles) is the beacon. At the end of every tick:

- `uncontested` (the default): a team gains a tick of progress when it has someone holding in the
  zone and nobody of the other team stands in it unbroken. Progress pauses as soon as the other
  team gets someone into the zone and keeps them there; a creature knocked off balance, flanked or
  pressed, no longer counts. One whose hold a TRICK broke still counts: it is on its feet.
- `majority`: a team gains a tick of progress when more of its creatures are holding in the zone
  than the other team has in it unbroken, holding or not.

Walking through the zone is not holding it. Reaching `capture_ticks` wins at once.

**Breakthrough.** A team that breaks a hold in the zone by flanking it gains a tick of progress for
it, on top of any tick for holding the zone alone that same tick (`BrokeThrough`), so a flank in
the yard can be worth two. Only one team gains in a tick: if both would, the zone is contested and
neither does.

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

Props are worked with INTERACT (the beacon with HOLD) and grant nothing permanent. Each is data in the arena file.

| Prop | Interaction | Effect | Telegraph | Counterplay |
|---|---|---|---|---|
| Beacon (`Z`) | HOLD in the zone | Capture progress | Progress shown to both teams | Stand in the zone unbroken to pause it; flank its holders, or trick them and press them off, to clear it |
| Gate lever (`L`, gates `G`/`g`) | INTERACT from a tile touching the lever | Every gate swings `swing_ticks` later (at least 1), then the lever rests `cooldown_turns` whole turns | The pull, and the tick the gates will swing, are an event both teams see | HOLD on a tile touching the lever: a pull is turned away, one composure for each such holder, whichever way it faces; or PRESS the one pulling before it pulls. A gate cannot close on someone standing in it, or on a twig |
| Map event (`event:` and its tiles) | Comes round by itself on `event_first` and every `event_every` turns after | Its effect on everyone standing on its tiles | Its tiles are marked a whole turn before, from the end of the turn before it comes; it lands on that turn's second tick | Stand elsewhere when it comes, or under a leaf umbrella |
| Switch (`S`): a tap, a shell, a sluice, a tree | INTERACT from a tile touching it, or a pebble bell rung beside it | The map event lands a tick later; then every switch rests `switch_rest` whole turns | The turn, and the tiles, are an event both teams see | HOLD on a tile touching the switch, as at the lever. A switch does nothing while the event is already coming, nor for the rest of a turn it came round by itself in |
| Item spot (`*`) | Step onto what turns up there (INTERACT sends someone) | See [Found things](#found-things) | It glints a whole turn before | Get there first, or press the carrier and take it |

## The map event

An arena may have one map event. Its effects are ones a match already has, and none costs
composure.

| Effect | What happens to anyone on its tiles when it lands |
|---|---|
| Soak | Off balance, as when boxed in by a flank: it stops for the rest of the turn, stops holding, does not count in the zone, and drops what it carries. Under a leaf umbrella it stays dry |
| Wash | Pushed one tile onto the free open ground beside it nearest the board's middle row, off the event's tiles (upper first among equals, then nearer its own side), then off balance as for a soak; off balance where it stands if there is nowhere to go. Under a leaf umbrella it stays put |
| Close | The tiles go under for the rest of the turn: nobody can stand there, and whatever lay or stood wedged there floats off. Anyone on them is washed off first, never onto the yard; a leaf umbrella does not help. With nowhere to go, it stays where it stands, off balance |

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

The Millpond's rising water washes a creature on a bank onto the free ground beside it, never
onto the jetty. When every such tile is taken, by someone of either team or a twig, it stays where
it stands, off balance, on the water until the banks come up again at the end of the turn. More
than one can be caught this way when the bank is crowded enough; nobody else can step onto the
water meanwhile. This happens in about 0.4% of matches and
is left so on purpose: the bank was crowded, and the creature caught there loses the rest of its
turn just as one washed off would.

## Found things

Things turn up on an arena's item spots, which come in pairs mirrored across the board, on turn 2
and every three turns after. At the end of the turn before, a seeded draw picks a pair whose
spots are clear and one item, and the same item glints on both spots for both teams to see.
Puffs turn up 26% of the time, twigs 22%, leaf umbrellas 18%, pebble bells 16%, honey 12% and
springy seeds, the one that only moves its user, 6%.

A creature that steps onto something with empty paws picks it up, whatever its order; one thing
at a time, and both teams can see who carries what. Pressed, it loses what it carries to the
presser (onto the ground if the presser's paws are full). Flanked, soaked, blown over or knocked
out, it drops what it carries where it ends up (or loses it, if something already lies there).
Its hold broken by a TRICK, it keeps it. It is used by [USE ITEM](#use-item): marked on its tiles
the tick it is used and landing at the start of the next. Nothing found costs composure, nothing
outlasts the match, and the fallback never uses anything.

| Item | Reach | What USE ITEM does | Lasts |
|---|---|---|---|
| Dandelion puff | 2 | Blows across the tile and the two either side of it, across the way from its user to the tile (along whichever way is further; across the board when level). Everyone there is pushed one tile on and knocked off balance, or knocked off balance where it stands if the way on is blocked. A holder with a holder of its team beside it, corner to corner, braces and is not moved. Twigs there blow over | Lands and is gone |
| Twig | 2 | Wedges into the tile, if nobody and nothing is there and anyone could stand there: nobody can stand there, and a gate cannot close on it | To the end of the turn two after it lands |
| Leaf umbrella | 1 | Opens over whoever of its user's team stands on the tile, itself included: the map event cannot soak or wash it, and a press against it while it is not holding bounces off, hurting nobody. A puff catches it like a sail and carries it two tiles | To the end of the next turn |
| Pebble bell | 2 | Rings where its user stands: everyone of the other team within two tiles who has not settled its press, trick or interaction this turn loses it. Rung beside a switch that is ready, it sets the map event off | Lands and is gone |
| Honey drop | 2 | Spills over the tile and the four round it, wherever anyone could stand: whoever steps in, or is already there, is stuck where it stands for the rest of the turn, and can still hold | To the end of the turn two after it lands |
| Springy seed | 2 | Hops its user onto the tile, if it is free ground: over anyone or a twig in between, never over a wall, an obstacle or water. The hop is its step that tick | Lands and is gone |

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
| Within reach of the other team (not counting whom it is pressing or tricking) | −caution for each creature of theirs, for a cautious creature; nothing for a bold one, which does not avoid them but does not seek them out either |
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
became nearly one in two (1.5 a team-turn), and the kinds fell 0.155 apart under thinking coaches
(0.083 at 200%; see [Fairness](#fairness)). Only about one step in ten has two ways there that arrive equally soon, so
beyond that every way a profile chooses costs time. At 100%, under rules version 4, 79% of
team-turns had none. It weighs only the way there: what a creature does with a moment to spare is
weighed as the table says.

Ties are broken by a seeded draw. Now and then (`variability`, 1% to 6%) it takes its second-best
choice instead, if both are on time. A wary creature (caution 150 or more) sent to walk waits one
tick, once a turn, rather than take a step that puts it within reach of more of the other team
than it is now; told to hold, press or trick, it goes anyway. Once arrived with ticks to spare it
may take one follow-up step beside a friend or out of reach, if that beats its patience; when its
press is turned away, a creature that `steps back` takes one step out of reach.

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
| Left side | 0.502 |
| Kinds, highest to lowest | Scholar 0.529, Lazybones 0.512, Grump 0.511, Guardian 0.506, Explorer 0.504, Troublemaker 0.500, Showoff 0.498, Sweetheart 0.495, Oddball 0.477, Wallflower 0.460 |
| Spread between kinds | 0.068 (0.054 with seed 8), each kind over 3,600 to 7,000 matches |
| Won by capture, knockout, at the cap, drawn | 33%, 0%, 62%, 6%, in 10.9 turns |
| Beacon never lit; won from behind on the beacon | 27%; 5% |
| Flanks | 0.40 a match, 88% followed by beacon progress that turn |
| Tricks | 3.84 landed and 0.57 read a match |
| Presses | 0.90 a match on someone not holding (0.28 of them catching a trickster), 0.06 things taken, 0.13 collisions, 0.33 turned away head on |
| Knockouts | 0.71 creatures a match |

With both teams coached by `familiar` (`--coach familiar,familiar --pairings 400 --seed 9`),
kinds score within 0.088 of each other, Explorers highest (0.546) and Showoffs lowest (0.458);
75% of those matches are won by capture, in about 8.7 turns, the beacon is never lit in 3% of
them, and 21% are won from behind on the beacon. Thinking coaches trick 5.2 times a match, and
catch a trickster with a press 0.78 times.

On every arena, over 4,000 Exhibition-coached matches each (`--pairings 2000 --seed 7 --file
arenas/<arena>.arena`), with seed 8 in brackets:

| Arena | Left side | Spread between kinds | Won by capture | Beacon never lit | Won from behind |
|---|---|---|---|---|---|
| Backyard | 0.502 | 0.068 (0.054) | 33% | 27% | 5% |
| Sandy Cove | 0.498 | 0.089 (0.079) | 71% | 6% | 12% |
| Millpond | 0.505 | 0.066 (0.075) | 30% | 39% | 4% |
| Woodland Glade | 0.496 | 0.115 (0.112) | 37% | 32% | 5% |

Knowing your creatures (`--coach familiar,generic --pairings 500 --seed 7`) is worth 0.636 on
Backyard, 0.609 on Sandy Cove, 0.618 at the Millpond and 0.587 in the glade.

Rules version 9 brought PRESS, HOLD and TRICK, with a facing for every hold. Against the same
seeds under rules version 8, the Exhibition coach's matches on Backyard last as long (10.9 turns
against 10.7) and are won at the beacon as often (33% against 32%), but far fewer are drawn (6%
against 16%), because tricks and head-on presses cost composure and the cap's second count
decides more of them. Tricks became the commonest way a hold breaks, about four a match against
0.95 flanks before; flanks still land 0.4 times a match and nearly always pay. Kinds are closer
together under the Exhibition coach (0.068 against 0.073, 0.054 against 0.090 with seed 8; Sandy
Cove 0.089 against 0.114) and about as close under thinking coaches (0.088 against 0.087). Twice
as many creatures are knocked out (0.71 a match against 0.31), and still almost no match is won
by knockout. Under thinking coaches, matches are won at the beacon more often and sooner (75%
against 60%, 8.7 turns against 9.2), the beacon is lit in all but 3% of them (9% before), and the
winner came from behind in 21% (17%). What changed most is what knowing your creatures is worth:
0.636 against 0.595 on Backyard, and 0.587 against 0.523 in the glade (0.609 against 0.631 on
Sandy Cove, 0.618 against 0.602 at the Millpond), since a trick lands only from where its target
does not face and whoever walks there in time decides it. While a trick knocked its holder off
balance, as a flank does, thinking coaches won 92% of Backyard matches at the beacon in 6.6 turns,
and the version that shipped leaves a tricked holder on its feet.

Under rules version 8 the same runs gave: Backyard 0.501, 0.073 (0.090), 32%, 30%, 8%; Sandy Cove
0.506, 0.114 (0.104), 66%, 9%, 11%; Millpond 0.503, 0.075 (0.055), 31%, 29%, 6%; Woodland Glade
0.488, 0.118 (0.127), 41%, 26%, 8%; and knowing your creatures 0.595, 0.631, 0.602 and 0.523.

Rules version 8 let a flank in the yard take the ground and break through (see
[Encirclement](#encirclement)). Before it, with the same seeds, the yard was held by both teams
for 24 to 29 of a match's 33 or so ticks, most often two of one team holding against one of the
other: one holder was enough to stop the beacon, and a flank that broke it left the flankers
standing beside it, not holding, so only about two flanks in five led to progress that turn. The
beacon was never lit in 45% of Exhibition matches on Backyard, 10% on Sandy Cove, 52% at the
Millpond and 59% in the glade, those matches being decided on composure or drawn, and the winner
had been behind on the beacon in 1% to 3% of them (10% on Sandy Cove). Under thinking coaches
(familiar against familiar) 39% of Backyard matches were won by capture, 19% at the Millpond and
20% in the glade, against 59%, 36% and 52% now. Matches are no shorter: 10.9 turns on Backyard
before, 10.7 now.

Rules that let holding count for more ended matches too, but handed them to a team that only
holds, and were dropped: counting the zone by `majority` won 98% of Backyard matches at the
beacon in about 4 turns, and the turtle beat Exhibition with it; counting it when a team has two
more holding in it than the other has in it, or by `majority` on the last two turns only, let the
turtle beat Exhibition 0.92 and 0.77 of the time. Taking the ground and breaking through reward
only a team that comes at a holder, and the turtle loses more often than before (see [Holding
still](#holding-still)). What it costs is a little of what knowing your creatures is worth: 0.629
to 0.594 on Backyard and 0.595 to 0.525 in the glade, because a flank that lands now counts
for so much whoever planned it. Jake chose this on 2026-10-16.

Rules version 7 replaced an order for each creature with three orders for the team, and halved
the pull (see [How a creature walks](#how-a-creature-walks)). Under version 6, with an order each
and the pull at 400%, Exhibition matches put the kinds 0.108 apart on Backyard, 0.107 on Sandy
Cove, 0.133 to 0.141 at the Millpond and 0.123 to 0.152 in the glade; familiar coaching kept them
within 0.078, and knowing your creatures was worth 0.55 to 0.62. With team orders and the pull
left at 400%, the Exhibition spread stayed about where it was but the kinds fell 0.143 to 0.155
apart under thinking coaches (Wallflowers at 0.40): a coach could no longer send a creature that
wanders, hangs back or keeps to its friends exactly where its way of walking cost least, and with
creatures walking further and more often, nearly half of them went their own way each turn. The
leanings that pick who takes which job were not what did it (with them taken out of the sharing
out the kinds were 0.170 apart), and neither was the team planning round how each creature walks
(0.143, tried and dropped). Halving the pull brought back about one creature in three going its
own way each turn, as chosen for rules version 5, and the kinds within 0.083 under thinking
coaches.

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

About half of Exhibition matches still go to the turn cap. With good holding on both sides the
zone stays contested, and the cap's order (beacon progress, then composure, then creatures
standing) decides them; flanks are what move the beacon and composure both. Every tie between equal tiles, in the rules and in the coaches,
is broken from each team's own side of the board, so the board plays the same from both sides.

`tournament` also says how its matches ended: how many never lit the beacon at all, how many were
won by a team that had been behind on the beacon, and how often a flank was followed by beacon
progress that turn.

Before rules version 2, under strong coaching, a bold creature went out of its way to walk into
the other team's reach, and a social one detoured to stay beside its teammates because company
counted on every tile of a long way round: Grumps scored 0.554 and Explorers 0.459. Both now
have to be paid for like any other preference.

## Holding still

The `turtle` coach is the strategy encirclement was made against: it HOLDs a tile of the zone for
every creature, all facing the other team's side of the board, so each walks to one and holds it
for the rest of the match. A creature whose tile was taken first goes after whoever took it,
which is what HOLD asks; under rules version 6, when it gave each creature its own HOLD, the
turtle never contested at all and Exhibition scored 0.608 against it. Over 2,000 matches each
(`--coach <coach>,turtle --pairings 1000 --seed 3`; 600 for familiar and generic, with
`--pairings 300`):

| Against turtle | Score | How |
|---|---|---|
| exhibition | 0.880 | 83% won or lost at the cap, 12% drawn, 5% by capture |
| familiar | 0.888 | 51% at the cap, 38% by capture, 10% by knockout, 1% drawn |
| generic | 0.790 | 55% at the cap, 37% by capture, 6% by knockout, 1% drawn |

Under rules version 8 they scored 0.760, 0.803 and 0.736: three holders that all face one way are
tricked from their sides turn after turn, losing composure each time.
Under rules version 7, before a flank took the ground, they scored 0.661, 0.790 and 0.695.

Three holders in a five-tile cross keep it contested for as long as they stand, so they are
almost never beaten by capture; they are beaten on the cap, because a team that only holds never
stops anyone from standing in the zone too and is flanked turn after turn. Under rules version 3
(`majority`, no flanking) the turtle beat the Exhibition coach 99.9% of the time and the familiar
coach 83%.

## Knowing your creatures

The `familiar` and `generic` coaches (`plan.rs`) think each turn through the same way: for each of
the team's three orders, try each sensible order (each square of the zone or of theirs to PRESS,
each square of theirs holding to TRICK, each square of ours in it to HOLD facing the nearest of
the other team or its side of the board, each thing to INTERACT with), play the turn out in their
head against what they expect the other team to do, and keep whatever leaves the board best. Their
USE ITEMs are the Exhibition coach's: anything carried is used where it plainly helps. The
familiar coach imagines its creatures as they are, so it knows who will take which job; the
generic one imagines every creature as one that leans no way at all.

| Coaches | Familiar's score |
|---|---|
| familiar against generic, 1,000 matches | 0.636 |
| familiar against Exhibition, 1,000 matches | 0.995 |
| generic against Exhibition, 1,000 matches | 0.946 |

Under rules version 9 the coach who knows its creatures wins 63.6% on Backyard (59.5% under
version 8): a trick lands only from a side its target does not face, so who gets round in time,
and who hangs back or wanders on the way, decides more than it did.

Under rules version 8 the coach who knows its creatures wins 59.4% on Backyard (62.9% under
version 7: a flank that lands counts for more now, whoever planned it). Under rules version 7 it
won 62.9% (62.1% under
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
