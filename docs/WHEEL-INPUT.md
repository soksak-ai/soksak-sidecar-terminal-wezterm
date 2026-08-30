# Native wheel input

The common terminal Kit converts device deltas into whole terminal-cell steps and owns ordinary
scrollback. It sends this provider only the two PTY routes: mouse reporting and alternate scroll.
The provider does not encode local scrollback.

For mouse reporting, the provider sends wheel events through the live terminal model. The model's
current mouse tracking and encoding state therefore selects SGR cell reports, default X10 reports,
or UTF-8 extended-coordinate reports. Vertical and horizontal directions, repeated steps, cell
position, and Shift/Alt/Control modifiers remain native model inputs; Meta maps to the model's Alt
modifier. Decimal mouse mode 1015 is not implemented by the pinned model, and this provider does
not add a generic encoder for it.

For alternate scroll, the provider requires the alternate screen and DEC mode 1007, with mouse
reporting inactive. Each normalized step becomes one native wheel event, and the model emits the
cursor sequence selected by its current cursor-key mode. A mouse mode enabled after routing takes
priority and causes the stale alternate-scroll route to be refused.

The provider rechecks every supplied route against the live parsed modes while holding the mirror
seat. A route selected before mouse tracking, alternate screen, or mode 1007 changes fails with
`WHEEL_MODE_CHANGED`; it is never re-encoded under another route.

`tests/wheel_input.rs` is the owner acceptance matrix. `make verify
TARGET=aarch64-apple-darwin` runs it with the rest of the canonical gate.
