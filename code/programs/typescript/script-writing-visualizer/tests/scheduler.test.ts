import { describe, it, expect } from "vitest";
import {
  INTERVALS,
  MAX_BOX,
  intervalFor,
  initStates,
  isDue,
  dueCount,
  masteredCount,
  pickNext,
  review,
  reviewIn,
  type ItemState,
} from "../src/scheduler.ts";

function item(over: Partial<ItemState> = {}): ItemState {
  return { letterIndex: 0, box: 0, dueAtSession: 0, introducedAt: 0, lapses: 0, reps: 0, ...over };
}

describe("initStates", () => {
  it("makes every item due immediately in box 0", () => {
    const s = initStates(3);
    expect(s).toHaveLength(3);
    expect(s.map((x) => x.letterIndex)).toEqual([0, 1, 2]);
    expect(s.every((x) => x.box === 0 && x.dueAtSession === 0 && x.reps === 0)).toBe(true);
    expect(dueCount(s, 0)).toBe(3);
  });
  it("honours a starting session and clamps negatives", () => {
    expect(initStates(2, 5).every((x) => x.dueAtSession === 5 && x.introducedAt === 5)).toBe(true);
    expect(initStates(-4)).toEqual([]);
  });
});

describe("intervalFor", () => {
  it("maps boxes to intervals and clamps out-of-range", () => {
    expect(intervalFor(0)).toBe(INTERVALS[0]);
    expect(intervalFor(MAX_BOX)).toBe(INTERVALS[MAX_BOX]);
    expect(intervalFor(999)).toBe(INTERVALS[MAX_BOX]);
    expect(intervalFor(-3)).toBe(INTERVALS[0]);
  });
});

describe("review", () => {
  it("promotes a box and expands the interval on correct", () => {
    const a = review(item({ box: 0, reps: 0 }), true, 10);
    expect(a.box).toBe(1);
    expect(a.dueAtSession).toBe(10 + intervalFor(1));
    expect(a.reps).toBe(1);
    expect(a.lapses).toBe(0);
    // chained promotions keep expanding
    const b = review(a, true, a.dueAtSession);
    expect(b.box).toBe(2);
    expect(b.dueAtSession).toBe(a.dueAtSession + intervalFor(2));
  });

  it("caps the box at MAX_BOX", () => {
    let s = item({ box: MAX_BOX });
    s = review(s, true, 0);
    expect(s.box).toBe(MAX_BOX);
    expect(s.dueAtSession).toBe(0 + INTERVALS[MAX_BOX]!);
  });

  it("resets to box 0 and counts a lapse on wrong", () => {
    const s = review(item({ box: 4, reps: 9, lapses: 1 }), false, 20);
    expect(s.box).toBe(0);
    expect(s.dueAtSession).toBe(20 + intervalFor(0)); // due again very soon
    expect(s.lapses).toBe(2);
    expect(s.reps).toBe(10);
  });

  it("does not mutate the input", () => {
    const original = item({ box: 2 });
    const snapshot = { ...original };
    review(original, true, 3);
    expect(original).toEqual(snapshot);
  });
});

describe("pickNext", () => {
  it("returns -1 for an empty set", () => {
    expect(pickNext([], 0)).toBe(-1);
  });

  it("prefers the most-overdue due item", () => {
    const items = [
      item({ letterIndex: 0, dueAtSession: 5 }),
      item({ letterIndex: 1, dueAtSession: 2 }), // most overdue at session 10
      item({ letterIndex: 2, dueAtSession: 8 }),
    ];
    expect(pickNext(items, 10)).toBe(1);
  });

  it("breaks ties by fewest reps, then lowest index", () => {
    const items = [
      item({ letterIndex: 2, dueAtSession: 0, reps: 1 }),
      item({ letterIndex: 0, dueAtSession: 0, reps: 3 }),
      item({ letterIndex: 1, dueAtSession: 0, reps: 1 }),
    ];
    // reps: idx2=1, idx1=1 tie → lowest index → 1
    expect(pickNext(items, 0)).toBe(1);
  });

  it("falls back to the soonest-due item when nothing is due (never stalls)", () => {
    const items = [
      item({ letterIndex: 0, dueAtSession: 30 }),
      item({ letterIndex: 1, dueAtSession: 12 }),
      item({ letterIndex: 2, dueAtSession: 20 }),
    ];
    expect(pickNext(items, 5)).toBe(1); // none due; earliest future
  });
});

describe("reviewIn + integration", () => {
  it("updates only the answered item", () => {
    const items = initStates(3);
    const after = reviewIn(items, 1, true, 0);
    expect(after[0]).toEqual(items[0]); // untouched
    expect(after[2]).toEqual(items[2]); // untouched
    expect(after[1]!.box).toBe(1);
    expect(after[1]!.reps).toBe(1);
  });

  it("a correct streak masters an item and it stops being picked while easier ones remain", () => {
    let items = initStates(2);
    let session = 0;
    // Drill letter 0 correct several times; letter 1 never touched.
    for (let k = 0; k < 4; k++) {
      items = reviewIn(items, 0, true, session);
      session += 1;
    }
    expect(masteredCount(items)).toBeGreaterThanOrEqual(1);
    // letter 1 is still box 0, due since session 0 → far more overdue → picked
    expect(pickNext(items, session)).toBe(1);
  });

  it("a missed item comes back soon (box 0, due next session)", () => {
    let items = initStates(3);
    // master 1 and 2 out to the future, miss 0
    items = reviewIn(items, 1, true, 0);
    items = reviewIn(items, 2, true, 0);
    items = reviewIn(items, 0, false, 0); // wrong → due at session 0+interval(0)
    const dueSoon = items.find((x) => x.letterIndex === 0)!;
    expect(dueSoon.box).toBe(0);
    expect(dueSoon.dueAtSession).toBe(intervalFor(0));
    expect(pickNext(items, intervalFor(0))).toBe(0); // it's the one that resurfaces
  });
});
