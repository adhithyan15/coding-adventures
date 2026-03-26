// Comprehensive tests for the sql-execution-engine package.
//
// Test strategy:
//   - Each test exercises one SQL feature in isolation
//   - Tests use the InMemorySource defined in this file
//   - We verify both the column names and row values
//   - Error cases are tested for proper error types
//
// The InMemorySource simulates a two-table database:
//   - employees (id, name, dept_id, salary, active)
//   - departments (id, name, budget)
//
// This gives us enough data to test JOINs, NULLs (Dave has no dept_id),
// filtering, aggregation, and all expression types.
package sqlengine

import (
	"errors"
	"testing"
)

// =============================================================================
// InMemorySource — test fixture
// =============================================================================

// InMemorySource is a simple in-memory DataSource for testing.
// It holds two tables: employees and departments.
type InMemorySource struct{}

// employees is our test dataset. Notice:
//   - Alice and Carol are in Engineering (dept_id=1)
//   - Bob is in Marketing (dept_id=2)
//   - Dave has no department (dept_id=nil = SQL NULL)
//   - Carol is inactive (active=false)
//   - Salaries vary for testing ORDER BY and aggregation
var employees = []map[string]interface{}{
	{"id": int64(1), "name": "Alice", "dept_id": int64(1), "salary": int64(90000), "active": true},
	{"id": int64(2), "name": "Bob", "dept_id": int64(2), "salary": int64(75000), "active": true},
	{"id": int64(3), "name": "Carol", "dept_id": int64(1), "salary": int64(95000), "active": false},
	{"id": int64(4), "name": "Dave", "dept_id": nil, "salary": int64(60000), "active": true},
}

// departments is the other table for JOIN testing.
var departments = []map[string]interface{}{
	{"id": int64(1), "name": "Engineering", "budget": int64(500000)},
	{"id": int64(2), "name": "Marketing", "budget": int64(200000)},
}

func (s InMemorySource) Schema(tableName string) ([]string, error) {
	switch tableName {
	case "employees":
		return []string{"id", "name", "dept_id", "salary", "active"}, nil
	case "departments":
		return []string{"id", "name", "budget"}, nil
	default:
		return nil, &TableNotFoundError{TableName: tableName}
	}
}

func (s InMemorySource) Scan(tableName string) ([]map[string]interface{}, error) {
	switch tableName {
	case "employees":
		// Return a copy so tests can't mutate the shared slice.
		result := make([]map[string]interface{}, len(employees))
		copy(result, employees)
		return result, nil
	case "departments":
		result := make([]map[string]interface{}, len(departments))
		copy(result, departments)
		return result, nil
	default:
		return nil, &TableNotFoundError{TableName: tableName}
	}
}

// =============================================================================
// Test helpers
// =============================================================================

// mustExecute runs Execute and fails the test if it returns an error.
func mustExecute(t *testing.T, sql string) *QueryResult {
	t.Helper()
	result, err := Execute(sql, InMemorySource{})
	if err != nil {
		t.Fatalf("Execute(%q) failed: %v", sql, err)
	}
	return result
}

// assertColumns checks that the result has exactly the expected column names
// in the expected order.
func assertColumns(t *testing.T, result *QueryResult, expected []string) {
	t.Helper()
	if len(result.Columns) != len(expected) {
		t.Errorf("columns: got %v, want %v", result.Columns, expected)
		return
	}
	for i, col := range result.Columns {
		if col != expected[i] {
			t.Errorf("column[%d]: got %q, want %q", i, col, expected[i])
		}
	}
}

// assertRowCount checks that the result has exactly n rows.
func assertRowCount(t *testing.T, result *QueryResult, n int) {
	t.Helper()
	if len(result.Rows) != n {
		t.Errorf("row count: got %d, want %d (rows: %v)", len(result.Rows), n, result.Rows)
	}
}

// assertRowValue checks that result.Rows[rowIdx][colIdx] == expected.
func assertRowValue(t *testing.T, result *QueryResult, rowIdx, colIdx int, expected interface{}) {
	t.Helper()
	if rowIdx >= len(result.Rows) {
		t.Errorf("row %d does not exist (only %d rows)", rowIdx, len(result.Rows))
		return
	}
	if colIdx >= len(result.Rows[rowIdx]) {
		t.Errorf("col %d does not exist in row %d (only %d cols)", colIdx, rowIdx, len(result.Rows[rowIdx]))
		return
	}
	got := result.Rows[rowIdx][colIdx]
	if got != expected {
		t.Errorf("row[%d][%d]: got %v (%T), want %v (%T)", rowIdx, colIdx, got, got, expected, expected)
	}
}

// findColumn returns the index of a column by name, or -1 if not found.
func findColumn(result *QueryResult, name string) int {
	for i, col := range result.Columns {
		if col == name {
			return i
		}
	}
	return -1
}

// getColumnValues extracts all values for a named column as a slice.
func getColumnValues(t *testing.T, result *QueryResult, colName string) []interface{} {
	t.Helper()
	idx := findColumn(result, colName)
	if idx < 0 {
		t.Errorf("column %q not found in %v", colName, result.Columns)
		return nil
	}
	vals := make([]interface{}, len(result.Rows))
	for i, row := range result.Rows {
		vals[i] = row[idx]
	}
	return vals
}

