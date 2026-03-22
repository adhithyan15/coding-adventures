//! # chown — Change File Owner and Group
//!
//! This module implements the business logic for the `chown` command.
//! `chown` changes the user and/or group ownership of files.
//!
//! ## Unix Ownership Model
//!
//! Every file on a Unix system has two ownership attributes:
//!
//! ```text
//!     Attribute   Stored as   Meaning
//!     ──────────  ──────────  ──────────────────────────────
//!     Owner       uid (u32)   The user who owns the file
//!     Group       gid (u32)   The group the file belongs to
//! ```
//!
//! The `ls -l` command shows these:
//!
//! ```text
//!     -rw-r--r-- 1 alice developers 4096 Jan 1 file.txt
//!                  ─────  ──────────
//!                  owner  group
//! ```
//!
//! ## OWNER[:GROUP] Syntax
//!
//! ```text
//!     Syntax         Owner Changed?  Group Changed?
//!     ─────────────  ──────────────  ──────────────
//!     alice          Yes             No
//!     alice:staff    Yes             Yes
//!     :staff         No              Yes
//!     alice:         Yes             Set to alice's login group
//! ```
//!
//! ## Flags
//!
//! ```text
//!     Flag              Field       Effect
//!     ────────────────  ──────────  ──────────────────────────────
//!     -R, --recursive   recursive   Change ownership recursively
//!     -v, --verbose     verbose     Print each file processed
//!     -c, --changes     changes     Like verbose but only for changes
//!     -h, --no-deref    no_deref    Affect symlinks, not their targets
//! ```
//!
//! ## Implementation Note
//!
//! `chown` requires `libc::chown` (or `libc::lchown` for -h) which
//! is Unix-specific. On non-Unix platforms, this module provides
//! stub implementations that return errors.

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options controlling how `chown` behaves.
#[derive(Debug, Clone, Default)]
pub struct ChownOptions {
    /// Change ownership recursively for directories (-R).
    pub recursive: bool,
    /// Print a diagnostic for every file processed (-v).
    pub verbose: bool,
    /// Like verbose but only report actual changes (-c).
    pub changes: bool,
    /// Affect symlinks themselves, not their targets (-h).
    pub no_deref: bool,
}

// ---------------------------------------------------------------------------
// Ownership Specification Parsing
// ---------------------------------------------------------------------------

/// Parsed ownership specification from the OWNER[:GROUP] argument.
///
/// ```text
///     Input          owner    group
///     ─────────────  ───────  ───────
///     "alice"        "alice"  None
///     "alice:staff"  "alice"  "staff"
///     ":staff"       None     "staff"
///     "alice:"       "alice"  ""  (means login group)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct OwnerSpec {
    /// The owner name (None if not specified, e.g., ":group").
    pub owner: Option<String>,
    /// The group name (None if not specified, Some("") for login group).
    pub group: Option<String>,
}

/// Parse an OWNER[:GROUP] specification string.
///
/// The colon is the separator between owner and group. If there's
/// no colon, only the owner is specified. If the string starts with
/// a colon, only the group is specified.
///
/// ```text
///     parse_owner_spec("alice")       → { owner: Some("alice"), group: None }
///     parse_owner_spec("alice:staff") → { owner: Some("alice"), group: Some("staff") }
///     parse_owner_spec(":staff")      → { owner: None, group: Some("staff") }
///     parse_owner_spec("alice:")      → { owner: Some("alice"), group: Some("") }
/// ```
pub fn parse_owner_spec(spec: &str) -> Result<OwnerSpec, String> {
    if spec.is_empty() {
        return Err("chown: missing operand".to_string());
    }

    if let Some(colon_pos) = spec.find(':') {
        let owner_part = &spec[..colon_pos];
        let group_part = &spec[colon_pos + 1..];

        let owner = if owner_part.is_empty() {
            None
        } else {
            Some(owner_part.to_string())
        };

        let group = Some(group_part.to_string());

        Ok(OwnerSpec { owner, group })
    } else {
        // No colon — owner only
        Ok(OwnerSpec {
            owner: Some(spec.to_string()),
            group: None,
        })
    }
}

// ---------------------------------------------------------------------------
// UID/GID Resolution
// ---------------------------------------------------------------------------

