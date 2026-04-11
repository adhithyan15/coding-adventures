/// # cpu-cache -- Configurable CPU cache hierarchy simulator
///
/// This crate simulates a multi-level cache hierarchy like those found in
/// modern CPUs. The same `Cache` struct serves as L1, L2, or L3 by configuring
/// size, associativity, and latency differently.
///
/// ## Modules
/// - `cache_line` - `CacheLine`: the smallest unit of cached data
/// - `cache_set` - `CacheSet` + `CacheConfig`: set-associative lookup with LRU
/// - `cache` - `Cache`: a single configurable cache level
/// - `hierarchy` - `CacheHierarchy`: L1I/L1D/L2/L3 composition
/// - `stats` - `CacheStats`: hit rate, miss rate, eviction tracking
///
/// ## Quick start
/// ```
/// use cpu_cache::{Cache, CacheConfig, CacheHierarchy};
///
/// let l1d = Cache::new(CacheConfig::new("L1D", 1024, 64, 4, 1));
/// let l2 = Cache::new(CacheConfig::new("L2", 4096, 64, 8, 10));
/// let mut hierarchy = CacheHierarchy::new(None, Some(l1d), Some(l2), None, 100);
/// let result = hierarchy.read(0x1000, false, 0);
/// assert_eq!(result.served_by, "memory");
/// ```
pub mod cache;
pub mod cache_line;
pub mod cache_set;
pub mod hierarchy;
pub mod stats;

// Re-export the main types at the crate root for convenient access.
pub use cache::{Cache, CacheAccess};
pub use cache_line::CacheLine;
pub use cache_set::{CacheConfig, CacheSet, WritePolicy};
pub use hierarchy::{CacheHierarchy, HierarchyAccess};
pub use stats::CacheStats;
