/**
 * @coding-adventures/correlation-vector
 *
 * Append-only provenance tracking for any data pipeline.
 *
 * What Is a Correlation Vector?
 * ==============================
 * A Correlation Vector (CV) is a lightweight, append-only record that follows
 * a piece of data through every transformation it undergoes. Assign a CV to
 * anything when it is born. Every system, stage, or function that touches it
 * appends its contribution. At any point you can ask:
 *
 *   "Where did this come from and what happened to it?"
 *
 * and get a complete, ordered answer.
 *
 * The concept originated in distributed systems tracing, where a request flows
 * through dozens of microservices and you need to reconstruct the entire
 * journey across all of them. This implementation generalizes the idea to any
 * pipeline: compiler passes, ETL transformations, build systems, ML
 * preprocessing — anywhere data flows through a sequence of transformations.
 *
 * A Concrete Analogy: The Hospital Wristband
 * ==========================================
 * Imagine a patient arriving at a hospital. They get a wristband with a unique
 * ID the moment they walk in. Every doctor, nurse, and technician who touches
 * them scans that wristband and records what they did. At the end, you have
 * a complete, ordered record of everything that happened to that patient.
 *
 * The CV is the wristband. The CVLog is the hospital's record system. Each
 * `contribute()` call is a caregiver scanning the wristband.
 *
 * Three Domain Examples
 * =====================
 *
 * Compiler:
 *   cv born at parse time  →  scope_analysis contributes "resolved"
 *                          →  variable_renamer contributes "renamed foo→a"
 *                          →  dce contributes "deleted: unreachable"
 *
 * ETL pipeline:
 *   cv born at ingestion   →  validator contributes "schema_checked"
 *                          →  normalizer contributes "date_format_converted"
 *                          →  enricher contributes "geo_lookup_appended"
 *
 * Build system:
 *   cv born at source file →  compiler contributes "compiled to .o"
 *                          →  linker contributes "linked into binary"
 *                          →  packager contributes "bundled into .tar.gz"
 *
 * Same library. Different tags. Full provenance in every case.
 *
 * ID Format
 * =========
 * Every entity gets a CV ID: a stable, dot-separated string:
 *
 *   a3f1b2c4.1        — first root CV with that base
 *   a3f1b2c4.2        — second root CV with same base
 *   a3f1b2c4.1.1      — first entity derived from a3f1b2c4.1
 *   a3f1b2c4.1.2      — second entity derived from a3f1b2c4.1
 *   a3f1b2c4.1.1.1    — entity derived from a3f1b2c4.1.1
 *   00000000.1        — synthetic entity with no natural origin
 *
 * The base segment is the first 8 hex characters of SHA-256(source + ":" + location).
 * For synthetic entities (no origin), the base is always "00000000".
 *
 * Reading the ancestry from the ID is immediate: count the dots. One dot means
 * a root. Two dots means a first-generation child. Each additional dot is one
 * more generation of derivation.
 *
 * The Enabled Flag
 * ================
 * The `enabled` flag is the tracing switch. When `false`, every mutating
 * operation (`contribute`, `delete`, `passthrough`) is a no-op. `create`,
 * `derive`, and `merge` still return valid CV IDs (the entity needs its ID
 * regardless of whether tracing is active), but nothing is written to the log.
 *
 * This means production code pays essentially zero overhead when tracing is
 * off, and full provenance when it is on.
 *
 * @module
 */

import { stringify } from "@coding-adventures/json-serializer";
import { sha256Hex as _sha256Hex } from "@coding-adventures/sha256";

export const VERSION = "0.1.0";

// ─── SHA-256 Helper ──────────────────────────────────────────────────────────
//
// We delegate SHA-256 computation to the repo's own @coding-adventures/sha256
// package (FIPS 180-4). That package's `sha256Hex` takes a Uint8Array, so this
// thin adapter encodes a UTF-8 string and calls through.
//
// We use only the first 8 hex characters (32 bits) of the 64-character digest
// as the CV ID base — enough uniqueness given the per-context counter that
// follows the base segment.
//
// @example
// ```ts
// sha256Hex("app.ts:5:12")
// // → "a3f1b2c4..." (64 hex chars — we take just the first 8)
// ```
function sha256Hex(data: string): string {
  const enc = new TextEncoder();
  return _sha256Hex(enc.encode(data));
}

// ─── Public Types ────────────────────────────────────────────────────────────

