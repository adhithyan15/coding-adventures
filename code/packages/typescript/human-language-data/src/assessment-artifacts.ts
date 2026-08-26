// ---------------------------------------------------------------------------
// assessment-artifacts.ts — a contract may not promise a file that is not there.
//
// `<track>/assessment.json` is HL16's evidence contract. It says, per level,
// which task-shape inventory each of the four skills is measured against and
// which timed mock, rubric and answer key constitute the proof. Every one of
// those is a PATH into the track directory.
//
// Nothing checked that the paths led anywhere. Measured on 2026-08-26 across
// all 23 registered tracks: 13 carry a contract, all 13 dangle, and between
// them they name 351 distinct artifacts that do not exist — 276 mock papers,
// rubrics and answer keys, and 75 task-shape inventories. Spanish alone
// declares `mocks/a1/rubric.md`, `mocks/a1/mock-1-answer-key.md` and 24 more.
// No `mocks/` directory exists in any track in the repository.
//
// Every gate was green, because the parser checks that `rubric` is a non-empty
// string and nothing checks that the string is a file. That is the same shape
// as HL20 §1's flattering failure: presence of a CLAIM counting as presence of
// the THING. A contract that names an answer key nobody wrote is not weaker
// evidence than one that names nothing — it is worse, because it reads as
// stronger.
//
// ## Ceiling, not licence
//
// Failing on all 351 today would make `main` permanently red and block every
// unrelated change, because the missing artifacts are years of authoring work
// (a C2 DELE mock for Persian is not a merge away). So the known set is PINNED,
// per track, under `core/assessment-artifact-ceiling/`, and the pin is a
// ceiling in the strict sense:
//
//   - a reference that dangles and is NOT pinned is a hard error. New debt
//     cannot be added, in any track, ever;
//   - a pinned path that now EXISTS is also an error, telling the author to
//     lower the pin. A ceiling nobody is forced to lower rots into a floor,
//     and the next reader cannot tell the difference between "still owed" and
//     "paid years ago";
//   - the pin is the SET of paths, not a count of them. A count is satisfied by
//     paying one debt and taking out another, which is the trade this file
//     exists to make impossible.
//
// Per-track shards rather than one ledger, for the reason HL21 already gives
// for the level snapshots: thirteen independent authors closing thirteen
// independent debts must not collide on one file.
// ---------------------------------------------------------------------------
import { readdirSync } from "node:fs";
import { join, resolve } from "node:path";

import type { AssessmentContract, AssessmentSkill } from "./assessment.js";
import { artifactExists, type StatProbe } from "./artifact-presence.js";
import { defaultCurriculumRoot, loadAssessmentContracts } from "./loader.js";
import { readLedgerFile } from "./shard.js";

/** Where the per-track ceilings live, relative to the curriculum root. */
export const ARTIFACT_CEILING_DIR = "core/assessment-artifact-ceiling";

/** A track id, repeated here rather than imported so this module joins no path it has not vetted. */
const TRACK_ID = /^[a-z][a-z0-9-]*$/;

/** One path a contract promises, and the field that promised it. */
export interface ArtifactReference {
  /** Dotted location inside the contract, e.g. `A1.fullMocks[a1-mock-1].answerKey`. */
  where: string;
  /** The raw value, fragment included: `task-shapes/a1.json#reading`. */
  reference: string;
  /** The file half — everything before the first `#`. This is what must exist. */
  path: string;
}

export interface TrackArtifactAudit {
  language: string;
  /** Every reference the contract makes, in declaration order. */
  references: ArtifactReference[];
  /** Distinct paths that exist. */
  present: string[];
  /** Distinct paths that do not, sorted. */
  missing: string[];
}

export interface AssessmentArtifactReport {
  tracks: TrackArtifactAudit[];
  summary: {
    /** Tracks holding a parseable contract. Absence of a contract is backlog, not debt. */
    tracksAudited: number;
    tracksWithMissingArtifacts: number;
    /** Reference occurrences, so a rubric shared by two mocks counts twice. */
    referencesChecked: number;
    /** Distinct paths, corpus-wide. Two tracks owing `mocks/a1/rubric.md` are two. */
    distinctArtifactsDeclared: number;
    distinctArtifactsMissing: number;
  };
}

