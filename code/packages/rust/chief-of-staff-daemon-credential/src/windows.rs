use super::{
    fresh_credential, validate_credential, CredentialFileError, ENCODED_CREDENTIAL_BYTES,
    MAX_PATH_BYTES,
};
use coding_adventures_zeroize::Zeroizing;
use std::ffi::c_void;
use std::fs::File;
use std::io::{Read, Write};
use std::mem::{size_of, MaybeUninit};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Component, Path};
use std::ptr::{addr_of, null, null_mut};
use std::time::Duration;
use windows_sys::Win32::Foundation::{
    GetLastError, BOOL, ERROR_ALREADY_EXISTS, ERROR_FILE_EXISTS, ERROR_FILE_NOT_FOUND,
    ERROR_PATH_NOT_FOUND, ERROR_SHARING_VIOLATION, HANDLE, INVALID_HANDLE_VALUE, PSID,
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
    CreateFileW, FileAttributeTagInfo, GetFileInformationByHandleEx, CREATE_NEW,
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
    FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::SystemServices::{
    ACCESS_ALLOWED_ACE_TYPE, SECURITY_DESCRIPTOR_REVISION,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

const OWNER_ACCESS: u32 = FILE_GENERIC_READ | FILE_GENERIC_WRITE;
const MAX_SECURITY_DESCRIPTOR_BYTES: u32 = 64 * 1024;
// See the matching constant in unix.rs: losing threads in the create race
// poll here while the winner finishes writing and fsync-ing the file it
// just created. Widened from 250ms to 3s of headroom to absorb scheduling
// noise on loaded/shared CI runners (see lessons.md).
const PUBLICATION_RETRIES: usize = 3000;
const MINIMUM_SID_BYTES: usize = 8;

enum OpenFailure {
    Missing,
    Busy,
    Public(CredentialFileError),
}

enum CreateFailure {
    Exists,
    Public(CredentialFileError),
}

pub(super) fn load_or_create(path: &Path) -> Result<Zeroizing<String>, CredentialFileError> {
    validate_path(path)?;
    let _parent_locks = lock_parents(path)?;
    match open_existing(path) {
        Ok(file) => load(file),
        Err(OpenFailure::Public(error)) => Err(error),
        Err(OpenFailure::Busy) => Err(CredentialFileError::AccessFailed),
        Err(OpenFailure::Missing) => {
            let credential = fresh_credential()?;
            match create_new(path) {
                Ok(mut file) => {
                    verify_file(&file)?;
                    file.write_all(credential.as_bytes())
                        .and_then(|()| file.sync_all())
                        .map_err(|_| CredentialFileError::AccessFailed)?;
                    Ok(credential)
                }
                Err(CreateFailure::Exists) => open_existing(path)
                    .map_err(|failure| match failure {
                        OpenFailure::Missing
                        | OpenFailure::Busy
                        | OpenFailure::Public(CredentialFileError::AccessFailed) => {
                            CredentialFileError::AccessFailed
                        }
                        OpenFailure::Public(error) => error,
                    })
                    .and_then(load),
                Err(CreateFailure::Public(error)) => Err(error),
            }
        }
    }
}

fn validate_path(path: &Path) -> Result<(), CredentialFileError> {
    let encoded_units = path.as_os_str().encode_wide().count();
    if !path.is_absolute()
        || encoded_units == 0
        || encoded_units > MAX_PATH_BYTES
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
        || !matches!(path.components().next_back(), Some(Component::Normal(_)))
    {
        return Err(CredentialFileError::InvalidPath);
    }
    if path.as_os_str().encode_wide().any(|unit| unit == 0) {
        return Err(CredentialFileError::InvalidPath);
    }
    Ok(())
}

fn lock_parents(path: &Path) -> Result<Vec<OwnedHandle>, CredentialFileError> {
    let parent = path.parent().ok_or(CredentialFileError::InvalidPath)?;
    let mut ancestors: Vec<_> = parent
        .ancestors()
        .filter(|ancestor| !ancestor.as_os_str().is_empty())
        .collect();
    ancestors.reverse();
    ancestors
        .into_iter()
        .map(open_directory)
        .collect::<Result<Vec<_>, _>>()
}

fn open_directory(path: &Path) -> Result<OwnedHandle, CredentialFileError> {
    let wide = wide_path(path)?;
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        )
    };
    let handle = owned_handle(raw).map_err(|_| CredentialFileError::ParentUnavailable)?;
    let attributes = file_attributes(raw).map_err(|_| CredentialFileError::ParentUnavailable)?;
    if attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err(CredentialFileError::ParentUnavailable);
    }
    Ok(handle)
}

