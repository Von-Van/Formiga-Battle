# Personality in Formiga Battle

Battle reads who a companion is from Desktop's own record and nowhere else. It has no taxonomy of
its own, no Battle stats and nothing it remembers about a creature that could make it stronger.
This page lists every part of Desktop's record that says who a companion is, as of Desktop 0.67.6
(`formiga-core`), and what Battle does with it.

A profile changes how a creature carries out an order and nothing else. It never changes what
MOVE, HOLD and CONTEST do to each other, composure, pace, the one order a turn, or whether a
creature does what it was told. Every leaning is a preference between ways and moments, paid for in
time when it costs a detour, so being more likely to do one thing is always being less likely to do
another.

## Where it comes from

| Desktop | Type and range | Default and history | Battle |
|---|---|---|---|
| `Creature::id` | `u64` | Stable for the companion's life; the identity every contract uses | `Profile::creature_id`, the only identity Battle keeps |
| `Creature::name` | 1 to 24 characters | | Shown |
| `Creature::temperament` | `Option<Temperament>` | Given to every companion made since 0.62.0. Absent for older ones, whose temperament Desktop reads from their genome (`Temperament::read`): five axes from the values they had, the other four middling, the kind that fits best, no tension | Read through Desktop's own `Creature::temperament()`, so an older companion is read exactly as Desktop reads it |
| `Temperament::kind` | 10 kinds: Sweetheart, Troublemaker, Grump, Explorer, Wallflower, Showoff, Scholar, Oddball, Lazybones, Guardian | Drawn first; the axes are drawn loosely around it | Recorded and counted by the fairness tournament. Not read by the rules: the axes say everything the kind does |
| `Temperament::axes` | 9 axes, each 0 to 1, 0.5 in the middle | | Turned into thousandths, then into the leanings below |
| `Temperament::tension` | `Option` of 10: BraveButNervous, AffectionateButIndependent, PlayfulButShy, GentleButStubborn, SeriousButCurious, ConfidentButEasilyEmbarrassed, LazyButCompetitive, SuspiciousButLoyal, DramaticButCowardly, GrumpyButAffectionate | Drawn now and then; holds some axes to ranges that fit both halves | Through the axes it holds, and BraveButNervous and DramaticButCowardly step back after a contest |
| `Creature::traits()` | The three words a profile shows | Read out of the axes and the kind | Not read: the axes they come from are |
| `Creature::personality` | `PersonalityGenome`: activity, curiosity, boldness, playfulness, sociability, routine affinity, sleep timing, window tolerance, cursor interest, decision temperature | Taken from the axes for a companion with a temperament | Not read: it is the axes again, tuned for the desktop. For an older companion it is what its axes are read from |
| `Creature::tendencies` | `LearnedTendencies`, 8 leanings, each -100 to 100 | Rests at a baseline its temperament sets; learned a little at a time and drifts back while nothing pushes it. Whole numbers in older files read as the same values | Rounded to whole numbers, then into the leanings below. This is the drift: as Desktop's leanings move, so does how the creature walks in Battle |
| `SaveFile::relationships` | Per pair: affinity, familiarity, playfulness, avoidance, each 0 to 255 | | In the bands Desktop hands every companion app: none below 16, low below 64, medium below 112, high from there. Affinity and avoidance are read; familiarity and playfulness are not |
| `Creature::leaning` | `RoamingLeaning` | The owner's wish for where it roams the desktop | Not read: it is the person's, not the creature's |
| `Creature::memory.habits`, `accessory`, `appearance` | | | Not read by the rules. The look is for drawing |

## What it becomes

Each axis is read as how far it leans from the middle, from -500 to 500 (`lean`). Each learned
leaning is its own value, -100 to 100. Every result is held to -500..=500.

| Battle leaning | From | What it does |
|---|---|---|
| `caution` | (−6 × lean(boldness) + 4 × lean(suspicion)) / 10 − sleep security | Keeps out of the other team's reach and close to cover. Over about 440 it will take a two-step detour round a single creature of theirs. Below zero it does nothing: a bold creature does not avoid the other team, and does not seek it out |
| `togetherness` | 0.8 × lean(social) + 2 × sociability | Keeps beside teammates, more so the ones it is bonded to, and away from one it avoids. Counted as a way's average, so it chooses between ways of about the same length and rarely pays for a long one |
| `wanderlust` | 0.6 × lean(curiosity) + 2 × exploration − home affinity | Takes the outside lanes rather than the middle |
| `opportunism` | (5 × lean(feistiness) + 3 × lean(impulsiveness) + 2 × lean(playfulness)) / 10 + play | On the way to one contest, takes another that walks into reach: a chance of opportunism in a thousand |
| `patience` | (−7 × lean(impulsiveness) − 3 × lean(energy)) / 10 + 2 × routine | With a moment to spare after arriving, stays where it was sent |
| `variability` | 40 + impulsiveness / 10, in thousandths | How often it takes its second-best way, if both are on time: 4% to 14%. The same side of a companion Desktop's own decision temperature comes from |
| `steps_back` | Tension BraveButNervous or DramaticButCowardly, or caution over 250 | Takes one step out of reach after its contest |

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

## Explaining it

Every step a creature takes that is not the plain way carries a reason (`event::Reason`), and the
recap says it in words: "Pip took the side route to avoid the contested centre." The reason is the
part of the creature's score that made the difference against what a creature with no leanings at
all would have done, so it is always true of the board as it was.

## The Handler's Notebook

Battle keeps a page for each of the Handler's companions, by colony and Desktop's creature id, in
its own data folder (`notebook.json`; `FORMIGA_BATTLE_DATA_DIR` for development). After every
match it records what the events showed of each: every reason a step was not the plain way, the
contests it took on a chance, held firm in or caught someone with, which teammate it kept near,
and a few moments worth remembering. Notes are drawn from the last ten matches only, so as
Desktop's leanings drift and the creature walks differently, its notes change with it.

A way of walking is noted once it is a habit: at least three times and in at least a fifth of its
steps ("Sometimes wanders out along the edges"), and "Often" from a third. Notes never carry a
number, a rating or a guess about what a creature is like inside, and nothing in the notebook
changes how a match is played.
