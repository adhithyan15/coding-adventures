//! # uname — Print System Information
//!
//! This module implements the business logic for the `uname` command.
//! `uname` prints various pieces of system information: kernel name,
//! hostname, kernel release, kernel version, machine architecture,
//! and operating system.
//!
//! ## Default Output
//!
//! With no flags, `uname` prints just the kernel name:
//!
//! ```text
//!     $ uname
//!     Darwin          (on macOS)
//!     Linux           (on Linux)
//! ```
//!
//! ## All Information (-a)
//!
//! ```text
//!     $ uname -a
//!     Darwin hostname 23.1.0 Darwin Kernel Version 23.1.0 arm64 arm64 Darwin
//! ```
//!
//! ## Individual Fields
//!
//! ```text
//!     Flag    Field             Example
//!     ─────   ────────────────  ──────────────────
//!     -s      Kernel name       Darwin
//!     -n      Node name         hostname.local
//!     -r      Kernel release    23.1.0
//!     -v      Kernel version    Darwin Kernel Version 23.1.0: ...
//!     -m      Machine           arm64
//!     -p      Processor         arm64 (or "unknown")
//!     -i      Hardware platform arm64 (or "unknown")
//!     -o      Operating system  Darwin (or GNU/Linux)
//! ```
//!
//! ## Implementation Strategy
//!
//! On Unix systems, we call `libc::uname()` which fills a `utsname`
//! struct with null-terminated C strings. We convert these to Rust
//! strings for safe handling.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Container for all system information fields returned by uname.
#[derive(Debug, Clone)]
pub struct UnameInfo {
    /// Kernel/OS name (e.g., "Darwin", "Linux").
    pub sysname: String,
    /// Network node hostname (e.g., "myhost.local").
    pub nodename: String,
    /// Kernel release version (e.g., "23.1.0").
    pub release: String,
    /// Kernel version string (detailed build info).
    pub version: String,
    /// Machine hardware name (e.g., "arm64", "x86_64").
    pub machine: String,
}

/// Retrieve system information by calling `libc::uname()`.
///
/// This function wraps the POSIX `uname(2)` system call, which
/// fills a `utsname` structure with system identification info.
///
/// # How uname(2) Works
///
/// The kernel maintains a `utsname` struct with five fields:
///
/// ```text
///     struct utsname {
///         sysname[65]:   "Darwin"
///         nodename[65]:  "myhost.local"
///         release[65]:   "23.1.0"
///         version[65]:   "Darwin Kernel Version 23.1.0: ..."
///         machine[65]:   "arm64"
///     }
/// ```
///
/// Each field is a null-terminated C string. We use `CStr::from_ptr`
/// to safely convert them to Rust strings.
pub fn get_system_info() -> Result<UnameInfo, String> {
    unsafe {
        let mut buf: libc::utsname = std::mem::zeroed();
        let ret = libc::uname(&mut buf);
        if ret != 0 {
            return Err("uname: failed to get system information".to_string());
        }

        Ok(UnameInfo {
            sysname: cstr_to_string(buf.sysname.as_ptr()),
            nodename: cstr_to_string(buf.nodename.as_ptr()),
            release: cstr_to_string(buf.release.as_ptr()),
            version: cstr_to_string(buf.version.as_ptr()),
            machine: cstr_to_string(buf.machine.as_ptr()),
        })
    }
}

