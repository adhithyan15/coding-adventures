import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";

it("services Chapter 146 quality R4 with an honest next boundary", () => {
  // Hold this chapter's historical boundary while Chapter 159 has its own
  // before/after owner for the next retrieval tranche.
  const lessons = loadTrackLessons("punjabi").filter(
    (lesson) => Number(lesson.frontmatter.sequence) <= 8760,
  ).sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const reviewIds = ["PA-R158-food-quality-recall", "PA-R158-truth-and-food-check"];
  const review = lessons.filter((lesson) => reviewIds.includes(lesson.realization.lessonId));
  expect(review.map((lesson) => lesson.realization.lessonId)).toEqual(reviewIds);
  expect(review.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual([8750, 8760]);
  expect(review.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 300)).toBe(true);
  expect(review.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(review.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(review.every((lesson) => !lesson.body.includes("timed-assessment-production"))).toBe(true);

  const qualityAtoms = [1, 2, 3, 4, 5].map(
    (number) => `PA-LEX-C146-QUAL146-0${number}`,
  );
  const practised = new Set(
    review.flatMap((lesson) => lesson.frontmatter["practises.knowledge"] ?? []),
  );
  expect(qualityAtoms.every((atom) => practised.has(atom))).toBe(true);

  const before = measureContinuity(
    lessons.filter((lesson) => !reviewIds.includes(lesson.realization.lessonId)),
  );
  const after = measureContinuity(lessons);
  expect(before.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 513, R4: 581 });
  expect(after.summary.missedByWindow).toEqual({ R1: 56, R2: 567, R3: 513, R4: 578 });

  const missedPairs = (report: typeof before): Set<string> => new Set(
    report.reinforcement.flatMap((defect) =>
      defect.missed.map((window) => `${window}|${defect.atom}`),
    ),
  );
  const previousPairs = missedPairs(before);
  const currentPairs = missedPairs(after);
  expect([...previousPairs].filter((pair) => !currentPairs.has(pair)).sort()).toEqual(
    qualityAtoms.map((atom) => `R4|${atom}`).sort(),
  );
  // Extending the measurable tail opens three *different* pairs. #16801 owns
  // their later retrieval; they must not be credited to this lexical review.
  expect([...currentPairs].filter((pair) => !previousPairs.has(pair)).sort()).toEqual([
    "R2|PA-A1-TIMED-REPAIR-01",
    "R4|PA-LEX-C147-QUAL147-05",
    "R4|PA-LEX-C148-ACT148-01",
  ]);
});
