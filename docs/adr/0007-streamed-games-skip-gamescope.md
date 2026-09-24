# 0007. Launch streamed games without gamescope

- Status: Accepted, pending live test of this code path
- Date: 2026-09-24

## Context

Over Remote Play, mouselook stopped at the edge of the client's window. The
camera turned until the Mac client's cursor reached its own window edge, then
stopped, so a full 360 was impossible.

The cause was the stream **mode**, not input handling. Steam's
`streaming_log.txt` showed every session as a desktop stream:

```
Bringing streamed game to foreground - failed
>>> Switching video stream from NONE to Desktop_MovieStream
```

In desktop mode the client sends absolute cursor positions confined to its
window. Only game mode (`k_EStreamActivityGame`, `GameOverlay_MovieStream`)
makes the client capture the mouse and send relative motion.

Steam could not enter game mode because gamescope runs the game on its own
nested X display (`:1`). Steam runs on `:0`, looks for the game window there,
and gets nothing:

```
Adding window 37748737 (4) for process 568841 and gameID 553850
GameScope focus changed to appID 553850
Changing record window: (nil) (0)
```

## Decision

A launch counts as streamed only when Steam says so. For a game launched from
a Remote Play client it sets `SteamStreaming=1` and
`SteamStreamingMaximumResolution=WxH` (seen on 2026-09-24 in the shim's own
environment for a launch from a Steam Deck). The published target file is not
enough on its own, for two reasons:

- It comes too late. The host-side watcher writes it when Steam logs the
  stream, the same second the launch begins, so a launch from the client
  raced it and went through gamescope.
- It stays too long. The watcher arms it for a connected client, and a client
  whose Steam is merely open elsewhere stays connected for hours. A game
  started at the desk meanwhile would have launched as if streamed.

The file still supplies the output, size and refresh when present. Without it,
the size comes from `SteamStreamingMaximumResolution` and the output from
`STEAM_COMMAND_RUNNER_STREAM_OUTPUT`, which only the gamescope path uses.

While a launch is streamed and `stream.bypass_gamescope` is true (the
default), the shim runs the game directly on the host display instead of
wrapping it in gamescope. It keeps `pre_command`, `env`, `inner_env`,
`game_args` and the hooks. Every launch, direct or through gamescope, writes
one decision line to `~/.steam-command-runner-shim.log`, whether or not
`shim_debug` is on:

```
2026-09-24T08:40:18Z app 553850: streaming to steam, launching without gamescope, overlay X86_64 only (from .../game.exe)
2026-09-24T08:36:22Z app 553850: no stream target, launching through gamescope
```

An earlier version printed only to stderr and claimed it reached the journal.
It does not: Steam discards a launched game's stderr. A launch that went
through gamescope because the stream target was withdrawn too early left no
trace at all.

Local play is unchanged and keeps gamescope with its per-game tuning.

## Alternatives rejected

Each was tried live on 2026-09-23 and 24:

- **Fix the camera in the input shim (extest).** It can only turn positions it
  receives into motion. At the edge the client sends nothing new: in a
  measured hold, the extest device's X position sat at the clamp value for 91%
  of 13 seconds, emitting no events.
- **gamescope's `wayland_mouse_relmotion_without_keyboard_focus`.** Set with
  `gamescopectl` at runtime. No effect, because Steam never switched the client
  to relative mode.
- **Write `GAMESCOPE_FOCUSED_APP` on `:0` by hand.** Steam accepted the focus
  but still logged `Changing record window: (nil)`, because the window it
  needs to capture is on `:1`.
- **Run Steam itself inside one gamescope, Steam Deck style.** Would work,
  but it replaces the whole desktop session. Too large a change for this
  problem.

## Consequences

- Streamed games lose gamescope's HDR, FSR and render-size control. The stream
  is SDR H.264 at the client's size anyway, so nothing is lost in practice.
- A game started locally and streamed later is still inside gamescope and
  streams in desktop mode until relaunched.
- The game window now lands on the host display. Whatever places windows on
  the streamed output (nix-config's steam-stream-mode) has to cope with the
  game's helper windows, such as Wine's tray window.
- A game started with the stream target present but the Remote Play client
  already gone still skips gamescope. The host withdraws the target when
  streaming ends, which bounds that window.

## Evidence

With Helldivers 2 launched bare (`%command%`) and the 32-bit overlay removed
([0008](0008-matching-overlay-only.md)):

```
SynchronizeClientState(): setting activity to k_EStreamActivityGame: HELLDIVERS™ 2
>>> Switching video stream from Desktop_MovieStream to GameOverlay_MovieStream_634286
>>> Capture method set to Game Vulkan NV12 + VAAPI H264
```

An `evtest` capture of the input device during a blind mouselook test showed
zero absolute events and 420 relative X events, 12,343 px net. The camera
turned freely. Design:
`docs/superpowers/specs/2026-09-24-remote-play-game-mode-design.md` in
nix-config.

## Revisit when

Steam can foreground and capture a window on a nested gamescope display, or a
game misbehaves without gamescope while streaming. The per-game
`stream_bypass_gamescope = false` is the escape hatch for the second case.
