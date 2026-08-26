// ---------------------------------------------------------------------------
// assessment-artifact-cli.ts — the CI face of the dangling-reference ratchet.
//
//   --check  fail if any assessment contract promises a file that is not there
//            and is not pinned, or if a pinned debt has quietly been paid.
//   --write  regenerate the per-track ceilings from the corpus as it stands.
//
// `--check` prints its measurement on SUCCESS as well as on failure. A gate
// whose green output is silence cannot be told apart from a gate that was never
// invoked, and this repository has shipped several of those.
// ---------------------------------------------------------------------------
import { lstatSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import {
  ARTIFACT_CEILING_DIR,
  auditAssessmentArtifacts,
  checkAssessmentArtifacts,
  ceilingPath,
  generatedCeilings,
  renderArtifactCheck,
} from "./assessment-artifacts.js";
import { defaultCurriculumRoot } from "./loader.js";

/** `lstat`, or `undefined` if there is nothing there. Never follows a link. */
function entryIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch (error) {
    const code = (error as NodeJS.ErrnoException | null)?.code;
    if (code === "ENOENT" || code === "ENOTDIR") return undefined;
    throw error;
  }
}

/**
 * Write a ceiling shard, refusing to write THROUGH anything.
 *
 * `ceilingPath` validates the track id and calls `resolve`, but `resolve` is
 * purely LEXICAL — it cannot see a symlink, and `writeFileSync` follows one. A
 * committed `core/assessment-artifact-ceiling/spanish.json -> ../../../../.git/hooks/post-checkout`
 * would turn an ordinary `npm run generate:assessment-artifacts` into an
 * arbitrary write, and the same trick on the DIRECTORY makes
 * `mkdirSync(..., { recursive: true })` succeed silently against a link and put
 * every shard outside the tree.
 *
 * `book-cli.ts` already carries this guard for the generated `.tex`, and
 * `shard.ts` states the rule it comes from: a guard living only inside the
 * reader is a guard the writer forgets. `readCeiling`'s read side is covered by
 * `assertRealFile`; this is the write side.
 */
function writeCeilingFile(path: string, contents: string): void {
  const directory = dirname(path);
  const existingDirectory = entryIfPresent(directory);
  if (existingDirectory && !existingDirectory.isDirectory()) {
    throw new Error(
      `assessment artifacts: '${directory}' is not a real directory — refusing to write through it`,
    );
  }
  mkdirSync(directory, { recursive: true });
  const existing = entryIfPresent(path);
  if (existing && !existing.isFile()) {
    throw new Error(
      `assessment artifacts: '${path}' is not a regular file — refusing to write through it`,
    );
  }
  writeFileSync(path, contents, "utf8");
}

/** Ordered so the failure an author must ACT on is read first. */
const KIND_ORDER = [
  "audit-went-blind",
  "new-dangling-reference",
  "ceiling-has-fallen",
  "stale-ceiling-file",
] as const;

export function runAssessmentArtifactCli(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
  write: (path: string, contents: string) => void = writeCeilingFile,
  out: (text: string) => void = (text) => process.stdout.write(text),
  err: (text: string) => void = (text) => process.stderr.write(text),
): number {
  const mode = args.length === 1 ? args[0] : undefined;
  if (mode !== "--check" && mode !== "--write") {
    err("usage: assessment-artifact-cli (--check | --write)\n");
    return 2;
  }

  if (mode === "--write") {
    const outputs = generatedCeilings(auditAssessmentArtifacts(root));
    for (const [relative, contents] of outputs) {
      const language = relative.slice(`${ARTIFACT_CEILING_DIR}/`.length, -".json".length);
      write(ceilingPath(root, language), contents);
      out(`generated ${relative}\n`);
    }
    out(`${outputs.size} ceiling file(s) written\n`);
    return 0;
  }

  const result = checkAssessmentArtifacts(root);
  out(`${renderArtifactCheck(result).join("\n")}\n`);
  if (result.diagnostics.length === 0) {
    out(`OK — no unpinned dangling assessment-contract references.\n`);
    return 0;
  }
  const sorted = [...result.diagnostics].sort(
    (a, b) =>
      KIND_ORDER.indexOf(a.kind) - KIND_ORDER.indexOf(b.kind)
      || a.language.localeCompare(b.language)
      || a.message.localeCompare(b.message),
  );
  for (const diagnostic of sorted) err(`${diagnostic.kind}: ${diagnostic.message}\n`);
  err(`\n${sorted.length} assessment-artifact problem(s).\n`);
  return 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runAssessmentArtifactCli());
}
