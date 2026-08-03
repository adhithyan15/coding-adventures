//! Secure user-scoped publication and native registration for the Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_daemon_service_files::{
    render_launchd, render_systemd_user, render_windows_task, ServiceFile, ServiceFileError,
    ServicePlatform, LAUNCHD_LABEL, WINDOWS_TASK_NAME,
};
use std::ffi::OsString;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_PATH_BYTES: usize = 4096;
const TEMP_ATTEMPTS: u64 = 32;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

/// Platform-specific, explicit inputs needed to install for one user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallTarget {
    /// macOS LaunchAgent installation.
    Launchd {
        /// Absolute current-user home directory.
        home: String,
        /// Current graphical user identifier used by launchctl's `gui` domain.
        user_id: u32,
        /// Absolute path to the trusted launchctl executable.
        launchctl: String,
    },
    /// Linux systemd user-service installation.
    SystemdUser {
        /// Absolute current-user home directory.
        home: String,
        /// Absolute path to the trusted systemctl executable.
        systemctl: String,
    },
    /// Windows Task Scheduler installation.
    WindowsTaskScheduler {
        /// Absolute current-user roaming AppData directory.
        app_data: String,
        /// Absolute path to the trusted schtasks executable.
        schtasks: String,
    },
}

/// Explicit request for one deterministic daemon installation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallRequest {
    /// Platform and native registration inputs.
    pub target: InstallTarget,
    /// Absolute path to the Chief daemon executable.
    pub executable: String,
    /// Absolute path to the Chief TOML configuration.
    pub config_path: String,
}

/// One shell-free native supervisor invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeCommand {
    /// Absolute trusted executable path.
    pub program: String,
    /// Tokenized native arguments.
    pub args: Vec<OsString>,
    /// Whether failure aborts registration.
    pub required: bool,
}

/// Pure validated install plan that may be reviewed before mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallPlan {
    /// Target native supervisor.
    platform: ServicePlatform,
    /// Existing trusted per-user root directory.
    user_root: String,
    /// Missing descendant directories to create in order.
    directories: Vec<String>,
    /// Concrete final definition path.
    definition_path: String,
    /// Rendered native definition.
    service_file: ServiceFile,
    /// Direct native registration invocations.
    commands: Vec<NativeCommand>,
    executable: String,
    config_path: String,
}

impl InstallPlan {
    /// Return the target native supervisor.
    pub fn platform(&self) -> ServicePlatform {
        self.platform
    }

    /// Return the existing trusted per-user root directory.
    pub fn user_root(&self) -> &str {
        &self.user_root
    }

    /// Return the descendant directories that will be created or validated.
    pub fn directories(&self) -> &[String] {
        &self.directories
    }

    /// Return the concrete final definition path.
    pub fn definition_path(&self) -> &str {
        &self.definition_path
    }

    /// Return the rendered native service definition.
    pub fn service_file(&self) -> &ServiceFile {
        &self.service_file
    }

    /// Return the direct native registration invocations.
    pub fn commands(&self) -> &[NativeCommand] {
        &self.commands
    }
}

/// Result of publishing the native definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Publication {
    /// The absent final name was claimed atomically.
    Created,
    /// A secure existing definition was byte-identical and reused.
    Unchanged,
}

/// Successful installation receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallReceipt {
    /// Concrete installed definition path.
    pub definition_path: String,
    /// Whether this call created or reused the definition.
    pub publication: Publication,
    /// Number of required native registration commands that succeeded.
    pub required_commands_run: usize,
}

/// Stable failure category that never includes caller-controlled path text.
#[derive(Debug)]
pub enum InstallError {
    /// A service-file input failed platform validation.
    ServiceFile(ServiceFileError),
    /// A user root or native supervisor executable path was invalid.
    InvalidInstallPath,
    /// The plan targets a different operating system than the current process.
    WrongPlatform,
    /// A daemon, config, directory, or definition path was linked or non-regular.
    UnsafeFileType,
    /// A required directory could not be inspected or created safely.
    DirectoryUnavailable,
    /// The definition could not be written, synchronized, or published.
    PublicationFailed,
    /// An existing definition was not owner-only on Unix.
    InsecurePermissions,
    /// An existing definition differed and was preserved.
    DefinitionConflict,
    /// A required native supervisor command failed.
    RegistrationFailed,
}

