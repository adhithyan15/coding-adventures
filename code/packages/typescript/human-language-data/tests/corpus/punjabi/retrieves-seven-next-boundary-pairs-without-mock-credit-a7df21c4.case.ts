import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";

const lessons = loadTrackLessons("punjabi").sort(
  (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
);
const ids = [
  "PA-W14-timed-repair-return",
  "PA-R159-people-quality-recall",
  "PA-R159-strength-weakness-recall",
  "PA-R159-hide-and-quality-mix",
];
const reviews = lessons.filter((lesson) => ids.includes(lesson.realization.lessonId));
const before = measureContinuity(
  lessons.filter((lesson) => !ids.includes(lesson.realization.lessonId)),
);
const after = measureContinuity(lessons);
const missedPairs = (report: typeof before): Set<string> => new Set(
  report.reinforcement.flatMap((defect) =>
    defect.missed.map((window) => `${window}|${defect.atom}`),
  ),
);

it("retrieves the seven Punjabi next-boundary pairs without claiming mock credit", () => {
  expect(reviews.map((lesson) => lesson.realization.lessonId)).toEqual(ids);
  expect(reviews.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual([
    8770, 8780, 8790, 8800,
  ]);
  expect(reviews.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 300)).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(reviews[0].realization.type).toBe("writing");
  expect(reviews[0].frontmatter.skills).toContain("writing");
  expect(reviews[0].frontmatter["practises.knowledge"]).toContain("PA-A1-TIMED-REPAIR-01");
  expect(reviews.slice(1).every((lesson) => lesson.realization.type === "review")).toBe(true);
  expect(reviews.slice(1).every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(reviews.every((lesson) => !lesson.body.includes("timed-assessment-production"))).toBe(true);

  const lexicalAtoms = [
    ...[1, 2, 3, 4, 5].map((number) => `PA-LEX-C147-QUAL147-0${number}`),
    "PA-LEX-C148-ACT148-01",
  ];
  const practised = new Set(
    reviews.slice(1).flatMap((lesson) => lesson.frontmatter["practises.knowledge"] ?? []),
  );
  expect(lexicalAtoms.every((atom) => practised.has(atom))).toBe(true);

  const previousPairs = missedPairs(before);
  const currentPairs = missedPairs(after);
  expect([...previousPairs].filter((pair) => !currentPairs.has(pair)).sort()).toEqual([
    "R2|PA-A1-TIMED-REPAIR-01",
    ...lexicalAtoms.map((atom) => `R4|${atom}`),
  ].sort());
});
