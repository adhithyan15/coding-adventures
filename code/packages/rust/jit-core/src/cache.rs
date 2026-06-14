//! `JITCache` and `JITCacheEntry` — compiled function storage for jit-core.
//!
//! The cache maps function names to [`JITCacheEntry`] objects.  Each entry
//! holds the native binary produced by the backend, the post-optimisation
//! CIR for debugging, and runtime execution statistics.
//!
//! # Deoptimisation tracking
//!
//! Each entry tracks how many times the compiled function has been executed
//! (`exec_count`) and how many times it fell back to the interpreter
//! (`deopt_count`).  When `deopt_count / exec_count > 0.1` the JIT marks
//! the function as `UNSPECIALIZABLE` and invalidates the compiled version.
//!
//! ## Atomic counters
//!
//! `exec_count` and `deopt_count` use
//! [`Arc<AtomicU64>`](std::sync::Arc) so that the JIT handler closure
//! registered with `vm-core` can increment `exec_count` without holding
//! a mutable reference to the `JITCache`.
//!
//! # Invalidation
//!
//! [`JITCache::invalidate`] removes the entry from `_entries`.  A separate
//! `_invalidated` set is maintained so the JIT knows not to attempt
//! re-compilation of the same function.
//!
//! # Example
//!
//! ```
//! use jit_core::cache::{JITCache, JITCacheEntry};
//!
//! let mut cache = JITCache::new();
//! let entry = JITCacheEntry::new("add", b"binary".to_vec(), "null", 2, vec![], 0);
//! cache.put(entry);
//! assert!(cache.contains("add"));
//!
//! let e = cache.get("add").unwrap();
//! e.inc_exec();
//! assert_eq!(e.exec_count(), 1);
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::cir::CIRInstr;

// ---------------------------------------------------------------------------
// JITCacheEntry
// ---------------------------------------------------------------------------

/// A compiled function's cached state, including the binary and statistics.
///
/// # Statistics fields
///
/// | Field | Type | Purpose |
/// |---|---|---|
/// | `exec_count` | `Arc<AtomicU64>` | Times the compiled binary ran successfully |
/// | `deopt_count` | `Arc<AtomicU64>` | Times execution fell back to the interpreter |
///
/// Both counters use `Arc<AtomicU64>` so the JIT handler closure can update
/// `exec_count` without owning the `JITCacheEntry`.  Call `exec_count_arc()`
/// and `deopt_count_arc()` to obtain a clone of the Arc for the closure.
pub struct JITCacheEntry {
    /// Name of the compiled function as it appears in the `IIRModule`.
    pub fn_name: String,

    /// Opaque binary blob produced by the backend (`Backend::compile`).
    pub binary: Vec<u8>,

    /// Short name of the backend that produced this binary.
    pub backend_name: String,

    /// Number of parameters the function accepts.
    pub param_count: usize,

    /// Post-optimisation CIR — the IR that was handed to the backend.
    /// Preserved for [`crate::core::JITCore::dump_ir`] and debugging.
    pub ir: Vec<CIRInstr>,

    /// Wall-clock nanoseconds spent in specialise + optimise + compile.
    pub compilation_time_ns: u64,

    exec_count: Arc<AtomicU64>,
    deopt_count: Arc<AtomicU64>,
}

impl JITCacheEntry {
    /// Create a new cache entry.
    ///
    /// `exec_count` and `deopt_count` start at zero.
    pub fn new(
        fn_name: impl Into<String>,
        binary: Vec<u8>,
        backend_name: impl Into<String>,
        param_count: usize,
        ir: Vec<CIRInstr>,
        compilation_time_ns: u64,
    ) -> Self {
        JITCacheEntry {
            fn_name: fn_name.into(),
            binary,
            backend_name: backend_name.into(),
            param_count,
            ir,
            compilation_time_ns,
            exec_count: Arc::new(AtomicU64::new(0)),
            deopt_count: Arc::new(AtomicU64::new(0)),
        }
    }

    // ------------------------------------------------------------------
    // Counter accessors
    // ------------------------------------------------------------------

    /// Current successful execution count.
    pub fn exec_count(&self) -> u64 {
        self.exec_count.load(Ordering::Relaxed)
    }

    /// Current deoptimisation count.
    pub fn deopt_count(&self) -> u64 {
        self.deopt_count.load(Ordering::Relaxed)
    }

