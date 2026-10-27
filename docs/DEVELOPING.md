# Developing

## The quality gate

```
./scripts/check.sh
```

Formatting, clippy with warnings as errors, clippy again under the `dev`
feature, the tests, and a release build. Add `--smoke` to start the real game
for a few seconds and fail if it panics or comes up without sound, or
`--visual` to check that the objects and the astronaut still look like
themselves.

CI runs the same gate on every push (`.github/workflows/check.yml`), plus the
performance measurements the brief sets budgets for.

## While working

```
cargo run --features dev
```

Keeps the assets on disk instead of inside the executable, so editing a level
file reloads it in the running game. F2 draws the collision shapes, and
F3 plays the landing cartoon from any screen, with a fresh planet each
press, so it can be watched without finishing every level.

## Making levels

Levels are small RON files in `assets/levels/`, and a level file can only
place things, never change how they behave. [LEVELS.md](LEVELS.md) is the
format. A level dropped into an `assets/levels/` folder beside the binary is
picked up without rebuilding.

## The browser version

```
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.128
./scripts/web.sh
python3 -m http.server --directory web 8080
```

`scripts/web.sh` builds the release wasm, runs `wasm-bindgen` over it into
`web/` and prints both sizes: 38.0 MB of module, 8.9 MB over the wire. The
`wasm-bindgen-cli` version has to match the `wasm-bindgen` crate the build
links against, or the glue and the module disagree about their interface.
`web/index.html` is the only source file in that folder; the other two are
built and git-ignored.

The browser build differs in four ways, all of them in the code:

- **The page owns the canvas's size** and the game follows it. Window sizing
  is desktop-only (`window_size` in `src/settings.rs`), and the UI scales to
  whatever size the page gives (`ui_fit` in `src/ui.rs`).
- **Progress goes into the browser's storage**, through the same three
  operations as a file (`files` in `src/save.rs`).
- **No gamepad.** `gilrs` has no web backend, so `bevy_gilrs` is a
  desktop-only dependency.
- **Sound waits for a key.** A browser hands out a silent audio device until
  the player has touched the page, and `web/index.html` wakes it on the first
  key.

The build script rewrites `$HOME` out of the module. A wasm module keeps the
file path of every panic site as plain data, which `strip` does not touch, so
an unremapped build publishes the home directory of whoever built it.

Clippy does not cover this build by default. Check it with:

```
cargo clippy --target wasm32-unknown-unknown
```

## Publishing to the browser

Tagging a version also deploys the browser build to GitHub Pages
(`.github/workflows/pages.yml`), or run that workflow by hand. It compiles
the module on the runner and hands `web/` straight to Pages, so no build
output is ever committed. One switch has to be thrown by hand, once:
Settings, Pages, Build and deployment, Source: GitHub Actions.

Pages on a free account only serves a public repository. Making this
repository private takes the site down with it.

## Packaging

```
./scripts/package.sh
```

Builds the `dist` profile: fat LTO, stripped, assets baked in, so the result
is one executable that runs from an empty folder. `scripts/macos-app.sh`
wraps that binary in the smallest `.app` macOS accepts, and only runs on a
Mac. CI builds all three desktops when you tick `binaries` on a manual run of
the `check` workflow.

## Releasing

Tag a version and CI builds all three desktops and publishes them:

```
git tag v0.1.0 && git push origin v0.1.0
```

The tag has to match `version` in `Cargo.toml`, and all three builds have to
pass their own tests, or nothing is published. The tests and the shipped
build run as separate jobs: on Windows they are about 53 and 36 minutes, and
together they overrun the 90 a job is given. A release asset downloads
without a GitHub account and never expires, unlike the artifacts from a
`check` run. `.github/workflows/release.yml` does it, and its release notes
carry the per-OS first-run steps from
[PLAYING.md](PLAYING.md#running-a-build-someone-sent-you).

## The paperwork branch

The third-party crate list, the known-shortcuts log and the audit
and oldest-supported-rust CI jobs live on the `paperwork` branch, which is
this project with all of that still in place. Picking any of it up means
merging a branch, not redoing the work. Note that the branch still carries
the old dual MIT or Apache-2.0 licence, where master is MIT alone.
