/**
 * Kind shapes — the TypeScript interfaces for each built-in kind.
 *
 * One interface per kind, each `readonly` end-to-end.  Stages should
 * never mutate kind values; the orchestrator caches and shares them
 * across consumers, so a sneaky in-place edit would corrupt the
 * downstream pipeline in a hard-to-reproduce way.
 *
 * See FM01 §2.3 for the design rationale of each kind.
 *
 * The shape types here intentionally omit a runtime discriminator (no
 * `kind: "ContentSource" as const` field on every type).  TypeScript's
 * structural typing handles "is this a ContentNode?" by checking the
 * shape; the runtime `KindDescriptor` from `kinds.ts` provides the
 * separate string discriminator the orchestrator uses for typing edges
 * in pipeline configs.  Mixing the two — putting `kind` on the value
 * AND on the descriptor — invited the "{ ...Kinds.X, kind: 'Stream' }"
 * confusion documented in `kinds.ts`.
 *
 * Kind shapes that reference unspecified kernel concepts (StyleDocument,
 * Interactivity) carry intentionally minimal stub types so the kernel
 * is buildable today.  FM04 (Style IR) and FM05 (Interactivity IR) will
 * tighten these without changing field names.
 */

import type { DocumentNode } from "@coding-adventures/document-ast";
import type { JsonValue, ReadonlyRecord } from "./utility.js";
import type { LogicalId, RevisionId } from "./identity.js";

// ─── Asset roles & references ─────────────────────────────────────────────

/**
 * The role an asset plays inside a content document.  Used by AssetRef
 * (which lives inside ContentNode) to tell renderers what kind of
 * placeholder to leave when the asset isn't ready yet, and used by Asset
 * itself to choose a default mime category.
 */
export type AssetRole =
  | "image"
  | "video"
  | "audio"
  | "font"
  | "embed"
  | "binary";

/**
 * A pointer from a ContentNode to an Asset it depends on.  The
 * `nodePath` is a list of child indices walking from the document root
 * to the referencing node, used by the orchestrator's incremental
 * rebuild to know which downstream stages need to re-run when an asset
 * changes.
 */
export interface AssetRef {
  readonly id: LogicalId;
  readonly nodePath: readonly number[];
  readonly role: AssetRole;
}

// ─── ContentSource ────────────────────────────────────────────────────────

/**
 * Raw bytes plus enough metadata for downstream parsers to know what
 * they're looking at.  Produced by sources, consumed by parsers.
 *
 * `bytes` is a `Uint8Array`, not a string, because not every source is
 * UTF-8 — a parser might need to detect or honour an explicit encoding
 * before decoding.  The `mimeType` field is advisory; parsers that care
 * about mime sniff again to be sure.
 */
export interface ContentSource {
  readonly path: string;
  readonly bytes: Uint8Array;
  readonly mimeType: string | null;
  readonly identity: LogicalId;
  readonly revision: RevisionId;
  readonly providerMeta: ReadonlyRecord<string, JsonValue>;
}

// ─── ContentNode ──────────────────────────────────────────────────────────

/**
 * A parsed content document.  Wraps `document-ast`'s `DocumentNode`
 * with the surrounding metadata the rest of the pipeline cares about:
 * frontmatter, assigned route, asset references, source provenance.
 *
 * `route` is null until a collector assigns one.  Stages between the
 * parser and the collector should not depend on it.
 */
export interface ContentNode {
  readonly identity: LogicalId;
  readonly revision: RevisionId;
  readonly document: DocumentNode;
  readonly frontmatter: ReadonlyRecord<string, JsonValue>;
  readonly route: string | null;
  readonly assetRefs: readonly AssetRef[];
  readonly sourcePath: string;
}

// ─── Collection ───────────────────────────────────────────────────────────

/**
 * An ordering key for `CollectionEntry`.  Three forms cover the common
 * cases (lexicographic name, numeric position, RFC-3339 date) plus a
 * composite for tie-breaking.  Keeping the union closed lets sorting
 * logic be exhaustive and total.
 */
export type OrderKey =
  | { readonly kind: "lexicographic"; readonly value: string }
  | { readonly kind: "numeric"; readonly value: number }
  | { readonly kind: "date"; readonly value: string }
  | { readonly kind: "composite"; readonly value: readonly OrderKey[] };

/**
 * One entry in a Collection.  Stores a *reference* (LogicalId + revision)
 * rather than embedding the full ContentNode — collections of millions
 * of items must remain cheap to construct, hash, and diff.
 */
export interface CollectionEntry {
  readonly identity: LogicalId;
  readonly revision: RevisionId;
  readonly route: string | null;
  readonly orderKey: OrderKey;
  readonly overlay: ReadonlyRecord<string, JsonValue>;
}

/**
 * An ordered set of CollectionEntry references plus a grouping
 * discriminant.  Two collections with the same discriminant are
 * "the same kind of collection" and stages may operate across them
 * (e.g. a single pagination stage handling all `tag:*` collections).
 */
