/**
 * groups -- print the groups a user is in.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `groups` utility in TypeScript.
 * It prints the group memberships for the current user or specified users.
 *
 * === How groups Works ===
 *
 * With no arguments, it prints the current user's groups:
 *
 *     $ groups
 *     staff admin wheel
 *
 * With a username argument, it prints that user's groups prefixed
 * with the username:
 *
 *     $ groups alice
 *     alice : staff admin
 *
 * === Implementation ===
 *
 * Node.js doesn't provide a direct API to list group names. We use
 * `child_process.execSync` to call the system `id -Gn` command, which
 * is available on all POSIX systems.
 *
 * For the current user, we can also use `process.getgroups()` to get
 * group IDs, but converting those to names requires system calls that
 * Node.js doesn't expose natively.
 *
 * @module groups
 */

import * as path from "node:path";
import { execSync } from "node:child_process";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// Import CLI Builder.
// ---------------------------------------------------------------------------

import { Parser } from "@coding-adventures/cli-builder";

// ---------------------------------------------------------------------------
// Locate the JSON spec file.
// ---------------------------------------------------------------------------

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SPEC_FILE = path.resolve(__dirname, "..", "groups.json");

// ---------------------------------------------------------------------------
// Business Logic: Get group names for a user.
// ---------------------------------------------------------------------------

/**
 * Get the group names for the current user.
 *
 * We shell out to `id -Gn` which prints all group names separated
 * by spaces. This is more reliable than trying to resolve group IDs
 * to names ourselves.
 *
 * @returns An array of group name strings.
 */
export function getCurrentGroups(): string[] {
  try {
    const output = execSync("id -Gn", { encoding: "utf-8" }).trim();
    return output.split(/\s+/);
  } catch {
    return [];
  }
}

/**
 * Get the group names for a specified user.
 *
 * We shell out to `id -Gn <username>` to get the groups for an
 * arbitrary user. If the user doesn't exist, the command will fail
 * and we return null.
 *
 * @param username - The username to look up.
 * @returns An array of group names, or null if the user doesn't exist.
 */
export function getUserGroups(username: string): string[] | null {
  try {
    const output = execSync(`id -Gn ${username}`, {
      encoding: "utf-8",
    }).trim();
    return output.split(/\s+/);
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then print groups.
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
        process.stderr.write(`groups: ${error.message}\n`);
      }
      process.exit(1);
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

  const args = (result as { arguments: Record<string, unknown> }).arguments;
  let users = args["users"] as string[] | string | undefined;

  if (!users || (Array.isArray(users) && users.length === 0)) {
    // No arguments: print current user's groups.
    const groups = getCurrentGroups();
    process.stdout.write(groups.join(" ") + "\n");
  } else {
    if (typeof users === "string") {
      users = [users];
    }

    for (const user of users) {
      const groups = getUserGroups(user);
      if (groups === null) {
        process.stderr.write(`groups: '${user}': no such user\n`);
        process.exitCode = 1;
      } else {
        process.stdout.write(`${user} : ${groups.join(" ")}\n`);
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
