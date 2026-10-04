//! Identity and output validation for the bundled Forme product worker.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, Read, Write},
    path::{Component, Path},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;

pub const MAX_WORKER_MESSAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_WORKER_EXECUTABLE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_LAUNCHER_EXECUTABLE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ARTIFACT_FILES: usize = 10_000;
const MAX_ARTIFACT_ENTRIES: usize = 20_000;
const MAX_ARTIFACT_DIRECTORIES: usize = 10_000;
const MAX_ARTIFACT_DEPTH: usize = 64;
const MAX_ARTIFACT_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_SCRATCH_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_RESIDENT_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WorkerError {
    #[error("worker identity mismatch")]
    Identity,
    #[error("worker protocol violation")]
    Protocol,
    #[error("worker output failed validation")]
    Output,
    #[error("worker input/output failed")]
    Io,
    #[error("worker reported a known build failure")]
    Build,
    #[error("worker exceeded its deadline")]
    Timeout,
    #[error("the required operating-system sandbox is unavailable")]
    Sandbox,
}

#[derive(Debug)]
pub struct NativeProductWorker {
    executable: std::path::PathBuf,
    expected_sha256: String,
    launcher: std::path::PathBuf,
    expected_launcher_sha256: String,
    workspace_root: std::path::PathBuf,
    timeout: Duration,
}

#[derive(Debug)]
pub struct WorkerRun {
    workspace: std::path::PathBuf,
    output: ValidatedWorkerOutput,
}

impl WorkerRun {
    pub fn output(&self) -> &ValidatedWorkerOutput {
        &self.output
    }

    pub fn output_root(&self) -> std::path::PathBuf {
        self.workspace.join("output")
    }

    pub fn retire(mut self) -> Result<(), WorkerError> {
        let workspace = std::mem::take(&mut self.workspace);
        if workspace.as_os_str().is_empty() {
            return Ok(());
        }
        fs::remove_dir_all(workspace).map_err(|_| WorkerError::Io)
    }
}

impl Drop for WorkerRun {
    fn drop(&mut self) {
        if !self.workspace.as_os_str().is_empty() {
            let _ = fs::remove_dir_all(&self.workspace);
        }
    }
}

impl NativeProductWorker {
    pub fn open(
        executable: std::path::PathBuf,
        expected_sha256: String,
        launcher: std::path::PathBuf,
        expected_launcher_sha256: String,
        workspace_root: std::path::PathBuf,
    ) -> Result<Self, WorkerError> {
        Self::open_with_timeout(
            executable,
            expected_sha256,
            launcher,
            expected_launcher_sha256,
            workspace_root,
            Duration::from_secs(30),
        )
    }

    pub fn open_with_timeout(
        executable: std::path::PathBuf,
        expected_sha256: String,
        launcher: std::path::PathBuf,
        expected_launcher_sha256: String,
        workspace_root: std::path::PathBuf,
        timeout: Duration,
    ) -> Result<Self, WorkerError> {
        if timeout.is_zero() {
            return Err(WorkerError::Protocol);
        }
        #[cfg(not(target_os = "macos"))]
        return Err(WorkerError::Sandbox);
        #[cfg(target_os = "macos")]
        verify_worker_identity(&executable, &expected_sha256)?;
        verify_executable_identity(
            &launcher,
            &expected_launcher_sha256,
            MAX_LAUNCHER_EXECUTABLE_BYTES,
        )?;
        create_private_directory(&workspace_root)?;
        Ok(Self {
            executable,
            expected_sha256,
            launcher,
            expected_launcher_sha256,
            workspace_root,
            timeout,
        })
    }

