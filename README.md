# Steam Command Runner

A Steam compatibility tool and command wrapper for Linux gaming. It simplifies managing per-game configurations (especially for Gamescope), managing Steam launch options, and wrapping game commands.

## Features

-   **Gamescope Shim**: Transparently configure Gamescope arguments per-game without changing Steam launch options.
-   **Launch Option Manager**: Bulk update or clear Steam launch options for your games.
-   **Game Search**: Quickly find Steam App IDs.
-   **Config Management**: robust hierarchical configuration system (Global -> Per-Game).

## Quick Start

## Installation

### Option 1: Binary Release (Easiest)
1. Go to the [Releases Page](https://github.com/alisonjenkins/steam-command-runner/releases).
2. Download the zip for your architecture (`x86_64` or `arm64`).
3. Extract the binary and place it in `~/.local/bin` (create it if it doesn't exist):
   ```bash
   mkdir -p ~/.local/bin
   unzip steam-command-runner-linux-x86_64.zip
   mv steam-command-runner ~/.local/bin/
   ```

4. **Important**: Ensure `~/.local/bin` is in your `PATH`.
   Check if it's already there:
   ```bash
   echo $PATH | grep "$HOME/.local/bin"
   ```
   If not, add this to your shell config file (e.g., `~/.bashrc`, `~/.zshrc`):
   ```bash
   export PATH="$HOME/.local/bin:$PATH"
   ```
   Then reload your shell:
   ```bash
   source ~/.bashrc  # or ~/.zshrc
   ```

### Option 2: Nix Flake
Add to your `flake.nix` inputs:
```nix
inputs.steam-command-runner.url = "github:alisonjenkins/steam-command-runner";
```
Then add package `inputs.steam-command-runner.packages.${system}.default` to your system configuration.

Or run directly:
```bash
nix run github:alisonjenkins/steam-command-runner -- help
```

### Option 3: Manual Build
Clone and build with Cargo:
```bash
cargo build --release
cp target/release/steam-command-runner ~/.local/bin/
```

## Quick Start
1. **Install the Shim**:
   ```bash
   steam-command-runner install
   ```
   This creates `~/.local/bin/gamescope` symlinked to the tool.

2. **Configure Steam**:
   Set your game's launch options to:
   ```bash
   /home/youruser/.local/bin/gamescope -- %command%
   ```
   *Note: Use `steam-command-runner launch-options set-all` to automate this!*

## Documentation

-   [Usage Guide](docs/usage.md) - Detailed guide on using the CLI and Shim features.
-   [Architecture](docs/architecture.md) - Overview of the codebase and internal design.
