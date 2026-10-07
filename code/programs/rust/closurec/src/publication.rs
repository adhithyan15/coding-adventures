//! Publish an output set without exposing partial files or overwriting collisions.
//!
//! Preparation is complete before this module runs. Staging finishes before any
//! original is moved. Each installation uses a no-clobber link to a complete,
//! synced file; the sidecar is last. Reported pre-commit failures roll back in
//! reverse order. Multiple paths are not one atomic filesystem operation, and
//! this does not promise crash recovery or serialize outside modifications.
use crate::run::CompilerError;
use std::collections::HashSet;
use std::fs::{self, File, Metadata, OpenOptions, Permissions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

#[cfg(unix)]
mod unix_security;
#[cfg(windows)]
mod windows_security;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Identity([u64; 3]);

#[cfg(unix)]
fn file_identity(file: &File) -> io::Result<Identity> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file.metadata()?;
    Ok(Identity([metadata.dev(), metadata.ino(), 0]))
}

#[cfg(windows)]
fn file_identity(file: &File) -> io::Result<Identity> {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;
    // FILE_ID_INFO is ULONGLONG plus FILE_ID_128 (sixteen BYTEs). The full
    // 128-bit ID is necessary on ReFS: BY_HANDLE_FILE_INFORMATION's 64-bit
    // identifier is not guaranteed unique there. Keep the File alive for the
    // duration of the transaction so an unlinked object's ID cannot be reused.
    // https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_info
    #[repr(C)]
    struct FileIdInfo {
        volume: u64,
        id: [u8; 16],
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetFileInformationByHandleEx(
            handle: *mut c_void,
            class: i32,
            buffer: *mut c_void,
            size: u32,
        ) -> i32;
    }
    let mut info = FileIdInfo {
        volume: 0,
        id: [0; 16],
    };
    // SAFETY: AsRawHandle borrows this live File. Class 18 (FileIdInfo) accepts
    // a writable, correctly aligned FILE_ID_INFO buffer of exactly this size;
    // the synchronous call neither retains the pointer nor transfers ownership.
    // Primitive class/BOOL types avoid invalid Rust enum representations.
    let success = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            18,
            (&mut info as *mut FileIdInfo).cast(),
            std::mem::size_of::<FileIdInfo>() as u32,
        )
    };
    if success == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Identity([
        info.volume,
        u64::from_le_bytes(info.id[..8].try_into().unwrap()),
        u64::from_le_bytes(info.id[8..].try_into().unwrap()),
    ]))
}

#[cfg(not(any(unix, windows)))]
fn file_identity(_file: &File) -> io::Result<Identity> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "output publication requires filesystem object identities",
    ))
}

