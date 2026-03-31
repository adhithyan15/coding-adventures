/**
 * memory-storage.test.ts — Tests for the MemoryStorage implementation.
 *
 * Tests are organized by capability:
 *   1. Connection (open/close)
 *   2. CRUD (get/getAll/put/delete)
 *   3. SQL Querying (query)
 *   4. Transactions (commit/rollback)
 */

import { describe, it, expect, beforeEach } from "vitest";
import { MemoryStorage } from "../memory-storage.js";
import type { Storage } from "../types.js";

// ── Test fixtures ──────────────────────────────────────────────────────────

function createStorage(): Storage {
  return new MemoryStorage([
    { name: "users", keyPath: "id" },
    { name: "posts", keyPath: "id" },
  ]);
}

async function createSeededStorage(): Promise<Storage> {
  const storage = createStorage();
  await storage.open();
  await storage.put("users", { id: "1", name: "Alice", age: 30 });
  await storage.put("users", { id: "2", name: "Bob", age: 25 });
  await storage.put("users", { id: "3", name: "Carol", age: 35 });
  await storage.put("posts", { id: "p1", userId: "1", title: "Hello World" });
  await storage.put("posts", { id: "p2", userId: "2", title: "Second Post" });
  return storage;
}

// ── 1. Connection ──────────────────────────────────────────────────────────

describe("Connection", () => {
  it("open() creates stores defined in schema", async () => {
    const storage = createStorage();
    await storage.open();
    // Should not throw when accessing declared stores
    const users = await storage.getAll("users");
    expect(users).toEqual([]);
  });

  it("open() is idempotent — calling twice does not clear data", async () => {
    const storage = createStorage();
    await storage.open();
    await storage.put("users", { id: "1", name: "Alice", age: 30 });
    await storage.open();
    const user = await storage.get("users", "1");
    expect(user).toEqual({ id: "1", name: "Alice", age: 30 });
  });

  it("close() is a no-op for MemoryStorage", async () => {
    const storage = createStorage();
    await storage.open();
    storage.close();
    // MemoryStorage doesn't enforce closed state — this is intentional
  });
});

// ── 2. CRUD ────────────────────────────────────────────────────────────────

describe("CRUD", () => {
  let storage: Storage;

  beforeEach(async () => {
    storage = createStorage();
    await storage.open();
  });

  describe("put()", () => {
    it("stores a record that can be retrieved by get()", async () => {
      await storage.put("users", { id: "1", name: "Alice", age: 30 });
      const user = await storage.get("users", "1");
      expect(user).toEqual({ id: "1", name: "Alice", age: 30 });
    });

    it("upserts — replaces an existing record with the same key", async () => {
      await storage.put("users", { id: "1", name: "Alice", age: 30 });
      await storage.put("users", { id: "1", name: "Alice Updated", age: 31 });
      const user = await storage.get("users", "1");
      expect(user).toEqual({ id: "1", name: "Alice Updated", age: 31 });
    });

    it("throws if record is missing the keyPath field", async () => {
      await expect(
        storage.put("users", { name: "No ID" }),
      ).rejects.toThrow('Record missing key field "id"');
    });

    it("throws for unknown store", async () => {
      await expect(
        storage.put("unknown", { id: "1" }),
      ).rejects.toThrow("Unknown store: unknown");
    });
  });

  describe("get()", () => {
    it("returns undefined for a missing key", async () => {
      const result = await storage.get("users", "nonexistent");
      expect(result).toBeUndefined();
    });

    it("returns the record for an existing key", async () => {
      await storage.put("users", { id: "1", name: "Alice", age: 30 });
      const user = await storage.get("users", "1");
      expect(user).toEqual({ id: "1", name: "Alice", age: 30 });
    });

    it("throws for unknown store", async () => {
      await expect(storage.get("unknown", "1")).rejects.toThrow(
        "Unknown store: unknown",
      );
    });
  });

  describe("getAll()", () => {
    it("returns empty array for an empty store", async () => {
      const users = await storage.getAll("users");
      expect(users).toEqual([]);
    });

    it("returns all records in the store", async () => {
      await storage.put("users", { id: "1", name: "Alice", age: 30 });
      await storage.put("users", { id: "2", name: "Bob", age: 25 });
      const users = await storage.getAll("users");
      expect(users).toHaveLength(2);
      expect(users).toContainEqual({ id: "1", name: "Alice", age: 30 });
      expect(users).toContainEqual({ id: "2", name: "Bob", age: 25 });
    });

    it("different stores are independent", async () => {
      await storage.put("users", { id: "1", name: "Alice", age: 30 });
      await storage.put("posts", { id: "p1", userId: "1", title: "Hello" });
      const users = await storage.getAll("users");
      const posts = await storage.getAll("posts");
      expect(users).toHaveLength(1);
      expect(posts).toHaveLength(1);
    });
  });

  describe("delete()", () => {
    it("removes an existing record", async () => {
      await storage.put("users", { id: "1", name: "Alice", age: 30 });
      await storage.delete("users", "1");
      const user = await storage.get("users", "1");
      expect(user).toBeUndefined();
    });

    it("is a no-op for a missing key", async () => {
      // Should not throw
      await storage.delete("users", "nonexistent");
      const users = await storage.getAll("users");
      expect(users).toEqual([]);
    });

    it("does not affect other records", async () => {
      await storage.put("users", { id: "1", name: "Alice", age: 30 });
      await storage.put("users", { id: "2", name: "Bob", age: 25 });
      await storage.delete("users", "1");
      const users = await storage.getAll("users");
      expect(users).toHaveLength(1);
      expect(users[0]).toEqual({ id: "2", name: "Bob", age: 25 });
    });
  });
});

