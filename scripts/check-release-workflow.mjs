#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const workflow = fs.readFileSync(path.join(root, ".github/workflows/release.yml"), "utf8");
const manifest = JSON.parse(fs.readFileSync(path.join(root, "sidecar.json"), "utf8"));
const ownerPath = `soksak-sidecars/${manifest.id}`;
const targets = JSON.parse(fs.readFileSync(path.join(root, "release/targets.json"), "utf8"));
const requireText = (value, label) => { if (!workflow.includes(value)) throw new Error(`release workflow is missing ${label}: ${value}`); };
const cargo = fs.readFileSync(path.join(root, "Cargo.toml"), "utf8");
const stage = fs.readFileSync(path.join(root, "stage.sh"), "utf8");
if (!/^edition = "2024"$/m.test(cargo)) throw new Error("Rust packages must use edition 2024");
if (/\bpath\s*=\s*"\.\.\//.test(cargo)) throw new Error("Cargo dependencies must not require sibling checkouts");
if (!cargo.includes('rev = "2b7d7ee5855a2dbef4507da44c347ad4fd74e552"')) throw new Error("Cargo must pin the terminal sidecar kit commit");
if (!cargo.includes('rev = "cab0691a1a01fca7436ac29f6cc2850245788ea6"')) throw new Error("Cargo must pin the terminal contract commit");
requireText("https://github.com/soksak-ai/soksak-spec/releases/download/v0.0.25/soksak-ai-plugin-spec-0.0.25.tgz", "immutable spec package");
requireText("2aba0fb38b99e4f646d403e4ea5ac00ec1bb5b4ba1db9283509aa31c8c6c8d57", "spec package digest");
requireText("node-version-file: soksak-sidecars/soksak-sidecar-terminal-wezterm/.dependency/spec-package/package.json", "Node owner file");
requireText("--spec-package .dependency/spec-package", "package validator input");
if (/path:\s+soksak-(?:kits|contracts)\//.test(workflow)) throw new Error("Cargo dependencies must not be staged as sibling repositories");
if (workflow.includes("repository: soksak-ai/soksak-spec")) throw new Error("release workflow must not checkout the spec source");
if (workflow.includes("pnpm/action-setup")) throw new Error("release workflow must not rebuild the spec package");
requireText(`path: ${ownerPath}`, "owner checkout path");
requireText(`working-directory: ${ownerPath}`, "owner working directory");
requireText(`${ownerPath}/\${{ steps.archive.outputs.asset }}`, "artifact upload path");
requireText(".dependency/spec-package/release-template/", "immutable package tools");
for (const obsolete of ["release/source-dependencies.json", "release/dependencies.json"]) {
  if (fs.existsSync(path.join(root, obsolete))) throw new Error(`${obsolete} is obsolete`);
}
for (const { target, runner } of targets) { requireText(`target: ${target}`, "release target"); requireText(`runner: ${runner}`, "release runner"); }
requireText("release-template/sidecar/build-release.mjs", "canonical release builder");
requireText("release-template/sidecar/validate-with-spec.mjs", "canonical release validator");
requireText("release-template/publish-canonical-release.mjs", "canonical immutable publisher");
requireText("cp dist/sidecar.json package/sidecar.json", "target-specific manifest packaging");
requireText("cp dist/soksak-sidecar-terminal-wezterm* package/dist/", "target-specific executable packaging");
if (!stage.includes('staged="$name$ext"')) throw new Error("stage.sh must select the target executable name");
if (/"version":\s*"[0-9]+\.[0-9]+\.[0-9]+"/.test(stage)) throw new Error("stage.sh must not duplicate the sidecar version");
if (!stage.includes('sed "s#\\\"process\\\": \\\"dist/$name\\\"#\\\"process\\\": \\\"dist/$staged\\\"#" sidecar.json')) {
  throw new Error("stage.sh must derive the staged manifest from sidecar.json");
}
requireText("GH_TOKEN: ${{ steps.release-token.outputs.token }}", "GitHub CLI release token");
for (const duplicate of ["build-release.mjs", "release-contract.mjs", "validate-with-spec.mjs"]) if (fs.existsSync(path.join(root, "scripts", duplicate))) throw new Error(`local spec copy is forbidden: scripts/${duplicate}`);
if (fs.existsSync(path.join(root, "validation/spec-validator.json"))) throw new Error("local spec pin copy is forbidden");
console.log("release workflow contract: passed");
