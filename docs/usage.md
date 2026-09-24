# Steam Command Runner - Usage Guide

**Steam Command Runner** is a versatility tool for Linux gaming, designed to wrap game commands, manage per-game configurations (especially for Gamescope), and simplify Steam launch options.

## Core Capabilities

1.  **Command Wrapper**: Acts as a launcher that can inject environment variables and arguments.
2.  **Gamescope Integration**:
    *   **Proactive**: Generate launch arguments for Steam.
    *   **Transparent Shim**: Masquerade as the `gamescope` binary to automatically apply per-game configuration without changing Steam launch options for every game.
3.  **Launch Option Management**: programmatic control over Steam's `localconfig.vdf` to set launch options for all games or specific ones.
4.  **Configuration**: Hierarchical configuration (Global -> Per-Game).

## Installation

### Via Nix (Recommended)
```bash
nix profile install github:alisonjenkins/steam-command-runner
```

### Via Cargo
```bash
cargo install --path .
```

## Basic Usage

The binary is `steam-command-runner`.

### Running Games
Run a command with the tool's wrappers applied.
```bash
steam-command-runner run --app-id 12345 -- /path/to/game
```

### Searching Games
Find the App ID for a game.
```bash
steam-command-runner search "Cyberpunk"
# Output:
# 1091500: Cyberpunk 2077
```

## Gamescope Integration

### Method 1: The Shim (Recommended)
This is the most powerful feature. It allows you to configure gamescope arguments centrally without editing Steam launch options for every game.

**Why use this?**
-   **Cleaner Steam setup**: You use the standard `gamescope %command%` (or absolute path) for everything.
-   **Zero overhead**: The tool replaces itself with the real `gamescope`, so there's no extra process running during your game.
-   **Compatibility**: Avoiding nested "runner" commands ensures Steam Input, Overlay, and Stop functions work as expected.

1.  **Install the Shim**:
    ```bash
    steam-command-runner install
    # Creates ~/.local/bin/gamescope -> steam-command-runner
    ```
2.  **Set Steam Launch Option**:
    Use the standard gamescope launch option.
    ```
    /home/user/.local/bin/gamescope -- %command%
    ```
3.  **Configure Per-Game**:
    Use the config command to set specific arguments for a game.
    ```bash
    # Enable gamescope and set arguments for specific game (e.g., 1080p, 144Hz)
    steam-command-runner config edit --app-id 1091500
    ```

### Remote Play streaming

While a Remote Play client is streaming, the shim runs the game directly instead of
inside gamescope. It knows a stream is active when it can read and parse the stream
target the host publishes, `$XDG_RUNTIME_DIR/stream-mode/target.json` by default or the
path in `STEAM_COMMAND_RUNNER_STREAM_TARGET`. A missing, unreadable or malformed target
means "not streaming": the game launches under gamescope as usual.

Steam only streams a game in game mode, where the client captures the mouse and sends
relative motion, when the game window is on Steam's own X display. gamescope moves the
window to a nested display, so Steam streams the desktop instead and camera look stops at
the edge of the client window.

The direct launch keeps `pre_command`, `[env]`, `[inner_env]`, `game_args` and the hooks.
It also keeps only the Steam overlay build that matches the game binary. Steam binds game
capture to the first process whose overlay registers the game window, and a launcher or
anti-cheat helper of the other bitness can win that race, exit, and freeze the stream.

```toml
# Global config
[stream]
bypass_gamescope = true   # default
overlay = "auto"          # auto | both | x86_64 | i386

# Per-game config (games/<appid>.toml)
stream_bypass_gamescope = false   # keep gamescope for this game while streaming
stream_overlay = "both"           # keep both overlay builds for this game
```

A streamed game also renders at the client's resolution. Unity, Unreal, Godot
and Source games are recognised from their files and get the engine's
resolution arguments; the resolution such an engine had saved is put back
afterwards, in case it saves the streamed size on exit. For other engines,
add a rule that rewrites the game's own setting for the length of the run;
the replaced values are put back when the game exits, or at its next launch
if the shim was killed:

