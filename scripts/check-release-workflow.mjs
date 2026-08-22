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
if (/\bpath\s*=\s*"\.\.\//.test(cargo)) throw new Error("Cargo dependencies must not require sibling checkouts");
requireText("ref: 876e74628167f8e0ea6f2939a9ec6d13a300418f", "terminal sidecar kit commit");
requireText("ref: cab0691a1a01fca7436ac29f6cc2850245788ea6", "terminal contract commit");
requireText("ref: ef67d91f635524a667b8c78052358173d55bf019", "platform spec commit");
requireText(`path: ${ownerPath}`, "owner checkout path");
requireText(`working-directory: ${ownerPath}`, "owner working directory");
requireText(`${ownerPath}/\${{ steps.archive.outputs.asset }}`, "artifact upload path");
requireText(`working-directory: ${ownerPath}/.dependency/soksak-spec`, "validator build directory");
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
