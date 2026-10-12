# Tech debt

Known shortcuts, and when they get dealt with. Reviewed at Milestone 9:
each item below is either still open with a plan, or consciously accepted.

## Open

| Item | Why it's there | Plan |
|---|---|---|
| The long-session memory measurement is headless: it covers our own allocations, not the GPU driver's. 400 play/fail/retry/reload rounds move RSS by +40 KB and then flatten. | A windowed soak takes the machine over for half an hour. | Run a 30-minute windowed soak on the `dist` build when the machine is free, and record the result here. |

| Native Wayland is built in but has never run on a Wayland desktop. | This machine is X11 only (DECISIONS.md M0-2). | Start it once on a Wayland session and look. |

## Accepted

| Item | Why we live with it |
|---|---|
| Window contents lag about two refreshes behind a live resize, so the view "jumps" while dragging an edge. | Presentation latency from vsync (DECISIONS.md M0-11). Fixing it means presenting without vsync, which costs more than the problem. |
| A glowing object casts no coloured light on the astronaut passing it. | Section 5 asks for this "if affordable". It needs a light pass we don't have, and the halos plus bloom already bleed colour between things. Not worth a second render pass. |
| Object notes aren't nudged onto the pad's pulse. | Section 6.1 offers this as optional. It turns out there is nothing to quantise *to*: the pad is a drone with no beat in it, by design (M8-5). A pulse would have to be invented first. |
| The audio crate pulls in an ogg decoder (about 1.5 MB) although no sound is ever loaded from a file. | `bevy_kira_audio` does not compile with every decoder feature off — its prelude re-exports a type that one of them provides. Tried at M9. |
| A `--features dev` build reads `assets/` from the working folder, so running that binary from elsewhere needs `BEVY_ASSET_ROOT=<project dir>`. | Hot reload needs the folder. Shipped builds carry their assets inside (M5-7), so this only affects development. |
| Level 4 lands careless play 37 % of the time, against level 3's 26 %: the fourth level is easier than the third. | The measured curve is right everywhere else, and these are the teaching levels. Worth a look if playtesting agrees it reads as a dip. |
