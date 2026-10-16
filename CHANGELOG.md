# Changelog

## Unreleased

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
