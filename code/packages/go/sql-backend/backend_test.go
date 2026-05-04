package sqlbackend

import (
	"errors"
	"math"
	"reflect"
	"testing"
)

func TestSQLValues(t *testing.T) {
	cases := []struct {
		value any
		name  string
	}{
		{nil, "NULL"},
		{true, "BOOLEAN"},
		{42, "INTEGER"},
		{int64(42), "INTEGER"},
		{3.5, "REAL"},
		{"hi", "TEXT"},
		{[]byte{1, 2}, "BLOB"},
	}
	for _, tc := range cases {
		if !IsSQLValue(tc.value) {
			t.Fatalf("IsSQLValue(%#v) = false", tc.value)
		}
		got, err := SQLTypeName(tc.value)
		if err != nil {
			t.Fatalf("SQLTypeName(%#v) failed: %v", tc.value, err)
		}
		if got != tc.name {
			t.Fatalf("SQLTypeName(%#v) = %q, want %q", tc.value, got, tc.name)
		}
	}
	if IsSQLValue(math.NaN()) {
		t.Fatal("NaN should not be a SQL value")
	}
	if _, err := SQLTypeName(struct{}{}); err == nil {
		t.Fatal("SQLTypeName accepted non-SQL value")
	}
	assertCompareLess(t, nil, int64(0))
	assertCompareLess(t, true, int64(0))
	assertCompareLess(t, int64(2), "2")
	assertCompareLess(t, []byte{1}, []byte{1, 2})
}

func TestIteratorsReturnCopies(t *testing.T) {
	blob := []byte{1, 2}
	it := NewListRowIterator([]Row{{"id": int64(1), "blob": blob}})
	row, ok := it.Next()
	if !ok || row["id"] != int64(1) {
		t.Fatalf("Next = %#v, %v", row, ok)
	}
	row["blob"].([]byte)[0] = 9
	if blob[0] != 1 {
		t.Fatalf("iterator leaked blob mutation, got %d", blob[0])
	}
	if _, ok := it.Next(); ok {
		t.Fatal("iterator returned extra row")
	}
	it.Close()
	if _, ok := it.Next(); ok {
		t.Fatal("closed iterator returned row")
	}

	rows := []Row{{"id": int64(1)}, {"id": int64(2)}}
	cursor := NewListCursor(rows)
	if _, ok := cursor.CurrentRow(); ok {
		t.Fatal("fresh cursor has current row")
	}
	row, ok = cursor.Next()
	if !ok || row["id"] != int64(1) {
		t.Fatalf("cursor Next = %#v, %v", row, ok)
	}
	current, ok := cursor.CurrentRow()
	if !ok || current["id"] != int64(1) {
		t.Fatalf("CurrentRow = %#v, %v", current, ok)
	}
	current["id"] = int64(99)
	if rows[0]["id"] != int64(1) {
		t.Fatal("cursor current row was not copied")
	}
	cursor.Close()
	if _, ok := cursor.Next(); ok {
		t.Fatal("closed cursor returned row")
	}
}

func TestSchemaAdapter(t *testing.T) {
	backend := users(t)
	names, err := BackendAsSchemaProvider(backend).ColumnNames("users")
	if err != nil {
		t.Fatalf("ColumnNames failed: %v", err)
	}
	if !reflect.DeepEqual(names, []string{"id", "name", "age", "email"}) {
		t.Fatalf("ColumnNames = %#v", names)
	}
	if _, err := BackendAsSchemaProvider(backend).ColumnNames("missing"); !isErr[*TableNotFound](err) {
		t.Fatalf("ColumnNames missing error = %T %v", err, err)
	}
}

