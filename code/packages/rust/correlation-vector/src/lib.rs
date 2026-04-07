//! # correlation-vector
//!
//! A Correlation Vector (CV) is a lightweight, **append-only provenance record**
//! that follows a piece of data through every transformation it undergoes.
//!
//! ## Why do we need this?
//!
//! Imagine a compiler reading a source file. It creates thousands of AST nodes,
//! then passes them through scope analysis, constant folding, dead-code elimination,
//! and finally code generation. By the time output is produced, how do you know
//! which original source lines contributed to a specific output instruction? Or why
//! a particular variable disappeared?
//!
//! Assign a CV to every node at parse time. Every stage that touches a node appends
//! its contribution. Later you can reconstruct the full history: "this instruction
//! came from `app.ts:42`, was renamed by `variable_renamer`, and inlined by `dce`."
//!
//! The same idea applies to:
//! - **ETL pipelines**: track which database rows were merged, split, or dropped
//! - **Build systems**: track which source files produced which object files
//! - **ML preprocessing**: track which training samples were filtered or augmented
//!
//! ## Core idea: the ID scheme
//!
//! Every CV gets a stable ID that encodes its lineage:
//!
//! ```text
//! a3f1.1       — root CV (born directly, not derived from anything)
//! a3f1.2       — another root CV with the same origin hash
//! a3f1.1.1     — first entity derived from a3f1.1
//! a3f1.1.2     — second entity derived from a3f1.1
//! a3f1.1.1.1   — entity derived from a3f1.1.1
//! 00000000.1   — synthetic entity (no natural origin)
//! ```
//!
//! The base (`a3f1`) is an 8-character hex prefix of the SHA-256 hash of the
//! origin's identifying string. The trailing numbers are per-base and per-parent
//! sequence counters. You can read parentage directly from the ID.
//!
//! ## The `enabled` flag
//!
//! The log has an `enabled` boolean. When `false`:
//! - `create`, `derive`, `merge` still return valid IDs (counters still increment)
//! - No entries are stored — the log stays empty
//! - `contribute`, `delete`, `passthrough` are no-ops
//! - `get` returns `None`
//!
//! This lets production code pay zero overhead when tracing is off, while keeping
//! the same API surface as when tracing is on.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// Re-export the sha256 function we use for ID generation.
// We take only the first 8 hex characters of the hash.
use coding_adventures_sha256::sha256_hex;

// ===========================================================================
// Data Types
// ===========================================================================

/// Where and when an entity was born.
///
/// An origin answers the question: "Where did this thing come from before
/// we started tracking it?" For a source file node, the origin is the file
/// name and line:column. For a database row, it might be the table name and
/// row ID. For a synthetic entity, `origin` is `None`.
///
/// The `meta` field is a free-form map for any additional context that does
/// not fit neatly into `source` or `location`. The values can be any JSON-
/// compatible type (strings, numbers, arrays, objects, booleans, null).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Origin {
    /// Identifies the origin system, file, or input stream.
    /// Examples: `"app.ts"`, `"orders_table"`, `"stdin"`.
    pub source: String,

    /// Position within the source. Free-form — the consumer defines the format.
    /// Examples: `"5:12"` (line:col), `"row_id:8472"`, `"byte:4096"`.
    pub location: String,

    /// Optional ISO 8601 timestamp, for time-relevant origins.
    /// Examples: `"2024-01-15T09:30:00Z"`.
    pub timestamp: Option<String>,

    /// Any additional origin context.
    #[serde(default)]
    pub meta: HashMap<String, Value>,
}

/// A record of one stage processing one entity.
///
/// Every time a stage touches an entity, it appends a `Contribution` to that
/// entity's CV. The order of contributions is semantically meaningful — it is
/// the sequence in which stages processed the entity.
///
/// # Example
///
/// ```text
/// // Compiler variable renamer contributing to a node's CV:
/// Contribution {
///     source: "variable_renamer",
///     tag: "renamed",
///     meta: { "from": "userPreferences", "to": "a" }
/// }
///
/// // ETL date normalizer contributing to a row's CV:
/// Contribution {
///     source: "date_normalizer",
///     tag: "converted",
///     meta: { "from_format": "MM/DD/YYYY", "to_format": "ISO8601" }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contribution {
    /// Who contributed — the stage name, service name, or pass name.
    pub source: String,

    /// What happened — a domain-defined label for the type of action.
    /// The CV library imposes no constraints on this value.
    pub tag: String,

    /// Arbitrary key-value detail. Values are any JSON-compatible type.
    #[serde(default)]
    pub meta: HashMap<String, Value>,
}

/// A record that an entity was intentionally removed.
///
/// The CV entry remains in the log permanently after deletion — this is how
/// you answer "why did this thing disappear?" long after the fact. The
/// `deleted` field is non-`None` when the entity has been deleted.
///
/// Calling `contribute` on a deleted entity is an error (the entity is gone;
/// further contributions make no sense). Deriving from or merging with a
/// deleted entity is allowed (e.g., creating a tombstone record).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionRecord {
    /// Who deleted the entity.
    pub source: String,

    /// Why the entity was deleted.
    pub reason: String,

    /// Any additional deletion context.
    #[serde(default)]
    pub meta: HashMap<String, Value>,
}

/// The full record for a single CV ID.
///
/// A `CVEntry` is the complete provenance for one tracked entity. It contains:
/// - A stable `id` that never changes
/// - The list of `parent_ids` (empty for roots; one or more for derived/merged)
/// - The optional `origin` (where this entity was born)
/// - The ordered list of `contributions` (what happened to it)
/// - An optional `deleted` record (if the entity was removed)
///
/// # ID Structure
///
/// ```text
/// "a3f1.1"       → a root CV (base "a3f1", sequence 1)
/// "a3f1.1.2"     → second child derived from "a3f1.1"
/// "00000000.1"   → a synthetic CV (no natural origin)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CVEntry {
    /// Stable, globally unique identifier. Never changes after creation.
    pub id: String,

    /// Parent CV IDs. Empty for root CVs. One entry for derived CVs.
    /// Multiple entries for merged CVs.
    #[serde(default)]
    pub parent_ids: Vec<String>,

    /// Where this entity was born. `None` for synthetic entities.
    pub origin: Option<Origin>,

    /// Ordered history of every stage that processed this entity.
    #[serde(default)]
    pub contributions: Vec<Contribution>,

    /// If set, this entity was intentionally deleted.
    pub deleted: Option<DeletionRecord>,
}

