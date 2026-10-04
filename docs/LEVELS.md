# Making levels

Levels are data, not code. Each level is one file in `assets/levels/`, named `<something>.level.ron`. Adding or editing a level never requires recompiling.

- **Live editing:** run the game with `cargo run --features dev`. Saving a level file then reloads it while the game runs. Each save starts a fresh attempt, so you immediately see the edited level from the start. A broken file shows an error message on screen instead of crashing, and the previous version stays until you fix it.
- **Checking:** `cargo test` loads and validates every level in `assets/levels/`.

The format is [RON](https://github.com/ron-rs/ron) ("Rusty Object Notation"). It looks like Rust struct syntax: `( field: value, ... )`, `[ list, ... ]`, `// comments`, and trailing commas are fine.

## Coordinates

Every level is exactly **one portrait screen, 1080 wide and 1920 tall**, and the whole level is always visible. Positions are written as `(x, depth)`:

- **`x`** runs from the left edge: `0` is the left wall, `540` the center, `1080` the right wall.
- **`depth`** runs downward from the top: `0` is the top edge, `1920` the bottom edge.

For scale, the astronaut is a circle with a radius of 28 units, so the level is about 19 astronauts wide. At full fall speed it covers 450 units per second: a straight drop from top to bottom takes about 5 s. Falling out of the bottom ends the attempt.

## Annotated example

```ron
// Comments start with // and are ignored.
(
    // Shown when the level starts.
    name: "Test Fall",

    // Palette and scenery. One of: DeepSpace, SpaceClouds, UpperAtmosphere,
    // CrystalAurora, HighClouds, CloudSea, Sunset, WarmEvening, Twilight,
    // Homecoming. (All look like DeepSpace until Milestone 7.)
    theme: DeepSpace,

    // Where the astronaut floats while the player studies the level. The
    // first left/right press starts the fall from here.
    start: (540, 200),

    // The landing pad: its center and its (width, height). The whole pad must
    // be inside the level and below the start. Reaching it completes the level.
    beacon: (center: (540, 1880), size: (300, 40)),

    // Stardust motes to collect. Place them on risky or clever routes.
    // 3–6 per level (Milestone 6), at most 64. Collecting all of them in one
    // attempt earns a star. Optional.
    stardust: [
        (260, 700),
        (820, 1100),
        (360, 1500),
    ],

    // Objects: what the astronaut touches (see "Objects" below). Optional.
    objects: [
        StaticCloud(center: (330, 1250), radius: 95),
    ],

    // Seconds for the "fast run" star: a good attempt, not a perfect one.
    // The clock starts when the fall starts, not while studying the level.
    target_time: 8.0,
)
```

## Objects

Each object is written as `Type(fields...)`. An effect happens when the astronaut *starts* touching an object; staying inside doesn't repeat it. Objects have **placement fields only**: where they are and how big. How strongly an object acts is set once per type in `src/config.rs` and is the same in every level (the consistency rule). Sizes must be 20–300 (ribbon widths 30–240).

| Object | Fields | What it does (designer notes; players are never told) |
|---|---|---|
| `CloudPuff` | `center: (x, depth)`, `radius` | Bounces the astronaut upward and away from its center, always at least 45° up, at the same speed however it's touched. The astronaut then coasts. |
| `WindRibbon` | `path: [(x, depth), ...]` (2–16 points, in flow order), `width` (30–240) | While inside, pulls the astronaut along the flow of the nearest segment. On leaving (at the end, or out the side) the astronaut keeps that speed and coasts. Chevrons show the flow direction. Every path point must be inside the level. |
| `TwinPortals` | `a: (x, depth)`, `b: (x, depth)`, `radius` | Entering either end sends the astronaut out of the other, keeping speed and direction and any active effects. Portals are ignored for 0.5 s afterwards, so you can't bounce between them. Written as one entry, so a portal can never be unpaired. The two ends must be at least three radii apart. |
| `GravityOrb` | `center: (x, depth)`, `radius` (the core) | Pulls the astronaut toward its center inside a field four times the core radius (the field is drawn faintly). Good for orbits and slingshots. Speed inside is capped, and touching the core pushes you softly back out. |
| `Bubble` | `center: (x, depth)`, `radius` | Catches the astronaut and floats it upward for 2.5 s (gentle sideways nudging still works), then pops and the fall resumes from rest. While held, nothing else affects the astronaut except hazards. **Each bubble works once per attempt** and comes back on a retry or failure. |
| `SporeFlower` | `center: (x, depth)`, `radius` | Swaps left and right for 4 seconds; a shimmer around the astronaut shows it's active. Touching another flower resets the 4 seconds instead of adding to them. |
| `CometStone` | `center: (x, depth)`, `radius` | Throws the astronaut into a dive far faster than normal (2.4×) for 0.6 s, then it eases back. Sideways steering still works. A fast shortcut, dangerous near hazards. |
| `Crystal` | `center: (x, depth)`, `size` (center to tip) | Reverses sideways motion, with a minimum sideways speed so it's always noticeable. The fall speed is kept. Falling straight onto it pushes away from its center. |
| `StaticCloud` | `center: (x, depth)`, `radius` | Hazard: touching it ends the attempt. Its hitbox is 80 % of its visible size, so near misses are safe. It can't cover the start, the beacon or any stardust. |

More object types arrive in Milestones 3 and 4.

## Rules the game checks

A level that breaks any of these rules is rejected, and the message says what's wrong:

- The file must be valid RON with exactly these fields. **Unknown fields are errors.** This is deliberate: a level can only *place* things and can never change how anything *behaves* (see the consistency rule in `STARDROP.md` §3.3). All behavior tuning lives in `src/config.rs`.
- `name` isn't empty.
- `target_time` is more than 0.
- All numbers are ordinary numbers: no `NaN` and no `inf`.
- The start, the whole beacon pad, every checkpoint and every stardust mote are inside the level.
- The beacon is below the start, and its width and height are positive.
- There are at most 64 stardust motes.
- Objects are inside the level, sized 20–300, and never overlap the start (the astronaut would begin inside them).
- Static clouds don't cover the beacon or any stardust.


## Themes

`theme` picks the altitude a level falls through: its sky colours, its background layers and how much its lights glow. The ten themes run from `DeepSpace` down to `Homecoming`, and each one's art is a block of constants in `src/theme.rs` — change a colour there and every level using that theme changes with it. A level file can only *pick* a theme, never describe one, for the same reason it can't change how objects behave.

A theme also picks the level's musical key: the ten altitudes descend from A down to a low D, so the game falls in sound as well as in colour (`src/music.rs`).

## Rules the tests check

These aren't part of the file format; the solver checks them for every shipped level (`src/solver.rs`):

- Every level is beatable, and all of its stardust is reachable in one attempt.
- `target_time` is no tighter than the fastest route × 1.4.
- Levels 6 to 10 carry at least six objects each, and something dangerous guards the last third, so the approach to the beacon is never a free glide.
- From the third level on, letting go must miss: a straight drop with no steering never lands on the beacon. The first two levels teach the controls, so there a straight drop may land.
