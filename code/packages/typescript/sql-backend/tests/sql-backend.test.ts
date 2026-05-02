import { describe, expect, test } from "vitest";
import {
  Backend,
  ColumnAlreadyExists,
  ColumnDef,
  ColumnNotFound,
  ConstraintViolation,
  IndexAlreadyExists,
  IndexDef,
  IndexNotFound,
  InMemoryBackend,
  ListCursor,
  ListRowIterator,
  Row,
  TableAlreadyExists,
  TableNotFound,
  TriggerAlreadyExists,
  TriggerDef,
  TriggerNotFound,
  Unsupported,
  backendAsSchemaProvider,
  compareSqlValues,
  isSqlValue,
  sqlTypeName,
} from "../src/index.js";

describe("SQL values", () => {
  test("classifies and compares portable SQL values", () => {
    expect(sqlTypeName(null)).toBe("NULL");
    expect(sqlTypeName(false)).toBe("BOOLEAN");
    expect(sqlTypeName(42)).toBe("INTEGER");
    expect(sqlTypeName(3.5)).toBe("REAL");
    expect(sqlTypeName("hi")).toBe("TEXT");
    expect(sqlTypeName(new Uint8Array([1, 2]))).toBe("BLOB");
    expect(isSqlValue({})).toBe(false);
    expect(isSqlValue(Number.NaN)).toBe(false);
    expect(compareSqlValues(null, 0)).toBeLessThan(0);
    expect(compareSqlValues(true, 0)).toBeLessThan(0);
    expect(compareSqlValues(2, "2")).toBeLessThan(0);
    expect(compareSqlValues(new Uint8Array([1]), new Uint8Array([1, 2]))).toBeLessThan(0);
  });
});

describe("row iterators", () => {
  test("return row copies and track cursor state", () => {
    const blob = new Uint8Array([1, 2]);
    const iterator = new ListRowIterator([{ id: 1, blob }]);
    const first = iterator.next();
    expect(first?.id).toBe(1);
    (first?.blob as Uint8Array)[0] = 9;
    expect(blob[0]).toBe(1);
    expect(iterator.next()).toBeNull();
    iterator.close();
    expect(iterator.next()).toBeNull();

    const rows: Row[] = [{ id: 1 }, { id: 2 }];
    const cursor = new ListCursor(rows);
    expect(cursor.currentRow()).toBeNull();
    expect(cursor.next()?.id).toBe(1);
    const current = cursor.currentRow();
    expect(current?.id).toBe(1);
    current!.id = 100;
    expect(rows[0].id).toBe(1);
    cursor.close();
    expect(cursor.next()).toBeNull();
  });
});

describe("schema metadata", () => {
  test("exposes column defaults and schema provider adapter", () => {
    const pk = new ColumnDef("id", "INTEGER", { primaryKey: true });
    const unique = new ColumnDef("email", "TEXT", { unique: true });
    const nullDefault = ColumnDef.withDefault("middle", "TEXT", null);

    expect(pk.effectiveNotNull).toBe(true);
    expect(pk.effectiveUnique).toBe(true);
    expect(unique.effectiveNotNull).toBe(false);
    expect(unique.effectiveUnique).toBe(true);
    expect(nullDefault.hasDefault).toBe(true);
    expect(nullDefault.defaultValue).toBeNull();

    const schema = backendAsSchemaProvider(users());
    expect(schema.columns("users")).toEqual(["id", "name", "age", "email"]);
    expect(() => schema.columns("missing")).toThrow(TableNotFound);
  });
});

