/**
 * Tiny hand-rolled directory walker.
 *
 * v0 supports exactly one glob shape: `**` `/*.<ext>` — recursive
 * file matching with a simple extension filter.  No brace expansion,
 * no character classes, no negation.  The purpose is to keep this
 * stage self-contained without pulling in `fast-glob` or `picomatch`
 * (both fine libraries, but more dependency surface than v0 needs).
 *
 * Returns an async generator yielding absolute file paths in
 * deterministic order (sorted within each directory) — sorted output
 * is what makes pipeline runs reproducible across filesystems with
 * different inode-order guarantees.
 *
 * Skips: dot-files and dot-directories (they're almost always
 * editor/git artefacts, not content), symlinks (cycle hazard),
 * non-regular files (sockets, devices).
 */

import { lstat, readdir } from "node:fs/promises";
import { join, extname } from "node:path";

/**
 * Parse a v0 glob pattern.  Returns the matching extension (with
 * leading dot, lower-case) or throws if the pattern is unsupported.
 */
export function parseGlob(pattern: string): { ext: string } {
  // We accept exactly: "**" "/" "*" "." <ext> — e.g. "**/*.md"
  const match = /^\*\*\/\*\.([a-zA-Z0-9]+)$/.exec(pattern);
  if (!match) {
    throw new Error(
      `forme-source-fs: glob ${JSON.stringify(pattern)} is not supported in v0. ` +
      `Only "**/*.<ext>" patterns work (e.g. "**/*.md").`,
    );
  }
  return { ext: "." + match[1]!.toLowerCase() };
}

/**
 * Walk `root` recursively, yielding absolute paths to regular files
 * whose extension matches `ext`.  Output order is deterministic
 * (sorted lexicographically within each directory).
 */
export async function* walkFiles(
  root: string,
  ext: string,
): AsyncGenerator<string, void, void> {
  // We yield the entries in BFS-like order — process current dir,
  // then recurse into subdirs.  Within each dir, sort entries so
  // output is stable across filesystems.
  let entries: string[];
  try {
    entries = await readdir(root);
  } catch (err) {
    if (isNotFound(err)) return;
    throw err;
  }
  entries.sort();

  const subdirs: string[] = [];
  for (const name of entries) {
    if (name.startsWith(".")) continue; // skip dotfiles + dotdirs
    const path = join(root, name);
    let stats;
    try {
      // Use lstat (not stat) so symlinks report as symbolic — stat
      // would dereference and we'd never see them.  Cycle hazard.
      stats = await lstat(path);
    } catch {
      // Race: file disappeared between readdir and lstat.  Skip.
      continue;
    }
    if (stats.isSymbolicLink()) continue;
    if (stats.isDirectory()) {
      subdirs.push(path);
      continue;
    }
    if (!stats.isFile()) continue;
    if (extname(name).toLowerCase() !== ext) continue;
    yield path;
  }
  for (const dir of subdirs) {
    yield* walkFiles(dir, ext);
  }
}

function isNotFound(err: unknown): boolean {
  return typeof err === "object" && err !== null
    && (err as { code?: unknown }).code === "ENOENT";
}
