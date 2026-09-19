# Tech debt

Known shortcuts, and when we plan to deal with them.

| Item | Why it's there | Plan |
|---|---|---|
| The game always renders continuously (Bevy's `game()` update mode), including when paused or in menus. | Milestone 1 has only the falling screen. | M5: menus, pause and level-complete screens switch to low-power updates. |
| Wayland support is disabled. | It needs the `libwayland-dev` / `libxkbcommon-dev` system packages. | Re-enable the `wayland` feature once they're installed (decide by M9). |
| The game always loads `levels/test.level.ron`. | Level select arrives in M5. | M5 level select chooses the file. |
| Window contents lag ~2 refreshes behind a live resize, so the view "jumps". | Presentation latency from vsync (DECISIONS.md M0-11). | Revisit with the M5 window/fullscreen settings. |
| The window opens landscape (1280×720), so the portrait playfield uses only its middle third. | The default window was chosen before the portrait redesign. | M5 settings: choose a sensible default size (e.g. a taller window) and remember it. |
| RSS still creeps slightly: about +300 KB between minutes 2 and 4.5 of a respawn loop (~30 respawns), and slowing. Our entities, meshes and materials are proven constant (`soak_tests.rs`). | Likely allocator/driver caches; not our objects. | Re-measure in a 30-min soak at M7 (more assets), with the `dist` build. |
| Running the binary directly looks for `assets/` next to the executable (Bevy default). `cargo run` sets the right folder; otherwise use `BEVY_ASSET_ROOT=<project dir>`. | Normal during development. | M9: the release package puts `assets/` next to the binary. |
