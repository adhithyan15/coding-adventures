//! Windows discretionary file policy, distinct from Rust's readonly attribute.
//!
//! Every prepared object starts with an explicit protected current-user DACL.
//! Existing policy comes from a held handle. New-file policy is calculated by
//! Windows from the actual destination parent and creator token. Only after
//! installing a complete file do we expose its intended policy. This module
//! preserves owner/group/DACL/protection, not audit SACLs or integrity claims.
use super::invalid;
use std::ffi::c_void;
use std::fs::File;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;
use std::ptr::{null, null_mut};

const POLICY_INFORMATION: u32 = 7; // owner, primary group, DACL
const MAX_DESCRIPTOR_BYTES: usize = 131_072;
const MAX_TOKEN_BYTES: usize = 4096;
const FILE_ALL_ACCESS: u32 = 0x001f_01ff;
const SE_DACL_PROTECTED: u16 = 0x1000;
const READ_CONTROL: u32 = 0x0002_0000;
const FILE_GENERIC_ACCESS: u32 = 0xf000_0000;
const OWNER_RIGHTS: [u32; 3] = [0x0000_0101, 0x0300_0000, 4]; // S-1-3-4

/// Admission is distinct from effective access: an earlier allow or a token
/// privilege can satisfy a fresh open despite an explicitly unsupported denial.
/// Walk the bounded copied ACL without sorting or changing its ACEs. This is
/// deliberately not an implementation of Windows AccessCheck.
fn admit_dacl(acl: &[u32]) -> io::Result<()> {
    if acl.len() < 2
        || acl.len() > usize::from(u16::MAX) / 4
        || !matches!(acl[0] & 0xff, 2 | 4)
        || (acl[0] >> 16) as usize != acl.len() * 4
    {
        return Err(invalid("unsupported final ACL bounds"));
    }
    let mut cursor = 2;
    for _ in 0..(acl[1] & 0xffff) {
        let header = *acl
            .get(cursor)
            .ok_or_else(|| invalid("missing final ACE"))?;
        let bytes = (header >> 16) as usize;
        if bytes < 4 || !bytes.is_multiple_of(4) {
            return Err(invalid("unsupported final ACE size"));
        }
        let end = cursor
            .checked_add(bytes / 4)
            .ok_or_else(|| invalid("final ACE bounds overflow"))?;
        let ace = acl
            .get(cursor..end)
            .ok_or_else(|| invalid("truncated final ACE"))?;
        cursor = end;
        if header & 0x0800 != 0 {
            // INHERIT_ONLY_ACE: inert on this file
            continue;
        }
        match header & 0xff {
            // Ordinary, compound, object, callback and callback-object grants.
            // These are copied OS policy, not grants synthesized by this guard.
            0 | 4 | 5 | 9 | 11 => (),
            1 | 6 | 10 | 12 => admit_denial(ace)?,
            _ => return Err(invalid("unsupported effective final ACE encoding")),
        }
    }
    if cursor != acl.len() {
        return Err(invalid("unaccounted final ACL entries"));
    }
    Ok(())
}

fn admit_denial(ace: &[u32]) -> io::Result<()> {
    let kind = ace[0] & 0xff;
    let mask = *ace
        .get(1)
        .ok_or_else(|| invalid("missing denial access mask"))?;
    let sid_start = if matches!(kind, 6 | 12) {
        let flags = *ace
            .get(2)
            .ok_or_else(|| invalid("missing denial object flags"))?;
        if flags & !3 != 0 {
            return Err(invalid("unsupported denial object flags"));
        }
        // Each optional GUID occupies four DWORDs. Both absent puts SidStart
        // immediately after Flags, as specified by the native ACE layouts.
        3 + flags.count_ones() as usize * 4
    } else {
        2
    };
    let header = *ace
        .get(sid_start)
        .ok_or_else(|| invalid("missing denial SID"))?;
    let subauthorities = (header >> 8) & 0xff;
    if header & 0xff != 1 || subauthorities > 15 {
        return Err(invalid("unsupported denial SID"));
    }
    let sid_end = sid_start + 2 + subauthorities as usize;
    let sid = ace
        .get(sid_start..sid_end)
        .ok_or_else(|| invalid("truncated denial SID"))?;
    if matches!(kind, 1 | 6) && sid_end != ace.len() {
        return Err(invalid("unsupported trailing denial data"));
    }
    // All four file generic mappings include READ_CONTROL. Object filters and
    // callback conditions cannot turn this explicitly unsupported class into
    // a supported policy; conservatively reject it regardless of applicability.
    if sid == OWNER_RIGHTS && mask & (READ_CONTROL | FILE_GENERIC_ACCESS) != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "unsupported OWNER RIGHTS denial of verification READ_CONTROL",
        ));
    }
    Ok(())
}

