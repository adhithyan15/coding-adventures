# frozen_string_literal: true

require_relative "test_helper"

# =============================================================================
# Tests for CodingAdventures::SqlCsvSource::CsvDataSource
#
# These tests run end-to-end against real CSV fixture files, exercising the
# full pipeline: CSV file → CsvDataSource → sql-execution-engine → QueryResult.
#
# Fixture data:
#
#   employees.csv
#     id  | name  | dept_id | salary | active
#     1   | Alice | 1       | 90000  | true
#     2   | Bob   | 2       | 75000  | true
#     3   | Carol | 1       | 95000  | false
#     4   | Dave  | (null)  | 60000  | true
#
#   departments.csv
#     id | name        | budget
#     1  | Engineering | 500000
#     2  | Marketing   | 200000
# =============================================================================

FIXTURES = File.expand_path("fixtures", __dir__)

EE  = CodingAdventures::SqlExecutionEngine
CSV_DS = CodingAdventures::SqlCsvSource::CsvDataSource

class TestCoerce < Minitest::Test
  # Access the private coerce method via a helper source instance.
  def source
    @source ||= CSV_DS.new(FIXTURES)
  end

  def coerce(val)
    source.send(:coerce, val)
  end

  def test_empty_string_is_nil
    assert_nil coerce("")
  end

  def test_true_string
    assert_equal true, coerce("true")
  end

  def test_false_string
    assert_equal false, coerce("false")
  end

  def test_integer
    assert_equal 42, coerce("42")
    assert_instance_of Integer, coerce("42")
  end

  def test_negative_integer
    assert_equal(-7, coerce("-7"))
  end

  def test_float
    assert_in_delta 3.14, coerce("3.14"), 1e-9
    assert_instance_of Float, coerce("3.14")
  end

  def test_string_passthrough
    assert_equal "hello", coerce("hello")
  end

  def test_string_with_spaces
    assert_equal "Alice Smith", coerce("Alice Smith")
  end
end

class TestSchema < Minitest::Test
  def source
    @source ||= CSV_DS.new(FIXTURES)
  end

  def test_employees_schema
    assert_equal ["id", "name", "dept_id", "salary", "active"], source.schema("employees")
  end

  def test_departments_schema
    assert_equal ["id", "name", "budget"], source.schema("departments")
  end

  def test_unknown_table_raises
    assert_raises(CodingAdventures::SqlExecutionEngine::TableNotFoundError) do
      source.schema("nonexistent")
    end
  end
end

class TestScan < Minitest::Test
  def source
    @source ||= CSV_DS.new(FIXTURES)
  end

  def test_employees_count
    assert_equal 4, source.scan("employees").length
  end

  def test_alice_row_types
    alice = source.scan("employees").first
    assert_equal 1, alice["id"]
    assert_instance_of Integer, alice["id"]
    assert_equal "Alice", alice["name"]
    assert_equal true, alice["active"]
  end

  def test_carol_active_is_false
    carol = source.scan("employees")[2]
    assert_equal false, carol["active"]
  end

  def test_dave_dept_id_is_nil
    dave = source.scan("employees")[3]
    assert_nil dave["dept_id"]
  end

  def test_departments_budget_is_integer
    rows = source.scan("departments")
    assert_equal 500_000, rows[0]["budget"]
    assert_instance_of Integer, rows[0]["budget"]
  end

  def test_unknown_table_raises
    assert_raises(CodingAdventures::SqlExecutionEngine::TableNotFoundError) do
      source.scan("missing")
    end
  end
end

class TestExecuteSql < Minitest::Test
  def source
    @source ||= CSV_DS.new(FIXTURES)
  end

  def run_sql(sql)
    EE.execute(sql, source)
  end

  # ── Test 1: SELECT * FROM employees ──────────────────────────────────────
  def test_select_star_employees_columns
    result = run_sql("SELECT * FROM employees")
    assert_equal ["id", "name", "dept_id", "salary", "active"], result.columns
  end

  def test_select_star_employees_row_count
    result = run_sql("SELECT * FROM employees")
    assert_equal 4, result.rows.length
  end

  def test_select_star_employees_types_coerced
    result = run_sql("SELECT * FROM employees")
    alice = result.rows[0]
    assert_equal 1, alice["id"]
    assert_equal "Alice", alice["name"]
    assert_equal true, alice["active"]
    dave = result.rows[3]
    assert_nil dave["dept_id"]
  end

  # ── Test 2: WHERE active = true ──────────────────────────────────────────
  def test_select_active_employees
    result = run_sql("SELECT name FROM employees WHERE active = true")
    names = result.rows.map { |r| r["name"] }.sort
    assert_equal ["Alice", "Bob", "Dave"], names
  end

  # ── Test 3: WHERE dept_id IS NULL ────────────────────────────────────────
  def test_select_where_null
    result = run_sql("SELECT * FROM employees WHERE dept_id IS NULL")
    assert_equal 1, result.rows.length
    assert_equal "Dave", result.rows[0]["name"]
  end

  # ── Test 4: INNER JOIN ───────────────────────────────────────────────────
  def test_inner_join
    result = run_sql(
      "SELECT e.name, d.name " \
      "FROM employees AS e " \
      "INNER JOIN departments AS d ON e.dept_id = d.id"
    )
    # Alice (eng), Bob (mkt), Carol (eng) — Dave excluded (NULL dept_id)
    assert_equal 3, result.rows.length
    emp_names = result.rows.map { |r| r["e.name"] }.sort
    assert_equal ["Alice", "Bob", "Carol"], emp_names
  end

  # ── Test 5: GROUP BY ─────────────────────────────────────────────────────
  def test_group_by_dept_id
    result = run_sql(
      "SELECT dept_id, COUNT(*) AS cnt FROM employees GROUP BY dept_id"
    )
    # Three groups: 1 (Alice+Carol), 2 (Bob), NULL (Dave)
    assert_equal 3, result.rows.length
    dept1 = result.rows.find { |r| r["dept_id"] == 1 }
    assert_equal 2, dept1["cnt"]
  end

  # ── Test 6: ORDER BY + LIMIT ─────────────────────────────────────────────
  def test_order_by_salary_desc_limit_2
    result = run_sql(
      "SELECT name, salary FROM employees ORDER BY salary DESC LIMIT 2"
    )
    assert_equal 2, result.rows.length
    assert_equal "Carol", result.rows[0]["name"]
    assert_equal 95_000,  result.rows[0]["salary"]
    assert_equal "Alice", result.rows[1]["name"]
    assert_equal 90_000,  result.rows[1]["salary"]
  end

  # ── Test 7: Unknown table ────────────────────────────────────────────────
  def test_unknown_table_raises
    assert_raises(CodingAdventures::SqlExecutionEngine::TableNotFoundError) do
      run_sql("SELECT * FROM ghosts")
    end
  end
end
