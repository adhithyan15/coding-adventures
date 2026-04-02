-- coding-adventures-sql-execution-engine-0.1.0-1.rockspec
-- =========================================================
-- LuaRocks package specification for sql_execution_engine.
-- No external dependencies: the SQL tokenizer and parser are built-in.

package = "coding-adventures-sql-execution-engine"
version = "0.1.0-1"

source = {
  url = "*** source URL not set ***",
}

description = {
  summary  = "SELECT-only SQL execution engine with pluggable data sources",
  detailed = [[
    Implements a complete SELECT-only SQL execution engine using a materialized
    pipeline model.  Supports WHERE, GROUP BY, HAVING, ORDER BY, LIMIT/OFFSET,
    DISTINCT, JOIN, aggregate functions, and three-valued NULL logic.
    Ships with an InMemoryDataSource for testing and examples.
    No external dependencies — the SQL lexer and parser are built-in.
  ]],
  license  = "MIT",
}

dependencies = {
  "lua >= 5.4",
}

build = {
  type    = "builtin",
  modules = {
    ["coding_adventures.sql_execution_engine"] =
      "src/coding_adventures/sql_execution_engine/init.lua",
  },
}
