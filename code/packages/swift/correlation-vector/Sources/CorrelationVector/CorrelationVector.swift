// CorrelationVector.swift
// ============================================================================
// Correlation Vector — append-only provenance tracking for any pipeline
// ============================================================================
//
// A Correlation Vector (CV) is a lightweight, append-only provenance record
// that follows a piece of data through every transformation it undergoes.
// Assign a CV to anything when it is born; every stage appends its contribution.
//
// The concept originates from distributed systems tracing, where a request
// flows through dozens of microservices and you need to reconstruct what
// happened across all of them. This implementation generalises the idea to
// ANY pipeline: compiler passes, ETL, document transformations, build systems,
// ML preprocessing, or anywhere data flows through a sequence of stages.
//
// The library is intentionally domain-agnostic. It knows nothing about
// compilers, JavaScript, or IR nodes. Consumers attach semantic meaning
// through the `source` and `tag` fields of contributions, and through
// arbitrary `JsonValue` metadata.
//
// ============================================================================
// Core design choices
// ============================================================================
//
// 1. Immutable value types for the data model (`Origin`, `Contribution`, etc.)
//    — these are pure data and benefit from value semantics. Swift structs with
//    `Sendable` conformance are safe to share across concurrency domains.
//
// 2. A mutable class (`CVLog`) for the log itself — the log is a long-lived,
//    shared owner of all CV entries for a pipeline run. We mark it
//    `@unchecked Sendable` because internal mutation is guarded at the
//    call-site (single-threaded use within a pipeline stage).
//
// 3. SHA-256 for ID derivation — the base segment of a CV ID is the first
//    8 hex characters of SHA-256(originString). This is:
//    - Deterministic (same input → same ID)
//    - Globally unique with high probability (collision at 2^32 birthday bound)
//    - Human-readable (short enough to scan in logs)
//
// 4. Dot-notation for ancestry — `a3f1.1.1.1` can be parsed without consulting
//    the log: four segments means three levels of derivation. The depth of
//    nesting is immediately visible.
//
// ============================================================================

import Foundation
import SHA256
import JsonValue
import JsonSerializer

// ============================================================================
// MARK: — Data structures
// ============================================================================
//
// All data types are `Sendable` structs. `Sendable` is a Swift 6 protocol
// marking a type safe to share across concurrency domains. Structs with
// Sendable fields are automatically Sendable; we mark it explicitly for
// clarity and compiler verification.

// ============================================================================
// Origin — where an entity was born
// ============================================================================
//
// An Origin records the provenance at the moment of creation. For a compiler,
// this might be a file path and line:column. For an ETL pipeline it might be a
// table name and row ID. The `synthetic` flag signals that the entity was
// created programmatically (no natural source), and the base segment of its
// CV ID will be "00000000".
//
// Examples:
//   Origin(string: "app.ts:5:12", synthetic: false)   // token from source file
//   Origin(string: nil, synthetic: true)               // synthetic temp node

/// The birth record of a tracked entity.
///
/// - `string`: An arbitrary string identifying where this entity came from
///   (e.g., file path, table:row, URL). Used as the input to SHA-256 to
///   derive the base segment of the CV ID.
/// - `synthetic`: When `true`, the entity has no natural origin. The CV ID
///   base segment will be `"00000000"` regardless of `string`.
public struct Origin: Sendable, Equatable {
    /// A string identifying the natural origin (file path, row ID, URL, …).
    /// `nil` is legal; a nil origin with `synthetic: false` hashes the empty
    /// string, which produces a consistent (though collision-prone) base.
    public let string: String?

    /// `true` when this entity was created programmatically with no natural
    /// origin. The base segment of its CV ID will be `"00000000"`.
    public let synthetic: Bool

    public init(string: String?, synthetic: Bool) {
        self.string = string
        self.synthetic = synthetic
    }
}

// ============================================================================
// Contribution — one stage's record of having touched an entity
// ============================================================================
//
// Every time a stage processes an entity, it appends a Contribution to that
// entity's CV. Think of contributions as a git commit history for a single
// piece of data: each entry records WHO touched it, WHAT they did, and any
// relevant detail.
//
// `source` identifies the actor (pass name, service name, stage name).
// `tag`    classifies the action (domain-defined: "resolved", "renamed", etc).
// `meta`   carries arbitrary detail as a JsonValue (often .object([...])).
// `timestamp` is an ISO 8601 UTC string recording when the contribution was made.
//
// Examples:
//   Contribution(source: "scope_analysis", tag: "resolved",
//                meta: .object([("binding", .string("local:count"))]),
//                timestamp: "2026-04-01T12:00:00Z")
//
//   Contribution(source: "dce", tag: "deleted",
//                meta: .object([("reason", .string("unreachable"))]),
//                timestamp: "2026-04-01T12:00:01Z")

