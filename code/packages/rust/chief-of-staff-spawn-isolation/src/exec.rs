//! # A verified executable (D18S S-K1; P2.6d-2a, P2.6d-3)
//!
//! A broker is launched by the supervisor from a binary pinned by its
//! SHA-256. Checking a path's hash and then `exec`ing the path is a race:
//! the file can be replaced in between. So the binary is opened once,
//! hashed *through that descriptor*, and that descriptor is what is
//! executed.
//!
//! [`VerifiedExecutable`] is the open, hashed file. It does not launch
//! anything itself: `chief-of-staff-linux-sandbox`'s
//! `LinuxConfinement::prepare_verified` takes it, re-verifies it, and
//! execs a duplicate of its descriptor under the broker's confinement
//! (`execveat(fd, "", AT_EMPTY_PATH)`), with the keys on 3..3+n.
//!
//! macOS has no `execveat`, so there a binary is executed by path
//! (`chief-of-staff-macos-sandbox`, D18S step 7). That confinement checks
//! that the path still names the inode of this descriptor, before the
//! spawn and again in its shim just before the exec.
//!
//! Windows has no exec of a handle either: `CreateProcessW` takes a path
//! (`chief-of-staff-windows-sandbox`, D18S step 8). So on Windows the file
//! is held open **sharing read only**: while it is held, no one can write,
//! rename or delete it, so the bytes at its path are the bytes hashed. The
//! confinement checks that the path is exactly this handle's final path,
//! and that the child's image is that same name, before the child runs.
//!
//! ```text
//!                 opened with                       checked each verify
//!   Linux, macOS  O_RDONLY                          regular, not group- or
//!                                                   world-writable, owned by
//!                                                   root or this user, digest
//!   Windows       read, share read only, the last   regular, not a reparse
//!                 reparse point not followed        point, owned by this
//!                                                   user, Administrators,
//!                                                   SYSTEM or TrustedInstaller,
//!                                                   no one else may write it
//!                                                   (by its access list),
//!                                                   digest
//! ```
//!
//! The share mode alone is not enough on Windows: it keeps out handles
//! opened while this one is held, but not a writable mapping someone made
//! earlier and kept after closing their handle. So no one but those four
//! may hold any write-class right on the file at all.
//!
//! (P2.6d-2a had a second launcher here, `isolate_and_exec`, which placed
//! the descriptors and exec'd without a sandbox. P2.6d-3 replaced it: there
//! is one way to start a broker, and it is confined.)

use std::fs::File;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::{FileExt, MetadataExt};
#[cfg(windows)]
use std::os::windows::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use coding_adventures_sha256::Sha256Hasher;

/// The largest binary [`VerifiedExecutable`] will hash.
pub const MAX_EXECUTABLE_BYTES: u64 = 128 * 1024 * 1024;

/// Why an executable was not accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyError {
    /// The path is not absolute (S-K1: no search, no working directory).
    NotAbsolute,
    /// It could not be opened, inspected or read.
    Unreadable,
    /// Not a regular file.
    NotRegular,
    /// Group- or world-writable: someone else could change it in place.
    /// (Unix only.)
    Writable,
    /// Owned by neither root nor this process's user. (Unix only.)
    Owner,
    /// Larger than [`MAX_EXECUTABLE_BYTES`].
    TooLarge,
    /// Its size changed while it was being hashed.
    Changed,
    /// Its SHA-256 is not the pinned one.
    DigestMismatch,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotAbsolute => "executable path is not absolute",
            Self::Unreadable => "executable could not be read",
            Self::NotRegular => "executable is not a regular file",
            Self::Writable => "executable is writable by others",
            Self::Owner => "executable is owned by someone else",
            Self::TooLarge => "executable is too large to verify",
            Self::Changed => "executable changed while it was verified",
            Self::DigestMismatch => "executable does not match its pinned digest",
        })
    }
}

impl std::error::Error for VerifyError {}

/// An executable, open, whose contents matched a pinned SHA-256 when it was
/// opened and match it again before every launch.
///
/// The only way to get one is [`VerifiedExecutable::open`]. To run one, the
/// confinement re-verifies it and then executes a duplicate of the very
/// descriptor it hashed ([`VerifiedExecutable::descriptor`]).
#[derive(Debug)]
pub struct VerifiedExecutable {
    file: File,
    path: PathBuf,
    digest: [u8; 32],
}

