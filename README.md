# Formiga Battle

A separate desktop app for Formiga Desktop's companions: isometric, simultaneous-order 3v3
tactics, against the AI or a friend in a private room. You are the Handler: each turn you give
each of your three Formigas one order (MOVE, HOLD or CONTEST), both teams' orders lock together,
and the turn plays out at once. How each creature carries out its order comes from who it is in
Desktop, never from stats, so knowing your Formigas is the skill and rerolling them for a better
trio is not.

Battle opens on a window where you choose three of your companions and play the visitors, three
companions from a neighbouring colony, in the Backyard: an isometric board in Formiga Home's
style, the creatures drawn exactly as Desktop draws them. Each turn you give your orders on their
cards and the board (or let the timer run out and they choose for themselves), lock in, and watch
the turn play out tick by tick with a recap that says why each of them did what it did. The same
match also runs headless from the command line. There is no online play yet, and Desktop does
not open Battle yet: Battle's side of that hand-off is built (`--formiga-battle <dir>`), Desktop's
is still to come.

- [docs/RULES.md](docs/RULES.md): how a match resolves, every provisional number, and the
  fairness tournament's results.
- [docs/PERSONALITY.md](docs/PERSONALITY.md): every part of Desktop's record of a companion, and
  what Battle does with it.

## Commands

```bash
cargo run --release -p formiga-battle                                    # the window, on the sample colony
cargo run --release -p formiga-battle -- --from-save <colony.json>       # on a colony, read and never written
cargo run --release -p formiga-battle -- --help
cargo run --release -p formiga-battle -- roster                          # the sample colony's companions
cargo run --release -p formiga-battle -- roster --from-save <colony.json> # read, never written
cargo run --release -p formiga-battle -- match --seed 4 --board          # an AI match, told turn by turn
cargo run --release -p formiga-battle -- match --from-save <colony.json> --left 0,2,3 --record m.json
cargo run --release -p formiga-battle -- replay m.json                   # replays and checks every turn
cargo run --release -p formiga-battle -- tournament --pairings 5000      # fairness by kind and trio
cargo run --release -p formiga-battle -- arena                           # draws Backyard
```

In the window: choose a card (or 1, 2, 3), then MOVE, HOLD or CONTEST (or M, H, C), then a tile
or a creature on the board; Enter locks in, right-click or Escape cancels.

Review pictures, without touching the screen: `--render-board <png>` draws the board with no
window (`--turns`, `--into`, `--scale`); `--snap <png> --review picking|orders|playing --at <s>`
opens the window behind everything, pictures it and closes. Look at them after changing anything
visual.

The gate, run locally (the repository is private, so nothing runs on GitHub):

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

The developer toolkit in Desktop runs Battle's tests against a Desktop checkout's own crates:
`devtools/formiga validate --expansions --step battle`.

## Crates

- **`formiga-battle-core`**: the rules, with no drawing, clock or network. `arena` (boards as
  data, in `arenas/`), `order`, `game` (a match, and a turn tick by tick), `profile` (a creature
  as Desktop records it), `interpret` (how a creature carries out its order, and why), `ai` (the
  fallback for a missing order, and the AI coach), `recap`, `replay` and `tournament`.
- **`formiga-battle-contract`**: what Desktop hands Battle, a read-only roster built from
  Desktop's own trip projection (`formiga-travel`) plus the learned leanings a trip leaves at
  home, and Battle's acknowledgement, read and written through `formiga-expansion-rulebook` like
  every companion app's documents. It is Desktop's: this is a copy of it as Desktop has adopted
  it, kept here byte for byte until a Desktop release carries it, when Battle takes it by tag
  like the other Desktop crates.
- **`formiga-battle`**: the app. `host` (Desktop's session, or a rehearsal), `roster` (a
  contender read as a match profile), `table` (one sitting
  at the board without the window: picking, orders, locking, the turn being shown), `playback`
  (a resolved turn at any moment, from its events), `scene` and `iso` (the board painted with
  `formiga-art`'s tools at pixel scale), `look` (each creature's own frames), `app` (the egui
  window) and `cli` (the headless commands).

`formiga-core`, `formiga-art`, `formiga-travel` and `formiga-expansion-rulebook` come from Desktop
by one release tag (`v0.67.6`) in the root `Cargo.toml`, so their types agree with Desktop's. Desktop is
authoritative for the colony; Battle never writes a colony file.
