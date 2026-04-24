# FM01 — Forme Kernel: Types, Kinds, Stages, Capabilities, Identity, Manifest

> **Status:** Code-ready specification. Read alongside FM00 (vision).
> **Scope:** The kernel packages that every other Forme package imports —
> `forme-types`, `forme-stage`, `forme-capability`, `forme-identity`,
> `forme-manifest`, and the shared error model.
> **Out of scope:** The plugin host runtime (FM02) and the orchestrator
> (FM03), which consume this kernel but have their own specifications.

---

## 0. Preface

FM00 sketched a vision: every blog platform, SSG, typesetting system, and
CMS is the same pipeline welded to a different backend, and Forme
separates the two. That spec named the shapes but did not pin them down.

FM01 pins them down.

After FM01, a TypeScript programmer holding only this document, the
existing `document-ast` package, and the coding-adventures build
conventions can implement the entire kernel. Every subsequent Forme
package — every parser, every renderer, every emitter, every editor
plugin — will import from the packages this spec defines.

The kernel is small on purpose. It is the set of things that must never
change their contract once plugins exist, because breaking them breaks
every plugin at once. Everything else lives in layers above.

### 0.1 Relationship to FM00

Where FM00 and FM01 disagree, FM01 — the one closer to running code —
wins, and FM00 is updated. Where FM01 is silent on something FM00
addresses, FM00 applies.

### 0.2 What this spec pins down

1. **The Kind system** — the compile-time and runtime representation of
   the data types that flow between stages.
2. **The Stage contract** — the typed interface every pipeline package
   implements.
3. **The StageContext** — the runtime object stages receive, including
   every capability-gated API.
4. **The Capability model** — the permission vocabulary, declaration
   syntax, and enforcement contract.
5. **The Plugin manifest** — the schema for `plugin.toml`, including
   the `apiVersion` compatibility rules.
6. **The Identity scheme** — how content gets stable IDs across renames
   and revisions.
7. **The Error model** — the typed errors the kernel throws and what
   invariants they preserve.

### 0.3 Compatibility promise

The kernel's public types and function signatures are stable within an
`apiVersion`. Breaking changes require a new `apiVersion`. Plugins
declare the `apiVersion` they target; the host refuses to load plugins
targeting an unsupported version.

`apiVersion` is a single integer that increments only on source-breaking
changes to the contracts in this document. Non-breaking additions
(a new optional field, a new kind added to a union in a way that is
source-compatible because consumers do exhaustiveness checks that
default gracefully) do not bump `apiVersion` — they are captured in the
kernel package's semver minor.

The initial `apiVersion` is `1`.

---

## 1. Terminology

- **Kind** — a type in Forme's pipeline type system. `ContentSource`,
  `ContentNode`, `Collection`, `Asset`, `RenderedPage`, etc.
- **Stage** — a package implementing the `Stage<In, Out>` contract.
  Every parser, transform, collector, renderer, emitter is a stage.
- **Pipeline** — a typed DAG of stages wired by a configuration.
- **Orchestrator** — the runtime that executes a pipeline (spec: FM03).
- **Plugin** — a package loaded through the plugin host (spec: FM02)
  that contributes one or more stages, kinds, or extension points.
- **Plugin host** — the runtime that loads plugins, validates their
  manifests, enforces their capabilities, and hands them to the
  orchestrator.
- **Kernel** — the six packages specified in FM01.
- **Capability** — a declared permission a plugin needs to perform a
  potentially-sensitive operation (network, disk, environment, time).
- **Manifest** — the `plugin.toml` file declaring a plugin's name,
  version, targeted `apiVersion`, capabilities, and contributions.
- **Identity** — a stable ID assigned by `forme-identity` to a piece
  of content or a plugin, independent of its location on disk.
- **Revision** — a content-hash ID that identifies an exact version of
  a piece of content.
- **Document AST** — the existing `@coding-adventures/document-ast`
  package, which Forme adopts as the Content IR without modification.

---

## 2. The Kind System

### 2.1 What a Kind is

A Kind is a unit of data flowing through a pipeline. Every edge in a
pipeline DAG is typed by a Kind. Every Stage declares the Kind it
consumes and the Kind it produces.

Kinds exist in two layers:

1. **At compile time** — as TypeScript types. Stages are parameterised
   by their input and output TypeScript types, and the compiler enforces
   that wired stages have compatible I/O.
2. **At runtime** — as `KindDescriptor` objects with string tags. The
   orchestrator reads these when building the DAG, checking that
   adjacent stages have compatible descriptors before invoking any code.

The two layers exist because TypeScript types are erased at runtime.
Pipeline configuration comes from YAML or TOML the compiler never sees;
the orchestrator must check compatibility with runtime data.

### 2.2 Kind taxonomy

The kernel defines exactly twelve built-in kinds. Everything downstream
either targets one of these or registers a new kind at manifest-load
time (§2.5).

```typescript
export const KINDS = [
  "ContentSource",
  "ContentNode",
  "Collection",
  "Asset",
  "Document",
  "RenderedPage",
  "PrintForme",
  "RequestHandler",
  "SearchIndex",
  "Feed",
  "DeployArtifact",
  "Stream",
] as const;

export type KindName = (typeof KINDS)[number] | `ext:${string}`;
```

The prefix `ext:` is reserved for plugin-contributed kinds. `Stream<K>`
is a meta-kind that wraps another kind; see §4.6.

### 2.3 Kind shapes

Every Kind has a precise TypeScript shape. Every shape is `readonly`
end-to-end — the kernel does not produce mutable values.

#### 2.3.1 `ContentSource`

Raw bytes plus enough metadata for downstream parsers to know what
they're looking at.

```typescript
export interface ContentSource {
  readonly kind: "ContentSource";
  /** Path the source was loaded from, for logging and error messages. */
  readonly path: string;
  /** Bytes. Parsers choose their own text decoder. */
  readonly bytes: Uint8Array;
  /** IANA mime type if known, else null. */
  readonly mimeType: string | null;
  /**
   * Logical identity (§7). Stable across edits to the same logical
   * document (same UUID across revisions).
   */
  readonly identity: LogicalId;
  /** Content-hash identity (§7). Changes with content. */
  readonly revision: RevisionId;
  /** Arbitrary source-provider-specific metadata (author, timestamps). */
  readonly providerMeta: ReadonlyRecord<string, JsonValue>;
}
```

#### 2.3.2 `ContentNode`

A parsed content document. Wraps `document-ast`'s `DocumentNode` with
frontmatter, identity, and routing information.

```typescript
import type { DocumentNode } from "@coding-adventures/document-ast";

export interface ContentNode {
  readonly kind: "ContentNode";
  readonly identity: LogicalId;
  readonly revision: RevisionId;
  /** Parsed structured content. */
  readonly document: DocumentNode;
  /** Parsed frontmatter. */
  readonly frontmatter: ReadonlyRecord<string, JsonValue>;
  /**
   * The assigned route for this content, if the pipeline has decided
   * one. Null early in the pipeline; typically set by a collector.
   */
  readonly route: string | null;
  /** References to assets this content depends on. */
  readonly assetRefs: readonly AssetRef[];
  /** Original source path for error messages. */
  readonly sourcePath: string;
}

export interface AssetRef {
  readonly id: LogicalId;
  /**
   * Where in the document the asset is referenced, for incremental
   * rebuild dependency tracking.
   */
  readonly nodePath: readonly number[];
  /** The intended role (image, embed, font, …). */
  readonly role: AssetRole;
}

export type AssetRole =
  | "image"
  | "video"
  | "audio"
  | "font"
  | "embed"
  | "binary";
```

#### 2.3.3 `Collection`

An ordered set of `ContentNode`s plus a grouping discriminant.

```typescript
export interface Collection {
  readonly kind: "Collection";
  /** Human-readable name — "posts", "docs", "tag:rust", "author:alice". */
  readonly name: string;
  /** Ordered list of content IDs; the Collection does not embed content. */
  readonly entries: readonly CollectionEntry[];
  /**
   * Logical grouping discriminant. Two collections with the same
   * discriminant are considered "the same kind of collection" and
   * stages may operate across them (e.g. pagination).
   */
  readonly discriminant: string;
  /** Site-wide or category-scoped metadata. */
  readonly meta: ReadonlyRecord<string, JsonValue>;
}

export interface CollectionEntry {
  readonly identity: LogicalId;
  readonly revision: RevisionId;
  readonly route: string | null;
  /** Ordering key derived by the collector (date, sidebar-index, …). */
  readonly orderKey: OrderKey;
  /** Per-entry metadata overlay the collector emits. */
  readonly overlay: ReadonlyRecord<string, JsonValue>;
}

export type OrderKey =
  | { readonly kind: "lexicographic"; readonly value: string }
  | { readonly kind: "numeric"; readonly value: number }
  | { readonly kind: "date"; readonly value: string } // RFC 3339
  | { readonly kind: "composite"; readonly value: readonly OrderKey[] };
```

