import { describe, expect, test } from "vitest";
import {
  NotSupportedError,
  OperationalError,
  ProgrammingError,
  apilevel,
  connect,
  paramstyle,
  threadsafety,
} from "../src/index.js";

describe("module metadata", () => {
  test("exposes DB-API inspired constants", () => {
    expect(apilevel).toBe("2.0");
    expect(threadsafety).toBe(1);
    expect(paramstyle).toBe("qmark");
  });
});

describe("in-memory database", () => {
  test("creates a table, inserts rows, and selects them", () => {
    const conn = connect(":memory:");
    conn.execute("CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)");
    conn.executemany("INSERT INTO users VALUES (?, ?, ?)", [
      [1, "Alice", true],
      [2, "Bob", false],
      [3, "Carol", true],
    ]);

    const cur = conn.execute(
      "SELECT name FROM users WHERE active = ? ORDER BY id ASC",
      [true],
    );

    expect(cur.description?.[0][0]).toBe("name");
    expect(cur.fetchall()).toEqual([["Alice"], ["Carol"]]);
  });

  test("fetchone and fetchmany consume the cursor", () => {
    const conn = connect(":memory:");
    conn.execute("CREATE TABLE nums (n INTEGER)");
    conn.executemany("INSERT INTO nums VALUES (?)", [[1], [2], [3]]);

    const cur = conn.execute("SELECT n FROM nums ORDER BY n ASC");

    expect(cur.fetchone()).toEqual([1]);
    expect(cur.fetchmany(1)).toEqual([[2]]);
    expect(cur.fetchall()).toEqual([[3]]);
    expect(cur.fetchone()).toBeNull();
  });

  test("updates rows selected by a WHERE predicate", () => {
    const conn = connect(":memory:");
    conn.execute("CREATE TABLE users (id INTEGER, name TEXT)");
    conn.executemany("INSERT INTO users VALUES (?, ?)", [
      [1, "Alice"],
      [2, "Bob"],
    ]);

    const cur = conn.execute("UPDATE users SET name = ? WHERE id = ?", ["Bobby", 2]);

    expect(cur.rowcount).toBe(1);
    expect(conn.execute("SELECT name FROM users ORDER BY id").fetchall()).toEqual([
      ["Alice"],
      ["Bobby"],
    ]);
  });

  test("deletes rows selected by a WHERE predicate", () => {
    const conn = connect(":memory:");
    conn.execute("CREATE TABLE users (id INTEGER, name TEXT)");
    conn.executemany("INSERT INTO users VALUES (?, ?)", [
      [1, "Alice"],
      [2, "Bob"],
      [3, "Carol"],
    ]);

    const cur = conn.execute("DELETE FROM users WHERE id IN (?, ?)", [1, 3]);

    expect(cur.rowcount).toBe(2);
    expect(conn.execute("SELECT id, name FROM users").fetchall()).toEqual([[2, "Bob"]]);
  });

  test("rolls back to the transaction snapshot", () => {
    const conn = connect(":memory:");
    conn.execute("CREATE TABLE users (id INTEGER, name TEXT)");
    conn.commit();

    conn.execute("INSERT INTO users VALUES (?, ?)", [1, "Alice"]);
    conn.rollback();

    expect(conn.execute("SELECT * FROM users").fetchall()).toEqual([]);
  });

  test("commit keeps changes visible", () => {
    const conn = connect(":memory:");
    conn.execute("CREATE TABLE users (id INTEGER, name TEXT)");
    conn.execute("INSERT INTO users VALUES (?, ?)", [1, "Alice"]);
    conn.commit();
    conn.rollback();

    expect(conn.execute("SELECT name FROM users").fetchall()).toEqual([["Alice"]]);
  });

  test("drops tables", () => {
    const conn = connect(":memory:");
    conn.execute("CREATE TABLE users (id INTEGER)");
    conn.execute("DROP TABLE users");

    expect(() => conn.execute("SELECT * FROM users")).toThrow(OperationalError);
  });
});

describe("errors", () => {
  test("rejects wrong parameter counts", () => {
    const conn = connect(":memory:");
    expect(() => conn.execute("SELECT ? FROM t", [])).toThrow(ProgrammingError);
    expect(() => conn.execute("SELECT 1 FROM t", [1])).toThrow(ProgrammingError);
  });

  test("rejects unknown tables as operational errors", () => {
    const conn = connect(":memory:");
    expect(() => conn.execute("SELECT * FROM missing")).toThrow(OperationalError);
  });

  test("rejects file-backed connections in Level 0", () => {
    expect(() => connect("app.db")).toThrow(NotSupportedError);
  });
});
