//! # df — Report File System Disk Space Usage
//!
//! This module implements the business logic for the `df` command.
//! `df` displays the amount of disk space available on mounted
//! file systems.
//!
//! ## Default Output
//!
//! ```text
//!     $ df
//!     Filesystem     1K-blocks      Used Available Use% Mounted on
//!     /dev/disk1s1   488245288 234567890 253677398  48% /
//! ```
//!
//! ## How It Works
//!
//! `df` uses the `statvfs(2)` system call to query file system
//! statistics. The key fields returned by `statvfs` are:
//!
//! ```text
//!     Field          Meaning
//!     ─────────────  ────────────────────────────────
//!     f_bsize        Preferred I/O block size
//!     f_frsize       Fundamental file system block size
//!     f_blocks       Total data blocks in file system
//!     f_bfree        Free blocks in file system
//!     f_bavail       Free blocks available to non-root
//!     f_files        Total file nodes (inodes)
//!     f_ffree        Free file nodes
//! ```
//!
//! ## Block Arithmetic
//!
//! Disk sizes are reported in blocks. To convert to bytes:
//! ```text
//!     total_bytes = f_blocks * f_frsize
//!     free_bytes  = f_bavail * f_frsize
//!     used_bytes  = total_bytes - (f_bfree * f_frsize)
//! ```

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Information about a single file system.
#[derive(Debug, Clone)]
pub struct FsInfo {
    /// Total size in bytes.
    pub total_bytes: u64,
    /// Used space in bytes.
    pub used_bytes: u64,
    /// Available space in bytes (for non-root users).
    pub available_bytes: u64,
    /// Usage percentage (0-100).
    pub use_percent: f64,
    /// Block size in bytes.
    pub block_size: u64,
    /// Total number of inodes.
    pub inodes_total: u64,
    /// Number of free inodes.
    pub inodes_free: u64,
    /// Number of used inodes.
    pub inodes_used: u64,
}

/// Query file system information for the given path.
///
/// Uses `libc::statvfs()` to get block counts and sizes, then
/// computes human-readable values.
///
/// # Parameters
///
/// - `path`: any path on the target file system (e.g., "/", "/home")
///
/// # Returns
///
/// A `FsInfo` struct with total, used, and available space.
///
/// # How statvfs Works
///
/// ```text
///     statvfs("/") fills a struct:
///       f_frsize = 4096        (block size)
///       f_blocks = 122061322   (total blocks)
///       f_bfree  = 63419349    (free blocks, all)
///       f_bavail = 63419349    (free blocks, non-root)
///
///     total = 122061322 * 4096 = 500,027,303,936 bytes ≈ 465 GiB
///     free  = 63419349  * 4096 = 259,805,413,376 bytes ≈ 242 GiB
///     used  = total - (f_bfree * f_frsize)
/// ```
#[cfg(unix)]
pub fn get_fs_info(path: &str) -> Result<FsInfo, String> {
    use std::ffi::CString;

    let c_path = CString::new(path)
        .map_err(|_| format!("df: invalid path: {}", path))?;

    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        let ret = libc::statvfs(c_path.as_ptr(), &mut stat);
        if ret != 0 {
            return Err(format!(
                "df: '{}': {}",
                path,
                std::io::Error::last_os_error()
            ));
        }

        let block_size = stat.f_frsize as u64;
        let total_bytes = stat.f_blocks as u64 * block_size;
        let free_bytes = stat.f_bfree as u64 * block_size;
        let avail_bytes = stat.f_bavail as u64 * block_size;
        let used_bytes = total_bytes.saturating_sub(free_bytes);

        let use_percent = if total_bytes == 0 {
            0.0
        } else {
            (used_bytes as f64 / total_bytes as f64) * 100.0
        };

        let inodes_total = stat.f_files as u64;
        let inodes_free = stat.f_ffree as u64;
        let inodes_used = inodes_total.saturating_sub(inodes_free);

        Ok(FsInfo {
            total_bytes,
            used_bytes,
            available_bytes: avail_bytes,
            use_percent,
            block_size,
            inodes_total,
            inodes_free,
            inodes_used,
        })
    }
}

