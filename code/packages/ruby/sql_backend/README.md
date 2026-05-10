# coding_adventures_sql_backend

Ruby port of the mini-sqlite backend contract. The gem defines the storage
boundary used by the SQL VM: schema lookup, scans, positioned writes, DDL,
indexes, savepoints, triggers, and version fields.

The package also ships an in-memory backend for conformance tests and for
small embedded uses.

```ruby
require "coding_adventures/sql_backend"

include CodingAdventures::SqlBackend

backend = InMemoryBackend.new
backend.create_table(
  "users",
  [
    ColumnDef.new(name: "id", type_name: "INTEGER", primary_key: true),
    ColumnDef.new(name: "name", type_name: "TEXT", not_null: true)
  ],
  if_not_exists: false
)

backend.insert("users", {"id" => 1, "name" => "Ada"})
rows = backend.scan("users").to_a
```