/**
 * Origin describes where and when an entity was born.
 *
 * Think of Origin as the "birth certificate" of a tracked entity. It answers:
 *   - WHO created it? (`source` — the file, system, or data source)
 *   - WHERE in that source? (`location` — line:col, byte offset, row ID, etc.)
 *   - WHEN was it created? (`timestamp` — ISO 8601, if time-relevant)
 *   - WHAT extra context? (`meta` — any additional domain-specific data)
 *
 * @example
 * ```ts
 * // A TypeScript AST node at line 5, column 12
 * const origin: Origin = {
 *   source: "app.ts",
 *   location: "5:12",
 *   timestamp: "2026-04-05T10:00:00Z",
 *   meta: { nodeKind: "Identifier" }
 * };
 *
 * // A database row
 * const rowOrigin: Origin = {
 *   source: "orders_table",
 *   location: "row_id:8472",
 *   meta: {}
 * };
 * ```
 */
export interface Origin {
  /** Identifies the origin system, file, or data source. */
  source: string;
  /** Position within the source (line:col, byte offset, row ID, etc.). */
  location: string;
  /** ISO 8601 timestamp, if time-relevant. Optional. */
  timestamp?: string;
  /** Any additional origin context the domain needs to record. */
  meta: Record<string, unknown>;
}

/**
 * Contribution records that a stage processed a tracked entity.
 *
 * Every time a pipeline stage "touches" an entity, it appends a Contribution.
 * The order of contributions is semantically meaningful — it is the sequence
 * in which stages processed the entity.
 *
 * The CV library imposes no constraints on `source` or `tag` values — those
 * are defined by the consumer domain. A compiler might use:
 *   source="scope_analysis", tag="resolved", meta={binding: "local:x"}
 * An ETL pipeline might use:
 *   source="date_normalizer", tag="converted", meta={from: "MM/DD", to: "ISO"}
 *
 * @example
 * ```ts
 * const contribution: Contribution = {
 *   source: "variable_renamer",
 *   tag: "renamed",
 *   meta: { from: "userPreferences", to: "a" }
 * };
 * ```
 */
export interface Contribution {
  /** Who/what contributed (stage name, service name, pass name). */
  source: string;
  /** What happened (domain-defined label for the action). */
  tag: string;
  /** Arbitrary key-value detail about what was done. */
  meta: Record<string, unknown>;
}

/**
 * DeletionRecord explains why an entity was intentionally removed.
 *
 * The CV entry remains in the log permanently after deletion — this is how
 * you can answer "why did this disappear?" long after the fact. The deletion
 * is a record, not a removal.
 *
 * @example
 * ```ts
 * const deletion: DeletionRecord = {
 *   source: "dead_code_eliminator",
 *   reason: "unreachable from entry point",
 *   meta: { entryPointCv: "a3f1b2c4.1" }
 * };
 * ```
 */
export interface DeletionRecord {
  /** Who performed the deletion. */
  source: string;
  /** Why the entity was deleted. */
  reason: string;
  /** Any additional context about the deletion. */
  meta: Record<string, unknown>;
}

/**
 * CVEntry is the full provenance record for a single tracked entity.
 *
 * Every entity in the pipeline has exactly one CVEntry in the CVLog:
 *   - `id`: its stable, never-changing identity string
 *   - `parentIds`: IDs of entities this was derived from (empty for roots)
 *   - `origin`: where/when it was born (null for synthetics)
 *   - `contributions`: ordered history of every stage that touched it
 *   - `deleted`: set if the entity was intentionally removed; null otherwise
 *
 * CVEntry is append-only: contributions are only ever added, never removed.
 * Even deleted entries remain in the log forever.
 */
export interface CVEntry {
  /** Stable, globally unique identity string (e.g., "a3f1b2c4.3"). */
  id: string;
  /** IDs of parent entities. Empty for roots; one or more for derived/merged. */
  parentIds: string[];
  /** Where/when this entity was born. Null for synthetics. */
  origin: Origin | null;
  /** Append-only ordered history of every stage that touched this entity. */
  contributions: Contribution[];
  /** Non-null if this entity was intentionally deleted. */
  deleted: DeletionRecord | null;
}

// ─── CVLog Class ─────────────────────────────────────────────────────────────

