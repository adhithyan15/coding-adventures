import { describe, expect, it } from "vitest";
import { measureGlossedNotTaught, renderGlossedNotTaught } from "../src/glossed-not-taught.js";
import type { ParsedLesson } from "../src/parse.js";

function lesson(id: string, headword: string, body: string): ParsedLesson {
  return {
    language: "hindi",
    script: "devanagari",
    frontmatter: { id, headword, gloss: "fixture" },
    body,
    preamble: "",
    blocks: [],
    sourceHash: "fixture",
    realization: {
      concept: id,
      language: "hindi",
      lessonId: id,
      chapter: 1,
      type: "word",
      headword,
      gloss: "fixture",
      romanization: "",
      script: "devanagari",
      gender: null,
      sounds: [],
      roots: [],
      etymologyHook: "",
    },
  };
}

function withScript(value: ParsedLesson, language: string, script: string): ParsedLesson {
  return {
    ...value,
    language,
    script,
    realization: { ...value.realization, language, script },
  };
}

describe("measureGlossedNotTaught", () => {
  it("reports native-script tokens that never occur in a headword", () => {
    const report = measureGlossedNotTaught(
      [
        lesson("HI-C01", "नमस्ते", "Say नमस्ते. चाबी means key. अमरूद means guava."),
        lesson("HI-C02", "शुभ रात", "Retrieve रात and compare चाबी again."),
      ],
      "hindi",
    );

    expect(report.distinctHeadwordTokens).toBe(3);
    expect(report.candidates).toEqual([
      { token: "चाबी", occurrences: 2, lessonIds: ["HI-C01", "HI-C02"] },
      { token: "अमरूद", occurrences: 1, lessonIds: ["HI-C01"] },
    ]);
    expect(renderGlossedNotTaught(report)).toContain("candidates: 2");
  });

  it("rejects an unknown track", () => {
    expect(() => measureGlossedNotTaught([], "missing")).toThrow("unknown or lessonless track");
  });

  it("supports registry script aliases and Japanese's three script families", () => {
    const report = measureGlossedNotTaught(
      [withScript(lesson("JA-C01", "日本", "日本 and かな, but not English."), "japanese", "japanese")],
      "japanese",
    );
    expect(report.candidates.map((candidate) => candidate.token)).toEqual(["かな"]);
  });
});
