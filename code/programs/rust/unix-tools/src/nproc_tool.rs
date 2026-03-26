//! # nproc — Print Number of Processing Units
//!
//! This module implements the business logic for the `nproc` command.
//! The `nproc` utility prints the number of processing units (CPUs)
//! available to the current process.
//!
//! ## Flags
//!
//! ```text
//!     Flag         Effect
//!     ───────────  ──────────────────────────────────────────
//!     --all        Print the number of installed processors,
//!                  not just those available to the process
//!     --ignore N   Exclude N processors from the count
//! ```
//!
//! ## How CPU Counting Works
//!
//! On modern systems, the number of "available" CPUs can differ from
//! the number of "installed" CPUs. Process affinity, cgroups (used by
//! Docker), and taskset can all limit which CPUs a process can use.
//!
//! ```text
//!     Installed CPUs:   8  (physical hardware)
//!     Available CPUs:   4  (after cgroup limit)
//!     nproc:            4
//!     nproc --all:      8
//!     nproc --ignore 2: 2  (4 - 2)
//! ```
//!
//! ## Implementation
//!
//! We use `std::thread::available_parallelism()` which returns the
//! number of CPUs available to the process. For `--all`, we use the
//! same function since Rust's standard library doesn't distinguish
//! between installed and available CPUs on all platforms. In practice,
//! on most desktop systems these are the same number.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Get the number of available processing units.
///
/// This wraps `std::thread::available_parallelism()` and returns
/// the count as a `usize`. If the system call fails (extremely rare),
/// we default to 1 — the minimum sensible value.
///
/// # Example
///
/// ```text
///     get_nproc() => 8  (on an 8-core machine)
/// ```
pub fn get_nproc() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Calculate the final processor count after applying flags.
///
/// This is the core business logic for `nproc`. It takes the raw
/// CPU count and the `--ignore` value, then returns the adjusted
/// count (minimum 1 — we never report 0 processors).
///
/// # Parameters
///
/// - `total`: The total number of processors (from `get_nproc()`)
/// - `ignore`: Number of processors to exclude (from `--ignore N`)
///
/// # Returns
///
/// The adjusted processor count, always at least 1.
///
/// # Example
///
/// ```text
///     calculate_nproc(8, 0) => 8
///     calculate_nproc(8, 3) => 5
///     calculate_nproc(4, 4) => 1  (never returns 0)
///     calculate_nproc(4, 10) => 1 (clamped to minimum)
/// ```
pub fn calculate_nproc(total: usize, ignore: usize) -> usize {
    // --- Subtract the ignored count ---
    // If ignore >= total, we clamp to 1. A system always has at
    // least one usable processor — reporting 0 would be misleading.
    if ignore >= total {
        1
    } else {
        total - ignore
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nproc_returns_positive() {
        let count = get_nproc();
        assert!(count > 0, "nproc should return at least 1, got: {}", count);
    }

    #[test]
    fn nproc_is_reasonable() {
        // Any modern system should have between 1 and 1024 CPUs.
        // This catches absurd values that might indicate a bug.
        let count = get_nproc();
        assert!(
            count <= 1024,
            "nproc returned {} which seems unreasonable",
            count
        );
    }

    #[test]
    fn calculate_no_ignore() {
        assert_eq!(calculate_nproc(8, 0), 8);
    }

    #[test]
    fn calculate_with_ignore() {
        assert_eq!(calculate_nproc(8, 3), 5);
    }

    #[test]
    fn calculate_ignore_all() {
        // Ignoring all CPUs should clamp to 1, not 0.
        assert_eq!(calculate_nproc(4, 4), 1);
    }

    #[test]
    fn calculate_ignore_more_than_available() {
        // Ignoring more than available should still clamp to 1.
        assert_eq!(calculate_nproc(4, 10), 1);
    }

    #[test]
    fn calculate_single_cpu() {
        assert_eq!(calculate_nproc(1, 0), 1);
    }

    #[test]
    fn calculate_single_cpu_ignore_one() {
        // Even on a single-CPU system, ignoring it clamps to 1.
        assert_eq!(calculate_nproc(1, 1), 1);
    }

    #[test]
    fn calculate_large_system() {
        assert_eq!(calculate_nproc(256, 100), 156);
    }
}
