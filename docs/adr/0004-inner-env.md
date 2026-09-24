# 0004. Keep game-only variables out of gamescope with `inner_env`

- Status: Accepted
- Date: 2026-08-16

## Context

`env` is set on the process the runner starts. When gamescope wraps the game,
that process is gamescope, and gamescope passes everything on to the game. So
`MANGOHUD=1` in `env` also reached gamescope itself.

MangoHud loads as an implicit Vulkan layer keyed off `MANGOHUD=1`. Inside
gamescope's own Vulkan instance it is useless (the HUD you see comes from the
game's instance) and fatal: gamescope segfaults at exit in
`CVulkanDevice::~CVulkanDevice`, because MangoHud's object map is already gone
when the global device destructor frees command buffers.

## Decision

`inner_env` holds variables meant for the game only. Under gamescope they are
emitted as `KEY=VALUE` arguments to an `env` wrapper after gamescope's `--`,
so gamescope never sees them. Without gamescope they are set on the game
process directly.

The shim warns when `MANGOHUD`, `MANGOHUD_CONFIG` or `ENABLE_VKBASALT` appear
in `env` (`is_compositor_hostile`).

## Alternatives rejected

- **Unset the variables in gamescope's environment.** gamescope passes its
  environment on to the game, so the game would lose them too.
- **Tell users to use `env MANGOHUD=1` in Launch Options.** Correct, but
  per-game and easy to forget. This is the problem the shim exists to solve.

## Consequences

Anything that loads an implicit Vulkan layer belongs in `inner_env`.

## Evidence

Commits `fe7b4c1` and `7602aa8`. The segfault at exit stopped once `MANGOHUD`
moved to `inner_env`.

## Revisit when

MangoHud stops crashing inside a compositor's Vulkan instance.
