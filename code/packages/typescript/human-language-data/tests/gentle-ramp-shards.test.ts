import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  GENTLE_RAMP_SNAPSHOT_DIR,
  generatedGentleRampSnapshotOutputsFromReport,
  installGentleRampOwnerTree,
  safeGentleRampOwnerOutput,
} from "../src/gentle-ramp-snapshot-cli.js";
import {
  gentleRampOwnerContents,
  readGentleRampOwners,
} from "../src/gentle-ramp-shards.js";
import type { GentleRampReport, TrackGentleRamp } from "../src/gentle-ramp.js";

const roots: string[] = [];

function track(language: string, lessonCount = 2): TrackGentleRamp {
  return {
    language,
    lessonCount,
    atomMeasurableLessons: lessonCount,
    atomMeasurementBlindLessons: 0,
    durationViolations: 0,
    atomLessonSpikes: 0,
    atomChapterSpikes: 0,
    glyphLessonSpikes: 0,
    scriptSystemSpikes: 0,
    scriptClosureViolations: 0,
    neverTaughtGlyphs: 0,
    orderDefects: 0,
    lessonsWithoutSequence: 0,
    unknownPrerequisites: 0,
    forwardPrerequisites: 0,
    forwardReviews: 0,
    forwardReferences: 0,
    atomsTaught: 1,
    atomsNeverRevisited: 0,
    reinforcementWindowMisses: 0,
    reinforcementMissesByWindow: { R1: 0, R2: 0, R3: 0, R4: 0 },
    payoffSurprises: 0,
    writingPracticeLessons: 1,
    firstWritingPracticeAt: 0,
    lessonsBeforeWritingPractice: 0,
    findings: [],
    next: null,
  };
}

function tracks(): TrackGentleRamp[] {
  return [track("alpha", 2), track("beta", 1)];
}

function report(values: TrackGentleRamp[]): GentleRampReport {
  return {
    schemaVersion: 1,
    rule: {
      maxLessonSeconds: 300,
      durationBoundary: "strictly-greater-than",
      atomMeasurement: "declared-atoms-only",
      ranking: "learner-first-named-debt-no-composite-score",
    },
    tracks: values,
    workQueue: values.flatMap((value) => value.findings),
    summary: {
      tracks: values.length,
      tracksWithDetectedCliffs: 0,
      tracksWithNoWritingPractice: 0,
      tracksWhereWritingStartsLate: 0,
      atomMeasurementBlindLessons: 0,
      findings: 0,
    },
  };
}

function temporaryRoot(): string {
  const root = mkdtempSync(join(tmpdir(), "hl-gentle-ramp-shards-"));
  roots.push(root);
  mkdirSync(join(root, GENTLE_RAMP_SNAPSHOT_DIR), { recursive: true });
  return root;
}

function writeOwners(root: string, values = tracks()): void {
  for (const [relative, contents] of gentleRampOwnerContents(values)) {
    const path = join(root, GENTLE_RAMP_SNAPSHOT_DIR, relative);
    mkdirSync(join(path, ".."), { recursive: true });
    writeFileSync(path, contents, "utf8");
  }
}

function identities(values = tracks()): ReadonlyMap<string, readonly string[]> {
  return new Map(values.map((value) => [
    value.language,
    Array.from({ length: value.lessonCount }, (_, index) =>
      `${value.language.toUpperCase()}-${index + 1}`),
  ]));
}

function readOwners(root: string, values = tracks()): TrackGentleRamp[] {
  const ids = identities(values);
  return readGentleRampOwners(root, {
    expectedLanguages: values.map((value) => value.language),
    expectedLessonIds: ids,
    expectedNarrationLessonIds: ids,
  });
}

function owner(root: string, language: string, family: "metrics" | "findings", name: string): string {
  return join(root, GENTLE_RAMP_SNAPSHOT_DIR, `${language}.d`, family, `${name}.json`);
}

afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

