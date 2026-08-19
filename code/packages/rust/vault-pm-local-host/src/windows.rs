use super::{LocalHostError, MAX_PATH_BYTES};
use std::ffi::c_void;
use std::fs::File;
use std::io::{Read, Write};
use std::mem::{size_of, MaybeUninit};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Component, Path, PathBuf};
use std::ptr::{addr_of, null, null_mut};
use std::sync::atomic::{AtomicU64, Ordering};
use windows_sys::Win32::Foundation::{
    BOOL, ERROR_ALREADY_EXISTS, ERROR_FILE_EXISTS, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND,
    HANDLE, INVALID_HANDLE_VALUE, PSID,
};
use windows_sys::Win32::Security::{
    AclSizeInformation, AddAccessAllowedAce, EqualSid, GetAce, GetAclInformation,
    GetKernelObjectSecurity, GetLengthSid, GetSecurityDescriptorControl, GetSecurityDescriptorDacl,
    GetSecurityDescriptorOwner, GetTokenInformation, InitializeAcl, InitializeSecurityDescriptor,
    SetSecurityDescriptorControl, SetSecurityDescriptorDacl, SetSecurityDescriptorOwner, TokenUser,
    ACCESS_ALLOWED_ACE, ACL, ACL_REVISION, ACL_SIZE_INFORMATION, DACL_SECURITY_INFORMATION,
    OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR,
    SE_DACL_PROTECTED, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateDirectoryW, CreateFileW, DeleteFileW, FileAttributeTagInfo, GetFileInformationByHandleEx,
    MoveFileExW, CREATE_NEW, FILE_ALL_ACCESS, FILE_ATTRIBUTE_DIRECTORY,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, MOVEFILE_REPLACE_EXISTING,
    MOVEFILE_WRITE_THROUGH, OPEN_EXISTING,
};
use windows_sys::Win32::System::SystemServices::{
    ACCESS_ALLOWED_ACE_TYPE, SECURITY_DESCRIPTOR_REVISION,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

const MAX_SECURITY_DESCRIPTOR_BYTES: u32 = 64 * 1024;
const MINIMUM_SID_BYTES: usize = 8;
const MAX_TEMPORARY_ATTEMPTS: usize = 64;
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

pub(super) fn ensure_private_directory(path: &Path) -> Result<(), LocalHostError> {
    validate_absolute(path)?;
    let mut ancestors: Vec<_> = path
        .ancestors()
        .filter(|ancestor| !ancestor.as_os_str().is_empty())
        .collect();
    ancestors.reverse();
    let final_index = ancestors.len().saturating_sub(1);
    for (index, ancestor) in ancestors.into_iter().enumerate() {
        match open_directory(ancestor) {
            Ok(handle) => {
                if index == final_index {
                    verify_owner_acl(handle.as_raw_handle() as HANDLE)?;
                }
            }
            Err(LocalHostError::ParentUnavailable | LocalHostError::AccessFailed) => {
                create_private_directory(ancestor)?;
                let handle = open_directory(ancestor)?;
                if index == final_index {
                    verify_owner_acl(handle.as_raw_handle() as HANDLE)?;
                }
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// Windows equivalent of the Unix leaf-only runtime directory check.
///
/// VLT-PM48 ships a Unix-domain-socket agent only; Windows named-pipe support
/// is an explicitly deferred follow-up (see that spec's §9). Nothing in this
/// crate binds a socket on Windows, so this simply reuses the existing
/// recursive walk rather than adding an untested second code path for a
/// directory nothing yet uses. A future named-pipe implementation should
/// revisit this the same way the Unix build's leaf-only runtime-directory
/// check diverged from [`ensure_private_directory`]: `%TEMP%` can also
/// involve reparse points this walk has not been proven against.
pub(super) fn ensure_private_runtime_directory(path: &Path) -> Result<(), LocalHostError> {
    ensure_private_directory(path)
}

pub(super) fn open_private_lock(path: &Path) -> Result<File, LocalHostError> {
    validate_absolute(path)?;
    let parent = path.parent().ok_or(LocalHostError::InvalidPath)?;
    let parent_handle = open_directory(parent)?;
    verify_owner_acl(parent_handle.as_raw_handle() as HANDLE)?;
    let wide = wide_path(path)?;
    let mut security = OwnerSecurity::new(FILE_ALL_ACCESS)?;
    let attributes = security.attributes();
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            &attributes,
            CREATE_NEW,
            FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        )
    };
    let raw = if raw == INVALID_HANDLE_VALUE
        && matches!(
            unsafe { windows_sys::Win32::Foundation::GetLastError() },
            ERROR_ALREADY_EXISTS | ERROR_FILE_EXISTS
        ) {
        unsafe {
            CreateFileW(
                wide.as_ptr(),
                FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                0,
            )
        }
    } else {
        raw
    };
    let handle = owned_handle(raw).map_err(|_| LocalHostError::AccessFailed)?;
    verify_regular_file(handle.as_raw_handle() as HANDLE)?;
    verify_owner_acl(handle.as_raw_handle() as HANDLE)?;
    Ok(File::from(handle))
}

pub(super) fn load_private_config(
    path: &Path,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>, LocalHostError> {
    validate_absolute(path)?;
    let parent = path.parent().ok_or(LocalHostError::InvalidPath)?;
    let parent_handle = open_directory(parent)?;
    verify_owner_acl(parent_handle.as_raw_handle() as HANDLE)?;
    let wide = wide_path(path)?;
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return match unsafe { windows_sys::Win32::Foundation::GetLastError() } {
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Ok(None),
            _ => Err(LocalHostError::AccessFailed),
        };
    }
    let handle = owned_handle(raw).map_err(|_| LocalHostError::AccessFailed)?;
    verify_regular_file(handle.as_raw_handle() as HANDLE)?;
    verify_owner_acl(handle.as_raw_handle() as HANDLE)?;
    let file = File::from(handle);
    let mut bytes = Vec::new();
    file.take((max_bytes as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| LocalHostError::AccessFailed)?;
    if bytes.is_empty() || bytes.len() > max_bytes {
        return Err(LocalHostError::InvalidConfigBytes);
    }
    Ok(Some(bytes))
}

pub(super) fn create_private_config(
    path: &Path,
    bytes: &[u8],
    max_bytes: usize,
) -> Result<(), LocalHostError> {
    if load_private_config(path, max_bytes)?.is_some() {
        return Err(LocalHostError::ConfigAlreadyExists);
    }
    verify_config_parent(path)?;
    let (temporary, temporary_path) = create_temporary(path)?;
    let result = (|| {
        persist_temporary(&temporary, bytes)?;
        let source = wide_path(&temporary_path)?;
        let destination = wide_path(path)?;
        if unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(
                match unsafe { windows_sys::Win32::Foundation::GetLastError() } {
                    ERROR_ALREADY_EXISTS | ERROR_FILE_EXISTS => LocalHostError::ConfigAlreadyExists,
                    _ => LocalHostError::AccessFailed,
                },
            );
        }
        Ok(())
    })();
    if result.is_err() {
        delete_temporary(&temporary_path);
    }
    result
}

pub(super) fn compare_exchange_private_config(
    path: &Path,
    expected: &[u8],
    replacement: &[u8],
    max_bytes: usize,
) -> Result<(), LocalHostError> {
    if load_private_config(path, max_bytes)?.as_deref() != Some(expected) {
        return Err(LocalHostError::ConfigConflict);
    }
    let (temporary, temporary_path) = create_temporary(path)?;
    let result = (|| {
        persist_temporary(&temporary, replacement)?;
        let source = wide_path(&temporary_path)?;
        let destination = wide_path(path)?;
        let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;
        if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), flags) } == 0 {
            return Err(LocalHostError::AccessFailed);
        }
        Ok(())
    })();
    if result.is_err() {
        delete_temporary(&temporary_path);
    }
    result
}