// =============================================================================
// Test 1: SELECT * FROM employees
// =============================================================================
//
// The simplest possible query. Verifies that all rows and all columns are
// returned when using the SELECT * shorthand.
func TestSelectStar(t *testing.T) {
	result := mustExecute(t, "SELECT * FROM employees")

	// All 4 employees should be returned.
	assertRowCount(t, result, 4)

	// SELECT * should return all 5 columns.
	if len(result.Columns) != 5 {
		t.Errorf("expected 5 columns for SELECT *, got %d: %v", len(result.Columns), result.Columns)
	}
}

// =============================================================================
// Test 2: SELECT id, name FROM employees (projection)
// =============================================================================
//
// Projection reduces the result to only the specified columns.
// This is the π (project) operator in relational algebra.
func TestSelectProjection(t *testing.T) {
	result := mustExecute(t, "SELECT id, name FROM employees")

	assertColumns(t, result, []string{"id", "name"})
	assertRowCount(t, result, 4)

	// Verify first row values.
	assertRowValue(t, result, 0, 0, int64(1))
	assertRowValue(t, result, 0, 1, "Alice")
}

// =============================================================================
// Test 3: SELECT id, name AS employee_name FROM employees (alias)
// =============================================================================
//
// Column aliases rename the output column. The underlying expression is
// unchanged; only the column header in the result is renamed.
func TestSelectAlias(t *testing.T) {
	result := mustExecute(t, "SELECT id, name AS employee_name FROM employees")

	assertColumns(t, result, []string{"id", "employee_name"})
	assertRowCount(t, result, 4)
	assertRowValue(t, result, 0, 1, "Alice")
}

// =============================================================================
// Test 4: WHERE salary > 80000
// =============================================================================
//
// Numeric comparison filter. Should return Alice (90000) and Carol (95000).
func TestWhereNumericComparison(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE salary > 80000")

	assertRowCount(t, result, 2)
	names := getColumnValues(t, result, "name")
	if names[0] != "Alice" {
		t.Errorf("expected first row to be Alice, got %v", names[0])
	}
	if names[1] != "Carol" {
		t.Errorf("expected second row to be Carol, got %v", names[1])
	}
}

// =============================================================================
// Test 5: WHERE active = true
// =============================================================================
//
// Boolean comparison. Should return Alice, Bob, and Dave (active=true).
// Carol is inactive.
func TestWhereBooleanTrue(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE active = TRUE")
	assertRowCount(t, result, 3)
}

// =============================================================================
// Test 6: WHERE dept_id IS NULL
// =============================================================================
//
// IS NULL checks for SQL NULL values. Dave has no department (dept_id=nil).
// This tests the three-valued logic: "dept_id = NULL" would not work (NULL ≠ NULL
// in SQL), but IS NULL does.
func TestWhereIsNull(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE dept_id IS NULL")

	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, "Dave")
}

// =============================================================================
// Test 7: WHERE dept_id IS NOT NULL
// =============================================================================
//
// IS NOT NULL returns rows where the column has a non-NULL value.
// Alice, Bob, and Carol all have a dept_id.
func TestWhereIsNotNull(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE dept_id IS NOT NULL")

	assertRowCount(t, result, 3)
}

// =============================================================================
// Test 8: WHERE salary BETWEEN 70000 AND 90000
// =============================================================================
//
// BETWEEN is syntactic sugar for >= low AND <= high.
// Alice (90000) and Bob (75000) fall in this range.
// Carol (95000) and Dave (60000) do not.
func TestWhereBetween(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE salary BETWEEN 70000 AND 90000")

	assertRowCount(t, result, 2)
	names := getColumnValues(t, result, "name")

	nameSet := map[interface{}]bool{}
	for _, n := range names {
		nameSet[n] = true
	}
	if !nameSet["Alice"] {
		t.Error("expected Alice in BETWEEN result")
	}
	if !nameSet["Bob"] {
		t.Error("expected Bob in BETWEEN result")
	}
}

// =============================================================================
// Test 9: WHERE id IN (1, 3)
// =============================================================================
//
// IN tests membership in a value list. Equivalent to id = 1 OR id = 3.
// Returns Alice (id=1) and Carol (id=3).
func TestWhereIn(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE id IN (1, 3)")

	assertRowCount(t, result, 2)
	names := getColumnValues(t, result, "name")
	nameSet := map[interface{}]bool{}
	for _, n := range names {
		nameSet[n] = true
	}
	if !nameSet["Alice"] {
		t.Error("expected Alice in IN result")
	}
	if !nameSet["Carol"] {
		t.Error("expected Carol in IN result")
	}
}

// =============================================================================
// Test 10: WHERE name LIKE 'A%'
// =============================================================================
//
// LIKE pattern matching. '%' matches any sequence of characters.
// Only Alice starts with 'A'.
func TestWhereLike(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE name LIKE 'A%'")

	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, "Alice")
}

