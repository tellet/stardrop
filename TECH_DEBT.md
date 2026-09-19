# Tech debt

Known shortcuts, and when we plan to deal with them.

| Item | Why it's there | Plan |
|---|---|---|
| The game always renders continuously (Bevy's `game()` update mode), including when paused or in menus. | Milestone 1 has only the falling screen. | M5: menus, pause and level-complete screens switch to low-power updates. |
| Wayland support is disabled. | It needs the `libwayland-dev` / `libxkbcommon-dev` system packages. | Re-enable the `wayland` feature once they're installed (decide by M9). |
| The game always loads `levels/test.level.ron`. | Level select arrives in M5. | M5 level select chooses the file. |
| Window contents lag ~2 refreshes behind a live resize, so the view "jumps". | Presentation latency from vsync (DECISIONS.md M0-11). | Revisit with the M5 window/fullscreen settings. |
| RSS creeps up slowly during play: +640 KB from minute 1 to minute 5, and still slowing. Our entity count is constant. | Probably Bevy/driver caches warming up; not yet proven. | Re-measure in M2 with the entity-count overlay and the 20-retries test (Section 7.5). |
| Running the binary directly looks for `assets/` next to the executable (Bevy default). `cargo run` sets the right folder; otherwise use `BEVY_ASSET_ROOT=<project dir>`. | Normal during development. | M9: the release package puts `assets/` next to the binary. |
