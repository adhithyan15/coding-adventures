#!/usr/bin/env node

import path from "node:path";
import { fileURLToPath } from "node:url";
import { materializePackage } from "./materializer.ts";

function usage(): never {
  console.error("Usage: node --experimental-strip-types src/index.ts <package-name> [output-root]");
  process.exit(1);
}

function repoRootFromHere(): string {
  const scriptDir = path.dirname(fileURLToPath(import.meta.url));
  return path.resolve(scriptDir, "..", "..", "..", "..", "..");
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
