import { describe, expect, it } from "vitest";
import {
  crossLanguageConcepts,
  datasetFromLessons,
  indexById,
  isUnlocked,
  reviewTargets,
  unlockedIndices,
  unlockedOrAll,
} from "../src/concepts.js";
import {
  indicesByLanguage,
  loadLessons,
  nextDue,
  type Lesson,
} from "../src/lessons.js";
import { buildPool } from "../src/interleave.js";
import { initStates, pickNext, reviewIn } from "../src/scheduler.js";
import taxonomy from "../../../../learning/human-languages/concepts/taxonomy.json";
import type { Taxonomy } from "@coding-adventures/human-language-data/src/types.ts";

const TAXONOMY = taxonomy as unknown as Taxonomy;

/** A Lesson with everything defaulted, so a test states only what it cares about. */
function lesson(over: Partial<Lesson> & { id: string }): Lesson {
  return {
    language: "spanish",
    headword: "",
    gloss: "",
    type: "word",
    chapter: 1,
    concept: "",
    prerequisites: [],
    reviewsOf: [],
    roots: [],
    romanization: "",
    script: "latin",
    etymologyHook: "",
    body: "",
    estMinutes: 5,
    ...over,
  };
}

describe("prerequisite gating", () => {
  const known = (ids: string[]) => new Set(ids);

  it("unlocks a lesson with no prerequisites", () => {
    expect(isUnlocked(lesson({ id: "A" }), new Set(), known(["A"]))).toBe(true);
  });

  it("locks a lesson whose prerequisite has not been seen", () => {
    const l = lesson({ id: "B", prerequisites: ["A"] });
    expect(isUnlocked(l, new Set(), known(["A", "B"]))).toBe(false);
  });

  it("unlocks once every prerequisite has been seen", () => {
    const l = lesson({ id: "C", prerequisites: ["A", "B"] });
    expect(isUnlocked(l, new Set(["A"]), known(["A", "B", "C"]))).toBe(false);
    expect(isUnlocked(l, new Set(["A", "B"]), known(["A", "B", "C"]))).toBe(true);
  });

  it("fails closed for an unknown prerequisite", () => {
    const l = lesson({ id: "D", prerequisites: ["ES-C99-typo"] });
    expect(isUnlocked(l, new Set(), known(["D"]))).toBe(false);
  });

  it("returns the indices of teachable lessons only", () => {
    const lessons = [
      lesson({ id: "A" }),
      lesson({ id: "B", prerequisites: ["A"] }),
      lesson({ id: "C", prerequisites: ["B"] }),
    ];
    expect(unlockedIndices(lessons, new Set())).toEqual([0]);
    expect(unlockedIndices(lessons, new Set(["A"]))).toEqual([0, 1]);
    expect(unlockedIndices(lessons, new Set(["A", "B"]))).toEqual([0, 1, 2]);
  });

  it("opens with chapter-1 lessons on a fresh profile", () => {
    const lessons = [
      lesson({ id: "ES-C01-hola" }),
      lesson({ id: "ES-C17-futuro", prerequisites: ["ES-C01-hola"] }),
    ];
    expect(unlockedIndices(lessons, new Set())).toEqual([0]);
  });

  it("fails closed when a cycle locks everything", () => {
    const lessons = [
      lesson({ id: "A", prerequisites: ["B"] }),
      lesson({ id: "B", prerequisites: ["A"] }),
    ];
    expect(unlockedIndices(lessons, new Set())).toEqual([]);
    expect(unlockedOrAll(lessons, new Set())).toEqual([]);
  });

  it("prefers the gated pool when one exists", () => {
    const lessons = [lesson({ id: "A" }), lesson({ id: "B", prerequisites: ["A"] })];
    expect(unlockedOrAll(lessons, new Set())).toEqual([0]);
  });
});

