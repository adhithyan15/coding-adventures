import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../src/activity.js";
import { measureContinuity } from "../../src/continuity.js";
import { defaultCurriculumRoot, loadTrackLessons } from "../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";

it("pins Punjabi continuity", () => expectLanguageContinuity("punjabi"));
it("pins Punjabi modality", () => expectLanguageModality("punjabi"));
it("pins Punjabi lesson-content budgets", () =>
  expectLanguageLessonBudgets("punjabi", {
    lessons: 68,
    idioms: 4,
    senses: 3,
    cultureClaims: 7,
    unitPrefix: "PA",
  }));

it("keeps Punjabi's 78-row session map aligned with canonical order", () => {
  const ordered = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const markdown = readFileSync(
    join(defaultCurriculumRoot(), "punjabi", "session-map.md"),
    "utf8",
  );
  const rows = [...markdown.matchAll(/^\| (\d+) \| (\d+) \| ([^|]+?) \| (.+) \|$/gm)].map(
    (match) => ({
      session: Number(match[1]),
      chapter: match[2],
      lessonId: match[3]!.trim(),
    }),
  );
  expect(rows).toHaveLength(78);
  expect(rows.map((row) => row.session)).toEqual(Array.from({ length: 78 }, (_, index) => index + 1));
  expect(rows.map((row) => row.lessonId)).toEqual(
    ordered.map((lesson) => lesson.realization.lessonId),
  );
  expect(rows.map((row) => row.chapter)).toEqual(
    ordered.map((lesson) => String(lesson.frontmatter.chapter)),
  );
});

it("pins Punjabi's meaning-first response and writing runway", () => {
  const opening = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .slice(0, 10);
  expect(opening.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-C01-sat-sri-akal",
    "PA-C01-namaste",
    "PA-C01-han-nahin",
    "PA-W01-ha",
    "PA-W01-aa-matra",
    "PA-W01-bindi",
    "PA-W01-haan-assemble",
    "PA-W01-haan-guided-copy",
    "PA-W01-haan-delayed-copy",
    "PA-W01-haan-dictation",
  ]);
  expect(opening.every((lesson) => lesson.frontmatter.chapter === "1")).toBe(true);

  const meaningFirst = opening[2]!;
  expect(meaningFirst.realization.gloss).toContain("yes / no");
  expect(meaningFirst.frontmatter.skills).toEqual(["listening", "speaking"]);
  expect(meaningFirst.body).toContain("The printed forms are labels, not a reading test.");
  expect(meaningFirst.body).toContain("The next three tiny sessions will teach one\npiece at a time");
});

it("pins Punjabi's complete pre-A1 writing runway", () => {
  const punjabi = languageWritingStages("punjabi");
  expect(punjabi.defects).toEqual([]);
  expect(punjabi.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  expect(punjabi.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]);
});

it("migrates Punjabi Chapter 2 without inventing Gurmukhi writing credit", () => {
  const chapter = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .filter((lesson) => lesson.frontmatter.chapter === "2");

  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-C02-naam",
    "PA-C02-mera",
    "PA-C02-hai",
    "PA-C02-mera-naam-hai",
    "PA-C02-tu-tusi",
    "PA-C02-ki",
    "PA-C02-tuhada-naam-ki-hai",
    "PA-C02-khushi",
    "PA-C02-practice",
  ]);
  expect(chapter.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
  expect(
    chapter.every(
      (lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 240,
    ),
  ).toBe(true);
  expect(chapter.every((lesson) => lesson.frontmatter.skills?.includes("listening"))).toBe(true);
  expect(chapter.every((lesson) => lesson.frontmatter.skills?.includes("speaking"))).toBe(true);
  expect(chapter.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);

  const payoff = chapter.at(-1)!;
  expect(payoff.body).toContain("Independent Gurmukhi reading and writing are **not scored here**");
  expect(payoff.body).toContain("A romanized answer never counts as Gurmukhi writing");
});

it("migrates Punjabi Chapter 3 as a gentle oral wellbeing exchange", () => {
  const chapter = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .filter((lesson) => lesson.frontmatter.chapter === "3");

  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-C03-kivein",
    "PA-C03-tusi-kivein-ho",
    "PA-R03-wellbeing-r1",
    "PA-C03-main",
    "PA-C03-thik",
    "PA-C03-koi-gall-nahin",
    "PA-C03-practice",
  ]);
  expect(chapter.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
  expect(
    chapter.every(
      (lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 240,
    ),
  ).toBe(true);
  expect(chapter.every((lesson) => lesson.frontmatter.skills?.includes("listening"))).toBe(true);
  expect(chapter.every((lesson) => lesson.frontmatter.skills?.includes("speaking"))).toBe(true);
  expect(chapter.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);

  const payoff = chapter.at(-1)!;
  expect(payoff.body).toContain("return the question");
  expect(payoff.body).toContain("Independent Gurmukhi reading and writing are **not scored here**");
  expect(payoff.body).toContain("A romanized answer never counts as Gurmukhi writing");
});

it("closes Chapter 3's oral R1/R2/R3 windows without inventing script credit", () => {
  const ordered = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const checkpointIds = [
    "PA-R03-wellbeing-r1",
    "PA-R04-wellbeing-r2",
    "PA-R07-wellbeing-r3",
  ];
  const checkpoints = checkpointIds.map((id) =>
    ordered.find((lesson) => lesson.realization.lessonId === id)!,
  );
  expect(checkpoints.map((lesson) => ordered.indexOf(lesson))).toEqual([24, 32, 47]);
  expect(checkpoints.map((lesson) => lesson.frontmatter["introduces.knowledge"])).toEqual([
    [],
    [],
    [],
  ]);
  expect(checkpoints.every((lesson) => lesson.frontmatter.skills?.includes("listening"))).toBe(true);
  expect(checkpoints.every((lesson) => lesson.frontmatter.skills?.includes("speaking"))).toBe(true);
  expect(checkpoints.every((lesson) => !lesson.frontmatter.skills?.includes("reading"))).toBe(true);
  expect(checkpoints.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(checkpoints.flatMap((lesson) => compileLessonActivities(lesson.blocks))).toHaveLength(8);
  const handwrittenChapter4 = readFileSync(
    join(defaultCurriculumRoot(), "punjabi", "book", "chapters", "ch04-farewells.tex"),
    "utf8",
  );
  expect(handwrittenChapter4).toContain("canonical-insertion: PA-R04-wellbeing-r2");
  expect(handwrittenChapter4).toContain("label{lesson:PA-R04-wellbeing-r2}");
  expect(handwrittenChapter4).toContain("Romanization records speech only");

  const chapter3Atoms = new Set([
    "PA-GRAMMAR-TUSI-HO-03",
    "PA-ETYMON-QUESTION-K-03",
    "PA-ETYMON-FIRST-PERSON-M-03",
    "PA-ETYMON-NEGATIVE-NE-03",
    "PA-GRAMMAR-MAIN-HAAN-03",
    "PA-LEX-MAIN-03",
    "PA-LEX-THIK-03",
    "PA-PHRASE-HOW-ARE-YOU-03",
    "PA-PHRASE-I-AM-FINE-03",
    "PA-PHRASE-NO-PROBLEM-03",
    "PA-LEX-KIVEIN-03",
  ]);
  const report = measureContinuity(ordered);
  expect(report.reinforcement.filter((defect) => chapter3Atoms.has(defect.atom))).toEqual([]);
  expect(report.summary.missedByWindow).toEqual({ R1: 27, R2: 51, R3: 71, R4: 0 });
});