export interface Collection {
  readonly name: string;
  readonly entries: readonly CollectionEntry[];
  readonly discriminant: string;
  readonly meta: ReadonlyRecord<string, JsonValue>;
}

// ─── Asset ────────────────────────────────────────────────────────────────

/**
 * An image, video, font, or binary file plus the metadata derived
 * stages need.  `byteLength` duplicates `bytes.byteLength` so cheap
 * size checks don't have to touch the buffer.
 */
export interface Asset {
  readonly identity: LogicalId;
  readonly revision: RevisionId;
  readonly role: AssetRole;
  readonly mimeType: string;
  readonly bytes: Uint8Array;
  readonly byteLength: number;
  readonly dimensions: { readonly w: number; readonly h: number } | null;
  readonly durationMs: number | null;
  /** For derived assets (e.g. resized image), the original's id.  Null for originals. */
  readonly derivedFrom: LogicalId | null;
  readonly meta: ReadonlyRecord<string, JsonValue>;
}

// ─── Style & Interactivity stubs ──────────────────────────────────────────

/**
 * Minimal placeholder for the Style IR.  FM04 will replace this with
 * the full token / selector / rule model.  Until then, stages that
 * don't actually use style information can carry an empty StyleDocument
 * and downstream renderers will fall back to their built-in defaults.
 */
export interface StyleDocument {
  readonly tokens: ReadonlyRecord<string, JsonValue>;
  readonly rules: readonly JsonValue[];
  readonly theme: string | null;
}

/**
 * Empty StyleDocument constant — convenient for tests and default
 * pipelines that don't carry style information yet.
 */
export const EMPTY_STYLE: StyleDocument = {
  tokens: {},
  rules: [],
  theme: null,
};

/**
 * Minimal placeholder for the Interactivity IR.  FM05 will replace
 * with the full state / bindings / handlers model.  An empty
 * Interactivity is the default for static pages — those ship zero JS.
 */
export interface Interactivity {
  readonly state: readonly JsonValue[];
  readonly bindings: readonly JsonValue[];
  readonly handlers: readonly JsonValue[];
  readonly islands: readonly string[];
}

/**
 * Empty Interactivity constant — convenient for tests and the default
 * static-rendering path that has no client-side behaviour.
 */
export const EMPTY_INTERACTIVITY: Interactivity = {
  state: [],
  bindings: [],
  handlers: [],
  islands: [],
};

// ─── Document ─────────────────────────────────────────────────────────────

/**
 * The (content, style, interactivity) triple for one renderable unit.
 * The unit a renderer turns into a RenderedPage / PrintForme / etc.
 *
 * `route` is non-null here: by the time a Document exists the route
 * has been assigned (or the renderer wouldn't know where to put the
 * output).  This is the difference between a `ContentNode` (route may
 * be null) and a `Document` (route is set).
 */
export interface Document {
  readonly identity: LogicalId;
  readonly revision: RevisionId;
  readonly content: ContentNode;
  readonly style: StyleDocument;
  readonly interactivity: Interactivity;
  readonly route: string;
}

// ─── RenderedPage ─────────────────────────────────────────────────────────

/** Branded ID for a single style rule, before bundling. */
export type StyleRuleId = string & { readonly __brand: "StyleRuleId" };

/** Branded ID for an interactivity island, before bundling. */
export type IslandId = string & { readonly __brand: "IslandId" };

/**
 * Per-page metadata used by the renderer to populate <head> tags and
 * by feed/sitemap stages to know what to syndicate.
 */
export interface PageMeta {
  readonly title: string;
  readonly description: string | null;
  readonly canonicalUrl: string | null;
  readonly openGraph: ReadonlyRecord<string, string>;
  readonly structured: readonly JsonValue[];
  readonly extra: ReadonlyRecord<string, string>;
}

/**
 * The output of a web-backend renderer, before bundling and per-page
 * code-splitting.  The `usedStyle` and `usedIslands` arrays drive the
 * AOT compiler's "smallest artifact" decision (FM06).
 */
export interface RenderedPage {
  readonly route: string;
  readonly html: string;
  readonly usedStyle: readonly StyleRuleId[];
  readonly usedIslands: readonly IslandId[];
  readonly usedAssets: readonly LogicalId[];
  readonly meta: PageMeta;
  readonly source: LogicalId;
}

// ─── PrintForme ───────────────────────────────────────────────────────────

/**
 * A typed length with explicit unit.  Print backends translate to their
 * native unit (PDF uses points; LaTeX uses TeX `pt` which is *not* the
 * same as PostScript pt) so we never lose information by collapsing to
 * pixels.
 */
export type Length =
  | { readonly unit: "pt"; readonly value: number }
  | { readonly unit: "mm"; readonly value: number }
  | { readonly unit: "in"; readonly value: number };

export interface Margins {
  readonly top: Length;
  readonly right: Length;
  readonly bottom: Length;
  readonly left: Length;
}

