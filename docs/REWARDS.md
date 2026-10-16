# What a match earns

Battle keeps nothing of its own. Desktop keeps a colony's Battle record and hands it to Battle in
each session's snapshot; Battle writes what each match earns into the session as it goes
(`result.json`, written whole after every match), and Desktop applies it when Battle closes, each
match once, by its id. A rehearsal, with no Desktop behind it, keeps nothing at all.

| Against | Earns |
|---|---|
| A friend | Open Circuit points, a line in the match history, notebook entries for the three who played, and a friend match played for each of them |
| The visitors (the AI) | Backyard Cup progress, then the cup and the Woodland Cup unlock; a journal entry; keepsakes for Home to show |

Nothing else earns anything, and nothing earned changes how a match is played.

## Friend matches

A friend match's earnings are kept back until the relay confirms that both windows came to the
same board after the last turn (`docs/RELAY.md`). If the boards differ, or the room closes before
both are in, the match earns nothing. A match ended by a forfeit counts as it stands.

| | |
|---|---|
| Open Circuit points | 10 for finishing, 5 more for a win or 2 for a draw. From the sixth friend match in one day (UTC), 2 each, so playing the same friend over and over is never the way to points. A Handler who forfeits earns none |
| Match history | When, won or lost, how it ended, in how many turns, the three who played, and the other Handler's name as they typed it. The newest 20 are kept |
| Notebook | What the match showed of each of the three (`docs/PERSONALITY.md`), and its moments. A page keeps its last ten friend matches |
| Participation | A friend match played, for each of the three |

## Matches against the visitors

| | |
|---|---|
| Backyard Cup | Each win counts toward the cup. The third wins it: a trophy and a medal (`battle.trophy.backyard_cup`, `battle.medal.backyard_cup`), and the Woodland Cup unlocks (`cup.woodland`) |
| Journal | A line for every match: "Mochi, Biscuit and Fig won a Backyard Cup match against the visitors.", and one for winning the cup |

## The contract

`formiga-battle-contract` version 2 adds the record to the snapshot (`BattleRecord`) and the
result Battle answers with (`BattleResult`), and holds the one rule both sides apply:
`BattleRecord::apply` adds a match's points, history line, notebook additions, cup win and
unlocks, once per match id. Journal lines and keepsakes are Desktop's to keep. A result is
refused if a friend match carries AI rewards or an AI match carries friend rewards.
