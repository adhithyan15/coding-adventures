import { describe, expect, it } from "vitest";
import {
  HALF_LIFE_MS,
  HELD_THRESHOLD,
  MAX_INTERVAL_MS,
  MIN_INTERVAL_MS,
  type MasteryBook,
  dueAtoms,
  heldAtoms,
  intervalFor,
  masterySummary,
  newAtom,
  practise,
  practiseAll,
  strengthNow,
} from "../src/atommastery.ts";
import {
  MASTERY_SCHEMA_VERSION,
  MASTERY_STORAGE_KEY,
  emptyMastery,
  fromSavedMastery,
  loadMastery,
  parseMastery,
  saveMastery,
  toSavedMastery,
} from "../src/masterystore.ts";

// A fixed clock. Every function in the engine takes `now`, precisely so a test
// can watch a month pass without waiting for one.
const T0 = 1_700_000_000_000;
const DAY = 24 * 60 * 60 * 1000;

describe("atom mastery, the engine", () => {
  it("starts a freshly taught atom held weakly rather than not at all", () => {
    const a = newAtom("ES-LEX-GRACIAS", 42, T0);
    expect(a.strength).toBeGreaterThan(0);
    expect(a.strength).toBeLessThan(HELD_THRESHOLD);
    expect(a.introducedAt).toBe(42);
    expect(a.lapses).toBe(0);
    // It is scheduled, not merely recorded.
    expect(a.dueAt).toBeGreaterThan(T0);
  });

  it("moves a hit asymptotically toward 1 and never past it", () => {
    let a = newAtom("ES-LEX-HOLA", 1, T0);
    const strengths = [a.strength];
    for (let i = 0; i < 20; i += 1) {
      a = practise(a, true, T0);
      strengths.push(a.strength);
    }
    // Monotonic, bounded, and it does get close.
    for (let i = 1; i < strengths.length; i += 1) {
      expect(strengths[i]!).toBeGreaterThan(strengths[i - 1]!);
      expect(strengths[i]!).toBeLessThanOrEqual(1);
    }
    expect(a.strength).toBeGreaterThan(0.99);
  });

  it("makes a miss cost more than a hit gains", () => {
    const base = newAtom("ES-LEX-ADIOS", 1, T0);
    const afterHit = practise(base, true, T0);
    const gained = afterHit.strength - base.strength;
    const afterMiss = practise(afterHit, false, T0);
    const lost = afterHit.strength - afterMiss.strength;
    expect(lost).toBeGreaterThan(gained);
    expect(afterMiss.lapses).toBe(1);
  });

  it("decays with a half-life, so a fortnight away costs something real", () => {
    let a = newAtom("ES-LEX-DIA", 1, T0);
    a = practise(a, true, T0);
    a = practise(a, true, T0);
    const fresh = strengthNow(a, T0);
    // Exactly one half-life later, exactly half.
    expect(strengthNow(a, T0 + HALF_LIFE_MS)).toBeCloseTo(fresh / 2, 5);
    // And it never goes negative, however long you leave it.
    expect(strengthNow(a, T0 + 100 * HALF_LIFE_MS)).toBeGreaterThanOrEqual(0);
  });

  it("schedules a stronger atom further out, within the stated bounds", () => {
    expect(intervalFor(0)).toBe(MIN_INTERVAL_MS);
    expect(intervalFor(1)).toBe(MAX_INTERVAL_MS);
    expect(intervalFor(0.8)).toBeGreaterThan(intervalFor(0.5));
    // Junk in does not produce a junk schedule.
    expect(intervalFor(Number.NaN)).toBe(MIN_INTERVAL_MS);
    expect(intervalFor(-5)).toBe(MIN_INTERVAL_MS);
    expect(intervalFor(99)).toBe(MAX_INTERVAL_MS);
  });

  it("credits every atom an activity assessed, creating the unseen ones", () => {
    const book = practiseAll(new Map(), ["ES-LEX-UNO", "ES-LEX-DOS"], true, T0, 7);
    expect([...book.keys()].sort()).toEqual(["ES-LEX-DOS", "ES-LEX-UNO"]);
    expect(book.get("ES-LEX-UNO")!.introducedAt).toBe(7);
    // Empty and non-string entries are ignored rather than stored.
    const dirty = practiseAll(book, ["", "ES-LEX-TRES"], true, T0);
    expect(dirty.has("")).toBe(false);
    expect(dirty.has("ES-LEX-TRES")).toBe(true);
  });

  it("does not mutate the book it was given", () => {
    const before: MasteryBook = new Map([["ES-LEX-A", newAtom("ES-LEX-A", 1, T0)]]);
    const snapshot = before.get("ES-LEX-A")!.strength;
    practiseAll(before, ["ES-LEX-A", "ES-LEX-B"], true, T0);
    expect(before.size).toBe(1);
    expect(before.get("ES-LEX-A")!.strength).toBe(snapshot);
  });

  it("ranks due atoms by how overdue they are RELATIVE to their own interval", () => {
    // A short-interval atom a week late has decayed far more than a
    // long-interval atom a week late. Ranking by raw lateness would bury it.
    const weak = { ...newAtom("ES-WEAK", 1, T0), dueAt: T0 + DAY, lastSeen: T0 };
    const strong = {
      ...newAtom("ES-STRONG", 1, T0),
      strength: 0.95,
      dueAt: T0 + 100 * DAY,
      lastSeen: T0,
    };
    const book: MasteryBook = new Map([
      ["ES-WEAK", weak],
      ["ES-STRONG", strong],
    ]);
    // Both are exactly seven days past due.
    const now = T0 + 107 * DAY;
    const due = dueAtoms(book, now);
    expect(due.map((d) => d.atom)).toEqual(["ES-WEAK", "ES-STRONG"]);
  });

  it("returns nothing due before anything is due, and honours a limit", () => {
    const book = practiseAll(new Map(), ["A", "B", "C"], true, T0);
    expect(dueAtoms(book, T0)).toEqual([]);
    expect(dueAtoms(book, T0 + 10 * 365 * DAY, 2)).toHaveLength(2);
    expect(dueAtoms(book, T0 + 10 * 365 * DAY, 0)).toEqual([]);
  });

  it("counts an atom as held only once it is actually held", () => {
    let book = practiseAll(new Map(), ["ES-LEX-X"], true, T0);
    expect(heldAtoms(book, T0)).toEqual([]);
    for (let i = 0; i < 5; i += 1) book = practiseAll(book, ["ES-LEX-X"], true, T0);
    expect(heldAtoms(book, T0)).toEqual(["ES-LEX-X"]);
    // ...and stops counting it after enough time away.
    expect(heldAtoms(book, T0 + 5 * HALF_LIFE_MS)).toEqual([]);
  });

  it("summarises the book the way the app would show it", () => {
    let book = practiseAll(new Map(), ["A", "B"], true, T0);
    for (let i = 0; i < 6; i += 1) book = practiseAll(book, ["A"], true, T0);
    for (let i = 0; i < 3; i += 1) book = practiseAll(book, ["B"], false, T0);
    const summary = masterySummary(book, T0);
    expect(summary.tracked).toBe(2);
    expect(summary.held).toBe(1);
    expect(summary.stubborn).toBe(1);
    expect(summary.meanStrength).toBeGreaterThan(0);
    expect(masterySummary(new Map(), T0)).toEqual({
      tracked: 0,
      held: 0,
      due: 0,
      stubborn: 0,
      meanStrength: 0,
    });
  });
});

