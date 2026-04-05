# CV00 — Correlation Vector

## Overview

A Correlation Vector (CV) is a lightweight, append-only provenance record that
follows a piece of data through every transformation it undergoes. Assign a CV to
anything when it is born. Every system, stage, or function that touches it appends
its contribution. At any point you can ask "where did this come from and what happened
to it?" and get a complete, ordered answer.

The concept originated in distributed systems tracing, where a request flows through
dozens of microservices and you need to reconstruct what happened across all of them.
This implementation generalizes the idea to any pipeline — compiler passes, data ETL,
document transformations, build systems, ML preprocessing, or anywhere that data
flows through a sequence of transformations.

This is a **generic, domain-agnostic package**. It knows nothing about compilers,
JavaScript, or IR nodes. It is a data structure and a set of operations over it.
Consumers attach semantic meaning through the `source` and `tag` fields of
contributions, and through arbitrary metadata.

```
A compiler uses it like this:
  cv born at parse time  →  scope_analysis contributes "resolved"
                         →  variable_renamer contributes "renamed foo→a"
                         →  dce contributes "deleted: unreachable"

An ETL pipeline uses it like this:
  cv born at ingestion   →  validator contributes "schema_checked"
                         →  normalizer contributes "date_format_converted"
                         →  enricher contributes "geo_lookup_appended"

A build system uses it like this:
  cv born at source file →  compiler contributes "compiled to .o"
                         →  linker contributes "linked into binary"
                         →  packager contributes "bundled into .tar.gz"
```

Same library. Different tags. Full provenance in every case.

---

## Core Concepts

### The CV ID

Every tracked entity gets a **CV ID**: a stable, globally unique string assigned
at birth. It never changes. The entity can be transformed, renamed, merged, split,
or deleted — the CV ID is the one thing that remains constant, the thread you can
always pull to reconstruct history.

CV IDs use a dot-extension scheme:

```
base.N           — a root CV (born directly, not derived from another)
base.N.M         — derived from base.N (e.g., a split output)
base.N.M.K       — derived from base.N.M
```

This scheme means you can read the parentage directly from the ID without consulting
the log. A CV ID with two dots is a grandchild of its base. The depth of nesting is
immediately visible.

```
a3f1.1           root CV — first entity born with base "a3f1"
a3f1.2           root CV — second entity born with base "a3f1"
a3f1.1.1         first entity derived from a3f1.1
a3f1.1.2         second entity derived from a3f1.1
a3f1.1.1.1       entity derived from a3f1.1.1
00000000.1       synthetic entity — no natural origin (base is always 00000000 for these)
```

The base segment is typically an 8-character hex string derived from a hash of the
entity's origin (file path + position, ingestion timestamp, source identifier, etc.).
For programmatically created entities with no natural origin, use `00000000`.

### The Contribution

Every time a stage processes an entity, it appends a **Contribution** to that
entity's CV:

```
Contribution {
  source:  string    -- who/what contributed (stage name, service name, pass name)
  tag:     string    -- what happened (domain-defined label)
  meta:    map       -- arbitrary key-value detail (domain-defined)
}
```

`source` identifies the actor. `tag` classifies the action. `meta` carries detail.
The CV library imposes no constraints on the values of `source` or `tag` — those
are entirely defined by the consumer domain.

Examples across domains:

```
Compiler:
  {source: "variable_renamer", tag: "renamed", meta: {from: "userPreferences", to: "a"}}
  {source: "dce", tag: "deleted", meta: {reason: "unreachable from entry: main"}}

ETL:
  {source: "date_normalizer", tag: "converted", meta: {from_format: "MM/DD/YYYY", to_format: "ISO8601"}}
  {source: "geo_enricher", tag: "appended", meta: {field: "country_code", value: "US"}}

Build system:
  {source: "gcc", tag: "compiled", meta: {flags: "-O2 -std=c17", output: "main.o"}}
  {source: "ld", tag: "linked", meta: {output: "app", entry: "_start"}}
```

### The CV Entry

The full record for a single CV ID:

```
CVEntry {
  id:            CvId            -- stable identity string
  parent_ids:    CvId[]          -- empty for roots; one or more for derived/merged CVs
  origin:        Origin?         -- where/when this entity was born (nil for synthetics)
  contributions: Contribution[]  -- append-only history of what touched it
  deleted:       DeletionRecord? -- non-nil if this entity was deleted
}

Origin {
  source:    string    -- identifies the origin system or file
  location:  string    -- position within the source (line:col, byte offset, etc.)
  timestamp: string?   -- ISO 8601 timestamp if time-relevant
  meta:      map       -- any additional origin context
}

DeletionRecord {
  source:  string    -- who deleted it
  reason:  string    -- why it was deleted
  meta:    map
}
```