/**
 * CVLog is the provenance store for an entire pipeline run.
 *
 * It is the map that holds all CV entries and accumulates the history of
 * every entity that flows through the pipeline. Typically you create one
 * CVLog at the start of a pipeline run and pass it through every stage.
 *
 * Each stage reads the log (via `get`, `history`, `ancestors`, etc.) and
 * writes to it (via `contribute`, `derive`, `delete`, `passthrough`).
 *
 * Internal State
 * ==============
 * Beyond the public `entries` map, CVLog maintains two private counters:
 *
 * 1. `_baseCounters`: Map<base, number>
 *    Counts how many root CVs have been created for each 8-char hex base.
 *    When you call `create({ source: "app.ts", location: "5:12" })`, the
 *    base is sha256("app.ts:5:12").slice(0,8). The counter for that base
 *    starts at 1 and increments with each new root using the same base.
 *
 * 2. `_childCounters`: Map<parentId, number>
 *    Counts how many children have been derived from each parent CV ID.
 *    `derive("a3f1.1")` produces "a3f1.1.1", "a3f1.1.2", etc.
 *
 * These counters ensure IDs are unique even when the same origin is used
 * multiple times, and they live on the CVLog instance (not global state).
 *
 * @example
 * ```ts
 * const log = new CVLog();
 * const cvId = log.create({ source: "app.ts", location: "1:0", meta: {} });
 * log.contribute(cvId, "parser", "tokenized", { count: 42 });
 * log.contribute(cvId, "scope_analysis", "resolved", { binding: "local:x" });
 * console.log(log.history(cvId));
 * // [{ source: "parser", tag: "tokenized", ... }, { source: "scope_analysis", ... }]
 * ```
 */
export class CVLog {
  /**
   * The central map of CV ID → CVEntry.
   *
   * This is the heart of the log. Every entity that flows through the pipeline
   * has an entry here (when enabled). Entries are keyed by their CV ID string.
   */
  readonly entries: Map<string, CVEntry>;

  /**
   * Ordered list of unique source names that have contributed to any CV.
   *
   * This is the "pass order" — the sequence of stages that have touched at
   * least one entity. A source name is added the first time it contributes
   * to any CV in this log. Used for serialization and debugging.
   *
   * Example: ["parser", "scope_analysis", "variable_renamer", "dce"]
   */
  readonly passOrder: string[];

  /**
   * Whether provenance tracking is active.
   *
   * When `false`, mutating operations are no-ops. `create`, `derive`, and
   * `merge` still return valid CV IDs (entities need their ID whether or not
   * tracing is on), but nothing is written to `entries` or `passOrder`.
   *
   * This lets production code run at full speed without changing any
   * business logic — just construct CVLog with `enabled = false`.
   */
  readonly enabled: boolean;

  // Per-base counters: how many roots have been created for each 8-char base.
  // Example: { "a3f1b2c4": 3 } means three roots born from the same origin hash.
  private _baseCounters: Map<string, number>;

  // Per-parent counters: how many children have been derived from each parent.
  // Example: { "a3f1b2c4.1": 2 } means two children of "a3f1b2c4.1" exist.
  private _childCounters: Map<string, number>;

  /**
   * Construct a new, empty CVLog.
   *
   * @param enabled - When false, all write operations are no-ops but IDs are
   *   still generated. Defaults to true (full tracing).
   *
   * @example
   * ```ts
   * const log = new CVLog();           // full tracing
   * const silent = new CVLog(false);   // ID generation only, no storage
   * ```
   */
  constructor(enabled = true) {
    this.entries = new Map();
    this.passOrder = [];
    this.enabled = enabled;
    this._baseCounters = new Map();
    this._childCounters = new Map();
  }

  // ─── ID Generation ─────────────────────────────────────────────────────────

  /**
   * Compute the 8-character hex base for a given origin.
   *
   * The base is derived by:
   *   1. Concatenating source + ":" + location.
   *   2. Computing SHA-256 of that string.
   *   3. Taking the first 8 characters of the hex digest.
   *
   * For synthetic entities (no origin), the base is always "00000000".
   *
   * This means two entities born at the same source:location will share a
   * base — they're distinguished by the sequence number (the .N suffix).
   *
   * @example
   * ```ts
   * // Same source+location → same base → different sequence numbers
   * log.create({ source: "app.ts", location: "5:12", meta: {} }); // → "a3f1b2c4.1"
   * log.create({ source: "app.ts", location: "5:12", meta: {} }); // → "a3f1b2c4.2"
   * ```
   */
  private _computeBase(origin: Origin | undefined): string {
    if (!origin) return "00000000";
    const key = origin.source + ":" + origin.location;
    return sha256Hex(key).slice(0, 8);
  }

