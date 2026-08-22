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
if (/\bpath\s*=\s*"\.\.\//.test(cargo)) throw new Error("Cargo dependencies must not require sibling checkouts");
requireText("ref: 4af58a772a141b9e4d4258ae5f21a140aa162d82", "terminal sidecar kit commit");
requireText("ref: cab0691a1a01fca7436ac29f6cc2850245788ea6", "terminal contract commit");
requireText("ref: 1673f33d2102f6ad168f28871d312301fd307371", "platform spec commit");
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
requireText("GH_TOKEN: ${{ steps.release-token.outputs.token }}", "GitHub CLI release token");
for (const duplicate of ["build-release.mjs", "release-contract.mjs", "validate-with-spec.mjs"]) if (fs.existsSync(path.join(root, "scripts", duplicate))) throw new Error(`local spec copy is forbidden: scripts/${duplicate}`);
if (fs.existsSync(path.join(root, "validation/spec-validator.json"))) throw new Error("local spec pin copy is forbidden");
console.log("release workflow contract: passed");
