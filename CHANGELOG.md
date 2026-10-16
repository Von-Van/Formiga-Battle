# Changelog

## Unreleased

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
  says each team's plan, who took on what and why. Friend matches speak relay protocol 3, and
  recorded matches from earlier builds no longer replay.
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
  Millpond's water rises over the jetty's sides, washing off whoever is there and leaving the yard
  a line until the turn after (turns 5 and 10; a sluice on each bank). In Woodland Glade, a shaken
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
  a win ends in confetti.
- A click on the board does what the tile suggests: the ground to MOVE, one of theirs or the lever
  to CONTEST, the chosen one itself to HOLD where it stands; a right click holds there. A card by
  the pointer says what a click will do, and an arrow bobs over whoever is chosen.
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
  own trophy and medal. A
  friend match is played on the arena the Handler who opened the room picked.
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