func TestTablesColumnsScanAndFixtures(t *testing.T) {
	backend := users(t)
	if got := backend.Tables(); !reflect.DeepEqual(got, []string{"users"}) {
		t.Fatalf("Tables = %#v", got)
	}
	columns, err := backend.Columns("USERS")
	if err != nil {
		t.Fatalf("Columns failed: %v", err)
	}
	if columns[1].Name != "name" {
		t.Fatalf("Columns[1] = %#v", columns[1])
	}
	rows := collect(t, mustScan(t, backend, "users"))
	if got := rowValues(rows, "id"); !reflect.DeepEqual(got, []any{int64(1), int64(2), int64(3)}) {
		t.Fatalf("scan ids = %#v", got)
	}

	fixture := NewInMemoryBackendFromTables(map[string]struct {
		Columns []ColumnDef
		Rows    []Row
	}{
		"logs": {
			Columns: []ColumnDef{{Name: "id", TypeName: "INTEGER"}},
			Rows:    []Row{{"id": int64(1)}},
		},
	})
	if got := collect(t, mustScan(t, fixture, "logs")); !reflect.DeepEqual(got, []Row{{"id": int64(1)}}) {
		t.Fatalf("fixture rows = %#v", got)
	}
}

func TestInsertDefaultsAndConstraints(t *testing.T) {
	backend := NewInMemoryBackend()
	err := backend.CreateTable("items", []ColumnDef{
		{Name: "id", TypeName: "INTEGER", PrimaryKey: true},
		{Name: "status", TypeName: "TEXT", DefaultValue: "active", HasDefault: true},
	}, false)
	if err != nil {
		t.Fatalf("CreateTable failed: %v", err)
	}
	if err := backend.Insert("items", Row{"id": int64(1)}); err != nil {
		t.Fatalf("Insert failed: %v", err)
	}
	if got := collect(t, mustScan(t, backend, "items"))[0]["status"]; got != "active" {
		t.Fatalf("default status = %#v", got)
	}
	if err := backend.Insert("items", Row{"id": int64(2), "ghost": "x"}); !isErr[*ColumnNotFound](err) {
		t.Fatalf("unknown column error = %T %v", err, err)
	}
	if err := backend.Insert("items", Row{"id": math.NaN()}); err == nil {
		t.Fatal("Insert accepted NaN")
	}

	backend = users(t)
	if err := backend.Insert("users", Row{"id": int64(1), "name": "Dup", "age": int64(9), "email": "dup@example.com"}); !isErr[*ConstraintViolation](err) {
		t.Fatalf("duplicate primary key error = %T %v", err, err)
	}
	if err := backend.Insert("users", Row{"id": int64(4), "name": nil, "age": int64(9), "email": "dup@example.com"}); !isErr[*ConstraintViolation](err) {
		t.Fatalf("not null error = %T %v", err, err)
	}
	if err := backend.Insert("users", Row{"id": int64(4), "name": "Dup", "age": int64(9), "email": "alice@example.com"}); !isErr[*ConstraintViolation](err) {
		t.Fatalf("unique error = %T %v", err, err)
	}
}

func TestUniqueAllowsMultipleNulls(t *testing.T) {
	backend := NewInMemoryBackend()
	must(t, backend.CreateTable("users", []ColumnDef{
		{Name: "id", TypeName: "INTEGER", PrimaryKey: true},
		{Name: "email", TypeName: "TEXT", Unique: true},
	}, false))
	must(t, backend.Insert("users", Row{"id": int64(1), "email": nil}))
	must(t, backend.Insert("users", Row{"id": int64(2), "email": nil}))
	rows := collect(t, mustScan(t, backend, "users"))
	if len(rows) != 2 {
		t.Fatalf("rows = %d", len(rows))
	}
}

func TestUpdateAndDeletePositionedRows(t *testing.T) {
	backend := users(t)
	cursor, err := backend.OpenCursor("users")
	if err != nil {
		t.Fatalf("OpenCursor failed: %v", err)
	}
	row, ok := cursor.Next()
	if !ok || row["id"] != int64(1) {
		t.Fatalf("cursor first row = %#v, %v", row, ok)
	}
	if err := backend.Update("users", cursor, Row{"NAME": "ALICE"}); err != nil {
		t.Fatalf("Update failed: %v", err)
	}
	check := collect(t, mustScan(t, backend, "users"))[0]
	if check["name"] != "ALICE" {
		t.Fatalf("updated name = %#v", check["name"])
	}
	if err := backend.Update("users", cursor, Row{"missing": "x"}); !isErr[*ColumnNotFound](err) {
		t.Fatalf("missing assignment error = %T %v", err, err)
	}
	if err := backend.Delete("users", cursor); err != nil {
		t.Fatalf("Delete failed: %v", err)
	}
	if got := collect(t, mustScan(t, backend, "users"))[0]["id"]; got != int64(2) {
		t.Fatalf("first row after delete = %#v", got)
	}
	if err := backend.Update("users", cursor, Row{"name": "x"}); !isErr[*Unsupported](err) {
		t.Fatalf("stale cursor update error = %T %v", err, err)
	}
	if err := backend.Delete("users", NewListCursor(nil)); !isErr[*Unsupported](err) {
		t.Fatalf("foreign cursor delete error = %T %v", err, err)
	}
}

