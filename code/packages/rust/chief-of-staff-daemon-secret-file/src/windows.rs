use super::{SecretFileError, MAX_PATH_BYTES};
use coding_adventures_zeroize::Zeroizing;
use std::ffi::c_void;
use std::fs::File;
use std::io::Read;
#[cfg(test)]
use std::io::Write;
use std::mem::{size_of, MaybeUninit};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Component, Path};
use std::ptr::{addr_of, null, null_mut};
use windows_sys::Win32::Foundation::{BOOL, HANDLE, INVALID_HANDLE_VALUE, PSID};
use windows_sys::Win32::Security::{
    AclSizeInformation, EqualSid, GetAce, GetAclInformation, GetKernelObjectSecurity, GetLengthSid,
    GetSecurityDescriptorControl, GetSecurityDescriptorDacl, GetSecurityDescriptorOwner,
    GetTokenInformation, TokenUser, ACCESS_ALLOWED_ACE, ACL, ACL_SIZE_INFORMATION,
    DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SE_DACL_PROTECTED,
    TOKEN_QUERY, TOKEN_USER,
};
#[cfg(test)]
use windows_sys::Win32::Security::{
    AddAccessAllowedAce, InitializeAcl, InitializeSecurityDescriptor, SetSecurityDescriptorControl,
    SetSecurityDescriptorDacl, SetSecurityDescriptorOwner, ACL_REVISION, SECURITY_ATTRIBUTES,
    SECURITY_DESCRIPTOR,
};
#[cfg(test)]
use windows_sys::Win32::Storage::FileSystem::FILE_GENERIC_WRITE;
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FileAttributeTagInfo, GetFileInformationByHandleEx, FILE_ATTRIBUTE_DIRECTORY,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ, FILE_READ_ATTRIBUTES, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::SystemServices::ACCESS_ALLOWED_ACE_TYPE;
#[cfg(test)]
use windows_sys::Win32::System::SystemServices::SECURITY_DESCRIPTOR_REVISION;
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

#[cfg(test)]
const OWNER_ACCESS: u32 = FILE_GENERIC_READ | FILE_GENERIC_WRITE;
const MAX_SECURITY_DESCRIPTOR_BYTES: u32 = 64 * 1024;
const MINIMUM_SID_BYTES: usize = 8;

pub(super) fn read(
    path: &Path,
    expected_length: usize,
) -> Result<Zeroizing<Vec<u8>>, SecretFileError> {
    validate_path(path)?;
    let _parent_locks = lock_parents(path)?;
    let wide = wide_path(path)?;
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_GENERIC_READ,
            0,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            0,
        )
    };
    let file = File::from(owned_handle(raw).map_err(|_| SecretFileError::AccessFailed)?);
    verify_file(&file)?;
    let mut bytes = Zeroizing::new(Vec::with_capacity(expected_length + 1));
    file.take((expected_length + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| SecretFileError::AccessFailed)?;
    if bytes.len() != expected_length {
        return Err(SecretFileError::InvalidLength);
    }
    Ok(bytes)
}

fn validate_path(path: &Path) -> Result<(), SecretFileError> {
    let encoded_units = path.as_os_str().encode_wide().count();
    if !path.is_absolute()
        || encoded_units == 0
        || encoded_units > MAX_PATH_BYTES
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
        || !matches!(path.components().next_back(), Some(Component::Normal(_)))
        || path.as_os_str().encode_wide().any(|unit| unit == 0)
    {
        return Err(SecretFileError::InvalidPath);
    }
    Ok(())
}

fn lock_parents(path: &Path) -> Result<Vec<OwnedHandle>, SecretFileError> {
    let parent = path.parent().ok_or(SecretFileError::InvalidPath)?;
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

fn open_directory(path: &Path) -> Result<OwnedHandle, SecretFileError> {
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
    let handle = owned_handle(raw).map_err(|_| SecretFileError::ParentUnavailable)?;
    let attributes = file_attributes(raw).map_err(|_| SecretFileError::ParentUnavailable)?;
    if attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err(SecretFileError::ParentUnavailable);
    }
    Ok(handle)
}

fn wide_path(path: &Path) -> Result<Vec<u16>, SecretFileError> {
    let mut wide: Vec<_> = path.as_os_str().encode_wide().collect();
    if wide.is_empty() || wide.len() > MAX_PATH_BYTES || wide.contains(&0) {
        return Err(SecretFileError::InvalidPath);
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

fn verify_file(file: &File) -> Result<(), SecretFileError> {
    let raw = file.as_raw_handle() as HANDLE;
    let attributes = file_attributes(raw)?;
    if attributes.FileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
        return Err(SecretFileError::UnsafeFileType);
    }
    verify_owner_acl(raw)
}

fn file_attributes(raw: HANDLE) -> Result<FILE_ATTRIBUTE_TAG_INFO, SecretFileError> {
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
        return Err(SecretFileError::AccessFailed);
    }
    Ok(unsafe { information.assume_init() })
}

struct CurrentUserSid {
    _storage: Vec<usize>,
    sid: PSID,
}

