import { lstatSync, readdirSync, realpathSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { REINFORCEMENT_WINDOWS, type WindowName } from "./continuity.js";
import {
  GENTLE_RAMP_PRIORITIES,
  type GentleRampFinding,
  type GentleRampFindingKind,
  type TrackGentleRamp,
} from "./gentle-ramp.js";
import { isAbsentErrno, readLedgerFileWithSource } from "./shard.js";

export const GENTLE_RAMP_SNAPSHOT_DIR = "core/gentle-ramp-snapshots";
export const GENTLE_RAMP_OWNER_VERSION = 1;
export const GENTLE_RAMP_META_OWNER = "_meta.json";
export const GENTLE_RAMP_METRICS_DIR = "metrics";
export const GENTLE_RAMP_FINDINGS_DIR = "findings";

/** The 23 stored track fields. Windows expand these to 26 owners below. */
export const GENTLE_RAMP_METRICS = [
  "atomMeasurableLessons",
  "atomMeasurementBlindLessons",
  "durationViolations",
  "atomLessonSpikes",
  "atomChapterSpikes",
  "glyphLessonSpikes",
  "scriptSystemSpikes",
  "scriptClosureViolations",
  "neverTaughtGlyphs",
  "orderDefects",
  "lessonsWithoutSequence",
  "unknownPrerequisites",
  "forwardPrerequisites",
  "forwardReviews",
  "forwardReferences",
  "atomsTaught",
  "atomsNeverRevisited",
  "reinforcementWindowMisses",
  "reinforcementMissesByWindow",
  "payoffSurprises",
  "writingPracticeLessons",
  "firstWritingPracticeAt",
  "lessonsBeforeWritingPractice",
] as const satisfies readonly (keyof TrackGentleRamp)[];

// reinforcementMissesByWindow is one stable top-level owner. Its four bounded
// identities are validated and reconstructed in REINFORCEMENT_WINDOWS order.
// The list above therefore has 23 owner files, not 26. The issue's corrected
// 851-file target counts the four window identities as separate owners, so the
// serializer expands this field below.
export const GENTLE_RAMP_METRIC_OWNERS = [
  ...GENTLE_RAMP_METRICS.filter((metric) => metric !== "reinforcementMissesByWindow"),
  ...REINFORCEMENT_WINDOWS.map((window) => `reinforcementMissesByWindow-${window.name}` as const),
] as const;

type MetricName = (typeof GENTLE_RAMP_METRICS)[number];

export interface GentleRampOwnerReadOptions {
  expectedLanguages: readonly string[];
  expectedLessonIds: ReadonlyMap<string, readonly string[]>;
  expectedNarrationLessonIds: ReadonlyMap<string, readonly string[]>;
  rejectAggregates?: boolean;
  requireCanonicalBytes?: boolean;
}

const LANGUAGE = /^[a-z][a-z0-9-]*$/;
const LESSON_ID = /^[A-Za-z0-9][A-Za-z0-9-]*$/;
const WINDOWS_RESERVED = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;
const DANGEROUS_IDENTITIES = new Set(["__proto__", "constructor", "prototype"]);
const INTEGER_METRICS = new Set<MetricName>(
  GENTLE_RAMP_METRICS.filter((metric) =>
    metric !== "firstWritingPracticeAt" && metric !== "reinforcementMissesByWindow"),
);

function canonical(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function safeLanguage(value: string): boolean {
  return LANGUAGE.test(value) && !WINDOWS_RESERVED.test(value) &&
    !DANGEROUS_IDENTITIES.has(value.toLowerCase());
}

function safeLessonId(value: string): boolean {
  return LESSON_ID.test(value) && !WINDOWS_RESERVED.test(value) &&
    !DANGEROUS_IDENTITIES.has(value.toLowerCase());
}

function statIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch (cause) {
    const code = (cause as NodeJS.ErrnoException).code;
    if (isAbsentErrno(code)) return undefined;
    throw new Error(`gentle-ramp owner '${path}': cannot be inspected (${code ?? "unknown error"})`, {
      cause,
    });
  }
}

function assertRealDescendantComponents(root: string, target: string): void {
  const absoluteRoot = resolve(root);
  const absoluteTarget = resolve(target);
  const route = relative(absoluteRoot, absoluteTarget);
  if (route === ".." || route.startsWith(`..${sep}`) || resolve(absoluteRoot, route) !== absoluteTarget) {
    throw new Error(`gentle-ramp owner path '${target}' is outside '${root}'`);
  }
  const realRoot = realpathSync(absoluteRoot);
  let current = absoluteRoot;
  for (const component of route.split(sep).filter(Boolean)) {
    current = join(current, component);
    const stat = lstatSync(current);
    if (stat.isSymbolicLink()) {
      throw new Error(`gentle-ramp owner path component '${current}' must not be a symbolic link`);
    }
    const real = realpathSync(current);
    if (real !== realRoot && !real.startsWith(realRoot + sep)) {
      throw new Error(`gentle-ramp owner path component '${current}' resolves outside '${root}'`);
    }
  }
}

function object(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`gentle-ramp owner '${label}' must contain one JSON object`);
  }
  return value as Record<string, unknown>;
}