/**
 * Every artifact path a contract promises.
 *
 * Pure: no filesystem, so the enumeration can be asserted on its own. Both
 * halves of the contract are walked — the CEFR ladder and the non-CEFR external
 * capstones — because a capstone's mock is exactly as absent as a level's.
 *
 * The fragment is stripped, not validated. `task-shapes/a1.json#reading` names
 * a section inside a file; whether that section exists is `task-shapes.ts`'s
 * question, and this module is only asking whether the file does. Conflating
 * the two would make a missing FILE and a missing SECTION report identically,
 * and they need different work to fix.
 */
export function collectArtifactReferences(contract: AssessmentContract): ArtifactReference[] {
  const out: ArtifactReference[] = [];
  const add = (where: string, reference: string): void => {
    out.push({ where, reference, path: reference.split("#", 1)[0] ?? "" });
  };

  for (const level of contract.levels) {
    for (const skill of Object.keys(level.skills) as AssessmentSkill[]) {
      level.skills[skill].taskInventory.forEach((reference, index) => {
        add(`${level.level}.skills.${skill}.taskInventory[${index}]`, reference);
      });
    }
    for (const [id, component] of Object.entries(level.additionalComponents)) {
      component.taskInventory.forEach((reference, index) => {
        add(`${level.level}.additionalComponents.${id}.taskInventory[${index}]`, reference);
      });
    }
    for (const mock of level.fullMocks) {
      add(`${level.level}.fullMocks[${mock.id}].rubric`, mock.rubric);
      add(`${level.level}.fullMocks[${mock.id}].answerKey`, mock.answerKey);
    }
  }

  for (const capstone of contract.externalCapstones) {
    for (const skill of Object.keys(capstone.skills) as AssessmentSkill[]) {
      capstone.skills[skill].taskInventory.forEach((reference, index) => {
        add(`capstone[${capstone.id}].skills.${skill}.taskInventory[${index}]`, reference);
      });
    }
    for (const [id, component] of Object.entries(capstone.additionalComponents)) {
      component.taskInventory.forEach((reference, index) => {
        add(`capstone[${capstone.id}].additionalComponents.${id}.taskInventory[${index}]`, reference);
      });
    }
    for (const mock of capstone.fullMocks) {
      add(`capstone[${capstone.id}].fullMocks[${mock.id}].rubric`, mock.rubric);
      add(`capstone[${capstone.id}].fullMocks[${mock.id}].answerKey`, mock.answerKey);
    }
  }

  return out;
}

/**
 * Which of one track's promised artifacts are on disk.
 *
 * `parseAssessmentContract` has already rejected every reference that is not a
 * safe relative path — no leading `/`, no `D:\`, no `..` segment, no URL scheme
 * — so the `join` below cannot be walked out of the track directory. That check
 * lives in the parser rather than here on purpose: a path this module refused
 * would still have been WRITTEN, and the place to refuse it is where it is read.
 */
export function auditTrackArtifacts(
  root: string,
  language: string,
  contract: AssessmentContract,
  probe?: StatProbe,
): TrackArtifactAudit {
  if (!TRACK_ID.test(language)) {
    throw new Error(`assessment artifacts: unsafe track id '${language}'`);
  }
  const references = collectArtifactReferences(contract);
  const present: string[] = [];
  const missing: string[] = [];
  for (const path of [...new Set(references.map((reference) => reference.path))].sort()) {
    if (artifactExists(join(root, language, path), probe)) present.push(path);
    else missing.push(path);
  }
  return { language, references, present, missing };
}

/** Audit every track that holds an assessment contract. */
export function auditAssessmentArtifacts(
  root = defaultCurriculumRoot(),
  probe?: StatProbe,
): AssessmentArtifactReport {
  const tracks = loadAssessmentContracts(root).map(({ language, contract }) =>
    auditTrackArtifacts(root, language, contract, probe),
  );
  return {
    tracks,
    summary: {
      tracksAudited: tracks.length,
      tracksWithMissingArtifacts: tracks.filter((track) => track.missing.length > 0).length,
      referencesChecked: tracks.reduce((total, track) => total + track.references.length, 0),
      distinctArtifactsDeclared: tracks.reduce(
        (total, track) => total + track.present.length + track.missing.length,
        0,
      ),
      distinctArtifactsMissing: tracks.reduce((total, track) => total + track.missing.length, 0),
    },
  };
}

// --- the ceiling ----------------------------------------------------------

export interface ArtifactCeiling {
  version: 1;
  language: string;
  /**
   * Paths this track's contract promises and has not yet produced.
   *
   * Sorted, distinct, and exhaustive: the check treats anything outside this
   * list as new debt and fails.
   */
  unbuiltArtifacts: string[];
}