### The CVLog

The CVLog is the map that holds all CV entries for a pipeline run. It travels
alongside the data being processed, accumulating the history of every entity.

```
CVLog {
  entries:      Map<CvId, CVEntry>
  pass_order:   string[]          -- ordered list of source names that have contributed
  enabled:      boolean           -- when false, all write operations are no-ops
}
```

The `enabled` flag is the tracing switch. When `false`, every mutating operation
(`contribute`, `derive`, `merge`, `delete`) returns immediately without allocating
or writing anything. The `cv_id` fields are still present on entities — they just
never get any history populated. This means production code pays essentially zero
overhead when tracing is off, and full provenance when it is on.

---

## Operations

The CV library exposes six core operations. All operations take the CVLog as an
argument and return an updated CVLog (functional style). Mutable implementations
may update in place — the semantics are identical.

### `create(log, origin?) → (cv_id, log)`

Born a new root CV. The entity has no parents — it was created from nothing or from
an external source.

```
# Parsing a source file — each token/node gets a root CV
{cv_id, log} = CV.create(log, origin: %{source: "app.ts", location: "5:12"})

# Ingesting a database row
{cv_id, log} = CV.create(log, origin: %{source: "orders_table", location: "row_id:8472"})
```

If tracing is disabled, this still allocates and returns a `cv_id` (the ID is needed
by the entity regardless). The difference is the CVLog entry is not populated.

### `contribute(log, cv_id, source, tag, meta?) → log`

Record that a stage processed this entity.

```
log = CV.contribute(log, cv_id, "scope_analysis", "resolved",
        %{binding: "local:count:fn_main"})

log = CV.contribute(log, cv_id, "validator", "schema_checked",
        %{schema: "orders_v2", result: "pass"})
```

Contributions are appended in call order. The order is semantically meaningful —
it is the sequence in which stages processed the entity.

### `derive(log, parent_cv_id, origin?) → (cv_id, log)`

Create a new CV that is descended from an existing one. Use this when one entity
is split into multiple outputs, or when a transformation produces a new entity that
is conceptually "the same thing" expressed differently.

```
# Destructuring {a, b} = x into two separate bindings
{cv_a, log} = CV.derive(log, original_cv_id)
{cv_b, log} = CV.derive(log, original_cv_id)

# An ETL stage splits one wide record into two narrower ones
{cv_left, log} = CV.derive(log, source_cv_id, origin: %{source: "splitter", location: "col:0-5"})
{cv_right, log} = CV.derive(log, source_cv_id, origin: %{source: "splitter", location: "col:6-end"})
```

The derived CV's ID is the parent ID with a new numeric suffix appended:
`parent_cv_id + "." + next_sequence`.

### `merge(log, parent_cv_ids, origin?) → (cv_id, log)`

Create a new CV descended from multiple existing CVs. Use this when multiple
entities are combined into one output.

```
# Inlining a function: the call site and function body merge into one expression
{merged_cv, log} = CV.merge(log, [call_site_cv, function_body_cv])

# Joining two database tables into one result row
{row_cv, log} = CV.merge(log, [orders_cv, customers_cv],
                  origin: %{source: "join_stage", location: "orders.customer_id=customers.id"})
```

The merged CV's `parent_ids` lists all parents. Its ID uses the `00000000` base with
a new sequence (since it has no single natural origin), unless an `origin` is provided.

### `delete(log, cv_id, source, reason, meta?) → log`

Record that an entity was intentionally removed. The CV entry remains in the log
permanently — this is how you can answer "why did this disappear?" long after the
fact.

```
log = CV.delete(log, cv_id, "dead_code_eliminator",
        "unreachable from entry point",
        %{entry_point_cv: main_cv_id})

log = CV.delete(log, cv_id, "deduplicator",
        "duplicate of earlier record",
        %{original_cv: earlier_cv_id})
```

