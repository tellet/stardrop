# Stardrop

A calm falling game. You are an astronaut dropping through one screen of sky
at a time, steering left and right past clouds, crystals, bubbles and
gravity wells, towards a beacon at the bottom. Nothing chases you and no
timer presses on you.

## Playing

| | |
|---|---|
| Left / Right, A / D, or a gamepad stick | steer, and the first press starts the fall |
| R | start the attempt again |
| Esc | pause, and go back from any screen |
| Enter / Space | choose |
| F1 | frame times |

Fourteen levels, each unlocking the next. Each offers three stars: reaching
the beacon, collecting all its stardust in one attempt, and touching the
number of objects the level's phrase asks for.

Progress and settings are saved to your OS data folder
(`~/.local/share/stardrop/` on Linux). The game never writes anywhere else.

## Building

Rust 1.95 or newer. On Linux you also need `libasound2-dev`, `libudev-dev`,
`libwayland-dev` and `libxkbcommon-dev`:

```
cargo run --release
```

`scripts/package.sh` builds the version to hand to someone: one executable
with the assets inside, which runs from an empty folder. CI builds the same
thing for Linux, Windows and macOS when you tick `binaries` on a manual run
of the `check` workflow.

`cargo run --features dev` keeps assets on disk and reloads levels when they
change. `./scripts/check.sh` is the quality gate: formatting, clippy, tests
and a release build. Add `--smoke` to start the real game for a few seconds
and fail if it panics, or `--visual` to check that the objects and the
astronaut still look like themselves.

## Making levels

Levels are small text files in `assets/levels/`, and a level file can only
place things, never change how they behave. `docs/LEVELS.md` is the format.
A level dropped into an `assets/levels/` folder beside the binary is picked
up without rebuilding.

## How it is put together

`STARDROP.md` is the brief: what this game is and the standards it holds to.
`DECISIONS.md` records every decision that cost thought and what it was
measured against.

The gameplay rules are plain Rust with no engine in them (`physics/`,
`objects/`, `collision.rs`, `scoring.rs`), which is why they can be tested
directly, and why `solver.rs` can play the whole game headlessly to prove
every level is beatable.

Every sound and every shape is generated in code. Nothing here is a
recording or a picture.

Licences, the third-party crate list and the known-shortcuts log live on the
`paperwork` branch, which is this project with all of that still in place.
