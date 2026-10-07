import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { isEtymologyAtom } from "../../../src/level-gate.js";
import { loadTrackLessons } from "../../../src/loader.js";

it("retrieves short writing skills before the R1/R2 boundary without mock credit", () => {
  // Pin the Chapter 157/158 boundary independently of the later Chapter 159
  // tranche, which has its own exact pair-diff test.
  const lessons = loadTrackLessons("punjabi").filter(
    (lesson) => Number(lesson.frontmatter.sequence) <= 8760,
  ).sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const ids = [
    "PA-W13-form-clock-return",
    "PA-W13-repair-check-return",
    "PA-W13-message-opening-return",
    "PA-W13-message-purpose-return",
  ];
  const reviews = lessons.filter((lesson) => ids.includes(lesson.realization.lessonId));
  expect(reviews.map((lesson) => lesson.realization.lessonId)).toEqual(ids);
  expect(reviews.map((lesson) => lesson.realization.type)).toEqual(ids.map(() => "writing"));
  expect(reviews.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual([8742, 8744, 8746, 8748]);
  expect(reviews.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 300)).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(reviews.every((lesson) => !lesson.body.includes("hl-writing-stage: timed-assessment-production"))).toBe(true);

  const form = ["PA-A1-FORM-MIXED-CUES-01", "PA-A1-FORM-TIMED-01", "PA-A1-TIMED-REPAIR-01"];
  const message = ["PA-A1-MESSAGE-PACING-01", "PA-A1-MESSAGE-TIMED-01"];
  for (const lesson of reviews.slice(0, 2)) {
    expect(lesson.frontmatter["practises.knowledge"]).toEqual(form);
  }
  for (const lesson of reviews.slice(2)) {
    expect(lesson.frontmatter["practises.knowledge"]).toEqual(message);
  }

  const before = measureContinuity(
    lessons.filter((lesson) => !ids.includes(lesson.realization.lessonId)),
  );
  const after = measureContinuity(lessons);
  expect(before.summary.missedByWindow).toEqual({ R1: 57, R2: 568, R3: 513, R4: 574 });
  expect(after.summary.missedByWindow).toEqual({ R1: 56, R2: 567, R3: 513, R4: 578 });
  const pairs = (report: typeof before): Set<string> => new Set(
    report.reinforcement.flatMap((defect) =>
      defect.missed.map((window) => `${window}|${defect.atom}`),
    ),
  );
  const previous = pairs(before);
  const current = pairs(after);
  expect([...previous].filter((pair) => !current.has(pair)).sort()).toEqual([
    "R1|PA-A1-TIMED-REPAIR-01",
    "R2|PA-A1-FORM-MIXED-CUES-01",
    "R2|PA-A1-FORM-TIMED-01",
  ]);
  // The two message atoms receive their second revisits before their R2
  // windows close. More distant pairs now become measurable under #16801.
  expect([...current].filter((pair) => !previous.has(pair)).sort()).toEqual([
    "R2|PA-A1-TIMED-REPAIR-01",
    "R4|PA-LEX-C147-QUAL147-03",
    "R4|PA-LEX-C147-QUAL147-04",
    "R4|PA-LEX-C147-QUAL147-05",
    "R4|PA-LEX-C148-ACT148-01",
  ]);
  expect(after.reinforcement.filter((defect) =>
    defect.revisits < 2 && !isEtymologyAtom(defect.atom),
  )).toEqual([]);
});
