# Changelog

## Unreleased

- Battle opened by itself, from Applications or the Start menu rather than from Formiga, says
  where your own companions come from before anything else: a card over the Battleground says
  Battle plays with your own colony when it comes by train from Formiga, and to choose "Play
  Formiga Battle…" from the Formiga icon in the menu bar (the notification area on Windows), with
  formigaworld.com for anyone without Formiga yet. "Look around with the sample colony" puts the
  card away and leaves the rehearsal as it was, and "Close" closes Battle. `--sample` opens on the
  sample colony with no card, as before, and `--review welcome` pictures the card.
- Battle remembers its settings between launches: the music and sound volumes, the mute, reduced
  motion and the window's size, in one small file of its own (`settings.json`, under Application
  Support on macOS and `%APPDATA%` on Windows). Reduced motion follows Desktop's setting until it
  is changed in Battle. Only how Battle looks and sounds is kept there; Desktop still keeps the
  colony's record. Review pictures read and keep none of it.
- A friend match now carries the Handler's anonymous key: 128 random bits Formiga Desktop makes
  once for the colony and keeps with its Battle record (Battle contract version 5), so a friend's
  Battle can know the same Handler again. It is made from nothing about the Handler, the colony or
  the computer, a rehearsal has none, and the relay and each window drop anything that is not a
  key. The notice of what a friend match shares, and `docs/FRIENDS.md`, now list it. Relay
  protocol 4: a friend match needs both Handlers on a Battle that speaks it, and the site's relay
  must speak it too. Recognising a returning friend, rematches and rivalries come later.

## [0.67.14] - 2026-10-17

Built on Formiga Desktop 0.67.14, released alongside it and Formiga Hill and Home 0.67.14 so every
app has the same version again. Rules version 9, relay protocol 3 and Battle contract version 4
are unchanged. A friend match needs both Handlers on Battle 0.67.14 and Formiga Desktop 0.67.14.

