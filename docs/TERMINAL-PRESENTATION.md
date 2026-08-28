# Terminal presentation

WezTerm's terminal model owns parsed cursor state. The sidecar reads
`Terminal::cursor_pos().shape` and maps the engine's blinking and steady block, underline, and bar
variants to `TerminalCursorStyle`. It does not parse CSI in an adapter.

The terminal model applies DECSET/DECRST 12 only to the current shape's blink value, and DECRQM 12
reports that state. DECTCEM visibility remains
separate in `TerminalModes.show_cursor`.

The provider maps WezTerm's configured default to its documented steady block and declares the
default 800 ms renderer animation interval. The common terminal Kit schedules frames only while the
semantic cursor is visible and blinking.

The pinned terminal model separately exposes explicit OSC 4/10/11/12 color override presence and
effective palette state. This sidecar maps only the raw indexed, foreground, background and cursor
overrides to `TerminalThemeOverrides`. It does not infer presence by comparing effective colors
with the configured palette, and it does not parse the input stream again.

`tests/conformance.rs::cursor_style` runs the contract-owned DECSCUSR, mode 12, DECTCEM, and warm
rehydrate cases. `make verify TARGET=aarch64-apple-darwin` verifies this provider only.