fn verify_config_parent(path: &Path) -> Result<(), LocalHostError> {
    validate_absolute(path)?;
    let parent = path.parent().ok_or(LocalHostError::InvalidPath)?;
    let handle = open_directory(parent)?;
    verify_owner_acl(handle.as_raw_handle() as HANDLE)
}

fn create_temporary(path: &Path) -> Result<(File, PathBuf), LocalHostError> {
    let parent = path.parent().ok_or(LocalHostError::InvalidPath)?;
    for _ in 0..MAX_TEMPORARY_ATTEMPTS {
        let sequence = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
        let temporary_path = parent.join(format!(
            ".vault-pm.toml.tmp.{}.{}",
            std::process::id(),
            sequence
        ));
        let wide = wide_path(&temporary_path)?;
        let mut security = OwnerSecurity::new(FILE_ALL_ACCESS)?;
        let attributes = security.attributes();
        let raw = unsafe {
            CreateFileW(
                wide.as_ptr(),
                FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                &attributes,
                CREATE_NEW,
                FILE_FLAG_OPEN_REPARSE_POINT,
                0,
            )
        };
        if raw != INVALID_HANDLE_VALUE {
            let handle = owned_handle(raw).map_err(|_| LocalHostError::AccessFailed)?;
            verify_regular_file(handle.as_raw_handle() as HANDLE)?;
            verify_owner_acl(handle.as_raw_handle() as HANDLE)?;
            return Ok((File::from(handle), temporary_path));
        }
        if !matches!(
            unsafe { windows_sys::Win32::Foundation::GetLastError() },
            ERROR_ALREADY_EXISTS | ERROR_FILE_EXISTS
        ) {
            return Err(LocalHostError::AccessFailed);
        }
    }
    Err(LocalHostError::AccessFailed)
}

