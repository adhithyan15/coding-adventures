/**
 * IndexedDB Storage Tests
 * ========================
 *
 * These tests verify the IndexedDB implementation using `fake-indexeddb`,
 * which provides a complete W3C IndexedDB 2.0 implementation in Node.js.
 *
 * Test structure:
 * 1. Run the contract tests (shared with all backends)
 * 2. Run IDB-specific tests (database creation, indexes, etc.)
 *
 * The contract tests guarantee that IndexedDBStorage is interchangeable
 * with any other BookmarkStorage implementation. The IDB-specific tests
 * verify internal details that only matter for this particular backend.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { IndexedDBStorage } from "../src/storage/indexeddb-storage";
import { runStorageContractTests } from "./bookmark-storage.test";
import { InMemoryStorage } from "./test-helpers";

// =========================================================================
// Contract tests — IndexedDBStorage must pass the same tests as all backends
// =========================================================================

/**
 * Each test gets a fresh database by using a unique name.
 *
 * Why? IndexedDB databases persist across tests in fake-indexeddb.
 * If two tests use the same database name, one test's data leaks
 * into another. By incrementing a counter, each test gets isolation.
 */
let dbCounter = 0;

function createFreshIndexedDBStorage(): IndexedDBStorage {
  // Each test gets a unique database name to avoid cross-test
  // interference. fake-indexeddb persists databases within a
  // test file, so same-name databases leak data between tests.
  dbCounter++;
  return new IndexedDBStorage(`test-bookmarks-${dbCounter}`)
}

runStorageContractTests("IndexedDB", createFreshIndexedDBStorage);

// =========================================================================
// Contract tests — InMemoryStorage (verify the test fake is faithful)
// =========================================================================

runStorageContractTests("InMemory", () => new InMemoryStorage());

// =========================================================================
// IDB-specific tests
// =========================================================================

describe("IndexedDBStorage — IDB-specific behavior", () => {
  let storage: IndexedDBStorage;

  beforeEach(async () => {
    storage = createFreshIndexedDBStorage();
    await storage.initialize();
  });

  it("can be initialized multiple times without error", async () => {
    // Re-initializing should be safe (e.g., if the popup opens twice)
    await expect(storage.initialize()).resolves.toBeUndefined();
  });

  it("throws if used before initialization", async () => {
    const uninitializedStorage = new IndexedDBStorage("test-uninitialized");
    await expect(uninitializedStorage.getAll()).rejects.toThrow(
      "Database not initialized"
    );
  });

  it("rejects saving a duplicate URL", async () => {
    await storage.save({
      url: "https://example.com",
      title: "First",
      note: "",
    });

    // IndexedDB's unique index on URL should cause this to fail
    await expect(
      storage.save({
        url: "https://example.com",
        title: "Duplicate",
        note: "",
      })
    ).rejects.toThrow();
  });
});
