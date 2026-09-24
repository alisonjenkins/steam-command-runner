# 0008. Keep only the overlay matching the game while streaming

- Status: Accepted, pending live test of this code path
- Date: 2026-09-24

## Context

With gamescope out of the way ([0007](0007-streamed-games-skip-gamescope.md)),
Steam entered game mode but the video froze on its first frame. Steam binds
game capture to the process whose overlay first registers the game window, and
never re-binds:

```
AppID 553850 adding PID 589592 as a tracked process
>>> Switching video stream from Desktop_MovieStream to GameOverlay_MovieStream_589592
AppID 553850 no longer tracking PID 589592, exit code -1
```

Process 589592 was Helldivers 2's GameGuard monitor: a 32-bit Wine process
with scrambled argv that lives about 8 seconds and respawns. Steam preloads
both the 32-bit and 64-bit `gameoverlayrenderer.so` into every process of the
game, so GameGuard's copy registered the window first. The game itself is
64-bit.

## Decision

While streaming, the shim finds the game binary in Steam's `%command%` chain,
reads its architecture from the PE or ELF header, and removes the other
architecture's `gameoverlayrenderer.so` from `LD_PRELOAD`. A 64-bit game keeps
`ubuntu12_64/gameoverlayrenderer.so` only.

Finding the binary (`find_game_binary`): the last `.exe` argument that exists
on disk, otherwise the first argument after the last `--` if it is an ELF
file. If the architecture cannot be read, both overlays stay and the stderr
line says why.

The filtered value is the one the game would actually see: `inner_env`, then
`env`, then inherited (`effective_ld_preload`).

`stream.overlay` (`auto`, `both`, `x86_64`, `i386`) and per-game
`stream_overlay` override the automatic choice.

## Alternatives rejected

- **Force Steam to re-attribute the window.** Rewriting `_NET_WM_PID` and
  unmapping and remapping the window were both tried. Steam caches the pid per
  window id and ignored both.
- **Keep the overlay out of GameGuard specifically.** Nothing in the
  environment identifies GameGuard, and a fix for one anti-cheat helper would
  miss the next.

## Consequences

- A helper of the same bitness as the game can still win. That needs a
  per-game `stream_overlay`, not a code change.
- Outside a stream `LD_PRELOAD` is untouched, so local play keeps Steam's exact
  behaviour.

## Evidence

After the manual equivalent (a wrapper that removed the 32-bit overlay):
`Capture method set to Game Vulkan NV12 + VAAPI H264`, bound to the real game
process, with live video. Unit tests cover PE and ELF detection, the real HD2
command chain, filtering, and `LD_PRELOAD` precedence.

## Revisit when

Steam re-binds capture when the registering process exits.
