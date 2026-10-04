//! Native-owned local publication targets and atomic complete-tree commits.

use crate::worker::WorkerRun;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use uuid::Uuid;

const OWNER_FILE: &str = ".forme-owner";
const MAX_TARGETS: usize = 32;
const MAX_TREE_ENTRIES: usize = 20_000;
const MAX_TREE_DIRECTORIES: usize = 10_000;
const MAX_TREE_DEPTH: usize = 64;
const MAX_TREE_PATH_BYTES: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TargetError {
    #[error("publication target request is invalid")]
    Invalid,
    #[error("publication target is unsafe or stale")]
    Unsafe,
    #[error("publication failed before commit")]
    Failed,
    #[error("publication outcome is indeterminate")]
    Indeterminate,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetReview {
    pub target_id: String,
    pub label: String,
    pub destination: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationReceipt {
    pub target_id: String,
    pub manifest_sha256: String,
}

#[derive(Debug)]
pub struct NativeTargets {
    targets: Mutex<Vec<LocalTarget>>,
    forbidden: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
struct LocalTarget {
    review: TargetReview,
    root: PathBuf,
    identity: DirectoryIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DirectoryIdentity {
    canonical: PathBuf,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl NativeTargets {
    pub fn new(forbidden: Vec<PathBuf>) -> Result<Self, TargetError> {
        let mut canonical = Vec::new();
        for path in forbidden {
            if path.exists() {
                canonical.push(fs::canonicalize(path).map_err(|_| TargetError::Unsafe)?);
            }
        }
        Ok(Self {
            targets: Mutex::new(Vec::new()),
            forbidden: canonical,
        })
    }

    pub fn configure_path(&self, selected: PathBuf) -> Result<TargetReview, TargetError> {
        let identity = directory_identity(&selected)?;
        if self.forbidden.iter().any(|path| {
            identity.canonical == *path
                || identity.canonical.starts_with(path)
                || path.starts_with(&identity.canonical)
        }) {
            return Err(TargetError::Unsafe);
        }
        let mut targets = self
            .targets
            .lock()
            .map_err(|_| TargetError::Indeterminate)?;
        if targets.len() >= MAX_TARGETS {
            return Err(TargetError::Invalid);
        }
        let raw_name = identity
            .canonical
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or(TargetError::Unsafe)?;
        let name = Some(raw_name)
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= 255
                    && value.chars().all(|character| {
                        character.is_ascii_alphanumeric() || " ._-".contains(character)
                    })
            })
            .unwrap_or("selected folder");
        let target_id = format!("local-{}", Uuid::now_v7().hyphenated());
        let review = TargetReview {
            target_id: target_id.clone(),
            label: "Local folder".to_owned(),
            destination: format!("Local folder “{name}”"),
        };
        targets.push(LocalTarget {
            review: review.clone(),
            root: identity.canonical.clone(),
            identity,
        });
        Ok(review)
    }

    pub fn list(&self) -> Result<Vec<TargetReview>, TargetError> {
        let targets = self
            .targets
            .lock()
            .map_err(|_| TargetError::Indeterminate)?;
        Ok(targets.iter().map(|target| target.review.clone()).collect())
    }

    pub fn clear(&self) -> Result<(), TargetError> {
        self.targets
            .lock()
            .map_err(|_| TargetError::Indeterminate)?
            .clear();
        Ok(())
    }

    pub fn publish(
        &self,
        target_id: &str,
        run: &WorkerRun,
    ) -> Result<PublicationReceipt, TargetError> {
        if !valid_target_id(target_id) {
            return Err(TargetError::Invalid);
        }
        let mut targets = self
            .targets
            .lock()
            .map_err(|_| TargetError::Indeterminate)?;
        let target = targets
            .iter_mut()
            .find(|candidate| candidate.review.target_id == target_id)
            .ok_or(TargetError::Invalid)?;
        if directory_identity(&target.root)? != target.identity {
            return Err(TargetError::Unsafe);
        }
        verify_owned_or_empty(&target.root, target_id)?;
        let new_identity = publish_tree(target, run)?;
        target.identity = new_identity;
        Ok(PublicationReceipt {
            target_id: target_id.to_owned(),
            manifest_sha256: run.output().manifest_sha256.clone(),
        })
    }
}

fn directory_identity(path: &Path) -> Result<DirectoryIdentity, TargetError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| TargetError::Unsafe)?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(TargetError::Unsafe);
    }
    let canonical = fs::canonicalize(path).map_err(|_| TargetError::Unsafe)?;
    let canonical_metadata = fs::symlink_metadata(&canonical).map_err(|_| TargetError::Unsafe)?;
    if !canonical_metadata.file_type().is_dir() || canonical_metadata.file_type().is_symlink() {
        return Err(TargetError::Unsafe);
    }
    validate_directory_security(&canonical, &canonical_metadata)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(DirectoryIdentity {
            canonical,
            device: canonical_metadata.dev(),
            inode: canonical_metadata.ino(),
        })
    }
    #[cfg(not(unix))]
    Ok(DirectoryIdentity { canonical })
}

