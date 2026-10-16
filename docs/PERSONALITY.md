# Personality in Formiga Battle

Battle reads who a companion is from Desktop's own record and nowhere else. It has no taxonomy of
its own, no Battle stats and nothing it remembers about a creature that could make it stronger.
This page lists every part of Desktop's record that says who a companion is, as of Desktop 0.67.6
(`formiga-core`), and what Battle does with it.

A profile changes which job a creature takes on from its team's plan, how it carries it out, and
nothing else. It never changes what MOVE, HOLD and CONTEST do to each other, composure, pace, the
one job a turn, or whether a creature does what its team asked. Every leaning is a preference
between jobs, ways and moments, paid for in steps or time, so being more likely to do one thing is
always being less likely to do another.

## Where it comes from

| Desktop | Type and range | Default and history | Battle |
|---|---|---|---|
| `Creature::id` | `u64` | Stable for the companion's life; the identity every contract uses | `Profile::creature_id`, the only identity Battle keeps |
| `Creature::name` | 1 to 24 characters | | Shown |
| `Creature::temperament` | `Option<Temperament>` | Given to every companion made since 0.62.0. Absent for older ones, whose temperament Desktop reads from their genome (`Temperament::read`): five axes from the values they had, the other four middling, the kind that fits best, no tension | Read through Desktop's own `Creature::temperament()`, so an older companion is read exactly as Desktop reads it |
| `Temperament::kind` | 10 kinds: Sweetheart, Troublemaker, Grump, Explorer, Wallflower, Showoff, Scholar, Oddball, Lazybones, Guardian | Drawn first; the axes are drawn loosely around it | Recorded and counted by the fairness tournament. Not read by the rules: the axes say everything the kind does |
| `Temperament::axes` | 9 axes, each 0 to 1, 0.5 in the middle | | Turned into thousandths, then into the leanings below |
| `Temperament::tension` | `Option` of 10: BraveButNervous, AffectionateButIndependent, PlayfulButShy, GentleButStubborn, SeriousButCurious, ConfidentButEasilyEmbarrassed, LazyButCompetitive, SuspiciousButLoyal, DramaticButCowardly, GrumpyButAffectionate | Drawn now and then; holds some axes to ranges that fit both halves | Through the axes it holds, and BraveButNervous and DramaticButCowardly step back when a contest is turned away |
| `Creature::traits()` | The three words a profile shows | Read out of the axes and the kind | Not read: the axes they come from are |
| `Creature::personality` | `PersonalityGenome`: activity, curiosity, boldness, playfulness, sociability, routine affinity, sleep timing, window tolerance, cursor interest, decision temperature | Taken from the axes for a companion with a temperament | Not read: it is the axes again, tuned for the desktop. For an older companion it is what its axes are read from |
| `Creature::tendencies` | `LearnedTendencies`, 8 leanings, each -100 to 100 | Rests at a baseline its temperament sets; learned a little at a time and drifts back while nothing pushes it. Whole numbers in older files read as the same values | Rounded to whole numbers, then into the leanings below. This is the drift: as Desktop's leanings move, so does how the creature walks in Battle |
| `SaveFile::relationships` | Per pair: affinity, familiarity, playfulness, avoidance, each 0 to 255 | | In the bands Desktop hands every companion app: none below 16, low below 64, medium below 112, high from there. Affinity and avoidance are read; familiarity and playfulness are not |
| `Creature::leaning` | `RoamingLeaning` | The owner's wish for where it roams the desktop | Not read: it is the person's, not the creature's |
| `Creature::memory.habits`, `accessory`, `appearance` | | | Not read by the rules. The look is for drawing |

## What it becomes

Each axis is read as how far it leans from the middle, from -500 to 500 (`lean`). Each learned
leaning is its own value, -100 to 100. Every result is held to -500..=500. On the way there,
Battle counts what each leaning makes of a tile four times over (the pull, rules version 5), so
each turn about one creature of a team's three does something of its own.

