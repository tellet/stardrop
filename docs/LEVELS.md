# Making levels

Levels are data, not code. Each level is one file in `assets/levels/`, named `<something>.level.ron`. Adding or editing a level never requires recompiling.

- **Live editing:** run the game with `cargo run --features dev`. Saving a level file then reloads it while the game runs. A broken file shows an error message on screen instead of crashing, and the previous version keeps playing until you fix it.
- **Checking:** `cargo test` loads and validates every level in `assets/levels/`.

The format is [RON](https://github.com/ron-rs/ron) ("Rusty Object Notation"). It looks like Rust struct syntax: `( field: value, ... )`, `[ list, ... ]`, `// comments`, and trailing commas are fine.

## Coordinates

Positions are written as `(x, depth)`:

- **`x`** runs from the level's left edge: `0` is the left edge, `960` the center, `1920` the right edge. Every level is exactly 1920 wide and the whole width is always on screen.
- **`depth`** runs downward from the top of the level: `0` is the top, `height` is the bottom.

For scale, one screen is 1920 × 1080 units. The astronaut is a circle with a radius of 28 units. At full fall speed it covers 450 units per second (about 2.4 s per screen height).

## Annotated example

```ron
// Comments start with // and are ignored.
(
    // Shown when the level starts.
    name: "Test Fall",

    // Total height in units. At least 1080 (one screen). Levels should be
    // about 5–12 screens (5400–12960), getting longer later in the game.
    height: 10800,

    // Palette and scenery. One of: DeepSpace, SpaceClouds, UpperAtmosphere,
    // CrystalAurora, HighClouds, CloudSea, Sunset, WarmEvening, Twilight,
    // Homecoming. (All look like DeepSpace until Milestone 7.)
    theme: DeepSpace,

    // Where the astronaut starts falling, at rest.
    start: (960, 300),

    // The landing pad: its center and its (width, height). The whole pad must
    // be inside the level and below the start. Reaching it completes the level.
    beacon: (center: (960, 10770), size: (360, 40)),

    // Checkpoint rings, listed top to bottom, each between the start and the
    // beacon. Falling past a ring's depth activates it, wherever you are
    // sideways. Missing the beacon returns you to the ring's position.
    // Optional: leave the list out for no checkpoints.
    checkpoints: [
        (960, 3600),
        (700, 7200),
    ],

    // Stardust motes to collect. Place them on risky or clever routes.
    // 3–6 per level (Milestone 6), at most 64. Collected motes stay collected
    // through checkpoint respawns and come back on a retry (R). Optional.
    stardust: [
        (400, 1800),
        (1500, 4200),
    ],

    // Seconds for the "fast run" star: a good run, not a perfect one.
    target_time: 30.0,
)
```

## Rules the game checks

A level that breaks any of these rules is rejected, and the message says what's wrong:

- The file must be valid RON with exactly these fields. **Unknown fields are errors.** This is deliberate: a level can only *place* things and can never change how anything *behaves* (see the consistency rule in `STARDROP.md` §3.3). All behavior tuning lives in `src/config.rs`.
- `name` isn't empty.
- `height` is at least 1080.
- `target_time` is more than 0.
- All numbers are ordinary numbers: no `NaN` and no `inf`.
- The start, the whole beacon pad, every checkpoint and every stardust mote are inside the level.
- The beacon is below the start, and its width and height are positive.
- Checkpoints are between the start and the beacon, listed top to bottom.
- There are at most 64 stardust motes.

Objects (clouds, crystals, ...) get their own fields in Milestones 3–4, and this document will be extended then.