```toml
# games/553850.toml (Helldivers 2)
[[stream_resolution_rules]]
file = "{prefix}/drive_c/users/steamuser/AppData/Roaming/Arrowhead/Helldivers2/user_settings.config"
pattern = '(?m)^(\s*(?:screen|render)_resolution = \[\s*)\d+(\s+)\d+'
replacement = '${1}{width}${2}{height}'
```

`{prefix}` is the Proton prefix and `{game_dir}` the game binary's directory.
Refer to groups as `$1` or `${1}`. A replacement the pattern could not find
again is refused, since it could not be put back.
`stream_set_resolution = false` turns it off for one game, and
`[stream] set_resolution = false` for all.

Every launch through the shim writes one line to `~/.steam-command-runner-shim.log`
saying which way it went, whether or not `shim_debug` is on:

```
2026-09-24T08:40:18Z app 553850: streaming to steam, launching without gamescope, overlay X86_64 only (from .../game.exe)
2026-09-24T08:36:22Z app 553850: no stream target, launching through gamescope
```

Steam throws away a launched game's stderr, so this file is the place to look,
not the journal.

Setting `gamescope_enabled = false` for a game also launches it directly, stream or not.

### Method 2: Launch Option Generator (Legacy/Alternative)
You *can* use `steam-command-runner` to generate arguments directly in the launch option string, but this is **not recommended** for general use because it makes launch options messy and harder to maintain.

```bash
gamescope $(steam-command-runner gamescope args) -- %command%
```
*Downside: You must update this string manually if you change how you want arguments generated, and it relies on shell expansion which can be brittle in some Steam environments.*

## Configuration Management

Configuration is stored in `~/.config/steam-command-runner/`.

-   **Global Config**: Applies to all games.
-   **Per-Game Config**: Overrides global settings for a specific App ID.

### Environment variables: `[env]` vs `[inner_env]`

Both tables set environment variables, but at different points in the launch chain:

-   **`[env]`**: set on the process the runner execs. When gamescope is in play that
    process *is* gamescope, so gamescope inherits these too. Use it for variables the
    compositor needs (or doesn't mind).
-   **`[inner_env]`**: emitted as `KEY=VALUE` assignments to the `env` wrapper on the
    inner command, past gamescope's `--`. Gamescope never sees them. Without gamescope
    they are set directly on the game process, so behaviour is the same either way.

Put `MANGOHUD`, `MANGOHUD_CONFIG` and `ENABLE_VKBASALT` in `[inner_env]`. Those load
implicit Vulkan layers, and in `[env]` the layer is loaded into gamescope's own Vulkan
instance: the HUD you see still comes from the game, and gamescope segfaults at exit in
`CVulkanDevice::~CVulkanDevice` when MangoHud's overlay data has already been torn down.
The shim prints a warning if it finds one of them in `[env]`.

```toml
# Global config
[env]
DXVK_ASYNC = "1"        # gamescope may inherit this harmlessly

[inner_env]
MANGOHUD = "1"          # game only — never gamescope
```

### Commands

-   **Show Config**: `steam-command-runner config show [--app-id <ID>]`
-   **Edit Config**: `steam-command-runner config edit [--app-id <ID>]`
-   **Path**: `steam-command-runner config path`

## Launch Options Management

You can bulk-manage Steam launch options to apply standard fixes or tools.

-   **Set Single**: `steam-command-runner launch-options set --app-id 12345 --options "gamemoderun %command%"`
-   **Set All**: `steam-command-runner launch-options set-all` (Applying a default template)
-   **Clear All**: `steam-command-runner launch-options clear-all`

## Troubleshooting

### Shim Not Working (PATH Issues)
If you set the launch option to `gamescope %command%` but the runner config isn't applying (e.g., arguments missing), Steam might be using the system `gamescope` instead of the shim in `~/.local/bin`.

This happens if `~/.local/bin` is not in Steam's `PATH`.

**Solution**: Use the absolute path to the shim **AND** include `--` to separate the gamescope arguments from the command. This is critical for compatibility with Steam's wrappers.

```bash
/home/YOUR_USER/.local/bin/gamescope -- %command%
```
*(Replace `YOUR_USER` with your actual username)*