impl CurrentUserSid {
    fn query() -> Result<Self, SecretFileError> {
        let mut raw_token = 0;
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw_token) } == 0 {
            return Err(SecretFileError::AccessFailed);
        }
        let _token = owned_handle(raw_token).map_err(|_| SecretFileError::AccessFailed)?;
        let mut needed = 0;
        unsafe {
            GetTokenInformation(raw_token, TokenUser, null_mut(), 0, &mut needed);
        }
        if needed == 0 || needed > MAX_SECURITY_DESCRIPTOR_BYTES {
            return Err(SecretFileError::AccessFailed);
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
            return Err(SecretFileError::AccessFailed);
        }
        let token_user = storage.as_ptr().cast::<TOKEN_USER>();
        let sid = unsafe { (*token_user).User.Sid };
        if sid.is_null() || unsafe { GetLengthSid(sid) } == 0 {
            return Err(SecretFileError::AccessFailed);
        }
        Ok(Self {
            _storage: storage,
            sid,
        })
    }
}

fn verify_owner_acl(raw: HANDLE) -> Result<(), SecretFileError> {
    let current = CurrentUserSid::query()?;
    let information = OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
    let mut needed = 0;
    unsafe {
        GetKernelObjectSecurity(raw, information, null_mut(), 0, &mut needed);
    }
    if needed == 0 || needed > MAX_SECURITY_DESCRIPTOR_BYTES {
        return Err(SecretFileError::AccessFailed);
    }
    let mut storage = vec![0usize; bytes_to_words(needed)];
    let descriptor: PSECURITY_DESCRIPTOR = storage.as_mut_ptr().cast();
    if unsafe { GetKernelObjectSecurity(raw, information, descriptor, needed, &mut needed) } == 0 {
        return Err(SecretFileError::AccessFailed);
    }
    let mut owner = null_mut();
    let mut owner_defaulted: BOOL = 0;
    if unsafe { GetSecurityDescriptorOwner(descriptor, &mut owner, &mut owner_defaulted) } == 0 {
        return Err(SecretFileError::AccessFailed);
    }
    if owner.is_null() || unsafe { EqualSid(owner, current.sid) } == 0 {
        return Err(SecretFileError::InsecureOwner);
    }
    let mut control = 0;
    let mut revision = 0;
    if unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) } == 0 {
        return Err(SecretFileError::AccessFailed);
    }
    if control & SE_DACL_PROTECTED == 0 {
        return Err(SecretFileError::InsecurePermissions);
    }
    let mut present: BOOL = 0;
    let mut dacl: *mut ACL = null_mut();
    let mut defaulted: BOOL = 0;
    if unsafe { GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted) }
        == 0
        || present == 0
        || dacl.is_null()
    {
        return Err(SecretFileError::InsecurePermissions);
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
        return Err(SecretFileError::InsecurePermissions);
    }
    let mut ace_pointer: *mut c_void = null_mut();
    if unsafe { GetAce(dacl, 0, &mut ace_pointer) } == 0 || ace_pointer.is_null() {
        return Err(SecretFileError::AccessFailed);
    }
    let ace = unsafe { &*ace_pointer.cast::<ACCESS_ALLOWED_ACE>() };
    let sid_offset = size_of::<ACCESS_ALLOWED_ACE>() - size_of::<u32>();
    if usize::from(ace.Header.AceSize) < sid_offset + MINIMUM_SID_BYTES {
        return Err(SecretFileError::InsecurePermissions);
    }
    let sid = addr_of!(ace.SidStart).cast_mut().cast();
    let sid_bytes = unsafe { GetLengthSid(sid) } as usize;
    if u32::from(ace.Header.AceType) != ACCESS_ALLOWED_ACE_TYPE
        || ace.Header.AceFlags != 0
        || sid_bytes < MINIMUM_SID_BYTES
        || usize::from(ace.Header.AceSize) < sid_offset + sid_bytes
        || unsafe { EqualSid(sid, current.sid) } == 0
    {
        return Err(SecretFileError::InsecurePermissions);
    }
    Ok(())
}

fn bytes_to_words(bytes: u32) -> usize {
    (bytes as usize).div_ceil(size_of::<usize>())
}

#[cfg(test)]
struct OwnerSecurity {
    _owner: CurrentUserSid,
    _acl_storage: Vec<u32>,
    descriptor: SECURITY_DESCRIPTOR,
}

#[cfg(test)]
impl OwnerSecurity {
    fn new() -> Result<Self, SecretFileError> {
        let owner = CurrentUserSid::query()?;
        let sid_bytes = unsafe { GetLengthSid(owner.sid) } as usize;
        let acl_bytes =
            size_of::<ACL>() + size_of::<ACCESS_ALLOWED_ACE>() - size_of::<u32>() + sid_bytes;
        let mut acl_storage = vec![0u32; acl_bytes.div_ceil(size_of::<u32>())];
        let acl = acl_storage.as_mut_ptr().cast::<ACL>();
        if unsafe { InitializeAcl(acl, acl_bytes as u32, ACL_REVISION) } == 0
            || unsafe { AddAccessAllowedAce(acl, ACL_REVISION, OWNER_ACCESS, owner.sid) } == 0
        {
            return Err(SecretFileError::AccessFailed);
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
            return Err(SecretFileError::AccessFailed);
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

#[cfg(test)]
pub(super) fn write_test_secret(path: &Path, bytes: &[u8]) {
    use windows_sys::Win32::Storage::FileSystem::CREATE_NEW;

    validate_path(path).unwrap();
    let _parent_locks = lock_parents(path).unwrap();
    let wide = wide_path(path).unwrap();
    let mut security = OwnerSecurity::new().unwrap();
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
    let mut file = File::from(owned_handle(raw).unwrap());
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