  /**
   * Allocate the next root ID for the given base.
   *
   * Looks up (or initializes) the counter for `base`, increments it, and
   * returns "base.N".
   *
   * @example
   * ```ts
   * _nextRootId("a3f1b2c4")  // first call → "a3f1b2c4.1"
   * _nextRootId("a3f1b2c4")  // second call → "a3f1b2c4.2"
   * ```
   */
  private _nextRootId(base: string): string {
    const n = (this._baseCounters.get(base) ?? 0) + 1;
    this._baseCounters.set(base, n);
    return `${base}.${n}`;
  }

  /**
   * Allocate the next child ID for the given parent CV ID.
   *
   * The child ID is `parentCvId + "." + M` where M is the next child
   * sequence for that parent.
   *
   * @example
   * ```ts
   * _nextChildId("a3f1b2c4.1")  // first call → "a3f1b2c4.1.1"
   * _nextChildId("a3f1b2c4.1")  // second call → "a3f1b2c4.1.2"
   * ```
   */
  private _nextChildId(parentCvId: string): string {
    const m = (this._childCounters.get(parentCvId) ?? 0) + 1;
    this._childCounters.set(parentCvId, m);
    return `${parentCvId}.${m}`;
  }

  // ─── Core Mutation Operations ───────────────────────────────────────────────

  /**
   * Born a new root CV. The entity has no parents — it was created from
   * nothing or from an external source.
   *
   * The CV ID is deterministic: `base.N` where base = first 8 chars of
   * SHA-256(origin.source + ":" + origin.location), and N is the next
   * sequence number for entities born at that same origin.
   *
   * When `enabled = false`, a valid CV ID is still computed and returned
   * (the entity needs its ID regardless), but no entry is written to the log.
   *
   * @param origin - Where/when this entity was born. Omit for synthetic entities.
   * @returns The newly created CV ID string.
   *
   * @example
   * ```ts
   * // Parsing a source file — each AST node gets a root CV
   * const cvId = log.create({ source: "app.ts", location: "5:12", meta: {} });
   * // → "a3f1b2c4.1" (or similar 8-char hex base)
   *
   * // Ingesting a database row
   * const rowId = log.create({ source: "orders", location: "row:8472", meta: {} });
   * ```
   */
  create(origin?: Origin): string {
    const base = this._computeBase(origin);
    const id = this._nextRootId(base);

    if (this.enabled) {
      const entry: CVEntry = {
        id,
        parentIds: [],
        origin: origin ?? null,
        contributions: [],
        deleted: null,
      };
      this.entries.set(id, entry);
    }

    return id;
  }

  /**
   * Record that a stage processed this entity.
   *
   * Contributions are appended in call order — the order is semantically
   * meaningful: it reflects the sequence in which stages processed the entity.
   *
   * Throws if the entity has already been deleted, because contributing to
   * a deleted entity is a programming error — it means the caller is still
   * processing something that was supposed to be gone.
   *
   * When `enabled = false`, this is a no-op (returns immediately).
   *
   * @param cvId - The CV ID to contribute to.
   * @param source - Who/what contributed (e.g., "scope_analysis", "gcc").
   * @param tag - What happened (e.g., "resolved", "compiled", "failed").
   * @param meta - Arbitrary domain-specific detail. Defaults to {}.
   * @throws Error if cvId refers to a deleted entity.
   *
   * @example
   * ```ts
   * log.contribute(cvId, "scope_analysis", "resolved", { binding: "local:x" });
   * log.contribute(cvId, "variable_renamer", "renamed", { from: "x", to: "a" });
   * ```
   */
  contribute(
    cvId: string,
    source: string,
    tag: string,
    meta: Record<string, unknown> = {},
  ): void {
    if (!this.enabled) return;

    const entry = this.entries.get(cvId);
    if (!entry) return; // unknown ID — silently ignore when enabled

    if (entry.deleted !== null) {
      throw new Error(
        `Cannot contribute to deleted CV "${cvId}": it was deleted by "${entry.deleted.source}" (reason: "${entry.deleted.reason}"). ` +
        `Contributing to a deleted entity is a programming error — check your pipeline logic.`,
      );
    }

    entry.contributions.push({ source, tag, meta });

    // Track pass order: record each source name the first time it contributes
    // to any CV in this log. This gives a global view of which stages ran.
    if (!this.passOrder.includes(source)) {
      (this.passOrder as string[]).push(source);
    }
  }

