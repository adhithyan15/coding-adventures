# correlation-vector (Rust)

**Package:** `coding_adventures_correlation_vector`
**Spec:** [CV00-correlation-vector.md](../../../specs/CV00-correlation-vector.md)
**Layer:** CV00 — Provenance tracking

---

## What is a Correlation Vector?

A Correlation Vector (CV) is a lightweight, **append-only provenance record** that follows
a piece of data through every transformation it undergoes.

Assign a CV to an entity when it is born. Every system, stage, or function that touches it
appends its **contribution** — who processed it, what they did, and any relevant metadata.
At any point you can ask:

> "Where did this come from and what happened to it?"

and get a complete, ordered answer.

### Use cases

| Domain | What you track | Sample tags |
|--------|---------------|-------------|
| Compiler | AST nodes | `parsed`, `resolved`, `renamed`, `eliminated` |
| ETL pipeline | Database rows | `schema_checked`, `date_converted`, `deduplicated` |
| Build system | Source files | `compiled`, `linked`, `packaged` |
| ML preprocessing | Training samples | `filtered`, `normalized`, `augmented` |

The CV library knows nothing about any of these domains. It is a generic data structure.
The domain gives meaning to `source`, `tag`, and `meta`.

---

## ID Scheme

Every tracked entity gets a **CV ID** — a stable string that never changes:

```
a3f1.1       — root CV (born directly from "a3f1" origin hash, first one)
a3f1.2       — another root CV with the same origin hash
a3f1.1.1     — first entity derived from a3f1.1
a3f1.1.2     — second entity derived from a3f1.1
a3f1.1.1.1   — entity derived from a3f1.1.1
00000000.1   — synthetic entity (no natural origin)
```

The base (`a3f1`) is the first 8 hex characters of SHA-256(`"source:location"`). You can
read parentage directly from the ID — no log lookup needed for basic lineage tracing.

---

## Quick Start

```rust
use coding_adventures_correlation_vector::{CVLog, Origin};
use std::collections::HashMap;

// Create a log with tracing enabled.
let mut log = CVLog::new(true);

// Assign a CV to a source file node at parse time.
let origin = Origin {
    source: "app.ts".into(),
    location: "42:7".into(),
    timestamp: None,
    meta: HashMap::new(),
};
let cv_id = log.create(Some(origin));

// Each pipeline stage appends its contribution.
log.contribute(&cv_id, "scope_analysis", "resolved",
    [("binding".to_string(), serde_json::json!("local:count:fn_main"))].into()
).unwrap();

log.contribute(&cv_id, "variable_renamer", "renamed",
    [("from".to_string(), serde_json::json!("count")),
     ("to".to_string(), serde_json::json!("a"))].into()
).unwrap();

// Later: inspect the full history.
let history = log.history(&cv_id);
for contrib in &history {
    println!("{}: {} — {:?}", contrib.source, contrib.tag, contrib.meta);
}
// scope_analysis: resolved — {"binding": "local:count:fn_main"}
// variable_renamer: renamed — {"from": "count", "to": "a"}

// Serialize the whole log for cross-process transmission.
let json = log.to_json_string().unwrap();
let log2 = CVLog::from_json_string(&json).unwrap();
```

---

## API Reference

### Write operations

| Method | Description |
|--------|-------------|
| `CVLog::new(enabled)` | Create an empty log. When `enabled=false`, all writes are no-ops but IDs still generate. |
| `create(origin)` | Create a root CV. Returns the new CV ID. |
| `contribute(cv_id, source, tag, meta)` | Append a contribution. Returns `Err` if entity is deleted. |
| `derive(parent_cv_id, origin)` | Create a child CV from one parent. Returns the new CV ID. |
| `merge(parent_cv_ids, origin)` | Create a CV from multiple parents. Returns the new CV ID. |
| `delete(cv_id, source, reason, meta)` | Mark an entity as deleted (entry stays in log forever). |
| `passthrough(cv_id, source)` | Record a no-change stage visit. |

### Read operations

| Method | Description |
|--------|-------------|
| `get(cv_id)` | Return the full `CVEntry` or `None`. |
| `ancestors(cv_id)` | All ancestor IDs, nearest first (BFS). |
| `descendants(cv_id)` | All descendant IDs (reverse index scan). |
| `history(cv_id)` | Ordered contributions for this entity. |
| `lineage(cv_id)` | Full entries for entity + all ancestors, oldest first. |
| `to_json_string()` | Serialize log to compact JSON string. |
| `from_json_string(s)` | Deserialize from JSON, reconstructing sequence counters. |

---

## Enabled vs. Disabled

```rust
// In production — zero overhead tracing.
let mut log = CVLog::new(false);
let id = log.create(None);  // still returns a valid ID
// contribute/delete/passthrough are all no-ops
// get() returns None

// In development — full provenance.
let mut log = CVLog::new(true);
let id = log.create(None);
log.contribute(&id, "stage", "processed", HashMap::new()).unwrap();
assert_eq!(log.history(&id).len(), 1);
```

---

## Serialization Format

```json
{
  "entries": {
    "a3f1b2c4.1": {
      "id": "a3f1b2c4.1",
      "parent_ids": [],
      "origin": { "source": "app.ts", "location": "5:12", "timestamp": null, "meta": {} },
      "contributions": [
        { "source": "parser", "tag": "created", "meta": { "token": "IDENTIFIER" } }
      ],
      "deleted": null
    }
  },
  "pass_order": ["parser"],
  "enabled": true
}
```

---

## Dependencies

- [`coding_adventures_sha256`](../sha256) — SHA-256 hash for ID generation
- [`coding_adventures_json_value`](../json-value) — JSON value types (transitive)
- [`coding_adventures_json_serializer`](../json-serializer) — JSON serialization (transitive)
- `serde` + `serde_json` — struct serialization (meta fields use `serde_json::Value`)

---

## How it fits in the stack

```
CV00-correlation-vector  ←  this package
        ↑
IR00-semantic-ir        ←  assigns cv_id to every AST node, carries CVLog through pipeline
        ↑
compiler passes         ←  contribute/derive/merge/delete as they transform the AST
```

The CV library has no knowledge of compilers, IR nodes, or any specific domain. It is a
pure data structure and a set of operations over it.
