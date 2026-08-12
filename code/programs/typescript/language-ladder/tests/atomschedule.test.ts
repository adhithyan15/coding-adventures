import { describe, expect, it } from "vitest";
import { type MasteryBook, newAtom, practiseAll } from "../src/atommastery.ts";
import {
  type SchedulableLesson,
  coverageOf,
  refreshesOf,
  reviewPicks,
} from "../src/atomschedule.ts";

const T0 = 1_700_000_000_000;
const DAY = 24 * 60 * 60 * 1000;

/** A book in which every named atom is overdue right now. */
function overdue(atoms: string[]): MasteryBook {
  const book: MasteryBook = new Map();
  for (const atom of atoms) {
    book.set(atom, { ...newAtom(atom, 1, T0 - 90 * DAY), dueAt: T0 - DAY, lastSeen: T0 - 90 * DAY });
  }
  return book;
}

function lesson(id: string, refreshes: string[], language = "spanish"): SchedulableLesson {
  return { id, language, refreshes };
}

describe("what a lesson would refresh", () => {
  it("takes the union of what its activities assess, deduplicated and sorted", () => {
    expect(
      refreshesOf({
        activities: [{ assesses: ["B", "A"] }, { assesses: ["A", "C"] }],
      }),
    ).toEqual(["A", "B", "C"]);
  });

  it("survives a lesson with no activities and drops junk entries", () => {
    expect(refreshesOf({})).toEqual([]);
    expect(refreshesOf({ activities: [] })).toEqual([]);
    expect(refreshesOf({ activities: [{ assesses: ["", "A"] as string[] }] })).toEqual(["A"]);
  });

  // The invariant this module exists to keep. A lesson with no authored
  // activity still credits its introduced atoms on a meaning check, so it must
  // also be schedulable for them — otherwise those atoms come due and nothing
  // can ever clear them. The first lesson of the Spanish course is exactly this
  // shape, which is how the bug was found.
  it("includes a lesson's introduced atoms, because a meaning check credits them", () => {
    expect(refreshesOf({ introducesAtoms: ["ES-LEX-HOLA", "ES-SOUND-H-SILENT"] })).toEqual([
      "ES-LEX-HOLA",
      "ES-SOUND-H-SILENT",
    ]);
    // Union, not either/or, and still deduplicated.
    expect(
      refreshesOf({
        activities: [{ assesses: ["A", "SHARED"] }],
        introducesAtoms: ["SHARED", "B"],
      }),
    ).toEqual(["A", "B", "SHARED"]);
  });

  it("can schedule a lesson that has no activities at all", () => {
    const book = overdue(["ES-LEX-HOLA"]);
    const meaningOnly = { id: "ES-C01-hola", language: "spanish", refreshes: refreshesOf({ introducesAtoms: ["ES-LEX-HOLA"] }) };
    expect(reviewPicks(book, [meaningOnly], new Set(["ES-C01-hola"]), T0)).toHaveLength(1);
  });
});

describe("choosing what to review", () => {
  it("returns nothing when nothing is due", () => {
    const book = practiseAll(new Map(), ["A"], true, T0);
    expect(reviewPicks(book, [lesson("l1", ["A"])], new Set(["l1"]), T0)).toEqual([]);
  });

  it("prefers the lesson that covers the most due atoms", () => {
    const book = overdue(["A", "B", "C"]);
    const picks = reviewPicks(
      book,
      [lesson("small", ["A"]), lesson("big", ["A", "B", "C"])],
      new Set(["small", "big"]),
      T0,
      1,
    );
    expect(picks.map((p) => p.lessonId)).toEqual(["big"]);
    expect(picks[0]!.covers).toEqual(["A", "B", "C"]);
  });

  it("does not pay twice for the same atom", () => {
    // Once `big` has covered A, B and C, a lesson offering only those is worth
    // nothing and must not appear at all.
    const book = overdue(["A", "B", "C", "D"]);
    const picks = reviewPicks(
      book,
      [lesson("big", ["A", "B", "C"]), lesson("dupe", ["A", "B"]), lesson("rest", ["D"])],
      new Set(["big", "dupe", "rest"]),
      T0,
      3,
    );
    expect(picks.map((p) => p.lessonId)).toEqual(["big", "rest"]);
  });

  it("never offers a lesson the learner has not completed", () => {
    const book = overdue(["A", "B"]);
    const picks = reviewPicks(
      book,
      [lesson("unseen", ["A", "B"]), lesson("done", ["A"])],
      new Set(["done"]),
      T0,
    );
    expect(picks.map((p) => p.lessonId)).toEqual(["done"]);
  });

  it("honours the limit, and a limit of zero means no queue", () => {
    const book = overdue(["A", "B", "C"]);
    const lessons = [lesson("l1", ["A"]), lesson("l2", ["B"]), lesson("l3", ["C"])];
    const all = new Set(["l1", "l2", "l3"]);
    expect(reviewPicks(book, lessons, all, T0, 2)).toHaveLength(2);
    expect(reviewPicks(book, lessons, all, T0, 0)).toEqual([]);
  });

  it("is deterministic — the same book and clock give the same queue", () => {
    const book = overdue(["A", "B"]);
    const lessons = [lesson("zebra", ["A"]), lesson("alpha", ["A"])];
    const all = new Set(["zebra", "alpha"]);
    const first = reviewPicks(book, lessons, all, T0, 1);
    const again = reviewPicks(book, lessons, all, T0, 1);
    expect(first).toEqual(again);
    // The tie broke on id, not on array order.
    expect(first[0]!.lessonId).toBe("alpha");
  });

  it("stops cleanly when no completed lesson can cover what is left", () => {
    const book = overdue(["A", "ORPHAN"]);
    const picks = reviewPicks(book, [lesson("l1", ["A"])], new Set(["l1"]), T0, 5);
    expect(picks.map((p) => p.lessonId)).toEqual(["l1"]);
  });

  it("reports how much of the debt a queue would clear", () => {
    const book = overdue(["A", "B", "C", "D"]);
    const picks = reviewPicks(book, [lesson("l1", ["A", "B"])], new Set(["l1"]), T0, 1);
    expect(coverageOf(book, picks, T0)).toBe(0.5);
    // Nothing due means the queue is trivially complete, not divided by zero.
    expect(coverageOf(new Map(), [], T0)).toBe(1);
  });
});
