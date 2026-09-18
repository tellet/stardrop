# Stardrop — Project Brief & Roadmap

> This file is the single source of truth for this project. Read it fully before writing any code, and re-read the relevant milestone before starting work on it.

---

## 1. Who I am and how to work with me

- I'm **new to Rust** and new to making games. This project is how I keep learning both.
- I'm building the game together with you (Claude Code). You write most of the code; I review, test and steer.
- **Teaching matters as much as the game.** The first time a Rust concept appears in a milestone (ownership, borrowing, `&` vs `&mut`, `Option`, `Result`, `match`, enums with data, traits, lifetimes, modules, derive macros, closures, iterators, generics), explain it briefly in plain language, using the code you just wrote as the example.
- When the compiler throws an error, show me the error, explain what it means and why Rust complains, and only then fix it.

### Working agreement

1. Work on **one milestone at a time**, in order. For big milestones, work one bullet at a time.
2. Before starting a milestone, give me a short plan (files to create or change, approach) and wait for my OK.
3. After finishing, tell me exactly how to test it against the milestone's **Done when** check.
4. Keep the project compiling and runnable after every step. Never leave it broken.
5. Suggest a git commit message at the end of each completed step.
6. Don't add features from later milestones early. Do, however, avoid choices that would block later milestones.
7. If something in this brief is unclear or seems like a bad idea, say so and ask rather than guess.
8. Keep dependencies minimal. Before adding any crate, tell me what it's for and confirm it's compatible with our Bevy version.
9. **Performance, efficient resource usage and code quality are my top priorities.** Follow Section 7 at all times, and flag any trade-off before making it.

---

## 2. Game vision

**Stardrop** is a calm, beautiful single-player falling game. A small astronaut drifts down from the edge of space toward home, a glowing landing beacon on the planet's surface. The astronaut is always falling and can't slow down or go up; the only way to change course is by touching the objects floating in the sky.

**The heart of the game is discovery.** Every type of object always behaves the same way, but different types behave very differently, and not all of them are helpful. The game never explains what an object does. The player learns by touching things, failing gently, and trying again, and each new discovery is recorded in an illustrated field journal.

- **Players:** single player.
- **Length:** 10 hand-made levels, each a few minutes at most. Each level is a stage of the descent, so the scenery changes from deep space to the planet's surface over the course of the game.
- **Controls:** left/right (A/D or arrow keys, or a gamepad stick) for gentle sideways drift. R to retry, Esc to pause, J to open the journal. Nothing else.
- **Tone:** peaceful, wondering, a little playful. Failure is soft and instant, never punishing. No violence, no death: when things go wrong, the astronaut simply fades back to the last checkpoint.
- **Feel:** looks like a dreamy illustrated sky; sounds like calm ambient music that the player composes by falling.

---

## 3. Core rules

### 3.1 Movement
- The astronaut is always falling under constant gravity, up to a **terminal velocity** (a maximum fall speed), so the game stays readable and fair.
- **Air control:** left/right input applies gentle horizontal acceleration, with a maximum horizontal speed and soft deceleration when no input is given. The player can position themselves, but can never slow their fall or rise on their own.
- Only objects can make big changes to the astronaut's path.
- **Level edges:** soft invisible walls on the left and right; the astronaut stops at them with a small cushioned bump.
- Physics runs at a fixed timestep and is fully deterministic: the same inputs always produce the same fall.

### 3.2 Goal, failure and checkpoints
- **Goal:** land on the glowing **beacon** at the bottom of the level.
- **Missing the beacon** (falling past the bottom of the level) or touching a hazard sends the astronaut back to the last **checkpoint**: a soft fade out and fade in, under one second, with the fall resuming from rest.
- **Checkpoints** are gentle glowing rings placed through the level; passing through one activates it with a soft chime. Every level starts at an implicit checkpoint at the top.
- **Retry (R)** restarts the whole level instantly.
- There are no lives and no game over. The player can always keep trying.

