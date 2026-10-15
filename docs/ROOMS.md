# Rooms: playing a friend

Two Handlers play each other through a room service (`crates/formiga-battle-room`). The room is
authoritative: it holds the match, locks each turn, resolves it with the same rules crate, and
sends both clients the orders it locked. Each client resolves the turn itself from those orders,
so it can animate it, and checks its board's hash against the room's.

## Running one

```bash
cargo run --release -p formiga-battle -- serve                      # this Mac only: 127.0.0.1:7878
cargo run --release -p formiga-battle -- serve --bind 0.0.0.0:7878  # anyone on the local network
```

In the window, under "Play a friend", one Handler opens a room and reads out its six-character
code; the other types it and joins. `FORMIGA_BATTLE_ROOM` sets the address the window offers.

The service is plain TCP, one JSON message a line. On the internet it goes behind a TLS proxy;
where it is hosted is still open.

## A room, start to end

| Step | What happens |
|---|---|
| Hello | Each connection says its protocol and rules version first. A room that speaks another version refuses it in words, before anything else |
| Open | The opener sends three companions, as Desktop's roster has them, and is given a code and a seat token. The opener plays the left team |
| Join | The friend sends the code and three companions. The room checks both teams (three different companions, every field within Desktop's bounds, every look drawable), picks the seed, and tells both the match has begun |
| A turn | Each side sends its orders; resending replaces them until the turn locks. The other side is told when one is ready. The turn locks when both are in, or at the deadline: the ruleset's 40 seconds plus 3 for the network. Orders not given, or that cannot be carried out, are filled by the rules' fallback, the same on both sides |
| Resolved | Both are sent the orders as locked, the board's hash and the outcome. A client whose board differs asks for the whole match again |
| Away | A dropped connection keeps its seat. The other Handler is told; the timer runs on and the fallback gives that side's orders. Coming back with the seat's token, a Handler is sent the whole match so far and carries on |
| Forfeit | Three turns in a row with no orders from a side, and it forfeits, said in words to both |
| Closing | A room nobody joins closes after 15 minutes; a room with nobody connected after 2; a finished room is kept 2 minutes |

## What a room trusts

A code opens one room and proves nothing about who anyone is. A seat token lets its holder back
into that seat and nothing more; tokens are compared in constant time. Nothing about a match with
a friend is written into anyone's colony: rewards from it wait for a trust model of their own.
The room logs only how many rooms are open: never a code, a token, a name or anyone's companions.
Every line is bounded (128 KiB) before it is parsed, a connection may send at most 20 messages a
second, and the service opens at most 500 rooms.