fn persist_temporary(mut file: &File, bytes: &[u8]) -> Result<(), LocalHostError> {
    file.write_all(bytes)
        .map_err(|_| LocalHostError::AccessFailed)?;
    file.sync_all().map_err(|_| LocalHostError::AccessFailed)
}

fn delete_temporary(path: &Path) {
    if let Ok(wide) = wide_path(path) {
        unsafe {
            DeleteFileW(wide.as_ptr());
        }
    }
}

fn create_private_directory(path: &Path) -> Result<(), LocalHostError> {
    let wide = wide_path(path)?;
    let mut security = OwnerSecurity::new(FILE_ALL_ACCESS)?;
    let attributes = security.attributes();
    if unsafe { CreateDirectoryW(wide.as_ptr(), &attributes) } == 0 {
        return match unsafe { windows_sys::Win32::Foundation::GetLastError() } {
            ERROR_ALREADY_EXISTS => Ok(()),
            _ => Err(LocalHostError::AccessFailed),
        };
    }
    Ok(())
}

fn open_directory(path: &Path) -> Result<OwnedHandle, LocalHostError> {
    let wide = wide_path(path)?;
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        )
    };
    let handle = owned_handle(raw).map_err(|_| LocalHostError::ParentUnavailable)?;
    let attributes = file_attributes(handle.as_raw_handle() as HANDLE)?;
    if attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err(LocalHostError::UnsafeObjectType);
    }
    Ok(handle)
}

fn validate_absolute(path: &Path) -> Result<(), LocalHostError> {
    let encoded_units = path.as_os_str().encode_wide().count();
    if !path.is_absolute()
        || encoded_units == 0
        || encoded_units > MAX_PATH_BYTES
        || path
            .components()
            .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
        || path.as_os_str().encode_wide().any(|unit| unit == 0)
    {
        return Err(LocalHostError::InvalidPath);
    }
    Ok(())
}

fn wide_path(path: &Path) -> Result<Vec<u16>, LocalHostError> {
    let mut wide: Vec<_> = path.as_os_str().encode_wide().collect();
    if wide.is_empty() || wide.len() > MAX_PATH_BYTES || wide.contains(&0) {
        return Err(LocalHostError::InvalidPath);
    }
    wide.push(0);
    Ok(wide)
}