func TestDDL(t *testing.T) {
	backend := NewInMemoryBackend()
	must(t, backend.CreateTable("t", []ColumnDef{{Name: "id", TypeName: "INTEGER"}}, false))
	must(t, backend.CreateTable("T", nil, true))
	if err := backend.CreateTable("t", nil, false); !isErr[*TableAlreadyExists](err) {
		t.Fatalf("duplicate table error = %T %v", err, err)
	}
	if err := backend.CreateTable("dupe", []ColumnDef{{Name: "id"}, {Name: "ID"}}, false); !isErr[*ColumnAlreadyExists](err) {
		t.Fatalf("duplicate column error = %T %v", err, err)
	}
	must(t, backend.Insert("t", Row{"id": int64(1)}))
	must(t, backend.AddColumn("t", ColumnDef{Name: "status", TypeName: "TEXT", DefaultValue: "new", HasDefault: true}))
	if got := collect(t, mustScan(t, backend, "t"))[0]["status"]; got != "new" {
		t.Fatalf("backfilled status = %#v", got)
	}
	if err := backend.AddColumn("t", ColumnDef{Name: "status"}); !isErr[*ColumnAlreadyExists](err) {
		t.Fatalf("duplicate add column error = %T %v", err, err)
	}
	if err := backend.AddColumn("t", ColumnDef{Name: "required", NotNull: true}); !isErr[*ConstraintViolation](err) {
		t.Fatalf("required add column error = %T %v", err, err)
	}
	must(t, backend.DropTable("t", false))
	must(t, backend.DropTable("t", true))
	if err := backend.DropTable("t", false); !isErr[*TableNotFound](err) {
		t.Fatalf("drop missing error = %T %v", err, err)
	}
}

func TestTransactionsAndSavepoints(t *testing.T) {
	backend := users(t)
	handle, err := backend.BeginTransaction()
	if err != nil {
		t.Fatalf("BeginTransaction failed: %v", err)
	}
	must(t, backend.Insert("users", Row{"id": int64(4), "name": "Dave", "age": int64(41), "email": "dave@example.com"}))
	must(t, backend.Rollback(handle))
	if containsID(t, backend, int64(4)) {
		t.Fatal("rollback kept inserted row")
	}

	committed, err := backend.BeginTransaction()
	if err != nil {
		t.Fatalf("BeginTransaction failed: %v", err)
	}
	must(t, backend.Insert("users", Row{"id": int64(4), "name": "Dave", "age": int64(41), "email": "dave@example.com"}))
	must(t, backend.Commit(committed))
	if !containsID(t, backend, int64(4)) {
		t.Fatal("commit lost inserted row")
	}
	active, err := backend.BeginTransaction()
	if err != nil {
		t.Fatalf("BeginTransaction failed: %v", err)
	}
	if got, ok := backend.CurrentTransaction(); !ok || got != active {
		t.Fatalf("CurrentTransaction = %v, %v", got, ok)
	}
	if _, err := backend.BeginTransaction(); !isErr[*Unsupported](err) {
		t.Fatalf("nested transaction error = %T %v", err, err)
	}
	must(t, backend.Commit(active))
	if err := backend.Commit(active); !isErr[*Unsupported](err) {
		t.Fatalf("stale commit error = %T %v", err, err)
	}

	handle, err = backend.BeginTransaction()
	if err != nil {
		t.Fatalf("BeginTransaction failed: %v", err)
	}
	must(t, backend.CreateSavepoint("s1"))
	must(t, backend.Insert("users", Row{"id": int64(5), "name": "Eve", "age": int64(22), "email": "eve@example.com"}))
	must(t, backend.RollbackToSavepoint("s1"))
	if containsID(t, backend, int64(5)) {
		t.Fatal("rollback to savepoint kept row")
	}
	must(t, backend.ReleaseSavepoint("s1"))
	if err := backend.ReleaseSavepoint("s1"); !isErr[*Unsupported](err) {
		t.Fatalf("release missing savepoint error = %T %v", err, err)
	}
	must(t, backend.Commit(handle))

	must(t, backend.CreateSavepoint("implicit"))
	if _, ok := backend.CurrentTransaction(); !ok {
		t.Fatal("implicit savepoint did not begin transaction")
	}
	current, _ := backend.CurrentTransaction()
	must(t, backend.Rollback(current))
}