impl Display for InstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ServiceFile(_) => "chief daemon install: service definition rejected",
            Self::InvalidInstallPath => "chief daemon install: invalid install path",
            Self::WrongPlatform => "chief daemon install: plan targets another platform",
            Self::UnsafeFileType => "chief daemon install: unsafe file type",
            Self::DirectoryUnavailable => "chief daemon install: directory unavailable",
            Self::PublicationFailed => "chief daemon install: publication failed",
            Self::InsecurePermissions => "chief daemon install: insecure permissions",
            Self::DefinitionConflict => "chief daemon install: definition conflict",
            Self::RegistrationFailed => "chief daemon install: native registration failed",
        })
    }
}

impl std::error::Error for InstallError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ServiceFile(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ServiceFileError> for InstallError {
    fn from(error: ServiceFileError) -> Self {
        Self::ServiceFile(error)
    }
}

/// Injectable shell-free command execution boundary.
pub trait CommandExecutor {
    /// Execute one exact native command and report whether it succeeded.
    fn execute(&mut self, command: &NativeCommand) -> bool;
}

/// Production executor that suppresses native tool output and never invokes a shell.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemCommandExecutor;

impl CommandExecutor for SystemCommandExecutor {
    fn execute(&mut self, command: &NativeCommand) -> bool {
        Command::new(&command.program)
            .args(&command.args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }
}

/// Produce a pure, validated platform install plan.
pub fn plan_install(request: &InstallRequest) -> Result<InstallPlan, InstallError> {
    match &request.target {
        InstallTarget::Launchd {
            home,
            user_id,
            launchctl,
        } => {
            validate_unix_absolute(home)?;
            validate_unix_absolute(launchctl)?;
            let service_file = render_launchd(&request.executable, &request.config_path)?;
            let library = join_unix(home, "Library");
            let agents = join_unix(&library, "LaunchAgents");
            let definition_path = join_unix(&agents, "dev.chiefofstaff.plist");
            let domain = format!("gui/{user_id}");
            Ok(InstallPlan {
                platform: ServicePlatform::Launchd,
                user_root: home.clone(),
                directories: vec![library, agents],
                definition_path: definition_path.clone(),
                service_file,
                commands: vec![
                    NativeCommand {
                        program: launchctl.clone(),
                        args: vec!["bootout".into(), format!("{domain}/{LAUNCHD_LABEL}").into()],
                        required: false,
                    },
                    NativeCommand {
                        program: launchctl.clone(),
                        args: vec!["bootstrap".into(), domain.into(), definition_path.into()],
                        required: true,
                    },
                ],
                executable: request.executable.clone(),
                config_path: request.config_path.clone(),
            })
        }
        InstallTarget::SystemdUser { home, systemctl } => {
            validate_unix_absolute(home)?;
            validate_unix_absolute(systemctl)?;
            let service_file = render_systemd_user(&request.executable, &request.config_path)?;
            let config = join_unix(home, ".config");
            let systemd = join_unix(&config, "systemd");
            let user = join_unix(&systemd, "user");
            let definition_path = join_unix(&user, "chief-of-staff.service");
            Ok(InstallPlan {
                platform: ServicePlatform::SystemdUser,
                user_root: home.clone(),
                directories: vec![config, systemd, user],
                definition_path,
                service_file,
                commands: vec![
                    NativeCommand {
                        program: systemctl.clone(),
                        args: vec!["--user".into(), "daemon-reload".into()],
                        required: true,
                    },
                    NativeCommand {
                        program: systemctl.clone(),
                        args: vec![
                            "--user".into(),
                            "enable".into(),
                            "--now".into(),
                            "chief-of-staff.service".into(),
                        ],
                        required: true,
                    },
                ],
                executable: request.executable.clone(),
                config_path: request.config_path.clone(),
            })
        }
        InstallTarget::WindowsTaskScheduler { app_data, schtasks } => {
            validate_windows_absolute(app_data)?;
            validate_windows_absolute(schtasks)?;
            let service_file = render_windows_task(&request.executable, &request.config_path)?;
            let chief = join_windows(app_data, "ChiefOfStaff");
            let tasks = join_windows(&chief, "Tasks");
            let definition_path = join_windows(&tasks, "daemon.xml");
            Ok(InstallPlan {
                platform: ServicePlatform::WindowsTaskScheduler,
                user_root: app_data.clone(),
                directories: vec![chief, tasks],
                definition_path: definition_path.clone(),
                service_file,
                commands: vec![NativeCommand {
                    program: schtasks.clone(),
                    args: vec![
                        "/Create".into(),
                        "/TN".into(),
                        WINDOWS_TASK_NAME.into(),
                        "/XML".into(),
                        definition_path.into(),
                        "/F".into(),
                    ],
                    required: true,
                }],
                executable: request.executable.clone(),
                config_path: request.config_path.clone(),
            })
        }
    }
}

/// Apply one reviewed plan and register it with the current user's supervisor.
pub fn apply_install(
    plan: &InstallPlan,
    executor: &mut impl CommandExecutor,
) -> Result<InstallReceipt, InstallError> {
    ensure_current_platform(plan.platform)?;
    validate_regular_input(Path::new(&plan.executable))?;
    validate_regular_input(Path::new(&plan.config_path))?;
    for command in &plan.commands {
        validate_regular_input(Path::new(&command.program))?;
    }
    validate_root(Path::new(&plan.user_root))?;
    for directory in &plan.directories {
        ensure_directory(Path::new(directory))?;
    }
    let publication = publish_definition(
        Path::new(&plan.definition_path),
        plan.service_file.contents.as_bytes(),
        plan.service_file.mode,
    )?;

    let mut required_commands_run = 0;
    for command in &plan.commands {
        let success = executor.execute(command);
        if command.required {
            if !success {
                return Err(InstallError::RegistrationFailed);
            }
            required_commands_run += 1;
        }
    }
    Ok(InstallReceipt {
        definition_path: plan.definition_path.clone(),
        publication,
        required_commands_run,
    })
}

/// Plan, publish, and register one user-scoped daemon definition.
pub fn install(
    request: &InstallRequest,
    executor: &mut impl CommandExecutor,
) -> Result<InstallReceipt, InstallError> {
    let plan = plan_install(request)?;
    apply_install(&plan, executor)
}

fn ensure_current_platform(platform: ServicePlatform) -> Result<(), InstallError> {
    let matches = match platform {
        ServicePlatform::Launchd => cfg!(target_os = "macos"),
        ServicePlatform::SystemdUser => cfg!(target_os = "linux"),
        ServicePlatform::WindowsTaskScheduler => cfg!(windows),
    };
    if matches {
        Ok(())
    } else {
        Err(InstallError::WrongPlatform)
    }
}

fn validate_unix_absolute(value: &str) -> Result<(), InstallError> {
    if invalid_text(value)
        || !value.starts_with('/')
        || value
            .split('/')
            .any(|component| matches!(component, "." | ".."))
    {
        Err(InstallError::InvalidInstallPath)
    } else {
        Ok(())
    }
}

fn validate_windows_absolute(value: &str) -> Result<(), InstallError> {
    let bytes = value.as_bytes();
    let drive = bytes.first().is_some_and(u8::is_ascii_alphabetic)
        && bytes.get(1) == Some(&b':')
        && bytes.get(2) == Some(&b'\\');
    let unc = value.starts_with(r"\\")
        && value
            .trim_start_matches('\\')
            .split('\\')
            .filter(|part| !part.is_empty())
            .take(2)
            .count()
            == 2;
    if invalid_text(value)
        || value.contains('/')
        || (!drive && !unc)
        || value
            .split('\\')
            .any(|component| matches!(component, "." | ".."))
    {
        Err(InstallError::InvalidInstallPath)
    } else {
        Ok(())
    }
}

fn invalid_text(value: &str) -> bool {
    value.is_empty()
        || value.len() > MAX_PATH_BYTES
        || value.chars().any(char::is_control)
        || value.contains('"')
}

fn join_unix(root: &str, child: &str) -> String {
    format!("{}/{}", root.trim_end_matches('/'), child)
}

fn join_windows(root: &str, child: &str) -> String {
    format!(r"{}\{}", root.trim_end_matches('\\'), child)
}

fn validate_regular_input(path: &Path) -> Result<(), InstallError> {
    validate_native_absolute(path)?;
    validate_ancestor_chain(path.parent().ok_or(InstallError::InvalidInstallPath)?)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| InstallError::UnsafeFileType)?;
    if !metadata.is_file() || metadata_is_link(&metadata) {
        return Err(InstallError::UnsafeFileType);
    }
    Ok(())
}

fn validate_root(path: &Path) -> Result<(), InstallError> {
    validate_native_absolute(path)?;
    validate_ancestor_chain(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| InstallError::DirectoryUnavailable)?;
    if !metadata.is_dir() || metadata_is_link(&metadata) {
        return Err(InstallError::DirectoryUnavailable);
    }
    #[cfg(unix)]
    verify_directory_permissions(&metadata)?;
    Ok(())
}

fn validate_native_absolute(path: &Path) -> Result<(), InstallError> {
    if !path.is_absolute()
        || path.as_os_str().len() > MAX_PATH_BYTES
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        Err(InstallError::InvalidInstallPath)
    } else {
        Ok(())
    }
}

