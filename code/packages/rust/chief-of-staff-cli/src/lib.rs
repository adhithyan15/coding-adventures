//! Concrete local operator CLI for the D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_cli_core::{
    execute, parse_argv, render_result, CliAction, CliError, VaultCommand, VaultPut,
};
use chief_of_staff_daemon::{
    load_config_file, open_chief_vault, resolve_startup_paths, ChiefDaemonError,
};
use chief_of_staff_daemon_api::{DaemonClient, DaemonClientError};
use chief_of_staff_daemon_credential::{load_or_create_credential, CredentialFileError};
use chief_of_staff_daemon_installer::{
    install, CommandExecutor, InstallError, InstallRequest, InstallTarget, Publication,
    SystemCommandExecutor,
};
use chief_of_staff_vault_secret_store::{ChiefSecretStore, StoreError, MAX_PAYLOAD_BYTES};
use coding_adventures_json_value::JsonValue;
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Display, Formatter};
use std::env;
use std::ffi::OsString;
use std::io::{self, IsTerminal, Read, Write};
use std::path::Path;
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use websocket_runtime::WebSocketClientOptions;

const CONFIG_SUFFIX_UNIX: &str = ".chief-of-staff/config.toml";
const CONFIG_SUFFIX_WINDOWS: &str = ".chief-of-staff\\config.toml";
const DAEMON_TARGET: &str = "/chief";

/// Platform values needed to derive one native current-user installation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeInstallPlatform {
    /// macOS launchd user domain.
    Launchd {
        /// Current graphical user identifier.
        user_id: u32,
        /// Absolute trusted launchctl path.
        launchctl: String,
    },
    /// Linux systemd user manager.
    SystemdUser {
        /// Absolute trusted systemctl path.
        systemctl: String,
    },
    /// Windows Task Scheduler current-user task.
    WindowsTaskScheduler {
        /// Absolute roaming AppData directory.
        app_data: String,
        /// Absolute trusted schtasks path.
        schtasks: String,
    },
}

/// Explicit environment snapshot used to derive a reviewable install request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallEnvironment {
    /// Absolute current-user home directory.
    pub home: String,
    /// Absolute path of the running `chief-of-staff` executable.
    pub current_executable: String,
    /// Native current-user supervisor values.
    pub platform: NativeInstallPlatform,
}

/// Derive the sibling daemon and default config paths without mutating the host.
pub fn build_install_request(
    environment: &InstallEnvironment,
) -> Result<InstallRequest, CliAppError> {
    let windows = matches!(
        environment.platform,
        NativeInstallPlatform::WindowsTaskScheduler { .. }
    );
    let executable = sibling_executable(&environment.current_executable, windows)?;
    let config_path = if windows {
        join_windows(&environment.home, CONFIG_SUFFIX_WINDOWS)
    } else {
        join_unix(&environment.home, CONFIG_SUFFIX_UNIX)
    };
    let target = match &environment.platform {
        NativeInstallPlatform::Launchd { user_id, launchctl } => InstallTarget::Launchd {
            home: environment.home.clone(),
            user_id: *user_id,
            launchctl: launchctl.clone(),
        },
        NativeInstallPlatform::SystemdUser { systemctl } => InstallTarget::SystemdUser {
            home: environment.home.clone(),
            systemctl: systemctl.clone(),
        },
        NativeInstallPlatform::WindowsTaskScheduler { app_data, schtasks } => {
            InstallTarget::WindowsTaskScheduler {
                app_data: app_data.clone(),
                schtasks: schtasks.clone(),
            }
        }
    };
    Ok(InstallRequest {
        target,
        executable,
        config_path,
    })
}

/// Where `vault put` reads its secret from.
///
/// A trait so tests can supply bytes without a real stdin, and so the
/// terminal check (D18U U-C2) is part of the contract rather than an
/// afterthought.
pub trait SecretInput {
    /// Whether the input is an interactive terminal, which would echo a
    /// typed secret to the screen.
    fn is_terminal(&self) -> bool;
    /// Read at most `limit` bytes, all the way to end of input.
    fn read_up_to(&mut self, limit: usize) -> io::Result<Zeroizing<Vec<u8>>>;
}

/// The process's real stdin.
pub struct StdinSecretInput;

impl SecretInput for StdinSecretInput {
    fn is_terminal(&self) -> bool {
        io::stdin().is_terminal()
    }

    fn read_up_to(&mut self, limit: usize) -> io::Result<Zeroizing<Vec<u8>>> {
        read_bounded(&mut io::stdin().lock(), limit)
    }
}