function exactKeys(value: Record<string, unknown>, expected: readonly string[], label: string): void {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    throw new Error(`gentle-ramp owner '${label}' must contain exactly: ${wanted.join(", ")}`);
  }
}

function assertNoCaseFoldCollisions(values: readonly string[], label: string): void {
  const seen = new Map<string, string>();
  for (const value of values) {
    const folded = value.toLowerCase();
    const prior = seen.get(folded);
    if (prior !== undefined) {
      throw new Error(`${label} has a duplicate or case-fold collision between '${prior}' and '${value}'`);
    }
    seen.set(folded, value);
  }
}

function assertIdentitySet(actual: readonly string[], expected: readonly string[], label: string): void {
  assertNoCaseFoldCollisions(actual, `${label} actual identities`);
  assertNoCaseFoldCollisions(expected, `${label} expected identities`);
  const found = [...actual].sort();
  const wanted = [...expected].sort();
  const missing = wanted.filter((identity) => !found.includes(identity));
  const extra = found.filter((identity) => !wanted.includes(identity));
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `${label} do not match` +
      `${missing.length > 0 ? `; missing: ${missing.join(", ")}` : ""}` +
      `${extra.length > 0 ? `; extra: ${extra.join(", ")}` : ""}`,
    );
  }
}

function assertIdentityMap(
  identities: ReadonlyMap<string, readonly string[]>,
  languages: readonly string[],
  label: string,
): void {
  assertIdentitySet([...identities.keys()], languages, `${label} languages`);
  const globalIds: string[] = [];
  for (const [language, ids] of identities) {
    if (!safeLanguage(language)) throw new Error(`${label} language '${language}' is unsafe or reserved`);
    for (const id of ids) {
      if (!safeLessonId(id)) throw new Error(`${label} lesson id '${id}' is unsafe or reserved`);
    }
    assertNoCaseFoldCollisions(ids, `${label} ${language} lesson ids`);
    globalIds.push(...ids);
  }
  assertNoCaseFoldCollisions(globalIds, `${label} global lesson ids`);
}

function nonNegativeInteger(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    throw new Error(`gentle-ramp owner '${label}' value must be a non-negative safe integer`);
  }
  return value;
}

function metricValue(metric: MetricName, value: unknown, label: string): TrackGentleRamp[MetricName] {
  if (INTEGER_METRICS.has(metric)) return nonNegativeInteger(value, label);
  if (metric === "firstWritingPracticeAt") {
    return value === null ? null : nonNegativeInteger(value, label);
  }
  const windows = object(value, `${label}.value`);
  exactKeys(windows, REINFORCEMENT_WINDOWS.map((window) => window.name), `${label}.value`);
  return Object.fromEntries(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, nonNegativeInteger(windows[window.name], label)]),
  ) as Record<WindowName, number>;
}

