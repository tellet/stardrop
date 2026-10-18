# Stardrop — design

Two parts. **Part 1** is the brief: what this game is and the standards it
holds to, written before any code and still the source of truth. **Part 2** is
the decision log: every choice that cost thought, what it was measured
against, and what was considered instead.

The brief is kept as it was written, so a few of its details are now history:
it asks for ten levels and the game has fourteen, and it plans for sprite and
sound files where everything ended up generated in code. Where the game went
somewhere else, Part 2 says so and why. Section 11 is the exception: it was
written after the game was finished, for the one thing added since.
`CREDITS.md`, `TECH_DEBT.md` and the licences it mentions live on the
`paperwork` branch.

# Part 1 — the brief

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

**The heart of the game is discovery, played like a quiz.** The whole level is visible before the fall, so every attempt is a guess about what the objects will do. Every type of object always behaves the same way, but different types behave very differently, and not all of them are helpful. The game never explains what an object does. The player learns by touching things, failing gently, and trying again, and each new discovery is recorded in an illustrated field journal.

- **Players:** single player.
- **Length:** 10 hand-made levels. Each level is **one portrait screen, fully visible from the start**: a small puzzle to read, plan and try, where one attempt lasts only seconds. Each level is a stage of the descent, so the scenery changes from deep space to the planet's surface over the course of the game.
- **Controls:** left/right (A/D or arrow keys, or a gamepad stick) for gentle sideways drift. R to retry, Esc to pause, J to open the journal. Nothing else. The first left/right press of an attempt starts the fall.
- **Tone:** peaceful, wondering, a little playful. Failure is soft and instant, never punishing. No violence, no death: when things go wrong, the astronaut simply fades back to the start for a fresh attempt.
- **Feel:** looks like a dreamy illustrated sky; sounds like calm ambient music that the player composes by falling.

---

## 3. Core rules

### 3.1 Movement
- The astronaut is always falling under constant gravity, up to a **terminal velocity** (a maximum fall speed), so the game stays readable and fair.
- **Air control:** left/right input applies gentle horizontal acceleration, with a maximum horizontal speed and soft deceleration when no input is given. The player can position themselves, but can never slow their fall or rise on their own.
- Only objects can make big changes to the astronaut's path.
- **Before the fall:** each attempt starts with the astronaut floating at the start with a gentle bob, for as long as the player wants to study the level. The first left/right press starts the fall, and the attempt's timer.
- **Level edges:** invisible walls on the left and right; the astronaut bounces off them gently and coasts away, like in space (see the decision log M1-5).
- Physics runs at a fixed timestep and is fully deterministic: the same inputs always produce the same fall.

### 3.2 Goal and failure
- **Goal:** land on the glowing **beacon** at the bottom of the screen.
- **Missing the beacon** (falling out of the bottom of the screen) or touching a hazard ends the attempt: a soft fade out and in, under one second, back to the start for a fresh attempt (stardust and time reset).
- **Retry (R)** restarts the attempt instantly.
- There are no checkpoints (an attempt lasts only seconds), no lives and no game over. The player can always keep trying.

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
| **Static cloud** | A dark, softly crackling cloud. Touching it ends the attempt (soft fade back to the start). | Hazard |

- Each object type has a **distinct silhouette, color and idle animation**, so types are recognizable at a glance once learned, and all instances of a type look alike.
- Every contact triggers a visual reaction on the object (squish, ripple, flash) so the player always sees *what* they touched.

### 3.5 Collectibles & scoring
- Each level has a few **stardust motes** to collect, placed along risky or clever routes.
- Up to **3 stars** per level: 1 for reaching the beacon, 1 for collecting all stardust in one attempt, 1 for **finishing the level's phrase** — touching the number of objects the level asks for, in that one attempt.
- The phrase star is the scoring half of the musical system (Section 6.1): every object touched plays the next note of the level's scale, so finishing a phrase is something the player hears rather than something the interface has to explain. It also gives the player a reason to go *through* a level rather than around it.
- Completing a level unlocks the next. Best time, stars and stardust are saved. Time is still shown on the level-complete screen; it just isn't what a star is for, because a stopwatch and a calm fall pull in opposite directions.

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
| 1 | **First Light** | — (learn drifting, stardust, the beacon) | Deep space, starfield and distant nebula |
| 2 | **Soft Landing** | Cloud puff | Deep space, glowing space clouds |
| 3 | **Solar Breeze** | Wind ribbon | Upper atmosphere, aurora curtains |
| 4 | **Glass Garden** | Crystal | Aurora, floating crystal fields |
| 5 | **Storm Front** | Static cloud | High clouds, distant lightning glow |
| 6 | **Slow Bloom** | Bubble | Above the cloud sea, pastel dawn |
| 7 | **Heavy Heart** | Comet stone | Sunset layer, falling meteor streaks |
| 8 | **Pollen Drift** | Spore flower | Warm evening sky, floating blossoms |
| 9 | **Orbit** | Gravity orb | Twilight, first stars reappearing, planet lights below |
| 10 | **Homecoming** | Twin portals (+ everything) | Night-to-dawn over the landscape, the home beacon |

- Every level is exactly **one portrait screen (1080 × 1920 units)**, fully visible from the start. Later levels become denser and more intricate, not longer.
- Level 1 has no objects at all, just the fall, the controls and the beacon, so the player understands the goal before the first discovery.
- A static cloud must never be the first object type the player sees in a level where it's new; it's introduced alongside a clear safe path around it.

### Level format — levels are data, not code
- Each level is a **RON** file in `assets/levels/` describing: name, background theme, astronaut start, beacon position and size, stardust, object instances (type plus allowed per-instance fields only, see Section 3.3), target time, music layer set.
- Adding or editing a level never requires recompiling. In debug builds, editing a level file **hot-reloads** it while the game runs.
- Level loading **validates** the data and returns clear errors (a custom error type): e.g. overlapping objects that shouldn't overlap, an unpaired portal, a beacon outside the level, stardust inside a static cloud. An invalid level shows an error message in-game, never a crash.
- `cargo test` loads and validates every shipped level.
- The format is documented with an annotated example in `docs/LEVELS.md`.

---

## 5. Art direction — soft, glowing, dreamlike

The game should look like a calm illustrated storybook sky that happens to be interactive.