/// Read up to `limit` bytes into one zeroizing allocation made up front.
///
/// The buffer is sized once and never grows: a `Vec` that reallocates leaves
/// the old copy of the secret behind, unzeroed (D18U U-E6).
pub fn read_bounded(reader: &mut impl Read, limit: usize) -> io::Result<Zeroizing<Vec<u8>>> {
    let mut buffer = Zeroizing::new(vec![0_u8; limit]);
    let mut filled = 0;
    while filled < limit {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    buffer.truncate(filled);
    Ok(buffer)
}

/// Parse explicit argv values, perform one action, and write only public output.
pub fn run<I, W>(args: I, output: &mut W) -> Result<(), CliAppError>
where
    I: IntoIterator<Item = OsString>,
    W: Write,
{
    run_with_input(args, &mut StdinSecretInput, output)
}

/// [`run`], with the secret input supplied by the caller.
pub fn run_with_input<I, W>(
    args: I,
    input: &mut impl SecretInput,
    output: &mut W,
) -> Result<(), CliAppError>
where
    I: IntoIterator<Item = OsString>,
    W: Write,
{
    let argv = args
        .into_iter()
        .map(|value| {
            value
                .into_string()
                .map_err(|_| CliAppError::InvalidInvocation)
        })
        .collect::<Result<Vec<_>, _>>()?;
    match parse_argv(&argv).map_err(CliAppError::Core)? {
        CliAction::Help(help) => write_public(output, &help),
        CliAction::Version(version) => write_public(output, &version),
        CliAction::InstallDaemon => install_daemon(output),
        CliAction::Command(command) => execute_remote(command, output),
        CliAction::Vault(command) => execute_vault(command, input, output),
    }
}

/// Render an error and every `source()` beneath it, one per `: ` segment.
///
/// The top-level messages are deliberately generic ("command failed"), so
/// printing only them hides the one fact an operator needs — which flag was
/// missing, which secret record is corrupt. Every error type in this chain is
/// payload-blind by construction (field names, secret *names*, never values),
/// so walking the chain adds detail without adding secrets.
pub fn describe_error(error: &CliAppError) -> String {
    use std::error::Error as _;
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        // Some errors already print their cause inside their own message
        // (the declarative parser's does); repeating it would only add noise.
        let cause_text = cause.to_string();
        if !text.contains(&cause_text) {
            text.push_str(": ");
            text.push_str(&cause_text);
        }
        source = cause.source();
    }
    text
}

/// Resolve process argv and environment and run one CLI action.
pub fn run_from_env() -> Result<(), CliAppError> {
    let stdout = io::stdout();
    run(env::args_os(), &mut stdout.lock())
}

fn execute_remote(
    command: chief_of_staff_cli_core::CliCommand,
    output: &mut impl Write,
) -> Result<(), CliAppError> {
    let home = platform_home().ok_or(CliAppError::HomeUnavailable)?;
    let paths = resolve_startup_paths(Vec::<OsString>::new(), Some(home.clone()))
        .map_err(CliAppError::Daemon)?;
    execute_remote_at(command, paths.home(), paths.config(), output)
}

fn execute_remote_at(
    command: chief_of_staff_cli_core::CliCommand,
    home: &Path,
    config_path: &Path,
    output: &mut impl Write,
) -> Result<(), CliAppError> {
    let config = load_config_file(config_path).map_err(CliAppError::Daemon)?;
    let credential_path = config
        .orchestrator()
        .credential_path()
        .resolve(home)
        .map_err(|_| CliAppError::ConfigPathUnavailable)?;
    let credential =
        load_or_create_credential(&credential_path).map_err(CliAppError::Credential)?;
    let host = config.orchestrator().bind().to_string();
    let mut client = DaemonClient::connect(
        &host,
        config.orchestrator().port(),
        DAEMON_TARGET,
        WebSocketClientOptions::default(),
    )
    .map_err(CliAppError::Client)?;
    if let Err(error) = client.authenticate(&credential) {
        let _ = client.close();
        return Err(CliAppError::Client(error));
    }
    let result = execute(&mut client, command).map_err(CliAppError::Core);
    let close = client.close().map_err(CliAppError::Client);
    let rendered = render_result(&result?).map_err(CliAppError::Core)?;
    close?;
    output
        .write_all(rendered.as_bytes())
        .map_err(|_| CliAppError::OutputUnavailable)
}

fn execute_vault(
    command: VaultCommand,
    input: &mut impl SecretInput,
    output: &mut impl Write,
) -> Result<(), CliAppError> {
    let home = platform_home().ok_or(CliAppError::HomeUnavailable)?;
    let paths = resolve_startup_paths(Vec::<OsString>::new(), Some(home.clone()))
        .map_err(CliAppError::Daemon)?;
    execute_vault_at(
        command,
        paths.home(),
        paths.config(),
        input,
        unix_time_ms()?,
        output,
    )
}

/// Run one vault command against the configured store (D18U U-C1..U-C7).
///
/// The vault is opened **before** stdin is read. If the configuration or the
/// KEK is wrong, the secret is never read into memory at all.
fn execute_vault_at(
    command: VaultCommand,
    home: &Path,
    config_path: &Path,
    input: &mut impl SecretInput,
    now_ms: u64,
    output: &mut impl Write,
) -> Result<(), CliAppError> {
    let config = load_config_file(config_path).map_err(CliAppError::Daemon)?;
    let store = ChiefSecretStore::new(
        open_chief_vault(&config, home)
            .map_err(CliAppError::Daemon)?
            .ok_or(CliAppError::VaultNotConfigured)?,
    );
    let result = match command {
        VaultCommand::Put(put) => {
            put_secret(&store, &put, input, now_ms)?;
            changed("put", put.name.as_str())
        }
        VaultCommand::Delete(name) => {
            store.delete(&name).map_err(CliAppError::Vault)?;
            changed("delete", name.as_str())
        }
        VaultCommand::List => JsonValue::Object(vec![(
            "secrets".to_string(),
            JsonValue::Array(
                store
                    .names()
                    .map_err(CliAppError::Vault)?
                    .into_iter()
                    .map(|name| JsonValue::String(name.as_str().to_string()))
                    .collect(),
            ),
        )]),
    };
    let rendered = render_result(&result).map_err(CliAppError::Core)?;
    output
        .write_all(rendered.as_bytes())
        .map_err(|_| CliAppError::OutputUnavailable)
}