#[repr(C)]
struct Attributes {
    size: u32,
    descriptor: *mut c_void,
    inherit: i32,
}
#[repr(C)]
struct GenericMapping {
    read: u32,
    write: u32,
    execute: u32,
    all: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut c_void;
    fn CreateDirectoryW(path: *const u16, attributes: *const Attributes) -> i32;
    fn CreateFileW(
        path: *const u16,
        access: u32,
        share: u32,
        attributes: *const Attributes,
        disposition: u32,
        flags: u32,
        template: *mut c_void,
    ) -> *mut c_void;
}
#[link(name = "advapi32")]
unsafe extern "system" {
    fn OpenProcessToken(process: *mut c_void, access: u32, token: *mut *mut c_void) -> i32;
    fn GetTokenInformation(
        token: *mut c_void,
        class: i32,
        buffer: *mut c_void,
        size: u32,
        needed: *mut u32,
    ) -> i32;
    fn GetKernelObjectSecurity(
        handle: *mut c_void,
        information: u32,
        descriptor: *mut c_void,
        size: u32,
        needed: *mut u32,
    ) -> i32;
    fn GetSecurityDescriptorOwner(
        descriptor: *mut c_void,
        sid: *mut *mut c_void,
        defaulted: *mut i32,
    ) -> i32;
    fn GetSecurityDescriptorGroup(
        descriptor: *mut c_void,
        sid: *mut *mut c_void,
        defaulted: *mut i32,
    ) -> i32;
    fn GetSecurityDescriptorDacl(
        descriptor: *mut c_void,
        present: *mut i32,
        acl: *mut *mut c_void,
        defaulted: *mut i32,
    ) -> i32;
    fn GetSecurityDescriptorControl(
        descriptor: *mut c_void,
        control: *mut u16,
        revision: *mut u32,
    ) -> i32;
    fn InitializeAcl(acl: *mut c_void, bytes: u32, revision: u32) -> i32;
    fn AddAccessAllowedAce(acl: *mut c_void, revision: u32, access: u32, sid: *mut c_void) -> i32;
    fn InitializeSecurityDescriptor(descriptor: *mut c_void, revision: u32) -> i32;
    fn SetSecurityDescriptorOwner(
        descriptor: *mut c_void,
        owner: *mut c_void,
        defaulted: i32,
    ) -> i32;
    fn SetSecurityDescriptorDacl(
        descriptor: *mut c_void,
        present: i32,
        acl: *mut c_void,
        defaulted: i32,
    ) -> i32;
    fn SetSecurityDescriptorControl(descriptor: *mut c_void, bits: u16, values: u16) -> i32;
    fn CreatePrivateObjectSecurityEx(
        parent: *mut c_void,
        creator: *mut c_void,
        result: *mut *mut c_void,
        object_type: *const c_void,
        container: i32,
        flags: u32,
        token: *mut c_void,
        mapping: *mut GenericMapping,
    ) -> i32;
    fn DestroyPrivateObjectSecurity(descriptor: *mut *mut c_void) -> i32;
    fn GetSecurityDescriptorLength(descriptor: *mut c_void) -> u32;
    fn SetSecurityInfo(
        handle: *mut c_void,
        object_type: i32,
        information: u32,
        owner: *mut c_void,
        group: *mut c_void,
        dacl: *mut c_void,
        sacl: *mut c_void,
    ) -> u32;
}

