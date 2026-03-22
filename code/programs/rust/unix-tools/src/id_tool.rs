//! # id — Print Real and Effective User and Group IDs
//!
//! This module implements the business logic for the `id` command.
//! `id` displays the user and group information for the current user
//! (or a specified user).
//!
//! ## Default Output
//!
//! With no flags, `id` prints all identity information:
//!
//! ```text
//!     $ id
//!     uid=501(alice) gid=20(staff) groups=20(staff),12(everyone),61(localaccounts)
//! ```
//!
//! ## Individual Fields
//!
//! ```text
//!     Flag    Output
//!     ─────   ──────────────────────────
//!     -u      Effective user ID only
//!     -g      Effective group ID only
//!     -G      All group IDs
//!     -n      Print names instead of numbers (with -u, -g, or -G)
//!     -r      Print real ID instead of effective ID
//! ```
//!
//! ## Implementation Strategy
//!
//! We use libc functions to get the raw numeric IDs:
//! - `getuid()` / `geteuid()` for user IDs
//! - `getgid()` / `getegid()` for group IDs
//! - `getgroups()` for supplementary groups
//! - `getpwuid()` to look up user names from UIDs
//! - `getgrgid()` to look up group names from GIDs

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Container for user identity information.
#[derive(Debug, Clone)]
pub struct IdInfo {
    /// Real user ID.
    pub uid: u32,
    /// Effective user ID.
    pub euid: u32,
    /// Real group ID.
    pub gid: u32,
    /// Effective group ID.
    pub egid: u32,
    /// User name (looked up from effective UID).
    pub username: String,
    /// Group name (looked up from effective GID).
    pub groupname: String,
    /// All supplementary group IDs.
    pub groups: Vec<u32>,
    /// Group names for supplementary groups.
    pub group_names: Vec<String>,
}

/// Retrieve identity information for the current user.
///
/// This calls several libc functions to gather the complete picture:
///
/// ```text
///     getuid()    → real user ID
///     geteuid()   → effective user ID (may differ after sudo)
///     getgid()    → real group ID
///     getegid()   → effective group ID
///     getgroups() → supplementary group list
///     getpwuid()  → user name from UID
///     getgrgid()  → group name from GID
/// ```
pub fn get_user_info() -> Result<IdInfo, String> {
    unsafe {
        let uid = libc::getuid();
        let euid = libc::geteuid();
        let gid = libc::getgid();
        let egid = libc::getegid();

        // --- Look up user name ---
        let username = uid_to_name(euid).unwrap_or_else(|| euid.to_string());

        // --- Look up group name ---
        let groupname = gid_to_name(egid).unwrap_or_else(|| egid.to_string());

        // --- Get supplementary groups ---
        // First call with 0 to get the count, then allocate and call again.
        let ngroups = libc::getgroups(0, std::ptr::null_mut());
        let mut group_ids: Vec<libc::gid_t> = vec![0; ngroups as usize];
        let actual = libc::getgroups(ngroups, group_ids.as_mut_ptr());
        if actual < 0 {
            return Err("id: failed to get supplementary groups".to_string());
        }
        group_ids.truncate(actual as usize);

        // --- Look up group names ---
        let group_names: Vec<String> = group_ids
            .iter()
            .map(|&g| gid_to_name(g).unwrap_or_else(|| g.to_string()))
            .collect();

        Ok(IdInfo {
            uid,
            euid,
            gid,
            egid,
            username,
            groupname,
            groups: group_ids,
            group_names,
        })
    }
}

/// Format the full id output (default, no flags).
///
/// Produces the standard format:
/// ```text
///     uid=501(alice) gid=20(staff) groups=20(staff),12(everyone)
/// ```
pub fn format_id_full(info: &IdInfo) -> String {
    let mut result = format!(
        "uid={}({}) gid={}({})",
        info.euid, info.username, info.egid, info.groupname
    );

    if !info.groups.is_empty() {
        let groups_str: Vec<String> = info
            .groups
            .iter()
            .zip(info.group_names.iter())
            .map(|(id, name)| format!("{}({})", id, name))
            .collect();
        result.push_str(&format!(" groups={}", groups_str.join(",")));
    }

    result
}

