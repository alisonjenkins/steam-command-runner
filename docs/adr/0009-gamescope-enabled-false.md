# 0009. `gamescope_enabled = false` means no gamescope

- Status: Accepted
- Date: 2026-09-24

## Context

Until 2026-09-24 the shim still started gamescope when a game set
`gamescope_enabled = false`, and only dropped the configured arguments. Only
the `run` subcommand honoured the flag fully. The name promised one thing and
the shim did another. It cost a debugging detour when the flag was expected to
remove gamescope and did not.

## Decision

In the shim, `gamescope_enabled = false` launches the game directly, stream or
not, through the same path as a streamed launch
([0007](0007-streamed-games-skip-gamescope.md)).

## Consequences

Games that set the flag now run without gamescope under the shim. Subnautica's
config in nix-config sets it, and SteamVR renders to the headset without
gamescope either way.

## Evidence

`launch_mode` unit tests cover both flags in every combination.

## Revisit when

A game needs gamescope started with no arguments. Give that its own setting
rather than overloading this one.