fn bool_result(value: i32) -> io::Result<()> {
    if value == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
fn storage(bytes: usize, cap: usize) -> io::Result<Vec<u64>> {
    if bytes == 0 || bytes > cap {
        return Err(invalid("native access-policy buffer exceeds limit"));
    }
    Ok(vec![0; bytes.div_ceil(8)])
}
fn wide(path: &Path) -> io::Result<Vec<u16>> {
    let length = path.as_os_str().encode_wide().count();
    if length > 32_766 || path.as_os_str().encode_wide().any(|unit| unit == 0) {
        return Err(invalid("invalid or oversized native publication path"));
    }
    let mut result = Vec::with_capacity(length + 1);
    result.extend(path.as_os_str().encode_wide());
    result.push(0);
    Ok(result)
}

struct Descriptor {
    words: Vec<u64>,
    bytes: usize,
}
impl Descriptor {
    fn pointer(&self) -> *mut c_void {
        self.words.as_ptr().cast_mut().cast()
    }
    fn range(&self) -> &[u8] {
        // SAFETY: initialized u64 storage is byte-readable and `bytes` is
        // bounded by its allocated length in every constructor.
        unsafe { std::slice::from_raw_parts(self.words.as_ptr().cast(), self.bytes) }
    }
    fn slice(&self, pointer: *mut c_void, bytes: usize) -> io::Result<&[u8]> {
        let offset = (pointer as usize)
            .checked_sub(self.pointer() as usize)
            .ok_or_else(|| invalid("native access-policy pointer outside buffer"))?;
        self.range()
            .get(
                offset
                    ..offset
                        .checked_add(bytes)
                        .ok_or_else(|| invalid("native access-policy range overflow"))?,
            )
            .ok_or_else(|| invalid("native access-policy pointer outside buffer"))
    }
    fn sid(&self, pointer: *mut c_void) -> io::Result<Vec<u32>> {
        let header = self.slice(pointer, 8)?;
        if header[0] != 1 || header[1] > 15 {
            return Err(invalid("unsupported native SID"));
        }
        let bytes = self.slice(pointer, 8 + usize::from(header[1]) * 4)?;
        // The checked SID length is a multiple of four. Array chunks preserve
        // every header/subauthority byte and remove a fallible slice conversion.
        Ok(bytes
            .as_chunks::<4>()
            .0
            .iter()
            .copied()
            .map(u32::from_le_bytes)
            .collect())
    }
    fn policy(&self) -> io::Result<Policy> {
        let (mut owner, mut group, mut acl) = (null_mut(), null_mut(), null_mut());
        let (mut defaulted, mut present, mut control, mut revision) = (0, 0, 0, 0);
        // SAFETY: this is a successful OS-produced self-relative descriptor;
        // getters borrow it, write correctly typed stack outputs, and do not
        // retain pointers. Returned component pointers are range checked below.
        unsafe {
            bool_result(GetSecurityDescriptorOwner(
                self.pointer(),
                &mut owner,
                &mut defaulted,
            ))?;
            bool_result(GetSecurityDescriptorGroup(
                self.pointer(),
                &mut group,
                &mut defaulted,
            ))?;
            bool_result(GetSecurityDescriptorDacl(
                self.pointer(),
                &mut present,
                &mut acl,
                &mut defaulted,
            ))?;
            bool_result(GetSecurityDescriptorControl(
                self.pointer(),
                &mut control,
                &mut revision,
            ))?;
        }
        if present == 0 || control & 0x8000 == 0 || revision != 1 {
            return Err(invalid("unsupported native security descriptor"));
        }
        let dacl = if acl.is_null() {
            None
        } else {
            let header = self.slice(acl, 8)?;
            let length = usize::from(u16::from_le_bytes([header[2], header[3]]));
            let count = usize::from(u16::from_le_bytes([header[4], header[5]]));
            let bytes = self.slice(acl, length)?;
            if length < 8 || !matches!(header[0], 2 | 4) {
                return Err(invalid("unsupported native ACL"));
            }
            let mut used = 8usize;
            for _ in 0..count {
                let ace = bytes
                    .get(
                        used..used
                            .checked_add(4)
                            .ok_or_else(|| invalid("native ACL overflow"))?,
                    )
                    .ok_or_else(|| invalid("invalid native ACL entry"))?;
                let size = usize::from(u16::from_le_bytes([ace[2], ace[3]]));
                if size < 4 || size % 4 != 0 {
                    return Err(invalid("invalid native ACL entry"));
                }
                used = used
                    .checked_add(size)
                    .filter(|end| *end <= length)
                    .ok_or_else(|| invalid("invalid native ACL entry length"))?;
            }
            // Preserve ACE bytes/order, ignoring unused ACL allocation slack.
            let mut canonical = bytes[..used].to_vec();
            canonical[1] = 0;
            canonical[2..4].copy_from_slice(&(used as u16).to_le_bytes());
            canonical[6..8].fill(0);
            Some(
                canonical
                    .chunks(4)
                    .map(|part| {
                        let mut word = [0; 4];
                        word[..part.len()].copy_from_slice(part);
                        u32::from_le_bytes(word)
                    })
                    .collect(),
            )
        };
        Ok(Policy {
            owner: self.sid(owner)?,
            group: self.sid(group)?,
            dacl,
            protected: control & SE_DACL_PROTECTED != 0,
        })
    }
}

fn descriptor(file: &File) -> io::Result<Descriptor> {
    for _ in 0..3 {
        let mut needed = 0;
        // SAFETY: a live borrowed handle, null zero-length query buffer, and
        // writable DWORD output; no pointer is retained.
        let query = unsafe {
            GetKernelObjectSecurity(
                file.as_raw_handle(),
                POLICY_INFORMATION,
                null_mut(),
                0,
                &mut needed,
            )
        };
        if query == 0 && io::Error::last_os_error().raw_os_error() != Some(122) {
            return Err(io::Error::last_os_error());
        }
        let mut result = Descriptor {
            words: storage(needed as usize, MAX_DESCRIPTOR_BYTES)?,
            bytes: needed as usize,
        };
        let supplied = needed;
        // SAFETY: storage is aligned and initialized, `supplied` is within the
        // checked allocation, and the live handle remains borrowed throughout.
        let success = unsafe {
            GetKernelObjectSecurity(
                file.as_raw_handle(),
                POLICY_INFORMATION,
                result.words.as_mut_ptr().cast(),
                supplied,
                &mut needed,
            )
        };
        if success != 0 {
            if needed > supplied {
                return Err(invalid("native descriptor exceeded supplied buffer"));
            }
            result.bytes = needed as usize;
            return Ok(result);
        }
        if io::Error::last_os_error().raw_os_error() != Some(122) {
            return Err(io::Error::last_os_error());
        }
    }
    Err(invalid(
        "native access policy changed during bounded capture",
    ))
}

struct Token(OwnedHandle);
impl Token {
    fn current() -> io::Result<Self> {
        let mut handle = null_mut();
        // SAFETY: GetCurrentProcess supplies a borrowed pseudo-handle. On
        // success OpenProcessToken transfers one real QUERY handle to us.
        unsafe {
            bool_result(OpenProcessToken(GetCurrentProcess(), 8, &mut handle))?;
        }
        if handle.is_null() {
            return Err(invalid("native token query returned null"));
        }
        // SAFETY: success transferred ownership; OwnedHandle closes it once.
        Ok(Self(unsafe { OwnedHandle::from_raw_handle(handle) }))
    }
    fn sid(&self, class: i32) -> io::Result<Vec<u32>> {
        let mut needed = 0;
        // SAFETY: live QUERY token, supported private class, bounded query
        // outputs. TokenUser/TokenOwner begin with one PSID field.
        let success = unsafe {
            GetTokenInformation(self.0.as_raw_handle(), class, null_mut(), 0, &mut needed)
        };
        if success == 0 && io::Error::last_os_error().raw_os_error() != Some(122) {
            return Err(io::Error::last_os_error());
        }
        let mut result = Descriptor {
            words: storage(needed as usize, MAX_TOKEN_BYTES)?,
            bytes: needed as usize,
        };
        let supplied = needed;
        // SAFETY: initialized pointer-aligned storage covers the requested
        // bytes. The successful token buffer and its SID remain owned below.
        unsafe {
            bool_result(GetTokenInformation(
                self.0.as_raw_handle(),
                class,
                result.words.as_mut_ptr().cast(),
                supplied,
                &mut needed,
            ))?;
        }
        if needed > supplied || result.bytes < std::mem::size_of::<*mut c_void>() {
            return Err(invalid("invalid token SID buffer"));
        }
        // SAFETY: the leading pointer is inside aligned initialized storage;
        // sid() validates its target range before inspecting/copying bytes.
        let pointer = unsafe { result.words.as_ptr().cast::<*mut c_void>().read() };
        result.sid(pointer)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Policy {
    owner: Vec<u32>,
    group: Vec<u32>,
    dacl: Option<Vec<u32>>,
    protected: bool,
}
impl Policy {
    pub(super) fn check_verification_policy(&self) -> io::Result<()> {
        if let Some(acl) = &self.dacl {
            admit_dacl(acl)?;
        }
        Ok(())
    }
    pub(super) fn has_inherited_entries(&self) -> bool {
        let Some(acl) = &self.dacl else {
            return false;
        };
        let mut cursor = 2;
        for _ in 0..(acl[1] & 0xffff) {
            let header = acl[cursor];
            if header & 0x1000 != 0 {
                return true;
            }
            cursor += (header >> 16) as usize / 4;
        }
        false
    }
    pub(super) fn capture(file: &File) -> io::Result<Self> {
        descriptor(file)?.policy()
    }
    pub(super) fn check_assignable_owner(&self) -> io::Result<()> {
        let token = Token::current()?;
        if self.owner != token.sid(1)? && self.owner != token.sid(4)? {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "unsupported output owner; cannot preserve its policy",
            ));
        }
        Ok(())
    }
    pub(super) fn apply(&self, file: &File) -> io::Result<()> {
        let protection = if self.protected {
            0x8000_0000
        } else {
            0x2000_0000
        };
        // SAFETY: live writable file handle; owner/group/ACL are aligned,
        // validated copied OS components and remain borrowed for this synchronous
        // call. Object type 1 is SE_FILE_OBJECT. A null ACL is deliberate only
        // for a captured null DACL. No SACL is requested or claimed preserved.
        let error = unsafe {
            SetSecurityInfo(
                file.as_raw_handle(),
                1,
                POLICY_INFORMATION | protection,
                self.owner.as_ptr().cast_mut().cast(),
                self.group.as_ptr().cast_mut().cast(),
                self.dacl
                    .as_ref()
                    .map_or(null_mut(), |acl| acl.as_ptr().cast_mut().cast()),
                null_mut(),
            )
        };
        if error != 0 {
            return Err(io::Error::from_raw_os_error(error as i32));
        }
        let actual = Self::capture(file)?;
        if actual != *self {
            return Err(invalid(
                "installed access policy differs from intended policy",
            ));
        }
        Ok(())
    }
}

struct PrivateSecurity {
    owner: Vec<u32>,
    acl: Vec<u32>,
    descriptor: [u64; 8],
}
impl PrivateSecurity {
    fn new() -> io::Result<Self> {
        let owner = Token::current()?.sid(1)?;
        let bytes = 8 + 8 + owner.len() * 4;
        let mut result = Self {
            owner,
            acl: vec![0; bytes.div_ceil(4)],
            descriptor: [0; 8],
        };
        let sd = result.descriptor.as_mut_ptr().cast();
        // SAFETY: ACL storage covers header+one allow ACE+bounded SID. The
        // absolute descriptor's aligned 64 bytes exceed its native size (40
        // on Win64/20 on Win32). It contains pointers into owned heap buffers;
        // these remain stable through every attributes/syscall pair.
        unsafe {
            bool_result(InitializeAcl(
                result.acl.as_mut_ptr().cast(),
                bytes as u32,
                2,
            ))?;
            bool_result(AddAccessAllowedAce(
                result.acl.as_mut_ptr().cast(),
                2,
                FILE_ALL_ACCESS,
                result.owner.as_mut_ptr().cast(),
            ))?;
            bool_result(InitializeSecurityDescriptor(sd, 1))?;
            bool_result(SetSecurityDescriptorOwner(
                sd,
                result.owner.as_mut_ptr().cast(),
                0,
            ))?;
            bool_result(SetSecurityDescriptorDacl(
                sd,
                1,
                result.acl.as_mut_ptr().cast(),
                0,
            ))?;
            bool_result(SetSecurityDescriptorControl(
                sd,
                SE_DACL_PROTECTED,
                SE_DACL_PROTECTED,
            ))?;
        }
        Ok(result)
    }
    fn attributes(&mut self) -> Attributes {
        Attributes {
            size: std::mem::size_of::<Attributes>() as u32,
            descriptor: self.descriptor.as_mut_ptr().cast(),
            inherit: 0,
        }
    }
}

pub(super) fn create_directory(path: &Path) -> io::Result<()> {
    let path = wide(path)?;
    let mut security = PrivateSecurity::new()?;
    let attributes = security.attributes();
    // SAFETY: terminated path and complete native attributes remain borrowed;
    // creation is exclusive and copies the descriptor before returning.
    unsafe { bool_result(CreateDirectoryW(path.as_ptr(), &attributes)) }
}
pub(super) fn create_file(path: &Path) -> io::Result<File> {
    let path = wide(path)?;
    let mut security = PrivateSecurity::new()?;
    let attributes = security.attributes();
    // SAFETY: terminated path, complete borrowed descriptor, CREATE_NEW=1
    // gives exclusive creation. The real successful handle transfers to File;
    // INVALID_HANDLE_VALUE never enters an owning Rust handle.
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            0xc00f_0000,
            7,
            &attributes,
            1,
            0x0020_0080,
            null_mut(),
        )
    };
    if handle as isize == -1 {
        return Err(io::Error::last_os_error());
    }
    if handle.is_null() {
        return Err(invalid("native file creation returned null"));
    }
    // SAFETY: unique successful file handle; File closes it exactly once.
    Ok(unsafe { File::from_raw_handle(handle) })
}