describe("gentle-ramp direct owners", () => {
  it("emits 37 stable owners per language and reconstructs the exact public track", () => {
    const root = temporaryRoot();
    const values = tracks();
    const outputs = gentleRampOwnerContents(values);
    writeOwners(root, values);

    expect(outputs.size).toBe(74);
    expect([...outputs.keys()].filter((path) => path.startsWith("alpha.d/"))).toHaveLength(37);
    expect(readOwners(root, values)).toEqual(values);
  });

  it("checks exact language identities before opening owner bytes", () => {
    const root = temporaryRoot();
    writeOwners(root);
    writeFileSync(join(root, GENTLE_RAMP_SNAPSHOT_DIR, "alpha.d", "_meta.json"), "not json\n");
    rmSync(join(root, GENTLE_RAMP_SNAPSHOT_DIR, "beta.d"), { recursive: true, force: true });

    expect(() => readOwners(root)).toThrow(/missing.*beta|beta.*missing/i);
  });

  it("rejects aggregate resurrection and unexpected language owners", () => {
    const aggregateRoot = temporaryRoot();
    writeOwners(aggregateRoot);
    writeFileSync(join(aggregateRoot, GENTLE_RAMP_SNAPSHOT_DIR, "alpha.json"), "{}\n");
    expect(() => readOwners(aggregateRoot)).toThrow(/aggregate|resurrected/i);

    const extraRoot = temporaryRoot();
    writeOwners(extraRoot);
    mkdirSync(join(extraRoot, GENTLE_RAMP_SNAPSHOT_DIR, "ghost.d"));
    expect(() => readOwners(extraRoot)).toThrow(/extra.*ghost|ghost.*extra/i);
  });

  it("rejects missing, extra, nested, and non-regular metric owners", () => {
    const missingRoot = temporaryRoot();
    writeOwners(missingRoot);
    rmSync(owner(missingRoot, "alpha", "metrics", "atomsTaught"));
    expect(() => readOwners(missingRoot)).toThrow(/missing.*atomsTaught|atomsTaught.*missing/i);

    const extraRoot = temporaryRoot();
    writeOwners(extraRoot);
    writeFileSync(owner(extraRoot, "alpha", "metrics", "unknown"), "{}\n");
    expect(() => readOwners(extraRoot)).toThrow(/extra.*unknown|unknown.*extra/i);

    const nestedRoot = temporaryRoot();
    writeOwners(nestedRoot);
    mkdirSync(owner(nestedRoot, "alpha", "metrics", "nested"));
    expect(() => readOwners(nestedRoot)).toThrow(/regular|direct|nested/i);
  });

  it("rejects symlinked owners and owner directories", () => {
    const ownerRoot = temporaryRoot();
    writeOwners(ownerRoot);
    const linked = owner(ownerRoot, "alpha", "metrics", "atomsTaught");
    rmSync(linked);
    symlinkSync(owner(ownerRoot, "beta", "metrics", "atomsTaught"), linked);
    expect(() => readOwners(ownerRoot)).toThrow(/symbolic|regular|direct/i);

    const directoryRoot = temporaryRoot();
    writeOwners(directoryRoot);
    const findings = join(directoryRoot, GENTLE_RAMP_SNAPSHOT_DIR, "alpha.d", "findings");
    rmSync(findings, { recursive: true, force: true });
    symlinkSync(join(directoryRoot, GENTLE_RAMP_SNAPSHOT_DIR, "beta.d", "findings"), findings);
    expect(() => readOwners(directoryRoot)).toThrow(/symbolic/i);
  });

  it("binds path, language, metric, and finding identities", () => {
    const root = temporaryRoot();
    writeOwners(root);
    const metricPath = owner(root, "alpha", "metrics", "atomsTaught");
    const metric = JSON.parse(readFileSync(metricPath, "utf8"));
    metric.language = "beta";
    writeFileSync(metricPath, `${JSON.stringify(metric, null, 2)}\n`);
    expect(() => readOwners(root)).toThrow(/does not belong|alpha.*beta|language/i);

    writeOwners(root);
    const findingPath = owner(root, "alpha", "findings", "duration");
    const finding = JSON.parse(readFileSync(findingPath, "utf8"));
    finding.kind = "atom-step";
    writeFileSync(findingPath, `${JSON.stringify(finding, null, 2)}\n`);
    expect(() => readOwners(root)).toThrow(/does not belong|duration.*atom-step/i);
  });

  it("rejects noncanonical bytes and dangerous JSON keys", () => {
    const canonicalRoot = temporaryRoot();
    writeOwners(canonicalRoot);
    const path = owner(canonicalRoot, "alpha", "metrics", "atomsTaught");
    writeFileSync(path, JSON.stringify(JSON.parse(readFileSync(path, "utf8"))));
    expect(() => readOwners(canonicalRoot)).toThrow(/canonical/i);

    writeOwners(canonicalRoot);
    const reordered = JSON.parse(readFileSync(path, "utf8"));
    writeFileSync(path, `${JSON.stringify({
      metric: reordered.metric,
      value: reordered.value,
      language: reordered.language,
    }, null, 2)}\n`);
    expect(() => readOwners(canonicalRoot)).toThrow(/canonical/i);

    const dangerousRoot = temporaryRoot();
    writeOwners(dangerousRoot);
    const dangerousPath = owner(dangerousRoot, "alpha", "metrics", "atomsTaught");
    const text = readFileSync(dangerousPath, "utf8");
    writeFileSync(dangerousPath, text.replace("{\n", '{\n  "__proto__": {},\n'));
    expect(() => readOwners(dangerousRoot)).toThrow(/__proto__|dangerous/i);
  });

  it("derives lessonCount from exact source and narration identities", () => {
    const root = temporaryRoot();
    writeOwners(root);
    const source = new Map(identities());
    source.set("alpha", ["ALPHA-1"]);
    expect(() => readGentleRampOwners(root, {
      expectedLanguages: ["alpha", "beta"],
      expectedLessonIds: source,
      expectedNarrationLessonIds: identities(),
    })).toThrow(/narration.*ALPHA-2|ALPHA-2.*narration/i);

    expect(() => readGentleRampOwners(root, {
      expectedLanguages: ["alpha", "beta"],
      expectedLessonIds: source,
      expectedNarrationLessonIds: source,
    })).toThrow(/lessonCount|atom measurement/i);
  });

  it("rejects contradictory derived totals and finding payloads", () => {
    const root = temporaryRoot();
    writeOwners(root);
    const path = owner(root, "alpha", "metrics", "reinforcementMissesByWindow-R1");
    const metric = JSON.parse(readFileSync(path, "utf8"));
    metric.value = 1;
    writeFileSync(path, `${JSON.stringify(metric, null, 2)}\n`);
    expect(() => readOwners(root)).toThrow(/reinforcement total|R1-R4/i);

    writeOwners(root);
    const findingPath = owner(root, "alpha", "findings", "duration");
    const finding = JSON.parse(readFileSync(findingPath, "utf8"));
    finding.finding = {
      language: "alpha",
      kind: "duration",
      count: 1,
      unit: "lesson(s)",
      detail: "split lessons whose effective duration exceeds the five-minute maximum",
    };
    writeFileSync(findingPath, `${JSON.stringify(finding, null, 2)}\n`);
    expect(() => readOwners(root)).toThrow(/findings.*metrics|do not match/i);
  });

  it("restores the complete previous tree when installed verification fails", () => {
    const root = temporaryRoot();
    const before = tracks();
    writeOwners(root, before);
    const after = structuredClone(before);
    after[0]!.atomsTaught += 1;
    const ids = identities(after);
    const generatedReport = report(after);

    expect(() => installGentleRampOwnerTree(root, {
      report: generatedReport,
      languages: after.map((value) => value.language),
      sourceIds: ids,
      narrationIds: ids,
      outputs: generatedGentleRampSnapshotOutputsFromReport(generatedReport),
    }, {
      afterInstalled: () => {
        throw new Error("injected installed-verification failure");
      },
    })).toThrow(/injected installed-verification failure/);

    expect(readOwners(root, before)).toEqual(before);
    expect(existsSync(join(root, `${GENTLE_RAMP_SNAPSHOT_DIR}.backup`))).toBe(false);
  });
});

