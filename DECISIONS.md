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

## M0-6 · Build profiles

| Setting | Why |
|---|---|
| `[profile.dev] opt-level = 1` | Our code gets light optimization but still compiles fast. |
| `[profile.dev.package."*"] opt-level = 3` | Bevy is fully optimized once and then cached. A debug build of Bevy is unplayably slow. |
| `--features dev` → `bevy/dynamic_linking` | Bevy becomes a shared library, so relinking after a change takes a fraction of the time. Development only; never ship it. |
| Linker: clang + mold (user's `~/.cargo/config`) | Much faster linking than GNU ld. |
| release `lto = "fat"` | Optimizes across crate boundaries: faster and smaller binary, but slower to build. |
| release `codegen-units = 1` | LLVM sees the whole crate at once, so it optimizes better. Slower build. |
| release `strip = true` | Removes debug symbols; much smaller binary. |
| release `panic = "abort"` | No unwinding tables: smaller and slightly faster. A panic ends the process straight away. That's fine because game logic must not panic, and save files will be written atomically (M5). |
| `profile.profiling` | Release plus line-level debug info, so profilers can show function names. |

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
| Incremental rebuild, release (fat LTO) | 172 s | Thin LTO: 34 s. |
| Benchmark `summarize 240 frame times` | 304 ns | criterion, release. |
