import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { measureContinuity } from "../../../src/continuity.js";
import { defaultCurriculumRoot, loadTrackLessons } from "../../../src/loader.js";

it("closes Chapter 3's oral R1-R4 windows without inventing script credit", () => {
  const ordered = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const checkpointIds = [
    "PA-R03-wellbeing-r1",
    "PA-R04-wellbeing-r2",
    "PA-R07-wellbeing-r3",
    "PA-R18-wellbeing-r4",
  ];
  const checkpoints = checkpointIds.map((id) =>
    ordered.find((lesson) => lesson.realization.lessonId === id)!,
  );
  expect(checkpoints.map((lesson) => ordered.indexOf(lesson))).toEqual([27, 36, 63, 132]);
  expect(checkpoints.map((lesson) => lesson.frontmatter["introduces.knowledge"])).toEqual([
    [],
    [],
    [],
    [],
  ]);
  expect(checkpoints.every((lesson) => lesson.frontmatter.skills?.includes("listening"))).toBe(true);
  expect(checkpoints.every((lesson) => lesson.frontmatter.skills?.includes("speaking"))).toBe(true);
  expect(checkpoints.every((lesson) => !lesson.frontmatter.skills?.includes("reading"))).toBe(true);
  expect(checkpoints.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(checkpoints.flatMap((lesson) => compileLessonActivities(lesson.blocks))).toHaveLength(11);

  // Chapter 4 used to splice its R2 checkpoint into hand-written TeX behind a
  // comment. The generated chapter must instead name and render the canonical
  // lesson so lesson-level validation sees it.
  const generatedChapter4 = readFileSync(
    join(defaultCurriculumRoot(), "punjabi", "book", "chapters", "ch04-farewells.tex"),
    "utf8",
  );
  expect(generatedChapter4.startsWith("% GENERATED FILE.")).toBe(true);
  expect(generatedChapter4).not.toContain("canonical-insertion:");
  expect(generatedChapter4).toContain("% canonical-lessons: ");
  expect(generatedChapter4).toContain("PA-R04-wellbeing-r2");
  expect(generatedChapter4).toContain("label{lesson:PA-R04-wellbeing-r2}");
  expect(generatedChapter4).toContain("Romanization records speech only");

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
});