/// The map that holds all CV entries for one pipeline run.
///
/// The `CVLog` is the central data structure. It travels alongside the data
/// being processed, accumulating the history of every entity. When processing
/// is done, it can be serialized to JSON for storage or cross-process transmission.
///
/// # Thread safety
///
/// `CVLog` is not `Sync` — it is designed for single-threaded use within one
/// pipeline stage. If you need to share a log across threads, wrap it in a
/// `Mutex<CVLog>`.
///
/// # Performance
///
/// When `enabled` is `false`, all write operations are no-ops. The IDs are
/// still generated (the entity needs its ID regardless of tracing), but no
/// history is stored. This means production code pays essentially zero overhead
/// when tracing is off.
pub struct CVLog {
    /// All CV entries, keyed by CV ID.
    pub entries: HashMap<String, CVEntry>,

    /// Ordered list of source names that have contributed to the log.
    /// A source name is added when it makes its first contribution to any CV.
    pub pass_order: Vec<String>,

    /// When `false`, all write operations are no-ops (no entries stored).
    /// IDs are still generated and returned.
    pub enabled: bool,

    /// Per-base sequence counters for root CVs.
    /// Key: base string (e.g., "a3f1b2c4").
    /// Value: last-used sequence number for that base.
    base_counters: HashMap<String, u32>,

    /// Per-parent sequence counters for derived CVs.
    /// Key: parent CV ID.
    /// Value: last-used child sequence number for that parent.
    child_counters: HashMap<String, u32>,
}

// ===========================================================================
// CVLog Implementation
// ===========================================================================

