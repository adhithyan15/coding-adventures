import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";
import { directedLessonSeconds } from "../timed-lesson-budget.js";

it("keeps Bengali's timed checkpoint inside the lesson and the full paper separate", () => {
  const lesson = loadTrackLessons("bengali").find(
    (candidate) => candidate.realization.lessonId === "BN-W40-a1-timed-production",
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
  const mockDir = resolve(
    dirname(fileURLToPath(import.meta.url)),
    "../../../../../../learning/human-languages/bengali/mocks/a1",
  );
  for (const name of ["mock-1.md", "mock-2.md"]) {
    const mock = readFileSync(resolve(mockDir, name), "utf8");
    expect(mock).toContain("## Writing — 20 minutes");
    expect(mock).toContain("spend 7 minutes on the practical form");
    expect(mock).toContain("13 minutes on the named-reader message");
  }
  expect(timed?.markdown).toContain("**stop**");
  expect(timed?.markdown).not.toMatch(/[\u0980-\u09FF]/u);

  const bengali = languageWritingStages("bengali");
  expect(bengali.defects).toEqual([]);
  expect(bengali.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
