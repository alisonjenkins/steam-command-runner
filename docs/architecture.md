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
3.  Runs the `pre_launch` hook, if configured.
4.  Decides how to launch (`src/shim/launch.rs`):
    - **gamescope**: the normal case. Builds the real gamescope command line,
      with game-only variables and the Steam overlay's `LD_PRELOAD` on the
      inner command.
    - **direct**: while a Remote Play client is streaming, or when the game
      sets `gamescope_enabled = false`. Runs the game on the host display and
      keeps only the Steam overlay that matches the game's architecture.
5.  Spawns the command, waits for it, runs the `post_exit` hook, and exits with
    the child's exit code.

The runner stays in the process tree as the parent. It used to `exec()` and
vanish, but then there was nothing left to run `post_exit` after the game
ended. `spawn()` passes the environment on exactly as `exec()` did.

## Why it works this way

The decisions behind the shim, with the evidence for each, are in
[`docs/adr/`](adr/README.md). Read the relevant record before changing
launch behaviour. Most of them were learned from a game that would not start
or a stream that looked right and was not.

| Behaviour | Record |
|---|---|
| Shim installed as `gamescope` | [0002](adr/0002-gamescope-shim.md) |
| Spawn and wait, not exec | [0003](adr/0003-spawn-and-wait.md) |
| `inner_env` for MangoHud and similar | [0004](adr/0004-inner-env.md) |
| Overlay `LD_PRELOAD` on the inner command | [0005](adr/0005-overlay-preload-on-inner-command.md) |
| gamescope sized to the streaming client | [0006](adr/0006-render-at-client-resolution.md) |
| Streamed games skip gamescope | [0007](adr/0007-streamed-games-skip-gamescope.md) |
| Only the matching overlay while streaming | [0008](adr/0008-matching-overlay-only.md) |
| `gamescope_enabled = false` | [0009](adr/0009-gamescope-enabled-false.md) |

## Diagnosing a launch

- The shim prints one line to stderr for every direct launch, and it reaches
  the journal through Steam: `journalctl --user | grep steam-command-runner`.
- `shim_debug = true` in `config.toml` logs every decision and the final
  command line to `~/.steam-command-runner-shim.log`.
- The Launch Options must call the shim by absolute path
  (`/home/<user>/.local/bin/gamescope -- %command%`). A bare `gamescope` can
  reach the real binary and skip the runner without any error.