impl CVLog {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    /// Create a new, empty CVLog.
    ///
    /// # Arguments
    ///
    /// * `enabled` — When `true`, all operations are active. When `false`,
    ///   write operations are no-ops (IDs are still generated).
    ///
    /// # Examples
    ///
    /// ```
    /// use coding_adventures_correlation_vector::CVLog;
    ///
    /// let log = CVLog::new(true);
    /// assert!(log.enabled);
    /// assert!(log.entries.is_empty());
    /// ```
    pub fn new(enabled: bool) -> Self {
        CVLog {
            entries: HashMap::new(),
            pass_order: Vec::new(),
            enabled,
            base_counters: HashMap::new(),
            child_counters: HashMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // ID generation
    // -----------------------------------------------------------------------

    /// Compute the 8-character hex base for a root CV from its origin.
    ///
    /// The base is the first 8 hex characters of the SHA-256 hash of the
    /// string `"{source}:{location}"`. For synthetic entities (no origin),
    /// the base is always `"00000000"`.
    ///
    /// # Why SHA-256?
    ///
    /// SHA-256 gives us deterministic, collision-resistant IDs. Two different
    /// source files at the same line number get different bases. The same file
    /// at the same line always gets the same base, making the ID reproducible
    /// across runs (important for build systems and caches).
    ///
    /// Taking only 8 hex characters gives us 32 bits of collision resistance —
    /// sufficient for a single pipeline run with millions of entities, while
    /// keeping the ID human-readable.
    fn origin_base(origin: Option<&Origin>) -> String {
        match origin {
            // No natural origin → use the special "no-origin" base.
            None => "00000000".to_string(),

            // Hash "source:location" to get a stable 8-character hex base.
            Some(o) => {
                let input = format!("{}:{}", o.source, o.location);
                let full_hash = sha256_hex(input.as_bytes());
                // Take first 8 hex chars = 32 bits of the 256-bit hash.
                full_hash[..8].to_string()
            }
        }
    }

    /// Allocate the next root ID for a given base.
    ///
    /// The sequence counter for each base starts at 0 and increments on every
    /// call. The returned ID is `"{base}.{n}"` where n starts at 1.
    ///
    /// Example: if base is `"a3f1b2c4"`, successive calls return
    /// `"a3f1b2c4.1"`, `"a3f1b2c4.2"`, `"a3f1b2c4.3"`, etc.
    fn next_root_id(&mut self, base: &str) -> String {
        let counter = self.base_counters.entry(base.to_string()).or_insert(0);
        *counter += 1;
        format!("{}.{}", base, counter)
    }

    /// Allocate the next child ID for a given parent CV ID.
    ///
    /// The sequence counter for each parent starts at 0 and increments on every
    /// call. The returned ID is `"{parent_id}.{m}"` where m starts at 1.
    ///
    /// Example: if parent is `"a3f1.1"`, successive calls return
    /// `"a3f1.1.1"`, `"a3f1.1.2"`, etc.
    fn next_child_id(&mut self, parent_id: &str) -> String {
        let counter = self
            .child_counters
            .entry(parent_id.to_string())
            .or_insert(0);
        *counter += 1;
        format!("{}.{}", parent_id, counter)
    }

    // -----------------------------------------------------------------------
    // Write operations
    // -----------------------------------------------------------------------

    /// Create a new root CV and return its ID.
    ///
    /// A root CV has no parents — it was created from scratch or from an
    /// external source. Call this when an entity first enters your pipeline.
    ///
    /// The returned ID is stable for the same origin input. Creating 100 CVs
    /// with the same `origin.source` and `origin.location` gives IDs like
    /// `"a3f1.1"` through `"a3f1.100"` — same base, different sequence numbers.
    ///
    /// When `enabled` is `false`, the ID is still generated and returned,
    /// but no entry is stored in the log.
    ///
    /// # Examples
    ///
    /// ```
    /// use coding_adventures_correlation_vector::{CVLog, Origin};
    ///
    /// let mut log = CVLog::new(true);
    /// let origin = Origin {
    ///     source: "app.ts".into(),
    ///     location: "5:12".into(),
    ///     timestamp: None,
    ///     meta: Default::default(),
    /// };
    /// let id = log.create(Some(origin));
    /// assert!(id.contains('.'));  // "xxxxxxxx.1"
    /// ```
    pub fn create(&mut self, origin: Option<Origin>) -> String {
        let base = Self::origin_base(origin.as_ref());
        let id = self.next_root_id(&base);

        // When tracing is disabled, we still need to return a valid ID
        // (the entity holds onto it), but we skip storing any entry.
        if self.enabled {
            let entry = CVEntry {
                id: id.clone(),
                parent_ids: vec![],
                origin,
                contributions: vec![],
                deleted: None,
            };
            self.entries.insert(id.clone(), entry);
        }

        id
    }

    /// Record that a stage processed this entity.
    ///
    /// Contributions are appended in call order. The order is semantically
    /// meaningful — it is the sequence in which stages processed the entity.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the entity has been deleted (via `delete`). You cannot
    /// contribute to a deleted entity — it no longer exists.
    ///
    /// Returns `Ok(())` silently when `enabled` is `false` or the entity is
    /// not found in the log.
    ///
    /// # Arguments
    ///
    /// * `cv_id` — The ID of the entity being processed.
    /// * `source` — Who is contributing (stage name, service name, pass name).
    /// * `tag` — What happened (domain-defined label).
    /// * `meta` — Arbitrary key-value detail.
    ///
    /// # Examples
    ///
    /// ```
    /// use coding_adventures_correlation_vector::CVLog;
    /// use std::collections::HashMap;
    ///
    /// let mut log = CVLog::new(true);
    /// let id = log.create(None);
    /// log.contribute(&id, "parser", "created", HashMap::new()).unwrap();
    /// let history = log.history(&id);
    /// assert_eq!(history.len(), 1);
    /// assert_eq!(history[0].source, "parser");
    /// ```
    pub fn contribute(
        &mut self,
        cv_id: &str,
        source: &str,
        tag: &str,
        meta: HashMap<String, Value>,
    ) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        let entry = match self.entries.get_mut(cv_id) {
            // Not found — silently succeed (may be a disabled-mode ID).
            None => return Ok(()),
            Some(e) => e,
        };

        // Guard: contributing to a deleted entity is an error.
        if entry.deleted.is_some() {
            return Err(format!(
                "cannot contribute to deleted CV {}: entity has been removed",
                cv_id
            ));
        }

        entry.contributions.push(Contribution {
            source: source.to_string(),
            tag: tag.to_string(),
            meta,
        });

        // Track pass order — only add the source if it hasn't appeared before.
        if !self.pass_order.contains(&source.to_string()) {
            self.pass_order.push(source.to_string());
        }

        Ok(())
    }

    /// Create a new CV descended from an existing one.
    ///
    /// Use this when one entity is split into multiple outputs, or when a
    /// transformation produces a new entity that is conceptually "the same
    /// thing expressed differently".
    ///
    /// The derived CV's ID is the parent ID with a new numeric suffix:
    /// `parent_cv_id + "." + M` where M increments per parent.
    ///
    /// ```text
    /// // Destructuring {a, b} = x into two bindings:
    /// let cv_a = log.derive("a3f1.1", None);  // → "a3f1.1.1"
    /// let cv_b = log.derive("a3f1.1", None);  // → "a3f1.1.2"
    /// ```
    ///
    /// When `enabled` is `false`, the ID is still generated and returned,
    /// but no entry is stored.
    pub fn derive(&mut self, parent_cv_id: &str, origin: Option<Origin>) -> String {
        let id = self.next_child_id(parent_cv_id);

        if self.enabled {
            let entry = CVEntry {
                id: id.clone(),
                parent_ids: vec![parent_cv_id.to_string()],
                origin,
                contributions: vec![],
                deleted: None,
            };
            self.entries.insert(id.clone(), entry);
        }

        id
    }

    /// Create a new CV descended from multiple existing CVs.
    ///
    /// Use this when multiple entities are combined into one output — for
    /// example, inlining a function (call site + function body merge) or
    /// joining two database tables into one result row.
    ///
    /// The merged CV's ID uses the `00000000` base (since there is no single
    /// natural parent), unless an `origin` is provided (in which case the
    /// origin's hash is used for the base).
    ///
    /// ```text
    /// // Joining two table rows:
    /// let merged = log.merge(&["orders.1", "customers.3"], None);
    /// // → "00000000.1" (first merge with no origin)
    /// ```
    ///
    /// When `enabled` is `false`, the ID is still generated and returned,
    /// but no entry is stored.
    pub fn merge(&mut self, parent_cv_ids: &[&str], origin: Option<Origin>) -> String {
        // The base for a merge comes from the origin if provided, else "00000000".
        let base = Self::origin_base(origin.as_ref());
        let id = self.next_root_id(&base);

        if self.enabled {
            let entry = CVEntry {
                id: id.clone(),
                parent_ids: parent_cv_ids.iter().map(|s| s.to_string()).collect(),
                origin,
                contributions: vec![],
                deleted: None,
            };
            self.entries.insert(id.clone(), entry);
        }

        id
    }

    /// Record that an entity was intentionally removed.
    ///
    /// The CV entry remains in the log permanently — this is how you answer
    /// "why did this thing disappear?" long after the fact.
    ///
    /// After deletion:
    /// - `history(cv_id)` includes the deletion as its final "entry"
    /// - `contribute(cv_id, ...)` returns `Err`
    /// - `derive` and `merge` using this CV as a parent still work
    ///
    /// When `enabled` is `false`, this is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// use coding_adventures_correlation_vector::CVLog;
    /// use std::collections::HashMap;
    ///
    /// let mut log = CVLog::new(true);
    /// let id = log.create(None);
    /// log.delete(&id, "dce", "unreachable", HashMap::new());
    /// assert!(log.get(&id).unwrap().deleted.is_some());
    /// ```
    pub fn delete(
        &mut self,
        cv_id: &str,
        source: &str,
        reason: &str,
        meta: HashMap<String, Value>,
    ) {
        if !self.enabled {
            return;
        }

        if let Some(entry) = self.entries.get_mut(cv_id) {
            entry.deleted = Some(DeletionRecord {
                source: source.to_string(),
                reason: reason.to_string(),
                meta,
            });
        }
    }

    /// Record that a stage examined this entity but made no changes.
    ///
    /// This is the identity contribution — it says "I looked at this, I didn't
    /// change anything." It is important for reconstructing which stages an
    /// entity passed through even when nothing was transformed.
    ///
    /// ```text
    /// // Type checker examined the node but didn't modify it:
    /// log.passthrough(&id, "type_checker");
    /// ```
    ///
    /// In performance-sensitive pipelines, `passthrough` can be omitted for
    /// known-clean stages to reduce log size. The tradeoff is that the stage
    /// will be invisible in the history for unaffected entities.
    ///
    /// When `enabled` is `false`, this is a no-op.
    pub fn passthrough(&mut self, cv_id: &str, source: &str) {
        if !self.enabled {
            return;
        }

        // passthrough is a special contribution with tag "passthrough".
        // We don't use the Result — passthrough on a deleted entry is silently
        // ignored (the entity is gone; recording that we saw it is harmless).
        let _ = self.contribute(cv_id, source, "passthrough", HashMap::new());
    }

    // -----------------------------------------------------------------------
    // Read operations
    // -----------------------------------------------------------------------

    /// Return the full entry for a CV ID, or `None` if not found.
    ///
    /// Returns `None` when `enabled` is `false` (nothing was stored).
    ///
    /// # Examples
    ///
    /// ```
    /// use coding_adventures_correlation_vector::CVLog;
    ///
    /// let mut log = CVLog::new(true);
    /// let id = log.create(None);
    /// let entry = log.get(&id).unwrap();
    /// assert_eq!(entry.id, id);
    /// ```
    pub fn get(&self, cv_id: &str) -> Option<&CVEntry> {
        self.entries.get(cv_id)
    }

    /// Return all ancestor CV IDs, ordered from nearest to most distant.
    ///
    /// Walks the `parent_ids` chain recursively. For a linear derivation chain
    /// `A → B → C → D`, `ancestors("D")` returns `["C", "B", "A"]` (nearest first).
    ///
    /// For merged CVs, all branches of the parent tree are explored. The order
    /// follows a breadth-first traversal to maintain "nearest first" ordering.
    ///
    /// Cycles are impossible by construction (a CV cannot appear before itself
    /// in the log), but we guard against pathological inputs with a `visited` set.
    ///
    /// # Examples
    ///
    /// ```
    /// use coding_adventures_correlation_vector::CVLog;
    ///
    /// let mut log = CVLog::new(true);
    /// let a = log.create(None);
    /// let b = log.derive(&a, None);
    /// let c = log.derive(&b, None);
    /// assert_eq!(log.ancestors(&c), vec![b.clone(), a.clone()]);
    /// ```
    pub fn ancestors(&self, cv_id: &str) -> Vec<String> {
        // BFS: process parents level by level so that "nearest first" holds.
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        visited.insert(cv_id.to_string());

        // Seed the queue with the direct parents of cv_id.
        let mut queue = std::collections::VecDeque::new();
        if let Some(entry) = self.entries.get(cv_id) {
            for p in &entry.parent_ids {
                if visited.insert(p.clone()) {
                    queue.push_back(p.clone());
                }
            }
        }

        // BFS over all ancestors.
        while let Some(current) = queue.pop_front() {
            result.push(current.clone());
            if let Some(entry) = self.entries.get(&current) {
                for p in &entry.parent_ids {
                    if visited.insert(p.clone()) {
                        queue.push_back(p.clone());
                    }
                }
            }
        }

        result
    }

    /// Return all CV IDs that have this CV ID anywhere in their ancestor chain.
    ///
    /// This is the inverse of `ancestors`. It scans all entries in the log and
    /// includes any entry whose `ancestors()` set contains `cv_id`.
    ///
    /// For large logs, this can be slow (O(n * depth)). For performance-critical
    /// applications, maintain a reverse index keyed by `parent_id`.
    ///
    /// # Examples
    ///
    /// ```
    /// use coding_adventures_correlation_vector::CVLog;
    ///
    /// let mut log = CVLog::new(true);
    /// let parent = log.create(None);
    /// let child1 = log.derive(&parent, None);
    /// let child2 = log.derive(&parent, None);
    /// let mut desc = log.descendants(&parent);
    /// desc.sort();
    /// assert_eq!(desc.len(), 2);
    /// ```
    pub fn descendants(&self, cv_id: &str) -> Vec<String> {
        // Build a reverse parent→children index for efficient lookup.
        // Key: parent_id. Value: list of child IDs that have this parent.
        let mut children_of: HashMap<String, Vec<String>> = HashMap::new();
        for (id, entry) in &self.entries {
            for parent in &entry.parent_ids {
                children_of
                    .entry(parent.clone())
                    .or_default()
                    .push(id.clone());
            }
        }

        // BFS from cv_id through the children index.
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        visited.insert(cv_id.to_string());

        let mut queue = std::collections::VecDeque::new();
        if let Some(children) = children_of.get(cv_id) {
            for child in children {
                if visited.insert(child.clone()) {
                    queue.push_back(child.clone());
                }
            }
        }

        while let Some(current) = queue.pop_front() {
            result.push(current.clone());
            if let Some(children) = children_of.get(&current) {
                for child in children {
                    if visited.insert(child.clone()) {
                        queue.push_back(child.clone());
                    }
                }
            }
        }

        result
    }

    /// Return the ordered contributions for a CV ID.
    ///
    /// The contributions are in call order — the sequence in which stages
    /// processed the entity. This is the entity's history of what happened to it.
    ///
    /// Returns an empty `Vec` when the entity is not found (or when
    /// `enabled` is `false`).
    ///
    /// Note: the deletion record is NOT included here — use `get` to check
    /// `entry.deleted` directly if you need the deletion info.
    ///
    /// # Examples
    ///
    /// ```
    /// use coding_adventures_correlation_vector::CVLog;
    /// use std::collections::HashMap;
    ///
    /// let mut log = CVLog::new(true);
    /// let id = log.create(None);
    /// log.contribute(&id, "stage_a", "processed", HashMap::new()).unwrap();
    /// log.contribute(&id, "stage_b", "transformed", HashMap::new()).unwrap();
    /// let history = log.history(&id);
    /// assert_eq!(history.len(), 2);
    /// assert_eq!(history[0].source, "stage_a");
    /// assert_eq!(history[1].source, "stage_b");
    /// ```
    pub fn history(&self, cv_id: &str) -> Vec<Contribution> {
        match self.entries.get(cv_id) {
            None => vec![],
            Some(entry) => entry.contributions.clone(),
        }
    }

    /// Return the full CV entries for the entity and all its ancestors,
    /// ordered from **oldest ancestor to the entity itself**.
    ///
    /// This is the complete provenance chain — you can read it top-to-bottom
    /// to see exactly how the entity came to be and what happened to it.
    ///
    /// For a linear chain `A → B → C → D`, `lineage("D")` returns
    /// `[entry_A, entry_B, entry_C, entry_D]`.
    ///
    /// For merged CVs, all branches are included. The ordering is ancestors
    /// first, entity last — but within the ancestor set, ordering follows BFS
    /// from nearest to most distant (reversed to oldest-first).
    ///
    /// Returns an empty `Vec` when the entity is not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use coding_adventures_correlation_vector::CVLog;
    ///
    /// let mut log = CVLog::new(true);
    /// let a = log.create(None);
    /// let b = log.derive(&a, None);
    /// let chain = log.lineage(&b);
    /// assert_eq!(chain.len(), 2);
    /// assert_eq!(chain[0].id, a);
    /// assert_eq!(chain[1].id, b);
    /// ```
    pub fn lineage(&self, cv_id: &str) -> Vec<CVEntry> {
        if self.entries.get(cv_id).is_none() {
            return vec![];
        }

        // Get ancestors (nearest first), then reverse to get oldest first.
        let mut ancestor_ids = self.ancestors(cv_id);
        ancestor_ids.reverse(); // now oldest first

        // Build the lineage: all ancestors (oldest first) + the entity itself.
        let mut result = Vec::new();
        for id in &ancestor_ids {
            if let Some(entry) = self.entries.get(id) {
                result.push(entry.clone());
            }
        }
        if let Some(entry) = self.entries.get(cv_id) {
            result.push(entry.clone());
        }

        result
    }

    // -----------------------------------------------------------------------
    // Serialization
    // -----------------------------------------------------------------------

    /// Serialize the full CVLog to a JSON string.
    ///
    /// The format matches the canonical CV interchange format defined in the
    /// spec. It can be transmitted across processes or stored in a file.
    ///
    /// ```json
    /// {
    ///   "entries": { "a3f1.1": { ... }, "a3f1.2": { ... } },
    ///   "pass_order": ["parser", "scope_analysis"],
    ///   "enabled": true
    /// }
    /// ```
    ///
    /// # Implementation note
    ///
    /// We use `serde_json` directly here because our data types already use
    /// `serde_json::Value` for meta fields. Converting the entire structure
    /// through the internal `JsonValue` pipeline would require a complex
    /// double-conversion. For the CV package, `serde_json` IS the right tool
    /// because our data model is inherently JSON-native.
    pub fn to_json_string(&self) -> Result<String, String> {
        // Build a serde-serializable snapshot.
        // We need a wrapper struct for the top-level shape.
        #[derive(Serialize)]
        struct LogSnapshot<'a> {
            entries: &'a HashMap<String, CVEntry>,
            pass_order: &'a Vec<String>,
            enabled: bool,
        }

        let snap = LogSnapshot {
            entries: &self.entries,
            pass_order: &self.pass_order,
            enabled: self.enabled,
        };

        serde_json::to_string(&snap).map_err(|e| format!("serialization error: {}", e))
    }