function parseFinding(value: unknown, language: string, kind: GentleRampFindingKind, label: string): GentleRampFinding | null {
  if (value === null) return null;
  const finding = object(value, `${label}.finding`);
  exactKeys(finding, ["kind", "language", "count", "unit", "detail"], `${label}.finding`);
  if (finding.kind !== kind || finding.language !== language) {
    throw new Error(`gentle-ramp finding '${label}' does not belong to its owner`);
  }
  if (typeof finding.unit !== "string" || finding.unit.length === 0 ||
      typeof finding.detail !== "string" || finding.detail.length === 0) {
    throw new Error(`gentle-ramp finding '${label}' must carry non-empty unit and detail`);
  }
  return {
    language,
    kind,
    count: nonNegativeInteger(finding.count, `${label}.finding.count`),
    unit: finding.unit,
    detail: finding.detail,
  };
}

function assertTrackForOwners(track: TrackGentleRamp): void {
  if (!safeLanguage(track.language)) throw new Error(`gentle-ramp language '${track.language}' is unsafe`);
  nonNegativeInteger(track.lessonCount, `${track.language}.lessonCount`);
  for (const metric of GENTLE_RAMP_METRICS) metricValue(metric, track[metric], `${track.language}.${metric}`);
  if (track.atomMeasurableLessons + track.atomMeasurementBlindLessons !== track.lessonCount) {
    throw new Error(`gentle-ramp atom measurement counts for '${track.language}' do not equal lessonCount`);
  }
  const windowMisses = REINFORCEMENT_WINDOWS.reduce(
    (sum, window) => sum + track.reinforcementMissesByWindow[window.name],
    0,
  );
  if (track.reinforcementWindowMisses !== windowMisses) {
    throw new Error(`gentle-ramp reinforcement total for '${track.language}' does not equal R1-R4`);
  }
  if (track.lessonsBeforeWritingPractice !== (track.firstWritingPracticeAt ?? track.lessonCount)) {
    throw new Error(`gentle-ramp writing prefix for '${track.language}' is inconsistent`);
  }
  if (track.writingPracticeLessons > track.lessonCount ||
      (track.writingPracticeLessons === 0) !== (track.firstWritingPracticeAt === null) ||
      (track.firstWritingPracticeAt !== null && track.firstWritingPracticeAt >= track.lessonCount)) {
    throw new Error(`gentle-ramp writing metrics for '${track.language}' are inconsistent`);
  }
  const kinds = track.findings.map((finding) => finding.kind);
  assertNoCaseFoldCollisions(kinds, `gentle-ramp findings for ${track.language}`);
  for (const finding of track.findings) {
    if (!GENTLE_RAMP_PRIORITIES.includes(finding.kind) || finding.language !== track.language) {
      throw new Error(`gentle-ramp finding '${finding.kind}' does not belong to '${track.language}'`);
    }
  }
  const ordered = GENTLE_RAMP_PRIORITIES.flatMap((kind) =>
    track.findings.filter((finding) => finding.kind === kind));
  if (canonical(ordered) !== canonical(track.findings)) {
    throw new Error(`gentle-ramp findings for '${track.language}' are not in priority order`);
  }
  if (canonical(track.next) !== canonical(track.findings[0] ?? null)) {
    throw new Error(`gentle-ramp next finding for '${track.language}' is not derived from priority order`);
  }
  const definitions: Record<GentleRampFindingKind, readonly [number, string, string]> = {
    duration: [
      track.durationViolations,
      "lesson(s)",
      "split lessons whose effective duration exceeds the five-minute maximum",
    ],
    "order-integrity": [
      track.orderDefects + track.unknownPrerequisites,
      "order/dependency defect(s)",
      "declare a unique reading order and close every prerequisite and review before use",
    ],
    "forward-language": [
      track.forwardReferences,
      "use(s)",
      "teach target-language material before load-bearing use",
    ],
    "script-closure": [
      track.neverTaughtGlyphs,
      "glyph(s)",
      "teach every load-bearing glyph before asking the learner to decode it",
    ],
    "writing-ramp": [
      track.lessonCount === 0 ? 0 : track.firstWritingPracticeAt ?? track.lessonCount,
      "opening lesson(s)",
      track.firstWritingPracticeAt === null
        ? "add observable, guided writing practice from the opening lesson"
        : "move the first gentle writing microstep to lesson one",
    ],
    "atom-step": [
      track.atomLessonSpikes + track.atomChapterSpikes,
      "lesson/chapter spike(s)",
      "split knowledge-atom spikes into smaller prerequisite-safe steps",
    ],
    "glyph-step": [
      track.glyphLessonSpikes + track.scriptSystemSpikes,
      "lesson/system spike(s)",
      "split new glyphs and writing systems across gentler steps",
    ],
    reinforcement: [
      track.reinforcementWindowMisses,
      "missed window(s)",
      "add retrieval at the expanding R1-R4 intervals",
    ],
    "payoff-surprise": [
      track.payoffSurprises,
      "chapter payoff finding(s)",
      "make each chapter payoff assess only taught, representative material",
    ],
    "measurement-blind": [
      track.atomMeasurementBlindLessons,
      "lesson(s)",
      "migrate undeclared lesson knowledge so gentleness can be measured",
    ],
  };
  const expectedFindings = GENTLE_RAMP_PRIORITIES.flatMap((kind) => {
    const [count, unit, detail] = definitions[kind];
    return count === 0 ? [] : [{ language: track.language, kind, count, unit, detail }];
  });
  if (canonical(track.findings) !== canonical(expectedFindings)) {
    throw new Error(`gentle-ramp findings for '${track.language}' do not match their metrics`);
  }
}

