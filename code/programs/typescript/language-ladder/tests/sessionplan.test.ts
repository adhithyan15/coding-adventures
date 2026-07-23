import { describe, it, expect } from "vitest";
import type { Lesson } from "../src/lessons";
import { loadLessons } from "../src/lessons";
import { planSession, initProgress, applyAnswer } from "../src/sessionplan";
import { cellKey, cellWeight, type GridCell } from "../src/quiz";

function L(language: string, concept: string, id: string, roots: string[] = []): Lesson {
  return {
    id, language, headword: `${language}:${concept}`, gloss: concept, type: "word",
    chapter: 1, concept, prerequisites: [], reviewsOf: [], roots,
    romanization: "x", script: language, etymologyHook: "",
  };
}

describe("planSession — assembling the two passes", () => {
  const lessons = [
    L("spanish", "THANKS", "es-t", ["gratia"]),
    L("french", "THANKS", "fr-t", ["gratia"]),
    L("spanish", "HELLO", "es-h"),
    L("french", "HELLO", "fr-h"),
  ];

  it("teaching pass is the current concept swept across the active chain, with connections", () => {
    const plan = planSession("THANKS", ["THANKS", "HELLO"], lessons, 10);
    expect(plan.concept).toBe("THANKS");
    expect(plan.teaching.map((s) => s.language)).toEqual(["spanish", "french"]);
    // french links back to spanish via the shared root
    expect(plan.teaching[1].connections).toEqual([{ to: "spanish", sharedRoots: ["gratia"] }]);
  });

  it("review pass spans EVERY covered concept, not just the current one", () => {
    const plan = planSession("THANKS", ["THANKS", "HELLO"], lessons, 10);
    const concepts = new Set(plan.reviewGrid.map((c) => c.concept));
    expect(concepts).toEqual(new Set(["THANKS", "HELLO"])); // CONTROL: a plan that only reviewed the current concept fails
  });

  it("an un-covered concept never enters the review grid", () => {
    const plan = planSession("THANKS", ["THANKS"], lessons, 10); // HELLO not covered
    expect(new Set(plan.reviewGrid.map((c) => c.concept))).toEqual(new Set(["THANKS"]));
  });
});

describe("applyAnswer — threading SRS state + the mistakes log", () => {
  const grid: GridCell[] = [{ concept: "C", language: "spanish", lesson: L("spanish", "C", "sc") }];
  const cell = grid[0];
  const key = cellKey(cell);

  it("a wrong answer demotes the cell and logs the confusion", () => {
    const p = applyAnswer(initProgress(), cell, false, 5, "fr:wrong");
    const st = p.states.get(key)!;
    expect(st.box).toBe(0);
    expect(st.dueAtSession).toBe(5); // due now → resurfaces
    expect(st.lapses).toBe(1);
    expect(p.log).toEqual([{ cellKey: key, correct: false, chosenKey: "fr:wrong" }]);
  });

  it("a correct answer promotes the cell so it comes back LATER, not now", () => {
    const p = applyAnswer(initProgress(), cell, true, 5);
    const st = p.states.get(key)!;
    expect(st.box).toBe(1);
    expect(st.dueAtSession).toBeGreaterThan(5); // scheduled out, not due now
    expect(p.log).toEqual([{ cellKey: key, correct: true }]); // no chosenKey on a hit
  });

  it("a missed-then-reviewed cell outweighs a mastered one in the draw", () => {
    // miss it (session 5) → demoted, due at 5. cellWeight at session 5 is high.
    const missed = applyAnswer(initProgress(), cell, false, 5);
    const wMissed = cellWeight(missed.states.get(key), 5);
    // a mastered cell (several correct answers) is scheduled far out → floor weight.
    let mastered = initProgress();
    for (let s = 0; s < 4; s++) mastered = applyAnswer(mastered, cell, true, s);
    const wMastered = cellWeight(mastered.states.get(key), 5);
    expect(wMissed).toBeGreaterThan(wMastered); // CONTROL: no promote/demote effect → equal → fails
  });

  it("does not mutate the input progress", () => {
    const p0 = initProgress();
    applyAnswer(p0, cell, false, 5, "x");
    expect(p0.states.size).toBe(0);
    expect(p0.log).toEqual([]);
  });
});

describe("planSession against the real curriculum", () => {
  it("teaches COURTESY-THANKS across the whole chain and reviews it alongside GREETING-HELLO", () => {
    const lessons = loadLessons();
    const plan = planSession("COURTESY-THANKS", ["COURTESY-THANKS", "GREETING-HELLO"], lessons, 10);
    expect(plan.teaching.length).toBe(10); // all ten chain languages teach thanks
    const reviewConcepts = new Set(plan.reviewGrid.map((c) => c.concept));
    expect(reviewConcepts).toEqual(new Set(["COURTESY-THANKS", "GREETING-HELLO"]));
  });
});
