//! Descriptor-bound Linux/macOS mode policy, with explicit ACL rejection.
//!
//! chmod is not an ACL-copy operation. An extended access/default ACL can gain
//! effective grants when mode bits change, and changing group can clear set-ID
//! bits. Reject unsupported ACLs, apply ownership before final mode, and verify
//! the resulting policy. Other Unix targets remain unsupported, not presumed
//! equivalent. Separate empty probes validate capability before originals move.
use super::invalid;
use std::fs::{File, Permissions};
use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Policy {
    uid: u32,
    gid: u32,
    mode: u32,
}

impl Policy {
    pub(super) fn capture(file: &File) -> io::Result<Self> {
        let metadata = file.metadata()?;
        reject_acl(file, metadata.is_dir())?;
        Ok(Self {
            uid: metadata.uid(),
            gid: metadata.gid(),
            mode: metadata.mode() & 0o7777,
        })
    }

    pub(super) fn check_assignable_owner(&self) -> io::Result<()> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            unsafe extern "C" {
                fn geteuid() -> u32;
            }
            // SAFETY: this native getter takes no pointers and transfers no resources.
            if self.uid != unsafe { geteuid() } {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "unsupported foreign output owner",
                ));
            }
            Ok(())
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(unsupported(
                "output ownership requires verified Linux or macOS support",
            ))
        }
    }

    pub(super) fn apply(&self, file: &File) -> io::Result<()> {
        self.check_assignable_owner()?;
        reject_acl(file, false)?;
        let current = file.metadata()?;
        if current.uid() != self.uid || current.gid() != self.gid {
            assign_owner(file, self.uid, self.gid)?;
        }
        // fchown can clear set-ID bits. Restoring mode must therefore follow it.
        file.set_permissions(Permissions::from_mode(self.mode))?;
        if Self::capture(file)? != *self {
            return Err(invalid(
                "installed Unix owner/group/mode differs from intended policy",
            ));
        }
        Ok(())
    }

    pub(super) fn restore_privacy(&self, file: &File) -> io::Result<()> {
        // A setgid parent can assign a creation group the caller cannot later
        // chown back to. Privacy needs owner-only mode, not that old group.
        // Do not let a failed group reset leave final reader grants active.
        self.check_assignable_owner()?;
        let current = Self::capture(file)?;
        if current.uid != self.uid || self.mode & 0o077 != 0 {
            return Err(invalid("cannot restore owner-only candidate privacy"));
        }
        file.set_permissions(Permissions::from_mode(self.mode))?;
        let private = Self::capture(file)?;
        if private.uid != self.uid || private.mode != self.mode {
            return Err(invalid(
                "restored Unix candidate privacy differs from intended mode",
            ));
        }
        Ok(())
    }
}

fn unsupported(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, message)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn assign_owner(file: &File, uid: u32, gid: u32) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn fchown(fd: i32, uid: u32, gid: u32) -> i32;
    }
    // SAFETY: Linux/macOS use 32-bit uid_t/gid_t and a borrowed live descriptor.
    // fchown synchronously changes this held inode, never a pathname replacement.
    if unsafe { fchown(file.as_raw_fd(), uid, gid) } == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn assign_owner(_file: &File, _uid: u32, _gid: u32) -> io::Result<()> {
    Err(unsupported(
        "output ownership requires verified Linux or macOS support",
    ))
}