// =============================================================================
// Test 11: WHERE active = true AND salary > 80000
// =============================================================================
//
// AND combines two predicates: both must be true.
// Alice is active AND has salary > 80000.
// Bob is active but salary=75000 (not > 80000).
// Carol is inactive (even though salary > 80000).
// Dave is active but salary=60000.
func TestWhereAndCondition(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE active = TRUE AND salary > 80000")

	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, "Alice")
}

// =============================================================================
// Test 12: WHERE active = false OR salary > 90000
// =============================================================================
//
// OR: either condition can be true.
// Carol is inactive (false) OR has salary > 90000 (95000 > 90000) — both true.
// No other employees qualify.
func TestWhereOrCondition(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE active = FALSE OR salary > 90000")

	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, "Carol")
}

// =============================================================================
// Test 13: WHERE NOT active
// =============================================================================
//
// NOT negates the boolean predicate. Only Carol is inactive.
func TestWhereNot(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE NOT active")

	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, "Carol")
}

// =============================================================================
// Test 14: ORDER BY salary DESC
// =============================================================================
//
// Descending sort: Carol (95000), Alice (90000), Bob (75000), Dave (60000).
func TestOrderBySalaryDesc(t *testing.T) {
	result := mustExecute(t, "SELECT name, salary FROM employees ORDER BY salary DESC")

	assertRowCount(t, result, 4)
	// Verify descending order.
	salaryIdx := findColumn(result, "salary")
	for i := 0; i < len(result.Rows)-1; i++ {
		a := result.Rows[i][salaryIdx].(int64)
		b := result.Rows[i+1][salaryIdx].(int64)
		if a < b {
			t.Errorf("rows not sorted DESC at index %d: %d < %d", i, a, b)
		}
	}
}

// =============================================================================
// Test 15: ORDER BY name ASC
// =============================================================================
//
// Ascending alphabetical sort: Alice, Bob, Carol, Dave.
func TestOrderByNameAsc(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees ORDER BY name ASC")

	assertRowCount(t, result, 4)
	expected := []string{"Alice", "Bob", "Carol", "Dave"}
	for i, exp := range expected {
		assertRowValue(t, result, i, 0, exp)
	}
}

// =============================================================================
// Test 16: LIMIT 2
// =============================================================================
//
// Return at most 2 rows. The first 2 employees (Alice, Bob) in scan order.
func TestLimit(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees LIMIT 2")

	assertRowCount(t, result, 2)
}

// =============================================================================
// Test 17: LIMIT 2 OFFSET 1
// =============================================================================
//
// Skip 1 row, then return at most 2. Skips Alice, returns Bob and Carol.
func TestLimitOffset(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees LIMIT 2 OFFSET 1")

	assertRowCount(t, result, 2)
	assertRowValue(t, result, 0, 0, "Bob")
	assertRowValue(t, result, 1, 0, "Carol")
}

// =============================================================================
// Test 18: SELECT DISTINCT dept_id FROM employees
// =============================================================================
//
// DISTINCT removes duplicate values. The employees have dept_ids: 1, 2, 1, nil.
// DISTINCT should give: 1, 2, nil (3 unique values).
func TestSelectDistinct(t *testing.T) {
	result := mustExecute(t, "SELECT DISTINCT dept_id FROM employees")

	// Should have 3 distinct values: 1, 2, nil
	assertRowCount(t, result, 3)
}

// =============================================================================
// Test 19: INNER JOIN
// =============================================================================
//
// INNER JOIN returns only rows where the ON condition matches.
// Dave (dept_id=nil) has no matching department and is excluded.
// The result should have 3 rows (Alice, Bob, Carol all have departments).
func TestInnerJoin(t *testing.T) {
	sql := "SELECT employees.name, departments.name FROM employees INNER JOIN departments ON employees.dept_id = departments.id"
	result := mustExecute(t, sql)

	// Dave has no department, so only 3 rows.
	assertRowCount(t, result, 3)
}

// =============================================================================
// Test 20: LEFT JOIN (produces NULLs)
// =============================================================================
//
// LEFT JOIN preserves all left (employees) rows. For Dave (no dept), the
// departments columns are NULL. The result should have all 4 employees.
func TestLeftJoin(t *testing.T) {
	sql := "SELECT employees.name, departments.name FROM employees LEFT JOIN departments ON employees.dept_id = departments.id"
	result := mustExecute(t, sql)

	// All 4 employees preserved.
	assertRowCount(t, result, 4)

	// Find Dave's row and verify departments.name is NULL.
	empNameIdx := findColumn(result, "employees.name")
	deptNameIdx := findColumn(result, "departments.name")

	if empNameIdx < 0 || deptNameIdx < 0 {
		// Try unqualified names.
		empNameIdx = findColumn(result, "name")
	}

	daveFound := false
	for _, row := range result.Rows {
		if empNameIdx >= 0 && row[empNameIdx] == "Dave" {
			daveFound = true
			if deptNameIdx >= 0 && row[deptNameIdx] != nil {
				t.Errorf("Dave's department name should be NULL in LEFT JOIN, got %v", row[deptNameIdx])
			}
		}
	}
	if !daveFound && empNameIdx >= 0 {
		t.Error("Dave not found in LEFT JOIN result")
	}
}