- Less plain text where Battle should be pictures, painted at the notebook's pixel as its other
  pictures are. A match ends with a rubber stamp inked onto the board in the board's own letters,
  and the result is lettered on a ribbon beside a rosette: gold round the cup's colour for a win,
  green for playing well, two colours for a draw, a brass star for a Field Lesson learned and a
  pencilled one still to earn. Each thing a match earned sits beside its keepsake (a brass medal
  for points, a punched ticket for a friend match, the cup's rosette, a trophy, a key for a place
  opened, the colony's journal, a page of the Handler's Notebook, the trio's pennant). The turns
  are brass studs along the leather, the last ringed and glinting while it is given. The slips
  over the board saying a turn plays out, or that it is the Handler's move, are ribbons: the
  notebook's green, or the Handler's team colour. Cups' wins are counted in painted rosettes in
  each cup's colour, a place still to open hangs a padlock, and the place chosen has its name on
  a garden sign under its picture.
- For contributors: `--render-window <png>` makes the same picture of the window as `--snap`,
  with the same `--review`, `--at` and `--window`, without opening a window, by painting egui's
  last frame on the processor.

## [0.67.13] - 2026-10-16

A patch to 0.67.12, built on Formiga Desktop 0.67.11 as it was. Rules version 9, relay protocol 3
and Battle contract version 4 are unchanged. A friend match needs both Handlers on Battle 0.67.13
and Formiga Desktop 0.67.11.

- The CLEAR and TURN buttons on the orders given work again: the card under them took their
  clicks. The words on each card now wrap short of its buttons instead of running under them.

## [0.67.12] - 2026-10-16

Built on Formiga Desktop 0.67.11, with Battle contract version 4: each match played as a trio says
which three played. A Formiga Desktop 0.67.10 still reads its results, leaving out only who played.
Rules version 9 and relay protocol 3 are unchanged. A friend match needs both Handlers on Battle
0.67.12 and Formiga Desktop 0.67.11.

- A trio whose members change during one Battle session keeps every lineup that played as it,
  not only the three it had when Battle opened.

## [0.67.11] - 2026-10-16

Built on Formiga Desktop 0.67.10, as Battle 0.67.10 was. Rules version 9 is new and relay
protocol 3 is unchanged: a friend match needs both Handlers on Battle 0.67.11 and Formiga Desktop
0.67.10.

- Rules version 9, a new circle of orders: PRESS, HOLD and TRICK take the place of TAKE, DEFEND
  and INTERACT. A PRESS pushes anyone not holding one tile back, stops it for the turn and takes
  what it carries; two presses meeting cost each one composure and stop both; a HOLD turns a
  press away. Two presses from different sides still flank a lone holder. A HOLD now faces north,
  east, south or west, chosen when the order is given and shown on the board, and costs a presser
  coming at it head on one composure. A TRICK slips round a holder's side or back, costs it one
  composure and breaks its hold for the turn without moving it; from the front, or from where
  another holder of its team faces, it is read and comes to nothing, and anyone pressing catches
  a trickster. See docs/RULES.md.
- INTERACT is its own command for the board: levers, taps and switches, and fetching what turns
  up. A press stops a creature working one.
- USE ITEM is a bonus beyond the three orders, for a Formiga carrying something: it is used at the
  start of the turn from where the carrier stands, which takes its first step, and then it gets on
  with its job. Something taken from the other team can be used from the next turn.
- A switch can no longer set the map event off again in the turn it came round by itself, so a
  wave, the pond rising or a shaken tree comes at most once a turn.
- Recorded matches from earlier rules versions are refused rather than replayed differently, and
  Battles on rules version 8 and 9 do not meet in friend matches. The relay protocol stays 3; a
  plan may now carry a USE ITEM for each creature besides its three orders.
- Field Lessons, from the station: seven short lessons on the Backyard, under the real rules, on
  lighting the beacon, holding and which way a holder faces, flanks, tricks, team orders, the
  sprinkler and USE ITEM, and who your companions are. Each begins from a board set up part way through a match,
  with a goal, the squares that matter marked, notes where the field guide would be, and a few
  turns to do it in; the timer waits, and a lesson earns and keeps nothing. Your own three play
  them, doing exactly as told until the last lesson, where they are themselves. A colony that has
  never played is pointed to them. `--review lessons` and `--review lesson` picture them.
- A match against the visitors played from Formiga Desktop 0.67.8 leaves its notebook entries out
  of the result, which that Desktop would otherwise turn away whole.
- A gate a twig keeps from closing no longer goes on looking as if it is about to swing for the
  rest of the turn.
- A trio given a colour of its own flies the Handler's pennant as it is now in that colour, on the
  Handler's page and to a friend, rather than the pennant the Handler had when the colour was
  chosen.
- The Handler's name and club are kept as they are typed, so closing Battle before leaving the
  field no longer loses them.

## [0.67.10] - 2026-10-16

A patch to 0.67.9, built on Formiga Desktop 0.67.10, which takes the colony to Battle by train as
for a trip to Formiga Hill. Rules version 8 and relay protocol 3 are unchanged. A friend match
needs both Handlers on Battle 0.67.10 and Formiga Desktop 0.67.10.

- Battle no longer freezes after a turn with Interact orders, when two lines over the board came at
  the same moment.
- Going back to the station from the Handler's page, from any notebook's first page, or after a
  match no longer crashes Battle.

## [0.67.9] - 2026-10-16

Released with Formiga Desktop 0.67.9 and built on its shared crates, with Battle contract version 3,
so it needs Desktop 0.67.9. Rules version 8 and relay protocol 3 are new: a friend match needs both
Handlers on Battle 0.67.9, and an older Battle is told to update.

- The Handler's things are painted as the notebook's other pictures are. A pennant is chosen
  from sample tiles of cloth (its cut, cloth, symbol and edge) and stirs on its cane in a tuft of
  grass; the Handler's title is engraved on a brass plate, with a ribbon rosette for each cup won
  and a pencilled one for each still to win; each trio is a team picture taped into the
  notebook, its three on a strip of lawn by their pennant with their name on a ribbon, faded once
  retired; and a match opens under a ribbon strung between the two sides' pennants.
- Matches against the visitors write the Handler's Notebook too, as friend matches do: what each
  of the three was seen to do, and its moments, listed with what the match earned. The notebook
  also notes now when a companion tends to hang back or is good at flanking with a friend, and
  while you give orders, pointing at one of yours shows the first thing it has noted of it. This
  needs Desktop's Battle contract version 3 as it is on the desktop-battle-ai-notebook branch,
  which Battle is pinned to until it merges.
- Rules version 8: a flank in the yard takes the ground. When two of a team break a hold in the
  yard, those of them standing in the yard start holding where they stand, and the beacon goes up
  a tick for the breakthrough as well as for holding the yard alone. Before, about half of all
  matches on Backyard, the Millpond and the glade ended without anyone ever lighting a tick of the
  beacon, decided on composure or drawn; now about a quarter to a third do, more are won from
  behind, and matches are no shorter. A team that only digs in loses more often than before.
  Knowing your companions counts for a little less (59% for the coach who knows them on
  Backyard, from 63%). Matches recorded under version 7 no longer replay, and friends need the
  same version to play.
- While you give orders, faint footprints show who of your three would probably take which job
  with the plan as it stands, and pointing at one says what it would probably do and why
  ("probably flanks, keen to go"). It is your own team's sharing out only: how each walks, who
  thinks again and what the other team does stay unknown. A holder that two of theirs could
  flank now says so on its tag.
- As a turn plays, the lines that matter most come first: a knockout, a breakthrough or the
  beacon lit is never crowded out by a lesser line. When one of your own goes its own way (the
  long way round, beside a friend, along the edge, on a whim, hanging back), a word goes over it
  and the line says why, once a turn, so a companion being itself is not mistaken for a slip or
  the other team's doing.
- The match screen says more of what a turn came to. While orders are given, the "Just now"
  card lists what came of each of last turn's orders. The top strip says "Last turn" on the
  last turn and "One tick to go" beside a beacon one tick from lighting. A match won when time
  ran out shows the counts that decided it: beacon, composure and creatures standing.
- Your Handler, your trios and their records. "Your Handler & history" at the station opens the
  Handler's own page first: sign your name, add a club if you like, and choose a pennant from
  three cuts, eight cloths, twelve symbols and three trims, painted at the notebook's pixel. Your
  standing sits beside it: a title that rises with Open Circuit points, and a rosette for each
  cup won against the visitors, with "Champion of the Circuit" once all four are, each saying
  how it is earned. On the Who page, name the three you picked as a trio. Whenever those three
  are picked again, in any order, the match is played as that trio and counts on its own record,
  against friends and against the visitors apart, with its cup wins; any three still play as
  before, and nothing about a trio changes how they play. Two of a trio's three with someone new
  offer to put the new one in: the trio keeps its name and record, and remembers who played with
  it before and until when. Trios can be renamed, given their own colours, or retired and brought
  back. Your pennant flies at the station, beside your side's name in a match and in the
  introduction over the board as it begins ("The Thistle Three vs The visitors"); the result
  says how your trio stands now, journal lines for matches against the visitors name the trio,
  and History says which trio played. Against a friend, each side sees the other's pennant,
  club, standing and trio, and the notice of what a friend match shares lists them. Desktop
  keeps all of it with your colony (Battle contract version 3, Desktop save version 32), and a
  name signed on the Friend page becomes your Handler's.
- After each turn, each team order can be told in one line: who took it on, what came of it
  (held, flanked, turned away, caught on the way, getting into place to flank), and whether one of
  them went its own way doing it. Squares are named as a player sees them ("the middle of the
  yard"), not by number. `formiga-battle match` prints it; the match screen can show it.
- A match won when time runs out says which count decided it and by how much: ahead on the
  beacon, or level on it with more composure left, or with more still standing. It used to say
  "on the beacon tally" whatever decided it.
- `formiga-battle tournament` says how its matches ended: how many never lit the beacon, how
  many were won from behind on the beacon, and how many flanks led to beacon progress that turn.
- Rules version 7: team orders. Each turn a Handler gives the whole team up to three orders
  instead of one to each companion: TAKE some squares (walk onto an empty one, move theirs off
  one, two flanking a holder), DEFEND some squares (stand on them, guard beside them, head off
  whoever comes), or INTERACT with something (fetch a found thing, pull the lever or set off a
  switch, go after what one of theirs carries, or have one of yours use what it carries). Naming
  a square in two orders makes it more pressing and brings a second companion to stand guard.
  The team shares the plan out by itself: what the plan needs first, then who is nearest and
  what each is already doing, and last, by no more than about two steps' walk, who each one is.
  A bold one would rather be the one to meet the other team, a wary one the one to stand and
  hold, a sociable one the one beside a friend, a curious one the one to fetch what lies in the
  grass. Alone where two were needed to flank, a reckless one goes at the holder anyway and a
  wary one waits beside it; and partway through a turn, a companion whose job came to nothing
  (what it went for was taken, its target dug in or was knocked out, a teammate got there first,
  the ground was marked for the map event) thinks again from the same plan, once. The account
  says each team's plan, who took on what and why. Personality pulls half as hard on the way a
  companion walks as it did, so that again about one in three goes its own way each turn: with
  team orders they walk further and more often, and at the old pull nearly half did, which left
  the kinds that lean hardest well behind against the thinking coaches. Friend matches speak relay
  protocol 3, and recorded matches from earlier builds no longer replay.
- Friend matches are more careful with the relay: Battle follows no redirects and never leaves
  HTTPS, takes a room's code and seat only in the shapes a relay gives them, cleans the other
  Handler's name and any reason for an ending before showing or keeping them, and waits longer
  between looks while the relay cannot be reached. The notice of what a friend match shares now
  names everything sent for each companion and when the relay deletes a room. The local relay
  for development asks for JSON and refuses bodies nested too deep, as the site's does.
- Rules version 6: every arena has a map event, warned on the board the turn before it comes
  round and set off early from a switch of its own. Backyard's sprinkler sweeps the whole yard
  (turns 4, 8 and 12; a garden tap on each side). Sandy Cove's wave washes whoever stands on the
  sandbar's ends toward the middle of the beach (every third turn; a big shell on each side).
  Millpond's water rises over the banks either side of the jetty for the rest of the turn, washing
  off whoever waits there to come at the yard (turns 5 and 10; a sluice on each bank). In Woodland Glade, a shaken
  oak sheds acorns on whoever stands round it and drops a twig (a breeze shakes a pair every fourth
  turn; a contest shakes one). Nobody loses composure to any of them. Recorded matches from earlier
  builds no longer replay.
- Found things: a dandelion puff, a twig, a leaf umbrella, a pebble bell, a honey drop or, rarely,
  a springy seed glints on two mirrored spots and turns up the turn after. Whoever steps onto one
  picks it up and can USE it once: a puff bowls over whoever stands across its way, a twig blocks a
  tile for two turns (keeps an open gate from closing, and floats as a raft in the millpond), a
  leaf umbrella keeps a teammate dry and bounces contests off, a bell spoils the other team's
  contests nearby (and sets the map event off, rung by a switch), honey stops whoever steps in,
  and a seed hops its user two tiles. Cautious companions keep off ground about to be hit;
  curious, playful ones go out of their way for what lies in the grass.
- Battle opens at the Battleground, the first spread of the Handler's notebook. On its left page
  the colony's train, the one Desktop and Hill share, pulls in with every companion already riding
  it, a face at each window, and a coach more for every three past six. On its right, two railway
  tickets: Out in the World, to play a friend online, and Close to Home, to play the visiting team
  offline, painted at the notebook's own pixel with a globe and a cottage on their stubs. Above them,
  a little scene taped onto the page: two of your companions facing each other across the beacon
  under bunting, now and then cheering, with the page's title on a ribbon. A click hurries the train in; with reduced motion it is already in. After a match, Back
  to the station brings it in again.
- History & settings, beneath the two tickets: every match the colony has played with who went
  and how it ended, beside the Open Circuit points and each cup's progress; and reduced motion and
  sound for the sitting, with keys to remember.
- Every button, card and switch answers a touch the same way: it lifts when pointed at, sinks when
  pressed and springs back, with a brass sparkle for the way on and a shake for the way back. Cards
  drop onto each page as it turns, a chosen companion hops, and the next place in the team breathes
  until it is filled. None of it moves under reduced motion.
- Getting ready is the Handler's notebook, its pages turned as Desktop's journal turns them: where
  to play, with the place pictured large and each cup's progress; who goes, every companion on a
  card with temperament, habits and notebook note, beside a lineup of the three chosen; and, for a
  friend, the room. Tabs along the top turn back to any page, and the way back to the station is
  beside them.
- The friend's room is its own page: sign the name your friend sees on a line, then open a room or
  join one side by side. An open room's code is shown on a ticket with a button to copy it, and a
  mistyped code says what a code is made of.
- A turn is told over the board as it plays: a line over the head of whoever it happened to, a
  word in Formiga's own pixel lettering where it happened (POW!, CLANG!, WHAM!, BOOM!, MISS!), the
  composure it cost floating up as a number, and a sound for each. The last two lines stay on a
  card in the board's corner, and the whole match, in full, opens from there (L).
- A match begins with READY? and GO!, each turn's orders with a slip in the Handler's colour, and
  it ends with HOORAY!, SO CLOSE! or A TIE! big on the board, and confetti for a win.
- The team's orders are three coloured tokens on the notes page: pick TAKE, DEFEND or INTERACT (or
  T, D, I) and click the board, or drag across it to name more squares; a click takes or interacts,
  a right click defends. Each order's squares are washed in its colour (gold, sky, plum) under a
  flag, a shield or a sparkle, side by side where two orders name one square, and a card by the
  pointer says what a click will do. As a turn is revealed, footprints run from each of the team to
  the job it took on; where a companion's leanings chose its job it says so over its head
  (CHARGE!, STEADY!, PALS!, OOH!), and a change of plan mid-turn is told with why.
- The right page keeps the orders and a glance at the arena: how to win, the orders and the
  board's marks, one at a time, the detail on hover.
- Sound: cheerful placeholder effects and music, all made in code, with music and sound volumes
  and a mute behind the speaker on the leather. `formiga-battle sounds <folder>` writes them all
  as WAV files to listen to.
- Rules version 5: a creature's personality pulls four times as hard on the way it walks, so
  each turn about one of a team's three goes its own way rather than the plain way: sometimes
  none, sometimes two (about one team-turn in four before). Fewer matches are won at the beacon
  (13%, from 21%), and the kinds score further apart under the Exhibition coach. Recorded matches
  from earlier builds no longer replay.

## [0.67.8] - 2026-10-15

The first Battle to download: an app for macOS and an installer for Windows, released with Formiga
Desktop 0.67.8 and built on its shared crates. A friend match needs both Handlers on Battle 0.67.8.

- Battle comes as an app to download for macOS and Windows, with an icon of its own: the
  Backyard's beacon between the two teams' pennants. Installed in Applications, or with the
  Windows installer, Formiga Desktop finds it on its own.
- Built on Formiga Desktop 0.67.8, which keeps every cup's trophy and medal (the Woodland,
  Millpond and Sandy Cove Cups' as well as the Backyard Cup's) and offers them to Formiga Home,
  and reads its colonies (save version 31) for `--from-save`. A rehearsal from a colony file says
  it was opened by the Desktop this Battle is built on, rather than naming 0.67.7.