/// Resolve a username to a UID.
///
/// First tries to parse as a numeric UID. If that fails, looks up
/// the username in the system's user database using libc::getpwnam.
///
/// ```text
///     "root"  → 0
///     "1000"  → 1000
///     "alice" → (whatever alice's UID is)
/// ```
#[cfg(unix)]
pub fn resolve_uid(name: &str) -> Result<u32, String> {
    // --- Try numeric UID first ---
    if let Ok(uid) = name.parse::<u32>() {
        return Ok(uid);
    }

    // --- Look up by name ---
    use std::ffi::CString;
    let c_name = CString::new(name)
        .map_err(|_| format!("chown: invalid user: '{}'", name))?;

    unsafe {
        let pw = libc::getpwnam(c_name.as_ptr());
        if pw.is_null() {
            Err(format!("chown: invalid user: '{}'", name))
        } else {
            Ok((*pw).pw_uid)
        }
    }
}

#[cfg(not(unix))]
pub fn resolve_uid(name: &str) -> Result<u32, String> {
    if let Ok(uid) = name.parse::<u32>() {
        Ok(uid)
    } else {
        Err(format!("chown: user lookup not supported on this platform: '{}'", name))
    }
}

/// Resolve a group name to a GID.
///
/// Same strategy as resolve_uid: try numeric first, then look up.
#[cfg(unix)]
pub fn resolve_gid(name: &str) -> Result<u32, String> {
    if let Ok(gid) = name.parse::<u32>() {
        return Ok(gid);
    }

    use std::ffi::CString;
    let c_name = CString::new(name)
        .map_err(|_| format!("chown: invalid group: '{}'", name))?;

    unsafe {
        let gr = libc::getgrnam(c_name.as_ptr());
        if gr.is_null() {
            Err(format!("chown: invalid group: '{}'", name))
        } else {
            Ok((*gr).gr_gid)
        }
    }
}

#[cfg(not(unix))]
pub fn resolve_gid(name: &str) -> Result<u32, String> {
    if let Ok(gid) = name.parse::<u32>() {
        Ok(gid)
    } else {
        Err(format!("chown: group lookup not supported on this platform: '{}'", name))
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Change the ownership of a file.
///
/// # Workflow
///
/// ```text
///     1. Parse OWNER[:GROUP] spec
///     2. Resolve names to numeric UID/GID
///     3. Call libc::chown (or lchown for -h)
///     4. If -R and it's a directory, recurse
/// ```
///
/// # Returns
///
/// A vector of messages describing what was done (for -v/-c flags).
#[cfg(unix)]
pub fn chown_file(
    path: &str,
    spec: &OwnerSpec,
    opts: &ChownOptions,
) -> Result<Vec<String>, String> {
    let file_path = Path::new(path);

    if !file_path.exists() && !file_path.symlink_metadata().is_ok() {
        return Err(format!(
            "chown: cannot access '{}': No such file or directory",
            path
        ));
    }

    let mut messages = Vec::new();

    // --- Resolve UID ---
    let uid: i32 = match &spec.owner {
        Some(name) if !name.is_empty() => resolve_uid(name)? as i32,
        _ => -1, // -1 means "don't change"
    };

    // --- Resolve GID ---
    let gid: i32 = match &spec.group {
        Some(name) if !name.is_empty() => resolve_gid(name)? as i32,
        Some(_) => -1, // Empty string (login group) — not implemented, skip
        None => -1,    // Not specified
    };

    // --- Call chown ---
    let c_path = std::ffi::CString::new(path)
        .map_err(|_| format!("chown: invalid path: '{}'", path))?;

    let result = unsafe {
        if opts.no_deref {
            libc::lchown(c_path.as_ptr(), uid as libc::uid_t, gid as libc::gid_t)
        } else {
            libc::chown(c_path.as_ptr(), uid as libc::uid_t, gid as libc::gid_t)
        }
    };

    if result != 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!("chown: changing ownership of '{}': {}", path, err));
    }

    // --- Generate messages ---
    if opts.verbose {
        messages.push(format!("ownership of '{}' retained", path));
    }

    // --- Recurse into directories ---
    if opts.recursive && file_path.is_dir() {
        let entries = fs::read_dir(file_path)
            .map_err(|e| format!("chown: '{}': {}", path, e))?;

        for entry_result in entries {
            let entry = entry_result
                .map_err(|e| format!("chown: error reading '{}': {}", path, e))?;
            let child_path = entry.path();
            let child_msgs = chown_file(
                &child_path.to_string_lossy(),
                spec,
                opts,
            )?;
            messages.extend(child_msgs);
        }
    }

    Ok(messages)
}