    pub fn build(
        &self,
        project: &serde_json::Value,
        revision: &str,
    ) -> Result<WorkerRun, WorkerError> {
        if revision.is_empty() || revision.len() > 256 || revision.chars().any(char::is_control) {
            return Err(WorkerError::Protocol);
        }
        verify_worker_identity(&self.executable, &self.expected_sha256)?;
        verify_executable_identity(
            &self.launcher,
            &self.expected_launcher_sha256,
            MAX_LAUNCHER_EXECUTABLE_BYTES,
        )?;
        verify_private_directory(&self.workspace_root)?;
        let workspace = self
            .workspace_root
            .join(Uuid::now_v7().hyphenated().to_string());
        create_private_directory(&workspace)?;
        let worker_copy = workspace.join(if cfg!(windows) {
            "forme-product-worker.exe"
        } else {
            "forme-product-worker"
        });
        let launcher_copy = workspace.join("forme-sandbox-macos");
        let output_root = workspace.join("output");
        let result = (|| {
            copy_verified_executable(
                &self.executable,
                &worker_copy,
                &self.expected_sha256,
                MAX_WORKER_EXECUTABLE_BYTES,
            )?;
            copy_verified_executable(
                &self.launcher,
                &launcher_copy,
                &self.expected_launcher_sha256,
                MAX_LAUNCHER_EXECUTABLE_BYTES,
            )?;
            create_private_directory(&output_root)?;
            self.run_in_workspace(
                project,
                revision,
                &workspace,
                &output_root,
                &worker_copy,
                &launcher_copy,
            )
        })();
        match result {
            Ok(output) => Ok(WorkerRun { workspace, output }),
            Err(error) => match fs::remove_dir_all(&workspace) {
                Ok(()) => Err(error),
                Err(_) => Err(WorkerError::Output),
            },
        }
    }

    fn run_in_workspace(
        &self,
        project: &serde_json::Value,
        revision: &str,
        workspace: &Path,
        output_root: &Path,
        executable: &Path,
        launcher: &Path,
    ) -> Result<ValidatedWorkerOutput, WorkerError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Request<'a> {
            schema_version: u8,
            project: &'a serde_json::Value,
            revision: &'a str,
            output: &'a str,
        }

        let output = output_root.to_str().ok_or(WorkerError::Protocol)?;
        let request = serde_json::to_vec(&Request {
            schema_version: 1,
            project,
            revision,
            output,
        })
        .map_err(|_| WorkerError::Protocol)?;
        if request.len() > MAX_WORKER_MESSAGE_BYTES {
            return Err(WorkerError::Protocol);
        }

        let (mut command, readiness, readiness_writer) = launcher_command(
            launcher,
            executable,
            workspace,
            &self.expected_sha256,
            self.timeout,
        )?;
        command
            .current_dir(workspace)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|_| WorkerError::Sandbox)?;
        drop(readiness_writer);
        let mut stdin = child.stdin.take().ok_or(WorkerError::Io)?;
        let stdout = child.stdout.take().ok_or(WorkerError::Io)?;
        let stderr = child.stderr.take().ok_or(WorkerError::Io)?;
        let writer = thread::spawn(move || -> io::Result<()> {
            stdin.write_all(&request)?;
            stdin.flush()
        });
        let stdout_reader = thread::spawn(move || read_capped(stdout, MAX_WORKER_MESSAGE_BYTES));
        let stderr_reader = thread::spawn(move || read_capped(stderr, 1));
        let readiness_reader = thread::spawn(move || read_capped(readiness, 2_048));

        let deadline = Instant::now() + self.timeout;
        let mut next_scratch_check = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {}
                Err(_) => {
                    terminate_process_tree(&mut child);
                    let _ = child.wait();
                    let _ = join_io(writer);
                    let _ = join_io(stdout_reader);
                    let _ = join_io(stderr_reader);
                    let _ = join_io(readiness_reader);
                    return Err(WorkerError::Io);
                }
            }
            if Instant::now() >= deadline {
                terminate_process_tree(&mut child);
                let _ = child.wait();
                join_io(writer)?;
                let _ = join_io(stdout_reader);
                let _ = join_io(stderr_reader);
                let _ = join_io(readiness_reader);
                return Err(WorkerError::Timeout);
            }
            if Instant::now() >= next_scratch_check {
                if !scratch_within_budget(workspace) {
                    terminate_process_tree(&mut child);
                    let _ = child.wait();
                    let _ = join_io(writer);
                    let _ = join_io(stdout_reader);
                    let _ = join_io(stderr_reader);
                    let _ = join_io(readiness_reader);
                    return Err(WorkerError::Output);
                }
                next_scratch_check = Instant::now() + Duration::from_millis(10);
            }
            thread::sleep(Duration::from_millis(5));
        };
        if !scratch_within_budget(workspace) {
            let _ = join_io(writer);
            let _ = join_io(stdout_reader);
            let _ = join_io(stderr_reader);
            let _ = join_io(readiness_reader);
            return Err(WorkerError::Output);
        }
        join_io(writer)?;
        let response = join_io(stdout_reader)?;
        let error_output = join_io(stderr_reader)?;
        let readiness = join_io(readiness_reader)?;
        validate_launcher_readiness(&readiness, &self.expected_sha256)?;
        if !status.success() || !error_output.is_empty() {
            return Err(WorkerError::Protocol);
        }
        if response == br#"{"schemaVersion":1,"error":{"code":"BUILD_FAILED"}}"# {
            return Err(WorkerError::Build);
        }
        validate_worker_response(revision, output_root, &response)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LauncherReadiness {
    protocol: u8,
    provider: String,
    manifest_hash: String,
    config_schema_hash: Option<String>,
    entry_hash: String,
}