    /// Increment the execution counter by one.
    ///
    /// Called by the JIT handler wrapper each time the compiled binary runs.
    pub fn inc_exec(&self) {
        self.exec_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the deoptimisation counter by one.
    ///
    /// Called by the deopt stub or `JITCore::record_deopt`.
    pub fn inc_deopt(&self) {
        self.deopt_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Return a clone of the execution-count `Arc` for use in closures.
    ///
    /// The JIT handler closure captures this `Arc` so it can call
    /// `fetch_add` without holding a reference to the `JITCacheEntry`.
    pub fn exec_count_arc(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.exec_count)
    }

    /// Return a clone of the deopt-count `Arc` for use in closures.
    pub fn deopt_count_arc(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.deopt_count)
    }

    // ------------------------------------------------------------------
    // Derived metrics
    // ------------------------------------------------------------------

    /// Fraction of executions that deopted.  Returns `0.0` if never executed.
    ///
    /// Used by the JIT to decide whether to permanently invalidate a function:
    /// `deopt_rate() > 0.10` → invalidate.
    pub fn deopt_rate(&self) -> f64 {
        let exec = self.exec_count();
        if exec == 0 {
            return 0.0;
        }
        self.deopt_count() as f64 / exec as f64
    }

    /// Binary size in bytes.
    pub fn binary_size(&self) -> usize {
        self.binary.len()
    }

    /// Number of CIR instructions in the post-optimisation IR.
    pub fn ir_size(&self) -> usize {
        self.ir.len()
    }

    /// Return a statistics snapshot as a flat `HashMap`.
    ///
    /// Keys: `"fn_name"`, `"backend"`, `"param_count"`, `"ir_size"`,
    /// `"binary_size"`, `"compilation_time_ns"`, `"exec_count"`,
    /// `"deopt_count"`, `"deopt_rate"`.
    pub fn as_stats(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("fn_name".into(), self.fn_name.clone());
        m.insert("backend".into(), self.backend_name.clone());
        m.insert("param_count".into(), self.param_count.to_string());
        m.insert("ir_size".into(), self.ir_size().to_string());
        m.insert("binary_size".into(), self.binary_size().to_string());
        m.insert("compilation_time_ns".into(), self.compilation_time_ns.to_string());
        m.insert("exec_count".into(), self.exec_count().to_string());
        m.insert("deopt_count".into(), self.deopt_count().to_string());
        m.insert("deopt_rate".into(), format!("{:.4}", self.deopt_rate()));
        m
    }
}

impl std::fmt::Debug for JITCacheEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JITCacheEntry")
            .field("fn_name", &self.fn_name)
            .field("backend_name", &self.backend_name)
            .field("param_count", &self.param_count)
            .field("ir_size", &self.ir_size())
            .field("binary_size", &self.binary_size())
            .field("exec_count", &self.exec_count())
            .field("deopt_count", &self.deopt_count())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// JITCache
// ---------------------------------------------------------------------------

/// Dictionary-backed cache mapping function names to compiled binaries.
///
/// Not thread-safe.  Each `JITCore` instance owns exactly one `JITCache`.
#[derive(Default)]
pub struct JITCache {
    /// Active cache entries.
    entries: HashMap<String, JITCacheEntry>,
    /// Names of permanently invalidated functions — never re-compiled.
    invalidated: std::collections::HashSet<String>,
}

impl JITCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        JITCache {
            entries: HashMap::new(),
            invalidated: std::collections::HashSet::new(),
        }
    }

    /// Return a shared reference to the entry for `fn_name`, or `None`.
    pub fn get(&self, fn_name: &str) -> Option<&JITCacheEntry> {
        self.entries.get(fn_name)
    }

    /// Store `entry`, overwriting any previous entry for the same name.
    ///
    /// Also removes `fn_name` from the invalidated set (re-compiling an
    /// invalidated function clears its invalidation).
    pub fn put(&mut self, entry: JITCacheEntry) {
        self.invalidated.remove(&entry.fn_name);
        self.entries.insert(entry.fn_name.clone(), entry);
    }

    /// Remove the compiled binary for `fn_name` and mark it as permanently
    /// invalidated so the JIT will not attempt re-compilation.
    pub fn invalidate(&mut self, fn_name: &str) {
        self.entries.remove(fn_name);
        self.invalidated.insert(fn_name.to_string());
    }

    /// Return `true` if `fn_name` has been permanently invalidated.
    pub fn is_invalidated(&self, fn_name: &str) -> bool {
        self.invalidated.contains(fn_name)
    }

    /// Return `true` if a compiled binary is available for `fn_name`.
    pub fn contains(&self, fn_name: &str) -> bool {
        self.entries.contains_key(fn_name)
    }

    /// Number of cached (compiled) functions.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return `true` if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return per-function statistics snapshots.
    ///
    /// Keys in the outer map are function names.  Each inner map has the
    /// same keys as [`JITCacheEntry::as_stats`].
    pub fn stats(&self) -> HashMap<String, HashMap<String, String>> {
        self.entries
            .iter()
            .map(|(name, entry)| (name.clone(), entry.as_stats()))
            .collect()
    }
}