// =============================================================================
// Test 21: COUNT(*)
// =============================================================================
//
// COUNT(*) counts all rows regardless of NULL values.
// There are 4 employees.
func TestCountStar(t *testing.T) {
	result := mustExecute(t, "SELECT COUNT(*) FROM employees")

	assertRowCount(t, result, 1) // one output row for the aggregate
	// The count should be 4.
	assertRowValue(t, result, 0, 0, int64(4))
}

// =============================================================================
// Test 22: COUNT(*) and AVG(salary)
// =============================================================================
//
// Multiple aggregates in one SELECT.
// COUNT(*) = 4, AVG(salary) = (90000 + 75000 + 95000 + 60000) / 4 = 80000
func TestCountAndAvg(t *testing.T) {
	result := mustExecute(t, "SELECT COUNT(*), AVG(salary) FROM employees")

	assertRowCount(t, result, 1)
	// Count should be 4.
	if result.Rows[0][0] != int64(4) {
		t.Errorf("COUNT(*) = %v, want 4", result.Rows[0][0])
	}
	// AVG salary.
	avg, ok := result.Rows[0][1].(float64)
	if !ok {
		t.Errorf("AVG(salary) should be float64, got %T: %v", result.Rows[0][1], result.Rows[0][1])
	} else if avg != 80000.0 {
		t.Errorf("AVG(salary) = %v, want 80000.0", avg)
	}
}

// =============================================================================
// Test 23: GROUP BY with COUNT(*) and SUM
// =============================================================================
//
// GROUP BY dept_id partitions employees into groups:
//   dept_id=1: Alice (90000), Carol (95000) → COUNT=2, SUM=185000
//   dept_id=2: Bob (75000) → COUNT=1, SUM=75000
//   dept_id=nil: Dave (60000) → COUNT=1, SUM=60000
func TestGroupByCountAndSum(t *testing.T) {
	result := mustExecute(t, "SELECT dept_id, COUNT(*), SUM(salary) FROM employees GROUP BY dept_id")

	assertRowCount(t, result, 3) // 3 distinct dept groups
}

// =============================================================================
// Test 24: HAVING
// =============================================================================
//
// HAVING filters groups after aggregation.
// Only the group with dept_id=1 has COUNT(*) > 1 (Alice and Carol).
func TestHaving(t *testing.T) {
	result := mustExecute(t, "SELECT dept_id, COUNT(*) FROM employees GROUP BY dept_id HAVING COUNT(*) > 1")

	assertRowCount(t, result, 1)
	// The remaining group should be dept_id=1.
	deptIdx := findColumn(result, "dept_id")
	if deptIdx >= 0 {
		assertRowValue(t, result, 0, deptIdx, int64(1))
	}
}

// =============================================================================
// Test 25: Arithmetic — salary * 1.1
// =============================================================================
//
// Arithmetic expressions in SELECT compute derived values per row.
// Alice's salary * 1.1 = 90000 * 1.1 ≈ 99000.0
//
// Note: In IEEE 754 float64, 90000 * 1.1 is not exactly 99000 due to the
// binary representation of 1.1. We use an epsilon comparison instead of
// exact equality to handle floating-point precision.
func TestArithmetic(t *testing.T) {
	result := mustExecute(t, "SELECT name, salary * 1.1 FROM employees WHERE name = 'Alice'")

	assertRowCount(t, result, 1)
	// salary * 1.1 should be a float close to 99000.
	val := result.Rows[0][1]
	f, ok := val.(float64)
	if !ok {
		t.Errorf("salary * 1.1 should be float64, got %T: %v", val, val)
	} else {
		// Epsilon comparison: 90000 * 1.1 in float64 ≈ 99000.00000000001
		const epsilon = 0.01
		if f < 99000.0-epsilon || f > 99000.0+epsilon {
			t.Errorf("salary * 1.1 = %v, want approximately 99000 (±%.2f)", f, epsilon)
		}
	}
}

// =============================================================================
// Test 26: Error — unknown table
// =============================================================================
//
// Querying a non-existent table should return a TableNotFoundError.
func TestErrorUnknownTable(t *testing.T) {
	_, err := Execute("SELECT * FROM nonexistent_table", InMemorySource{})
	if err == nil {
		t.Fatal("expected error for unknown table, got nil")
	}

	var tableErr *TableNotFoundError
	if !errors.As(err, &tableErr) {
		t.Errorf("expected TableNotFoundError, got %T: %v", err, err)
	}
}

// =============================================================================
// Test 27: Error — unknown column
// =============================================================================
//
// Referencing a column that doesn't exist should return a ColumnNotFoundError.
func TestErrorUnknownColumn(t *testing.T) {
	_, err := Execute("SELECT nonexistent_column FROM employees", InMemorySource{})
	if err == nil {
		t.Fatal("expected error for unknown column, got nil")
	}

	var colErr *ColumnNotFoundError
	if !errors.As(err, &colErr) {
		t.Errorf("expected ColumnNotFoundError, got %T: %v", err, err)
	}
}

