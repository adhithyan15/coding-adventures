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

  expect(script).toMatchObject({
    kind: "required",
    category: "script",
    lessons: ["FA-C01-salam", "FA-W00-alef-guided-copy"],
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