    /// Reconstruct a CVLog from its JSON representation.
    ///
    /// The counters (`base_counters` and `child_counters`) are reconstructed
    /// by scanning all entries in the deserialized log and re-deriving the
    /// sequence numbers from the IDs. This ensures that new IDs allocated after
    /// deserialization don't collide with existing ones.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the JSON is malformed or does not match the expected
    /// CVLog schema.
    ///
    /// # Examples
    ///
    /// ```
    /// use coding_adventures_correlation_vector::CVLog;
    ///
    /// let mut log = CVLog::new(true);
    /// let id = log.create(None);
    /// let json = log.to_json_string().unwrap();
    /// let log2 = CVLog::from_json_string(&json).unwrap();
    /// assert_eq!(log2.get(&id).unwrap().id, id);
    /// ```
    pub fn from_json_string(s: &str) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct LogSnapshot {
            entries: HashMap<String, CVEntry>,
            pass_order: Vec<String>,
            enabled: bool,
        }

        let snap: LogSnapshot =
            serde_json::from_str(s).map_err(|e| format!("deserialization error: {}", e))?;

        // Reconstruct the sequence counters from the existing IDs.
        // An ID like "a3f1.3" means base "a3f1" has counter ≥ 3.
        // An ID like "a3f1.3.2" means parent "a3f1.3" has child counter ≥ 2.
        //
        // We scan every ID and update the appropriate counter to be at least
        // the sequence number embedded in the ID.
        let mut base_counters: HashMap<String, u32> = HashMap::new();
        let mut child_counters: HashMap<String, u32> = HashMap::new();