struct Observed {
    file: File,
    id: Identity,
    len: u64,
    modified: Option<SystemTime>,
    permissions: Permissions,
    #[cfg(windows)]
    policy: windows_security::Policy,
    #[cfg(unix)]
    policy: unix_security::Policy,
}
impl Observed {
    fn from_file(file: File) -> io::Result<Self> {
        let id = file_identity(&file)?;
        let metadata = file.metadata()?;
        #[cfg(windows)]
        let policy = windows_security::Policy::capture(&file)?;
        #[cfg(unix)]
        let policy = unix_security::Policy::capture(&file)?;
        Ok(Self {
            file,
            id,
            len: metadata.len(),
            modified: metadata.modified().ok(),
            permissions: metadata.permissions(),
            #[cfg(any(windows, unix))]
            policy,
        })
    }
    fn refresh(&mut self) -> io::Result<()> {
        let metadata = self.file.metadata()?;
        self.len = metadata.len();
        self.modified = metadata.modified().ok();
        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}
fn failure(path: &Path, action: &str, error: io::Error) -> CompilerError {
    CompilerError::OutputWriteError {
        path: path.to_path_buf(),
        kind: error.kind(),
        message: format!("{action}: {error}"),
    }
}

// Open a regular file/directory without accepting final symlinks. Check the
// pathname again against the live handle. These checks do not pretend to lock
// an arbitrary outside process out of a namespace between separate syscalls.
fn observe(path: &Path, directory: bool) -> io::Result<Option<Observed>> {
    let before = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    check_kind(&before, directory)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // OPEN_REPARSE_POINT prevents following a replaced final symlink;
        // BACKUP_SEMANTICS permits directory handles for ownership checks.
        options.custom_flags(0x0020_0000 | 0x0200_0000);
        options.access_mode(0x0002_0080); // READ_CONTROL | FILE_READ_ATTRIBUTES
    }
    let observed = Observed::from_file(options.open(path)?)?;
    check_kind(&observed.file.metadata()?, directory)?;
    let after = fs::symlink_metadata(path)?;
    check_kind(&after, directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if observed.id != Identity([after.dev(), after.ino(), 0]) {
            return Err(invalid("destination changed while opening its identity"));
        }
    }
    Ok(Some(observed))
}
fn check_kind(metadata: &Metadata, directory: bool) -> io::Result<()> {
    if metadata.file_type().is_symlink()
        || if directory {
            !metadata.is_dir()
        } else {
            !metadata.is_file()
        }
    {
        return Err(invalid("publication requires an ordinary directory or regular file; final symlinks are rejected"));
    }
    Ok(())
}

struct Destination {
    path: PathBuf,
    original: Option<Observed>,
}

#[cfg(windows)]
fn validate_component(name: &std::ffi::OsStr) -> io::Result<()> {
    let text = name.to_string_lossy();
    let stem = text.split('.').next().unwrap_or("").to_uppercase();
    let numbered = (stem.starts_with("COM") || stem.starts_with("LPT"))
        && stem.len() == 4
        && matches!(stem.as_bytes()[3], b'1'..=b'9');
    if text.ends_with([' ', '.'])
        || text.contains(':')
        || matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        )
        || numbered
    {
        return Err(invalid("ambiguous Windows output filename"));
    }
    Ok(())
}
#[cfg(not(windows))]
fn validate_component(_name: &std::ffi::OsStr) -> io::Result<()> {
    Ok(())
}

fn normalize(path: &Path) -> io::Result<PathBuf> {
    let spelling = path.to_string_lossy();
    if spelling.is_empty() || spelling.ends_with('/') || cfg!(windows) && spelling.ends_with('\\') {
        return Err(invalid(
            "output destination must name a file, without a trailing directory separator",
        ));
    }
    // GetFullPathName-based Windows absolute conversion can erase trailing
    // dots/spaces or collapse a missing parent followed by `..`. Validate the
    // user's spelling and ambiguous traversal before that lossy normalization.
    let raw = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut prefix = PathBuf::new();
    for component in raw.components() {
        match component {
            std::path::Component::Normal(name) => validate_component(name)?,
            std::path::Component::ParentDir if !prefix.is_dir() => {
                return Err(invalid("ambiguous missing-parent output traversal"))
            }
            _ => {}
        }
        prefix.push(component.as_os_str());
    }
    let absolute = std::path::absolute(path)?;
    let name = absolute
        .file_name()
        .ok_or_else(|| invalid("output destination must name a file"))?;
    validate_component(name)?;
    let mut cursor = absolute
        .parent()
        .ok_or_else(|| invalid("output destination has no parent"))?;
    let mut suffix = Vec::new();
    loop {
        match fs::canonicalize(cursor) {
            Ok(mut parent) => {
                // Verify that the existing anchor really is a directory and
                // supplies usable identities before creating staging paths.
                observe(&parent, true)?.ok_or_else(|| invalid("output parent disappeared"))?;
                for part in suffix.iter().rev() {
                    parent.push(part);
                }
                parent.push(name);
                return Ok(parent);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let part = cursor
                    .file_name()
                    .ok_or_else(|| invalid("ambiguous missing-parent output path"))?;
                validate_component(part)?;
                suffix.push(part.to_os_string());
                cursor = cursor
                    .parent()
                    .ok_or_else(|| invalid("output parent has no existing anchor"))?;
            }
            Err(error) => return Err(error),
        }
    }
}

