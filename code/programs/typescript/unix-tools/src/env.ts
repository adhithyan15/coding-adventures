/**
 * env -- run a program in a modified environment.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the POSIX `env` utility in TypeScript.
 * It prints the current environment variables, or runs a command with
 * a modified environment.
 *
 * === How env Works ===
 *
 * env has two modes of operation:
 *
 * **Print mode** (no command given):
 *
 *     $ env
 *     HOME=/Users/alice
 *     PATH=/usr/bin:/bin
 *     SHELL=/bin/zsh
 *
 * **Run mode** (command given):
 *
 *     $ env FOO=bar myprogram arg1 arg2
 *
 * This runs `myprogram arg1 arg2` with the environment variable FOO
 * set to "bar" in addition to the inherited environment.
 *
 * === Environment Modification ===
 *
 * The arguments before the command name are parsed as either:
 * - `NAME=VALUE` pairs that set environment variables, or
 * - The first argument that doesn't match `NAME=VALUE` starts the command.
 *
 * The -i flag starts with a completely empty environment:
 *
 *     $ env -i HOME=/tmp sh -c 'echo $HOME'
 *     /tmp
 *
 * The -u flag removes specific variables:
 *
 *     $ env -u HOME printenv HOME
 *     (no output -- HOME was removed)
 *
 * === Output Separator ===
 *
 * By default, each variable is printed on its own line (separated by
 * newline). The -0 flag uses NUL (\\0) as the separator instead,
 * which is useful for piping to `xargs -0`.
 *
 * @module env
 */

import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { execSync } from "node:child_process";

// ---------------------------------------------------------------------------
// Import CLI Builder.
// ---------------------------------------------------------------------------

import { Parser } from "@coding-adventures/cli-builder";

// ---------------------------------------------------------------------------
// Locate the JSON spec file.
// ---------------------------------------------------------------------------

const __filename_env = fileURLToPath(import.meta.url);
const __dirname_env = path.dirname(__filename_env);
const SPEC_FILE = path.resolve(__dirname_env, "..", "env.json");

// ---------------------------------------------------------------------------
// Types: Options that control env's behavior.
// ---------------------------------------------------------------------------

/**
 * Configuration for the env command.
 *
 *     Flag   Option               Meaning
 *     ----   ------               -------
 *     -i     ignoreEnvironment    Start with empty environment
 *     -u     unsetVars            List of vars to remove
 *     -0     nullSeparator        Use NUL instead of newline
 *     -C     chdir                Change to directory before running
 */
export interface EnvOptions {
  /** Start with an empty environment (-i). */
  ignoreEnvironment: boolean;
  /** Variables to remove from the environment (-u). */
  unsetVars: string[];
  /** Use NUL instead of newline as output separator (-0). */
  nullSeparator: boolean;
  /** Change to this directory before running the command (-C). */
  chdir: string | null;
}

// ---------------------------------------------------------------------------
// Business Logic: Build a modified environment.
// ---------------------------------------------------------------------------

/**
 * Build a modified environment from a base, applying additions and removals.
 *
 * This function implements the core logic of env:
 * 1. Start with the base environment (or empty if -i was given).
 * 2. Remove any variables specified by -u.
 * 3. Add any NAME=VALUE pairs from the command line.
 *
 * The order matters: removals happen before additions, so you can
 * do `env -u FOO FOO=newvalue` to replace a variable.
 *
 * @param base          The starting environment (typically process.env).
 * @param opts          Options controlling env behavior.
 * @param assignments   Array of "NAME=VALUE" strings to add.
 * @returns             The modified environment as a Record.
 *
 * @example
 * ```ts
 * const env = buildEnvironment(
 *   { HOME: "/Users/alice", PATH: "/usr/bin" },
 *   { ignoreEnvironment: false, unsetVars: ["PATH"], nullSeparator: false, chdir: null },
 *   ["FOO=bar"]
 * );
 * // env === { HOME: "/Users/alice", FOO: "bar" }
 * ```
 */
export function buildEnvironment(
  base: Record<string, string | undefined>,
  opts: EnvOptions,
  assignments: string[]
): Record<string, string> {
  // --- Step 1: Start with base or empty environment --------------------

  let env: Record<string, string> = {};

  if (!opts.ignoreEnvironment) {
    // Copy the base environment (filtering out undefined values).
    for (const [key, value] of Object.entries(base)) {
      if (value !== undefined) {
        env[key] = value;
      }
    }
  }

  // --- Step 2: Remove variables specified by -u ------------------------

  for (const name of opts.unsetVars) {
    delete env[name];
  }

  // --- Step 3: Apply NAME=VALUE assignments ----------------------------
  // Each assignment is a string like "FOO=bar". The first '=' separates
  // the name from the value. Values can contain '=' characters.

  for (const assignment of assignments) {
    const eqIndex = assignment.indexOf("=");
    if (eqIndex > 0) {
      const name = assignment.substring(0, eqIndex);
      const value = assignment.substring(eqIndex + 1);
      env[name] = value;
    }
  }

  return env;
}

