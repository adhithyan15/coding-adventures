# @coding-adventures/forme-types

The Forme kernel's shared TypeScript type vocabulary — Kinds, KindDescriptors, branded identity types, and JSON utilities.

This is the leaf of the Forme dependency graph: every other Forme package imports from here. It contains **types and constants only**. There is no I/O, no runtime side-effects, no implementation logic. The matching implementations (identity hashing, capability parsing, the Stage interface) live in sibling packages.

See [code/specs/FM01-forme-kernel.md](../../../specs/FM01-forme-kernel.md) §2 for the full design.

## What's in here

| Module        | Purpose                                                                  |
| ------------- | ------------------------------------------------------------------------ |
| `utility.ts`  | `JsonValue`, `ReadonlyRecord`                                            |
| `identity.ts` | `LogicalId`, `RevisionId` branded type aliases                           |
| `kinds.ts`    | `KIND` constants, `KindDescriptor`, canonical `Kinds` object, `streamOf` |
| `shapes.ts`   | TypeScript interfaces for all 12 built-in kinds + stub style/interactivity |
| `payload.ts`  | `KindPayload<K>` mapped type — descriptor → value type                    |

## Quick reference

### The 12 built-in Kinds

```
Void                  → no payload (source-stage input, sink-stage output)
ContentSource         → raw bytes + metadata from a storage adapter
ContentNode           → parsed document (DocumentNode + frontmatter + identity)
Collection            → ordered set of content references with grouping discriminant
Asset                 → image / video / font / binary with metadata
Document              → (content, style, interactivity) triple ready to render
RenderedPage          → HTML + metadata for a web page
PrintForme            → backend-neutral page for LaTeX / PDF / EPUB
RequestHandler        → per-request handler (Workers, Node, Deno, Bun)
SearchIndex           → serialised search index
Feed                  → RSS / Atom / JSON Feed / sitemap
DeployArtifact        → final shippable bundle
Stream<K>             → meta-kind wrapping a kind for streaming
```

### Defining a stage's I/O

```typescript
import { Kinds, streamOf } from "@coding-adventures/forme-types";

// A source: takes nothing, produces a stream of ContentSources.
const source = {
  consumes: Kinds.Void,
  produces: streamOf(Kinds.ContentSource),
  // ...
};

// A parser: takes one ContentSource, produces one ContentNode.
const parser = {
  consumes: Kinds.ContentSource,
  produces: Kinds.ContentNode,
  // ...
};
```

### Branded ID types

```typescript
import type { LogicalId, RevisionId } from "@coding-adventures/forme-types";

const id  = "01952c0d-7e63-7000-8000-000000000000" as LogicalId;
const rev = "blake2b:abc123..." as RevisionId;
```

The implementations of `computeRevisionId`, `canonicalJson`, etc. live in `@coding-adventures/forme-identity`. This package only declares the types so any package that *carries* an ID can use it without depending on the hashing code.

## Spec divergences (v0)

These deliberate differences from FM01 are documented here so future readers can find them:

1. **`Stream` descriptors.** FM01 §3.6 sketches `{ ...Kinds.X, kind: "Stream" }` which adds an unrelated `kind` field to a `KindDescriptor`. We use `streamOf(K)` producing `{ name: "Stream", version: "1.0", inner: K }` — a closed-shape descriptor with a dedicated `inner` field. See [src/kinds.ts](src/kinds.ts) header.
2. **`RevisionId` algorithm.** FM01 §7 specifies `blake3:<hex>`. The monorepo currently has a from-scratch BLAKE2b but no BLAKE3. v0 uses `blake2b:<hex>`; the format is forward-compatible (the `<algo>:` prefix tells consumers what was used).

## Coverage

```bash
npm install
npx vitest run --coverage
```

Lines and branches >95% (most of the package is types — non-executable — but the const values, helpers, and public exports are exercised).