fn preflight(outputs: &[(PathBuf, String)]) -> Result<Vec<Destination>, CompilerError> {
    let mut destinations = Vec::<Destination>::with_capacity(outputs.len());
    let mut keys = HashSet::<PathBuf>::new();
    let mut identities = HashSet::new();
    for (requested, _) in outputs {
        let path =
            normalize(requested).map_err(|error| failure(requested, "preflight output", error))?;
        #[cfg(windows)]
        let key = PathBuf::from(path.to_string_lossy().to_lowercase());
        #[cfg(not(windows))]
        let key = path.clone();
        if keys
            .iter()
            .any(|other| key.starts_with(other) || other.starts_with(&key))
            || !keys.insert(key)
        {
            return Err(failure(
                requested,
                "preflight output",
                invalid("colliding output destinations"),
            ));
        }
        let original =
            observe(&path, false).map_err(|error| failure(requested, "preflight output", error))?;
        if let Some(file) = &original {
            #[cfg(any(windows, unix))]
            file.policy
                .check_assignable_owner()
                .map_err(|error| failure(requested, "preflight output policy", error))?;
            if file.permissions.readonly() {
                return Err(failure(
                    requested,
                    "preflight output",
                    io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "read-only output destination",
                    ),
                ));
            }
            if !identities.insert(file.id) {
                return Err(failure(
                    requested,
                    "preflight output",
                    invalid("colliding output file identities"),
                ));
            }
        }
        destinations.push(Destination { path, original });
    }
    Ok(destinations)
}

struct OwnedDirectory {
    path: PathBuf,
    observed: Observed,
}
impl OwnedDirectory {
    fn check(&self) -> io::Result<()> {
        match observe(&self.path, true)? {
            Some(current)
                if current.id == self.observed.id && same_policy(&current, &self.observed) =>
            {
                Ok(())
            }
            _ => Err(invalid("owned staging directory changed or disappeared")),
        }
    }
    fn remove_empty(&self) -> io::Result<()> {
        self.check()?;
        fs::remove_dir(&self.path)
    }
}
struct Stage {
    destination: Destination,
    directory: OwnedDirectory,
    new: Option<Observed>,
    backup_created: bool,
    backed_up: bool,
    installed: bool,
    #[cfg(windows)]
    intended_policy: windows_security::Policy,
    #[cfg(unix)]
    intended_policy: Option<unix_security::Policy>,
    #[cfg(any(windows, unix))]
    parent: Observed,
}
impl Stage {
    fn new_path(&self) -> PathBuf {
        self.directory.path.join("new")
    }
    fn old_path(&self) -> PathBuf {
        self.directory.path.join("old")
    }
}

// Hooks are private call-site parameters used only by tests. The production
// entry point always supplies an infallible callback, with no CLI/env switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Stage,
    Install,
    BackupRemove,
    #[cfg(any(windows, unix))]
    Policy,
    RollbackRemove,
    Restore,
    Cleanup,
}