fn validate_ancestor_chain(path: &Path) -> Result<(), InstallError> {
    let mut ancestors: Vec<_> = path.ancestors().collect();
    ancestors.reverse();
    for ancestor in ancestors {
        if ancestor.as_os_str().is_empty() {
            continue;
        }
        let metadata = fs::symlink_metadata(ancestor).map_err(|_| InstallError::UnsafeFileType)?;
        if !metadata.is_dir() || metadata_is_link(&metadata) {
            return Err(InstallError::UnsafeFileType);
        }
    }
    Ok(())
}

fn ensure_directory(path: &Path) -> Result<(), InstallError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_dir() || metadata_is_link(&metadata) {
                return Err(InstallError::DirectoryUnavailable);
            }
            #[cfg(unix)]
            verify_directory_permissions(&metadata)?;
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or(InstallError::DirectoryUnavailable)?;
            let parent_metadata =
                fs::symlink_metadata(parent).map_err(|_| InstallError::DirectoryUnavailable)?;
            if !parent_metadata.is_dir() || metadata_is_link(&parent_metadata) {
                return Err(InstallError::DirectoryUnavailable);
            }
            fs::create_dir(path).map_err(|_| InstallError::DirectoryUnavailable)?;
            #[cfg(unix)]
            fs::set_permissions(path, unix_permissions(0o700))
                .map_err(|_| InstallError::DirectoryUnavailable)?;
            let metadata =
                fs::symlink_metadata(path).map_err(|_| InstallError::DirectoryUnavailable)?;
            if !metadata.is_dir() || metadata_is_link(&metadata) {
                return Err(InstallError::DirectoryUnavailable);
            }
            #[cfg(unix)]
            verify_directory_permissions(&metadata)?;
            Ok(())
        }
        Err(_) => Err(InstallError::DirectoryUnavailable),
    }
}

