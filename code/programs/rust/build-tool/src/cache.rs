// JSON-based build cache management.
//
// # Why caching?
//
// Without caching, every "build" would rebuild every package — even those
// whose source files haven't changed. This is wasteful for large monorepos.
// The cache records the SHA256 hash of each package's source files and
// dependencies at build time. On the next build, we compare current hashes
// against cached hashes to determine which packages actually need rebuilding.
//
// # Cache format
//
// The cache file is a JSON object mapping package names to cache entries:
//
//   {
//       "python/logic-gates": {
//           "package_hash": "abc123...",
//           "deps_hash": "def456...",
//           "last_built": "2024-01-15T10:30:00Z",
//           "status": "success"
//       }
//   }
//
// # Atomic writes
//
// To prevent corruption if the process is interrupted mid-write, we write
// to a temporary file first, then atomically rename it. On POSIX systems,
// fs::rename is atomic within the same filesystem.
//
// # Thread safety
//
// The cache uses a Mutex internally so it can be shared across threads
// during parallel builds. This mirrors the Go implementation which uses
// sync.Mutex.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single package's cached build state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// SHA256 of source files.
    pub package_hash: String,
    /// SHA256 of dependency hashes.
    pub deps_hash: String,
    /// ISO 8601 timestamp of last build.
    pub last_built: String,
    /// "success" or "failed".
    pub status: String,
}

/// Provides a read/write interface for the build cache file.
/// Uses a Mutex for thread-safe access during parallel builds.
pub struct BuildCache {
    entries: Mutex<BTreeMap<String, Entry>>,
}

impl BuildCache {
    /// Creates an empty BuildCache.
    pub fn new() -> Self {
        BuildCache {
            entries: Mutex::new(BTreeMap::new()),
        }
    }

    /// Reads cache entries from a JSON file. If the file doesn't exist
    /// or is malformed, we start with an empty cache — no error is raised.
    /// A missing cache simply means everything gets rebuilt, which is the
    /// safe default.
    pub fn load(&self, path: &Path) {
        let mut entries = self.entries.lock().unwrap();

        let data = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => {
                *entries = BTreeMap::new();
                return;
            }
        };

        match serde_json::from_str::<BTreeMap<String, Entry>>(&data) {
            Ok(raw) => *entries = raw,
            Err(_) => *entries = BTreeMap::new(),
        }
    }

    /// Writes cache entries to a JSON file with atomic write.
    ///
    /// The atomicity guarantee: we write to path + ".tmp" first, then rename.
    /// If the process crashes during the write, the original cache file is
    /// untouched. If it crashes during the rename, the temporary file may
    /// be left behind, but no data is lost.
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let entries = self.entries.lock().unwrap();

        let data = serde_json::to_string_pretty(&*entries)?;
        let data = format!("{}\n", data);

        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, data.as_bytes())?;
        fs::rename(&tmp_path, path)?;

        Ok(())
    }

    /// Determines if a package needs rebuilding. A package needs
    /// rebuilding if any of these conditions hold:
    ///
    ///  1. It's not in the cache (never built before).
    ///  2. Its source hash changed (files were modified).
    ///  3. Its dependency hash changed (a dependency was modified).
    ///  4. Its last build failed.
    ///
    /// This is the decision function at the heart of incremental builds.
    pub fn needs_build(&self, name: &str, pkg_hash: &str, deps_hash: &str) -> bool {
        let entries = self.entries.lock().unwrap();

        let entry = match entries.get(name) {
            Some(e) => e,
            None => return true,
        };

        if entry.status == "failed" {
            return true;
        }
        if entry.package_hash != pkg_hash {
            return true;
        }
        if entry.deps_hash != deps_hash {
            return true;
        }

        false
    }

    /// Stores a build result in the cache.
    pub fn record(&self, name: &str, pkg_hash: &str, deps_hash: &str, status: &str) {
        let mut entries = self.entries.lock().unwrap();

        // Get current time in RFC3339 format.
        // We use a simple approach since we don't have the chrono crate.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Format as ISO 8601 (simplified — just the unix timestamp wrapped).
        // For a production tool we'd use chrono, but this keeps dependencies minimal.
        let last_built = format_rfc3339(now);

        entries.insert(
            name.to_string(),
            Entry {
                package_hash: pkg_hash.to_string(),
                deps_hash: deps_hash.to_string(),
                last_built,
                status: status.to_string(),
            },
        );
    }

    /// Returns a copy of all cache entries (for inspection/testing).
    #[cfg(test)]
    pub fn entries(&self) -> BTreeMap<String, Entry> {
        self.entries.lock().unwrap().clone()
    }
}

