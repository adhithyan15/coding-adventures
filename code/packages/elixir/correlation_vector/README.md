# coding_adventures_correlation_vector

Append-only provenance tracking for any data pipeline.

Part of the [coding-adventures](https://github.com/coding-adventures/coding-adventures) monorepo.
Implements the **CV00 — Correlation Vector** specification.

---

## What Is a Correlation Vector?

A Correlation Vector (CV) is a lightweight, append-only record that follows a
piece of data through every transformation it undergoes. Assign a CV to anything
when it is born. Every system, stage, or function that touches it appends its
contribution. At any point you can ask "where did this come from and what happened
to it?" and get a complete, ordered answer.

The concept originated in distributed-systems request tracing (think Zipkin or
W3C Trace Context). This implementation generalises it to any pipeline:
compiler passes, data ETL, document transformations, build systems, ML
preprocessing, or anywhere that data flows through a sequence of transformations.

**This is a generic, domain-agnostic library.** It knows nothing about compilers,
JavaScript, or IR nodes. It is a data structure and a set of operations. Consumers
attach semantic meaning through the `source` and `tag` fields of contributions, and
through arbitrary metadata.

---

## Installation

Add to your `mix.exs` deps:

```elixir
{:coding_adventures_correlation_vector, path: "../correlation_vector"}
```

---

## Quick Start

```elixir
alias CodingAdventures.CorrelationVector
alias CodingAdventures.CorrelationVector.Origin

# Create a fresh log
log = CorrelationVector.new()

# Born a new entity with an origin
origin = %Origin{source: "app.ts", location: "5:12"}
{cv_id, log} = CorrelationVector.create(log, origin)

# Each stage that processes it appends a contribution
log = CorrelationVector.contribute(log, cv_id, "parser", "created", %{token: "IDENTIFIER"})
log = CorrelationVector.passthrough(log, cv_id, "type_checker")
log = CorrelationVector.contribute(log, cv_id, "variable_renamer", "renamed",
        %{from: "userPreferences", to: "a"})

# Query the history
CorrelationVector.history(log, cv_id)
# => [
#   %Contribution{source: "parser", tag: "created", meta: %{token: "IDENTIFIER"}},
#   %Contribution{source: "type_checker", tag: "passthrough", meta: %{}},
#   %Contribution{source: "variable_renamer", tag: "renamed", meta: %{from: "userPreferences", to: "a"}}
# ]
```

---

## Core Operations

| Function | Description |
|---|---|
| `new(enabled \\ true)` | Create a fresh CVLog |
| `create(log, origin \\ nil)` | Born a new root CV; returns `{cv_id, log}` |
| `contribute(log, cv_id, source, tag, meta \\ %{})` | Append a contribution |
| `derive(log, parent_cv_id, origin \\ nil)` | Create a child CV; returns `{cv_id, log}` |
| `merge(log, parent_cv_ids, origin \\ nil)` | Create a multi-parent CV; returns `{cv_id, log}` |
| `delete(log, cv_id, source, reason, meta \\ %{})` | Record intentional deletion |
| `passthrough(log, cv_id, source)` | Record a stage observed but did not change |

## Query Operations

| Function | Description |
|---|---|
| `get(log, cv_id)` | Full entry or nil |
| `ancestors(log, cv_id)` | Ancestor IDs, nearest first |
| `descendants(log, cv_id)` | All descendant IDs |
| `history(log, cv_id)` | Contributions in order |
| `lineage(log, cv_id)` | Full entries, oldest ancestor first |

## Serialization

| Function | Description |
|---|---|
| `serialize(log)` | CVLog → plain Elixir map |
| `to_json_string(log)` | CVLog → `{:ok, json_string}` |
| `from_json_string(json)` | JSON string → `{:ok, log}` |

---

## ID Format

```
base.N       — root CV; base = first 8 chars of SHA-256(source + ":" + location)
base.N.M     — derived from base.N (child number M)
00000000.N   — synthetic entity (no origin or merged with no origin)
```

---

## Enabled / Disabled

When `enabled: false`, all mutating operations become no-ops. CV IDs are still
generated so callers can hold references — but the log is never populated. This
lets production code pay zero overhead when tracing is off.

```elixir
log = CorrelationVector.new(false)
{cv_id, log} = CorrelationVector.create(log)  # ID generated, log empty
CorrelationVector.get(log, cv_id)              # => nil
```

---

## Serialization Format

The JSON interchange format is defined by CV00 and is compatible with all
language implementations in this monorepo:

```json
{
  "entries": {
    "a3f1b2c4.1": {
      "id": "a3f1b2c4.1",
      "parent_ids": [],
      "origin": { "source": "app.ts", "location": "5:12", "timestamp": null, "meta": {} },
      "contributions": [
        { "source": "parser", "tag": "created", "meta": {} }
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

- `coding_adventures_sha256` — pure-Elixir SHA-256 for ID generation (no `:crypto`)
- `coding_adventures_json_serializer` — for `to_json_string/1`
- `coding_adventures_json_value` — for `from_json_string/1` (parse + native conversion)
- All transitive deps of `json_serializer` (declared in `mix.exs` per lessons.md)

---

## Spec

See `code/specs/CV00-correlation-vector.md` for the full specification.
