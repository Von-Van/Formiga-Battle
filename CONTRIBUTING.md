# Contributing and making your own version

Formiga is a personal project rather than an openly contributed community project. I am
not currently accepting pull requests, feature submissions, or requests to maintain changes in this
repository. Unsolicited contributions may be closed without review.

You are warmly encouraged to use the code as a starting point for your own experiments, arenas,
and ways for creatures to play. Fork it, remix it, improve it, or take it in a completely different
direction under the terms of the [MIT License](LICENSE).

If you publish a version of your own, please:

- Preserve the license and copyright notice.
- Give the project a distinct name and icon so people do not mistake it for an official Formiga
  release.
- Clearly describe it as an independent fork or derivative.
- Point friend matches at a relay of your own (below), not at the one on formigaworld.com.

## Working on your version

### Setting up

- Install [rustup](https://rustup.rs). The pinned toolchain, Rust 1.97.1 with rustfmt and Clippy,
  installs itself from `rust-toolchain.toml` the first time you run `cargo`.
- On macOS 14 or later, install the Xcode Command Line Tools (`xcode-select --install`). On Windows
  10 or 11, install the Visual Studio C++ build tools. Those are the platforms Battle is released
  for.
- The first build fetches Formiga Desktop's shared crates from GitHub, by the release tag named in
  the root `Cargo.toml`. To try Battle against a Desktop checkout of your own instead, put a
  `[patch]` in `.cargo/config.toml`, which git ignores.

Battle never writes a colony, so it is safe to run against anything. The one file it keeps is its
own `settings.json` (volumes, mute, motion and window size); set `FORMIGA_BATTLE_DATA_DIR` to keep
it somewhere else while you work:

```sh
cargo run --release -p formiga-battle -- --sample                  # the sample colony
cargo run --release -p formiga-battle -- --from-save <colony.json> # a colony, read and never written
```

Friend matches go to the relay on formigaworld.com unless `FORMIGA_BATTLE_RELAY` says otherwise,
and that relay only lets released copies of a released version play. While you work, run a local
relay and point Battle at it:

```sh
cargo run --release -p formiga-battle -- relay   # listens on 127.0.0.1:7878
FORMIGA_BATTLE_RELAY=http://127.0.0.1:7878/api/battle cargo run --release -p formiga-battle
```

Two windows started that way, from the same build, can play each other.

### Finding your way around

The [Crates](README.md#crates) section of the README says what each crate and module is for. The
short version:

- `formiga-battle-core`: the rules, with no drawing, clock or network. Every match is
  worked out the same on every machine from its arena, rules, seed, rosters and orders.
- `formiga-battle-room`: friend matches, the relay as a state machine (the reference the site's
  relay follows), and the HTTP client Battle uses.
- `formiga-battle`: the window and the headless commands (`cargo run -p formiga-battle -- --help`).
- `formiga-battle-contract`, `formiga-core`, `formiga-art`, `formiga-travel` and
  `formiga-expansion-rulebook` are Formiga Desktop's, taken from its release tag. Changes to them,
  the roster contract included, belong in Desktop.

### Checks

Battle has no GitHub Actions, so the gate runs on your own machine. Run it before sharing a build:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Changing things safely

- **Rules.** Anything that changes how a turn resolves needs `RULES_VERSION` moved in
  `crates/formiga-battle-core/src/ruleset.rs`, so a friend on another version is told rather than
  quietly playing different rules. Run `tournament --pairings 5000` before and after, and update
  [docs/RULES.md](docs/RULES.md) with what it says.
- **The relay.** A change to what is sent or answered moves `RELAY_VERSION`, and
  [docs/RELAY.md](docs/RELAY.md) changes with it. The relay on the site has to change in step. It
  also hands out a copy of `Ruleset::default()`, which each window checks against its own, so a
  change to the default ruleset needs the site's copy changed too, or every friend match is
  refused.
- **What is shared.** [docs/FRIENDS.md](docs/FRIENDS.md) and the notice the window shows before a
  friend match say exactly what leaves the computer. If that changes, both change with it.
- **Arenas.** A new one is an `.arena` file in `arenas/`, a line in `BUILT_IN` in
  `crates/formiga-battle-core/src/arena.rs`, a branch of `Dressing` in
  `crates/formiga-battle/src/garden.rs` to dress it as a place, and a cup in `rewards::CUPS`.
- **Anything visual.** Look at the pictures rather than trusting the numbers:
  `--render-board <png>` draws the board with no window, and `--render-window <png> --review
  picking|orders|playing` pictures the whole window without opening one (`--snap <png>` opens it
  to take the same picture). Each takes `--arena`.
- **Releases.** Battle's version follows Formiga Desktop's. `scripts/set-version.sh X.Y.Z vX.Y.Z`
  moves it and Desktop's tag everywhere they appear, and `scripts/set-version.sh --check` says
  whether they agree. Add what changed under the coming version's `## [X.Y.Z] - date` section in
  [CHANGELOG.md](CHANGELOG.md); the release scripts look for that heading.

Formiga intentionally avoids accounts, telemetry and anything that identifies a computer, and a
friend match keeps nothing once it is over. Forks are encouraged to preserve those defaults and to
explain clearly if they choose a different model.