/// Formats a unix timestamp as an RFC3339 string (UTC).
/// We implement this by hand to avoid adding a datetime dependency.
/// The result is like "2026-03-19T10:30:00Z".
fn format_rfc3339(unix_secs: u64) -> String {
    // Constants for time calculation.
    const SECS_PER_MINUTE: u64 = 60;
    const SECS_PER_HOUR: u64 = 3600;
    const SECS_PER_DAY: u64 = 86400;

    // Calculate time of day.
    let time_of_day = unix_secs % SECS_PER_DAY;
    let hours = time_of_day / SECS_PER_HOUR;
    let minutes = (time_of_day % SECS_PER_HOUR) / SECS_PER_MINUTE;
    let seconds = time_of_day % SECS_PER_MINUTE;

    // Calculate date from days since epoch (1970-01-01).
    let days = unix_secs / SECS_PER_DAY;

    // Compute year, month, day using a civil calendar algorithm.
    // Based on Howard Hinnant's algorithm for converting days to y/m/d.
    let era;
    let doe;
    let yoe;
    let doy;
    let mp;

    // Shift epoch from 1970-01-01 to 0000-03-01 for easier leap year handling.
    let z = days as i64 + 719468;
    era = if z >= 0 { z / 146097 } else { (z - 146096) / 146097 };
    doe = (z - era * 146097) as u64; // day of era [0, 146096]
    yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era
    let y = yoe as i64 + era * 400;
    doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    mp = (5 * doy + 2) / 153; // month index [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let y = if m <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, minutes, seconds
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_new_cache_is_empty() {
        let cache = BuildCache::new();
        assert!(cache.entries().is_empty());
    }

    #[test]
    fn test_needs_build_not_in_cache() {
        let cache = BuildCache::new();
        assert!(cache.needs_build("python/logic-gates", "abc", "def"));
    }

    #[test]
    fn test_needs_build_after_success() {
        let cache = BuildCache::new();
        cache.record("python/logic-gates", "abc", "def", "success");
        // Same hashes — no rebuild needed.
        assert!(!cache.needs_build("python/logic-gates", "abc", "def"));
    }

    #[test]
    fn test_needs_build_hash_changed() {
        let cache = BuildCache::new();
        cache.record("python/logic-gates", "abc", "def", "success");
        // Package hash changed — rebuild needed.
        assert!(cache.needs_build("python/logic-gates", "CHANGED", "def"));
    }

    #[test]
    fn test_needs_build_deps_hash_changed() {
        let cache = BuildCache::new();
        cache.record("python/logic-gates", "abc", "def", "success");
        // Deps hash changed — rebuild needed.
        assert!(cache.needs_build("python/logic-gates", "abc", "CHANGED"));
    }

    #[test]
    fn test_needs_build_after_failure() {
        let cache = BuildCache::new();
        cache.record("python/logic-gates", "abc", "def", "failed");
        // Previous failure — always rebuild.
        assert!(cache.needs_build("python/logic-gates", "abc", "def"));
    }

    #[test]
    fn test_save_and_load() {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_cache_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let cache_path = dir.join(".build-cache.json");

        // Save some entries.
        let cache = BuildCache::new();
        cache.record("python/logic-gates", "abc", "def", "success");
        cache.record("go/graph", "xyz", "uvw", "failed");
        cache.save(&cache_path).unwrap();

        // Load into a new cache.
        let cache2 = BuildCache::new();
        cache2.load(&cache_path);

        // Verify entries survived the round-trip.
        assert!(!cache2.needs_build("python/logic-gates", "abc", "def"));
        assert!(cache2.needs_build("go/graph", "xyz", "uvw")); // failed = rebuild

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_missing_file() {
        let cache = BuildCache::new();
        cache.load(Path::new("/nonexistent/cache.json"));
        assert!(cache.entries().is_empty());
    }

    #[test]
    fn test_load_malformed_file() {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_cache_bad_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let cache_path = dir.join("bad-cache.json");
        fs::write(&cache_path, "this is not json").unwrap();

        let cache = BuildCache::new();
        cache.load(&cache_path);
        assert!(cache.entries().is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_format_rfc3339() {
        // 2024-01-01T00:00:00Z = 1704067200
        let result = format_rfc3339(1704067200);
        assert_eq!(result, "2024-01-01T00:00:00Z");
    }
}
