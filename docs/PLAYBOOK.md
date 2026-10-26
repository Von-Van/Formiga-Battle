# Playbooks

A playbook is a Handler's plan for a whole match, written before it starts: one to three stages
of what they want, rather than where each order goes. The compiler lives in the match core
(`crates/formiga-battle-core/src/playbook.rs`); the page and the match screen use it from the app
(`crates/formiga-battle/src/app/playbooks.rs`, `table.rs`).

## In the window

- **The Playbooks page**, one of the Handler's own pages beside Handler and Circuit. Choose a
  trio, start a playbook from one of the samples (a trio keeps three at most), name it, and set
  each stage. "Play by this one" makes it the trio's playbook. Removing a playbook takes a second
  press.
- **The Who page.** When the three picked are a trio, a row under "Playing as" chooses which of
  its playbooks fills in its orders, or none, with the way to the Playbooks page.
- **The match.** Against the visitors (a cup match, an event fixture or a rival), the playbook
  fills in each turn's orders as the turn begins. A note above the orders names the playbook and
  the stage. Change, clear or add any order before locking in; the match is played from what is
  sent. A friend match or a Field Lesson never uses a playbook.
- **The whole match** (L) opens each turn with what the playbook made of it, and what the Handler
  changed.

Playbooks are kept with the career, in Battle's own file for the colony (`careers/<colony>.json`,
the `playbooks` field, by trio id). A rehearsal on the sample colony keeps none once Battle
closes.

## What a playbook is

Each stage has:

| Field | Values | What it does |
| --- | --- | --- |
| Intent | Secure, Pressure, Draw out, Counter, Fetch, Work the board | What the stage wants (below) |
| Aim | Centre, Near, Far | Which way across the board it looks first, among squares or creatures otherwise as good as each other, measured from each team's own edge |
| Posture | All in, Balanced, Guarded | How many of the team stay on the beacon: none, one or two |
| Facing | Them, Across | Which way its HOLDs face: toward the nearest of the other team, or toward their side |
| Turns | 1 to 4 | How long it is given to get its goal |
| Fallback | Next, Again, First | What happens when its turns run out without the goal: on to the next stage, the same stage again, or back to the first |
| Comes in whenever | Only in its turn, or one of eight things on the board (below) | Brings the stage in from whatever stage the team is on |

| Trigger | When the board shows it |
| --- | --- |
| They hold the beacon | One of theirs is holding a square of the beacon |
| The beacon is clear of them | None of theirs is on the beacon |
| We are behind / ahead | One team's beacon is further up than the other's |
| Something turns up nearby | Something lies or glints within 5 steps of one of ours with empty paws |
| The tap would soak them | The tap is ready and would soak more of theirs than ours |
| One of ours is out / one of theirs is out | Someone has been knocked out |

| Intent | Orders it becomes | Its goal |
| --- | --- | --- |
| Secure | PRESS onto the beacon's free squares, HOLD the ones already ours | Two of ours holding the beacon and none of theirs on it |
| Pressure | PRESS whoever of theirs is in the beacon or within 5 steps of its middle; a holder only when two can flank it | One of theirs loses composure or is knocked out |
| Draw out | HOLD squares of the beacon facing them, so a PRESS at us is turned away and costs them composure; the tap when it would soak more of theirs than ours | One of theirs loses composure, or fewer of theirs on the beacon |
| Counter | TRICK their holders | A hold of theirs on the beacon broken |
| Fetch | INTERACT with found things within 5 steps of one of ours with empty paws | One more of ours carrying something |
| Work the board | INTERACT with the tap (one that soaks them first) and the lever | The tap turned or the lever pulled |

A stage with nothing to aim at (nothing to fetch, nothing ready to work, no holder to trick,
nobody near enough to press) gives its hands to the beacon that turn and says so.

## How it plays

A playbook compiles each turn to an ordinary team plan: at most three PRESS, HOLD, TRICK and
INTERACT orders naming squares, and a USE ITEM for anyone carrying something with a plain use for
it. The team shares the plan out by itself (`muster`), and the turn resolves under the same rules
as any other. A playbook never changes who takes which job, what a job is worth, or what an order
does. It reads only the board, the arena and its own creatures, never the other team's leanings.

Two timings exist, to compare:

- **One stage at a time** (`Timing::Successive`), what the window uses. Stage one plays from the
  first turn. Each stage plays at least the turn it began. Before each later turn, a stage whose
  trigger the board shows comes in, the first such in the playbook's order; otherwise the stage
  is checked for its goal (on to the next stage) and its turns (its fallback). After the last
  stage the team plays Secure until a trigger brings a stage back.
- **All stages at once** (`Timing::Bundled`). Every turn, the first stage's posture keeps hands on
  the beacon and the rest are shared among the stages in order, the earlier first. A stage with a
  trigger plays only while the board shows it.

Where an intent can be worded more than one way (one PRESS naming two squares or two naming one
each, a HOLD facing them or across), the compiler asks the team's own share-out which wording these
three creatures would carry out as meant, and gives that one. This is the "reliability" a playbook
offers: it uses what the Handler knows about their own trio, and nothing else.

## Changing its orders

`Runner::sent` records what the Handler sent, when it differs from the playbook's plan: how many
of its orders were kept and how many of the Handler's own were added. Changing them costs
nothing. The stage carries on, its turn counts, and the next turn is compiled from the board as
usual.

## What it says

Every turn a playbook plays leaves a `Compiled` record: the turn, which stage ran (or that every
stage has had its turn), why it played otherwise than asked (nothing to aim at, short of hands),
whether it moved on from the stage before and why (goal got, turns run out and which fallback, or
a trigger), whether it was worded for the trio rather than plainly, the plan it compiled, and what
the Handler sent instead, if anything. `Compiled::line`
says it in one sentence:

```
Turn 4: Take and keep played stage 2, Draw out (stage 1 got its goal).
```

## Determinism and replay

Compiling is pure: the same board and the same playbook give the same plan, with no rolls. A
playbook keeps only which stage it is on and what the board was when that stage began, so a match
played from one comes out the same every time, and replays from its recorded plans like any other
without the playbook.

## The comparison

```
cargo run --release -p formiga-battle-core --example playbook_compare -- [pairings] [arena]
```

plays each sample playbook (with its triggers, by turns only, and all at once) against the
Exhibition and familiar coaches, beside those coaches against each other, and then the playbooks against each
other. Random trios stay with their pilot and every pairing is played from both sides, by the same
loop as the fairness tournament (`tournament::play_by`).

`--review playbooks` pictures the page, and `--review planned` a match with the orders filled in.