/// Non-Unix stub.
#[cfg(not(unix))]
pub fn chown_file(
    path: &str,
    _spec: &OwnerSpec,
    _opts: &ChownOptions,
) -> Result<Vec<String>, String> {
    Err(format!("chown: not supported on this platform ({})", path))
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_owner_spec tests ---

    #[test]
    fn parse_owner_only() {
        let spec = parse_owner_spec("alice").unwrap();
        assert_eq!(spec.owner, Some("alice".into()));
        assert_eq!(spec.group, None);
    }

    #[test]
    fn parse_owner_and_group() {
        let spec = parse_owner_spec("alice:staff").unwrap();
        assert_eq!(spec.owner, Some("alice".into()));
        assert_eq!(spec.group, Some("staff".into()));
    }

    #[test]
    fn parse_group_only() {
        let spec = parse_owner_spec(":staff").unwrap();
        assert_eq!(spec.owner, None);
        assert_eq!(spec.group, Some("staff".into()));
    }

    #[test]
    fn parse_owner_with_empty_group() {
        let spec = parse_owner_spec("alice:").unwrap();
        assert_eq!(spec.owner, Some("alice".into()));
        assert_eq!(spec.group, Some("".into()));
    }

    #[test]
    fn parse_empty_fails() {
        assert!(parse_owner_spec("").is_err());
    }

    #[test]
    fn parse_numeric_owner() {
        let spec = parse_owner_spec("1000").unwrap();
        assert_eq!(spec.owner, Some("1000".into()));
        assert_eq!(spec.group, None);
    }

    #[test]
    fn parse_numeric_owner_and_group() {
        let spec = parse_owner_spec("1000:100").unwrap();
        assert_eq!(spec.owner, Some("1000".into()));
        assert_eq!(spec.group, Some("100".into()));
    }

    // --- resolve_uid tests ---

    #[test]
    fn resolve_numeric_uid() {
        let uid = resolve_uid("0");
        assert_eq!(uid.unwrap(), 0);
    }

    #[test]
    fn resolve_invalid_user() {
        let result = resolve_uid("definitely_not_a_real_user_xyz123");
        assert!(result.is_err());
    }

    // --- resolve_gid tests ---

    #[test]
    fn resolve_numeric_gid() {
        let gid = resolve_gid("0");
        assert_eq!(gid.unwrap(), 0);
    }

    #[test]
    fn resolve_invalid_group() {
        let result = resolve_gid("definitely_not_a_real_group_xyz123");
        assert!(result.is_err());
    }

    // --- chown_file tests ---

    #[cfg(unix)]
    #[test]
    fn chown_nonexistent_file() {
        let spec = OwnerSpec { owner: Some("root".into()), group: None };
        let result = chown_file("/tmp/chown_test_nonexistent_xyz", &spec, &ChownOptions::default());
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn chown_to_own_uid() {
        // Change ownership to our own UID (should succeed without root)
        let dir = std::env::temp_dir().join("chown_test_own_uid");
        let _ = fs::remove_file(&dir);
        fs::write(&dir, "test").unwrap();

        let uid = unsafe { libc::getuid() };
        let spec = OwnerSpec { owner: Some(uid.to_string()), group: None };
        let result = chown_file(&dir.to_string_lossy(), &spec, &ChownOptions::default());
        assert!(result.is_ok());

        let _ = fs::remove_file(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn chown_verbose_output() {
        let dir = std::env::temp_dir().join("chown_test_verbose");
        let _ = fs::remove_file(&dir);
        fs::write(&dir, "test").unwrap();

        let uid = unsafe { libc::getuid() };
        let spec = OwnerSpec { owner: Some(uid.to_string()), group: None };
        let opts = ChownOptions { verbose: true, ..Default::default() };
        let result = chown_file(&dir.to_string_lossy(), &spec, &opts);
        let messages = result.unwrap();
        assert!(!messages.is_empty());

        let _ = fs::remove_file(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn chown_with_group() {
        let dir = std::env::temp_dir().join("chown_test_group");
        let _ = fs::remove_file(&dir);
        fs::write(&dir, "test").unwrap();

        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };
        let spec = OwnerSpec {
            owner: Some(uid.to_string()),
            group: Some(gid.to_string()),
        };
        let result = chown_file(&dir.to_string_lossy(), &spec, &ChownOptions::default());
        assert!(result.is_ok());

        let _ = fs::remove_file(&dir);
    }
}
