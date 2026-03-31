/**
 * Bookmark Storage — Contract Tests
 * ===================================
 *
 * What are contract tests?
 * -------------------------
 * When you have multiple implementations of the same interface (IndexedDB,
 * InMemory, future Google Drive, etc.), you need to ensure they ALL behave
 * the same way. Contract tests are a reusable test suite that any backend
 * must pass.
 *
 * The function `runStorageContractTests` takes a factory that creates a
 * fresh storage instance, then exercises every operation in the
 * BookmarkStorage interface. If a backend passes these tests, it's
 * interchangeable with any other backend.
 *
 * ```
 *   runStorageContractTests("IndexedDB", () => new IndexedDBStorage())
 *   runStorageContractTests("InMemory",  () => new InMemoryStorage())
 *   runStorageContractTests("Drive",     () => new DriveStorage())  // future
 * ```
 */

import { describe, it, expect, beforeEach, test } from "vitest";
import type { BookmarkStorage } from "../src/storage/bookmark-storage";

// Vitest requires at least one test in a file. This file exports a
// reusable contract test function that other test files call.
// The actual test runs happen in indexeddb-storage.test.ts.
test("contract test module loads", () => {
  expect(runStorageContractTests).toBeDefined();
});

/**
 * Run the full contract test suite against a storage backend.
 *
 * @param name - A human-readable name for the backend (used in test output)
 * @param createStorage - A factory that creates a fresh, uninitialized storage
 */
