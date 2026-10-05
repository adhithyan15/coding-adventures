#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const dependencyFields = [
  "dependencies",
  "devDependencies",
  "optionalDependencies",
  "peerDependencies",
];

export async function localInstallOrder(projectDirectory) {
  const ordered = [];
  const visited = new Set();
  const visiting = new Set();

  async function visit(packageDirectory) {
    const directory = path.resolve(packageDirectory);
    if (visited.has(directory)) return;
    if (visiting.has(directory)) {
      throw new Error(`Local package dependency cycle at ${directory}`);
    }

    visiting.add(directory);
    const manifest = JSON.parse(
      await readFile(path.join(directory, "package.json"), "utf8"),
    );
    const localDependencies = dependencyFields.flatMap((field) =>
      Object.entries(manifest[field] ?? {})
        .filter(([, version]) => typeof version === "string" && version.startsWith("file:"))
        .map(([name, version]) => ({
          name,
          directory: path.resolve(directory, version.slice("file:".length)),
        })),
    );

    localDependencies.sort((left, right) => left.name.localeCompare(right.name));
    for (const dependency of localDependencies) await visit(dependency.directory);

    visiting.delete(directory);
    visited.add(directory);
    ordered.push(directory);
  }

  await visit(projectDirectory);
  return ordered;
}

export function runCommand(command, args, cwd) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, stdio: "inherit" });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) return resolve();
      reject(new Error(signal
        ? `${command} terminated by ${signal} in ${cwd}`
        : `${command} exited with status ${code} in ${cwd}`));
    });
  });
}

export async function bootstrap(projectDirectory, options = {}) {
  const npm = options.npmCommand ?? (process.platform === "win32" ? "npm.cmd" : "npm");
  const install = options.install ?? runCommand;
  const log = options.log ?? console.log;
  const frozen = options.frozen === true;
  for (const directory of await localInstallOrder(projectDirectory)) {
    const manifest = JSON.parse(
      await readFile(path.join(directory, "package.json"), "utf8"),
    );
    log(`[bootstrap] ${manifest.name ?? directory}`);
    await install(
      npm,
      frozen
        ? [
            "ci",
            "--silent",
            "--ignore-scripts",
            "--legacy-peer-deps",
            "--audit=false",
            "--fund=false",
          ]
        : ["install", "--silent", "--package-lock=false", "--legacy-peer-deps"],
      directory,
    );
    if (frozen) {
      await install(npm, ["run", "build", "--if-present"], directory);
    }
  }
}

const scriptPath = fileURLToPath(import.meta.url);
if (process.argv[1] && path.resolve(process.argv[1]) === scriptPath) {
  const args = process.argv.slice(2);
  const frozen = args.includes("--frozen");
  const positional = args.filter((argument) => argument !== "--frozen");
  if (positional.length > 1 || args.some((argument) => argument.startsWith("--") && argument !== "--frozen")) {
    console.error("usage: bootstrap.mjs [project-directory] [--frozen]");
    process.exitCode = 2;
  } else {
    bootstrap(path.resolve(positional[0] ?? process.cwd()), { frozen }).catch((error) => {
      console.error(error instanceof Error ? error.message : error);
      process.exitCode = 1;
    });
  }
}