/** Stable owner bytes beneath core/gentle-ramp-snapshots/. */
export function gentleRampOwnerContents(tracks: readonly TrackGentleRamp[]): Map<string, string> {
  const languages = tracks.map((track) => track.language);
  assertNoCaseFoldCollisions(languages, "gentle-ramp track languages");
  const out = new Map<string, string>();
  for (const track of [...tracks].sort((left, right) => left.language < right.language ? -1 : left.language > right.language ? 1 : 0)) {
    assertTrackForOwners(track);
    const prefix = `${track.language}.d`;
    out.set(`${prefix}/${GENTLE_RAMP_META_OWNER}`, canonical({
      version: GENTLE_RAMP_OWNER_VERSION,
      language: track.language,
    }));
    for (const metric of GENTLE_RAMP_METRICS) {
      if (metric === "reinforcementMissesByWindow") {
        for (const window of REINFORCEMENT_WINDOWS) {
          const owner = `reinforcementMissesByWindow-${window.name}`;
          out.set(`${prefix}/${GENTLE_RAMP_METRICS_DIR}/${owner}.json`, canonical({
            language: track.language,
            metric: owner,
            value: track.reinforcementMissesByWindow[window.name],
          }));
        }
      } else {
        out.set(`${prefix}/${GENTLE_RAMP_METRICS_DIR}/${metric}.json`, canonical({
          language: track.language,
          metric,
          value: track[metric],
        }));
      }
    }
    for (const kind of GENTLE_RAMP_PRIORITIES) {
      out.set(`${prefix}/${GENTLE_RAMP_FINDINGS_DIR}/${kind}.json`, canonical({
        language: track.language,
        kind,
        finding: track.findings.find((entry) => entry.kind === kind) ?? null,
      }));
    }
  }
  return out;
}

function assertOwnerFiles(directory: string, expected: readonly string[], label: string): void {
  const entries = readdirSync(directory, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.isSymbolicLink() || !entry.isFile()) {
      throw new Error(`gentle-ramp owner '${label}/${entry.name}' must be a real direct-child regular file`);
    }
  }
  assertIdentitySet(entries.map((entry) => entry.name), expected, `${label} owner names`);
}

function assertCanonicalOwner(text: string, expected: unknown, label: string): void {
  if (text !== canonical(expected)) {
    throw new Error(`gentle-ramp owner '${label}' is not canonical`);
  }
}