fn put_secret(
    store: &ChiefSecretStore,
    put: &VaultPut,
    input: &mut impl SecretInput,
    now_ms: u64,
) -> Result<(), CliAppError> {
    if input.is_terminal() {
        return Err(CliAppError::SecretInputIsTerminal);
    }
    // Two bytes past the payload bound: room for one stripped "\r\n" on a
    // maximum-size secret, and anything beyond that is visibly too long
    // rather than silently truncated (U-C1).
    let stdin = input
        .read_up_to(MAX_PAYLOAD_BYTES + 3)
        .map_err(|_| CliAppError::SecretInputUnavailable)?;
    let secret = put.secret_bytes(&stdin);
    if secret.len() > MAX_PAYLOAD_BYTES {
        return Err(CliAppError::SecretTooLarge);
    }
    store
        .put(&put.name, &put.policy_at(now_ms), secret)
        .map_err(CliAppError::Vault)
}

/// The result of a write: what happened, to which secret, and when the daemon
/// will see it. Never the value (U-C6).
fn changed(action: &str, name: &str) -> JsonValue {
    JsonValue::Object(vec![
        ("action".to_string(), JsonValue::String(action.to_string())),
        ("secret".to_string(), JsonValue::String(name.to_string())),
        (
            "effective".to_string(),
            JsonValue::String("after the next daemon restart".to_string()),
        ),
    ])
}

fn unix_time_ms() -> Result<u64, CliAppError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| CliAppError::ClockUnavailable)?;
    u64::try_from(elapsed.as_millis()).map_err(|_| CliAppError::ClockUnavailable)
}

fn install_daemon(output: &mut impl Write) -> Result<(), CliAppError> {
    let environment = install_environment()?;
    let request = build_install_request(&environment)?;
    apply_install_request(&request, &mut SystemCommandExecutor, output)
}

fn apply_install_request(
    request: &InstallRequest,
    executor: &mut impl CommandExecutor,
    output: &mut impl Write,
) -> Result<(), CliAppError> {
    load_config_file(Path::new(&request.config_path)).map_err(CliAppError::Daemon)?;
    let receipt = install(request, executor).map_err(CliAppError::Install)?;
    let publication = match receipt.publication {
        Publication::Created => "created",
        Publication::Unchanged => "unchanged",
    };
    writeln!(
        output,
        "Chief daemon service {publication}: {}",
        receipt.definition_path
    )
    .map_err(|_| CliAppError::OutputUnavailable)
}

fn write_public(output: &mut impl Write, value: &str) -> Result<(), CliAppError> {
    output
        .write_all(value.as_bytes())
        .and_then(|()| {
            if value.ends_with('\n') {
                Ok(())
            } else {
                output.write_all(b"\n")
            }
        })
        .map_err(|_| CliAppError::OutputUnavailable)
}

fn sibling_executable(current: &str, windows: bool) -> Result<String, CliAppError> {
    if windows {
        let (parent, _) = current
            .rsplit_once('\\')
            .ok_or(CliAppError::CurrentExecutableUnavailable)?;
        if parent.is_empty() || current.contains('/') {
            return Err(CliAppError::CurrentExecutableUnavailable);
        }
        Ok(format!(r"{parent}\chief-of-staff-daemon.exe"))
    } else {
        unix_sibling_path(current, "chief-of-staff-daemon")
            .ok_or(CliAppError::CurrentExecutableUnavailable)
    }
}

/// Derive a POSIX sibling path with plain string splitting instead of
/// `std::path::Path`. This branch always targets a POSIX native supervisor
/// (launchd or systemd), regardless of which platform `chief-of-staff-cli`
/// itself was built and tested on — but `Path::join` uses the *host*
/// platform's separator, so on a Windows host it silently produced a mixed
/// `/opt/chief\chief-of-staff-daemon` path instead of the required
/// `/opt/chief/chief-of-staff-daemon`. Matches `Path`'s own POSIX parent
/// semantics (root normalizes to `/`, a trailing slash is insignificant, a
/// bare relative name has no usable parent) without depending on the host's
/// separator convention.
fn unix_sibling_path(current: &str, filename: &str) -> Option<String> {
    let trimmed = current.trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.rsplit_once('/') {
        Some(("", _name)) => Some(format!("/{filename}")),
        Some((parent, _name)) => Some(format!("{parent}/{filename}")),
        None => None,
    }
}

fn join_unix(root: &str, suffix: &str) -> String {
    format!("{}/{}", root.trim_end_matches('/'), suffix)
}

fn join_windows(root: &str, suffix: &str) -> String {
    format!(r"{}\{}", root.trim_end_matches('\\'), suffix)
}

fn install_environment() -> Result<InstallEnvironment, CliAppError> {
    let home = os_text(platform_home().ok_or(CliAppError::HomeUnavailable)?)?;
    let current_executable = env::current_exe()
        .map_err(|_| CliAppError::CurrentExecutableUnavailable)?
        .into_os_string();
    let current_executable = os_text(current_executable)?;
    let platform = native_install_platform()?;
    Ok(InstallEnvironment {
        home,
        current_executable,
        platform,
    })
}

#[cfg(target_os = "macos")]
fn native_install_platform() -> Result<NativeInstallPlatform, CliAppError> {
    let output = Command::new("/usr/bin/id")
        .arg("-u")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|_| CliAppError::PlatformUnavailable)?;
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|_| CliAppError::PlatformUnavailable)?
        .trim();
    if !output.status.success()
        || text.is_empty()
        || text.len() > 10
        || !text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(CliAppError::PlatformUnavailable);
    }
    let user_id = text
        .parse::<u32>()
        .map_err(|_| CliAppError::PlatformUnavailable)?;
    Ok(NativeInstallPlatform::Launchd {
        user_id,
        launchctl: "/bin/launchctl".to_string(),
    })
}

