// Tests for sql-csv-source.
//
// Test strategy:
//   - End-to-end tests exercise the full stack: CSVDataSource → sql-execution-engine
//   - Unit tests verify Schema, Scan, and coerce independently
//   - Fixtures in testdata/ provide stable, known CSV data
//
// Test data:
//
//	employees.csv:
//	  id | name  | dept_id | salary | active
//	   1 | Alice |       1 |  90000 | true
//	   2 | Bob   |       2 |  75000 | true
//	   3 | Carol |       1 |  95000 | false
//	   4 | Dave  |  (nil) |  60000 | true    ← empty dept_id → SQL NULL
//
//	departments.csv:
//	  id | name        | budget
//	   1 | Engineering | 500000
//	   2 | Marketing   | 200000
package sqlcsvsource

import (
	"errors"
	"testing"

	sqlengine "github.com/adhithyan15/coding-adventures/code/packages/go/sql-execution-engine"
)

// testdataDir is the path (relative to the package root) to the directory
// containing the fixture CSV files used by all tests.
const testdataDir = "testdata"

// newSource creates a CSVDataSource pointing at the testdata directory.
// Used by most tests as a shorthand.
func newSource() *CSVDataSource {
	return New(testdataDir)
}

// mustExecute runs a SQL query and fails the test if it returns an error.
// Returns the QueryResult on success.
func mustExecute(t *testing.T, sql string) *sqlengine.QueryResult {
	t.Helper()
	result, err := sqlengine.Execute(sql, newSource())
	if err != nil {
		t.Fatalf("query %q failed: %v", sql, err)
	}
	return result
}

// colIndex returns the index of colName in result.Columns, or -1 if not found.
func colIndex(result *sqlengine.QueryResult, colName string) int {
	for i, c := range result.Columns {
		if c == colName {
			return i
		}
	}
	return -1
}

// colValues extracts all values for a named column from a result.
// Fails the test if the column is not found.
func colValues(t *testing.T, result *sqlengine.QueryResult, colName string) []interface{} {
	t.Helper()
	idx := colIndex(result, colName)
	if idx < 0 {
		t.Fatalf("column %q not found in result columns %v", colName, result.Columns)
	}
	vals := make([]interface{}, len(result.Rows))
	for i, row := range result.Rows {
		vals[i] = row[idx]
	}
	return vals
}

// =============================================================================
// Test 1: SELECT * FROM employees — full table scan
// =============================================================================
//
// The simplest query. Verifies:
//   - Schema is called and returns the right column names in order
//   - Scan returns all four rows
//   - Type coercion: integers, booleans, and nil all work

func TestSelectStar(t *testing.T) {
	result := mustExecute(t, "SELECT * FROM employees")

	// All five column names must be present. The sql-execution-engine's SELECT *
	// expansion uses projectStar, which sorts column names alphabetically for
	// determinism. We verify presence, not order, since ordering is controlled
	// by the engine.
	wantCols := map[string]bool{
		"id": true, "name": true, "dept_id": true, "salary": true, "active": true,
	}
	if len(result.Columns) != 5 {
		t.Fatalf("got %d columns, want 5: %v", len(result.Columns), result.Columns)
	}
	for _, col := range result.Columns {
		if !wantCols[col] {
			t.Errorf("unexpected column %q in result %v", col, result.Columns)
		}
	}

	// Four rows.
	if len(result.Rows) != 4 {
		t.Fatalf("got %d rows, want 4", len(result.Rows))
	}

	// id values must be int64, not strings.
	ids := colValues(t, result, "id")
	for _, id := range ids {
		if _, ok := id.(int64); !ok {
			t.Errorf("id value %v (%T) is not int64", id, id)
		}
	}

	// salary values must be int64.
	salaries := colValues(t, result, "salary")
	for _, s := range salaries {
		if _, ok := s.(int64); !ok {
			t.Errorf("salary value %v (%T) is not int64", s, s)
		}
	}

	// active values must be bool.
	actives := colValues(t, result, "active")
	for _, a := range actives {
		if _, ok := a.(bool); !ok {
			t.Errorf("active value %v (%T) is not bool", a, a)
		}
	}
}

// =============================================================================
// Test 2: WHERE with typed values
// =============================================================================
//
// Typed comparisons require coerced values. Without coercion:
//   - `active = true`    would compare string "true" to SQL bool true → no match
//   - `salary > 80000`   would do string vs int comparison → wrong results

func TestWhereActiveTrue(t *testing.T) {
	result := mustExecute(t, "SELECT id, name FROM employees WHERE active = true")

	// Alice(1), Bob(2), Dave(4) are active. Carol(3) is not.
	if len(result.Rows) != 3 {
		t.Fatalf("got %d rows, want 3 (Alice, Bob, Dave)", len(result.Rows))
	}

	names := colValues(t, result, "name")
	for _, n := range names {
		if n == "Carol" {
			t.Error("Carol (active=false) should not be in results")
		}
	}
}

