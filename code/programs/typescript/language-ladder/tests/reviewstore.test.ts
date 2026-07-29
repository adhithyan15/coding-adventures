import { describe, it, expect } from "vitest";
import {
  toSavedReview,
  fromSavedReview,
  parseReview,
  loadReview,
  saveReview,
  emptyReview,
  REVIEW_SCHEMA_VERSION,
  type SavedReview,
  type StorageLike,
} from "../src/reviewstore";
import type { Progress } from "../src/sessionplan";
import type { QuizState } from "../src/quiz";

function progressOf(
  entries: Array<[string, QuizState]>,
  log: Progress["log"] = [],
): Progress {
  return { states: new Map(entries), log };
}

const ST: QuizState = { box: 2, dueAtSession: 7, lapses: 1, reps: 3 };

describe("toSavedReview / fromSavedReview — the round trip", () => {
  it("preserves states, log, and session exactly", () => {
    const progress = progressOf(
      [
        ['["THANKS","hindi","hi-t"]', ST],
        ['["THANKS","tamil","ta-t"]', { box: 0, dueAtSession: 9, lapses: 0, reps: 1 }],
      ],
      [
        { cellKey: '["THANKS","tamil","ta-t"]', correct: false, chosenKey: '["THANKS","hindi","hi-t"]' },
        { cellKey: '["THANKS","hindi","hi-t"]', correct: true },
      ],
    );
    const restored = fromSavedReview(toSavedReview(progress, 9));
    expect(restored.session).toBe(9);
    expect([...restored.progress.states.entries()]).toEqual([...progress.states.entries()]);
    expect(restored.progress.log).toEqual(progress.log);
  });

  it("CONTROL: tolerates a raw non-array states/log without throwing", () => {
    // fromSavedReview is a public entry point; a caller may hand it an untrusted
    // object whose fields aren't arrays. Without the Array.isArray guards the
    // for...of would throw; it must instead yield an empty restore.
    const bogus = { version: REVIEW_SCHEMA_VERSION, session: 2, states: "nope", log: 42 } as unknown as SavedReview;
    const restored = fromSavedReview(bogus);
    expect([...restored.progress.states.entries()]).toEqual([]);
    expect(restored.progress.log).toEqual([]);
    expect(restored.session).toBe(2);
  });

  it("a wrong answer keeps its chosenKey; a correct one never carries one", () => {
    const saved: SavedReview = {
      version: REVIEW_SCHEMA_VERSION,
      session: 1,
      states: [],
      // a correct row with a stray chosenKey must be sanitized away on restore
      log: [{ cellKey: "a", correct: true, chosenKey: "sneaky" } as never],
    };
    const restored = fromSavedReview(saved);
    expect(restored.progress.log).toEqual([{ cellKey: "a", correct: true }]);
  });
});

describe("parseReview — defensive against untrusted input", () => {
  it("round-trips a well-formed blob", () => {
    const saved = toSavedReview(progressOf([['["C","spanish","es"]', ST]]), 4);
    const parsed = parseReview(JSON.stringify(saved));
    expect(parsed).toEqual(saved);
  });

  it("CONTROL: a wrong-version blob loads EMPTY, not the stale data", () => {
    const stale = { version: 999, session: 5, states: [["k", ST]], log: [] };
    const parsed = parseReview(JSON.stringify(stale));
    // If the version gate were removed, this would surface the stale state and fail.
    expect(parsed).toEqual(emptyReview());
    expect(parsed.states).toEqual([]);
  });

  it("non-JSON, null, and non-object blobs load empty rather than throwing", () => {
    expect(parseReview("not json{")).toEqual(emptyReview());
    expect(parseReview(null)).toEqual(emptyReview());
    expect(parseReview("[]")).toEqual(emptyReview());
    expect(parseReview("42")).toEqual(emptyReview());
  });

  it("drops malformed state rows and clamps out-of-range fields", () => {
    const blob = JSON.stringify({
      version: REVIEW_SCHEMA_VERSION,
      session: -3, // clamped to 0
      states: [
        ["good", { box: 999, dueAtSession: 2.9, lapses: -1, reps: "x" }], // clamped/coerced
        ["bad-not-a-pair"], // dropped (length !== 2)
        [42, ST], // dropped (key not a string)
        "nope", // dropped (not an array)
      ],
      log: [
        { cellKey: "k", correct: false, chosenKey: "j" }, // kept
        { cellKey: 5, correct: true }, // dropped (cellKey not a string)
        { correct: true }, // dropped (no cellKey)
        null, // dropped
      ],
    });
    const parsed = parseReview(blob);
    expect(parsed.session).toBe(0);
    expect(parsed.states).toEqual([
      ["good", { box: 5 /* MAX_BOX */, dueAtSession: 2, lapses: 0, reps: 0 }],
    ]);
    expect(parsed.log).toEqual([{ cellKey: "k", correct: false, chosenKey: "j" }]);
  });
});

describe("loadReview / saveReview — through a fake storage port", () => {
  function fakeStorage(): StorageLike & { data: Map<string, string> } {
    const data = new Map<string, string>();
    return {
      data,
      getItem: (k) => data.get(k) ?? null,
      setItem: (k, v) => void data.set(k, v),
    };
  }

  it("save then load restores the same Progress + session", () => {
    const store = fakeStorage();
    const progress = progressOf(
      [['["C","latin","la"]', ST]],
      [{ cellKey: '["C","latin","la"]', correct: true }],
    );
    expect(saveReview(store, progress, 6)).toBe(true);
    const { progress: back, session } = loadReview(store);
    expect(session).toBe(6);
    expect([...back.states.entries()]).toEqual([...progress.states.entries()]);
    expect(back.log).toEqual(progress.log);
  });

  it("a null storage (private mode / SSR) is a no-op, not a crash", () => {
    expect(saveReview(null, progressOf([]), 0)).toBe(false);
    expect(loadReview(null)).toEqual({ progress: { states: new Map(), log: [] }, session: 0 });
  });

  it("CONTROL: a storage whose getItem throws loads empty rather than propagating", () => {
    const throwing: StorageLike = {
      getItem: () => {
        throw new Error("SecurityError");
      },
      setItem: () => {},
    };
    // If loadReview didn't guard getItem, this would throw and break app startup.
    expect(loadReview(throwing)).toEqual({ progress: { states: new Map(), log: [] }, session: 0 });
  });
});
