defmodule CodingAdventures.SqlCsvSourceTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.SqlCsvSource
  alias CodingAdventures.SqlCsvSource.CsvDataSource
  alias CodingAdventures.SqlExecutionEngine.Errors.TableNotFoundError

  # ---------------------------------------------------------------------------
  # Fixture directory
  # ---------------------------------------------------------------------------
  #
  # All tests use the CSV files under test/fixtures/:
  #
  #   employees.csv:
  #     id | name  | dept_id | salary | active
  #      1 | Alice |       1 |  90000 |   true
  #      2 | Bob   |       2 |  75000 |   true
  #      3 | Carol |       1 |  95000 |  false
  #      4 | Dave  |  (nil) |  60000 |   true   ← dept_id is empty → nil
  #
  #   departments.csv:
  #     id | name        | budget
  #      1 | Engineering | 500000
  #      2 | Marketing   | 200000
  #
  # The fixture directory path is relative to the test file's location.
  # Path.expand/2 with __DIR__ gives an absolute path that works regardless
  # of which directory `mix test` is invoked from.

  @fixtures Path.expand("fixtures", __DIR__)

  # Each test creates a fresh source module.  Because CsvDataSource.new/1
  # generates a unique module atom per call, there is no risk of test
  # interference from module redefinition.
  defp source, do: SqlCsvSource.new(@fixtures)

  # Run a query and assert success, returning the QueryResult.
  defp query!(sql) do
    case SqlCsvSource.execute(sql, source()) do
      {:ok, result} -> result
      {:error, msg} -> flunk("Query failed: #{msg}\nSQL: #{sql}")
    end
  end

  # ---------------------------------------------------------------------------
  # Test 1: SELECT * FROM employees — full table scan
  # ---------------------------------------------------------------------------
  #
  # The simplest query: read every column of every row.  This exercises:
  # - CsvDataSource schema dispatch (to expand * into column list)
  # - CsvDataSource scan dispatch (to read all rows)
  # - Type coercion for integers, booleans, and nil
  #
  # We verify:
  # a) All five columns are present and in header order.
  # b) All four employees are returned.
  # c) Values have the correct native types after coercion.

  describe "SELECT * FROM employees" do
    test "returns all five columns" do
      # The sql-execution-engine's SELECT * expansion sorts columns alphabetically
      # for determinism (Map.keys order is not guaranteed in Elixir).
      # We verify presence of all five columns, not their order.
      result = query!("SELECT * FROM employees")
      assert length(result.columns) == 5
      assert Enum.sort(result.columns) == ["active", "dept_id", "id", "name", "salary"]
    end

    test "returns all four rows" do
      result = query!("SELECT * FROM employees")
      assert length(result.rows) == 4
    end

    test "id column is coerced to integer" do
      result = query!("SELECT * FROM employees")
      id_idx = Enum.find_index(result.columns, &(&1 == "id"))
      ids = Enum.map(result.rows, &Enum.at(&1, id_idx))
      # All IDs must be integers, not strings like "1", "2", "3", "4".
      assert Enum.all?(ids, &is_integer/1)
      assert Enum.sort(ids) == [1, 2, 3, 4]
    end

    test "salary column is coerced to integer" do
      result = query!("SELECT * FROM employees")
      salary_idx = Enum.find_index(result.columns, &(&1 == "salary"))
      salaries = Enum.map(result.rows, &Enum.at(&1, salary_idx))
      assert Enum.all?(salaries, &is_integer/1)
      assert Enum.sort(salaries) == [60_000, 75_000, 90_000, 95_000]
    end

    test "active column is coerced to boolean" do
      result = query!("SELECT * FROM employees")
      active_idx = Enum.find_index(result.columns, &(&1 == "active"))
      actives = Enum.map(result.rows, &Enum.at(&1, active_idx))
      # All values must be true or false, not "true" or "false" strings.
      assert Enum.all?(actives, &is_boolean/1)
    end
  end

  # ---------------------------------------------------------------------------
  # Test 2: WHERE clause with typed values
  # ---------------------------------------------------------------------------
  #
  # SQL WHERE requires typed comparisons.  Because `active` is coerced to a
  # boolean and `salary` to an integer, the engine can evaluate:
  #   active = true     (boolean equality, not string equality)
  #   salary > 80000    (integer comparison, not lexicographic string comparison)
  #
  # Without coercion, the engine would compare the string "true" against the
  # SQL literal `true`, which would not match.

  describe "WHERE clause with typed values" do
    test "WHERE active = true returns only active employees" do
      result = query!("SELECT id, name FROM employees WHERE active = true")
      name_idx = Enum.find_index(result.columns, &(&1 == "name"))
      names = Enum.map(result.rows, &Enum.at(&1, name_idx))
      # Alice, Bob, Dave are active.  Carol is not.
      assert Enum.sort(names) == ["Alice", "Bob", "Dave"]
      refute "Carol" in names
    end

    test "WHERE salary > 80000 returns high earners" do
      result = query!("SELECT name FROM employees WHERE salary > 80000")
      name_idx = Enum.find_index(result.columns, &(&1 == "name"))
      names = Enum.map(result.rows, &Enum.at(&1, name_idx))
      # Alice (90000) and Carol (95000) qualify.
      assert Enum.sort(names) == ["Alice", "Carol"]
    end

    test "compound WHERE: active = true AND salary > 80000" do
      result =
        query!("SELECT name FROM employees WHERE active = true AND salary > 80000")

      name_idx = Enum.find_index(result.columns, &(&1 == "name"))
      names = Enum.map(result.rows, &Enum.at(&1, name_idx))
      # Only Alice: active=true AND salary=90000.
      # Carol: salary>80000 but active=false → excluded.
      # Bob: active=true but salary=75000 ≤ 80000 → excluded.
      assert names == ["Alice"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test 3: IS NULL — Dave has an empty dept_id
  # ---------------------------------------------------------------------------
  #
  # In the CSV, Dave's dept_id field is empty ("").  coerce("") returns nil.
  # The SQL engine treats nil as SQL NULL.  So `dept_id IS NULL` correctly
  # selects Dave and only Dave.
  #
  # This is the most important coercion test: if we left the empty string as ""
  # instead of converting to nil, IS NULL queries would return no rows.

  describe "IS NULL" do
    test "WHERE dept_id IS NULL returns only Dave" do
      result = query!("SELECT name FROM employees WHERE dept_id IS NULL")
      name_idx = Enum.find_index(result.columns, &(&1 == "name"))
      names = Enum.map(result.rows, &Enum.at(&1, name_idx))
      assert names == ["Dave"]
    end

    test "WHERE dept_id IS NOT NULL excludes Dave" do
      result = query!("SELECT name FROM employees WHERE dept_id IS NOT NULL")
      name_idx = Enum.find_index(result.columns, &(&1 == "name"))
      names = Enum.map(result.rows, &Enum.at(&1, name_idx))
      assert Enum.sort(names) == ["Alice", "Bob", "Carol"]
      refute "Dave" in names
    end
  end

  # ---------------------------------------------------------------------------
  # Test 4: INNER JOIN across two CSV files
  # ---------------------------------------------------------------------------
  #
  # Joining employees.csv with departments.csv exercises:
  # 1. schema dispatch for both "employees" and "departments"
  # 2. scan dispatch for both tables
  # 3. Type coercion: dept_id and departments.id must both be integers for the
  #    ON clause (e.dept_id = d.id) to match via integer equality
  # 4. Three-valued NULL logic: Dave's dept_id=nil → no match → excluded
  #
  # INNER JOIN excludes rows where the join key is NULL — this is correct
  # SQL semantics implemented by the execution engine.

  describe "INNER JOIN" do
    test "employees JOIN departments returns three rows (Dave excluded)" do
      result =
        query!("""
        SELECT e.name, d.name
        FROM employees AS e
        INNER JOIN departments AS d ON e.dept_id = d.id
        """)

      # Dave has dept_id=nil; INNER JOIN on NULL produces no match.
      assert length(result.rows) == 3
    end

    test "joined rows pair employees with correct department names" do
      result =
        query!("""
        SELECT e.name, d.name
        FROM employees AS e
        INNER JOIN departments AS d ON e.dept_id = d.id
        ORDER BY e.name
        """)

      e_name_idx = Enum.find_index(result.columns, &(&1 == "e.name"))
      d_name_idx = Enum.find_index(result.columns, &(&1 == "d.name"))

      pairs =
        Enum.map(result.rows, fn row ->
          {Enum.at(row, e_name_idx), Enum.at(row, d_name_idx)}
        end)

      assert {"Alice", "Engineering"} in pairs
      assert {"Bob", "Marketing"} in pairs
      assert {"Carol", "Engineering"} in pairs
      refute Enum.any?(pairs, fn {name, _} -> name == "Dave" end)
    end
  end

  # ---------------------------------------------------------------------------
  # Test 5: GROUP BY with COUNT aggregate
  # ---------------------------------------------------------------------------
  #
  # GROUP BY groups rows by dept_id (integer) and COUNT(*) counts each group.
  # Dave's nil dept_id forms its own NULL group per SQL semantics.

  describe "GROUP BY / COUNT aggregate" do
    test "COUNT per dept_id returns correct group sizes" do
      result =
        query!("SELECT dept_id, COUNT(*) FROM employees GROUP BY dept_id")

      # Build a map: dept_id → count from the result rows.
      dept_idx = Enum.find_index(result.columns, &(&1 == "dept_id"))

      count_idx =
        Enum.find_index(result.columns, fn c -> String.contains?(c, "COUNT") end)

      counts =
        result.rows
        |> Enum.map(fn row -> {Enum.at(row, dept_idx), Enum.at(row, count_idx)} end)
        |> Map.new()

      # Engineering (dept_id=1): Alice + Carol = 2
      assert counts[1] == 2
      # Marketing (dept_id=2): Bob = 1
      assert counts[2] == 1
      # NULL group (dept_id=nil): Dave = 1
      assert counts[nil] == 1
    end
  end

  # ---------------------------------------------------------------------------
  # Test 6: ORDER BY + LIMIT
  # ---------------------------------------------------------------------------
  #
  # ORDER BY salary DESC sorts by integer salary (not string lexicographic).
  # LIMIT 2 keeps only the top two rows.
  # This exercises: coerce → sort (numeric) → slice.
  #
  # String sort would put "95000" before "90000" (both start with "9") but
  # "75000" before "90000" (different leading digit) — basically correct by
  # accident for these values.  A proper test would use values where string
  # and integer sort differ, but the important thing here is the pipeline works.

  describe "ORDER BY + LIMIT" do
    test "top 2 earners sorted by salary descending" do
      result =
        query!("SELECT name, salary FROM employees ORDER BY salary DESC LIMIT 2")

      assert length(result.rows) == 2

      name_idx = Enum.find_index(result.columns, &(&1 == "name"))
      salary_idx = Enum.find_index(result.columns, &(&1 == "salary"))

      names = Enum.map(result.rows, &Enum.at(&1, name_idx))
      salaries = Enum.map(result.rows, &Enum.at(&1, salary_idx))

      # Carol (95000) is first, Alice (90000) is second.
      assert names == ["Carol", "Alice"]
      assert salaries == [95_000, 90_000]
    end
  end

  # ---------------------------------------------------------------------------
  # Test 7: SELECT * FROM departments — second table
  # ---------------------------------------------------------------------------
  #
  # Verifies the adapter works for any table in the directory, not just
  # employees.  Also verifies that budget values are integers.

  describe "SELECT * FROM departments" do
    test "returns three columns" do
      # The engine sorts SELECT * columns alphabetically.
      result = query!("SELECT * FROM departments")
      assert length(result.columns) == 3
      assert Enum.sort(result.columns) == ["budget", "id", "name"]
    end

    test "returns two department rows with correct types" do
      result = query!("SELECT * FROM departments")
      assert length(result.rows) == 2

      id_idx = Enum.find_index(result.columns, &(&1 == "id"))
      budget_idx = Enum.find_index(result.columns, &(&1 == "budget"))

      ids = Enum.map(result.rows, &Enum.at(&1, id_idx))
      budgets = Enum.map(result.rows, &Enum.at(&1, budget_idx))

      assert Enum.all?(ids, &is_integer/1)
      assert Enum.all?(budgets, &is_integer/1)
      assert Enum.sort(budgets) == [200_000, 500_000]
    end
  end

  # ---------------------------------------------------------------------------
  # Test 8: Error — unknown table raises TableNotFoundError
  # ---------------------------------------------------------------------------
  #
  # Querying a table that has no corresponding CSV file must raise
  # TableNotFoundError.  The execution engine wraps raised exceptions into
  # {:error, message} tuples, so we need to trigger the raise directly through
  # CsvDataSource to observe the error type.

  describe "error: unknown table" do
    test "do_schema/2 raises TableNotFoundError for missing file" do
      assert_raise TableNotFoundError, fn ->
        CsvDataSource.do_schema(@fixtures, "no_such_table")
      end
    end

    test "do_scan/2 raises TableNotFoundError for missing file" do
      assert_raise TableNotFoundError, fn ->
        CsvDataSource.do_scan(@fixtures, "no_such_table")
      end
    end

    test "executing a query against missing table returns {:error, _}" do
      # The engine catches the TableNotFoundError and returns {:error, msg}.
      result = SqlCsvSource.execute("SELECT * FROM nonexistent", source())
      assert {:error, msg} = result
      assert String.contains?(msg, "nonexistent") or String.contains?(msg, "not found")
    end
  end

  # ---------------------------------------------------------------------------
  # Test 9: Coercion edge cases via do_scan/2
  # ---------------------------------------------------------------------------
  #
  # These tests inspect the raw output of do_scan/2 (bypassing the SQL engine)
  # to directly verify type coercion logic.

  describe "type coercion" do
    test "empty dept_id becomes nil (SQL NULL)" do
      rows = CsvDataSource.do_scan(@fixtures, "employees")
      dave = Enum.find(rows, fn r -> r["name"] == "Dave" end)
      assert dave["dept_id"] == nil
    end

    test "salary values are integers, not strings" do
      rows = CsvDataSource.do_scan(@fixtures, "employees")
      assert Enum.all?(rows, fn r -> is_integer(r["salary"]) end)
    end

    test "active=true becomes boolean true" do
      rows = CsvDataSource.do_scan(@fixtures, "employees")
      alice = Enum.find(rows, fn r -> r["name"] == "Alice" end)
      assert alice["active"] == true
    end

    test "active=false becomes boolean false" do
      rows = CsvDataSource.do_scan(@fixtures, "employees")
      carol = Enum.find(rows, fn r -> r["name"] == "Carol" end)
      assert carol["active"] == false
    end

    test "name values remain strings" do
      rows = CsvDataSource.do_scan(@fixtures, "employees")
      assert Enum.all?(rows, fn r -> is_binary(r["name"]) end)
    end

    test "id values are integers" do
      rows = CsvDataSource.do_scan(@fixtures, "employees")
      assert Enum.all?(rows, fn r -> is_integer(r["id"]) end)
    end
  end

  # ---------------------------------------------------------------------------
  # Test 10: do_schema/2 returns columns in header order
  # ---------------------------------------------------------------------------

  describe "do_schema/2 column ordering" do
    test "employees columns match CSV header order" do
      cols = CsvDataSource.do_schema(@fixtures, "employees")
      assert cols == ["id", "name", "dept_id", "salary", "active"]
    end

    test "departments columns match CSV header order" do
      cols = CsvDataSource.do_schema(@fixtures, "departments")
      assert cols == ["id", "name", "budget"]
    end
  end
end