/// Format uname output based on which fields are requested.
///
/// If no specific fields are requested, defaults to showing just
/// the kernel name (equivalent to -s).
///
/// # Parameters
///
/// - `info`: the UnameInfo struct with all system data
/// - `show_all`: show all fields (-a)
/// - `show_kernel_name`: show sysname (-s)
/// - `show_nodename`: show hostname (-n)
/// - `show_release`: show kernel release (-r)
/// - `show_version`: show kernel version (-v)
/// - `show_machine`: show machine architecture (-m)
/// - `show_processor`: show processor type (-p)
/// - `show_hardware`: show hardware platform (-i)
/// - `show_os`: show operating system (-o)
pub fn format_uname(
    info: &UnameInfo,
    show_all: bool,
    show_kernel_name: bool,
    show_nodename: bool,
    show_release: bool,
    show_version: bool,
    show_machine: bool,
    show_processor: bool,
    show_hardware: bool,
    show_os: bool,
) -> String {
    let mut parts = Vec::new();

    // If -a is set, show everything
    // If nothing is set, default to -s (kernel name)
    let default = !show_kernel_name && !show_nodename && !show_release
        && !show_version && !show_machine && !show_processor
        && !show_hardware && !show_os && !show_all;

    if show_all || show_kernel_name || default {
        parts.push(info.sysname.clone());
    }
    if show_all || show_nodename {
        parts.push(info.nodename.clone());
    }
    if show_all || show_release {
        parts.push(info.release.clone());
    }
    if show_all || show_version {
        parts.push(info.version.clone());
    }
    if show_all || show_machine {
        parts.push(info.machine.clone());
    }
    if show_all || show_processor {
        // Processor type — often same as machine, or "unknown"
        parts.push(info.machine.clone());
    }
    if show_all || show_hardware {
        // Hardware platform — often same as machine, or "unknown"
        parts.push(info.machine.clone());
    }
    if show_all || show_os {
        // Operating system name — derived from sysname
        let os_name = match info.sysname.as_str() {
            "Linux" => "GNU/Linux".to_string(),
            other => other.to_string(),
        };
        parts.push(os_name);
    }

    parts.join(" ")
}

/// Convert a C string pointer to a Rust String.
///
/// # Safety
///
/// The pointer must point to a valid null-terminated C string.
unsafe fn cstr_to_string(ptr: *const libc::c_char) -> String {
    std::ffi::CStr::from_ptr(ptr)
        .to_string_lossy()
        .into_owned()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_system_info_succeeds() {
        let info = get_system_info();
        assert!(info.is_ok(), "get_system_info should succeed");
    }

    #[test]
    fn sysname_is_nonempty() {
        let info = get_system_info().unwrap();
        assert!(!info.sysname.is_empty(), "sysname should not be empty");
    }

    #[test]
    fn nodename_is_nonempty() {
        let info = get_system_info().unwrap();
        assert!(!info.nodename.is_empty(), "nodename should not be empty");
    }

    #[test]
    fn machine_is_nonempty() {
        let info = get_system_info().unwrap();
        assert!(!info.machine.is_empty(), "machine should not be empty");
    }

    #[test]
    fn format_default_shows_sysname() {
        let info = get_system_info().unwrap();
        let output = format_uname(&info, false, false, false, false, false, false, false, false, false);
        assert_eq!(output, info.sysname);
    }

    #[test]
    fn format_kernel_name_only() {
        let info = get_system_info().unwrap();
        let output = format_uname(&info, false, true, false, false, false, false, false, false, false);
        assert_eq!(output, info.sysname);
    }

    #[test]
    fn format_nodename_only() {
        let info = get_system_info().unwrap();
        let output = format_uname(&info, false, false, true, false, false, false, false, false, false);
        assert_eq!(output, info.nodename);
    }

    #[test]
    fn format_all_has_multiple_fields() {
        let info = get_system_info().unwrap();
        let output = format_uname(&info, true, false, false, false, false, false, false, false, false);
        // -a should produce at least sysname + nodename + release
        let parts: Vec<&str> = output.split_whitespace().collect();
        assert!(parts.len() >= 3, "uname -a should have multiple fields, got: {}", output);
    }

    #[test]
    fn format_multiple_flags() {
        let info = get_system_info().unwrap();
        let output = format_uname(&info, false, true, true, false, false, false, false, false, false);
        assert!(output.contains(&info.sysname));
        assert!(output.contains(&info.nodename));
    }

    #[test]
    fn format_os_linux() {
        let info = UnameInfo {
            sysname: "Linux".into(),
            nodename: "host".into(),
            release: "5.0".into(),
            version: "#1".into(),
            machine: "x86_64".into(),
        };
        let output = format_uname(&info, false, false, false, false, false, false, false, false, true);
        assert_eq!(output, "GNU/Linux");
    }

    #[test]
    fn format_os_darwin() {
        let info = UnameInfo {
            sysname: "Darwin".into(),
            nodename: "host".into(),
            release: "23.0".into(),
            version: "#1".into(),
            machine: "arm64".into(),
        };
        let output = format_uname(&info, false, false, false, false, false, false, false, false, true);
        assert_eq!(output, "Darwin");
    }
}