#[cfg(target_os = "macos")]
fn launcher_command(
    launcher: &Path,
    executable: &Path,
    workspace: &Path,
    expected_sha256: &str,
    timeout: Duration,
) -> Result<(Command, File, File), WorkerError> {
    use std::os::{
        fd::{AsRawFd, FromRawFd},
        unix::process::CommandExt,
    };
    let wall_milliseconds: u64 = timeout
        .as_millis()
        .try_into()
        .map_err(|_| WorkerError::Sandbox)?;
    let identity = format!("sha256:{expected_sha256}");
    let mut descriptors = [0_i32; 2];
    if unsafe { libc::pipe(descriptors.as_mut_ptr()) } != 0 {
        return Err(WorkerError::Sandbox);
    }
    for descriptor in descriptors {
        if unsafe { libc::fcntl(descriptor, libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
            unsafe {
                libc::close(descriptors[0]);
                libc::close(descriptors[1]);
            }
            return Err(WorkerError::Sandbox);
        }
    }
    let readiness = unsafe { File::from_raw_fd(descriptors[0]) };
    let readiness_writer = unsafe { File::from_raw_fd(descriptors[1]) };
    let readiness_descriptor = readiness_writer.as_raw_fd();
    let mut command = Command::new(launcher);
    command.args([
        "--provider=forme-macos-v1".to_owned(),
        format!("--manifest-hash={identity}"),
        "--schema-hash=-".to_owned(),
        format!("--entry-hash={identity}"),
        format!("--memory-bytes={MAX_RESIDENT_BYTES}"),
        format!("--cpu-ms={wall_milliseconds}"),
        format!("--wall-clock-ms={wall_milliseconds}"),
        "--fd-limit=64".to_owned(),
        format!("--working-directory={}", workspace.display()),
        "--runtime-kind=binary".to_owned(),
        format!("--runtime={}", executable.display()),
        format!("--runtime-root={}", workspace.display()),
        format!("--entry={}", executable.display()),
        "--schema=-".to_owned(),
        "--stage=authoring".to_owned(),
        "--instance=desktop".to_owned(),
    ]);
    unsafe {
        command.pre_exec(move || {
            if libc::setpgid(0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }
            let file_limit = libc::rlimit {
                rlim_cur: MAX_ARTIFACT_FILE_BYTES,
                rlim_max: MAX_ARTIFACT_FILE_BYTES,
            };
            if libc::setrlimit(libc::RLIMIT_FSIZE, &file_limit) != 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::dup2(readiness_descriptor, 3) < 0 {
                return Err(io::Error::last_os_error());
            }
            if readiness_descriptor != 3 {
                libc::close(readiness_descriptor);
            }
            Ok(())
        });
    }
    Ok((command, readiness, readiness_writer))
}

#[cfg(not(target_os = "macos"))]
fn launcher_command(
    _launcher: &Path,
    _executable: &Path,
    _workspace: &Path,
    _expected_sha256: &str,
    _timeout: Duration,
) -> Result<(Command, File, File), WorkerError> {
    Err(WorkerError::Sandbox)
}

fn validate_launcher_readiness(bytes: &[u8], expected_sha256: &str) -> Result<(), WorkerError> {
    let readiness: LauncherReadiness =
        serde_json::from_slice(bytes).map_err(|_| WorkerError::Sandbox)?;
    let identity = format!("sha256:{expected_sha256}");
    if readiness.protocol != 1
        || readiness.provider != "forme-macos-v1"
        || readiness.manifest_hash != identity
        || readiness.config_schema_hash.is_some()
        || readiness.entry_hash != identity
    {
        return Err(WorkerError::Sandbox);
    }
    Ok(())
}

#[cfg(unix)]
fn terminate_process_tree(child: &mut std::process::Child) {
    let process_group = -(child.id() as i32);
    unsafe {
        libc::kill(process_group, libc::SIGKILL);
    }
}

#[cfg(not(unix))]
fn terminate_process_tree(child: &mut std::process::Child) {
    let _ = child.kill();
}

fn copy_verified_executable(
    source: &Path,
    destination: &Path,
    expected_sha256: &str,
    maximum_bytes: u64,
) -> Result<(), WorkerError> {
    let metadata = fs::symlink_metadata(source).map_err(|_| WorkerError::Identity)?;
    if !metadata.file_type().is_file() || metadata.len() > maximum_bytes {
        return Err(WorkerError::Identity);
    }
    fs::copy(source, destination).map_err(|_| WorkerError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(destination, fs::Permissions::from_mode(0o500))
            .map_err(|_| WorkerError::Io)?;
    }
    verify_executable_identity(destination, expected_sha256, maximum_bytes)
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkerSuccess {
    schema_version: u8,
    revision: String,
    build_id: String,
    manifest_sha256: String,
    manifest: WorkerDeployManifest,
    files: Vec<WorkerFile>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkerDeployManifest {
    version: u8,
    file_count: usize,
    total_size_bytes: u64,
    files: BTreeMap<String, WorkerDeployFile>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkerDeployFile {
    output_path: String,
    content_type: String,
    size_bytes: u64,
    sha256: String,
    source: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedWorkerOutput {
    pub revision: String,
    pub build_id: String,
    pub manifest_sha256: String,
    pub files: Vec<WorkerFile>,
}

pub fn verify_worker_identity(path: &Path, expected_sha256: &str) -> Result<(), WorkerError> {
    verify_executable_identity(path, expected_sha256, MAX_WORKER_EXECUTABLE_BYTES)
}

fn verify_executable_identity(
    path: &Path,
    expected_sha256: &str,
    maximum_bytes: u64,
) -> Result<(), WorkerError> {
    if !valid_sha256(expected_sha256) {
        return Err(WorkerError::Identity);
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| WorkerError::Identity)?;
    if !metadata.file_type().is_file() || metadata.len() > maximum_bytes {
        return Err(WorkerError::Identity);
    }
    let digest = hash_file(path, metadata.len()).map_err(|_| WorkerError::Identity)?;
    if digest != expected_sha256 {
        return Err(WorkerError::Identity);
    }
    Ok(())
}

pub fn validate_worker_response(
    expected_revision: &str,
    output_root: &Path,
    response: &[u8],
) -> Result<ValidatedWorkerOutput, WorkerError> {
    if response.is_empty() || response.len() > MAX_WORKER_MESSAGE_BYTES {
        return Err(WorkerError::Protocol);
    }
    let parsed: WorkerSuccess =
        serde_json::from_slice(response).map_err(|_| WorkerError::Protocol)?;
    if serde_json::to_vec(&parsed).map_err(|_| WorkerError::Protocol)? != response {
        return Err(WorkerError::Protocol);
    }
    if parsed.schema_version != 1
        || parsed.revision != expected_revision
        || parsed.revision.is_empty()
        || parsed.revision.len() > 256
        || parsed.build_id.is_empty()
        || parsed.build_id.len() > 256
        || !valid_base64_sha256(&parsed.manifest_sha256)
        || parsed.manifest.version != 1
        || parsed.manifest.file_count != parsed.files.len()
        || parsed.manifest.files.len() != parsed.files.len()
        || parsed.manifest.file_count > MAX_ARTIFACT_FILES
        || parsed.files.len() > MAX_ARTIFACT_FILES
    {
        return Err(WorkerError::Protocol);
    }

    let mut declared = BTreeMap::new();
    let mut portable_identities = BTreeSet::new();
    let mut declared_total = 0_u64;
    for file in &parsed.files {
        let portable_identity = file.path.to_ascii_lowercase();
        if !portable_path(&file.path)
            || file.size > MAX_ARTIFACT_FILE_BYTES
            || !valid_sha256(&file.sha256)
            || declared.insert(file.path.clone(), file).is_some()
            || portable_identities.iter().any(|existing: &String| {
                portable_identity == *existing
                    || portable_identity.starts_with(&format!("{existing}/"))
                    || existing.starts_with(&format!("{portable_identity}/"))
            })
        {
            return Err(WorkerError::Protocol);
        }
        portable_identities.insert(portable_identity);
        declared_total = declared_total
            .checked_add(file.size)
            .ok_or(WorkerError::Protocol)?;
        if declared_total > MAX_ARTIFACT_BYTES {
            return Err(WorkerError::Protocol);
        }
    }
    if declared_total != parsed.manifest.total_size_bytes {
        return Err(WorkerError::Protocol);
    }
    for (path, entry) in &parsed.manifest.files {
        let declared_file = declared.get(path).ok_or(WorkerError::Protocol)?;
        if entry.output_path != *path
            || entry.size_bytes != declared_file.size
            || entry.sha256 != hex_to_base64(&declared_file.sha256).ok_or(WorkerError::Protocol)?
            || entry.content_type.is_empty()
            || entry.content_type.len() > 256
            || entry.source != "extra"
        {
            return Err(WorkerError::Protocol);
        }
    }
    let canonical = format!(
        "{}\n",
        serde_json::to_string_pretty(&parsed.manifest).map_err(|_| WorkerError::Protocol)?
    );
    let manifest_digest = BASE64.encode(Sha256::digest(canonical.as_bytes()));
    if manifest_digest != parsed.manifest_sha256 {
        return Err(WorkerError::Protocol);
    }

    let actual = collect_output_files(output_root)?;
    if actual.len() != declared.len() {
        return Err(WorkerError::Output);
    }
    for (path, (size, digest)) in actual {
        let expected = declared.get(&path).ok_or(WorkerError::Output)?;
        if expected.size != size || expected.sha256 != digest {
            return Err(WorkerError::Output);
        }
    }
    Ok(ValidatedWorkerOutput {
        revision: parsed.revision,
        build_id: parsed.build_id,
        manifest_sha256: parsed.manifest_sha256,
        files: parsed.files,
    })
}

fn collect_output_files(root: &Path) -> Result<BTreeMap<String, (u64, String)>, WorkerError> {
    let metadata = fs::symlink_metadata(root).map_err(|_| WorkerError::Output)?;
    if !metadata.file_type().is_dir() {
        return Err(WorkerError::Output);
    }
    let mut pending = vec![(root.to_path_buf(), 0_usize)];
    let mut files = BTreeMap::new();
    let mut total = 0_u64;
    let mut entries_seen = 0_usize;
    let mut directories_seen = 0_usize;
    while let Some((directory, depth)) = pending.pop() {
        directories_seen = directories_seen.checked_add(1).ok_or(WorkerError::Output)?;
        if depth > MAX_ARTIFACT_DEPTH || directories_seen > MAX_ARTIFACT_DIRECTORIES {
            return Err(WorkerError::Output);
        }
        let entries = fs::read_dir(&directory).map_err(|_| WorkerError::Output)?;
        for entry in entries {
            let entry = entry.map_err(|_| WorkerError::Output)?;
            entries_seen = entries_seen.checked_add(1).ok_or(WorkerError::Output)?;
            if entries_seen > MAX_ARTIFACT_ENTRIES {
                return Err(WorkerError::Output);
            }
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|_| WorkerError::Output)?;
            if metadata.file_type().is_symlink() {
                return Err(WorkerError::Output);
            }
            if metadata.file_type().is_dir() {
                pending.push((path, depth + 1));
                continue;
            }
            if !metadata.file_type().is_file() || metadata.len() > MAX_ARTIFACT_FILE_BYTES {
                return Err(WorkerError::Output);
            }
            total = total
                .checked_add(metadata.len())
                .ok_or(WorkerError::Output)?;
            if total > MAX_ARTIFACT_BYTES || files.len() >= MAX_ARTIFACT_FILES {
                return Err(WorkerError::Output);
            }
            let relative = path.strip_prefix(root).map_err(|_| WorkerError::Output)?;
            let portable = portable_from_path(relative).ok_or(WorkerError::Output)?;
            let digest = hash_file(&path, metadata.len()).map_err(|_| WorkerError::Output)?;
            if files.insert(portable, (metadata.len(), digest)).is_some() {
                return Err(WorkerError::Output);
            }
        }
    }
    Ok(files)
}

fn scratch_within_budget(root: &Path) -> bool {
    let mut pending = vec![(root.to_path_buf(), 0_usize)];
    let mut total = 0_u64;
    let mut entries_seen = 0_usize;
    let mut directories_seen = 0_usize;
    while let Some((directory, depth)) = pending.pop() {
        directories_seen = match directories_seen.checked_add(1) {
            Some(value) => value,
            None => return false,
        };
        if depth > MAX_ARTIFACT_DEPTH + 2 || directories_seen > MAX_ARTIFACT_DIRECTORIES + 2 {
            return false;
        }
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(_) => return false,
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => return false,
            };
            entries_seen = match entries_seen.checked_add(1) {
                Some(value) => value,
                None => return false,
            };
            if entries_seen > MAX_ARTIFACT_ENTRIES + 4 {
                return false;
            }
            let metadata = match fs::symlink_metadata(entry.path()) {
                Ok(metadata) => metadata,
                Err(_) => return false,
            };
            if metadata.file_type().is_symlink() {
                return false;
            }
            if metadata.file_type().is_dir() {
                pending.push((entry.path(), depth + 1));
            } else if metadata.file_type().is_file() {
                total = match total.checked_add(metadata.len()) {
                    Some(value) => value,
                    None => return false,
                };
                if total > MAX_SCRATCH_BYTES {
                    return false;
                }
            } else {
                return false;
            }
        }
    }
    true
}

fn hash_file(path: &Path, expected_size: u64) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut observed = 0_u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        observed += count as u64;
        if observed > expected_size {
            return Err(io::Error::other("file changed while hashing"));
        }
        hasher.update(&buffer[..count]);
    }
    if observed != expected_size {
        return Err(io::Error::other("file changed while hashing"));
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn portable_from_path(path: &Path) -> Option<String> {
    let mut output = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => output.push(value.to_str()?.to_owned()),
            _ => return None,
        }
    }
    let value = output.join("/");
    portable_path(&value).then_some(value)
}

