/**
 * reducer.test.ts — Unit tests for the journal app reducer.
 *
 * The reducer is a pure function: (state, action) => newState. These tests
 * verify correctness, immutability, and edge cases for all four action types.
 */

import { describe, it, expect } from "vitest";
import { reducer, initialState } from "../reducer.js";
import type { AppState } from "../types.js";
import {
  ENTRY_CREATE,
  ENTRY_UPDATE,
  ENTRY_DELETE,
  ENTRIES_LOAD,
} from "../actions.js";
import type { Entry } from "../types.js";

// ── Helpers ──────────────────────────────────────────────────────────────────

function makeEntry(overrides: Partial<Entry> = {}): Entry {
  return {
    id: "test-1",
    title: "Test Entry",
    content: "Hello world",
    createdAt: "2026-04-02",
    updatedAt: 1000000,
    ...overrides,
  };
}

function stateWith(entries: Entry[]): AppState {
  return { entries };
}

// ── ENTRY_CREATE ─────────────────────────────────────────────────────────────

describe("ENTRY_CREATE", () => {
  it("appends a new entry to the list", () => {
    const result = reducer(initialState, {
      type: ENTRY_CREATE,
      id: "abc-123",
      title: "My Entry",
      content: "Some content",
      createdAt: "2026-04-02",
      updatedAt: 1000,
    });

    expect(result.entries).toHaveLength(1);
    expect(result.entries[0]).toEqual({
      id: "abc-123",
      title: "My Entry",
      content: "Some content",
      createdAt: "2026-04-02",
      updatedAt: 1000,
    });
  });

  it("does not replace existing entries", () => {
    const existing = makeEntry({ id: "existing" });
    const state = stateWith([existing]);

    const result = reducer(state, {
      type: ENTRY_CREATE,
      id: "new-id",
      title: "New",
      content: "",
      createdAt: "2026-04-03",
      updatedAt: 2000,
    });

    expect(result.entries).toHaveLength(2);
    expect(result.entries[0]).toEqual(existing);
  });

  it("returns a new state object (immutable)", () => {
    const result = reducer(initialState, {
      type: ENTRY_CREATE,
      id: "x",
      title: "T",
      content: "C",
      createdAt: "2026-01-01",
      updatedAt: 1,
    });

    expect(result).not.toBe(initialState);
    expect(result.entries).not.toBe(initialState.entries);
  });
});

// ── ENTRY_UPDATE ─────────────────────────────────────────────────────────────

describe("ENTRY_UPDATE", () => {
  it("updates title, content, and updatedAt", () => {
    const entry = makeEntry({ id: "e1" });
    const state = stateWith([entry]);

    const result = reducer(state, {
      type: ENTRY_UPDATE,
      id: "e1",
      title: "Updated Title",
      content: "Updated content",
      updatedAt: 9999,
    });

    expect(result.entries[0]).toEqual({
      ...entry,
      title: "Updated Title",
      content: "Updated content",
      updatedAt: 9999,
    });
  });

  it("does not modify other entries", () => {
    const e1 = makeEntry({ id: "e1", title: "First" });
    const e2 = makeEntry({ id: "e2", title: "Second" });
    const state = stateWith([e1, e2]);

    const result = reducer(state, {
      type: ENTRY_UPDATE,
      id: "e1",
      title: "Changed",
      content: "new",
      updatedAt: 5000,
    });

    expect(result.entries[1]).toEqual(e2);
  });

  it("returns state unchanged for unknown id", () => {
    const entry = makeEntry({ id: "e1" });
    const state = stateWith([entry]);

    const result = reducer(state, {
      type: ENTRY_UPDATE,
      id: "nonexistent",
      title: "X",
      content: "Y",
      updatedAt: 999,
    });

    expect(result).toBe(state);
  });

  it("preserves createdAt (does not overwrite)", () => {
    const entry = makeEntry({ id: "e1", createdAt: "2026-01-15" });
    const state = stateWith([entry]);

    const result = reducer(state, {
      type: ENTRY_UPDATE,
      id: "e1",
      title: "New",
      content: "New",
      updatedAt: 5000,
    });

    expect(result.entries[0]!.createdAt).toBe("2026-01-15");
  });
});

// ── ENTRY_DELETE ─────────────────────────────────────────────────────────────

describe("ENTRY_DELETE", () => {
  it("removes the entry with matching id", () => {
    const e1 = makeEntry({ id: "e1" });
    const e2 = makeEntry({ id: "e2" });
    const state = stateWith([e1, e2]);

    const result = reducer(state, { type: ENTRY_DELETE, id: "e1" });

    expect(result.entries).toHaveLength(1);
    expect(result.entries[0]!.id).toBe("e2");
  });

  it("returns state with empty entries when deleting the only entry", () => {
    const entry = makeEntry({ id: "only" });
    const state = stateWith([entry]);

    const result = reducer(state, { type: ENTRY_DELETE, id: "only" });

    expect(result.entries).toHaveLength(0);
  });

  it("does not fail for unknown id", () => {
    const entry = makeEntry({ id: "e1" });
    const state = stateWith([entry]);

    const result = reducer(state, { type: ENTRY_DELETE, id: "nonexistent" });

    expect(result.entries).toHaveLength(1);
  });
});

// ── ENTRIES_LOAD ─────────────────────────────────────────────────────────────

describe("ENTRIES_LOAD", () => {
  it("replaces entries array entirely", () => {
    const loaded = [
      makeEntry({ id: "a" }),
      makeEntry({ id: "b" }),
    ];

    const result = reducer(initialState, {
      type: ENTRIES_LOAD,
      entries: loaded,
    });

    expect(result.entries).toEqual(loaded);
  });

  it("clears entries when loaded with empty array", () => {
    const state = stateWith([makeEntry()]);

    const result = reducer(state, { type: ENTRIES_LOAD, entries: [] });

    expect(result.entries).toHaveLength(0);
  });

  it("replaces pre-existing entries", () => {
    const state = stateWith([makeEntry({ id: "old" })]);
    const loaded = [makeEntry({ id: "new" })];

    const result = reducer(state, { type: ENTRIES_LOAD, entries: loaded });

    expect(result.entries).toHaveLength(1);
    expect(result.entries[0]!.id).toBe("new");
  });
});

// ── Unknown actions ──────────────────────────────────────────────────────────

describe("unknown actions", () => {
  it("returns state unchanged for unknown action type", () => {
    const state = stateWith([makeEntry()]);
    const result = reducer(state, { type: "UNKNOWN_ACTION" });
    expect(result).toBe(state);
  });
});
