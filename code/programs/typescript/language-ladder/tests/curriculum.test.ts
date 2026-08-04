import { describe, expect, it } from "vitest";
import {
  LANGUAGE_CURRICULA,
  LANGUAGE_ORDER,
  SPINE_NODES,
  curriculumForLanguage,
  mappedLessonIds,
  mixedCurriculumFrontier,
} from "../src/curriculum";

describe("per-language shared-spine maps", () => {
  it("bundles one complete map for every active language", () => {
    expect(LANGUAGE_CURRICULA.map((curriculum) => curriculum.language)).toEqual(LANGUAGE_ORDER);
    expect(LANGUAGE_CURRICULA).toHaveLength(20);
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

  it("makes Persian and Urdu script introduction an inline local extension", () => {
    for (const language of ["persian", "urdu"]) {
      const curriculum = curriculumForLanguage(language)!;
      const script = curriculum.extensions.find((extension) => extension.category === "script");
      expect(script?.kind).toBe("required");
      expect(script?.lessons).toHaveLength(1);
      const segment = curriculum.path.find((item) => item.inline.includes(script!.id));
      expect(segment?.spine_node).toBe("SPINE-MEET-GREET");
    }
  });

  it("exposes only mapped lessons for a selected mix", () => {
    const ids = mappedLessonIds(["persian", "urdu"]);
    expect(ids).toEqual(new Set([
      "FA-C01-salam",
      "FA-C01-mamnoon",
      "FA-C01-bale",
      "FA-C01-na",
      "FA-C02-esm-e-man",
      "UR-C01-salam",
      "UR-C01-shukriya",
      "UR-C01-ji-han",
      "UR-C01-nahin",
      "UR-C02-mera-naam",
    ]));
  });

  it("computes independent next steps before grouping shareable abilities", () => {
    const progress = new Map<string, ReadonlySet<string>>([
      ["persian", new Set(["FA-C01-salam"])],
      ["urdu", new Set()],
    ]);
    const frontier = mixedCurriculumFrontier(["persian", "urdu"], progress);
    expect(frontier.steps.map((step) => [step.language, step.lessonId])).toEqual([
      ["persian", "FA-C01-mamnoon"],
      ["urdu", "UR-C01-salam"],
    ]);
    expect(frontier.bySpineNode.get("SPINE-COURTESY-THANK")?.map((step) => step.language))
      .toEqual(["persian"]);
    expect(frontier.bySpineNode.get("SPINE-MEET-GREET")?.map((step) => step.language))
      .toEqual(["urdu"]);
  });
});
