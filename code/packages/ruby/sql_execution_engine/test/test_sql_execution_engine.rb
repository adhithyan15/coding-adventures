# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the SQL Execution Engine
# ================================================================
#
# Data model used throughout:
#
# employees:
#   id | name  | dept_id | salary | active
#   ---+-------+---------+--------+-------
#   1  | Alice | 1       | 90000  | true
#   2  | Bob   | 2       | 75000  | true
#   3  | Carol | 1       | 95000  | false
#   4  | Dave  | NULL    | 60000  | true
#
# departments:
#   id | name        | budget
#   ---+-------------+-------
#   1  | Engineering | 500000
#   2  | Marketing   | 200000
# ================================================================

class InMemorySource
  include CodingAdventures::SqlExecutionEngine::DataSource

  EMPLOYEES = [
    {"id" => 1, "name" => "Alice", "dept_id" => 1,    "salary" => 90_000, "active" => true},
    {"id" => 2, "name" => "Bob",   "dept_id" => 2,    "salary" => 75_000, "active" => true},
    {"id" => 3, "name" => "Carol", "dept_id" => 1,    "salary" => 95_000, "active" => false},
    {"id" => 4, "name" => "Dave",  "dept_id" => nil,  "salary" => 60_000, "active" => true}
  ].freeze

  DEPARTMENTS = [
    {"id" => 1, "name" => "Engineering", "budget" => 500_000},
    {"id" => 2, "name" => "Marketing",   "budget" => 200_000}
  ].freeze

  def schema(table_name)
    case table_name
    when "employees"   then ["id", "name", "dept_id", "salary", "active"]
    when "departments" then ["id", "name", "budget"]
    else raise CodingAdventures::SqlExecutionEngine::TableNotFoundError.new(table_name)
    end
  end

  def scan(table_name)
    case table_name
    when "employees"   then EMPLOYEES.map(&:dup)
    when "departments" then DEPARTMENTS.map(&:dup)
    else raise CodingAdventures::SqlExecutionEngine::TableNotFoundError.new(table_name)
    end
  end
end

SOURCE = InMemorySource.new
EE = CodingAdventures::SqlExecutionEngine

def run_sql(sql)
  EE.execute(sql, SOURCE)
end

# ================================================================
# Test class
# ================================================================