pub(super) fn publish_outputs(outputs: &[(PathBuf, String)]) -> Result<Vec<String>, CompilerError> {
    publish_with_hook(outputs, &mut |_, _| Ok(()))
}
fn publish_with_hook(
    outputs: &[(PathBuf, String)],
    hook: &mut impl FnMut(Phase, &Path) -> io::Result<()>,
) -> Result<Vec<String>, CompilerError> {
    let destinations = preflight(outputs)?;
    let mut stages = Vec::with_capacity(outputs.len());
    let mut parents = Vec::new();
    let prepared = (|| {
        for (destination, (_, body)) in destinations.into_iter().zip(outputs) {
            let path = destination.path.clone();
            ensure_parents(path.parent().unwrap(), &mut parents)
                .map_err(|error| failure(&path, "create output parents", error))?;
            #[cfg(any(windows, unix))]
            let parent = observe(path.parent().unwrap(), true)
                .map_err(|error| failure(&path, "capture output parent policy", error))?
                .ok_or_else(|| {
                    failure(
                        &path,
                        "capture output parent policy",
                        invalid("output parent disappeared"),
                    )
                })?;
            #[cfg(windows)]
            let intended_policy = match &destination.original {
                Some(original) => original.policy.clone(),
                None => windows_security::new_file_policy(&parent.file)
                    .map_err(|error| failure(&path, "derive output policy", error))?,
            };
            let directory = private_directory(path.parent().unwrap())
                .map_err(|error| failure(&path, "create staging directory", error))?;
            stages.push(Stage {
                destination,
                directory,
                new: None,
                backup_created: false,
                backed_up: false,
                installed: false,
                #[cfg(windows)]
                intended_policy,
                #[cfg(unix)]
                intended_policy: None,
                #[cfg(any(windows, unix))]
                parent,
            });
            let stage = stages.last_mut().unwrap();
            #[cfg(windows)]
            verify_policy_support(stage)
                .map_err(|error| failure(&path, "preflight native output policy", error))?;
            #[cfg(unix)]
            {
                stage.intended_policy =
                    Some(verify_unix_policy_support(stage).map_err(|error| {
                        failure(&path, "preflight native output policy", error)
                    })?);
            }
            hook(Phase::Stage, &path).map_err(|error| failure(&path, "stage output", error))?;
            stage
                .directory
                .check()
                .map_err(|error| failure(&path, "check staging directory", error))?;
            let file = create_stage_file(&stage.new_path())
                .map_err(|error| failure(&path, "create stage file", error))?;
            stage.new = Some(
                Observed::from_file(file)
                    .map_err(|error| failure(&path, "identify stage file", error))?,
            );
            let new = stage.new.as_mut().unwrap();
            new.file
                .write_all(body.as_bytes())
                .map_err(|error| failure(&path, "write stage file", error))?;
            #[cfg(not(unix))]
            if let Some(old) = &stage.destination.original {
                new.file
                    .set_permissions(old.permissions.clone())
                    .map_err(|error| failure(&path, "preserve output permissions", error))?;
            }
            new.file
                .sync_all()
                .map_err(|error| failure(&path, "sync stage file", error))?;
            new.refresh()
                .map_err(|error| failure(&path, "verify stage file", error))?;
        }
        for stage in &mut stages {
            let path = &stage.destination.path;
            hook(Phase::Install, path).map_err(|error| failure(path, "install output", error))?;
            stage
                .directory
                .check()
                .map_err(|error| failure(path, "check staging directory", error))?;
            let current = observe(path, false)
                .map_err(|error| failure(path, "recheck destination", error))?;
            #[cfg(any(windows, unix))]
            {
                let parent = observe(path.parent().unwrap(), true)
                    .map_err(|error| failure(path, "recheck output parent", error))?
                    .ok_or_else(|| {
                        failure(
                            path,
                            "recheck output parent",
                            invalid("output parent disappeared"),
                        )
                    })?;
                if parent.id != stage.parent.id || parent.policy != stage.parent.policy {
                    return Err(failure(
                        path,
                        "recheck output parent",
                        invalid("output parent identity or access policy changed"),
                    ));
                }
            }
            match (&stage.destination.original, &current) {
                (None, None) => {}
                (Some(old), Some(current)) if same_original(old, current) => {
                    // A private directory can still acquire an unexpected
                    // occupant. Exclusive link creation cannot overwrite it;
                    // object identity alone never claims ownership of its path.
                    fs::hard_link(path, stage.old_path()).map_err(|error| {
                        failure(path, "create original backup without overwrite", error)
                    })?;
                    stage.backup_created = true;
                    stage.backed_up = true;
                    hook(Phase::BackupRemove, path)
                        .map_err(|error| failure(path, "remove backed-up original", error))?;
                    remove_original(path, old)
                        .map_err(|error| failure(path, "remove backed-up original", error))?;
                }
                _ => {
                    return Err(failure(
                        path,
                        "recheck destination",
                        invalid("output destination changed during publication"),
                    ))
                }
            }
            let prepared = observe(&stage.new_path(), false)
                .map_err(|error| failure(path, "recheck prepared file", error))?
                .ok_or_else(|| {
                    failure(
                        path,
                        "recheck prepared file",
                        invalid("prepared file disappeared"),
                    )
                })?;
            let expected = stage.new.as_ref().unwrap();
            if !same_original(expected, &prepared) {
                return Err(failure(
                    path,
                    "recheck prepared file",
                    invalid("prepared content identity changed"),
                ));
            }
            // hard_link fails if the destination exists on both Windows and
            // Unix; rename(stage,destination) would clobber a Unix racing file.
            fs::hard_link(stage.new_path(), path).map_err(|error| {
                failure(path, "install complete output without overwrite", error)
            })?;
            stage.installed = true;
            #[cfg(unix)]
            {
                hook(Phase::Policy, path)
                    .map_err(|error| failure(path, "apply output policy", error))?;
                stage
                    .intended_policy
                    .as_ref()
                    .unwrap()
                    .apply(&expected.file)
                    .map_err(|error| failure(path, "apply and verify output policy", error))?;
            }
            #[cfg(windows)]
            {
                hook(Phase::Policy, path)
                    .map_err(|error| failure(path, "apply output policy", error))?;
                let file = windows_security::policy_handle(path)
                    .map_err(|error| failure(path, "open installed output policy", error))?;
                if file_identity(&file)
                    .map_err(|error| failure(path, "identify policy target", error))?
                    != expected.id
                {
                    return Err(failure(
                        path,
                        "identify policy target",
                        invalid("installed policy target changed"),
                    ));
                }
                // Windows' automatic inheritance uses the opened handle's
                // parent. Legacy explicit unprotected ACLs must not acquire
                // additional destination-parent grants. Inherited entries need
                // the actual destination parent; exact readback rejects drift.
                let policy_file = if stage.intended_policy.has_inherited_entries() {
                    &file
                } else {
                    &expected.file
                };
                stage
                    .intended_policy
                    .apply(policy_file)
                    .map_err(|error| failure(path, "apply and verify output policy", error))?;
            }
            let installed = observe(path, false)
                .map_err(|error| failure(path, "verify installed output", error))?
                .ok_or_else(|| {
                    failure(
                        path,
                        "verify installed output",
                        invalid("installed output disappeared"),
                    )
                })?;
            let new = stage.new.as_ref().unwrap();
            if installed.id != new.id
                || installed.len != new.len
                || installed.modified != new.modified
            {
                return Err(failure(
                    path,
                    "verify installed output",
                    invalid("installed output content identity changed"),
                ));
            }
            #[cfg(unix)]
            if installed.policy != *stage.intended_policy.as_ref().unwrap() {
                return Err(failure(
                    path,
                    "verify installed output",
                    invalid("installed Unix access policy changed"),
                ));
            }
        }
        Ok(())
    })();
    if let Err(mut error) = prepared {
        #[cfg(any(windows, unix))]
        if let CompilerError::OutputWriteError { message, .. } = &mut error {
            for stage in &stages {
                if let Some(new) = &stage.new {
                    if let Err(error) = restore_candidate_privacy(new) {
                        message.push_str(&format!(
                            "; restore prepared-file privacy {}: {error}",
                            stage.new_path().display()
                        ));
                    }
                }
            }
        }
        let problems = rollback(&mut stages, hook);
        let mut cleanup = cleanup_stages(&stages, false, hook);
        for directory in parents.iter().rev() {
            if let Err(error) = directory.remove_empty() {
                cleanup.push(format!(
                    "parent cleanup {}: {error}",
                    directory.path.display()
                ));
            }
        }
        if let CompilerError::OutputWriteError { message, .. } = &mut error {
            for problem in problems.into_iter().chain(cleanup) {
                message.push_str("; ");
                message.push_str(&problem);
            }
        }
        return Err(error);
    }
    // All files are installed: this is the commit point. Cleanup is best effort
    // with explicit warnings, because partially deleting backups cannot be undone.
    Ok(cleanup_stages(&stages, true, hook)
        .into_iter()
        .map(|problem| format!("warning: output publication committed; {problem}\n"))
        .collect())
}

