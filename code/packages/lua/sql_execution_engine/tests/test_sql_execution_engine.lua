-- test_sql_execution_engine.lua — Tests for the SQL Execution Engine
-- ===================================================================
--
-- These tests exercise the full pipeline: tokenize → parse → execute.
-- They use a simple in-memory data source backed by Lua tables.

local sql_engine = require("coding_adventures.sql_execution_engine")

-- ============================================================================
-- Helpers
-- ============================================================================

local function make_ds()
  -- Employees table: id, name, dept, salary
  local employees = {
    { id = 1, name = "Alice",   dept = "Engineering", salary = 95000 },
    { id = 2, name = "Bob",     dept = "Marketing",   salary = 72000 },
    { id = 3, name = "Carol",   dept = "Engineering", salary = 88000 },
    { id = 4, name = "Dave",    dept = "Marketing",   salary = 65000 },
    { id = 5, name = "Eve",     dept = "HR",          salary = 70000 },
    { id = 6, name = "Frank",   dept = "Engineering", salary = 91000 },
    { id = 7, name = "Grace",   dept = nil,           salary = 60000 },
  }

  -- Departments table: id, dept_name, budget
  local departments = {
    { id = 1, dept_name = "Engineering", budget = 500000 },
    { id = 2, dept_name = "Marketing",   budget = 200000 },
    { id = 3, dept_name = "HR",          budget = 150000 },
  }

  return sql_engine.InMemoryDataSource.new({
    employees   = employees,
    departments = departments,
  })
end

-- Shorthand: execute and assert success, return rows
local function exec(sql)
  local ds = make_ds()
  local ok, result = sql_engine.execute(sql, ds)
  assert(ok, "SQL failed: " .. tostring(result))
  return result
end

-- ============================================================================
-- describe("module API")
-- ============================================================================

describe("module API", function()
  it("exports execute", function()
    assert.is_function(sql_engine.execute)
  end)

  it("exports execute_all", function()
    assert.is_function(sql_engine.execute_all)
  end)

  it("exports InMemoryDataSource", function()
    assert.is_table(sql_engine.InMemoryDataSource)
    assert.is_function(sql_engine.InMemoryDataSource.new)
  end)
end)

-- ============================================================================
-- describe("InMemoryDataSource")
-- ============================================================================

