package.path =
  "../src/?.lua;" ..
  "../src/?/init.lua;" ..
  "../../csv_parser/src/?.lua;" ..
  "../../csv_parser/src/?/init.lua;" ..
  "../../sql_execution_engine/src/?.lua;" ..
  "../../sql_execution_engine/src/?/init.lua;" ..
  package.path

local m = require("coding_adventures.sql_csv_source")

local fixtures = "fixtures"

local function row_maps(result)
  local rows = {}
  for _, row in ipairs(result.rows) do
    local mapped = {}
    for index, column in ipairs(result.columns) do
      mapped[column] = row[index]
    end
    rows[#rows + 1] = mapped
  end
  return rows
end

local function sorted_names(result)
  local names = {}
  for _, row in ipairs(row_maps(result)) do
    names[#names + 1] = row.name
  end
  table.sort(names)
  return names
end

describe("sql_csv_source", function()
  it("has VERSION and exports CsvDataSource", function()
    assert.equals("0.1.0", m.VERSION)
    assert.is_table(m.CsvDataSource)
    assert.is_function(m.execute_csv)
  end)

  it("coerces CSV string values", function()
    assert.is_nil(m.coerce(""))
    assert.equals(true, m.coerce("true"))
    assert.equals(false, m.coerce("False"))
    assert.equals(42, m.coerce("42"))
    assert.equals(-7, m.coerce("-7"))
    assert.equals(3.14, m.coerce("3.14"))
    assert.equals("Alice Smith", m.coerce("Alice Smith"))
  end)

  it("reads schema columns in header order", function()
    local source = m.CsvDataSource.new(fixtures)
    assert.same({"id", "name", "dept_id", "salary", "active"}, source:schema("employees"))
    assert.same({"id", "name", "budget"}, source:schema("departments"))
  end)

  it("raises when schema is requested for a missing table", function()
    local source = m.CsvDataSource.new(fixtures)
    local ok, err = pcall(function() source:schema("missing") end)
    assert.is_false(ok)
    assert.truthy(tostring(err):find("table not found: missing", 1, true))
  end)

  it("scans rows with coerced values", function()
    local source = m.CsvDataSource.new(fixtures)
    local rows = source:scan("employees")

    assert.equals(4, #rows)
    assert.equals(1, rows[1].id)
    assert.equals("Alice", rows[1].name)
    assert.equals(90000, rows[1].salary)
    assert.equals(true, rows[1].active)
    assert.equals(false, rows[3].active)
    assert.is_nil(rows[4].dept_id)
  end)

  it("raises when scanning a missing table", function()
    local source = m.CsvDataSource.new(fixtures)
    local ok, err = pcall(function() source:scan("ghosts") end)
    assert.is_false(ok)
    assert.truthy(tostring(err):find("table not found: ghosts", 1, true))
  end)

  it("executes SELECT queries against CSV files", function()
    local ok, result = m.execute_csv("SELECT * FROM employees", fixtures)
    assert.is_true(ok)
    assert.same({"active", "dept_id", "id", "name", "salary"}, result.columns)
    assert.equals(4, #result.rows)

    local rows = row_maps(result)
    assert.equals("Alice", rows[1].name)
    assert.equals(true, rows[1].active)
    assert.equals(2, rows[2].dept_id)
  end)

  it("filters active employees", function()
    local ok, result = m.execute_csv("SELECT name FROM employees WHERE active = true", fixtures)
    assert.is_true(ok)
    assert.same({"Alice", "Bob", "Dave"}, sorted_names(result))
  end)

  it("supports IS NULL predicates", function()
    local ok, result = m.execute_csv("SELECT name FROM employees WHERE dept_id IS NULL", fixtures)
    assert.is_true(ok)
    assert.equals(1, #result.rows)
    assert.equals("Dave", row_maps(result)[1].name)
  end)

  it("supports joins through the execution engine", function()
    local ok, result = m.execute_csv(
      "SELECT e.name, d.name FROM employees AS e INNER JOIN departments AS d ON e.dept_id = d.id",
      fixtures
    )
    assert.is_true(ok)
    assert.equals(3, #result.rows)
  end)

  it("reports missing tables through execute_csv", function()
    local ok, err = m.execute_csv("SELECT * FROM ghosts", fixtures)
    assert.is_false(ok)
    assert.truthy(tostring(err):find("table not found: ghosts", 1, true))
  end)
end)
