//! Platform-specific checks shared by native authority boundaries.

use std::{fs::File, path::Path};

/// Return whether the open object has no macOS ACL entry granting mutating
/// authority. Deny-only ACLs (including macOS' common `everyone deny delete`
/// home-directory entry) do not weaken the POSIX owner/mode checks.
#[cfg(target_os = "macos")]
pub(crate) fn has_no_mutating_acl(file: &File) -> bool {
    use std::{ffi::c_void, os::fd::AsRawFd, ptr};

    type Acl = *mut c_void;
    type AclEntry = *mut c_void;
    type AclPermset = *mut c_void;

    unsafe extern "C" {
        fn acl_get_fd_np(fd: libc::c_int, acl_type: libc::c_int) -> Acl;
        fn acl_get_entry(acl: Acl, entry_id: libc::c_int, entry: *mut AclEntry) -> libc::c_int;
        fn acl_get_tag_type(entry: AclEntry, tag: *mut libc::c_int) -> libc::c_int;
        fn acl_get_permset(entry: AclEntry, permset: *mut AclPermset) -> libc::c_int;
        fn acl_get_perm_np(permset: AclPermset, permission: libc::c_int) -> libc::c_int;
        fn acl_free(object: *mut c_void) -> libc::c_int;
    }

    const ACL_TYPE_EXTENDED: libc::c_int = 0x0000_0100;
    const ACL_FIRST_ENTRY: libc::c_int = 0;
    const ACL_NEXT_ENTRY: libc::c_int = -1;
    const ACL_EXTENDED_ALLOW: libc::c_int = 1;
    const MUTATING_PERMISSIONS: [libc::c_int; 8] = [
        1 << 2,
        1 << 4,
        1 << 5,
        1 << 6,
        1 << 8,
        1 << 10,
        1 << 12,
        1 << 13,
    ];

    let acl = unsafe { acl_get_fd_np(file.as_raw_fd(), ACL_TYPE_EXTENDED) };
    if acl.is_null() {
        return std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT);
    }
    let mut safe = true;
    let mut entry: AclEntry = ptr::null_mut();
    let mut entry_id = ACL_FIRST_ENTRY;
    loop {
        let result = unsafe { acl_get_entry(acl, entry_id, &mut entry) };
        if result != 0 {
            safe = std::io::Error::last_os_error().raw_os_error() == Some(libc::EINVAL);
            break;
        }
        let mut tag = 0;
        let mut permissions: AclPermset = ptr::null_mut();
        if unsafe { acl_get_tag_type(entry, &mut tag) } != 0
            || unsafe { acl_get_permset(entry, &mut permissions) } != 0
        {
            safe = false;
            break;
        }
        if tag == ACL_EXTENDED_ALLOW {
            for permission in MUTATING_PERMISSIONS {
                let present = unsafe { acl_get_perm_np(permissions, permission) };
                if present != 0 {
                    safe = false;
                    break;
                }
            }
            if !safe {
                break;
            }
        }
        entry_id = ACL_NEXT_ENTRY;
    }
    if unsafe { acl_free(acl) } != 0 {
        return false;
    }
    safe
}

#[cfg(target_os = "macos")]
pub(crate) fn ancestors_have_no_mutating_acl(path: &Path) -> bool {
    use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt};

    let canonical = match std::fs::canonicalize(path) {
        Ok(path) => path,
        Err(_) => return false,
    };
    canonical.ancestors().all(|ancestor| {
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(ancestor)
            .is_ok_and(|directory| has_no_mutating_acl(&directory))
    })
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn has_no_mutating_acl(_file: &File) -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn ancestors_have_no_mutating_acl(_path: &Path) -> bool {
    true
}
