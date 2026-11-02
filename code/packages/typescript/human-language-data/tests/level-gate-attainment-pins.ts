import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { CEFR_LEVELS, levelRank, type CefrLevel } from "../src/levels.js";

/** One language owns both distinct level verdicts and their current evidence. */
export type LevelProgressOwner = Readonly<{
  attained: CefrLevel | null;
  reach: CefrLevel | null;
  evidence: string;
}>;

export type LevelProgressOwners = Readonly<Record<string, LevelProgressOwner>>;

export type MeasuredLevelProgress = Readonly<{
  attained: CefrLevel | null;
  reach: CefrLevel | null;
}>;

const OWNER_NAME = /^[A-Za-z][A-Za-z0-9-]*\.json$/;
const MAX_EVIDENCE_SCALARS = 500;

const sorted = (values: Iterable<string>): string[] => [...values].sort((a, b) => a.localeCompare(b));

const difference = (left: Iterable<string>, right: ReadonlySet<string>): string[] =>
  sorted([...left].filter((value) => !right.has(value)));

/**
 * Read one exact progress verdict per independently registered track.
 *
 * Identity closure happens before JSON parsing. A malformed surviving owner
 * must not hide the more fundamental deletion of another language's owner,
 * and a directory must not define its own supposedly complete identity set.
 */
export function loadLevelProgressOwners(
  directory: string,
  expectedLanguages: readonly string[],
): LevelProgressOwners {
  if (expectedLanguages.length === 0) {
    throw new Error("level-progress owner set must not be empty");
  }
  if (lstatSync(directory).isSymbolicLink()) {
    throw new Error(`level-progress owner directory must not be a symbolic link: ${directory}`);
  }

  const entries = readdirSync(directory, { withFileTypes: true });
  const folded = new Set<string>();
  entries.sort((left, right) => left.name.localeCompare(right.name));
  for (const entry of entries) {
    if (!OWNER_NAME.test(entry.name)) {
      throw new Error(`unsafe level-progress owner '${entry.name}'`);
    }
    if (entry.isSymbolicLink() || !entry.isFile()) {
      throw new Error(`level-progress owner must be a real file: ${entry.name}`);
    }
    const language = entry.name.slice(0, -".json".length);
    const key = language.toLocaleLowerCase("en-US");
    if (folded.has(key)) {
      throw new Error(`level-progress owners collide when case-folded: ${entry.name}`);
    }
    folded.add(key);
  }

  for (const entry of entries) {
    const language = entry.name.slice(0, -".json".length);
    const key = language.toLocaleLowerCase("en-US");
    if (language !== key) {
      throw new Error(`unsafe level-progress owner '${entry.name}': use lowercase`);
    }
  }

  const expectedNames = new Set(
    expectedLanguages.map((language) => {
      if (!/^[a-z][a-z0-9-]*$/.test(language)) {
        throw new Error(`unsafe expected level-progress language '${language}'`);
      }
      return `${language}.json`;
    }),
  );
  if (expectedNames.size !== expectedLanguages.length) {
    throw new Error("expected level-progress languages must be unique");
  }
  const foundNames = new Set(entries.map((entry) => entry.name));
  const missing = difference(expectedNames, foundNames);
  const extra = difference(foundNames, expectedNames);
  if (missing.length > 0 || extra.length > 0) {
    const details = [
      missing.length > 0 ? `missing ${missing.join(", ")}` : "",
      extra.length > 0 ? `unexpected ${extra.join(", ")}` : "",
    ].filter(Boolean);
    throw new Error(`level-progress owner identity mismatch: ${details.join("; ")}`);
  }

  const owners: Record<string, LevelProgressOwner> = {};
  for (const entry of entries) {
    const language = entry.name.slice(0, -".json".length);
    const parsed: unknown = JSON.parse(readFileSync(join(directory, entry.name), "utf8"));
    if (
      typeof parsed !== "object" ||
      parsed === null ||
      Array.isArray(parsed) ||
      sorted(Object.keys(parsed)).join("\0") !== "attained\0evidence\0reach"
    ) {
      throw new Error(`${entry.name}: expected exactly 'attained', 'reach', and 'evidence' fields`);
    }
    const { attained, reach, evidence } = parsed as Record<string, unknown>;
    if (attained !== null && !CEFR_LEVELS.includes(attained as CefrLevel)) {
      throw new Error(`${entry.name}: unsafe attained level ${JSON.stringify(attained)}`);
    }
    if (reach !== null && !CEFR_LEVELS.includes(reach as CefrLevel)) {
      throw new Error(`${entry.name}: unsafe reach level ${JSON.stringify(reach)}`);
    }
    if (typeof evidence !== "string" || evidence.trim().length === 0) {
      throw new Error(`${entry.name}: evidence must be a non-empty string`);
    }
    if ([...evidence].length > MAX_EVIDENCE_SCALARS) {
      throw new Error(`${entry.name}: evidence must be at most ${MAX_EVIDENCE_SCALARS} Unicode scalars`);
    }
    if (
      attained !== null &&
      (reach === null || levelRank(reach as CefrLevel) < levelRank(attained as CefrLevel))
    ) {
      throw new Error(`${entry.name}: reach ${String(reach)} ranks below attained ${String(attained)}`);
    }
    owners[language] = {
      attained: attained as CefrLevel | null,
      reach: reach as CefrLevel | null,
      evidence: evidence.trim(),
    };
  }
  return owners;
}

/** Compare independent pins with the measured report using actionable diagnostics. */
export function assertLevelProgressOwners(
  owners: LevelProgressOwners,
  measured: Readonly<Record<string, MeasuredLevelProgress>>,
): void {
  const ownerLanguages = new Set(Object.keys(owners));
  const measuredLanguages = new Set(Object.keys(measured));
  const missing = difference(ownerLanguages, measuredLanguages);
  const extra = difference(measuredLanguages, ownerLanguages);
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `measured level-progress identity mismatch: ` +
        [
          missing.length > 0 ? `missing ${missing.join(", ")}` : "",
          extra.length > 0 ? `unexpected ${extra.join(", ")}` : "",
        ]
          .filter(Boolean)
          .join("; "),
    );
  }

  for (const language of sorted(ownerLanguages)) {
    const owner = owners[language]!;
    const actual = measured[language]!;
    for (const field of ["attained", "reach"] as const) {
      if (owner[field] !== actual[field]) {
        throw new Error(
          `${language} ${field}: expected ${String(owner[field])}, actual ${String(actual[field])}`,
        );
      }
    }
  }
}
