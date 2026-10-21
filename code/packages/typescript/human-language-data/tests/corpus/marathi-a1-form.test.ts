import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import { loadTrackLessons } from "../../src/loader.js";
import { deriveLessonModality } from "../../src/modality.js";
import { narrateLesson, renderLessonNarrationText } from "../../src/narration.js";
import { readingOrder } from "../../src/ramp.js";

const FORM_IDS = [
  "MR-A1F04-first-three-select",
  "MR-A1F05-last-three-select",
  "MR-A1F06-first-three-supported",
  "MR-A1F07-first-three-spelling",
  "MR-A1F08-last-three-supported",
  "MR-A1F09-last-three-agreement",
  "MR-A1F10-first-three-delayed",
  "MR-A1F11-last-three-delayed",
  "MR-A1F12-six-field-supported",
  "MR-A1F13-six-field-independent",
];

it("retrieves all six Marathi A1 form labels and their agreement in every due window", () => {
  const formAtoms = new Set([
    "MR-A1-FORM-LABEL-NAAV",
    "MR-A1-FORM-LABEL-SHAHAR",
    "MR-A1-FORM-LABEL-BHASHA",
    "MR-A1-FORM-LABEL-AVDATE-PEY",
    "MR-A1-FORM-LABEL-AVADTI-KRUTI",
    "MR-A1-FORM-LABEL-MITRACHE-NAAV",
    "MR-A1-FORM-AGREEMENT-NEUTER",
    "MR-A1-FORM-AGREEMENT-FEMININE",
    "MR-A1-FORM-POSSESSIVE-NEUTER",
  ]);
  const lessons = loadTrackLessons("marathi");
  expect(measureContinuity(lessons).reinforcement.filter(({ atom }) => formAtoms.has(atom))).toEqual([]);

  for (const id of [
    "MR-A1F01-shahar",
    "MR-A1F01-bhasha",
    "MR-A1F01-avdate-pey",
    "MR-A1F01-avadti-kruti",
    "MR-A1M02-name-line",
  ]) {
    const lesson = lessons.find((candidate) => candidate.realization.lessonId === id);
    expect(lesson?.body).toContain("Before looking back");
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson).toBeDefined();
    if (!lesson) continue;
    const warmup = deriveLessonModality(lesson).blocks.find((block) => block.title === "Warm-up");
    expect(warmup?.eyesOrHandsSteps.length).toBeGreaterThan(0);
    const narration = narrateLesson(lesson);
    expect(narration.notice?.waitUntilStopped).toContain("Warm-up");
    expect(renderLessonNarrationText(narration)).toContain("[once you have stopped driving");
  }
});

it("removes support gently before Marathi's independent A1 practical form", () => {
  const lessons = loadTrackLessons("marathi")
    .sort(readingOrder)
    .filter((lesson) => FORM_IDS.includes(lesson.realization.lessonId));

  expect(lessons.map((lesson) => lesson.realization.lessonId)).toEqual(FORM_IDS);
  expect(lessons.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual(
    Array.from({ length: 10 }, (_, index) => 1060 + index),
  );
  expect(lessons.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 260)).toBe(true);

  const stages = lessons.map((lesson) =>
    lesson.blocks.find((block) => block.writingStage)?.writingStage,
  );
  expect(stages).toEqual([
    undefined,
    undefined,
    ...Array(8).fill("controlled-composition"),
  ]);

  for (let index = 1; index < lessons.length; index += 1) {
    expect(lessons[index]?.frontmatter.prerequisites).toContain(
      lessons[index - 1]?.realization.lessonId,
    );
  }
});

it("keeps the final Marathi practical-form checkpoint untimed, model-free, and value-changing", () => {
  const lessons = loadTrackLessons("marathi");
  const supported = lessons.find(
    (lesson) => lesson.realization.lessonId === "MR-A1F12-six-field-supported",
  );
  const final = lessons.find(
    (lesson) => lesson.realization.lessonId === "MR-A1F13-six-field-independent",
  );

  expect(final).toBeDefined();
  expect(final?.frontmatter.romanization).toBeUndefined();
  expect(final?.frontmatter.type).toBe("writing");

  const markdown = final?.blocks.map((block) => block.markdown).join("\n") ?? "";
  expect(markdown).toContain("no romanization, Marathi answer bank, or completed model");
  expect(markdown).toContain("The task is untimed");
  expect(markdown).toContain("does not claim timed writing or full A1 readiness");

  const supportedAnswer = supported?.blocks.flatMap((block) => block.activities ?? [])[0]?.answer;
  const finalActivity = final?.blocks.flatMap((block) => block.activities ?? [])[0];
  expect(finalActivity?.prompt).toContain("Without a Marathi value model");
  expect(finalActivity?.answer).not.toBe(supportedAnswer);
  expect(finalActivity?.answer).toContain("दूध");
  expect(finalActivity?.answer).toContain("लिहिणे");
});

it("keeps timing evidence out of the untimed practical-form runway", () => {
  const lessons = loadTrackLessons("marathi")
    .filter((lesson) => FORM_IDS.includes(lesson.realization.lessonId));
  expect(lessons.some((lesson) => lesson.body.includes("timed-assessment-production"))).toBe(false);
});

it("introduces a partial timer before a full timed six-field form", () => {
  const ids = ["MR-A1F13-six-field-independent", "MR-A1F14-partial-timer", "MR-A1F15-timed-form"];
  const lessons = ids.map((id) => loadTrackLessons("marathi").find((lesson) => lesson.realization.lessonId === id));
  expect(lessons.every(Boolean)).toBe(true);
  expect(lessons[1]?.frontmatter.prerequisites).toContain(ids[0]);
  expect(lessons[2]?.frontmatter.prerequisites).toContain(ids[1]);
  for (const lesson of lessons.slice(1)) {
    expect(lesson?.frontmatter.delivery).toBe("script");
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.body).toContain("hl-writing-stage: timed-assessment-production");
    expect(lesson?.body).toContain("task fulfilment");
    expect(lesson?.body).toContain("organisation");
    expect(lesson?.body).toContain("Marathi control");
    expect(lesson?.body).toContain("orthographic control");
    expect(lesson?.body).toContain("repair");
    const activities = lesson?.blocks.flatMap((block) => block.activities ?? []) ?? [];
    expect(activities).toHaveLength(2);
    expect(activities[1]?.id).toContain("separate-repair");
    expect(activities[1]?.responseSeconds).toBeLessThanOrEqual(20);
  }
  expect(lessons[1]?.body).toContain("partial timer");
  expect(lessons[2]?.body).toContain("six fields");
  expect(lessons[2]?.body).toContain("two minutes");
  expect(lessons[2]?.body).toContain("not a full A1 mock");
});
