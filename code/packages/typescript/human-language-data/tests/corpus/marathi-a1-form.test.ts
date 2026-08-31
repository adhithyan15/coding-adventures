import { expect, it } from "vitest";
import { loadTrackLessons } from "../../src/loader.js";
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

it("removes support gently before Marathi's independent A1 practical form", () => {
  const lessons = loadTrackLessons("marathi")
    .sort(readingOrder)
    .filter((lesson) => FORM_IDS.includes(lesson.realization.lessonId));

  expect(lessons.map((lesson) => lesson.realization.lessonId)).toEqual(FORM_IDS);
  expect(lessons.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual(
    Array.from({ length: 10 }, (_, index) => 860 + index),
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
