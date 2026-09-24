# 0005. Put the Steam overlay's `LD_PRELOAD` on the inner command

- Status: Accepted
- Date: 2026-01-29 (recorded 2026-09-24)

## Context

The Steam overlay is `gameoverlayrenderer.so`, loaded through `LD_PRELOAD`.
Under the shim, the game runs inside gamescope, and the overlay stopped
appearing.

gamescope is normally installed with `CAP_SYS_NICE` so it can use realtime
scheduling. A binary that gains capabilities runs in the kernel's secure
execution mode, where the dynamic loader ignores `LD_PRELOAD`. So a preload set
on gamescope's own process does not reach the game reliably.

## Decision

The shim puts `LD_PRELOAD`, including the overlay for both architectures, on
the inner command with an `env` wrapper after gamescope's `--`, just like
`inner_env` ([0004](0004-inner-env.md)). It also sets
`ENABLE_VK_LAYER_VALVE_steam_overlay_1`, `ENABLE_GAMESCOPE_WSI` and the
`STEAM_GAMESCOPE_*` feature flags that Steam sets when it launches gamescope
itself.

## Alternatives rejected

- **`LD_PRELOAD` on the gamescope process.** Dropped by secure execution, as
  above.

## Consequences

Without gamescope ([0007](0007-streamed-games-skip-gamescope.md)) none of this
applies: the game inherits Steam's own `LD_PRELOAD` directly, and the
gamescope-only variables are left unset, because they would tell the overlay
that gamescope is present when it is not.

## Evidence

Reconstructed from the code and the comment in `src/shim/gamescope.rs`. The
original commit, `397aef5 fix: steam overlay inside gamescope`, has no body.

## Revisit when

gamescope stops needing capabilities, or Steam's overlay stops relying on
`LD_PRELOAD`.
