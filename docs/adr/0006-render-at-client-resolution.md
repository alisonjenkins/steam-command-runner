# 0006. Render at the streaming client's resolution

- Status: Accepted
- Date: 2026-08-25

## Context

Steam Remote Play captures a whole output. The host gives it a virtual output
at the client's resolution and moves the game there. That is not enough while
the game runs under gamescope: gamescope fixes its render size from `-W`/`-H`
when it starts. A game launched with the desktop's geometry keeps it and is
scaled into the smaller output, so it stays letterboxed. Observed: a 2540x1440
game inside a 1280x800 output.

The shim is the only point in Steam's launch chain that sees gamescope's
arguments before gamescope starts.

## Decision

When a stream target is published (`StreamTarget::detect`), the shim drops any
size, output and refresh flags and appends its own: `-W`, `-H`, `-w`, `-h` at
the client's size, `--prefer-output <output>`, and `-r` if the target has a
refresh rate. Every other flag (HDR, FSR, scaling) is kept.

The target is a JSON file written by the host-side watcher, present only while
a client streams:

```json
{"output":"steam","width":1280,"height":800,"refresh":60}
```

It is read from `STEAM_COMMAND_RUNNER_STREAM_TARGET`, or
`$XDG_RUNTIME_DIR/stream-mode/target.json` by default.

The shim waits up to 20 seconds (`STEAM_COMMAND_RUNNER_STREAM_WAIT`) for the
output to exist before launching, because gamescope resolves `--prefer-output`
once at startup.

## Alternatives rejected

- **Edit size flags in place.** Flags can repeat and gamescope honours the last
  one, so appending our own after removing theirs is the only predictable
  result.
- **Refuse to launch on a malformed target.** A game at the wrong size is a
  poor outcome. A game that will not start is a worse one. A missing or
  malformed file means "not streaming".
- **Block until niri answers.** If niri cannot be asked, the shim stops waiting
  immediately rather than hang a launch on an unanswerable question.

## Consequences

- The default target path is wrong on hosts where Steam runs in a container
  that cannot see `/run/user`. nix-config publishes under `~/.local/state` and
  sets `STEAM_COMMAND_RUNNER_STREAM_TARGET`. Until 2026-09-24 that variable was
  missing, so on ali-desktop this feature never ran at all. Check the variable
  first if streaming behaviour seems absent.
- Since [0007](0007-streamed-games-skip-gamescope.md), streamed games skip
  gamescope by default, so this applies only when `stream.bypass_gamescope` is
  off.

## Evidence

Commits `18f1095` and `0a72a40`. Tests in `src/shim/stream_target.rs` use the
real desktop arguments from the letterboxed launch.

## Revisit when

gamescope can change its render size after startup.
