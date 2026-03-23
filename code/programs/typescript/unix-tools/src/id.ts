/**
 * id -- print real and effective user and group IDs.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `id` utility in TypeScript. It
 * prints user and group identity information for the current user (or
 * a specified user).
 *
 * === Default Output ===
 *
 * With no flags, id prints the full identity string:
 *
 *     uid=501(username) gid=20(staff) groups=20(staff),501(access_bpf),...
 *
 * === Flag Modes ===
 *
 * - `-u`: Print only the effective user ID.
 * - `-g`: Print only the effective group ID.
 * - `-G`: Print all group IDs.
 * - `-n`: Print names instead of numbers (used with -u, -g, or -G).
 * - `-r`: Print the real ID instead of the effective ID.
 *
 * === Implementation Notes ===
 *
 * Node.js provides limited user identity information through the `os`
 * module:
 *
 * - `os.userInfo()` gives username, uid, gid.
 * - `process.getuid()` gives the effective user ID.
 * - `process.getgid()` gives the effective group ID.
 * - `process.getgroups()` gives supplementary group IDs.
 *
 * For group names and other details, we shell out to the system `id`
 * command, since Node.js doesn't have native getgrgid() bindings.
 *
 * @module id
 */

import * as os from "node:os";
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
const SPEC_FILE = path.resolve(__dirname, "..", "id.json");

// ---------------------------------------------------------------------------
// Types: User identity information.
// ---------------------------------------------------------------------------

/**
 * Structured identity information for a user.
 *
 * This captures the uid, gid, username, group name, and supplementary
 * group memberships.
 */
export interface IdInfo {
  /** Effective user ID. */
  uid: number;
  /** Effective group ID. */
  gid: number;
  /** Username corresponding to uid. */
  username: string;
  /** Group name corresponding to gid. */
  groupName: string;
  /** All group IDs (including primary). */
  groups: number[];
  /** All group names (parallel array to groups). */
  groupNames: string[];
}

// ---------------------------------------------------------------------------
// Business Logic: Get user identity information.
// ---------------------------------------------------------------------------

/**
 * Retrieve identity information for the current user.
 *
 * We combine Node.js APIs with system commands to get a complete
 * picture:
 *
 * - `os.userInfo()`: username, uid, gid
 * - `process.getgroups()`: supplementary group IDs
 * - `id -Gn`: supplementary group names (via shell)
 *
 * @returns A populated IdInfo object.
 */
export function getUserInfo(): IdInfo {
  const userInfo = os.userInfo();
  const uid = userInfo.uid;
  const gid = userInfo.gid;
  const username = userInfo.username;

  // Get supplementary group IDs.
  let groups: number[] = [];
  try {
    groups = process.getgroups ? process.getgroups() : [gid];
  } catch {
    groups = [gid];
  }

  // Get group names by shelling out to `id`.
  let groupNames: string[] = [];
  let groupName = String(gid);

  try {
    // Get the primary group name.
    groupName = execSync("id -gn", { encoding: "utf-8" }).trim();
    // Get all group names.
    const allNames = execSync("id -Gn", { encoding: "utf-8" }).trim();
    groupNames = allNames.split(/\s+/);
  } catch {
    // Fallback: use numeric IDs as names.
    groupName = String(gid);
    groupNames = groups.map(String);
  }

  return { uid, gid, username, groupName, groups, groupNames };
}

// ---------------------------------------------------------------------------
// Business Logic: Format the default id output.
// ---------------------------------------------------------------------------

/**
 * Format the full identity string (default mode, no flags).
 *
 * Output format matches GNU id:
 *
 *     uid=501(username) gid=20(staff) groups=20(staff),501(access_bpf)
 *
 * @param info - The user identity information.
 * @returns The formatted identity string.
 */
export function formatIdDefault(info: IdInfo): string {
  const uidPart = `uid=${info.uid}(${info.username})`;
  const gidPart = `gid=${info.gid}(${info.groupName})`;

  const groupEntries = info.groups.map((gid, i) => {
    const name = info.groupNames[i] ?? String(gid);
    return `${gid}(${name})`;
  });
  const groupsPart = `groups=${groupEntries.join(",")}`;

  return `${uidPart} ${gidPart} ${groupsPart}`;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then print identity info.
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
        process.stderr.write(`id: ${error.message}\n`);
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

  const flags = (result as { flags: Record<string, unknown> }).flags;

  const showUser = !!flags["user"];
  const showGroup = !!flags["group"];
  const showGroups = !!flags["groups"];
  const showName = !!flags["name"];
  const useZero = !!flags["zero"];
  const separator = useZero ? "\0" : " ";

  const info = getUserInfo();

  if (showUser) {
    // Print effective user ID (or name with -n).
    const value = showName ? info.username : String(info.uid);
    process.stdout.write(value + "\n");
  } else if (showGroup) {
    // Print effective group ID (or name with -n).
    const value = showName ? info.groupName : String(info.gid);
    process.stdout.write(value + "\n");
  } else if (showGroups) {
    // Print all group IDs (or names with -n).
    const values = showName
      ? info.groupNames
      : info.groups.map(String);
    process.stdout.write(values.join(separator) + "\n");
  } else {
    // Default: print full identity string.
    process.stdout.write(formatIdDefault(info) + "\n");
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