/** Strictly reconstruct the historical TrackGentleRamp snapshots from direct owners. */
export function readGentleRampOwners(root: string, options: GentleRampOwnerReadOptions): TrackGentleRamp[] {
  for (const language of options.expectedLanguages) {
    if (!safeLanguage(language)) throw new Error(`gentle-ramp expected language '${language}' is unsafe`);
  }
  assertNoCaseFoldCollisions(options.expectedLanguages, "gentle-ramp expected languages");
  assertIdentityMap(options.expectedLessonIds, options.expectedLanguages, "source");
  assertIdentityMap(options.expectedNarrationLessonIds, options.expectedLanguages, "narration");
  for (const language of options.expectedLanguages) {
    assertIdentitySet(
      options.expectedNarrationLessonIds.get(language) ?? [],
      options.expectedLessonIds.get(language) ?? [],
      `narration lesson identities for ${language}`,
    );
  }

  const directory = join(root, GENTLE_RAMP_SNAPSHOT_DIR);
  const stat = statIfPresent(directory);
  if (stat === undefined) throw new Error(`gentle-ramp owner directory '${directory}' is missing`);
  if (stat.isSymbolicLink() || !stat.isDirectory()) {
    throw new Error(`gentle-ramp owner directory '${directory}' must be a real directory`);
  }
  assertRealDescendantComponents(root, directory);

  const entries = readdirSync(directory, { withFileTypes: true });
  const ownerLanguages: string[] = [];
  for (const entry of entries) {
    if (entry.name.endsWith(".json")) {
      if (options.rejectAggregates !== false) {
        throw new Error(`gentle-ramp aggregate '${entry.name}' is resurrected beside direct owners`);
      }
      if (entry.isSymbolicLink() || !entry.isFile()) throw new Error(`unexpected gentle-ramp aggregate '${entry.name}'`);
      continue;
    }
    if (!entry.name.endsWith(".d") || entry.isSymbolicLink() || !entry.isDirectory()) {
      throw new Error(`unexpected gentle-ramp owner entry '${entry.name}'`);
    }
    const language = entry.name.slice(0, -2);
    if (!safeLanguage(language)) throw new Error(`gentle-ramp owner language '${language}' is unsafe`);
    assertRealDescendantComponents(root, join(directory, entry.name));
    ownerLanguages.push(language);
  }
  assertIdentitySet(ownerLanguages, options.expectedLanguages, "gentle-ramp owner languages");

  const tracks: TrackGentleRamp[] = [];
  for (const language of [...options.expectedLanguages].sort()) {
    const languageDirectory = join(directory, `${language}.d`);
    const languageEntries = readdirSync(languageDirectory, { withFileTypes: true });
    assertIdentitySet(
      languageEntries.map((entry) => entry.name),
      [GENTLE_RAMP_META_OWNER, GENTLE_RAMP_METRICS_DIR, GENTLE_RAMP_FINDINGS_DIR],
      `gentle-ramp ${language} layout`,
    );
    for (const entry of languageEntries) {
      const path = join(languageDirectory, entry.name);
      if (entry.isSymbolicLink()) throw new Error(`gentle-ramp owner '${language}.d/${entry.name}' must not be a symbolic link`);
      if (entry.name === GENTLE_RAMP_META_OWNER ? !entry.isFile() : !entry.isDirectory()) {
        throw new Error(`gentle-ramp owner '${language}.d/${entry.name}' has the wrong type`);
      }
      assertRealDescendantComponents(root, path);
    }

    const metaLabel = `${language}.d/${GENTLE_RAMP_META_OWNER}`;
    const metaPath = join(languageDirectory, GENTLE_RAMP_META_OWNER);
    const { value: metaValue, text: metaText } = readLedgerFileWithSource(metaPath);
    const meta = object(metaValue, metaLabel);
    exactKeys(meta, ["version", "language"], metaLabel);
    if (meta.version !== GENTLE_RAMP_OWNER_VERSION || meta.language !== language) {
      throw new Error(`gentle-ramp metadata '${language}' has the wrong version or language`);
    }
    if (options.requireCanonicalBytes !== false) {
      assertCanonicalOwner(metaText, { version: GENTLE_RAMP_OWNER_VERSION, language }, metaLabel);
    }

    const metricDirectory = join(languageDirectory, GENTLE_RAMP_METRICS_DIR);
    const findingDirectory = join(languageDirectory, GENTLE_RAMP_FINDINGS_DIR);
    assertOwnerFiles(metricDirectory, GENTLE_RAMP_METRIC_OWNERS.map((metric) => `${metric}.json`), `${language}.d/metrics`);
    assertOwnerFiles(findingDirectory, GENTLE_RAMP_PRIORITIES.map((kind) => `${kind}.json`), `${language}.d/findings`);

    const metrics = new Map<MetricName, unknown>();
    const reinforcementEntries: Partial<Record<WindowName, number>> = {};
    for (const metric of GENTLE_RAMP_METRIC_OWNERS) {
      const label = `${language}.d/metrics/${metric}.json`;
      const path = join(metricDirectory, `${metric}.json`);
      const { value, text } = readLedgerFileWithSource(path);
      const owner = object(value, label);
      exactKeys(owner, ["language", "metric", "value"], label);
      if (owner.language !== language || owner.metric !== metric) {
        throw new Error(`gentle-ramp metric '${label}' does not belong to its owner`);
      }
      if (metric.startsWith("reinforcementMissesByWindow-")) {
        const window = metric.slice("reinforcementMissesByWindow-".length) as WindowName;
        const parsed = nonNegativeInteger(owner.value, `${label}.value`);
        reinforcementEntries[window] = parsed;
        if (options.requireCanonicalBytes !== false) {
          assertCanonicalOwner(text, { language, metric, value: parsed }, label);
        }
      } else {
        const parsed = metricValue(metric as MetricName, owner.value, label);
        metrics.set(metric as MetricName, parsed);
        if (options.requireCanonicalBytes !== false) {
          assertCanonicalOwner(text, { language, metric, value: parsed }, label);
        }
      }
    }
    metrics.set("reinforcementMissesByWindow", Object.fromEntries(
      REINFORCEMENT_WINDOWS.map((window) => [window.name, reinforcementEntries[window.name]!]),
    ));

    const findings: GentleRampFinding[] = [];
    for (const kind of GENTLE_RAMP_PRIORITIES) {
      const label = `${language}.d/findings/${kind}.json`;
      const path = join(findingDirectory, `${kind}.json`);
      const { value, text } = readLedgerFileWithSource(path);
      const owner = object(value, label);
      exactKeys(owner, ["language", "kind", "finding"], label);
      if (owner.language !== language || owner.kind !== kind) {
        throw new Error(`gentle-ramp finding '${label}' does not belong to its owner`);
      }
      const finding = parseFinding(owner.finding, language, kind, label);
      if (options.requireCanonicalBytes !== false) {
        assertCanonicalOwner(text, { language, kind, finding }, label);
      }
      if (finding !== null) findings.push(finding);
    }

    const number = (metric: MetricName): number => metrics.get(metric) as number;
    const firstWritingPracticeAt = metrics.get("firstWritingPracticeAt") as number | null;
    const track: TrackGentleRamp = {
      language,
      lessonCount: options.expectedLessonIds.get(language)!.length,
      atomMeasurableLessons: number("atomMeasurableLessons"),
      atomMeasurementBlindLessons: number("atomMeasurementBlindLessons"),
      durationViolations: number("durationViolations"),
      atomLessonSpikes: number("atomLessonSpikes"),
      atomChapterSpikes: number("atomChapterSpikes"),
      glyphLessonSpikes: number("glyphLessonSpikes"),
      scriptSystemSpikes: number("scriptSystemSpikes"),
      scriptClosureViolations: number("scriptClosureViolations"),
      neverTaughtGlyphs: number("neverTaughtGlyphs"),
      orderDefects: number("orderDefects"),
      lessonsWithoutSequence: number("lessonsWithoutSequence"),
      unknownPrerequisites: number("unknownPrerequisites"),
      forwardPrerequisites: number("forwardPrerequisites"),
      forwardReviews: number("forwardReviews"),
      forwardReferences: number("forwardReferences"),
      atomsTaught: number("atomsTaught"),
      atomsNeverRevisited: number("atomsNeverRevisited"),
      reinforcementWindowMisses: number("reinforcementWindowMisses"),
      reinforcementMissesByWindow: metrics.get("reinforcementMissesByWindow") as Record<WindowName, number>,
      payoffSurprises: number("payoffSurprises"),
      writingPracticeLessons: number("writingPracticeLessons"),
      firstWritingPracticeAt,
      lessonsBeforeWritingPractice: number("lessonsBeforeWritingPractice"),
      findings,
      next: findings[0] ?? null,
    };
    assertTrackForOwners(track);
    tracks.push(track);
  }
  return tracks;
}
