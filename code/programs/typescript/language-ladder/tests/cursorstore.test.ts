import { describe, it, expect } from "vitest";
import {
  clampCursor,
  parseCursor,
  loadCursor,
  saveCursor,
  CURSOR_SCHEMA_VERSION,
  CURSOR_STORAGE_KEY,
  type StorageLike,
} from "../src/cursorstore";

describe("clampCursor", () => {
  it("bounds an index into [0, length-1]", () => {
    expect(clampCursor(5, 10)).toBe(5);
    expect(clampCursor(999, 10)).toBe(9); // past the end → last concept
    expect(clampCursor(-3, 10)).toBe(0);
    expect(clampCursor(3.9, 10)).toBe(3); // truncated
  });

  it("an empty spine or a non-finite index clamps to 0", () => {
    expect(clampCursor(4, 0)).toBe(0);
    expect(clampCursor(NaN, 10)).toBe(0);
    expect(clampCursor(Infinity, 10)).toBe(0);
  });
});

describe("parseCursor — defensive against untrusted input", () => {
  it("reads a well-formed blob", () => {
    expect(parseCursor(JSON.stringify({ version: CURSOR_SCHEMA_VERSION, index: 12 }))).toBe(12);
  });

  it("CONTROL: a wrong-version blob is dropped to 0, not trusted", () => {
    const stale = JSON.stringify({ version: 999, index: 12 });
    // Remove the version gate and this would return 12 and fail.
    expect(parseCursor(stale)).toBe(0);
  });

  it("non-JSON, null, arrays, and non-object blobs return 0 (no throw)", () => {
    expect(parseCursor("not json{")).toBe(0);
    expect(parseCursor(null)).toBe(0);
    expect(parseCursor("[]")).toBe(0);
    expect(parseCursor("42")).toBe(0);
  });

  it("a missing / non-numeric / negative index returns 0", () => {
    expect(parseCursor(JSON.stringify({ version: CURSOR_SCHEMA_VERSION }))).toBe(0);
    expect(parseCursor(JSON.stringify({ version: CURSOR_SCHEMA_VERSION, index: "x" }))).toBe(0);
    expect(parseCursor(JSON.stringify({ version: CURSOR_SCHEMA_VERSION, index: -1 }))).toBe(0);
    expect(parseCursor(JSON.stringify({ version: CURSOR_SCHEMA_VERSION, index: null }))).toBe(0);
  });
});

describe("loadCursor / saveCursor — through a fake storage port", () => {
  function fakeStorage(): StorageLike & { data: Map<string, string> } {
    const data = new Map<string, string>();
    return {
      data,
      getItem: (k) => data.get(k) ?? null,
      setItem: (k, v) => void data.set(k, v),
    };
  }

  it("save then load round-trips the index, clamped to the spine", () => {
    const store = fakeStorage();
    expect(saveCursor(store, 7)).toBe(true);
    expect(store.data.get(CURSOR_STORAGE_KEY)).toBe(
      JSON.stringify({ version: CURSOR_SCHEMA_VERSION, index: 7 }),
    );
    expect(loadCursor(store, 186)).toBe(7);
  });

  it("a saved index past the (now shorter) spine clamps down on load", () => {
    const store = fakeStorage();
    saveCursor(store, 40);
    // Curriculum shrank to 10 concepts since the save → resume at the last one.
    expect(loadCursor(store, 10)).toBe(9);
  });

  it("null storage is a no-op / start-at-0, never a crash", () => {
    expect(saveCursor(null, 5)).toBe(false);
    expect(loadCursor(null, 100)).toBe(0);
  });

  it("CONTROL: a storage whose getItem throws loads 0 rather than propagating", () => {
    const throwing: StorageLike = {
      getItem: () => {
        throw new Error("SecurityError");
      },
      setItem: () => {},
    };
    // Without the try/catch in loadCursor this would throw and break startup.
    expect(loadCursor(throwing, 50)).toBe(0);
  });

  it("no saved value starts at 0", () => {
    expect(loadCursor(fakeStorage(), 50)).toBe(0);
  });
});