// ── 3. SQL Querying ────────────────────────────────────────────────────────

describe("SQL Querying", () => {
  let storage: Storage;

  beforeEach(async () => {
    storage = await createSeededStorage();
  });

  it("SELECT * returns all records", async () => {
    const result = await storage.query("SELECT * FROM users");
    expect(result.rows).toHaveLength(3);
    expect(result.columns).toContain("id");
    expect(result.columns).toContain("name");
    expect(result.columns).toContain("age");
  });

  it("SELECT with specific columns", async () => {
    const result = await storage.query("SELECT name, age FROM users");
    expect(result.columns).toEqual(["name", "age"]);
    expect(result.rows).toHaveLength(3);
  });

  it("WHERE clause filters rows", async () => {
    const result = await storage.query("SELECT * FROM users WHERE age > 28");
    expect(result.rows).toHaveLength(2);
    const names = result.rows.map((r) => r["name"]);
    expect(names).toContain("Alice");
    expect(names).toContain("Carol");
  });

  it("WHERE with string equality", async () => {
    const result = await storage.query(
      "SELECT * FROM users WHERE name = 'Bob'",
    );
    expect(result.rows).toHaveLength(1);
    expect(result.rows[0]!["name"]).toBe("Bob");
  });

  it("ORDER BY sorts results", async () => {
    const result = await storage.query(
      "SELECT name FROM users ORDER BY age ASC",
    );
    const names = result.rows.map((r) => r["name"]);
    expect(names).toEqual(["Bob", "Alice", "Carol"]);
  });

  it("ORDER BY DESC reverses order", async () => {
    const result = await storage.query(
      "SELECT name FROM users ORDER BY age DESC",
    );
    const names = result.rows.map((r) => r["name"]);
    expect(names).toEqual(["Carol", "Alice", "Bob"]);
  });

  it("LIMIT restricts result count", async () => {
    const result = await storage.query(
      "SELECT * FROM users ORDER BY age ASC LIMIT 2",
    );
    expect(result.rows).toHaveLength(2);
  });

  it("COUNT(*) aggregate", async () => {
    const result = await storage.query("SELECT COUNT(*) AS total FROM users");
    expect(result.rows).toHaveLength(1);
    expect(result.rows[0]!["total"]).toBe(3);
  });

  it("query against different store/table", async () => {
    const result = await storage.query("SELECT * FROM posts");
    expect(result.rows).toHaveLength(2);
    expect(result.columns).toContain("title");
  });

  it("throws for unknown table", async () => {
    await expect(storage.query("SELECT * FROM nonexistent")).rejects.toThrow();
  });

  it("query on empty store returns no rows", async () => {
    const emptyStorage = new MemoryStorage([
      { name: "empty", keyPath: "id" },
    ]);
    await emptyStorage.open();
    // An empty store has no records, so schema inference returns just the keyPath.
    // SELECT * on an empty table should return 0 rows.
    const result = await emptyStorage.query("SELECT * FROM empty");
    expect(result.rows).toHaveLength(0);
  });
});

// ── 4. Transactions ────────────────────────────────────────────────────────

describe("Transactions", () => {
  let storage: Storage;

  beforeEach(async () => {
    storage = await createSeededStorage();
  });

  it("committed transaction persists changes", async () => {
    await storage.transaction(async (tx) => {
      await tx.put("users", { id: "4", name: "Dave", age: 40 });
      await tx.delete("users", "1");
    });

    const users = await storage.getAll("users");
    expect(users).toHaveLength(3); // was 3, added 1, deleted 1 = 3
    const ids = users.map((u: { id: string }) => u.id);
    expect(ids).toContain("4");
    expect(ids).not.toContain("1");
  });

  it("failed transaction rolls back all changes", async () => {
    const usersBefore = await storage.getAll("users");

    await expect(
      storage.transaction(async (tx) => {
        await tx.put("users", { id: "4", name: "Dave", age: 40 });
        await tx.delete("users", "1");
        throw new Error("Simulated failure");
      }),
    ).rejects.toThrow("Simulated failure");

    // All changes should be rolled back
    const usersAfter = await storage.getAll("users");
    expect(usersAfter).toHaveLength(usersBefore.length);
    const ids = usersAfter.map((u: { id: string }) => u.id);
    expect(ids).toContain("1"); // not deleted
    expect(ids).not.toContain("4"); // not added
  });

  it("transaction returns the callback result on success", async () => {
    const result = await storage.transaction(async (tx) => {
      await tx.put("users", { id: "4", name: "Dave", age: 40 });
      return "done";
    });
    expect(result).toBe("done");
  });

  it("transaction rollback restores exact previous state", async () => {
    // Modify a record, then roll back
    const aliceBefore = await storage.get("users", "1");

    await expect(
      storage.transaction(async (tx) => {
        await tx.put("users", { id: "1", name: "Alice Modified", age: 99 });
        throw new Error("rollback");
      }),
    ).rejects.toThrow("rollback");

    const aliceAfter = await storage.get("users", "1");
    expect(aliceAfter).toEqual(aliceBefore);
  });

  it("multiple stores are rolled back together", async () => {
    await expect(
      storage.transaction(async (tx) => {
        await tx.put("users", { id: "5", name: "Eve", age: 28 });
        await tx.put("posts", { id: "p3", userId: "5", title: "Eve's Post" });
        throw new Error("rollback");
      }),
    ).rejects.toThrow("rollback");

    const users = await storage.getAll("users");
    const posts = await storage.getAll("posts");
    expect(users).toHaveLength(3); // unchanged
    expect(posts).toHaveLength(2); // unchanged
  });
});
