#!/usr/bin/env node

/**
 * # Capability Analyzer CLI
 *
 * Command-line interface for the capability analyzer. Provides three subcommands:
 *
 * ## Subcommands
 *
 * ### `detect <file...>`
 * Scan one or more TypeScript/JavaScript files and report all detected capabilities.
 *
 * ```bash
 * ca-capability-analyzer detect src/server.ts src/db.ts
 * ```
 *
 * ### `banned <file...>`
 * Scan for banned constructs (eval, dynamic require, Reflect, etc).
 *
 * ```bash
 * ca-capability-analyzer banned src/**\/*.ts
 * ```
 *
 * ### `check <manifest> <file...>`
 * Compare detected capabilities against a manifest file. Returns non-zero
 * exit code if there are undeclared capabilities.
 *
 * ```bash
 * ca-capability-analyzer check required_capabilities.json src/**\/*.ts
 * ```
 */

import * as fs from "fs";
import { analyzeSource } from "./analyzer.js";
import { parseManifest, compareCapabilities } from "./manifest.js";
import type { DetectedCapability, BannedConstruct } from "./analyzer.js";

// ============================================================================
// Formatting Helpers
// ============================================================================

/**
 * Format a single capability as a human-readable line.
 *
 * Output format: `file:line  category:action:target  (evidence)`
 *
 * Example: `src/server.ts:42  fs:read:/etc/passwd  (fs.readFileSync("/etc/passwd"))`
 */
function formatCapability(cap: DetectedCapability): string {
  return `  ${cap.file}:${cap.line}  ${cap.category}:${cap.action}:${cap.target}  (${cap.evidence})`;
}

/**
 * Format a single banned construct as a human-readable line.
 */
function formatBanned(b: BannedConstruct): string {
  return `  ${b.file}:${b.line}  [${b.kind}]  ${b.evidence}`;
}

// ============================================================================
// File Reading Helper
// ============================================================================

/**
 * Read source files from disk given their paths.
 * Returns an array of { filename, source } objects.
 * Skips files that can't be read (with a warning to stderr).
 */
function readSourceFiles(paths: string[]): Array<{ filename: string; source: string }> {
  const files: Array<{ filename: string; source: string }> = [];
  for (const filepath of paths) {
    try {
      const source = fs.readFileSync(filepath, "utf-8");
      files.push({ filename: filepath, source });
    } catch (err) {
      console.error(`Warning: Could not read ${filepath}: ${err}`);
    }
  }
  return files;
}

// ============================================================================
// Subcommands
// ============================================================================

/**
 * ## detect subcommand
 *
 * Scans files and prints all detected capabilities to stdout.
 * Exit code 0 always (detection is informational).
 */
function cmdDetect(filePaths: string[]): number {
  if (filePaths.length === 0) {
    console.error("Usage: ca-capability-analyzer detect <file...>");
    return 1;
  }

  const files = readSourceFiles(filePaths);
  let totalCapabilities = 0;

  for (const { filename, source } of files) {
    const result = analyzeSource(source, filename);
    if (result.capabilities.length > 0) {
      console.log(`\n${filename}:`);
      for (const cap of result.capabilities) {
        console.log(formatCapability(cap));
        totalCapabilities++;
      }
    }
  }

  console.log(`\nTotal: ${totalCapabilities} capabilities detected in ${files.length} files.`);
  return 0;
}

/**
 * ## banned subcommand
 *
 * Scans files and prints all banned constructs to stdout.
 * Exit code 1 if any banned constructs found, 0 otherwise.
 */
function cmdBanned(filePaths: string[]): number {
  if (filePaths.length === 0) {
    console.error("Usage: ca-capability-analyzer banned <file...>");
    return 1;
  }

  const files = readSourceFiles(filePaths);
  let totalBanned = 0;

  for (const { filename, source } of files) {
    const result = analyzeSource(source, filename);
    if (result.banned.length > 0) {
      console.log(`\n${filename}:`);
      for (const b of result.banned) {
        console.log(formatBanned(b));
        totalBanned++;
      }
    }
  }

  if (totalBanned > 0) {
    console.log(`\nFOUND ${totalBanned} banned constructs in ${files.length} files.`);
    return 1;
  } else {
    console.log(`\nNo banned constructs found in ${files.length} files.`);
    return 0;
  }
}

