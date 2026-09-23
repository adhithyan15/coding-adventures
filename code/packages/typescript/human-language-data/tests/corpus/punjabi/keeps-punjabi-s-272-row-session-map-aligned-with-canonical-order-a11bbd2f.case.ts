import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { measureContinuity } from "../../../src/continuity.js";
import {
  DOC_SHARD_PLANS,
  defaultRepoRoot,
  unshardDocContents,
} from "../../../src/doc-shard-cli.js";
import { defaultCurriculumRoot, loadTrackLessons } from "../../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("keeps Punjabi's 272-row session map aligned with canonical order", () => {
  // 272 -> 278 rows: HL-C437 appends chapter 48, six `review` lessons, at the END
  // of the sequence. Appending rather than inserting is why this is a six-row
  // addition and not a renumbering: every existing session keeps its number.
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
  expect(rows).toHaveLength(278);
  expect(rows.map((row) => row.session)).toEqual(Array.from({ length: 278 }, (_, index) => index + 1));
  expect(rows.map((row) => row.lessonId)).toEqual(
    ordered.map((lesson) => lesson.realization.lessonId),
  );
  expect(rows.map((row) => row.chapter)).toEqual(
    ordered.map((lesson) => String(lesson.frontmatter.chapter)),
  );
});
