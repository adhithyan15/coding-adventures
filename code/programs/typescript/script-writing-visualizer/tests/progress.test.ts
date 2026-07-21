import { describe, expect, it } from "vitest";
import {
  emptyProgress,
  fromSaved,
  loadProgress,
  parseProgress,
  saveProgress,
  seenCount,
  STORAGE_KEY,
  toSaved,
  type StorageLike,
} from "../src/progress.ts";
import { initStates, MAX_BOX, type ItemState } from "../src/scheduler.ts";

/** A full ItemState with the fields a test cares about overridden. */
function state(letterIndex: number, over: Partial<ItemState> = {}): ItemState {
  return {
    letterIndex,
    box: 0,
    dueAtSession: 0,
    introducedAt: 0,
    lapses: 0,
    reps: 0,
    ...over,
  };
}

/** An in-memory stand-in for localStorage. */
function fakeStorage(initial: Record<string, string> = {}): StorageLike & {
  data: Record<string, string>;
} {
  const data = { ...initial };
  return {
    data,
    getItem: (k) => (k in data ? data[k]! : null),
    setItem: (k, v) => {
      data[k] = v;
    },
  };
}

describe("toSaved / fromSaved", () => {
  it("round-trips a studied item by id, not by position", () => {
    const ids = ["ES-C01-hola", "FR-C01-salut"];
    const states = initStates(2, 0);
    states[1] = state(1, { box: 3, dueAtSession: 12, reps: 4, lapses: 1 });

    const saved = toSaved(ids, states, 5);
    expect(saved.items["FR-C01-salut"]).toMatchObject({ box: 3, dueAtSession: 12, reps: 4 });

    // Insert a new lesson at the FRONT — indices all shift.
    const grown = ["AR-C01-salam", "ES-C01-hola", "FR-C01-salut"];
    const restored = fromSaved(grown, saved);

    // The French progress followed its id, not its old slot.
    expect(restored[2]).toMatchObject({ box: 3, dueAtSession: 12, reps: 4, lapses: 1 });
    expect(restored[0]!.box).toBe(0); // the newcomer is unseen

    // letterIndex is REBUILT positionally, never restored from the save —
    // otherwise every saved item would point at a stale slot.
    expect(restored[2]!.letterIndex).toBe(2);
  });

  it("does not persist untouched items", () => {
    const ids = ["A", "B"];
    const saved = toSaved(ids, initStates(2, 0), 0);
    expect(Object.keys(saved.items)).toHaveLength(0);
  });

  it("still skips untouched items after a reload at a later session", () => {
    // The regression this guards: fresh items are seeded with the CURRENT
    // session, so from session 1 onward every unseen lesson has a non-zero
    // dueAtSession. A guard that consulted dueAtSession would fail open here
    // and persist the entire curriculum instead of the one studied item.
    const ids = ["A", "B", "C"];

    const first = toSaved(ids, [
      state(0, { box: 1, dueAtSession: 1, reps: 1 }),
      state(1),
      state(2),
    ], 1);
    expect(Object.keys(first.items)).toEqual(["A"]);

    // Reload: rebuild positional state from the save, then save again.
    const reloaded = fromSaved(ids, first);
    const second = toSaved(ids, reloaded, first.session);

    expect(Object.keys(second.items)).toEqual(["A"]);
    expect(seenCount(ids, second)).toBe(1);
  });

  it("keeps the payload proportional to what was studied, not to the curriculum", () => {
    const ids = Array.from({ length: 500 }, (_, i) => `L${i}`);
    const states = initStates(ids.length, 9);
    states[3] = state(3, { box: 2, dueAtSession: 12, reps: 3 });

    const saved = toSaved(ids, states, 9);
    expect(Object.keys(saved.items)).toHaveLength(1);
    // And it stays that way across a reload.
    const again = toSaved(ids, fromSaved(ids, saved), saved.session);
    expect(Object.keys(again.items)).toHaveLength(1);
  });

  it("treats ids missing from the save as fresh", () => {
    const saved = toSaved(["A"], [state(0, { box: 2, dueAtSession: 4, reps: 1 })], 1);
    const states = fromSaved(["A", "B"], saved);
    expect(states[0]).toMatchObject({ box: 2, dueAtSession: 4 });
    expect(states[1]!.box).toBe(0);
    expect(states[1]!.reps).toBe(0);
  });

  it("clamps a box that is out of range or nonsense", () => {
    const saved = emptyProgress();
    saved.items["A"] = { box: 999, dueAtSession: 1, introducedAt: 0, lapses: 0, reps: 1 };
    saved.items["B"] = { box: -5, dueAtSession: 1, introducedAt: 0, lapses: 0, reps: 1 };
    const states = fromSaved(["A", "B"], saved);
    expect(states[0]!.box).toBe(MAX_BOX);
    expect(states[1]!.box).toBe(0);
  });
});