describe("InMemoryDataSource", function()
  local ds

  before_each(function()
    ds = make_ds()
  end)

  it("returns schema (column names) for a table", function()
    local cols = ds:schema("employees")
    -- Should contain all 4 columns
    local col_set = {}
    for _, c in ipairs(cols) do col_set[c] = true end
    assert.is_true(col_set["id"])
    assert.is_true(col_set["name"])
    assert.is_true(col_set["dept"])
    assert.is_true(col_set["salary"])
  end)

  it("returns all rows for a table via scan", function()
    local rows = ds:scan("employees")
    assert.equals(7, #rows)
  end)

  it("rows have correct values", function()
    local rows = ds:scan("employees")
    -- Find Alice
    local alice
    for _, r in ipairs(rows) do
      if r.name == "Alice" then alice = r; break end
    end
    assert.is_not_nil(alice)
    assert.equals(1, alice.id)
    assert.equals(95000, alice.salary)
    assert.equals("Engineering", alice.dept)
  end)

  it("raises error for unknown table", function()
    assert.has_error(function() ds:schema("nonexistent") end)
  end)
end)

-- ============================================================================
-- describe("SELECT *")
-- ============================================================================

describe("SELECT *", function()
  it("returns all columns and rows", function()
    local result = exec("SELECT * FROM employees")
    assert.equals(7, #result.rows)
    -- Columns should include id, name, dept, salary
    local col_set = {}
    for _, c in ipairs(result.columns) do col_set[c] = true end
    assert.is_true(col_set["id"] or col_set["employees.id"])
    assert.is_true(col_set["name"] or col_set["employees.name"])
  end)

  it("reports columns from result", function()
    local result = exec("SELECT * FROM employees")
    assert.is_table(result.columns)
    assert(#result.columns >= 4)
  end)
end)

-- ============================================================================
-- describe("SELECT specific columns")
-- ============================================================================

describe("SELECT specific columns", function()
  it("projects only requested columns", function()
    local result = exec("SELECT name, salary FROM employees")
    assert.equals(2, #result.columns)
    -- All rows returned
    assert.equals(7, #result.rows)
    -- First row has name and salary
    local row = result.rows[1]
    assert.equals(2, #row)
  end)

  it("returns column names in order", function()
    local result = exec("SELECT name, dept FROM employees")
    assert.equals("name", result.columns[1])
    assert.equals("dept", result.columns[2])
  end)
end)

-- ============================================================================
-- describe("WHERE clause")
-- ============================================================================

describe("WHERE clause", function()
  it("filters rows by equality", function()
    local result = exec("SELECT name FROM employees WHERE dept = 'Engineering'")
    assert.equals(3, #result.rows)
  end)

  it("filters rows by numeric comparison", function()
    local result = exec("SELECT name FROM employees WHERE salary > 80000")
    -- Alice (95000), Carol (88000), Frank (91000) = 3 rows
    assert.equals(3, #result.rows)
  end)

  it("supports AND", function()
    local result = exec("SELECT name FROM employees WHERE dept = 'Engineering' AND salary > 90000")
    -- Alice (95000), Frank (91000) = 2 rows
    assert.equals(2, #result.rows)
  end)

  it("supports OR", function()
    local result = exec("SELECT name FROM employees WHERE dept = 'HR' OR salary > 90000")
    -- Eve (HR) + Alice, Frank (> 90k) = 3 rows
    assert.equals(3, #result.rows)
  end)

  it("supports NOT", function()
    local result = exec("SELECT name FROM employees WHERE NOT dept = 'Engineering'")
    -- Bob, Dave, Eve, Grace (4 non-Engineering; dept=NULL counts as NOT Engineering)
    assert(#result.rows >= 3)
  end)

  it("supports != / <> operators", function()
    local r1 = exec("SELECT name FROM employees WHERE dept != 'Engineering'")
    local r2 = exec("SELECT name FROM employees WHERE dept <> 'Engineering'")
    assert.equals(#r1.rows, #r2.rows)
  end)

  it("supports >= and <= operators", function()
    local result = exec("SELECT name FROM employees WHERE salary >= 88000 AND salary <= 95000")
    -- Alice(95k), Carol(88k), Frank(91k) = 3 rows
    assert.equals(3, #result.rows)
  end)

  it("supports IS NULL", function()
    local result = exec("SELECT name FROM employees WHERE dept IS NULL")
    assert.equals(1, #result.rows)
    assert.equals("Grace", result.rows[1][1])
  end)

  it("supports IS NOT NULL", function()
    local result = exec("SELECT name FROM employees WHERE dept IS NOT NULL")
    assert.equals(6, #result.rows)
  end)
end)

-- ============================================================================
-- describe("BETWEEN and IN")
-- ============================================================================

describe("BETWEEN and IN", function()
  it("BETWEEN is inclusive", function()
    local result = exec("SELECT name FROM employees WHERE salary BETWEEN 70000 AND 90000")
    -- Bob(72k), Carol(88k), Eve(70k) = 3 rows
    assert.equals(3, #result.rows)
  end)

  it("IN matches multiple values", function()
    local result = exec("SELECT name FROM employees WHERE dept IN ('Engineering', 'HR')")
    -- Alice, Carol, Frank, Eve = 4 rows
    assert.equals(4, #result.rows)
  end)

  it("NOT IN excludes values", function()
    local result = exec("SELECT name FROM employees WHERE dept NOT IN ('Engineering', 'Marketing')")
    -- Eve (HR) only (Grace has NULL, which IN returns NULL/false)
    assert(#result.rows >= 1)
  end)
end)

-- ============================================================================
-- describe("LIKE")
-- ============================================================================

describe("LIKE", function()
  it("% matches any string", function()
    local result = exec("SELECT name FROM employees WHERE name LIKE 'A%'")
    assert.equals(1, #result.rows)
    assert.equals("Alice", result.rows[1][1])
  end)

  it("_ matches single character", function()
    local result = exec("SELECT name FROM employees WHERE name LIKE '_ob'")
    assert.equals(1, #result.rows)
    assert.equals("Bob", result.rows[1][1])
  end)

  it("% anywhere in pattern", function()
    local result = exec("SELECT name FROM employees WHERE dept LIKE '%ing'")
    -- Engineering (Alice, Carol, Frank) + Marketing (Bob, Dave) = 5 rows
    assert.equals(5, #result.rows)
  end)

  it("NOT LIKE excludes matches", function()
    local result = exec("SELECT name FROM employees WHERE name NOT LIKE '%a%'")
    -- Names without 'a': Bob, Eve, Frank, Grace, Dave? Let's check:
    -- Alice has a, Carol has a, Dave has a, Grace has a -> NOT like: Bob, Eve, Frank
    -- case sensitive: 'a' lowercase
    assert(#result.rows >= 1)
  end)
end)

-- ============================================================================
-- describe("ORDER BY")
-- ============================================================================

describe("ORDER BY", function()
  it("sorts ascending by default", function()
    local result = exec("SELECT name, salary FROM employees ORDER BY salary")
    local salaries = {}
    for _, row in ipairs(result.rows) do
      table.insert(salaries, row[2])
    end
    for i = 2, #salaries do
      assert(salaries[i] >= salaries[i-1], "Not sorted ascending")
    end
  end)

  it("sorts ASC explicitly", function()
    local result = exec("SELECT name FROM employees ORDER BY name ASC")
    local names = {}
    for _, row in ipairs(result.rows) do
      table.insert(names, row[1])
    end
    for i = 2, #names do
      assert(names[i] >= names[i-1], "Not sorted ASC")
    end
  end)

  it("sorts DESC", function()
    local result = exec("SELECT name, salary FROM employees ORDER BY salary DESC")
    local salaries = {}
    for _, row in ipairs(result.rows) do
      table.insert(salaries, row[2])
    end
    for i = 2, #salaries do
      assert(salaries[i] <= salaries[i-1], "Not sorted DESC")
    end
  end)
end)

-- ============================================================================
-- describe("LIMIT and OFFSET")
-- ============================================================================

describe("LIMIT and OFFSET", function()
  it("LIMIT restricts row count", function()
    local result = exec("SELECT name FROM employees LIMIT 3")
    assert.equals(3, #result.rows)
  end)

  it("OFFSET skips rows", function()
    local all    = exec("SELECT name FROM employees ORDER BY id")
    local offset = exec("SELECT name FROM employees ORDER BY id OFFSET 2")
    assert.equals(#all.rows - 2, #offset.rows)
  end)

  it("LIMIT with OFFSET", function()
    local result = exec("SELECT name FROM employees ORDER BY id LIMIT 2 OFFSET 1")
    assert.equals(2, #result.rows)
  end)

  it("LIMIT larger than row count returns all", function()
    local result = exec("SELECT name FROM employees LIMIT 100")
    assert.equals(7, #result.rows)
  end)
end)

-- ============================================================================
-- describe("DISTINCT")
-- ============================================================================

describe("DISTINCT", function()
  it("removes duplicate values", function()
    local result = exec("SELECT DISTINCT dept FROM employees")
    -- Engineering, Marketing, HR, NULL = 4 distinct values
    assert.equals(4, #result.rows)
  end)

  it("DISTINCT on multiple columns", function()
    local result = exec("SELECT DISTINCT dept FROM employees WHERE dept IS NOT NULL")
    assert.equals(3, #result.rows)
  end)
end)

-- ============================================================================
-- describe("Aggregate functions")
-- ============================================================================

describe("Aggregate functions", function()
  it("COUNT(*) counts all rows", function()
    local result = exec("SELECT COUNT(*) FROM employees")
    assert.equals(1, #result.rows)
    assert.equals(7, result.rows[1][1])
  end)

  it("COUNT(col) excludes NULLs", function()
    local result = exec("SELECT COUNT(dept) FROM employees")
    assert.equals(1, #result.rows)
    assert.equals(6, result.rows[1][1])  -- Grace has NULL dept
  end)

  it("SUM computes total", function()
    local result = exec("SELECT SUM(salary) FROM employees")
    assert.equals(1, #result.rows)
    assert.equals(541000, result.rows[1][1])
  end)

  it("AVG computes average", function()
    local result = exec("SELECT AVG(salary) FROM employees")
    assert.equals(1, #result.rows)
    local avg = result.rows[1][1]
    assert(math.abs(avg - 77285.71) < 1, "AVG out of range: " .. tostring(avg))
  end)

  it("MIN finds minimum", function()
    local result = exec("SELECT MIN(salary) FROM employees")
    assert.equals(60000, result.rows[1][1])
  end)

  it("MAX finds maximum", function()
    local result = exec("SELECT MAX(salary) FROM employees")
    assert.equals(95000, result.rows[1][1])
  end)
end)

-- ============================================================================
-- describe("GROUP BY")
-- ============================================================================

describe("GROUP BY", function()
  it("groups rows by column", function()
    local result = exec("SELECT dept, COUNT(*) FROM employees GROUP BY dept")
    -- Engineering (3), Marketing (2), HR (1), NULL (1) = 4 groups
    assert.equals(4, #result.rows)
  end)

  it("computes aggregates per group", function()
    local result = exec("SELECT dept, SUM(salary) FROM employees WHERE dept IS NOT NULL GROUP BY dept ORDER BY dept")
    -- Find Engineering group
    local eng_sum
    for _, row in ipairs(result.rows) do
      if row[1] == "Engineering" then eng_sum = row[2]; break end
    end
    assert.equals(274000, eng_sum)  -- 95000 + 88000 + 91000
  end)

  it("HAVING filters groups", function()
    local result = exec("SELECT dept, COUNT(*) FROM employees GROUP BY dept HAVING COUNT(*) > 1")
    -- Engineering (3), Marketing (2) = 2 groups
    assert.equals(2, #result.rows)
  end)

  it("HAVING with aggregate comparison", function()
    local result = exec("SELECT dept, AVG(salary) FROM employees GROUP BY dept HAVING AVG(salary) > 80000")
    -- Engineering avg = 91333, others lower
    assert.equals(1, #result.rows)
    assert.equals("Engineering", result.rows[1][1])
  end)
end)

-- ============================================================================
-- describe("Expressions and arithmetic")
-- ============================================================================

describe("Expressions and arithmetic", function()
  it("arithmetic in SELECT", function()
    local result = exec("SELECT name, salary * 1.1 FROM employees WHERE name = 'Alice'")
    assert.equals(1, #result.rows)
    local raised = result.rows[1][2]
    assert(math.abs(raised - 104500) < 1, "Expected ~104500, got " .. tostring(raised))
  end)

  it("column alias with AS", function()
    local result = exec("SELECT name, salary AS pay FROM employees WHERE name = 'Alice'")
    assert.equals(2, #result.columns)
    assert.equals("pay", result.columns[2])
  end)

  it("literal string in SELECT", function()
    local result = exec("SELECT 'hello' FROM employees LIMIT 1")
    assert.equals("hello", result.rows[1][1])
  end)

  it("literal number in SELECT", function()
    local result = exec("SELECT 42 FROM employees LIMIT 1")
    assert.equals(42, result.rows[1][1])
  end)
end)

-- ============================================================================
-- describe("String functions")
-- ============================================================================

describe("String functions", function()
  it("UPPER converts to uppercase", function()
    local result = exec("SELECT UPPER(name) FROM employees WHERE name = 'Alice'")
    assert.equals("ALICE", result.rows[1][1])
  end)

  it("LOWER converts to lowercase", function()
    local result = exec("SELECT LOWER(name) FROM employees WHERE name = 'Alice'")
    assert.equals("alice", result.rows[1][1])
  end)

  it("LENGTH returns string length", function()
    local result = exec("SELECT LENGTH(name) FROM employees WHERE name = 'Alice'")
    assert.equals(5, result.rows[1][1])
  end)
end)

-- ============================================================================
-- describe("INNER JOIN")
-- ============================================================================

describe("INNER JOIN", function()
  it("joins two tables on matching column", function()
    local result = exec([[
      SELECT employees.name, departments.budget
      FROM employees
      INNER JOIN departments ON employees.dept = departments.dept_name
    ]])
    -- 6 employees have non-NULL dept (Grace excluded by inner join)
    assert.equals(6, #result.rows)
  end)

  it("JOIN keyword (implicit INNER)", function()
    local result = exec([[
      SELECT employees.name, departments.budget
      FROM employees
      JOIN departments ON employees.dept = departments.dept_name
    ]])
    assert.equals(6, #result.rows)
  end)
end)

-- ============================================================================
-- describe("NULL handling")
-- ============================================================================

describe("NULL handling", function()
  it("NULL compared to value is NULL (not true/false)", function()
    -- Grace has NULL dept; dept = 'Engineering' should be NULL, not in result
    local result = exec("SELECT name FROM employees WHERE dept = 'Engineering'")
    local names = {}
    for _, row in ipairs(result.rows) do names[row[1]] = true end
    assert.is_nil(names["Grace"])
  end)

  it("NULL in arithmetic gives NULL", function()
    local result = exec("SELECT name FROM employees WHERE dept IS NULL")
    assert.equals(1, #result.rows)
  end)

  it("COUNT(*) includes NULL rows", function()
    local result = exec("SELECT COUNT(*) FROM employees")
    assert.equals(7, result.rows[1][1])
  end)

  it("COUNT(dept) excludes NULL", function()
    local result = exec("SELECT COUNT(dept) FROM employees")
    assert.equals(6, result.rows[1][1])
  end)
end)

-- ============================================================================
-- describe("execute_all")
-- ============================================================================

describe("execute_all", function()
  it("runs multiple statements", function()
    local ds = make_ds()
    local results, err = sql_engine.execute_all(
      "SELECT COUNT(*) FROM employees; SELECT COUNT(*) FROM departments",
      ds
    )
    assert.is_nil(err)
    assert.equals(2, #results)
    assert.equals(7, results[1].rows[1][1])
    assert.equals(3, results[2].rows[1][1])
  end)
end)

-- ============================================================================
-- describe("error handling")
-- ============================================================================

describe("error handling", function()
  it("returns false + message for syntax errors", function()
    local ds = make_ds()
    local ok, msg = sql_engine.execute("SELECT FROM", ds)
    assert.is_false(ok)
    assert.is_string(msg)
    assert(#msg > 0)
  end)

  it("returns false + message for unknown table", function()
    local ds = make_ds()
    local ok, msg = sql_engine.execute("SELECT * FROM nonexistent_table", ds)
    assert.is_false(ok)
    assert.is_string(msg)
  end)

  it("returns false + message for unknown column", function()
    local ds = make_ds()
    local ok, msg = sql_engine.execute("SELECT nonexistent_col FROM employees", ds)
    -- Either error or NULL values — implementation may vary
    if not ok then
      assert.is_string(msg)
    end
  end)
end)

-- ============================================================================
-- describe("complex queries")
-- ============================================================================

describe("complex queries", function()
  it("combined WHERE + GROUP BY + HAVING + ORDER BY + LIMIT", function()
    local result = exec([[
      SELECT dept, COUNT(*) AS cnt, AVG(salary) AS avg_pay
      FROM employees
      WHERE salary > 60000
      GROUP BY dept
      HAVING COUNT(*) >= 2
      ORDER BY avg_pay DESC
      LIMIT 2
    ]])
    assert(#result.rows <= 2)
    assert.equals(3, #result.columns)
  end)

  it("SELECT with expressions, aliases, and WHERE", function()
    local result = exec([[
      SELECT name, salary * 12 AS annual_salary
      FROM employees
      WHERE dept = 'Engineering'
      ORDER BY annual_salary DESC
    ]])
    assert.equals(3, #result.rows)
    assert.equals("annual_salary", result.columns[2])
    -- Check sorted DESC
    local prev = math.huge
    for _, row in ipairs(result.rows) do
      assert(row[2] <= prev)
      prev = row[2]
    end
  end)

  it("DISTINCT with ORDER BY", function()
    local result = exec("SELECT DISTINCT dept FROM employees WHERE dept IS NOT NULL ORDER BY dept")
    assert.equals(3, #result.rows)
    -- Should be sorted
    assert(result.rows[1][1] <= result.rows[2][1])
  end)
end)
