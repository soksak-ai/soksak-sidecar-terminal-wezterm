# Change log

This file records completed changes. Current behavior is defined by the terminal contract and the
documents in this directory.

## 2026-08-30

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