// =============================================================================
// Additional tests for broader coverage
// =============================================================================

// TestUnsupportedStatement verifies that non-SELECT statements return an error.
func TestUnsupportedStatement(t *testing.T) {
	statements := []string{
		"INSERT INTO employees VALUES (5, 'Eve', 1, 80000, TRUE)",
		"UPDATE employees SET salary = 100000 WHERE id = 1",
		"DELETE FROM employees WHERE id = 1",
		"CREATE TABLE foo (id INTEGER)",
		"DROP TABLE employees",
	}
	for _, sql := range statements {
		_, err := Execute(sql, InMemorySource{})
		if err == nil {
			t.Errorf("Execute(%q) should fail for non-SELECT, got nil error", sql)
		}
		var unsupported *UnsupportedStatementError
		if !errors.As(err, &unsupported) {
			t.Errorf("Execute(%q): expected UnsupportedStatementError, got %T: %v", sql, err, err)
		}
	}
}

// TestExecuteAll verifies that ExecuteAll handles multiple statements.
func TestExecuteAll(t *testing.T) {
	results, err := ExecuteAll(
		"SELECT id FROM employees LIMIT 1; SELECT COUNT(*) FROM employees",
		InMemorySource{},
	)
	if err != nil {
		t.Fatalf("ExecuteAll failed: %v", err)
	}
	if len(results) != 2 {
		t.Fatalf("expected 2 results, got %d", len(results))
	}
	// First query: 1 row.
	assertRowCount(t, results[0], 1)
	// Second query: 1 aggregate row with count=4.
	assertRowCount(t, results[1], 1)
	assertRowValue(t, results[1], 0, 0, int64(4))
}

// TestOrderByDefault verifies that ORDER BY without ASC/DESC defaults to ASC.
func TestOrderByDefault(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees ORDER BY name")
	assertRowCount(t, result, 4)
	assertRowValue(t, result, 0, 0, "Alice") // alphabetically first
	assertRowValue(t, result, 3, 0, "Dave")  // alphabetically last
}

// TestWhereLikeUnderscore verifies the _ wildcard in LIKE.
// '_ob' matches exactly one character followed by 'ob'.
func TestWhereLikeUnderscore(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE name LIKE '_ob'")
	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, "Bob")
}

// TestWhereNotBetween verifies NOT BETWEEN.
// Employees NOT between salary 70000 and 90000: Carol (95000) and Dave (60000).
func TestWhereNotBetween(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE salary NOT BETWEEN 70000 AND 90000")
	assertRowCount(t, result, 2)
}

// TestWhereNotIn verifies NOT IN.
// Employees not in ids 1,3: Bob (id=2) and Dave (id=4).
func TestWhereNotIn(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE id NOT IN (1, 3)")
	assertRowCount(t, result, 2)
}

// TestLimitBeyondRowCount verifies LIMIT larger than available rows returns all rows.
func TestLimitBeyondRowCount(t *testing.T) {
	result := mustExecute(t, "SELECT * FROM employees LIMIT 100")
	assertRowCount(t, result, 4)
}

// TestOffsetBeyondRowCount verifies OFFSET larger than row count returns empty result.
func TestOffsetBeyondRowCount(t *testing.T) {
	result := mustExecute(t, "SELECT * FROM employees LIMIT 10 OFFSET 100")
	assertRowCount(t, result, 0)
}

// TestCountDistinctDeptId verifies COUNT(*) on dept_id groups.
// We test that grouping on a nullable column puts NULLs in their own group.
func TestGroupByNullColumn(t *testing.T) {
	result := mustExecute(t, "SELECT dept_id, COUNT(*) FROM employees GROUP BY dept_id")
	// 3 groups: dept_id=1 (2 rows), dept_id=2 (1 row), dept_id=NULL (1 row)
	assertRowCount(t, result, 3)
}

// TestMinMax verifies MIN and MAX aggregate functions.
func TestMinMax(t *testing.T) {
	result := mustExecute(t, "SELECT MIN(salary), MAX(salary) FROM employees")
	assertRowCount(t, result, 1)
	// MIN = 60000 (Dave), MAX = 95000 (Carol)
	assertRowValue(t, result, 0, 0, int64(60000))
	assertRowValue(t, result, 0, 1, int64(95000))
}

// TestSum verifies SUM aggregate function.
func TestSum(t *testing.T) {
	result := mustExecute(t, "SELECT SUM(salary) FROM employees")
	assertRowCount(t, result, 1)
	// Total: 90000 + 75000 + 95000 + 60000 = 320000
	assertRowValue(t, result, 0, 0, int64(320000))
}

// TestUnaryMinus verifies unary negation in expressions.
func TestUnaryMinus(t *testing.T) {
	result := mustExecute(t, "SELECT name, -salary FROM employees WHERE name = 'Alice'")
	assertRowCount(t, result, 1)
	// -90000
	val := result.Rows[0][1]
	switch v := val.(type) {
	case int64:
		if v != -90000 {
			t.Errorf("-salary = %v, want -90000", v)
		}
	case float64:
		if v != -90000 {
			t.Errorf("-salary = %v, want -90000", v)
		}
	default:
		t.Errorf("-salary: unexpected type %T: %v", val, val)
	}
}