## [0.67.7] - 2026-10-15

The first release of Battle. Its version now follows the Formiga release it ships with, and it is
built on Formiga Desktop 0.67.7's shared crates. A friend match needs both Handlers on Battle
0.67.7.

- A fourth arena, Sandy Cove: the beacon halfway along a sandbar one tile wide, open to both
  sides, between columns of sea-worn rocks, with the sea, a lighthouse and beach huts round it.
- Battle looks like Formiga's own: the window is the Handler's notebook in Home's leather,
  paper and stitching, with Battle's name on a cloth patch, the board on the left page, the
  orders and the notebook as tabs on the cover, and cards, buttons, a brass watch for the timer
  and a row of lamps for each team's beacon in Home's and Hill's colours and stepped edges. The
  Backyard is a garden: mown lawn, a paved yard round a brass beacon, hedges, a brick wall with
  a picket gate, a hedgerow, a tree, a potting shed and bunting, with a robin, butterflies and
  light in the leaves. Two new arenas, Woodland Glade and Millpond, are dressed as places of
  their own. Marks on the board are corner brackets, footprints and pennants rather than filled
  tiles, with a key beside the board; a hedge someone stands behind is seen through. The board
  shows a hold broken by flanking, a creature off balance for the rest of the turn, holders
  bracing together, a wary one hanging back, and the beacon paused while both teams hold the
  yard. A slip names each turn as it plays out, a stamp comes down when the match ends, and
  pages settle in as they change. Reduced motion holds everything still.
