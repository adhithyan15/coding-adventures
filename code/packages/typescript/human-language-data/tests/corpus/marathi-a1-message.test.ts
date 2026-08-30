import { expect, it } from "vitest";
import { loadTrackLessons } from "../../src/loader.js";
import { readingOrder } from "../../src/ramp.js";

const MESSAGE_IDS = Array.from(
  { length: 18 },
  (_, index) => `MR-A1M${String(index + 1).padStart(2, "0")}-`,
);

it("removes support gently before Marathi's independent A1 named-reader message", () => {
  const lessons = loadTrackLessons("marathi")
    .sort(readingOrder)
    .filter((lesson) => MESSAGE_IDS.some((prefix) => lesson.realization.lessonId.startsWith(prefix)));

  expect(lessons).toHaveLength(18);
  expect(lessons.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-A1M01-reader-greeting",
    "MR-A1M02-name-line",
    "MR-A1M03-meeting-line",
    "MR-A1M04-language-line",
    "MR-A1M05-reading-line",
    "MR-A1M06-writing-line",
    "MR-A1M07-speaking-line",
    "MR-A1M08-thinking-line",
    "MR-A1M09-understanding-line",
    "MR-A1M10-thanks-line",
    "MR-A1M11-again-line",
    "MR-A1M12-tomorrow-line",
    "MR-A1M13-opening-block",
    "MR-A1M14-skills-block-one",
    "MR-A1M15-skills-block-two",
    "MR-A1M16-closing-block",
    "MR-A1M17-guided-message",
    "MR-A1M18-independent-message",
  ]);
  expect(lessons.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual(
    Array.from({ length: 18 }, (_, index) => 840 + index),
  );
  expect(lessons.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 299)).toBe(true);

  const stages = lessons.map((lesson) =>
    lesson.blocks.find((block) => block.writingStage)?.writingStage,
  );
  expect(stages).toEqual([
    "observe-trace",
    "guided-copy",
    "guided-copy",
    ...Array(9).fill("delayed-copy"),
    ...Array(4).fill("controlled-composition"),
    "connected-composition",
    "connected-composition",
  ]);

  for (let index = 1; index < lessons.length; index += 1) {
    expect(lessons[index]?.frontmatter.prerequisites).toContain(
      lessons[index - 1]?.realization.lessonId,
    );
  }
});

it("keeps the final Marathi named-reader checkpoint untimed and model-free", () => {
  const final = loadTrackLessons("marathi").find(
    (lesson) => lesson.realization.lessonId === "MR-A1M18-independent-message",
  );
  expect(final).toBeDefined();
  expect(final?.frontmatter.romanization).toBeUndefined();
  expect(final?.frontmatter.type).toBe("writing");

  const markdown = final?.blocks.map((block) => block.markdown).join("\n") ?? "";
  expect(markdown).toContain("Write **30–40 words** to Mira");
  expect(markdown).toContain("no romanization, line bank, block card, or visible answer model");
  expect(markdown).toContain("The task is untimed");
  expect(markdown).toContain("does not claim form completion");

  const activity = final?.blocks.flatMap((block) => block.activities ?? [])[0];
  expect(activity?.prompt).toContain("Without a model");
  expect(activity?.answer.trim().split(/\s+/)).toHaveLength(32);
});
