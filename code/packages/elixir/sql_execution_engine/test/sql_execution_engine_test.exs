defmodule CodingAdventures.SqlExecutionEngineTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.SqlExecutionEngine
  alias CodingAdventures.SqlExecutionEngine.{Result, Expression}
  alias CodingAdventures.SqlExecutionEngine.Errors.TableNotFoundError

  # ---------------------------------------------------------------------------
  # In-memory test data source
  # ---------------------------------------------------------------------------
  #
  # This module implements the DataSource behaviour using hardcoded Elixir data.
  # It represents two tables: `employees` and `departments`.
  #
  # employees:
  #   id | name  | dept_id | salary | active
  #    1 | Alice |       1 |  90000 |   true
  #    2 | Bob   |       2 |  75000 |   true
  #    3 | Carol |       1 |  95000 |  false
  #    4 | Dave  |    NULL |  60000 |   true
  #
  # departments:
  #   id | name        | budget
  #    1 | Engineering | 500000
  #    2 | Marketing   | 200000

  defmodule InMemorySource do
    @behaviour CodingAdventures.SqlExecutionEngine.DataSource

    @employees [
      %{"id" => 1, "name" => "Alice", "dept_id" => 1, "salary" => 90000, "active" => true},
      %{"id" => 2, "name" => "Bob",   "dept_id" => 2, "salary" => 75000, "active" => true},
      %{"id" => 3, "name" => "Carol", "dept_id" => 1, "salary" => 95000, "active" => false},
      %{"id" => 4, "name" => "Dave",  "dept_id" => nil, "salary" => 60000, "active" => true}
    ]

    @departments [
      %{"id" => 1, "name" => "Engineering", "budget" => 500_000},
      %{"id" => 2, "name" => "Marketing",   "budget" => 200_000}
    ]

    @impl true
    def schema("employees"),   do: ["id", "name", "dept_id", "salary", "active"]
    def schema("departments"), do: ["id", "name", "budget"]
    def schema(t),             do: raise(TableNotFoundError, t)

    @impl true
    def scan("employees"),   do: @employees
    def scan("departments"), do: @departments
    def scan(t),             do: raise(TableNotFoundError, t)
  end

  # Shorthand helper — execute and assert success.
  defp query!(sql) do
    case SqlExecutionEngine.execute(sql, InMemorySource) do
      {:ok, result} -> result
      {:error, msg} -> flunk("Query failed: #{msg}\nSQL: #{sql}")
    end
  end

  # ---------------------------------------------------------------------------
  # 1. SELECT * FROM employees — full scan
  # ---------------------------------------------------------------------------

  describe "SELECT * FROM" do
    test "returns all columns for all rows" do
      result = query!("SELECT * FROM employees")

      assert %Result{} = result
      assert length(result.rows) == 4

      # All employee columns should be present (order stable from schema)
      assert "id" in result.columns
      assert "name" in result.columns
      assert "salary" in result.columns
      assert "active" in result.columns
      assert "dept_id" in result.columns
    end

    test "row values match source data" do
      result = query!("SELECT * FROM employees")
      id_idx = Enum.find_index(result.columns, &(&1 == "id"))
      name_idx = Enum.find_index(result.columns, &(&1 == "name"))

      row1 = hd(result.rows)
      assert Enum.at(row1, id_idx) == 1
      assert Enum.at(row1, name_idx) == "Alice"
    end

    test "departments full scan" do
      result = query!("SELECT * FROM departments")
      assert length(result.rows) == 2
      assert "budget" in result.columns
    end
  end

  # ---------------------------------------------------------------------------
  # 2. Column projection
  # ---------------------------------------------------------------------------

  describe "column projection" do
    test "SELECT id, name returns only those columns" do
      result = query!("SELECT id, name FROM employees")

      assert result.columns == ["id", "name"]
      assert length(result.rows) == 4
      assert hd(result.rows) == [1, "Alice"]
    end

    test "projected rows have correct length" do
      result = query!("SELECT id, name FROM employees")
      assert Enum.all?(result.rows, fn row -> length(row) == 2 end)
    end
  end

  # ---------------------------------------------------------------------------
  # 3. Column aliases
  # ---------------------------------------------------------------------------

  describe "column aliases (AS)" do
    test "AS renames the output column" do
      result = query!("SELECT id, name AS employee_name FROM employees")

      assert result.columns == ["id", "employee_name"]
      assert hd(result.rows) == [1, "Alice"]
    end

    test "multiple aliases" do
      result = query!("SELECT id AS emp_id, salary AS pay FROM employees")

      assert result.columns == ["emp_id", "pay"]
    end
  end

  # ---------------------------------------------------------------------------
  # 4. WHERE with numeric comparison
  # ---------------------------------------------------------------------------

  describe "WHERE numeric comparison" do
    test "salary > 80000 filters correctly" do
      result = query!("SELECT * FROM employees WHERE salary > 80000")

      names = extract_col(result, "name")
      assert "Alice" in names
      assert "Carol" in names
      refute "Bob" in names
      refute "Dave" in names
    end

    test "salary = 75000 returns one row" do
      result = query!("SELECT id FROM employees WHERE salary = 75000")
      assert length(result.rows) == 1
    end

    test "salary >= 90000 includes boundary" do
      result = query!("SELECT id FROM employees WHERE salary >= 90000")
      assert length(result.rows) == 2
    end

    test "salary < 70000 returns Dave" do
      result = query!("SELECT name FROM employees WHERE salary < 70000")
      assert result.rows == [["Dave"]]
    end
  end

  # ---------------------------------------------------------------------------
  # 5. WHERE with boolean column
  # ---------------------------------------------------------------------------

  describe "WHERE boolean column" do
    test "active = true returns only active employees" do
      result = query!("SELECT name FROM employees WHERE active = true")

      names = Enum.map(result.rows, &hd/1)
      assert "Alice" in names
      assert "Bob" in names
      assert "Dave" in names
      refute "Carol" in names
    end

    test "active = false returns Carol" do
      result = query!("SELECT name FROM employees WHERE active = false")
      assert result.rows == [["Carol"]]
    end
  end

  # ---------------------------------------------------------------------------
  # 6. WHERE IS NULL
  # ---------------------------------------------------------------------------

  describe "WHERE IS NULL" do
    test "dept_id IS NULL returns Dave" do
      result = query!("SELECT name FROM employees WHERE dept_id IS NULL")
      assert result.rows == [["Dave"]]
    end
  end

  # ---------------------------------------------------------------------------
  # 7. WHERE IS NOT NULL
  # ---------------------------------------------------------------------------

  describe "WHERE IS NOT NULL" do
    test "dept_id IS NOT NULL excludes Dave" do
      result = query!("SELECT name FROM employees WHERE dept_id IS NOT NULL")

      names = Enum.map(result.rows, &hd/1)
      assert "Alice" in names
      assert "Bob" in names
      assert "Carol" in names
      refute "Dave" in names
    end
  end

  # ---------------------------------------------------------------------------
  # 8. WHERE BETWEEN
  # ---------------------------------------------------------------------------

  describe "WHERE BETWEEN" do
    test "salary BETWEEN 70000 AND 90000 includes boundaries" do
      result = query!("SELECT name FROM employees WHERE salary BETWEEN 70000 AND 90000")

      names = Enum.map(result.rows, &hd/1)
      # Alice (90000), Bob (75000) are in range; Carol (95000) and Dave (60000) are not
      assert "Alice" in names
      assert "Bob" in names
      refute "Carol" in names
      refute "Dave" in names
    end
  end

  # ---------------------------------------------------------------------------
  # 9. WHERE IN
  # ---------------------------------------------------------------------------

  describe "WHERE IN" do
    test "id IN (1, 3) returns Alice and Carol" do
      result = query!("SELECT name FROM employees WHERE id IN (1, 3)")

      names = Enum.map(result.rows, &hd/1)
      assert "Alice" in names
      assert "Carol" in names
      assert length(result.rows) == 2
    end
  end

  # ---------------------------------------------------------------------------
  # 10. WHERE LIKE
  # ---------------------------------------------------------------------------

  describe "WHERE LIKE" do
    test "name LIKE 'A%' returns Alice" do
      result = query!("SELECT name FROM employees WHERE name LIKE 'A%'")
      assert result.rows == [["Alice"]]
    end

    test "name LIKE '%o%' returns Bob and Carol (both contain 'o')" do
      result = query!("SELECT name FROM employees WHERE name LIKE '%o%'")
      # Bob contains 'o', Carol contains 'o' (Care-o-l)
      names = Enum.map(result.rows, &hd/1) |> Enum.sort()
      assert names == ["Bob", "Carol"]
    end

    test "name LIKE '_o_' does not match Alice" do
      result = query!("SELECT name FROM employees WHERE name LIKE '_o_'")
      assert result.rows == [["Bob"]]
    end
  end

  # ---------------------------------------------------------------------------
  # 11. WHERE with AND
  # ---------------------------------------------------------------------------

  describe "WHERE AND" do
    test "active = true AND salary > 80000" do
      result = query!("SELECT name FROM employees WHERE active = true AND salary > 80000")

      names = Enum.map(result.rows, &hd/1)
      # Alice: active=true, salary=90000 ✓
      # Bob: active=true, salary=75000 ✗
      # Carol: active=false ✗
      # Dave: active=true, salary=60000 ✗
      assert names == ["Alice"]
    end
  end

  # ---------------------------------------------------------------------------
  # 12. WHERE with OR
  # ---------------------------------------------------------------------------

  describe "WHERE OR" do
    test "active = false OR salary > 90000" do
      result = query!("SELECT name FROM employees WHERE active = false OR salary > 90000")

      names = Enum.map(result.rows, &hd/1)
      # Carol: active=false ✓; salary=95000 → also > 90000
      # No one else has salary > 90000
      assert names == ["Carol"]
    end

    test "id = 1 OR id = 2" do
      result = query!("SELECT id FROM employees WHERE id = 1 OR id = 2")
      ids = Enum.map(result.rows, &hd/1) |> Enum.sort()
      assert ids == [1, 2]
    end
  end

  # ---------------------------------------------------------------------------
  # 13. WHERE NOT
  # ---------------------------------------------------------------------------

  describe "WHERE NOT" do
    test "NOT active returns Carol" do
      result = query!("SELECT name FROM employees WHERE NOT active")
      assert result.rows == [["Carol"]]
    end
  end

  # ---------------------------------------------------------------------------
  # 14. ORDER BY DESC
  # ---------------------------------------------------------------------------

  describe "ORDER BY DESC" do
    test "ORDER BY salary DESC returns highest salary first" do
      result = query!("SELECT name, salary FROM employees ORDER BY salary DESC")

      salaries = Enum.map(result.rows, fn [_name, sal] -> sal end)
      assert salaries == [95000, 90000, 75000, 60000]
    end
  end

  # ---------------------------------------------------------------------------
  # 15. ORDER BY ASC
  # ---------------------------------------------------------------------------

  describe "ORDER BY ASC" do
    test "ORDER BY name ASC returns alphabetical order" do
      result = query!("SELECT name FROM employees ORDER BY name ASC")

      names = Enum.map(result.rows, &hd/1)
      assert names == ["Alice", "Bob", "Carol", "Dave"]
    end

    test "ORDER BY without explicit direction defaults to ASC" do
      result = query!("SELECT name FROM employees ORDER BY name")
      names = Enum.map(result.rows, &hd/1)
      assert names == ["Alice", "Bob", "Carol", "Dave"]
    end
  end

  # ---------------------------------------------------------------------------
  # 16. LIMIT
  # ---------------------------------------------------------------------------

  describe "LIMIT" do
    test "LIMIT 2 returns first 2 rows" do
      result = query!("SELECT id FROM employees LIMIT 2")
      assert length(result.rows) == 2
    end

    test "LIMIT greater than row count returns all rows" do
      result = query!("SELECT id FROM employees LIMIT 100")
      assert length(result.rows) == 4
    end
  end

  # ---------------------------------------------------------------------------
  # 17. LIMIT with OFFSET
  # ---------------------------------------------------------------------------

  describe "LIMIT OFFSET" do
    test "LIMIT 2 OFFSET 1 skips first row" do
      result = query!("SELECT id FROM employees ORDER BY id ASC LIMIT 2 OFFSET 1")
      ids = Enum.map(result.rows, &hd/1)
      # Sorted by id: [1, 2, 3, 4]; skip 1, take 2 → [2, 3]
      assert ids == [2, 3]
    end

    test "OFFSET beyond row count returns empty" do
      result = query!("SELECT id FROM employees LIMIT 2 OFFSET 100")
      assert result.rows == []
    end
  end

  # ---------------------------------------------------------------------------
  # 18. DISTINCT
  # ---------------------------------------------------------------------------

  describe "SELECT DISTINCT" do
    test "DISTINCT dept_id returns unique values" do
      result = query!("SELECT DISTINCT dept_id FROM employees")

      # dept_ids: 1, 2, 1, nil → distinct: [1, 2, nil]
      dept_ids = Enum.map(result.rows, &hd/1) |> Enum.sort(&compare_with_nil/2)
      assert length(dept_ids) == 3
      assert 1 in dept_ids
      assert 2 in dept_ids
      assert nil in dept_ids
    end

    test "DISTINCT on non-duplicate column returns all rows" do
      result = query!("SELECT DISTINCT id FROM employees")
      assert length(result.rows) == 4
    end
  end

  # ---------------------------------------------------------------------------
  # 19. INNER JOIN
  # ---------------------------------------------------------------------------

  describe "INNER JOIN" do
    test "INNER JOIN on dept_id = d.id" do
      result =
        query!("""
          SELECT e.name, d.name
          FROM employees AS e
          INNER JOIN departments AS d ON e.dept_id = d.id
        """)

      # Dave has dept_id = NULL, so he is excluded from INNER JOIN.
      # Alice (dept 1 = Engineering), Bob (dept 2 = Marketing), Carol (dept 1 = Engineering)
      assert length(result.rows) == 3
      assert result.columns == ["e.name", "d.name"]

      rows_sorted = Enum.sort(result.rows)
      assert ["Alice", "Engineering"] in rows_sorted
      assert ["Bob", "Marketing"] in rows_sorted
      assert ["Carol", "Engineering"] in rows_sorted
    end
  end

  # ---------------------------------------------------------------------------
  # 20. LEFT JOIN
  # ---------------------------------------------------------------------------

  describe "LEFT JOIN" do
    test "LEFT JOIN includes Dave with NULL department" do
      result =
        query!("""
          SELECT e.name, d.name
          FROM employees AS e
          LEFT JOIN departments AS d ON e.dept_id = d.id
        """)

      # All 4 employees; Dave's d.name is NULL
      assert length(result.rows) == 4

      dave_row = Enum.find(result.rows, fn [name | _] -> name == "Dave" end)
      assert dave_row == ["Dave", nil]
    end
  end

  # ---------------------------------------------------------------------------
  # 21. COUNT(*)
  # ---------------------------------------------------------------------------

  describe "COUNT(*)" do
    test "COUNT(*) from employees returns 4" do
      result = query!("SELECT COUNT(*) FROM employees")

      assert result.columns == ["COUNT(*)"]
      assert result.rows == [[4]]
    end
  end

  # ---------------------------------------------------------------------------
  # 22. COUNT(*) AS alias, AVG(salary) AS alias
  # ---------------------------------------------------------------------------

  describe "COUNT and AVG with aliases" do
    test "COUNT(*) AS total, AVG(salary) AS avg_sal" do
      result = query!("SELECT COUNT(*) AS total, AVG(salary) AS avg_sal FROM employees")

      assert result.columns == ["total", "avg_sal"]
      [[total, avg_sal]] = result.rows
      assert total == 4
      # (90000 + 75000 + 95000 + 60000) / 4 = 320000 / 4 = 80000.0
      assert avg_sal == 80000.0
    end
  end

  # ---------------------------------------------------------------------------
  # 23. GROUP BY with COUNT and SUM
  # ---------------------------------------------------------------------------

  describe "GROUP BY" do
    test "GROUP BY dept_id with COUNT and SUM" do
      result =
        query!("""
          SELECT dept_id, COUNT(*) AS cnt, SUM(salary) AS total
          FROM employees
          GROUP BY dept_id
        """)

      assert result.columns == ["dept_id", "cnt", "total"]

      # 3 groups: dept_id = 1, 2, nil
      assert length(result.rows) == 3

      # Find the group for dept_id = 1 (Alice + Carol)
      dept1 = Enum.find(result.rows, fn [dept_id | _] -> dept_id == 1 end)
      assert dept1 != nil
      [_, cnt1, total1] = dept1
      assert cnt1 == 2
      assert total1 == 185_000

      # Find the group for dept_id = 2 (Bob)
      dept2 = Enum.find(result.rows, fn [dept_id | _] -> dept_id == 2 end)
      [_, cnt2, total2] = dept2
      assert cnt2 == 1
      assert total2 == 75_000
    end
  end

  # ---------------------------------------------------------------------------
  # 24. GROUP BY with HAVING
  # ---------------------------------------------------------------------------

  describe "HAVING" do
    test "HAVING SUM(salary) > 100000 keeps only dept 1" do
      result =
        query!("""
          SELECT dept_id, SUM(salary) AS total
          FROM employees
          GROUP BY dept_id
          HAVING SUM(salary) > 100000
        """)

      # dept_id 1: 90000 + 95000 = 185000 ✓
      # dept_id 2: 75000 ✗
      # dept_id nil: 60000 ✗
      assert length(result.rows) == 1
      [[dept_id, total]] = result.rows
      assert dept_id == 1
      assert total == 185_000
    end
  end

  # ---------------------------------------------------------------------------
  # 25. Arithmetic expression in SELECT
  # ---------------------------------------------------------------------------

  describe "arithmetic expressions" do
    test "salary * 1.1 AS raised" do
      result = query!("SELECT salary * 1.1 AS raised FROM employees WHERE id = 1")

      assert result.columns == ["raised"]
      [[raised]] = result.rows
      assert_in_delta raised, 99_000.0, 0.01
    end

    test "arithmetic with addition" do
      result = query!("SELECT salary + 5000 AS bumped FROM employees WHERE id = 2")
      [[bumped]] = result.rows
      assert bumped == 80_000
    end
  end

  # ---------------------------------------------------------------------------
  # 26. Error: unknown table
  # ---------------------------------------------------------------------------

  describe "error handling" do
    test "unknown table raises TableNotFoundError" do
      assert_raise TableNotFoundError, ~r/no_such_table/, fn ->
        SqlExecutionEngine.execute("SELECT * FROM no_such_table", InMemorySource)
        |> then(fn
          {:ok, _} -> :ok
          {:error, msg} -> raise TableNotFoundError, msg
        end)
      end
    end

    test "unknown table propagates as error tuple" do
      # The execute/2 function wraps exceptions in {:error, ...}
      result = SqlExecutionEngine.execute("SELECT * FROM no_such_table", InMemorySource)
      assert {:error, msg} = result
      assert msg =~ "no_such_table"
    end
  end

  # ---------------------------------------------------------------------------
  # 27. Error: unknown column
  # ---------------------------------------------------------------------------

  describe "unknown column" do
    test "unknown column propagates as error tuple" do
      result = SqlExecutionEngine.execute("SELECT unknown_col FROM employees", InMemorySource)
      assert {:error, msg} = result
      assert msg =~ "unknown_col"
    end
  end

  # ---------------------------------------------------------------------------
  # Additional edge cases
  # ---------------------------------------------------------------------------

  describe "edge cases" do
    test "empty result from WHERE that matches nothing" do
      result = query!("SELECT * FROM employees WHERE id = 999")
      assert result.rows == []
    end

    test "SELECT with literal number" do
      result = query!("SELECT 42 FROM employees LIMIT 1")
      assert result.rows == [[42]]
    end

    test "SELECT with NULL literal" do
      result = query!("SELECT * FROM employees WHERE dept_id IS NULL")
      assert length(result.rows) == 1
    end

    test "WHERE NOT active = false (double negation)" do
      result = query!("SELECT name FROM employees WHERE NOT active = false")
      names = Enum.map(result.rows, &hd/1)
      # NOT (active = false) = active = true
      assert "Alice" in names
      assert "Bob" in names
      assert "Dave" in names
      refute "Carol" in names
    end

    test "ORDER BY with multiple columns" do
      result = query!("SELECT dept_id, name FROM employees ORDER BY dept_id ASC, name ASC")
      # Sort by dept_id (NULLs first in our impl), then name
      # NULL, 1, 1, 2 → [Dave, Alice, Carol, Bob]
      names = Enum.map(result.rows, fn [_d, name] -> name end)
      assert names == ["Dave", "Alice", "Carol", "Bob"]
    end

    test "CROSS JOIN produces all combinations" do
      # The grammar requires an ON clause for all join types.
      # CROSS JOIN with ON 1=1 is equivalent to a cartesian product.
      result =
        query!("""
          SELECT e.name, d.name
          FROM employees AS e
          CROSS JOIN departments AS d ON 1 = 1
        """)

      # 4 employees × 2 departments = 8 rows
      assert length(result.rows) == 8
    end

    test "MIN and MAX aggregates" do
      result = query!("SELECT MIN(salary) AS lo, MAX(salary) AS hi FROM employees")
      [[lo, hi]] = result.rows
      assert lo == 60_000
      assert hi == 95_000
    end

    test "COUNT(col) ignores NULLs" do
      result = query!("SELECT COUNT(dept_id) AS cnt FROM employees")
      [[cnt]] = result.rows
      # Dave's dept_id is NULL, so count is 3
      assert cnt == 3
    end

    test "SUM over NULL-containing column ignores NULLs" do
      # dept_id: 1, 2, 1, nil → SUM = 4
      result = query!("SELECT SUM(dept_id) AS s FROM employees")
      [[s]] = result.rows
      assert s == 4
    end
  end

  # ---------------------------------------------------------------------------
  # Three-valued logic unit tests (Expression module)
  # ---------------------------------------------------------------------------

  describe "three-valued logic" do
    test "sql_and: false AND nil = false" do
      assert Expression.sql_and(false, nil) == false
    end

    test "sql_and: nil AND false = false" do
      assert Expression.sql_and(nil, false) == false
    end

    test "sql_and: true AND nil = nil" do
      assert Expression.sql_and(true, nil) == nil
    end

    test "sql_and: true AND true = true" do
      assert Expression.sql_and(true, true) == true
    end

    test "sql_or: true OR nil = true" do
      assert Expression.sql_or(true, nil) == true
    end

    test "sql_or: false OR nil = nil" do
      assert Expression.sql_or(false, nil) == nil
    end

    test "sql_or: false OR false = false" do
      assert Expression.sql_or(false, false) == false
    end

    test "sql_not: NOT nil = nil" do
      assert Expression.sql_not(nil) == nil
    end

    test "sql_not: NOT true = false" do
      assert Expression.sql_not(true) == false
    end

    test "sql_not: NOT false = true" do
      assert Expression.sql_not(false) == true
    end
  end

  # ---------------------------------------------------------------------------
  # execute_all/2
  # ---------------------------------------------------------------------------

  describe "execute_all/2" do
    test "executes multiple statements separated by semicolons" do
      {:ok, results} =
        SqlExecutionEngine.execute_all(
          "SELECT COUNT(*) FROM employees; SELECT COUNT(*) FROM departments",
          InMemorySource
        )

      assert length(results) == 2
      [r1, r2] = results
      assert r1.rows == [[4]]
      assert r2.rows == [[2]]
    end
  end

  # ---------------------------------------------------------------------------
  # UnsupportedQueryError
  # ---------------------------------------------------------------------------

  describe "unsupported statements" do
    test "INSERT raises UnsupportedQueryError wrapped in error tuple" do
      result =
        SqlExecutionEngine.execute(
          "INSERT INTO employees VALUES (5, 'Eve', 1, 85000, true)",
          InMemorySource
        )

      assert {:error, msg} = result
      assert msg =~ "insert_stmt" or msg =~ "Unsupported"
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp extract_col(%Result{columns: columns, rows: rows}, col_name) do
    idx = Enum.find_index(columns, &(&1 == col_name))
    Enum.map(rows, &Enum.at(&1, idx))
  end

  # Comparator for sorting that places nil values at the end.
  defp compare_with_nil(nil, nil), do: true
  defp compare_with_nil(nil, _), do: false
  defp compare_with_nil(_, nil), do: true
  defp compare_with_nil(a, b), do: a <= b
end
