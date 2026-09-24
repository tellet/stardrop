# Tech debt

Known shortcuts, and when we plan to deal with them.

| Item | Why it's there | Plan |
|---|---|---|
| Wayland support is disabled. | It needs the `libwayland-dev` / `libxkbcommon-dev` system packages. | Re-enable the `wayland` feature once they're installed (decide by M9). |
| Window contents lag ~2 refreshes behind a live resize, so the view "jumps". | Presentation latency from vsync (DECISIONS.md M0-11). | Live with it, or revisit at M9. |
| The long-session memory measurement is headless: it exercises our own allocations, not the GPU driver's. 400 play/fail/retry/reload rounds move RSS by +44 KB and then flatten (DECISIONS.md M7). | A windowed 30-minute soak takes the machine over for half an hour. | Run one on the `dist` build at M9, when the game also has audio to soak. |
| A glowing object doesn't cast coloured light on the astronaut passing it. | Section 5 asks for it "if affordable"; it needs a light pass we don't have. | Reconsider at M9 if there's room. |
| A `--features dev` build reads `assets/` from the working folder, so running that binary from elsewhere needs `BEVY_ASSET_ROOT=<project dir>`. Release builds carry their assets inside (M5-7). | Hot reload needs the folder. | Nothing to do: it only affects development. |
