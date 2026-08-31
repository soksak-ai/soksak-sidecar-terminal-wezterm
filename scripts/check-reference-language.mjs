import { readFileSync } from "node:fs";

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
const documents = new Map([
  ["Cargo.toml", read("Cargo.toml")],
  ["README.md", read("README.md")],
  ["README.CHANGELOG.md", read("README.CHANGELOG.md")],
  ["README.CHANGELOG.ko.md", read("README.CHANGELOG.ko.md")],
  ["docs/CHANGELOG.md", read("docs/CHANGELOG.md")],
  ["docs/TERMINAL-PRESENTATION.md", read("docs/TERMINAL-PRESENTATION.md")],
]);

const forbidden = [
  /not against another engine/i,
  /\bour fork\b/i,
  /published crate does not carry/i,
  /unpatched crate/i,
  /the initial engine/i,
  /the defect was fixed at the engine boundary/i,
  /초기 엔진/,
  /결함은 엔진 경계에서 수정했다/,
  /\bpinned fork\b/i,
  /soksak-ai\/wezterm/i,
];

for (const [path, text] of documents) {
  for (const pattern of forbidden) {
    if (pattern.test(text)) throw new Error(`${path} retains comparison/provenance prose: ${pattern}`);
  }
}

const cargo = documents.get("Cargo.toml");
for (const required of [
  "This unit pins wezterm-term, wezterm-surface, and termwiz to one immutable MIT-licensed source revision",
  "wraps a double-width grapheme before placement when only one column remains",
  'wezterm-term = { git = "https://github.com/min-median-max/wezterm", rev = "17c7f4aa77e43ad14459cfe6f5da76b1a0a57a2f" }',
  'wezterm-surface = { git = "https://github.com/min-median-max/wezterm", rev = "17c7f4aa77e43ad14459cfe6f5da76b1a0a57a2f" }',
  'termwiz = { git = "https://github.com/min-median-max/wezterm", rev = "17c7f4aa77e43ad14459cfe6f5da76b1a0a57a2f" }',
]) {
  if (!cargo.includes(required)) throw new Error(`Cargo.toml is missing dependency contract: ${required}`);
}

if (!documents.get("README.md").includes("## Graded against the declared reference state")) {
  throw new Error("README.md does not name the declared reference state directly");
}
if (!documents.get("README.CHANGELOG.md").includes("Revision `eebf29473eb5b7a07c9cb5c833d42fa90fb00777` changed width-two grapheme handling")) {
  throw new Error("English qualification history does not describe the release behavior");
}
if (!documents.get("README.CHANGELOG.ko.md").includes("Revision `eebf29473eb5b7a07c9cb5c833d42fa90fb00777`은 남은 칸이 하나일 때")) {
  throw new Error("Korean qualification history does not describe the release behavior");
}
if (!documents.get("docs/CHANGELOG.md").includes("DECSET/DECRST 12 now changes only cursor blink state")) {
  throw new Error("change log does not describe cursor behavior directly");
}
if (!documents.get("docs/TERMINAL-PRESENTATION.md").includes("The terminal model applies DECSET/DECRST 12 only to the current shape's blink value")) {
  throw new Error("presentation does not describe cursor behavior directly");
}