fn portable_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.chars().any(unsafe_text)
        && value.split('/').count() <= MAX_ARTIFACT_DEPTH
        && value.split('/').all(portable_component)
}

fn portable_component(component: &str) -> bool {
    if component.is_empty()
        || component == "."
        || component == ".."
        || component.len() > 255
        || component.ends_with('.')
        || component.ends_with(' ')
        || component.contains(':')
    {
        return false;
    }
    let stem = component
        .split_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(component)
        .to_ascii_lowercase();
    !matches!(
        stem.as_str(),
        "con"
            | "prn"
            | "aux"
            | "nul"
            | "com1"
            | "com2"
            | "com3"
            | "com4"
            | "com5"
            | "com6"
            | "com7"
            | "com8"
            | "com9"
            | "lpt1"
            | "lpt2"
            | "lpt3"
            | "lpt4"
            | "lpt5"
            | "lpt6"
            | "lpt7"
            | "lpt8"
            | "lpt9"
    )
}

fn unsafe_text(value: char) -> bool {
    value.is_control()
        || matches!(
            value,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

fn valid_base64_sha256(value: &str) -> bool {
    value.len() == 44 && BASE64.decode(value).is_ok_and(|bytes| bytes.len() == 32)
}

fn hex_to_base64(value: &str) -> Option<String> {
    if !valid_sha256(value) {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(pair).ok()?;
        bytes[index] = u8::from_str_radix(text, 16).ok()?;
    }
    Some(BASE64.encode(bytes))
}

fn read_capped(mut reader: impl Read, maximum: usize) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(output);
        }
        if output.len().saturating_add(count) > maximum {
            return Err(io::Error::other("worker output exceeded its bound"));
        }
        output.extend_from_slice(&buffer[..count]);
    }
}

fn join_io<T>(handle: thread::JoinHandle<io::Result<T>>) -> Result<T, WorkerError> {
    handle
        .join()
        .map_err(|_| WorkerError::Io)?
        .map_err(|_| WorkerError::Protocol)
}

fn create_private_directory(path: &Path) -> Result<(), WorkerError> {
    fs::create_dir_all(path).map_err(|_| WorkerError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| WorkerError::Io)?;
    }
    verify_private_directory(path)
}

fn verify_private_directory(path: &Path) -> Result<(), WorkerError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| WorkerError::Io)?;
    if !metadata.file_type().is_dir() {
        return Err(WorkerError::Output);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
        if metadata.uid() != unsafe { libc::geteuid() }
            || metadata.permissions().mode() & 0o077 != 0
            || !crate::security::ancestors_have_no_mutating_acl(path)
        {
            return Err(WorkerError::Output);
        }
        let directory = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)
            .map_err(|_| WorkerError::Output)?;
        if !crate::security::has_no_mutating_acl(&directory) {
            return Err(WorkerError::Output);
        }
    }
    Ok(())
}
