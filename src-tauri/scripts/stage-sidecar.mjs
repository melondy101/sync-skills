// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Stage the MCP server binary where the bundler expects it.
//
// `bundle.externalBin` wants `<name>-<target-triple>[.exe]` inside `src-tauri/bin/`
// *before* bundling starts, but cargo produces `skill-manager-mcp` inside
// `target/<profile>/`. This copies one to the other, so the installer carries the
// server next to the app executable — which is what `mcp_config::default_entry`
// resolves, and without it a registered tool points at a file that never exists.
//
// Run through `build.beforeBundleCommand`, i.e. after cargo has built the bins.

import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const BIN_NAME = "skill-manager-mcp";
const here = dirname(fileURLToPath(import.meta.url));
const srcTauri = resolve(here, "..");
const ext = process.platform === "win32" ? ".exe" : "";

function targetDirectory() {
  const out = execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], {
    cwd: srcTauri,
    encoding: "utf8",
  });
  return JSON.parse(out).target_directory;
}

function hostTriple() {
  const out = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
  const line = out.split(/\r?\n/).find((l) => l.startsWith("host:"));
  if (!line) throw new Error("rustc -vV did not report a host triple");
  return line.slice("host:".length).trim();
}

function candidateSources(profiles) {
  const target = targetDirectory();
  const triple = hostTriple();
  const names = profiles.flatMap((profile) => [
    join(target, triple, profile, `${BIN_NAME}${ext}`),
    join(target, profile, `${BIN_NAME}${ext}`),
  ]);
  return { found: names.filter((p) => existsSync(p) && statSync(p).size > 0), names };
}

// `--debug` is the only profile switch the bundler path needs; release is default.
const profiles = process.argv.includes("--debug")
  ? ["debug"]
  : ["release", "debug"];

const { found, names } = candidateSources(profiles);
if (found.length === 0) {
  console.error(
    `[stage-sidecar] no built ${BIN_NAME}${ext} found. Build the workspace before bundling.\nSearched:\n  ${names.join("\n  ")}`,
  );
  process.exit(1);
}

const destinationDir = join(srcTauri, "bin");
mkdirSync(destinationDir, { recursive: true });
const destination = join(destinationDir, `${BIN_NAME}-${hostTriple()}${ext}`);
copyFileSync(found[0], destination);
console.log(`[stage-sidecar] ${found[0]} -> ${destination}`);
