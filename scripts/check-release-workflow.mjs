#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (name) => fs.readFileSync(path.join(root, name), "utf8");
const workflow = read(".github/workflows/release.yml");
const manifest = JSON.parse(read("sidecar.json"));
const targets = JSON.parse(read("release/targets.json"));
const makefile = read("Makefile");
const stage = read("scripts/stage-built.sh");
const ownerPath = `soksak-sidecars/${manifest.id}`;
const requireText = (value, label) => { if (!workflow.includes(value)) throw new Error(`release workflow is missing ${label}: ${value}`); };
for (const target of ["preflight", "prepare", "build", "stage", "verify"]) if (!new RegExp(`^${target}:`, "m").test(makefile)) throw new Error(`Makefile target is missing: ${target}`);
for (const value of ["spec_url:", "spec_sha256:", "${{ inputs.spec_url }}", "${{ inputs.spec_sha256 }}"]) requireText(value, "release-train input");
requireText('make verify TARGET="${{ matrix.target }}"', "owner Make verification");
requireText('make stage TARGET="${{ matrix.target }}" OUT=dist', "owner Make staging");
requireText("release-template/sidecar/pack-target.mjs", "canonical target packer");
requireText(`path: ${ownerPath}`, "owner checkout path");
requireText(`working-directory: ${ownerPath}`, "owner working directory");
requireText("choco install make --version=4.4.1", "Windows Make environment");
if (!stage.includes('staged=$name$ext')) throw new Error("stage-built does not select the target executable name");
if (!stage.includes("absolute candidate output")) throw new Error("stage-built does not permit isolated absolute output");
for (const { target, runner } of targets) { requireText(`target: ${target}`, "release target"); requireText(`runner: ${runner}`, "release runner"); }
for (const match of workflow.matchAll(/^\s*-?\s*uses:\s*([^\s#]+)/gm)) if (!/^[^@\s]+@[a-f0-9]{40}$/.test(match[1])) throw new Error(`workflow action is not commit-pinned: ${match[1]}`);
for (const obsolete of ["stage.sh", "export PATH=", "tar -czf", "SOKSAK_PTYD_BIN", "SOKSAK_CORE_WORKTREE"]) if (workflow.includes(obsolete)) throw new Error(`workflow retains obsolete behavior: ${obsolete}`);
console.log("release workflow contract: passed");
