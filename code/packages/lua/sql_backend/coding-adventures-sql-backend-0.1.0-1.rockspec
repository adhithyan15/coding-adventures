package = "coding-adventures-sql-backend"
version = "0.1.0-1"

source = {
  url = "*** source URL not set ***",
}

description = {
  summary = "Mini-SQLite backend contract for Lua",
  detailed = [[
    Provides the Lua port of the mini-sqlite storage backend contract,
    including SQL value helpers and an in-memory backend for conformance tests.
  ]],
  license = "MIT",
}

dependencies = {
  "lua >= 5.4",
}

build = {
  type = "builtin",
  modules = {
    ["coding_adventures.sql_backend"] =
      "src/coding_adventures/sql_backend/init.lua",
  },
}
