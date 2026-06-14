package = "coding-adventures-mini-sqlite"
version = "0.1.0-1"

source = {
  url = "*** source URL not set ***",
}

description = {
  summary = "Level 0 mini-sqlite facade backed by in-memory tables",
  detailed = [[
    Provides a DB-API-inspired mini-sqlite facade for Lua. Supports in-memory
    CREATE/DROP/INSERT/UPDATE/DELETE, qmark parameter binding, cursor fetch
    helpers, snapshot commit/rollback, and SELECT delegation through
    coding-adventures-sql-execution-engine.
  ]],
  license = "MIT",
}

dependencies = {
  "lua >= 5.4",
  "coding-adventures-sql-execution-engine >= 0.1.0-1",
}

build = {
  type = "builtin",
  modules = {
    ["coding_adventures.mini_sqlite"] =
      "src/coding_adventures/mini_sqlite/init.lua",
  },
}
