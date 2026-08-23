# soksak-sidecar-terminal-wezterm

The terminal-domain restore sidecar built on the **wezterm-term** VT engine. It is
the **second engine unit** implementing the contract `soksak-spec-sidecar-terminal`
— the same contract the other engine units implement on their own engines. One contract, many engine units, one at a time behind a terminal
plugin's manifest declaration (NAMING §8: the unit name carries the engine, exactly
as `[redacted]` carries Chromium).

## The contract lives elsewhere — this repo does not copy it

The normative wire (server face, consumer/daemon peering, checkpoint policy, failure
semantics, acceptance) is owned by **one** repo, and it is not this one:
`soksak-contract-terminal`. It owns
`SPEC.md`, the corpus, the declared reference states, and the assertions this unit is graded by.
This unit implements that contract; it does not restate it.

## Engine seat vs shared machinery

The restore domain is engine-agnostic: the tee consumer `daemon.rs`, the checkpoint
policy `checkpoint.rs`, the mirror + ANSI serializer `mirror.rs`, the daemon wire
`proto.rs`, and the service runtime `service.rs`/`main.rs` never name an engine. The
engine lives behind one face in `engine.rs`, implemented here on `wezterm-term`,
exposing `feed`/`resize`/grid·mode·cursor reads. A different engine unit swaps that
one file; the restore domain logic stays put.

## Graded against the declared reference state

The contract declares the screen each corpus stream must produce, and this unit is graded
against that declaration: its mirror's screen must equal the reference state, and the screen its own
restore paint rebuilds must equal the same reference state. Nothing renders the paint on this unit's
behalf. The declared reference state is the sole correctness criterion for this implementation.

## The gate

**This unit passes when `scripts/gate.sh` passes, and by no other means.** One command, all of
it blocking: the seven fixtures against the contract's declared reference states, the unit tests, and
the performance budgets (SPEC.md §14.2). The benchmark is ignored
in the ordinary test run — it would slow the development loop — so the gate is what makes the
budget binding rather than decorative. The contract repo's own `scripts/gate.sh` runs this one
alongside the other units and adds the guard that only shows when they stand side by side.

## Acceptance

The contract's acceptance suite belongs to the kit, not to this repo. The seven engine-neutral
restore fixtures live in `soksak-kit-terminal-conformance`, and this unit stands its mirror up
against them in one line (`tests/conformance.rs`). GREEN on that shared suite is the unit's
gate — and with no copy here, there is nothing to drift. Installed PTY and recovery-sidecar
composition belongs to the terminal acceptance repository, which installs both products through
Core and verifies warm and archived restore across every terminal plugin.

## Licensing is per-unit

This unit ships the wezterm-term engine (MIT) and carries its `LICENSE` +
`THIRD-PARTY-NOTICES`. No license crosses between units. The conformance judge is a dev-dependency and ships
nowhere, so its Apache-2.0 does not reach this unit either.

## Qualification verdict

Conformance result against `soksak-spec-sidecar-terminal`: **7 of 7**, and the unit
clears the contract-owned performance budget. The owner pins `soksak-ai/wezterm` commit
`eebf29473eb5b7a07c9cb5c833d42fa90fb00777`; no local checkout or path dependency is
part of the build.
