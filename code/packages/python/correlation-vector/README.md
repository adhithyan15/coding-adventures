# coding-adventures-correlation-vector

Append-only provenance tracking for any data pipeline.

Assign a **Correlation Vector (CV)** to any entity when it is born.  Every
system, stage, or function that touches it appends its contribution.  At any
point you can ask "where did this come from and what happened to it?" and get a
complete, ordered answer.

This is the Python implementation of the CV00 spec.  The same API is
implemented in Elixir, Rust, TypeScript, Go, Ruby, Swift, Kotlin, and Lua.

---

## Where it fits in the stack

```
┌─────────────────────────────────────────┐
│  correlation-vector  (this package)     │
│  • tracks provenance of data entities   │
│  • domain-agnostic: works in compilers, │
│    ETL, build systems, ML pipelines     │
├─────────────────────────────────────────┤
│  json-serializer  (stringify)           │
│  json-value       (parse_native)        │
│  sha256           (sha256_hex)          │
└─────────────────────────────────────────┘
```

---

## Quick start

```python
from coding_adventures_correlation_vector import CVLog, Origin

# Create a new log for this pipeline run.
log = CVLog()

# A token is born at parse time — assign it a CV.
cv_id = log.create(Origin(source="app.ts", location="5:12"))

# Each compiler pass appends its contribution.
log.contribute(cv_id, "parser",           "created",  {"token": "IDENTIFIER"})
log.contribute(cv_id, "scope_analysis",   "resolved", {"binding": "local:count"})
log.contribute(cv_id, "variable_renamer", "renamed",  {"from": "count", "to": "a"})

# Dead-code elimination removes it.
log.delete(cv_id, "dce", "unreachable from entry: main")

# At any point, inspect the full history.
for contrib in log.history(cv_id):
    print(f"  {contrib.source}: {contrib.tag}  meta={contrib.meta}")

# Serialise the whole log for cross-process transmission or storage.
json_text = log.to_json_string()

# Restore it on the other end.
restored = CVLog.from_json_string(json_text)
```

---

## CV ID format

CV IDs encode parentage in the string itself:

```
a3f1b2c4.1          root CV — first entity with base "a3f1b2c4"
a3f1b2c4.2          root CV — second entity with same base
a3f1b2c4.1.1        first child derived from a3f1b2c4.1
a3f1b2c4.1.2        second child derived from a3f1b2c4.1
a3f1b2c4.1.1.1      grandchild
00000000.1          synthetic entity — no natural origin
```

The base is the first 8 hex characters of `SHA-256(source + ":" + location)`.
Synthetic entities (no origin) always use `00000000`.

---

## API reference

### Dataclasses

| Class | Fields |
|---|---|
| `Origin` | `source`, `location`, `timestamp?`, `meta` |
| `Contribution` | `source`, `tag`, `meta` |
| `DeletionRecord` | `source`, `reason`, `meta` |
| `CVEntry` | `id`, `parent_ids`, `origin?`, `contributions`, `deleted?` |

### CVLog operations

| Method | Description |
|---|---|
| `create(origin?)` | Born a root CV; returns its ID |
| `contribute(cv_id, source, tag, meta?)` | Append a contribution; raises `ValueError` if entry is deleted |
| `derive(parent_cv_id, origin?)` | Create a child CV; returns its ID |
| `merge(parent_cv_ids, origin?)` | Create a multi-parent CV; returns its ID |
| `delete(cv_id, source, reason, meta?)` | Tombstone an entry |
| `passthrough(cv_id, source)` | Record that a stage made no changes |
| `get(cv_id)` | Return the CVEntry or `None` |
| `ancestors(cv_id)` | All ancestor IDs, nearest parent first |
| `descendants(cv_id)` | All descendant IDs |
| `history(cv_id)` | Contributions in append order |
| `lineage(cv_id)` | Full entries from oldest ancestor to entity |
| `serialize()` | Plain dict matching JSON schema |
| `to_json_string()` | Compact JSON string |
| `from_json_string(s)` | Reconstruct from JSON string |
| `deserialize(data)` | Reconstruct from plain dict |

### Enabled flag

```python
log = CVLog(enabled=False)  # tracing off

cv_id = log.create(origin)   # still returns a valid ID
log.contribute(cv_id, ...)   # no-op
log.get(cv_id)               # always None
log.history(cv_id)           # always []
```

Set `enabled=False` in production to pay near-zero overhead while keeping the
same API surface.

---

## Development

```bash
# From the package directory:
uv venv --quiet --clear
uv pip install -e ../sha256 -e ../directed-graph -e ../state-machine \
    -e ../grammar-tools -e ../lexer -e ../parser -e ../json-lexer \
    -e ../json-parser -e ../json-value -e ../json-serializer -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
.venv/bin/ruff check src/
```

---

## Relationship to other packages

- **json-serializer** — used by `to_json_string()` for canonical JSON output.
- **json-value** — used by `from_json_string()` to parse JSON into native
  Python types.
- **sha256** — used by `_base_from_origin()` to compute deterministic 8-hex
  base segments from origin strings.
- **IR00-semantic-ir** — the Semantic IR assigns a `cv_id` to every AST node
  and carries a CVLog through the compiler pipeline.  The CV library is the
  implementation of that tracking mechanism.
