import { expect, it } from "vitest";
import { loadTrackLessons } from "../../src/loader.js";
import { readingOrder } from "../../src/ramp.js";

const NAME_FIELD_IDS = [
  "HI-A1F01-name-label",
  "HI-A1F01-name-select",
  "HI-A1F01-name-supported",
  "HI-A1F01-name-delayed",
  "HI-A1F01-name-no-model",
];

it("removes support gently before Hindi's first A1 name field", () => {
  const lessons = loadTrackLessons("hindi")
    .sort(readingOrder)
    .filter((lesson) => NAME_FIELD_IDS.includes(lesson.realization.lessonId));

  expect(lessons.map((lesson) => lesson.realization.lessonId)).toEqual(NAME_FIELD_IDS);
  expect(lessons.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual([
    2560, 2570, 2580, 2590, 2600,
  ]);
  expect(lessons.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 180)).toBe(true);

  const stages = lessons.map((lesson) =>
    lesson.blocks.find((block) => block.writingStage)?.writingStage,
  );
  expect(stages).toEqual([
    "guided-copy",
    undefined,
    "guided-copy",
    "controlled-composition",
    "controlled-composition",
  ]);

  for (let index = 1; index < lessons.length; index += 1) {
    expect(lessons[index]?.frontmatter.prerequisites).toContain(
      lessons[index - 1]?.realization.lessonId,
    );
  }
});

it("keeps supported copying separate from delayed and no-model Hindi form evidence", () => {
  const lessons = loadTrackLessons("hindi");
  const supported = lessons.find(
    (lesson) => lesson.realization.lessonId === "HI-A1F01-name-supported",
  );
  const delayed = lessons.find(
    (lesson) => lesson.realization.lessonId === "HI-A1F01-name-delayed",
  );
  const final = lessons.find(
    (lesson) => lesson.realization.lessonId === "HI-A1F01-name-no-model",
  );

  expect(final).toBeDefined();
  expect(final?.frontmatter.romanization).toBeUndefined();
  expect(final?.frontmatter.type).toBe("writing");

  const supportedMarkdown = supported?.blocks.map((block) => block.markdown).join("\n") ?? "";
  const delayedMarkdown = delayed?.blocks.map((block) => block.markdown).join("\n") ?? "";
  const finalMarkdown = final?.blocks.map((block) => block.markdown).join("\n") ?? "";
  expect(supportedMarkdown).toContain("not independent writing evidence");
  expect(delayedMarkdown).toContain("there is no copyable answer beside the line");
  expect(finalMarkdown).toContain("There is no Devanagari value bank");
  expect(finalMarkdown).toContain("does not claim full A1 readiness");

  const finalActivity = final?.blocks.flatMap((block) => block.activities ?? [])[0];
  expect(finalActivity?.prompt).toContain("no bank, romanization, or copyable answer");
  expect(finalActivity?.answer).toBe("मीरा");
});

it("keeps timing claims out of Hindi's first untimed A1 field runway", () => {
  const lessons = loadTrackLessons("hindi")
    .filter((lesson) => NAME_FIELD_IDS.includes(lesson.realization.lessonId));
  expect(lessons.some((lesson) => lesson.body.includes("timed-assessment-production"))).toBe(false);
});