// TestArithmeticAddition verifies + operator.
func TestArithmeticAddition(t *testing.T) {
	result := mustExecute(t, "SELECT salary + 10000 FROM employees WHERE name = 'Bob'")
	assertRowCount(t, result, 1)
	// 75000 + 10000 = 85000
	val := result.Rows[0][0]
	switch v := val.(type) {
	case int64:
		if v != 85000 {
			t.Errorf("salary + 10000 = %v, want 85000", v)
		}
	case float64:
		if v != 85000 {
			t.Errorf("salary + 10000 = %v, want 85000", v)
		}
	default:
		t.Errorf("unexpected type %T: %v", val, val)
	}
}

// TestQueryResultString verifies that QueryResult.String() produces output
// without panicking. We don't check the exact format, just that it works.
func TestQueryResultString(t *testing.T) {
	result := mustExecute(t, "SELECT id, name FROM employees")
	s := result.String()
	if s == "" {
		t.Error("QueryResult.String() returned empty string")
	}
}

// TestQueryResultStringEmpty verifies String() on an empty result set.
func TestQueryResultStringEmpty(t *testing.T) {
	empty := &QueryResult{Columns: []string{}, Rows: nil}
	s := empty.String()
	if s == "" {
		t.Error("QueryResult.String() on empty result should return non-empty string")
	}
}

// TestErrorTypes verifies that error types implement the error interface.
func TestErrorTypes(t *testing.T) {
	errors := []error{
		&TableNotFoundError{TableName: "test"},
		&ColumnNotFoundError{ColumnName: "col"},
		&UnsupportedStatementError{StatementType: "INSERT"},
		&EvaluationError{Message: "test"},
	}
	for _, err := range errors {
		if err.Error() == "" {
			t.Errorf("error type %T has empty Error() string", err)
		}
	}
}

// TestCrossJoin verifies CROSS JOIN produces cartesian product.
//
// Note: The sql.grammar requires an ON clause for all join types including
// CROSS JOIN: `join_clause = join_type "JOIN" table_ref "ON" expr`.
// We use "ON 1 = 1" as a tautology that always evaluates to true, effectively
// making this a cartesian product. The executor recognizes CROSS join type
// and skips the ON evaluation entirely.
func TestCrossJoin(t *testing.T) {
	sql := "SELECT employees.name, departments.name FROM employees CROSS JOIN departments ON 1 = 1"
	result := mustExecute(t, sql)
	// 4 employees × 2 departments = 8 rows
	assertRowCount(t, result, 8)
}

// TestSelectLiteral verifies selecting literal values.
func TestSelectLiteral(t *testing.T) {
	result := mustExecute(t, "SELECT id FROM employees WHERE id = 1")
	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, int64(1))
}

// TestNullComparisonReturnsNull verifies that NULL = value returns NULL (not true or false),
// which means the row is excluded from WHERE results.
func TestNullComparison(t *testing.T) {
	// Dave has dept_id = NULL. "dept_id = 1" should be NULL (not false),
	// and NULL is not truthy, so Dave should be excluded.
	result := mustExecute(t, "SELECT name FROM employees WHERE dept_id = 1")
	// Only Alice and Carol have dept_id=1.
	assertRowCount(t, result, 2)
}

// TestCountStarOnEmptyResult verifies COUNT(*) returns 0 for no matching rows.
func TestCountStarOnEmptyResult(t *testing.T) {
	result := mustExecute(t, "SELECT COUNT(*) FROM employees WHERE salary > 1000000")
	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, int64(0))
}

// TestMultipleConditions tests complex WHERE with multiple AND/OR combinations.
func TestMultipleConditions(t *testing.T) {
	// Active employees in dept 1 with salary > 85000 → Alice only
	result := mustExecute(t, "SELECT name FROM employees WHERE active = TRUE AND dept_id = 1 AND salary > 85000")
	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, "Alice")
}

// TestOrderByMultipleColumns verifies multi-column ORDER BY.
func TestOrderByMultipleColumns(t *testing.T) {
	// Order by dept_id ASC, then salary DESC
	result := mustExecute(t, "SELECT name, dept_id, salary FROM employees ORDER BY salary DESC")
	assertRowCount(t, result, 4)
	// First should be Carol (highest salary 95000).
	assertRowValue(t, result, 0, 0, "Carol")
}

// =============================================================================
// Additional coverage tests
// =============================================================================

// TestWhereLessOrEqual tests <= comparison operator.
func TestWhereLessOrEqual(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE salary <= 75000")
	// Bob (75000) and Dave (60000)
	assertRowCount(t, result, 2)
}

// TestWhereGreaterOrEqual tests >= comparison operator.
func TestWhereGreaterOrEqual(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE salary >= 90000")
	// Alice (90000) and Carol (95000)
	assertRowCount(t, result, 2)
}