fn owned_handle(raw: HANDLE) -> Result<OwnedHandle, ()> {
    if raw == INVALID_HANDLE_VALUE || raw == 0 {
        Err(())
    } else {
        Ok(unsafe { OwnedHandle::from_raw_handle(raw as *mut c_void) })
    }
}

fn file_attributes(raw: HANDLE) -> Result<FILE_ATTRIBUTE_TAG_INFO, LocalHostError> {
    let mut information = MaybeUninit::<FILE_ATTRIBUTE_TAG_INFO>::zeroed();
    if unsafe {
        GetFileInformationByHandleEx(
            raw,
            FileAttributeTagInfo,
            information.as_mut_ptr().cast(),
            size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    } == 0
    {
        return Err(LocalHostError::AccessFailed);
    }
    Ok(unsafe { information.assume_init() })
}

fn verify_regular_file(raw: HANDLE) -> Result<(), LocalHostError> {
    let attributes = file_attributes(raw)?;
    if attributes.FileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
        return Err(LocalHostError::UnsafeObjectType);
    }
    Ok(())
}

struct CurrentUserSid {
    _storage: Vec<usize>,
    sid: PSID,
}

impl CurrentUserSid {
    fn query() -> Result<Self, LocalHostError> {
        let mut raw_token = 0;
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw_token) } == 0 {
            return Err(LocalHostError::AccessFailed);
        }
        let _token = owned_handle(raw_token).map_err(|_| LocalHostError::AccessFailed)?;
        let mut needed = 0;
        unsafe {
            GetTokenInformation(raw_token, TokenUser, null_mut(), 0, &mut needed);
        }
        if needed == 0 || needed > MAX_SECURITY_DESCRIPTOR_BYTES {
            return Err(LocalHostError::AccessFailed);
        }
        let mut storage = vec![0usize; bytes_to_words(needed)];
        if unsafe {
            GetTokenInformation(
                raw_token,
                TokenUser,
                storage.as_mut_ptr().cast(),
                needed,
                &mut needed,
            )
        } == 0
        {
            return Err(LocalHostError::AccessFailed);
        }
        let token_user = storage.as_ptr().cast::<TOKEN_USER>();
        let sid = unsafe { (*token_user).User.Sid };
        if sid.is_null() || unsafe { GetLengthSid(sid) } == 0 {
            return Err(LocalHostError::AccessFailed);
        }
        Ok(Self {
            _storage: storage,
            sid,
        })
    }
}