describe("InMemoryBackend", () => {
  test("lists tables, columns, and scanned rows", () => {
    const backend = users();
    expect(backend.tables()).toEqual(["users"]);
    expect(backend.columns("USERS")[1].name).toBe("name");
    expect(collect(backend.scan("users")).map((row) => row.id)).toEqual([1, 2, 3]);
    expect(() => backend.columns("missing")).toThrow(TableNotFound);
  });

  test("constructs fixtures directly with fromTables", () => {
    const backend = InMemoryBackend.fromTables({
      logs: {
        columns: [new ColumnDef("id", "INTEGER")],
        rows: [{ id: 1 }],
      },
    });
    expect(collect(backend.scan("logs"))).toEqual([{ id: 1 }]);
  });

  test("inserts rows with defaults and validates unknown columns", () => {
    const backend = new InMemoryBackend();
    backend.createTable(
      "items",
      [
        new ColumnDef("id", "INTEGER", { primaryKey: true }),
        ColumnDef.withDefault("status", "TEXT", "active"),
      ],
      false
    );

    backend.insert("items", { id: 1 });
    expect(collect(backend.scan("items"))[0].status).toBe("active");
    expect(() => backend.insert("items", { id: 2, ghost: "x" })).toThrow(ColumnNotFound);
    expect(() => backend.insert("items", { id: Number.NaN as never })).toThrow(TypeError);
  });

  test("enforces primary key, not null, and unique constraints", () => {
    const backend = users();
    expect(() =>
      backend.insert("users", {
        id: 1,
        name: "Dup",
        age: 9,
        email: "dup@example.com",
      })
    ).toThrow(ConstraintViolation);
    expect(() =>
      backend.insert("users", {
        id: 4,
        name: null,
        age: 9,
        email: "dup@example.com",
      })
    ).toThrow(ConstraintViolation);
    expect(() =>
      backend.insert("users", {
        id: 4,
        name: "Dup",
        age: 9,
        email: "alice@example.com",
      })
    ).toThrow(ConstraintViolation);
  });

  test("allows multiple nulls in unique columns", () => {
    const backend = new InMemoryBackend();
    backend.createTable(
      "users",
      [
        new ColumnDef("id", "INTEGER", { primaryKey: true }),
        new ColumnDef("email", "TEXT", { unique: true }),
      ],
      false
    );
    backend.insert("users", { id: 1, email: null });
    backend.insert("users", { id: 2, email: null });
    expect(collect(backend.scan("users"))).toHaveLength(2);
  });

  test("updates and deletes positioned cursor rows", () => {
    const backend = users();
    const cursor = backend.openCursor("users");
    expect(cursor.next()?.id).toBe(1);

    backend.update("users", cursor, { NAME: "ALICE" });
    expect(backend.openCursor("users").next()?.name).toBe("ALICE");
    expect(() => backend.update("users", cursor, { missing: "x" })).toThrow(ColumnNotFound);

    backend.delete("users", cursor);
    expect(backend.openCursor("users").next()?.id).toBe(2);
    expect(() => backend.update("users", cursor, { name: "x" })).toThrow(Unsupported);
    expect(() => backend.delete("users", new ListCursor([]))).toThrow(Unsupported);
  });

  test("creates, drops, and alters tables", () => {
    const backend = new InMemoryBackend();
    backend.createTable("t", [new ColumnDef("id", "INTEGER")], false);
    backend.createTable("T", [], true);
    expect(() => backend.createTable("t", [], false)).toThrow(TableAlreadyExists);
    expect(() =>
      backend.createTable(
        "dupe",
        [new ColumnDef("id", "INTEGER"), new ColumnDef("ID", "INTEGER")],
        false
      )
    ).toThrow(ColumnAlreadyExists);

    backend.insert("t", { id: 1 });
    backend.addColumn("t", ColumnDef.withDefault("status", "TEXT", "new"));
    expect(collect(backend.scan("t"))[0].status).toBe("new");
    expect(() => backend.addColumn("t", new ColumnDef("status", "TEXT"))).toThrow(
      ColumnAlreadyExists
    );
    expect(() => backend.addColumn("t", new ColumnDef("required", "TEXT", { notNull: true }))).toThrow(
      ConstraintViolation
    );

    backend.dropTable("t", false);
    backend.dropTable("t", true);
    expect(() => backend.dropTable("t", false)).toThrow(TableNotFound);
  });

  test("commits, rolls back, and rejects stale transaction handles", () => {
    const backend = users();
    const handle = backend.beginTransaction();
    backend.insert("users", { id: 4, name: "Dave", age: 41, email: "dave@example.com" });
    backend.rollback(handle);
    expect(collect(backend.scan("users")).some((row) => row.id === 4)).toBe(false);

    const committed = backend.beginTransaction();
    backend.insert("users", { id: 4, name: "Dave", age: 41, email: "dave@example.com" });
    backend.commit(committed);
    expect(collect(backend.scan("users")).some((row) => row.id === 4)).toBe(true);

    const active = backend.beginTransaction();
    expect(backend.currentTransaction()).toEqual(active);
    expect(() => backend.beginTransaction()).toThrow(Unsupported);
    backend.commit(active);
    expect(() => backend.commit(active)).toThrow(Unsupported);
  });

  test("supports savepoints inside an active transaction", () => {
    const backend = users();
    const handle = backend.beginTransaction();
    backend.createSavepoint("s1");
    backend.insert("users", { id: 4, name: "Dave", age: 41, email: "dave@example.com" });
    backend.rollbackToSavepoint("s1");
    expect(collect(backend.scan("users")).some((row) => row.id === 4)).toBe(false);
    backend.releaseSavepoint("s1");
    expect(() => backend.releaseSavepoint("s1")).toThrow(Unsupported);
    backend.commit(handle);
  });

  test("creates an implicit transaction for savepoints", () => {
    const backend = users();
    backend.createSavepoint("implicit");
    expect(backend.currentTransaction()).not.toBeNull();
    const handle = backend.currentTransaction()!;
    backend.rollback(handle);
    expect(backend.currentTransaction()).toBeNull();
  });

  test("lists, scans, and drops indexes", () => {
    const backend = users();
    backend.createIndex(new IndexDef("idx_age", "users", { columns: ["age"] }));

    expect(backend.listIndexes("users")[0].name).toBe("idx_age");
    const rowids = Array.from(backend.scanIndex("idx_age", [25], [30]));
    expect(rowids).toEqual([1, 0]);
    expect(collect(backend.scanByRowids("users", rowids)).map((row) => row.id)).toEqual([2, 1]);

    backend.dropIndex("idx_age");
    expect(backend.listIndexes()).toEqual([]);
    expect(() => backend.dropIndex("idx_age")).toThrow(IndexNotFound);
    backend.dropIndex("idx_age", { ifExists: true });
  });

  test("validates index inputs", () => {
    const backend = users();
    backend.createIndex(new IndexDef("idx_email", "users", { columns: ["email"], unique: true }));
    expect(() =>
      backend.createIndex(new IndexDef("IDX_EMAIL", "users", { columns: ["email"] }))
    ).toThrow(IndexAlreadyExists);
    expect(() =>
      backend.createIndex(new IndexDef("idx_missing", "missing", { columns: ["id"] }))
    ).toThrow(TableNotFound);
    expect(() =>
      backend.createIndex(new IndexDef("idx_bad", "users", { columns: ["missing"] }))
    ).toThrow(ColumnNotFound);
    expect(() => Array.from(backend.scanIndex("missing", null, null))).toThrow(IndexNotFound);
  });

  test("stores triggers and version fields", () => {
    const backend = users();
    const initialSchemaVersion = backend.getSchemaVersion();
    backend.setUserVersion(7);
    expect(backend.getUserVersion()).toBe(7);
    expect(() => backend.setUserVersion(-1)).toThrow(RangeError);
    expect(initialSchemaVersion).toBeGreaterThan(0);

    const trigger = new TriggerDef("tr_users_ai", "users", "AFTER", "INSERT", "SELECT 1");
    backend.createTrigger(trigger);
    expect(backend.listTriggers("USERS")).toEqual([trigger]);
    expect(() => backend.createTrigger(trigger)).toThrow(TriggerAlreadyExists);
    backend.dropTrigger("TR_USERS_AI");
    expect(backend.listTriggers("users")).toEqual([]);
    expect(() => backend.dropTrigger("tr_users_ai")).toThrow(TriggerNotFound);
    backend.dropTrigger("tr_users_ai", { ifExists: true });
  });

  test("drops table-owned indexes and triggers", () => {
    const backend = users();
    backend.createIndex(new IndexDef("idx_age", "users", { columns: ["age"] }));
    backend.createTrigger(new TriggerDef("tr_users_ai", "users", "AFTER", "INSERT", "SELECT 1"));
    backend.dropTable("users", false);
    expect(backend.listIndexes()).toEqual([]);
    expect(backend.listTriggers("users")).toEqual([]);
  });
});

