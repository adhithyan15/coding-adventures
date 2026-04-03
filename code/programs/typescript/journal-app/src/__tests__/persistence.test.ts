/**
 * persistence.test.ts — Tests for the storage persistence middleware.
 *
 * Uses a mock storage object to verify that the middleware calls the
 * correct storage methods for each action type without actually touching
 * IndexedDB.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { Store } from "@coding-adventures/store";
import type { KVStorage } from "@coding-adventures/indexeddb";
import { reducer, initialState } from "../reducer.js";
import { createPersistenceMiddleware } from "../persistence.js";
import {
  ENTRY_CREATE,
  ENTRY_UPDATE,
  ENTRY_DELETE,
  ENTRIES_LOAD,
} from "../actions.js";
import type { AppState } from "../types.js";

// ── Mock storage ─────────────────────────────────────────────────────────────

function createMockStorage(): KVStorage {
  return {
    open: vi.fn().mockResolvedValue(undefined),
    get: vi.fn().mockResolvedValue(undefined),
    getAll: vi.fn().mockResolvedValue([]),
    put: vi.fn().mockResolvedValue(undefined),
    delete: vi.fn().mockResolvedValue(undefined),
    close: vi.fn(),
  };
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe("persistence middleware", () => {
  let mockStorage: KVStorage;
  let testStore: Store<AppState>;

  beforeEach(() => {
    mockStorage = createMockStorage();
    testStore = new Store<AppState>(initialState, reducer);
    testStore.use(createPersistenceMiddleware(mockStorage));
  });

  it("calls storage.put on ENTRY_CREATE", () => {
    testStore.dispatch({
      type: ENTRY_CREATE,
      id: "new-id",
      title: "Test",
      content: "Content",
      createdAt: "2026-04-02",
      updatedAt: 1000,
    });

    expect(mockStorage.put).toHaveBeenCalledWith("entries", {
      id: "new-id",
      title: "Test",
      content: "Content",
      createdAt: "2026-04-02",
      updatedAt: 1000,
    });
  });

  it("calls storage.put on ENTRY_UPDATE", () => {
    // First create an entry
    testStore.dispatch({
      type: ENTRY_CREATE,
      id: "e1",
      title: "Original",
      content: "Original content",
      createdAt: "2026-04-02",
      updatedAt: 1000,
    });

    vi.mocked(mockStorage.put).mockClear();

    // Then update it
    testStore.dispatch({
      type: ENTRY_UPDATE,
      id: "e1",
      title: "Updated",
      content: "Updated content",
      updatedAt: 2000,
    });

    expect(mockStorage.put).toHaveBeenCalledWith("entries", {
      id: "e1",
      title: "Updated",
      content: "Updated content",
      createdAt: "2026-04-02",
      updatedAt: 2000,
    });
  });

  it("calls storage.delete on ENTRY_DELETE", () => {
    testStore.dispatch({
      type: ENTRY_CREATE,
      id: "e1",
      title: "To Delete",
      content: "",
      createdAt: "2026-04-02",
      updatedAt: 1000,
    });

    testStore.dispatch({ type: ENTRY_DELETE, id: "e1" });

    expect(mockStorage.delete).toHaveBeenCalledWith("entries", "e1");
  });

  it("does not call any storage methods on ENTRIES_LOAD", () => {
    testStore.dispatch({
      type: ENTRIES_LOAD,
      entries: [
        {
          id: "loaded",
          title: "Loaded",
          content: "",
          createdAt: "2026-04-02",
          updatedAt: 1000,
        },
      ],
    });

    expect(mockStorage.put).not.toHaveBeenCalled();
    expect(mockStorage.delete).not.toHaveBeenCalled();
  });

  it("does not call storage on unknown action types", () => {
    testStore.dispatch({ type: "UNKNOWN" });

    expect(mockStorage.put).not.toHaveBeenCalled();
    expect(mockStorage.delete).not.toHaveBeenCalled();
  });
});