- A cup for each arena: the Backyard Cup opens the Woodland Cup on Woodland Glade, and that opens
  the Millpond Cup on Millpond, and that the Sandy Cove Cup, each won by three wins there with its
  own trophy and medal. A friend match is played on the arena the Handler who opened the room
  picked.
- A friend match earns only once the relay confirms both windows came to the same board after its
  last turn: a match whose boards first differ on the last turn no longer counts on one side. A
  room its opener leaves before anyone joins closes at once, and the development relay answers a
  request too big to read with a refusal rather than dropping the connection. Battle's whole-match
  relay test runs against the relay `FORMIGA_BATTLE_RELAY` names, when it names one.
- Only matching, unaltered copies play a friend: a joiner must have the same Battle, opened by
  the same Desktop, and the same copy of Battle (its program's SHA-256) as the friend who opened
  the room, or both copies must be builds the relay knows as that version's release. Someone with
  another version is told which their friend has. A match whose two boards came out differently
  earns nothing. The relay speaks version 2, and the window says what is shared for this.
- Rules version 4, encirclement: two contesters coming at a holder from two sides at once break
  its hold. It loses one composure and is pushed off its tile, out of the yard if it can be, or
  knocked off balance where it stands, and either way stops counting in the yard for the rest
  of the turn. Two holders corner to corner turn a flank away together. The beacon now counts
  the handoff's way: progress pauses whenever someone of the other team stands in the yard
  unbroken, and three ticks of having it to yourselves light it. The Backyard's yard is a cross
  of five tiles rather than a square of nine. A wary creature hangs back only when sent to walk,
  and a nervous one steps back only when its contest is turned away. A team that only ever
  holds the yard now loses to every coach: 68% to 90% of the time, where it used to beat the
  visitors nearly always.
- What a match earns, kept by Desktop: a friend match earns Open Circuit points, a match-history
  line, notebook entries and a match played for each of the three; a match against the visitors
  earns Backyard Cup progress (three wins win the cup, a trophy and a medal, and unlock the
  Woodland Cup) and a journal entry. Battle writes it into Desktop's session after every match
  and keeps nothing of its own: the notebook now lives in Desktop's record. Contract version 2.
- Play a friend: one Handler opens a room and reads out its six-character code; the other joins.
  Formiga's relay passes each turn's orders on once both are in or at the deadline, never runs
  the rules, and deletes the room when the match ends. Both windows work out every turn and
  check each other's board. Before anything is sent, the window says exactly what is shared and
  how long it is kept. The relay's address is built in and never shown.
- The Handler's Notebook: after every match, each of your companions' pages notes what it was
  seen to do (the ways it keeps taking, the chances it takes, the friend it stays near) and a
  few moments worth remembering, drawn from its last ten matches so the notes change as it does.
  Shown on the team cards and beside the board; kept in Battle's own data folder.
- Coaches that think a turn ahead, one knowing its creatures and one imagining them plain, and
  the tournament to measure what knowing them is worth.
- Rules version 3: knowing your creatures counts for more. A wary creature sent to walk or hold
  waits a tick, once a turn, rather than step in among the other team, and impulse picks a
  second-best way far less often, so what a creature does can be learned; a coach who knows its
  creatures now wins 52.4% against one who plans as well without knowing them (from 51.1%).
  The notebook notes who hangs back.
- Rules version 2: a bold creature no longer goes out of its way into the other team's reach,
  and company and lanes count as a way's average, so no kind of creature does consistently
  better under strong coaching.
- The window: choose three companions, give each an order on its card and the board with a turn
  timer, lock in against the visiting colony's AI, and watch each turn play out on an isometric
  Backyard drawn in Formiga Home's style, with every creature in Desktop's own frames, a recap of
  why each did what it did, and the result. Review pictures with `--render-board` and `--snap`.
- Battle's side of being opened by Desktop: `--formiga-battle <dir>` reads the roster Desktop
  wrote and answers with an acknowledgement, or why not.
- The rules sandbox: Backyard, the three orders, simultaneous ticks with the group-pressure rule,
  composure and playful knockouts, the beacon, the gate and its lever, the turn cap, the fallback
  for missing orders, the Exhibition AI coach, recaps that say why, replays that check every turn,
  and the fairness tournament.
- Profiles read from Desktop 0.67.6's own record: temperament, learned leanings and bonds, with
  the roster contract's first version.