/** Standardised page-size names that print backends agree on. */
export type PageSizeName =
  | "A4" | "A5" | "Letter" | "Legal" | "Tabloid" | "B5" | "B6";

export type PageSize =
  | { readonly kind: "named"; readonly name: PageSizeName }
  | { readonly kind: "custom"; readonly w: Length; readonly h: Length };

export interface PageSettings {
  readonly size: PageSize;
  readonly margins: Margins;
  readonly orientation: "portrait" | "landscape";
}

/**
 * One running header or footer on a print page.  The `content` is a
 * DocumentNode so it can carry rich content (page numbers via inline
 * raw nodes, an image of a logo, etc.).
 */
export interface RunningElement {
  readonly position:
    | "header-left" | "header-center" | "header-right"
    | "footer-left" | "footer-center" | "footer-right";
  readonly content: DocumentNode;
}

/**
 * Backend-neutral composed page destined for a print backend (LaTeX,
 * direct PDF, EPUB).  Layout decisions are deferred to the backend —
 * a PrintForme says "this content, this style, this page geometry"
 * and lets the backend handle line breaking, page breaking, kerning.
 */
export interface PrintForme {
  readonly source: LogicalId;
  readonly page: PageSettings;
  readonly runningElements: readonly RunningElement[];
  readonly content: ContentNode;
  readonly style: StyleDocument;
  readonly usedAssets: readonly LogicalId[];
}

// ─── RequestHandler ───────────────────────────────────────────────────────

/**
 * Runtime requirements for a per-request handler.  Emitters use this
 * to decide whether to bundle for Cloudflare Workers, Node, Deno, etc.
 */
export type RuntimeRequirement =
  | { readonly kind: "cloudflare-worker" }
  | { readonly kind: "node"; readonly minVersion: string }
  | { readonly kind: "deno"; readonly minVersion: string }
  | { readonly kind: "bun"; readonly minVersion: string };

/**
 * A dynamic, per-request handler emitted by `render-dynamic` backends.
 * The `code` is a serialised JS string that the emitter packages for
 * its target runtime; the orchestrator does not execute it.
 */
export interface RequestHandler {
  readonly routePattern: string;
  readonly code: string;
  readonly runtime: RuntimeRequirement;
  readonly staticAssets: readonly LogicalId[];
}

// ─── SearchIndex ──────────────────────────────────────────────────────────

/**
 * The output of a search indexer.  Indexers vary widely (Pagefind,
 * MiniSearch, SQLite FTS, semantic embeddings) so the payload is a
 * file-tree blob plus an indexer name; downstream stages discriminate
 * on `indexer` if they care.
 */
export interface SearchIndex {
  readonly indexer: string;
  readonly indexer_version: string;
  readonly files: ReadonlyRecord<string, Uint8Array>;
  readonly manifest: JsonValue;
}

// ─── Feed ─────────────────────────────────────────────────────────────────

export type FeedFormat = "rss" | "atom" | "jsonfeed" | "sitemap";

/**
 * A syndication feed (RSS, Atom, JSON Feed, or sitemap).  Carries the
 * already-serialised file bytes; consumers either upload them to a
 * destination or include them in a deploy artifact.
 */
export interface Feed {
  readonly format: FeedFormat;
  readonly files: ReadonlyRecord<string, Uint8Array>;
}

// ─── DeployArtifact ───────────────────────────────────────────────────────

export type DeployVariant =
  | { readonly kind: "dist-tree" }
  | { readonly kind: "worker-bundle"; readonly runtime: RuntimeRequirement }
  | { readonly kind: "email-bundle" }
  | { readonly kind: "epub-bundle" }
  | { readonly kind: "pdf"; readonly pageCount: number };

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

export interface DeployManifest {
  readonly routes: readonly DeployRoute[];
  readonly assets: readonly DeployAssetEntry[];
  readonly buildTime: string;
  readonly buildId: RevisionId;
}

/**
 * The final shippable thing.  Variant tags discriminate "this is a
 * static-files tree" from "this is a Worker bundle" from "this is an
 * email package" — emitters select on it to decide how to deliver.
 */
export interface DeployArtifact {
  readonly variant: DeployVariant;
  readonly files: ReadonlyRecord<string, Uint8Array>;
  readonly manifest: DeployManifest;
}

// ─── Stream value type ───────────────────────────────────────────────────

/**
 * The runtime value type for streams.  When a stage's `produces`
 * descriptor is a stream wrapper (built via `streamOf`), it returns
 * an AsyncIterable directly — the orchestrator does the bookkeeping.
 *
 * This `Stream` type exists for the rare case where a value-shaped
 * stream needs to be passed around as a discrete value (e.g. cached,
 * fanned out manually).  The common case is for stages to just return
 * `AsyncIterable<T>` from their `run` method.
 */
export interface Stream<T> {
  readonly iterator: () => AsyncIterable<T>;
}