pub(super) fn policy_handle(path: &Path) -> io::Result<File> {
    std::fs::OpenOptions::new()
        .access_mode(0x000e_0080)
        .custom_flags(0x0020_0000)
        .open(path)
}

pub(super) fn new_file_policy(parent: &File) -> io::Result<Policy> {
    let parent = descriptor(parent)?;
    let token = Token::current()?;
    let mut mapping = GenericMapping {
        read: 0x0012_0089,
        write: 0x0012_0116,
        execute: 0x0012_00a0,
        all: FILE_ALL_ACCESS,
    };
    let mut allocated = PrivateDescriptor(null_mut());
    // SAFETY: live descriptor/token and correct file generic mapping. A null
    // creator asks Windows for its normal inherited/default creator policy;
    // no privilege/owner check bypass flags are used. Native ACL/SID formats
    // bound OS-owned allocation; caller-owned copies are checked below.
    unsafe {
        bool_result(CreatePrivateObjectSecurityEx(
            parent.pointer(),
            null_mut(),
            &mut allocated.0,
            null(),
            0,
            1,
            token.0.as_raw_handle(),
            &mut mapping,
        ))?;
    }
    if allocated.0.is_null() {
        return Err(invalid("native inherited policy returned null"));
    }
    // SAFETY: successful OS-owned descriptor; length query borrows it.
    let bytes = unsafe { GetSecurityDescriptorLength(allocated.0) } as usize;
    let mut copied = Descriptor {
        words: storage(bytes, MAX_DESCRIPTOR_BYTES)?,
        bytes,
    };
    // SAFETY: the successful OS descriptor covers `bytes`, checked destination
    // covers that length, and the distinct buffers cannot overlap.
    unsafe {
        std::ptr::copy_nonoverlapping(
            allocated.0.cast::<u8>(),
            copied.words.as_mut_ptr().cast(),
            bytes,
        );
    }
    let result = copied.policy()?;
    result.check_assignable_owner()?;
    Ok(result)
}
struct PrivateDescriptor(*mut c_void);
impl Drop for PrivateDescriptor {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this owns the sole successful CreatePrivateObjectSecurityEx
            // allocation; the matching SDK destructor releases it once.
            unsafe {
                DestroyPrivateObjectSecurity(&mut self.0);
            }
        }
    }
}

