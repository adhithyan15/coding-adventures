import { describe, expect, it } from "vitest";
import { buildCompletionPlan, CERTIFIABLE_LEVELS, TASK_SHAPE_LEVELS } from "../src/completion-plan.js";
import { LEVEL_VOCABULARY } from "../src/level-gate.js";

function build(complete: boolean) {
  const language = "persian";
  return buildCompletionPlan({
    levelGate: {
      vocabularyTargets: LEVEL_VOCABULARY,
      tracks: [{ language, touches: null, attained: null, inProgressAt: "pre-A1", blockers: [], vocabulary: 0 }],
      summary: {
        tracksOverstating: 0,
        tracksWithAnyLevel: 0,
        attainedByLevel: { "pre-A1": 0, A1: 0, A2: 0, B1: 0, B2: 0, C1: 0, C2: 0 },
      },
    },
    scriptClosure: {
      tracks: [],
      violations: [],
      unknownScriptTracks: [],
      summary: {
        tracksWithScript: 0,
        tracksTeachingNothing: 0,
        violations: 0,
        exposureOnly: 0,
        exposureExemptedGlyphs: 0,
        headwordsWithoutRomanization: 0,
        tracksWithUnknownScript: 0,
      },
    },
    assessmentContracts: [language],
    taskShapes: TASK_SHAPE_LEVELS.map((level) => ({ language, level })),
    inventories: CERTIFIABLE_LEVELS.map((level) => ({ language, level })),
    externalCapstones: [{
      language,
      id: "samfa-academic",
      requiredAfterLevel: "C2",
      name: "SAMFA Academic",
      complete,
      missingArtifacts: complete ? [] : ["capstones/samfa.json", "mocks/samfa/rubric.md"],
    }],
  });
}

describe("external capstone planning", () => {
  it("keeps a declared capstone open until every referenced artifact exists", () => {
    const plan = build(false);
    expect(plan.head[0]).toMatchObject({
      id: "external-capstone/persian/samfa-academic",
      kind: "external-capstone",
      level: "C2",
      outstanding: 2,
    });
    expect(plan.head[0]?.goal).toContain("without inventing a CEFR equivalence");
    expect(plan.projection.find((entry) => entry.kind === "external-capstone")?.items).toBe(1);
  });

  it("retires the work item only when the capstone artifacts are complete", () => {
    const plan = build(true);
    expect(plan.head.some((item) => item.kind === "external-capstone")).toBe(false);
    expect(plan.projection.find((entry) => entry.kind === "external-capstone")?.items).toBe(0);
  });
});