`Collection` deliberately does not embed `ContentNode` values. It
stores references. The orchestrator resolves references when a stage
needs the actual content, which allows `Collection` to remain cheap to
produce and compare even on million-entry sites.

#### 2.3.4 `Asset`

An image, video, font, or binary file, with metadata.

```typescript
export interface Asset {
  readonly kind: "Asset";
  readonly identity: LogicalId;
  readonly revision: RevisionId;
  readonly role: AssetRole;
  readonly mimeType: string;
  readonly bytes: Uint8Array;
  /** Size of `bytes` — duplicated for cheap checks. */
  readonly byteLength: number;
  /** Width/height for raster assets. Null otherwise. */
  readonly dimensions: { readonly w: number; readonly h: number } | null;
  /** Durations for time-based assets, in milliseconds. Null otherwise. */
  readonly durationMs: number | null;
  /**
   * For derived assets (e.g. a resized image), the ID of the source
   * asset. Null for originals.
   */
  readonly derivedFrom: LogicalId | null;
  /** Processor-specific metadata (codec, colour profile, etc.). */
  readonly meta: ReadonlyRecord<string, JsonValue>;
}
```

#### 2.3.5 `Document`

The content + style + interactivity triple for a renderable unit.

```typescript
export interface Document {
  readonly kind: "Document";
  readonly identity: LogicalId;
  readonly revision: RevisionId;
  readonly content: ContentNode;
  readonly style: StyleDocument;
  readonly interactivity: Interactivity;
  /** Route this document will render to. */
  readonly route: string;
}
```

`StyleDocument` and `Interactivity` live in their own specs (FM04,
FM05) but the type names exist in the kernel so every package that
holds a `Document` reference is stable. The kernel ships stub types:

```typescript
// Stub shapes for v0; actual schemas in FM04 and FM05.
export interface StyleDocument {
  readonly kind: "StyleDocument";
  readonly tokens: ReadonlyRecord<string, JsonValue>;
  readonly rules: readonly JsonValue[];
  readonly theme: string | null;
}

export interface Interactivity {
  readonly kind: "Interactivity";
  readonly state: readonly JsonValue[];
  readonly bindings: readonly JsonValue[];
  readonly handlers: readonly JsonValue[];
  readonly islands: readonly string[];
}
```

These stubs are sufficient for v0 (which uses only static rendering and
a default theme) and become concrete in FM04 / FM05 without breaking
the kernel contract.

#### 2.3.6 `RenderedPage`

The output of a web-backend renderer, before bundling and per-page
code-splitting.

```typescript
export interface RenderedPage {
  readonly kind: "RenderedPage";
  /** Route this page sits at. */
  readonly route: string;
  /** Full HTML document as a string. */
  readonly html: string;
  /**
   * Style rules actually matched by this page. The AOT compiler
   * uses this to produce per-page CSS.
   */
  readonly usedStyle: readonly StyleRuleId[];
  /**
   * Islands actually referenced on this page. The AOT compiler
   * uses this to bundle per-island JS.
   */
  readonly usedIslands: readonly IslandId[];
  /** Asset IDs referenced by this page. */
  readonly usedAssets: readonly LogicalId[];
  /** Meta tags (title, description, OG, etc.). */
  readonly meta: PageMeta;
  /** Source-map back to the originating Document. */
  readonly source: LogicalId;
}

export interface PageMeta {
  readonly title: string;
  readonly description: string | null;
  readonly canonicalUrl: string | null;
  readonly openGraph: ReadonlyRecord<string, string>;
  readonly structured: readonly JsonValue[]; // JSON-LD
  readonly extra: ReadonlyRecord<string, string>;
}

export type StyleRuleId = string & { readonly __brand: "StyleRuleId" };
export type IslandId    = string & { readonly __brand: "IslandId" };
```

#### 2.3.7 `PrintForme`

Backend-neutral composed page destined for a print backend (LaTeX,
direct PDF, EPUB). Layout is deferred to the backend.

```typescript
export interface PrintForme {
  readonly kind: "PrintForme";
  /** The document we're printing. */
  readonly source: LogicalId;
  /** Physical page settings (size, margins, orientation). */
  readonly page: PageSettings;
  /** Running headers and footers. */
  readonly runningElements: readonly RunningElement[];
  /** The document content, already transformed by print-specific stages. */
  readonly content: ContentNode;
  /** The style to apply. */
  readonly style: StyleDocument;
  /** Asset IDs referenced. */
  readonly usedAssets: readonly LogicalId[];
}

export interface PageSettings {
  readonly size: PageSize;
  readonly margins: Margins;
  readonly orientation: "portrait" | "landscape";
}

export type PageSize =
  | { readonly kind: "named"; readonly name: PageSizeName }
  | { readonly kind: "custom"; readonly w: Length; readonly h: Length };

export type PageSizeName =
  | "A4" | "A5" | "Letter" | "Legal" | "Tabloid" | "B5" | "B6";

export interface Margins {
  readonly top: Length;
  readonly right: Length;
  readonly bottom: Length;
  readonly left: Length;
}

export type Length =
  | { readonly unit: "pt"; readonly value: number }
  | { readonly unit: "mm"; readonly value: number }
  | { readonly unit: "in"; readonly value: number };

export interface RunningElement {
  readonly position: "header-left" | "header-center" | "header-right"
                   | "footer-left" | "footer-center" | "footer-right";
  readonly content: DocumentNode;
}
```

#### 2.3.8 `RequestHandler`

A dynamic, per-request function emitted by `render-dynamic` backends.

```typescript
export interface RequestHandler {
  readonly kind: "RequestHandler";
  /** Route pattern this handler responds to. */
  readonly routePattern: string;
  /**
   * Serialised handler code. The emit stage will bundle this as a
   * Worker, Node module, etc. Not executed during the pipeline.
   */
  readonly code: string;
  /**
   * Runtime environment requirements (Workers, Node 20+, etc.).
   */
  readonly runtime: RuntimeRequirement;
  /** Assets the handler needs available at runtime. */
  readonly staticAssets: readonly LogicalId[];
}

export type RuntimeRequirement =
  | { readonly kind: "cloudflare-worker" }
  | { readonly kind: "node"; readonly minVersion: string }
  | { readonly kind: "deno"; readonly minVersion: string }
  | { readonly kind: "bun"; readonly minVersion: string };
```

#### 2.3.9 `SearchIndex`

The opaque output of a search indexer.

```typescript
export interface SearchIndex {
  readonly kind: "SearchIndex";
  /** Which indexer produced this — stages handling indexes discriminate on this. */
  readonly indexer: string; // e.g. "pagefind", "minisearch", "sqlite-fts"
  readonly indexer_version: string;
  /**
   * The serialised index, possibly multi-file (sharded).
   * Each entry maps a file path (relative to site root) to its bytes.
   */
  readonly files: ReadonlyRecord<string, Uint8Array>;
  /** Small metadata snippet the client loads first. */
  readonly manifest: JsonValue;
}
```

#### 2.3.10 `Feed`

A syndication feed (RSS, Atom, JSON Feed, sitemap).

```typescript
export interface Feed {
  readonly kind: "Feed";
  readonly format: FeedFormat;
  /** The resulting file — `<path>` = `<bytes>`. */
  readonly files: ReadonlyRecord<string, Uint8Array>;
}

export type FeedFormat = "rss" | "atom" | "jsonfeed" | "sitemap";
```

#### 2.3.11 `DeployArtifact`

The final shippable thing an emitter produces.

```typescript
export interface DeployArtifact {
  readonly kind: "DeployArtifact";
  readonly variant: DeployVariant;
  /**
   * The artifact's file tree. For `dist-tree`, keys are file paths
   * within the artifact root.
   */
  readonly files: ReadonlyRecord<string, Uint8Array>;
  /**
   * Routing manifest: per-route, what file or handler serves it.
   */
  readonly manifest: DeployManifest;
}

export type DeployVariant =
  | { readonly kind: "dist-tree" } // static files
  | { readonly kind: "worker-bundle"; readonly runtime: RuntimeRequirement }
  | { readonly kind: "email-bundle" } // multipart MIME packages
  | { readonly kind: "epub-bundle" }
  | { readonly kind: "pdf"; readonly pageCount: number };

export interface DeployManifest {
  readonly routes: readonly DeployRoute[];
  readonly assets: readonly DeployAssetEntry[];
  readonly buildTime: string; // RFC 3339
  readonly buildId: RevisionId;
}

export interface DeployRoute {
  readonly pattern: string;
  readonly target:
    | { readonly kind: "file"; readonly path: string }
    | { readonly kind: "handler" };
  readonly islands: readonly IslandId[];
  readonly css: readonly string[];
}

export interface DeployAssetEntry {
  readonly id: LogicalId;
  readonly path: string;
  readonly mime: string;
  readonly sha256: string;
}
```