#[cfg(test)]
pub(super) fn allow_then_owner_rights_denial(file: &File) -> io::Result<Policy> {
    allow_then_denial(file, READ_CONTROL, OWNER_RIGHTS)
}

#[cfg(test)]
pub(super) fn allow_then_denial(file: &File, mask: u32, sid: [u32; 3]) -> io::Result<Policy> {
    // The private creator gives exactly one protected current-user allow.
    // Append a denial without canonicalizing ACE order: native access can
    // succeed after the earlier FullControl grant satisfies the request.
    let mut policy = Policy::capture(file)?;
    let acl = policy.dacl.as_mut().unwrap();
    assert_eq!(acl[1] & 0xffff, 1);
    acl.extend_from_slice(&[(20u32 << 16) | 1, mask]);
    acl.extend_from_slice(&sid);
    acl[0] = (acl[0] & 0xffff) | ((acl.len() as u32 * 4) << 16);
    acl[1] = (acl[1] & 0xffff_0000) | 2;
    policy.apply(file)?;
    Ok(policy)
}

#[cfg(test)]
pub(super) fn token_owner_diagnostics() -> io::Result<String> {
    let token = Token::current()?;
    Ok(format!(
        "token_user={:?}; token_default_owner={:?}",
        token.sid(1)?,
        token.sid(4)?
    ))
}

