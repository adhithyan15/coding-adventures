/**
 * Kinds — the runtime-visible type system of the Forme pipeline.
 *
 * A "Kind" is the discriminator for a unit of data flowing through a
 * pipeline.  Every edge between two stages is typed by a Kind, and every
 * stage declares the Kinds it consumes and produces.  See FM01 §2 for
 * the full design.
 *
 * Kinds exist in two parallel layers:
 *
 *   1. At compile time — as the TypeScript interfaces declared in
 *      `shapes.ts`.  The compiler enforces that wired stages have
 *      compatible I/O when stages are composed in pure TypeScript.
 *
 *   2. At runtime — as the `KindDescriptor` value declared in this file.
 *      The orchestrator reads descriptors when building a DAG from
 *      configuration the compiler never sees (TOML, plugin manifests).
 *
 * Two layers exist because TypeScript types are erased at runtime.
 *
 * === Stream descriptors — note on spec divergence ===
 *
 * FM01 §3.6 sketches stream descriptors with the syntax
 * `{ ...Kinds.X, kind: "Stream" }`, which adds an unrelated `kind`
 * field to a `KindDescriptor`.  We instead use the `Stream` KindName
 * with an `inner` field on the descriptor itself, produced by the
 * `streamOf()` helper:
 *
 *   streamOf(Kinds.ContentSource)
 *     === { name: "Stream", version: "1.0", inner: Kinds.ContentSource }
 *
 * This keeps `KindDescriptor` a single closed shape and makes "is this a
 * stream?" a clean `descriptor.name === "Stream"` check.  The Stream
 * *value* type (the AsyncIterable wrapper passed at runtime) is unchanged.
 */

import type { JsonValue, ReadonlyRecord } from "./utility.js";

// ─── API version ──────────────────────────────────────────────────────────

/**
 * The version of the kernel API surface.  Stages declare the version they
 * target; the host refuses to load stages targeting a different version.
 *
 * Bump this only on source-breaking changes to the kernel contracts.
 * Adding optional fields, growing a union in a backward-compatible way,
 * adding a new Kind that defaults gracefully — none of these bump the
 * API version.
 *
 * Initial value: 1.
 */
export const KERNEL_API_VERSION = 1 as const;

export type KernelApiVersion = typeof KERNEL_API_VERSION;

// ─── Kind names ───────────────────────────────────────────────────────────

/**
 * The closed set of built-in Kind names.  Plugins extend this through
 * `ext:<name>` strings (FM01 §2.5); kernel-level kinds cannot be
 * overridden.
 *
 * `Void` is the special name for "no payload" — used as the input of
 * source stages and the output of sink stages.
 *
 * `Stream` is the meta-kind that wraps another kind to represent a lazy
 * stream of values; see `streamOf`.
 */
export const KINDS = Object.freeze([
  "Void",
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
] as const);

/** A built-in Kind name. */
export type BuiltinKindName = (typeof KINDS)[number];

/**
 * A Kind name — either built-in or a plugin-contributed `ext:` name.
 * Plugin-contributed names live in their own namespace and are
 * registered with the host at manifest-load time.
 */
export type KindName = BuiltinKindName | `ext:${string}`;

// ─── Kind descriptors ─────────────────────────────────────────────────────

/**
 * The runtime type tag for a kind.  Carried by every stage's `consumes`
 * and `produces` declaration; the orchestrator reads these to verify
 * compatibility before the pipeline starts.
 *
 * Compatibility rules — see FM01 §2.6 and the `forme-orchestrator`
 * implementation:
 *
 *   1. Name match or registered subtype.
 *   2. Major-version match; minor versions are forward-compatible.
 *   3. Discriminant equality if both sides declare one.
 *   4. Constraint satisfaction (best-effort; unknown keys warn).
 */
export interface KindDescriptor {
  /** The Kind name.  Built-in or `ext:`-prefixed. */
  readonly name: KindName;

  /** Semver-compatible version of the kind's shape. */
  readonly version: string;

  /**
   * Optional polymorphism discriminant.  When two stages both declare a
   * discriminant, they must be equal for the edge to type-check.  Used
   * by polymorphic kinds like `Feed` (rss vs atom vs sitemap) when a
   * downstream stage cares about the variant.
   */
  readonly discriminant?: string;

  /**
   * Open-ended constraint vocabulary.  A producer documents what it
   * guarantees ("mimeType starts with text/markdown"); a consumer
   * documents what it requires.  The orchestrator does best-effort
   * structural matching and warns rather than errors on unknown keys —
   * stages can declare custom constraints without teaching the
   * orchestrator about them.
   */
  readonly constraints?: ReadonlyRecord<string, JsonValue>;

  /**
   * For descriptors with `name === "Stream"`, the inner kind being
   * streamed.  Absent for non-stream descriptors.
   */
  readonly inner?: KindDescriptor;
}

// ─── Canonical descriptors ────────────────────────────────────────────────

/**
 * The canonical descriptor for each built-in kind.  Stages reference
 * these directly:
 *
 *   defineStage({
 *     consumes: Kinds.ContentSource,
 *     produces: Kinds.ContentNode,
 *     ...
 *   });
 *
 * Initial version is "1.0" for every kernel kind.  When a kind's shape
 * grows, bump the minor; when it breaks compatibility, bump the major
 * AND the kernel API version.
 */
export const Kinds = Object.freeze({
  Void:           Object.freeze({ name: "Void",           version: "1.0" }),
  ContentSource:  Object.freeze({ name: "ContentSource",  version: "1.0" }),
  ContentNode:    Object.freeze({ name: "ContentNode",    version: "1.0" }),
  Collection:     Object.freeze({ name: "Collection",     version: "1.0" }),
  Asset:          Object.freeze({ name: "Asset",          version: "1.0" }),
  Document:       Object.freeze({ name: "Document",       version: "1.0" }),
  RenderedPage:   Object.freeze({ name: "RenderedPage",   version: "1.0" }),
  PrintForme:     Object.freeze({ name: "PrintForme",     version: "1.0" }),
  RequestHandler: Object.freeze({ name: "RequestHandler", version: "1.0" }),
  SearchIndex:    Object.freeze({ name: "SearchIndex",    version: "1.0" }),
  Feed:           Object.freeze({ name: "Feed",           version: "1.0" }),
  DeployArtifact: Object.freeze({ name: "DeployArtifact", version: "1.0" }),
} as const) satisfies Record<Exclude<BuiltinKindName, "Stream">, KindDescriptor>;

// ─── Stream helpers ───────────────────────────────────────────────────────

/**
 * Produce a `Stream<K>` descriptor wrapping the given inner descriptor.
 *
 * Example:
 *
 *   const sourceProduces = streamOf(Kinds.ContentSource);
 *   //  → { name: "Stream", version: "1.0", inner: Kinds.ContentSource }
 *
 * Equivalent to writing the descriptor by hand; the helper exists so
 * stage definitions stay short and readable.
 */
export function streamOf(inner: KindDescriptor): KindDescriptor {
  return { name: "Stream", version: "1.0", inner };
}

/**
 * Predicate: is this descriptor a stream wrapper?  When true,
 * `descriptor.inner` is set to the wrapped descriptor.
 */
export function isStreamDescriptor(d: KindDescriptor): boolean {
  return d.name === "Stream";
}
