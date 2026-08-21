import { beforeAll, describe, expect, it } from "vitest";
import {
  LANGUAGE_CURRICULA,
  LANGUAGE_ORDER,
  MAPPED_LANGUAGE_IDS,
  SPINE_NODES,
  curriculumForLanguage,
  loadCurriculumPlans,
  mappedLessonIds,
  mixedCurriculumFrontier,
  spineNodeById,
} from "../src/curriculum";

// The plans are fetched, not bundled into the shell (see src/curriculum.ts), so
// every assertion below is about state that exists only after the fetch. The
// app awaits the same promise before its first plan-dependent render.
beforeAll(async () => {
  await loadCurriculumPlans();
});

describe("per-language shared-spine maps", () => {
  it("knows which tracks are mapped without loading a single plan", () => {
    expect([...MAPPED_LANGUAGE_IDS]).toEqual(LANGUAGE_ORDER);
  });

  it("names each track the same in its directory and in its plan", () => {
    expect(LANGUAGE_CURRICULA.map((curriculum) => curriculum.language))
      .toEqual([...MAPPED_LANGUAGE_IDS]);
  });

  it("bundles one complete map for every active language", () => {
    expect(LANGUAGE_CURRICULA.map((curriculum) => curriculum.language)).toEqual(LANGUAGE_ORDER);
    expect(LANGUAGE_CURRICULA).toHaveLength(LANGUAGE_ORDER.length);
    for (const curriculum of LANGUAGE_CURRICULA) {
      expect(Object.keys(curriculum.spine)).toEqual(SPINE_NODES.map((node) => node.id));
    }
  });

  it("keeps repeated local visits and explicit relocations", () => {
    const spanish = curriculumForLanguage("spanish")!;
    expect(spanish.spine["SPINE-MEET-GREET"]?.segments.length).toBeGreaterThan(1);
    expect(spanish.spine["SPINE-TAKE-LEAVE"]?.relocates["GREETING-GOODNIGHT"])
      .toBe("SPINE-TIME-OF-DAY");
  });

  it("resolves a shared ability by its stable node id", () => {
    expect(spineNodeById("SPINE-MEET-GREET")?.canDo).toContain("greeting");
    expect(spineNodeById("MISSING")).toBeUndefined();
  });

  it("derives each selected track's admitted lessons from its authored path", () => {
    for (const language of LANGUAGE_ORDER) {
      const expected = new Set(
        curriculumForLanguage(language)!.path.flatMap((segment) => segment.lessons),
      );
      expect(mappedLessonIds([language]), `${language} mapped lessons`).toEqual(expected);
    }
  });

  it("computes one independent next step per selected track before grouping abilities", () => {
    const selected = ["persian", "urdu"];
    const progress = new Map<string, ReadonlySet<string>>(selected.map((language) => [
      language,
      new Set<string>(),
    ]));
    const frontier = mixedCurriculumFrontier(selected, progress);

    expect(frontier.steps.map((step) => step.language)).toEqual(selected);
    for (const step of frontier.steps) {
      expect(mappedLessonIds([step.language]).has(step.lessonId)).toBe(true);
    }
    expect(frontier.bySpineNode.get("SPINE-MEET-GREET")?.map((step) => step.language))
      .toEqual(selected);
  });
});