describe("gentle-ramp owner conflict surface", () => {
  it("changes one metric owner when an independent metric changes", () => {
    const before = tracks();
    const after = structuredClone(before);
    after[0]!.atomsTaught += 1;
    const beforeOwners = gentleRampOwnerContents(before);
    const afterOwners = gentleRampOwnerContents(after);

    expect([...beforeOwners.keys()].filter((path) => beforeOwners.get(path) !== afterOwners.get(path))).toEqual([
      "alpha.d/metrics/atomsTaught.json",
    ]);
  });

  it("accepts only fixed contained owner output paths", () => {
    const root = temporaryRoot();
    expect(safeGentleRampOwnerOutput(
      root,
      "core/gentle-ramp-snapshots/alpha.d/metrics/atomsTaught.json",
    )).toContain("alpha.d");
    for (const path of [
      "../escape.json",
      "core/gentle-ramp-snapshots/alpha.json",
      "core/gentle-ramp-snapshots/alpha.d/metrics/unknown.json",
      "core/gentle-ramp-snapshots/alpha.d/nested/atomsTaught.json",
      "core/gentle-ramp-snapshots/con.d/metrics/atomsTaught.json",
    ]) {
      expect(() => safeGentleRampOwnerOutput(root, path), path).toThrow(/unsafe/i);
    }
  });
});
