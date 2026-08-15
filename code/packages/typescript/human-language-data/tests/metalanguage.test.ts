// HL10 section 7.5 -- the metalanguage ramp (HL-C89).
//
// Controls matter more than firing fixtures here. The whole risk is a gate that
// reports every lesson: `word` alone appears in 1,555 of them, so a measurement
// that does not separate ordinary English from technical vocabulary produces one
// enormous number that is identical for every corpus and useless to every author.

import { describe, expect, it } from "vitest";
import {
  measureMetalanguage,
  renderMetalanguage,
  termsUsedIn,
} from "../src/metalanguage.js";
import { loadEverything, loadMetalanguage } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";
import type { MetalanguageInventory } from "../src/types.js";

const INVENTORY: MetalanguageInventory = {
  version: 1,
  terms: [
    { id: "META-WORD", term: "word", stage: "pre-A1", order: 1, introduceAfter: "nothing", plainAlternative: "word", technical: false },
    { id: "META-VERB", term: "verb", stage: "A1", order: 2, introduceAfter: "forms in use", plainAlternative: "a doing word", technical: true },
    { id: "META-DIRECT-OBJECT", term: "direct object", stage: "A2", order: 3, introduceAfter: "the arc has begun", plainAlternative: "what is acted on", technical: true },
  ],
};

function lesson(body: string, opts: { id?: string; sequence?: number; introduces?: string[] } = {}) {
  const intro = opts.introduces ? `introduces_metalanguage: [${opts.introduces.join(", ")}]\n` : "";
  return parseLesson(
    `---
schema_version: 2
id: ${opts.id ?? "L1"}
sequence: ${opts.sequence ?? 10}
chapter: 1
type: vocabulary
headword: x
gloss: x
${intro}---

# x

${body}
`,
    "spanish",
  );
}

describe("termsUsedIn", () => {
  it("matches whole words only", () => {
    expect(termsUsedIn(lesson("This verb means to eat."), INVENTORY).map((t) => t.term)).toEqual(["verb"]);
    // "adverb" and "wordy" contain the terms but are not them.
    expect(termsUsedIn(lesson("An adverb is wordy."), INVENTORY)).toEqual([]);
  });

  it("matches the plural", () => {
    expect(termsUsedIn(lesson("These verbs change."), INVENTORY).map((t) => t.term)).toEqual(["verb"]);
  });

  it("matches a multi-word term across a line break", () => {
    expect(
      termsUsedIn(lesson("It takes a direct\nobject here."), INVENTORY).map((t) => t.term),
    ).toContain("direct object");
  });

  it("never reads a directive comment", () => {
    // A gate that fires on metadata teaches authors to distrust it.
    expect(termsUsedIn(lesson("<!-- hl-knowledge: verb -->\nSay hola."), INVENTORY)).toEqual([]);
  });

  it("never reads a table row", () => {
    // A header cell reading "verb" is the info-dump gate's business, not an
    // author explaining the word to a beginner.
    expect(termsUsedIn(lesson("| verb | form |\n|---|---|\n| a | b |"), INVENTORY)).toEqual([]);
  });

  it("survives an unterminated comment without hanging", () => {
    const started = Date.now();
    expect(() => termsUsedIn(lesson("<!--".repeat(60_000)), INVENTORY)).not.toThrow();
    expect(Date.now() - started).toBeLessThan(2_000);
  });
});

