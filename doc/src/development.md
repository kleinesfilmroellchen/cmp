# Development info

This is a quick-reference page for anyone interested in testing and developing cmp right now. Much of this information will be obsoleted.

## Asset preprocessing

CMP uses libresprite for automatically exporting .ase files on build. This is optional, as the QOI image source files are also committed to the repository. Simply install the latest release of libresprite for this step to work.

## Paths and files

CMP uses system paths for storing settings and save files. Settings are saved in the system-dependent config directory for an app. Save files are saved in the system-dependent data directory for an app. The logs contain information on where that path is exactly.

CMP save files are a serialization of a relevant part of the game world that has been compressed with [brotli](https://datatracker.ietf.org/doc/html/rfc7932) to save space.

## Settings and arguments

Settings are stored in a game-settings.toml file. Some settings can currently only be changed there. Refer to the `config::GameSettings` struct for a full list, but important settings only accessible here are:

- `show_fps`: Shows the FPS UI in the top left of the screen.
- `show_debug`: Shows various graphical debug components (area indices, navmesh components, pathfinding debugging, etc.)

Command-line arguments are:

- `--version`: Show CMP version
- `--settings-file`: Use an alternative settings file (very useful for testing combinations of settings)

## Controls

- Click & Drag: Move camera
- Scroll: Zoom camera in and out
- Click on objects: Bring up world info UI for the clicked-on object.
- `Escape`: Close world info UI, or stop any in-progress action (such as building)

## Dev keybinds

- `Ctrl-V`: Toggle V-sync.
- `Ctrl-S`: Save to a default save slot.