fn publish_definition(
    path: &Path,
    contents: &[u8],
    mode: Option<u32>,
) -> Result<Publication, InstallError> {
    match fs::symlink_metadata(path) {
        Ok(existing) => return verify_existing(path, &existing, contents, mode),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => return Err(InstallError::PublicationFailed),
    }
    let parent = path.parent().ok_or(InstallError::InvalidInstallPath)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(InstallError::InvalidInstallPath)?;
    for _ in 0..TEMP_ATTEMPTS {
        let nonce = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(".{file_name}.{}.{}.tmp", std::process::id(), nonce));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = match options.open(&temporary) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(InstallError::PublicationFailed),
        };
        let write_result = file.write_all(contents).and_then(|()| file.sync_all());
        drop(file);
        if write_result.is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(InstallError::PublicationFailed);
        }
        match fs::hard_link(&temporary, path) {
            Ok(()) => {
                let _ = fs::remove_file(&temporary);
                sync_parent(parent)?;
                let metadata =
                    fs::symlink_metadata(path).map_err(|_| InstallError::PublicationFailed)?;
                verify_existing(path, &metadata, contents, mode)?;
                return Ok(Publication::Created);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&temporary);
                let metadata =
                    fs::symlink_metadata(path).map_err(|_| InstallError::PublicationFailed)?;
                return verify_existing(path, &metadata, contents, mode);
            }
            Err(_) => {
                let _ = fs::remove_file(&temporary);
                return Err(InstallError::PublicationFailed);
            }
        }
    }
    Err(InstallError::PublicationFailed)
}

