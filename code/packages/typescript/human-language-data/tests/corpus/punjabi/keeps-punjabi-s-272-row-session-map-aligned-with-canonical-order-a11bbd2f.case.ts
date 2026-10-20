import { expect, it } from "vitest";
import {
  DOC_SHARD_PLANS,
  defaultRepoRoot,
  unshardDocContents,
} from "../../../src/doc-shard-cli.js";
import { loadTrackLessons } from "../../../src/loader.js";

it("keeps Punjabi's session map aligned with canonical order", () => {
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

  // Row count comes from the reconstructed direct owners. Identity equality
  // below still rejects a missing or extra row; this derived range avoids a
  // second mutable owner for the same assembled count.
  expect(rows.map((row) => row.session)).toEqual(
    Array.from({ length: rows.length }, (_, index) => index + 1),
  );
  expect(rows.map((row) => row.lessonId)).toEqual(
    ordered.map((lesson) => lesson.realization.lessonId),
  );
  expect(rows.map((row) => row.chapter)).toEqual(
    ordered.map((lesson) => String(lesson.frontmatter.chapter)),
  );
});
