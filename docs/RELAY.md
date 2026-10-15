# The relay's contract

What the relay on the Formiga site must do, request by request. The reference is
`crates/formiga-battle-room/src/relay.rs`; where this page and the reference differ, the reference
is right and this page is wrong. `formiga-battle relay` serves the reference locally, and Battle's
tests play whole matches through it.

The relay never runs the rules and never needs to know them. It stores JSON it is given, checks
its shape and size, and keeps time.

## Requests

Base: `https://www.formigaworld.com/api/battle`. Every body and answer is JSON, at most 64 KiB in
and 256 KiB out. Every answer carries `Cache-Control: no-store`. The seat travels only in the
`X-Formiga-Seat` header, never in a URL.

| Request | Body | Answer |
|---|---|---|
| `POST /rooms` | an Entrant | 201 `{"code", "seat"}` |
| `POST /rooms/{code}/join` | an Entrant | 201 `{"code", "seat"}` |
| `GET /rooms/{code}` | | 200 a View |
| `POST /rooms/{code}/orders` | `{"turn", "orders"}` | 200 a View |
| `POST /rooms/{code}/board` | `{"turn", "hash", "finished"}` | 200 a View |
| `DELETE /rooms/{code}` | | 200 `{}` |

A code is six characters from `34679ACDEFGHJKMNPQRTUVWXY`, read generously (any case; spaces and
dashes ignored). A seat is 32 lowercase hex digits from a cryptographic random source.

An **Entrant** is `{"protocol": 2, "rules_version": n, "build": a Build, "handler": "…", "team": [3 contenders], "bonds": [≤ 3]}`.
The relay checks `protocol == 2`, three contenders and at most three bonds, and trims `handler` to
at most 24 printable characters ("A Handler" if empty). A joiner's `rules_version` must equal the
opener's. It does not look inside the contenders.

A **Build** is `{"battle": "0.1.0", "desktop": "0.67.7" or null, "digest": "…"}`: Battle's
version, the version of the Desktop that opened it (`null` in a rehearsal), and the SHA-256 of
Battle's program file as 64 lowercase hex digits. Versions are 1 to 24 characters of letters,
digits and `.+-`; anything else is malformed. The relay knows the **released builds**: for some
Battle versions, the set of digests that version was released as (one per platform), and nothing
for any other version. Then:

- An entrant whose Battle version has released builds, and whose digest is none of them, is
  refused `build`, opening or joining.
- A joiner whose `battle` or `desktop` differs from the opener's is refused `mismatch`, naming the
  opener's (`null` and a version differ).
- A joiner whose digest differs from the opener's is refused `build`, unless the version has
  released builds (both digests are then among them).

The site keeps the released builds in its own repository, in `lib/battle-builds.mjs` (`{"0.2.0":
["<digest>", …]}`), changed by a pull request when a Battle release's digests are known; the
reference takes them through `Relay::knowing`.

`orders` is three entries, each `null` or an order as Battle writes it; the relay stores them as
given. `hash` is 16 lowercase hex digits. 64-bit numbers (`seed`, `hash`) always travel as 16 hex
digits, because JavaScript numbers lose anything past 53 bits.

## Refusals

| Status | Body | When |
|---|---|---|
| 400 | `{"refused": "malformed"}` | The body is not the shape above, or too large |
| 404 | `{"refused": "no_such_room"}` | No room under that code, or a seat that is not one of its two (the same answer, so a seat cannot be guessed at) |
| 409 | `{"refused": "version", "relay": 2, "rules_version": n}` | Another protocol, or a joiner playing by other rules than the opener (`n` is the opener's) |
| 409 | `{"refused": "mismatch", "battle": "…", "desktop": "…" or null}` | A joiner with another Battle or Desktop than the opener's (these are the opener's) |
| 409 | `{"refused": "build"}` | A copy of Battle the relay does not admit, or not the opener's copy |
| 409 | `{"refused": "room_full"}` | Joining a room that already has two |
| 409 | `{"refused": "not_this_turn"}` | Orders for a turn that is not the one being played; a board for a turn not yet locked |
| 422 | `{"refused": "team", "reason": "…"}` | Not three companions |
| 503 | `{"refused": "busy"}` | 500 rooms open already |

## A room

A room is `waiting` until joined, then `playing`, then `over`. Joining picks the match seed (64
random bits) and sets turn 1's deadline to now + 44 seconds.

Before answering any request about a room, the relay brings it up to now: while `playing` and
the deadline has passed, it locks the turn with whatever orders each side has sent (`null` for a
side that sent none), sets the next deadline 44 seconds after the one that passed, and repeats.
A side with no orders in three locked turns running forfeits: the room is `over`, with
`ending: {"forfeited": side, "reason": "<name> gave no orders for 3 turns in a row."}`. When
12 turns are locked the room is `over` (`reason: "The match is over."`).

- **Orders** are kept for the turn being played; sent again before the turn locks, they replace
  the last. The moment both sides have sent orders, the turn locks.
- **Board** reports are kept per side as `(turn, hash, finished)`. When both sides have reported
  the same turn: different hashes set `out_of_step`; the same hash with both `finished` makes the
  room `over` with `reason: "The match is over."`.
- **Leave** while waiting closes the room. While playing, the leaver forfeits:
  `ending: {"forfeited": side, "reason": "<name> left the match."}`.

A **View** is everything one side may see: `relay`, `code`, `stage`, `you` (`"left"` or
`"right"`), `entrants` (both, once joined), `seed`, `ruleset` (Battle's default ruleset, as JSON),
`turn`, `deadline_ms`, `opponent_ready` (whether the other side has sent orders this turn, never
what they are), `opponent_quiet_ms` (since the other side last asked), `locked` (every locked turn,
oldest first: `{"turn", "submitted": [left orders, right orders]}`), `out_of_step`, and `ending`
once over.

## Keeping nothing

A room is deleted, with everything in it, at the first request after any of: both sides have seen
it `over` having reported their board for the last locked turn (or it ended by a forfeit); 15
minutes waiting; 5 minutes without a request from either side; 2 minutes `over`; 2 hours since it
opened. The store's own expiry should be set to match, so a room nobody asks about again is still
gone. A seat is stored only as its SHA-256 digest, compared in constant time. The relay logs
nothing about a room: no code, seat, name, team or orders.
