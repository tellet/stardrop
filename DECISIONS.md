# Decisions log

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
- **Revisit:** before release (M9). Native Wayland gives slightly better frame pacing and scaling on Wayland desktops.

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