  /**
   * Create a new CV that is descended from an existing one.
   *
   * Use this when one entity is split into multiple outputs, or when a
   * transformation produces a new entity that is conceptually "the same
   * thing" expressed differently.
   *
   * The derived CV's ID is `parentCvId + "." + M` where M increments each
   * time you derive from the same parent.
   *
   * Reading the ID tells you the ancestry immediately:
   *   "a3f1b2c4.1.2" is the second entity derived from "a3f1b2c4.1"
   *
   * @param parentCvId - The CV ID of the entity being derived from.
   * @param origin - Optional origin for the new entity.
   * @returns The new child CV ID.
   *
   * @example
   * ```ts
   * // Destructuring {a, b} = x into two separate bindings
   * const cvA = log.derive(originalCvId);  // → "a3f1b2c4.1.1"
   * const cvB = log.derive(originalCvId);  // → "a3f1b2c4.1.2"
   * ```
   */
  derive(parentCvId: string, origin?: Origin): string {
    const id = this._nextChildId(parentCvId);

    if (this.enabled) {
      const entry: CVEntry = {
        id,
        parentIds: [parentCvId],
        origin: origin ?? null,
        contributions: [],
        deleted: null,
      };
      this.entries.set(id, entry);
    }

    return id;
  }

  /**
   * Create a new CV descended from multiple existing CVs.
   *
   * Use this when multiple entities are combined into one output — for
   * example, function inlining (call site + function body → merged expression)
   * or a database JOIN (orders row + customers row → result row).
   *
   * The merged CV's `parentIds` lists all parents. Its ID uses the
   * "00000000" base (synthetic, since it has multiple parents with no single
   * natural origin), unless an `origin` is provided.
   *
   * @param parentCvIds - IDs of all entities being merged.
   * @param origin - Optional origin for the merged entity.
   * @returns The new merged CV ID.
   *
   * @example
   * ```ts
   * // Inlining: call site + function body → merged expression
   * const mergedId = log.merge([callSiteCv, functionBodyCv]);
   *
   * // JOIN: orders row + customers row → result row
   * const rowId = log.merge([ordersCv, customersCv], {
   *   source: "join_stage",
   *   location: "orders.customer_id=customers.id",
   *   meta: {}
   * });
   * ```
   */
  merge(parentCvIds: string[], origin?: Origin): string {
    // The merged CV has no single natural origin, so we use "00000000" as the
    // base — unless an explicit origin was provided (in which case we use its hash).
    const base = this._computeBase(origin);
    const id = this._nextRootId(base);

    if (this.enabled) {
      const entry: CVEntry = {
        id,
        parentIds: [...parentCvIds],
        origin: origin ?? null,
        contributions: [],
        deleted: null,
      };
      this.entries.set(id, entry);
    }

    return id;
  }

  /**
   * Record that an entity was intentionally removed.
   *
   * The CV entry remains in the log permanently — this is how you can answer
   * "why did this disappear?" long after the fact. Think of it as stamping
   * "DELETED" on the record rather than shredding it.
   *
   * After deletion, calling `contribute` on this CV throws an error.
   * Calling `derive` or `merge` with a deleted CV as a parent is still
   * allowed (e.g., deriving a tombstone record from a deleted entity).
   *
   * When `enabled = false`, this is a no-op.
   *
   * @param cvId - The CV ID of the entity to delete.
   * @param source - Who performed the deletion.
   * @param reason - Why the entity was deleted.
   * @param meta - Additional context about the deletion. Defaults to {}.
   *
   * @example
   * ```ts
   * log.delete(cvId, "dead_code_eliminator",
   *   "unreachable from entry point",
   *   { entryPointCv: mainCvId }
   * );
   * ```
   */
  delete(
    cvId: string,
    source: string,
    reason: string,
    meta: Record<string, unknown> = {},
  ): void {
    if (!this.enabled) return;

    const entry = this.entries.get(cvId);
    if (!entry) return;

    entry.deleted = { source, reason, meta };

    // Track pass order for the deleting stage, same as contributions.
    if (!this.passOrder.includes(source)) {
      (this.passOrder as string[]).push(source);
    }
  }