#[cfg(unix)]
fn validate_directory_security(
    canonical: &Path,
    target_metadata: &fs::Metadata,
) -> Result<(), TargetError> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if target_metadata.uid() != unsafe { libc::geteuid() }
        || target_metadata.permissions().mode() & 0o022 != 0
        || !crate::security::ancestors_have_no_mutating_acl(canonical)
    {
        return Err(TargetError::Unsafe);
    }
    for ancestor in canonical.ancestors().skip(1) {
        let metadata = fs::symlink_metadata(ancestor).map_err(|_| TargetError::Unsafe)?;
        if !metadata.file_type().is_dir()
            || metadata.file_type().is_symlink()
            || metadata.permissions().mode() & 0o022 != 0
        {
            return Err(TargetError::Unsafe);
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_directory_security(
    _canonical: &Path,
    _target_metadata: &fs::Metadata,
) -> Result<(), TargetError> {
    Err(TargetError::Unsafe)
}

fn verify_owned_or_empty(root: &Path, target_id: &str) -> Result<(), TargetError> {
    let entries = fs::read_dir(root).map_err(|_| TargetError::Unsafe)?;
    let mut count = 0_usize;
    let mut owned = false;
    for entry in entries {
        let entry = entry.map_err(|_| TargetError::Unsafe)?;
        count = count.checked_add(1).ok_or(TargetError::Unsafe)?;
        if count > 10_001 {
            return Err(TargetError::Unsafe);
        }
        owned |= entry.file_name() == OWNER_FILE;
    }
    if count == 0 {
        return Ok(());
    }
    if !owned {
        return Err(TargetError::Unsafe);
    }
    let owner_path = root.join(OWNER_FILE);
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(owner_path).map_err(|_| TargetError::Unsafe)?;
    let metadata = file.metadata().map_err(|_| TargetError::Unsafe)?;
    if !metadata.file_type().is_file() || metadata.len() > 512 {
        return Err(TargetError::Unsafe);
    }
    let mut owner = String::new();
    file.take(513)
        .read_to_string(&mut owner)
        .map_err(|_| TargetError::Unsafe)?;
    if owner != target_id {
        return Err(TargetError::Unsafe);
    }
    validate_existing_tree(root)
}

fn validate_existing_tree(root: &Path) -> Result<(), TargetError> {
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    let root_metadata = fs::symlink_metadata(root).map_err(|_| TargetError::Unsafe)?;
    #[cfg(unix)]
    let root_device = root_metadata.dev();
    let mut pending = vec![(root.to_path_buf(), 0_usize, 0_usize)];
    let mut entries = 0_usize;
    let mut directories = 0_usize;
    while let Some((directory, depth, path_bytes)) = pending.pop() {
        directories = directories.checked_add(1).ok_or(TargetError::Unsafe)?;
        if depth > MAX_TREE_DEPTH || directories > MAX_TREE_DIRECTORIES {
            return Err(TargetError::Unsafe);
        }
        for entry in fs::read_dir(directory).map_err(|_| TargetError::Unsafe)? {
            let entry = entry.map_err(|_| TargetError::Unsafe)?;
            entries = entries.checked_add(1).ok_or(TargetError::Unsafe)?;
            let name_bytes = entry.file_name().as_encoded_bytes().len();
            let child_path_bytes = path_bytes
                .checked_add(name_bytes + usize::from(path_bytes != 0))
                .ok_or(TargetError::Unsafe)?;
            if entries > MAX_TREE_ENTRIES || child_path_bytes > MAX_TREE_PATH_BYTES {
                return Err(TargetError::Unsafe);
            }
            let metadata = fs::symlink_metadata(entry.path()).map_err(|_| TargetError::Unsafe)?;
            if metadata.file_type().is_symlink() {
                return Err(TargetError::Unsafe);
            }
            #[cfg(unix)]
            if metadata.dev() != root_device || metadata.uid() != unsafe { libc::geteuid() } {
                return Err(TargetError::Unsafe);
            }
            #[cfg(target_os = "macos")]
            {
                use std::os::unix::fs::OpenOptionsExt;
                let mut options = OpenOptions::new();
                options
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
                if metadata.file_type().is_dir() {
                    options.custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC);
                }
                let opened = options
                    .open(entry.path())
                    .map_err(|_| TargetError::Unsafe)?;
                if !crate::security::has_no_mutating_acl(&opened) {
                    return Err(TargetError::Unsafe);
                }
            }
            if metadata.file_type().is_dir() {
                pending.push((entry.path(), depth + 1, child_path_bytes));
            } else if !metadata.file_type().is_file() {
                return Err(TargetError::Unsafe);
            }
        }
    }
    Ok(())
}

fn publish_tree(target: &LocalTarget, run: &WorkerRun) -> Result<DirectoryIdentity, TargetError> {
    let parent = target.root.parent().ok_or(TargetError::Unsafe)?;
    #[cfg(target_os = "macos")]
    let parent_handle = open_parent(target)?;
    let nonce = Uuid::now_v7().hyphenated().to_string();
    let stage_name = format!(".forme-stage-{nonce}");
    let stage = parent.join(&stage_name);
    #[cfg(not(target_os = "macos"))]
    let backup = parent.join(format!(".forme-backup-{nonce}"));
    #[cfg(target_os = "macos")]
    let (stage_handle, stage_identity) =
        create_private_directory_at(&parent_handle, &stage_name, &stage)?;
    #[cfg(not(target_os = "macos"))]
    fs::create_dir(&stage).map_err(|_| TargetError::Failed)?;
    #[cfg(target_os = "macos")]
    let prepared = prepare_stage_at(&stage_handle, target, run);
    #[cfg(not(target_os = "macos"))]
    let prepared = prepare_stage(&stage, target, run);
    if let Err(error) = prepared {
        #[cfg(target_os = "macos")]
        if remove_tree_at(&parent_handle, &stage_name, &stage_identity).is_err() {
            return Err(TargetError::Indeterminate);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = fs::remove_dir_all(&stage);
        return Err(error);
    }
    #[cfg(target_os = "macos")]
    {
        let ready = (|| {
            verify_owned_or_empty(&target.root, &target.review.target_id)?;
            verify_target_at(&parent_handle, target)?;
            verify_directory_entry(&parent_handle, &stage_name, &stage_handle, &stage_identity)?;
            target
                .root
                .file_name()
                .and_then(|value| value.to_str())
                .map(str::to_owned)
                .ok_or(TargetError::Unsafe)
        })();
        let target_name = match ready {
            Ok(name) => name,
            Err(error) => {
                if remove_tree_at(&parent_handle, &stage_name, &stage_identity).is_err() {
                    return Err(TargetError::Indeterminate);
                }
                return Err(error);
            }
        };
        if atomic_exchange_at(&parent_handle, &target_name, &stage_name).is_err() {
            if remove_tree_at(&parent_handle, &stage_name, &stage_identity).is_err() {
                return Err(TargetError::Indeterminate);
            }
            return Err(TargetError::Failed);
        }
        verify_directory_entry(&parent_handle, &target_name, &stage_handle, &stage_identity)
            .map_err(|_| TargetError::Indeterminate)?;
        let new_identity = DirectoryIdentity {
            canonical: target.root.clone(),
            device: stage_identity.device,
            inode: stage_identity.inode,
        };
        if remove_tree_at(&parent_handle, &stage_name, &target.identity).is_err() {
            return Err(TargetError::Indeterminate);
        }
        sync_directory(parent).map_err(|_| TargetError::Indeterminate)?;
        Ok(new_identity)
    }
    #[cfg(not(target_os = "macos"))]
    if fs::rename(&target.root, &backup).is_err() {
        let _ = fs::remove_dir_all(&stage);
        return Err(TargetError::Failed);
    }
    #[cfg(not(target_os = "macos"))]
    if fs::rename(&stage, &target.root).is_err() {
        if fs::rename(&backup, &target.root).is_err() {
            return Err(TargetError::Indeterminate);
        }
        let _ = fs::remove_dir_all(&stage);
        return Err(TargetError::Failed);
    }
    #[cfg(not(target_os = "macos"))]
    let new_identity = directory_identity(&target.root).map_err(|_| TargetError::Indeterminate)?;
    #[cfg(not(target_os = "macos"))]
    if fs::remove_dir_all(&backup).is_err() {
        return Err(TargetError::Indeterminate);
    }
    #[cfg(not(target_os = "macos"))]
    {
        sync_directory(parent).map_err(|_| TargetError::Indeterminate)?;
        Ok(new_identity)
    }
}

#[cfg(target_os = "macos")]
fn open_parent(target: &LocalTarget) -> Result<File, TargetError> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let parent = target.root.parent().ok_or(TargetError::Unsafe)?;
    let expected = directory_identity(parent)?;
    let handle = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(parent)
        .map_err(|_| TargetError::Unsafe)?;
    let metadata = handle.metadata().map_err(|_| TargetError::Unsafe)?;
    if metadata.dev() != expected.device || metadata.ino() != expected.inode {
        return Err(TargetError::Unsafe);
    }
    Ok(handle)
}

#[cfg(target_os = "macos")]
fn create_private_directory_at(
    parent: &File,
    name: &str,
    canonical: &Path,
) -> Result<(File, DirectoryIdentity), TargetError> {
    use std::{
        ffi::CString,
        os::fd::{AsRawFd, FromRawFd},
    };
    let name_text = name;
    let name = CString::new(name_text).map_err(|_| TargetError::Failed)?;
    let result = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) };
    if result != 0 {
        return Err(TargetError::Failed);
    }
    let mut named = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            named.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        let removed =
            unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) } == 0
                && parent.sync_all().is_ok();
        if removed {
            return Err(TargetError::Failed);
        }
        return Err(TargetError::Indeterminate);
    }
    let named = unsafe { named.assume_init() };
    let identity = DirectoryIdentity {
        canonical: canonical.to_path_buf(),
        device: named.st_dev as u64,
        inode: named.st_ino,
    };
    let opened = (|| {
        parent.sync_all().map_err(|_| TargetError::Failed)?;
        let descriptor = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if descriptor < 0 {
            return Err(TargetError::Failed);
        }
        let directory = unsafe { File::from_raw_fd(descriptor) };
        let metadata = directory.metadata().map_err(|_| TargetError::Failed)?;
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if metadata.dev() != identity.device
            || metadata.ino() != identity.inode
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.permissions().mode() & 0o077 != 0
            || !crate::security::has_no_mutating_acl(&directory)
        {
            return Err(TargetError::Failed);
        }
        Ok(directory)
    })();
    match opened {
        Ok(directory) => Ok((directory, identity)),
        Err(error) => {
            if remove_tree_at(parent, name_text, &identity).is_err() {
                Err(TargetError::Indeterminate)
            } else {
                Err(error)
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn verify_directory_entry(
    parent: &File,
    name: &str,
    open: &File,
    expected: &DirectoryIdentity,
) -> Result<(), TargetError> {
    use std::{ffi::CString, mem::MaybeUninit, os::fd::AsRawFd};
    let name = CString::new(name).map_err(|_| TargetError::Unsafe)?;
    let mut named = MaybeUninit::<libc::stat>::uninit();
    let mut opened = MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            named.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
        || unsafe { libc::fstat(open.as_raw_fd(), opened.as_mut_ptr()) } != 0
    {
        return Err(TargetError::Unsafe);
    }
    let named = unsafe { named.assume_init() };
    let opened = unsafe { opened.assume_init() };
    if !same_entry(&named, &opened)
        || named.st_mode & libc::S_IFMT != libc::S_IFDIR
        || named.st_dev as u64 != expected.device
        || named.st_ino != expected.inode
        || named.st_uid != unsafe { libc::geteuid() }
        || named.st_mode & 0o077 != 0
        || !crate::security::has_no_mutating_acl(open)
    {
        return Err(TargetError::Unsafe);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn same_entry(left: &libc::stat, right: &libc::stat) -> bool {
    left.st_dev == right.st_dev
        && left.st_ino == right.st_ino
        && left.st_uid == right.st_uid
        && left.st_mode & libc::S_IFMT == right.st_mode & libc::S_IFMT
}

#[cfg(target_os = "macos")]
fn verify_target_at(parent: &File, target: &LocalTarget) -> Result<(), TargetError> {
    use std::{
        ffi::CString,
        mem::MaybeUninit,
        os::fd::{AsRawFd, FromRawFd},
    };
    let name = target
        .root
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(TargetError::Unsafe)?;
    let name = CString::new(name).map_err(|_| TargetError::Unsafe)?;
    let mut status = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            status.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return Err(TargetError::Unsafe);
    }
    let status = unsafe { status.assume_init() };
    if status.st_mode & libc::S_IFMT != libc::S_IFDIR
        || status.st_dev as u64 != target.identity.device
        || status.st_ino != target.identity.inode
        || status.st_uid != unsafe { libc::geteuid() }
        || status.st_mode & 0o022 != 0
    {
        return Err(TargetError::Unsafe);
    }
    let descriptor = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(TargetError::Unsafe);
    }
    let directory = unsafe { File::from_raw_fd(descriptor) };
    let mut opened = MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(directory.as_raw_fd(), opened.as_mut_ptr()) } != 0
        || !same_entry(&status, &unsafe { opened.assume_init() })
        || !crate::security::has_no_mutating_acl(&directory)
    {
        return Err(TargetError::Unsafe);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn atomic_exchange_at(parent: &File, left: &str, right: &str) -> std::io::Result<()> {
    use std::{ffi::CString, os::fd::AsRawFd};
    let left = CString::new(left).map_err(|_| std::io::Error::other("invalid exchange name"))?;
    let right = CString::new(right).map_err(|_| std::io::Error::other("invalid exchange name"))?;
    let result = unsafe {
        libc::renameatx_np(
            parent.as_raw_fd(),
            left.as_ptr(),
            parent.as_raw_fd(),
            right.as_ptr(),
            libc::RENAME_SWAP,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(target_os = "macos")]
fn remove_tree_at(parent: &File, name: &str, expected: &DirectoryIdentity) -> std::io::Result<()> {
    use std::{
        ffi::CString,
        mem::MaybeUninit,
        os::fd::{AsRawFd, FromRawFd},
    };
    let name = CString::new(name).map_err(|_| std::io::Error::other("invalid tree name"))?;
    let descriptor = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let root = unsafe { File::from_raw_fd(descriptor) };
    let mut status = MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(root.as_raw_fd(), status.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let status = unsafe { status.assume_init() };
    if status.st_dev as u64 != expected.device
        || status.st_ino != expected.inode
        || status.st_uid != unsafe { libc::geteuid() }
        || !crate::security::has_no_mutating_acl(&root)
    {
        return Err(std::io::Error::other("old publication identity changed"));
    }
    let mut budget = RemovalBudget::default();
    remove_directory_contents(&root, status.st_dev, 0, 0, &mut budget)?;
    let mut named = MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            named.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
        || !same_entry(&status, &unsafe { named.assume_init() })
    {
        return Err(std::io::Error::other(
            "old publication identity changed before removal",
        ));
    }
    if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    parent.sync_all()?;
    Ok(())
}

#[cfg(target_os = "macos")]
#[derive(Default)]
struct RemovalBudget {
    entries: usize,
    directories: usize,
}

#[cfg(target_os = "macos")]
fn remove_directory_contents(
    directory: &File,
    root_device: libc::dev_t,
    depth: usize,
    path_bytes: usize,
    budget: &mut RemovalBudget,
) -> std::io::Result<()> {
    use std::{
        ffi::CStr,
        os::fd::{AsRawFd, FromRawFd},
    };
    budget.directories = budget
        .directories
        .checked_add(1)
        .ok_or_else(|| std::io::Error::other("publication directory overflow"))?;
    if depth > MAX_TREE_DEPTH || budget.directories > MAX_TREE_DIRECTORIES {
        return Err(std::io::Error::other(
            "publication directory budget exceeded",
        ));
    }
    let duplicate = unsafe { libc::dup(directory.as_raw_fd()) };
    if duplicate < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let stream = unsafe { libc::fdopendir(duplicate) };
    if stream.is_null() {
        unsafe { libc::close(duplicate) };
        return Err(std::io::Error::last_os_error());
    }
    let mut names = Vec::new();
    loop {
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if name.to_bytes() == b"." || name.to_bytes() == b".." {
            continue;
        }
        budget.entries = budget
            .entries
            .checked_add(1)
            .ok_or_else(|| std::io::Error::other("publication entry overflow"))?;
        if budget.entries > MAX_TREE_ENTRIES {
            unsafe { libc::closedir(stream) };
            return Err(std::io::Error::other("publication entry budget exceeded"));
        }
        names.push(name.to_owned());
    }
    if unsafe { libc::closedir(stream) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    for name in names {
        let child_path_bytes = path_bytes
            .checked_add(name.to_bytes().len() + usize::from(path_bytes != 0))
            .ok_or_else(|| std::io::Error::other("publication path overflow"))?;
        if child_path_bytes > MAX_TREE_PATH_BYTES {
            return Err(std::io::Error::other("publication path budget exceeded"));
        }
        let mut status = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                directory.as_raw_fd(),
                name.as_ptr(),
                status.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error());
        }
        let status = unsafe { status.assume_init() };
        if status.st_dev != root_device || status.st_uid != unsafe { libc::geteuid() } {
            return Err(std::io::Error::other(
                "publication entry crossed its device or owner",
            ));
        }
        match status.st_mode & libc::S_IFMT {
            libc::S_IFDIR => {
                let child = unsafe {
                    libc::openat(
                        directory.as_raw_fd(),
                        name.as_ptr(),
                        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                    )
                };
                if child < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                let child = unsafe { File::from_raw_fd(child) };
                let mut opened = std::mem::MaybeUninit::<libc::stat>::uninit();
                if unsafe { libc::fstat(child.as_raw_fd(), opened.as_mut_ptr()) } != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                let opened = unsafe { opened.assume_init() };
                if !same_entry(&status, &opened) || !crate::security::has_no_mutating_acl(&child) {
                    return Err(std::io::Error::other(
                        "publication directory identity changed",
                    ));
                }
                remove_directory_contents(
                    &child,
                    root_device,
                    depth + 1,
                    child_path_bytes,
                    budget,
                )?;
                let mut named = std::mem::MaybeUninit::<libc::stat>::uninit();
                if unsafe {
                    libc::fstatat(
                        directory.as_raw_fd(),
                        name.as_ptr(),
                        named.as_mut_ptr(),
                        libc::AT_SYMLINK_NOFOLLOW,
                    )
                } != 0
                    || !same_entry(&opened, &unsafe { named.assume_init() })
                {
                    return Err(std::io::Error::other(
                        "publication directory changed before removal",
                    ));
                }
                if unsafe {
                    libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR)
                } != 0
                {
                    return Err(std::io::Error::last_os_error());
                }
            }
            libc::S_IFREG => {
                let child = unsafe {
                    libc::openat(
                        directory.as_raw_fd(),
                        name.as_ptr(),
                        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                    )
                };
                if child < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                let child = unsafe { File::from_raw_fd(child) };
                let mut opened = std::mem::MaybeUninit::<libc::stat>::uninit();
                if unsafe { libc::fstat(child.as_raw_fd(), opened.as_mut_ptr()) } != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                let opened = unsafe { opened.assume_init() };
                if !same_entry(&status, &opened) || !crate::security::has_no_mutating_acl(&child) {
                    return Err(std::io::Error::other("publication file identity changed"));
                }
                let mut named = std::mem::MaybeUninit::<libc::stat>::uninit();
                if unsafe {
                    libc::fstatat(
                        directory.as_raw_fd(),
                        name.as_ptr(),
                        named.as_mut_ptr(),
                        libc::AT_SYMLINK_NOFOLLOW,
                    )
                } != 0
                    || !same_entry(&opened, &unsafe { named.assume_init() })
                {
                    return Err(std::io::Error::other(
                        "publication file changed before removal",
                    ));
                }
                if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            _ => return Err(std::io::Error::other("unsafe publication entry type")),
        }
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn prepare_stage(stage: &Path, target: &LocalTarget, run: &WorkerRun) -> Result<(), TargetError> {
    for file in &run.output().files {
        let source = run.output_root().join(&file.path);
        let destination = stage.join(&file.path);
        let parent = destination.parent().ok_or(TargetError::Failed)?;
        fs::create_dir_all(parent).map_err(|_| TargetError::Failed)?;
        copy_verified_file(&source, &destination, file.size, &file.sha256)?;
    }
    let owner_path = stage.join(OWNER_FILE);
    let mut owner_options = OpenOptions::new();
    owner_options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        owner_options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let mut owner = owner_options
        .open(owner_path)
        .map_err(|_| TargetError::Failed)?;
    owner
        .write_all(target.review.target_id.as_bytes())
        .and_then(|()| owner.sync_all())
        .map_err(|_| TargetError::Failed)?;
    sync_tree_directories(stage).map_err(|_| TargetError::Failed)
}

#[cfg(target_os = "macos")]
fn prepare_stage_at(
    stage: &File,
    target: &LocalTarget,
    run: &WorkerRun,
) -> Result<(), TargetError> {
    use std::os::unix::fs::MetadataExt;
    use std::{
        ffi::CString,
        os::fd::{AsRawFd, FromRawFd},
    };
    let root_device = stage.metadata().map_err(|_| TargetError::Failed)?.dev();
    for record in &run.output().files {
        let components = record.path.split('/').collect::<Vec<_>>();
        let (file_name, parents) = components.split_last().ok_or(TargetError::Failed)?;
        let mut current = stage.try_clone().map_err(|_| TargetError::Failed)?;
        for component in parents {
            current = open_or_create_private_directory_at(&current, component, root_device)?;
        }
        let name = CString::new(*file_name).map_err(|_| TargetError::Failed)?;
        let destination = unsafe {
            libc::openat(
                current.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if destination < 0 {
            return Err(TargetError::Failed);
        }
        let mut destination = unsafe { File::from_raw_fd(destination) };
        copy_verified_file_into(
            &run.output_root().join(&record.path),
            &mut destination,
            record.size,
            &record.sha256,
        )?;
        if !crate::security::has_no_mutating_acl(&destination) {
            return Err(TargetError::Failed);
        }
        current.sync_all().map_err(|_| TargetError::Failed)?;
    }
    let owner_name = CString::new(OWNER_FILE).expect("fixed owner filename");
    let owner = unsafe {
        libc::openat(
            stage.as_raw_fd(),
            owner_name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if owner < 0 {
        return Err(TargetError::Failed);
    }
    let mut owner = unsafe { File::from_raw_fd(owner) };
    owner
        .write_all(target.review.target_id.as_bytes())
        .and_then(|()| owner.sync_all())
        .map_err(|_| TargetError::Failed)?;
    if !crate::security::has_no_mutating_acl(&owner) {
        return Err(TargetError::Failed);
    }
    stage.sync_all().map_err(|_| TargetError::Failed)
}

#[cfg(target_os = "macos")]
fn open_or_create_private_directory_at(
    parent: &File,
    name: &str,
    root_device: u64,
) -> Result<File, TargetError> {
    use std::{
        ffi::CString,
        mem::MaybeUninit,
        os::fd::{AsRawFd, FromRawFd},
    };
    let name = CString::new(name).map_err(|_| TargetError::Failed)?;
    let created = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } == 0;
    if !created {
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(TargetError::Failed);
        }
    } else {
        parent.sync_all().map_err(|_| TargetError::Failed)?;
    }
    let descriptor = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(TargetError::Failed);
    }
    let directory = unsafe { File::from_raw_fd(descriptor) };
    let mut status = MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(directory.as_raw_fd(), status.as_mut_ptr()) } != 0 {
        return Err(TargetError::Failed);
    }
    let status = unsafe { status.assume_init() };
    if status.st_dev as u64 != root_device
        || status.st_uid != unsafe { libc::geteuid() }
        || status.st_mode & 0o077 != 0
        || !crate::security::has_no_mutating_acl(&directory)
    {
        return Err(TargetError::Failed);
    }
    Ok(directory)
}

#[cfg(not(target_os = "macos"))]
fn copy_verified_file(
    source: &Path,
    destination: &Path,
    expected_size: u64,
    expected_sha256: &str,
) -> Result<(), TargetError> {
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt;
    let mut destination_options = OpenOptions::new();
    destination_options.write(true).create_new(true);
    #[cfg(unix)]
    destination_options
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW);
    let mut destination = destination_options
        .open(destination)
        .map_err(|_| TargetError::Failed)?;
    copy_verified_file_into(source, &mut destination, expected_size, expected_sha256)
}

fn copy_verified_file_into(
    source_path: &Path,
    destination: &mut File,
    expected_size: u64,
    expected_sha256: &str,
) -> Result<(), TargetError> {
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt;
    let mut source_options = OpenOptions::new();
    source_options.read(true);
    #[cfg(unix)]
    source_options.custom_flags(libc::O_NOFOLLOW);
    let mut source = source_options
        .open(source_path)
        .map_err(|_| TargetError::Failed)?;
    let metadata = source.metadata().map_err(|_| TargetError::Failed)?;
    if !metadata.file_type().is_file() || metadata.len() != expected_size {
        return Err(TargetError::Failed);
    }
    let mut hasher = Sha256::new();
    let mut observed = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = source.read(&mut buffer).map_err(|_| TargetError::Failed)?;
        if count == 0 {
            break;
        }
        observed = observed
            .checked_add(count as u64)
            .ok_or(TargetError::Failed)?;
        if observed > expected_size {
            return Err(TargetError::Failed);
        }
        hasher.update(&buffer[..count]);
        destination
            .write_all(&buffer[..count])
            .map_err(|_| TargetError::Failed)?;
    }
    if observed != expected_size || format!("{:x}", hasher.finalize()) != expected_sha256 {
        return Err(TargetError::Failed);
    }
    destination.sync_all().map_err(|_| TargetError::Failed)
}

#[cfg(not(target_os = "macos"))]
fn sync_tree_directories(root: &Path) -> std::io::Result<()> {
    let mut pending = vec![(root.to_path_buf(), 0_usize)];
    let mut directories = Vec::new();
    let mut entries = 0_usize;
    while let Some((directory, depth)) = pending.pop() {
        if depth > MAX_TREE_DEPTH || directories.len() >= MAX_TREE_DIRECTORIES {
            return Err(std::io::Error::other(
                "publication tree exceeds its directory budget",
            ));
        }
        directories.push(directory.clone());
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            entries = entries
                .checked_add(1)
                .ok_or_else(|| std::io::Error::other("publication tree entry overflow"))?;
            if entries > MAX_TREE_ENTRIES {
                return Err(std::io::Error::other(
                    "publication tree exceeds its entry budget",
                ));
            }
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_dir() {
                pending.push((entry.path(), depth + 1));
            }
        }
    }
    for directory in directories.into_iter().rev() {
        sync_directory(&directory)?;
    }
    Ok(())
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    return File::open(path)?.sync_all();
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

fn valid_target_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
