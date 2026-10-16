# Playing

| | |
|---|---|
| Left / Right, A / D, or a gamepad stick | steer, and the first press starts the fall |
| R | start the attempt again |
| Esc | pause, and go back from any screen |
| Enter / Space | choose |
| J | the field journal, from a menu or a paused level |
| F1 | frame times |

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