#### 2.3.12 `Stream<K>`

A meta-kind wrapping another kind to represent a lazy stream of values.
Stages declaring `Stream<K>` as input are called once per value
iterated. Stages declaring `Stream<K>` as output may produce zero,
one, or many values.

```typescript
export interface Stream<K extends KindDescriptor> {
  readonly kind: "Stream";
  readonly inner: K;
  readonly iterator: () => AsyncIterable<KindPayload<K>>;
}
```

See §3.6 for how stages actually produce and consume streams.

### 2.4 Kind descriptors — runtime representation

```typescript
export interface KindDescriptor {
  /** The kind name, e.g. "ContentNode" or "ext:youtube-embed". */
  readonly name: KindName;
  /** Semver-compatible version of the kind's shape. */
  readonly version: string;
  /** Optional discriminant for polymorphic kinds. */
  readonly discriminant?: string;
  /**
   * Kind-specific constraints the stage accepts/produces — e.g.
   * a parser that accepts only `ContentSource` with `mimeType`
   * starting with "text/markdown".
   */
  readonly constraints?: ReadonlyRecord<string, JsonValue>;
}
```

Every type in §2.3 has a canonical descriptor:

```typescript
export const Kinds = {
  Void:            { name: "Void",            version: "1.0" },
  ContentSource:   { name: "ContentSource",   version: "1.0" },
  ContentNode:     { name: "ContentNode",     version: "1.0" },
  Collection:      { name: "Collection",      version: "1.0" },
  Asset:           { name: "Asset",           version: "1.0" },
  Document:        { name: "Document",        version: "1.0" },
  RenderedPage:    { name: "RenderedPage",    version: "1.0" },
  PrintForme:      { name: "PrintForme",      version: "1.0" },
  RequestHandler:  { name: "RequestHandler",  version: "1.0" },
  SearchIndex:     { name: "SearchIndex",     version: "1.0" },
  Feed:            { name: "Feed",            version: "1.0" },
  DeployArtifact:  { name: "DeployArtifact",  version: "1.0" },
} as const satisfies Record<string, KindDescriptor>;
```

`Kinds.Void` is the special descriptor with no payload, used by source
stages that take no upstream input.

Version semantics:

- **Major** bump when the shape has a breaking change. Stages declaring
  the old major are incompatible with producers of the new major.
- **Minor** bump when a backward-compatible field is added. Producers of
  the new minor can feed consumers of the old minor (the consumer
  ignores the new field).

The kernel's initial kinds are all `"1.0"`.

### 2.5 Kind extensibility

Plugins register new kinds at manifest-load time:

```toml
# In the plugin manifest:
[[extends.kind]]
name          = "ext:youtube-embed"
version       = "1.0"
schema        = "./schema/youtube-embed.json"
subtype-of    = "ContentNode"  # optional
```

The host registers the new kind descriptor and makes it available to
stages that declare it as input or output.

Kernel-level kinds cannot be extended — plugins cannot change
`ContentSource` — but they can declare kinds that are **subtypes** of
kernel kinds. A stage declaring `ext:youtube-embed` as input accepts
any `ContentNode` that matches the subtype's constraints.

Subtypes let plugins specialise. For instance, a first-party
`parse-markdown` produces plain `ContentNode`; a plugin can register
`ext:blog-post` as a subtype of `ContentNode` with required frontmatter
fields, and a downstream `collect-chronological` that declares
`ext:blog-post` as input statically excludes non-posts.

### 2.6 Kind compatibility rules

When the orchestrator wires stage A's output edge to stage B's input
edge, it checks:

1. **Name match or subtype.** `B.consumes.name === A.produces.name`,
   OR `B.consumes.name` is a subtype of `A.produces.name` (via the
   `subtype-of` chain in the kind registry).
2. **Version compatibility.** `A.produces.version >= B.consumes.version`,
   subject to major-version match. A `1.2` producer can feed a `1.1`
   consumer; a `2.0` producer cannot feed a `1.x` consumer without an
   explicit adapter.
3. **Discriminant match (if declared).** If both declare a
   discriminant, they must be equal.
4. **Constraint satisfaction.** Constraints are open-ended. A
   well-formed stage documents the constraints it respects and
   produces. The orchestrator does best-effort structural matching;
   unmatched constraints generate warnings, not errors, so the
   user can declare custom constraints without knowing how to teach
   the orchestrator about them.

If compatibility fails, the orchestrator refuses to start the pipeline
with an error pointing at the exact edge.

### 2.7 Utility types

```typescript
export type JsonValue =
  | null | boolean | number | string
  | readonly JsonValue[]
  | { readonly [key: string]: JsonValue };

export type ReadonlyRecord<K extends string, V> = {
  readonly [key in K]: V;
};

/**
 * Given a `KindDescriptor`, the corresponding TypeScript payload type.
 * Stages use this to type their run method. See Appendix A.
 */
export type KindPayload<K extends KindDescriptor> = /* … see Appendix A … */;
```

---

## 3. The Stage Contract

### 3.1 The `Stage<In, Out>` interface

```typescript
export interface Stage<
  In extends KindDescriptor = KindDescriptor,
  Out extends KindDescriptor = KindDescriptor
> {
  // ─── Static identification ──────────────────────────────────────
  /** Package-qualified name, e.g. "@forme/parse-markdown". */
  readonly name: string;
  /** Semver of this stage package. */
  readonly version: string;
  /** Forme kernel apiVersion this stage targets. */
  readonly apiVersion: number;
  /** Short human description for logs and tool UI. */
  readonly description: string;

  // ─── Type contract ──────────────────────────────────────────────
  readonly consumes: In;
  readonly produces: Out;

  // ─── Capability declarations ────────────────────────────────────
  /**
   * Every capability this stage may exercise. Stages that call
   * context APIs without a corresponding declaration will fail at
   * runtime with a CapabilityError.
   */
  readonly capabilities: readonly Capability[];

  // ─── Configuration ──────────────────────────────────────────────
  /**
   * JSON Schema describing the stage's configuration object. The
   * orchestrator validates pipeline config against this before
   * calling `run`. Null for stages with no config.
   */
  readonly configSchema: JsonSchema | null;

  // ─── Execution ──────────────────────────────────────────────────
  /**
   * Process a single input value to a single output, a Promise for a
   * single output, or a stream of outputs.
   *
   * A stage that declares `produces` with `kind: "Stream"` MUST return
   * an AsyncIterable matching the stream element type. A stage that
   * declares a non-stream `Out` MUST return a `KindPayload<Out>` or a
   * Promise for one. The orchestrator validates this at first call.
   *
   * Stages MUST NOT mutate their input. Stages MUST NOT retain
   * references to context APIs after `run` resolves.
   */
  run(
    input: KindPayload<In>,
    config: unknown,
    ctx: StageContext
  ): StageOutput<Out>;

  // ─── Optional lifecycle hooks ───────────────────────────────────
  /**
   * Called once before the first `run`. Stages may use this to
   * prepare caches or validate configuration.
   */
  init?(config: unknown, ctx: StageInitContext): Promise<void>;

  /**
   * Called once after the pipeline completes or cancels. Stages
   * MUST release any resources acquired in `init` or across `run`
   * calls.
   */
  dispose?(ctx: StageInitContext): Promise<void>;
}

export type StageOutput<Out extends KindDescriptor> =
  | KindPayload<Out>
  | Promise<KindPayload<Out>>
  | AsyncIterable<KindPayload<Out>>;

export type JsonSchema = JsonValue; // structural — runtime validator agnostic
```

### 3.2 Construction

Stages are **values**, not classes. The convention is for a stage
package to default-export a factory:

```typescript
// @forme/parse-markdown/src/index.ts
import { defineStage, Kinds } from "@coding-adventures/forme-types";

export default defineStage({
  name: "@forme/parse-markdown",
  version: "0.1.0",
  apiVersion: 1,
  description: "Parses CommonMark + GFM into a ContentNode.",
  consumes: Kinds.ContentSource,
  produces: Kinds.ContentNode,
  capabilities: [],
  configSchema: markdownConfigSchema,
  async run(source, config, ctx) { /* … */ },
});
```

`defineStage` is a no-op at runtime that exists to improve TypeScript
inference; it just returns the object literal narrowed to the
type-parameterised `Stage<In, Out>`. This keeps stages debuggable as
plain objects with no hidden state.

### 3.3 Purity and hidden state

Stages MUST be pure in the functional sense:

- No module-level mutable state. Top-level `let` is forbidden. Top-level
  `const` is fine for truly constant data.
