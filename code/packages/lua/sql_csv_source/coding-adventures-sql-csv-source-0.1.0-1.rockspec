package = "coding-adventures-sql-csv-source"
version = "0.1.0-1"
source = {
  url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
  summary = "CSV-backed SQL data source for the Lua SQL execution engine",
  license = "MIT",
}
dependencies = {
  "lua >= 5.4",
  "coding-adventures-csv-parser >= 0.1.0-1",
  "coding-adventures-sql-execution-engine >= 0.1.0-1",
}
build = {
  type = "builtin",
  modules = {
    ["coding_adventures.sql_csv_source"] = "src/coding_adventures/sql_csv_source/init.lua",
  },
}
