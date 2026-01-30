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

#### Why use a Shim?
Normally, to inject dynamic arguments into `gamescope`, you would need to set a complex launch option like:
`steam-command-runner run --gamescope-args="..." -- %command%`

However, this has significant downsides:
1.  **Nesting Complexity**: Steam already wraps games in containers (Pressure Vessel) and potentially other compatibility tools (Proton). Adding another "runner" layer can interfere with signal propagation (e.g., stopping the game) or process tree tracking.
2.  **Launch Option Clutter**: You must update the launch options for *every single game* to point to the runner.
3.  **Steam Integration**: Steam expects certain behaviors from the immediate child process.

**The Solution**: By symlinking `gamescope` -> `steam-command-runner`, we can use the *standard* launch option:
`gamescope %command%`

When Steam calls "gamescope", it actually calls our tool. Our tool:
1.  Detects it is being called as `gamescope`.
2.  Loads the per-game configuration for the current App ID.
3.  Constructs the *real* gamescope command line.
4.  **Replaces itself** (exec) with the real `gamescope` process.

This "exec" step is critical: `steam-command-runner` completely disappears from the process tree. To Steam, it looks like it launched `gamescope` directly. This preserves signal handling, overlay injection, and compatibility tool logic perfectly.