### 3.3 The consistency rule
**Every object of the same type behaves identically, in every level.** This is what makes discovery fair and learnable, and it's enforced in code:
- Behavior strength (bounce speed, pull force, durations) is defined **once per object type** in the global config, never per level or per instance.
- Level files may only set an instance's **position, size, orientation, path** (for wind ribbons) and **pairing** (for portals). Nothing that changes what the object does.
- Object behaviors are implemented as pure functions of the type's config and the astronaut's state, and are unit-tested.

### 3.4 The objects
Nine object types. The descriptions below are for us; the player is never told any of this.

| Object | Behavior | Nature |
|---|---|---|
| **Cloud puff** | On contact, bounces the astronaut upward and away from the cloud's center. The cloud squishes and wobbles. | Helpful |
| **Wind ribbon** | A glowing stream along a curved path. While inside it, the astronaut is pushed along the ribbon's direction, and released at its end. | Helpful, if you read it right |
| **Bubble** | Captures the astronaut and floats slowly upward for a few seconds (slight sideways nudging still works), then pops and the fall resumes from rest. | Mixed: saves speed, costs time, can drift you off course |
| **Crystal** | Reverses the astronaut's horizontal direction (with a minimum sideways speed so the effect is always noticeable). | Mixed |
| **Gravity orb** | Pulls the astronaut toward its center within a radius, allowing orbits and slingshots. Touching the core gives a soft bounce. | Powerful, hard to master |
| **Comet stone** | Launches the astronaut downward far beyond normal terminal velocity for a short time. | Mixed: fast shortcuts, dangerous near hazards |
| **Spore flower** | Releases a puff of spores that swaps left/right controls for a few seconds. A visible shimmer around the astronaut shows the effect is active. | Mischievous |
| **Twin portals** | Entering one portal exits from its paired portal, keeping speed and direction. Brief cooldown so you can't bounce between them. | Mixed, level-dependent |
| **Static cloud** | A dark, softly crackling cloud. Touching it sends the astronaut back to the last checkpoint. | Hazard |

- Each object type has a **distinct silhouette, color and idle animation**, so types are recognizable at a glance once learned, and all instances of a type look alike.
- Every contact triggers a visual reaction on the object (squish, ripple, flash) so the player always sees *what* they touched.

### 3.5 Collectibles & scoring
- Each level has a few **stardust motes** to collect, placed along risky or clever routes.
- Up to **3 stars** per level: 1 for reaching the beacon, 1 for collecting all stardust in one run, 1 for beating the level's target time.
- Completing a level unlocks the next. Best time, stars and stardust are saved.

### 3.6 The field journal
- The first time the astronaut touches an object type, a small "new entry" notice appears, and the journal gains an illustrated page for it with a short note in the astronaut's voice, written as a guess based on what they just saw (e.g. *"Bubbles: slow, cozy, never in a hurry."*).
- Untouched object types appear in the journal as silhouettes with "???".
- The journal is available from the pause menu and the level select (J key), and is saved with progress.

---

## 4. Levels

Ten levels, each introducing **one** new object type using the **introduce → test → twist** pattern:
1. **Introduce** it where touching it is safe and its effect is easy to see.
2. **Test** it: a section where using it on purpose is the best way forward.
3. **Twist** it: combine it with earlier objects in a surprising way.

Each level is a stage of the descent, and its scenery reflects the altitude.

| # | Name | New object | Setting |
|---|---|---|---|
| 1 | **First Light** | — (learn drifting, checkpoints, stardust, the beacon) | Deep space, starfield and distant nebula |
| 2 | **Soft Landing** | Cloud puff | Deep space, glowing space clouds |
| 3 | **Solar Breeze** | Wind ribbon | Upper atmosphere, aurora curtains |
| 4 | **Glass Garden** | Crystal | Aurora, floating crystal fields |
| 5 | **Storm Front** | Static cloud | High clouds, distant lightning glow |
| 6 | **Slow Bloom** | Bubble | Above the cloud sea, pastel dawn |
| 7 | **Heavy Heart** | Comet stone | Sunset layer, falling meteor streaks |
| 8 | **Pollen Drift** | Spore flower | Warm evening sky, floating blossoms |
| 9 | **Orbit** | Gravity orb | Twilight, first stars reappearing, planet lights below |
| 10 | **Homecoming** | Twin portals (+ everything) | Night-to-dawn over the landscape, the home beacon |