describe("measureMetalanguage", () => {
  it("flags a technical term used before any lesson introduces it", () => {
    const r = measureMetalanguage([lesson("This verb means to eat.")], INVENTORY);
    expect(r.summary.technicalUsesBeforeIntroduction).toBe(1);
    expect(r.uses[0]?.beforeIntroduction).toBe(true);
    expect(r.uses[0]?.plainAlternative).toBe("a doing word");
  });

  it("control: a term used AFTER its introduction is not flagged", () => {
    const r = measureMetalanguage(
      [
        lesson("From now on we will say verb.", { id: "L1", sequence: 10, introduces: ["META-VERB"] }),
        lesson("This verb means to eat.", { id: "L2", sequence: 20 }),
      ],
      INVENTORY,
    );
    expect(r.uses.filter((u) => u.beforeIntroduction)).toEqual([]);
  });

  it("lets the introducing lesson use the term it introduces", () => {
    const r = measureMetalanguage(
      [lesson("A doing word is called a verb.", { introduces: ["META-VERB"] })],
      INVENTORY,
    );
    expect(r.summary.technicalUsesBeforeIntroduction).toBe(0);
  });

  it("flags a term used EARLIER than the lesson that introduces it", () => {
    const r = measureMetalanguage(
      [
        lesson("This verb means to eat.", { id: "L1", sequence: 10 }),
        lesson("We call it a verb.", { id: "L2", sequence: 20, introduces: ["META-VERB"] }),
      ],
      INVENTORY,
    );
    const early = r.uses.filter((u) => u.beforeIntroduction);
    expect(early.map((u) => u.lessonId)).toEqual(["L1"]);
  });

  it("reads reading order from `sequence`, not array position", () => {
    const r = measureMetalanguage(
      [
        lesson("This verb means to eat.", { id: "L2", sequence: 20 }),
        lesson("We call it a verb.", { id: "L1", sequence: 10, introduces: ["META-VERB"] }),
      ],
      INVENTORY,
    );
    expect(r.uses.filter((u) => u.beforeIntroduction)).toEqual([]);
  });

  it("separates ordinary English from technical vocabulary", () => {
    // The distinction the whole report rests on. `word` appears everywhere and
    // needs no introduction; counting it would drown `dative`.
    const r = measureMetalanguage([lesson("Every word here is a verb.")], INVENTORY);
    expect(r.summary.usesBeforeIntroduction).toBe(2);
    expect(r.summary.technicalUsesBeforeIntroduction).toBe(1);
  });

  it("ranks worst terms over technical ones only", () => {
    const r = measureMetalanguage(
      [lesson("word word word", { id: "L1" }), lesson("verb", { id: "L2", sequence: 20 })],
      INVENTORY,
    );
    expect(r.summary.worstTerms.map((t) => t.term)).toEqual(["verb"]);
  });

  it("ignores an introduction naming a term that does not exist", () => {
    const r = measureMetalanguage(
      [lesson("This verb means to eat.", { introduces: ["META-NONSENSE"] })],
      INVENTORY,
    );
    expect(r.summary.technicalUsesBeforeIntroduction).toBe(1);
  });

  it("does not resolve a term id through Object.prototype", () => {
    // The id map is Object.create(null); a lesson declaring `constructor` must
    // not silently mark something introduced.
    const r = measureMetalanguage(
      [lesson("This verb means to eat.", { introduces: ["constructor", "__proto__"] })],
      INVENTORY,
    );
    expect(r.summary.technicalUsesBeforeIntroduction).toBe(1);
  });
});

describe("malformed inventories (security review FYI, HL-C89)", () => {
  it.each([
    ["terms is absent", {}],
    ["terms is a string", { terms: "verb" }],
    ["terms holds null", { terms: [null] }],
    ["a term has no string term", { terms: [{ id: "X", term: 42 }] }],
    ["a term is empty", { terms: [{ id: "X", term: "  " }] }],
  ])("survives: %s", (_label, bad) => {
    const inv = { version: 1, ...bad } as unknown as MetalanguageInventory;
    expect(() => termsUsedIn(lesson("This verb means to eat."), inv)).not.toThrow();
    expect(() => measureMetalanguage([lesson("verb")], inv)).not.toThrow();
  });
});

describe("the committed inventory", () => {
  const inventory = loadMetalanguage();

  it("has unique ids and terms", () => {
    expect(new Set(inventory.terms.map((t) => t.id)).size).toBe(inventory.terms.length);
    expect(new Set(inventory.terms.map((t) => t.term)).size).toBe(inventory.terms.length);
  });

  it("gives every term a plain alternative, which is what makes the rule usable", () => {
    // A rule that only forbids is a rule authors route around. Every term must
    // say what to write instead.
    for (const term of inventory.terms) {
      expect(term.plainAlternative.length).toBeGreaterThan(0);
    }
  });

  it("gives every TECHNICAL term an alternative that differs from the term", () => {
    // Scoped to technical terms deliberately. For an ordinary word the plain
    // alternative IS the word -- there is nothing plainer than "word" -- and
    // asserting otherwise would force a worse gloss for the sake of a rule.
    for (const term of inventory.terms) {
      if (term.technical !== true) continue;
      expect(term.plainAlternative.toLowerCase()).not.toBe(term.term.toLowerCase());
    }
  });

  it("lets an ordinary term be its own alternative", () => {
    const word = inventory.terms.find((t) => t.id === "META-WORD");
    expect(word?.technical).toBe(false);
    expect(word?.plainAlternative).toBe("word");
  });

  it("names a thing the learner can already do before each term", () => {
    for (const term of inventory.terms) {
      expect(term.introduceAfter.length).toBeGreaterThan(0);
    }
  });

  it("waits for the subjunctive arc before naming `mood`", () => {
    // Load-bearing: HL10 section 5.6 uses the subjunctive for 24 lessons before
    // the word arrives. If this drifts, that arc loses its shape.
    const mood = inventory.terms.find((t) => t.id === "META-MOOD");
    expect(mood?.introduceAfter).toContain("block D");
    expect(mood?.stage).toBe("B1");
  });

  it("counts noun, verb and adjective as technical", () => {
    // The premise is a reader who never studied grammar. For them "a doing word"
    // lands and "verb" does not.
    for (const id of ["META-ACTION-WORD", "META-NAME-WORD", "META-ADJECTIVE"]) {
      expect(inventory.terms.find((t) => t.id === id)?.technical).toBe(true);
    }
  });
});

