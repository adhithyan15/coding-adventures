/**
 * storage.shared.ts — Shared test suite for KVStorage implementations.
 *
 * This function registers tests against a factory that creates a fresh
 * KVStorage instance. The same tests run against both MemoryStorage and
 * (in a browser test environment) IndexedDBStorage.
 */

import { describe, it, expect, beforeEach } from "vitest";
import type { KVStorage } from "../types.js";

interface TestRecord {
  id: string;
  name: string;
  value?: number;
  [key: string]: unknown;
}

export function runStorageTests(
  name: string,
  createStorage: () => KVStorage,
) {
  describe(name, () => {
    let storage: KVStorage;

    beforeEach(async () => {
      storage = createStorage();
      await storage.open();
    });

    it("getAll returns empty array for empty store", async () => {
      const all = await storage.getAll<TestRecord>("items");
      expect(all).toEqual([]);
    });

    it("put + get round-trips a record", async () => {
      const record: TestRecord = { id: "abc", name: "Alpha" };
      await storage.put("items", record);
      const result = await storage.get<TestRecord>("items", "abc");
      expect(result).toEqual(record);
    });

    it("put + getAll returns the record", async () => {
      await storage.put("items", { id: "a", name: "Alpha" });
      await storage.put("items", { id: "b", name: "Beta" });
      const all = await storage.getAll<TestRecord>("items");
      expect(all).toHaveLength(2);
      expect(all.map((r) => r.name).sort()).toEqual(["Alpha", "Beta"]);
    });

    it("put overwrites existing record with same key", async () => {
      await storage.put("items", { id: "a", name: "Old" });
      await storage.put("items", { id: "a", name: "New" });
      const result = await storage.get<TestRecord>("items", "a");
      expect(result?.name).toBe("New");
    });

    it("get returns undefined for missing key", async () => {
      const result = await storage.get<TestRecord>("items", "nonexistent");
      expect(result).toBeUndefined();
    });

    it("delete removes a record", async () => {
      await storage.put("items", { id: "a", name: "Alpha" });
      await storage.delete("items", "a");
      const result = await storage.get<TestRecord>("items", "a");
      expect(result).toBeUndefined();
    });

    it("delete is a no-op for missing key", async () => {
      await expect(storage.delete("items", "nonexistent")).resolves.toBeUndefined();
    });

    it("getAll returns empty after all records deleted", async () => {
      await storage.put("items", { id: "a", name: "Alpha" });
      await storage.delete("items", "a");
      const all = await storage.getAll<TestRecord>("items");
      expect(all).toEqual([]);
    });

    it("operates independently across stores", async () => {
      await storage.put("items", { id: "a", name: "Item A" });
      await storage.put("other", { id: "a", name: "Other A" });
      const item = await storage.get<TestRecord>("items", "a");
      const other = await storage.get<TestRecord>("other", "a");
      expect(item?.name).toBe("Item A");
      expect(other?.name).toBe("Other A");
    });

    it("preserves nested objects", async () => {
      const record = {
        id: "nested",
        name: "Nested",
        items: [{ child: true }],
        meta: { depth: 3 },
      };
      await storage.put("items", record);
      const result = await storage.get<typeof record>("items", "nested");
      expect(result?.items).toEqual([{ child: true }]);
      expect(result?.meta).toEqual({ depth: 3 });
    });
  });
}
