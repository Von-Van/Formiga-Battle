# Formiga Battle

A separate desktop app for Formiga Desktop's companions: isometric, simultaneous-order 3v3
tactics, against the AI or a friend in a private room. You are the Handler: each turn you give
each of your three Formigas one order (MOVE, HOLD or CONTEST), both teams' orders lock together,
and the turn plays out at once. How each creature carries out its order comes from who it is in
Desktop, never from stats, so knowing your Formigas is the skill and rerolling them for a better
trio is not.

This checkout is the rules sandbox: the whole match runs headless and deterministically, from the
command line. There is no window yet, no online play, and Desktop does not open Battle yet.

- [docs/RULES.md](docs/RULES.md): how a match resolves, every provisional number, and the
  fairness tournament's results.
- [docs/PERSONALITY.md](docs/PERSONALITY.md): every part of Desktop's record of a companion, and
  what Battle does with it.

## Commands

```bash
cargo run --release -p formiga-battle -- --help
cargo run --release -p formiga-battle -- roster                          # the sample colony's companions
cargo run --release -p formiga-battle -- roster --from-save <colony.json> # read, never written
cargo run --release -p formiga-battle -- match --seed 4 --board          # an AI match, told turn by turn
cargo run --release -p formiga-battle -- match --from-save <colony.json> --left 0,2,3 --record m.json
cargo run --release -p formiga-battle -- replay m.json                   # replays and checks every turn
cargo run --release -p formiga-battle -- tournament --pairings 5000      # fairness by kind and trio
cargo run --release -p formiga-battle -- arena                           # draws Backyard
```

The gate, run locally (the repository is private, so nothing runs on GitHub):

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

## Crates

- **`formiga-battle-core`**: the rules, with no drawing, clock or network. `arena` (boards as
  data, in `arenas/`), `order`, `game` (a match, and a turn tick by tick), `profile` (a creature
  as Desktop records it), `interpret` (how a creature carries out its order, and why), `ai` (the
  fallback for a missing order, and the AI coach), `recap`, `replay` and `tournament`.
- **`formiga-battle-contract`**: what Desktop hands Battle, a read-only roster built from
  Desktop's own trip projection (`formiga-travel`) plus the learned leanings a trip leaves at
  home, read and written through `formiga-expansion-rulebook` like every companion app's
  documents. A draft kept here until Desktop adopts it.
- **`formiga-battle`**: the command line.

`formiga-core`, `formiga-travel` and `formiga-expansion-rulebook` come from Desktop by one release
tag (`v0.67.6`) in the root `Cargo.toml`, so their types agree with Desktop's. Desktop is
authoritative for the colony; Battle never writes a colony file.