describe("Backend base defaults", () => {
  test("reports unsupported optional operations", () => {
    const backend = new ReadOnlyBackend();
    expect(backend.currentTransaction()).toBeNull();
    expect(() => backend.createSavepoint("s")).toThrow(Unsupported);
    expect(() => backend.createTrigger(new TriggerDef("t", "x", "AFTER", "INSERT", ""))).toThrow(
      Unsupported
    );
    expect(backend.listTriggers("x")).toEqual([]);
    expect(backend.getUserVersion()).toBe(0);
    expect(() => backend.setUserVersion(1)).toThrow(Unsupported);
  });
});

function users(): InMemoryBackend {
  const backend = new InMemoryBackend();
  backend.createTable(
    "users",
    [
      new ColumnDef("id", "INTEGER", { primaryKey: true }),
      new ColumnDef("name", "TEXT", { notNull: true }),
      new ColumnDef("age", "INTEGER"),
      new ColumnDef("email", "TEXT", { unique: true }),
    ],
    false
  );
  backend.insert("users", { id: 1, name: "Alice", age: 30, email: "alice@example.com" });
  backend.insert("users", { id: 2, name: "Bob", age: 25, email: "bob@example.com" });
  backend.insert("users", { id: 3, name: "Carol", age: null, email: null });
  return backend;
}

