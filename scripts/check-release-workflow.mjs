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
for (const duplicate of ["build-release.mjs", "release-contract.mjs", "validate-with-spec.mjs"]) if (fs.existsSync(path.join(root, "scripts", duplicate))) throw new Error(`local spec copy is forbidden: scripts/${duplicate}`);
if (fs.existsSync(path.join(root, "validation/spec-validator.json"))) throw new Error("local spec pin copy is forbidden");
console.log("release workflow contract: passed");
