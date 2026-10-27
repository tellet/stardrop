# Playing

| | |
|---|---|
| Left / Right, A / D, or a gamepad stick | steer, and the first press starts the fall (no gamepad in a browser) |
| Hold the left or right half of the screen | the same, by thumb. Both halves at once cancel out, like both keys |
| R | start the attempt again |
| Esc | pause, and go back from any screen |
| Enter / Space | choose |
| J | the field journal, from a menu or a paused level |
| F1 | frame times |

On a touch screen, tap a menu row to choose it, **||** in a level's top right
corner opens the pause menu, where starting over and leaving live, and
**Back** in the top left corner does what Esc does. Both appear only once you
have touched the screen.

## The three stars

Each level offers three: reaching the beacon, collecting all its stardust in
one attempt, and finishing the level's phrase, which means touching the number
of objects the level asks for in that one attempt. Every object touched plays
the next note of the level's scale, so a finished phrase is something you hear
rather than something the screen has to tell you.

Completing a level unlocks the next one.

## Where progress lives

Progress and settings are saved to your OS data folder
(`~/.local/share/stardrop/` on Linux). The game never writes anywhere else.

In a browser they go into the browser's own storage for the site you are
playing on. They survive a reload and a closed tab, and clearing the site's
data erases them. Storage belongs to one site, so progress made on one
address is not there on another, and a private window keeps nothing at all.

## Running a build someone sent you

Linux just runs. Nothing is signed by Apple or Microsoft, so those two stop it
the first time. Each of these is a one-off.

| | |
|---|---|
| Linux | `tar -xzf stardrop-linux-*.tar.gz && ./stardrop` |
| macOS | `tar -xzf stardrop-macos-*.tar.gz`, then right-click `Stardrop.app` and choose **Open**, then **Open Anyway**. Or `xattr -dr com.apple.quarantine Stardrop.app` once. |
| Windows | **More info**, then **Run anyway**, when SmartScreen says it protected your PC |

The macOS build is for Apple Silicon and will not start on an Intel Mac.

Each build is one file with the whole game inside it. It runs from wherever
you put it and installs nothing.
