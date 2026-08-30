# Change log

This file records completed changes. Current behavior is defined by the terminal contract and the
documents in this directory.

## 2026-08-30

- Release 0.0.38 pins common terminal Kit v0.0.34 at
  `20fb2d73d13e5bcde592380d3052c5d2204a592f`.
- DEC9 and DEC1001 are distinct native WezTerm engine states, exposed by provider commit
  `17c7f4aa77e43ad14459cfe6f5da76b1a0a57a2f`; neither is inferred from another mouse mode.
- Wheel and pointer admission use the Kit's public `mouse_reporting()` and `reports_pointer()`
  policy helpers before the native encoder runs.
- Cross-root release verification compares the complete release file set and every shipped byte,
  including Mach-O load commands and UUID.
- Wheel reports now flow through the live engine mouse mode and encoder.
- SGR cell, default X10, and UTF-8 reports preserve both axes, repetition, position, and modifiers.
- Alternate-screen mode 1007 emits native cursor input and refuses stale or superseded routes.
- Normal scrollback remains owned by the common terminal Kit; no generic wheel encoder was added.

## 2026-08-29

- Pointer press, motion, and release use WezTerm's live terminal mouse input API.
- Selection and wheel remain explicit unsupported operations.

## 2026-08-28

- Terminal theme overrides now come from WezTerm's explicit color override state.
- An explicit color equal to the configured base remains present until OSC reset or RIS.
- Cursor shape and blink state now come from WezTerm's terminal model.
- DECSET/DECRST 12 now changes only cursor blink state; the selected cursor shape remains unchanged.
- The renderer receives WezTerm's 800 ms cursor animation policy.
- Contract cursor acceptance and the arm64 owner gate passed.