/// Format a single user ID output (-u flag).
pub fn format_user_id(info: &IdInfo, show_name: bool, use_real: bool) -> String {
    let id = if use_real { info.uid } else { info.euid };
    if show_name {
        unsafe {
            uid_to_name(id).unwrap_or_else(|| id.to_string())
        }
    } else {
        id.to_string()
    }
}

/// Format a single group ID output (-g flag).
pub fn format_group_id(info: &IdInfo, show_name: bool, use_real: bool) -> String {
    let id = if use_real { info.gid } else { info.egid };
    if show_name {
        unsafe {
            gid_to_name(id).unwrap_or_else(|| id.to_string())
        }
    } else {
        id.to_string()
    }
}

/// Format all group IDs (-G flag).
pub fn format_groups(info: &IdInfo, show_name: bool) -> String {
    if show_name {
        info.group_names.join(" ")
    } else {
        info.groups
            .iter()
            .map(|g| g.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Convert a UID to a user name using `getpwuid()`.
///
/// Returns None if the UID doesn't map to a known user.
unsafe fn uid_to_name(uid: u32) -> Option<String> {
    let pw = libc::getpwuid(uid);
    if pw.is_null() {
        return None;
    }
    Some(
        std::ffi::CStr::from_ptr((*pw).pw_name)
            .to_string_lossy()
            .into_owned(),
    )
}

/// Convert a GID to a group name using `getgrgid()`.
///
/// Returns None if the GID doesn't map to a known group.
unsafe fn gid_to_name(gid: u32) -> Option<String> {
    let gr = libc::getgrgid(gid);
    if gr.is_null() {
        return None;
    }
    Some(
        std::ffi::CStr::from_ptr((*gr).gr_name)
            .to_string_lossy()
            .into_owned(),
    )
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_user_info_succeeds() {
        let info = get_user_info();
        assert!(info.is_ok(), "get_user_info should succeed");
    }

    #[test]
    fn username_is_nonempty() {
        let info = get_user_info().unwrap();
        assert!(!info.username.is_empty(), "username should not be empty");
    }

    #[test]
    fn groupname_is_nonempty() {
        let info = get_user_info().unwrap();
        assert!(!info.groupname.is_empty(), "groupname should not be empty");
    }

    #[test]
    fn has_at_least_one_group() {
        let info = get_user_info().unwrap();
        assert!(!info.groups.is_empty(), "should have at least one group");
    }

    #[test]
    fn format_full_contains_uid() {
        let info = get_user_info().unwrap();
        let output = format_id_full(&info);
        assert!(output.contains("uid="), "full output should contain uid=");
        assert!(output.contains("gid="), "full output should contain gid=");
    }

    #[test]
    fn format_user_id_numeric() {
        let info = get_user_info().unwrap();
        let output = format_user_id(&info, false, false);
        let parsed: Result<u32, _> = output.parse();
        assert!(parsed.is_ok(), "user id should be numeric, got: {}", output);
    }

    #[test]
    fn format_user_id_name() {
        let info = get_user_info().unwrap();
        let output = format_user_id(&info, true, false);
        assert!(!output.is_empty(), "user name should not be empty");
        assert_eq!(output, info.username);
    }

    #[test]
    fn format_group_id_numeric() {
        let info = get_user_info().unwrap();
        let output = format_group_id(&info, false, false);
        let parsed: Result<u32, _> = output.parse();
        assert!(parsed.is_ok(), "group id should be numeric, got: {}", output);
    }

    #[test]
    fn format_groups_has_entries() {
        let info = get_user_info().unwrap();
        let output = format_groups(&info, false);
        assert!(!output.is_empty(), "groups output should not be empty");
    }

    #[test]
    fn format_groups_names() {
        let info = get_user_info().unwrap();
        let output = format_groups(&info, true);
        assert!(!output.is_empty(), "group names should not be empty");
    }

    #[test]
    fn real_vs_effective_uid() {
        let info = get_user_info().unwrap();
        // In normal test environment, real and effective should match
        let real = format_user_id(&info, false, true);
        let effective = format_user_id(&info, false, false);
        assert_eq!(real, effective, "real and effective UID should match in tests");
    }
}