describe("atom mastery, the store", () => {
  it("round-trips a book unchanged", () => {
    const book = practiseAll(new Map(), ["ES-LEX-B", "ES-LEX-A"], true, T0, 3);
    const back = fromSavedMastery(toSavedMastery(book));
    expect([...back.keys()].sort()).toEqual([...book.keys()].sort());
    expect(back.get("ES-LEX-A")).toEqual(book.get("ES-LEX-A"));
  });

  it("writes a stable, sorted payload so saves are diffable", () => {
    const book = practiseAll(new Map(), ["ES-Z", "ES-A", "ES-M"], true, T0);
    expect(toSavedMastery(book).atoms.map((a) => a.atom)).toEqual(["ES-A", "ES-M", "ES-Z"]);
  });

  it("drops a payload from another schema version rather than guessing", () => {
    const raw = JSON.stringify({ version: MASTERY_SCHEMA_VERSION + 1, atoms: [{ atom: "X" }] });
    expect(parseMastery(raw)).toEqual(emptyMastery());
  });

  it("survives every shape of broken payload", () => {
    for (const raw of ["", "null", "[]", "{", '{"version":1}', '"a string"', "42"]) {
      const parsed = parseMastery(raw);
      expect(parsed.version).toBe(MASTERY_SCHEMA_VERSION);
      expect(fromSavedMastery(parsed).size).toBe(0);
    }
    expect(parseMastery(null)).toEqual(emptyMastery());
  });

  it("drops individual bad rows without losing the good ones", () => {
    const saved = {
      version: MASTERY_SCHEMA_VERSION,
      atoms: [
        null,
        "not an object",
        { atom: "" },
        { atom: "ES-GOOD", strength: 5, lastSeen: -1, lapses: -3, dueAt: 1.7 },
      ] as unknown as never[],
    };
    const book = fromSavedMastery(saved);
    expect([...book.keys()]).toEqual(["ES-GOOD"]);
    const good = book.get("ES-GOOD")!;
    // Every out-of-range field was clamped, not trusted.
    expect(good.strength).toBe(1);
    expect(good.lastSeen).toBe(0);
    expect(good.lapses).toBe(0);
    expect(good.dueAt).toBe(1);
  });

  it("reads and writes through a storage port, and shrugs off a broken one", () => {
    const backing = new Map<string, string>();
    const storage = {
      getItem: (k: string) => backing.get(k) ?? null,
      setItem: (k: string, v: string) => void backing.set(k, v),
      removeItem: (k: string) => void backing.delete(k),
    };
    const book = practiseAll(new Map(), ["ES-LEX-Q"], true, T0);
    saveMastery(storage, book);
    expect(backing.has(MASTERY_STORAGE_KEY)).toBe(true);
    expect([...loadMastery(storage).keys()]).toEqual(["ES-LEX-Q"]);

    // No storage at all (server render, blocked cookies) is not an error.
    expect(loadMastery(null).size).toBe(0);
    expect(() => saveMastery(null, book)).not.toThrow();

    // A quota-exceeded or private-mode storage must not take the app down.
    const hostile = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("quota");
      },
      removeItem: () => {},
    };
    expect(loadMastery(hostile).size).toBe(0);
    expect(() => saveMastery(hostile, book)).not.toThrow();
  });
});
