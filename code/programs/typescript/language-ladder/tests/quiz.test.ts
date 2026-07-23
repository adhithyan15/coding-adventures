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

import { cellWeight, cellDue, pickNext, makeRng, type QuizState } from "../src/quiz";

const distinct = <T,>(xs: T[]) => new Set(xs).size;

describe("cellWeight — the SRS bias", () => {
  const S = 10;
  it("weights never-seen above not-yet-due, and overdue/low-box/lapsed highest", () => {
    const unseen = cellWeight(undefined, S);
    const notDue = cellWeight({ box: 5, dueAtSession: 100, lapses: 0, reps: 3 }, S);
    const dueHighBox = cellWeight({ box: 5, dueAtSession: 10, lapses: 0, reps: 3 }, S);
    const dueLowBoxOverdue = cellWeight({ box: 0, dueAtSession: 4, lapses: 2, reps: 1 }, S);
    expect(unseen).toBeGreaterThan(notDue);
    expect(dueLowBoxOverdue).toBeGreaterThan(dueHighBox); // missed material outweighs due-but-known
    expect(dueHighBox).toBeGreaterThan(notDue);
    // more overdue → strictly heavier
    expect(cellWeight({ box: 1, dueAtSession: 2, lapses: 0, reps: 1 }, S))
      .toBeGreaterThan(cellWeight({ box: 1, dueAtSession: 9, lapses: 0, reps: 1 }, S));
  });

  it("cellDue is dueAtSession <= session", () => {
    expect(cellDue({ box: 0, dueAtSession: 10, lapses: 0, reps: 0 }, 10)).toBe(true);
    expect(cellDue({ box: 0, dueAtSession: 11, lapses: 0, reps: 0 }, 10)).toBe(false);
  });
});

describe("pickNext — the weighted draw", () => {
  it("returns null for an empty grid", () => {
    expect(pickNext([], new Map(), 0, makeRng(1))).toBeNull();
  });

  it("is deterministic for a given seed", () => {
    const grid = coveredGrid(["A", "B"], [L("spanish", "A", "a"), L("french", "B", "b")], 10);
    const draw = (seed: number) => {
      const rng = makeRng(seed);
      return Array.from({ length: 20 }, () => cellKey(pickNext(grid, new Map(), 0, rng)!));
    };
    expect(draw(42)).toEqual(draw(42)); // same seed, same sequence
  });

  it("CONTROL: over many draws the sample spans MULTIPLE concepts AND languages", () => {
    // 2 concepts × 2 languages, all unseen (equal weight) → the draw must not
    // collapse to one bucket. A pickNext that always returned grid[0] fails both.
    const grid = coveredGrid(
      ["A", "B"],
      [L("spanish", "A", "sa"), L("french", "A", "fa"), L("spanish", "B", "sb"), L("french", "B", "fb")],
      10,
    );
    const rng = makeRng(7);
    const drawn = Array.from({ length: 300 }, () => pickNext(grid, new Map(), 0, rng)!);
    expect(distinct(drawn.map((c) => c.concept))).toBeGreaterThan(1);
    expect(distinct(drawn.map((c) => c.language))).toBeGreaterThan(1);
  });

  it("CONTROL: the draw biases toward the missed/overdue cell over a mastered one", () => {
    const grid = coveredGrid(["A", "B"], [L("spanish", "A", "missed"), L("french", "B", "known")], 10);
    const states = new Map<string, QuizState>([
      [cellKey(grid.find((c) => c.lesson.id === "missed")!), { box: 0, dueAtSession: 4, lapses: 2, reps: 1 }],
      [cellKey(grid.find((c) => c.lesson.id === "known")!), { box: 5, dueAtSession: 100, lapses: 0, reps: 5 }],
    ]);
    const rng = makeRng(3);
    const drawn = Array.from({ length: 400 }, () => pickNext(grid, states, 10, rng)!);
    const missed = drawn.filter((c) => c.lesson.id === "missed").length;
    const known = drawn.filter((c) => c.lesson.id === "known").length;
    // missed weight ~16 vs known ~1 → missed should dominate by a wide margin.
    // (Under a UNIFORM draw these would be ~equal — that is the injected failure.)
    expect(missed).toBeGreaterThan(known * 3);
  });
});
