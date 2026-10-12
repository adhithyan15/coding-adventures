import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";
import { directedLessonSeconds } from "../timed-lesson-budget.js";

it("keeps Malayalam's timed checkpoint inside the lesson and the full paper separate", () => {
  const lesson = loadTrackLessons("malayalam").find(
    (candidate) => candidate.realization.lessonId === "ML-W108-a1-timed-production",
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
  expect(timed?.markdown).toContain("all six fields in Malayalam script");
  expect(timed?.markdown).toContain("**stop**");
  expect(timed?.markdown).not.toMatch(/[\u0D00-\u0D7F]/u);

  expect(markdown).toContain("../mocks/a1/mock-1.md");
  expect(markdown).toContain("separate assessment session");
  expect(markdown).toContain("not show you any copyable Malayalam answer");
  const mockDir = resolve(
    dirname(fileURLToPath(import.meta.url)),
    "../../../../../../learning/human-languages/malayalam/mocks/a1",
  );
  for (const name of ["mock-1.md", "mock-2.md"]) {
    const mock = readFileSync(resolve(mockDir, name), "utf8");
    expect(mock).toContain("## Writing — 20 minutes");
    expect(mock).toContain("spend 7 minutes on the practical form");
    expect(mock).toContain("13 minutes on the named-reader message");
    expect(mock).toMatch(/1\. Complete[\s\S]*2\. Write 30–40 Malayalam words/);
    expect(mock).toMatch(/The reader\s+and purpose must be clear/);
  }
  expect(markdown).not.toContain("<!-- hl-activity:");

  const malayalam = languageWritingStages("malayalam");
  expect(malayalam.defects).toEqual([]);
  expect(malayalam.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