fn open_existing(path: &Path) -> Result<File, OpenFailure> {
    for _ in 0..PUBLICATION_RETRIES {
        match open_existing_once(path) {
            Err(OpenFailure::Busy) => std::thread::sleep(Duration::from_millis(1)),
            result => return result,
        }
    }
    Err(OpenFailure::Public(CredentialFileError::AccessFailed))
}

fn open_existing_once(path: &Path) -> Result<File, OpenFailure> {
    let wide = wide_path(path).map_err(OpenFailure::Public)?;
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_GENERIC_READ,
            0,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return match unsafe { GetLastError() } {
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Err(OpenFailure::Missing),
            ERROR_SHARING_VIOLATION => Err(OpenFailure::Busy),
            _ => Err(OpenFailure::Public(CredentialFileError::AccessFailed)),
        };
    }
    Ok(File::from(unsafe {
        OwnedHandle::from_raw_handle(raw as *mut c_void)
    }))
}

fn create_new(path: &Path) -> Result<File, CreateFailure> {
    let wide = wide_path(path).map_err(CreateFailure::Public)?;
    let mut security = OwnerSecurity::new().map_err(CreateFailure::Public)?;
    let attributes = security.attributes();
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            OWNER_ACCESS,
            0,
            &attributes,
            CREATE_NEW,
            FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return match unsafe { GetLastError() } {
            ERROR_ALREADY_EXISTS | ERROR_FILE_EXISTS => Err(CreateFailure::Exists),
            _ => Err(CreateFailure::Public(CredentialFileError::AccessFailed)),
        };
    }
    Ok(File::from(unsafe {
        OwnedHandle::from_raw_handle(raw as *mut c_void)
    }))
}

fn wide_path(path: &Path) -> Result<Vec<u16>, CredentialFileError> {
    let mut wide: Vec<_> = path.as_os_str().encode_wide().collect();
    if wide.is_empty() || wide.len() > MAX_PATH_BYTES || wide.contains(&0) {
        return Err(CredentialFileError::InvalidPath);
    }
    wide.push(0);
    Ok(wide)
}

fn owned_handle(raw: HANDLE) -> Result<OwnedHandle, ()> {
    if raw == INVALID_HANDLE_VALUE {
        Err(())
    } else {
        Ok(unsafe { OwnedHandle::from_raw_handle(raw as *mut c_void) })
    }
}

fn load(file: File) -> Result<Zeroizing<String>, CredentialFileError> {
    verify_file(&file)?;
    let mut bytes = Zeroizing::new(Vec::with_capacity(ENCODED_CREDENTIAL_BYTES + 1));
    file.take((ENCODED_CREDENTIAL_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| CredentialFileError::AccessFailed)?;
    validate_credential(&bytes)
}

fn verify_file(file: &File) -> Result<(), CredentialFileError> {
    let raw = file.as_raw_handle() as HANDLE;
    let attributes = file_attributes(raw)?;
    if attributes.FileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
        return Err(CredentialFileError::UnsafeFileType);
    }
    verify_owner_acl(raw)
}

fn file_attributes(raw: HANDLE) -> Result<FILE_ATTRIBUTE_TAG_INFO, CredentialFileError> {
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
        return Err(CredentialFileError::AccessFailed);
    }
    Ok(unsafe { information.assume_init() })
}

struct CurrentUserSid {
    _storage: Vec<usize>,
    sid: PSID,
}

impl CurrentUserSid {
    fn query() -> Result<Self, CredentialFileError> {
        let mut raw_token = 0;
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw_token) } == 0 {
            return Err(CredentialFileError::AccessFailed);
        }
        let _token = owned_handle(raw_token).map_err(|_| CredentialFileError::AccessFailed)?;
        let mut needed = 0;
        unsafe {
            GetTokenInformation(raw_token, TokenUser, null_mut(), 0, &mut needed);
        }
        if needed == 0 || needed > MAX_SECURITY_DESCRIPTOR_BYTES {
            return Err(CredentialFileError::AccessFailed);
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
            return Err(CredentialFileError::AccessFailed);
        }
        let token_user = storage.as_ptr().cast::<TOKEN_USER>();
        let sid = unsafe { (*token_user).User.Sid };
        if sid.is_null() || unsafe { GetLengthSid(sid) } == 0 {
            return Err(CredentialFileError::AccessFailed);
        }
        Ok(Self {
            _storage: storage,
            sid,
        })
    }
}

