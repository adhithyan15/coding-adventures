import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { measureContinuity, REINFORCEMENT_WINDOWS } from "../../../src/continuity.js";
import {
  DOC_SHARD_PLANS,
  defaultRepoRoot,
  unshardDocContents,
} from "../../../src/doc-shard-cli.js";
import { loadTrackChapters, loadTrackLessons } from "../../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("keeps the Gujarati session map aligned with canonical chapter and lesson order", () => {
  const ordered = loadTrackLessons("gujarati").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const ledger = loadTrackChapters().find((track) => track.language === "gujarati");
  expect(ledger).toBeDefined();

  const sessionMapPath = "code/learning/human-languages/gujarati/session-map.md";
  const plan = DOC_SHARD_PLANS.find((candidate) => candidate.path === sessionMapPath);
  expect(plan).toBeDefined();
  const markdown = unshardDocContents(defaultRepoRoot(), plan!);
  const inventory = markdown
    .split("## Canonical session inventory", 2)[1]
    ?.split("## Current boundary", 1)[0];
  expect(inventory).toBeDefined();

  const rows = [...inventory!.matchAll(
    /^\| (\d+(?:-\d+)?) \| (\d+) \| ([^|]+?) \| (.+) \|$/gm,
  )].map((match) => ({
    range: match[1]!,
    chapter: Number(match[2]),
    title: match[3]!.trim(),
    ids: [...match[4]!.matchAll(/`(GU-[A-Za-z0-9-]+)`/g)].map((id) => id[1]!),
  }));

  expect(rows.map(({ chapter, title }) => ({ chapter, title }))).toEqual(
    ledger!.chapters.map(({ chapter, title }) => ({ chapter, title })),
  );
  expect(rows.flatMap((row) => row.ids)).toEqual(
    ordered.map((lesson) => lesson.realization.lessonId),
  );

  let nextSession = 1;
  for (const row of rows) {
    const [startText, endText = startText] = row.range.split("-");
    const start = Number(startText);
    const end = Number(endText);
    expect(start).toBe(nextSession);
    expect(end - start + 1).toBe(row.ids.length);
    nextSession = end + 1;
  }
  expect(nextSession - 1).toBe(ordered.length);
  expect(new Set(rows.flatMap((row) => row.ids)).size).toBe(ordered.length);
});
