# 0002. Intercept `gamescope` through an argv[0] shim

- Status: Accepted
- Date: 2026-01-30 (recorded 2026-09-24)

## Context

Per-game gamescope settings (resolution, HDR, FSR, refresh) used to live in
each game's Steam Launch Options. Changing them meant editing every game, and
wrapping the launch in our own command risked breaking what Steam expects of
its direct child: signals for the Stop button, process-tree tracking, overlay
injection.

## Decision

The runner binary is installed as `~/.local/bin/gamescope`, ahead of the real
gamescope on `PATH`. When invoked under that name (`is_invoked_as_gamescope`
checks the basename of argv[0]), it loads the config for `$SteamAppId`, builds
the real gamescope command line, and runs it. Launch Options stay the same for
every game:

```
/home/<user>/.local/bin/gamescope -- %command%
```

The real gamescope is found by searching `PATH` and skipping any candidate
with the runner's own inode (`find_real_gamescope`).

## Alternatives rejected

- **`steam-command-runner run --app-id N -- %command%` in every game's Launch
  Options.** Works, but every game needs editing, and it is easy to get wrong.
  The `run` subcommand still exists for manual use.
- **`gamescope $(steam-command-runner gamescope args) -- %command%`.** Shell
  expansion in Launch Options is brittle, and the string still has to be
  edited per game.

## Consequences

- The absolute path matters. Steam's launch environment does not always put
  `~/.local/bin` first, so a bare `gamescope` can reach the real binary and
  silently skip the runner.
- Anything the shim does applies to every game launched this way, which is
  what makes stream-aware launching ([0007](0007-streamed-games-skip-gamescope.md))
  possible without per-game setup.

## Evidence

`docs/usage.md` covers setup and the `PATH` pitfall. `b6ac38e` switched
`launch-options` to the absolute shim path after launches bypassed it.

## Revisit when

Steam offers a supported per-game wrapper hook, or gamescope reads per-game
config itself.