struct OwnerSecurity {
    _owner: CurrentUserSid,
    _acl_storage: Vec<u32>,
    descriptor: SECURITY_DESCRIPTOR,
}

impl OwnerSecurity {
    fn new() -> Result<Self, CredentialFileError> {
        let owner = CurrentUserSid::query()?;
        let sid_bytes = unsafe { GetLengthSid(owner.sid) } as usize;
        let acl_bytes =
            size_of::<ACL>() + size_of::<ACCESS_ALLOWED_ACE>() - size_of::<u32>() + sid_bytes;
        let mut acl_storage = vec![0u32; acl_bytes.div_ceil(size_of::<u32>())];
        let acl = acl_storage.as_mut_ptr().cast::<ACL>();
        if unsafe { InitializeAcl(acl, acl_bytes as u32, ACL_REVISION) } == 0
            || unsafe { AddAccessAllowedAce(acl, ACL_REVISION, OWNER_ACCESS, owner.sid) } == 0
        {
            return Err(CredentialFileError::AccessFailed);
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
            return Err(CredentialFileError::AccessFailed);
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

fn verify_owner_acl(raw: HANDLE) -> Result<(), CredentialFileError> {
    let current = CurrentUserSid::query()?;
    let information = OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
    let mut needed = 0;
    unsafe {
        GetKernelObjectSecurity(raw, information, null_mut(), 0, &mut needed);
    }
    if needed == 0 || needed > MAX_SECURITY_DESCRIPTOR_BYTES {
        return Err(CredentialFileError::AccessFailed);
    }
    let mut storage = vec![0usize; bytes_to_words(needed)];
    let descriptor: PSECURITY_DESCRIPTOR = storage.as_mut_ptr().cast();
    if unsafe { GetKernelObjectSecurity(raw, information, descriptor, needed, &mut needed) } == 0 {
        return Err(CredentialFileError::AccessFailed);
    }

    let mut owner = null_mut();
    let mut owner_defaulted: BOOL = 0;
    if unsafe { GetSecurityDescriptorOwner(descriptor, &mut owner, &mut owner_defaulted) } == 0 {
        return Err(CredentialFileError::AccessFailed);
    }
    if owner.is_null() || unsafe { EqualSid(owner, current.sid) } == 0 {
        return Err(CredentialFileError::InsecureOwner);
    }

    let mut control = 0;
    let mut revision = 0;
    if unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) } == 0 {
        return Err(CredentialFileError::AccessFailed);
    }
    if control & SE_DACL_PROTECTED == 0 {
        return Err(CredentialFileError::InsecurePermissions);
    }

    let mut present: BOOL = 0;
    let mut dacl: *mut ACL = null_mut();
    let mut defaulted: BOOL = 0;
    if unsafe { GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted) }
        == 0
    {
        return Err(CredentialFileError::AccessFailed);
    }
    if present == 0 || dacl.is_null() {
        return Err(CredentialFileError::InsecurePermissions);
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
    {
        return Err(CredentialFileError::AccessFailed);
    }
    if unsafe { acl_information.assume_init() }.AceCount != 1 {
        return Err(CredentialFileError::InsecurePermissions);
    }

    let mut ace_pointer: *mut c_void = null_mut();
    if unsafe { GetAce(dacl, 0, &mut ace_pointer) } == 0 || ace_pointer.is_null() {
        return Err(CredentialFileError::AccessFailed);
    }
    let ace = ace_pointer.cast::<ACCESS_ALLOWED_ACE>();
    let ace = unsafe { &*ace };
    let sid_offset = size_of::<ACCESS_ALLOWED_ACE>() - size_of::<u32>();
    if usize::from(ace.Header.AceSize) < sid_offset + MINIMUM_SID_BYTES {
        return Err(CredentialFileError::InsecurePermissions);
    }
    let sid = addr_of!(ace.SidStart).cast_mut().cast();
    let sid_bytes = unsafe { GetLengthSid(sid) } as usize;
    let minimum_size = sid_offset + sid_bytes;
    if u32::from(ace.Header.AceType) != ACCESS_ALLOWED_ACE_TYPE
        || ace.Header.AceFlags != 0
        || sid_bytes < MINIMUM_SID_BYTES
        || usize::from(ace.Header.AceSize) < minimum_size
        || ace.Mask != OWNER_ACCESS
        || unsafe { EqualSid(sid, current.sid) } == 0
    {
        return Err(CredentialFileError::InsecurePermissions);
    }
    Ok(())
}

fn bytes_to_words(bytes: u32) -> usize {
    (bytes as usize).div_ceil(size_of::<usize>())
}
