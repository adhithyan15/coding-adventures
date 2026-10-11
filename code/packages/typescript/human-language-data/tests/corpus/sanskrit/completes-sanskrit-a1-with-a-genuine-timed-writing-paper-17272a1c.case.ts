import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

// A timed lesson may rehearse an exam skill, but the authentic paper belongs
// outside its five-minute envelope. Count the clock or summed task timings,
// whichever is longer, and require an explicit cap for the later review.
const minutes = "(?:\\d+|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|twenty)";
const minuteValues: Record<string, number> = {
  one: 1, two: 2, three: 3, four: 4, five: 5, six: 6, seven: 7,
  eight: 8, nine: 9, ten: 10, eleven: 11, twelve: 12, thirteen: 13,
  twenty: 20,
};

function minuteCount(value: string): number {
  return minuteValues[value.toLowerCase()] ?? Number(value);
}

function directedLessonSeconds(markdown: string): number {
  const plain = markdown.replaceAll("**", "");
  const beforeWrapUp = plain.split(/^## Wrap-up/mi)[0];
  const wrapUp = plain.split(/^## Wrap-up/mi)[1] ?? "";
  const clock = [
    ...beforeWrapUp.matchAll(new RegExp(`\\bset (?:one|a) timer for (${minutes}) minutes?`, "gi")),
    ...beforeWrapUp.matchAll(new RegExp(`\\bset a (${minutes})-minute timer`, "gi")),
  ].map((match) => minuteCount(match[1]));
  const subtasks = [
    ...beforeWrapUp.matchAll(new RegExp(`\\bTask \\d+[^\\n]*?(${minutes}) minutes?`, "gi")),
  ].map((match) => minuteCount(match[1]));
  const review = wrapUp.match(new RegExp(`\\b(?:at most|no more than) (${minutes}) minutes?`, "i"));
  if (!review) return Number.POSITIVE_INFINITY;
  return (Math.max(0, ...clock, subtasks.reduce((sum, value) => sum + value, 0))
    + minuteCount(review[1])) * 60;
}

it("keeps Sanskrit's timed checkpoint inside the lesson and the full paper separate", () => {
  const lesson = loadTrackLessons("sanskrit").find(
    (candidate) =>
      candidate.realization.lessonId === "SA-W65-a1-timed-production",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.skills).toEqual(["writing"]);
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(280);

  const timed = lesson?.blocks.find(
    (block) => block.writingStage === "timed-assessment-production",
  );
  const markdown = lesson?.blocks.map((block) => `## ${block.title}\n${block.markdown}`).join("\n") ?? "";
  expect(directedLessonSeconds(markdown)).toBeLessThanOrEqual(
    Number(lesson?.frontmatter["duration.max_seconds"]),
  );
  expect(timed?.markdown).toContain("three-minute timer");
  expect(markdown).toContain("../mocks/a1/mock-1.md");
  expect(markdown).toContain("separate assessment session");
  expect(timed?.markdown).toContain("**stop**");
  expect(timed?.markdown).not.toMatch(/[\u0900-\u097F]/u);

  const sanskrit = languageWritingStages("sanskrit");
  expect(sanskrit.defects).toEqual([]);
  expect(sanskrit.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});

it("rejects split 7+13-minute tasks and an unbounded post-task review", () => {
  const split = [
    "## Writing", "Task 1 — form: 7 minutes", "Task 2 — message: 13 minutes",
    "## Wrap-up", "Spend at most one minute reviewing the attempt.",
  ].join("\n");
  expect(directedLessonSeconds(split)).toBe(21 * 60);
  expect(directedLessonSeconds(split)).toBeGreaterThan(300);
  expect(directedLessonSeconds(split.replace("Spend at most one minute", "Spend time")))
    .toBe(Number.POSITIVE_INFINITY);
});