#[cfg(test)]
mod admission_tests {
    use super::*;

    fn denial(kind: u32, flags: u32, mask: u32, sid: &[u32]) -> Vec<u32> {
        let mut ace = vec![kind, mask];
        if matches!(kind, 6 | 12) {
            ace.push(flags);
            ace.resize(3 + (flags & 3).count_ones() as usize * 4, 0);
        }
        ace.extend_from_slice(sid);
        if matches!(kind, 10 | 12) {
            ace.extend_from_slice(&[0x7874_7261, 0]); // opaque callback data
        }
        ace[0] |= (ace.len() as u32 * 4) << 16;
        ace
    }

    fn acl(aces: &[Vec<u32>]) -> Vec<u32> {
        let mut result = vec![4, aces.len() as u32];
        for ace in aces {
            result.extend_from_slice(ace);
        }
        result[0] |= (result.len() as u32 * 4) << 16;
        result
    }

    #[test]
    fn every_denial_layout_and_file_generic_mask_rejects_owner_rights() {
        for kind in [1, 6, 10, 12] {
            for flags in 0..=3 {
                for mask in [
                    READ_CONTROL,
                    0x1000_0000,
                    0x2000_0000,
                    0x4000_0000,
                    0x8000_0000,
                ] {
                    let bytes = acl(&[denial(kind, flags, mask, &OWNER_RIGHTS)]);
                    assert_eq!(
                        admit_dacl(&bytes).unwrap_err().kind(),
                        io::ErrorKind::PermissionDenied,
                        "kind={kind}, flags={flags}, mask={mask:x}"
                    );
                }
            }
        }
    }