- **Smooth, not pixel art:** soft rounded shapes, clean silhouettes, gentle gradients. Rendered at the window's native resolution.
- **Layered depth:** 3–5 background layers per level (distant stars or sky gradient, far clouds or nebula, mid-distance features, near drifting particles) that drift gently on their own (the camera doesn't move), plus an optional foreground layer of soft out-of-focus shapes. The scenery also fills the areas beside the portrait playfield.
- **Glow and light:** HDR rendering with **bloom** for glowing things (the beacon, stardust, wind ribbons, orbs), with bloom strength tuned per level. Colored soft light on the astronaut as they pass near glowing objects, if affordable.
- **Palette per altitude:** each level has a small defined palette (defined in code as constants per theme) so the game moves through a deliberate color journey: indigo and violet in space, green and teal auroras, pastel dawn, warm gold and coral sunset, deep blue twilight, a soft dawn at home.
- **Motion everywhere, but calm:** objects breathe, bob and sway; clouds drift; stars twinkle; nothing is ever completely still, but nothing is jittery either. All animation uses easing curves, never linear snaps.
- **Astronaut:** small, round, readable; a slight tumble that follows sideways movement; a little flash of the visor when touching objects; a trail of faint sparkles showing the recent path.
- **Feedback:** every object contact gets a visual reaction (squish, ripple, flash, particle burst in the object's color).
- **Camera:** fixed. The whole portrait playfield is always visible and centered on the screen.
- **Aspect ratio:** the playfield is 9:16 portrait, centered on screen. Wider (or taller) windows show more scenery around it and never cut off gameplay.
- **Art assets:** start with simple shapes and gradients drawn in code as placeholders. Final art can be sprites (drawn by me, or from free CC0 sources) or shapes and shaders generated in code. Keep a `CREDITS.md` listing every asset and its license.
- **Accessibility:** a "reduce motion" setting that calms background animation and screen effects; every object type must be distinguishable by shape, not color alone.

---

## 6. Audio direction — the fall is the music

The sound should be comforting, smooth and warm at all times, and it also quietly teaches the player what objects do.

### 6.1 Musical system
- A soft **ambient pad** plays under every level, with a slow-changing chord progression, all in a single key per level: a **minor pentatonic** scale, so any combination of notes sounds harmonious *and* the game sounds like empty space rather than a sunny afternoon.
- The music is built on **open fourths, fifths and octaves**. Thirds are avoided: they are what make a chord sound warm and domestic.
- **Everything heard in a level rings on.** Every note is followed by a long tail of reflections (a second or more), because the sound of somewhere enormous is the sound of something taking a long time to fade. Menu and journal sounds stay close and dry.
- **Every object type is an instrument**, and touching an object plays a note from the level's current scale:
  - Cloud puff: softly struck metal, two tines a hair apart so the note beats against itself.
  - Wind ribbon: breathy flute swell, lasting while inside the ribbon.
  - Bubble: gentle glass chime on capture, soft "pop" plink on release.
  - Crystal: a glass bell heard from a long way off — almost no strike, all ringing.
  - Gravity orb: warm low hum that slowly rises in pitch while orbiting.
  - Comet stone: deep, soft whoosh with a low bass note.
  - Spore flower: a music box further out of tune than a music box should be, and without its sparkle.
  - Twin portals: a swelling reverse-cymbal shimmer into a soft chord.
  - Static cloud: a muffled, low, *sympathetic* tone, never harsh or startling.
- **Helpful objects** play the open notes of the scale (root, fourth, fifth); **tricky and harmful ones** play the notes that colour it (the flat third and flat seventh), still within the key, so players subconsciously learn which objects are friends.
- Consecutive object touches step up the scale, so a good run literally sounds like a rising melody.
- Optionally, notes are gently aligned to the pad's pulse (a tiny delay of at most a few tens of milliseconds) so they always feel musical. Evaluate whether this feels good; skip it if it feels laggy.
- Reaching the beacon plays an open chord — root, fifth, octave, no third — that completes the melody without making it cosy.
- The music gains subtle layers as the player descends through a level (e.g. soft strings joining as the astronaut passes the middle of the screen), and each level's music reflects its setting.

### 6.2 Sound design rules
- **Nothing sharp or piercing.** Soft attacks (a few milliseconds of fade-in on every sound), natural fade-outs, high frequencies gently rolled off.
- **Pitch variation** on repeated non-melodic sounds (about ±5%) so nothing sounds mechanical.
- **Ambience** per altitude: faint cosmic hum in space, soft wind at altitude, distant birds and breeze near the surface.
- **Stardust:** tiny twinkles, each one a step higher. **Failed attempt:** a gentle reverse swell, never a "failure" buzz.
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
- **No hitches:** restarting an attempt, retrying, loading a level and discovering a new object must not cause visible stutters.
- **Level load and retry:** under **200 ms**.
- **Idle efficiency:** in menus, the journal, while paused and on the level-complete screen, the game must not burn CPU/GPU at full speed. Use Bevy's reactive/low-power update modes where little is animating, and vsync/frame limiting in-game. Laptop fans should stay quiet.
- **Memory:** measure a baseline in Milestone 0 and track it at each milestone. Memory must stay flat over a long session: no growth across retries, attemptsints and level changes (no leaked entities, asset handles, textures or audio instances).
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
- **Cheap collision:** objects use simple shapes (circles, capsules, rectangles, and path segments for wind ribbons). A level is one screen with a limited number of objects: benchmark a plain loop against a simple spatial grid, and pick the grid only if it measurably wins.
- **Fixed timestep for physics:** gameplay physics runs in `FixedUpdate`, independent of frame rate; rendering interpolates the astronaut's position so motion stays perfectly smooth on any refresh rate.
- **Pooled, capped effects:** particles, trails and short-lived effects come from pools with hard maximum counts.
- **Bloom and effects are budgeted:** measure their GPU cost; provide a "graphics quality" setting (e.g. bloom and particle density) for weaker machines.
- **Textures:** sensible sizes, texture atlases where many small sprites are used, backgrounds sized for their on-screen use (not huge images scaled down).
- **Audio:** short sounds preloaded, `.ogg` compressed; music and ambience streamed; a cap on simultaneous sound instances.
- **Assets:** loaded once, handles kept in resources; level-specific assets unloaded when leaving that level.

### 7.3 Measuring
- **F1 debug overlay:** FPS, frame time (current / average / worst over the last few seconds), entity count, particle count, active sound count, memory usage where available, the astronaut's velocity, and active effects (e.g. inverted controls timer).
- **F2 (debug builds only):** show collision shapes and wind ribbon paths.
- **Profiling support** set up early (Bevy's tracing integration with a profiler such as Tracy, behind a cargo feature so it costs nothing when off), and show me how to use it.
- **Benchmarks** (e.g. with `criterion`) for the hottest pure functions: collision checks, object effect functions, the broad-phase lookup. Report before/after numbers when that code changes.
- **Profile before optimizing.** Don't trade readability for speed without a measurement showing it matters; comment what was measured.

### 7.4 Code quality
- **Formatting & linting:** `cargo fmt` always; Clippy configured in `Cargo.toml` via `[lints]` with `clippy::all` as errors and a curated selection of `clippy::pedantic` lints (explain which ones and why). Warn on `clippy::unwrap_used` and `clippy::expect_used` outside tests.
- **No panics in game logic:** use `Option`/`Result`. `unwrap()`/`expect()` only where failure is truly impossible, with a comment explaining why. File and asset errors are handled gracefully.
- **Separate logic from the engine:** movement physics, object effects, collision math, scoring, scale/note selection, level validation and save logic live in **plain Rust with no Bevy dependency**. Bevy systems stay thin: read components, call the logic, write results back.
- **Objects as an enum with data:** object types are one enum (`ObjectKind`), and the effect of touching an object is one exhaustive `match`. Adding a new object type makes the compiler point to every place that must handle it.
- **Tests:**
  - Unit tests for every object effect (e.g. "a crystal always reverses horizontal velocity and enforces the minimum speed"), movement limits, attempt logic, star scoring, note selection and level validation (with deliberately broken level files).
  - A **determinism test**: replaying a recorded input sequence on a level always produces the same final position.
  - A test that loads and validates every shipped level.
  - A few integration tests with a minimal Bevy `App` for key flows (touching a new object adds a journal entry exactly once).
- **Strong types:** enums instead of boolean flags or magic strings; newtypes where units could be confused (world units vs screen pixels, seconds vs frames); invalid states unrepresentable where reasonable.
- **Small, focused units:** each system does one job; short functions; split modules that grow past a few hundred lines.
- **Minimal visibility:** private by default, `pub(crate)` when needed, `pub` rarely.
- **Documentation:** `//!` module docs everywhere and `///` docs on public items. Comments explain *why*, not *what*.
- **No leftovers:** no commented-out code, no unused imports, no stray prints (use Bevy's logging macros).
- **Dependency discipline:** every crate needs a stated reason and a compatibility check; prefer the standard library and Bevy's own features.
- **Decisions log:** Part 2 of this document (what, why, alternatives) and `TECH_DEBT.md` (known shortcuts).
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
  `astronaut`, `physics`, `objects`, `collision`, `level`, `attempt`, `collectibles`, `scoring`, `journal`, `camera`, `background`, `effects`, `music`, `audio`, `ui`, `menu`, `settings`, `save`, `config`, `debug`.
- **Top-level app state** as an enum: `MainMenu`, `LevelSelect`, `Journal`, `Playing`, `Paused`, `LevelComplete`.
- **In-level sub-state** as an enum: `Ready` (floating at the start while the player studies the level; the level name fades in), `Falling`, `Restarting` (the soft fade after a failed attempt), `Landed`.
- **All tuning numbers in one place:** a `GameConfig` resource in `config.rs` holding gravity, terminal velocity, air control, and every object type's behavior parameters. This is also where the consistency rule (Section 3.3) lives.
- **Events** for everything that happens between systems: `ObjectTouched { kind, entity }`, `ObjectDiscovered(kind)`, `AttemptFailed`, `AttemptRestarted`, `StardustCollected`, `LevelCompleted`. Music, sound, visual effects and the journal react to these events rather than being called from gameplay code.

---

## 9. Roadmap

Milestones 0–9 are done: the fall, one-screen portrait levels, the nine
objects, the game flow with journal and saving, the fourteen levels, the
visual polish, the music, and a final pass against every budget. Part 2
records what each one actually built. Milestone 10 is polish with no end date.

Stretch goals, none of them promised:
- **Ghost replay:** race a translucent recording of your best run (the deterministic physics makes this cheap: record inputs, not positions).
- **Time-trial leaderboard** per level (local).
- **Level editor:** place objects, stardust and the beacon in-game, test-play, save.
- **Bonus levels** unlocked by collecting all stardust.
- **Alternate characters** (the ninja, the frog) with their own visual themes; identical physics.
- **Web (WASM) build** so friends can play in a browser.
- **The landing cartoon** for finishing every level. Done: Section 11 and M10-13.

---

## 10. Definition of done for every step

- The quality check command passes: formatted, **zero** compiler and Clippy warnings, all tests pass, release build succeeds.
- New logic has unit tests; hot-path code has benchmarks where relevant.
- No per-frame allocations or unbounded growth introduced; new systems use run conditions and change detection where applicable.
- Performance budgets from Section 7.1 still hold (measured in a release build).
- New modules and public items are documented.
- New Rust concepts used in the step have been explained to me.
- At the end of a milestone: the self-review from Section 7.5 has been done and reported, and Part 2 is up to date.

---

## 11. The landing cartoon

Added after Milestone 9, once the fourteen levels were done. A wordless
8.5-second scene, the only part of the game that is not a level: the astronaut
lands on an unknown planet, the dust settles, and they plant a flag.

### When it plays

- Landing the last level shows the usual "Landed" screen. Continuing from it
  goes to the cartoon rather than the level list, but only when every level in
  the order is completed.
- Entering the cartoon frees the level it came from, the way going back to a
  menu does: otherwise the beacon, every object and the astronaut who just
  landed all draw on top of the planet.
- The cartoon fades to the main menu when it ends.
- Any key, mouse button or gamepad button skips straight to that fade.
- A **The landing** row appears on the main menu once all fourteen levels are
  done, so it can be watched again. The journal stays a book of objects.
- Nothing is written on the screen at any point (Part 2, M10-10). The scene
  says it or it goes unsaid.

### The stage

The camera does not move. The playfield is fixed at x ∈ [-540, 540] and
y ∈ [0, -1920], and the whole scene is built inside it.

| | |
|---|---|
| Surface top | y = -1450: sky fills the upper three quarters |
| The astronaut enters at | y = +120, above the frame |
| Total length | 8.5 s |
| Sizes below | in astronaut radii, r = 28 world units |
| The pack and the pole | stand on the ground under each of them, which on a rocky planet is nowhere near the height under the astronaut's feet |

### The script

Every boundary is a named constant, and `beat_at(t) -> Beat` is a pure
function. Progress inside a beat is eased, never linear (§5).

| t | Beat | What happens |
|---|---|---|
| 0.00 | Descent | The astronaut drifts down from above the frame, leaning with `Tumble`, shedding a spark every 0.12 s. Props sway, steam vents breathe. |
| 2.20 | Touchdown | Dust bursts in the ground's own colour, the suit squashes and recovers, whatever grows near the landing bends away from it, vents puff hard. |
| 2.20–3.60 | Settle | The dust dies out. The astronaut straightens up and bobs once. |
| 3.60 | The pack | It slides off their back, 4.2 r to their left, and eases up to a 4.6 by 4.2 r body with a handle, two feet and an aerial on it: 6 r wide and 6.1 r tall all told, far larger than the astronaut. It thumps down with a ground puff. The size is the joke, and the parts are what make it read as a bag of stuff rather than a box. |
| 5.00 | The flag rises | A hatch opens and a flag lifts out: a 3.2 r pole, nearly white whatever the planet's colours are, with a 1.4 r cloth notched at the fly, bright against anything. Comically small against the pack, which still stands half again as tall and three times as wide. |
| 6.40 | Planting | They carry it across and push the pole into the ground 1.6 r to their right, where nothing stands behind it. A small puff, and the cloth unfurls and starts waving. |
| 7.40 | Hold | They stand beside it. The flag waves, one last sparkle. |
| 8.50 | — | The fade to the main menu. |

The cloth carries the same star the level-complete screen uses.

### The planet

`src/ending/planet.rs`: plain Rust using only `bevy::math`, so it is tested
without an `App`. One seed, one planet; the seed is fresh on every watch, so
rewatching gives a new world.

```rust
Planet { palette, ground: [f32; 96], spans: [GroundSpan; 2], props: Vec<Prop>, landing_x: f32 }
GroundKind = Rock | Desert | Water | Overgrowth
PropKind   = Mushroom | GrapeCluster | Flower | Boulder | SteamVent | Reed
```

- **Ground:** height samples 11.25 units apart across the generated width,
  which is 154 of them, from two octaves of value noise over `SplitMix64`
  (`src/rng.rs`). The ground is generated 1.6x the playfield's width so a
  window of any shape sees ground to both edges.

| Ground | Height | Shape |
|---|---|---|
| Rock | ±260 | Two octaves, the second at half weight: jagged ridges and canyons |
| Desert | ±120 | One long octave: smooth dunes |
| Overgrowth | ±90 | Soft mounds |
| Water | flat | A level surface 40 units below the ground it cuts into |

- **Spans:** one or two, meeting at a seeded x. Two spans are always different
  kinds, so a mixed planet reads as a mix.
- **Props:** drawn from a weight table per ground kind, so a planet's life
  belongs to its ground. Capped at 24.

| Ground | Mushroom | Grapes | Flower | Boulder | Steam vent | Reed |
|---|---|---|---|---|---|---|
| Rock | 0 | 0 | 1 | 6 | 3 | 0 |
| Desert | 0 | 0 | 1 | 3 | 1 | 2 |
| Overgrowth | 5 | 4 | 5 | 1 | 1 | 3 |
| Water's edge | 1 | 1 | 2 | 1 | 0 | 6 |

| Prop | Size |
|---|---|
| Mushroom | cap 1.4–4.0 r across, stem 2–6 r tall |
| Grape cluster | 7–13 blobs of 0.5–1.1 r, hanging |
| Flower | a 0.6–1.2 r head on a 1–2.5 r stem |
| Boulder | 0.8–2.5 r |
| Reed | 2–5 r tall, 0.1 r thick |
| Steam vent | a 1–2 r crack in the ground, an emitter and nothing else |

- **The landing spot:** the sample nearest the middle of the screen whose
  slope is under 0.15, with no prop within 3 r of it. Looked for within 324
  units of the middle first, then within 335, and if a planet of jagged rock
  offers nothing that gentle anywhere in reach, the flattest sample there is.
  The wider reach is what it is because the pack and the flag have to fit on
  screen beside the astronaut. Five planets in a thousand land on something
  steeper than 0.15 (91 of 20,000 seeds, worst 0.33), which beats landing
  off screen.
- **Palette:** three hues from the seed under constraints: the ground is at
  least 0.2 darker than the sky, saturation never above 0.7, and props sit at
  the complementary hue ±30°, so no planet is ugly and no two look alike.

### Drawing it

All geometry is built once on entering the state, with `MeshBuilder` and the
soft blobs of `src/soft_mesh.rs`. Colours are baked into the vertices, so every
prop shares one material. One mesh for the sky gradient, one for the ground,
one for water, one per prop, and a handful for the pack and the flag: under 50
entities, no per-frame allocation, and everything despawned on leaving the
state.

Dust, steam and sparkles all go through the existing `Burst` message
(`src/particles.rs`), which the quality setting already thins. `reduce_motion`
calms the sway and the dust; there is no screen shake to switch off.

### Sound

No new synth recipes. The cartoon writes one message per beat and `audio.rs`
plays the matching voice, the same way objects do.

| Beat | Voice |
|---|---|
| Touchdown | `Whoosh`, and the landing chord on `Bell` |
| The pack | `Muffled` |
| The flag rises | `Shimmer` |
| Planting | `Marimba`, then a `Bell` chord as it goes in |

The menu pad keeps playing underneath.

### Files and tests

New: `src/ending/mod.rs` (the plugin and the timeline), `planet.rs` (the
generator), `art.rs` (the meshes). Touched: `lib.rs`, `app_state.rs`
(the new state animates continuously), `menu.rs` (where continuing goes, and
the rewatch row), `audio.rs`.

Tests:

- The generator is deterministic: one seed, one planet.
- No prop sits on the landing spot, and every height stays inside the playfield.
- Every ground kind appears across many seeds, and the prop cap holds.
- The beats tile the timeline with no gap and none runs past the end.
- The astronaut's feet meet the ground height at the landing x at touchdown.
- An `App` test: entering and leaving the state leaks no entities.

# Part 2 — decisions

What we decided, why, and what we considered instead. Newest at the bottom.

---

## M0-1 · Bevy 0.19.1, default features off

- **What:** `bevy = "=0.19.1"`, `default-features = false`, and an explicit feature list in `Cargo.toml` (each feature has a comment). All versions are pinned exactly (`=`).
- **Why:** Bevy's defaults include 3D/PBR, glTF, audio, scenes, picking, animation and more. We don't use any of them, and they cost compile time, binary size and startup time. Pinning exactly means an update can never silently change engine behavior, so every upgrade is a deliberate decision.
- **Left out for now:**
  - `bevy_gilrs`/`gamepad`: gamepad support, added in M1.
  - `bevy_gizmos`: F2 collision shapes, added in M3.
  - `png`: sprites, added in M7.
  - `bevy_audio`: M8 chooses between kira and seedling.
  - `sysinfo_plugin`: memory in the overlay, added later.
- **Alternatives:** Bevy's `2d` feature collection. It is simpler, but it also pulls in scenes, picking, gizmos, audio and the gamepad crate.

## M0-2 · No Wayland backend (for now)

- **What:** the `wayland` feature is off; only `x11` is on.
- **Why:** building it needs the `libwayland-dev` and `libxkbcommon-dev` system packages. The dev machine runs X11, and Wayland desktops run X11 apps through XWayland.
- **Closed 2026-10-12.** The packages went on, and both backends are now built in: one binary picks whichever the desktop runs. Costs 1.4 MB of binary (48.2 -> 49.6 MB) and means anyone building this on Linux needs those two packages. Checked on X11, where winit still picks x11 and nothing changed. **Not checked on a Wayland session**: this machine doesn't run one.

## M0-3 · Tonemapping: TonyMcMapface

- **What:** the camera uses `Tonemapping::TonyMcMapface`. This needs the `tonemapping_luts`, `ktx2` and `zstd_rust` features.
- **Why:** with HDR and bloom, glowing colors go above 1.0. TonyMcMapface fades very bright colors gently toward white without shifting their hue, which suits the "soft, dreamy glow" direction. Bevy uses it in its own bloom examples.
- **Alternatives:**
  - `None` (Bevy's 2D default): bright colors clip harshly.
  - `AcesFitted`: needs no lookup table, but strongly shifts hues (blues turn magenta).
  - `AgX`: very neutral but slightly washed-out.
- **Cost:** a few hundred KB of embedded lookup table plus a pure-Rust zstd decoder.

## M0-4 · Camera: fixed design size, `ScalingMode::AutoMin`

- **What:** the camera always shows at least 1920×1080 world units, whatever the window size.
- **Why:** this is Section 5's rule, "16:9 design; other ratios show extra background, never cut off gameplay". World units stay independent of screen pixels.

## M0-5 · Low-power updates by default; continuous while F1 is open

- **What:** `WinitSettings` uses `reactive_low_power(1 s)`: the app only wakes on window events or once a second. Opening the F1 overlay switches it to `continuous` (still capped by vsync).
- **Why:** Section 7.1, idle efficiency. An empty screen should cost almost nothing. Frame-time numbers are only meaningful when the app renders continuously.
- **Later:** once screens exist, the app state will choose the update mode (see TECH_DEBT.md).
- **Update (M1):** the game now animates all the time, so it uses Bevy's `WinitSettings::game()` (continuous while focused) and the overlay no longer changes the mode. Low-power mode returns for menus in M5.

## M0-6 · Build profiles

| Setting | Why |
|---|---|
| `[profile.dev] opt-level = 1` | Our code gets light optimization but still compiles fast. |
| `[profile.dev.package."*"] opt-level = 3` | Bevy is fully optimized once and then cached. A debug build of Bevy is unplayably slow. |
| `--features dev` → `bevy/dynamic_linking` | Bevy becomes a shared library, so relinking after a change takes a fraction of the time. Development only; never ship it. |
| Linker: clang + mold (user's `~/.cargo/config`) | Much faster linking than GNU ld. |
| release `lto = "thin"`, dist `lto = "fat"` | Optimizes across crate boundaries: faster and smaller binary, but slower to build. Fat LTO is reserved for `dist` (M0-10). |
| release `codegen-units = 1` | LLVM sees the whole crate at once, so it optimizes better. Slower build. |
| release `strip = true` | Removes debug symbols; much smaller binary. |
| release `panic = "abort"` | No unwinding tables: smaller and slightly faster. A panic ends the process straight away. That's fine because game logic must not panic, and save files will be written atomically (M5). |
| `profile.profiling` | `dist` plus line-level debug info, so profilers can show function names. |

## M0-7 · Lints

- `clippy::all` is denied: any warning from the default Clippy set fails the build.
- Curated `clippy::pedantic` lints, set to warn (and `-D warnings` in the check script turns them into errors):
  - **Casts:** `cast_lossless`, `cast_possible_truncation`, `cast_sign_loss`, `cast_precision_loss`. Game code converts between `f32`, `f64`, `u32` and `usize` often, and silent truncation is a classic source of bugs.
  - **Floats:** `float_cmp`. Comparing floats with `==` is almost always wrong in physics code.
  - **Readability:** `explicit_iter_loop`, `manual_let_else`, `semicolon_if_nothing_returned`, `uninlined_format_args`, `unnested_or_patterns`, `redundant_closure_for_method_calls`.
  - **Wasted work:** `implicit_clone`, `inefficient_to_string`.
  - **Copy-paste slips:** `match_same_arms`, which catches duplicated match arms. That matters for our exhaustive `ObjectKind` matches.
  - **Docs:** `doc_markdown`, which requires code names in doc comments to be in backticks.
- Not enabled from pedantic: `needless_pass_by_value` (Bevy systems take `Res<T>` by value by design), `module_name_repetitions` (noisy) and `must_use_candidate` (noisy for systems).
- `unwrap_used` and `expect_used` warn; tests are exempted in `clippy.toml`. `print_stdout`, `print_stderr` and `dbg_macro` warn, because we use Bevy's logging macros instead.
- `too_many_arguments` and `type_complexity` are allowed: Bevy systems naturally take many parameters and long query types.
- `rustc`: `unsafe_code = "forbid"`, `missing_docs = "warn"`.

## M0-8 · Library + binary layout

- **What:** all code lives in the library (`src/lib.rs`); `src/main.rs` just calls `stardrop::run()`.
- **Why:** benchmarks (`benches/`) and integration tests (`tests/`) can only use a library crate. Only the pieces benchmarks need are `pub`; everything else is private or `pub(crate)`.

## M0-9 · Benchmarks with criterion `=0.8.2`

- **What:** a dev-dependency only, never shipped. `default-features = false` skips plotting and rayon.
- **Why:** the brief asks for statistically sound before/after numbers for hot functions. The standard library's `#[bench]` is still unstable.

## M0-10 · `release` uses thin LTO; `dist` uses fat LTO

- **What:** `[profile.release]` uses `lto = "thin"`. A new `[profile.dist]` inherits it with `lto = "fat"`. `profiling` inherits from `dist`.
- **Why:** with fat LTO, every release build took ~172 s even after a one-line change, and the quality check runs one each time. Thin LTO: 34 s, binary 43.0 MB instead of 39.7 MB. Bevy's official game template uses the same split.
- **Rule:** performance budgets and anything we ship are measured with `cargo build --profile dist`.

## M0-11 · Window resize "jumps": platform effect, accepted

- **Symptom:** while the window is being resized, the circle jumps instead of staying smoothly centered.
- **Investigation:** I measured the time from each resize step until our window shows a correctly centered frame, by reading only our own window's pixels over X11 (50 steps, 2 runs per configuration):

  | Configuration | Median | p90 | Worst |
  |---|---|---|---|
  | Low-power + vsync (current) | 29 ms | 42–46 ms | 63–66 ms |
  | `desired_maximum_frame_latency = 1` | 29 ms | 30–46 ms | 63 ms |
  | Pipelined rendering disabled | 29 ms | 29 ms | 46 ms |
  | Continuous (F1 open) + vsync | 21 ms | 24–34 ms | 41–52 ms |
  | Continuous + pipelined rendering disabled | 46 ms | 46–48 ms | 63 ms |
  | Vsync off (Immediate; Mailbox is unavailable on this NVIDIA/X11 setup) | **8 ms** | 10–12 ms | 12–20 ms, but **~220 % CPU** in continuous mode, and tearing |

- **Root cause:** the window manager resizes the window immediately. With vsync, our correctly sized frame reaches the screen ~2 refreshes later, and meanwhile the old, wrong-size image is shown. Every rendered frame is already correct; only the presentation latency differs.
- **Decision:** keep vsync. The only real fix is turning vsync off, which brings tearing and high power use, and both go against Section 7. Resizing mid-game is rare. We'll revisit when fullscreen and window settings arrive (M5).

## M1-1 · Pure logic may use `bevy::math`

- **What:** the engine-free modules (`physics::movement`, later collision, effects and so on) import only `bevy::math` (`Vec2` and friends), never ECS types.
- **Why:** `bevy::math` is a re-export of the plain `glam` vector library with no engine inside. Using it avoids converting between two `Vec2` types at every boundary, and the logic stays testable without an `App`.

## M1-2 · Movement feel and timing

- **Values** (in `config.rs`):
  - gravity 450 u/s²
  - terminal velocity 450 u/s
  - max sideways speed 500 u/s
  - sideways acceleration 2000 u/s²
  - braking 1250 u/s²
  - wall restitution 0.3
  - stick dead zone 15 %
- **Why:**
  - About 1.6 s of preview below the astronaut at full fall speed, the usual reaction-time guideline for a calm game.
  - About 0.25 s to reach full sideways speed.
  - About 4 s to cross the screen.
- **Physics at 120 Hz** (Bevy's default is 64): one circle is essentially free (3 ns per step). It makes input more responsive and reduces tunneling for fast objects later.
- **Vertical speed eases toward terminal velocity at the rate of gravity** from either side. Objects that push the astronaut beyond terminal velocity (the comet stone) therefore wind down smoothly, with no special case.
- **Semi-implicit Euler** integration: as cheap as plain Euler, and stable for orbits later.
- **Interpolated drawing** between the last two physics positions, using `Time<Fixed>::overstep_fraction`.

## M1-3 · Camera follow

- **What:** vertical-only follow using exponential smoothing (`smooth_nudge`, rate 5/s, frame-rate independent). The target keeps the astronaut on the upper-third line plus a look-ahead of 0.25 s × fall speed. The view is clamped to the level. Jumps of more than one screen cut instead of panning.
- **Note:** a smoothed follower trails a steadily moving target by speed ÷ rate (90 units at terminal velocity). That roughly cancels the look-ahead, so at full speed the astronaut sits almost exactly on the upper-third line. That is the intended result.

## M1-4 · Parallax: two stacked tiles per layer, moved as one parent

- **What:** each layer is one parent entity with two copies of a dot tile, each 3600 units tall. Scrolling moves only the 3 parents, and each layer shares one mesh and one material, so each layer is drawn in a single batch.
- **Alternatives:** moving every dot each frame (hundreds of transform writes) or a custom shader (unnecessary for placeholders).
- **Random placement:** a tiny built-in `SplitMix64` generator with a fixed seed instead of the `rand` crate. We only need deterministic decorative randomness.

## M1-5 · Walls bounce, pushes coast (changes Section 3.1's "cushioned bump")

- **What:** side walls bounce the astronaut back at 60 % of its speed. Steering and braking then fade back in over 0.5 s (smoothstep), so the rebound shows even while the key is held. Sideways motion has a `Drift` state:
  - `Steered`: the player's own motion. Letting go brakes quickly (1250 u/s²).
  - `Coasting`: set by a wall bounce, and later by object pushes. Only a gentle 100 u/s² drag applies, so the astronaut drifts about 450 units over about 3 s. Any steering switches back to `Steered`.
- **Why:** player feedback. The brief's cushioned stop felt wrong "in space", and the first bounce version still stopped dead because braking ate the rebound in about 0.1 s. Keeping quick braking for the player's own steering preserves precise control.
- **Consequences:**
  - Object effects in M3–M4 (cloud puff, orbit slingshots) should set `Coasting`, so their pushes aren't braked away.
  - Walls are now a small but real movement tool; level design (M6) should account for them.
- **Alternatives:**
  - Lower braking for everything: spacey, but imprecise when the player lets go.
  - The brief's original cushion: felt wrong to the user.

## M2-1 · Level format: serde + RON, validated into a `Level` type

- **Dependencies:** `serde =1.0.229` and `ron =0.12.2`. Bevy already compiles both (for asset metadata), so they cost nothing extra.
- **Hand-written error type** (`LevelError` implementing `Display` and `std::error::Error`) instead of `thiserror`: about 20 lines, no new dependency.
- **Two types:** `LevelFile` is what's on disk. `Level` is validated and in world coordinates, and `Level::parse` is the only way to get one. The rest of the game can therefore never see an invalid level ("make invalid states unrepresentable").
- **`deny_unknown_fields`** enforces the consistency rule mechanically: a level file can't carry behavior fields.
- **File coordinates** are `(x from the left edge, depth downward)`, the convention of tile editors like Tiled. They are converted to world coordinates on load.
- **Checkpoints activate by depth** (user's choice, option A): crossing a ring's depth anywhere counts, so a safety net can never be missed.
- **Minimum height of one screen (1080):** the camera clamps between the level's top and bottom, and `f32::clamp` panics if the minimum is larger than the maximum. Validation rules this out.
- Validation reports the **first** error. That's simple, and the author fixes one problem at a time with hot reload.

## M2-2 · Level loading: a Bevy asset that holds a `Result`

- **What:** `LevelLoader` always produces `LevelAsset(Result<Level, LevelError>)`. Only I/O errors (missing or unreadable file) fail the load itself.
- **Why:** a broken level becomes plain data. The game shows the message on screen, and during hot reload the previous version keeps playing. Bevy's own load-failure path would drop the asset and wrap our message in generic text.
- **Hot reload** (with `--features dev`): scenery is despawned and respawned. The astronaut keeps its position unless the edit left it outside the level. Some editors write a file in two steps, so a save can reload twice; that's harmless.
- **Scenery shares one set of meshes and materials** (`SceneryAssets`), so reloading never creates new assets.
- **`LevelPhase` state** (`Loading` → `Falling`): gameplay systems run only once a valid level exists, using run conditions. A missing resource would otherwise stop Bevy with an error.

## M2-3 · Landing, and the "still falling" run condition

- **Collision** (`collision.rs`): pure circle-circle and circle-rectangle checks that compare squared distances (no square roots).
- **Landing:** touching the beacon pad sets the astronaut at rest on top of it, sends `LevelCompleted` and switches to `LevelPhase::Landed`, where physics stops.
- **`still_falling` run condition:** the phase must be `Falling` *and* no phase change may be pending. Several fixed steps can run in one frame, but state changes only apply between frames. Without the pending check, the step right after landing moved the astronaut again and completed the level twice. A test reproduces that failure when the check is weakened.
- **`add_play_rules`:** the level's gameplay rules are registered separately from file loading, so tests run the real rules in a minimal app.

## M2-4 · Checkpoints, respawn and retry

- **Activation** (option A): `newly_passed` returns the deepest ring at or above the astronaut, if it's further along than the active one. Skipping several rings at once activates the deepest. The active ring glows brighter; its color only updates when progress changes or rings respawn.
- **Progress** is `Option<usize>`: `None` means "no checkpoint yet, use the start". If a level edit removes the active checkpoint, `checkpoints.get(i)` returns `None` and the respawn falls back to the start, so it can't crash.
- **Respawn:** `LevelPhase::Respawning` pauses physics. A full-screen fade in night blue (0.3 s out, 0.1 s hold, 0.3 s in, smoothstep) covers the screen, and the astronaut is moved at rest while it's covered, with a `Respawn` event. "Under one second" is a **compile-time assertion**.
- **Retry (R):** teleport to the start, clear progress, send `RunRestarted` and resume falling. No loading and no spawning, so it's instant and can't leak. `NextState::set_if_neq` avoids re-running Falling's enter/exit systems when already falling.
- **Camera** cuts (no pan) on `Respawn` and `RunRestarted`.
- **The astronaut spawns on `OnExit(Loading)`, not `OnEnter(Falling)`:** respawns and retries also re-enter Falling and would have spawned extra astronauts.
- **F3** (debug builds) jumps to and activates the next checkpoint.

## M2-5 · Stardust and run tracking

- **`StardustSet`:** a `u64` bitset, one bit per mote. Collecting, copying, clearing and counting never allocate. Validation caps levels at 64 motes, far above the 3–6 the brief asks for.
- **Collection:** the astronaut is checked against each mote's position from the level data, not entity transforms. For at most 64 motes, a straight loop beats any lookup structure. `bypass_change_detection` for the "already collected?" read means `CurrentRun` is only marked changed on a real pickup, so mote visibility only updates then.
- **Stardust persistence:** motes stay collected through respawns, and come back on a retry (`RunRestarted`) or a level reload.
- **Run time** counts in `FixedUpdate` during `Falling` and `Respawning`, so it is always a whole number of 1/120 s physics steps and identical at any frame rate. That also makes it suitable for replays later.
- **Leak test** (`soak_tests.rs`): 20 retries, respawns and scenery reloads leave the entity, mesh and material counts unchanged. Sabotaging the reload makes it fail (58 → 118 entities).

## M2-6 · A startup crash, and the `--smoke` check

- **Bug:** the run condition `resource_changed::<ActiveLevel>` needs the resource to exist, so the game crashed in its first frames, before the level had loaded. Unit tests missed it because their apps always insert a level first.
- **Fix:** `resource_exists_and_changed`. A regression test (`runs_before_any_level_has_loaded`) starts the plugins with no level.
- **Process:** `./scripts/check.sh --smoke` also runs the real release game for 6 s and fails on any panic or error. I verified it fails on the crashing build. It opens a window, so it's opt-in, but it's used before every commit.
- **Rule for later:** conditions and systems that read resources created at runtime (`ActiveLevel`, and more in M5) use `resource_exists…`, `Option<Res<T>>` or state gating.

## M2.5-1 · Design change: portrait one-screen puzzle levels

- **What:** every level is one fully visible 1080 × 1920 portrait screen. Attempts start floating at the start (the fall begins on the first ←/→), and a failed attempt fades back to the start with stardust and time reset. There are no checkpoints, and the camera is fixed.
- **Why:** after playing Milestones 1–2, the user wanted players to see the whole path from the start, "more like a quiz". Seeing the level turns object discovery into forming and testing guesses, and short attempts make retrying cheap. This is the Peggle / Cut the Rope family of puzzle-arcade games. The user chose "puzzle + stars for mastery".
- **Alternatives:**
  - landscape one-screen levels: short falls; routes would have to be mostly sideways
  - a zoomed-out overview, then follow as before: planning relies on memory
- **Consequences:**
  - camera follow, look-ahead and scrolling parallax are removed (M1-3 and M1-4 superseded)
  - checkpoints are removed (M2-4 partly superseded; the fade and retry stay)
  - the level format loses `height` and `checkpoints`
  - the brief is updated, with a new Milestone 2.5

## M2.5-2 · Implementing portrait puzzle levels

- **Camera:** fixed at the playfield's center with `ScalingMode::AutoMin { 1080, 1920 }`, so the whole playfield is always visible. Dimmed veils and faint edge lines mark the walls beside it.
- **Background:** dot layers drift upward on their own (300 u/s × layer depth factor). There's no camera to follow, so only 3 layer transforms change per frame.
- **`LevelPhase`:** `Loading → Ready → Falling → Restarting | Landed`.
  - In `Ready`, the physics body itself bobs (a sine wave, 6 units, 2.4 s period), so the fall starts exactly where the astronaut is drawn.
  - The first non-zero `AirControl` switches to `Falling`.
  - The bob's clock lives in a resource, reset on entering `Ready` and on every restart. Messages last only about two frames, too short to carry the reset through a 0.7 s fade.
- **Attempts:** failure, R and a level reload all end in `restart()`, which moves the astronaut to the start and sends `AttemptRestarted`. Collectibles reset on that one message.
  - Reload restarts skip the first load (`is_added`): inserting the level must not count as a reload. Tests caught this.
- **Run time** counts only while falling (`still_falling`), so studying the level is free.
- **Text:** plain words (no arrows or "·"), because Bevy's built-in font is a small subset. Real fonts come in M7.

## M3-1 · The object framework

- **Two enums:** `ObjectKind` (fieldless: a *type*, used in events, tuning and later the journal) and `Placement` (an enum *with data*: one placed object, carrying only placement fields). Behavior is one exhaustive `match` in `objects::behavior::on_enter`, a pure function of the type's tuning and the astronaut's state.
- **Consistency rule, three ways:**
  - tuning exists only in `GameConfig.objects`, once per type
  - level files can't hold behavior fields (`deny_unknown_fields`, per enum variant)
  - effects are pure functions of the type's tuning and the astronaut's state
- **`Shape` trait** (`collision.rs`) with `CircleShape`. The wind ribbon's path shape joins it in step 3.
- **Contact on entry:** each object entity has `Contact(bool)`. An effect fires only on a not-touching → touching change, and contacts are cleared on every fresh attempt.
- **Hitboxes vs outlines:** validation uses the full visible outline. Play uses the hitbox, which for hazards is 80 % of the outline (`hitbox_scale`); a test checks that a visible graze is safe.
- **Hazards end attempts through `attempt::fail`**, the same path as falling out.
- **Visual reaction:** `effects::ContactPulse`, an always-present component holding `Option<f32>`, so touches never add or remove components. The squish is a half-sine bump.
- **Assets:** unit-sized meshes shared per type (`ObjectAssets`), scaled per object; reloads create none.
- **F2** needs Bevy's gizmo features, which are only in the `dev` cargo feature. `check.sh` runs Clippy a second time with `--features dev`, so that code is linted too.

## M3-2 · Cloud puff and crystal

- **Pushes use `Body::launched(velocity)`:** the astronaut takes the new velocity, **coasts** (so braking doesn't swallow the push; the user's "spacey" preference, M1-5) and gets the same brief control fade-in as a wall bounce. `wall_recovery` is renamed `control_recovery`, since walls no longer own it.
- **Cloud puff:** fixed speed (650 u/s) directed away from the cloud's center, with the tilt clamped to at most 45° from vertical. It depends only on *where* it's touched, never on incoming speed, so it's the same every time.
- **Crystal:** negates the sideways velocity, with at least 350 u/s, and keeps the fall speed. With zero sideways speed, it pushes away from the crystal's center (right if exactly centered).
- **Hitboxes:** helpful and mixed objects use their full outline (generous). The crystal uses the circle through its tips.
- **Caught before running:** `touch_objects` first asked for `&Motion` and `&mut Motion` on the astronaut. Bevy rejects that at startup (B0001, the same rule as Rust's borrowing), so it now uses a single `&mut` handle.

## M3-3 · Wind ribbon

- **Shape:** `PathShape<'a>`, a chain of capsules (a centerline plus a half-width) that *borrows* the ribbon's points. `ObjectShape` is an enum of `Circle` and `Path` that implements `Shape`, so collision, validation and F2 treat every object alike.
- **Behavior:** continuous, in the pure function `while_inside`. Each step inside, the velocity moves toward `direction of the nearest segment × 500 u/s` at 1800 u/s² (4× gravity, so ribbons can carry you up, and you can still steer a little). The astronaut is marked `Coasting` inside, so leaving at the end or out the side keeps the speed ("released at its end", and the spacey feel).
- **Entering** a ribbon still sends `ObjectTouched` (for the journal and sounds), with `Outcome::Nothing` as its instant effect.
- **Meshes per ribbon:** unlike other types, each ribbon's strip and chevrons follow its own path. They're freed when the ribbon despawns (the last handle drops the asset). The leak test includes a ribbon and confirms the mesh count stays constant over 20 reloads.
- **Validation:** 2–16 points, all inside the level; width 30–240; not overlapping the start. Per-type size ranges replace the single range.

## M3-4 · Broad phase: plain loop (measured)

- **Measured** (criterion, release). The grid was written only in the benchmark file:

  | Objects on screen | Plain loop | Uniform grid (240-unit cells) |
  |---|---|---|
  | 12 | 171 ns | 91 ns |
  | 40 (a very busy level) | 560 ns | 228 ns |

- **Decision:** keep the plain loop. The grid is about 2.5× faster, but the loop costs at most about 1.1 µs per frame (two physics steps with 40 objects): 0.1 % of the 1 ms budget. The grid would also need rebuilding on every reload and special handling for long ribbons. Section 7.3 says not to trade readability for speed unless a measurement shows it matters, and this one shows it doesn't.
- **Revisit** if a level ever holds hundreds of objects. The benchmark stays in `benches/logic.rs`.

## M4-1 · Timed effects and how they combine

- **One `Effects` value on the astronaut's `Body`** (plain Rust, `Copy`), holding "seconds left" per effect: `swapped_controls` (spores) and `comet_dive` so far; bubble capture and the portal cooldown join them. `Effects::after(dt)` counts them all down, never below zero.
- **The rules** (the brief asks for these to be written down):
  1. **At most one of each kind.** Touching the same kind again resets its timer; effects never stack.
  2. **The latest push wins.** A cloud bounce during a comet dive replaces the velocity.
  3. **Spores only change the controls**, so they combine with everything, including pushes.
  4. **A fresh attempt clears everything** (the astronaut is rebuilt with `Body::at_rest`).
- **`on_enter` now returns the astronaut's new state** (`Outcome::Changed(Body)`) instead of a velocity. Boosts, swapped controls and (soon) bubble capture then all fit one exhaustive `match`, and each object's effect stays a pure function.
- **Comet dive:** the terminal-velocity rule is skipped while `comet_dive` is running, so the dive holds its speed; when it ends, the existing "ease toward terminal velocity" takes over with no special case.
- **Spores** are shown by a magenta ring around the astronaut, and both timers appear in the F1 overlay.

## M4-2 · Bubble

- **Capture copies the bubble's tuning into the astronaut's state** (`Effects::captured: Option<Capture>` holding rise speed, nudge speed and seconds left). Movement physics therefore never needs to know what a bubble is, and the values still come from the one per-type config.
- **While captured**, `step` ignores gravity, air control and drag: the bubble carries the astronaut up, sideways nudging is capped at 120 u/s, and the side walls still hold. When the timer ends, that same step sets the velocity to zero: "the fall resumes from rest".
- **Only hazards reach the astronaut inside a bubble** (the user's choice): every other object is skipped, including wind. Static clouds still end the attempt, so "a static cloud always sends you back" has no exceptions.
- **One use per attempt** (the user's choice): the bubble entity carries a `Used` flag and hides itself; a fresh attempt restores every one.
- The astronaut is drawn inside a bubble while held, the same way spores show a shimmer.

## M4-3 · Gravity orb

- **Two radii:** the level sets the visible core; the pull field is 4× that (per-type tuning, so every orb of the same size has the same reach). The field is the hitbox; the core is the outline used for validation and drawing, and the field is drawn faintly so players can judge its reach.
- **Pull = strength ÷ distance**, capped at 2000 u/s². That shape makes orbits at different radii feel similar, instead of the violent spike a true inverse-square law gives up close.
- **Speed inside the field is clamped to 700 u/s** (Section 3.4: "clamped so orbits feel good and never produce extreme speeds"). A test runs the worst case, falling straight in for 5 seconds, and checks the cap holds.
- **Touching the core** replaces the velocity with a soft 420 u/s push straight out, instead of letting the astronaut pass through the center.
- Like the wind ribbon, the orb acts continuously (`while_inside`) and entering it does nothing by itself.

## M4-4 · Twin portals

- **One entry per pair** (the user's choice): `TwinPortals(a:, b:, radius:)`. A portal can't be unpaired or double-paired, because a level file can't express it ("make invalid states unrepresentable"). Validation only has to check that the ends are at least three radii apart.
- **A third shape:** `ObjectShape::Pair([CircleShape; 2])`, touched if either end is.
- **`on_enter` takes the placed object**, not just its type and center, because a portal must know which end was touched (the nearer one). The effect is still one exhaustive `match`, now over `Placement`.
- **`Outcome::Teleported`** exists so the astronaut *jumps*: the drawn position is reset too, instead of being smoothly blended across the screen.
- **Cooldown** of 0.5 s in `Effects`, so you can't bounce straight back. Speed, direction and all other effects (spores, dives) carry through unchanged.

## M5-1 · Menu input needs mouse and UI picking features

- **What:** added Bevy's `mouse`, `bevy_picking` and `ui_picking` features so menu buttons respond to hover and clicks (the user chose keyboard + gamepad + mouse).
- **Cost:** the release binary grew from 44.4 MB to 46.0 MB (+1.6 MB, 3.6 %).
- **Alternative:** hand-written hit testing against UI rectangles with only the `mouse` feature. It avoids the growth but reimplements what Bevy already does, and would need redoing for every widget.

## M5-2 · Packaging: one standalone binary, with an optional assets folder (done in M5-7)

- **What:** assets get embedded in the executable, so the game ships as a single file. If an `assets/` folder exists next to the binary, it takes priority.
- **Why:** the user asked whether a standalone binary is possible. Embedding alone would break Section 4's rule that levels are editable without recompiling; the override keeps both.
- **Cost:** binary grows by the embedded asset size (today ~1 KB of levels; sounds and music in M8 will add a few MB), plus one small crate for embedding.
- **Unaffected:** the save file always lives in the OS's user-data folder, so the game never writes to its install directory.

## M5-3 · Screens, and what pausing must not do

- **`AppState`** (`MainMenu`, `LevelSelect`, `Journal`, `Playing`, `Paused`, `LevelComplete`) with per-screen power modes: only `Playing` renders continuously; menus wake on input or 4 times a second. That clears the M1 tech-debt item.
- **The in-level phase stays its own state** rather than a sub-state of `Playing`, because pausing leaves `Playing` and would otherwise destroy and recreate the phase (reloading the level on resume).
- **Loading and unloading hang off entering screens, not leaving `Playing`:** the level is loaded when play is entered (skipped if that same level is already loaded, so resuming keeps it) and freed when a menu is entered. Three bugs came from getting this wrong; a temporary autopilot that walked the whole flow without input found them all.
- **Run conditions must include "a level exists":** between finishing one level and loading the next, systems that read `ActiveLevel` would stop the game. `playing` and `still_falling` now require it. In tests (no `AppState`) "no app state" counts as playing, so small test apps stay simple.
- **`apply_level_changes` takes the level handle as optional:** leaving a level drops the handle while the asset system still delivers one last event.
- **Menu navigation** is shared by every screen: a focused item, moved by keys, stick, or mouse hover, activated by Enter/Space/gamepad A or a click.

## M5-4 · Saving progress

- **Where:** the OS's data folder via `dirs = "=7.0.0"` (e.g. `~/.local/share/stardrop/`). It pulls in only `dirs-sys`, `option-ext` and `libc` (already present). The game never writes to its own folder, so it still works installed read-only.
- **Atomic writes:** the save is written to `save.writing.ron` and then renamed over `save.ron`. A rename is atomic, so a crash mid-write can't leave half a file. The previous save is kept as `save.backup.ron`.
- **Never lose progress silently:** an unreadable save is *renamed* to `save.unreadable.ron` (not overwritten), the backup is tried next, and only then does the game start fresh. Tests cover a missing save, a broken save with a good backup, and both broken.
- **Keyed by file name, not position:** reordering levels or inserting one can't move someone's progress onto the wrong level.
- **Stars merge across attempts** (Section 3.5's "best"): a fast run and a thorough run together earn both stars.
- **Written when leaving play** (entering a menu or the complete screen) and on quit, not on every change.

## M5-5 · The journal shows the real objects

- **What:** the journal's pages are laid out in a grid far away in the world (y = 40 000), and the camera simply looks there while the journal is open. Discovered types are drawn by the *same* code that draws them in play; undiscovered ones are dark discs with "???".
- **Why:** no second set of pictures to draw or keep in step with the real objects. Rendering meshes into UI images would have cost a render target per page.
- **The camera follows the screen, not the other way round:** `look_at_current_screen` derives the camera's position from `AppState` every frame and writes only on change. The first version had the journal move the camera, and because the camera doesn't exist on the very first frame, Bevy skipped that whole system, including building the pages.
- **Shared object assets are built when the plugin is built,** not in a startup system, since the journal can open before startup systems have run.
- **Notes** are the astronaut's guesses, never explanations (Section 3.6), and the journal never says what an object *does* mechanically.

## M5-6 · Settings

- **Stored** as `settings.ron` beside the save, written atomically through the same helper, and loaded at startup. Unreadable settings fall back to the defaults (they're not progress, so there's nothing to rescue).
- **Applied immediately** when changed: fullscreen switches the window mode, "plain" graphics removes bloom from the camera, and reduce motion scales background drift and the waiting bob to a quarter.
- **Rows are activated, not dragged:** Enter steps a row to its next value (volumes 0/25/50/75/100). That reuses the existing menu navigation instead of adding sliders with their own input handling. Sliders can come with the M7 visual pass.
- **Volumes are stored but silent** until audio exists in M8; the screen says so.

## M5-7 · The shipped game is one file

- **What:** `bevy_embedded_assets = "=0.16.0"` bakes `assets/` into release builds (`ReplaceAndFallback`), so the game ships as a single executable. Files that aren't embedded still load from an `assets/` folder beside the binary, so players can drop in *new* levels; replacing a shipped one would need our own asset reader, which isn't worth the code today.
- **Off during development:** `--features dev` skips the plugin entirely, so the folder is used and hot reload keeps working. Verified by grepping both binaries: the release one contains the level text, the dev one doesn't.
- **Cost:** +0.09 MB (46.57 → 46.66 MB) for 72 KB of assets. Sounds and music in M8 will add their own size.
- **Verified** by copying only the binary into an empty folder: it started and found all ten levels.
- Saves and settings still live in the OS data folder, so the game never writes where it's installed.

## M6-1 · A solver proves every level is beatable

- **What:** `sim.rs` plays a level without Bevy, using the same pure rules; `solver.rs` beam-searches steering inputs over it. A test requires every shipped level to be landable *and* to have all its stardust reachable in one attempt; another test replays identical inputs through the real game and the simulator and requires the same result, so the two can't drift.
- **Target times** come from the measured fastest route x1.4 (the user's choice), rounded to half a second, instead of being guessed.
- **The solver found real faults:** one stub level whose stardust could not all be collected, and after each fix a further level that was outright unbeatable.
- **It also had three of its own,** each of which made it report "no route" for levels that are easy by hand:
  1. pruning purely by depth threw away lines that rise (a bubble lift);
  2. without spreading the beam sideways it collapsed onto one line and never met objects elsewhere;
  3. treating two astronauts in the same spot as the same state merged "drifting right" into "falling straight" on the very first branch, killing the right-hand half of the search.
  The cell now covers position *and* velocity.
- **`examples/probe.rs`** is the design tool: it prints a level's routes, where they pass, and how far each missed mote was.

## M7-1 · The art is drawn in code, not painted

- **What:** every shape is a mesh built at runtime (`soft_mesh.rs`), with its colours baked into its vertices. No textures ship with the game.
- **Why:** it keeps the single-file binary, stays sharp at any window size, costs no texture memory, and a whole altitude is a handful of colour constants I can change and see immediately. The user chose this over painted sprites.
- **The one rule that makes it work:** every shape fades out at its rim instead of ending in a hard edge. A soft falloff is the difference between "painted" and "cut out".
- **Learned the hard way:** a big quad with transparent corners does *not* look soft. The GPU interpolates across each triangle, so a four-corner shape shows a hard crease along the diagonal. Aurora curtains and meteor streaks are now stacks of overlapping soft discs instead, and wind ribbons carry four lines of vertices across their width (clear, solid, solid, clear).

## M7-2 · One baked mesh per background layer

- **What:** a layer's 150 stars (or 12 clouds, or 7 curtains) are baked into a single mesh, drawn as two stacked copies that scroll. A theme's meshes are built the first time that theme is shown and kept.
- **Why:** the obvious way — one entity per star — cost ~1000 entities and a transform each frame. Baking makes a layer 2 entities and 1 draw call, and moving the layer moves one parent.
- **Cost:** ten themes of meshes if a player visits them all, a couple of hundred kilobytes. Level changes then rebuild nothing.

## M7-3 · Effects come from a pool, never from spawning

- **What:** `particles.rs` spawns 192 spark entities at startup and reuses them forever. Anything wanting an effect writes a `Burst` message; nothing else spawns, and asking for more sparks than the pool holds recycles the oldest.
- **Why:** Section 7.2 asks for pooled, capped effects. It also means a burst can never allocate mid-frame or leak.
- **Sparks fade by shrinking,** not by changing their colour: they share one material per colour, so a per-spark fade would mean a material per spark.
- **A subtlety the soak test caught:** the pool's starting material was freed once every slot had been recoloured, so the asset count *dropped* during a long run. The material now lives in the colour cache, and the pool creates and frees nothing after startup.

## M7-4 · The window opens portrait

- **What:** the game opens a 9:16 window about two-thirds of the monitor's height, centred, and remembers a size the player sets (written to the settings file a second after they stop dragging).
- **Why:** the playfield is portrait. A landscape window showed the game in its middle third, which was the oldest thing on the debt list.
- **The monitor's size isn't known until the window exists,** so the fitting happens on the first frame a monitor is reported, not in the window's opening settings.

## Measurements (M7)

Measured 2026-09-24, same machine as the M0 baseline.

| Metric | Value | Notes |
|---|---|---|
| Our own systems, per frame | **0.110 ms** | Busiest shipped level, headless, release. Budget: 1 ms. `cargo test --release --lib -- --ignored frame_cost --nocapture` |
| Whole frame, full quality (bloom) | **0.93 ms** avg, 4.98 ms worst | Vsync off. 60 FPS needs 16.7 ms. |
| Whole frame, plain quality (no bloom) | **0.51 ms** avg, 1.56 ms worst | Bloom costs about 0.4 ms a frame here. |
| Memory over 400 play/fail/retry/reload rounds | 24812 KB → **24856 KB** (+44 KB) | Headless, release; the growth is in the first 100 rounds and then flat. `--ignored long_session` |
| Entities in play | 1024 | Including the 192-spark pool and the background. |

## M8-1 · bevy_kira_audio, chosen over bevy_seedling

- **What:** `bevy_kira_audio = "=0.26.0"` (kira 0.12), with only its `ogg` feature.
- **Both fit Bevy 0.19** — kira_audio 0.26 wants bevy ^0.19, seedling 0.8 wants ^0.19 — so the choice was about the shape of the work, not compatibility.
- **Why kira:** channels map one-to-one onto the five volume settings, every command takes a tween (so nothing ever cuts), instances can be controlled individually, and playback rate gives pitch. Seedling's node graph is more powerful and probably where Bevy audio is going, but it would have spent this milestone on plumbing.
- **What it costs:** +1.5 MB of binary (48.2 MB, was 46.7), mostly symphonia's ogg decoder — which we don't even use, since nothing loads from a file. Worth revisiting at M9 if size matters.
- **The one gap:** the wrapper doesn't expose kira's filters, so the low-pass Section 6.2 asks for while paused can't be applied at runtime. See M8-3.

## M8-2 · Every sound is synthesised, none recorded

- **What:** `synth.rs` builds every note and every loop out of sine waves, filtered noise and envelopes. No audio files ship with the game.
- **Why:** the same reasoning as the art (M7-1). One file to ship, no licences to track, and every note is exactly in the level's key instead of a sample pitched towards it. It also means a sound is a few numbers, so it can be reshaped in seconds.
- **One sample per instrument, pitched when played.** A marimba is rendered once at 440 Hz; the note the game wants divided by 440 is its playback rate. Nine instruments cost 2.8 MB instead of nine instruments times every note.
- **Verified by measurement, not by ear alone:** a test renders each sound and a Goertzel filter confirms its strongest frequency is the one it was built at, to the hertz.
- **`cargo run --release --example listen`** writes every sound to `target/sounds/*.wav`, so the palette can be judged without playing the game.

## M8-3 · The music never stops, so it can never cut

- **What:** four loops start together and run forever: the pad, a deeper layer, a muffled copy of the pad, and the ambience. Everything the game does to the music is a volume change on one of them.
- **The deeper layer** fades in as the astronaut falls, reaching full by 85 % of the way down.
- **Pausing** fades the bright pad out and the muffled one in over 350 ms. Because both loops have been playing in step since the level began, they are at the same moment of the same music: the crossfade is inaudible as a change of *position*, only of tone. This is how Section 6.2's low-pass got done without a runtime filter.
- **Ducking:** an object's note drops the music by a third for a moment, so notes stay audible without anything getting louder.
- **A loop has to be periodic to loop.** Three things broke this and each was audible as a tick every eight seconds: envelopes that hadn't finished, partials whose frequency didn't fit a whole number of cycles into the loop (now snapped, at most 5 cents off), and — the subtle one — a one-pole filter starting from silence, so the loop's first moment was filtered differently from its last. The filter now runs a throwaway pass first to warm up. A test measures the step across the join against the ordinary step between samples.
- **Real noise cannot loop,** so the ambience's "noise" is 48 quiet sines at scattered pitches, all snapped to the loop.

## M8-4 · The music is built off the main thread

- **What:** changing altitude spawns an `AsyncComputeTaskPool` task that renders the four loops; the game swaps them in on a later frame and fades them up.
- **Why:** building one altitude's music takes **74 ms**. On the main thread that would be four dropped frames at the start of every level — exactly the hitch Section 7.1 forbids.
- The fade-in hides the wait entirely: the music is a second and a half from silence anyway.

## M8-5 · Cold and vast, not sunny (after playtesting)

- **What the playtest said:** "the sounds are a bit too sunny and not spacey-ish at all".
- **Why it was:** three choices, all of them safe. Major pentatonic everywhere; struck, bright timbres (marimba, bell, music box); and — the real culprit — no reverb at all, so every note stopped dead the moment it was played.
- **The fix, in three parts:** the scale is now **minor pentatonic**; melodies and the landing chord are built on **open fourths and fifths**, never thirds (the landing chord is root-fifth-octave); and every sound heard in a level is put through a **synthesised reverb** with a ~2.5 s tail. Voices are darker too: lower filter cutoffs, slower swells, and pairs of partials a few cents apart so sustained notes beat against themselves instead of sitting still.
- **The reverb** is a Schroeder network — four feedback delays whose lengths share no whole-number ratios (so their echoes never line up into a rhythm), then two all-pass diffusers to smear them, then a filter, because distance takes the top off everything.
- **Menus stay dry.** The interface is not in the sky; only what happens in a level rings.
- **Cost:** one-shot sounds grew from 2.78 MB to **7.65 MB**, since every one now carries seconds of tail. Bed build time is unchanged at ~77 ms.
- **The brief §6.1 was updated** to match: it specified major pentatonic, which was the root of the problem.

## M8-6 · `target/` is kept small on purpose

- **What happened:** the build folder grew past ten gigabytes and had to be deleted by hand. One release build alone is **1.5 GB**; a debug build of Bevy with full debug info is several more, and every extra profile is another full copy.
- **Three changes:** `[profile.dev] debug = false` (we debug through tests and logs, never a step debugger); the `dist` and `profiling` profiles are removed until M9 actually needs them (they are recorded in M0-10 and are four lines to restore); and `scripts/check.sh` sets `CARGO_INCREMENTAL=0`, since it rebuilds everything anyway.
- **What to run when disk gets tight:** `cargo clean --profile dev` drops the debug build and keeps the release one you play. `~/.cargo/registry` (2.7 GB) is shared with every other project — leave it.

## M8-7 · Three things the first playtest found

- **"The sounds are a bit sharp."** Not the sounds themselves — they are nearly pure tones — but how they were played. A sample played faster is higher *and shorter*, and the melody climbed far enough that a stardust twinkle built at 1760 Hz came out at 9 kHz as a chirp. Now `playable_rate` folds every note into the octave around the pitch its sound was built at (an octave is the one interval the ear hears as the same note), stardust climbs within a single octave, struck notes swell over 18 ms instead of starting at full, and everything gets a second gentle roll-off at the end — 2.6 kHz in a level, 1.8 kHz for menu sounds.
- **"The background music doesn't change when I'm stardropping."** The machinery was working — a probe showed the layer fading in to 0.69 during a fall — but the layer *sounded* like more pad: another slow drone, same register. It is now an octave lower and pulses four times per loop, so the fall deepens rather than just getting louder. It also arrives sooner (full by 55 % of the way down, catching up half again as fast), and in a four-second fall it now reaches 0.99.
- **"The sounds in levels are the same as in the menu."** They were: menus and levels played the identical bed, and the menu's key is the same as the first level's. A menu is nowhere, so it now plays the pad alone, quieter, with no air moving in it; a level adds the ambience, and falling adds the layer.
- **The lesson for the rest of the audio work:** check what the game *plays*, not only what the synthesiser *makes*. Every one of these was invisible in the rendered WAVs.

## M8-8 · New silhouettes, and an astronaut who is a person

- **What the playtest said:** the shapes were "too childish and basic", and "the cosmonaut is just a circle".
- **It was right.** Every object was a primitive: a cloud was three equal circles, a crystal one rhombus, a flower a five-petal daisy, the hazard a spiky star, and the astronaut a plain disc with an ellipse on it.
- **The shapes now carry the idea:** the cloud is seven uneven lobes with light on one side; the crystal a cluster of shards with a facet catching the light; the orb a core inside a disc of matter seen nearly edge-on; the portals swirls turning inside a dark throat; the flower a seed pod with loose spores around it; the comet a lopsided pitted rock with a narrow hot trail; the hazard a hole with long feelers in a bruise of light; the stream of air narrows and fades at both ends instead of stopping square.
- **The astronaut** has shoulders, a body, a domed helmet with a window in it, and a shoulder lamp — at 28 units across, the lamp is what you actually see from the far side of a level. Two tries: the first had a visor so large it read as a teacup.
- **Three new mesh tools** carry all of it: a filled outline with a soft rim (rocks, shards, petals), an arc that fades at both ends (rims, swirls, accretion discs) and a tapered spike (petals, tendrils).
- **Everything is still shared and unit-sized,** so this costs no memory per object: nine types' worth of meshes, scaled per object.

## M8-9 · Levels 6-10 rebuilt, and a number for "hard"

- **What the playtest said:** "the higher levels are a bit too simple".
- **Why they were:** every level had three or four objects, whatever its number. The game got *new* objects as it went, but never more of them, and never a route that needed two moves in a row.
- **Now:** eight to eleven objects each, routes that need a sequence, and a hazard guarding the approach to every late beacon. Object counts run 0, 3, 3, 3, 4 across the teaching levels and 8, 8, 9, 8, 11 across the rest.
- **"Hard" is now measured, not asserted.** `difficulty_of_every_level` plays 300 careless runs per level — hold a direction for a third of a second, pick again — and reports how many land. That is the closest thing to a first-time player this project has.

  | | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
  |---|---|---|---|---|---|---|---|---|---|---|
  | before | 41 % | 37 % | 26 % | 37 % | 11 % | 17 % | 17 % | 30 % | 5 % | 23 % |
  | after | 41 % | 37 % | 26 % | 37 % | 11 % | 7 % | 7 % | 7 % | 5 % | 5 % |

- **It took four rounds of tuning,** and the measurement earned its keep every time: the finale was the third-*easiest* level in the game, and one hazard added to Slow Bloom made it instantly the hardest (4.7 %) until its radius came down.
- **Still uneven at the front:** level 4 (37 %) is easier than level 3 (26 %). Left alone for now — those are the teaching levels, and they weren't what the playtest was about.
- **Target times still come from the solver** (fastest route x1.4), so the two levels with a comet dive in them ask 3.5 s, which only the dive can make.

## M9-1 · What shipping looks like

- **One file.** `scripts/package.sh` builds the `dist` profile and drops `stardrop`, the README and the credits into `package/`. The binary carries its own assets (M5-7), so it runs from an empty folder — checked by copying it into one and starting it.
- **`dist` and `profiling` are back** in `Cargo.toml`, removed at M8-6 to keep `target/` small. Build them when needed, `cargo clean --profile dist` after.
- **44.3 MB** with fat LTO, against 48.2 MB for the everyday release build.

## Measurements (M9) — the Section 7.1 budgets

Measured 2026-10-04 on the `dist` build, same machine as the M0 baseline.

| Budget | Required | Measured | |
|---|---|---|---|
| Frame rate, full quality | stable 60 FPS | **0.65 ms** a frame with vsync off (≈1500 FPS); 16.68 ms locked, worst 17.0 | ✅ 25x headroom |
| Frame rate, plain quality | — | **0.51 ms** a frame | ✅ |
| Our own systems | under 1 ms a frame | **0.100 ms** | ✅ |
| No hitches | no visible stutter | worst frame **17.0 ms** across a whole level, vsync on | ✅ |
| Level load and retry | under 200 ms | level change **50 ms**; retry moves one entity and loads nothing | ✅ |
| Idle efficiency | quiet in menus | **2.7 %** of one core in a menu, against 15.4 % while falling | ✅ |
| Memory | flat over a session | **390 MB** in menus, **411 MB** playing, steady; +40 KB over 400 play/fail/retry/reload rounds | ✅ |
| Startup | a couple of seconds | **420 ms** to the first frame | ✅ |
| Binary | one file | **44.3 MB**, assets inside | ✅ |

The two numbers worth knowing the shape of: the game spends **0.65 ms** of a
16.7 ms frame doing everything it does, and a tenth of a millisecond of that
is our own code — the rest is the renderer. There is room for a great deal
more sky.

## M10-1 · The melody earns the third star

- **Why:** the musical system was the one thing in this game nobody else is doing, and it was decoration. The streak changed nothing, and the scoring was the three-star frame every puzzle game ships with.
- **What changed:** the "fast run" star is gone. The third star is now the **phrase**: touch the number of objects the level asks for, in one attempt. Every touch already plays the next note of the level's scale, so the player hears the phrase fill up and the interface never has to explain it.
- **It fixes a design fight too.** A stopwatch star and a calm game pull in opposite directions, and two levels asked 3.5 s, which only the comet dive could make. Time is still shown after a landing. It just isn't what a star is for.
- **Levels say how long their phrase is** (`phrase: 5`), validated against the number of objects they contain, so the star is always earnable. Teaching levels ask for 2 or 3, the back half for 5. First Light had no objects at all and now has two clouds, which also teaches on the first fall that touching things is the point.
- **The solver proves it.** A third goal, `Goal::Phrase(n)`, searches for a landing route that touches n objects. The first version scored touches without a cap and the search taught itself to bounce on one cloud forever rather than fall: a longer melody always beat a deeper fall. Capping the reward at what the level asks for fixed it.
- **Saves migrate.** Version 2 drops the old "fast" star rather than carrying it over, because a star has to have been earned at the thing it is for.

## M10-2 · The suite can finally see the screen

- **Why:** three of the last round of bugs were visual, and every one got through the tests. A shape can break silently in a way nothing here could notice.
- **What:** `cargo run --release --example golden` draws all nine objects and the astronaut in a fixed grid, renders them, and compares the result against a reference in the repository. `--write` accepts a new look. `./scripts/check.sh --visual` runs it.
- **The reference is not a picture.** It is the image reduced to a 45x30 grid of brightness readings (`tests/golden/shapes.txt`, 2.8 KB): small enough to read in a diff, insensitive to antialiasing noise, and still able to see a silhouette change, a part gone missing or a layer slipped behind another. A committed PNG would have been a binary blob that every tiny renderer difference invalidates.
- **Proved both ways:** two runs in a row differ in 0 of 1350 cells, and taking one petal off the spore flower fails it with 15 cells changed and a map of where.
- **Not in CI.** It needs a real GPU; a software renderer would make it flaky, and a flaky check is worse than none.
- **The journal, the game and this check now draw objects from one definition** (`objects::preview_placement` and `astronaut::spawn_figure`), so what the check guards is what the player sees.

## M10-3 · Stars you can see, and a phrase you can watch

- **Stars were ASCII** (`*` and `-`) on every screen, after everything else had been drawn properly. They are now a five-pointed star generated in code as a small image, tinted gold when earned and nearly invisible when not. Two tries: a cosine radius profile makes petals, not points — a star needs straight edges from tip to notch — and the first pointed version came out rotated, because picture rows run downwards and up is negative y.
- **The finish screen was one string** with four facts crammed into it. It is now the stars, then time, stardust and phrase as labelled rows, then the choices.
- **The phrase is visible while falling:** a row of marks along the top, one per note the level asks for, lighting as each object is touched. The melody is still the real feedback; this is for the player who hasn't yet noticed that the notes mean something. They sit at the top because the bottom belongs to the beacon.

## M10-4 · Four more levels, locked behind the ten

- **What:** Thin Air (streams), Cold Current (orbits and doorways), Late Sun (two dives) and Night Flight (everything, narrowest pad in the game). Nine to eleven objects each, phrase of 5. They sit at the end of `levels.list.ron`, so the existing rule — a level unlocks the one after it — keeps them locked until Homecoming is finished.
- **Nothing new is introduced.** By level 11 every object has been met; these ask the player to use them in sequence.
- **Tuned against the measurement, not by feel.** Careless play lands 6.0 %, 4.7 %, 5.0 % and 3.7 % of the time, against 5.0 % for Homecoming, so the game keeps descending to the end. Thin Air started at 16 %: its last stream funnelled anything into the pad. Night Flight started at 7.7 %.
- **Each one needed the solver to make it honest.** Three of the four shipped with a mote that could not be collected along with the others, and the solver found all three before a player could.

## M10-5 · CI pins the toolchain

- **What happened:** the first CI run failed on two `#[expect(clippy::float_cmp)]` attributes being *unfulfilled*. The code was fine. CI ran a newer clippy than this machine's 1.98.1, that clippy no longer flags those comparisons, and an expectation that doesn't fire is an error under `-D warnings`.
- **Fix:** the workflow installs 1.98.1 rather than `stable`. Consistent with every dependency being pinned exactly: an update must not change what the gate accepts without someone choosing it.
- **Rejected:** a `rust-toolchain.toml`, which would pin developers too. Tried it, and rustup tried to install 1.98.1 as a second toolchain beside the identical `stable` one, failed on file conflicts, and left a broken toolchain directory behind. Pinning CI gets the determinism without touching anyone's machine.
- **Also rejected:** turning the two `expect`s into `allow`s. `expect` is how you find out an exception is no longer needed; silencing it forever to survive a toolchain difference is the wrong trade.
- **Bumping the version** means editing the workflow, running `./scripts/check.sh`, and fixing whatever the newer clippy sees.

## M10-6 · Resident memory is reported on CI, judged here

- **What happened:** the memory soak failed on CI with +1988 KB over its second half, against +12 to +16 KB on this machine, every time.
- **It is not a leak.** The run sat at 33544 KB at rounds 100, 200 *and* 300, then stepped up once at the end. A leak grows every round. How many arenas the allocator keeps depends on the machine's cores and its libc, and a 4-core shared runner is not this 12-thread desktop.
- **The fix is about what the number means.** Resident memory is the operating system's figure, not the game's. What the game actually promises — no entity, mesh or material left behind — is checked deterministically by `twenty_retries_failures_and_reloads_leave_no_trace`, which runs everywhere and passed on CI. The soak now prints its numbers on CI and asserts only off it.
- **Rejected:** loosening the threshold to a few megabytes. That keeps the shape of a check while removing its ability to see anything.
- **Also fixed:** `cache-on-failure: true`, so a red build no longer throws away the compile cache and makes the next push rebuild everything from cold.

## M10-7 · The three gaps a Rust reviewer would name

- **A licence.** The game had none, so anyone it was sent to had no terms at all. Now MIT *or* Apache-2.0, the usual Rust pairing and what nearly every crate here uses: the MIT half is short and permissive, the Apache half grants patent rights explicitly. `LICENSE-APACHE` is the canonical text, copied byte for byte from a vendored crate rather than retyped. The copyright line says "the Stardrop authors"; put a name there if you would rather.
- **An MSRV that meant something.** `rust-version = "1.95"` was a claim nothing tested, while CI built with 1.98.1. A job now runs `cargo check --all-targets` on 1.95.0 — a check rather than a build, because it only has to catch our own code reaching for something newer.
- **Supply chain.** `rustsec/audit-check` now runs against `Cargo.lock` on every push. Licences are a separate question from vulnerabilities, so `docs/THIRD-PARTY.md` lists all 393 crates compiled into the binary and what each is offered under, regenerated by `./scripts/third-party.sh`. Nothing is unstated; 8 crates are MPL-2.0, which is file-level copyleft and fine for shipping a binary we don't modify.
- **Left alone on purpose:** `cargo-deny` would check licences against a policy in CI, but its config schema changes between major versions and there is no way to validate it here before pushing. The generated list is honest about what is in the binary, which is the part that matters for shipping.

## M10-8 · The audit needed no token after all

- **What failed.** `rustsec/audit-check` scanned cleanly, found no vulnerabilities, and then died: `Resource not accessible by integration`. The action reports by creating a GitHub check run, which needs a `checks: write` token. A repository hands out a read-only one by default, and a fork's pull request gets a read-only one no matter what the workflow asks for.
- **Fixed by dropping the action.** The job installs `cargo-audit` and runs it. No token, no permissions to grant, the output lands in the job log, and the job's own red or green is the report. `rust-cache` keeps `~/.cargo/bin`, so the tool compiles once.
- **Warnings now fail**, with one named exception. `cargo audit` exits 0 on an unmaintained or yanked crate, so those scroll past unread; `--deny warnings` makes them count. `ttf-parser` is unmaintained with no replacement upstream and reaches us through winit's Wayland window decorations, so it is ignored by advisory id rather than by switching the whole class off.
- **One yanked crate removed:** `yoke-derive` 0.8.3 → 0.8.4, a lock-file line.

## M10-9 · Master carries the game, `paperwork` carries the paperwork

- **Why.** Stardrop is a pet project that is not going public: the point is a binary for each desktop to show friends. Licence files, a 393-crate licence inventory and an advisory job are the cost of publishing, not of playing, so they left master.
- **The branch.** `paperwork` sits at 60278bf, the project with all of it still in place and working: `LICENSE-MIT`, `LICENSE-APACHE`, `CREDITS.md`, `TECH_DEBT.md`, `docs/THIRD-PARTY.md`, `scripts/third-party.{sh,py}`, and the `advisories` and `oldest-supported-rust` CI jobs. Nothing was deleted, only moved, so going public later means merging a branch rather than redoing the work.
- **Master keeps the brief, this log and the level format.** They explain why the code looks like it does, which is still worth having in a year.
- **CI on master.** The Linux gate still runs on every push: format, clippy, tests, a release build and the Section 7.1 budgets. The three desktop binaries build only when asked for, through a `workflow_dispatch` input, because fat LTO on three runners is 20 minutes nobody needs on an ordinary push. Windows and macOS are compiled nowhere else, so that job runs the tests before it builds.
- **Two shortcuts that were in `TECH_DEBT.md` and are still open**, recorded here so they are not lost with the file: the long-session memory measurement is headless and judged only off CI (M10-6), and native Wayland is built in but has never run on a Wayland desktop (M0-2).

## M10-10 · No counters at all, and notes you can see

- **What was wrong.** The row along the top was the phrase: one mark per object the level asks you to touch. It was drawn with the same five-pointed star as a star you earn, and stardust, the thing you actually pick up, had no counter at all while you fell. Every level holds 3 motes and asks for a phrase of 2 to 5, so on most levels the row looked like a stardust counter that lit at the wrong moments. A playtest read it exactly that way.
- **Tried first:** notes for the phrase instead of stars, so a star on screen only ever meant a star earned, plus a row of round marks for the stardust in the top corner. Playing it settled the question the other way. Both rows came out. A counter is a thing to watch instead of the fall, and this game is one screen you read before you take it; the sky is better empty.
- **What stayed is the feedback itself.** Nothing static, nothing to track: the melody in your ear, and a note you can see for every touch.
- **Note signs.** A quaver rises from the astronaut on every touch and every mote, in the object's own colour, and the pitch it stands for decides how high it starts and how small it is. The sound was already a melody (M10-1); nothing on screen said so, so it read as sound effects.
- **Drawn, not typed.** Bevy's built-in font has no musical glyphs, so the note is generated the same way the star is: signed distances for a tilted head, a stem and a flag, white with the shape in the alpha. `the_picture_is_a_note` probes six places in the image, which is what stops a change to the maths quietly producing a blob.
- **Pooled, like the sparks.** Twelve sign entities spawned once and reused (`src/note_signs.rs`), so no amount of touching grows the world. They are taken down on a retry and on leaving the fall.
- **Two melody counters, on purpose.** `audio.rs` counts touches to pick the note to play, and `note_signs.rs` counts them to pick the note to draw. Neither can read the other's count without an ordering rule between two plugins that are otherwise independent, and both reset on the same message. `steps_match_what_audio_plays` holds them to the same note, so the invariant is checked rather than hoped for.

## M10-11 · Handing the game to someone

- **The executable bit survives, which I got wrong first.** The Windows artifact comes back `-rw-rw-rw-`, and I read that as the artifact zip storing no permissions. It stores them: the Linux artifact is `-rwxr-xr-x` in the zip and runs straight out of it. Windows files simply have no executable bit to carry. So Linux ships the plain binary, and the tar is kept only for macOS, where the thing being shipped is a signed bundle rather than one file.
- **macOS gets a real `.app`** (`scripts/macos-app.sh`), tarred inside the artifact: the binary, the smallest `Info.plist` that counts as a bundle, and an ad-hoc signature. The signature is what lets Apple Silicon load it at all. It is *not* a developer identity, so Gatekeeper still objects to anything downloaded, and the answer is the one-time "right-click, Open" the README now explains. Notarising for real costs $99 a year, which is not worth it for a game shown to friends.
- **Left undone:** the macOS build is Apple Silicon only, because that is what `macos-latest` runners are. A universal binary means a second fat-LTO compile on that runner, and nobody here has an Intel Mac to need it.
- **The Linux build was reported silent, and is not.** Running the downloaded artifact here, it opens the device with no warning and registers a PipeWire stream, unmuted at full volume. PipeWire remembers an output per application name, so a stream pointed once at an inaudible sink goes there every time; `pactl list sink-inputs` is what to look at, not the build.
- **Also fixed:** the smoke test now fails on `Failed to setup audio`. `bevy_kira_audio` only *warns* when it cannot open the device and then plays nothing, so a build that starts stone silent used to pass every gate. Checked by feeding the check a log with that line and watching it go red.

## M10-12 · Releases, because an artifact is only for the author

- **The problem with what we had.** A build from the `check` workflow is an Actions artifact: downloading one needs a GitHub account with read access to the repository, and it is deleted after 90 days. Nobody the game is meant for could ever fetch one. A release asset needs no account, no login and never expires.
- **Tag to publish.** `git tag v0.1.0 && git push origin v0.1.0` builds all three desktops and publishes them with notes that carry the first-run instructions (`.github/workflows/release.yml`). The `check` workflow keeps its `binaries` button for trying a build without tagging anything.
- **The tag must match `version` in `Cargo.toml`**, checked before anything is built. Otherwise the file someone downloads says one thing and the page they got it from says another.
- **Tests run on each platform before its build is published.** Releases are rare and the minutes are cheap; handing someone a binary whose tests never ran on its own operating system is not.
- **Nothing is published unless all three built**, because `publish` waits on the whole matrix. A release with two of the three files in it is worse than no release.
- **Archived except on Windows.** A browser download of a bare binary loses the executable bit, so Linux and macOS ship a tar; Windows has no such bit and gets the `.exe` straight, which is one less step for the person running it.
- **The repository has to be public** for any of this to reach anyone. That is the price of the whole approach, and the alternative (itch.io, free, keeps the repository private, and its app installs builds so macOS never flags them) is written down here in case the price turns out to be the wrong one.

## M10-13 · The landing cartoon

- **What.** Finishing every level plays an 8.5-second wordless scene (Section 11): the astronaut comes down on a generated planet, the dust settles, a pack far larger than them comes off their back, and a very small flag goes into the ground. `src/ending/` is one plugin in three files — the generator (`planet.rs`), the meshes (`art.rs`) and the plugin with the timeline (`mod.rs`) — plus a new `AppState::Ending`.
- **Why generate the planet instead of drawing one.** Fourteen hand-made levels earn a surprise at the end, and a surprise the player has seen once is not one. The generator is a fifth of the work of an illustration and gives a different world every watch: four ground kinds (rock, desert, water, overgrowth) across one or two spans, six props placed by a weight table per ground, and a palette of one hue with its opposite for everything growing.
- **The two ways a generated planet can be unplayable**, both closed by rules rather than by retrying: a planet of nothing but water (a single span is never water, and a water span is an edge span of at most 40% of the width), and a planet with nowhere flat (the landing spot is the flattest dry sample near the middle, and falls back to the flattest dry sample anywhere). 2,000 seeds are checked for both in the tests, along with the heights staying inside the playfield and no prop standing on the landing spot.
- **Every motion is a pure function of elapsed seconds** (`beat_at`, `descent_y`, `squash`, `pack_scale`, `flag_at`, `flag_wave`), tested without an `App`, and the systems only apply them. The same reason the physics is written this way: the shape of a motion is worth arguing about in a test, not in a window.
- **Drawn in front of the game's own background.** `background.rs` paints the sky at z -900 and the scenery panels beside the playfield at -540; the cartoon starts at -520, so the menu theme's stars cannot float over a daylit planet. The ground reaches 5,000 units either side and 2,000 below the playfield, because the camera shows more scenery the wider or taller the window is.
- **Nothing new in the mixer.** Each beat is a message, and `audio.rs` answers it with a voice that already exists: `Whoosh` and the beacon's own landing chord at touchdown, `Muffled` for the pack, `Shimmer` for the flag, `Marimba` as it goes in. The descent and the hold are silent on purpose.
- **A fresh seed every watch**, not one saved per player. A planet that is always the same is a place; a different one each time is a reason to press it again. The main menu grows a **The landing** row once all fourteen levels are done.
- **What it cost.** The `dist` binary goes from 47.91 MB to 48.08 MB (+167 KB). The scene is 16 to 30 entities, all geometry built once on entering the state and despawned on leaving it, with the dust, steam and sparkles coming from the existing spark pool, so nothing here can grow the world. Frame cost is unmeasured: the busiest-level budget test (0.112 ms of 1 ms) does not cover this screen.
- **What a fresh review caught**, after every test passed: entering the cartoon did not free the level, so the beacon, every object and the astronaut who had just landed drew straight over the planet. The level is now freed on entering `Ending` (`free_the_level_on`, `src/level/mod.rs`), which also hides the old astronaut, restores the menu theme and brings back the menu pad. Four smaller things went with it: the pack and the pole took their height from the ground under the astronaut rather than the ground under themselves (up to 266 units out, a pack and a half in the air), the 0.15 slope rule was not actually implemented, the rewatch row was missing from the first main menu after launch because the level list is an asset that arrives later, and the astronaut stood still from the landing to the end.
- **Watching it without finishing the game.** F3 in a `--features dev` build opens the cartoon from any screen, the way F1 and F2 open the other debug tools. Fourteen levels is too long a wait to see one change to the art, and the alternative was editing the save file. Nothing in a shipped build is bound to it.
- **What the first watch changed.** Two things that no test could have told me: the flag was too small and too dull to be the point of the scene, and the pack read as a plain box. The pole went from 2 r to 3.2 r and turned nearly white (a planet's own colours can leave a coloured pole darker than the ground it stands in: on seed 3 it was), the cloth grew to 1.4 r, took a notch in its far edge and a fold along its bottom, and the pole gained a ball on top. The pack grew to 4.6 by 4.2 r and gained a handle, two feet, an aerial, a seam and a latched hatch, with both lamps moved to one side because symmetrical ones read as eyes. They also swapped sides: the pack now lands to the astronaut's left and the flag goes in to their right, so neither stands in front of the other. That costs 65 units of landing search (400 to 335), and the planets that land on a slope went from 3 in a thousand to 5.
- **Left out.** No camera move (the playfield is fixed, Section 5), no new synth voices, no universal seed saved per player, and no water ripple at the touchdown: the water is one static mesh, and rippling it means rebuilding geometry every frame, which Section 7.2 forbids. No golden-image check either: the art is pinned by mesh tests (the ground reaches past both window edges, every prop has faces, the pack towers over the flag, the flag outshines the ground it stands in).

## Measurements (M8)

Measured 2026-09-24, same machine as the M0 baseline.

| Metric | Value | Notes |
|---|---|---|
| One altitude's music, built | **74 ms** | Off the main thread (M8-4). `--ignored bed_cost` |
| One altitude's music, held | **5.4 MB** | Four 8-second loops, stereo f32 in the mixer. Freed when the level changes. |
| Every one-shot sound | **7.65 MB** | Fifteen sounds, built once at startup, kept for the whole session. Was 2.78 MB before the reverb tails (M8-5). |
| Our own systems, per frame | 0.100 ms | Unchanged by audio (the mixer runs on its own thread). Budget: 1 ms. |
| Binary | **48.2 MB** | Was 46.7 MB before the audio crate. |
| Memory over 400 rounds | +40 KB, then flat | Unchanged by audio. |

---

## Baseline measurements (M0)

Measured 2026-09-18.
- **Machine:** 12 threads, 30 GB RAM, NVIDIA RTX 5060 (Vulkan, driver 595.91), X11 at scale 1.25, 60 Hz.
- **Build:** Rust 1.98.1, clang + mold linker.

| Metric | Value | Notes |
|---|---|---|
| Release binary (stripped, fat LTO) | **39.7 MB** | Mostly wgpu, naga and Bevy reflection data. Thin LTO: 43.0 MB. |
| Memory at the empty screen (RSS) | **~410 MB** | About 230 MB of it is shared (GPU driver mappings); ours is a small fraction. Watch it for growth, not absolute size. |
| Idle CPU, low-power mode | **~0.5 %** | Only wakes on input or once a second. |
| CPU with F1 open (continuous, vsync 60) | **~12 %** of one core | Bevy's render loop for an empty scene. |
| Frame time with F1 open | 16.7 ms avg, 20.2 ms worst | Vsync-bound; our own systems take a negligible share. |
| Startup to first rendered frame | **0.35–0.7 s** | Three runs. |
| Clean build, dev | 182 s | Separate target folder. |
| Clean build, release | 256 s | |
| Incremental rebuild, dev (touch `lib.rs`) | 3.2 s | |
| Incremental rebuild, dev with `--features dev` | **1.5 s** | Dynamic linking. |
| Incremental rebuild, release (fat LTO) | 172 s | Thin LTO: 34 s. `release` now uses thin LTO (M0-10). |
| Benchmark `summarize 240 frame times` | 304 ns | criterion, release. |

## Measurements (M1)

Release build, same machine as M0, 60 Hz with vsync.

| Metric | Value | Notes |
|---|---|---|
| Our systems per frame | **~10–15 µs** | Chrome trace; each system averages 0.5–1.3 µs. Budget: 1 ms. |
| Worst single system call | 28 µs | `scroll_parallax`, a rare outlier. |
| One-time parallax spawn | 1.4 ms | Startup only; 480 dot entities. |
| `movement step` benchmark | 3.0 ns | criterion |
| CPU while falling | ~16–17 % of one core | Continuous rendering at 60 FPS. |
| RSS | 341 MB at start → 343.2 MB after 5 min | Still creeping slightly; see TECH_DEBT.md. |
| Frame time | 16.7 ms avg, worst ~18–21 ms | Vsync-bound. |

## Measurements (M2)

| Metric | Value | Notes |
|---|---|---|
| Parse + validate test level | 3.7 µs | criterion. Budget for load + retry: 200 ms. |
| Retry | Moves one entity, no loading or spawning | Constant time; effectively instant. |
| Movement step | 6.7 ns | Was 3.0 ns before the wall bounce and drift logic. |
| Respawn soak, 5 min, about 30 respawns | RSS 429 → 440 MB (peak at 1 min) → 433.8 → 434.1 MB (min 2–4.5) | Settles after warm-up. No warnings or errors. |
| CPU while falling | ~15–16 % of one core | Release build, 60 FPS. |

## Measurements (M3)

| Metric | Value | Notes |
|---|---|---|
| Cloud puff effect | 19 ns | `on_enter` |
| Crystal effect | 1.2 ns | `on_enter` |
| Wind ribbon step (16 points) | 72 ns | `while_inside` |
| Contact check, plain loop | 171 ns (12 objects) / 560 ns (40) | Per physics step; see M3-4. |
| Parse + validate test level | 6.7 µs | Was 3.7 µs; the level now has objects. Budget: 200 ms. |
| Movement step | 6.7 ns | Unchanged. |

## Measurements (M4)

Benchmarks now use the game's real `GameConfig` (no constant folding), so these
replace the M3 numbers rather than comparing directly.

| Metric | Value | Notes |
|---|---|---|
| Movement step | 8.4 ns | Was 6.7 ns before timed effects. |
| Cloud puff / crystal effect | 50 ns / 11 ns | Now matched on the placed object. |
| Wind ribbon step (16 points) | 66 ns | |
| Contact check, plain loop | 79 ns (12 objects) / 261 ns (40) | Two per frame. |
| Contact check, grid | 86 ns / 229 ns | Now level with the loop: the loop stays (M3-4). |
| Parse + validate test level | 10.0 µs | Nine object types in it. Budget: 200 ms. |