#[cfg(target_os = "linux")]
fn native_install_platform() -> Result<NativeInstallPlatform, CliAppError> {
    Ok(NativeInstallPlatform::SystemdUser {
        systemctl: "/usr/bin/systemctl".to_string(),
    })
}

#[cfg(windows)]
fn native_install_platform() -> Result<NativeInstallPlatform, CliAppError> {
    let app_data = os_text(env::var_os("APPDATA").ok_or(CliAppError::PlatformUnavailable)?)?;
    let system_root = os_text(env::var_os("SystemRoot").ok_or(CliAppError::PlatformUnavailable)?)?;
    Ok(NativeInstallPlatform::WindowsTaskScheduler {
        app_data,
        schtasks: join_windows(&system_root, r"System32\schtasks.exe"),
    })
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn native_install_platform() -> Result<NativeInstallPlatform, CliAppError> {
    Err(CliAppError::UnsupportedPlatform)
}

#[cfg(unix)]
fn platform_home() -> Option<OsString> {
    env::var_os("HOME")
}

#[cfg(windows)]
fn platform_home() -> Option<OsString> {
    env::var_os("USERPROFILE")
}

#[cfg(not(any(unix, windows)))]
fn platform_home() -> Option<OsString> {
    None
}

fn os_text(value: OsString) -> Result<String, CliAppError> {
    value
        .into_string()
        .map_err(|_| CliAppError::PlatformUnavailable)
}

/// Stable top-level CLI failure that never includes secrets or adapter payloads.
#[derive(Debug)]
pub enum CliAppError {
    /// Argv was not valid Unicode for the declarative parser.
    InvalidInvocation,
    /// The current user's home environment was unavailable.
    HomeUnavailable,
    /// The running CLI path or its sibling daemon path was unavailable.
    CurrentExecutableUnavailable,
    /// Required native per-user platform values were unavailable.
    PlatformUnavailable,
    /// The selected target has no supported native daemon installer.
    UnsupportedPlatform,
    /// A configured home-relative path could not be resolved.
    ConfigPathUnavailable,
    /// Declarative parsing, dispatch, or rendering failed.
    Core(CliError),
    /// Configuration loading failed.
    Daemon(ChiefDaemonError),
    /// Local operator credential loading failed.
    Credential(CredentialFileError),
    /// Authenticated WebSocket client setup or exchange failed.
    Client(DaemonClientError),
    /// Native daemon publication or registration failed.
    Install(InstallError),
    /// Public stdout output failed.
    OutputUnavailable,
    /// `[vault] kek_path` is not configured, so there is no vault to write.
    VaultNotConfigured,
    /// `vault put` was run with an interactive terminal on stdin.
    SecretInputIsTerminal,
    /// The secret could not be read from stdin.
    SecretInputUnavailable,
    /// The secret on stdin is larger than a sealed record can hold.
    SecretTooLarge,
    /// The sealed secret store refused the operation.
    Vault(StoreError),
    /// The wall clock was unavailable for the record's rotation time.
    ClockUnavailable,
}

impl Display for CliAppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInvocation => "chief CLI: invalid invocation",
            Self::HomeUnavailable => "chief CLI: home directory unavailable",
            Self::CurrentExecutableUnavailable => "chief CLI: executable path unavailable",
            Self::PlatformUnavailable => "chief CLI: platform values unavailable",
            Self::UnsupportedPlatform => "chief CLI: unsupported platform",
            Self::ConfigPathUnavailable => "chief CLI: configured path unavailable",
            Self::Core(_) => "chief CLI: command failed",
            Self::Daemon(_) => "chief CLI: configuration unavailable",
            Self::Credential(_) => "chief CLI: credential unavailable",
            Self::Client(_) => "chief CLI: daemon connection failed",
            Self::Install(_) => "chief CLI: daemon installation failed",
            Self::OutputUnavailable => "chief CLI: output unavailable",
            Self::VaultNotConfigured => {
                "chief CLI: no vault configured; set [vault] kek_path to an owner-only 32-byte key file"
            }
            Self::SecretInputIsTerminal => {
                "chief CLI: refusing to read a secret from a terminal, which would echo it; pipe it on stdin"
            }
            Self::SecretInputUnavailable => "chief CLI: could not read the secret from stdin",
            Self::SecretTooLarge => "chief CLI: secret is larger than 64 KiB",
            Self::Vault(_) => "chief CLI: vault operation failed",
            Self::ClockUnavailable => "chief CLI: wall clock unavailable",
        })
    }
}

