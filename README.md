# Steam Command Runner

A Steam compatibility tool and command wrapper for Linux gaming. It simplifies managing per-game configurations (especially for Gamescope), managing Steam launch options, and wrapping game commands.

## Features

-   **Gamescope Shim**: Transparently configure Gamescope arguments per-game without changing Steam launch options.
-   **Launch Option Manager**: Bulk update or clear Steam launch options for your games.
-   **Game Search**: Quickly find Steam App IDs.
-   **Config Management**: robust hierarchical configuration system (Global -> Per-Game).

## Quick Start

### Installation (Nix)
```bash
nix profile install github:alisonjenkins/steam-command-runner
```

### Installation (Cargo)
```bash
cargo install --path .
```

### Basic Usage

**Search for a game:**
```bash
steam-command-runner search "Elden Ring"
```

**Install the Gamescope Shim:**
```bash
steam-command-runner install
```

**Configure Gamescope for a specific game:**
```bash
steam-command-runner config edit --app-id 1245620
```

## Documentation

-   [Usage Guide](docs/usage.md) - Detailed guide on using the CLI and Shim features.
-   [Architecture](docs/architecture.md) - Overview of the codebase and internal design.