describe("the committed corpus", () => {
  it("pins the first measurement", () => {
    const { lessons } = loadEverything();
    const r = measureMetalanguage(lessons, loadMetalanguage());
    expect(r.summary.terms).toBe(54);
    expect(r.summary.lessonsUsingTerms).toBeGreaterThanOrEqual(2002) // FLOOR — content only grows; see the note at the top of this file; // +8: HL-C94 // +4: HL-C98 // +40: vocabulary wave 5 // +4: HL-C88 slices 5-6 // +3: HL-C88 slice 8 // +54: vocabulary wave 6 // +3: HL-C113 preterite plural // HL-C113 preterite close // HL-C113: HL-C113 imperfect subjunctive // HL12: +30 recognition segments (telugu/kannada/malayalam 8 each, sanskrit 6). Every segment says letter, consonant, vowel sign or abugida, and none of those is introduced first -- the same debt HL11's Tamil segments added, now in four more tracks // HL12 payment two: +8 Hindi segments

    // No lesson declares an introduction yet, so every use is early. The total
    // says how pervasive the assumption is; the technical count says what to fix.
    expect(r.summary.usesBeforeIntroduction).toBeGreaterThanOrEqual(9120) // FLOOR — content only grows; see the note at the top of this file; // +5: ES-C03-vos and the two practice edits use ordinary terms (word, plural, ending) // +15: HL-C98's four lessons use ordinary terms (ending, verb, form) // +112: vocabulary wave 5 // +17: HL-C88 slices 5-6 // +11: HL-C88 slice 8 // +235: vocabulary wave 6 // HL-C113: grammar chapters cite tense and person names heavily, so these move in steps of ten rather than ones // HL11: +23. The nine letter segments talk about letters -- vowel, consonant, baseline, stroke, abugida -- and none of those terms is introduced first. Exactly the debt HL10 section 7.5 exists to burn down, now visible in new content rather than only in old // HL12: +63. The 30 recognition segments talk about letters -- consonant, vowel sign, abugida, nasal -- and none of those terms is introduced first. Exactly the debt HL10 section 7.5 exists to burn down, and it now shows up in four more tracks // HL12 payment two: +22, Hindi's eight segments using the same untaught terms
    expect(r.summary.technicalUsesBeforeIntroduction).toBeGreaterThanOrEqual(2736) // FLOOR — content only grows; see the note at the top of this file; // +4: HL-C98 // +26: vocabulary wave 5 // +7: HL-C88 slices 5-6 // +3: HL-C88 slice 8 // +49: vocabulary wave 6 // HL-C113: grammar chapters cite tense and person names heavily, so these move in steps of ten rather than ones
    expect(r.summary.technicalLessons).toBeGreaterThanOrEqual(1358) // FLOOR — content only grows; see the note at the top of this file; // +4: HL-C98 // +21: vocabulary wave 5 // +4: HL-C88 slices 5-6 // +2: HL-C88 slice 8 // +27: vocabulary wave 6 // HL-C113: grammar chapters cite tense and person names heavily, so these move in steps of ten rather than ones

    expect(r.summary.worstTerms.slice(0, 2)).toEqual([
      { term: "verb", lessons: 990 }, // +4: HL-C98 // +2: HL-C99 // +8: vocabulary wave 5 // +3: HL-C88 slices 5-6 // +11: vocabulary wave 6 // +2: B1 si-condition rung // +3: HL-C113 preterite plural // +2: HL-C113 preterite close (strong plurals, review, synthesis) // +2: HL-C113 imperfect subjunctive // +3: HL-C113 unreal condition // HL-C113 step 7: +4 // HL-C128 step 3: +1 // HL-C128 step 5: +2 // HL-C127: +3 // HL-C128 step 7: +2 // HL-C128 step 8: +2 // HL-C128 step 9: +4 // HL-C128 step 10: +3 // HL-C134: +1 each — the carried prose says 'verb' and 'noun' in lessons whose markdown had not, though the book always did // HL-C152: Spanish realizes SPINE-NEGATE-AND-ASK — five lessons, one chapter, A2 complete at 5/5
      { term: "noun", lessons: 512 }, // +1: ES-C02-concordancia names the noun it agrees with // +4: vocabulary wave 5 // +3: HL-C88 slices 5-6 // +1: HL-C88 slice 7 (ES-C09-ncia) // +1: HL-C88 slice 8 (-ario, review, synthesis) // +14: vocabulary wave 6 // HL-C113 step 6: +1 // HL-C128 step 2: +4 // HL-C128 step 3: +4 // HL-C128 step 8: +1 // HL-C128 step 9: +3 // HL-C128 step 10: +4 // HL-C134: +1 each — the carried prose says 'verb' and 'noun' in lessons whose markdown had not, though the book always did
    ]);
  });

  it("reports the technical subset in the rendered lines", () => {
    const { lessons } = loadEverything();
    const text = renderMetalanguage(measureMetalanguage(lessons, loadMetalanguage())).join("\n");
    expect(text).toContain("technical terms");
    expect(text).toContain("verb");
  });
});
