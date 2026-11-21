/**
 * Command-line interface for ca-capability-analyzer.
 */

import * as path from "node:path";
import * as process from "node:process";
import { analyzePackage } from "./analyzer.js";
import { formatReportText, formatReportJson } from "./report.js";

export const VERSION = "0.1.0";

export function runCli(argv: string[]): number {
  if (!Array.isArray(argv)) {
    console.error("Error: argv must be an array of strings");
    return 2;
  }

  let targetDir = ".";
  let strict = false;
  let jsonOutput = false;
  let verbose = false;
  let includeTests = false;

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (typeof arg !== "string" || arg.includes("\0")) {
      console.error("Error: invalid argument");
      return 2;
    }
    if (arg === "--help" || arg === "-h") {
      printHelp();
      return 0;
    }
    if (arg === "--version" || arg === "-v") {
      console.log(`ca-capability-analyzer v${VERSION}`);
      return 0;
    }
    if (arg === "--strict") {
      strict = true;
    } else if (arg === "--json") {
      jsonOutput = true;
    } else if (arg === "--verbose") {
      verbose = true;
    } else if (arg === "--include-tests") {
      includeTests = true;
    } else if (arg === "--dir") {
      i++;
      if (i >= argv.length || typeof argv[i] !== "string" || argv[i].includes("\0") || argv[i].length === 0) {
        console.error("Error: --dir requires a valid non-empty path argument");
        return 2;
      }
      targetDir = argv[i];
    } else if (arg.startsWith("--dir=")) {
      const val = arg.slice(6);
      if (val.length === 0) {
        console.error("Error: --dir requires a non-empty path argument");
        return 2;
      }
      targetDir = val;
    } else if (!arg.startsWith("-")) {
      if (arg.length === 0) {
        console.error("Error: target directory path cannot be empty");
        return 2;
      }
      targetDir = arg;
    } else {
      console.error(`Error: unknown option '${arg}'`);
      printHelp();
      return 2;
    }
  }

  const resolvedDir = path.resolve(targetDir);

  try {
    const result = analyzePackage(resolvedDir, {
      strict,
      verbose,
      includeTests,
    });

    if (jsonOutput) {
      console.log(formatReportJson(result));
    } else {
      console.log(formatReportText(result, verbose));
    }

    return result.ok ? 0 : 1;
  } catch (err) {
    console.error(`Error during analysis: ${String(err)}`);
    return 2;
  }
}

function printHelp(): void {
  console.log(`
ca-capability-analyzer v${VERSION}
Static capability analyzer and VER15 CWE detector for TypeScript/JavaScript.

USAGE:
    ca-capability-analyzer [OPTIONS] [DIRECTORY]

OPTIONS:
    --dir <PATH>       Target package directory to analyze (default: current directory)
    --strict           Strict mode: fail on any undeclared capability or unexempt CWE weakness
    --json             Output analysis result in JSON format
    --verbose          Show detailed file-by-file evidence
    --include-tests    Include test files in analysis (by default excluded)
    -h, --help         Print help information
    -v, --version      Print version information

EXIT CODES:
    0    Pass: all detected capabilities are declared, no unexempt CWEs
    1    Violations found: undeclared capability or CWE in strict mode
    2    Error: invalid arguments, missing path, or read failure
`);
}
