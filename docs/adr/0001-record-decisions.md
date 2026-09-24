# 0001. Record decisions as ADRs

- Status: Accepted
- Date: 2026-09-24

## Context

The runner sits between Steam, Proton, pressure-vessel and gamescope. Every
one of those has behaviour that is undocumented or surprising, and most of
what the runner does exists to work around one of them. That knowledge lived
in commit bodies, code comments and chat. Some commits have no body at all
(`397aef5 fix: steam overlay inside gamescope`), so the reason is gone.

On 2026-09-23 and 24, several hours went into re-deriving facts that had
already been learned once, and into testing theories that the existing
evidence already ruled out.

## Decision

Each decision that shapes behaviour gets a short record in `docs/adr/`:
context, decision, rejected alternatives, consequences, evidence, and the
observation that would make it wrong. `docs/architecture.md` stays the map of
how the pieces fit, and links here for the reasons.

## Alternatives rejected

- **Code comments only.** Comments explain a line, not a choice between
  designs, and they get deleted with the code they sat on.
- **Commit bodies only.** Useful, but not discoverable. Nobody reads
  `git log` before changing a function.

## Consequences

A change that contradicts a record needs a new record. That is deliberate
friction.

## Revisit when

Records stop being read. If a mistake recorded here gets made again, find
out why the record was missed.
