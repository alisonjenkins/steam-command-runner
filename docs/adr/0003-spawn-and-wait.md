# 0003. Spawn and wait for the game instead of exec

- Status: Accepted (supersedes the "replaces itself" design)
- Date: 2026-09-04

## Context

The shim originally `exec()`ed the real gamescope, so the runner vanished from
the process tree. The earlier architecture notes called that step critical.

Then `pre_launch` and `post_exit` hooks were added to the config, for example
switching a keyboard to a game's keymap and back. `hooks::execute()` had no
callers: configuring a hook silently did nothing. And after an `exec()` there
is no process left to run anything once the game exits.

## Decision

The shim runs `pre_launch`, spawns the game (or gamescope), waits for it, runs
`post_exit`, and exits with the child's exit code, clamped to 0 to 255.

## Alternatives rejected

- **Keep `exec()`, run `post_exit` elsewhere.** Nothing else in the launch
  chain knows when the game ended.
- **Wire hooks into `run` only.** This host launches every game through the
  shim, never through `run`, so hooks would still never fire.

## Consequences

- `spawn()` inherits the environment exactly as `exec()` did, so Steam-set
  variables (`LIBEI_SOCKET`, `LD_PRELOAD`) reach the game unchanged.
- The runner stays in the process tree as the parent of gamescope or the game.
  Steam's Stop button and exit tracking have worked through it since
  2026-09-04.
- A `pre_launch` hook that fails is logged and the launch continues. A game
  that will not start is worse than a keymap that did not switch.

## Evidence

Commit `4260376`: "hooks::execute() had zero callers anywhere in the crate."
The UHK keymap hooks for Helldivers 2 fire on launch and exit.

## Revisit when

Signals or exit codes stop reaching Steam correctly through the extra process.
