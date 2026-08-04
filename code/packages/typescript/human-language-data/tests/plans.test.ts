import { describe, expect, it } from "vitest";
import {
  extensionsForSegment,
  mixedCurriculumFrontier,
  nextCurriculumLesson,
  orderedCurriculumLessonIds,
} from "../src/plans.js";
import type { LanguageCurriculum } from "../src/types.js";

const track = (language: string, secondNode = "THANK"): LanguageCurriculum => ({
  version: 1,
  language,
  path: [
    {
      id: `${language}-1`,
      spine_node: "GREET",
      lessons: [`${language}-script`, `${language}-hello`],
      before: [`${language}-script-extension`],
      inline: [],
      after: [],
    },
    {
      id: `${language}-2`,
      spine_node: secondNode,
      lessons: [`${language}-thanks`],
      before: [],
      inline: [],
      after: [],
    },
    {
      id: `${language}-3`,
      spine_node: "GREET",
      lessons: [`${language}-formal-hello`],
      before: [],
      inline: [],
      after: [],
    },
  ],
  spine: {
    GREET: { segments: [`${language}-1`, `${language}-3`], omits: [], relocates: {} },
    [secondNode]: { segments: [`${language}-2`], omits: [], relocates: {} },
  },
  extensions: [{
    id: `${language}-script-extension`,
    stage: "pre-A1",
    kind: "required",
    category: "script",
    canDo: "I can read the first shape.",
    prerequisites: [],
    lessons: [`${language}-script`],
  }],
});

describe("per-language curriculum plans", () => {
  it("preserves repeated spine-node visits in exact local lesson order", () => {
    const curriculum = track("fa");
    expect(orderedCurriculumLessonIds(curriculum)).toEqual([
      "fa-script",
      "fa-hello",
      "fa-thanks",
      "fa-formal-hello",
    ]);
    expect(curriculum.spine.GREET?.segments).toEqual(["fa-1", "fa-3"]);
  });

  it("advances one local lesson at a time and surfaces its attached extension", () => {
    const curriculum = track("ur");
    const first = nextCurriculumLesson(curriculum, new Set());
    expect(first).toMatchObject({ lessonId: "ur-script", spineNode: "GREET" });
    expect(first?.extensions.map(({ relation, extension }) => [relation, extension.category]))
      .toEqual([["before", "script"]]);

    const second = nextCurriculumLesson(curriculum, new Set(["ur-script"]));
    expect(second).toMatchObject({ lessonId: "ur-hello", spineNode: "GREET" });
    expect(second?.extensions).toEqual([]);
    expect(extensionsForSegment(curriculum, curriculum.path[0]!)).toHaveLength(1);
  });

  it("keeps progress independent and groups only simultaneously ready abilities", () => {
    const fa = track("fa");
    const ur = track("ur", "NAMES");
    const progress = new Map<string, ReadonlySet<string>>([
      ["fa", new Set(["fa-script", "fa-hello"])],
      ["ur", new Set()],
    ]);
    const frontier = mixedCurriculumFrontier([fa, ur], ["ur", "fa"], progress);
    expect(frontier.steps.map((step) => [step.language, step.lessonId])).toEqual([
      ["ur", "ur-script"],
      ["fa", "fa-thanks"],
    ]);
    expect(frontier.bySpineNode.get("GREET")?.map((step) => step.language)).toEqual(["ur"]);
    expect(frontier.bySpineNode.get("THANK")?.map((step) => step.language)).toEqual(["fa"]);
  });
});