#[cfg(target_os = "linux")]
fn reject_acl(file: &File, directory: bool) -> io::Result<()> {
    use std::ffi::{c_char, c_void};
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn fgetxattr(fd: i32, name: *const c_char, value: *mut c_void, size: usize) -> isize;
    }
    // Only presence is needed for copy-or-reject. No caller-owned buffer grows,
    // even for an oversized ACL. Errors other than definite attribute absence
    // fail closed, including unsupported queries and access denied.
    // https://man7.org/linux/man-pages/man2/fgetxattr.2.html
    for name in [c"system.posix_acl_access", c"system.posix_acl_default"] {
        if !directory && name == c"system.posix_acl_default" {
            continue;
        }
        // SAFETY: live borrowed descriptor, terminated constant name, null
        // zero-length size query; the OS neither writes nor retains a buffer.
        let size = unsafe { fgetxattr(file.as_raw_fd(), name.as_ptr(), std::ptr::null_mut(), 0) };
        if size >= 0 {
            return Err(unsupported(
                "extended/default Linux ACL output policy is unsupported",
            ));
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(61) {
            return Err(error);
        } // ENODATA
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn reject_acl(file: &File, _directory: bool) -> io::Result<()> {
    use std::ffi::c_void;
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn acl_get_fd_np(fd: i32, kind: i32) -> *mut c_void;
        fn acl_get_entry(acl: *mut c_void, kind: i32, entry: *mut *mut c_void) -> i32;
        fn acl_get_flagset_np(acl: *mut c_void, flags: *mut *mut c_void) -> i32;
        fn acl_get_flag_np(flags: *mut c_void, flag: i32) -> i32;
        fn acl_valid(acl: *mut c_void) -> i32;
        fn acl_free(acl: *mut c_void) -> i32;
    }
    struct Acl(*mut c_void);
    impl Drop for Acl {
        fn drop(&mut self) {
            // SAFETY: sole owned successful native ACL allocation; matching
            // SDK destructor frees it once. Borrowed entry/flag pointers are
            // never freed separately. Apple bounds native ACLs to128 entries.
            unsafe {
                acl_free(self.0);
            }
        }
    }
    // SAFETY: live descriptor and documented ACL_TYPE_EXTENDED=0x100. The SDK
    // returns an owned ACL (bounded native allocation) or reports failure.
    let pointer = unsafe { acl_get_fd_np(file.as_raw_fd(), 0x100) };
    if pointer.is_null() {
        let error = io::Error::last_os_error();
        // Apple's filesec_get_property reports ENOENT when no ACL is present.
        // https://github.com/apple-oss-distributions/Libc/blob/main/gen/filesec.c
        return if error.raw_os_error() == Some(2) {
            Ok(())
        } else {
            Err(error)
        };
    }
    if pointer as usize <= 16 || pointer as usize >= usize::MAX - 16 {
        return Err(unsupported("unsupported native ACL sentinel"));
    }
    let acl = Acl(pointer);
    let (mut entry, mut flags) = (std::ptr::null_mut(), std::ptr::null_mut());
    // SAFETY: owned successful SDK ACL and correctly typed writable outputs;
    // returned component pointers remain borrowed only while this ACL lives.
    unsafe {
        if acl_valid(acl.0) != 0 {
            return Err(io::Error::last_os_error());
        }
        if acl_get_flagset_np(acl.0, &mut flags) != 0 {
            return Err(io::Error::last_os_error());
        }
        if flags.is_null() {
            return Err(invalid("native ACL flags returned null"));
        }
        let no_inherit = acl_get_flag_np(flags, 1 << 17);
        if no_inherit == -1 {
            return Err(io::Error::last_os_error());
        }
        if no_inherit != 0 {
            return Err(unsupported(
                "extended macOS ACL control policy is unsupported",
            ));
        }
        if acl_get_entry(acl.0, 0, &mut entry) == 0 {
            return Err(unsupported(
                "extended/inherited macOS ACL output policy is unsupported",
            ));
        }
    }
    // Unlike Linux libacl, Darwin returns0 for a found entry and-1/EINVAL
    // for no first entry. acl_valid above establishes that the ACL is valid.
    // https://github.com/apple-oss-distributions/Libc/blob/main/posix1e/acl_entry.c
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(22) {
        Ok(())
    } else {
        Err(error)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn reject_acl(_file: &File, _directory: bool) -> io::Result<()> {
    Err(unsupported(
        "output ACL verification requires verified Linux or macOS support",
    ))
}
