# Changelog

## Unreleased

- Play a friend: one Handler opens a room on a room service (`formiga-battle serve`) and passes
  on its six-character code; the other joins. The room locks each turn when both have given
  orders or at the deadline, resolves it, and both windows check their board against the
  room's. A Handler who drops out keeps their seat and can come back; three turns in a row with
  no orders forfeits, said in words.
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
