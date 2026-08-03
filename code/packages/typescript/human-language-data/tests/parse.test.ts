import { describe, it, expect } from "vitest";
import { parseBodyBlocks, parseLesson, buildDataset } from "../src/parse.js";
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
    expect(p.body).toBe("body");
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

  it("accepts an explicit, open script id (any script — no code change needed)", () => {
    // A brand-new script the built-in map has never heard of, passed straight in.
    const p = parseLesson(
      lesson({ id: "HE1", chapter: "1", type: "word", headword: "שלום", gloss: "hello", concept_tag: "GREETING-HELLO", romanization: "shalom" }),
      "hebrew",
      "hebrew",
    );
    expect(p.script).toBe("hebrew");
    expect(p.realization.romanization).toBe("shalom");
  });

  it("preserves the preamble and parses stable typed body blocks", () => {
    const body = [
      "# hola — hello",
      "",
      "## Warm-up",
      "Remember yesterday.",
      "",
      "## The word, taken apart",
      "A root note.",
      "",
      "## Script — shape and stroke",
      "A writing note.",
      "",
      "## Wrap-up Recall",
      "Say it once.",
    ].join("\n");
    const parsed = parseBodyBlocks(body);
    expect(parsed.preamble).toBe("# hola — hello");
    expect(parsed.blocks).toEqual([
      { type: "warmup", title: "Warm-up", markdown: "Remember yesterday." },
      { type: "etymology", title: "The word, taken apart", markdown: "A root note." },
      { type: "script", title: "Script — shape and stroke", markdown: "A writing note." },
      { type: "recall", title: "Wrap-up Recall", markdown: "Say it once." },
    ]);
  });

  it("marks unregistered headings as unknown instead of discarding them", () => {
    expect(parseBodyBlocks("## A surprising section\nKeep me.").blocks).toEqual([
      { type: "unknown", title: "A surprising section", markdown: "Keep me." },
    ]);
  });

  it("recognizes scoped taken-apart headings as etymology blocks", () => {
    const parsed = parseBodyBlocks([
      "## The phrase, taken apart",
      "A phrase history.",
      "",
      "## The four seasons, taken apart",
      "Four word histories.",
    ].join("\n"));
    expect(parsed.blocks.map((block) => block.type)).toEqual(["etymology", "etymology"]);
  });

  it("parses block-boundary knowledge without rendering the directive", () => {
    const parsed = parseBodyBlocks([
      "## Guided Practice",
      "<!-- hl-knowledge: introduces=[]; assesses=[ES-LEX-HOLA, ES-SOUND-H-SILENT] -->",
      "",
      "Say *hola*.",
    ].join("\n"));
    expect(parsed.blocks).toEqual([{
      type: "guided-production",
      title: "Guided Practice",
      markdown: "Say *hola*.",
      knowledge: {
        introduces: [],
        assesses: ["ES-LEX-HOLA", "ES-SOUND-H-SILENT"],
      },
    }]);
  });

  it("preserves and flags a malformed or misplaced block knowledge directive", () => {
    const [block] = parseBodyBlocks([
      "## Guided Practice",
      "Say *hola*.",
      "<!-- hl-knowledge: assesses=[ES-LEX-HOLA] -->",
    ].join("\n")).blocks;
    expect(block?.knowledge).toBeUndefined();
    expect(block?.knowledgeDirectiveError).toMatch(/expected one first-line/);
    expect(block?.markdown).toContain("hl-knowledge");
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