func TestWhereSalaryGt(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE salary > 80000")

	// Alice(90000) and Carol(95000) qualify. Bob(75000) and Dave(60000) don't.
	if len(result.Rows) != 2 {
		t.Fatalf("got %d rows, want 2 (Alice, Carol)", len(result.Rows))
	}

	names := make(map[interface{}]bool)
	for _, n := range colValues(t, result, "name") {
		names[n] = true
	}
	if !names["Alice"] {
		t.Error("Alice (salary=90000) should be in results")
	}
	if !names["Carol"] {
		t.Error("Carol (salary=95000) should be in results")
	}
}

func TestWhereCompound(t *testing.T) {
	result := mustExecute(t,
		"SELECT name FROM employees WHERE active = true AND salary > 80000")

	// Only Alice: active=true AND salary=90000.
	if len(result.Rows) != 1 {
		t.Fatalf("got %d rows, want 1 (Alice only)", len(result.Rows))
	}
	if name := colValues(t, result, "name")[0]; name != "Alice" {
		t.Errorf("got %v, want Alice", name)
	}
}

// =============================================================================
// Test 3: IS NULL — Dave's dept_id is empty → nil
// =============================================================================
//
// The empty CSV field "" must be coerced to nil for IS NULL to work.
// Without coercion, IS NULL would compare string "" to NULL → no match.

func TestIsNull(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE dept_id IS NULL")

	// Only Dave has an empty (nil) dept_id.
	if len(result.Rows) != 1 {
		t.Fatalf("got %d rows, want 1 (Dave only)", len(result.Rows))
	}
	if name := colValues(t, result, "name")[0]; name != "Dave" {
		t.Errorf("got %v, want Dave", name)
	}
}

func TestIsNotNull(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE dept_id IS NOT NULL")

	// Alice, Bob, Carol have non-nil dept_id. Dave does not.
	if len(result.Rows) != 3 {
		t.Fatalf("got %d rows, want 3 (Alice, Bob, Carol)", len(result.Rows))
	}
	for _, n := range colValues(t, result, "name") {
		if n == "Dave" {
			t.Error("Dave (dept_id=nil) should not be in IS NOT NULL results")
		}
	}
}

// =============================================================================
// Test 4: INNER JOIN across two CSV files
// =============================================================================
//
// Joining employees and departments requires:
//   - Both CSV files to be read (Schema + Scan called for each)
//   - Integer equality for ON e.dept_id = d.id (both must be int64)
//   - NULL exclusion for Dave (INNER JOIN excludes NULL join keys)

func TestInnerJoin(t *testing.T) {
	// The engine projects SELECT e.name, d.name as two columns both named "name"
	// (the bare column name, since the engine doesn't prefix with the alias in
	// the output). We use explicit aliases to get distinct, testable column names.
	result := mustExecute(t, `
		SELECT e.name AS emp_name, d.name AS dept_name
		FROM employees AS e
		INNER JOIN departments AS d ON e.dept_id = d.id
	`)

	// Alice(dept=1), Bob(dept=2), Carol(dept=1) join. Dave(dept=nil) does not.
	if len(result.Rows) != 3 {
		t.Fatalf("got %d rows, want 3 (Alice, Bob, Carol)", len(result.Rows))
	}

	// Build a lookup: employee name → department name.
	eIdx := colIndex(result, "emp_name")
	dIdx := colIndex(result, "dept_name")
	if eIdx < 0 || dIdx < 0 {
		t.Fatalf("expected columns emp_name and dept_name, got %v", result.Columns)
	}

	deptFor := make(map[string]string)
	for _, row := range result.Rows {
		eName, _ := row[eIdx].(string)
		dName, _ := row[dIdx].(string)
		deptFor[eName] = dName
	}

	wantDepts := map[string]string{
		"Alice": "Engineering",
		"Bob":   "Marketing",
		"Carol": "Engineering",
	}
	for emp, wantDept := range wantDepts {
		if got := deptFor[emp]; got != wantDept {
			t.Errorf("%s: dept = %q, want %q", emp, got, wantDept)
		}
	}
	if _, hasDave := deptFor["Dave"]; hasDave {
		t.Error("Dave (dept_id=nil) should not appear in INNER JOIN result")
	}
}

// =============================================================================
// Test 5: GROUP BY with COUNT
// =============================================================================
//
// GROUP BY groups rows by dept_id (int64) value and COUNT(*) counts each group.
// Dave's nil dept_id forms its own NULL group.

