import { beforeAll, expect, it } from "vitest";
import {
  curriculumForLanguage,
  loadCurriculumPlans,
  mappedLessonIds,
  mixedCurriculumFrontier,
} from "../../src/curriculum";

beforeAll(async () => {
  await loadCurriculumPlans();
});

it("pins Persian's lesson-one alef bridge and greeting payoff", () => {
  const curriculum = curriculumForLanguage("persian")!;
  const script = curriculum.extensions.find((extension) =>
    extension.id === "FA-EXT-001-INLINE-SCRIPT"
  );
  const payoff = curriculum.extensions.find((extension) =>
    extension.id === "FA-EXT-001-GREETING-PAYOFF"
  );

  // The inline-script extension grew from two lessons to four: the same alef now
  // carries the whole pre-A1 writing ladder, adding a delayed copy with the
  // model covered and a dictation from the long vowel alone. One stroke is
  // enough to ask all four stages, because the stages are about what the hand is
  // asked to do rather than about how much language is on the page.
  expect(script).toMatchObject({
    kind: "required",
    category: "script",
    lessons: [
      "FA-C01-salam",
      "FA-W00-alef-guided-copy",
      "FA-W00-alef-delayed-copy",
      "FA-W00-alef-dictation",
    ],
  });
  expect(payoff).toMatchObject({
    kind: "required",
    category: "consolidation",
    prerequisites: ["FA-EXT-001-INLINE-SCRIPT"],
    lessons: ["FA-C01-practice"],
  });
  const mapped = mappedLessonIds(["persian"]);
  for (const lessonId of ["FA-C01-salam", "FA-W00-alef-guided-copy", "FA-C01-practice"]) {
    expect(mapped.has(lessonId), lessonId).toBe(true);
  }
});

it("keeps Persian on the greeting spine until the alef bridge is complete", () => {
  const progress = new Map<string, ReadonlySet<string>>([
    ["persian", new Set(["FA-C01-salam"])],
    ["urdu", new Set()],
  ]);
  const frontier = mixedCurriculumFrontier(["persian", "urdu"], progress);

  expect(frontier.steps.map((step) => [step.language, step.lessonId])).toEqual([
    ["persian", "FA-W00-alef-guided-copy"],
    ["urdu", "UR-C01-salam"],
  ]);
  expect(frontier.bySpineNode.get("SPINE-MEET-GREET")?.map((step) => step.language))
    .toEqual(["persian", "urdu"]);
  expect(frontier.bySpineNode.get("SPINE-COURTESY-THANK")).toBeUndefined();
});
