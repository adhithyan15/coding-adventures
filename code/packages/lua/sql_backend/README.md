# coding-adventures-sql-backend

Lua port of the mini-sqlite backend contract. The package exposes schema
metadata, typed backend errors, row iterators, positioned cursors, indexes,
triggers, transactions, savepoints, version fields, and an in-memory backend.

```lua
local backend = require("coding_adventures.sql_backend")

local db = backend.InMemoryBackend.new()
db:create_table("users", {
  backend.column_def({ name = "id", type_name = "INTEGER", primary_key = true }),
  backend.column_def({ name = "name", type_name = "TEXT", not_null = true }),
}, { if_not_exists = false })

db:insert("users", { id = 1, name = "Ada" })
local rows = db:scan("users"):to_table()
```