- No implicit dependencies. The only state a stage may read comes from
  `input`, `config`, or `ctx`. The only state a stage may write is its
  return value.
- No ambient I/O. A stage that needs filesystem, network, environment,
  or time access declares the capability and receives a wrapped API
  through `ctx`. Direct `fetch`, `fs.readFile`, `process.env`, `Date.now`
  will throw `CapabilityError` when called from inside a plugin's
  sandbox (§5.3).

Rationale: purity is what makes the pipeline testable, cacheable, and
parallelisable. A single hidden cache inside a stage package becomes a
debugging disaster three versions later when its behavior changes based
on invocation order.

The sole exception is the `Logger` available via `ctx.logger`, which
produces observable side effects for humans but does not influence
pipeline outputs.

### 3.4 Determinism

A stage's output MUST be a pure function of `(input, config)`, given
fixed capability outputs. If two invocations with the same `input` and
`config` produce different outputs without changes in context-provided
data, the stage is incorrect.

Determinism is what lets the orchestrator's cache (FM03) skip
re-executing a stage whose inputs haven't changed. Stages that genuinely
need non-determinism — e.g. a stage that picks a build timestamp —
must read it from `ctx.time` (which is deterministic in reproducible-build
mode) or declare a capability that disables caching for that stage.

### 3.5 Error semantics

A stage signals failure by throwing or returning a rejected Promise.
The kernel provides `StageError` (§6) with provenance fields the stage
SHOULD populate:

```typescript
throw new StageError({
  code: "PARSE_ERROR",
  message: "Invalid frontmatter on line 3",
  inputPath: source.path,
  inputId: source.identity,
  recoverable: false,
});
```

An unhandled non-`StageError` is wrapped by the orchestrator into a
`StageError` with `code: "UNCAUGHT"` and the original error as `cause`.

A stage MAY throw a `CancellationError` in response to
`ctx.cancellation` being signalled (§4.4). It MUST NOT catch and
swallow cancellations.

### 3.6 Input/output forms

The kernel distinguishes three forms:

| Form      | Input                       | Output                   | Use case                          |
| --------- | --------------------------- | ------------------------ | --------------------------------- |
| Single    | `KindPayload<In>`           | `KindPayload<Out>` / Promise | Pure transforms              |
| Streaming | `Stream<In>`                | `Stream<Out>`            | Fan-out (sources, collectors)    |
| Hybrid    | `Stream<In>` → reduce       | `KindPayload<Out>`       | Collectors, aggregators          |

A source stage produces `Stream<ContentSource>` because a filesystem
directory has many files. A parser stage consumes `ContentSource`,
produces `ContentNode`, and the orchestrator calls it once per
streamed source. A collector stage consumes a stream of `ContentNode`
and produces a single `Collection`.

Formally:

```typescript
// Source: produces a stream.
consumes: Kinds.Void;
produces: { ...Kinds.ContentSource, kind: "Stream" };

// Parser: one-to-one.
consumes: Kinds.ContentSource;
produces: Kinds.ContentNode;

// Collector: reduces a stream.
consumes: { ...Kinds.ContentNode, kind: "Stream" };
produces: Kinds.Collection;
```

### 3.7 Parallelism

The orchestrator is free to invoke a stage on many inputs in parallel
(up to its configured concurrency limit) if the stage is a pure
one-to-one transform. Stages MUST NOT assume single-threaded
invocation. Per-invocation state goes in local variables; cross-
invocation state is forbidden (§3.3).

---

## 4. `StageContext`

The object stages receive as their third argument. It bundles the
runtime facilities every capability-gated API the stage might need,
plus logging and cancellation.

### 4.1 Shape

```typescript
export interface StageContext {
  /** Logger for diagnostic output. No capability needed. */
  readonly logger: Logger;
  /** Cancellation signal (§4.4). */
  readonly cancellation: CancellationToken;
  /** Reproducible-build clock (§4.3). */
  readonly time: Clock;
  /** Stage-local cache (§4.5). */
  readonly cache: Cache;
  /** Per-stage telemetry emitter (§4.6). */
  readonly telemetry: TelemetryEmitter;

  /** Capability-gated APIs. Access gated on declarations (§5). */
  readonly storage: StorageApi;        // capability: storage:*
  readonly network: NetworkApi;        // capability: network:*
  readonly env: EnvApi;                // capability: env:*
  readonly filesystem: FilesystemApi;  // capability: filesystem:*
  readonly shell: ShellApi;            // capability: system:shell (first-party only)

  /** Event bus for cross-stage coordination (§4.7). */
  readonly events: EventBus;
}
```

An `init` hook receives `StageInitContext`, which is the same shape
minus per-run concerns:

```typescript
export interface StageInitContext
  extends Omit<StageContext, "cancellation" | "cache"> {
  /** Config passed to the stage — already validated against configSchema. */
  readonly config: unknown;
}
```

### 4.2 Logger

```typescript
export interface Logger {
  trace(message: string, fields?: Record<string, JsonValue>): void;
  debug(message: string, fields?: Record<string, JsonValue>): void;
  info(message: string, fields?: Record<string, JsonValue>): void;
  warn(message: string, fields?: Record<string, JsonValue>): void;
  error(message: string, fields?: Record<string, JsonValue>): void;
  /** Scoped logger; child inherits the parent's fields. */
  child(fields: Record<string, JsonValue>): Logger;
}
```

The logger is always available and never fails. Output routing
(console, file, structured JSON) is configured by the orchestrator.

### 4.3 Clock

```typescript
export interface Clock {
  /** Current UTC time in milliseconds since epoch. */
  nowMs(): number;
  /** Current UTC time as RFC 3339. */
  nowIso(): string;
  /** Monotonic timestamp in milliseconds. */
  monotonicMs(): number;
}
```

In **reproducible-build mode** (set in orchestrator config), `nowMs`
and `nowIso` return a fixed value for the duration of the build. This
is what makes two consecutive builds of the same inputs produce
byte-identical outputs.

### 4.4 Cancellation

```typescript
export interface CancellationToken {
  readonly cancelled: boolean;
  /** Reason for cancellation, if any. */
  readonly reason: string | null;
  /**
   * Throws `CancellationError` if cancellation has been requested.
   * Stages call this at safe points inside long-running work.
   */
  throwIfCancelled(): void;
  /** Register a cleanup callback. Called when cancellation fires. */
  onCancel(callback: () => void): void;
  /** Shortcut for `AbortSignal` interop. */
  readonly signal: AbortSignal;
}
```

Stages SHOULD honor cancellation at loop boundaries and before
expensive operations. The orchestrator cancels a pipeline when:

- The user hits Ctrl-C in the CLI.
- A peer stage errors with a non-recoverable failure (fail-fast
  orchestrator modes).
- The overall build deadline expires.

### 4.5 Cache

A stage-local, content-addressed cache.

```typescript
export interface Cache {
  /**
   * Cache key namespaced by stage name. Returns the cached value
   * if present, else calls `compute`, stores, and returns.
   */
  getOrCompute<T>(
    key: string,
    compute: () => Promise<T>
  ): Promise<T>;

  /** Invalidate a key. */
  invalidate(key: string): Promise<void>;

  /** Typed key helper. */
  keyFor(parts: readonly (string | number)[]): string;
}
```

The cache is managed by the orchestrator (FM03) and scoped per stage,
per pipeline-run namespace. Stages should NOT build their own caches;
they should use `ctx.cache`.

### 4.6 TelemetryEmitter

```typescript
export interface TelemetryEmitter {
  /**
   * Emit a structured telemetry event. The event schema is declared
   * by the stage's package — unknown event names are dropped.
   */
  emit(event: string, fields: Record<string, JsonValue>): void;
}
```

Telemetry is always capability-gated — a stage that calls `emit`
without declaring `telemetry:emit` gets a no-op emitter. See §9 of
FM00 for the discipline around what telemetry may carry.

### 4.7 EventBus

```typescript
export interface EventBus {
  /** Emit an event. Other stages subscribed to the name receive it. */
  emit(event: string, payload: JsonValue): void;

  /**
   * Subscribe to an event. Returns an unsubscribe function. The
   * subscription is torn down automatically on stage dispose.
   */
  on(event: string, handler: (payload: JsonValue) => void): () => void;
}
```

The event bus is intended for *coordination*, not data flow. Data flows
along the pipeline's typed edges. Events are for things like
"incremental rebuild invalidated ID X" or "preview server wants a
flush." Using events to smuggle unstructured data between stages is a
smell and is caught by code review, not the type system.

### 4.8 Capability-gated APIs

Each API is fully specified. A stage that tries to call an API without
having declared the corresponding capability receives the **no-op
variant** of that API — methods throw `CapabilityError` with a helpful
message.