        for id in snap.entries.keys() {
            let parts: Vec<&str> = id.splitn(2, '.').collect();
            if parts.len() < 2 {
                continue;
            }
            let base = parts[0];

            // Split the suffix "3.2.1" into segments.
            let suffix_parts: Vec<&str> = id.split('.').collect();
            // suffix_parts[0] = base, suffix_parts[1] = first N, ...

            if suffix_parts.len() >= 2 {
                // Root or top-level: the first N after the base.
                if let Ok(n) = suffix_parts[1].parse::<u32>() {
                    let entry = base_counters.entry(base.to_string()).or_insert(0);
                    if n > *entry {
                        *entry = n;
                    }
                }
            }

            if suffix_parts.len() >= 3 {
                // Derived: "base.N.M" → parent is "base.N", child sequence is M.
                // More deeply nested: "base.N.M.K" → parent is "base.N.M", child is K.
                // We need to update child_counters for EVERY prefix.
                for split_at in 2..suffix_parts.len() {
                    let parent_id = suffix_parts[..split_at].join(".");
                    let child_seq = match suffix_parts[split_at].parse::<u32>() {
                        Ok(n) => n,
                        Err(_) => continue,
                    };
                    let entry = child_counters.entry(parent_id).or_insert(0);
                    if child_seq > *entry {
                        *entry = child_seq;
                    }
                }
            }
        }

        Ok(CVLog {
            entries: snap.entries,
            pass_order: snap.pass_order,
            enabled: snap.enabled,
            base_counters,
            child_counters,
        })
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Helpers ─────────────────────────────────────────────────────────────

    /// Create a simple Origin for testing.
    fn make_origin(source: &str, location: &str) -> Origin {
        Origin {
            source: source.to_string(),
            location: location.to_string(),
            timestamp: None,
            meta: HashMap::new(),
        }
    }

    /// Create a meta map with a single string value.
    fn meta(key: &str, val: &str) -> HashMap<String, Value> {
        let mut m = HashMap::new();
        m.insert(key.to_string(), Value::String(val.to_string()));
        m
    }

    // ─── Group 1: Root lifecycle ──────────────────────────────────────────────

    /// Creating a root CV with an origin should produce an ID of the form
    /// "{8-hex-chars}.{N}". The base comes from SHA-256("source:location")[:8].
    #[test]
    fn root_create_with_origin_id_format() {
        let mut log = CVLog::new(true);
        let origin = make_origin("app.ts", "5:12");
        let id = log.create(Some(origin));

        // The ID should have exactly one dot (base.N for a root).
        let dots: usize = id.chars().filter(|&c| c == '.').count();
        assert_eq!(dots, 1, "root ID should have exactly one dot, got: {}", id);

        // The base should be 8 hex characters.
        let parts: Vec<&str> = id.splitn(2, '.').collect();
        assert_eq!(parts[0].len(), 8, "base should be 8 chars, got: {}", parts[0]);
        assert!(
            parts[0].chars().all(|c| c.is_ascii_hexdigit()),
            "base should be hex, got: {}",
            parts[0]
        );

        // The sequence number should be 1 for the first ID.
        assert_eq!(parts[1], "1", "first root ID should end in .1");
    }

    /// The base for a synthetic root (no origin) is always "00000000".
    #[test]
    fn root_create_without_origin_uses_zero_base() {
        let mut log = CVLog::new(true);
        let id = log.create(None);
        assert!(id.starts_with("00000000."), "synthetic root should start with '00000000.', got: {}", id);
    }