fn verify_owner_acl(raw: HANDLE) -> Result<(), LocalHostError> {
    let current = CurrentUserSid::query()?;
    let information = OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
    let mut needed = 0;
    unsafe {
        GetKernelObjectSecurity(raw, information, null_mut(), 0, &mut needed);
    }
    if needed == 0 || needed > MAX_SECURITY_DESCRIPTOR_BYTES {
        return Err(LocalHostError::AccessFailed);
    }
    let mut storage = vec![0usize; bytes_to_words(needed)];
    let descriptor: PSECURITY_DESCRIPTOR = storage.as_mut_ptr().cast();
    if unsafe { GetKernelObjectSecurity(raw, information, descriptor, needed, &mut needed) } == 0 {
        return Err(LocalHostError::AccessFailed);
    }
    let mut owner = null_mut();
    let mut owner_defaulted: BOOL = 0;
    if unsafe { GetSecurityDescriptorOwner(descriptor, &mut owner, &mut owner_defaulted) } == 0 {
        return Err(LocalHostError::AccessFailed);
    }
    if owner.is_null() || unsafe { EqualSid(owner, current.sid) } == 0 {
        return Err(LocalHostError::InsecureOwner);
    }
    let mut control = 0;
    let mut revision = 0;
    if unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) } == 0 {
        return Err(LocalHostError::AccessFailed);
    }
    if control & SE_DACL_PROTECTED == 0 {
        return Err(LocalHostError::InsecurePermissions);
    }
    let mut present: BOOL = 0;
    let mut dacl: *mut ACL = null_mut();
    let mut defaulted: BOOL = 0;
    if unsafe { GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted) }
        == 0
        || present == 0
        || dacl.is_null()
    {
        return Err(LocalHostError::InsecurePermissions);
    }
    let mut acl_information = MaybeUninit::<ACL_SIZE_INFORMATION>::zeroed();
    if unsafe {
        GetAclInformation(
            dacl,
            acl_information.as_mut_ptr().cast(),
            size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    } == 0
        || unsafe { acl_information.assume_init() }.AceCount != 1
    {
        return Err(LocalHostError::InsecurePermissions);
    }
    let mut ace_pointer: *mut c_void = null_mut();
    if unsafe { GetAce(dacl, 0, &mut ace_pointer) } == 0 || ace_pointer.is_null() {
        return Err(LocalHostError::AccessFailed);
    }
    let ace = unsafe { &*ace_pointer.cast::<ACCESS_ALLOWED_ACE>() };
    let sid_offset = size_of::<ACCESS_ALLOWED_ACE>() - size_of::<u32>();
    if usize::from(ace.Header.AceSize) < sid_offset + MINIMUM_SID_BYTES {
        return Err(LocalHostError::InsecurePermissions);
    }
    let sid = addr_of!(ace.SidStart).cast_mut().cast();
    let sid_bytes = unsafe { GetLengthSid(sid) } as usize;
    if u32::from(ace.Header.AceType) != ACCESS_ALLOWED_ACE_TYPE
        || ace.Header.AceFlags != 0
        || sid_bytes < MINIMUM_SID_BYTES
        || usize::from(ace.Header.AceSize) < sid_offset + sid_bytes
        || unsafe { EqualSid(sid, current.sid) } == 0
    {
        return Err(LocalHostError::InsecurePermissions);
    }
    Ok(())
}

struct OwnerSecurity {
    _owner: CurrentUserSid,
    _acl_storage: Vec<u32>,
    descriptor: SECURITY_DESCRIPTOR,
}

impl OwnerSecurity {
    fn new(access: u32) -> Result<Self, LocalHostError> {
        let owner = CurrentUserSid::query()?;
        let sid_bytes = unsafe { GetLengthSid(owner.sid) } as usize;
        let acl_bytes =
            size_of::<ACL>() + size_of::<ACCESS_ALLOWED_ACE>() - size_of::<u32>() + sid_bytes;
        let mut acl_storage = vec![0u32; acl_bytes.div_ceil(size_of::<u32>())];
        let acl = acl_storage.as_mut_ptr().cast::<ACL>();
        if unsafe { InitializeAcl(acl, acl_bytes as u32, ACL_REVISION) } == 0
            || unsafe { AddAccessAllowedAce(acl, ACL_REVISION, access, owner.sid) } == 0
        {
            return Err(LocalHostError::AccessFailed);
        }
        let mut descriptor = unsafe { MaybeUninit::<SECURITY_DESCRIPTOR>::zeroed().assume_init() };
        let descriptor_ptr: PSECURITY_DESCRIPTOR =
            (&mut descriptor as *mut SECURITY_DESCRIPTOR).cast();
        if unsafe { InitializeSecurityDescriptor(descriptor_ptr, SECURITY_DESCRIPTOR_REVISION) }
            == 0
            || unsafe { SetSecurityDescriptorOwner(descriptor_ptr, owner.sid, 0) } == 0
            || unsafe { SetSecurityDescriptorDacl(descriptor_ptr, 1, acl, 0) } == 0
            || unsafe {
                SetSecurityDescriptorControl(descriptor_ptr, SE_DACL_PROTECTED, SE_DACL_PROTECTED)
            } == 0
        {
            return Err(LocalHostError::AccessFailed);
        }
        Ok(Self {
            _owner: owner,
            _acl_storage: acl_storage,
            descriptor,
        })
    }

    fn attributes(&mut self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: (&mut self.descriptor as *mut SECURITY_DESCRIPTOR).cast(),
            bInheritHandle: 0,
        }
    }
}

fn bytes_to_words(bytes: u32) -> usize {
    (bytes as usize).div_ceil(size_of::<usize>())
}