fn ensure_parents(parent: &Path, owned: &mut Vec<OwnedDirectory>) -> io::Result<()> {
    let mut missing = Vec::new();
    let mut cursor = parent;
    while observe(cursor, true)?.is_none() {
        missing.push(cursor.to_path_buf());
        cursor = cursor
            .parent()
            .ok_or_else(|| invalid("missing output parent anchor"))?;
    }
    // Heap state replaces recursive path descent, including direct long-path
    // callers. Only exclusive mkdir successes become cleanup-owned parents.
    for path in missing.into_iter().rev() {
        match fs::create_dir(&path) {
            Ok(()) => {
                let observed = observe(&path, true)
                    .map_err(|error| {
                        io::Error::new(
                            error.kind(),
                            format!(
                                "cannot identify created parent {}; retained directory: {error}",
                                path.display()
                            ),
                        )
                    })?
                    .ok_or_else(|| {
                        invalid(format!("created parent disappeared: {}", path.display()))
                    })?;
                owned.push(OwnedDirectory { path, observed });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                observe(&path, true)?.ok_or_else(|| invalid("output parent disappeared"))?;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn reserve_sequence(counter: &AtomicU64) -> io::Result<u64> {
    let mut current = counter.load(Ordering::Relaxed);
    loop {
        let next = current
            .checked_add(1)
            .ok_or_else(|| invalid("staging sequence exhausted"))?;
        match counter.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return Ok(current),
            Err(observed) => current = observed,
        }
    }
}

fn private_directory(parent: &Path) -> io::Result<OwnedDirectory> {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    for _ in 0..32 {
        let sequence = reserve_sequence(&SEQUENCE)?;
        let path = parent.join(format!(".closurec-{}-{sequence}", std::process::id()));
        #[cfg(unix)]
        let mut builder = fs::DirBuilder::new();
        #[cfg(not(any(unix, windows)))]
        let builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        #[cfg(windows)]
        let created = windows_security::create_directory(&path);
        #[cfg(not(windows))]
        let created = builder.create(&path);
        match created {
            Ok(()) => {
                let observed = observe(&path, true).map_err(|error|io::Error::new(error.kind(),format!("cannot identify created staging directory {}; retained directory: {error}",path.display())))?.ok_or_else(|| {
                    invalid(format!(
                        "created staging directory disappeared: {}",
                        path.display()
                    ))
                })?;
                return Ok(OwnedDirectory { path, observed });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "staging directory collision retries exhausted",
    ))
}

fn create_stage_file(path: &Path) -> io::Result<File> {
    #[cfg(windows)]
    {
        windows_security::create_file(path)
    }
    #[cfg(not(windows))]
    {
        let mut options = OpenOptions::new();
        options.write(true).read(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(path)
    }
}

#[cfg(unix)]
fn verify_unix_policy_support(stage: &Stage) -> io::Result<unix_security::Policy> {
    stage.directory.check()?;
    // Native ordinary creation in the actual destination parent measures
    // umask and filesystem group/setgid behavior without changing global state.
    // This inode is permanently separate from the future data-bearing file.
    let path = stage.directory.path.with_extension("policy-probe");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)?;
    let probe = Observed::from_file(file).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "cannot identify native creation probe; retained empty probe {}: {error}",
                path.display()
            ),
        )
    })?;
    let intended = stage
        .destination
        .original
        .as_ref()
        .map_or_else(|| probe.policy.clone(), |old| old.policy.clone());
    let checked = (|| {
        intended.apply(&probe.file)?;
        let fresh = observe(&path, false)?
            .ok_or_else(|| invalid("active Unix policy probe disappeared"))?;
        if fresh.id != probe.id || fresh.len != 0 || fresh.policy != intended {
            return Err(invalid(
                "fresh Unix policy probe identity or policy changed",
            ));
        }
        Ok(())
    })();
    let cleaned = (|| {
        use std::os::unix::fs::PermissionsExt;
        probe.file.set_permissions(Permissions::from_mode(0o600))?;
        remove_owned(&path, &probe)
    })();
    match (checked, cleaned) {
        (Ok(()), Ok(())) => Ok(intended),
        (Err(error), Ok(())) => Err(error),
        (checked, Err(error)) => Err(io::Error::new(error.kind(), format!(
            "native Unix policy probe cleanup failed; retained empty probe {}: {error}; validation: {checked:?}", path.display()
        ))),
    }
}