impl VerifiedExecutable {
    /// Open `path` and check it against `expected` SHA-256.
    pub fn open(path: &Path, expected: [u8; 32]) -> Result<Self, VerifyError> {
        if !path.is_absolute() {
            return Err(VerifyError::NotAbsolute);
        }
        let file = open_for_hashing(path).map_err(|_| VerifyError::Unreadable)?;
        let verified = Self {
            file,
            path: path.to_path_buf(),
            digest: expected,
        };
        verified.verify()?;
        Ok(verified)
    }

    /// The path it was opened from. On Linux it is for messages only and
    /// is never executed. On macOS it is what runs, after the confinement
    /// has checked that it still names this descriptor's inode. On Windows
    /// it is what runs too, while this handle keeps the file from being
    /// changed, renamed or deleted.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The SHA-256 its bytes were pinned to, and matched when it was opened.
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }

    /// A duplicate of the descriptor that was hashed: same open file, so
    /// the same bytes. Execute this, never [`Self::path`].
    pub fn descriptor(&self) -> io::Result<File> {
        self.file.try_clone()
    }

    /// Hash it again through its descriptor.
    pub fn verify(&self) -> Result<(), VerifyError> {
        let before = self.file.metadata().map_err(|_| VerifyError::Unreadable)?;
        if !before.file_type().is_file() {
            return Err(VerifyError::NotRegular);
        }
        check_owner_and_mode(&self.file, &before)?;
        if before.len() > MAX_EXECUTABLE_BYTES {
            return Err(VerifyError::TooLarge);
        }
        let mut hasher = Sha256Hasher::new();
        let mut buffer = vec![0u8; 64 * 1024];
        let mut offset = 0u64;
        loop {
            let read = match read_at(&self.file, &mut buffer, offset) {
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(VerifyError::Unreadable),
            };
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            offset += read as u64;
            if offset > MAX_EXECUTABLE_BYTES {
                return Err(VerifyError::TooLarge);
            }
        }
        let after = self.file.metadata().map_err(|_| VerifyError::Unreadable)?;
        if offset != before.len() || after.len() != before.len() {
            return Err(VerifyError::Changed);
        }
        if hasher.digest() != self.digest {
            return Err(VerifyError::DigestMismatch);
        }
        Ok(())
    }
}

/// Open a file an agent will be granted to read, under the same owner and
/// access-list rule as an executable, without a digest (Windows; D18S
/// P2.8c).
///
/// ```text
///   opened with                           refused when
///   read; share read and write, never     not absolute, not a regular file,
///   delete; the last reparse point        a reparse point, owned by someone
///   not followed                          untrusted, or writable (by its
///                                         access list) by anyone untrusted
/// ```
///
/// While the handle is held, no one can rename or delete the file, so its
/// path keeps naming it. Its owner may still write it. The caller checks
/// that the path is the handle's final path.
#[cfg(windows)]
pub fn open_trusted_file(path: &Path) -> Result<File, VerifyError> {
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    if !path.is_absolute() {
        return Err(VerifyError::NotAbsolute);
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|_| VerifyError::Unreadable)?;
    check_trusted_file(&file)?;
    Ok(file)
}

/// The check [`open_trusted_file`] makes, on a file already open: a regular
/// file, not a reparse point, owned and writable only by the trusted. Run
/// it again after changing the file's access list. Writing the list back
/// re-applies its directory's inheritable entries, which no earlier check
/// saw (D18S P2.8c, review).
#[cfg(windows)]
pub fn check_trusted_file(file: &File) -> Result<(), VerifyError> {
    let metadata = file.metadata().map_err(|_| VerifyError::Unreadable)?;
    if !metadata.file_type().is_file() {
        return Err(VerifyError::NotRegular);
    }
    check_owner_and_mode(file, &metadata)
}

/// Unix: `O_RDONLY`, never `O_PATH`, which `execveat` would accept but
/// which cannot be read to hash (S-I4d).
#[cfg(unix)]
fn open_for_hashing(path: &Path) -> io::Result<File> {
    File::open(path)
}

/// Windows: sharing read only, so while this handle is held nothing can
/// write, rename or delete the file, and with the last component's reparse
/// point not followed, so a link is opened as itself and then refused.
#[cfg(windows)]
fn open_for_hashing(path: &Path) -> io::Result<File> {
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

/// Unix: no one but root or this user may own it or write to it.
#[cfg(unix)]
fn check_owner_and_mode(_file: &File, metadata: &std::fs::Metadata) -> Result<(), VerifyError> {
    if metadata.mode() & 0o022 != 0 {
        return Err(VerifyError::Writable);
    }
    // SAFETY: geteuid has no preconditions and touches no memory.
    let me = unsafe { libc::geteuid() };
    if metadata.uid() != 0 && metadata.uid() != me {
        return Err(VerifyError::Owner);
    }
    Ok(())
}

/// Windows: not a reparse point (a symlink or junction opened as itself),
/// and owned and writable only by this user and the system's own
/// principals ([`windows_access::check`]).
#[cfg(windows)]
fn check_owner_and_mode(file: &File, metadata: &std::fs::Metadata) -> Result<(), VerifyError> {
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(VerifyError::NotRegular);
    }
    windows_access::check(file)
}