describe("parseProgress", () => {
  it("returns an empty record for null, empty, or non-JSON input", () => {
    expect(parseProgress(null).items).toEqual({});
    expect(parseProgress("").items).toEqual({});
    expect(parseProgress("{not json").items).toEqual({});
  });

  it("rejects a payload that isn't an object", () => {
    expect(parseProgress("[1,2,3]").items).toEqual({});
    expect(parseProgress('"a string"').items).toEqual({});
    expect(parseProgress("null").items).toEqual({});
  });

  it("drops a payload from a different schema version", () => {
    const raw = JSON.stringify({ version: 99, session: 3, items: { A: { box: 2 } } });
    expect(parseProgress(raw)).toEqual(emptyProgress());
  });

  it("keeps valid entries and skips malformed ones", () => {
    const raw = JSON.stringify({
      version: 1,
      session: 7,
      items: {
        good: { box: 2, dueAtSession: 9 },
        notAnObject: 5,
        nested: [1],
        missingFields: {},
      },
    });
    const parsed = parseProgress(raw);
    expect(parsed.session).toBe(7);
    expect(parsed.items["good"]).toMatchObject({ box: 2, dueAtSession: 9 });
    expect(parsed.items["notAnObject"]).toBeUndefined();
    expect(parsed.items["nested"]).toBeUndefined();
    // Present but empty → defaults, not a crash.
    expect(parsed.items["missingFields"]).toMatchObject({ box: 0, dueAtSession: 0, reps: 0 });
  });

  it("does not let a __proto__ key reach Object.prototype", () => {
    const raw = '{"version":1,"session":0,"items":{"__proto__":{"box":5,"polluted":true}}}';
    const parsed = parseProgress(raw);
    expect(parsed.items["__proto__"]).toBeUndefined();
    // The prototype chain is untouched.
    expect(({} as Record<string, unknown>)["polluted"]).toBeUndefined();
    expect(Object.getPrototypeOf(parsed.items)).toBeNull();
  });

  it("coerces a non-finite or negative session to 0", () => {
    const raw = JSON.stringify({ version: 1, session: -4, items: {} });
    expect(parseProgress(raw).session).toBe(0);
  });
});

describe("storage adapter", () => {
  it("saves and reloads through a storage port", () => {
    const storage = fakeStorage();
    const progress = toSaved(["A"], [state(0, { box: 2, dueAtSession: 6, reps: 2 })], 3);

    expect(saveProgress(storage, progress)).toBe(true);
    expect(storage.data[STORAGE_KEY]).toBeDefined();
    expect(loadProgress(storage).items["A"]).toMatchObject({ box: 2, dueAtSession: 6, reps: 2 });
  });

  it("degrades rather than throwing when storage is unavailable", () => {
    expect(loadProgress(null)).toEqual(emptyProgress());
    expect(saveProgress(null, emptyProgress())).toBe(false);
  });

  it("degrades when storage throws (private mode, quota)", () => {
    const hostile: StorageLike = {
      getItem() {
        throw new Error("denied");
      },
      setItem() {
        throw new Error("quota");
      },
    };
    expect(loadProgress(hostile)).toEqual(emptyProgress());
    expect(saveProgress(hostile, emptyProgress())).toBe(false);
  });

  it("survives a hand-corrupted value in storage", () => {
    const storage = fakeStorage({ [STORAGE_KEY]: "<<garbage>>" });
    expect(loadProgress(storage)).toEqual(emptyProgress());
  });
});

describe("seenCount", () => {
  it("counts only ids with saved history", () => {
    const saved = toSaved(["A", "B"], [state(0, { box: 1, dueAtSession: 2, reps: 1 }), state(1)], 1);
    expect(seenCount(["A", "B", "C"], saved)).toBe(1);
  });
});
