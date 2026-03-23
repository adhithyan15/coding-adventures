//! # groups — Print the Groups a User Is In
//!
//! This module implements the business logic for the `groups` command.
//! `groups` prints the supplementary group memberships for the current
//! user (or a specified user).
//!
//! ## How It Works
//!
//! Every Unix user belongs to a primary group (stored in `/etc/passwd`)
//! and zero or more supplementary groups (stored in `/etc/group`).
//! The `groups` command lists all of them.
//!
//! ```text
//!     $ groups
//!     staff everyone localaccounts
//!
//!     $ groups root
//!     root : wheel daemon kmem sys tty
//! ```
//!
//! ## Implementation Strategy
//!
//! We use the same libc functions as `id -Gn`:
//! - `getgroups()` to get the list of supplementary group IDs
//! - `getgrgid()` to convert each GID to a group name
//!
//! For a specified user, we would need to look up their groups
//! from the group database, but for the current user we can use
//! the process's own group list.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Get the list of group names for the current user.
///
/// This calls `libc::getgroups()` to get the numeric GIDs of all
/// groups the current process belongs to, then looks up each GID
/// in the group database using `getgrgid()`.
///
/// # Returns
///
/// A Vec of group name strings, e.g., `["staff", "everyone", "admin"]`.
///
/// # How getgroups(2) Works
///
/// ```text
///     1. Call getgroups(0, NULL) → returns the count of groups
///     2. Allocate an array of that size
///     3. Call getgroups(count, array) → fills array with GIDs
///     4. Look up each GID → group name via getgrgid()
/// ```
#[cfg(unix)]
pub fn get_groups() -> Result<Vec<String>, String> {
    unsafe {
        // --- Step 1: Get the count ---
        let ngroups = libc::getgroups(0, std::ptr::null_mut());
        if ngroups < 0 {
            return Err("groups: failed to get group count".to_string());
        }

        // --- Step 2: Allocate and fill ---
        let mut gids: Vec<libc::gid_t> = vec![0; ngroups as usize];
        let actual = libc::getgroups(ngroups, gids.as_mut_ptr());
        if actual < 0 {
            return Err("groups: failed to get group list".to_string());
        }
        gids.truncate(actual as usize);

        // --- Step 3: Look up names ---
        let names: Vec<String> = gids
            .iter()
            .map(|&gid| {
                let gr = libc::getgrgid(gid);
                if gr.is_null() {
                    // If we can't find the name, fall back to the numeric GID.
                    // This can happen with deleted groups or NIS/LDAP issues.
                    gid.to_string()
                } else {
                    std::ffi::CStr::from_ptr((*gr).gr_name)
                        .to_string_lossy()
                        .into_owned()
                }
            })
            .collect();

        Ok(names)
    }
}

/// Non-Unix stub so the code compiles on all platforms.
#[cfg(not(unix))]
pub fn get_groups() -> Result<Vec<String>, String> {
    Err("groups: not supported on this platform".to_string())
}

/// Format group output as a space-separated string.
///
/// ```text
///     ["staff", "everyone", "admin"] → "staff everyone admin"
/// ```
pub fn format_groups(groups: &[String]) -> String {
    groups.join(" ")
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn get_groups_succeeds() {
        let result = get_groups();
        assert!(result.is_ok(), "get_groups should succeed");
    }

    #[cfg(unix)]
    #[test]
    fn has_at_least_one_group() {
        let groups = get_groups().unwrap();
        assert!(
            !groups.is_empty(),
            "current user should belong to at least one group"
        );
    }

    #[cfg(unix)]
    #[test]
    fn group_names_are_nonempty() {
        let groups = get_groups().unwrap();
        for name in &groups {
            assert!(!name.is_empty(), "group name should not be empty");
        }
    }

    #[test]
    fn format_groups_single() {
        let groups = vec!["staff".to_string()];
        assert_eq!(format_groups(&groups), "staff");
    }

    #[test]
    fn format_groups_multiple() {
        let groups = vec![
            "staff".to_string(),
            "everyone".to_string(),
            "admin".to_string(),
        ];
        assert_eq!(format_groups(&groups), "staff everyone admin");
    }

    #[test]
    fn format_groups_empty() {
        let groups: Vec<String> = vec![];
        assert_eq!(format_groups(&groups), "");
    }

    #[cfg(unix)]
    #[test]
    fn groups_are_consistent() {
        // Call get_groups() twice and verify both calls return the same
        // set of groups. We sort both lists before comparing because on
        // some CI environments (e.g., GitHub Actions Ubuntu runners),
        // the order from getgroups(2) can vary between calls.
        let mut first = get_groups().unwrap();
        let mut second = get_groups().unwrap();
        first.sort();
        second.sort();
        first.dedup();
        second.dedup();
        assert_eq!(first, second, "group list should be consistent");
    }
}
