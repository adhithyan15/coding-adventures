# @coding-adventures/correlation-vector

Append-only provenance tracking for any data pipeline.

Assign a CV (Correlation Vector) to any entity when it is born. Every pipeline
stage that processes it appends a contribution. At any point you can ask: _"Where
did this come from and what happened to it?"_ — and get a complete, ordered answer.

This is **PR 1/10**: the TypeScript reference implementation. All other language
implementations (Python, Ruby, Go, Rust, Elixir, Swift, Kotlin, Lua) will follow
this same API surface.

## What Is a Correlation Vector?

A CV is inspired by distributed systems tracing (Microsoft's Correlation Vector
protocol), generalized to any pipeline. Instead of tracking HTTP requests across
microservices, it tracks _data_ through _transformations_:

```
Compiler:
  cv born at parse time  →  scope_analysis contributes "resolved"
                         →  variable_renamer contributes "renamed x→a"
                         →  dce contributes "deleted: unreachable"

ETL pipeline:
  cv born at ingestion   →  validator contributes "schema_checked"
                         →  normalizer contributes "date_format_converted"
                         →  enricher contributes "geo_lookup_appended"

Build system:
  cv born at source file →  compiler contributes "compiled to .o"
                         →  linker contributes "linked into binary"
                         →  packager contributes "bundled into .tar.gz"
```

Same library. Different tags. Full provenance in every case.

## Installation

```bash
npm install @coding-adventures/correlation-vector
```

## Quick Start

```typescript
import { CVLog } from "@coding-adventures/correlation-vector";

// Create a log for a pipeline run.
const log = new CVLog();

// Assign a CV to an entity at birth.
const cvId = log.create({
  source: "app.ts",
  location: "5:12",
  meta: {}
});

// Each pipeline stage appends its contribution.
log.contribute(cvId, "parser", "tokenized", { token: "IDENTIFIER" });
log.contribute(cvId, "scope_analysis", "resolved", { binding: "local:count" });
log.contribute(cvId, "variable_renamer", "renamed", { from: "count", to: "a" });

// Query the history at any point.
console.log(log.history(cvId));
// [
//   { source: "parser", tag: "tokenized", meta: { token: "IDENTIFIER" } },
//   { source: "scope_analysis", tag: "resolved", meta: { binding: "local:count" } },
//   { source: "variable_renamer", tag: "renamed", meta: { from: "count", to: "a" } }
// ]
```

## API Reference

### Types

```typescript
interface Origin {
  source: string;        // e.g., "app.ts"
  location: string;      // e.g., "5:12" or "row_id:8472"
  timestamp?: string;    // ISO 8601 (optional)
  meta: Record<string, unknown>;
}

interface Contribution {
  source: string;        // who contributed
  tag: string;           // what happened
  meta: Record<string, unknown>;
}

interface DeletionRecord {
  source: string;        // who deleted it
  reason: string;        // why
  meta: Record<string, unknown>;
}

interface CVEntry {
  id: string;                     // stable ID, never changes
  parentIds: string[];            // [] for roots
  origin: Origin | null;          // null for synthetics
  contributions: Contribution[];  // append-only history
  deleted: DeletionRecord | null; // set if deleted
}
```

### CVLog

```typescript
const log = new CVLog(enabled = true);

// Core operations
log.create(origin?)                              // → cvId (base.N)
log.contribute(cvId, source, tag, meta?)         // → void (throws if deleted)
log.derive(parentCvId, origin?)                  // → cvId (parent.M)
log.merge(parentCvIds, origin?)                  // → cvId
log.delete(cvId, source, reason, meta?)          // → void
log.passthrough(cvId, source)                    // → void

// Queries
log.get(cvId)                                    // → CVEntry | undefined
log.ancestors(cvId)                              // → string[] (nearest first)
log.descendants(cvId)                            // → string[]
log.history(cvId)                                // → Contribution[]
log.lineage(cvId)                                // → CVEntry[] (oldest first)

// Serialization
log.serialize()                                  // → plain object
log.toJsonString()                               // → JSON string
CVLog.deserialize(obj)                           // → CVLog
CVLog.fromJsonString(json)                       // → CVLog
```

## CV ID Format

```
a3f1b2c4.1        — first root CV born with that origin hash
a3f1b2c4.2        — second root CV with the same origin
a3f1b2c4.1.1      — first entity derived from a3f1b2c4.1
a3f1b2c4.1.2      — second entity derived from a3f1b2c4.1
a3f1b2c4.1.1.1    — entity derived from a3f1b2c4.1.1
00000000.1        — synthetic entity (no origin, or from a merge)
```

The 8-char hex base is the first 8 characters of `SHA-256(source + ":" + location)`.
You can read the ancestry directly from the ID: each dot is one generation.

## Derivation and Merging

```typescript
// One entity split into two
const cvA = log.derive(originalCvId);  // → "base.1.1"
const cvB = log.derive(originalCvId);  // → "base.1.2"

// Two entities combined into one
const merged = log.merge([cvA, cvB]);
log.ancestors(merged);  // → [cvA, cvB]

// Deep chain
const a = log.create(...);
const b = log.derive(a);
const c = log.derive(b);
const d = log.derive(c);
log.ancestors(d);        // → [c, b, a]  (nearest first)
log.lineage(d);          // → [entryA, entryB, entryC, entryD]  (oldest first)
```

## Deletion

```typescript
// Delete is a record, not a removal. The entry stays in the log forever.
log.delete(cvId, "dead_code_eliminator", "unreachable from entry point", {
  entryPoint: mainCvId
});

// Attempting to contribute after deletion throws:
log.contribute(cvId, "later_stage", "tag");
// → Error: Cannot contribute to deleted CV "..."
```

## Disabled Mode

```typescript
// Production mode: no overhead, IDs still generated for entities
const log = new CVLog(false);

const cvId = log.create(...);  // ID allocated but not stored
log.contribute(cvId, ...);     // no-op
log.get(cvId);                 // → undefined
log.history(cvId);             // → []
```

## Serialization

```typescript
// Save the full provenance log
const json = log.toJsonString();

// Restore it later — counters are reconstructed, no ID collisions
const restored = CVLog.fromJsonString(json);

// Or use the plain object form
const obj = log.serialize();
const restored2 = CVLog.deserialize(obj);
```

The JSON format uses `snake_case` keys (`parent_ids`, `pass_order`) for
cross-language interoperability with Python, Go, Ruby, etc.

## How It Fits in the Stack

This package is spec CV00. It has no domain knowledge — it knows nothing about
compilers, ETL, or build systems. Consumers attach meaning through the `source`
and `tag` fields:

- **IR00 (Semantic IR)** uses this library to track every AST node through the
  compiler pipeline. The `source` is a compiler pass name; the `tag` describes
  the transformation applied.
- **Any pipeline** can use it: assign a CV at entry, contribute at each stage,
  query at the end.

## Testing

```bash
npm test
npm run test:coverage
```

Coverage target: ≥95% lines, branches, functions, statements.

## License

MIT