func TestIndexes(t *testing.T) {
	backend := users(t)
	must(t, backend.CreateIndex(IndexDef{Name: "idx_age", Table: "users", Columns: []string{"age"}}))
	indexes := backend.ListIndexes("USERS")
	if len(indexes) != 1 || indexes[0].Name != "idx_age" {
		t.Fatalf("ListIndexes = %#v", indexes)
	}
	rowids, err := backend.ScanIndex("idx_age", []any{int64(25)}, []any{int64(30)}, true, true)
	if err != nil {
		t.Fatalf("ScanIndex failed: %v", err)
	}
	if !reflect.DeepEqual(rowids, []int{1, 0}) {
		t.Fatalf("rowids = %#v", rowids)
	}
	it, err := backend.ScanByRowids("users", rowids)
	if err != nil {
		t.Fatalf("ScanByRowids failed: %v", err)
	}
	rows := collect(t, it)
	if got := rowValues(rows, "id"); !reflect.DeepEqual(got, []any{int64(2), int64(1)}) {
		t.Fatalf("scan by rowids = %#v", got)
	}
	must(t, backend.DropIndex("idx_age", false))
	if got := backend.ListIndexes(""); len(got) != 0 {
		t.Fatalf("indexes after drop = %#v", got)
	}
	if err := backend.DropIndex("idx_age", false); !isErr[*IndexNotFound](err) {
		t.Fatalf("drop missing index error = %T %v", err, err)
	}
	must(t, backend.DropIndex("idx_age", true))
}

func TestIndexValidation(t *testing.T) {
	backend := users(t)
	must(t, backend.CreateIndex(IndexDef{Name: "idx_email", Table: "users", Columns: []string{"email"}, Unique: true}))
	if err := backend.CreateIndex(IndexDef{Name: "IDX_EMAIL", Table: "users", Columns: []string{"email"}}); !isErr[*IndexAlreadyExists](err) {
		t.Fatalf("duplicate index error = %T %v", err, err)
	}
	if err := backend.CreateIndex(IndexDef{Name: "idx_missing", Table: "missing", Columns: []string{"id"}}); !isErr[*TableNotFound](err) {
		t.Fatalf("missing table index error = %T %v", err, err)
	}
	if err := backend.CreateIndex(IndexDef{Name: "idx_bad", Table: "users", Columns: []string{"missing"}}); !isErr[*ColumnNotFound](err) {
		t.Fatalf("missing column index error = %T %v", err, err)
	}
	if _, err := backend.ScanIndex("missing", nil, nil, true, true); !isErr[*IndexNotFound](err) {
		t.Fatalf("scan missing index error = %T %v", err, err)
	}
}

