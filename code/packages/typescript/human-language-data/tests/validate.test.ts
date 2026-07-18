import { describe, it, expect } from "vitest";
import { parseLesson } from "../src/parse.js";
import { validate, hasErrors, summarize } from "../src/validate.js";
import type { ScriptData, Taxonomy } from "../src/types.js";

const lesson = (fields: Record<string, string>) =>
  ["---", ...Object.entries(fields).map(([k, v]) => `${k}: ${v}`), "---", "body"].join("\n");

const taxonomy: Taxonomy = {
  version: 1,
  concepts: {
    "GREETING-HELLO": { family: "GREETING", gloss: "hello", core: true },
    "COURTESY-THANKS": { family: "COURTESY", gloss: "thanks", core: true },
  },
};

const good = (lang: string, id: string, concept: string, extra: Record<string, string> = {}) =>
  parseLesson(lesson({ id, chapter: "1", type: "word", headword: "w", gloss: "g", concept_tag: concept, ...extra }), lang);

describe("validate", () => {
  it("passes a clean set with no errors", () => {
    const issues = validate({
      taxonomy,
      lessons: [good("spanish", "ES1", "GREETING-HELLO"), good("german", "DE1", "GREETING-HELLO")],
    });
    expect(hasErrors(issues)).toBe(false);
  });

  it("errors on an unresolved concept tag", () => {
    const issues = validate({ taxonomy, lessons: [good("spanish", "ES1", "NOT-A-REAL-TAG-lowercase!")] });
    expect(issues.some((i) => i.code === "unresolved-concept" && i.level === "error")).toBe(true);
  });

  it("rejects a concept tag that collides with an Object prototype member", () => {
    // Without an own-property check, `constructor`/`toString` would resolve via
    // the prototype chain and spuriously validate as canonical.
    for (const evil of ["constructor", "toString", "__proto__", "hasOwnProperty"]) {
      const issues = validate({ taxonomy, lessons: [good("spanish", "ES1", evil)] });
      expect(
        issues.some((i) => i.code === "unresolved-concept" && i.level === "error"),
        `${evil} should not validate`,
      ).toBe(true);
    }
  });

  it("accepts a namespaced tag without complaint", () => {
    const issues = validate({ taxonomy, lessons: [good("spanish", "ES1", "ES-WORD-DIA")] });
    expect(issues.some((i) => i.code === "unresolved-concept")).toBe(false);
  });

  it("errors on a duplicate realization within a language", () => {
    const issues = validate({
      taxonomy,
      lessons: [good("spanish", "ES1", "GREETING-HELLO"), good("spanish", "ES2", "GREETING-HELLO")],
    });
    expect(issues.some((i) => i.code === "duplicate-realization" && i.level === "error")).toBe(true);
  });

  it("errors on missing required fields", () => {
    const bare = parseLesson(lesson({ type: "word", concept_tag: "GREETING-HELLO" }), "spanish");
    const issues = validate({ taxonomy, lessons: [bare] });
    for (const code of ["missing-headword", "missing-gloss", "missing-chapter"]) {
      expect(issues.some((i) => i.code === code)).toBe(true);
    }
  });

  it("warns (not errors) on a non-Latin lesson missing romanization", () => {
    const issues = validate({ taxonomy, lessons: [good("telugu", "TE1", "GREETING-HELLO")] });
    expect(issues.some((i) => i.code === "missing-romanization" && i.level === "warning")).toBe(true);
    expect(hasErrors(issues)).toBe(false);
  });

  it("warns on an over-length etymology hook", () => {
    const long = "x".repeat(200);
    const issues = validate({ taxonomy, lessons: [good("spanish", "ES1", "GREETING-HELLO", { etymology_hook: long })] });
    expect(issues.some((i) => i.code === "long-etymology-hook")).toBe(true);
  });

  it("exempts practice lessons from the concept checks but flags unknown types", () => {
    const practice = parseLesson(lesson({ id: "P", chapter: "1", type: "practice-mix", headword: "(p)", gloss: "recap", concept_tag: "CH1-PRACTICE" }), "spanish");
    const weird = parseLesson(lesson({ id: "W", chapter: "1", type: "quiz", headword: "w", gloss: "g", concept_tag: "GREETING-HELLO" }), "spanish");
    const issues = validate({ taxonomy, lessons: [practice, weird] });
    expect(issues.some((i) => i.code === "unresolved-concept")).toBe(false); // practice exempt
    expect(issues.some((i) => i.code === "unknown-type")).toBe(true); // quiz flagged
  });

  it("treats missing core coverage as info, but as error for parity-complete tracks", () => {
    const lessons = [good("spanish", "ES1", "GREETING-HELLO")]; // missing COURTESY-THANKS
    const infoIssues = validate({ taxonomy, lessons });
    expect(infoIssues.some((i) => i.code === "core-coverage" && i.level === "info")).toBe(true);
    const errIssues = validate({ taxonomy, lessons, completeTracks: new Set(["spanish"]) });
    expect(errIssues.some((i) => i.code === "core-coverage" && i.level === "error")).toBe(true);
  });

  it("warns about uncovered glyphs, and escalates to an error once the script is complete", () => {
    const telugu = (complete: boolean): ScriptData => ({
      script: "telugu", name: "Telugu", font: "f", direction: "ltr", system: "abugida",
      letters: [{ glyph: "క", sound: "ka", role: "consonant", components: [], strokeOrder: [], strokeOrderNote: "" }],
      marks: [], complete,
    });
    const l = good("telugu", "TE1", "GREETING-HELLO", { headword: "కమ", romanization: "kama" });
    const warned = validate({ taxonomy, lessons: [l], scripts: { telugu: telugu(false) } });
    expect(warned.some((i) => i.code === "uncovered-glyphs" && i.level === "warning")).toBe(true);
    expect(hasErrors(warned)).toBe(false);
    const errored = validate({ taxonomy, lessons: [l], scripts: { telugu: telugu(true) } });
    expect(errored.some((i) => i.code === "uncovered-glyphs" && i.level === "error")).toBe(true);
  });

  it("covers letters via their contextual forms (abjad/cursive scripts)", () => {
    const arabic: ScriptData = {
      script: "arabic", name: "Arabic", font: "f", direction: "rtl", system: "abjad",
      letters: [{ glyph: "م", sound: "m", role: "consonant", forms: { initial: "مـ", medial: "ـمـ", final: "ـم", isolated: "م" }, components: [], strokeOrder: [], strokeOrderNote: "" }],
      marks: [{ mark: "َ", sound: "a", role: "diacritic", attachesAs: "above" }],
    };
    // Headword built from a form + a mark — both must count as covered.
    const l = good("arabic", "AR1", "GREETING-HELLO", { headword: "مَ", romanization: "ma" });
    const issues = validate({ taxonomy, lessons: [l], scripts: { arabic } });
    expect(issues.some((i) => i.code === "uncovered-glyphs")).toBe(false);
  });

  it("summarize counts levels", () => {
    const issues = validate({ taxonomy, lessons: [good("spanish", "ES1", "bad!")] });
    expect(summarize(issues)).toMatch(/error\(s\)/);
  });
});
