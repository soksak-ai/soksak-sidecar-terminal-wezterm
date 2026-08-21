#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { ROOT, readTargetMatrix } from "./release-contract.mjs";
const workflow = fs.readFileSync(path.join(ROOT, ".github/workflows/release.yml"), "utf8");
const sources = JSON.parse(fs.readFileSync(path.join(ROOT, "release/source-dependencies.json"), "utf8"));
const requireText = (value, label) => { if (!workflow.includes(value)) throw new Error(`release workflow is missing ${label}: ${value}`); };
requireText(`path: ${sources.ownerPath}`, "owner checkout path");
requireText(`working-directory: ${sources.ownerPath}`, "owner working directory");
requireText(`${sources.ownerPath}/\${{ steps.archive.outputs.asset }}`, "artifact upload path");
requireText(`working-directory: ${sources.ownerPath}/.dependency/soksak-spec`, "validator build directory");
for (const dependency of sources.dependencies) {
  if (!/^[0-9a-f]{40}$/.test(dependency.commit)) throw new Error(`dependency commit must be exact: ${dependency.repository}`);
  requireText(`repository: ${dependency.repository}`, "dependency repository"); requireText(`ref: ${dependency.commit}`, "dependency commit"); requireText(`path: ${dependency.path}`, "dependency checkout path");
}
for (const { target, runner } of readTargetMatrix()) { requireText(`target: ${target}`, "release target"); requireText(`runner: ${runner}`, "release runner"); }
console.log("release workflow contract: passed");
