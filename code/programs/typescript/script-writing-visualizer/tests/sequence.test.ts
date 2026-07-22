import { describe, it, expect } from "vitest";
import type { Lesson } from "../src/lessons";
import { loadLessons } from "../src/lessons";
import {
  LANGUAGE_CHAIN,
  chainIndex,
  isChainLanguage,
  activeChain,
  teachingSweep,
  sweepableConcepts,
  type ChainLanguage,
} from "../src/sequence";

// A minimal Lesson factory — only the fields the sequencing layer reads matter;
// the rest are filled with harmless defaults.
function L(language: string, concept: string, chapter: number, id?: string): Lesson {
  return {
    id: id ?? `${language}-${concept}-${chapter}`,
    language,
    headword: "x",
    gloss: "x",
    type: "word",
    chapter,
    concept,
    prerequisites: [],
    reviewsOf: [],
    romanization: "x",
    script: language,
    etymologyHook: "",
  };
}

const langsOf = (stops: { language: ChainLanguage }[]) => stops.map((s) => s.language);

describe("the language chain", () => {
  it("is ten distinct languages in the fixed order", () => {
    expect(LANGUAGE_CHAIN).toEqual([
      "spanish", "latin", "french", "german", "arabic",
      "hindi", "tamil", "kannada", "telugu", "malayalam",
    ]);
    expect(new Set(LANGUAGE_CHAIN).size).toBe(10);
  });

  it("chainIndex and isChainLanguage locate a language, or reject a non-chain one", () => {
    expect(chainIndex("spanish")).toBe(0);
    expect(chainIndex("malayalam")).toBe(9);
    expect(chainIndex("klingon")).toBe(-1);
    expect(isChainLanguage("hindi")).toBe(true);
    expect(isChainLanguage("klingon")).toBe(false);
  });

  it("activeChain takes the first N and clamps out-of-range counts", () => {
    expect(activeChain(0)).toEqual([]);
    expect(activeChain(3)).toEqual(["spanish", "latin", "french"]);
    expect(activeChain(99)).toEqual([...LANGUAGE_CHAIN]);
    expect(activeChain(-5)).toEqual([]);
  });
});

describe("teachingSweep", () => {
  // A concept taught (out of chain order in the input) by french, spanish, german.
  const scrambled: Lesson[] = [
    L("french", "GREETING", 1),
    L("german", "GREETING", 1),
    L("spanish", "GREETING", 1),
  ];

  it("walks the concept in CHAIN order, not input order", () => {
    const sweep = teachingSweep("GREETING", scrambled, activeChain(10));
    expect(langsOf(sweep)).toEqual(["spanish", "french", "german"]);
    // CONTROL: input order was french, german, spanish — prove we reordered.
    expect(langsOf(sweep)).not.toEqual(["french", "german", "spanish"]);
  });

  it("includes only ACTIVE languages — adding/removing a language changes the sweep", () => {
    const full = teachingSweep("GREETING", scrambled, activeChain(10));
    const narrow = teachingSweep("GREETING", scrambled, activeChain(3)); // spanish, latin, french
    expect(langsOf(full)).toEqual(["spanish", "french", "german"]);
    expect(langsOf(narrow)).toEqual(["spanish", "french"]); // german not yet active; latin doesn't teach it
    expect(narrow.length).toBeLessThan(full.length); // CONTROL: dropping german shortened it
  });

  it("skips active languages that do not teach the concept", () => {
    // Concept taught only by spanish and arabic; active includes all ten.
    const partial = [L("spanish", "NUM-5", 4), L("arabic", "NUM-5", 4)];
    const sweep = teachingSweep("NUM-5", partial, activeChain(10));
    expect(langsOf(sweep)).toEqual(["spanish", "arabic"]);
  });

  it("orders lessons within a language by chapter then id", () => {
    const many = [
      L("spanish", "C", 5, "s-b"),
      L("spanish", "C", 2, "s-a"),
      L("spanish", "C", 5, "s-a2"),
    ];
    const sweep = teachingSweep("C", many, activeChain(1));
    expect(sweep[0].lessons.map((l) => l.chapter)).toEqual([2, 5, 5]);
    expect(sweep[0].lessons.map((l) => l.id)).toEqual(["s-a", "s-a2", "s-b"]);
  });

  it("returns nothing for the empty concept (writing lessons carry none)", () => {
    expect(teachingSweep("", [L("spanish", "", 1)], activeChain(10))).toEqual([]);
  });
});

describe("teachingSweep against the real curriculum", () => {
  const lessons = loadLessons();

  it("GREETING-HELLO sweeps all ten languages in exact chain order when all are active", () => {
    const sweep = teachingSweep("GREETING-HELLO", lessons, activeChain(10));
    expect(langsOf(sweep)).toEqual([...LANGUAGE_CHAIN]); // all ten, chain-ordered
    for (const stop of sweep) expect(stop.lessons.length).toBeGreaterThan(0);
  });

  it("a shorter active prefix yields a shorter sweep, still chain-ordered", () => {
    const four = teachingSweep("GREETING-HELLO", lessons, activeChain(4));
    expect(langsOf(four)).toEqual(["spanish", "latin", "french", "german"]);
    // CONTROL: chain order is a property of the SWEEP, not of the (id-sorted)
    // input — reversing the language filter must not reorder the output.
    const scrambledInput = [...lessons].reverse();
    const same = teachingSweep("GREETING-HELLO", scrambledInput, activeChain(4));
    expect(langsOf(same)).toEqual(["spanish", "latin", "french", "german"]);
  });
});

describe("sweepableConcepts", () => {
  it("lists concepts in book order — earliest chapter first", () => {
    const fixtures = [
      L("spanish", "LATER", 4),
      L("spanish", "EARLY", 1),
      L("french", "MIDDLE", 2),
      L("spanish", "", 1), // writing lesson — contributes nothing
    ];
    expect(sweepableConcepts(fixtures, activeChain(10))).toEqual(["EARLY", "MIDDLE", "LATER"]);
    // CONTROL: this is NOT alphabetical (which would be EARLY, LATER, MIDDLE)
    // and NOT insertion order (LATER, EARLY, MIDDLE).
    expect(sweepableConcepts(fixtures, activeChain(10))).not.toEqual(["EARLY", "LATER", "MIDDLE"]);
  });

  it("only counts concepts from ACTIVE languages", () => {
    const fixtures = [L("spanish", "A", 1), L("german", "B", 1)];
    expect(sweepableConcepts(fixtures, activeChain(1))).toEqual(["A"]); // german inactive
  });

  it("the real curriculum offers the whole-chain greetings, ordered early", () => {
    const lessons = loadLessons();
    const concepts = sweepableConcepts(lessons, activeChain(10));
    expect(concepts).toContain("GREETING-HELLO");
    expect(concepts).toContain("COURTESY-THANKS");
    // a foundational greeting appears before a late-chapter concept
    const hello = concepts.indexOf("GREETING-HELLO");
    expect(hello).toBeGreaterThanOrEqual(0);
    expect(hello).toBeLessThan(concepts.length - 1);
  });
});
