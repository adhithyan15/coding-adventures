// A missing `paper` field is legacy debt, not a valid new full mock. Keep that
// debt in an exact, per-track pin separate from the artifact-path ceiling: the
// latter must not grow merely because old contracts never named their papers.
import { readdirSync, realpathSync } from "node:fs";
import { isAbsolute, relative, resolve, sep } from "node:path";

import type { AssessmentContract, AssessmentFullMock } from "./assessment.js";
import { artifactExists } from "./artifact-presence.js";
import { readLedgerFile } from "./shard.js";

export const PAPER_OMISSION_DIR = "core/assessment-paper-omission-ceiling";
const TRACK_ID = /^[a-z][a-z0-9-]*$/;

export interface PaperOmissionCeiling {
  version: 1;
  language: string;
  omittedMocks: string[];
}

/** JSON tuples avoid collisions even if future mock ids contain delimiters. */
function identity(kind: "level" | "capstone", scope: string, mock: AssessmentFullMock): string {
  return JSON.stringify([kind, scope, mock.id]);
}

/** The exact identities still omitting a learner-facing paper. */
export function omittedPaperMocks(contract: AssessmentContract): string[] {
  const omitted = [
    ...contract.levels.flatMap((level) => level.fullMocks
      .filter((mock) => mock.paper === undefined)
      .map((mock) => identity("level", level.level, mock))),
    ...contract.externalCapstones.flatMap((capstone) => capstone.fullMocks
      .filter((mock) => mock.paper === undefined)
      .map((mock) => identity("capstone", capstone.id, mock))),
  ];
  return omitted.sort();
}

export function serializePaperOmissionCeiling(ceiling: PaperOmissionCeiling): string {
  return `${JSON.stringify({
    version: 1,
    language: ceiling.language,
    note: "Legacy-only ceiling. New full mocks must declare a real paper; this exact set may only fall.",
    omittedMocks: ceiling.omittedMocks,
  }, null, 2)}\n`;
}

export function paperOmissionPath(root: string, language: string): string {
  if (!TRACK_ID.test(language)) throw new Error(`assessment papers: unsafe track id '${language}'`);
  return resolve(root, PAPER_OMISSION_DIR, `${language}.json`);
}

export function readPaperOmissionCeiling(root: string, language: string): PaperOmissionCeiling | null {
  const path = paperOmissionPath(root, language);
  if (!artifactExists(path)) return null;
  // `lstat` sees only the last component. A linked `core/` or ceiling
  // directory could otherwise import a pin from outside the curriculum tree.
  const expectedDir = resolve(realpathSync(root), PAPER_OMISSION_DIR);
  const fromDir = relative(expectedDir, realpathSync(path));
  if (fromDir === ".." || fromDir.startsWith(`..${sep}`) || isAbsolute(fromDir)) {
    throw new Error(`assessment papers: '${path}' resolves outside the omission-ceiling directory`);
  }
  const raw = readLedgerFile<{ version?: unknown; language?: unknown; omittedMocks?: unknown }>(path);
  if (raw.version !== 1 || raw.language !== language) {
    throw new Error(`assessment papers: invalid omission ceiling for ${language}`);
  }
  const entries = raw.omittedMocks;
  if (!Array.isArray(entries)
    || entries.some((entry) => typeof entry !== "string" || entry.trim() === "")
    || new Set(entries).size !== entries.length
    || [...entries].sort().some((entry, index) => entry !== entries[index])) {
    throw new Error(`assessment papers: ${language} omittedMocks must be a sorted unique string array`);
  }
  return { version: 1, language, omittedMocks: [...entries] as string[] };
}

export type PaperOmissionDiagnosticKind =
  | "new-paper-omission"
  | "paper-omission-ceiling-has-fallen"
  | "stale-paper-omission-ceiling-file";

export interface PaperOmissionDiagnostic {
  kind: PaperOmissionDiagnosticKind;
  language: string;
  message: string;
}

/** Compare current omissions to the fixed legacy set; neither swaps nor growth pass. */
export function checkPaperOmissions(
  root: string,
  contracts: Array<{ language: string; contract: AssessmentContract }>,
): { diagnostics: PaperOmissionDiagnostic[]; pinnedOmissions: number } {
  const diagnostics: PaperOmissionDiagnostic[] = [];
  let pinnedOmissions = 0;
  const audited = new Set(contracts.map(({ language }) => language));
  for (const { language, contract } of contracts) {
    const current = new Set(omittedPaperMocks(contract));
    const pinned = new Set(readPaperOmissionCeiling(root, language)?.omittedMocks ?? []);
    for (const mock of current) {
      if (pinned.has(mock)) {
        pinnedOmissions++;
      } else {
        diagnostics.push({
          kind: "new-paper-omission",
          language,
          message: `${language}/assessment.json mock ${mock} omits paper. Write the paper and declare its safe relative path; do not grow ${PAPER_OMISSION_DIR}/${language}.json.`,
        });
      }
    }
    for (const mock of pinned) {
      if (!current.has(mock)) diagnostics.push({
        kind: "paper-omission-ceiling-has-fallen",
        language,
        message: `${PAPER_OMISSION_DIR}/${language}.json still pins ${mock}, which now declares a paper or is gone. Lower the omission ceiling.`,
      });
    }
  }
  const dir = resolve(root, PAPER_OMISSION_DIR);
  if (artifactExists(dir)) {
    for (const name of readdirSync(dir).filter((entry) => entry.endsWith(".json")).sort()) {
      const language = name.slice(0, -5);
      if (!audited.has(language)) diagnostics.push({
        kind: "stale-paper-omission-ceiling-file",
        language,
        message: `${PAPER_OMISSION_DIR}/${name} has no audited assessment contract.`,
      });
      else if (contracts.find((entry) => entry.language === language)
        && omittedPaperMocks(contracts.find((entry) => entry.language === language)!.contract).length === 0) {
        diagnostics.push({
          kind: "stale-paper-omission-ceiling-file",
          language,
          message: `${PAPER_OMISSION_DIR}/${name} remains although every mock declares a paper. Remove the empty pin.`,
        });
      }
    }
  }
  return { diagnostics, pinnedOmissions };
}