impl std::fmt::Debug for JITCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JITCache")
            .field("len", &self.entries.len())
            .field("invalidated", &self.invalidated.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(name: &str) -> JITCacheEntry {
        JITCacheEntry::new(name, vec![0u8], "null", 0, vec![], 0)
    }

    #[test]
    fn put_and_get() {
        let mut cache = JITCache::new();
        cache.put(make_entry("add"));
        assert!(cache.contains("add"));
        assert!(cache.get("add").is_some());
    }

    #[test]
    fn get_missing_returns_none() {
        let cache = JITCache::new();
        assert!(cache.get("missing").is_none());
    }

    #[test]
    fn invalidate_removes_entry() {
        let mut cache = JITCache::new();
        cache.put(make_entry("add"));
        cache.invalidate("add");
        assert!(!cache.contains("add"));
        assert!(cache.is_invalidated("add"));
    }

    #[test]
    fn put_after_invalidate_clears_invalidation() {
        let mut cache = JITCache::new();
        cache.invalidate("add");
        assert!(cache.is_invalidated("add"));
        cache.put(make_entry("add"));
        assert!(!cache.is_invalidated("add"));
        assert!(cache.contains("add"));
    }

    #[test]
    fn exec_count_incremented_via_arc() {
        let mut cache = JITCache::new();
        cache.put(make_entry("f"));
        let exec_arc = cache.get("f").unwrap().exec_count_arc();
        exec_arc.fetch_add(1, Ordering::Relaxed);
        assert_eq!(cache.get("f").unwrap().exec_count(), 1);
    }

    #[test]
    fn deopt_count_incremented_via_arc() {
        let mut cache = JITCache::new();
        cache.put(make_entry("f"));
        let deopt_arc = cache.get("f").unwrap().deopt_count_arc();
        deopt_arc.fetch_add(3, Ordering::Relaxed);
        assert_eq!(cache.get("f").unwrap().deopt_count(), 3);
    }

    #[test]
    fn deopt_rate_zero_when_never_executed() {
        let entry = make_entry("f");
        assert_eq!(entry.deopt_rate(), 0.0);
    }

    #[test]
    fn deopt_rate_calculation() {
        let entry = make_entry("f");
        entry.inc_exec();
        entry.inc_exec();
        entry.inc_exec();
        entry.inc_exec();
        entry.inc_deopt();
        assert!((entry.deopt_rate() - 0.25).abs() < 1e-9);
    }

    #[test]
    fn stats_snapshot_has_expected_keys() {
        let entry = make_entry("add");
        let stats = entry.as_stats();
        for key in &["fn_name", "backend", "param_count", "ir_size", "binary_size",
                     "compilation_time_ns", "exec_count", "deopt_count", "deopt_rate"] {
            assert!(stats.contains_key(*key), "missing key: {key}");
        }
    }

    #[test]
    fn cache_stats_returns_all_entries() {
        let mut cache = JITCache::new();
        cache.put(make_entry("f1"));
        cache.put(make_entry("f2"));
        let stats = cache.stats();
        assert!(stats.contains_key("f1"));
        assert!(stats.contains_key("f2"));
    }

    #[test]
    fn cache_len_and_is_empty() {
        let mut cache = JITCache::new();
        assert!(cache.is_empty());
        cache.put(make_entry("f"));
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());
    }

    #[test]
    fn inc_exec_and_deopt_on_entry() {
        let entry = make_entry("g");
        entry.inc_exec();
        entry.inc_exec();
        entry.inc_deopt();
        assert_eq!(entry.exec_count(), 2);
        assert_eq!(entry.deopt_count(), 1);
        assert!((entry.deopt_rate() - 0.5).abs() < 1e-9);
    }
}