    #[test]
    fn inert_denials_unrelated_sids_and_other_rights_are_admitted_unchanged() {
        let unrelated = [0x0000_0101, 0x0100_0000, 0]; // Everyone S-1-1-0
        for kind in [1, 6, 10, 12] {
            let mut inert = denial(kind, 3, READ_CONTROL, &OWNER_RIGHTS);
            inert[0] |= 0x0800;
            for ace in [
                inert,
                denial(kind, 3, READ_CONTROL, &unrelated),
                denial(kind, 3, 2, &OWNER_RIGHTS),
            ] {
                let bytes = acl(&[ace]);
                let before = bytes.clone();
                admit_dacl(&bytes).unwrap();
                assert_eq!(bytes, before);
            }
        }
        admit_dacl(&acl(&[])).unwrap();
    }

    #[test]
    fn ace_order_does_not_hide_a_denial_or_change_policy_bytes() {
        let mut grant = denial(1, 0, FILE_ALL_ACCESS, &OWNER_RIGHTS);
        grant[0] &= !0xff;
        let deny = denial(1, 0, READ_CONTROL, &OWNER_RIGHTS);
        for entries in [vec![grant.clone(), deny.clone()], vec![deny, grant]] {
            let bytes = acl(&entries);
            let before = bytes.clone();
            assert!(admit_dacl(&bytes).is_err());
            assert_eq!(bytes, before);
        }
    }

    #[test]
    fn malformed_or_unknown_effective_encodings_fail_closed_without_panics() {
        let good = acl(&[denial(1, 0, READ_CONTROL, &OWNER_RIGHTS)]);
        for end in 0..good.len() {
            assert_eq!(
                admit_dacl(&good[..end]).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
        for (offset, replacement) in [
            (0, 0),
            (1, 0),
            (1, u32::MAX),
            (2, 0),
            (2, (4 << 16) | 1),
            (2, (24 << 16) | 1),
            (2, (20 << 16) | 0xff),
            (4, 0x0102),
            (4, 0x1001),
            (4, 0x0201),
        ] {
            let mut bad = good.clone();
            bad[offset] = replacement;
            assert_eq!(
                admit_dacl(&bad).unwrap_err().kind(),
                io::ErrorKind::InvalidInput,
                "offset={offset}, replacement={replacement:x}"
            );
        }
        for kind in [6, 12] {
            let mut bad = denial(kind, 0, READ_CONTROL, &OWNER_RIGHTS);
            bad[2] = 4;
            assert_eq!(
                admit_dacl(&acl(&[bad])).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
            let mut truncated_guid = denial(kind, 0, READ_CONTROL, &OWNER_RIGHTS);
            truncated_guid[2] = 3;
            assert_eq!(
                admit_dacl(&acl(&[truncated_guid])).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
    }
}
