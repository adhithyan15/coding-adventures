import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

/**
 * The audit reports missing declarations, but does not grant a path-based read.
 * Python's existing HL21 boundary opens each descendant relative to a pinned
 * parent handle (NtCreateFile on Windows, dir_fd/O_NOFOLLOW on Unix). Node's
 * path-based fs.open cannot provide that guarantee for a swapped ancestor.
 */
export function readMockEvidence(root: string, track: string, paths: readonly string[]): Map<string, string> {
  if (paths.length === 0) return new Map();
  const script = fileURLToPath(new URL(
    "../../../../learning/human-languages/data/scripts/mock_evidence_read.py", import.meta.url,
  ));
  const python = process.platform === "win32" ? "python" : "python3";
  const result = spawnSync(python, [script], {
    input: JSON.stringify({ root, track, paths }),
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error !== undefined || result.status !== 0) {
    let detail = result.error?.message ?? `reader exit ${result.status}`;
    try {
      const response = JSON.parse(result.stdout) as { error?: { path?: unknown; kind?: unknown } };
      const path = response.error?.path;
      const kind = response.error?.kind;
      if (typeof path === "string" && paths.includes(path)
        && typeof kind === "string" && /^[A-Za-z][A-Za-z0-9]*$/.test(kind)) {
        detail = `${track}/${path}: ${kind}`;
      }
    } catch { /* No structured diagnostic; the read still fails closed. */ }
    throw new Error(
      `mock evidence: anchored read failed for '${track}' (${detail})`,
      { cause: result.error },
    );
  }
  let response: unknown;
  try {
    response = JSON.parse(result.stdout);
  } catch (cause) {
    throw new Error(`mock evidence: invalid anchored-read response for '${track}'`, { cause });
  }
  if (typeof response !== "object" || response === null || Array.isArray(response)) {
    throw new Error(`mock evidence: invalid anchored-read response for '${track}'`);
  }
  const files = (response as Record<string, unknown>)["files"];
  if (typeof files !== "object" || files === null || Array.isArray(files)) {
    throw new Error(`mock evidence: missing anchored-read files for '${track}'`);
  }
  const found = new Map<string, string>();
  for (const path of paths) {
    const text = (files as Record<string, unknown>)[path];
    if (!Object.hasOwn(files, path) || typeof text !== "string") {
      throw new Error(`mock evidence: anchored read omitted '${track}/${path}'`);
    }
    found.set(path, text);
  }
  if (Object.keys(files).length !== found.size) {
    throw new Error(`mock evidence: anchored read returned extra files for '${track}'`);
  }
  return found;
}
