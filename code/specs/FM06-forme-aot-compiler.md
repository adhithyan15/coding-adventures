# FM06 — Forme AOT Compiler

> **Status:** Implementation-reconciliation specification.
> **Scope:** Deterministic per-page dependency selection, incremental AOT
> caching, and static artifact emission after rendering.
> **Delivery owner:** Existing `forme-aot-*` packages; remaining release gaps
> are tracked in the [Forme completion roadmap](FM00-forme-completion-roadmap.md).

## Implementation status

| Surface | Status | Evidence / next step |
|---|---|---|
| Per-page Style IR slicing | Implemented | `forme-aot-css-slicer` is exercised by both live Forme sites. |
| Incremental slice cache | Implemented | `forme-aot-incremental-cache` plus `forme-aot-fs-cache`. |
| Page/style/script emission | Implemented | Page, HTML document, style-tag, script-tag, and bundle emitters exist. |
| Site metadata emission | Implemented | Sitemap, robots, discovery links, and deploy-manifest emitters exist. |
| Interactivity tree shaking | Blocked | Requires the normative FM05 schema and usage tracking in FM-B013. |
| Multi-backend release proof | Blocked | FM-B017 owns the non-HTML backend boundary. |

## 1. Purpose

FM06 is the canonical specification location referenced by the shipped
`forme-aot-*` packages. Those packages landed before this file, so this
document records their stable boundary and prevents their historical FM06
cross-references from pointing at a missing specification.

The AOT layer consumes validated IR plus exact per-page usage and emits the
smallest deterministic static artifacts required by each page. It does not
execute the pipeline ([FM03](FM03-forme-orchestrator.md)), define Style IR
([FM04](FM04-forme-style-ir.md)), or publish artifacts to an external target
([FM08](FM08-forme-deploy-runner.md)).

## 2. Inputs and outputs

Inputs are validated Content, Style, and eventually Interactivity values,
rendered-page usage records, and target configuration. Outputs are immutable
artifact values with canonical bytes, media types, logical routes, and
content-addressed hashes. The same inputs and configuration must produce the
same ordered output and hashes.

## 3. Per-page slicing

`forme-aot-css-slicer` intersects a validated `StyleDocument` with the exact
`usedStyle` identifiers reported for a page. It emits scoped CSS and a stable
content hash. Unknown identifiers are diagnostics, not an invitation to retain
unrelated rules silently. Aggregate pages that cannot expose an inspectable AST
may deliberately retain a complete trusted theme, as documented by FM04.

FM-B013 will extend this section with the equivalent `usedInteractivity`
contract after FM05 is normative.

## 4. Incremental cache

`forme-aot-incremental-cache` derives canonical keys from the validated IR,
sorted usage sets, active contexts, compiler version, and target options.
`forme-aot-fs-cache` supplies the filesystem backend. Corrupt, missing, or
version-mismatched entries fail open to recompilation and must never become a
successful artifact.

This cache is an AOT optimization. Pipeline-wide affected-set scheduling and
stream checkpoints remain FM03 responsibilities.

## 5. Artifact emission

The page and bundle emitters turn compiled bytes into immutable artifact
records. Emitters must reject path collisions, traversal, non-canonical input,
and hash mismatches before returning a deployable value. External publication
is intentionally deferred to FM08 so pure compilation retains no deployment
authority.

## 6. Package map

| Area | Packages |
|---|---|
| Slicing and cache | `forme-aot-css-slicer`, `forme-aot-incremental-cache`, `forme-aot-fs-cache` |
| Page composition | `forme-aot-page-emitter`, `forme-aot-page-bundle-emitter`, `forme-aot-html-doc-emitter` |
| Inline dependencies | `forme-aot-style-tag-emitter`, `forme-aot-script-tag-emitter` |
| Discovery and metadata | `forme-aot-meta-link-tags`, `forme-aot-rss-discovery-link`, `forme-aot-sitemap-emitter`, `forme-aot-robots-emitter`, `forme-aot-manifest-emitter` |
| Deployment handoff | `forme-aot-deploy-manifest-emitter` |

## 7. Related specifications

- [FM01](FM01-forme-kernel.md) — kinds, artifacts, revisions, and usage records
- [FM03](FM03-forme-orchestrator.md) — execution, caching, and reproducibility
- [FM04](FM04-forme-style-ir.md) — Style IR and `usedStyle`
- [FM05](FM05-forme-interactivity-ir.md) — future interactivity usage contract
- [FM07](FM07-forme-cli-dev-server.md) — user-facing build and preview commands
- [FM08](FM08-forme-deploy-runner.md) — external publication boundary