func TestGroupByCount(t *testing.T) {
	result := mustExecute(t, "SELECT dept_id, COUNT(*) FROM employees GROUP BY dept_id")

	// Find which column index holds the COUNT value.
	countIdx := -1
	for i, c := range result.Columns {
		if len(c) >= 5 && c[:5] == "COUNT" {
			countIdx = i
			break
		}
	}
	if countIdx < 0 {
		t.Fatalf("no COUNT column found in %v", result.Columns)
	}
	deptIdx := colIndex(result, "dept_id")
	if deptIdx < 0 {
		t.Fatalf("no dept_id column found in %v", result.Columns)
	}

	// Build map: dept_id → count.
	counts := make(map[interface{}]int64)
	for _, row := range result.Rows {
		deptID := row[deptIdx]
		cnt, _ := row[countIdx].(int64)
		counts[deptID] = cnt
	}

	// Engineering (dept_id=1): Alice + Carol = 2.
	if counts[int64(1)] != 2 {
		t.Errorf("dept_id=1: got count %d, want 2", counts[int64(1)])
	}
	// Marketing (dept_id=2): Bob = 1.
	if counts[int64(2)] != 1 {
		t.Errorf("dept_id=2: got count %d, want 1", counts[int64(2)])
	}
	// NULL group (dept_id=nil): Dave = 1.
	if counts[nil] != 1 {
		t.Errorf("dept_id=nil: got count %d, want 1", counts[nil])
	}
}

// =============================================================================
// Test 6: ORDER BY salary DESC LIMIT 2
// =============================================================================
//
// Sorting requires integer comparison (not string comparison) for correctness.
// LIMIT slices to the top 2 results.

func TestOrderByLimit(t *testing.T) {
	result := mustExecute(t,
		"SELECT name, salary FROM employees ORDER BY salary DESC LIMIT 2")

	if len(result.Rows) != 2 {
		t.Fatalf("got %d rows, want 2", len(result.Rows))
	}

	names := colValues(t, result, "name")
	salaries := colValues(t, result, "salary")

	// Carol (95000) first, Alice (90000) second.
	if names[0] != "Carol" {
		t.Errorf("row 0 name: got %v, want Carol", names[0])
	}
	if names[1] != "Alice" {
		t.Errorf("row 1 name: got %v, want Alice", names[1])
	}
	if salaries[0] != int64(95000) {
		t.Errorf("row 0 salary: got %v, want 95000", salaries[0])
	}
	if salaries[1] != int64(90000) {
		t.Errorf("row 1 salary: got %v, want 90000", salaries[1])
	}
}

// =============================================================================
// Test 7: SELECT * FROM departments — second table
// =============================================================================
//
// Verifies the source works for any table in the directory, not just employees.

func TestDepartments(t *testing.T) {
	result := mustExecute(t, "SELECT * FROM departments")

	// Verify all three column names are present (engine sorts alphabetically for SELECT *).
	wantCols := map[string]bool{"id": true, "name": true, "budget": true}
	if len(result.Columns) != 3 {
		t.Fatalf("got columns %v, want [id, name, budget]", result.Columns)
	}
	for _, col := range result.Columns {
		if !wantCols[col] {
			t.Errorf("unexpected column %q in result", col)
		}
	}

	if len(result.Rows) != 2 {
		t.Fatalf("got %d rows, want 2", len(result.Rows))
	}

	// Budgets must be int64.
	budgets := colValues(t, result, "budget")
	for _, b := range budgets {
		if _, ok := b.(int64); !ok {
			t.Errorf("budget %v (%T) is not int64", b, b)
		}
	}
}

// =============================================================================
// Test 8: TableNotFoundError for unknown table
// =============================================================================
//
// Querying a table with no corresponding CSV file must return an error that
// wraps or is *sqlengine.TableNotFoundError.

func TestUnknownTable(t *testing.T) {
	_, err := sqlengine.Execute("SELECT * FROM no_such_table", newSource())
	if err == nil {
		t.Fatal("expected an error for unknown table, got nil")
	}

	// The engine wraps runtime errors. The underlying cause should be
	// TableNotFoundError. We check either the error message contains the
	// table name, or errors.As finds the correct type.
	var tne *sqlengine.TableNotFoundError
	if errors.As(err, &tne) {
		if tne.TableName != "no_such_table" {
			t.Errorf("TableNotFoundError.TableName = %q, want %q", tne.TableName, "no_such_table")
		}
	} else {
		// If the engine wraps it differently, fall back to string check.
		if errStr := err.Error(); !contains(errStr, "no_such_table") {
			t.Errorf("error message %q does not mention table name", errStr)
		}
	}
}

// contains is a simple substring check used to avoid importing strings in tests.
func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > 0 && searchSubstring(s, substr))
}

func searchSubstring(s, sub string) bool {
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}