#[cfg(windows)]
fn verify_policy_support(stage: &Stage) -> io::Result<()> {
    // Never widen the future data-bearing inode, even while it is empty:
    // tightening a DACL cannot revoke a reader's already-open handle. This
    // distinct probe remains empty and is destroyed before writing any bytes.
    let private_path = stage.directory.path.join("policy");
    let probe = Observed::from_file(windows_security::create_file(&private_path)?)?;
    let public_path = stage.directory.path.with_extension("policy-probe");
    let mut linked = false;
    let checked = (|| {
        if stage.intended_policy.has_inherited_entries() {
            fs::hard_link(&private_path, &public_path)?;
            linked = true;
            let handle = windows_security::policy_handle(&public_path)?;
            if file_identity(&handle)? != probe.id {
                return Err(invalid("native policy probe identity changed"));
            }
            stage.intended_policy.apply(&handle)?;
        } else {
            stage.intended_policy.apply(&probe.file)?;
        }
        // Retained handles keep rights granted before the DACL changed. Prove
        // the exact fresh open required by final verification while the intended
        // policy is active; OWNER RIGHTS can remove the owner's implicit access.
        let active_path = if linked { &public_path } else { &private_path };
        let fresh = observe(active_path, false)?
            .ok_or_else(|| invalid("active policy probe disappeared"))?;
        if fresh.id != probe.id || fresh.len != 0 || fresh.policy != stage.intended_policy {
            return Err(invalid(
                "fresh native policy probe identity or policy changed",
            ));
        }
        Ok(())
    })();
    // This held handle keeps WRITE_DAC/WRITE_OWNER despite a restrictive test
    // policy. Restore privacy before identity-checked removal of owned links.
    let cleaned = (|| {
        probe.policy.apply(&probe.file)?;
        if linked {
            remove_owned(&public_path, &probe)?;
        }
        remove_owned(&private_path, &probe)
    })();
    match (checked, cleaned) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (checked, Err(error)) => Err(io::Error::new(error.kind(), format!(
            "native policy probe cleanup failed; retained empty probe {} and {}: {error}; validation: {checked:?}",
            private_path.display(), public_path.display()
        ))),
    }
}

