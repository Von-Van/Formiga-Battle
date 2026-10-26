# What a match earns

Battle keeps two small files of its own: how it looks and sounds (`settings.json`, see the
[README](../README.md)), and each colony's record against the named rivals (see [The career
against the rivals](#the-career-against-the-rivals)). Nothing a match earns goes in either. Desktop
keeps a colony's Battle record and hands it to Battle in each session's snapshot; Battle writes
what each match earns into the session as it goes (`result.json`, written whole after every
match), and Desktop applies it when Battle closes, each match once, by its id. A rehearsal, with no Desktop behind it, keeps no record at all.

| Against | Earns |
|---|---|
| A friend | Open Circuit points, a line in the match history, notebook entries for the three who played, and a friend match played for each of them |
| The visitors (the AI) | Progress toward the cup of the arena played on, and once it is won the cup, its trophy and medal and the next cup's unlock; a journal entry; keepsakes for Home to show; notebook entries for the three who played |

Nothing else earns anything, and nothing earned changes how a match is played.

## Friend matches

A friend match's earnings are kept back until the relay confirms that both windows came to the
same board after the last turn (`docs/RELAY.md`). If the boards differ, or the room closes before
both are in, the match earns nothing. A match ended by a forfeit counts as it stands.

| | |
|---|---|
| Open Circuit points | 10 for finishing, 5 more for a win or 2 for a draw. From the sixth friend match in one day (UTC), 2 each, so playing the same friend over and over is never the way to points. A Handler who forfeits earns none |
| Match history | When, won or lost, how it ended, in how many turns, the three who played and the trio they played as, and the other Handler's name as they typed it, with their trio's name and pennant. The newest 20 are kept |
| Notebook | What the match showed of each of the three (`docs/PERSONALITY.md`), and its moments. A page keeps its last ten matches, against friends and the visitors alike |
| Participation | A friend match played, for each of the three |

## Matches against the visitors

| | |
|---|---|
| Cups | Each cup is played on its own arena: the Backyard Cup in the Backyard, open from the start; the Woodland Cup (`cup.woodland`) on Woodland Glade; the Millpond Cup (`cup.millpond`) on Millpond; the Sandy Cove Cup (`cup.cove`) on Sandy Cove. A win counts toward the cup of the arena it was played on, if that cup is open. The third wins it: a trophy and a medal (`battle.trophy.<arena>_cup`, `battle.medal.<arena>_cup`), and the next cup unlocks under its id. The visitors play only on arenas whose cup is open; friend matches may be played on any arena |
| Journal | A line for every match: "Mochi, Biscuit and Fig won a Backyard Cup match against the visitors.", and one for winning a cup |
| Notebook | As for a friend match: what the match showed of each of the three, and its moments |

## Trios

A trio is three companions the Handler has named as a team, on the Who page. Whenever those
three are picked, in any order, the match is played as that trio, and it counts on the trio's own
record whoever the match was against: matches against friends and against the visitors, each
played, won and drawn, and its wins toward each cup. Points still come only from friends, and
cups only from the visitors. Any three still play exactly as before, and a trio changes nothing
about how they play.

A trio's members can change: picking two of its three with someone new offers to put the new one
in. The trio keeps its name and record, and the three it had before are kept with the day they
stopped playing together (the last eight lineups), so a past match is never credited to someone
who was not there. A trio can be renamed, given its own colours or retired (kept with its record
but no longer recognised); eight can be flown at once, and twelve kept in all. A companion who
has left the colony leaves an empty place in its trio, and the trio's record stays.

The Handler's own page keeps their name, club and pennant, and their standing: a title that
rises with Open Circuit points (New Handler, then Handler at 30, Seasoned Handler at 100, Circuit
Regular at 250, Circuit Veteran at 500 and Circuit Legend at 1000, all provisional), and the cups
won against the visitors, with "Champion of the Circuit" once all four are.

## The contract

`formiga-battle-contract` version 2 adds the record to the snapshot (`BattleRecord`) and the
result Battle answers with (`BattleResult`), and holds the one rule both sides apply:
`BattleRecord::apply` adds a match's points, history line, notebook additions, cup win and
unlocks, once per match id. Journal lines and keepsakes are Desktop's to keep. A result is
refused if a friend match carries AI rewards or an AI match carries friend rewards; notebook
additions are both kinds' since version 3.

Version 3 adds the Handler card and trios to the record, the trio a match was played as to its
rewards and history line, and the registry a session ends with to the result.
`BattleRecord::register` keeps the registry (the card, and each trio's name, members, pennant
and retirement) before the session's matches are applied, so a match played as a trio named
that session counts for it; only `apply` adds to a trio's record.

## The career against the rivals

The visitors close to home are four named Handlers (`crates/formiga-battle/src/rival.rs`), each
playing one of the coaching styles in [RULES.md](RULES.md#coaching-styles). How a colony has fared
against each (played, won, drawn and lost, and the last meeting) is the one record of play Battle
keeps itself (Jake, 2026-10-23), in `careers/<colony id>.json` in Battle's data folder, until a
Desktop contract carries it. Each match is counted once, by the same match id Desktop applies it
by; a file for another colony, or one that cannot be read, is a fresh career rather than a refused
match. A rehearsal keeps none. The career earns nothing and changes nothing about how a match is
played.

Events are kept there too: the Open under way or last finished, as a bracket
(`crates/formiga-battle-world`), how many have been entered, and the titles won. An event's
fixtures against the colony are ordinary matches against the visitors, earning toward the place's
cup as any other; the rivals' own fixtures are played out under the same rules and earn nothing.
A fixture is recorded once, by its match id, so a crash never plays it twice.

A season (`formiga-battle-world`'s `season.rs`) is an Open of four, a Spring and a Summer Regional
of eight, and a Championship of the four with most season points, with last season's champion
defending. Season points (8, 5, 3 and 1 for winning, the final, a semi-final and a quarter-final)
decide only seeding and who reaches the Championship, and start from nothing each season. The
career keeps the season, every season's champion for the Hall of Champions, and what the
Battleground noticed, each with the match ids that show it. The colony's standing comes from what
it has won, never from how much it has played.