| Battle leaning | From | What it does |
|---|---|---|
| `caution` | (−6 × lean(boldness) + 4 × lean(suspicion)) / 10 − sleep security | Keeps out of the other team's reach and close to cover. Over about 110 it will take a two-step detour round a single creature of theirs. Below zero it does nothing: a bold creature does not avoid the other team, and does not seek it out. At 150 or more it is wary: sent to walk, it waits one tick, once a turn, rather than step within reach of more of the other team; sent to hold or contest, it goes. It keeps off ground marked for the map event, a puff or honey, or already sticky, by half as much as it keeps out of reach of one of the other team |
| `togetherness` | 0.8 × lean(social) + 2 × sociability | Keeps beside teammates, more so the ones it is bonded to, and away from one it avoids. Counted as a way's average, so it mostly chooses between ways of about the same length, and pays for a longer one only when it leans hard |
| `wanderlust` | 0.6 × lean(curiosity) + 2 × exploration − home affinity | Takes the outside lanes rather than the middle. With `opportunism`, draws it out of its way to something lying on the ground |
| `opportunism` | (5 × lean(feistiness) + 3 × lean(impulsiveness) + 2 × lean(playfulness)) / 10 + play | On the way to one contest, takes another that walks into reach: a chance of opportunism in a thousand |
| `patience` | (−7 × lean(impulsiveness) − 3 × lean(energy)) / 10 + 2 × routine | With a moment to spare after arriving, stays where it was sent |
| `variability` | 10 + impulsiveness / 20, in thousandths | How often it takes its second-best way, if both are on time: 1% to 6%, small, so what a creature does can be learned. The same side of a companion Desktop's own decision temperature comes from |
| `steps_back` | Tension BraveButNervous or DramaticButCowardly, or caution over 250 | Takes one step out of reach when its contest is turned away by a holder |

| Desktop field | Read by |
|---|---|
| social | togetherness |
| energy | patience |
| boldness | caution |
| playfulness | opportunism |
| curiosity | wanderlust |
| feistiness | opportunism |
| impulsiveness | opportunism, patience, variability |
| suspicion | caution |
| affection | Not read. It is about the person at the desk, and Battle has no obedience to give it |
| cursor trust | Not read. There is no cursor in Battle |
| sociability | togetherness |
| climbing | Not read. Arenas are flat |
| sleep security | caution |
| exploration | wanderlust |
| play | opportunism |
| home affinity | wanderlust |
| routine | patience |

## Who takes which job

When a team shares out its Handler's plan (`muster`, see [RULES.md](RULES.md#sharing-out-the-plan)),
what the plan needs comes first, then how far each creature would walk and what it is already
doing. Who it is counts last, and never for more than `LEANING_CAP` (180, about two steps' walk)
either way, so it decides between jobs that are nearly as good as each other and never takes a
creature off a job only it is near. The same leanings that pick its way pick its job:

| Kind of job | Leaning toward it |
|---|---|
| Meeting the other team: moving one of theirs off a square, flanking, heading one off | (opportunism − caution) / 4 |
| Standing and holding a square | (caution + patience) / 5 |
| Guarding beside a square | togetherness / 4 + caution / 8 |
| Fetching something lying on the ground | (wanderlust + opportunism / 2) / 4 |
| The lever, a switch, using something carried | opportunism / 6 |
| Any job beside a teammate | +60 or +30 for a strong or a fair liking, −120 or −60 for a strong or a fair avoidance |
| Any job | a whim, from 0 to `variability` + 1, rolled from the seed |

A creature already doing a job, or already standing where it is done, keeps to it by `STICK` (80)
and a fifth of its patience above zero. Ground marked for the map event or a puff is 150 less
worth standing on, and half its caution less again.

Two more places show who it is. Alone where two were needed to flank a holder, one whose
opportunism beats its caution by more than 150 goes at the holder anyway and is likely turned away;
any other stands beside it and waits. And thinking again partway through a turn about ground
marked for something about to land, the warier it is, the likelier it finds another job.

Every job a creature takes says why (`muster::Why`): the job a creature with no leanings would
have taken in its place was the same (nearest, or already at it), or the leaning that made the
most difference against it (bold, careful, beside a friend, away from one it avoids, curious, on a
whim), or it was the only one free for a job meant for two.

With the Exhibition coach on Backyard, about one job in eleven goes to a different creature than
it would without leanings; nearly all the rest of what a creature does of its own is still the way
it walks there.

## Explaining it

Every step a creature takes that is not the plain way carries a reason (`event::Reason`), and the
recap says it in words: "Pip took the side route to avoid the contested centre." The reason is the
part of the creature's score that made the difference against what a creature with no leanings at
all would have done, so it is always true of the board as it was.

## The Handler's Notebook

Desktop keeps a page for each of the Handler's companions in the colony's Battle record, by its
creature id, and hands it to Battle in each session. After every match with a friend, Battle
records what the events showed of each: every reason a step was not the plain way (and
every tick it hung back), the
contests it took on a chance, held firm in or caught someone with, which teammate it kept near,
and a few moments worth remembering. Notes are drawn from the last ten friend matches only, so as
Desktop's leanings drift and the creature walks differently, its notes change with it.

A way of walking is noted once it is a habit: at least three times and in at least a fifth of its
steps ("Sometimes wanders out along the edges"), and "Often" from a third. Notes never carry a
number, a rating or a guess about what a creature is like inside, and nothing in the notebook
changes how a match is played.