/**
 * ## check subcommand
 *
 * Compares detected capabilities against a manifest file.
 * Prints matched, undeclared, and unused capabilities.
 * Exit code 1 if there are any undeclared capabilities, 0 otherwise.
 */
function cmdCheck(manifestPath: string, filePaths: string[]): number {
  if (filePaths.length === 0) {
    console.error("Usage: ca-capability-analyzer check <manifest.json> <file...>");
    return 1;
  }

  // Load and parse manifest
  let manifestJson: string;
  try {
    manifestJson = fs.readFileSync(manifestPath, "utf-8");
  } catch (err) {
    console.error(`Error: Could not read manifest file ${manifestPath}: ${err}`);
    return 1;
  }

  let manifest;
  try {
    manifest = parseManifest(manifestJson);
  } catch (err) {
    console.error(`Error: Invalid manifest file ${manifestPath}: ${err}`);
    return 1;
  }

  // Analyze all source files
  const files = readSourceFiles(filePaths);
  const allCapabilities: DetectedCapability[] = [];

  for (const { filename, source } of files) {
    const result = analyzeSource(source, filename);
    allCapabilities.push(...result.capabilities);
  }

  // Compare
  const comparison = compareCapabilities(allCapabilities, manifest);

  // Report
  if (comparison.matched.length > 0) {
    console.log(`\nMatched (${comparison.matched.length}):`);
    for (const cap of comparison.matched) {
      console.log(formatCapability(cap));
    }
  }

  if (comparison.undeclared.length > 0) {
    console.log(`\nUNDECLARED (${comparison.undeclared.length}):`);
    for (const cap of comparison.undeclared) {
      console.log(formatCapability(cap));
    }
  }

  if (comparison.unused.length > 0) {
    console.log(`\nUnused declarations (${comparison.unused.length}):`);
    for (const decl of comparison.unused) {
      console.log(`  ${decl.category}:${decl.action}:${decl.target}`);
    }
  }

  // Summary
  const total = allCapabilities.length;
  console.log(`\nSummary: ${total} detected, ${comparison.matched.length} matched, ${comparison.undeclared.length} undeclared, ${comparison.unused.length} unused declarations.`);

  return comparison.undeclared.length > 0 ? 1 : 0;
}

// ============================================================================
// Main Entry Point
// ============================================================================

/**
 * ## CLI Argument Parsing
 *
 * We use a simple hand-rolled argument parser since we only have three
 * subcommands with positional arguments. No need for a dependency like
 * `commander` or `yargs`.
 *
 * ```
 * process.argv = ["node", "cli.ts", "detect", "file1.ts", "file2.ts"]
 *                   ^0       ^1        ^2        ^3          ^4
 * ```
 *
 * So `process.argv[2]` is the subcommand, and `process.argv.slice(3)` are
 * the arguments to that subcommand.
 */
export function main(args: string[] = process.argv.slice(2)): number {
  const subcommand = args[0];

  switch (subcommand) {
    case "detect":
      return cmdDetect(args.slice(1));

    case "banned":
      return cmdBanned(args.slice(1));

    case "check": {
      if (args.length < 3) {
        console.error("Usage: ca-capability-analyzer check <manifest.json> <file...>");
        return 1;
      }
      const manifestPath = args[1];
      const filePaths = args.slice(2);
      return cmdCheck(manifestPath, filePaths);
    }

    default:
      console.error(`Capability Analyzer — Static analysis for OS capability usage

Usage:
  ca-capability-analyzer detect <file...>              Detect capabilities
  ca-capability-analyzer banned <file...>              Find banned constructs
  ca-capability-analyzer check <manifest> <file...>    Compare against manifest

Examples:
  ca-capability-analyzer detect src/server.ts
  ca-capability-analyzer banned src/**/*.ts
  ca-capability-analyzer check required_capabilities.json src/**/*.ts
`);
      return subcommand === undefined || subcommand === "--help" || subcommand === "-h" ? 0 : 1;
  }
}

// Run if this is the entry point
const isMainModule = process.argv[1]?.endsWith("cli.ts") || process.argv[1]?.endsWith("cli.js");
if (isMainModule) {
  process.exit(main());
}
