# soksak-sidecar-terminal-wezterm

The terminal-domain restore sidecar built on the **wezterm-term** VT engine. It is
the **second engine unit** implementing the contract `soksak-spec-sidecar-terminal`
— the same contract the other engine units implement on their own engines. One contract, many engine units, one at a time behind a terminal
plugin's manifest declaration (NAMING §8: the unit name carries the engine, exactly
as `soksak-sidecar-browser-chromium` carries Chromium).

## The contract lives elsewhere — this repo does not copy it

The normative wire (server face, consumer/daemon peering, checkpoint policy, failure
semantics, acceptance) is owned by **one** repo, and it is not this one:
`soksak-contract-terminal` (`~/.soksak-dev/contracts/soksak-contract-terminal`). It owns
`SPEC.md`, the corpus, the declared goldens, and the assertions this unit is graded by.
This unit implements that contract; it does not restate it.

## Engine seat vs shared machinery

The restore domain is engine-agnostic: the tee consumer `daemon.rs`, the checkpoint
policy `checkpoint.rs`, the mirror + ANSI serializer `mirror.rs`, the daemon wire
`proto.rs`, and the service runtime `service.rs`/`main.rs` never name an engine. The
engine lives behind one face in `engine.rs`, implemented here on `wezterm-term`,
exposing `feed`/`resize`/grid·mode·cursor reads. A different engine unit swaps that
one file; the restore domain logic stays put.

## Graded against a declared golden, not against another engine

The contract declares the screen each corpus stream must produce, and this unit is graded
against that declaration: its mirror's screen must equal the golden, and the screen its own
restore paint rebuilds must equal the same golden. Nothing renders the paint on this unit's
behalf, and no engine's behaviour defines correctness — the standard is external to every
implementation, this one included.

## The gate

**This unit passes when `scripts/gate.sh` passes, and by no other means.** One command, all of
it blocking: the seven fixtures against the contract's declared goldens, the unit tests, the
real-daemon integration, and the performance budgets (SPEC.md §14.2). The benchmark is ignored
in the ordinary test run — it would slow the development loop — so the gate is what makes the
budget binding rather than decorative. The contract repo's own `scripts/gate.sh` runs this one
alongside the other units and adds the guard that only shows when they stand side by side.

## Acceptance

The contract's acceptance suite belongs to the kit, not to this repo. The seven engine-neutral
restore fixtures live in `soksak-kit-terminal-conformance`, and this unit stands its mirror up
against them in one line (`tests/conformance.rs`). GREEN on that shared suite is the unit's
gate — and with no copy here, there is nothing to drift. Real-daemon integration
(`tests/ptyd_integration.rs`, driven by `scripts/e2e/ptyd-integration.sh`) exercises the
tee→mirror→checkpoint round trip against an isolated `soksak-ptyd` binary.

## Licensing is per-unit

This unit ships the wezterm-term engine (MIT) and carries its `LICENSE` +
`THIRD-PARTY-NOTICES`. No license crosses between units. The conformance judge is a dev-dependency and ships
nowhere, so its Apache-2.0 does not reach this unit either.

## Qualification verdict

Conformance result against `soksak-spec-sidecar-terminal`: **7 of 7 on the fixtures, and it
clears the performance floor.** The unit passes its gate.

**Both halves were earned, and the performance half was earned twice.** The contract's floor is
the rate at which the daemon delivers to the tee with **no app attached** — the mode this
sidecar exists for, where nothing throttles the daemon's pty read (SPEC §14.1). And the verdict
is not a ratio: the gate holds a real tee subscriber at this unit's own measured feed rate,
floods a real `soksak-ptyd`, and reads back what the daemon dropped.

At **68 MB/s** against a demand of 84–89, this unit **lost 16.5 MB of a 67 MB flood** — with the
app closed and a session dumping output, its mirror was missing about a quarter of the screen it
exists to restore. That was a real, observed loss, and it was recorded here as a failure rather
than argued away.

The mirror was made faster (feed **68 → 102 MB/s**), and against the same daemon at the same
demand it now **drops nothing**. The floor never moved to accommodate it.

### Fixture history (both defects closed at their owner)

The published engine was **6 of 7**. Fixture ② (`cjk_width`) was RED, and not because of a
fixture quirk: given 79 columns filled and one column left, the engine packed a
double-width character into that column. A width-2 character occupies two adjacent columns
and cannot be placed in one; with autowrap on it must move to the next line. Its print path
checked only whether the cursor had passed the right margin, never whether the grapheme fit
in what was left, so the grid ended up with a row claiming 81 columns of content in an
80-column grid and a scrollback one row short. Three independent engines wrap, and two of
them keep a dedicated cell state for the column the moved character leaves behind.

The gap was closed at its owner. A local fork (`../../vendor/wezterm-term`, consumed by
Cargo path dependency) adds the missing check; against that engine the **unchanged** suite
is 7 of 7, the engine's own test suite is unregressed, and the fixtures repeat 20/20
without flaking. A patch for wezterm upstream is prepared. Release eligibility waits on the
fix reaching a published crate — until then the unit consumes the fork by path.
