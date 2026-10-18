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
pass their own tests, or nothing is published. A release asset downloads
without a GitHub account and never expires, unlike the artifacts from a
`check` run. `.github/workflows/release.yml` does it, and its release notes
carry the per-OS first-run steps from
[PLAYING.md](PLAYING.md#running-a-build-someone-sent-you).

## The paperwork branch

Licences, the third-party crate list, the known-shortcuts log and the audit
and oldest-supported-rust CI jobs live on the `paperwork` branch, which is
this project with all of that still in place. Going public later means
merging a branch, not redoing the work.
