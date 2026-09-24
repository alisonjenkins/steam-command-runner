# Architecture decision records

Each file here records one decision: what we chose, what we rejected, and the
evidence that settled it. Most of these were learned the hard way, from a game
that would not start or a stream that looked right and was not. The code shows
*what* the runner does. These say *why*, so nobody has to rediscover it.

Read the relevant record before changing behaviour it covers. If the evidence
has changed, write a new record that supersedes the old one rather than
editing history.

| # | Decision | Status |
|---|---|---|
| [0001](0001-record-decisions.md) | Record decisions as ADRs | Accepted |
| [0002](0002-gamescope-shim.md) | Intercept `gamescope` through an argv[0] shim | Accepted |
| [0003](0003-spawn-and-wait.md) | Spawn and wait for the game instead of exec | Accepted |
| [0004](0004-inner-env.md) | Keep game-only variables out of gamescope with `inner_env` | Accepted |
| [0005](0005-overlay-preload-on-inner-command.md) | Put the Steam overlay's `LD_PRELOAD` on the inner command | Accepted |
| [0006](0006-render-at-client-resolution.md) | Render at the streaming client's resolution | Accepted |
| [0007](0007-streamed-games-skip-gamescope.md) | Launch streamed games without gamescope | Accepted, pending live test |
| [0008](0008-matching-overlay-only.md) | Keep only the overlay matching the game while streaming | Accepted, pending live test |
| [0009](0009-gamescope-enabled-false.md) | `gamescope_enabled = false` means no gamescope | Accepted |
| [0010](0010-format-on-commit.md) | Format on commit with prek | Accepted |

## Template

```markdown
# NNNN. Title in the imperative

- Status: Proposed | Accepted | Superseded by NNNN
- Date: YYYY-MM-DD

## Context
The problem, and the facts that constrain the answer. Quote log lines and
error text exactly: they are what someone will search for.

## Decision
What we do, in one paragraph.

## Alternatives rejected
Each option we tried or considered, and the specific reason it failed.

## Consequences
What this costs, what it makes easy, what to watch for.

## Evidence
How we know. Commands, log lines, test names, commits.

## Revisit when
The observation that would make this decision wrong.
```
