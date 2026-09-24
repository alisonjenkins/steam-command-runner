# 0010. Format on commit with prek

- Status: Accepted
- Date: 2026-09-24

## Context

The crate was not `rustfmt`-clean, so every change either reformatted
unrelated lines or had to format files selectively.

## Decision

One commit formats the whole crate. From then on a prek pre-commit hook runs
`cargo fmt --all`, and blocks the commit if it changed anything. The devshell
installs the hook on entry and provides `git` and `prek`. The hook falls back
to `nix develop` when `cargo` is not on `PATH`, so a commit from outside the
devshell is formatted too.

## Alternatives rejected

- **CI check only.** Catches it after the fact, with a fix-up commit each time.
- **The Python `pre-commit` tool.** prek reads the same config and needs no
  Python environment.

## Evidence

A deliberately misformatted commit was blocked, and the hook fixed the file.

## Revisit when

The repo gains CI; add `cargo fmt --check` there as well.
