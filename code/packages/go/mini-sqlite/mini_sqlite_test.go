package minisqlite

import (
	"errors"
	"reflect"
	"testing"
)

func mustConnect(t *testing.T) *Connection {
	t.Helper()
	conn, err := Connect(":memory:")
	if err != nil {
		t.Fatalf("Connect failed: %v", err)
	}
	return conn
}

func mustExecute(t *testing.T, conn *Connection, sql string, params ...any) *Cursor {
	t.Helper()
	cur, err := conn.Execute(sql, params...)
	if err != nil {
		t.Fatalf("Execute(%q) failed: %v", sql, err)
	}
	return cur
}

func TestModuleConstants(t *testing.T) {
	if APILevel != "2.0" {
		t.Fatalf("APILevel = %q", APILevel)
	}
	if ThreadSafety != 1 {
		t.Fatalf("ThreadSafety = %d", ThreadSafety)
	}
	if ParamStyle != "qmark" {
		t.Fatalf("ParamStyle = %q", ParamStyle)
	}
}

func TestCreateInsertSelect(t *testing.T) {
	conn := mustConnect(t)
	mustExecute(t, conn, "CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)")
	_, err := conn.Executemany("INSERT INTO users VALUES (?, ?, ?)", [][]any{
		{int64(1), "Alice", true},
		{int64(2), "Bob", false},
		{int64(3), "Carol", true},
	})
	if err != nil {
		t.Fatalf("Executemany failed: %v", err)
	}

	cur := mustExecute(t, conn, "SELECT name FROM users WHERE active = ? ORDER BY id ASC", true)
	if len(cur.Description) != 1 || cur.Description[0].Name != "name" {
		t.Fatalf("description = %#v", cur.Description)
	}
	want := [][]any{{"Alice"}, {"Carol"}}
	if got := cur.Fetchall(); !reflect.DeepEqual(got, want) {
		t.Fatalf("rows = %#v, want %#v", got, want)
	}
}

func TestFetchoneFetchmany(t *testing.T) {
	conn := mustConnect(t)
	mustExecute(t, conn, "CREATE TABLE nums (n INTEGER)")
	_, err := conn.Executemany("INSERT INTO nums VALUES (?)", [][]any{{int64(1)}, {int64(2)}, {int64(3)}})
	if err != nil {
		t.Fatalf("Executemany failed: %v", err)
	}
	cur := mustExecute(t, conn, "SELECT n FROM nums ORDER BY n ASC")

	row, ok := cur.Fetchone()
	if !ok || !reflect.DeepEqual(row, []any{int64(1)}) {
		t.Fatalf("Fetchone = %#v, %v", row, ok)
	}
	if got := cur.Fetchmany(1); !reflect.DeepEqual(got, [][]any{{int64(2)}}) {
		t.Fatalf("Fetchmany = %#v", got)
	}
	if got := cur.Fetchall(); !reflect.DeepEqual(got, [][]any{{int64(3)}}) {
		t.Fatalf("Fetchall = %#v", got)
	}
	if _, ok := cur.Fetchone(); ok {
		t.Fatal("Fetchone after exhaustion returned a row")
	}
}

func TestUpdateRows(t *testing.T) {
	conn := mustConnect(t)
	mustExecute(t, conn, "CREATE TABLE users (id INTEGER, name TEXT)")
	_, _ = conn.Executemany("INSERT INTO users VALUES (?, ?)", [][]any{{int64(1), "Alice"}, {int64(2), "Bob"}})

	cur := mustExecute(t, conn, "UPDATE users SET name = ? WHERE id = ?", "Bobby", int64(2))
	if cur.Rowcount() != 1 {
		t.Fatalf("rowcount = %d", cur.Rowcount())
	}
	want := [][]any{{"Alice"}, {"Bobby"}}
	if got := mustExecute(t, conn, "SELECT name FROM users ORDER BY id").Fetchall(); !reflect.DeepEqual(got, want) {
		t.Fatalf("rows = %#v, want %#v", got, want)
	}
}

func TestDeleteRows(t *testing.T) {
	conn := mustConnect(t)
	mustExecute(t, conn, "CREATE TABLE users (id INTEGER, name TEXT)")
	_, _ = conn.Executemany("INSERT INTO users VALUES (?, ?)", [][]any{
		{int64(1), "Alice"},
		{int64(2), "Bob"},
		{int64(3), "Carol"},
	})

	cur := mustExecute(t, conn, "DELETE FROM users WHERE id IN (?, ?)", int64(1), int64(3))
	if cur.Rowcount() != 2 {
		t.Fatalf("rowcount = %d", cur.Rowcount())
	}
	want := [][]any{{int64(2), "Bob"}}
	if got := mustExecute(t, conn, "SELECT id, name FROM users").Fetchall(); !reflect.DeepEqual(got, want) {
		t.Fatalf("rows = %#v, want %#v", got, want)
	}
}

func TestRollbackRestoresSnapshot(t *testing.T) {
	conn := mustConnect(t)
	mustExecute(t, conn, "CREATE TABLE users (id INTEGER, name TEXT)")
	if err := conn.Commit(); err != nil {
		t.Fatalf("Commit failed: %v", err)
	}
	mustExecute(t, conn, "INSERT INTO users VALUES (?, ?)", int64(1), "Alice")
	if err := conn.Rollback(); err != nil {
		t.Fatalf("Rollback failed: %v", err)
	}
	if got := mustExecute(t, conn, "SELECT * FROM users").Fetchall(); len(got) != 0 {
		t.Fatalf("rows after rollback = %#v", got)
	}
}

func TestCommitKeepsChanges(t *testing.T) {
	conn := mustConnect(t)
	mustExecute(t, conn, "CREATE TABLE users (id INTEGER, name TEXT)")
	mustExecute(t, conn, "INSERT INTO users VALUES (?, ?)", int64(1), "Alice")
	if err := conn.Commit(); err != nil {
		t.Fatalf("Commit failed: %v", err)
	}
	if err := conn.Rollback(); err != nil {
		t.Fatalf("Rollback failed: %v", err)
	}
	want := [][]any{{"Alice"}}
	if got := mustExecute(t, conn, "SELECT name FROM users").Fetchall(); !reflect.DeepEqual(got, want) {
		t.Fatalf("rows = %#v, want %#v", got, want)
	}
}

func TestDropTable(t *testing.T) {
	conn := mustConnect(t)
	mustExecute(t, conn, "CREATE TABLE users (id INTEGER)")
	mustExecute(t, conn, "DROP TABLE users")

	_, err := conn.Execute("SELECT * FROM users")
	var opErr *OperationalError
	if !errors.As(err, &opErr) {
		t.Fatalf("error = %T %v, want OperationalError", err, err)
	}
}

func TestWrongParameterCounts(t *testing.T) {
	conn := mustConnect(t)
	_, err := conn.Execute("SELECT ? FROM t")
	var progErr *ProgrammingError
	if !errors.As(err, &progErr) {
		t.Fatalf("not enough params error = %T %v", err, err)
	}
	_, err = conn.Execute("SELECT 1 FROM t", int64(1))
	if !errors.As(err, &progErr) {
		t.Fatalf("too many params error = %T %v", err, err)
	}
}

func TestUnsupportedFileConnection(t *testing.T) {
	_, err := Connect("app.db")
	var notSupported *NotSupportedError
	if !errors.As(err, &notSupported) {
		t.Fatalf("error = %T %v, want NotSupportedError", err, err)
	}
}