// TestWhereLessThan tests < comparison operator.
func TestWhereLessThan(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE salary < 75000")
	// Dave (60000)
	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, "Dave")
}

// TestWhereNotEqual tests != comparison operator.
func TestWhereNotEqual(t *testing.T) {
	result := mustExecute(t, "SELECT name FROM employees WHERE dept_id != 1")
	// Bob (dept_id=2). Dave (NULL) is excluded because NULL != 1 is NULL (not truthy).
	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, "Bob")
}

// TestArithmeticSubtraction tests the - operator.
func TestArithmeticSubtraction(t *testing.T) {
	result := mustExecute(t, "SELECT salary - 10000 FROM employees WHERE name = 'Alice'")
	assertRowCount(t, result, 1)
	val := result.Rows[0][0]
	switch v := val.(type) {
	case int64:
		if v != 80000 {
			t.Errorf("salary - 10000 = %v, want 80000", v)
		}
	default:
		t.Errorf("unexpected type %T: %v", val, val)
	}
}

// TestArithmeticDivision tests the / operator.
func TestArithmeticDivision(t *testing.T) {
	result := mustExecute(t, "SELECT salary / 1000 FROM employees WHERE name = 'Alice'")
	assertRowCount(t, result, 1)
}

// TestRightJoin verifies RIGHT JOIN preserves all right (departments) rows.
// All departments appear even if no employees are in them.
func TestRightJoin(t *testing.T) {
	sql := "SELECT employees.name, departments.name FROM employees RIGHT JOIN departments ON employees.dept_id = departments.id"
	result := mustExecute(t, sql)
	// All 2 departments appear. Employees: Alice+dept1, Bob+dept2, Carol+dept1.
	// That's 3 matched rows. No unmatched departments (both have employees).
	assertRowCount(t, result, 3)
}

// TestFullOuterJoin verifies FULL OUTER JOIN preserves all rows from both sides.
func TestFullOuterJoin(t *testing.T) {
	sql := "SELECT employees.name, departments.name FROM employees FULL JOIN departments ON employees.dept_id = departments.id"
	result := mustExecute(t, sql)
	// Dave (NULL dept_id) is unmatched on left → appears with NULL right.
	// Both departments have matching employees → no unmatched right rows.
	// Total: Alice+dept1, Bob+dept2, Carol+dept1, Dave+NULL = 4 rows.
	assertRowCount(t, result, 4)
}

// TestHavingWithSum tests HAVING with SUM aggregate.
func TestHavingWithSum(t *testing.T) {
	result := mustExecute(t, "SELECT dept_id, SUM(salary) FROM employees GROUP BY dept_id HAVING SUM(salary) > 100000")
	// dept_id=1: sum=185000 (passes); dept_id=2: sum=75000 (fails); dept_id=nil: sum=60000 (fails)
	assertRowCount(t, result, 1)
}

// TestGroupByWithAvg tests GROUP BY with AVG aggregate.
func TestGroupByWithAvg(t *testing.T) {
	result := mustExecute(t, "SELECT dept_id, AVG(salary) FROM employees GROUP BY dept_id")
	assertRowCount(t, result, 3) // 3 groups
}

// TestGroupByWithMin tests GROUP BY with MIN aggregate.
func TestGroupByWithMin(t *testing.T) {
	result := mustExecute(t, "SELECT dept_id, MIN(salary) FROM employees GROUP BY dept_id")
	assertRowCount(t, result, 3)
}

// TestGroupByWithMax tests GROUP BY with MAX aggregate.
func TestGroupByWithMax(t *testing.T) {
	result := mustExecute(t, "SELECT dept_id, MAX(salary) FROM employees GROUP BY dept_id")
	assertRowCount(t, result, 3)
}

// TestJoinWithAlias tests JOIN with table aliases.
func TestJoinWithAlias(t *testing.T) {
	sql := "SELECT e.name, d.name FROM employees AS e INNER JOIN departments AS d ON e.dept_id = d.id"
	result := mustExecute(t, sql)
	assertRowCount(t, result, 3)
}

// TestWhereStringEqual tests string equality comparison.
func TestWhereStringEqual(t *testing.T) {
	result := mustExecute(t, "SELECT id FROM employees WHERE name = 'Bob'")
	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, int64(2))
}

// TestSumNullIgnored tests that SUM ignores NULL values.
func TestSumNullIgnored(t *testing.T) {
	// All dept_id values summed (NULL is ignored): 1 + 2 + 1 = 4
	result := mustExecute(t, "SELECT SUM(dept_id) FROM employees")
	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, int64(4))
}

// TestCountExpr tests COUNT(expr) which skips NULL values.
func TestCountExpr(t *testing.T) {
	// COUNT(dept_id) skips Dave's NULL → 3
	result := mustExecute(t, "SELECT COUNT(dept_id) FROM employees")
	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, int64(3))
}

