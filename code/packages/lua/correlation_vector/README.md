# coding-adventures-correlation-vector (Lua)

Append-only provenance tracking for any entity in any pipeline.

Assign a **Correlation Vector (CV)** to a piece of data when it is born.
Every system, stage, or function that touches it appends its contribution.
At any point you can ask "where did this come from and what happened to it?"
and get a complete, ordered answer.

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo and implements spec **CV00-correlation-vector.md**.

---

## What problem does this solve?

Imagine a multi-pass compiler, an ETL pipeline, or a build system.  Data flows
through many stages.  Without provenance tracking, debugging "why did this variable
disappear?" requires adding print statements and re-running.

With CVs, you just call `cvlog:history(var_cv_id)` and instantly see:

```
parser          → created as IDENTIFIER
scope_analysis  → resolved to local binding
dce             → deleted: unreachable from entry: main
```

The same library works for any pipeline — domain agnostic.

---

## Stack position

```
correlation_vector   ← this package
       ↓
sha256               (base-ID generation via SHA-256 hashing)
json_serializer      (serialize/deserialize to JSON)
```

---

## Installation

```bash
luarocks install coding-adventures-correlation-vector
```

Or from source:

```bash
luarocks make --local coding-adventures-correlation-vector-0.1.0-1.rockspec
```

---

## Usage

```lua
local cv = require("coding_adventures.correlation_vector")

-- Create a log (tracing on by default).
local log = cv.new()

-- Born a root CV for a source token at line 5, column 12.
local cv_id = log:create({ origin_string = "app.ts:5:12" })
-- → e.g. "a3f1b2c4.0"

-- Record that scope_analysis processed this entity.
log:contribute(cv_id, {
  source = "scope_analysis",
  tag    = "resolved",
  meta   = { binding = "local:count:fn_main" },
})

-- A stage examined the entity but made no changes.
log:passthrough(cv_id, { source = "type_checker" })

-- Split the entity into two derived entities.
local left_id  = log:derive(cv_id, { source = "splitter", tag = "split_left" })
local right_id = log:derive(cv_id, { source = "splitter", tag = "split_right" })

-- Merge two entities into one.
local merged_id = log:merge({ left_id, right_id }, { source = "merger", tag = "joined" })

-- Delete an entity (entry stays in log for audit purposes).
log:delete(right_id, { by = "dead_code_eliminator" })

-- Query history for an entity.
local hist = log:history(cv_id)
-- → array of { source, tag, meta, timestamp }

-- Walk ancestry (nearest parent first).
local ancestors = log:ancestors(merged_id)
-- → { left_id, right_id, cv_id }  (order may vary for merge parents)

-- Full provenance chain, oldest first.
local lineage = log:lineage(merged_id)
-- → array of entry tables from root to merged_id

-- Serialize for storage or cross-process transfer.
local json_str = log:serialize()

-- Reconstruct from JSON.
local log2 = cv.deserialize(json_str)
```

---

## Disabled mode (zero overhead in production)

```lua
-- In production, disable tracing with a config flag.
local log = cv.new({ enabled = false })

-- CV IDs are still allocated and returned (entities still carry their cv_id).
-- Nothing is written to the log — zero allocation overhead.
local cv_id = log:create({ origin_string = "heavy_file.ts" })
print(cv_id)           -- "a3f1b2c4.0" (ID exists)
print(log:get(cv_id))  -- nil (nothing stored)
print(#log:history(cv_id))  -- 0 (empty)
```

---

## ID format

| Pattern       | Meaning                          |
|---------------|----------------------------------|
| `a3f1b2c4.0`  | Root CV, base derived from SHA-256 of origin |
| `00000000.0`  | Synthetic root (no natural origin) |
| `a3f1b2c4.0.1` | Derived from `a3f1b2c4.0`      |
| `a3f1b2c4.0.1.2` | Derived from `a3f1b2c4.0.1`  |
| `deadbeef.5`  | Merged CV, base from SHA-256 of sorted parent IDs |

---

## API reference

| Method | Description |
|--------|-------------|
| `cv.new(opts)` | Create a new CVLog. `opts.enabled` (default true) |
| `log:create(opts)` | Create a root CV. `opts.origin_string`, `opts.synthetic`, `opts.meta` |
| `log:contribute(cv_id, opts)` | Append a contribution. `opts.source`, `opts.tag`, `opts.meta` |
| `log:derive(parent_cv_id, opts)` | Create a child CV |
| `log:merge(cv_ids, opts)` | Create a CV from multiple parents |
| `log:delete(cv_id, opts)` | Mark CV as deleted. `opts.by` |
| `log:passthrough(cv_id, opts)` | Record a pass-through. `opts.source` |
| `log:get(cv_id)` | Return entry or nil |
| `log:ancestors(cv_id)` | Ancestor IDs, nearest first |
| `log:descendants(cv_id)` | Descendant IDs |
| `log:history(cv_id)` | Contributions array |
| `log:lineage(cv_id)` | Entries from oldest ancestor to self |
| `log:serialize()` | CVLog → JSON string |
| `cv.deserialize(json_str)` | JSON string → CVLog |

---

## Running tests

```bash
# Install dependencies first (leaf to root), then:
luarocks make --local coding-adventures-correlation-vector-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
```

---

## License

MIT
