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