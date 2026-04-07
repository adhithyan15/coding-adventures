# correlation-vector (Go)

Append-only provenance tracking for any data pipeline.

Assign a **Correlation Vector (CV)** to any entity when it is born. Every
system, stage, or function that touches it appends its contribution. At any
point you can ask "where did this come from and what happened to it?" and get
a complete, ordered answer.

This is the Go implementation of the CV00 spec. The same API is implemented
in Python, TypeScript, Elixir, Rust, Ruby, Swift, Kotlin, and Lua.

---

## Where it fits in the stack

```
┌─────────────────────────────────────────┐
│  correlation-vector  (this package)     │
│  • tracks provenance of data entities   │
│  • domain-agnostic: works in compilers, │
│    ETL, build systems, ML pipelines     │
├─────────────────────────────────────────┤
│  json-serializer  (Serialize)           │
│  json-value       (Value round-trips)   │
│  sha256           (SHA256Hex)           │
└─────────────────────────────────────────┘
```

---

## Quick start

```go
import cv "github.com/adhithyan15/coding-adventures/code/packages/go/correlation-vector"

// Create a new log for this pipeline run.
log := cv.NewCVLog(true)

// A token is born at parse time — assign it a CV.
cvID := log.Create(&cv.Origin{Source: "app.ts", Location: "5:12"})

// Each compiler pass appends its contribution.
log.Contribute(cvID, "parser",           "created",  map[string]any{"token": "IDENTIFIER"})
log.Contribute(cvID, "scope_analysis",   "resolved", map[string]any{"binding": "local:count"})
log.Contribute(cvID, "variable_renamer", "renamed",  map[string]any{"from": "count", "to": "a"})

// Dead-code elimination removes it.
log.Delete(cvID, "dce", "unreachable from entry: main", nil)

// At any point, inspect the full history.
for _, contrib := range log.History(cvID) {
    fmt.Printf("  %s: %s  meta=%v\n", contrib.Source, contrib.Tag, contrib.Meta)
}

// Serialise the whole log for cross-process transmission or storage.
jsonText, _ := log.ToJSONString()

// Restore it on the other end.
restored, _ := cv.DeserializeFromJSON(jsonText)
_ = restored
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

## API

### Types

| Type | Description |
|------|-------------|
| `Origin` | Source location where an entity was born (`Source`, `Location`). |
| `Contribution` | A single contribution record (`Source`, `Tag`, `Meta`, `Timestamp`). |
| `DeletionRecord` | Tombstone record (`Source`, `Reason`, `Meta`, `Timestamp`). |
| `CVEntry` | One tracked entity (`CVID`, `ParentIDs`, `PassOrder`, `Contributions`, `Deleted`). |
| `CVLog` | The log — holds all entries, exposes the full API. |

### Mutating operations

| Method | Description |
|--------|-------------|
| `Create(origin *Origin) string` | Generate a root CV ID and store the entry. |
| `Contribute(cvID, source, tag string, meta map[string]any)` | Append a contribution. |
| `Derive(parentCVID string, origin *Origin) string` | Create a child CV. |
| `Merge(parentCVIDs []string, origin *Origin) string` | Create a multi-parent CV. |
| `Delete(cvID, source, reason string, meta map[string]any)` | Tombstone an entry. |
| `Passthrough(cvID, source string)` | Record an identity contribution. |

### Query operations

| Method | Description |
|--------|-------------|
| `Get(cvID string) *CVEntry` | Retrieve an entry by ID (nil if missing). |
| `Ancestors(cvID string) []string` | Return ancestor IDs, nearest first. |
| `Descendants(cvID string) []string` | Return all descendant IDs. |
| `History(cvID string) []Contribution` | Return contribution history in order. |
| `Lineage(cvID string) []*CVEntry` | Return the full ancestry chain as entries. |

### Serialisation

| Method | Description |
|--------|-------------|
| `Serialize() map[string]any` | Convert the log to a native Go map. |
| `ToJSONString() (string, error)` | Serialise to a JSON string. |
| `DeserializeFromJSON(s string) (*CVLog, error)` | Deserialise from a JSON string. |

---

## Enabled flag

`NewCVLog(enabled bool)` controls whether tracing is active.

When `enabled` is `false`, all mutating operations are no-ops — they return
immediately without allocating or writing anything. `Create` and `Derive`
still generate and return IDs (callers need those to tag their data
structures), but no entries are stored. This gives production code essentially
zero overhead when tracing is off.

---

## Dependencies

This package intentionally uses the repo's own infrastructure:

- `code/packages/go/sha256` — `SHA256Hex` for deterministic base segments
- `code/packages/go/json-serializer` — `Serialize` for structured JSON output
- `code/packages/go/json-value` — `Value` type for the parsed JSON tree

These dependencies act as an integration test for those packages under
realistic load (10,000-create uniqueness tests hammer the SHA-256 path).

---

## Test coverage

55 tests across all 7 groups from the CV00 spec (97.4% coverage):

1. **Root lifecycle** — create, contribute, passthrough, delete
2. **Derivation** — child IDs, `Ancestors`, `Descendants`
3. **Merging** — multi-parent CVs, `Ancestors` across parents
4. **Deep ancestry chain** — A→B→C→D, `Lineage`, transitive ancestors
5. **Disabled log** — all operations complete, nothing stored, IDs still returned
6. **Serialisation roundtrip** — counters survive, every field identical post-restore
7. **ID uniqueness** — 10,000 creates with the same origin produce no collisions

---

## Module path

```
github.com/adhithyan15/coding-adventures/code/packages/go/correlation-vector
```
