import { describe, it, expect } from "vitest";
import { parseLesson, buildDataset } from "../src/parse.js";
import type { Taxonomy } from "../src/types.js";

const lesson = (fields: Record<string, string>) =>
  ["---", ...Object.entries(fields).map(([k, v]) => `${k}: ${v}`), "---", "body"].join("\n");

const taxonomy: Taxonomy = {
  version: 1,
  concepts: {
    "GREETING-HELLO": { family: "GREETING", gloss: "hello", core: true },
    "TIME-DAY": { family: "TIME", gloss: "day", core: false },
  },
};

describe("parseLesson", () => {
  it("derives romanization from headword for Latin scripts", () => {
    const p = parseLesson(
      lesson({ id: "ES-C01-hola", chapter: "1", type: "word", headword: "hola", gloss: "hello", concept_tag: "GREETING-HELLO" }),
      "spanish",
    );
    expect(p.script).toBe("latin");
    expect(p.realization.romanization).toBe("hola");
    expect(p.realization.concept).toBe("GREETING-HELLO");
  });

  it("keeps romanization empty for a non-Latin lesson that omits it", () => {
    const p = parseLesson(
      lesson({ id: "TE-C01-hi", chapter: "1", type: "word", headword: "నమస్కారం", gloss: "hello", concept_tag: "GREETING-HELLO" }),
      "telugu",
    );
    expect(p.script).toBe("telugu");
    expect(p.realization.romanization).toBe("");
  });

  it("prefers an explicit romanization field", () => {
    const p = parseLesson(
      lesson({ id: "TE-C01-hi", chapter: "1", type: "word", headword: "నమస్కారం", gloss: "hello", concept_tag: "GREETING-HELLO", romanization: "namaskāram" }),
      "telugu",
    );
    expect(p.realization.romanization).toBe("namaskāram");
  });

  it("sniffs gender from the gloss when no field is present", () => {
    const masc = parseLesson(lesson({ id: "x", chapter: "1", type: "word", headword: "día", gloss: "day (el día — masculine)", concept_tag: "TIME-DAY" }), "spanish");
    expect(masc.realization.gender).toBe("masc");
    const fem = parseLesson(lesson({ id: "y", chapter: "1", type: "word", headword: "noche", gloss: "night (feminine)", concept_tag: "TIME-NIGHT" }), "spanish");
    expect(fem.realization.gender).toBe("fem");
    const none = parseLesson(lesson({ id: "z", chapter: "1", type: "word", headword: "hola", gloss: "hello", concept_tag: "GREETING-HELLO" }), "spanish");
    expect(none.realization.gender).toBeNull();
  });

  it("marks chapter NaN when missing", () => {
    const p = parseLesson(lesson({ id: "x", type: "word", headword: "h", gloss: "g", concept_tag: "GREETING-HELLO" }), "spanish");
    expect(Number.isNaN(p.realization.chapter)).toBe(true);
  });

  it("defaults unknown languages to the latin script", () => {
    const p = parseLesson(lesson({ id: "x", chapter: "1", type: "word", headword: "h", gloss: "g", concept_tag: "GREETING-HELLO" }), "esperanto");
    expect(p.script).toBe("latin");
  });
});

describe("buildDataset", () => {
  it("joins the same concept across languages and excludes practice lessons", () => {
    const lessons = [
      parseLesson(lesson({ id: "ES", chapter: "1", type: "word", headword: "hola", gloss: "hello", concept_tag: "GREETING-HELLO" }), "spanish"),
      parseLesson(lesson({ id: "DE", chapter: "1", type: "word", headword: "hallo", gloss: "hello", concept_tag: "GREETING-HELLO" }), "german"),
      parseLesson(lesson({ id: "ES-P", chapter: "1", type: "practice-mix", headword: "(practice)", gloss: "recap", concept_tag: "CH1-PRACTICE" }), "spanish"),
      parseLesson(lesson({ id: "ES-DIA", chapter: "1", type: "word", headword: "día", gloss: "day", concept_tag: "ES-WORD-DIA" }), "spanish"),
    ];
    const ds = buildDataset(taxonomy, lessons);

    const hello = ds.concepts.find((c) => c.id === "GREETING-HELLO");
    expect(hello?.realizations.map((r) => r.language).sort()).toEqual(["german", "spanish"]);
    expect(hello?.namespaced).toBe(false);
    expect(hello?.core).toBe(true);

    const dia = ds.concepts.find((c) => c.id === "ES-WORD-DIA");
    expect(dia?.namespaced).toBe(true);
    expect(dia?.family).toBe("(namespaced)");

    // The practice lesson contributes no concept.
    expect(ds.concepts.some((c) => c.id === "CH1-PRACTICE")).toBe(false);
    expect(ds.languages).toEqual(["german", "spanish"]);
  });
});
