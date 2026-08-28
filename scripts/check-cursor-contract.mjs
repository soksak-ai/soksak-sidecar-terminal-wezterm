import { readFileSync } from "node:fs";

const source = readFileSync(new URL("../src/engine.rs", import.meta.url), "utf8");
const required = [
  "fn cursor_style(&self) -> TerminalCursorStyle",
  "fn cursor_animation(&self) -> TerminalCursorAnimation",
];
const missing = required.filter((signature) => !source.includes(signature));
if (missing.length !== 0) {
  process.stderr.write(`CURSOR_ENGINE_STATE_MISSING: ${missing.join(", ")}\n`);
  process.exit(1);
}
process.stdout.write("cursor engine state contract: passed\n");
