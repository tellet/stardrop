# Stardrop

A calm falling game. You are an astronaut dropping through one screen of sky
at a time, steering left and right past clouds, crystals, bubbles and gravity
wells, towards a beacon at the bottom. Nothing chases you and no timer presses
on you.

The whole level is visible before you move, so every attempt is a guess about
what the objects will do. Every type always behaves the same way, and the game
never explains any of them. You learn by touching things, failing gently, and
trying again. Fourteen levels, each unlocking the next, each with three stars
to earn.

Every sound and every shape is generated in code. Nothing here is a recording
or a picture.

## Building

Rust 1.95 or newer. On Linux, four system packages as well:

```
sudo apt install libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev
cargo run --release
```

## In a browser

```
./scripts/web.sh
python3 -m http.server --directory web 8080
```

The same game, compiled to WebAssembly into `web/` and served as static
files. Keyboard only, and progress lives in the browser's own storage instead
of a folder. Tagging a version publishes it to GitHub Pages.

## Licence

MIT, in [LICENSE](LICENSE). The one asset that came from anywhere else is the
UI font, Bevy's built-in Fira Mono subset, under the SIL Open Font License
1.1. Every sound and every shape is generated at runtime, so nothing else is
shipped. The crates compiled into a release build are listed with their
licences in `docs/THIRD-PARTY.md` on the `paperwork` branch.

## The rest

| | |
|---|---|
| [docs/PLAYING.md](docs/PLAYING.md) | the controls, the stars, where progress is saved, how to run a build someone sent you |
| [docs/DEVELOPING.md](docs/DEVELOPING.md) | the quality gate, hot reload, packaging, releasing |
| [docs/DESIGN.md](docs/DESIGN.md) | the brief this was built from, and every decision that cost thought |
| [docs/LEVELS.md](docs/LEVELS.md) | the level file format |
