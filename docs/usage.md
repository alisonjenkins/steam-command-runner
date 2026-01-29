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

### Method 1: The Shim (Transparent)
This is the most powerful feature. It allows you to configure gamescope arguments centrally without editing Steam launch options for every game.

1.  **Install the Shim**:
    ```bash
    steam-command-runner install
    # Creates ~/.local/bin/gamescope -> steam-command-runner
    ```
2.  **Set Steam Launch Option**:
    Use the standard gamescope launch option in Steam. Because `~/.local/bin` is usually early in `PATH`, Steam will call the shim instead of system gamescope.
    ```
    gamescope %command%
    ```
3.  **Configure Per-Game**:
    Use the config command to set specific arguments for a game.
    ```bash
    # Enable gamescope and set arguments for specific game (e.g., 1080p, 144Hz)
    steam-command-runner config edit --app-id 1091500
    ```
    *Add to the config file:*
    ```toml
    gamescope_enabled = true
    gamescope_args = "-W 1920 -H 1080 -r 144"
    ```

When you launch the game, the shim intercepts the call, loads your config, allows the original arguments to pass through (or be overridden), and executes the real `gamescope`.

### Method 2: Launch Option Generator
If you prefer not to use the shim, you can generate arguments dynamically in the Steam launch option.

```bash
gamescope $(steam-command-runner gamescope args) -- %command%
```
*(Note: This requires `steam-command-runner` to be in your PATH visible to Steam)*

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

**Solution**: Use the absolute path to the shim in your launch options:
```bash
/home/YOUR_USER/.local/bin/gamescope %command%
```
*(Replace `YOUR_USER` with your actual username)*