/// A record of one stage having processed a tracked entity.
public struct Contribution: Sendable, Equatable {
    /// The actor that made this contribution (pass name, service name, …).
    public let source: String

    /// A domain-defined label classifying what happened (e.g., "resolved",
    /// "renamed", "compiled", "deleted").
    public let tag: String

    /// Arbitrary metadata about the contribution, encoded as a `JsonValue`.
    /// Callers typically pass `.object([…])` with domain-specific key-value pairs.
    /// `nil` means no metadata was provided.
    public let meta: JsonValue?

    /// ISO 8601 UTC timestamp marking when this contribution was recorded.
    public let timestamp: String

    public init(source: String, tag: String, meta: JsonValue? = nil, timestamp: String) {
        self.source = source
        self.tag = tag
        self.meta = meta
        self.timestamp = timestamp
    }
}

// ============================================================================
// DeletionRecord — soft-delete marker
// ============================================================================
//
// When an entity is intentionally removed from a pipeline, we record a
// DeletionRecord on its CV entry. The entry itself is NEVER removed from the
// log — this is how you can answer "why did this disappear?" long after the
// fact.
//
// Calling `contribute` on a deleted entry is a programming error and throws.
// Calling `derive` or `merge` with a deleted entry as a parent is allowed
// (e.g., "we derived a tombstone from this deleted node").

/// A soft-delete marker placed on a `CVEntry` when the entity is deleted.
public struct DeletionRecord: Sendable, Equatable {
    /// The actor that deleted this entity (pass name, pipeline stage, …).
    public let by: String

    /// ISO 8601 UTC timestamp marking when the deletion was recorded.
    public let at: String

    public init(by: String, at: String) {
        self.by = by
        self.at = at
    }
}

// ============================================================================
// CVEntry — the full provenance record for one entity
// ============================================================================
//
// A CVEntry holds everything we know about a single tracked entity:
//
//   cvId          — its stable, globally unique identity string (immutable)
//   origin        — where it was born (nil for derived/merged entities)
//   parentCvId    — the immediate parent CV for derived entities
//   mergedFrom    — the parent CVs for merged entities (>1 parent)
//   contributions — append-only history of every stage that touched it
//   deleted       — non-nil if this entity has been soft-deleted
//   passOrder     — deduplicated, ordered list of sources that touched it
//
// Note: `passOrder` is maintained by both `contribute` and `passthrough`.
// It answers "which stages saw this entity?" without scanning contributions.

/// The complete provenance record for one tracked entity.
public struct CVEntry: Sendable {
    /// Stable, globally unique identity string. Never changes after creation.
    public let cvId: String

    /// Where this entity was born. `nil` for derived or merged entities
    /// (whose lineage provides origin context via their parents).
    public var origin: Origin?

    /// For derived entities: the CV ID of the immediate parent this was
    /// derived from. `nil` for root entities and merged entities.
    public var parentCvId: String?

    /// For merged entities: the CV IDs of all parents that were merged.
    /// Empty for root and derived entities.
    public var mergedFrom: [String]

    /// Append-only history of every stage that transformed or examined
    /// this entity. Ordered chronologically (earliest first).
    public var contributions: [Contribution]

    /// Non-nil if this entity has been soft-deleted.
    public var deleted: DeletionRecord?

    /// Deduplicated, ordered list of source names that have touched this
    /// entity via `contribute` or `passthrough`. Maintained for O(1) lookups.
    public var passOrder: [String]

    public init(
        cvId: String,
        origin: Origin? = nil,
        parentCvId: String? = nil,
        mergedFrom: [String] = [],
        contributions: [Contribution] = [],
        deleted: DeletionRecord? = nil,
        passOrder: [String] = []
    ) {
        self.cvId = cvId
        self.origin = origin
        self.parentCvId = parentCvId
        self.mergedFrom = mergedFrom
        self.contributions = contributions
        self.deleted = deleted
        self.passOrder = passOrder
    }
}

// ============================================================================
// MARK: — Errors
// ============================================================================

