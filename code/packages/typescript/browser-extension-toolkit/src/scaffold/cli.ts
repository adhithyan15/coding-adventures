#!/usr/bin/env node

/**
 * Extension Scaffold CLI
 * ======================
 *
 * Command-line interface for the scaffold generator. Creates a new
 * browser extension project with all the boilerplate wired up.
 *
 * Usage:
 *   npx @coding-adventures/browser-extension-toolkit scaffold <name> [options]
 *
 * Arguments:
 *   name            Extension name in kebab-case (e.g., "my-extension")
 *
 * Options:
 *   --description   Short description (default: "A browser extension")
 *   --gecko-id      Firefox extension ID (default: "<name>@coding-adventures")
 *   --output-dir    Where to create the extension (default: current directory)
 *
 * Example:
 *   npx @coding-adventures/browser-extension-toolkit scaffold my-extension \
 *     --description "Highlights important text on web pages"
 */

import { scaffold } from "./scaffold.js";

/**
 * Parse command-line arguments into a key-value map.
 *
 * Handles two forms:
 * - `--key value` (space-separated)
 * - Positional arguments (collected as unnamed args)
 *
 * This is a minimal parser — no validation, no help text generation.
 * For a real CLI tool, you'd use a library like `commander` or the
 * toolkit's own `cli-builder` package.
 */
function parseArgs(
  argv: string[],
): { positional: string[]; flags: Record<string, string> } {
  const positional: string[] = [];
  const flags: Record<string, string> = {};

  let i = 0;
  while (i < argv.length) {
    const arg = argv[i];

    if (arg.startsWith("--")) {
      const key = arg.slice(2);
      const value = argv[i + 1];
      if (value && !value.startsWith("--")) {
        flags[key] = value;
        i += 2;
      } else {
        flags[key] = "true";
        i += 1;
      }
    } else {
      positional.push(arg);
      i += 1;
    }
  }

  return { positional, flags };
}

/**
 * Main entry point. Reads CLI args and runs the scaffold.
 */
function main(): void {
  // Skip the first two args: `node` and the script path.
  // The third arg should be "scaffold" (the subcommand).
  const rawArgs = process.argv.slice(2);

  // If the first arg is "scaffold", skip it
  const args =
    rawArgs[0] === "scaffold" ? rawArgs.slice(1) : rawArgs;

  const { positional, flags } = parseArgs(args);

  const name = positional[0];
  if (!name) {
    console.error(
      "Usage: extension-scaffold <name> [--description <desc>] [--output-dir <dir>]",
    );
    process.exit(1);
  }

  const description =
    flags["description"] ?? "A browser extension";
  const geckoId = flags["gecko-id"];
  const outputDir = flags["output-dir"];

  try {
    const dir = scaffold({
      name,
      description,
      outputDir,
      geckoId,
    });

    console.log(`Extension created at: ${dir}`);
    console.log("");
    console.log("Next steps:");
    console.log(`  cd ${name}`);
    console.log("  npm install");
    console.log("  npm run dev");
  } catch (error) {
    console.error(
      `Error: ${error instanceof Error ? error.message : String(error)}`,
    );
    process.exit(1);
  }
}

main();
