import { afterEach, describe, expect, it, vi } from "vitest";
import { parseLesson } from "../src/parse.js";
import { buildCurriculumGapReport } from "../src/report.js";
import { renderGentleRamp } from "../src/gentle-ramp.js";
import { runGentleRampReport } from "../src/gentle-ramp-cli.js";
import { loadChapterPolicy, loadEverything, loadTrackChapters } from "../src/loader.js";
import type { BookCorpus, ChapterPolicy, LanguageRegistry } from "../src/types.js";

const registry: LanguageRegistry = {
  version: 1,
  languages: [
    { id: "alpha", name: "Alpha", family: "Test", script: "latin", status: "active", bridges: [] },
    { id: "beta", name: "Beta", family: "Test", script: "latin", status: "active", bridges: [] },
  ],
};

const policy: ChapterPolicy = {
  version: 1,
  payoffRepresentativeness: 0.5,
  maxNewAtomsPerLesson: 3,
  maxNewAtomsPerChapter: 12,
  maxNewGlyphsPerLesson: 3,
  maxNewScriptSystemsPerLesson: 1,
};

const books: BookCorpus = { books: [] };

afterEach(() => vi.restoreAllMocks());

function lesson(options: {
  id: string;
  language: string;
  sequence: number;
  minutes: number;
  type?: "word" | "writing";
  prerequisites?: string[];
  atom?: string;
}) {
  const directive = options.atom
    ? `## Input\n<!-- hl-knowledge: introduces=[${options.atom}]; assesses=[] -->\n\nMeet it.\n`
    : "## Input\n<!-- hl-knowledge: introduces=[]; assesses=[] -->\n\nReview it.\n";
  return parseLesson(
    `---\nschema_version: 2\nid: ${options.id}\nchapter: 1\nsequence: ${options.sequence}\n` +
      `type: ${options.type ?? "word"}\nheadword: hello\ngloss: hello\nconcept_tag: TEST-HELLO\n` +
      `prerequisites: [${(options.prerequisites ?? []).join(", ")}]\nest_minutes: ${options.minutes}\n---\n\n` +
      directive,
    options.language,
  );
}

describe("the corpus-wide super-gentle ramp", () => {
  it("keeps exact five-minute lessons compliant and ranks named debt without a score", () => {
    const lessons = [
      lesson({
        id: "AL-1",
        language: "alpha",
        sequence: 1,
        minutes: 5,
        type: "writing",
        prerequisites: ["AL-2"],
        atom: "AL-HELLO",
      }),
      lesson({ id: "AL-2", language: "alpha", sequence: 2, minutes: 6, atom: "AL-BYE" }),
      lesson({ id: "BE-1", language: "beta", sequence: 1, minutes: 2 }),
      lesson({ id: "BE-2", language: "beta", sequence: 2, minutes: 2 }),
    ];

    const report = buildCurriculumGapReport({ registry, lessons, books, chapterPolicy: policy });
    const gentle = report.gentleRamp!;
    const alpha = gentle.tracks.find((track) => track.language === "alpha")!;
    const beta = gentle.tracks.find((track) => track.language === "beta")!;

    expect(report.summary.durationViolations).toBe(1);
    expect(alpha.durationViolations).toBe(1);
    expect(alpha.forwardPrerequisites).toBe(1);
    expect(alpha.firstWritingPracticeAt).toBe(0);
    expect(alpha.findings.map((entry) => entry.kind).slice(0, 2)).toEqual(["duration", "order-integrity"]);
    expect(beta.firstWritingPracticeAt).toBeNull();
    expect(beta.lessonsBeforeWritingPractice).toBe(2);
    expect(beta.findings.find((entry) => entry.kind === "writing-ramp")).toMatchObject({ count: 2 });
    expect(gentle.workQueue[0]).toMatchObject({ kind: "duration", language: "alpha", count: 1 });
    expect(gentle.rule.ranking).toBe("learner-first-named-debt-no-composite-score");
    expect(renderGentleRamp(gentle).join("\n")).toContain("writing-ramp");
  });

  it("rejects a flag used as another flag's value", () => {
    let error = "";
    vi.spyOn(process.stderr, "write").mockImplementation((chunk) => ((error += chunk), true));
    expect(runGentleRampReport(["--root", "--format"])).toBe(2);
    expect(error).toContain("--root requires a value");
  });

  it("pins the real corpus as debt rather than mistaking unmeasured tracks for gentle ones", () => {
    const { registry, lessons, books, curricula, spine } = loadEverything();
    const report = buildCurriculumGapReport({
      registry,
      lessons,
      books,
      curricula,
      spine,
      chapterPolicy: loadChapterPolicy(),
      trackChapters: loadTrackChapters(),
    }).gentleRamp!;

    expect(report.summary).toEqual({
      tracks: 23,
      tracksWithDetectedCliffs: 23,
      tracksWithNoWritingPractice: 3,
      tracksWhereWritingStartsLate: 7,
      atomMeasurementBlindLessons: 497,
      findings: 143,
    });
    expect(report.workQueue.slice(0, 3).map(({ language, kind, count }) => ({ language, kind, count }))).toEqual([
      { language: "kannada", kind: "duration", count: 1 },
      { language: "telugu", kind: "duration", count: 1 },
      { language: "italian", kind: "order-integrity", count: 22 },
    ]);
    expect(report.tracks.find((track) => track.language === "german")).toMatchObject({
      orderDefects: 0,
      forwardPrerequisites: 0,
      forwardReviews: 0,
    });
    expect(report.tracks.find((track) => track.language === "french")).toMatchObject({
      orderDefects: 0,
      forwardPrerequisites: 0,
      forwardReviews: 0,
    });
    expect(report.tracks.find((track) => track.language === "marathi")).toMatchObject({
      orderDefects: 0,
      forwardPrerequisites: 0,
      forwardReviews: 0,
    });
    expect(report.tracks.find((track) => track.language === "punjabi")).toMatchObject({
      orderDefects: 0,
      forwardPrerequisites: 0,
      forwardReviews: 0,
    });
    expect(report.tracks.find((track) => track.language === "arabic")).toMatchObject({
      orderDefects: 0,
      forwardPrerequisites: 0,
      forwardReviews: 0,
    });
    expect(report.tracks.every((track) => track.findings.length > 0)).toBe(true);
  }, 30_000);
});