/// An error thrown by CVLog operations.
///
/// Errors are programming mistakes — they indicate the caller is using the
/// API incorrectly (e.g., contributing to a deleted entry, looking up a
/// non-existent CV ID). They are not expected in correct code.
///
/// Named `CorrelationVectorError` (not `CVError`) to avoid collision with
/// `CoreVideo.CVError` which Apple's CoreVideo framework exposes under the
/// same short name.
public struct CorrelationVectorError: Error, Sendable {
    /// Human-readable description of what went wrong.
    public let message: String

    public init(_ message: String) {
        self.message = message
    }
}

// ============================================================================
// MARK: — CVLog
// ============================================================================
//
// CVLog is the central registry for a pipeline run. It holds all CV entries
// and provides the six core operations:
//
//   create      — born a new root CV
//   contribute  — record a stage's transformation
//   derive      — create a child CV from a parent
//   merge       — create a CV from multiple parents
//   delete      — soft-delete a CV entry
//   passthrough — record a stage examined but did not transform
//
// Plus five queries:
//   get         — look up a single entry
//   ancestors   — walk the parent chain (BFS, nearest-first)
//   descendants — find all entries that descend from a given ID
//   history     — return the contributions for a CV
//   lineage     — return the full ancestor chain as CVEntry objects
//
// And serialization:
//   serialize   — encode the log to a JSON string
//   deserialize — reconstruct a log from a JSON string
//
// ============================================================================
// The `enabled` flag
// ============================================================================
//
// When `enabled` is `false`, all WRITE operations are no-ops. The CV IDs are
// still generated and returned (entities need their IDs regardless of tracing),
// but no entries are stored in the log dictionary.
//
// This lets production code pay essentially zero overhead when tracing is off
// (no dictionary lookups, no allocations) while paying full provenance cost
// when it is on. Toggle it once at startup via a feature flag or config.
//
// ============================================================================
// Counter-based ID generation
// ============================================================================
//
// IDs are generated with a global monotonic counter:
//   base = sha8(originString)   or "00000000" for synthetics
//   cvId = "\(base).\(counter)"
//
// For derived IDs, the parent's full CV ID is used as the base:
//   childId = "\(parentCvId).\(counter)"
//
// The counter is never reset, so IDs are unique within a CVLog lifetime.
// This differs slightly from the spec's "per-base sequence counter" but
// achieves the same uniqueness guarantee more simply.

/// The central registry for a pipeline run's correlation vectors.
///
/// ```swift
/// let log = CVLog()
/// let id = log.create(originString: "app.ts:5:12")
/// try log.contribute(cvId: id, source: "parser", tag: "tokenized")
/// let childId = try log.derive(parentCvId: id, source: "scope_analysis", tag: "resolved")
/// ```
public final class CVLog: @unchecked Sendable {

    // =========================================================================
    // MARK: — State
    // =========================================================================

    /// Whether tracing is active. When `false`, write operations are no-ops.
    public let enabled: Bool

    /// All CV entries indexed by their CV ID.
    private var entries: [String: CVEntry] = [:]

    /// Global monotonic sequence counter. Increments on every create/derive/merge.
    private var counter: Int = 0

    // =========================================================================
    // MARK: — Initialisation
    // =========================================================================

    /// Create a new CVLog.
    ///
    /// - Parameter enabled: When `false`, write operations are no-ops.
    ///   CV IDs are still generated and returned, but no entries are stored.
    public init(enabled: Bool = true) {
        self.enabled = enabled
    }

    // =========================================================================
    // MARK: — Private helpers
    // =========================================================================

