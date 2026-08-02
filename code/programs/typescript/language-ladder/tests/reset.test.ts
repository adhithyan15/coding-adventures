import { describe, it, expect } from "vitest";
import {
  OWNED_STORAGE_KEYS,
  clearProgress,
  type RemovableStorage,
} from "../src/reset";
import { REVIEW_STORAGE_KEY } from "../src/reviewstore";
import { CURSOR_STORAGE_KEY } from "../src/cursorstore";
import { STORAGE_KEY as LESSON_SCHEDULE_KEY } from "../src/progress";
import { LANGUAGE_STORAGE_KEY } from "../src/languagestore";

function fakeStore(seed: Record<string, string> = {}): RemovableStorage & {
  data: Map<string, string>;
} {
  const data = new Map(Object.entries(seed));
  return { data, removeItem: (k) => void data.delete(k) };
}

describe("OWNED_STORAGE_KEYS", () => {
  it("lists exactly the four keys the app persists, sourced from their owners", () => {
    // If a persisted key were forgotten here, a reset would silently leave state
    // behind — so pin the set explicitly.
    expect(new Set(OWNED_STORAGE_KEYS)).toEqual(
      new Set([REVIEW_STORAGE_KEY, CURSOR_STORAGE_KEY, LESSON_SCHEDULE_KEY, LANGUAGE_STORAGE_KEY]),
    );
  });
});

describe("clearProgress", () => {
  it("removes every owned key", () => {
    const store = fakeStore({
      [REVIEW_STORAGE_KEY]: "r",
      [CURSOR_STORAGE_KEY]: "c",
      [LESSON_SCHEDULE_KEY]: "l",
      [LANGUAGE_STORAGE_KEY]: "g",
    });
    clearProgress(store);
    expect(store.data.size).toBe(0);
  });

  it("CONTROL: fails loudly if any owned key is left behind", () => {
    // Reconstruct the store, clear it, and assert NONE of the owned keys survive.
    // A clearProgress that missed (say) the cursor key would leave it in `data`
    // and fail this assertion — the check the OWNED list exists to guarantee.
    const seed: Record<string, string> = {};
    for (const k of OWNED_STORAGE_KEYS) seed[k] = "x";
    const store = fakeStore(seed);
    clearProgress(store);
    for (const k of OWNED_STORAGE_KEYS) {
      expect(store.data.has(k)).toBe(false);
    }
  });

  it("leaves keys the app does NOT own untouched", () => {
    const store = fakeStore({
      [REVIEW_STORAGE_KEY]: "r",
      "someone-elses-key": "keep me",
    });
    clearProgress(store);
    expect(store.data.has(REVIEW_STORAGE_KEY)).toBe(false);
    expect(store.data.get("someone-elses-key")).toBe("keep me");
  });

  it("a null storage is a no-op, not a crash", () => {
    expect(() => clearProgress(null)).not.toThrow();
  });

  it("keeps clearing even if one removeItem throws", () => {
    const data = new Map<string, string>(OWNED_STORAGE_KEYS.map((k) => [k, "x"]));
    const flaky: RemovableStorage = {
      removeItem: (k) => {
        if (k === CURSOR_STORAGE_KEY) throw new Error("locked");
        data.delete(k);
      },
    };
    clearProgress(flaky);
    // the throwing key remains, but the others were still cleared
    expect(data.has(CURSOR_STORAGE_KEY)).toBe(true);
    expect(data.has(REVIEW_STORAGE_KEY)).toBe(false);
    expect(data.has(LESSON_SCHEDULE_KEY)).toBe(false);
  });
});
