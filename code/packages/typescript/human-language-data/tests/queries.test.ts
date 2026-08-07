import { describe, it, expect } from "vitest";
import { parseLesson, buildDataset } from "../src/parse.js";
import {
  allConcepts,
  conceptsByLanguage,
  languagesForConcept,
  coverageByLanguage,
} from "../src/queries.js";
import type { Taxonomy } from "../src/types.js";

const lesson = (fields: Record<string, string>) =>
  ["---", ...Object.entries(fields).map(([k, v]) => `${k}: ${v}`), "---", "b"].join("\n");

const taxonomy: Taxonomy = {
  version: 1,
  concepts: {
    "GREETING-HELLO": { family: "GREETING", gloss: "hello", core: true },
    "COURTESY-THANKS": { family: "COURTESY", gloss: "thanks", core: true },
    "TIME-DAY": { family: "TIME", gloss: "day", core: false },
  },
};

const ds = buildDataset(taxonomy, [
  parseLesson(lesson({ id: "ES1", chapter: "1", type: "word", headword: "hola", gloss: "hello", concept_tag: "GREETING-HELLO" }), "spanish"),
  parseLesson(lesson({ id: "DE1", chapter: "1", type: "word", headword: "hallo", gloss: "hello", concept_tag: "GREETING-HELLO" }), "german"),
  parseLesson(lesson({ id: "ES2", chapter: "1", type: "word", headword: "gracias", gloss: "thanks", concept_tag: "COURTESY-THANKS" }), "spanish"),
  parseLesson(lesson({ id: "ES3", chapter: "1", type: "word", headword: "día", gloss: "day", concept_tag: "TIME-DAY" }), "spanish"),
]);

describe("queries", () => {
  it("allConcepts is id-sorted", () => {
    expect(allConcepts(ds).map((c) => c.id)).toEqual(["COURTESY-THANKS", "GREETING-HELLO", "TIME-DAY"]);
  });

  it("conceptsByLanguage filters to a track", () => {
    expect(conceptsByLanguage(ds, "german").map((c) => c.id)).toEqual(["GREETING-HELLO"]);
    expect(conceptsByLanguage(ds, "spanish").length).toBe(3);
  });

  it("languagesForConcept returns the cross-language join", () => {
    expect(languagesForConcept(ds, "GREETING-HELLO").map((r) => r.language).sort()).toEqual(["german", "spanish"]);
    expect(languagesForConcept(ds, "NOPE")).toEqual([]);
  });

  it("coverageByLanguage counts core vs total", () => {
    const cov = coverageByLanguage(ds);
    expect(cov.spanish).toEqual({ core: 2, total: 3 }); // hello+thanks core, day not
    expect(cov.german).toEqual({ core: 1, total: 1 });
  });
});