  /**
   * Record that a stage examined this entity but made no changes.
   *
   * This is the "identity contribution" — it records that a stage passed
   * through the entity without transforming it. This matters for pipeline
   * auditing: knowing which stages an entity visited, even when nothing
   * was changed, lets you reconstruct the full pipeline path.
   *
   * Example: a type-checker examines every node. For nodes it considers
   * well-typed, it records a passthrough. For nodes with errors, it records
   * a contribution with tag "type_error". The log then shows every node's
   * complete journey through the type-checker pass.
   *
   * In performance-sensitive pipelines, `passthrough` may be omitted for
   * known-clean stages to reduce log size. The tradeoff: that stage becomes
   * invisible in the history for unaffected entities.
   *
   * When `enabled = false`, this is a no-op.
   *
   * @param cvId - The CV ID of the entity that passed through.
   * @param source - The stage name.
   *
   * @example
   * ```ts
   * log.passthrough(cvId, "type_checker");
   * log.passthrough(cvId, "linter");
   * ```
   */
  passthrough(cvId: string, source: string): void {
    if (!this.enabled) return;

    const entry = this.entries.get(cvId);
    if (!entry) return;

    entry.contributions.push({ source, tag: "passthrough", meta: {} });

    if (!this.passOrder.includes(source)) {
      (this.passOrder as string[]).push(source);
    }
  }

  // ─── Query Operations ────────────────────────────────────────────────────────

  /**
   * Return the full CVEntry for a CV ID, or undefined if not found.
   *
   * Returns undefined when the log is disabled (nothing was stored) or when
   * the CV ID was never created in this log.
   *
   * @example
   * ```ts
   * const entry = log.get(cvId);
   * if (entry) {
   *   console.log(entry.contributions.length); // how many stages touched it
   * }
   * ```
   */
  get(cvId: string): CVEntry | undefined {
    if (!this.enabled) return undefined;
    return this.entries.get(cvId);
  }

  /**
   * Return all ancestor CV IDs, ordered from nearest parent to most distant.
   *
   * Walks the `parentIds` chain recursively. A CV can have multiple parents
   * (when created via `merge`), so this performs a breadth-first traversal
   * of the parent graph.
   *
   * Returns [] when the log is disabled, or when the CV ID has no parents
   * (root entities). Cycles are impossible by construction (a CV is always
   * created AFTER its parents), so no cycle detection is needed.
   *
   * @example
   * ```ts
   * // A → B → C → D chain
   * const ancestors = log.ancestors(cvIdD);
   * // → [cvIdC, cvIdB, cvIdA]  (nearest first)
   * ```
   */
  ancestors(cvId: string): string[] {
    if (!this.enabled) return [];

    const result: string[] = [];
    const visited = new Set<string>();

    // BFS from cvId's parents upward. We process level by level so that
    // parents appear before grandparents in the output (nearest-first).
    let currentLevel = this.entries.get(cvId)?.parentIds ?? [];

    while (currentLevel.length > 0) {
      const nextLevel: string[] = [];
      for (const parentId of currentLevel) {
        if (visited.has(parentId)) continue;
        visited.add(parentId);
        result.push(parentId);
        const parentEntry = this.entries.get(parentId);
        if (parentEntry) {
          nextLevel.push(...parentEntry.parentIds);
        }
      }
      currentLevel = nextLevel;
    }

    return result;
  }

  /**
   * Return all CV IDs that have this CV in their ancestor chain.
   *
   * This is the inverse of `ancestors`. It scans the entire log to find
   * every entity whose parentIds (directly or transitively) include cvId.
   *
   * Returns [] when the log is disabled or when no descendants exist.
   *
   * @example
   * ```ts
   * // Parent P with two children C1, C2
   * log.descendants(parentCvId);
   * // → [childCvId1, childCvId2]  (or their descendants too, recursively)
   * ```
   */
  descendants(cvId: string): string[] {
    if (!this.enabled) return [];

    const result: string[] = [];
    const visited = new Set<string>();

    // Build a parent→children index for efficient lookup. This avoids
    // scanning the entire log on every recursive step.
    const childrenOf = new Map<string, string[]>();
    for (const [id, entry] of this.entries) {
      for (const pid of entry.parentIds) {
        const children = childrenOf.get(pid) ?? [];
        children.push(id);
        childrenOf.set(pid, children);
      }
    }

    // BFS downward from cvId.
    const queue = [cvId];
    while (queue.length > 0) {
      const current = queue.shift()!;
      const children = childrenOf.get(current) ?? [];
      for (const childId of children) {
        if (!visited.has(childId)) {
          visited.add(childId);
          result.push(childId);
          queue.push(childId);
        }
      }
    }

    return result;
  }