    /// Return the current UTC time as an ISO 8601 string.
    ///
    /// ISO 8601 looks like: `2026-04-01T12:00:00Z`
    /// The trailing `Z` means UTC (Zulu time). All timestamps in the log are
    /// UTC so they sort correctly and are unambiguous across time zones.
    private func now() -> String {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime]
        return formatter.string(from: Date())
    }

    /// Consume and return the next sequence number.
    ///
    /// The counter increments atomically within a single thread. CVLog is
    /// intended for single-threaded pipeline use; for concurrent use callers
    /// must synchronise externally (e.g., via an Actor wrapper).
    private func nextId() -> Int {
        let n = counter
        counter += 1
        return n
    }

    /// Return the first 8 hex characters of SHA-256(input).
    ///
    /// SHA-256 takes `Data`; we encode the input string as UTF-8.
    /// The output is a 64-char lowercase hex string; we take only the first 8.
    ///
    /// Why 8 characters?
    ///   8 hex chars = 32 bits. The birthday bound for a 32-bit space is 2^16
    ///   ≈ 65,536 before collision probability exceeds 50%. For most pipelines
    ///   (far fewer than 65,536 distinct origins) this is safe. The counter
    ///   suffix guarantees intra-base uniqueness regardless.
    private func sha8(_ input: String) -> String {
        let data = Data(input.utf8)
        let hex = sha256Hex(data)
        return String(hex.prefix(8))
    }

    // =========================================================================
    // MARK: — Core operations
    // =========================================================================

    // -------------------------------------------------------------------------
    // create
    // -------------------------------------------------------------------------
    //
    // Born a new root CV. The entity has no parents — it was created from nothing
    // or from an external source.
    //
    // Algorithm:
    //   1. If synthetic, base = "00000000"
    //   2. Else, base = sha8(originString ?? "")
    //   3. counter += 1; cvId = "\(base).\(counter)"
    //   4. If enabled, store a new CVEntry in the log
    //   5. Return cvId
    //
    // The cvId is always returned even when `enabled == false` — the entity
    // needs its ID for downstream operations.

    /// Create a new root CV entry.
    ///
    /// - Parameters:
    ///   - originString: An arbitrary string describing where this entity was born
    ///     (file path, row ID, URL, …). Hashed to derive the base segment.
    ///     `nil` is treated as the empty string for hashing.
    ///   - synthetic: When `true`, use `"00000000"` as the base instead of
    ///     hashing the origin string. Synthetic entities have no natural origin.
    ///   - meta: Optional `JsonValue` metadata stored in the entry.
    /// - Returns: The newly assigned CV ID string (e.g., `"a3f1b2c4.0"`).
    @discardableResult
    public func create(
        originString: String? = nil,
        synthetic: Bool = false,
        meta: JsonValue? = nil
    ) -> String {
        // Derive the base: either the all-zeros sentinel for synthetic entities,
        // or the first 8 hex chars of SHA-256(originString).
        let base = synthetic ? "00000000" : sha8(originString ?? "")
        let n = nextId()
        let cvId = "\(base).\(n)"

        if enabled {
            entries[cvId] = CVEntry(
                cvId: cvId,
                origin: Origin(string: originString, synthetic: synthetic)
            )
        }

        return cvId
    }

    // -------------------------------------------------------------------------
    // contribute
    // -------------------------------------------------------------------------
    //
    // Record that a stage processed and transformed this entity.
    //
    // Errors:
    //   - CVEntry not found (programming error: use the ID from `create`)
    //   - CVEntry is deleted (programming error: cannot contribute after deletion)
    //
    // `passOrder` is updated on every contribute call (deduplicated).

    /// Append a contribution to an existing CV entry.
    ///
    /// - Parameters:
    ///   - cvId: The CV ID of the entity being processed.
    ///   - source: The stage/pass/service making this contribution.
    ///   - tag: A domain-defined label for what happened.
    ///   - meta: Optional `JsonValue` metadata about this contribution.
    /// - Throws: `CVError` if the entry is not found or has been deleted.
    public func contribute(
        cvId: String,
        source: String,
        tag: String,
        meta: JsonValue? = nil
    ) throws {
        // No-op when tracing is disabled.
        guard enabled else { return }

        guard var entry = entries[cvId] else {
            throw CorrelationVectorError("CVEntry not found: \(cvId)")
        }
        if entry.deleted != nil {
            throw CorrelationVectorError("Cannot contribute to deleted CV: \(cvId)")
        }

        let contribution = Contribution(
            source: source,
            tag: tag,
            meta: meta,
            timestamp: now()
        )
        entry.contributions.append(contribution)

        // Deduplicated append: only add to passOrder if not already present.
        // `contains` is O(n) over the pass list, which is typically very small
        // (single-digit number of stages per pipeline).
        if !entry.passOrder.contains(source) {
            entry.passOrder.append(source)
        }

        entries[cvId] = entry
    }

    // -------------------------------------------------------------------------
    // derive
    // -------------------------------------------------------------------------
    //
    // Create a new CV that is descended from an existing one.
    //
    // Use this when one entity is split into multiple outputs, or when a
    // transformation produces a new entity that is conceptually "the same thing"
    // expressed differently.
    //
    // The child ID is: "\(parentCvId).\(counter)"
    // This embeds the full ancestry in the ID string itself.
    //
    // Example:
    //   parent = "a3f1.1"
    //   child  = "a3f1.1.2"    (next counter value after parent was "a3f1.0")

    /// Create a child CV derived from an existing parent.
    ///
    /// - Parameters:
    ///   - parentCvId: The CV ID of the parent entity.
    ///   - source: The stage/pass creating this derived entity.
    ///   - tag: A domain-defined label for what kind of derivation this is.
    ///   - meta: Optional `JsonValue` metadata.
    /// - Returns: The new child CV ID.
    /// - Throws: `CVError` if the parent is not found or has been deleted.
    @discardableResult
    public func derive(
        parentCvId: String,
        source: String,
        tag: String,
        meta: JsonValue? = nil
    ) throws -> String {
        let n = nextId()
        let newCvId = "\(parentCvId).\(n)"

        if enabled {
            guard let parent = entries[parentCvId] else {
                throw CorrelationVectorError("Parent CV not found: \(parentCvId)")
            }
            if parent.deleted != nil {
                throw CorrelationVectorError("Cannot derive from deleted CV: \(parentCvId)")
            }

            let contribution = Contribution(
                source: source,
                tag: tag,
                meta: meta,
                timestamp: now()
            )
            entries[newCvId] = CVEntry(
                cvId: newCvId,
                parentCvId: parentCvId,
                contributions: [contribution],
                passOrder: [source]
            )
        }

        return newCvId
    }

    // -------------------------------------------------------------------------
    // merge
    // -------------------------------------------------------------------------
    //
    // Create a new CV descended from MULTIPLE existing CVs. Use this when
    // multiple entities are combined into one output (e.g., function inlining
    // merges the call site and function body; a SQL JOIN merges two rows).
    //
    // The merged ID base is sha8(sorted(cvIds).joined(",")), giving a
    // deterministic base that encodes the identity of the inputs.
    //
    // `mergedFrom` lists all parent CV IDs so `ancestors` can traverse them.

    /// Merge multiple CVs into a new combined CV.
    ///
    /// - Parameters:
    ///   - cvIds: The CV IDs of all entities being merged.
    ///   - source: The stage/pass performing the merge.
    ///   - tag: A domain-defined label for this merge.
    ///   - meta: Optional `JsonValue` metadata.
    /// - Returns: The new merged CV ID.
    /// - Throws: `CVError` if any of the input CVs is not found.
    @discardableResult
    public func merge(
        cvIds: [String],
        source: String,
        tag: String,
        meta: JsonValue? = nil
    ) throws -> String {
        // Sort the IDs before hashing so the result is independent of the
        // order in which the caller lists the parents.
        let sorted = cvIds.sorted()
        let base = sha8(sorted.joined(separator: ","))
        let n = nextId()
        let mergedCvId = "\(base).\(n)"

        if enabled {
            for cvId in cvIds {
                guard entries[cvId] != nil else {
                    throw CorrelationVectorError("CV not found for merge: \(cvId)")
                }
            }

            let contribution = Contribution(
                source: source,
                tag: tag,
                meta: meta,
                timestamp: now()
            )
            entries[mergedCvId] = CVEntry(
                cvId: mergedCvId,
                mergedFrom: cvIds,
                contributions: [contribution],
                passOrder: [source]
            )
        }

        return mergedCvId
    }

    // -------------------------------------------------------------------------
    // delete
    // -------------------------------------------------------------------------
    //
    // Soft-delete a CV entry. The entry is NOT removed from the log — it stays
    // permanently so you can always answer "why did this disappear?". A
    // DeletionRecord is attached to the entry recording who deleted it and when.
    //
    // After deletion, `contribute` on the same cvId will throw. `derive` and
    // `merge` with a deleted parent are still allowed.

    /// Soft-delete a CV entry.
    ///
    /// - Parameters:
    ///   - cvId: The CV ID of the entity to delete.
    ///   - by: The stage/pass/person performing the deletion.
    /// - Throws: `CVError` if the entry is not found.
    public func delete(cvId: String, by: String) throws {
        guard enabled else { return }

        guard var entry = entries[cvId] else {
            throw CorrelationVectorError("CVEntry not found: \(cvId)")
        }
        entry.deleted = DeletionRecord(by: by, at: now())
        entries[cvId] = entry
    }

    // -------------------------------------------------------------------------
    // passthrough
    // -------------------------------------------------------------------------
    //
    // Record that a stage EXAMINED this entity but made NO changes. This is
    // the "identity contribution" — it matters for reconstructing which stages
    // an entity passed through even when nothing was transformed.
    //
    // In performance-sensitive pipelines, passthrough may be omitted for
    // known-clean stages to reduce log size. The tradeoff: that stage will be
    // invisible in the history for unaffected entities.
    //
    // Returns the same cvId (unchanged), so call-sites can use the result
    // in a chain: `let id = try log.passthrough(cvId: id, source: "checker")`

    /// Record that a stage examined this entity without transforming it.
    ///
    /// - Parameters:
    ///   - cvId: The CV ID of the entity.
    ///   - source: The stage that examined it.
    /// - Returns: The same `cvId` (unchanged).
    /// - Throws: `CVError` if the entry is not found or has been deleted.
    @discardableResult
    public func passthrough(cvId: String, source: String) throws -> String {
        guard enabled else { return cvId }

        guard var entry = entries[cvId] else {
            throw CorrelationVectorError("CVEntry not found: \(cvId)")
        }
        if entry.deleted != nil {
            throw CorrelationVectorError("Cannot passthrough deleted CV: \(cvId)")
        }

        // Deduplicated append to passOrder.
        if !entry.passOrder.contains(source) {
            entry.passOrder.append(source)
        }
        entries[cvId] = entry
        return cvId
    }

    // =========================================================================
    // MARK: — Queries
    // =========================================================================

    // -------------------------------------------------------------------------
    // get
    // -------------------------------------------------------------------------

    /// Return the full entry for a CV ID, or `nil` if not found.
    ///
    /// Returns `nil` when:
    ///   - The CV ID was never registered (programming error)
    ///   - The log was created with `enabled: false` (entries are never stored)
    public func get(cvId: String) -> CVEntry? {
        return entries[cvId]
    }

    // -------------------------------------------------------------------------
    // ancestors
    // -------------------------------------------------------------------------
    //
    // Walk the parent chain recursively and return all ancestor CV IDs, ordered
    // nearest-first (immediate parents before grandparents before great-grandparents).
    //
    // We use breadth-first search (BFS) to achieve nearest-first ordering:
    //   - Start by enqueuing the direct parents of cvId
    //   - For each dequeued ID, add it to the result and enqueue its parents
    //   - A visited set prevents infinite loops on pathological inputs
    //
    // BFS vs DFS:
    //   BFS:  nearest-first (parents before grandparents) — this is what we want
    //   DFS:  follows one branch to its root before backtracking — depth-first order
    //
    // Cycles are impossible by construction (IDs are monotonically generated),
    // but we guard with `visited` anyway for safety.

    /// Return all ancestor CV IDs, nearest-first.
    ///
    /// For a CV derived from A, which was derived from B:
    ///   `ancestors(ofA)` → `["B"]`
    ///   `ancestors(ofAChild)` → `["A", "B"]`
    ///
    /// - Parameter cvId: The CV ID to find ancestors for.
    /// - Returns: Ancestor CV IDs, ordered nearest-first (immediate parent first).
    public func ancestors(of cvId: String) -> [String] {
        var result: [String] = []
        var visited = Set<String>()
        var queue: [String] = []

        // Seed the BFS queue with the direct parents of cvId.
        if let entry = entries[cvId] {
            if let p = entry.parentCvId { queue.append(p) }
            queue.append(contentsOf: entry.mergedFrom)
        }

        while !queue.isEmpty {
            let current = queue.removeFirst()
            guard !visited.contains(current) else { continue }
            visited.insert(current)
            result.append(current)

            // Enqueue the parents of `current` for the next BFS level.
            if let entry = entries[current] {
                if let p = entry.parentCvId { queue.append(p) }
                queue.append(contentsOf: entry.mergedFrom)
            }
        }

        return result
    }

    // -------------------------------------------------------------------------
    // descendants
    // -------------------------------------------------------------------------
    //
    // Return all CV IDs that directly descend from the given cvId. "Direct"
    // means either `parentCvId == cvId` (derived) or `mergedFrom.contains(cvId)`
    // (merged). This does NOT recursively walk the descendant tree — it only
    // returns direct children.
    //
    // For deep descendants, callers can call `descendants` recursively.

    /// Return all CV entries that are direct children of the given CV ID.
    ///
    /// - Parameter cvId: The CV ID to find direct descendants for.
    /// - Returns: CV IDs of all direct children (derived or merged).
    public func descendants(of cvId: String) -> [String] {
        return entries.values
            .filter { entry in
                entry.parentCvId == cvId || entry.mergedFrom.contains(cvId)
            }
            .map { $0.cvId }
    }

    // -------------------------------------------------------------------------
    // history
    // -------------------------------------------------------------------------

    /// Return the contributions for a CV entry in chronological order.
    ///
    /// Returns an empty array if the CV is not found (e.g., when `enabled`
    /// is `false`).
    ///
    /// - Parameter cvId: The CV ID to query.
    /// - Returns: Contributions in the order they were appended.
    public func history(of cvId: String) -> [Contribution] {
        return entries[cvId]?.contributions ?? []
    }

    // -------------------------------------------------------------------------
    // lineage
    // -------------------------------------------------------------------------
    //
    // Return the full provenance chain as CVEntry objects, ordered from
    // oldest ancestor to the entity itself. This is the complete picture:
    // not just IDs, but the full entry for each ancestor.
    //
    // The ordering is the REVERSE of `ancestors` (which is nearest-first):
    //   ancestors → [immediate parent, grandparent, …, oldest]
    //   lineage   → [oldest, …, grandparent, parent, self]

    /// Return the full ancestor chain as `CVEntry` objects, oldest-first.
    ///
    /// The last element is the entry for `cvId` itself.
    ///
    /// - Parameter cvId: The CV ID to build a lineage for.
    /// - Returns: CVEntry objects from oldest ancestor to `cvId`.
    public func lineage(of cvId: String) -> [CVEntry] {
        // `ancestors` returns nearest-first; we want oldest-first, so reverse.
        let ancestorIds = ancestors(of: cvId)
        var result: [CVEntry] = []

        for id in ancestorIds.reversed() {
            if let entry = entries[id] {
                result.append(entry)
            }
        }

        // Append the entry for cvId itself (it is not included in `ancestors`).
        if let entry = entries[cvId] {
            result.append(entry)
        }

        return result
    }

    // =========================================================================
    // MARK: — Serialization
    // =========================================================================
    //
    // The log serializes to JSON using the JsonSerializer package. This produces
    // a self-contained, portable representation of the full provenance log.
    //
    // The JSON schema is:
    //   {
    //     "enabled": true,
    //     "counter": 5,
    //     "entries": [ { "cv_id": "a3f1.1", ... }, ... ]
    //   }
    //
    // Note: `entries` is an array (not an object keyed by cv_id) to ensure
    // deterministic serialization order is possible without a sorted-key dict.

    /// Serialize the CVLog to a compact JSON string.
    ///
    /// The output can be passed to `deserialize(_:)` to reconstruct an equal log.
    public func serialize() -> String {
        let serializer = JsonSerializer()
        let logValue = buildJsonValue()
        return serializer.serialize(logValue)
    }

    // Build the JsonValue tree for the whole log.
    private func buildJsonValue() -> JsonValue {
        // Sort entries by cvId for deterministic output.
        let sortedEntries = entries.values
            .sorted { $0.cvId < $1.cvId }
            .map { buildEntryJsonValue($0) }

        return .object([
            ("enabled", .bool(enabled)),
            ("counter", .number(Double(counter))),
            ("entries", .array(sortedEntries)),
        ])
    }

    // Build the JsonValue representation of a single CVEntry.
    private func buildEntryJsonValue(_ entry: CVEntry) -> JsonValue {
        var pairs: [(key: String, value: JsonValue)] = []

        pairs.append(("cv_id", .string(entry.cvId)))

        if let origin = entry.origin {
            var originPairs: [(key: String, value: JsonValue)] = []
            originPairs.append(("string", origin.string.map { .string($0) } ?? .null))
            originPairs.append(("synthetic", .bool(origin.synthetic)))
            pairs.append(("origin", .object(originPairs)))
        } else {
            pairs.append(("origin", .null))
        }

        pairs.append(("parent_cv_id", entry.parentCvId.map { .string($0) } ?? .null))
        pairs.append(("merged_from", .array(entry.mergedFrom.map { .string($0) })))
        pairs.append(("contributions", .array(entry.contributions.map { buildContributionJson($0) })))

        if let deleted = entry.deleted {
            pairs.append(("deleted", .object([
                ("by", .string(deleted.by)),
                ("at", .string(deleted.at)),
            ])))
        } else {
            pairs.append(("deleted", .null))
        }

        pairs.append(("pass_order", .array(entry.passOrder.map { .string($0) })))

        return .object(pairs)
    }

    // Build the JsonValue representation of a single Contribution.
    private func buildContributionJson(_ c: Contribution) -> JsonValue {
        let pairs: [(key: String, value: JsonValue)] = [
            ("source", .string(c.source)),
            ("tag", .string(c.tag)),
            ("meta", c.meta ?? .null),
            ("timestamp", .string(c.timestamp)),
        ]
        return .object(pairs)
    }

    // -------------------------------------------------------------------------
    // deserialize
    // -------------------------------------------------------------------------

    /// Reconstruct a CVLog from a JSON string produced by `serialize()`.
    ///
    /// - Parameter jsonString: A JSON string produced by `CVLog.serialize()`.
    /// - Returns: A fully reconstructed `CVLog`.
    /// - Throws: `CVError` for malformed JSON structure, or propagates
    ///   `JsonSerializer` parsing errors.
    public static func deserialize(_ jsonString: String) throws -> CVLog {
        let serializer = JsonSerializer()
        let value = try serializer.deserialize(jsonString)

        guard case .object(let topPairs) = value else {
            throw CorrelationVectorError("Expected JSON object at top level")
        }
        let topDict = Dictionary(topPairs.map { ($0.key, $0.value) }, uniquingKeysWith: { $1 })

        let enabled = topDict["enabled"]?.boolValue ?? true
        let counter = Int(topDict["counter"]?.doubleValue ?? 0)

        let log = CVLog(enabled: enabled)
        log.counter = counter

        if case .array(let entriesArray) = topDict["entries"] ?? .null {
            for entryVal in entriesArray {
                if let entry = parseEntryJson(entryVal) {
                    log.entries[entry.cvId] = entry
                }
            }
        }

        return log
    }

    // Parse a single CVEntry from its JsonValue representation.
    private static func parseEntryJson(_ value: JsonValue) -> CVEntry? {
        guard case .object(let pairs) = value else { return nil }
        let dict = Dictionary(pairs.map { ($0.key, $0.value) }, uniquingKeysWith: { $1 })

        guard let cvId = dict["cv_id"]?.stringValue else { return nil }

        // Parse origin (may be null in JSON).
        let origin: Origin? = {
            guard let originVal = dict["origin"], !originVal.isNull else { return nil }
            guard case .object(let oPairs) = originVal else { return nil }
            let oDict = Dictionary(oPairs.map { ($0.key, $0.value) }, uniquingKeysWith: { $1 })
            let str = oDict["string"]?.stringValue
            let syn = oDict["synthetic"]?.boolValue ?? false
            return Origin(string: str, synthetic: syn)
        }()

        let parentCvId = dict["parent_cv_id"]?.stringValue

        let mergedFrom: [String] = {
            guard case .array(let a) = dict["merged_from"] ?? .null else { return [] }
            return a.compactMap { $0.stringValue }
        }()

        let contributions: [Contribution] = {
            guard case .array(let a) = dict["contributions"] ?? .null else { return [] }
            return a.compactMap { parseContributionJson($0) }
        }()

        let deleted: DeletionRecord? = {
            guard let dVal = dict["deleted"], !dVal.isNull else { return nil }
            guard case .object(let dPairs) = dVal else { return nil }
            let dDict = Dictionary(dPairs.map { ($0.key, $0.value) }, uniquingKeysWith: { $1 })
            guard let by = dDict["by"]?.stringValue,
                  let at = dDict["at"]?.stringValue else { return nil }
            return DeletionRecord(by: by, at: at)
        }()

        let passOrder: [String] = {
            guard case .array(let a) = dict["pass_order"] ?? .null else { return [] }
            return a.compactMap { $0.stringValue }
        }()

        return CVEntry(
            cvId: cvId,
            origin: origin,
            parentCvId: parentCvId,
            mergedFrom: mergedFrom,
            contributions: contributions,
            deleted: deleted,
            passOrder: passOrder
        )
    }

    // Parse a single Contribution from its JsonValue representation.
    private static func parseContributionJson(_ value: JsonValue) -> Contribution? {
        guard case .object(let pairs) = value else { return nil }
        let dict = Dictionary(pairs.map { ($0.key, $0.value) }, uniquingKeysWith: { $1 })

        guard let source = dict["source"]?.stringValue,
              let tag = dict["tag"]?.stringValue,
              let timestamp = dict["timestamp"]?.stringValue else { return nil }

        // meta is null → nil; meta is present → store it as JsonValue
        let meta = dict["meta"].flatMap { $0.isNull ? nil : $0 }

        return Contribution(source: source, tag: tag, meta: meta, timestamp: timestamp)
    }
}
