# 0011. Streamed games render at the client's resolution

- Status: Accepted
- Date: 2026-09-24

## Context

A streamed game is launched directly ([0007](0007-streamed-games-skip-gamescope.md)),
so nothing scales it: it renders at whatever resolution it saved. HD2 kept
a hand-set 1440x900 and stretched it into the 1728x1080 window streamed to a
Mac, which looked blurry. Set to 1728x1080 by hand, it would then squash that
into a Deck's 1280x800. A saved resolution suits at most one client, and the
library is over 1000 games, most not installed, so per-game setup does not
scale.

Steam gives the client's size to the launch as
`SteamStreamingMaximumResolution` (see 0007).

## Decision

For a streamed direct launch, `src/shim/resolution.rs`:

1. **Recognises the engine** from the files beside the game's binary and
   appends that engine's resolution arguments, which override the saved
   setting for the run:

   | Engine | Recognised by | Arguments |
   |---|---|---|
   | Unity | `<name>_Data/` beside `<name>` | `-screen-width W -screen-height H` |
   | Unreal | `/Binaries/Win64/` in the path, or `Engine/` beside the stub | `-ResX=W -ResY=H` |
   | Godot | `<name>.pck` beside `<name>` | `--resolution WxH` |
   | Source | a directory with `gameinfo.txt` beside the binary | `-w W -h H` |

   Of the games installed on 2026-09-24, about 46 were Unity and 14 Unreal,
   and the detection works for native Linux builds (`name.x86_64`) as well as
   Windows ones.

   Unity saves the size it ran at on exit (`Screenmanager Resolution
   Width/Height`): a Windows build into the prefix's `user.reg`, a native one
   into `~/.config/unity3d/<company>/<product>/prefs`, named by
   `<name>_Data/app.info`. Unreal (`GameUserSettings.ini`, under the project
   directory, which for a root stub is the sibling holding `Binaries/Win64`:
   `Palworld.exe` beside `Pal/`) and Source (`cfg/video.txt`) may do the same.
   The shim records those values before the launch and puts them back after
   it, which changes nothing when the engine left them alone. The registry's
   go back at the game's next launch rather than at exit: wineserver outlives
   the game and writes the registry out after it, over an earlier restore.
   Without an app id there is no next launch to defer to, so those are left
   and the shim log says so.

2. **Applies per-game rules** (`stream_resolution_rules`) for engines with no
   such argument: a file, a regular expression and a replacement using
   `{width}` and `{height}`. The rule rewrites the setting before launch and,
   after the game exits, puts back the values it replaced, match by match, in
   the file as it is then. Anything else the player changed during the run is
   kept.

Local launches are untouched. `stream.set_resolution = false`, globally or
per game as `stream_set_resolution`, turns it off.

## Alternatives rejected

- **gamescope scaling.** Takes the game off Steam's X display, which is what
  0007 exists to avoid.
- **A Wine virtual desktop at the client's size.** Many games still use their
  saved resolution inside it.
- **Per-game rules only.** Would need a rule for every game streamed. Engine
  arguments cover the common engines with none.
- **Restoring a whole settings file.** Would discard settings the player
  changed mid-game.

## Consequences

- A game on an unrecognised engine without a rule still renders at its saved
  resolution. The decision line in the shim log shows whether an engine or
  rule applied.
- Every rewrite and record is journalled in
  `$XDG_STATE_HOME/steam-command-runner/resolution-<appid>.json` until it is
  restored. A shim killed with the game leaves it behind, and the game's next
  launch restores from it before anything else, even the `pre_launch` hook,
  so neither that launch nor a hook backing up settings sees the client's
  size as the player's own.
- Values go back by position. If the file holds a different number of them
  than was recorded (an engine wrote a new key during the run), nothing is
  restored and the shim log says so, rather than put values on the wrong
  settings.
- Whether Unity, Unreal and Source games actually save the overriding size
  has not been observed here; the record and restore are harmless if they
  do not.
- Only launches. A client reconnecting to a game already running at another
  size does not change it.

## Evidence

- Tests in `src/shim/resolution.rs`: detection for each engine, arguments,
  rewrite and restore keeping other changes, a pattern matching nothing,
  restore after a killed launch, Unity's deferred restore, a changed count.
- `user.reg` of an installed Unity game (2321470) held
  `Screenmanager Resolution Height=0x2d0` (720), not the native 1080: Unity
  keeps the size it last ran at.
- HD2's `user_settings.config` recipe in nix-config
  (`home/machines/ali-desktop/default.nix`).

## Revisit when

- An engine's argument is ignored by a game: add a rule for that game.
- Steam starts scaling streamed games itself.