class TestSqlExecutionEngine < Minitest::Test
  # ------------------------------------------------------------------
  # Test 1: SELECT *
  # ------------------------------------------------------------------

  def test_select_star_returns_all_rows
    result = run_sql("SELECT * FROM employees")
    assert_equal 4, result.rows.size
  end

  def test_select_star_includes_all_columns
    result = run_sql("SELECT * FROM employees")
    assert_includes result.columns, "name"
    assert_includes result.columns, "salary"
  end

  # ------------------------------------------------------------------
  # Test 2: SELECT specific columns
  # ------------------------------------------------------------------

  def test_select_specific_columns
    result = run_sql("SELECT id, name FROM employees")
    assert_equal ["id", "name"], result.columns
    assert_equal 4, result.rows.size
  end

  # ------------------------------------------------------------------
  # Test 3: AS alias
  # ------------------------------------------------------------------

  def test_as_alias_renames_column
    result = run_sql("SELECT id, name AS employee_name FROM employees")
    assert_includes result.columns, "employee_name"
    refute_includes result.columns, "name"
  end

  # ------------------------------------------------------------------
  # Test 4: WHERE numeric comparison
  # ------------------------------------------------------------------

  def test_where_salary_greater_than
    result = run_sql("SELECT name FROM employees WHERE salary > 80000")
    names = result.rows.map { |r| r["name"] }.to_set
    assert_equal Set["Alice", "Carol"], names
  end

  # ------------------------------------------------------------------
  # Test 5: WHERE boolean
  # ------------------------------------------------------------------

  def test_where_active_true
    result = run_sql("SELECT name FROM employees WHERE active = TRUE")
    names = result.rows.map { |r| r["name"] }.to_set
    assert_equal Set["Alice", "Bob", "Dave"], names
  end

  # ------------------------------------------------------------------
  # Test 6: WHERE IS NULL
  # ------------------------------------------------------------------

  def test_where_is_null
    result = run_sql("SELECT name FROM employees WHERE dept_id IS NULL")
    assert_equal 1, result.rows.size
    assert_equal "Dave", result.rows[0]["name"]
  end

  # ------------------------------------------------------------------
  # Test 7: WHERE IS NOT NULL
  # ------------------------------------------------------------------

  def test_where_is_not_null
    result = run_sql("SELECT name FROM employees WHERE dept_id IS NOT NULL")
    names = result.rows.map { |r| r["name"] }
    refute_includes names, "Dave"
    assert_equal 3, result.rows.size
  end

  # ------------------------------------------------------------------
  # Test 8: WHERE BETWEEN
  # ------------------------------------------------------------------

  def test_where_between
    result = run_sql("SELECT name FROM employees WHERE salary BETWEEN 70000 AND 90000")
    names = result.rows.map { |r| r["name"] }.to_set
    assert_equal Set["Alice", "Bob"], names
  end

  # ------------------------------------------------------------------
  # Test 9: WHERE IN
  # ------------------------------------------------------------------

  def test_where_in
    result = run_sql("SELECT name FROM employees WHERE id IN (1, 3)")
    names = result.rows.map { |r| r["name"] }.to_set
    assert_equal Set["Alice", "Carol"], names
  end

  def test_where_not_in
    result = run_sql("SELECT name FROM employees WHERE id NOT IN (1, 3)")
    names = result.rows.map { |r| r["name"] }.to_set
    assert_equal Set["Bob", "Dave"], names
  end

  # ------------------------------------------------------------------
  # Test 10: WHERE LIKE
  # ------------------------------------------------------------------

  def test_where_like_prefix
    result = run_sql("SELECT name FROM employees WHERE name LIKE 'A%'")
    names = result.rows.map { |r| r["name"] }
    assert_equal ["Alice"], names
  end

  def test_where_like_suffix
    result = run_sql("SELECT name FROM employees WHERE name LIKE '%ob'")
    names = result.rows.map { |r| r["name"] }
    assert_equal ["Bob"], names
  end

  # ------------------------------------------------------------------
  # Test 11: WHERE AND / OR / NOT
  # ------------------------------------------------------------------

  def test_where_and
    result = run_sql("SELECT name FROM employees WHERE salary > 70000 AND active = TRUE")
    names = result.rows.map { |r| r["name"] }.to_set
    assert_equal Set["Alice", "Bob"], names
  end

  def test_where_not
    result = run_sql("SELECT name FROM employees WHERE NOT active = TRUE")
    names = result.rows.map { |r| r["name"] }
    assert_equal ["Carol"], names
  end

  # ------------------------------------------------------------------
  # Test 12: ORDER BY
  # ------------------------------------------------------------------

  def test_order_by_salary_desc
    result = run_sql("SELECT name FROM employees ORDER BY salary DESC")
    names = result.rows.map { |r| r["name"] }
    assert_equal "Carol", names[0]
    assert_equal "Dave", names[-1]
  end

  def test_order_by_name_asc
    result = run_sql("SELECT name FROM employees ORDER BY name ASC")
    names = result.rows.map { |r| r["name"] }
    assert_equal names.sort, names
  end

  # ------------------------------------------------------------------
  # Test 13: LIMIT and OFFSET
  # ------------------------------------------------------------------

  def test_limit
    result = run_sql("SELECT id FROM employees LIMIT 2")
    assert_equal 2, result.rows.size
  end

  def test_limit_offset
    all_ids = run_sql("SELECT id FROM employees ORDER BY id ASC").rows.map { |r| r["id"] }
    page    = run_sql("SELECT id FROM employees ORDER BY id ASC LIMIT 2 OFFSET 1").rows.map { |r| r["id"] }
    assert_equal 2, page.size
    assert_equal all_ids[1], page[0]
  end

  # ------------------------------------------------------------------
  # Test 14: SELECT DISTINCT
  # ------------------------------------------------------------------

  def test_select_distinct
    result = run_sql("SELECT DISTINCT dept_id FROM employees")
    # 3 distinct values: 1, 2, nil
    assert_equal 3, result.rows.size
  end

  # ------------------------------------------------------------------
  # Test 15: INNER JOIN
  # ------------------------------------------------------------------

  def test_inner_join
    result = run_sql(
      "SELECT employees.name, departments.name " \
      "FROM employees INNER JOIN departments " \
      "ON employees.dept_id = departments.id"
    )
    # Dave has NULL dept_id → excluded
    assert_equal 3, result.rows.size
  end

  # ------------------------------------------------------------------
  # Test 16: LEFT JOIN
  # ------------------------------------------------------------------

  def test_left_join_includes_dave
    result = run_sql(
      "SELECT employees.name " \
      "FROM employees LEFT JOIN departments " \
      "ON employees.dept_id = departments.id"
    )
    names = result.rows.map { |r| r["employees.name"] }
    assert_includes names, "Dave"
    assert_equal 4, result.rows.size
  end

  def test_left_join_null_for_dave_dept
    result = run_sql(
      "SELECT employees.name, departments.name AS dept_name " \
      "FROM employees LEFT JOIN departments " \
      "ON employees.dept_id = departments.id"
    )
    dave_row = result.rows.find { |r| r["employees.name"] == "Dave" }
    assert_nil dave_row["dept_name"]
  end

  # ------------------------------------------------------------------
  # Test 17: COUNT(*) and AVG
  # ------------------------------------------------------------------

  def test_count_star
    result = run_sql("SELECT COUNT(*) FROM employees")
    val = result.rows[0].values.first
    assert_equal 4, val
  end

  def test_avg_salary
    result = run_sql("SELECT AVG(salary) FROM employees")
    val = result.rows[0].values.first
    expected = (90_000 + 75_000 + 95_000 + 60_000) / 4.0
    assert_in_delta expected, val, 0.01
  end

  # ------------------------------------------------------------------
  # Test 18: GROUP BY with COUNT and SUM
  # ------------------------------------------------------------------

  def test_group_by_count
    result = run_sql(
      "SELECT dept_id, COUNT(*) FROM employees GROUP BY dept_id"
    )
    assert_equal 3, result.rows.size
  end

  def test_group_by_sum
    result = run_sql(
      "SELECT dept_id, SUM(salary) " \
      "FROM employees " \
      "WHERE dept_id IS NOT NULL " \
      "GROUP BY dept_id"
    )
    rows_by_dept = result.rows.each_with_object({}) { |r, h| h[r["dept_id"]] = r }
    assert_equal 185_000, rows_by_dept[1]["SUM(salary)"]
    assert_equal 75_000,  rows_by_dept[2]["SUM(salary)"]
  end

  # ------------------------------------------------------------------
  # Test 19: HAVING
  # ------------------------------------------------------------------

  def test_having_sum_filter
    result = run_sql(
      "SELECT dept_id, SUM(salary) " \
      "FROM employees " \
      "WHERE dept_id IS NOT NULL " \
      "GROUP BY dept_id " \
      "HAVING SUM(salary) > 100000"
    )
    assert_equal 1, result.rows.size
    assert_equal 1, result.rows[0]["dept_id"]
  end

  # ------------------------------------------------------------------
  # Test 20: Arithmetic in SELECT
  # ------------------------------------------------------------------

  def test_salary_times_constant
    result = run_sql("SELECT salary * 1.1 AS adjusted FROM employees WHERE id = 1")
    assert_in_delta 99_000.0, result.rows[0]["adjusted"], 0.01
  end

  # ------------------------------------------------------------------
  # Test 21: TableNotFoundError
  # ------------------------------------------------------------------

  def test_table_not_found
    assert_raises(CodingAdventures::SqlExecutionEngine::TableNotFoundError) do
      run_sql("SELECT * FROM nonexistent")
    end
  end

  def test_table_not_found_is_execution_error
    assert_raises(CodingAdventures::SqlExecutionEngine::ExecutionError) do
      run_sql("SELECT * FROM nonexistent")
    end
  end

  # ------------------------------------------------------------------
  # Test 22: ColumnNotFoundError
  # ------------------------------------------------------------------

  def test_column_not_found
    assert_raises(CodingAdventures::SqlExecutionEngine::ColumnNotFoundError) do
      run_sql("SELECT id FROM employees WHERE fake_col = 1")
    end
  end

  # ------------------------------------------------------------------
  # Test 23: execute_all
  # ------------------------------------------------------------------

  def test_execute_all_multiple_statements
    results = EE.execute_all(
      "SELECT id FROM employees; SELECT id FROM departments",
      SOURCE
    )
    assert_equal 2, results.size
    assert_equal 4, results[0].rows.size
    assert_equal 2, results[1].rows.size
  end

  # ------------------------------------------------------------------
  # Test 24: QueryResult
  # ------------------------------------------------------------------

  def test_query_result_to_s
    result = run_sql("SELECT id FROM employees WHERE id = 1")
    assert_match(/1 row/, result.to_s)
  end

  # ------------------------------------------------------------------
  # Test 25: MIN / MAX
  # ------------------------------------------------------------------

  def test_min_salary
    result = run_sql("SELECT MIN(salary) FROM employees")
    val = result.rows[0].values.first
    assert_equal 60_000, val
  end

  def test_max_salary
    result = run_sql("SELECT MAX(salary) FROM employees")
    val = result.rows[0].values.first
    assert_equal 95_000, val
  end

  # ------------------------------------------------------------------
  # Test 26: COUNT(col) skips NULLs
  # ------------------------------------------------------------------

  def test_count_column_skips_null
    result = run_sql("SELECT COUNT(dept_id) FROM employees")
    val = result.rows[0].values.first
    assert_equal 3, val
  end

  # ------------------------------------------------------------------
  # Test 27: Error class attributes
  # ------------------------------------------------------------------

  def test_table_not_found_attributes
    err = CodingAdventures::SqlExecutionEngine::TableNotFoundError.new("mytable")
    assert_equal "mytable", err.table_name
    assert_match(/mytable/, err.message)
  end

  def test_column_not_found_attributes
    err = CodingAdventures::SqlExecutionEngine::ColumnNotFoundError.new("mycol")
    assert_equal "mycol", err.column_name
    assert_match(/mycol/, err.message)
  end
end
