# mini-sqlite (Go)

Go Level 0 port of the Python `mini-sqlite` facade.

The package provides an in-memory database facade with qmark binding, basic
DDL/DML, cursors, and transaction snapshots. SELECT queries are delegated to
the existing Go `sql-execution-engine` package.

```go
conn, err := minisqlite.Connect(":memory:")
if err != nil {
    panic(err)
}

_, _ = conn.Execute("CREATE TABLE users (id INTEGER, name TEXT)")
_, _ = conn.Executemany("INSERT INTO users VALUES (?, ?)", [][]any{
    {int64(1), "Alice"},
    {int64(2), "Bob"},
})

cur, _ := conn.Execute("SELECT name FROM users WHERE id = ?", int64(1))
rows := cur.Fetchall()
fmt.Println(rows) // [[Alice]]
```

File-backed connections are reserved for a later storage backend port and
currently return `NotSupportedError`.