Calling `contribute` on a deleted CV is an error. Calling `derive` or `merge` with
a deleted CV as a parent is allowed (e.g., "we derived a tombstone record from this
deleted entity").

### `passthrough(log, cv_id, source) → log`

Record that a stage examined this entity but made no changes. This is important for
reconstructing which stages an entity passed through even when nothing was transformed.
It is the identity contribution.

```
log = CV.passthrough(log, cv_id, "type_checker")
```

In performance-sensitive pipelines, `passthrough` may be omitted for known-clean
stages to reduce log size. The tradeoff is that the stage will be invisible in the
history for unaffected entities.

---

## Querying the CVLog

### `get(log, cv_id) → CVEntry?`

Return the full entry for a CV ID, or nil if not found.

### `ancestors(log, cv_id) → CvId[]`

Walk the `parent_ids` chain recursively and return all ancestor CV IDs, ordered
from immediate parent to most distant ancestor. Cycles are impossible by construction
(a CV cannot be its own ancestor), but implementations should guard against
pathological inputs.

```
# For cv_id "a3f1.1.1.1":
CV.ancestors(log, "a3f1.1.1.1")
# → ["a3f1.1.1", "a3f1.1", "a3f1.1's parents if any"]
```

### `descendants(log, cv_id) → CvId[]`

Return all CV IDs that have this CV ID in their ancestor chain. This is the inverse
of `ancestors`. It is computed by scanning the log — implementations should index
by parent_id for efficient lookup on large logs.

### `history(log, cv_id) → Contribution[]`

Return the contributions for a CV ID in order. If the entity was deleted, the
deletion record is appended as the final entry.

### `lineage(log, cv_id) → CVEntry[]`

Return the full CV entries for the entity and all its ancestors, ordered from
oldest ancestor to the entity itself. This is the complete provenance chain.

### `serialize(log) → json`

Serialize the full CVLog to JSON for storage or cross-process transmission.

### `deserialize(json) → log`

Reconstruct a CVLog from its JSON representation.

---

## ID Generation

Implementations should provide a configurable ID generator. The default is:

1. Compute an 8-character hex base from the SHA-256 of the origin string (or use
   `00000000` for synthetic entities).
2. Maintain a per-base sequence counter.
3. Concatenate as `base.N` where N starts at 1.

For derived IDs, append `.M` to the parent ID where M is the next sequence number
for that parent's children.

The generator is pluggable — consumers can supply their own generator if they need
UUIDs, content-addressed IDs, or any other scheme, as long as the output satisfies:
- Globally unique within a CVLog
- Stable (same input produces the same ID deterministically, for reproducible builds)
- No dots in the base segment (dots are reserved as the derivation separator)

---

## Serialization Format

The JSON serialization is the canonical interchange format between language
implementations.

### CVEntry JSON

```json
{
  "id": "a3f1b2c4.3",
  "parent_ids": [],
  "origin": {
    "source": "app.ts",
    "location": "5:12",
    "timestamp": null,
    "meta": {}
  },
  "contributions": [
    {
      "source": "parser",
      "tag": "created",
      "meta": { "token": "IDENTIFIER" }
    },
    {
      "source": "scope_analysis",
      "tag": "resolved",
      "meta": { "binding": "local:count:fn_main" }
    },
    {
      "source": "variable_renamer",
      "tag": "renamed",
      "meta": { "from": "count", "to": "a" }
    }
  ],
  "deleted": null
}
```

### CVLog JSON

```json
{
  "entries": {
    "a3f1b2c4.3": { ... },
    "a3f1b2c4.4": { ... }
  },
  "pass_order": ["parser", "scope_analysis", "variable_renamer"],
  "enabled": true
}
```

---

## Polyglot Implementation Guide

One package per language, all implementing this same spec. The API surface is
identical across languages; only the syntax differs.

### Package names

| Language   | Package name          |
|------------|-----------------------|
| Elixir     | `correlation_vector`  |
| Rust       | `correlation_vector`  |
| TypeScript | `correlation-vector`  |
| Go         | `correlation_vector`  |
| Python     | `correlation_vector`  |
| Ruby       | `correlation_vector`  |
| Swift      | `CorrelationVector`   |
| Kotlin     | `correlation-vector`  |
| Lua        | `correlation_vector`  |

### Elixir

```elixir
defmodule CorrelationVector do
  @moduledoc """
  Append-only provenance tracking for any data pipeline.
  Assign a CV to any entity at birth; every stage appends its contribution.
  """

  defstruct entries: %{}, pass_order: [], enabled: true

  defmodule Entry do
    defstruct [:id, parent_ids: [], origin: nil, contributions: [], deleted: nil]
  end

  defmodule Contribution do
    defstruct [:source, :tag, meta: %{}]
  end

  defmodule Origin do
    defstruct [:source, :location, timestamp: nil, meta: %{}]
  end

  defmodule DeletionRecord do
    defstruct [:source, :reason, meta: %{}]
  end

  @spec create(t(), keyword()) :: {String.t(), t()}
  def create(log, opts \\ []), do: ...

  @spec contribute(t(), String.t(), String.t(), String.t(), map()) :: t()
  def contribute(log, cv_id, source, tag, meta \\ %{}), do: ...

  @spec derive(t(), String.t(), keyword()) :: {String.t(), t()}
  def derive(log, parent_cv_id, opts \\ []), do: ...

  @spec merge(t(), [String.t()], keyword()) :: {String.t(), t()}
  def merge(log, parent_cv_ids, opts \\ []), do: ...

  @spec delete(t(), String.t(), String.t(), String.t(), map()) :: t()
  def delete(log, cv_id, source, reason, meta \\ %{}), do: ...

  @spec passthrough(t(), String.t(), String.t()) :: t()
  def passthrough(log, cv_id, source), do: ...

  @spec get(t(), String.t()) :: Entry.t() | nil
  def get(log, cv_id), do: ...

  @spec ancestors(t(), String.t()) :: [String.t()]
  def ancestors(log, cv_id), do: ...

  @spec descendants(t(), String.t()) :: [String.t()]
  def descendants(log, cv_id), do: ...

  @spec history(t(), String.t()) :: [Contribution.t()]
  def history(log, cv_id), do: ...

  @spec lineage(t(), String.t()) :: [Entry.t()]
  def lineage(log, cv_id), do: ...

  @spec serialize(t()) :: map()
  def serialize(log), do: ...

  @spec deserialize(map()) :: t()
  def deserialize(data), do: ...
end
```

### Rust

```rust
pub struct CVLog {
    pub entries: HashMap<String, CVEntry>,
    pub pass_order: Vec<String>,
    pub enabled: bool,
}

pub struct CVEntry {
    pub id: String,
    pub parent_ids: Vec<String>,
    pub origin: Option<Origin>,
    pub contributions: Vec<Contribution>,
    pub deleted: Option<DeletionRecord>,
}

pub struct Contribution {
    pub source: String,
    pub tag: String,
    pub meta: HashMap<String, serde_json::Value>,
}

impl CVLog {
    pub fn new() -> Self { ... }
    pub fn create(&mut self, origin: Option<Origin>) -> String { ... }
    pub fn contribute(&mut self, cv_id: &str, source: &str, tag: &str,
                      meta: HashMap<String, Value>) { ... }
    pub fn derive(&mut self, parent_cv_id: &str, origin: Option<Origin>) -> String { ... }
    pub fn merge(&mut self, parent_cv_ids: &[&str], origin: Option<Origin>) -> String { ... }
    pub fn delete(&mut self, cv_id: &str, source: &str, reason: &str,
                  meta: HashMap<String, Value>) { ... }
    pub fn passthrough(&mut self, cv_id: &str, source: &str) { ... }
    pub fn get(&self, cv_id: &str) -> Option<&CVEntry> { ... }
    pub fn ancestors(&self, cv_id: &str) -> Vec<String> { ... }
    pub fn descendants(&self, cv_id: &str) -> Vec<String> { ... }
    pub fn history(&self, cv_id: &str) -> Vec<&Contribution> { ... }
    pub fn lineage(&self, cv_id: &str) -> Vec<&CVEntry> { ... }
}
```

### TypeScript

```typescript
export interface Origin {
  source: string;
  location: string;
  timestamp?: string;
  meta?: Record<string, unknown>;
}

export interface Contribution {
  source: string;
  tag: string;
  meta: Record<string, unknown>;
}

export interface CVEntry {
  id: string;
  parentIds: string[];
  origin: Origin | null;
  contributions: Contribution[];
  deleted: { source: string; reason: string; meta: Record<string, unknown> } | null;
}

export class CVLog {
  entries: Map<string, CVEntry>;
  passOrder: string[];
  enabled: boolean;

  constructor(enabled = true) { ... }
  create(origin?: Origin): string { ... }
  contribute(cvId: string, source: string, tag: string,
             meta?: Record<string, unknown>): void { ... }
  derive(parentCvId: string, origin?: Origin): string { ... }
  merge(parentCvIds: string[], origin?: Origin): string { ... }
  delete(cvId: string, source: string, reason: string,
         meta?: Record<string, unknown>): void { ... }
  passthrough(cvId: string, source: string): void { ... }
  get(cvId: string): CVEntry | undefined { ... }
  ancestors(cvId: string): string[] { ... }
  descendants(cvId: string): string[] { ... }
  history(cvId: string): Contribution[] { ... }
  lineage(cvId: string): CVEntry[] { ... }
  serialize(): object { ... }
  static deserialize(data: object): CVLog { ... }
}
```

### Go

```go
type Origin struct {
    Source    string            `json:"source"`
    Location  string            `json:"location"`
    Timestamp string            `json:"timestamp,omitempty"`
    Meta      map[string]any    `json:"meta"`
}

type Contribution struct {
    Source string         `json:"source"`
    Tag    string         `json:"tag"`
    Meta   map[string]any `json:"meta"`
}

type CVEntry struct {
    ID            string         `json:"id"`
    ParentIDs     []string       `json:"parent_ids"`
    Origin        *Origin        `json:"origin"`
    Contributions []Contribution `json:"contributions"`
    Deleted       *DeletionRecord `json:"deleted"`
}

type CVLog struct {
    Entries   map[string]*CVEntry `json:"entries"`
    PassOrder []string            `json:"pass_order"`
    Enabled   bool                `json:"enabled"`
}

func NewCVLog(enabled bool) *CVLog { ... }
func (l *CVLog) Create(origin *Origin) string { ... }
func (l *CVLog) Contribute(cvID, source, tag string, meta map[string]any) { ... }
func (l *CVLog) Derive(parentCvID string, origin *Origin) string { ... }
func (l *CVLog) Merge(parentCvIDs []string, origin *Origin) string { ... }
func (l *CVLog) Delete(cvID, source, reason string, meta map[string]any) { ... }
func (l *CVLog) Passthrough(cvID, source string) { ... }
func (l *CVLog) Get(cvID string) *CVEntry { ... }
func (l *CVLog) Ancestors(cvID string) []string { ... }
func (l *CVLog) Descendants(cvID string) []string { ... }
func (l *CVLog) History(cvID string) []Contribution { ... }
func (l *CVLog) Lineage(cvID string) []*CVEntry { ... }
```

### Python

```python
from dataclasses import dataclass, field
from typing import Any

@dataclass
class Origin:
    source: str
    location: str
    timestamp: str | None = None
    meta: dict[str, Any] = field(default_factory=dict)

@dataclass
class Contribution:
    source: str
    tag: str
    meta: dict[str, Any] = field(default_factory=dict)

@dataclass
class CVEntry:
    id: str
    parent_ids: list[str] = field(default_factory=list)
    origin: Origin | None = None
    contributions: list[Contribution] = field(default_factory=list)
    deleted: dict | None = None

class CVLog:
    def __init__(self, enabled: bool = True) -> None: ...
    def create(self, origin: Origin | None = None) -> str: ...
    def contribute(self, cv_id: str, source: str, tag: str,
                   meta: dict | None = None) -> None: ...
    def derive(self, parent_cv_id: str, origin: Origin | None = None) -> str: ...
    def merge(self, parent_cv_ids: list[str],
              origin: Origin | None = None) -> str: ...
    def delete(self, cv_id: str, source: str, reason: str,
               meta: dict | None = None) -> None: ...
    def passthrough(self, cv_id: str, source: str) -> None: ...
    def get(self, cv_id: str) -> CVEntry | None: ...
    def ancestors(self, cv_id: str) -> list[str]: ...
    def descendants(self, cv_id: str) -> list[str]: ...
    def history(self, cv_id: str) -> list[Contribution]: ...
    def lineage(self, cv_id: str) -> list[CVEntry]: ...
    def serialize(self) -> dict: ...
    @classmethod
    def deserialize(cls, data: dict) -> "CVLog": ...
```

### Ruby

```ruby
module CorrelationVector
  Origin = Data.define(:source, :location, :timestamp, :meta)
  Contribution = Data.define(:source, :tag, :meta)
  CVEntry = Data.define(:id, :parent_ids, :origin, :contributions, :deleted)

  class CVLog
    attr_reader :entries, :pass_order, :enabled

    def initialize(enabled: true) = ...
    def create(origin: nil) = ...           # → cv_id
    def contribute(cv_id, source, tag, meta: {}) = ...
    def derive(parent_cv_id, origin: nil) = ...   # → cv_id
    def merge(parent_cv_ids, origin: nil) = ...   # → cv_id
    def delete(cv_id, source, reason, meta: {}) = ...
    def passthrough(cv_id, source) = ...
    def get(cv_id) = ...                    # → CVEntry | nil
    def ancestors(cv_id) = ...              # → [cv_id]
    def descendants(cv_id) = ...            # → [cv_id]
    def history(cv_id) = ...               # → [Contribution]
    def lineage(cv_id) = ...               # → [CVEntry]
    def serialize = ...                     # → Hash
    def self.deserialize(data) = ...        # → CVLog
  end
end
```

---

## Test Coverage Requirements

Every implementation must cover:

### Root lifecycle
- Create a root CV with an origin → verify ID format (`base.N`)
- Contribute to it → verify contribution appears in history
- Pass it through a stage that makes no changes → verify passthrough recorded
- Delete it → verify deletion record present, further contributions raise error

### Derivation
- Derive two children from one parent → verify both have parent's ID as prefix
- Verify `ancestors(child)` returns `[parent_cv_id]`
- Verify `descendants(parent)` returns both child IDs

### Merging
- Merge three CVs into one → verify `parent_ids` lists all three
- Verify `ancestors(merged)` returns all three parents

### Deep ancestry chain
- A → B → C → D (each derived from the previous)
- Verify `ancestors(D)` = `[C, B, A]` (nearest first)
- Verify `lineage(D)` returns all four entries

### Disabled log
- Create a CVLog with `enabled: false`
- All operations complete without error
- All CV IDs are still generated and returned
- `get(cv_id)` returns nil (nothing was stored)
- `history(cv_id)` returns empty list

### Serialization roundtrip
- Build a CVLog with roots, derivations, merges, deletions
- Serialize to JSON
- Deserialize back
- Every entry is byte-for-byte identical to the original

### ID uniqueness
- Create 10,000 root CVs with the same origin → verify all IDs are unique
- Mix origins → verify no collisions across bases

---

## Usage Examples

### Compiler pass (Elixir)

```elixir
defmodule VariableRenamer do
  def run(node, cv_log, opts) do
    {renamed_node, cv_log} = rename_all(node, cv_log, opts.rename_map)
    {renamed_node, cv_log}
  end

  defp rename_identifier(node = %{kind: :identifier, attrs: %{name: name}}, cv_log, rename_map) do
    case Map.get(rename_map, name) do
      nil ->
        cv_log = CorrelationVector.passthrough(cv_log, node.cv_id, "variable_renamer")
        {node, cv_log}
      new_name ->
        cv_log = CorrelationVector.contribute(cv_log, node.cv_id, "variable_renamer",
                   "renamed", %{from: name, to: new_name})
        {%{node | attrs: %{node.attrs | name: new_name}}, cv_log}
    end
  end
end
```

### ETL pipeline (Python)

```python
class DateNormalizer:
    def process(self, record: dict, cv_id: str, log: CVLog) -> tuple[dict, CVLog]:
        raw_date = record["order_date"]
        try:
            normalized = parse_and_normalize(raw_date)
            log.contribute(cv_id, "date_normalizer", "converted",
                           meta={"from": raw_date, "to": normalized, "format": "ISO8601"})
            return {**record, "order_date": normalized}, log
        except ValueError as e:
            log.contribute(cv_id, "date_normalizer", "conversion_failed",
                           meta={"value": raw_date, "error": str(e)})
            return record, log
```

### Build system (Go)

```go
func CompileFile(src string, log *CVLog) (string, string) {
    cvID := log.Create(&Origin{Source: src, Location: "file"})

    obj, err := gcc(src)
    if err != nil {
        log.Contribute(cvID, "gcc", "compile_failed", map[string]any{"error": err.Error()})
        return cvID, ""
    }

    log.Contribute(cvID, "gcc", "compiled", map[string]any{
        "output": obj,
        "flags": "-O2",
    })
    return cvID, obj
}
```

---

## Relationship to Other Specs

- **IR00-semantic-ir.md** — The Semantic IR assigns a `cv_id` to every AST node
  and carries a CVLog through the compiler pipeline. The CV library is the
  implementation of that tracking. The IR spec defines what `source` and `tag`
  values mean in the compiler domain.
- **05c-jit-compilation-pipeline.md** — The JIT profiler tracks hot code paths.
  CV lineage can feed into this: if a node was inlined or merged, its CV ancestry
  tells the profiler which original source location to attribute heat to.

The CV library itself has no dependency on either of those specs.