  /**
   * Return the contributions for a CV in order.
   *
   * This is the "what happened to it?" query. Returns all Contribution records
   * in the order they were appended. Note: the deletion record (if any) is
   * NOT included — use `get(cvId).deleted` to check for deletion.
   *
   * Returns [] when the log is disabled, or when the CV ID has no contributions.
   *
   * @example
   * ```ts
   * log.history(cvId);
   * // → [
   * //   { source: "parser", tag: "created", meta: {} },
   * //   { source: "scope_analysis", tag: "resolved", meta: { binding: "local:x" } },
   * //   { source: "variable_renamer", tag: "renamed", meta: { from: "x", to: "a" } }
   * // ]
   * ```
   */
  history(cvId: string): Contribution[] {
    if (!this.enabled) return [];
    const entry = this.entries.get(cvId);
    if (!entry) return [];
    return [...entry.contributions];
  }

  /**
   * Return the full CV entries for the entity and all its ancestors, ordered
   * from oldest ancestor to the entity itself.
   *
   * This is the "complete provenance chain" query. For an entity D that is
   * derived from C → B → A, lineage(D) returns [A, B, C, D].
   *
   * Useful for displaying the full history of an entity from its origin,
   * through every transformation, to its current state.
   *
   * Returns [] when the log is disabled, or when the CV ID is not found.
   *
   * @example
   * ```ts
   * // Chain: A → B → C → D
   * const chain = log.lineage(cvIdD);
   * // → [entryA, entryB, entryC, entryD]
   * ```
   */
  lineage(cvId: string): CVEntry[] {
    if (!this.enabled) return [];

    const selfEntry = this.entries.get(cvId);
    if (!selfEntry) return [];

    // Get ancestors (nearest-first), then reverse to oldest-first.
    const ancestorIds = this.ancestors(cvId);
    const oldestFirst = [...ancestorIds].reverse();

    const result: CVEntry[] = [];
    for (const aid of oldestFirst) {
      const e = this.entries.get(aid);
      if (e) result.push(e);
    }
    result.push(selfEntry);

    return result;
  }

  // ─── Serialization ───────────────────────────────────────────────────────────

  /**
   * Serialize the CVLog to a plain JavaScript object.
   *
   * The shape matches the canonical JSON schema in the spec:
   * ```json
   * {
   *   "entries": {
   *     "a3f1b2c4.1": { "id": "...", "parent_ids": [], ... },
   *     ...
   *   },
   *   "pass_order": ["parser", "scope_analysis"],
   *   "enabled": true
   * }
   * ```
   *
   * Note that the serialized format uses snake_case keys (`parent_ids`,
   * `pass_order`) to match the spec and enable interoperability with other
   * language implementations.
   *
   * Use `toJsonString()` to get an actual JSON string for storage or
   * transmission. Use `serialize()` when you want to embed the log object
   * inside a larger structure.
   *
   * @returns A plain JS object suitable for JSON.stringify or embedding.
   *
   * @example
   * ```ts
   * const obj = log.serialize();
   * // { entries: { ... }, pass_order: [...], enabled: true }
   * ```
   */
  serialize(): object {
    const entriesObj: Record<string, unknown> = {};
    for (const [id, entry] of this.entries) {
      entriesObj[id] = {
        id: entry.id,
        parent_ids: entry.parentIds,
        origin: entry.origin
          ? {
              source: entry.origin.source,
              location: entry.origin.location,
              timestamp: entry.origin.timestamp ?? null,
              meta: entry.origin.meta,
            }
          : null,
        contributions: entry.contributions.map((c) => ({
          source: c.source,
          tag: c.tag,
          meta: c.meta,
        })),
        deleted: entry.deleted
          ? {
              source: entry.deleted.source,
              reason: entry.deleted.reason,
              meta: entry.deleted.meta,
            }
          : null,
      };
    }

    return {
      entries: entriesObj,
      pass_order: [...this.passOrder],
      enabled: this.enabled,
    };
  }

  /**
   * Serialize the CVLog to a JSON string.
   *
   * Uses `stringify()` from `@coding-adventures/json-serializer` for compact
   * JSON output — the same serializer used throughout this monorepo.
   *
   * @returns Compact JSON string representation of this CVLog.
   *
   * @example
   * ```ts
   * const json = log.toJsonString();
   * // '{"entries":{"a3f1b2c4.1":{"id":"a3f1b2c4.1",...}},"pass_order":[...],...}'
   *
   * // To reconstruct:
   * const restored = CVLog.fromJsonString(json);
   * ```
   */
  toJsonString(): string {
    return stringify(this.serialize());
  }