export function runStorageContractTests(
  name: string,
  createStorage: () => BookmarkStorage,
): void {
  describe(`${name} — BookmarkStorage contract`, () => {
    let storage: BookmarkStorage;

    beforeEach(async () => {
      storage = createStorage();
      await storage.initialize();
    });

    // =================================================================
    // save()
    // =================================================================

    it("saves a bookmark and returns it with generated fields", async () => {
      const input = {
        url: "https://example.com",
        title: "Example",
        note: "A test bookmark",
      };

      const saved = await storage.save(input);

      // Auto-generated fields should be present
      expect(saved.id).toBeTruthy();
      expect(saved.createdAt).toBeTruthy();
      expect(saved.updatedAt).toBeTruthy();

      // Input fields should be preserved
      expect(saved.url).toBe("https://example.com");
      expect(saved.title).toBe("Example");
      expect(saved.note).toBe("A test bookmark");

      // Timestamps should be valid ISO 8601
      expect(new Date(saved.createdAt).toISOString()).toBe(saved.createdAt);
      expect(new Date(saved.updatedAt).toISOString()).toBe(saved.updatedAt);

      // On creation, createdAt and updatedAt should be the same
      expect(saved.createdAt).toBe(saved.updatedAt);
    });

    it("generates unique IDs for different bookmarks", async () => {
      const a = await storage.save({
        url: "https://a.com",
        title: "A",
        note: "",
      });
      const b = await storage.save({
        url: "https://b.com",
        title: "B",
        note: "",
      });

      expect(a.id).not.toBe(b.id);
    });

    // =================================================================
    // getAll()
    // =================================================================

    it("returns all saved bookmarks", async () => {
      await storage.save({ url: "https://a.com", title: "A", note: "" });
      await storage.save({ url: "https://b.com", title: "B", note: "" });
      await storage.save({ url: "https://c.com", title: "C", note: "" });

      const all = await storage.getAll();
      expect(all).toHaveLength(3);
    });

    it("returns empty array when no bookmarks exist", async () => {
      const all = await storage.getAll();
      expect(all).toEqual([]);
    });

    it("returns bookmarks sorted by updatedAt descending", async () => {
      const a = await storage.save({ url: "https://a.com", title: "A", note: "" });
      await storage.save({ url: "https://b.com", title: "B", note: "" });

      // Small delay to ensure a distinct updatedAt timestamp
      await new Promise((r) => setTimeout(r, 10));

      // Update 'a' so it has the most recent updatedAt
      await storage.update(a.id, { note: "updated" });

      const all = await storage.getAll();
      expect(all[0].url).toBe("https://a.com");
    });

    // =================================================================
    // getByUrl()
    // =================================================================

    it("finds a bookmark by URL", async () => {
      await storage.save({
        url: "https://example.com/page",
        title: "Page",
        note: "Notes about this page",
      });

      const found = await storage.getByUrl("https://example.com/page");
      expect(found).not.toBeNull();
      expect(found!.title).toBe("Page");
      expect(found!.note).toBe("Notes about this page");
    });

    it("returns null for an unknown URL", async () => {
      const found = await storage.getByUrl("https://nonexistent.com");
      expect(found).toBeNull();
    });

    // =================================================================
    // getById()
    // =================================================================

    it("finds a bookmark by ID", async () => {
      const saved = await storage.save({
        url: "https://example.com",
        title: "Example",
        note: "",
      });

      const found = await storage.getById(saved.id);
      expect(found).not.toBeNull();
      expect(found!.url).toBe("https://example.com");
    });

    it("returns null for an unknown ID", async () => {
      const found = await storage.getById("nonexistent-id");
      expect(found).toBeNull();
    });

    // =================================================================
    // update()
    // =================================================================

    it("updates a bookmark's title", async () => {
      const saved = await storage.save({
        url: "https://example.com",
        title: "Old Title",
        note: "Some note",
      });

      const updated = await storage.update(saved.id, { title: "New Title" });

      expect(updated.title).toBe("New Title");
      expect(updated.note).toBe("Some note"); // unchanged
      expect(updated.url).toBe("https://example.com"); // unchanged
    });

    it("updates a bookmark's note", async () => {
      const saved = await storage.save({
        url: "https://example.com",
        title: "Title",
        note: "Old note",
      });

      const updated = await storage.update(saved.id, { note: "New note" });

      expect(updated.note).toBe("New note");
      expect(updated.title).toBe("Title"); // unchanged
    });

    it("updates both title and note at once", async () => {
      const saved = await storage.save({
        url: "https://example.com",
        title: "Old",
        note: "Old",
      });

      const updated = await storage.update(saved.id, {
        title: "New Title",
        note: "New Note",
      });

      expect(updated.title).toBe("New Title");
      expect(updated.note).toBe("New Note");
    });

    it("bumps updatedAt on update", async () => {
      const saved = await storage.save({
        url: "https://example.com",
        title: "Title",
        note: "",
      });

      // Small delay to ensure timestamp difference
      await new Promise((r) => setTimeout(r, 10));

      const updated = await storage.update(saved.id, { note: "Updated" });

      expect(new Date(updated.updatedAt).getTime()).toBeGreaterThan(
        new Date(saved.updatedAt).getTime()
      );
      // createdAt should NOT change
      expect(updated.createdAt).toBe(saved.createdAt);
    });

    it("throws when updating a non-existent bookmark", async () => {
      await expect(
        storage.update("nonexistent-id", { note: "test" })
      ).rejects.toThrow();
    });

    // =================================================================
    // delete()
    // =================================================================

    it("deletes a bookmark", async () => {
      const saved = await storage.save({
        url: "https://example.com",
        title: "Title",
        note: "",
      });

      await storage.delete(saved.id);

      const found = await storage.getById(saved.id);
      expect(found).toBeNull();
    });

    it("is idempotent — deleting a non-existent bookmark does not throw", async () => {
      // Should not throw
      await expect(storage.delete("nonexistent-id")).resolves.toBeUndefined();
    });

    it("only deletes the specified bookmark", async () => {
      const a = await storage.save({ url: "https://a.com", title: "A", note: "" });
      const b = await storage.save({ url: "https://b.com", title: "B", note: "" });

      await storage.delete(a.id);

      const all = await storage.getAll();
      expect(all).toHaveLength(1);
      expect(all[0].id).toBe(b.id);
    });
  });
}