#### 4.8.1 StorageApi (capability: `storage:read`, `storage:write`)

```typescript
export interface StorageApi {
  /** Read bytes at `path`. Throws if the file does not exist. */
  read(path: string): Promise<Uint8Array>;

  /** Write bytes at `path`. Creates parent directories as needed. */
  write(path: string, bytes: Uint8Array): Promise<void>;

  /** Check existence without reading. */
  exists(path: string): Promise<boolean>;

  /** List entries in a directory. */
  list(path: string): AsyncIterable<StorageEntry>;

  /** Watch a path for changes; used by the dev server. */
  watch(path: string): AsyncIterable<StorageChange>;

  /** Remove a file. Does NOT delete directories. */
  remove(path: string): Promise<void>;

  /** Metadata without reading content. */
  stat(path: string): Promise<StorageStat>;
}

export interface StorageEntry {
  readonly path: string;
  readonly type: "file" | "dir" | "symlink";
}

export interface StorageChange {
  readonly path: string;
  readonly kind: "added" | "modified" | "removed";
}

export interface StorageStat {
  readonly size: number;
  readonly mtimeMs: number;
  readonly type: "file" | "dir" | "symlink";
}
```

`storage:read` gates read/list/exists/stat/watch. `storage:write` gates
write/remove. A stage that needs both declares both.

The `path` here is relative to the pipeline's **configured storage
root**, not the process working directory. Escape attempts
(`../../../etc/passwd`) are refused by the host.

#### 4.8.2 NetworkApi (capability: `network:*` or `network:<host>`)

```typescript
export interface NetworkApi {
  fetch(input: string | Request, init?: RequestInit): Promise<Response>;
}
```

Capability scoping:

- `network:*` — unrestricted fetch. Shown as a warning on install.
- `network:<host>` — restricted to one host (and its subdomains).
  Wildcards via explicit declaration: `network:*.google.com`.
- `network:<scheme>:<host>` — further restricted by scheme.

The host rewrites `globalThis.fetch` inside the stage's sandbox to
reject unauthorised origins before the request is dispatched.

#### 4.8.3 EnvApi (capability: `env:<var>`)

```typescript
export interface EnvApi {
  get(name: string): string | undefined;
  getOrThrow(name: string): string;
}
```

Capabilities are per-variable: `env:GITHUB_TOKEN`, `env:CLOUDFLARE_API_KEY`.
Wildcard `env:*` exists but is a warning.

#### 4.8.4 FilesystemApi (capability: `filesystem:user`)

A direct-to-user-filesystem API distinct from `StorageApi`. The latter
is scoped to the pipeline's root; `FilesystemApi` reaches into arbitrary
paths. Only first-party stages typically need this (e.g. the desktop
shell reading a user-selected file). Third-party plugins requesting
this are a strong install-time warning.

```typescript
export interface FilesystemApi {
  readAbsolute(path: string): Promise<Uint8Array>;
  writeAbsolute(path: string, bytes: Uint8Array): Promise<void>;
  homeDir(): string;
  tempDir(): string;
}
```

#### 4.8.5 ShellApi (capability: `system:shell`)

```typescript
export interface ShellApi {
  run(
    command: string,
    args: readonly string[],
    options?: ShellOptions
  ): Promise<ShellResult>;
}

export interface ShellOptions {
  readonly cwd?: string;
  readonly env?: Record<string, string>;
  readonly timeoutMs?: number;
  readonly stdin?: Uint8Array;
}

export interface ShellResult {
  readonly exitCode: number;
  readonly stdout: Uint8Array;
  readonly stderr: Uint8Array;
}
```

**`system:shell` is never granted to third-party plugins.** First-party
stages that shell out (e.g. a LaTeX renderer invoking `xelatex`)
declare it and survive install-time review. Third-party equivalents
must be rewritten to avoid shell execution, or the user must manually
grant the capability with a stark warning.

---

## 5. The Capability Model

### 5.1 Capability names

A capability is a string. Conventionally structured as
`<realm>:<scope>[:<detail>]`:

```
storage:read
storage:write
storage:read-db
network:*
network:cloudflare
network:api.github.com
env:GITHUB_TOKEN
env:*
filesystem:user
system:shell
system:time-nondeterministic
content:extend
editor:inject-ui
telemetry:emit
```

A plugin declares the list of capability strings it exercises. The host
parses these into a `ParsedCapability` tree at load time.

```typescript
export type Capability = string;

export interface ParsedCapability {
  readonly realm: string;
  readonly scope: string;
  readonly detail: string | null;
  readonly wildcard: boolean; // true if any segment was "*"
}

export function parseCapability(cap: Capability): ParsedCapability;
export function matchesCapability(declared: Capability, requested: Capability): boolean;
```

### 5.2 Built-in capability realms

| Realm        | Example scopes                                      |
| ------------ | --------------------------------------------------- |
| `storage`    | `read`, `write`, `read-db`, `watch`                 |
| `network`    | `*`, any host, `<scheme>:<host>`                    |
| `env`        | `*`, `<VAR_NAME>`                                   |
| `filesystem` | `user`, `temp`                                      |
| `system`     | `shell` (first-party only), `time-nondeterministic` |
| `content`    | `extend`                                            |
| `editor`     | `inject-ui`, `command`, `sidebar`                   |
| `telemetry`  | `emit`                                              |
| `plugin`     | `load`, `register-kind`                             |

Plugins MAY define new realms under the `ext:` namespace for their own
extension points, but the host treats them as opaque — they gate only
calls into APIs the plugin itself provides, not anything kernel-level.

### 5.3 Enforcement

Enforcement happens in three places:

1. **Static — manifest validation.** Manifests with malformed
   capability strings fail to load.
2. **Runtime — API gating.** Every API in `StageContext` that
   corresponds to a capability is wrapped. A call without the
   corresponding declaration receives a no-op API whose methods throw
   `CapabilityError`. The host constructs the wrappers from the
   declared list; a stage cannot upgrade itself.
3. **Sandbox — ambient access blocking.** The plugin host runs plugin
   code in a context where `globalThis.fetch`, `require('fs')`,
   `process.env`, `Date.now`, and `child_process` are either
   inaccessible or re-routed to the gated versions.

The third layer is the strongest. Even if a stage implementor forgets
to go through `ctx.network`, the sandbox catches the direct call. The
first two layers are for correctness and ergonomics; the third is
defense in depth.

**Sandbox implementation note:** exact mechanism varies by runtime.
In Node (desktop shell and CLI), we use a VM context with a
custom Module loader and restricted builtins. In the browser (editor
preview), we use a Worker with `postMessage`-based APIs. FM02 (plugin
host) owns the detailed design.

### 5.4 Granting flow

Capabilities are granted at plugin **install time**, not every run.
The install UI shows the list, explains each, and asks for explicit
approval:

```
@forme/publish-mailgun asks for:
  ✓ network:api.mailgun.com   — send newsletter emails
  ✓ env:MAILGUN_API_KEY       — authenticate with Mailgun

  [ Install ]   [ Cancel ]
```

Revoking a capability later uninstalls or disables the plugin. There
is no "this plugin has it installed but is denied network access right
now" state — either the plugin is fully trusted to its declared scope,
or it is uninstalled.

Sensitive capabilities (`network:*`, `env:*`, `filesystem:user`,
`system:shell`) always surface a stark warning in the install UI.

### 5.5 First-party capabilities

First-party stages ship with the kernel and are given access to all
capabilities their manifest declares automatically (no install-time
prompt). This is the only difference between first-party and
third-party — the permission gate itself is identical.

The list of first-party package names is declared in the host's
configuration. A package publishing itself as `@forme/…` is a
convention signalling first-party but is not enforced at that layer;
the host's declared list is the source of truth.

---

## 6. Error Model

### 6.1 `StageError`

The primary error type thrown by stages.

```typescript
export interface StageErrorInit {
  readonly code: string;
  readonly message: string;
  readonly inputPath?: string;
  readonly inputId?: LogicalId;
  readonly stageName?: string;
  readonly cause?: unknown;
  readonly recoverable?: boolean;
  readonly fields?: Record<string, JsonValue>;
}

export class StageError extends Error {
  readonly code: string;
  readonly inputPath: string | null;
  readonly inputId: LogicalId | null;
  readonly stageName: string | null;
  readonly cause: unknown;
  readonly recoverable: boolean;
  readonly fields: Readonly<Record<string, JsonValue>>;

  constructor(init: StageErrorInit);

  /** A stable, machine-parseable representation for logs and diagnostics. */
  toJson(): JsonValue;
}
```

Codes are namespaced per realm:

```
PARSE_ERROR
PARSE_FRONTMATTER_INVALID
PARSE_NO_DOCUMENT
TRANSFORM_*
COLLECT_*
RENDER_*
EMIT_*
CAPABILITY_DENIED
CANCELLED
UNCAUGHT
TIMEOUT
IO_NOT_FOUND
IO_PERMISSION_DENIED
NETWORK_UNREACHABLE
```

The kernel maintains a canonical list; plugins SHOULD use its codes
when semantically applicable and MAY define their own under a
package-prefixed name (`foo/MY_SPECIAL_CODE`).

### 6.2 `CapabilityError`

Subclass of `StageError` with `code: "CAPABILITY_DENIED"` and a
structured `capability` field.

```typescript
export class CapabilityError extends StageError {
  readonly capability: string;

  constructor(init: StageErrorInit & { capability: string });
}
```

### 6.3 `CancellationError`

```typescript
export class CancellationError extends Error {
  readonly reason: string | null;

  constructor(reason?: string);
}
```

`CancellationError` is NOT a `StageError`. It propagates through the
orchestrator and unwinds the pipeline without triggering fallback /
retry logic.

### 6.4 Orchestrator handling (preview)

Full orchestrator error semantics live in FM03. The kernel promises:

- A stage's `StageError` with `recoverable: true` MAY be skipped by
  the orchestrator, emitting a warning; others with the same input
  continue.
- A `StageError` with `recoverable: false` terminates the pipeline
  by default (configurable to "best-effort").
- A `CapabilityError` always terminates.
- A `CancellationError` always propagates.
- Non-`StageError` throws become `StageError { code: "UNCAUGHT", cause }`.

---

## 7. Identity and Content Addressing

### 7.1 Logical identity vs revision identity

Two distinct IDs:

- **`LogicalId`** — identifies a *thing over time*. "This particular
  post," "this author," "this asset." Stable across content edits
  and file renames.
- **`RevisionId`** — identifies *a specific version of a thing*.
  Changes whenever content changes. Two entities with the same
  `RevisionId` are byte-identical.

```typescript
/** UUIDv7 as a branded string. Assigned when the entity first appears. */
export type LogicalId = string & { readonly __brand: "LogicalId" };

/**
 * `blake3:<hex>` — a content hash over the entity's canonical
 * serialisation. Changes with any content change.
 */
export type RevisionId = string & { readonly __brand: "RevisionId" };
```

### 7.2 Assigning `LogicalId`

Sources are responsible for assigning (or resolving) a `LogicalId` for
every `ContentSource` they emit.

- **Filesystem source** — derives from a persistent `id.json` adjacent
  to the source file, or frontmatter `id:` field. If none, generates a
  new UUIDv7 on first encounter and writes it back.
- **Database source** — uses the primary key, possibly namespaced.
- **External sources** (Notion, CMS) — mirrors the provider's ID.

Under rename, the `LogicalId` must persist. Sources MUST NOT generate
a new ID when a file is moved; the `id.json` / frontmatter field
travels with the file.

### 7.3 Computing `RevisionId`

A BLAKE3 hash over the canonical JSON serialisation of the entity. The
kernel package `forme-identity` provides:

```typescript
export function computeRevisionId(payload: JsonValue): RevisionId;

export function canonicalJson(value: JsonValue): string;
```

Canonical JSON rules (the same across the ecosystem — matches
RFC 8785 JCS):

- Object keys sorted lexicographically.
- No whitespace.
- Numbers normalised (e.g. `1.0` → `1`).
- UTF-8 text.

For `ContentNode`, the canonical form is the document AST + frontmatter
+ logical identity. `sourcePath` and similar transient fields are
excluded from the hash. This allows a post moved to a new filesystem
location to retain its revision ID.

### 7.4 Relationship to `content_addressable_storage`

The existing `content_addressable_storage` package in the monorepo
implements the same conceptual scheme. `forme-identity` either:

1. Depends on it directly and exposes a thin type-branded wrapper, or
2. Re-exports its functions under Forme-specific names.

The choice is a per-package spec (FM01a or similar) once
`content_addressable_storage`'s public API is reviewed for fit.

### 7.5 Use in caching

The orchestrator's cache is keyed by `(stage.name, config_hash, input_revision)`.
Because `RevisionId` is deterministic and stable, the cache correctly
hits on re-runs where nothing changed — even across renames, because
the revision is computed from content, not path.

---

## 8. Plugin Manifest v1

### 8.1 File format and location

The manifest is a TOML file named `plugin.toml`, located at the root
of the plugin package. It is loaded alongside the package's `package.json`;
the plugin host uses `package.json` only for the JavaScript loader
(`main`, `type: "module"`) and gets everything else from `plugin.toml`.

### 8.2 Schema

```toml
# ─── Top-level plugin identity ─────────────────────────────────────

[plugin]
name        = "@forme/embed-youtube"       # package-qualified name
version     = "0.1.0"                      # semver
api-version = 1                            # kernel API version targeted
entry       = "./dist/index.js"            # JS entry point (ES module)
description = "YouTube embed block"
license     = "MIT"
homepage    = "https://github.com/acme/forme-youtube"
repository  = "https://github.com/acme/forme-youtube.git"

# ─── Dependency declarations ───────────────────────────────────────

[requires]
# Plugins the host must load before this one.
plugins     = { "@forme/editor-core" = ">=0.1" }
# Kernel compatibility.
kernel      = ">=0.1 <0.2"

# ─── Capabilities ──────────────────────────────────────────────────

[capabilities]
declared = [
  "content:extend",
  "network:www.youtube.com",
  "network:i.ytimg.com",
]

# ─── Contributions ─────────────────────────────────────────────────
# Each [[extends.X]] block registers one extension at load time.

# A new kind:
[[extends.kind]]
name      = "ext:youtube-embed"
version   = "1.0"
subtype-of = "ContentNode"
schema    = "./schema/youtube-embed.json"

# A new content node type (extends the Content IR block vocabulary):
[[extends.content-block]]
type          = "custom_embed"
discriminant  = "youtube"
schema        = "./schema/youtube-block.json"

# An editor block:
[[extends.editor-block]]
for-kind      = "custom_embed"
for-disc      = "youtube"
component     = "./dist/editor.js#YoutubeEditor"
capabilities  = ["editor:inject-ui"]

# A backend handler:
[[extends.backend-handler]]
for-kind      = "custom_embed"
for-disc      = "youtube"
for-backend   = "web"
handler       = "./dist/render-web.js#renderYoutubeWeb"

[[extends.backend-handler]]
for-kind      = "custom_embed"
for-disc      = "youtube"
for-backend   = "latex"
handler       = "./dist/render-latex.js#renderYoutubeLatex"

# A pipeline stage:
[[extends.stage]]
name          = "@forme/embed-youtube/resolve-oembed"
entry         = "./dist/stage.js#default"

# ─── Integrity ─────────────────────────────────────────────────────

[integrity]
# Present on published releases. Verified by the host at install.
signature   = "..."
public-key  = "..."
```

### 8.3 Required fields

Any manifest must contain:

- `[plugin]` with `name`, `version`, `api-version`, `entry`,
  `description`, `license`.
- `[capabilities]` with a `declared` array (may be empty).
- At least one `[[extends.…]]` block OR a `[[extends.stage]]` block.

A plugin that does nothing is rejected at load time.

### 8.4 `api-version` compatibility

The kernel exposes a version constant:

```typescript
export const KERNEL_API_VERSION = 1;
```

A plugin loads only if `plugin.api-version === KERNEL_API_VERSION`.
A plugin targeting an older version fails to load; the host reports
the mismatch with upgrade guidance.

The kernel promises:

- Within an `apiVersion`, no breaking changes to the types or
  signatures in FM01.
- Additions within an `apiVersion` are either backward-compatible
  or confined to new, opt-in extension points.
- An `apiVersion` bump ships a migration guide describing every breaking
  change, with an automated codemod where practical.

### 8.5 Integrity

