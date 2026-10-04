# 0012. Treat `StreamForOpenVR=1` as a streamed launch

- Status: Accepted, pending live test
- Date: 2026-10-04

## Context

[0007](0007-streamed-games-skip-gamescope.md) made `SteamStreaming=1` the only
thing that marks a launch as streamed. Steam sets it for a game launched from a
Remote Play client.

A Steam Link VR session (a Quest 2 here) is not a Remote Play client session.
The headset sends a streaming request, Steam starts SteamVR, and SteamVR's
vrlink driver carries the session. A flat game started inside it is shown on a
virtual desktop screen in the headset. Steam never logs
`Streaming started to <client>` for it, and does not set `SteamStreaming` on
the game. So every such launch went through gamescope:

```
2026-10-04T10:58:09Z app 553850: no stream target, launching through gamescope
```

The target file was published 32 seconds before that launch. It was ignored,
as 0007 intended. Inside gamescope the game rendered at the desktop's
2540x1440 in a nested window that niri sees as gamescope's, not the game's.

Steam marks these launches differently. Helldivers 2's environment for that
launch carried:

```
StreamForOpenVR=1
SteamClientLaunch=1
```

and no `SteamStreaming`.

## Decision

`StreamForOpenVR=1` marks a launch as streamed too. It is treated exactly like
`SteamStreaming=1`: the published target supplies the output, size and refresh,
and without one the output comes from `STEAM_COMMAND_RUNNER_STREAM_OUTPUT`.

## Alternatives rejected

- **A `live` field in the published target, set when a stream starts.** The
  host learns that a VR session is showing a game from Steam's
  `>>> Starting desktop stream` line. In the session above that line came
  seven seconds after the launch: Steam starts desktop capture only once
  "Streamed game has created a window". Too late for a decision taken at
  launch.
- **Trust the published target alone again.** That brings back the problem
  0007 fixed: a client idle on another device keeps the target published, and
  a game started at the desk would skip gamescope.

## Consequences

- A game started into a Steam Link VR session loses gamescope's HDR, FSR and
  render-size control, the same trade 0007 made for Remote Play.
- If Steam sets `StreamForOpenVR=1` on a launch that is not streamed (for
  example, a flat game opened from the SteamVR dashboard with a wired headset),
  that launch also skips gamescope. Not observed yet.

## Evidence

- Environment of the shim process for app 553850 on 2026-10-04:
  `StreamForOpenVR=1`, no `SteamStreaming`.
- `remote_connections.txt`, 11:57:37: `Received streaming request ... with
  device ID ...`, then SteamVR (250820) launched. `driver_vrlink.txt`:
  `ReceivedHMDStaticProps. Model Number: Oculus Quest2`.
- `streaming_log.txt` for that session has no `Streaming started to` line.
- Tests: `a_launch_into_a_vr_stream_counts_as_streamed`,
  `a_vr_launch_without_a_published_target_still_counts`,
  `stream_for_openvr_other_than_1_is_not_streamed`.

## Revisit when

A launch at the desk carries `StreamForOpenVR=1`, or a VR-streamed launch
arrives without it.