impl std::error::Error for CliAppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Core(error) => Some(error),
            Self::Daemon(error) => Some(error),
            Self::Credential(error) => Some(error),
            Self::Client(error) => Some(error),
            Self::Install(error) => Some(error),
            Self::Vault(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_daemon_installer::{plan_install, NativeCommand};
    use std::error::Error;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use websocket_core::{Frame, MessageEvent};
    use websocket_runtime::{
        StopHandle, WebSocketConnectionInfo, WebSocketHandlerResult, WebSocketRuntime,
        WebSocketServerOptions,
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    type HostPlatform = transport_platform::bsd::KqueueTransportPlatform;
    #[cfg(target_os = "linux")]
    type HostPlatform = transport_platform::linux::EpollTransportPlatform;
    #[cfg(windows)]
    type HostPlatform = transport_platform::windows::WindowsTransportPlatform;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let root = fs::canonicalize(env::temp_dir()).unwrap();
            let path = root.join(format!(
                "chief-cli-{label}-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            }
            Self(path)
        }

        fn text(&self) -> String {
            self.0.to_string_lossy().into_owned()
        }

        fn write_config(&self, port: u16) -> PathBuf {
            let chief = self.0.join(".chief-of-staff");
            fs::create_dir_all(chief.join("run")).unwrap();
            let config = chief.join("config.toml");
            fs::write(
                &config,
                format!(
                    r#"
[orchestrator]
bind = "127.0.0.1"
port = {port}
packages_dir = "~/.chief-of-staff/agents/"
state_dir = "~/.chief-of-staff/state/"
credential_path = "~/.chief-of-staff/run/operator.credential"

[keyring]
trusted_keys = [
  {{ id = "prod-001", path = "~/.chief-of-staff/keys/prod.pub", type = "production" }},
]

[hosts.defaults]
restart_policy = "on-failure"
health_check_interval = 5000
executable = "~/.chief-of-staff/bin/chief-of-staff-host"
bootstrap_timeout = 10000
graceful_stop_timeout = 5000

[vault]
storage_path = "~/.chief-of-staff/vault/"
default_lease_ttl = 30
container = true

[privilege]
tier_1_auto_approve_timeout = 5
biometric_timeout = 30
hardware_key_timeout = 60
"#
                ),
            )
            .unwrap();
            config
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct RecordingExecutor(Vec<NativeCommand>);

    impl CommandExecutor for RecordingExecutor {
        fn execute(&mut self, command: &NativeCommand) -> bool {
            self.0.push(command.clone());
            true
        }
    }

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn help_and_version_do_not_touch_host_configuration() {
        let mut help = Vec::new();
        run(args(&["chief-of-staff", "--help"]), &mut help).unwrap();
        let help = String::from_utf8(help).unwrap();
        assert!(help.contains("chief-of-staff [OPTIONS]"));
        assert!(help.contains("install-daemon"));
        assert!(help.contains("agents"));

        let mut version = Vec::new();
        run(args(&["chief-of-staff", "--version"]), &mut version).unwrap();
        assert_eq!(version, b"0.1.0\n");
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        windows
    ))]
    fn start_mock_daemon(
        accept_credential: bool,
    ) -> (std::net::SocketAddr, StopHandle, thread::JoinHandle<()>) {
        let options = WebSocketServerOptions::default();
        let handler = move |_: WebSocketConnectionInfo, phase: &mut u8, event: MessageEvent| {
            let MessageEvent::Text(_) = event else {
                return WebSocketHandlerResult::default();
            };
            *phase += 1;
            let response = match *phase {
                1 if !accept_credential => {
                    r#"{"version":1,"id":"1","ok":false,"error":{"code":"authentication_failed","message":"authentication failed"}}"#
                }
                1 => r#"{"version":1,"id":"1","ok":true,"result":{"authenticated":true}}"#,
                2 => r#"{"version":1,"id":"2","ok":true,"result":[]}"#,
                _ => {
                    r#"{"version":1,"id":"3","ok":false,"error":{"code":"invalid_request","message":"invalid request"}}"#
                }
            };
            WebSocketHandlerResult::send(Frame::text(response))
        };

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        let mut runtime: WebSocketRuntime<HostPlatform, u8> =
            WebSocketRuntime::bind_kqueue_with_state(
                "127.0.0.1:0",
                options,
                |_| 0,
                handler,
                |_, _| {},
            )
            .unwrap();
        #[cfg(target_os = "linux")]
        let mut runtime: WebSocketRuntime<HostPlatform, u8> =
            WebSocketRuntime::bind_epoll_with_state(
                "127.0.0.1:0",
                options,
                |_| 0,
                handler,
                |_, _| {},
            )
            .unwrap();
        #[cfg(windows)]
        let mut runtime: WebSocketRuntime<HostPlatform, u8> =
            WebSocketRuntime::bind_windows_with_state(
                "127.0.0.1:0",
                options,
                |_| 0,
                handler,
                |_, _| {},
            )
            .unwrap();

        let address = runtime.local_addr();
        let stop = runtime.stop_handle();
        let server = thread::spawn(move || runtime.serve().unwrap());
        (address, stop, server)
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        windows
    ))]
    #[test]
    fn remote_command_loads_local_authority_and_round_trips_over_loopback() {
        let (address, stop, server) = start_mock_daemon(true);
        let directory = TestDirectory::new("remote");
        let config = directory.write_config(address.port());
        let mut output = Vec::new();

        execute_remote_at(
            chief_of_staff_cli_core::CliCommand::Agents,
            &directory.0,
            &config,
            &mut output,
        )
        .unwrap();

        assert_eq!(output, b"[]\n");
        let credential =
            fs::read(directory.0.join(".chief-of-staff/run/operator.credential")).unwrap();
        assert_eq!(credential.len(), 64);
        assert!(credential.iter().all(u8::is_ascii_hexdigit));
        stop.stop();
        server.join().unwrap();
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        windows
    ))]
    #[test]
    fn authentication_and_result_output_failures_are_stable() {
        let directory = TestDirectory::new("remote-errors");
        let (address, stop, server) = start_mock_daemon(false);
        let config = directory.write_config(address.port());
        let error = execute_remote_at(
            chief_of_staff_cli_core::CliCommand::Agents,
            &directory.0,
            &config,
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(matches!(error, CliAppError::Client(_)));
        assert_eq!(error.to_string(), "chief CLI: daemon connection failed");
        stop.stop();
        server.join().unwrap();

        let (address, stop, server) = start_mock_daemon(true);
        directory.write_config(address.port());
        let error = execute_remote_at(
            chief_of_staff_cli_core::CliCommand::Agents,
            &directory.0,
            &config,
            &mut FailingWriter,
        )
        .unwrap_err();
        assert!(matches!(error, CliAppError::OutputUnavailable));
        stop.stop();
        server.join().unwrap();
    }

    #[test]
    fn derives_reviewable_requests_for_every_native_supervisor() {
        let launchd = build_install_request(&InstallEnvironment {
            home: "/Users/alice".into(),
            current_executable: "/opt/chief/chief-of-staff".into(),
            platform: NativeInstallPlatform::Launchd {
                user_id: 501,
                launchctl: "/bin/launchctl".into(),
            },
        })
        .unwrap();
        assert_eq!(launchd.executable, "/opt/chief/chief-of-staff-daemon");
        assert_eq!(
            launchd.config_path,
            "/Users/alice/.chief-of-staff/config.toml"
        );
        assert!(plan_install(&launchd).is_ok());

        let systemd = build_install_request(&InstallEnvironment {
            home: "/home/alice".into(),
            current_executable: "/opt/chief/chief-of-staff".into(),
            platform: NativeInstallPlatform::SystemdUser {
                systemctl: "/usr/bin/systemctl".into(),
            },
        })
        .unwrap();
        assert!(plan_install(&systemd).is_ok());

        let windows = build_install_request(&InstallEnvironment {
            home: r"C:\Users\Alice".into(),
            current_executable: r"C:\Chief\chief-of-staff.exe".into(),
            platform: NativeInstallPlatform::WindowsTaskScheduler {
                app_data: r"C:\Users\Alice\AppData\Roaming".into(),
                schtasks: r"C:\Windows\System32\schtasks.exe".into(),
            },
        })
        .unwrap();
        assert_eq!(windows.executable, r"C:\Chief\chief-of-staff-daemon.exe");
        assert_eq!(
            windows.config_path,
            r"C:\Users\Alice\.chief-of-staff\config.toml"
        );
        assert!(plan_install(&windows).is_ok());
    }

    #[cfg(any(target_os = "macos", target_os = "linux", windows))]
    fn native_test_environment(directory: &TestDirectory) -> InstallEnvironment {
        #[cfg(windows)]
        let cli = directory.0.join("chief-of-staff.exe");
        #[cfg(not(windows))]
        let cli = directory.0.join("chief-of-staff");
        fs::write(&cli, b"fixture").unwrap();
        #[cfg(windows)]
        let daemon = directory.0.join("chief-of-staff-daemon.exe");
        #[cfg(not(windows))]
        let daemon = directory.0.join("chief-of-staff-daemon");
        fs::write(daemon, b"fixture").unwrap();

        #[cfg(target_os = "macos")]
        let platform = NativeInstallPlatform::Launchd {
            user_id: 501,
            launchctl: env::current_exe().unwrap().to_string_lossy().into_owned(),
        };
        #[cfg(target_os = "linux")]
        let platform = NativeInstallPlatform::SystemdUser {
            systemctl: env::current_exe().unwrap().to_string_lossy().into_owned(),
        };
        #[cfg(windows)]
        let platform = {
            let schtasks = directory.0.join("schtasks.exe");
            fs::write(&schtasks, b"fixture").unwrap();
            NativeInstallPlatform::WindowsTaskScheduler {
                app_data: directory.text(),
                schtasks: schtasks.to_string_lossy().into_owned(),
            }
        };
        InstallEnvironment {
            home: directory.text(),
            current_executable: cli.to_string_lossy().into_owned(),
            platform,
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux", windows))]
    #[test]
    fn validated_native_install_is_retryable_without_real_process_execution() {
        let directory = TestDirectory::new("install");
        directory.write_config(7463);
        let request = build_install_request(&native_test_environment(&directory)).unwrap();
        let mut executor = RecordingExecutor(Vec::new());
        let mut created = Vec::new();
        apply_install_request(&request, &mut executor, &mut created).unwrap();
        assert!(String::from_utf8(created)
            .unwrap()
            .contains("service created:"));
        assert!(!executor.0.is_empty());

        let mut unchanged = Vec::new();
        apply_install_request(&request, &mut executor, &mut unchanged).unwrap();
        assert!(String::from_utf8(unchanged)
            .unwrap()
            .contains("service unchanged:"));
    }

    #[test]
    fn invalid_sibling_paths_fail_stably() {
        let error = build_install_request(&InstallEnvironment {
            home: "/home/alice".into(),
            current_executable: "chief-of-staff".into(),
            platform: NativeInstallPlatform::SystemdUser {
                systemctl: "/usr/bin/systemctl".into(),
            },
        })
        .unwrap_err();
        assert!(matches!(error, CliAppError::CurrentExecutableUnavailable));
        assert_eq!(error.to_string(), "chief CLI: executable path unavailable");

        let error = sibling_executable(r"\chief-of-staff.exe", true).unwrap_err();
        assert!(matches!(error, CliAppError::CurrentExecutableUnavailable));
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("fixture"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn invalid_argv_and_stdout_errors_remain_payload_blind() {
        let output_error =
            run(args(&["chief-of-staff", "--help"]), &mut FailingWriter).unwrap_err();
        assert!(matches!(output_error, CliAppError::OutputUnavailable));
        assert_eq!(output_error.to_string(), "chief CLI: output unavailable");

        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let invocation_error = run(
                vec![
                    OsString::from("chief-of-staff"),
                    OsString::from_vec(vec![0xff]),
                ],
                &mut Vec::new(),
            )
            .unwrap_err();
            assert!(matches!(invocation_error, CliAppError::InvalidInvocation));
            assert_eq!(
                invocation_error.to_string(),
                "chief CLI: invalid invocation"
            );
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux", windows))]
    #[test]
    fn production_environment_resolves_to_a_reviewable_native_plan() {
        let environment = install_environment().unwrap();
        let request = build_install_request(&environment).unwrap();
        assert!(plan_install(&request).is_ok());
    }

    #[test]
    fn every_top_level_error_has_stable_display_and_source_behavior() {
        let core = parse_argv(&["chief-of-staff".to_string()]).unwrap_err();
        let errors = [
            CliAppError::HomeUnavailable,
            CliAppError::PlatformUnavailable,
            CliAppError::UnsupportedPlatform,
            CliAppError::ConfigPathUnavailable,
            CliAppError::Core(core),
            CliAppError::Daemon(ChiefDaemonError::HomeUnavailable),
            CliAppError::Credential(CredentialFileError::InvalidPath),
            CliAppError::Client(DaemonClientError::InvalidCredential),
            CliAppError::Install(InstallError::WrongPlatform),
        ];
        for error in &errors {
            assert!(error.to_string().starts_with("chief CLI:"));
        }
        assert!(errors[0].source().is_none());
        assert!(errors[4..].iter().all(|error| error.source().is_some()));
    }

    // ── vault (D18U "Provisioning commands") ─────────────────────────────────

    use chief_of_staff_vault_runtime::{ChiefVaultRuntime, VaultDeliveryMode, VaultLeaseRequest};
    use chief_of_staff_vault_secret_store::SecretName;

    /// Stdin stand-in: fixed bytes, a terminal flag, and a record of whether
    /// anything was read, so tests can prove a refusal happened *before* the
    /// secret was touched.
    struct FakeInput {
        terminal: bool,
        bytes: Vec<u8>,
        reads: usize,
    }

    impl FakeInput {
        fn piped(bytes: &[u8]) -> Self {
            Self {
                terminal: false,
                bytes: bytes.to_vec(),
                reads: 0,
            }
        }
    }

    impl SecretInput for FakeInput {
        fn is_terminal(&self) -> bool {
            self.terminal
        }
        fn read_up_to(&mut self, limit: usize) -> io::Result<Zeroizing<Vec<u8>>> {
            self.reads += 1;
            read_bounded(&mut self.bytes.as_slice(), limit)
        }
    }

    /// A home with a config whose `[vault]` has an owner-only 32-byte KEK.
    fn vault_home(label: &str) -> (TestDirectory, PathBuf) {
        let directory = TestDirectory::new(label);
        let config = directory.write_config(1);
        let text = fs::read_to_string(&config).unwrap().replace(
            "container = true\n",
            "container = true\nkek_path = \"~/.chief-of-staff/vault.kek\"\n",
        );
        fs::write(&config, text).unwrap();
        let kek = directory.0.join(".chief-of-staff").join("vault.kek");
        fs::write(&kek, [0x5a_u8; 32]).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&kek, fs::Permissions::from_mode(0o600)).unwrap();
        }
        (directory, config)
    }

    fn vault_run(
        directory: &TestDirectory,
        config: &Path,
        argv: &[&str],
        input: &mut FakeInput,
    ) -> Result<String, CliAppError> {
        let action = parse_argv(&argv.iter().map(|v| v.to_string()).collect::<Vec<_>>())
            .map_err(CliAppError::Core)?;
        let CliAction::Vault(command) = action else {
            panic!("expected a vault action");
        };
        let mut output = Vec::new();
        execute_vault_at(
            command,
            &directory.0,
            config,
            input,
            1_790_000_000_000,
            &mut output,
        )?;
        Ok(String::from_utf8(output).unwrap())
    }

    fn open_store(directory: &TestDirectory, config: &Path) -> ChiefSecretStore {
        let config = load_config_file(config).unwrap();
        ChiefSecretStore::new(open_chief_vault(&config, &directory.0).unwrap().unwrap())
    }

    const PUT: &[&str] = &[
        "chief-of-staff",
        "vault",
        "put",
        "weather-key",
        "--mode",
        "leased",
        "--tier",
        "1",
        "--allow-agent",
        "weather-host",
    ];

    #[test]
    fn vault_put_seals_a_record_the_daemon_loader_can_register() {
        let (directory, config) = vault_home("vault-put");
        let mut input = FakeInput::piped(b"api-key-value\n");
        let output = vault_run(&directory, &config, PUT, &mut input).unwrap();

        // U-C6: the output names the secret and never contains its value.
        assert!(output.contains("\"action\": \"put\""));
        assert!(output.contains("\"secret\": \"weather-key\""));
        assert!(output.contains("after the next daemon restart"));
        assert!(!output.contains("api-key-value"));

        // What the daemon will do at startup: register, then lease.
        let runtime = ChiefVaultRuntime::new();
        assert_eq!(
            open_store(&directory, &config)
                .register_all(&runtime)
                .unwrap(),
            1
        );
        let policy = runtime.secret_policy("weather-key").unwrap();
        assert_eq!(policy.allowed_mode, VaultDeliveryMode::Leased);
        assert_eq!(policy.privilege_tier, 1);
        assert_eq!(policy.rotated_at_ms, 1_790_000_000_000);
        let receipt = runtime
            .request_lease(VaultLeaseRequest {
                requesting_agent_id: Some("weather-host"),
                secret_name: "weather-key",
                ttl_ms: 60_000,
            })
            .unwrap();
        // U-C3: the one trailing newline `echo` adds is gone.
        assert_eq!(
            runtime.consume(&receipt.vault_ref).unwrap().as_bytes(),
            b"api-key-value"
        );
    }

    #[test]
    fn vault_list_and_delete_round_trip() {
        let (directory, config) = vault_home("vault-list");
        vault_run(&directory, &config, PUT, &mut FakeInput::piped(b"v")).unwrap();
        let listed = vault_run(
            &directory,
            &config,
            &["chief-of-staff", "vault", "list"],
            &mut FakeInput::piped(b""),
        )
        .unwrap();
        assert!(listed.contains("weather-key"));

        let deleted = vault_run(
            &directory,
            &config,
            &["chief-of-staff", "vault", "delete", "weather-key"],
            &mut FakeInput::piped(b""),
        )
        .unwrap();
        assert!(deleted.contains("\"action\": \"delete\""));
        assert!(open_store(&directory, &config).names().unwrap().is_empty());
    }

    #[test]
    fn vault_put_refuses_a_terminal_before_reading() {
        let (directory, config) = vault_home("vault-tty");
        let mut input = FakeInput {
            terminal: true,
            bytes: b"typed-secret".to_vec(),
            reads: 0,
        };
        let error = vault_run(&directory, &config, PUT, &mut input).unwrap_err();
        assert!(matches!(error, CliAppError::SecretInputIsTerminal));
        assert_eq!(input.reads, 0, "U-C2: nothing may be read from a terminal");
        assert!(describe_error(&error).contains("pipe it on stdin"));
    }

    #[test]
    fn vault_commands_refuse_without_a_configured_kek_and_never_read_stdin() {
        let directory = TestDirectory::new("vault-unconfigured");
        let config = directory.write_config(1);
        let mut input = FakeInput::piped(b"secret");
        let error = vault_run(&directory, &config, PUT, &mut input).unwrap_err();
        assert!(matches!(error, CliAppError::VaultNotConfigured));
        assert!(describe_error(&error).contains("[vault] kek_path"));
        assert_eq!(input.reads, 0);
    }

    #[test]
    fn vault_put_refuses_empty_and_oversized_secrets() {
        let (directory, config) = vault_home("vault-size");
        let empty = vault_run(&directory, &config, PUT, &mut FakeInput::piped(b"\n")).unwrap_err();
        assert!(matches!(empty, CliAppError::Vault(_)));
        assert!(describe_error(&empty).contains("payload is empty"));

        let huge = vec![b'x'; MAX_PAYLOAD_BYTES + 1];
        let error = vault_run(&directory, &config, PUT, &mut FakeInput::piped(&huge)).unwrap_err();
        assert!(matches!(error, CliAppError::SecretTooLarge));

        // Exactly at the bound, with the newline echo adds, is accepted.
        let mut max = vec![b'x'; MAX_PAYLOAD_BYTES];
        max.extend_from_slice(b"\r\n");
        vault_run(&directory, &config, PUT, &mut FakeInput::piped(&max)).unwrap();
        assert!(open_store(&directory, &config)
            .names()
            .unwrap()
            .contains(&SecretName::parse("weather-key").unwrap()));
    }

    #[test]
    fn vault_open_failures_surface_their_cause() {
        let (directory, config) = vault_home("vault-badkek");
        let kek = directory.0.join(".chief-of-staff").join("vault.kek");
        fs::write(&kek, [0x5a_u8; 31]).unwrap();
        let error = vault_run(&directory, &config, PUT, &mut FakeInput::piped(b"v")).unwrap_err();
        assert!(matches!(
            error,
            CliAppError::Daemon(ChiefDaemonError::ChiefVaultSecret(_))
        ));
        // The chain names the KEK file problem, not just "configuration".
        assert!(describe_error(&error).contains("vault KEK file failed"));
    }

    #[test]
    fn read_bounded_stops_at_the_limit_and_retries_interrupts() {
        struct Interrupting(usize);
        impl Read for Interrupting {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                self.0 += 1;
                if self.0 == 1 {
                    return Err(io::Error::from(io::ErrorKind::Interrupted));
                }
                if self.0 > 4 {
                    return Ok(0);
                }
                buf[0] = b'z';
                Ok(1)
            }
        }
        assert_eq!(&*read_bounded(&mut Interrupting(0), 10).unwrap(), b"zzz");
        assert_eq!(&*read_bounded(&mut &b"abcdef"[..], 4).unwrap(), b"abcd");
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::from(io::ErrorKind::BrokenPipe))
            }
        }
        assert!(read_bounded(&mut Broken, 4).is_err());
    }

    #[test]
    fn describe_error_does_not_repeat_a_cause_already_shown() {
        let error = CliAppError::Core(
            parse_argv(&["chief-of-staff".to_string(), "--bogus".to_string()]).unwrap_err(),
        );
        let text = describe_error(&error);
        let first = text.find("--bogus").expect("the cause is shown");
        assert!(!text[first + 1..].contains("--bogus"), "{text}");
    }

    #[test]
    fn vault_error_variants_have_stable_display() {
        for error in [
            CliAppError::VaultNotConfigured,
            CliAppError::SecretInputIsTerminal,
            CliAppError::SecretInputUnavailable,
            CliAppError::SecretTooLarge,
            CliAppError::ClockUnavailable,
        ] {
            assert!(error.to_string().starts_with("chief CLI:"));
            assert!(error.source().is_none());
        }
        assert!(unix_time_ms().unwrap() > 1_700_000_000_000);
    }
}