// ---------------------------------------------------------------------------
// Business Logic: Parse arguments into assignments and command.
// ---------------------------------------------------------------------------

/**
 * Parse the positional arguments into variable assignments and a command.
 *
 * env's argument parsing is unique: arguments that look like `NAME=VALUE`
 * are environment variable assignments. The first argument that does NOT
 * contain an `=` sign (or doesn't start with a valid variable name before
 * the `=`) begins the command to execute.
 *
 *     env FOO=bar BAZ=qux mycommand arg1 arg2
 *         ^^^^^^^ ^^^^^^^ ^^^^^^^^ ^^^^^^^^
 *         assignments       command  args
 *
 * @param args  The positional arguments from CLI parsing.
 * @returns     An object with `assignments` and `command` arrays.
 *
 * @example
 * ```ts
 * parseArgs(["FOO=bar", "echo", "hello"]);
 * // => { assignments: ["FOO=bar"], command: ["echo", "hello"] }
 * ```
 */
export function parseArgs(
  args: string[]
): { assignments: string[]; command: string[] } {
  const assignments: string[] = [];
  let commandStart = args.length; // Default: no command

  for (let i = 0; i < args.length; i++) {
    // A valid assignment looks like: NAME=VALUE
    // NAME must start with a letter or underscore, followed by
    // letters, digits, or underscores.
    const match = args[i].match(/^([A-Za-z_][A-Za-z0-9_]*)=(.*)$/);
    if (match) {
      assignments.push(args[i]);
    } else {
      commandStart = i;
      break;
    }
  }

  return {
    assignments,
    command: args.slice(commandStart),
  };
}

// ---------------------------------------------------------------------------
// Business Logic: Format environment for printing.
// ---------------------------------------------------------------------------

/**
 * Format environment variables for output.
 *
 * Each variable is displayed as `NAME=VALUE`, separated by either
 * newline (default) or NUL (-0).
 *
 *     HOME=/Users/alice
 *     PATH=/usr/bin
 *     SHELL=/bin/zsh
 *
 * @param env           The environment to format.
 * @param nullSeparator Use NUL instead of newline.
 * @returns             The formatted string.
 */
export function formatEnvironment(
  env: Record<string, string>,
  nullSeparator: boolean
): string {
  const separator = nullSeparator ? "\0" : "\n";
  const lines = Object.entries(env).map(([key, value]) => `${key}=${value}`);

  if (lines.length === 0) {
    return "";
  }

  return lines.join(separator) + separator;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then run env.
// ---------------------------------------------------------------------------

function main(): void {
  let result;

  try {
    const parser = new Parser(SPEC_FILE, process.argv);
    result = parser.parse();
  } catch (err: unknown) {
    if (err && typeof err === "object" && "errors" in err) {
      const errors = (err as { errors: Array<{ message: string }> }).errors;
      for (const error of errors) {
        process.stderr.write(`env: ${error.message}\n`);
      }
      process.exit(125);
    }
    throw err;
  }

  if ("text" in result) {
    process.stdout.write(result.text + "\n");
    process.exit(0);
  }

  if ("version" in result && !("flags" in result)) {
    process.stdout.write(result.version + "\n");
    process.exit(0);
  }

  const flags = result.flags || {};
  const rawArgs: string[] = result.args?.assignments_and_command || [];

  const opts: EnvOptions = {
    ignoreEnvironment: flags.ignore_environment || false,
    unsetVars: Array.isArray(flags.unset) ? flags.unset : (flags.unset ? [flags.unset] : []),
    nullSeparator: flags.null || false,
    chdir: (flags.chdir as string) || null,
  };

  const { assignments, command } = parseArgs(rawArgs);
  const env = buildEnvironment(process.env, opts, assignments);

  if (command.length === 0) {
    // Print mode: just output the environment.
    process.stdout.write(formatEnvironment(env, opts.nullSeparator));
    process.exit(0);
  }

  // Run mode: execute the command with the modified environment.
  try {
    const cmdString = command.map(arg => {
      // Quote arguments that contain spaces.
      if (arg.includes(" ")) return `"${arg}"`;
      return arg;
    }).join(" ");

    const cwd = opts.chdir || process.cwd();

    execSync(cmdString, {
      env,
      cwd,
      stdio: "inherit",
    });
  } catch (err: unknown) {
    if (err && typeof err === "object" && "status" in err) {
      process.exit((err as { status: number }).status || 126);
    }
    process.exit(126);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