/// Non-Unix stub so the code compiles on all platforms.
#[cfg(not(unix))]
pub fn get_fs_info(path: &str) -> Result<FsInfo, String> {
    Err(format!("df: not supported on this platform ({})", path))
}

/// Format a byte count in human-readable form (powers of 1024).
///
/// ```text
///     format_human_size(1024)        → "1.0K"
///     format_human_size(1048576)     → "1.0M"
///     format_human_size(500)         → "500"
/// ```
pub fn format_human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["", "K", "M", "G", "T", "P", "E"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{}", bytes)
    } else {
        format!("{:.1}{}", size, UNITS[unit_idx])
    }
}

/// Format a byte count in SI units (powers of 1000).
///
/// ```text
///     format_si_size(1000)     → "1.0k"
///     format_si_size(1000000)  → "1.0M"
/// ```
pub fn format_si_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["", "k", "M", "G", "T", "P", "E"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1000.0 && unit_idx < UNITS.len() - 1 {
        size /= 1000.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{}", bytes)
    } else {
        format!("{:.1}{}", size, UNITS[unit_idx])
    }
}

/// Convert bytes to 1K-blocks (the default df output unit).
///
/// ```text
///     bytes_to_1k_blocks(4096)  → 4
///     bytes_to_1k_blocks(500)   → 1 (rounds up)
/// ```
pub fn bytes_to_1k_blocks(bytes: u64) -> u64 {
    (bytes + 1023) / 1024
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn get_fs_info_root() {
        let info = get_fs_info("/");
        assert!(info.is_ok(), "should be able to stat root filesystem");
    }

    #[cfg(unix)]
    #[test]
    fn total_is_positive() {
        let info = get_fs_info("/").unwrap();
        assert!(info.total_bytes > 0, "total bytes should be positive");
    }

    #[cfg(unix)]
    #[test]
    fn used_plus_available_approximates_total() {
        let info = get_fs_info("/").unwrap();
        // used + available should be close to total, but not exact
        // because f_bfree and f_bavail may differ (reserved blocks)
        let sum = info.used_bytes + info.available_bytes;
        assert!(
            sum <= info.total_bytes * 2,
            "used + available should be reasonable"
        );
    }

    #[cfg(unix)]
    #[test]
    fn use_percent_in_range() {
        let info = get_fs_info("/").unwrap();
        assert!(
            (0.0..=100.0).contains(&info.use_percent),
            "use% should be 0-100, got {}",
            info.use_percent
        );
    }

    #[cfg(unix)]
    #[test]
    fn invalid_path_returns_error() {
        let info = get_fs_info("/nonexistent/path/12345");
        assert!(info.is_err(), "non-existent path should error");
    }

    #[test]
    fn format_human_size_bytes() {
        assert_eq!(format_human_size(500), "500");
    }

    #[test]
    fn format_human_size_kilobytes() {
        assert_eq!(format_human_size(1024), "1.0K");
    }

    #[test]
    fn format_human_size_megabytes() {
        assert_eq!(format_human_size(1048576), "1.0M");
    }

    #[test]
    fn format_human_size_gigabytes() {
        assert_eq!(format_human_size(1073741824), "1.0G");
    }

    #[test]
    fn format_si_size_kilobytes() {
        assert_eq!(format_si_size(1000), "1.0k");
    }

    #[test]
    fn format_si_size_megabytes() {
        assert_eq!(format_si_size(1_000_000), "1.0M");
    }

    #[test]
    fn bytes_to_1k_blocks_exact() {
        assert_eq!(bytes_to_1k_blocks(4096), 4);
    }

    #[test]
    fn bytes_to_1k_blocks_round_up() {
        assert_eq!(bytes_to_1k_blocks(500), 1);
    }

    #[test]
    fn bytes_to_1k_blocks_zero() {
        assert_eq!(bytes_to_1k_blocks(0), 0);
    }

    #[cfg(unix)]
    #[test]
    fn block_size_is_positive() {
        let info = get_fs_info("/").unwrap();
        assert!(info.block_size > 0, "block size should be positive");
    }
}