fn same_original(expected: &Observed, current: &Observed) -> bool {
    let same = expected.id == current.id
        && expected.len == current.len
        && expected.modified == current.modified;
    same && same_policy(expected, current)
}

#[cfg(any(windows, unix))]
fn restore_candidate_privacy(new: &Observed) -> io::Result<()> {
    #[cfg(windows)]
    {
        new.policy.apply(&new.file)
    }
    #[cfg(unix)]
    {
        new.policy.restore_privacy(&new.file)
    }
}

fn same_policy(expected: &Observed, current: &Observed) -> bool {
    #[cfg(any(windows, unix))]
    {
        expected.policy == current.policy
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = (expected, current);
        true
    }
}

fn remove_owned(path: &Path, expected: &Observed) -> io::Result<()> {
    match observe(path, false)? {
        None => Ok(()),
        Some(current) if current.id == expected.id => fs::remove_file(path),
        _ => Err(invalid("refusing to remove an unknown replacement file")),
    }
}
fn remove_original(path: &Path, expected: &Observed) -> io::Result<()> {
    let current = observe(path, false)?
        .ok_or_else(|| invalid("original destination disappeared before backup removal"))?;
    if !same_original(expected, &current) {
        return Err(invalid(
            "original destination changed before backup removal",
        ));
    }
    fs::remove_file(path)
}
fn rollback(
    stages: &mut [Stage],
    hook: &mut impl FnMut(Phase, &Path) -> io::Result<()>,
) -> Vec<String> {
    let mut problems = Vec::new();
    for stage in stages.iter_mut().rev() {
        let path = &stage.destination.path;
        if stage.installed {
            let removed = hook(Phase::RollbackRemove, path)
                .and_then(|()| remove_owned(path, stage.new.as_ref().unwrap()));
            if let Err(error) = removed {
                let recovery = if stage.backup_created {
                    format!("retained recovery {}", stage.old_path().display())
                } else {
                    "destination removal unconfirmed; no original backup was created".into()
                };
                problems.push(format!("rollback {}: {error}; {recovery}", path.display(),));
            } else {
                stage.installed = false;
            }
        }
        if stage.backed_up && !stage.installed {
            let restored = hook(Phase::Restore, path).and_then(|()| {
                stage.directory.check()?;
                let old = observe(&stage.old_path(), false)?
                    .ok_or_else(|| invalid("original backup disappeared"))?;
                let original = stage.destination.original.as_ref().unwrap();
                if !same_original(original, &old) {
                    return Err(invalid("original backup identity changed"));
                }
                match observe(path, false)? {
                    Some(current) if same_original(original, &current) => Ok(()),
                    Some(_) => Err(invalid(
                        "restore destination is occupied by an unknown replacement",
                    )),
                    None => fs::hard_link(stage.old_path(), path),
                }
            });
            if let Err(error) = restored {
                problems.push(format!(
                    "rollback restore {}: {error}; retained recovery {}",
                    path.display(),
                    stage.old_path().display()
                ));
            } else {
                stage.backed_up = false;
            }
        }
    }
    problems
}
fn cleanup_stages(
    stages: &[Stage],
    committed: bool,
    hook: &mut impl FnMut(Phase, &Path) -> io::Result<()>,
) -> Vec<String> {
    let mut problems = Vec::new();
    for stage in stages.iter().rev() {
        let cleanup = (|| {
            hook(Phase::Cleanup, &stage.destination.path)?;
            stage.directory.check()?;
            if let Some(new) = &stage.new {
                remove_owned(&stage.new_path(), new)?;
            }
            if stage.backed_up && !committed {
                return Err(invalid(format!(
                    "retained original recovery {}",
                    stage.old_path().display()
                )));
            }
            if stage.backup_created {
                remove_owned(
                    &stage.old_path(),
                    stage.destination.original.as_ref().unwrap(),
                )?;
            }
            stage.directory.remove_empty()
        })();
        if let Err(error) = cleanup {
            problems.push(format!(
                "owned cleanup {}: {error}",
                stage.directory.path.display()
            ));
        }
    }
    problems
}

#[cfg(test)]
mod tests;