    /// Contributing to a root CV should record the contribution in history.
    #[test]
    fn root_contribute_appears_in_history() {
        let mut log = CVLog::new(true);
        let id = log.create(None);
        log.contribute(&id, "parser", "created", HashMap::new()).unwrap();
        log.contribute(&id, "scope", "resolved", meta("binding", "local:x")).unwrap();

        let history = log.history(&id);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].source, "parser");
        assert_eq!(history[0].tag, "created");
        assert_eq!(history[1].source, "scope");
        assert_eq!(history[1].tag, "resolved");
        assert_eq!(
            history[1].meta.get("binding"),
            Some(&Value::String("local:x".to_string()))
        );
    }

    /// Passing through a stage should record a passthrough contribution.
    #[test]
    fn root_passthrough_recorded() {
        let mut log = CVLog::new(true);
        let id = log.create(None);
        log.passthrough(&id, "type_checker");

        let history = log.history(&id);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].source, "type_checker");
        assert_eq!(history[0].tag, "passthrough");
    }

    /// Deleting a CV sets the `deleted` field, and further contributions fail.
    #[test]
    fn root_delete_sets_record_and_blocks_contributions() {
        let mut log = CVLog::new(true);
        let id = log.create(None);
        log.contribute(&id, "stage_a", "processed", HashMap::new()).unwrap();
        log.delete(&id, "dce", "unreachable", meta("reason", "no callers"));

        let entry = log.get(&id).unwrap();
        assert!(entry.deleted.is_some(), "deleted field should be set");
        let del = entry.deleted.as_ref().unwrap();
        assert_eq!(del.source, "dce");
        assert_eq!(del.reason, "unreachable");
        assert_eq!(
            del.meta.get("reason"),
            Some(&Value::String("no callers".to_string()))
        );

        // Further contributions should fail.
        let result = log.contribute(&id, "optimizer", "tried", HashMap::new());
        assert!(result.is_err(), "contributing to a deleted CV should fail");
        assert!(
            result.unwrap_err().contains("deleted"),
            "error message should mention 'deleted'"
        );
    }

    /// The origin is stored in the CVEntry and accessible via `get`.
    #[test]
    fn root_origin_stored_in_entry() {
        let mut log = CVLog::new(true);
        let origin = Origin {
            source: "data.csv".to_string(),
            location: "row:42".to_string(),
            timestamp: Some("2024-01-15T09:30:00Z".to_string()),
            meta: meta("schema", "v2"),
        };
        let id = log.create(Some(origin));
        let entry = log.get(&id).unwrap();
        let stored_origin = entry.origin.as_ref().unwrap();
        assert_eq!(stored_origin.source, "data.csv");
        assert_eq!(stored_origin.location, "row:42");
        assert_eq!(
            stored_origin.timestamp.as_deref(),
            Some("2024-01-15T09:30:00Z")
        );
    }

    // ─── Group 2: Derivation ──────────────────────────────────────────────────

    /// Deriving two children from one parent gives IDs prefixed with parent ID.
    #[test]
    fn derive_two_children_have_parent_prefix() {
        let mut log = CVLog::new(true);
        let parent = log.create(Some(make_origin("app.ts", "10:5")));
        let child1 = log.derive(&parent, None);
        let child2 = log.derive(&parent, None);

        assert!(
            child1.starts_with(&parent),
            "child1 should start with parent ID. child1={}, parent={}",
            child1, parent
        );
        assert!(
            child2.starts_with(&parent),
            "child2 should start with parent ID. child2={}, parent={}",
            child2, parent
        );
        assert_ne!(child1, child2, "the two children should have different IDs");
    }

    /// The first derived child gets suffix ".1", the second gets ".2".
    #[test]
    fn derive_child_sequence_numbers() {
        let mut log = CVLog::new(true);
        let parent = log.create(None);
        let child1 = log.derive(&parent, None);
        let child2 = log.derive(&parent, None);

        assert!(child1.ends_with(".1"), "first child should end with .1, got {}", child1);
        assert!(child2.ends_with(".2"), "second child should end with .2, got {}", child2);
    }

    /// ancestors(child) returns the parent ID.
    #[test]
    fn ancestors_of_child_returns_parent() {
        let mut log = CVLog::new(true);
        let parent = log.create(None);
        let child = log.derive(&parent, None);

        let ancestors = log.ancestors(&child);
        assert_eq!(ancestors, vec![parent.clone()]);
    }

    /// descendants(parent) returns both children.
    #[test]
    fn descendants_of_parent_returns_children() {
        let mut log = CVLog::new(true);
        let parent = log.create(None);
        let child1 = log.derive(&parent, None);
        let child2 = log.derive(&parent, None);

        let mut desc = log.descendants(&parent);
        desc.sort();
        let mut expected = vec![child1, child2];
        expected.sort();
        assert_eq!(desc, expected);
    }

    /// Derived CV has the parent in its parent_ids.
    #[test]
    fn derived_cv_has_parent_id_in_entry() {
        let mut log = CVLog::new(true);
        let parent = log.create(None);
        let child = log.derive(&parent, None);
        let entry = log.get(&child).unwrap();
        assert_eq!(entry.parent_ids, vec![parent.clone()]);
    }

    // ─── Group 3: Merging ─────────────────────────────────────────────────────

    /// Merging three CVs produces an entry with all three as parent_ids.
    #[test]
    fn merge_three_cvs_parent_ids() {
        let mut log = CVLog::new(true);
        let a = log.create(None);
        let b = log.create(None);
        let c = log.create(None);

        let merged = log.merge(&[&a, &b, &c], None);
        let entry = log.get(&merged).unwrap();

        let mut parent_ids = entry.parent_ids.clone();
        parent_ids.sort();
        let mut expected = vec![a.clone(), b.clone(), c.clone()];
        expected.sort();
        assert_eq!(parent_ids, expected);
    }

    /// ancestors(merged) returns all three parents.
    #[test]
    fn ancestors_of_merged_returns_all_parents() {
        let mut log = CVLog::new(true);
        let a = log.create(None);
        let b = log.create(None);
        let c = log.create(None);

        let merged = log.merge(&[&a, &b, &c], None);
        let mut ancestors = log.ancestors(&merged);
        ancestors.sort();
        let mut expected = vec![a.clone(), b.clone(), c.clone()];
        expected.sort();
        assert_eq!(ancestors, expected);
    }

    /// Merge without origin gets "00000000" base.
    #[test]
    fn merge_without_origin_uses_zero_base() {
        let mut log = CVLog::new(true);
        let a = log.create(None);
        let b = log.create(None);
        let merged = log.merge(&[&a, &b], None);
        assert!(
            merged.starts_with("00000000."),
            "merge without origin should use '00000000.' base, got {}",
            merged
        );
    }

    /// Merge with origin uses the origin's hash for the base.
    #[test]
    fn merge_with_origin_uses_origin_hash() {
        let mut log = CVLog::new(true);
        let a = log.create(None);
        let b = log.create(None);
        let origin = make_origin("join_stage", "col:0-end");
        let merged = log.merge(&[&a, &b], Some(origin));
        assert!(
            !merged.starts_with("00000000."),
            "merge with origin should NOT use '00000000.' base, got {}",
            merged
        );
    }

    // ─── Group 4: Deep ancestry chain ─────────────────────────────────────────

    /// A → B → C → D chain: ancestors(D) = [C, B, A] (nearest first).
    #[test]
    fn deep_ancestry_chain_ancestors_nearest_first() {
        let mut log = CVLog::new(true);
        let a = log.create(None);
        let b = log.derive(&a, None);
        let c = log.derive(&b, None);
        let d = log.derive(&c, None);

        let ancestors = log.ancestors(&d);
        assert_eq!(ancestors, vec![c.clone(), b.clone(), a.clone()],
            "ancestors should be [C, B, A] (nearest first)");
    }

    /// lineage(D) returns all four entries: A, B, C, D (oldest first).
    #[test]
    fn deep_ancestry_chain_lineage_oldest_first() {
        let mut log = CVLog::new(true);
        let a = log.create(None);
        let b = log.derive(&a, None);
        let c = log.derive(&b, None);
        let d = log.derive(&c, None);

        let lineage = log.lineage(&d);
        assert_eq!(lineage.len(), 4, "lineage should have 4 entries");
        assert_eq!(lineage[0].id, a, "oldest ancestor first");
        assert_eq!(lineage[1].id, b);
        assert_eq!(lineage[2].id, c);
        assert_eq!(lineage[3].id, d, "entity itself last");
    }

    /// ancestors(root) returns empty (a root has no parents).
    #[test]
    fn ancestors_of_root_is_empty() {
        let mut log = CVLog::new(true);
        let root = log.create(None);
        let ancestors = log.ancestors(&root);
        assert!(ancestors.is_empty(), "root has no ancestors");
    }

    /// descendants of a leaf (nothing derived from it) is empty.
    #[test]
    fn descendants_of_leaf_is_empty() {
        let mut log = CVLog::new(true);
        let root = log.create(None);
        let leaf = log.derive(&root, None);
        let desc = log.descendants(&leaf);
        assert!(desc.is_empty(), "leaf has no descendants");
    }

    // ─── Group 5: Disabled log ────────────────────────────────────────────────

    /// All operations complete without error when enabled=false.
    #[test]
    fn disabled_log_all_ops_succeed() {
        let mut log = CVLog::new(false);
        let root = log.create(None);
        let child = log.derive(&root, None);
        let merged = log.merge(&[&root, &child], None);
        log.contribute(&root, "stage", "tag", HashMap::new()).unwrap();
        log.passthrough(&child, "inspector");
        log.delete(&merged, "dce", "no refs", HashMap::new());
        // If we get here without panic or error, all ops succeeded.
    }

    /// IDs are still generated when enabled=false.
    #[test]
    fn disabled_log_ids_still_generated() {
        let mut log = CVLog::new(false);
        let id1 = log.create(None);
        let id2 = log.create(None);
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
        assert_ne!(id1, id2, "IDs should still be unique even when disabled");
    }

    /// get() returns None when enabled=false.
    #[test]
    fn disabled_log_get_returns_none() {
        let mut log = CVLog::new(false);
        let id = log.create(None);
        assert!(log.get(&id).is_none(), "get should return None when disabled");
    }

    /// history() returns empty list when enabled=false.
    #[test]
    fn disabled_log_history_is_empty() {
        let mut log = CVLog::new(false);
        let id = log.create(None);
        log.contribute(&id, "stage", "tag", HashMap::new()).unwrap();
        assert!(log.history(&id).is_empty(), "history should be empty when disabled");
    }

    /// Counters still increment when disabled, so IDs after re-enabling don't collide.
    #[test]
    fn disabled_log_counters_increment() {
        let mut log = CVLog::new(false);
        let _ = log.create(None); // counter = 1
        let _ = log.create(None); // counter = 2
        log.enabled = true;
        let id = log.create(None);
        // Should be "00000000.3" — not "00000000.1"
        assert!(
            id.ends_with(".3"),
            "counter should have incremented even while disabled, got {}",
            id
        );
    }

    // ─── Group 6: Serialization roundtrip ─────────────────────────────────────

    /// Build a CVLog with roots, derivations, merges, deletions, and verify
    /// that serializing and deserializing preserves all entries byte-for-byte.
    #[test]
    fn serialization_roundtrip() {
        let mut log = CVLog::new(true);

        let root1 = log.create(Some(make_origin("src.ts", "1:1")));
        let root2 = log.create(Some(make_origin("lib.ts", "5:3")));
        log.contribute(&root1, "parser", "created", meta("token", "IDENTIFIER")).unwrap();
        log.contribute(&root1, "scope", "resolved", meta("binding", "local:x")).unwrap();
        let child = log.derive(&root1, None);
        let merged = log.merge(&[&root1, &root2], Some(make_origin("joiner", "0:end")));
        log.delete(&root2, "dce", "unreachable", HashMap::new());
        log.passthrough(&child, "type_checker");

        let json = log.to_json_string().unwrap();
        let log2 = CVLog::from_json_string(&json).unwrap();

        // Every entry should be present in the deserialized log.
        for (id, entry) in &log.entries {
            let entry2 = log2.get(id).expect(&format!("entry {} missing after roundtrip", id));
            assert_eq!(entry.id, entry2.id, "id mismatch for {}", id);
            assert_eq!(entry.parent_ids, entry2.parent_ids, "parent_ids mismatch for {}", id);
            assert_eq!(entry.contributions.len(), entry2.contributions.len(),
                "contribution count mismatch for {}", id);
            // Check deletion survived roundtrip.
            assert_eq!(
                entry.deleted.is_some(),
                entry2.deleted.is_some(),
                "deletion status mismatch for {}",
                id
            );
        }

        // pass_order preserved.
        assert_eq!(log.pass_order, log2.pass_order);
        // enabled preserved.
        assert_eq!(log.enabled, log2.enabled);

        // Verify specific IDs match.
        assert_eq!(log2.get(&root1).unwrap().contributions.len(), 2);
        assert!(log2.get(&root2).unwrap().deleted.is_some());
        assert_eq!(log2.get(&merged).unwrap().parent_ids.len(), 2);
    }

    /// Deserializing a CVLog and then allocating new IDs doesn't collide
    /// with existing IDs.
    #[test]
    fn deserialization_counters_reconstructed() {
        let mut log = CVLog::new(true);
        let _ = log.create(None); // 00000000.1
        let _ = log.create(None); // 00000000.2
        let id3 = log.create(None); // 00000000.3

        let json = log.to_json_string().unwrap();
        let mut log2 = CVLog::from_json_string(&json).unwrap();

        // The next ID allocated should be 00000000.4, not 00000000.1.
        let next = log2.create(None);
        assert!(
            next.ends_with(".4"),
            "after roundtrip, next ID should be .4, got {}",
            next
        );
        // It shouldn't conflict with existing IDs.
        assert_ne!(next, id3);
    }

    /// to_json_string produces valid JSON.
    #[test]
    fn to_json_string_produces_valid_json() {
        let mut log = CVLog::new(true);
        log.create(Some(make_origin("file.ts", "3:7")));
        let json = log.to_json_string().unwrap();
        // Verify it parses as valid JSON (using serde_json).
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("entries").is_some());
        assert!(parsed.get("pass_order").is_some());
        assert!(parsed.get("enabled").is_some());
    }

    // ─── Group 7: ID uniqueness ───────────────────────────────────────────────

    /// Create 10,000 root CVs with the same origin → all IDs should be unique.
    #[test]
    fn id_uniqueness_same_origin_10000() {
        let mut log = CVLog::new(true);
        let origin_factory = || make_origin("same_source", "1:1");
        let mut ids = std::collections::HashSet::new();

        for _ in 0..10_000 {
            let id = log.create(Some(origin_factory()));
            let inserted = ids.insert(id.clone());
            assert!(inserted, "duplicate ID generated: {}", id);
        }
        assert_eq!(ids.len(), 10_000);
    }

    /// Mix of origins → no collisions across bases.
    #[test]
    fn id_uniqueness_mixed_origins_no_collisions() {
        let mut log = CVLog::new(true);
        let mut ids = std::collections::HashSet::new();
        let sources = ["file_a.ts", "file_b.ts", "file_c.ts"];

        for source in &sources {
            for line in 1..=100u32 {
                let origin = make_origin(source, &format!("{}:0", line));
                let id = log.create(Some(origin));
                let inserted = ids.insert(id.clone());
                assert!(inserted, "duplicate ID: {}", id);
            }
        }
        assert_eq!(ids.len(), 300);
    }

    // ─── Additional edge cases ────────────────────────────────────────────────

    /// pass_order tracks sources in contribution order (first appearance).
    #[test]
    fn pass_order_tracks_contribution_order() {
        let mut log = CVLog::new(true);
        let id = log.create(None);
        log.contribute(&id, "parser", "created", HashMap::new()).unwrap();
        log.contribute(&id, "scope", "resolved", HashMap::new()).unwrap();
        log.contribute(&id, "parser", "revisited", HashMap::new()).unwrap(); // duplicate source
        log.contribute(&id, "dce", "eliminated", HashMap::new()).unwrap();

        // parser, scope, dce — in order of first appearance. "parser" only once.
        assert_eq!(log.pass_order, vec!["parser", "scope", "dce"]);
    }

    /// get() returns None for an unknown CV ID.
    #[test]
    fn get_unknown_id_returns_none() {
        let log = CVLog::new(true);
        assert!(log.get("not_a_real_id").is_none());
    }

    /// history() returns empty for an unknown CV ID.
    #[test]
    fn history_unknown_id_returns_empty() {
        let log = CVLog::new(true);
        assert!(log.history("ghost_id").is_empty());
    }

    /// lineage() returns empty for an unknown CV ID.
    #[test]
    fn lineage_unknown_id_returns_empty() {
        let log = CVLog::new(true);
        assert!(log.lineage("ghost_id").is_empty());
    }

    /// ancestors() returns empty for an unknown CV ID.
    #[test]
    fn ancestors_unknown_id_returns_empty() {
        let log = CVLog::new(true);
        assert!(log.ancestors("ghost_id").is_empty());
    }

    /// descendants() returns empty for an unknown CV ID.
    #[test]
    fn descendants_unknown_id_returns_empty() {
        let log = CVLog::new(true);
        assert!(log.descendants("ghost_id").is_empty());
    }

    /// Calling derive on a deleted CV is allowed.
    #[test]
    fn derive_from_deleted_cv_is_allowed() {
        let mut log = CVLog::new(true);
        let parent = log.create(None);
        log.delete(&parent, "dce", "pruned", HashMap::new());
        // Should not panic or error.
        let child = log.derive(&parent, None);
        assert!(!child.is_empty());
    }

    /// Merging with a deleted CV as a parent is allowed.
    #[test]
    fn merge_with_deleted_cv_as_parent_is_allowed() {
        let mut log = CVLog::new(true);
        let a = log.create(None);
        let b = log.create(None);
        log.delete(&a, "pruner", "gone", HashMap::new());
        // Should not panic.
        let merged = log.merge(&[&a, &b], None);
        let entry = log.get(&merged).unwrap();
        assert!(entry.parent_ids.contains(&a));
    }

    /// Two CVs with different origins get different bases.
    #[test]
    fn different_origins_get_different_bases() {
        let mut log = CVLog::new(true);
        let id1 = log.create(Some(make_origin("file_a.ts", "1:1")));
        let id2 = log.create(Some(make_origin("file_b.ts", "99:42")));
        let base1: &str = id1.splitn(2, '.').next().unwrap();
        let base2: &str = id2.splitn(2, '.').next().unwrap();
        assert_ne!(base1, base2, "different origins should produce different bases");
    }

    /// Same origin always produces the same base (deterministic).
    #[test]
    fn same_origin_always_same_base() {
        // Two separate logs, same origin → same base.
        let mut log1 = CVLog::new(true);
        let mut log2 = CVLog::new(true);
        let id1 = log1.create(Some(make_origin("app.ts", "10:5")));
        let id2 = log2.create(Some(make_origin("app.ts", "10:5")));
        let base1: &str = id1.splitn(2, '.').next().unwrap();
        let base2: &str = id2.splitn(2, '.').next().unwrap();
        assert_eq!(base1, base2, "same origin should always produce same base");
    }

    /// Deeply nested derivation produces the expected ID structure.
    #[test]
    fn deep_derivation_id_structure() {
        let mut log = CVLog::new(true);
        let a = log.create(None);   // "00000000.1"
        let b = log.derive(&a, None);   // "00000000.1.1"
        let c = log.derive(&b, None);   // "00000000.1.1.1"
        let d = log.derive(&c, None);   // "00000000.1.1.1.1"

        // Each level adds one segment.
        let a_dots = a.chars().filter(|&ch| ch == '.').count();
        let b_dots = b.chars().filter(|&ch| ch == '.').count();
        let c_dots = c.chars().filter(|&ch| ch == '.').count();
        let d_dots = d.chars().filter(|&ch| ch == '.').count();

        assert_eq!(a_dots, 1, "root has 1 dot");
        assert_eq!(b_dots, 2, "first derived has 2 dots");
        assert_eq!(c_dots, 3, "second derived has 3 dots");
        assert_eq!(d_dots, 4, "third derived has 4 dots");
    }

    /// Contribution meta values of various JSON types survive roundtrip.
    #[test]
    fn meta_various_types_roundtrip() {
        let mut log = CVLog::new(true);
        let id = log.create(None);
        let mut meta_map = HashMap::new();
        meta_map.insert("string".to_string(), Value::String("hello".to_string()));
        meta_map.insert("number".to_string(), Value::Number(42.into()));
        meta_map.insert("bool".to_string(), Value::Bool(true));
        meta_map.insert("null".to_string(), Value::Null);
        meta_map.insert(
            "array".to_string(),
            Value::Array(vec![Value::Number(1.into()), Value::Number(2.into())]),
        );
        log.contribute(&id, "stage", "tagged", meta_map.clone()).unwrap();

        let json = log.to_json_string().unwrap();
        let log2 = CVLog::from_json_string(&json).unwrap();
        let history = log2.history(&id);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].meta.get("string"), Some(&Value::String("hello".to_string())));
        assert_eq!(history[0].meta.get("bool"), Some(&Value::Bool(true)));
        assert_eq!(history[0].meta.get("null"), Some(&Value::Null));
    }
}
