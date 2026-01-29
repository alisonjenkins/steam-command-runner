# Steam Command Runner - Architecture

## Project Overview
**steam-command-runner** is a Steam compatibility tool and command wrapper for Linux gaming. It functions as a CLI to manage and launch Steam games, and includes specialized features like a "gamescope shim" to integrate with the Gamescope compositor.

## Technology Stack
- **Language**: Rust (Edition 2021)
- **Build System**: Cargo (Rust) + Nix (Flake)
- **Dependencies**: 
  - `clap` for CLI parsing.
  - `serde`/`toml`/`json` for configuration.
  - `reqwest` for Steam API.
  - `tracing` for logging.

## Codebase Structure

```
.
├── Cargo.toml          # Rust dependencies and project metadata
├── flake.nix           # Nix development environment and build definition
└── src
    ├── bin
    │   └── steam-command-runner.rs  # Main CLI entry point
    ├── lib.rs          # Library root, exports modules
    ├── cli/            # Command-line argument parsing and handlers
    ├── config/         # Configuration loading and management
    ├── shim/           # Special handling for "shim" modes (e.g., gamescope)
    ├── steam/          # Steam installation interaction logic
    ├── proton/         # Proton compatibility tool management
    └── runner/         # Game execution logic
```

## Key Mechanisms

### Entry Point
- **`src/bin/steam-command-runner.rs`**:
  - Checks if invoked as a shim (e.g., as `gamescope`) via `shim::is_invoked_as_gamescope()`.
  - If not a shim, parses CLI arguments using `clap` and executes the corresponding subcommand handler.

### Subcommands
- `run`: Launch a game by AppID.
- `install`/`uninstall`: Manage game installations.
- `search`: Search for games.
- `config`: Manage configuration.
- `proton`: Manage Proton versions.
- `gamescope`: Gamescope specific actions.

### Shim Functionality
The `shim` module allows the binary to behave differently based on how it's called (e.g., if renamed or symlinked to `gamescope`), enabling transparent wrapping of other tools.

This allows `steam-command-runner` to "replace" `gamescope` in the execution chain while still calling the real `gamescope` internally, effectively acting as a middleware that can inject arguments based on its own config.