  /**
   * Reconstruct a CVLog from its plain object representation.
   *
   * The inverse of `serialize()`. Restores all entries, pass order, enabled
   * flag, and internal counters (so subsequent creates/derives produce non-
   * colliding IDs).
   *
   * @param data - The plain object from `serialize()`.
   * @returns A fully reconstructed CVLog instance.
   *
   * @example
   * ```ts
   * const obj = log.serialize();
   * const restored = CVLog.deserialize(obj);
   * // restored is identical to log
   * ```
   */
  static deserialize(data: object): CVLog {
    const raw = data as {
      entries: Record<string, {
        id: string;
        parent_ids: string[];
        origin: {
          source: string;
          location: string;
          timestamp: string | null;
          meta: Record<string, unknown>;
        } | null;
        contributions: Array<{
          source: string;
          tag: string;
          meta: Record<string, unknown>;
        }>;
        deleted: {
          source: string;
          reason: string;
          meta: Record<string, unknown>;
        } | null;
      }>;
      pass_order: string[];
      enabled: boolean;
    };

    const log = new CVLog(raw.enabled);

    // Restore entries.
    for (const [id, rawEntry] of Object.entries(raw.entries)) {
      const entry: CVEntry = {
        id: rawEntry.id,
        parentIds: rawEntry.parent_ids,
        origin: rawEntry.origin
          ? {
              source: rawEntry.origin.source,
              location: rawEntry.origin.location,
              timestamp: rawEntry.origin.timestamp ?? undefined,
              meta: rawEntry.origin.meta,
            }
          : null,
        contributions: rawEntry.contributions.map((c) => ({
          source: c.source,
          tag: c.tag,
          meta: c.meta,
        })),
        deleted: rawEntry.deleted
          ? {
              source: rawEntry.deleted.source,
              reason: rawEntry.deleted.reason,
              meta: rawEntry.deleted.meta,
            }
          : null,
      };
      log.entries.set(id, entry);

      // Reconstruct the base and child counters so future create/derive calls
      // produce IDs that don't collide with the restored ones.
      //
      // ID format: "base.N" for roots, "parent.M" for children.
      // We scan all IDs and update counters to track the maximum N and M seen.
      _reconstructCounters(log, id);
    }

    // Restore pass order.
    (log.passOrder as string[]).push(...raw.pass_order);

    return log;
  }

  /**
   * Reconstruct a CVLog from a JSON string.
   *
   * The inverse of `toJsonString()`.
   *
   * @param json - JSON string produced by `toJsonString()`.
   * @returns A fully reconstructed CVLog instance.
   *
   * @example
   * ```ts
   * const json = log.toJsonString();
   * const restored = CVLog.fromJsonString(json);
   * ```
   */
  static fromJsonString(json: string): CVLog {
    return CVLog.deserialize(JSON.parse(json) as object);
  }
}

// ─── Counter Reconstruction Helper ──────────────────────────────────────────
//
// When deserializing a CVLog, we need to reconstruct the base and child
// counters so subsequent creates/derives don't produce colliding IDs.
//
// Given an ID like "a3f1b2c4.3.2", we parse:
//   - "a3f1b2c4" is the base
//   - "3" is the N for the root "a3f1b2c4.3" → _baseCounters["a3f1b2c4"] = max(current, 3)
//   - "2" is the M for the child "a3f1b2c4.3" → _childCounters["a3f1b2c4.3"] = max(current, 2)
//
// We walk from the full ID backward, updating counters for each level.
//
function _reconstructCounters(log: CVLog, id: string): void {
  const parts = id.split(".");

  if (parts.length < 2) return; // malformed — skip

  // The last part is the sequence number (N or M). The rest is the parent key.
  // For a root "a3f1b2c4.3": base = "a3f1b2c4", N = 3
  // For a child "a3f1b2c4.3.2": parent = "a3f1b2c4.3", M = 2
  // For deeper "a3f1b2c4.3.2.1": parent = "a3f1b2c4.3.2", M = 1

  const seq = parseInt(parts[parts.length - 1], 10);
  if (isNaN(seq)) return;

  if (parts.length === 2) {
    // Root: update the base counter.
    const base = parts[0];
    // Access private fields via type assertion for reconstruction.
    const counters = (log as unknown as { _baseCounters: Map<string, number> })._baseCounters;
    const current = counters.get(base) ?? 0;
    if (seq > current) counters.set(base, seq);
  } else {
    // Child: update the child counter for its parent ID.
    const parentId = parts.slice(0, parts.length - 1).join(".");
    const counters = (log as unknown as { _childCounters: Map<string, number> })._childCounters;
    const current = counters.get(parentId) ?? 0;
    if (seq > current) counters.set(parentId, seq);
  }
}