function collect(iterator: { next(): Row | null; close(): void }): Row[] {
  const rows: Row[] = [];
  try {
    for (;;) {
      const row = iterator.next();
      if (row === null) {
        break;
      }
      rows.push(row);
    }
  } finally {
    iterator.close();
  }
  return rows;
}

class ReadOnlyBackend extends Backend {
  tables(): string[] {
    return [];
  }

  columns(_table: string): ColumnDef[] {
    return [];
  }

  scan(_table: string): ListRowIterator {
    return new ListRowIterator([]);
  }

  insert(_table: string, _row: Row): void {
    throw new Unsupported("insert");
  }

  update(_table: string): void {
    throw new Unsupported("update");
  }

  delete(_table: string): void {
    throw new Unsupported("delete");
  }

  createTable(_table: string): void {
    throw new Unsupported("create table");
  }

  dropTable(_table: string): void {
    throw new Unsupported("drop table");
  }

  addColumn(_table: string): void {
    throw new Unsupported("add column");
  }

  createIndex(_index: IndexDef): void {
    throw new Unsupported("create index");
  }

  dropIndex(_name: string): void {
    throw new Unsupported("drop index");
  }

  listIndexes(): IndexDef[] {
    return [];
  }

  scanIndex(): Iterable<number> {
    throw new Unsupported("scan index");
  }

  scanByRowids(): ListRowIterator {
    return new ListRowIterator([]);
  }

  beginTransaction(): never {
    throw new Unsupported("transactions");
  }

  commit(): void {
    throw new Unsupported("transactions");
  }

  rollback(): void {
    throw new Unsupported("transactions");
  }
}
