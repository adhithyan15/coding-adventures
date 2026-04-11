# CorrelationVector (Swift)

A lightweight, append-only provenance tracking library. Assign a **Correlation Vector (CV)** to any
entity when it is born; every stage, pass, or service that touches it appends its contribution.
At any point you can ask "where did this come from and what happened to it?" and get a complete,
ordered answer.

This is a **generic, domain-agnostic** library. It knows nothing about compilers, ETL, or ML
pipelines. Consumers attach semantic meaning through the `source` and `tag` fields of contributions,
and through arbitrary `JsonValue` metadata.

Implements spec **CV00-correlation-vector**.

## Where it fits in the stack

```
sha256          ← base segment of CV IDs
json-value      ← typed metadata on contributions
json-serializer ← JSON encode/decode for CVLog

       ↓
correlation-vector    ← this package
       ↓
Semantic IR (IR00), JIT profiler, ETL pipelines, build systems, …
```

## Installation

Add to your `Package.swift`:

```swift
.package(path: "../correlation-vector"),
```

Then in your target:

```swift
.target(name: "MyTarget", dependencies: [
    .product(name: "CorrelationVector", package: "correlation-vector"),
])
```

## Quick start

```swift
import CorrelationVector
import JsonValue

// Create a log for a pipeline run
let log = CVLog()

// Born a new entity when it enters the pipeline
let tokenId = log.create(originString: "app.ts:5:12")

// Each stage records what it did
try log.contribute(cvId: tokenId, source: "scope_analysis", tag: "resolved",
                   meta: .object([("binding", .string("local:count:fn_main"))]))
try log.contribute(cvId: tokenId, source: "variable_renamer", tag: "renamed",
                   meta: .object([("from", .string("count")), ("to", .string("a"))]))

// A pass that makes no change records itself too
try log.passthrough(cvId: tokenId, source: "type_checker")

// When a node is eliminated, soft-delete it (entry stays in log permanently)
try log.delete(cvId: tokenId, by: "dead_code_eliminator")

// Query the full history
let history = log.history(of: tokenId)
// → [Contribution(source: "scope_analysis", ...), Contribution(source: "variable_renamer", ...)]

// Derive a child node from a parent (for splits/destructuring)
let parentId = log.create(originString: "binding_target")
let leftId = try log.derive(parentCvId: parentId, source: "destructurer", tag: "left")
let rightId = try log.derive(parentCvId: parentId, source: "destructurer", tag: "right")

// Merge multiple nodes (for inlining, joins)
let mergedId = try log.merge(cvIds: [leftId, rightId], source: "inliner", tag: "merged")

// Walk the ancestor chain (nearest-first)
let ancestorIds = log.ancestors(of: mergedId)   // → [leftId, rightId, parentId]

// Get the full lineage (oldest ancestor → self)
let lineage = log.lineage(of: mergedId)

// Serialize for storage or cross-process transmission
let json = log.serialize()
let restoredLog = try CVLog.deserialize(json)
```

## CV ID format

```
base.N           root CV (created directly, no parent)
base.N.M         derived from base.N
base.N.M.K       derived from base.N.M (grandchild)
00000000.N       synthetic entity (no natural origin)
```

The base segment is the first 8 hex characters of SHA-256(originString). This makes the ancestry
visible in the ID string itself — no log lookup needed to count nesting depth.

## The `enabled` flag

```swift
let log = CVLog(enabled: false)   // tracing off
```

When `enabled` is `false`, all write operations (`contribute`, `derive`, `merge`, `delete`,
`passthrough`) are immediate no-ops. CV IDs are still generated and returned — entities need their
IDs regardless of whether tracing is on. This lets production code pay essentially zero overhead
when tracing is disabled.

## API reference

| Method | Description |
|--------|-------------|
| `create(originString:synthetic:meta:) → String` | Born a new root CV |
| `contribute(cvId:source:tag:meta:)` | Record a transformation (throws if deleted) |
| `derive(parentCvId:source:tag:meta:) → String` | Create a child CV |
| `merge(cvIds:source:tag:meta:) → String` | Create a CV from multiple parents |
| `delete(cvId:by:)` | Soft-delete (entry stays in log permanently) |
| `passthrough(cvId:source:) → String` | Record a stage examined but did not transform |
| `get(cvId:) → CVEntry?` | Look up a single entry |
| `ancestors(of:) → [String]` | BFS ancestor walk, nearest-first |
| `descendants(of:) → [String]` | All direct children |
| `history(of:) → [Contribution]` | Contributions in order |
| `lineage(of:) → [CVEntry]` | Full ancestor chain, oldest-first |
| `serialize() → String` | JSON encode |
| `CVLog.deserialize(_:) → CVLog` | JSON decode |

## Running tests

```bash
cd code/packages/swift/correlation-vector
xcrun swift test --enable-code-coverage --verbose
```

Or via the monorepo build tool from the repo root.