Signed manifests ship with `[integrity].signature` (Ed25519) and
`[integrity].public-key`. The host verifies on install and on every
load. Unsigned plugins install only with an explicit flag and an
install-time warning ("this plugin is unsigned; anyone on your network
could substitute it").

Lockfile: the host maintains `forme.lock` in the pipeline directory,
pinning every loaded plugin's version and hash. Subsequent loads verify
the hash; drift triggers a reinstall with user confirmation.

---

## 9. Package Layout

The kernel is six npm packages under
`code/packages/typescript/forme-*`. Each has its own `package.json`,
`BUILD` / `BUILD_windows`, `README.md`, and `CHANGELOG.md` per the
repo's standards (CLAUDE.md §10 `Publishable packages`).

### 9.1 `@coding-adventures/forme-types`

Pure types. No runtime code except `Kinds`, `KERNEL_API_VERSION`, and
`defineStage` helper. No dependencies beyond `@coding-adventures/document-ast`.

- `src/kinds.ts` — `KINDS`, `Kinds`, `KindDescriptor`, `KindName`, `KindPayload`
- `src/content.ts` — `ContentSource`, `ContentNode`, `AssetRef`, etc.
- `src/collection.ts` — `Collection`, `CollectionEntry`, `OrderKey`
- `src/asset.ts` — `Asset`, `AssetRole`
- `src/document.ts` — `Document`, stub `StyleDocument`, stub `Interactivity`
- `src/rendered.ts` — `RenderedPage`, `PageMeta`
- `src/print.ts` — `PrintForme`, `PageSettings`, `Length`
- `src/handler.ts` — `RequestHandler`, `RuntimeRequirement`
- `src/index-artifacts.ts` — `SearchIndex`, `Feed`
- `src/deploy.ts` — `DeployArtifact`, `DeployManifest`, `DeployRoute`
- `src/json.ts` — `JsonValue`, `ReadonlyRecord`
- `src/version.ts` — `KERNEL_API_VERSION`
- `src/stage.ts` — `Stage<In, Out>`, `StageContext`, all `…Api` types, `defineStage`
- `src/index.ts` — barrel export

### 9.2 `@coding-adventures/forme-stage`

Helpers for authoring stages. The `Stage<In, Out>` interface lives in
`forme-types`; this package is runtime support.

- `src/define-stage.ts` — `defineStage(stage)` — passthrough with typing
- `src/compose.ts` — in-code pipeline composition helpers
- `src/testing.ts` — `createMockContext`, `runStage` utilities
- `src/void-kind.ts` — the `Kinds.Void` descriptor

### 9.3 `@coding-adventures/forme-capability`

Capability parsing, checking, and wrapping.

- `src/parse.ts` — `parseCapability`, `matchesCapability`
- `src/wrap.ts` — `wrapStorageApi`, `wrapNetworkApi`, etc.
- `src/errors.ts` — `CapabilityError`
- `src/registry.ts` — built-in capability list with descriptions for UIs

### 9.4 `@coding-adventures/forme-identity`

- `src/logical-id.ts` — UUIDv7 generation, branded type
- `src/revision-id.ts` — BLAKE3 hashing, `computeRevisionId`
- `src/canonical-json.ts` — RFC 8785-style canonical JSON serialisation
- `src/resolver.ts` — helpers for sources to assign/read LogicalIds

### 9.5 `@coding-adventures/forme-manifest`

Schema and parser for `plugin.toml`.

- `src/schema.ts` — TypeScript types for the manifest shape
- `src/parse.ts` — `parseManifest(toml: string): Manifest`
- `src/validate.ts` — structural validation errors
- `src/compat.ts` — `isCompatible(manifest, kernelVersion): boolean`
- `src/integrity.ts` — signature verification helpers

### 9.6 `@coding-adventures/forme-errors`

The error hierarchy.

- `src/stage-error.ts` — `StageError`
- `src/capability-error.ts` — `CapabilityError` (re-exported by `forme-capability`)
- `src/cancellation.ts` — `CancellationError`, `CancellationToken` type, `createCancellation()`
- `src/codes.ts` — canonical error-code list

### 9.7 Dependency graph

```
  document-ast
        ▲
        │
  forme-types ◄─── forme-stage
        ▲                      ▲
        │                      │
  forme-errors ◄───── forme-capability
        ▲
        │
  forme-identity (depends on forme-errors only)
        ▲
        │
  forme-manifest (depends on forme-errors and forme-types)
```

`forme-types` has no Forme dependencies — only `document-ast`. This is
deliberate: `forme-types` must be trivial to import into anything
without pulling the universe.

Per the repo's BUILD discipline (`lessons.md` 2026-04-21), every
package's `BUILD` and `BUILD_windows` lists transitive `file:` deps in
leaf-to-root order. The ordering follows the arrows above.

---

## 10. Testing Contract

Every kernel package ships tests exceeding 95% line and branch
coverage (CLAUDE.md §11).

### 10.1 `forme-types`

- Type-level tests (via `tsd` or `expect-type`) verifying all public
  types compile correctly and narrow as documented.
- Structural tests for `Kinds` descriptor constants.
- Property tests: every built-in kind descriptor round-trips through
  JSON.

### 10.2 `forme-stage`

- `defineStage` preserves object identity.
- `createMockContext` produces a context where every capability is
  either granted or denied per the mock's config.
- `runStage` correctly invokes init/run/dispose and threads errors.

### 10.3 `forme-capability`

- Parse tests for all canonical capability strings.
- Match tests: `network:*` matches `network:github.com`; the reverse
  does not; `network:a.b.com` matches `*.b.com`.
- Wrapping tests: an API wrapped without the capability throws
  `CapabilityError`; with the capability, passes through to the real impl.

### 10.4 `forme-identity`

- Golden tests: the canonical JSON of fixed inputs matches a recorded
  string.
- Round-trip: `parse(stringify(x)) === x` for representative JSON.
- Stability: a fixed `ContentNode` fixture produces the same
  `RevisionId` across runs.
- Ignored fields: changing `sourcePath` does not change the revision.

### 10.5 `forme-manifest`

- Parse tests for well-formed manifests.
- Validation tests for every documented rejection reason.
- `api-version` compatibility tests.
- Integrity tests (signed and unsigned paths).

### 10.6 `forme-errors`

- `StageError.toJson()` produces stable, documented shape.
- `CapabilityError` carries the capability field through serialisation.
- `CancellationError` is distinct from `StageError`.

### 10.7 Integration smoke test

A separate package `forme-kernel-integration-tests` wires the kernel
together in a minimal pipeline and runs:

1. Load a manifest, verify it parses.
2. Construct a `StageContext` with specific capabilities.
3. Run a stage that exercises each capability, confirm allowed calls
   succeed and denied calls throw.
4. Compute `RevisionId` for a `ContentNode` and verify stability.
5. Round-trip a `Collection` through JSON.

This test doubles as the build-verification entry point invoked by CI.

---

## 11. Examples

### 11.1 A minimal Stage

A stage that takes a `ContentSource` and produces a `ContentNode` by
parsing markdown. Trivial, real.

```typescript
// @forme/parse-markdown/src/index.ts
import { defineStage, Kinds, StageError } from "@coding-adventures/forme-types";
import { parseCommonMark } from "@coding-adventures/commonmark-parser";
import { parseFrontmatter } from "@coding-adventures/forme-parse-frontmatter";
import { computeRevisionId } from "@coding-adventures/forme-identity";

interface Config {
  readonly gfm: boolean;
}

const configSchema = { /* JSON Schema */ };

export default defineStage<typeof Kinds.ContentSource, typeof Kinds.ContentNode>({
  name: "@forme/parse-markdown",
  version: "0.1.0",
  apiVersion: 1,
  description: "Parses CommonMark (+ optional GFM) into a ContentNode.",
  consumes: Kinds.ContentSource,
  produces: Kinds.ContentNode,
  capabilities: [],
  configSchema,

  async run(source, config: Config, ctx) {
    ctx.cancellation.throwIfCancelled();

    const text = new TextDecoder("utf-8").decode(source.bytes);

    const { frontmatter, body } = parseFrontmatter(text);

    let document;
    try {
      document = parseCommonMark(body, { gfm: config.gfm });
    } catch (e) {
      throw new StageError({
        code: "PARSE_ERROR",
        message: e instanceof Error ? e.message : String(e),
        inputPath: source.path,
        inputId: source.identity,
        cause: e,
      });
    }

    const contentNode = {
      kind: "ContentNode" as const,
      identity: source.identity,
      revision: source.revision,        // preserved from source
      document,
      frontmatter,
      route: null,
      assetRefs: [],                    // populated by a later transform
      sourcePath: source.path,
    };

    return {
      ...contentNode,
      revision: computeRevisionId(contentNode),  // recomputed after parse
    };
  },
});
```

This stage:
- Declares zero capabilities (pure computation)
- Handles cancellation at entry
- Wraps underlying parser errors in `StageError` with full provenance
- Recomputes its output's `RevisionId` from its canonical form

### 11.2 A capability-gated stage

A source stage that reads markdown files from disk.

```typescript
import { defineStage, Kinds } from "@coding-adventures/forme-types";
import { assignOrGenerateId, computeRevisionId } from "@coding-adventures/forme-identity";

interface Config {
  readonly root: string;
  readonly glob: string;
}

export default defineStage({
  name: "@forme/source-fs",
  version: "0.1.0",
  apiVersion: 1,
  description: "Reads files from a local directory.",
  consumes: Kinds.Void,
  produces: { ...Kinds.ContentSource, kind: "Stream" },
  capabilities: ["storage:read"],
  configSchema,

  async *run(_void, config: Config, ctx) {
    const entries = ctx.storage.list(config.root);

    for await (const entry of entries) {
      ctx.cancellation.throwIfCancelled();
      if (entry.type !== "file") continue;
      if (!matches(entry.path, config.glob)) continue;

      const bytes = await ctx.storage.read(entry.path);
      const identity = await assignOrGenerateId(entry.path, ctx);

      const source = {
        kind: "ContentSource" as const,
        path: entry.path,
        bytes,
        mimeType: mimeFromPath(entry.path),
        identity,
        revision: "" as RevisionId,  // placeholder
        providerMeta: { mtimeMs: (await ctx.storage.stat(entry.path)).mtimeMs },
      };

      yield { ...source, revision: computeRevisionId({ ...source, revision: null }) };
    }
  },
});
```

This stage:
- Declares `storage:read`, which gates access to `ctx.storage`
- Produces a `Stream` (note the `async *run` generator)
- Yields values as it finds files, letting downstream parse in parallel
- Honors cancellation between files

### 11.3 A plugin extending a kind

A plugin that adds a YouTube embed block.

```toml
# plugin.toml

[plugin]
name        = "@forme/embed-youtube"
version     = "0.1.0"
api-version = 1
entry       = "./dist/index.js"
description = "Embed YouTube videos as a block."
license     = "MIT"

[capabilities]
declared = ["content:extend", "network:www.youtube.com"]

[[extends.content-block]]
type          = "custom_embed"
discriminant  = "youtube"
schema        = "./schema/block.json"

[[extends.backend-handler]]
for-kind      = "custom_embed"
for-disc      = "youtube"
for-backend   = "web"
handler       = "./dist/render.js#renderWeb"

[[extends.backend-handler]]
for-kind      = "custom_embed"
for-disc      = "youtube"
for-backend   = "latex"
handler       = "./dist/render.js#renderLatex"
```

The plugin adds no stages — it only contributes a content block kind
and renderers. The host registers these at load time and stages
downstream pick them up via the extension registry.

---

## 12. Open Questions

Items this spec deliberately defers.

1. **Module format for stages.** The spec assumes ESM. The
   `defineStage` factory returns a plain object; should we also
   support a class-based declaration for teams that prefer it? Held
   off on answering until a real plugin author asks.
2. **Concurrency control declarations.** A stage might want to say
   "run me at most 4 in parallel" for resource reasons. Currently
   this is the orchestrator's decision, not the stage's. Revisit
   when FM03 is drafted.
3. **Capability parameterisation beyond host suffix.** E.g.
   `storage:read:read-only-dirs` vs `storage:write:tmp-only`. For
   now the shape is `realm:scope`; finer-grained subtyping is
   deferred.
4. **Kind registry persistence.** Today a kind registered by a plugin
   is in-memory only. A long-running editor session across multiple
   pipeline runs might benefit from persisting the registry. Not a
   kernel concern in v0.
5. **Identity collisions across sources.** If two sources try to claim
   the same `LogicalId`, the orchestrator currently errors. Worth
   revisiting whether a source-scoped namespace is preferable.
6. **`JsonValue` vs `unknown` at stage boundaries.** `configSchema`
   validates `unknown` into a stage's config type. Forcing everything
   through `JsonValue` vs allowing generic TypeScript types is a
   tradeoff — the former is more portable, the latter is more ergonomic.
   Current choice: `unknown` passed to run, validated via JSON Schema.
7. **Structured diagnostics.** Today `StageError` has a `message` plus
   fields. A richer diagnostic format (think rustc's JSON output, with
   source spans and suggestions) would serve editor integration better.
   Candidate for FM01.1.
8. **BLAKE3 vs BLAKE2b vs SHA-256.** Current choice is BLAKE3 (fast,
   parallelisable). Revisit if a compelling reason surfaces — but the
   hash function is an `apiVersion`-bumpable detail.
9. **Stream backpressure.** `AsyncIterable` has no built-in backpressure
   beyond the consumer pulling one-at-a-time. If a source floods a
   slow parser, memory pressure builds. Need a backpressure discipline
   in FM03, but worth flagging here: stages MUST NOT accumulate
   buffered outputs beyond a configured window.

---

## 13. Success Criteria

FM01 is complete when:

1. **All six kernel packages exist** under `code/packages/typescript/forme-*`,
   each with `package.json`, `BUILD`, `BUILD_windows`, `README.md`, and
   `CHANGELOG.md`.
2. **Every type in §2 and §3 is exported** from `@coding-adventures/forme-types`
   exactly as specified, and a second reader can implement a new stage
   without reading any other FM spec.
3. **Test coverage exceeds 95%** across the kernel.
4. **A minimal end-to-end stage** (the `forme-parse-markdown` example in
   §11.1) compiles and runs against the kernel.
5. **`apiVersion` discipline is enforced** — a plugin targeting the
   wrong version fails to load with a clear error.
6. **Capability enforcement is testable** — running a stage that calls
   `ctx.network.fetch` without declaring `network:*` throws
   `CapabilityError` in the integration test.
7. **Reproducible builds are provable** — the integration smoke test
   runs twice in reproducible-build mode and produces byte-identical
   outputs from all stages.
8. **Documentation** for every exported symbol is complete enough that
   the next spec (FM02, plugin host) can reference FM01 types without
   needing supplementary text.

---

## Appendix A — Full `KindPayload` mapping

```typescript
export type KindPayload<K extends KindDescriptor> =
  K extends { name: "Void" }            ? void           :
  K extends { name: "ContentSource" }   ? ContentSource  :
  K extends { name: "ContentNode" }     ? ContentNode    :
  K extends { name: "Collection" }      ? Collection     :
  K extends { name: "Asset" }           ? Asset          :
  K extends { name: "Document" }        ? Document       :
  K extends { name: "RenderedPage" }    ? RenderedPage   :
  K extends { name: "PrintForme" }      ? PrintForme     :
  K extends { name: "RequestHandler" }  ? RequestHandler :
  K extends { name: "SearchIndex" }     ? SearchIndex    :
  K extends { name: "Feed" }            ? Feed           :
  K extends { name: "DeployArtifact" }  ? DeployArtifact :
  // Stream meta-kind
  K extends { kind: "Stream"; inner: infer I }
    ? I extends KindDescriptor
        ? AsyncIterable<KindPayload<I>>
        : never
  :
  // Plugin-extended kinds use module augmentation.
  K extends { name: `ext:${infer N}` }
    ? N extends keyof ExtKindPayloadMap
        ? ExtKindPayloadMap[N]
        : JsonValue
  :
  never;

/** Augmented by plugins via declaration merging. */
export interface ExtKindPayloadMap {}
```

A plugin that declares an extension kind augments the map:

```typescript
// In @forme/embed-youtube's type declarations:
declare module "@coding-adventures/forme-types" {
  interface ExtKindPayloadMap {
    "youtube-embed": {
      readonly videoId: string;
      readonly start: number | null;
    };
  }
}
```

---

## Appendix B — Glossary

Terms introduced in this spec; for the broader Forme vocabulary see
FM00 Appendix B.

- **apiVersion** — integer major version of the kernel contract.
  Plugins declare the one they target; the host refuses incompatible.
- **Canonical JSON** — RFC 8785-style serialisation with sorted keys,
  no whitespace, normalised numbers. Used for content-hash stability.
- **CapabilityError** — subclass of `StageError` for denied-capability
  accesses. Always fatal.
- **ContentSource / ContentNode / Collection / …** — built-in Kinds (§2).
- **defineStage** — compile-time helper; runtime no-op that improves
  TypeScript inference on stage definitions.
- **LogicalId** — UUIDv7-branded string identifying a thing over time.
- **Manifest** — `plugin.toml` describing a plugin to the host.
- **RevisionId** — `blake3:<hex>` branded string identifying an
  exact revision.
- **StageContext** — object passed as the third argument to `run`.
  Carries capability-gated APIs, logger, cancellation, cache, time,
  telemetry, events.
- **Stream** — meta-kind; an `AsyncIterable` of some other kind.
- **Subtype of** — relationship declared in a plugin's kind registration;
  allows a plugin kind to be accepted anywhere its supertype is.
- **Void** — the special Kind with no payload, used for source stages.

---

## Appendix C — Pointers to sibling specs

- **FM00** — Forme vision
- **FM02** (next) — Plugin host (loader, sandbox, extension registry)
- **FM03** — Orchestrator (DAG executor, cache, scheduling)
- **FM04** — Style IR (replaces the stub in §2.3.5)
- **FM05** — Interactivity IR (replaces the stub in §2.3.5)
- **FM06** — AOT compiler (per-page dependency analysis, bundling)

## Appendix D — This is a living document

Like FM00, FM01 evolves as implementation lands. Where `forme-types`
and FM01 disagree, the code wins and the spec is updated; the history
of the tension becomes part of the project's record.