func TestTriggersAndVersions(t *testing.T) {
	backend := users(t)
	if backend.GetSchemaVersion() == 0 {
		t.Fatal("schema version was not bumped by CreateTable")
	}
	must(t, backend.SetUserVersion(7))
	if backend.GetUserVersion() != 7 {
		t.Fatalf("user version = %d", backend.GetUserVersion())
	}
	if err := backend.SetUserVersion(-1); err == nil {
		t.Fatal("SetUserVersion accepted negative value")
	}
	trigger := TriggerDef{Name: "tr_users_ai", Table: "users", Timing: "AFTER", Event: "INSERT", Body: "SELECT 1"}
	must(t, backend.CreateTrigger(trigger))
	if got := backend.ListTriggers("USERS"); !reflect.DeepEqual(got, []TriggerDef{trigger}) {
		t.Fatalf("ListTriggers = %#v", got)
	}
	if err := backend.CreateTrigger(trigger); !isErr[*TriggerAlreadyExists](err) {
		t.Fatalf("duplicate trigger error = %T %v", err, err)
	}
	must(t, backend.DropTrigger("TR_USERS_AI", false))
	if got := backend.ListTriggers("users"); len(got) != 0 {
		t.Fatalf("triggers after drop = %#v", got)
	}
	if err := backend.DropTrigger("tr_users_ai", false); !isErr[*TriggerNotFound](err) {
		t.Fatalf("drop missing trigger error = %T %v", err, err)
	}
	must(t, backend.DropTrigger("tr_users_ai", true))
}

func TestDropTableRemovesOwnedIndexesAndTriggers(t *testing.T) {
	backend := users(t)
	must(t, backend.CreateIndex(IndexDef{Name: "idx_age", Table: "users", Columns: []string{"age"}}))
	must(t, backend.CreateTrigger(TriggerDef{Name: "tr_users_ai", Table: "users", Timing: "AFTER", Event: "INSERT", Body: "SELECT 1"}))
	must(t, backend.DropTable("users", false))
	if len(backend.ListIndexes("")) != 0 {
		t.Fatal("drop table kept index")
	}
	if len(backend.ListTriggers("users")) != 0 {
		t.Fatal("drop table kept trigger")
	}
}

func users(t *testing.T) *InMemoryBackend {
	t.Helper()
	backend := NewInMemoryBackend()
	must(t, backend.CreateTable("users", []ColumnDef{
		{Name: "id", TypeName: "INTEGER", PrimaryKey: true},
		{Name: "name", TypeName: "TEXT", NotNull: true},
		{Name: "age", TypeName: "INTEGER"},
		{Name: "email", TypeName: "TEXT", Unique: true},
	}, false))
	must(t, backend.Insert("users", Row{"id": int64(1), "name": "Alice", "age": int64(30), "email": "alice@example.com"}))
	must(t, backend.Insert("users", Row{"id": int64(2), "name": "Bob", "age": int64(25), "email": "bob@example.com"}))
	must(t, backend.Insert("users", Row{"id": int64(3), "name": "Carol", "age": nil, "email": nil}))
	return backend
}

func collect(t *testing.T, iterator RowIterator) []Row {
	t.Helper()
	defer iterator.Close()
	rows := []Row{}
	for {
		row, ok := iterator.Next()
		if !ok {
			return rows
		}
		rows = append(rows, row)
	}
}

func mustScan(t *testing.T, backend *InMemoryBackend, table string) RowIterator {
	t.Helper()
	iterator, err := backend.Scan(table)
	return mustIterator(t, iterator, err)
}

func mustIterator(t *testing.T, iterator RowIterator, err error) RowIterator {
	t.Helper()
	if err != nil {
		t.Fatalf("iterator failed: %v", err)
	}
	return iterator
}

func must(t *testing.T, err error) {
	t.Helper()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
}

func isErr[T error](err error) bool {
	var target T
	return errors.As(err, &target)
}

func rowValues(rows []Row, column string) []any {
	out := make([]any, len(rows))
	for i, row := range rows {
		out[i] = row[column]
	}
	return out
}

func containsID(t *testing.T, backend *InMemoryBackend, id int64) bool {
	t.Helper()
	for _, row := range collect(t, mustScan(t, backend, "users")) {
		if row["id"] == id {
			return true
		}
	}
	return false
}

func assertCompareLess(t *testing.T, left any, right any) {
	t.Helper()
	cmp, err := CompareSQLValues(left, right)
	if err != nil {
		t.Fatalf("CompareSQLValues failed: %v", err)
	}
	if cmp >= 0 {
		t.Fatalf("CompareSQLValues(%#v, %#v) = %d, want < 0", left, right, cmp)
	}
}
