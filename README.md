# Stardrop

A calm falling game. You are an astronaut dropping through one screen of sky
at a time, steering left and right past clouds, crystals, bubbles and
gravity wells, towards a beacon at the bottom.

There is no timer pressing on you and nothing chasing you. Every level is
one screen, visible from the start, so a fall is something you can read
before you take it.

## Playing

| | |
|---|---|
| Left / Right, A / D, or a gamepad stick | steer, and the first press starts the fall |
| R | start the attempt again |
| Esc | pause, and go back from any screen |
| Enter / Space | choose |
| F1 | frame times |

Fourteen levels, each unlocking the next. Each offers three stars: reaching the beacon, collecting all its
stardust in one attempt, and landing inside the level's target time.

Progress and settings are saved to your OS data folder
(`~/.local/share/stardrop/` on Linux). The game never writes anywhere else.

## Building

Rust 1.95 or newer. On Linux you also need `libasound2-dev`, `libudev-dev`,
`libwayland-dev` and `libxkbcommon-dev`:

```
cargo run --release
```

The shipped build carries its assets inside the executable, so the binary is
the whole game — copy it anywhere and it runs. `scripts/package.sh` builds
that version.

For development, `cargo run --features dev` keeps assets on disk and reloads
levels when they change. `./scripts/check.sh` is the quality gate: formatting,
clippy, tests and a release build. Add `--smoke` to start the real game for a
few seconds and fail if it panics, or `--visual` to check that the objects and
the astronaut still look like themselves. Every push runs the same gate on
Linux, Windows and macOS (`.github/workflows/check.yml`).

## Making levels

Levels are small text files in `assets/levels/`, and a level file can only
place things, never change how they behave. `docs/LEVELS.md` is the format,
the rules the game checks, and the rules the tests check.

A level dropped into an `assets/levels/` folder beside the binary is picked
up without rebuilding.

## How it is put together

| | |
|---|---|
| `STARDROP.md` | the brief: what this game is, and the standards it holds to |
| `DECISIONS.md` | every decision that cost thought, and what it was measured against |
| `TECH_DEBT.md` | known shortcuts, and when they get dealt with |
| `docs/LEVELS.md` | the level format |
| `CREDITS.md` | every asset and its licence |
| `docs/THIRD-PARTY.md` | every crate compiled in, and its licence |

The gameplay rules are plain Rust with no engine in them (`physics/`,
`objects/`, `collision.rs`, `scoring.rs`), which is why they can be tested
directly — and why `solver.rs` can play the whole game headlessly to prove
every level is beatable.

Every sound and every shape is generated in code. Nothing here is a
recording or a picture.