#[cfg(unix)]
fn read_at(file: &File, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
    file.read_at(buffer, offset)
}

/// `seek_read` moves the handle's file pointer as well; nothing here uses
/// it.
#[cfg(windows)]
fn read_at(file: &File, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
    file.seek_read(buffer, offset)
}

/// The Windows owner and access-list rule, the counterpart of Unix's
/// owner and mode bits.
///
/// ```text
///   trusted: this process's user, BUILTIN\Administrators (S-1-5-32-544),
///            SYSTEM (S-1-5-18), TrustedInstaller (S-1-5-80-956008885-...)
///
///   owner        must be trusted                       else Owner
///   no DACL      everyone has every right              Writable
///   allow ACE    a write-class right to anyone else    Writable
///   allow ACE    of a type this does not read          Writable
///   deny ACE     only takes rights away                ignored
///   inherit-only applies to children, not this file    ignored
/// ```
///
/// Write-class rights: write data, append data, delete, change the access
/// list, change the owner, and the generic write and all rights.
#[cfg(windows)]
mod windows_access {
    use super::VerifyError;
    use std::fs::File;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{LocalFree, HANDLE, PSID};
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSidToSidW, GetSecurityInfo, SE_FILE_OBJECT,
    };
    use windows_sys::Win32::Security::{
        AclSizeInformation, CreateWellKnownSid, EqualSid, GetAce, GetAclInformation,
        GetTokenInformation, TokenUser, WinBuiltinAdministratorsSid, WinLocalSystemSid,
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_SIZE_INFORMATION, DACL_SECURITY_INFORMATION,
        INHERIT_ONLY_ACE, OWNER_SECURITY_INFORMATION, TOKEN_USER,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    const TRUSTED_INSTALLER: &str =
        "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464";
    const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
    const ACCESS_DENIED_ACE_TYPE: u8 = 1;
    /// FILE_WRITE_DATA | FILE_APPEND_DATA | DELETE | WRITE_DAC | WRITE_OWNER
    /// | GENERIC_ALL | GENERIC_WRITE.
    const WRITE_CLASS: u32 = 0x0000_0002
        | 0x0000_0004
        | 0x0001_0000
        | 0x0004_0000
        | 0x0008_0000
        | 0x1000_0000
        | 0x4000_0000;
    const TOKEN_QUERY: u32 = 0x0008;

    pub fn check(file: &File) -> Result<(), VerifyError> {
        let trusted = Trusted::new().ok_or(VerifyError::Unreadable)?;
        let mut owner: PSID = std::ptr::null_mut();
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut descriptor: *mut core::ffi::c_void = std::ptr::null_mut();
        // SAFETY: the handle is open for reading, which includes
        // READ_CONTROL; the owner and DACL point into the descriptor, which
        // is freed with LocalFree below and not used after.
        let result = unsafe {
            GetSecurityInfo(
                file.as_raw_handle() as HANDLE,
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                std::ptr::null_mut(),
                &mut dacl,
                std::ptr::null_mut(),
                &mut descriptor,
            )
        };
        if result != 0 {
            return Err(VerifyError::Unreadable);
        }
        let verdict = if owner.is_null() || !trusted.contains(owner) {
            Err(VerifyError::Owner)
        } else if dacl.is_null() {
            Err(VerifyError::Writable)
        } else {
            // SAFETY: a valid DACL from the descriptor above.
            unsafe { check_entries(dacl, &trusted) }
        };
        // SAFETY: allocated by GetSecurityInfo, freed once.
        unsafe { LocalFree(descriptor as _) };
        verdict
    }

    /// # Safety
    /// `dacl` is a valid ACL.
    unsafe fn check_entries(dacl: *mut ACL, trusted: &Trusted) -> Result<(), VerifyError> {
        // SAFETY: plain data, filled by GetAclInformation.
        let mut size: ACL_SIZE_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: a valid ACL and a buffer of the size the class needs.
        if unsafe {
            GetAclInformation(
                dacl,
                &mut size as *mut _ as *mut core::ffi::c_void,
                std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation,
            )
        } == 0
        {
            return Err(VerifyError::Unreadable);
        }
        for index in 0..size.AceCount {
            let mut entry: *mut core::ffi::c_void = std::ptr::null_mut();
            // SAFETY: the index is below the count; the pointer is into the
            // ACL.
            if unsafe { GetAce(dacl, index, &mut entry) } == 0 {
                return Err(VerifyError::Unreadable);
            }
            // SAFETY: every ACE starts with an ACE_HEADER.
            let header = unsafe { &*(entry as *const ACE_HEADER) };
            if u32::from(header.AceFlags) & INHERIT_ONLY_ACE != 0
                || header.AceType == ACCESS_DENIED_ACE_TYPE
            {
                continue;
            }
            if header.AceType != ACCESS_ALLOWED_ACE_TYPE {
                // An allow of a kind this does not read (callback, object):
                // refused rather than guessed at.
                return Err(VerifyError::Writable);
            }
            let allowed = entry as *const ACCESS_ALLOWED_ACE;
            // SAFETY: an ACCESS_ALLOWED_ACE: a mask, then the SID in place.
            if unsafe { (*allowed).Mask } & WRITE_CLASS == 0 {
                continue;
            }
            // From the entry pointer itself, not a reference to the fixed
            // part: EqualSid reads the whole SID, past the struct's end.
            // SAFETY: the SID starts at SidStart, inside the entry.
            let sid = unsafe { std::ptr::addr_of!((*allowed).SidStart) } as PSID;
            if !trusted.contains(sid) {
                return Err(VerifyError::Writable);
            }
        }
        Ok(())
    }

    /// The four trusted principals' SIDs, owned.
    struct Trusted {
        user: Vec<u64>,
        administrators: Vec<u64>,
        system: Vec<u64>,
        installer: PSID,
    }

    impl Trusted {
        fn new() -> Option<Self> {
            Some(Self {
                user: current_user()?,
                administrators: well_known(WinBuiltinAdministratorsSid)?,
                system: well_known(WinLocalSystemSid)?,
                installer: {
                    let text: Vec<u16> = TRUSTED_INSTALLER
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect();
                    let mut sid: PSID = std::ptr::null_mut();
                    // SAFETY: a terminated string; the SID is LocalAlloc'd
                    // and freed in Drop.
                    if unsafe { ConvertStringSidToSidW(text.as_ptr(), &mut sid) } == 0 {
                        return None;
                    }
                    sid
                },
            })
        }

        fn contains(&self, sid: PSID) -> bool {
            let user = self.user_sid();
            [
                user,
                self.administrators.as_ptr() as PSID,
                self.system.as_ptr() as PSID,
                self.installer,
            ]
            .iter()
            // SAFETY: every SID compared is valid for this call.
            .any(|&known| unsafe { EqualSid(known, sid) } != 0)
        }

        fn user_sid(&self) -> PSID {
            // SAFETY: the buffer holds a TOKEN_USER whose SID points into
            // the same buffer.
            unsafe { (*(self.user.as_ptr() as *const TOKEN_USER)).User.Sid }
        }
    }

    impl Drop for Trusted {
        fn drop(&mut self) {
            // SAFETY: allocated by ConvertStringSidToSidW, freed once.
            unsafe { LocalFree(self.installer as _) };
        }
    }

    /// This process's user, as a TOKEN_USER in a u64-aligned buffer.
    fn current_user() -> Option<Vec<u64>> {
        let mut token: HANDLE = 0;
        // SAFETY: this process's own token, closed below.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return None;
        }
        let mut buffer = vec![0u64; 64];
        let mut written = 0u32;
        // SAFETY: a 512-byte buffer, ample for a TOKEN_USER and its SID.
        let ok = unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                buffer.as_mut_ptr() as *mut core::ffi::c_void,
                512,
                &mut written,
            )
        } != 0;
        // SAFETY: the token opened above, closed once.
        unsafe { windows_sys::Win32::Foundation::CloseHandle(token) };
        ok.then_some(buffer)
    }

    fn well_known(kind: i32) -> Option<Vec<u64>> {
        let mut buffer = vec![0u64; 9]; // SECURITY_MAX_SID_SIZE is 68 bytes
        let mut size = 72u32;
        // SAFETY: a buffer of `size` bytes; no domain SID is needed.
        if unsafe {
            CreateWellKnownSid(
                kind,
                std::ptr::null_mut(),
                buffer.as_mut_ptr() as PSID,
                &mut size,
            )
        } == 0
        {
            return None;
        }
        Some(buffer)
    }
}
