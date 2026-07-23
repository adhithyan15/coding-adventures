import { describe, it, expect } from "vitest";
import type { Lesson } from "../src/lessons";
import { loadLessons } from "../src/lessons";
import { coveredGrid, conceptsIn, languagesIn, cellKey } from "../src/quiz";

function L(language: string, concept: string, id: string, chapter = 1): Lesson {
  return {
    id,
    language,
    headword: `${language}-${concept}`,
    gloss: concept,
    type: "word",
    chapter,
    concept,
    prerequisites: [],
    reviewsOf: [],
    roots: [],
    romanization: "x",
    script: language,
    etymologyHook: "",
  };
}

describe("coveredGrid", () => {
  it("emits one cell per lesson at each (covered concept × active language) stop", () => {
    const lessons = [
      L("spanish", "GREET", "s1"),
      L("french", "GREET", "f1"),
      L("spanish", "NUM5", "s5"),
      L("german", "GREET", "g1"),
    ];
    // covered = {GREET}, active = spanish/latin/french (count 3) → german excluded, NUM5 not covered
    const grid = coveredGrid(["GREET"], lessons, 3);
    expect(grid.map((c) => [c.concept, c.language])).toEqual([
      ["GREET", "spanish"],
      ["GREET", "french"],
    ]);
  });

  it("includes only COVERED concepts and only ACTIVE languages", () => {
    const lessons = [L("spanish", "A", "a"), L("french", "A", "fa"), L("spanish", "B", "b")];
    // covered only {A}; active only spanish (count 1)
    const grid = coveredGrid(["A"], lessons, 1);
    expect(languagesIn(grid)).toEqual(["spanish"]);
    expect(conceptsIn(grid)).toEqual(["A"]); // B excluded — not covered
  });

  it("every cell's lesson genuinely has that concept and language — the grounding rule", () => {
    const lessons = [L("spanish", "A", "a"), L("hindi", "A", "ha")];
    const grid = coveredGrid(["A"], lessons, 10);
    for (const cell of grid) {
      expect(cell.lesson.concept).toBe(cell.concept); // CONTROL: no cell can mislabel its lesson
      expect(cell.lesson.language).toBe(cell.language);
    }
  });

  it("is deterministic: concepts sorted, then chain order, then lesson order", () => {
    const lessons = [
      L("french", "ZED", "fz"),
      L("spanish", "ALPHA", "sa2", 5),
      L("spanish", "ALPHA", "sa1", 2),
      L("spanish", "ZED", "sz"),
    ];
    const grid = coveredGrid(["ZED", "ALPHA"], lessons, 10);
    // ALPHA before ZED (sorted); within ALPHA spanish chapters 2 then 5; ZED spanish then french (chain)
    expect(grid.map((c) => c.lesson.id)).toEqual(["sa1", "sa2", "sz", "fz"]);
  });

  it("empty covered set → empty grid", () => {
    expect(coveredGrid([], [L("spanish", "A", "a")], 10)).toEqual([]);
  });

  it("cellKey is stable and distinguishes cells", () => {
    const grid = coveredGrid(["A"], [L("spanish", "A", "a"), L("hindi", "A", "ha")], 10);
    const keys = grid.map(cellKey);
    expect(new Set(keys).size).toBe(keys.length); // all distinct
  });
});

describe("coveredGrid against the real curriculum", () => {
  const lessons = loadLessons();

  it("COURTESY-THANKS covered over the full chain spans many languages, one concept", () => {
    const grid = coveredGrid(["COURTESY-THANKS"], lessons, 10);
    expect(conceptsIn(grid)).toEqual(["COURTESY-THANKS"]);
    // COURTESY-THANKS is taught in all ten chain languages.
    expect(languagesIn(grid).length).toBe(10);
  });

  it("two covered concepts interleave: the grid spans BOTH concepts and MANY languages", () => {
    const grid = coveredGrid(["GREETING-HELLO", "COURTESY-THANKS"], lessons, 10);
    // CONTROL: a broken grid that collapsed to one concept or one language fails here.
    expect(conceptsIn(grid)).toEqual(["COURTESY-THANKS", "GREETING-HELLO"]);
    expect(languagesIn(grid).length).toBeGreaterThan(1);
    // and every cell is really one of the two covered concepts (no leakage)
    for (const cell of grid) expect(["GREETING-HELLO", "COURTESY-THANKS"]).toContain(cell.concept);
  });

  it("a shorter active prefix genuinely shrinks the covered languages", () => {
    const four = coveredGrid(["COURTESY-THANKS"], lessons, 4);
    const ten = coveredGrid(["COURTESY-THANKS"], lessons, 10);
    expect(languagesIn(four).length).toBeLessThan(languagesIn(ten).length);
    expect(languagesIn(four)).toEqual(["spanish", "latin", "french", "german"]);
  });
});
