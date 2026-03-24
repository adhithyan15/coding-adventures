#!/usr/bin/env node

import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { materializePackage } from "./materializer.js";

function usage(): never {
  console.error("Usage: node dist/src/index.js <package-name> [output-root]");
  process.exit(1);
}

function repoRootFromHere(): string {
  let current = path.dirname(fileURLToPath(import.meta.url));

  while (true) {
    const sharedSourceRoot = path.join(current, "code", "src", "typescript");
    const shellRoot = path.join(current, "code", "packages", "typescript");

    if (existsSync(sharedSourceRoot) && existsSync(shellRoot)) {
      return current;
    }

    const parent = path.dirname(current);
    if (parent === current) {
      throw new Error("Could not locate repository root from package-materializer entrypoint");
    }
    current = parent;
  }
}

function main(): void {
  const [, , packageName, outputRoot] = process.argv;
  if (!packageName) usage();

  const result = materializePackage(packageName, {
    repoRoot: repoRootFromHere(),
    outputRoot,
  });

  console.log(result.outputDir);
}

main();
