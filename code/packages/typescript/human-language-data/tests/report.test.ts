import { describe, expect, it } from "vitest";
import { parseLesson } from "../src/parse.js";
import {
  buildCurriculumGapReport,
  estimateLessonDuration,
  renderCurriculumGapReport,
} from "../src/report.js";
import type { BookCorpus, LanguageRegistry } from "../src/types.js";

const registry: LanguageRegistry = {
  version: 1,
  languages: [
    { id: "alpha", name: "Alpha", family: "Test", script: "latin", status: "active", bridges: [] },
    { id: "beta", name: "Beta", family: "Test", script: "latin", status: "active", bridges: [] },
  ],
};

function lesson(options: {
  id: string;
  language: string;
  chapter: number;
  minutes: number;
  prerequisites?: string[];
  schemaVersion?: number;
  audioSeconds?: number;
  body?: string;
}) {
  const schema = options.schemaVersion ? `schema_version: ${options.schemaVersion}\n` : "";
  const audio = options.audioSeconds ? `audio_duration_seconds: ${options.audioSeconds}\n` : "";
  return parseLesson(
    `---\n${schema}${audio}id: ${options.id}\nchapter: ${options.chapter}\ntype: word\nheadword: hello\ngloss: hello\nconcept_tag: TEST-HELLO\nprerequisites: [${(options.prerequisites ?? []).join(", ")}]\nest_minutes: ${options.minutes}\n---\n\n${options.body ?? "Say hello."}\n`,
    options.language,
  );
}

describe("curriculum gap report", () => {
  it("uses the greater of declared and independently computed duration", () => {
    const declared = lesson({ id: "BE-C01", language: "beta", chapter: 1, minutes: 5 });
    const computed = lesson({
      id: "AL-C02",
      language: "alpha",
      chapter: 2,
      minutes: 1,
      body: `${"word ".repeat(700)}\nPause for 10 seconds. Repeat this?`,
    });

    expect(estimateLessonDuration(declared)).toMatchObject({
      declaredSeconds: 300,
      effectiveSeconds: 300,
      reasons: ["declared"],
    });
    const computedEstimate = estimateLessonDuration(computed);
    expect(computedEstimate.computedSeconds).toBeGreaterThan(300);
    expect(computedEstimate.effectiveSeconds).toBe(computedEstimate.computedSeconds);
    expect(computedEstimate.promptCount).toBe(1);
    expect(computedEstimate.repeatCueCount).toBe(1);
    expect(computedEstimate.explicitPauseSeconds).toBe(10);
    expect(computedEstimate.reasons).toEqual(["computed"]);

    const audio = estimateLessonDuration(
      lesson({ id: "AU-C01", language: "alpha", chapter: 1, minutes: 1, audioSeconds: 301 }),
    );
    expect(audio.authoredAudioSeconds).toBe(301);
    expect(audio.computedSeconds).toBeGreaterThan(300);
  });

  it("strips closed and adversarial unclosed HTML comments without a regex backtrack", () => {
    const closed = estimateLessonDuration(
      lesson({
        id: "AL-COMMENT",
        language: "alpha",
        chapter: 1,
        minutes: 1,
        body: "visible <!-- hidden words --> shown",
      }),
    );
    expect(closed.wordCount).toBe(2);

    const unclosed = estimateLessonDuration(
      lesson({
        id: "AL-UNCLOSED",
        language: "alpha",
        chapter: 1,
        minutes: 1,
        body: `visible ${"<!--".repeat(10_000)} hidden`,
      }),
    );
    expect(unclosed.wordCount).toBe(1);
  });

  it("reports duration, prerequisite, book, and schema migration gaps", () => {
    const lessons = [
      lesson({ id: "AL-C01", language: "alpha", chapter: 1, minutes: 4 }),
      lesson({
        id: "AL-C02",
        language: "alpha",
        chapter: 2,
        minutes: 1,
        schemaVersion: 2,
        body: "word ".repeat(700),
      }),
      lesson({
        id: "BE-C01",
        language: "beta",
        chapter: 1,
        minutes: 5,
        schemaVersion: 2,
        prerequisites: ["MISSING"],
      }),
    ];
    const books: BookCorpus = {
      books: [
        {
          language: "alpha",
          entrypoint: "alpha/book/book.tex",
          chapters: [
            { language: "alpha", chapter: 1, slug: "one", title: "One", source: "one.tex", tex: "x" },
          ],
        },
      ],
    };
    const report = buildCurriculumGapReport({ registry, lessons, books });

    expect(report.summary).toMatchObject({
      registeredTracks: 2,
      totalLessons: 3,
      authoredBooks: 1,
      durationViolations: 2,
      unknownPrerequisites: 1,
      laterChapterLessonsWithoutPrerequisites: 1,
      tracksWithoutBooks: 1,
      lessonChaptersWithoutBooks: 2,
      legacySchemaTracks: 0,
      mixedSchemaTracks: 1,
      version2SchemaTracks: 1,
    });
    expect(report.prerequisites.unknown).toEqual([
      { lessonId: "BE-C01", language: "beta", prerequisite: "MISSING" },
    ]);
    expect(report.books.tracks.find((track) => track.language === "alpha")).toMatchObject({
      missingBookChapters: [2],
      coveragePercent: 50,
    });
    expect(report.schemas.tracks.find((track) => track.language === "alpha")).toMatchObject({
      status: "mixed",
      versions: { "1": 1, "2": 1 },
    });
    expect(renderCurriculumGapReport(report)).toContain("2 lessons at or above 300 effective seconds");
  });
});