describe("reviews_of targets", () => {
  const lessons = [
    lesson({ id: "A" }),
    lesson({ id: "B" }),
    lesson({ id: "C", reviewsOf: ["A", "GONE", "B"] }),
  ];
  const byId = indexById(lessons);

  it("maps ids to indices", () => {
    expect(reviewTargets(lessons[2], byId)).toEqual([0, 1]);
  });

  // Asserting the exact array, not `.not.toContain(undefined)`: that weaker
  // form passes for [] and for a wholly broken implementation.
  it("drops dangling ids while preserving the order of the survivors", () => {
    expect(reviewTargets(lessons[2], byId)).toEqual([0, 1]);
    expect(reviewTargets(lesson({ id: "D", reviewsOf: ["B", "A"] }), byId)).toEqual([
      1, 0,
    ]);
  });

  it("is empty when a lesson reviews nothing", () => {
    expect(reviewTargets(lessons[0], byId)).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// The regression this module exists to prevent.
//
// v0.5.0 shipped a picker that scanned from the front, and because boxes 0/1
// fall due again after a single session it served item 0 forever. The rotating
// cursor fixed that. Adding prerequisite gating then RE-BROKE it in a new way:
// gating the *pick* (choose, reject, substitute a fallback) rejects the same
// index every turn and so serves the one fallback repeatedly.
//
// This test drives the real loop shape — scan, grade, advance — and asserts
// variety, so neither costume of the bug can come back unnoticed.
// ---------------------------------------------------------------------------
describe("gated rotation actually rotates", () => {
  it("serves many distinct lessons when most are locked behind chapter 1", () => {
    // The fixture has to reproduce the real failure, and the first attempt did
    // NOT: a chain whose pool order happens to match its unlock order lets the
    // broken picker look fine, because the rotation lands on an unlocked lesson
    // anyway. (Verified — the buggy implementation passed it.)
    //
    // What makes the bug bite is pool order being OUT OF SYNC with dependency
    // order, which is the real curriculum's situation: lessons rotate in id
    // order while prerequisites point wherever they like. So here the chain runs
    // BACKWARDS relative to the pool — the only initially-unlocked lesson in
    // each track sits last, and everything the rotation reaches first is locked.
    const lessons: Lesson[] = [];
    for (const lang of ["spanish", "french", "hindi"]) {
      // c1 needs c2 needs c3 … needs the root, which is pushed last.
      for (let c = 1; c <= 9; c += 1) {
        lessons.push(
          lesson({
            id: `${lang}-c${c}`,
            language: lang,
            prerequisites: [c === 9 ? `${lang}-root` : `${lang}-c${c + 1}`],
          }),
        );
      }
      lessons.push(lesson({ id: `${lang}-root`, language: lang }));
    }

    const groups = indicesByLanguage(lessons);
    const pool = buildPool(groups.map((g) => g.length));
    let schedule = initStates(lessons.length);
    let session = 0;
    let cursor = -1;
    const served: number[] = [];

    for (let turn = 0; turn < 30; turn += 1) {
      const seen = new Set(
        schedule.filter((s) => s.reps > 0 || s.lapses > 0 || s.box > 0)
          .map((s) => lessons[s.letterIndex]!.id),
      );
      const open = new Set(unlockedOrAll(lessons, seen));
      const { index, cursor: next } = nextDue(
        groups, pool, schedule, session, cursor, (i) => open.has(i),
      );
      cursor = next;
      const pick =
        index ??
        (() => {
          const states = schedule.filter((s) => open.has(s.letterIndex));
          return states.length > 0 ? pickNext(states, session) : null;
        })();
      if (pick === null) break;
      served.push(pick);
      schedule = reviewIn(schedule, pick, true, session);
      session += 1;
    }

    expect(served.length).toBe(30);

    // THE DISCRIMINATOR. Gating must not cost us the interleaving, so the very
    // first few picks have to cross tracks. Measured against the actual broken
    // implementation: it opens with ELEVEN consecutive Spanish lessons (it keeps
    // substituting the same fallback, which walks one track), where gating the
    // pool opens french / hindi / spanish / french / hindi / spanish.
    const firstSix = served.slice(0, 6).map((i) => lessons[i]!.language);
    expect(new Set(firstSix).size).toBeGreaterThan(1);

    // And no single lesson may dominate the session: 4 for the fixed version
    // against 11 for the broken one.
    const counts = new Map<number, number>();
    for (const i of served) counts.set(i, (counts.get(i) ?? 0) + 1);
    expect(Math.max(...counts.values())).toBeLessThan(served.length / 4);
  });

  it("unlocks the gated lessons once their prerequisite is studied", () => {
    const lessons = [
      lesson({ id: "A" }),
      lesson({ id: "B", prerequisites: ["A"] }),
    ];
    expect(unlockedOrAll(lessons, new Set())).toEqual([0]);
    expect(unlockedOrAll(lessons, new Set(["A"]))).toEqual([0, 1]);
  });
});

describe("cross-language concepts", () => {
  const lessons = [
    lesson({ id: "ES-1", language: "spanish", concept: "GREETING-HELLO", headword: "hola" }),
    lesson({ id: "FR-1", language: "french", concept: "GREETING-HELLO", headword: "bonjour" }),
    lesson({ id: "ES-2", language: "spanish", concept: "ES-ONLY", headword: "solo" }),
  ];

  it("keeps a concept realized by two or more languages", () => {
    const cards = crossLanguageConcepts(datasetFromLessons(TAXONOMY, lessons));
    expect(cards.map((c) => c.id)).toEqual(["GREETING-HELLO"]);
    expect(cards[0].realizations.map((r) => r.headword).sort()).toEqual([
      "bonjour",
      "hola",
    ]);
  });

  it("drops a concept only one language realizes — nothing to compare it with", () => {
    const cards = crossLanguageConcepts(datasetFromLessons(TAXONOMY, lessons));
    expect(cards.map((c) => c.id)).not.toContain("ES-ONLY");
  });

  it("honours a higher language floor", () => {
    const cards = crossLanguageConcepts(datasetFromLessons(TAXONOMY, lessons), 3);
    expect(cards).toEqual([]);
  });

  it("counts LANGUAGES, not lessons — two lessons in one track is not a card", () => {
    const sameTrack = [
      lesson({ id: "ES-1", language: "spanish", concept: "X" }),
      lesson({ id: "ES-2", language: "spanish", concept: "X" }),
    ];
    expect(crossLanguageConcepts(datasetFromLessons(TAXONOMY, sameTrack))).toEqual([]);
  });
});

describe("against the real curriculum", () => {
  const lessons = loadLessons();
  const dataset = datasetFromLessons(TAXONOMY, lessons);
  const cards = crossLanguageConcepts(dataset);

  it("finds real cross-language concepts", () => {
    expect(cards.length).toBeGreaterThan(10);
  });

  it("every card genuinely spans two or more languages", () => {
    for (const card of cards) {
      expect(new Set(card.realizations.map((r) => r.language)).size).toBeGreaterThan(1);
    }
  });

  it("GREETING-HELLO spans several tracks", () => {
    const hello = cards.find((c) => c.id === "GREETING-HELLO");
    expect(hello).toBeDefined();
    expect(new Set(hello!.realizations.map((r) => r.language)).size).toBeGreaterThan(2);
  });

  it("gating opens a non-empty pool on a fresh profile", () => {
    const open = unlockedIndices(lessons, new Set());
    expect(open.length).toBeGreaterThan(0);
    expect(open.length).toBeLessThan(lessons.length);
  });

  it("every lesson becomes reachable once everything is seen", () => {
    const all = new Set(lessons.map((l) => l.id));
    expect(unlockedIndices(lessons, all).length).toBe(lessons.length);
  });
});
