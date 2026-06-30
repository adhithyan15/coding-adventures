# mini-sqlite-wasm

WebAssembly bindings for the Rust mini-sqlite Level 0 facade.

This package exposes the `coding-adventures-mini-sqlite` in-memory SQL engine
to JavaScript (and any other Wasm host) via `wasm-bindgen`.

## What it does

- Opens an in-memory database (`:memory:` only — Level 0 has no persistence).
- Executes CREATE TABLE, DROP TABLE, INSERT, UPDATE, DELETE, SELECT.
- Supports `?`-style positional parameter binding.
- Provides cursor-style fetch methods (`fetchone`, `fetchmany`, `fetchall`).
- Implements snapshot-based `commit` / `rollback`.

## Where it fits in the stack

```
JavaScript / TypeScript
      │
      │  JSON strings (params + results)
      ▼
mini-sqlite-wasm                    ← you are here
  code/packages/wasm/mini-sqlite/
      │
      │  &[SqlValue]  /  Vec<Vec<SqlValue>>
      ▼
coding-adventures-mini-sqlite
  code/packages/rust/mini-sqlite/
      │
      ▼
coding-adventures-sql-execution-engine
  code/packages/rust/sql-execution-engine/
```

## Usage

### JavaScript / TypeScript

```js
import init, { Connection, open } from "mini-sqlite-wasm";

await init(); // load .wasm binary

// Constructor form
const db = new Connection();

// Free-function form (equivalent)
const db2 = open(":memory:");

// DDL / DML
db.execute("CREATE TABLE users (id, name, age)");
db.execute("INSERT INTO users VALUES (?, ?, ?)", JSON.stringify([1, "Alice", 30]));
db.execute("INSERT INTO users VALUES (?, ?, ?)", JSON.stringify([2, "Bob",   25]));

// Batch INSERT
db.executemany(
  "INSERT INTO users VALUES (?, ?, ?)",
  JSON.stringify([[3, "Charlie", 35], [4, "Diana", 28]])
);

// SELECT — returns JSON
const result = JSON.parse(db.query("SELECT * FROM users ORDER BY id"));
// {
//   columns: ["id", "name", "age"],
//   rows: [[1, "Alice", 30], [2, "Bob", 25], [3, "Charlie", 35], [4, "Diana", 28]]
// }

// SELECT with parameter binding
const filtered = JSON.parse(
  db.query("SELECT name FROM users WHERE age > ?", JSON.stringify([28]))
);

// Cursor-style fetch
db.execute_for_fetch("SELECT name FROM users ORDER BY id");
const first  = JSON.parse(db.fetchone()); // ["Alice"]
const second = JSON.parse(db.fetchone()); // ["Bob"]
const rest   = JSON.parse(db.fetchall()); // [["Charlie"], ["Diana"]]

// fetchmany
db.execute_for_fetch("SELECT id FROM users ORDER BY id");
const batch1 = JSON.parse(db.fetchmany(2)); // [[1], [2]]
const batch2 = JSON.parse(db.fetchmany(2)); // [[3], [4]]

// Transactions
db.execute("BEGIN");
db.execute("DELETE FROM users WHERE id = ?", JSON.stringify([1]));
db.rollback();  // Alice is back

db.execute("BEGIN");
db.execute("UPDATE users SET age = ? WHERE id = ?", JSON.stringify([31, 1]));
db.commit();    // change is permanent (in-memory only at Level 0)

// executemany
db.executemany(
  "INSERT INTO users VALUES (?, ?, ?)",
  JSON.stringify([[5, "Eve", 22]])
);
```

### Error handling

Errors are thrown as plain strings with a type prefix:

```js
try {
  db.execute("INSERT INTO t VALUES (?, ?)", JSON.stringify([1])); // wrong count
} catch (e) {
  // e === "ProgrammingError: parameter count mismatch: expected 2, got 1"
  if (e.startsWith("ProgrammingError")) { /* client-side mistake */ }
  if (e.startsWith("OperationalError")) { /* SQL runtime error  */ }
  if (e.startsWith("NotSupportedError")) { /* Level 0 limitation */ }
}

// File-backed connections are not supported at Level 0
try {
  open("app.db");
} catch (e) {
  // e starts with "NotSupportedError: …"
}
```

## Conformance

The conformance test suite lives at `code/specs/mini-sqlite-conformance/`.
A runner for this package will load `manifest.json`, execute each fixture's
`steps` array using the methods above, and report pass/fail.

## Building

This package is a Rust crate and requires the Wasm toolchain:

```sh
# Install the Wasm target and wasm-bindgen CLI if not already present
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli

# Build the Wasm binary and JS bindings
cd code/packages/wasm/mini-sqlite
wasm-pack build --target web --out-dir pkg
```

### Native tests (no Wasm toolchain required)

```sh
cargo test -- --nocapture
```

Native tests gate `JsValue` creation behind `#[cfg(not(target_arch = "wasm32"))]`
so the test suite runs on the host target without a browser or `wasm-bindgen-test`.

## API reference

| Method / function | Description |
|---|---|
| `new Connection()` | Open `:memory:` database |
| `open(database)` | Open database by path (`:memory:` only at Level 0) |
| `execute(sql, params?)` | DDL / DML; `params` is optional JSON array string |
| `executemany(sql, param_seq)` | Batch DML; `param_seq` is JSON array-of-arrays |
| `query(sql, params?)` | SELECT → `{"columns":[…],"rows":[[…],…]}` JSON string |
| `execute_for_fetch(sql, params?)` | SELECT → populates cursor buffer (no return value) |
| `fetchone()` | Next buffered row as JSON array string, or `null` |
| `fetchmany(size)` | Next `size` buffered rows as JSON array-of-arrays string |
| `fetchall()` | All remaining buffered rows as JSON array-of-arrays string |
| `commit()` | Commit transaction (discard rollback snapshot) |
| `rollback()` | Rollback transaction (restore snapshot) |
