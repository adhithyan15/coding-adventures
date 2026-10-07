import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { measureContinuity } from "../../../src/continuity.js";
import {
  DOC_SHARD_PLANS,
  defaultRepoRoot,
  unshardDocContents,
} from "../../../src/doc-shard-cli.js";
import { loadTrackLessons } from "../../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("keeps Punjabi's session map aligned with canonical order", () => {
  // 272 -> 278 rows: HL-C437 appends chapter 48, six `review` lessons, at the END
  // of the sequence. Appending rather than inserting is why this is a six-row
  // addition and not a renumbering: every existing session keeps its number.
  // 278 -> 532 rows: chapters 49-98 (250 word lessons and four reviews) are
  // appended the same way, after chapter 48.
  // 532 -> 533 rows: HL-C443 inserts ਮੌਸਮ (mausam) before the chapter-20 ੌ
  // lesson. This one IS an insertion, so every later session moves up by one.
  // 533 -> 818 rows: Punjabi A1. Three chapter-6 continuations are INSERTED
  // (every later session moves up by three), and chapters 99-152 (270 word
  // lessons and twelve reviews) are appended after chapter 98.
  const ordered = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const sessionMapPath = "code/learning/human-languages/punjabi/session-map.md";
  const plan = DOC_SHARD_PLANS.find((candidate) => candidate.path === sessionMapPath);
  expect(plan).toBeDefined();
  const markdown = unshardDocContents(defaultRepoRoot(), plan!);
  const rows = [...markdown.matchAll(/^\| (\d+) \| (\d+) \| ([^|]+?) \| (.+) \|$/gm)].map(
    (match) => ({
      session: Number(match[1]),
      chapter: match[2],
      lessonId: match[3]!.trim(),
    }),
  );
  // 818 -> 820: chapter 7's hona and khana each split a continuation off.
  // 820 -> 831: chapter 153 appends the 11-step fictional date-field runway.
  // 831 -> 838: chapter 154 joins age, phone, and date in seven short steps.
  // 838 -> 844: chapter 155 joins six taught fields in six bounded steps.
  // 844 -> 863: chapter 156 builds an untimed named-reader message in 19 short steps.
  // 863 -> 868: chapter 157 adds five bounded timed-writing and repair steps.
  // 868 -> 872: four short, unscored writing-skill returns.
  // 872 -> 874: two short, no-new-language Chapter 158 quality-word reviews.
  expect(rows).toHaveLength(874);
  expect(rows.map((row) => row.session)).toEqual(Array.from({ length: 874 }, (_, index) => index + 1));
  expect(rows.map((row) => row.lessonId)).toEqual(
    ordered.map((lesson) => lesson.realization.lessonId),
  );
  expect(rows.map((row) => row.chapter)).toEqual(
    ordered.map((lesson) => String(lesson.frontmatter.chapter)),
  );
});