const CEILING_NOTE =
  "GENERATED — do not hand-edit. Regenerate with `npm run generate:assessment-artifacts`. " +
  "This is a CEILING on unbuilt assessment artifacts (HL16/HL20): it may fall and may never grow. " +
  "Adding a contract reference to a file that does not exist fails CI; building one of these " +
  "artifacts also fails CI until this list is regenerated to drop it.";

export function serializeCeiling(ceiling: ArtifactCeiling): string {
  return `${JSON.stringify(
    { version: 1, language: ceiling.language, note: CEILING_NOTE, unbuiltArtifacts: ceiling.unbuiltArtifacts },
    null,
    2,
  )}\n`;
}

export function ceilingPath(root: string, language: string): string {
  if (!TRACK_ID.test(language)) {
    throw new Error(`assessment artifacts: unsafe track id '${language}'`);
  }
  return resolve(root, ARTIFACT_CEILING_DIR, `${language}.json`);
}

/**
 * The pinned ceiling for one track, or `null` when no pin has been written.
 *
 * A missing pin is not an empty pin. It means "this track has never been
 * pinned", which for a track that dangles is itself the failure — otherwise
 * deleting the pin would be a way to launder new debt.
 */
export function readCeiling(root: string, language: string): ArtifactCeiling | null {
  const path = ceilingPath(root, language);
  if (!artifactExists(path)) return null;
  const raw = readLedgerFile<{ version?: unknown; language?: unknown; unbuiltArtifacts?: unknown }>(path);
  if (raw.version !== 1) throw new Error(`assessment artifacts: ${language} ceiling version must be 1`);
  if (raw.language !== language) {
    throw new Error(`assessment artifacts: ${language} ceiling declares language '${String(raw.language)}'`);
  }
  if (
    !Array.isArray(raw.unbuiltArtifacts)
    || raw.unbuiltArtifacts.some((entry) => typeof entry !== "string" || entry.trim() === "")
  ) {
    throw new Error(`assessment artifacts: ${language} ceiling unbuiltArtifacts must be a string array`);
  }
  return { version: 1, language, unbuiltArtifacts: [...raw.unbuiltArtifacts] as string[] };
}

/** What the pins SHOULD say, given the corpus as it stands. */
export function generatedCeilings(report: AssessmentArtifactReport): Map<string, string> {
  return new Map(
    [...report.tracks]
      .filter((track) => track.missing.length > 0)
      .sort((a, b) => a.language.localeCompare(b.language))
      .map((track) => [
        `${ARTIFACT_CEILING_DIR}/${track.language}.json`,
        serializeCeiling({ version: 1, language: track.language, unbuiltArtifacts: track.missing }),
      ]),
  );
}

export type ArtifactDiagnosticKind =
  /** A dangling reference nobody pinned. The failure this module exists for. */
  | "new-dangling-reference"
  /** A pinned debt that has been paid, or is no longer referenced. Lower the ceiling. */
  | "ceiling-has-fallen"
  /** A pin file for a track with no contract, or none at all. */
  | "stale-ceiling-file"
  /** The audit examined nothing. A gate that cannot see is not a gate that passed. */
  | "audit-went-blind";

export interface ArtifactDiagnostic {
  kind: ArtifactDiagnosticKind;
  language: string;
  message: string;
}

export interface ArtifactCheckResult {
  report: AssessmentArtifactReport;
  diagnostics: ArtifactDiagnostic[];
  /** Pinned, still-unbuilt artifacts — printed on success so a green run says a number. */
  pinnedUnbuilt: number;
}

/**
 * Run the ratchet.
 *
 * Three failure kinds, deliberately distinct, because they need opposite
 * responses and a single "mismatch" message would send an author to the wrong
 * one. `new-dangling-reference` says WRITE THE FILE and must never be answered
 * by regenerating the pin; `ceiling-has-fallen` says REGENERATE THE PIN and must
 * never be answered by deleting the artifact.
 */
