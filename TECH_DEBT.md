# Tech debt

Known shortcuts, and when we plan to deal with them.

| Item | Why it's there | Plan |
|---|---|---|
| Power mode is owned by the F1 overlay (`debug` module). | Milestone 0 has only one screen, and it never animates. | Once app states exist (M2/M5), a single system chooses the update mode from the state (continuous while playing, low-power in menus). The overlay then only forces continuous mode. |
| Wayland support is disabled. | It needs the `libwayland-dev` / `libxkbcommon-dev` system packages. | Re-enable the `wayland` feature once they're installed (decide by M9). |
| The test circle in `background.rs` is a placeholder. | Needed to check that bloom works. | Remove it in Milestone 1 once the astronaut exists. |