fn verify_existing(
    path: &Path,
    metadata: &fs::Metadata,
    contents: &[u8],
    mode: Option<u32>,
) -> Result<Publication, InstallError> {
    if !metadata.is_file() || metadata_is_link(metadata) {
        return Err(InstallError::UnsafeFileType);
    }
    #[cfg(unix)]
    if let Some(required_mode) = mode {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o777 != required_mode {
            return Err(InstallError::InsecurePermissions);
        }
    }
    #[cfg(not(unix))]
    let _ = mode;
    if metadata.len() != contents.len() as u64 {
        return Err(InstallError::DefinitionConflict);
    }
    let mut existing = Vec::with_capacity(contents.len() + 1);
    File::open(path)
        .and_then(|file| {
            file.take((contents.len() + 1) as u64)
                .read_to_end(&mut existing)
        })
        .map_err(|_| InstallError::PublicationFailed)?;
    if existing == contents {
        Ok(Publication::Unchanged)
    } else {
        Err(InstallError::DefinitionConflict)
    }
}

fn sync_parent(parent: &Path) -> Result<(), InstallError> {
    #[cfg(unix)]
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| InstallError::PublicationFailed)?;
    #[cfg(not(unix))]
    let _ = parent;
    Ok(())
}

#[cfg(unix)]
fn unix_permissions(mode: u32) -> fs::Permissions {
    use std::os::unix::fs::PermissionsExt;
    fs::Permissions::from_mode(mode)
}

#[cfg(unix)]
fn verify_directory_permissions(metadata: &fs::Metadata) -> Result<(), InstallError> {
    use std::os::unix::fs::PermissionsExt;
    if metadata.permissions().mode() & 0o022 != 0 {
        Err(InstallError::InsecurePermissions)
    } else {
        Ok(())
    }
}

