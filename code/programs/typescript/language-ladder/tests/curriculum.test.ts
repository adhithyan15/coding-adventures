import { describe, expect, it } from "vitest";
import {
  LANGUAGE_CURRICULA,
  LANGUAGE_ORDER,
  SPINE_NODES,
  curriculumForLanguage,
  mappedLessonIds,
  mixedCurriculumFrontier,
  spineNodeById,
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

  it("resolves a shared ability by its stable node id", () => {
    expect(spineNodeById("SPINE-MEET-GREET")?.canDo).toContain("greeting");
    expect(spineNodeById("MISSING")).toBeUndefined();
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
      "FA-C03-shoma-to",
      "FA-C03-chist",
      "FA-C03-esm-e-shoma-chist",
      "FA-C03-khoshvaghtam",
      "FA-C03-practice",
      "FA-C04-hal",
      "FA-C04-chetor",
      "FA-C04-hal-e-shoma-chetor-ast",
      "FA-C04-khub",
      "FA-C04-khubam",
      "FA-C04-practice",
      "UR-C01-salam",
      "UR-C01-shukriya",
      "UR-C01-ji-han",
      "UR-C01-nahin",
      "UR-C02-mera-naam",
      "UR-C03-aap-tum-tu",
      "UR-C03-kya",
      "UR-C03-aap-ka-naam-kya-hai",
      "UR-C03-khushi-hui",
      "UR-C03-practice",
      "UR-C04-kaise-kaisi",
      "UR-C04-aap-kaise-hain",
      "UR-C04-main-hun",
      "UR-C04-thik",
      "UR-C04-main-thik-hun",
      "UR-C04-practice",
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
