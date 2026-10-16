# Playing a friend

Two Handlers play each other through Formiga's relay, a small service on the Formiga site. The
relay never runs the rules: it holds each side's orders until the turn locks and then hands both
sides the same locked orders. Each window works out the turn itself with the same rules crate,
and both tell the relay the board they came to, so a difference shows at once. Players never see
the relay or its address: Battle knows it (`https://www.formigaworld.com/api/battle`), and only
development points elsewhere (`FORMIGA_BATTLE_RELAY`).

## A match with a friend

| Step | What happens |
|---|---|
| Before anything is sent | The window says, every time, exactly what a friend match shares and how long it is kept, and nothing can be opened or joined until the Handler ticks "I understand what is shared" |
| Open | One Handler opens a room and reads out its six-character code; they play the left team |
| Join | The friend types the code and joins with their three. The relay checks the request's shape, and that both play by the same rules with the same Battle, the same Desktop and the same copy of Battle (below); each window checks both teams, and both versions, itself before the match begins |
| A turn | Each side sends its orders once. The other side is told only that they are ready, never what they are. The turn locks when both are in, or at the deadline: the ruleset's 40 seconds plus 4 for the network. Orders not given in time are filled by the rules' fallback, identically in both windows |
| Working it out | Each window resolves the locked turn, shows it, and reports the board's hash. Two different hashes for one turn mark the match out of step, and both are told it cannot be trusted |
| Away | A side that sends no orders for three locked turns in a row forfeits, said in words to both. Leaving a match forfeits it |
| The end | When both windows report the match over (or at the turn cap), the room is over. Once both have reported their last board, the relay deletes the room |

## What is shared, and kept

| | |
|---|---|
| Sent to the relay and on to the friend | The Handler's display name as typed; their three companions as Desktop's roster has them (name, look and size, what each wears, how it moves, temperament, traits, habits and learned leanings, the number Desktop knows each by, when each was born, its parent's number for a little one, and how the three get on); each turn's orders; a hash of the board after each turn; Battle's version, the version of the Desktop that opened it, and the SHA-256 of Battle's program file (the same for everyone with that copy) |
| Never sent | Anything else from the colony: the other companions, the journal, finds, homes, Desktop's settings. No account, email, device or computer identifier |
| Kept by the relay | Only while the room is open. Deleted when both sides have seen the end, after 15 minutes if nobody joins, after 5 minutes with nobody asking, after 2 minutes once over, and after 2 hours whatever happens. A seat's key is kept only as its SHA-256 digest. Nothing is logged about a room |
| Kept by the site's host | Its usual short-lived request logs, as for any visit to formigaworld.com |
| Kept on this computer | What the match earned, written to the colony through Desktop when Battle closes |

## Only matching, unaltered copies

A friend match is played only between the same Battle opened by the same Desktop, and only
between unaltered copies, as far as the relay can tell:

| Check | What it stops |
|---|---|
| Battle's version and Desktop's version must be the opener's | An old Battle or another Desktop: the joiner is told which versions their friend has. A rehearsal (no Desktop) plays only another rehearsal |
| Battle's digest must be the opener's, unless the relay knows both as builds of that version's release | A copy built somewhere else, or changed |
| Once a version's released builds are known, only they may open or join | A changed copy of a released version |
| Each window checks the other's versions, and that the relay's ruleset is its own | A relay or a friend's window that has fallen out of step with this Battle |
| Both windows report the board after every turn; a match earns only once the relay confirms both windows came to the same board after its last turn | A copy changed to play by other rules, even on the last turn |

None of this can stop a copy changed on purpose to report what a released one reports: whatever a
program says about itself, a changed program can say too. The checks keep out honest mistakes and
make playing with a changed copy a deliberate effort; the board check is what catches a copy that
plays by other rules.

`docs/RELAY.md` is the relay's contract, request by request; `crates/formiga-battle-room/src/relay.rs`
is its reference, and `formiga-battle relay` serves that reference locally for development.
