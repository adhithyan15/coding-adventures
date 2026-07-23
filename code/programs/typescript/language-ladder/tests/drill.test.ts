import { describe, it, expect } from "vitest";
import {
  confusabilityOrder,
  topDistractors,
  buildDrillQuestion,
  checkAnswer,
  emptyScore,
  record,
  accuracy,
} from "../src/drill.ts";
import type { LetterView } from "../src/core.ts";
import { buildScriptView } from "../src/core.ts";
import { SCRIPTS } from "../src/data.ts";

function lv(glyph: string, over: Partial<LetterView> = {}): LetterView {
  return {
    glyph,
    sound: glyph,
    role: "consonant",
    components: [],
    strokeOrder: [],
    strokeOrderNote: "",
    notes: "",
    falseFriend: false,
    strokeCount: 0,
    ...over,
  };
}

// a=target(consonant, ff), b=consonant ff, c=consonant, d=vowel, e=vowel ff
const LETTERS: LetterView[] = [
  lv("a", { sound: "ah", role: "consonant", falseFriend: true }),
  lv("b", { role: "consonant", falseFriend: true }),
  lv("c", { role: "consonant", falseFriend: false }),
  lv("d", { role: "vowel", falseFriend: false }),
  lv("e", { role: "vowel", falseFriend: true }),
];

describe("confusabilityOrder", () => {
  it("ranks same-role + same-false-friend highest, excludes the target, stable on ties", () => {
    const order = confusabilityOrder(LETTERS, 0); // target 'a': consonant + ff
    expect(order).not.toContain(0); // target excluded
    // b (consonant+ff) is the best match → first
    expect(order[0]).toBe(1);
    // c (consonant, not-ff) beats the vowels
    expect(order[1]).toBe(2);
    // e (vowel but shares false-friend=true) outranks d (vowel, not-ff)
    expect(order).toEqual([1, 2, 4, 3]); // full deterministic order
  });

  it("returns empty for an out-of-range target", () => {
    expect(confusabilityOrder(LETTERS, 99)).toEqual([]);
  });
});

describe("buildDrillQuestion", () => {
  it("prompts with the sound and places the answer at placeAt", () => {
    const q = buildDrillQuestion(LETTERS, 0, 4, topDistractors, 2);
    expect(q.promptSound).toBe("ah");
    expect(q.targetGlyph).toBe("a");
    expect(q.options).toHaveLength(4);
    expect(q.answerIndex).toBe(2);
    expect(q.options[q.answerIndex]!.glyph).toBe("a");
    expect(checkAnswer(q, 2)).toBe(true);
    expect(checkAnswer(q, 0)).toBe(false);
  });

  it("fills distractors from the most-confusable first", () => {
    const q = buildDrillQuestion(LETTERS, 0, 4, topDistractors, 0);
    const distractorGlyphs = q.options.filter((_, i) => i !== q.answerIndex).map((o) => o.glyph);
    expect(distractorGlyphs).toEqual(["b", "c", "e"]); // top-3 confusable (b,c,e — e shares ff)
    expect(distractorGlyphs).not.toContain("a"); // never the target
  });

  it("never includes the target among distractors and never duplicates", () => {
    // a sloppy chooser that returns the target + a duplicate + out-of-range
    const q = buildDrillQuestion(LETTERS, 0, 4, () => [0, 1, 1, 99], 0);
    const glyphs = q.options.map((o) => o.glyph);
    expect(new Set(glyphs).size).toBe(glyphs.length); // no dupes
    const nonAnswer = q.options.filter((_, i) => i !== q.answerIndex);
    expect(nonAnswer.every((o) => o.letterIndex !== 0)).toBe(true); // no target
    expect(q.options).toHaveLength(4); // topped up to the requested count
  });

  it("clamps optionCount to the available letters (small inventory)", () => {
    const two = LETTERS.slice(0, 2);
    const q = buildDrillQuestion(two, 0, 4, topDistractors, 0);
    expect(q.options).toHaveLength(2); // only 2 letters exist
    expect(q.options.map((o) => o.glyph).sort()).toEqual(["a", "b"]);
  });

  it("clamps placeAt into range", () => {
    const q = buildDrillQuestion(LETTERS, 0, 3, topDistractors, 99);
    expect(q.answerIndex).toBe(q.options.length - 1);
    expect(q.options[q.answerIndex]!.glyph).toBe("a");
  });

  it("throws on an out-of-range target", () => {
    expect(() => buildDrillQuestion(LETTERS, 99)).toThrow(RangeError);
  });
});

describe("scoring", () => {
  it("accumulates correct/total immutably and computes accuracy", () => {
    let s = emptyScore();
    expect(accuracy(s)).toBeNull(); // no answers yet → not 0/0
    s = record(s, true);
    s = record(s, false);
    s = record(s, true);
    expect(s).toEqual({ correct: 2, total: 3 });
    expect(accuracy(s)).toBe(67);
  });
});

describe("with real script data", () => {
  it("builds a valid Cyrillic question with 4 distinct options containing the answer", () => {
    const cyr = buildScriptView(SCRIPTS.find((s) => s.script === "cyrillic")!);
    const q = buildDrillQuestion(cyr, 2, 4, topDistractors, 1); // в
    expect(q.options).toHaveLength(4);
    expect(new Set(q.options.map((o) => o.glyph)).size).toBe(4);
    expect(q.options[q.answerIndex]!.glyph).toBe(q.targetGlyph);
    expect(checkAnswer(q, q.answerIndex)).toBe(true);
  });
});