- Levels are about 5–12 screen-heights tall, becoming longer as the game progresses.
- Level 1 has no objects at all, just the fall, the controls and the beacon, so the player understands the goal before the first discovery.
- A static cloud must never be the first object type the player sees in a level where it's new; it's introduced alongside a clear safe path around it.

### Level format — levels are data, not code
- Each level is a **RON** file in `assets/levels/` describing: name, height, background theme, astronaut start, beacon position and size, checkpoints, stardust, object instances (type plus allowed per-instance fields only, see Section 3.3), target time, music layer set.
- Adding or editing a level never requires recompiling. In debug builds, editing a level file **hot-reloads** it while the game runs.
- Level loading **validates** the data and returns clear errors (a custom error type): e.g. overlapping objects that shouldn't overlap, an unpaired portal, a beacon outside the level, a checkpoint inside a static cloud. An invalid level shows an error message in-game, never a crash.
- `cargo test` loads and validates every shipped level.
- The format is documented with an annotated example in `docs/LEVELS.md`.

---

## 5. Art direction — soft, glowing, dreamlike

The game should look like a calm illustrated storybook sky that happens to be interactive.

- **Smooth, not pixel art:** soft rounded shapes, clean silhouettes, gentle gradients. Rendered at the window's native resolution.
- **Layered depth:** 3–5 parallax background layers per level (distant stars or sky gradient, far clouds or nebula, mid-distance features, near drifting particles), plus an optional foreground layer of soft out-of-focus shapes.
- **Glow and light:** HDR rendering with **bloom** for glowing things (the beacon, stardust, wind ribbons, orbs, checkpoints), with bloom strength tuned per level. Colored soft light on the astronaut as they pass near glowing objects, if affordable.
- **Palette per altitude:** each level has a small defined palette (defined in code as constants per theme) so the game moves through a deliberate color journey: indigo and violet in space, green and teal auroras, pastel dawn, warm gold and coral sunset, deep blue twilight, a soft dawn at home.
- **Motion everywhere, but calm:** objects breathe, bob and sway; clouds drift; stars twinkle; nothing is ever completely still, but nothing is jittery either. All animation uses easing curves, never linear snaps.
- **Astronaut:** small, round, readable; a slight tumble that follows sideways movement; a little flash of the visor when touching objects; a trail of faint sparkles showing the recent path.
- **Feedback:** every object contact gets a visual reaction (squish, ripple, flash, particle burst in the object's color).
- **Camera:** smooth vertical follow with **look-ahead**: the astronaut sits in the upper third of the screen so the player can see what's coming below. The full level width is always visible; the camera only scrolls vertically.
- **Aspect ratio:** designed for 16:9; other ratios show extra background at the sides, never cut off gameplay.
- **Art assets:** start with simple shapes and gradients drawn in code as placeholders. Final art can be sprites (drawn by me, or from free CC0 sources) or shapes and shaders generated in code. Keep a `CREDITS.md` listing every asset and its license.
- **Accessibility:** a "reduce motion" setting that calms background animation and screen effects; every object type must be distinguishable by shape, not color alone.

---

## 6. Audio direction — the fall is the music

The sound should be comforting, smooth and warm at all times, and it also quietly teaches the player what objects do.

### 6.1 Musical system
- A soft **ambient pad** plays under every level, with a slow-changing chord progression, all in a single key per level (e.g. a major pentatonic scale, so any combination of notes sounds harmonious).
- **Every object type is an instrument**, and touching an object plays a note from the level's current scale:
  - Cloud puff: soft marimba.
  - Wind ribbon: breathy flute swell, lasting while inside the ribbon.
  - Bubble: gentle glass chime on capture, soft "pop" plink on release.
  - Crystal: bright but soft bell.
  - Gravity orb: warm low hum that slowly rises in pitch while orbiting.
  - Comet stone: deep, soft whoosh with a low bass note.
  - Spore flower: airy, slightly detuned music-box twinkle.
  - Twin portals: a swelling reverse-cymbal shimmer into a soft chord.
  - Static cloud: a muffled, low, *sympathetic* tone, never harsh or startling.
- **Helpful objects** play notes that sound resolved and bright; **tricky and harmful ones** play softer, suspended or minor-ish intervals (still within the key), so players subconsciously learn which objects are friends.
- Consecutive object touches step up the scale, so a good run literally sounds like a rising melody.
- Optionally, notes are gently aligned to the pad's pulse (a tiny delay of at most a few tens of milliseconds) so they always feel musical. Evaluate whether this feels good; skip it if it feels laggy.
- Reaching the beacon plays a warm resolving chord that completes the melody.
- The music gains subtle layers as the player descends through a level (e.g. soft strings joining halfway), and each level's music reflects its setting.

### 6.2 Sound design rules
- **Nothing sharp or piercing.** Soft attacks (a few milliseconds of fade-in on every sound), natural fade-outs, high frequencies gently rolled off.
- **Pitch variation** on repeated non-melodic sounds (about ±5%) so nothing sounds mechanical.
- **Ambience** per altitude: faint cosmic hum in space, soft wind at altitude, distant birds and breeze near the surface.
- **Checkpoint:** a soft chime. **Stardust:** tiny twinkles, each one a step higher. **Return to checkpoint:** a gentle reverse swell, never a "failure" buzz.
- **UI:** soft tuned notes in the menu's key; the journal pages turn with a gentle paper whisper.
- **Mixing:** everything is balanced so nothing ever clips; the pad ducks slightly under object notes; music always crossfades (1–2 s), never cuts.
- **While paused:** music and ambience are softly low-pass filtered and lowered.

### 6.3 Tech notes
- Choose an audio approach that supports scheduling, volume fades, crossfades, pitch control and filters. Evaluate `bevy_kira_audio` and `bevy_seedling` (or others) and recommend one, checking compatibility with our Bevy version first.
- Musical notes may be pre-recorded samples per instrument (pitched per note, or one sample per note), or synthesized; recommend the approach with the best balance of sound quality, memory use and simplicity.
- The scale and note-selection logic is plain Rust with unit tests (e.g. "the 5th consecutive touch in D major pentatonic plays the right note").
- Sounds live in `assets/sounds/` and `assets/music/`, in `.ogg` format. Sources: CC0 libraries, freesound.org (checking each license) or sounds I make. All listed in `CREDITS.md`.

---

## 7. Engineering standards — performance, resource usage, code quality

**These are my top priorities**, above the number of features and above speed of delivery. A smaller game that runs lean and has clean code is better than a bigger one that doesn't. When a choice trades one of these off against another (or against a feature), stop and explain the trade-off to me before deciding. Explain *why* each practice matters when you first apply it.

### 7.1 Performance budgets
- **Frame rate:** a stable 60 FPS on modest hardware (a laptop with integrated graphics) with bloom and all effects on, including the busiest levels. Support higher refresh rates smoothly where the display allows.
- **Frame time:** our own game logic (all our systems combined) stays under **1 ms** per frame in release builds.
- **No hitches:** respawning at a checkpoint, retrying, loading a level and discovering a new object must not cause visible stutters.
- **Level load and retry:** under **200 ms**.
- **Idle efficiency:** in menus, the journal, while paused and on the level-complete screen, the game must not burn CPU/GPU at full speed. Use Bevy's reactive/low-power update modes where little is animating, and vsync/frame limiting in-game. Laptop fans should stay quiet.
- **Memory:** measure a baseline in Milestone 0 and track it at each milestone. Memory must stay flat over a long session: no growth across retries, checkpoints and level changes (no leaked entities, asset handles, textures or audio instances).
- **Startup:** reach the main menu within a couple of seconds on the target hardware.
- **Budgets are measured, not guessed.** Numbers get reported at the end of any milestone that touches hot paths.

### 7.2 Resource-efficiency rules
- **Trim Bevy's features.** Disable Bevy's default features and enable only what a 2D game with HDR/bloom needs (no 3D/PBR, glTF, etc.). Document the chosen feature list in `Cargo.toml` with a comment per feature.
- **Release profile:** configure `[profile.release]` for a small, fast binary (LTO, a single codegen unit, stripped symbols, `panic` strategy chosen deliberately), and explain each setting to me.
- **No per-frame allocations in hot paths.** Systems that run every frame must not create new `Vec`s, `String`s or boxes. Reuse buffers stored in resources or `Local<T>`; only rebuild UI text when the underlying value changes.
- **Do less work, less often.**
  - Bevy **run conditions** so systems only run in the states where they matter.
  - **Change detection** (`Changed<T>`, `Added<T>`, `resource_changed`) instead of rechecking everything every frame.
  - Query filters (`With`, `Without`) so queries touch only relevant entities.
  - Event-driven effects: audio, visuals and the journal react to events, never poll.
- **Cheap collision:** objects use simple shapes (circles, capsules, rectangles, and path segments for wind ribbons). Only objects within a band around the astronaut's current height are checked (objects stored sorted by height, or a simple spatial grid; benchmark and choose).
- **Off-screen work is skipped:** objects far outside the camera view don't animate or emit particles.
- **Fixed timestep for physics:** gameplay physics runs in `FixedUpdate`, independent of frame rate; rendering interpolates the astronaut's position so motion stays perfectly smooth on any refresh rate.
- **Pooled, capped effects:** particles, trails and short-lived effects come from pools with hard maximum counts.
- **Bloom and effects are budgeted:** measure their GPU cost; provide a "graphics quality" setting (e.g. bloom and particle density) for weaker machines.
- **Textures:** sensible sizes, texture atlases where many small sprites are used, backgrounds sized for their on-screen use (not huge images scaled down).
- **Audio:** short sounds preloaded, `.ogg` compressed; music and ambience streamed; a cap on simultaneous sound instances.
- **Assets:** loaded once, handles kept in resources; level-specific assets unloaded when leaving that level.

### 7.3 Measuring
- **F1 debug overlay:** FPS, frame time (current / average / worst over the last few seconds), entity count, particle count, active sound count, memory usage where available, the astronaut's velocity, and active effects (e.g. inverted controls timer).
- **F2 (debug builds only):** show collision shapes and wind ribbon paths.
- **F3 (debug builds only):** jump to the next checkpoint.
- **Profiling support** set up early (Bevy's tracing integration with a profiler such as Tracy, behind a cargo feature so it costs nothing when off), and show me how to use it.
- **Benchmarks** (e.g. with `criterion`) for the hottest pure functions: collision checks, object effect functions, the broad-phase lookup. Report before/after numbers when that code changes.
- **Profile before optimizing.** Don't trade readability for speed without a measurement showing it matters; comment what was measured.

### 7.4 Code quality
- **Formatting & linting:** `cargo fmt` always; Clippy configured in `Cargo.toml` via `[lints]` with `clippy::all` as errors and a curated selection of `clippy::pedantic` lints (explain which ones and why). Warn on `clippy::unwrap_used` and `clippy::expect_used` outside tests.
- **No panics in game logic:** use `Option`/`Result`. `unwrap()`/`expect()` only where failure is truly impossible, with a comment explaining why. File and asset errors are handled gracefully.
- **Separate logic from the engine:** movement physics, object effects, collision math, scoring, scale/note selection, level validation and save logic live in **plain Rust with no Bevy dependency**. Bevy systems stay thin: read components, call the logic, write results back.
- **Objects as an enum with data:** object types are one enum (`ObjectKind`), and the effect of touching an object is one exhaustive `match`. Adding a new object type makes the compiler point to every place that must handle it.
- **Tests:**
  - Unit tests for every object effect (e.g. "a crystal always reverses horizontal velocity and enforces the minimum speed"), movement limits, checkpoint logic, star scoring, note selection and level validation (with deliberately broken level files).
  - A **determinism test**: replaying a recorded input sequence on a level always produces the same final position.
  - A test that loads and validates every shipped level.
  - A few integration tests with a minimal Bevy `App` for key flows (touching a new object adds a journal entry exactly once).
- **Strong types:** enums instead of boolean flags or magic strings; newtypes where units could be confused (world units vs screen pixels, seconds vs frames); invalid states unrepresentable where reasonable.
- **Small, focused units:** each system does one job; short functions; split modules that grow past a few hundred lines.
- **Minimal visibility:** private by default, `pub(crate)` when needed, `pub` rarely.
- **Documentation:** `//!` module docs everywhere and `///` docs on public items. Comments explain *why*, not *what*.
- **No leftovers:** no commented-out code, no unused imports, no stray prints (use Bevy's logging macros).
- **Dependency discipline:** every crate needs a stated reason and a compatibility check; prefer the standard library and Bevy's own features.
- **Decisions log:** `DECISIONS.md` (what, why, alternatives) and `TECH_DEBT.md` (known shortcuts).
- **Quality check command:** one command (a cargo alias or small script) running `fmt --check`, `clippy`, `test` and a release build.

### 7.5 Self-review at the end of every milestone
Before declaring a milestone done, review your own code from the milestone and briefly report on:
1. Performance: per-frame allocations, unnecessary work, systems running outside their state, unbounded growth.
2. Resource usage: entity counts and memory before and after 20 retries of a level and a full playthrough of all available levels; leaks; asset handling.
3. Quality: lint status, test coverage of new logic, naming, module boundaries, anything for `TECH_DEBT.md`.
4. Measurements: frame time and relevant benchmark numbers compared with the previous milestone.

Fix what's cheap to fix right away; list the rest for me to decide.

---

## 8. Tech stack & architecture

- **Language:** Rust (stable), using Cargo.
- **Engine:** Bevy **0.19** (pin the exact version in `Cargo.toml`; don't mix in syntax from older Bevy versions). 2D camera with HDR and bloom.
- **Physics:** custom and simple, no physics engine: the astronaut is a single circle, objects are simple shapes, and every effect is a small pure function.
- **Dev build speed:** Bevy's fast-compile settings for development (dynamic linking feature, optimized dependencies in dev profile).

### Code structure
- One Bevy **plugin per feature area**, each in its own module:
  `astronaut`, `physics`, `objects`, `collision`, `level`, `checkpoint`, `collectibles`, `scoring`, `journal`, `camera`, `background`, `effects`, `music`, `audio`, `ui`, `menu`, `settings`, `save`, `config`, `debug`.
- **Top-level app state** as an enum: `MainMenu`, `LevelSelect`, `Journal`, `Playing`, `Paused`, `LevelComplete`.
- **In-level sub-state** as an enum: `Intro` (short fly-in of the level name), `Falling`, `Respawning`, `Landed`.
- **All tuning numbers in one place:** a `GameConfig` resource in `config.rs` holding gravity, terminal velocity, air control, and every object type's behavior parameters. This is also where the consistency rule (Section 3.3) lives.
- **Events** for everything that happens between systems: `ObjectTouched { kind, entity }`, `ObjectDiscovered(kind)`, `CheckpointReached`, `Respawn`, `StardustCollected`, `LevelCompleted`. Music, sound, visual effects and the journal react to these events rather than being called from gameplay code.

---

## 9. Roadmap

Milestones 0–4 produce a **playable prototype** with placeholder visuals. Milestones 5–9 complete the game.

### Milestone 0 — Project setup
- Cargo project with Bevy 0.19, fast-compile dev settings, trimmed Bevy features, configured release profile.
- Window titled "Stardrop", 2D camera with HDR and bloom enabled, a soft vertical gradient background.
- Folder structure: `src/` modules, `assets/levels`, `assets/sprites`, `assets/sounds`, `assets/music`, `assets/fonts`, `docs/`.
- `config.rs` with an empty `GameConfig`; `.gitignore`; `CREDITS.md`, `DECISIONS.md`, `TECH_DEBT.md`.
- Lints configured; the quality check command set up; test and benchmark scaffolding (one trivial test and benchmark that run).
- Minimal F1 overlay with FPS and frame time; profiling feature flag wired up; low-power mode when nothing is animating.
- **Baseline measurements recorded** in `DECISIONS.md`: release binary size, memory at the empty screen, idle CPU usage, clean-build and incremental-build times.

**Done when:** `cargo run` opens a window with a smooth gradient and a softly glowing test circle; the quality check command passes; baseline numbers are recorded.
**Rust concepts:** Cargo, crates, features, profiles, modules, `main`, `const`, structs, `derive`, attributes, `#[cfg(test)]`.

### Milestone 1 — The fall
- The astronaut (placeholder: a glowing circle) falls under gravity up to terminal velocity, in `FixedUpdate`, with interpolated rendering.
- Gentle left/right air control with acceleration, max speed and soft deceleration; keyboard and gamepad.
- Soft invisible side walls.
- Camera follows vertically with look-ahead; full level width always visible; correct behavior at other aspect ratios.
- A long test level with simple placeholder background layers (parallax) to feel the speed of the fall.
- Unit tests for movement: terminal velocity is never exceeded, horizontal speed is capped, no input means gradual slow-down.

**Done when:** falling and drifting feels smooth, calm and responsive at 60 Hz and at higher refresh rates, and the determinism test (replaying recorded inputs gives the same result) passes.
**Rust concepts:** components, systems, queries, `Res` vs `ResMut`, `Query<&T>` vs `Query<&mut T>`, `Vec2` math, fixed timestep.

### Milestone 2 — Levels, goal and checkpoints
- Level RON format (Section 4), loading, validation with a custom error type, hot-reload in debug builds.
- Beacon at the bottom: landing on it completes the level (placeholder "Level complete" text).
- Checkpoints: activate when passed; missing the beacon returns the astronaut to the last active checkpoint with a fade (under one second).
- R retries the whole level instantly (under 200 ms).
- Stardust collectibles; run tracking (time, stardust collected).
- `docs/LEVELS.md` with an annotated example.

**Done when:** a test level can be completed or failed, checkpoints work, retry is instant, editing the level file updates the running game, and broken level files show a clear error instead of crashing.
**Rust concepts:** `serde` deserialization, custom error types, the `?` operator, `Result` flowing through functions, `Option` for "no checkpoint yet".

### Milestone 3 — Objects, part 1
- The object system: `ObjectKind` enum, per-type behavior config, collision shapes, broad-phase lookup by height, `ObjectTouched` events, visual reaction on contact (placeholder squish/flash).
- Enforcement of the consistency rule (Section 3.3) in the level format and validation.
- Object types: **cloud puff, wind ribbon, crystal, static cloud**.
- Each type gets a distinct placeholder shape and color.
- Unit tests for every effect; benchmarks for collision and effect functions.

**Done when:** a test level with all four types plays correctly; every instance of a type behaves identically; the F2 overlay shows collision shapes and ribbon paths.
**Rust concepts:** enums with data, exhaustive `match`, traits (e.g. for shapes), generics where they genuinely help, events.

### Milestone 4 — Objects, part 2
- Object types: **bubble, comet stone, spore flower, gravity orb, twin portals**.
- Timed effects (bubble capture, comet boost, inverted controls) modeled cleanly as timed state on the astronaut, with only one effect of each kind active at a time and clear rules for how they combine (document them in `DECISIONS.md`).
- Portal pairing validated at level load; exit cooldown.
- Gravity orb force clamped so orbits feel good and never produce extreme speeds.

**Done when:** a test level with all nine types plays correctly, effects combine predictably, and all effect tests pass. **This is the playable prototype.**
**Rust concepts:** state modeled with enums and structs, timers, borrowing multiple components safely, refactoring with the compiler's help.

### Milestone 5 — Game flow, journal and saving
- Main menu, level select (10 slots, stars shown, locked levels dimmed), pause menu, level-complete screen (time, stardust, stars, personal best; Next / Retry / Level select).
- Level intro: the level's name gently fades in over the first seconds of the fall.
- Star scoring and unlocking.
- **Field journal:** discovery on first touch (with a small "new entry" notice), entries with placeholder illustrations and the astronaut's notes, "???" silhouettes for undiscovered types.
- Save file: unlocked levels, best times, stars, stardust, discovered journal entries. Missing or corrupted save files never crash the game or wipe progress silently (keep a backup of the previous save).
- Settings: master/music/SFX/ambience volume, fullscreen, graphics quality, reduce motion. Saved and loaded at startup.

**Done when:** the whole game loop works from main menu to level select to playing to level complete, progress and journal persist across restarts, and settings apply immediately.
**Rust concepts:** `serde` serialization, file I/O with `Result`, state machines across screens, `impl Default`.

### Milestone 6 — The ten levels
- Build all 10 levels from Section 4, following **introduce → test → twist** for each new object.
- Each level has 3–6 stardust motes, sensible checkpoint spacing and a target time based on a good (not perfect) run.
- Difficulty rises gradually; later levels are longer and combine more object types.
- Playtest each level and adjust placement (not object behavior, see Section 3.3).

**Done when:** all 10 levels can be completed; a first-time player can figure out every object without being told; no level has an unfair moment (a hazard you couldn't see or react to in time).

### Milestone 7 — Visual polish
- Final astronaut: tumble following sideways motion, visor flash on contact, sparkle trail.
- Final look for all nine object types: distinct silhouettes, idle animations, contact reactions, particle bursts in each object's color.
- Backgrounds for every altitude theme (Section 5): 3–5 parallax layers, per-level palettes, bloom tuning, ambient particles.
- Beacon, checkpoints and stardust with glow and gentle animation; a landing celebration (soft light burst, drifting sparks).
- Smooth transitions between screens (fades, eased motion); journal illustrations.
- Graphics quality and reduce-motion settings take effect everywhere.

**Done when:** every level looks like a calm illustrated sky worth screenshotting, and performance budgets still hold on the target hardware at the highest quality setting.

### Milestone 8 — Music and sound
Implement everything from Section 6:
- Audio crate integrated with separate channels for music, instruments (object notes), SFX, ambience and UI.
- Ambient pad and per-level music with layers joining as the player descends; crossfades between screens and levels.
- Object instruments with scale-based note selection, rising sequences on consecutive touches, helpful-vs-tricky tonal differences.
- Beacon resolution chord, checkpoint chime, stardust twinkles, gentle respawn swell.
- Ambience per altitude; UI sounds; pause filtering; volume settings respected.

**Done when:** a good run through any level sounds like a small piece of music, nothing ever sounds harsh or clips, and the game is pleasant to listen to for a whole playthrough.

### Milestone 9 — Final pass
- Full playthrough of all 10 levels checking pacing, difficulty and the journal notes' wording.
- Final performance and memory measurements against Section 7.1, recorded in `DECISIONS.md`.
- Everything in `TECH_DEBT.md` either fixed or consciously accepted.
- Release build packaged with its assets, plus a short `README.md` (how to play, how to build, how to make levels).

**Done when:** the game is complete, polished and within every budget, and I'd be happy to send it to a friend.

### Milestone 10 — Stretch goals (optional, in any order)
- **Ghost replay:** race a translucent recording of your best run (the deterministic physics makes this cheap: record inputs, not positions).
- **Time-trial leaderboard** per level (local).
- **Level editor:** place objects, checkpoints and the beacon in-game, test-play, save.
- **Bonus levels** unlocked by collecting all stardust.
- **Alternate characters** (the ninja, the frog) with their own visual themes; identical physics.
- **Web (WASM) build** so friends can play in a browser.

---

## 10. Definition of done for every step

- The quality check command passes: formatted, **zero** compiler and Clippy warnings, all tests pass, release build succeeds.
- New logic has unit tests; hot-path code has benchmarks where relevant.
- No per-frame allocations or unbounded growth introduced; new systems use run conditions and change detection where applicable.
- Performance budgets from Section 7.1 still hold (measured in a release build).
- New modules and public items are documented.
- New Rust concepts used in the step have been explained to me.
- Any new asset is in `CREDITS.md` with its license.
- At the end of a milestone: the self-review from Section 7.5 has been done and reported, and `DECISIONS.md` / `TECH_DEBT.md` are up to date.

**Start with Milestone 0.** Give me your plan first.