export function checkAssessmentArtifacts(
  root = defaultCurriculumRoot(),
  probe?: StatProbe,
): ArtifactCheckResult {
  const report = auditAssessmentArtifacts(root, probe);
  const diagnostics: ArtifactDiagnostic[] = [];

  // Anti-vacuity, first. Every gate in this repository that has ever shipped
  // broken shipped by measuring nothing and reporting success — "compiled 0,
  // skipped 1, failed 0". If the registry, the loader, or a path change stops
  // this from finding contracts, that is a failure, not a clean bill.
  if (report.summary.tracksAudited === 0 || report.summary.referencesChecked === 0) {
    diagnostics.push({
      kind: "audit-went-blind",
      language: "-",
      message:
        `audited ${report.summary.tracksAudited} contract(s) and ${report.summary.referencesChecked} ` +
        `reference(s). A run that inspects nothing cannot pass.`,
    });
    return { report, diagnostics, pinnedUnbuilt: 0 };
  }

  let pinnedUnbuilt = 0;
  const audited = new Set(report.tracks.map((track) => track.language));
  /** Tracks whose pin already produced a per-path `ceiling-has-fallen`. */
  const alreadyToldToRegenerate = new Set<string>();

  for (const track of report.tracks) {
    const ceiling = readCeiling(root, track.language);
    const pinned = new Set(ceiling?.unbuiltArtifacts ?? []);
    const missing = new Set(track.missing);

    for (const path of track.missing) {
      if (pinned.has(path)) {
        pinnedUnbuilt += 1;
        continue;
      }
      const where = track.references.find((reference) => reference.path === path)?.where ?? "?";
      diagnostics.push({
        kind: "new-dangling-reference",
        language: track.language,
        message:
          `${track.language}/assessment.json ${where} promises '${path}', which does not exist. ` +
          `Write the artifact. Do NOT add it to ${ARTIFACT_CEILING_DIR}/${track.language}.json — ` +
          `that ceiling records debt taken on before this gate existed and may never grow.`,
      });
    }

    for (const path of pinned) {
      if (missing.has(path)) continue;
      const reason = track.present.includes(path)
        ? "now exists"
        : "is no longer referenced by the contract";
      alreadyToldToRegenerate.add(track.language);
      diagnostics.push({
        kind: "ceiling-has-fallen",
        language: track.language,
        message:
          `${ARTIFACT_CEILING_DIR}/${track.language}.json still pins '${path}', which ${reason}. ` +
          `Lower the ceiling: npm run generate:assessment-artifacts`,
      });
    }
  }

  // A pin belonging to no audited track. Either the track lost its contract or
  // somebody wrote a ceiling by hand for a track that never had one.
  const dir = resolve(root, ARTIFACT_CEILING_DIR);
  const files = artifactExists(dir)
    ? readdirSync(dir).filter((name) => name.endsWith(".json")).sort()
    : [];
  for (const name of files) {
    const language = name.slice(0, -".json".length);
    if (!audited.has(language)) {
      diagnostics.push({
        kind: "stale-ceiling-file",
        language,
        message:
          `${ARTIFACT_CEILING_DIR}/${name} pins a track with no loadable assessment contract. ` +
          `Lower the ceiling: npm run generate:assessment-artifacts`,
      });
      continue;
    }
    const track = report.tracks.find((entry: TrackArtifactAudit) => entry.language === language)!;
    // Only when the per-path loop above said nothing. A track that paid every
    // debt while its pin still lists them has already been told, once per path,
    // to regenerate; repeating the instruction at file granularity would double
    // every line of a fully-closed track's failure output.
    if (track.missing.length === 0 && !alreadyToldToRegenerate.has(language)) {
      diagnostics.push({
        kind: "stale-ceiling-file",
        language,
        message:
          `${ARTIFACT_CEILING_DIR}/${name} remains although ${language} now builds every artifact ` +
          `its contract promises. Delete it: npm run generate:assessment-artifacts`,
      });
    }
  }

  // A track that dangles with no pin at all is already reported above, one
  // diagnostic per path, which is the right granularity: the author needs the
  // list of files, not the fact that a file is absent.

  return { report, diagnostics, pinnedUnbuilt };
}

/** One line per track plus a corpus total — the positive reading a green run must print. */
export function renderArtifactCheck(result: ArtifactCheckResult): string[] {
  const { report } = result;
  const lines = [
    `Assessment-contract artifacts (HL16/HL20)`,
    `  ${report.summary.tracksAudited} contract(s) audited, ` +
      `${report.summary.referencesChecked} reference(s) resolved, ` +
      `${report.summary.distinctArtifactsDeclared} distinct artifact(s) declared`,
    `  ${report.summary.distinctArtifactsMissing} unbuilt across ` +
      `${report.summary.tracksWithMissingArtifacts} track(s); ${result.pinnedUnbuilt} of them pinned`,
  ];
  for (const track of report.tracks) {
    lines.push(
      `  ${track.language.padEnd(10)} ${String(track.present.length).padStart(3)} built, ` +
        `${String(track.missing.length).padStart(3)} unbuilt`,
    );
  }
  return lines;
}