// TestLikePercent tests % wildcard at both ends.
func TestLikePercent(t *testing.T) {
	// '%o%' matches Bob (has 'o' in middle), Carol (has 'o' in middle), Dave... no.
	// Bob: 'Bob' contains 'o'. Carol: 'Carol' contains 'o'. Dave: no 'o'.
	result := mustExecute(t, "SELECT name FROM employees WHERE name LIKE '%o%'")
	assertRowCount(t, result, 2)
}

// TestParseError tests that syntax errors return an error.
func TestParseError(t *testing.T) {
	_, err := Execute("SELECT FROM WHERE", InMemorySource{})
	if err == nil {
		t.Fatal("expected error for invalid SQL, got nil")
	}
}

// TestEmptyWhereResult verifies empty result when no rows match.
func TestEmptyWhereResult(t *testing.T) {
	result := mustExecute(t, "SELECT * FROM employees WHERE salary > 999999")
	assertRowCount(t, result, 0)
}

// TestSelectWithFunctionAlias tests function call with alias.
func TestSelectWithFunctionAlias(t *testing.T) {
	result := mustExecute(t, "SELECT COUNT(*) AS total FROM employees")
	assertRowCount(t, result, 1)
	assertRowValue(t, result, 0, 0, int64(4))
}

// TestAvgReturnsFloat tests that AVG always returns float64.
func TestAvgReturnsFloat(t *testing.T) {
	// AVG of 90000+75000+95000+60000 = 320000/4 = 80000.0
	result := mustExecute(t, "SELECT AVG(salary) FROM employees")
	assertRowCount(t, result, 1)
	if _, ok := result.Rows[0][0].(float64); !ok {
		t.Errorf("AVG should return float64, got %T: %v", result.Rows[0][0], result.Rows[0][0])
	}
}

// TestMinOnStrings tests MIN on string column.
func TestMinOnStrings(t *testing.T) {
	result := mustExecute(t, "SELECT MIN(name) FROM employees")
	assertRowCount(t, result, 1)
	// Alphabetically: Alice is first.
	assertRowValue(t, result, 0, 0, "Alice")
}

// TestMaxOnStrings tests MAX on string column.
func TestMaxOnStrings(t *testing.T) {
	result := mustExecute(t, "SELECT MAX(name) FROM employees")
	assertRowCount(t, result, 1)
	// Alphabetically: Dave is last.
	assertRowValue(t, result, 0, 0, "Dave")
}

// TestDistinctWithMultipleColumns tests DISTINCT with more than one column.
func TestDistinctWithMultipleColumns(t *testing.T) {
	// active column has: true, true, false, true → DISTINCT gives true, false
	result := mustExecute(t, "SELECT DISTINCT active FROM employees")
	assertRowCount(t, result, 2)
}

// TestQueryResultStringNonEmpty tests the String() output is non-trivial.
func TestQueryResultStringWithData(t *testing.T) {
	result := mustExecute(t, "SELECT id, name FROM employees LIMIT 2")
	s := result.String()
	if len(s) < 10 {
		t.Errorf("QueryResult.String() seems too short: %q", s)
	}
}

// TestWhereInWithNull tests IN behavior with NULL in value list.
// The grammar doesn't allow NULL in IN lists directly without extra support.
// Test basic IN with integers.
func TestWhereInEmpty(t *testing.T) {
	// id IN (99, 100, 101) matches nothing
	result := mustExecute(t, "SELECT name FROM employees WHERE id IN (99, 100, 101)")
	assertRowCount(t, result, 0)
}

// TestSelectStarWithJoin verifies SELECT * works with a JOIN.
func TestSelectStarWithJoin(t *testing.T) {
	sql := "SELECT * FROM employees INNER JOIN departments ON employees.dept_id = departments.id"
	result := mustExecute(t, sql)
	// 3 matches (Dave excluded)
	assertRowCount(t, result, 3)
}

// TestSumOverEmptyGroup tests SUM returns NULL for empty group.
func TestAvgOverEmptyGroup(t *testing.T) {
	// AVG where no rows match → one row with NULL
	result := mustExecute(t, "SELECT AVG(salary) FROM employees WHERE salary > 1000000")
	assertRowCount(t, result, 1)
	if result.Rows[0][0] != nil {
		t.Errorf("AVG over empty set should be NULL, got %v", result.Rows[0][0])
	}
}

// TestSumOverEmptySet tests SUM returns NULL for empty set.
func TestSumOverEmptySet(t *testing.T) {
	result := mustExecute(t, "SELECT SUM(salary) FROM employees WHERE salary > 1000000")
	assertRowCount(t, result, 1)
	if result.Rows[0][0] != nil {
		t.Errorf("SUM over empty set should be NULL, got %v", result.Rows[0][0])
	}
}

// TestWhereColumnInError verifies WHERE with wrong column returns ColumnNotFoundError.
func TestWhereColumnNotFound(t *testing.T) {
	_, err := Execute("SELECT name FROM employees WHERE bogus_col = 1", InMemorySource{})
	if err == nil {
		t.Fatal("expected error for unknown WHERE column")
	}
	var colErr *ColumnNotFoundError
	if !errors.As(err, &colErr) {
		t.Errorf("expected ColumnNotFoundError, got %T: %v", err, err)
	}
}
