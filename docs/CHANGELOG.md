# Change log

This file records completed changes. Current behavior is defined by the terminal contract and the
documents in this directory.

## 2026-08-28

- Cursor shape and blink state now come from WezTerm's terminal model.
- DECSET/DECRST 12 now changes only cursor blink state; the selected cursor shape remains unchanged.
- The renderer receives WezTerm's 800 ms cursor animation policy.
- Contract cursor acceptance and the arm64 owner gate passed.
