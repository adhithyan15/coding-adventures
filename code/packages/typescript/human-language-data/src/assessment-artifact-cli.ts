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
import { mkdirSync, writeFileSync } from "node:fs";
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
  write: (path: string, contents: string) => void = (path, contents) => {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, contents, "utf8");
  },
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