// =============================================================================
// Test 9: Schema unit test
// =============================================================================
//
// Test Schema directly to verify column ordering without going through the
// SQL engine.

func TestSchema(t *testing.T) {
	src := newSource()

	t.Run("employees", func(t *testing.T) {
		cols, err := src.Schema("employees")
		if err != nil {
			t.Fatalf("Schema(employees) error: %v", err)
		}
		want := []string{"id", "name", "dept_id", "salary", "active"}
		if len(cols) != len(want) {
			t.Fatalf("got %v, want %v", cols, want)
		}
		for i, w := range want {
			if cols[i] != w {
				t.Errorf("col[%d]: got %q, want %q", i, cols[i], w)
			}
		}
	})

	t.Run("departments", func(t *testing.T) {
		cols, err := src.Schema("departments")
		if err != nil {
			t.Fatalf("Schema(departments) error: %v", err)
		}
		want := []string{"id", "name", "budget"}
		if len(cols) != len(want) {
			t.Fatalf("got %v, want %v", cols, want)
		}
		for i, w := range want {
			if cols[i] != w {
				t.Errorf("col[%d]: got %q, want %q", i, cols[i], w)
			}
		}
	})

	t.Run("missing table", func(t *testing.T) {
		_, err := src.Schema("no_such_table")
		if err == nil {
			t.Fatal("expected error for missing table")
		}
		var tne *sqlengine.TableNotFoundError
		if !errors.As(err, &tne) {
			t.Errorf("expected TableNotFoundError, got %T: %v", err, err)
		}
	})
}

// =============================================================================
// Test 10: Scan unit test — type coercion verification
// =============================================================================
//
// Test Scan directly to verify coercion without the SQL execution layer.

func TestScan(t *testing.T) {
	src := newSource()
	rows, err := src.Scan("employees")
	if err != nil {
		t.Fatalf("Scan(employees) error: %v", err)
	}

	if len(rows) != 4 {
		t.Fatalf("got %d rows, want 4", len(rows))
	}

	// Find Dave's row (the one with nil dept_id).
	var daveRow map[string]interface{}
	for _, row := range rows {
		if row["name"] == "Dave" {
			daveRow = row
			break
		}
	}
	if daveRow == nil {
		t.Fatal("could not find Dave's row")
	}

	// Dave's dept_id must be nil (not "").
	if daveRow["dept_id"] != nil {
		t.Errorf("Dave dept_id: got %v (%T), want nil", daveRow["dept_id"], daveRow["dept_id"])
	}

	// All salary values must be int64.
	for _, row := range rows {
		if _, ok := row["salary"].(int64); !ok {
			t.Errorf("salary %v (%T) is not int64 for %v", row["salary"], row["salary"], row["name"])
		}
	}

	// All active values must be bool.
	for _, row := range rows {
		if _, ok := row["active"].(bool); !ok {
			t.Errorf("active %v (%T) is not bool for %v", row["active"], row["active"], row["name"])
		}
	}

	// All id values must be int64.
	for _, row := range rows {
		if _, ok := row["id"].(int64); !ok {
			t.Errorf("id %v (%T) is not int64 for %v", row["id"], row["id"], row["name"])
		}
	}

	// Carol's active must be false.
	for _, row := range rows {
		if row["name"] == "Carol" {
			if row["active"] != false {
				t.Errorf("Carol active: got %v, want false", row["active"])
			}
		}
	}
}

// =============================================================================
// Test 11: coerce unit tests
// =============================================================================
//
// Direct tests of the coerce function to verify edge cases.

func TestCoerce(t *testing.T) {
	tests := []struct {
		input string
		want  interface{}
		desc  string
	}{
		{"", nil, "empty string → nil"},
		{"true", true, "true → bool true"},
		{"false", false, "false → bool false"},
		{"True", "True", "True (capital) → string (case-sensitive)"},
		{"FALSE", "FALSE", "FALSE (upper) → string (case-sensitive)"},
		{"42", int64(42), "integer string → int64"},
		{"0", int64(0), "zero → int64"},
		{"-5", int64(-5), "negative integer → int64"},
		{"3.14", float64(3.14), "float string → float64"},
		{"1.0", float64(1.0), "1.0 → float64"},
		{"123abc", "123abc", "mixed alphanumeric stays string"},
		{"hello", "hello", "plain string stays string"},
		{"3.14abc", "3.14abc", "float with suffix stays string"},
	}

	for _, tt := range tests {
		t.Run(tt.desc, func(t *testing.T) {
			got := coerce(tt.input)
			if got != tt.want {
				t.Errorf("coerce(%q) = %v (%T), want %v (%T)",
					tt.input, got, got, tt.want, tt.want)
			}
		})
	}
}
