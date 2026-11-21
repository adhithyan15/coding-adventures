/**
 * File discovery and traversal with security quotas and symlink rejection.
 */

import * as fs from "node:fs";
import * as path from "node:path";

export const MAX_FILES_LIMIT = 1_000;
export const MAX_FILE_SIZE_BYTES = 2 * 1024 * 1024; // 2 MiB

const IGNORED_DIRS = new Set([
  "node_modules",
  "dist",
  "dist-test",
  ".git",
  "coverage",
  ".turbo",
  "build",
  "target",
  ".pytest_cache",
]);

const SOURCE_EXTENSIONS = new Set([".ts", ".tsx", ".js", ".mjs", ".cjs"]);

export function discoverSourceFiles(
  rootDir: string,
  options: { includeTests?: boolean } = {}
): string[] {
  if (typeof rootDir !== "string" || rootDir.length === 0 || rootDir.includes("\0")) {
    throw new Error("Invalid root directory path");
  }
  if (options !== undefined && (typeof options !== "object" || options === null || Array.isArray(options))) {
    throw new TypeError("options must be an object");
  }

  const discovered: string[] = [];

  const MAX_DIRECTORY_DEPTH = 32;

  function walk(currentDir: string, depth: number = 0): void {
    if (depth > MAX_DIRECTORY_DEPTH) {
      throw new Error(`Directory traversal exceeded maximum depth of ${MAX_DIRECTORY_DEPTH}`);
    }
    if (discovered.length >= MAX_FILES_LIMIT) {
      throw new Error(`Exceeded maximum file discovery quota of ${MAX_FILES_LIMIT} files`);
    }

    let entries: fs.Dirent[];
    try {
      entries = fs.readdirSync(currentDir, { withFileTypes: true });
    } catch (err) {
      throw new Error(`Failed to read directory at ${currentDir}: ${String(err)}`);
    }

    entries.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));

    for (const entry of entries) {
      if (entry.isSymbolicLink()) {
        // Refuse symlinks for security
        continue;
      }

      const fullPath = path.join(currentDir, entry.name);

      if (entry.isDirectory()) {
        if (!IGNORED_DIRS.has(entry.name)) {
          walk(fullPath, depth + 1);
        }
      } else if (entry.isFile()) {
        const ext = path.extname(entry.name).toLowerCase();
        if (SOURCE_EXTENSIONS.has(ext)) {
          if (!options.includeTests) {
            if (
              entry.name.includes(".test.") ||
              entry.name.includes(".spec.") ||
              currentDir.split(path.sep).includes("tests")
            ) {
              continue;
            }
          }

          // Check file size limit using lstatSync to reject symlinks (CWE-59 / CWE-367)
          const stat = fs.lstatSync(fullPath);
          if (stat.isSymbolicLink() || !stat.isFile()) {
            continue;
          }
          if (stat.size > MAX_FILE_SIZE_BYTES) {
            throw new Error(`File ${fullPath} exceeds maximum size limit of ${MAX_FILE_SIZE_BYTES} bytes`);
          }

          if (discovered.length >= MAX_FILES_LIMIT) {
            throw new Error(`Exceeded maximum file discovery quota of ${MAX_FILES_LIMIT} files`);
          }

          discovered.push(fullPath);
        }
      }
    }
  }

  const rootStat = fs.lstatSync(rootDir);
  if (rootStat.isSymbolicLink()) {
    throw new Error(`Root directory cannot be a symbolic link: ${rootDir}`);
  }

  if (rootStat.isFile()) {
    if (rootStat.size > MAX_FILE_SIZE_BYTES) {
      throw new Error(`File ${rootDir} exceeds maximum size limit of ${MAX_FILE_SIZE_BYTES} bytes`);
    }
    return [rootDir];
  }
  if (!rootStat.isDirectory()) {
    throw new Error(`Root path must be a regular file or directory: ${rootDir}`);
  }

  walk(rootDir);
  return discovered;
}