fn metadata_is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct RecordingExecutor {
        commands: Vec<NativeCommand>,
        fail_required: bool,
    }

    impl CommandExecutor for RecordingExecutor {
        fn execute(&mut self, command: &NativeCommand) -> bool {
            self.commands.push(command.clone());
            !(command.required && self.fail_required)
        }
    }

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = fs::canonicalize(std::env::temp_dir()).unwrap();
            let path = root.join(format!(
                "chief-installer-{label}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            #[cfg(unix)]
            fs::set_permissions(&path, unix_permissions(0o700)).unwrap();
            Self(path)
        }

        fn text(&self) -> String {
            self.0.to_string_lossy().into_owned()
        }

        fn input(&self, name: &str) -> String {
            let path = self.0.join(name);
            fs::write(&path, b"fixture").unwrap();
            path.to_string_lossy().into_owned()
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn plans_all_platforms_with_absolute_shell_free_commands() {
        let launchd = plan_install(&InstallRequest {
            target: InstallTarget::Launchd {
                home: "/Users/alice".into(),
                user_id: 501,
                launchctl: "/bin/launchctl".into(),
            },
            executable: "/Users/alice/bin/chief-of-staff-daemon".into(),
            config_path: "/Users/alice/.chief-of-staff/config.toml".into(),
        })
        .unwrap();
        assert_eq!(launchd.commands.len(), 2);
        assert!(!launchd.commands[0].required);
        assert_eq!(launchd.commands[1].args[1], "gui/501");

        let systemd = plan_install(&InstallRequest {
            target: InstallTarget::SystemdUser {
                home: "/home/alice".into(),
                systemctl: "/usr/bin/systemctl".into(),
            },
            executable: "/home/alice/bin/chief-of-staff-daemon".into(),
            config_path: "/home/alice/.chief-of-staff/config.toml".into(),
        })
        .unwrap();
        assert_eq!(systemd.commands.len(), 2);
        assert!(systemd.commands.iter().all(|command| command.required));

        let windows = plan_install(&InstallRequest {
            target: InstallTarget::WindowsTaskScheduler {
                app_data: r"C:\Users\Alice\AppData\Roaming".into(),
                schtasks: r"C:\Windows\System32\schtasks.exe".into(),
            },
            executable: r"C:\Chief\chief-of-staff-daemon.exe".into(),
            config_path: r"C:\Users\Alice\.chief-of-staff\config.toml".into(),
        })
        .unwrap();
        assert_eq!(windows.commands.len(), 1);
        assert_eq!(windows.commands[0].args[0], "/Create");
        assert_eq!(windows.commands[0].args[2], WINDOWS_TASK_NAME);
    }

    #[test]
    fn invalid_install_roots_and_tools_fail_closed() {
        let request = InstallRequest {
            target: InstallTarget::SystemdUser {
                home: "relative".into(),
                systemctl: "/usr/bin/systemctl".into(),
            },
            executable: "/bin/chief-of-staff-daemon".into(),
            config_path: "/tmp/config.toml".into(),
        };
        assert!(matches!(
            plan_install(&request),
            Err(InstallError::InvalidInstallPath)
        ));
    }

    #[test]
    fn wrong_platform_is_rejected_before_mutation() {
        let platform = if cfg!(target_os = "macos") {
            ServicePlatform::SystemdUser
        } else {
            ServicePlatform::Launchd
        };
        let plan = InstallPlan {
            platform,
            user_root: "/does/not/matter".into(),
            directories: Vec::new(),
            definition_path: "/does/not/matter/service".into(),
            service_file: ServiceFile {
                platform,
                install_path: "unused",
                contents: String::new(),
                mode: None,
            },
            commands: Vec::new(),
            executable: "/does/not/matter/daemon".into(),
            config_path: "/does/not/matter/config".into(),
        };
        let mut executor = RecordingExecutor {
            commands: Vec::new(),
            fail_required: false,
        };
        assert!(matches!(
            apply_install(&plan, &mut executor),
            Err(InstallError::WrongPlatform)
        ));
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn native_request(directory: &TestDirectory) -> InstallRequest {
        let executable = directory.input("chief-of-staff-daemon");
        let config_path = directory.input("config.toml");
        let supervisor = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        #[cfg(target_os = "macos")]
        let target = InstallTarget::Launchd {
            home: directory.text(),
            user_id: 501,
            launchctl: supervisor,
        };
        #[cfg(target_os = "linux")]
        let target = InstallTarget::SystemdUser {
            home: directory.text(),
            systemctl: supervisor,
        };
        InstallRequest {
            target,
            executable,
            config_path,
        }
    }

    #[cfg(windows)]
    fn native_request(directory: &TestDirectory) -> InstallRequest {
        InstallRequest {
            target: InstallTarget::WindowsTaskScheduler {
                app_data: directory.text(),
                schtasks: directory.input("schtasks.exe"),
            },
            executable: directory.input("chief-of-staff-daemon.exe"),
            config_path: directory.input("config.toml"),
        }
    }

    #[test]
    fn publication_is_atomic_idempotent_and_registration_retryable() {
        let directory = TestDirectory::new("round-trip");
        let request = native_request(&directory);
        let mut executor = RecordingExecutor {
            commands: Vec::new(),
            fail_required: true,
        };
        assert!(matches!(
            install(&request, &mut executor),
            Err(InstallError::RegistrationFailed)
        ));
        let plan = plan_install(&request).unwrap();
        assert_eq!(
            fs::read(&plan.definition_path).unwrap(),
            plan.service_file.contents.as_bytes()
        );

        executor.fail_required = false;
        let receipt = install(&request, &mut executor).unwrap();
        assert_eq!(receipt.publication, Publication::Unchanged);
        assert!(receipt.required_commands_run >= 1);

        fs::write(&plan.definition_path, b"tampered").unwrap();
        assert!(matches!(
            install(&request, &mut executor),
            Err(InstallError::DefinitionConflict)
        ));
        assert_eq!(fs::read(&plan.definition_path).unwrap(), b"tampered");
    }

    #[cfg(unix)]
    #[test]
    fn linked_inputs_and_broad_user_roots_are_rejected() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new("links");
        let request = native_request(&directory);
        let target = directory.0.join("real-config");
        fs::write(&target, b"fixture").unwrap();
        let link = directory.0.join("linked-config");
        symlink(&target, &link).unwrap();
        let mut linked = request.clone();
        linked.config_path = link.to_string_lossy().into_owned();
        let mut executor = RecordingExecutor {
            commands: Vec::new(),
            fail_required: false,
        };
        assert!(matches!(
            install(&linked, &mut executor),
            Err(InstallError::UnsafeFileType)
        ));

        fs::set_permissions(&directory.0, unix_permissions(0o777)).unwrap();
        assert!(matches!(
            install(&request, &mut executor),
            Err(InstallError::InsecurePermissions)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn broad_existing_service_directories_are_rejected() {
        let directory = TestDirectory::new("directory-permissions");
        let request = native_request(&directory);
        let plan = plan_install(&request).unwrap();
        let first = Path::new(&plan.directories[0]);
        fs::create_dir(first).unwrap();
        fs::set_permissions(first, unix_permissions(0o770)).unwrap();
        let mut executor = RecordingExecutor {
            commands: Vec::new(),
            fail_required: false,
        };

        assert!(matches!(
            apply_install(&plan, &mut executor),
            Err(InstallError::InsecurePermissions)
        ));
        assert!(!Path::new(&plan.definition_path).exists());
        assert!(executor.commands.is_empty());
    }

    #[test]
    fn non_regular_supervisor_is_rejected_before_mutation() {
        let directory = TestDirectory::new("supervisor-type");
        let request = native_request(&directory);
        let mut plan = plan_install(&request).unwrap();
        plan.commands[0].program = directory.text();
        let mut executor = RecordingExecutor {
            commands: Vec::new(),
            fail_required: false,
        };

        assert!(matches!(
            apply_install(&plan, &mut executor),
            Err(InstallError::UnsafeFileType)
        ));
        assert!(!Path::new(&plan.definition_path).exists());
        assert!(executor.commands.is_empty());
    }

    #[test]
    fn stable_errors_do_not_echo_paths() {
        let messages = [
            InstallError::InvalidInstallPath.to_string(),
            InstallError::WrongPlatform.to_string(),
            InstallError::UnsafeFileType.to_string(),
            InstallError::DirectoryUnavailable.to_string(),
            InstallError::PublicationFailed.to_string(),
            InstallError::InsecurePermissions.to_string(),
            InstallError::DefinitionConflict.to_string(),
            InstallError::RegistrationFailed.to_string(),
        ];
        assert!(messages
            .iter()
            .all(|message| message.starts_with("chief daemon install:")));
    }
}
