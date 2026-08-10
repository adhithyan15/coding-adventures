//! Strict local CLI grammar, rendering, and product composition for vault-pm.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_storage_fs::FsStorageBackend;
use coding_adventures_vault_pm_application::{
    complete_generation_zero, prepare_generation_zero, rehydrate_prepared_init, ApplicationError,
    BootstrapLocator, GenerationZeroPolicyV1, GenerationZeroRandomness, LocalStateStore,
    LocalStateStoreError, LocalVaultStateV1, V1ApplicationRepositoryFactory, VaultAccessV1,
    VaultDoctorStateV1, VaultStatusStateV1, GENERATION_ZERO_RANDOM_BYTES,
};
use coding_adventures_vault_pm_application_storage_core::StorageCoreApplicationStore;
use coding_adventures_vault_pm_cli_host::{
    CliHostError, ControllingTerminal, OsEntropy, SecretPrompt,
};
use coding_adventures_vault_pm_config::{
    parse_config, render_config, ConfigName, CredentialRef, StorageConfigV1, StorageKind,
    StorageLocation, VaultConfigV1, VaultLocator as ConfigVaultLocator, VaultPmConfigV1,
    DEFAULT_AUTO_LOCK_SECONDS, DEFAULT_CLIPBOARD_CLEAR_SECONDS,
};
use coding_adventures_vault_pm_local_host::{LocalHostError, LocalVaultPaths, LocalWriterGuard};
use coding_adventures_vault_pm_storage_storage_core::StorageCoreObjectStore;
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Debug, Formatter};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_VAULT_NAME: &str = "personal";
const DEFAULT_STORAGE_NAME: &str = "local";
const PRODUCTION_KDF_MEMORY_KIB: u32 = 64 * 1024;
const PRODUCTION_KDF_ITERATIONS: u32 = 3;
const PRODUCTION_KDF_LANES: u8 = 1;
const USAGE: &str = "Usage:\n  vault-pm init [--vault NAME] [--storage NAME]\n  vault-pm status [--json]\n  vault-pm doctor\n";

/// Stable process exit classes defined by VLT-PM00.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    /// The command completed successfully.
    Success = 0,
    /// The command or caller input was invalid.
    InvalidInput = 2,
    /// Authentication or an unlocked session is required.
    Locked = 3,
    /// A requested item or object was not found.
    NotFound = 4,
    /// A concurrent update or recovery conflict requires resolution.
    Conflict = 5,
    /// Persisted integrity, permissions, or authentication failed closed.
    Integrity = 6,
    /// A local or remote storage provider was unavailable.
    Provider = 7,
    /// The selected backend or capability is unsupported.
    Unsupported = 8,
    /// An internal invariant failed.
    Internal = 10,
}

/// Complete payload-free result returned to the thin executable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliOutput {
    exit_code: ExitCode,
    stdout: String,
    stderr: String,
}

impl CliOutput {
    /// Return the stable process exit class.
    pub const fn exit_code(&self) -> ExitCode {
        self.exit_code
    }

    /// Borrow public standard output.
    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    /// Borrow fixed, payload-free standard error.
    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    fn success(stdout: impl Into<String>) -> Self {
        Self {
            exit_code: ExitCode::Success,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    fn failure(error: CliFailure) -> Self {
        Self {
            exit_code: error.exit_code(),
            stdout: String::new(),
            stderr: format!("{}\n", error.message()),
        }
    }
}

/// Injected platform authorities used by the testable CLI driver.
///
/// Production uses [`NativeCliHost`]. Tests can replace paths, time, entropy,
/// and fixed-prompt secret collection without adding secret-bearing CLI flags.
pub trait CliHost {
    /// Resolve the complete platform-local layout.
    fn paths(&self) -> Result<LocalVaultPaths, HostError>;

    /// Collect and confirm a new vault passphrase.
    fn read_new_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError>;

    /// Collect the existing passphrase for journal recovery.
    fn read_existing_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError>;

    /// Fill the entire generation-zero randomness block.
    fn fill_entropy(&self, output: &mut [u8]) -> Result<(), HostError>;

    /// Return the advisory current Unix time in milliseconds.
    fn now_ms(&self) -> Result<u64, HostError>;

    /// Return the bounded Argon2id policy for a new vault.
    fn generation_zero_kdf(&self) -> (u32, u32, u8);
}

/// Payload-free failure at an injected CLI host boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostError {
    /// The caller or platform returned an invalid value.
    Invalid,
    /// A required local authority was unavailable.
    Unavailable,
    /// The current platform or capability is unsupported.
    Unsupported,
}

/// Production platform authorities for the local executable.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeCliHost;

impl CliHost for NativeCliHost {
    fn paths(&self) -> Result<LocalVaultPaths, HostError> {
        LocalVaultPaths::resolve().map_err(map_native_local_host)
    }

    fn read_new_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
        ControllingTerminal
            .read_new_passphrase()
            .map_err(map_native_cli_host)
    }

    fn read_existing_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
        ControllingTerminal
            .read_secret(SecretPrompt::Unlock)
            .map_err(map_native_cli_host)
    }

    fn fill_entropy(&self, output: &mut [u8]) -> Result<(), HostError> {
        OsEntropy.fill(output).map_err(map_native_cli_host)
    }

    fn now_ms(&self) -> Result<u64, HostError> {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| HostError::Unavailable)?;
        u64::try_from(elapsed.as_millis()).map_err(|_| HostError::Unavailable)
    }

    fn generation_zero_kdf(&self) -> (u32, u32, u8) {
        (
            PRODUCTION_KDF_MEMORY_KIB,
            PRODUCTION_KDF_ITERATIONS,
            PRODUCTION_KDF_LANES,
        )
    }
}

/// Parse and execute one argument vector through an injected host.
///
/// `arguments` excludes the executable name. Non-Unicode arguments and every
/// unrecognized token fail with the same bounded invalid-command class.
pub fn run<I, S>(arguments: I, host: &dyn CliHost) -> CliOutput
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let command = match parse(arguments) {
        Ok(command) => command,
        Err(error) => return CliOutput::failure(error),
    };
    if matches!(command, Command::Help) {
        return CliOutput::success(USAGE);
    }
    match execute(command, host) {
        Ok(output) => output,
        Err(error) => CliOutput::failure(error),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Command {
    Init {
        vault: ConfigName,
        storage: ConfigName,
    },
    Status {
        json: bool,
    },
    Doctor,
    Help,
}

fn parse<I, S>(arguments: I) -> Result<Command, CliFailure>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let values = arguments
        .into_iter()
        .map(|value| {
            value
                .into()
                .into_string()
                .map_err(|_| CliFailure::InvalidCommand)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let Some(name) = values.first().map(String::as_str) else {
        return Err(CliFailure::InvalidCommand);
    };
    match name {
        "--help" | "-h" | "help" if values.len() == 1 => Ok(Command::Help),
        "init" => parse_init(&values[1..]),
        "status" => parse_status(&values[1..]),
        "doctor" if values.len() == 1 => Ok(Command::Doctor),
        _ => Err(CliFailure::InvalidCommand),
    }
}

fn parse_init(arguments: &[String]) -> Result<Command, CliFailure> {
    let mut vault = None;
    let mut storage = None;
    let mut index = 0;
    while index < arguments.len() {
        let destination = match arguments[index].as_str() {
            "--vault" if vault.is_none() => &mut vault,
            "--storage" if storage.is_none() => &mut storage,
            _ => return Err(CliFailure::InvalidCommand),
        };
        let value = arguments.get(index + 1).ok_or(CliFailure::InvalidCommand)?;
        *destination =
            Some(ConfigName::new(value.clone()).map_err(|_| CliFailure::InvalidCommand)?);
        index += 2;
    }
    Ok(Command::Init {
        vault: vault.unwrap_or(ConfigName::new(DEFAULT_VAULT_NAME).expect("fixed valid name")),
        storage: storage
            .unwrap_or(ConfigName::new(DEFAULT_STORAGE_NAME).expect("fixed valid name")),
    })
}

fn parse_status(arguments: &[String]) -> Result<Command, CliFailure> {
    match arguments {
        [] => Ok(Command::Status { json: false }),
        [argument] if argument == "--json" => Ok(Command::Status { json: true }),
        _ => Err(CliFailure::InvalidCommand),
    }
}

fn execute(command: Command, host: &dyn CliHost) -> Result<CliOutput, CliFailure> {
    let paths = host.paths().map_err(map_host)?;
    let prepared = paths.prepare().map_err(map_local_host)?;
    let writer = prepared.try_acquire_writer().map_err(map_local_host)?;
    match command {
        Command::Init { vault, storage } => init(host, prepared.paths(), &writer, vault, storage),
        Command::Status { json } => status(prepared.paths(), &writer, json),
        Command::Doctor => doctor(prepared.paths(), &writer),
        Command::Help => unreachable!("help returns before host access"),
    }
}

fn init(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    vault_name: ConfigName,
    storage_name: ConfigName,
) -> Result<CliOutput, CliFailure> {
    match writer.load_config().map_err(map_local_host)? {
        Some(exact_config) => resume_init(host, paths, &exact_config, vault_name, storage_name),
        None => begin_init(host, paths, writer, vault_name, storage_name),
    }
}

fn begin_init(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    vault_name: ConfigName,
    storage_name: ConfigName,
) -> Result<CliOutput, CliFailure> {
    let passphrase = host.read_new_passphrase().map_err(map_host)?;
    let mut random_bytes = [0_u8; GENERATION_ZERO_RANDOM_BYTES];
    host.fill_entropy(&mut random_bytes).map_err(map_host)?;
    let (memory_kib, iterations, lanes) = host.generation_zero_kdf();
    let policy = GenerationZeroPolicyV1::new(
        memory_kib,
        iterations,
        lanes,
        host.now_ms().map_err(map_host)?,
    )
    .map_err(map_application)?;
    let prepared = prepare_generation_zero(
        passphrase,
        policy,
        GenerationZeroRandomness::new(random_bytes),
    )
    .map_err(map_application)?;
    let locator = prepared.bootstrap_locator();
    let exact_prepared = prepared.owner_state().encode().map_err(map_application)?;
    let application_store = application_store(paths);

    // Install the retry journal before making its random locator discoverable.
    // A crash before config publication therefore leaves only unreachable
    // opaque data; a crash after publication always has exact recovery bytes.
    application_store
        .compare_exchange(locator, None, &exact_prepared)
        .map_err(map_local_state)?;
    let config = initial_config(paths, locator, vault_name, storage_name)?;
    writer
        .create_config(render_config(&config).as_bytes())
        .map_err(map_local_host)?;

    let repository_factory = repository_factory(paths);
    complete_generation_zero(
        prepared,
        &application_store,
        &application_store,
        &repository_factory,
    )
    .map_err(map_application)?;
    Ok(CliOutput::success("Vault initialized.\n"))
}

fn resume_init(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    exact_config: &[u8],
    vault_name: ConfigName,
    storage_name: ConfigName,
) -> Result<CliOutput, CliFailure> {
    let config = decode_config(exact_config)?;
    if config.default_vault() != &vault_name {
        return Err(CliFailure::AlreadyInitialized);
    }
    let vault = configured_vault(paths, &config)?;
    if vault.local_store() != &storage_name {
        return Err(CliFailure::AlreadyInitialized);
    }
    let locator = application_locator(vault.locator());
    let application_store = application_store(paths);
    let exact_state = application_store
        .load(locator)
        .map_err(map_local_state)?
        .ok_or(CliFailure::Integrity)?;
    let state = LocalVaultStateV1::decode(&exact_state).map_err(map_application)?;
    let LocalVaultStateV1::PreparedInit(_) = state else {
        return Err(match state {
            LocalVaultStateV1::Active(_) => CliFailure::AlreadyInitialized,
            LocalVaultStateV1::PendingPublication { .. } => CliFailure::Conflict,
            LocalVaultStateV1::PreparedInit(_) => unreachable!(),
        });
    };
    let passphrase = host.read_existing_passphrase().map_err(map_host)?;
    let prepared = rehydrate_prepared_init(passphrase, state).map_err(map_application)?;
    let repository_factory = repository_factory(paths);
    complete_generation_zero(
        prepared,
        &application_store,
        &application_store,
        &repository_factory,
    )
    .map_err(map_application)?;
    Ok(CliOutput::success("Vault initialized.\n"))
}

fn status(
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    json: bool,
) -> Result<CliOutput, CliFailure> {
    let Some(exact_config) = writer.load_config().map_err(map_local_host)? else {
        return Ok(render_status_label("uninitialized", json));
    };
    let config = decode_config(&exact_config)?;
    let vault = configured_vault(paths, &config)?;
    let locator = application_locator(vault.locator());
    let application_store = application_store(paths);
    let access = VaultAccessV1::locked(locator);
    let report = access.status(&application_store).map_err(map_application)?;
    let label = match report.state() {
        VaultStatusStateV1::Absent => "uninitialized",
        VaultStatusStateV1::Prepared => "initializing",
        VaultStatusStateV1::Locked => "locked",
        VaultStatusStateV1::Unlocked => "unlocked",
        VaultStatusStateV1::RecoveryRequired => "recovery_required",
    };
    Ok(render_status_label(label, json))
}

fn doctor(paths: &LocalVaultPaths, writer: &LocalWriterGuard) -> Result<CliOutput, CliFailure> {
    let Some(exact_config) = writer.load_config().map_err(map_local_host)? else {
        return Ok(doctor_output(
            "initialization_required",
            ExitCode::InvalidInput,
        ));
    };
    let config = decode_config(&exact_config)?;
    let vault = configured_vault(paths, &config)?;
    let locator = application_locator(vault.locator());
    let application_store = application_store(paths);
    let access = VaultAccessV1::locked(locator);
    let report = access.doctor(&application_store, &application_store);
    let (label, code) = match report.state() {
        VaultDoctorStateV1::Healthy => ("healthy", ExitCode::Success),
        VaultDoctorStateV1::InitializationRequired => {
            ("initialization_required", ExitCode::InvalidInput)
        }
        VaultDoctorStateV1::RecoveryRequired => ("recovery_required", ExitCode::Conflict),
        VaultDoctorStateV1::LocalStateUnavailable
        | VaultDoctorStateV1::BootstrapUnavailable
        | VaultDoctorStateV1::RepositoryUnavailable => ("unavailable", ExitCode::Provider),
        VaultDoctorStateV1::UnsupportedCapability => ("unsupported", ExitCode::Unsupported),
        VaultDoctorStateV1::AuthenticationRequired => ("authentication_required", ExitCode::Locked),
        VaultDoctorStateV1::IntegrityFailure => ("integrity_failure", ExitCode::Integrity),
    };
    Ok(doctor_output(label, code))
}

fn render_status_label(label: &str, json: bool) -> CliOutput {
    if json {
        CliOutput::success(format!("{{\"state\":\"{label}\"}}\n"))
    } else {
        CliOutput::success(format!("Status: {label}\n"))
    }
}

fn doctor_output(label: &str, exit_code: ExitCode) -> CliOutput {
    CliOutput {
        exit_code,
        stdout: format!("Doctor: {label}\n"),
        stderr: String::new(),
    }
}

fn initial_config(
    paths: &LocalVaultPaths,
    locator: BootstrapLocator,
    vault_name: ConfigName,
    storage_name: ConfigName,
) -> Result<VaultPmConfigV1, CliFailure> {
    let location = paths
        .object_root()
        .to_str()
        .ok_or(CliFailure::Unsupported)?;
    let storage_config = StorageConfigV1::new(
        StorageKind::Filesystem,
        StorageLocation::new(location).map_err(|_| CliFailure::Unsupported)?,
        CredentialRef::none(),
    );
    let vault_config = VaultConfigV1::new(
        ConfigVaultLocator::new(*locator.as_bytes()),
        storage_name.clone(),
        Vec::new(),
        DEFAULT_AUTO_LOCK_SECONDS,
        DEFAULT_CLIPBOARD_CLEAR_SECONDS,
    )
    .map_err(|_| CliFailure::Internal)?;
    let mut vaults = BTreeMap::new();
    vaults.insert(vault_name.clone(), vault_config);
    let mut storage = BTreeMap::new();
    storage.insert(storage_name, storage_config);
    VaultPmConfigV1::new(vault_name, vaults, storage).map_err(|_| CliFailure::Internal)
}

fn decode_config(exact: &[u8]) -> Result<VaultPmConfigV1, CliFailure> {
    let text = core::str::from_utf8(exact).map_err(|_| CliFailure::Integrity)?;
    parse_config(text).map_err(|_| CliFailure::Integrity)
}

fn configured_vault<'a>(
    paths: &LocalVaultPaths,
    config: &'a VaultPmConfigV1,
) -> Result<&'a VaultConfigV1, CliFailure> {
    let vault = config.select_vault(None).ok_or(CliFailure::Integrity)?;
    if !vault.remote_stores().is_empty() {
        return Err(CliFailure::Unsupported);
    }
    let storage = config
        .storage()
        .get(vault.local_store())
        .ok_or(CliFailure::Integrity)?;
    let expected = paths
        .object_root()
        .to_str()
        .ok_or(CliFailure::Unsupported)?;
    if storage.kind() != StorageKind::Filesystem
        || storage.location().as_str() != expected
        || storage.credential_ref().as_str() != "none"
    {
        return Err(CliFailure::Unsupported);
    }
    Ok(vault)
}

fn application_locator(locator: ConfigVaultLocator) -> BootstrapLocator {
    BootstrapLocator::new(*locator.as_bytes())
}

fn application_store(paths: &LocalVaultPaths) -> StorageCoreApplicationStore<FsStorageBackend> {
    StorageCoreApplicationStore::new(FsStorageBackend::new(paths.application_state_root()))
}

fn repository_factory(
    paths: &LocalVaultPaths,
) -> V1ApplicationRepositoryFactory<StorageCoreObjectStore<FsStorageBackend>> {
    V1ApplicationRepositoryFactory::new(StorageCoreObjectStore::new(FsStorageBackend::new(
        paths.object_root(),
    )))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CliFailure {
    InvalidCommand,
    AlreadyInitialized,
    Locked,
    NotFound,
    Conflict,
    Integrity,
    Provider,
    Unsupported,
    Internal,
}

impl CliFailure {
    const fn exit_code(self) -> ExitCode {
        match self {
            Self::InvalidCommand | Self::AlreadyInitialized => ExitCode::InvalidInput,
            Self::Locked => ExitCode::Locked,
            Self::NotFound => ExitCode::NotFound,
            Self::Conflict => ExitCode::Conflict,
            Self::Integrity => ExitCode::Integrity,
            Self::Provider => ExitCode::Provider,
            Self::Unsupported => ExitCode::Unsupported,
            Self::Internal => ExitCode::Internal,
        }
    }

    const fn message(self) -> &'static str {
        match self {
            Self::InvalidCommand => "vault-pm: invalid command",
            Self::AlreadyInitialized => "vault-pm: already initialized",
            Self::Locked => "vault-pm: authentication required",
            Self::NotFound => "vault-pm: not found",
            Self::Conflict => "vault-pm: recovery or conflict required",
            Self::Integrity => "vault-pm: integrity check failed",
            Self::Provider => "vault-pm: storage unavailable",
            Self::Unsupported => "vault-pm: unsupported capability",
            Self::Internal => "vault-pm: internal invariant failed",
        }
    }
}

impl Debug for CliFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

fn map_application(error: ApplicationError) -> CliFailure {
    match error {
        ApplicationError::NotInitialized
        | ApplicationError::AlreadyInitialized
        | ApplicationError::InvalidInput
        | ApplicationError::BoundExceeded => CliFailure::InvalidCommand,
        ApplicationError::Locked | ApplicationError::AuthenticationFailed => CliFailure::Locked,
        ApplicationError::NotFound => CliFailure::NotFound,
        ApplicationError::ConcurrentHost | ApplicationError::ConflictRequired => {
            CliFailure::Conflict
        }
        ApplicationError::IntegrityFailure => CliFailure::Integrity,
        ApplicationError::StorageUnavailable => CliFailure::Provider,
        ApplicationError::Unsupported => CliFailure::Unsupported,
        ApplicationError::InternalInvariant => CliFailure::Internal,
    }
}

fn map_local_state(error: LocalStateStoreError) -> CliFailure {
    match error {
        LocalStateStoreError::Unavailable => CliFailure::Provider,
        LocalStateStoreError::ConcurrentHost => CliFailure::Conflict,
        LocalStateStoreError::Corruption => CliFailure::Integrity,
    }
}

fn map_local_host(error: LocalHostError) -> CliFailure {
    match error {
        LocalHostError::AlreadyLocked
        | LocalHostError::ConfigAlreadyExists
        | LocalHostError::ConfigConflict => CliFailure::Conflict,
        LocalHostError::InsecureOwner
        | LocalHostError::InsecurePermissions
        | LocalHostError::UnsafeObjectType => CliFailure::Integrity,
        LocalHostError::UnsupportedPlatform => CliFailure::Unsupported,
        LocalHostError::PlatformUnavailable
        | LocalHostError::ParentUnavailable
        | LocalHostError::AccessFailed => CliFailure::Provider,
        LocalHostError::InvalidPath | LocalHostError::InvalidConfigBytes => {
            CliFailure::InvalidCommand
        }
    }
}

fn map_host(error: HostError) -> CliFailure {
    match error {
        HostError::Invalid => CliFailure::InvalidCommand,
        HostError::Unavailable => CliFailure::Provider,
        HostError::Unsupported => CliFailure::Unsupported,
    }
}

fn map_native_cli_host(error: CliHostError) -> HostError {
    match error {
        CliHostError::EmptySecret
        | CliHostError::SecretTooLong
        | CliHostError::SecretMismatch
        | CliHostError::InvalidEntropyRequest => HostError::Invalid,
        CliHostError::UnsupportedPlatform => HostError::Unsupported,
        CliHostError::TerminalUnavailable
        | CliHostError::TerminalAccessFailed
        | CliHostError::TerminalModeFailed
        | CliHostError::SecretInputFailed
        | CliHostError::EntropyUnavailable => HostError::Unavailable,
    }
}

fn map_native_local_host(error: LocalHostError) -> HostError {
    match error {
        LocalHostError::UnsupportedPlatform => HostError::Unsupported,
        LocalHostError::InvalidPath | LocalHostError::InvalidConfigBytes => HostError::Invalid,
        _ => HostError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let suffix = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("vault-pm-cli-{}-{suffix}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }

        fn paths(&self) -> LocalVaultPaths {
            LocalVaultPaths::from_roots(
                self.0.join("config"),
                self.0.join("data"),
                self.0.join("cache"),
            )
            .unwrap()
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct TestHost {
        paths: LocalVaultPaths,
        secrets: Mutex<VecDeque<Vec<u8>>>,
    }

    impl TestHost {
        fn new(paths: LocalVaultPaths, secrets: impl IntoIterator<Item = Vec<u8>>) -> Self {
            Self {
                paths,
                secrets: Mutex::new(secrets.into_iter().collect()),
            }
        }

        fn secret(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
            self.secrets
                .lock()
                .unwrap()
                .pop_front()
                .map(Zeroizing::new)
                .ok_or(HostError::Unavailable)
        }
    }

    impl CliHost for TestHost {
        fn paths(&self) -> Result<LocalVaultPaths, HostError> {
            Ok(self.paths.clone())
        }

        fn read_new_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
            self.secret()
        }

        fn read_existing_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
            self.secret()
        }

        fn fill_entropy(&self, output: &mut [u8]) -> Result<(), HostError> {
            for (index, byte) in output.iter_mut().enumerate() {
                *byte = u8::try_from(index % 251).unwrap().wrapping_add(1);
            }
            Ok(())
        }

        fn now_ms(&self) -> Result<u64, HostError> {
            Ok(1_700_000_000_000)
        }

        fn generation_zero_kdf(&self) -> (u32, u32, u8) {
            (8 * 1024, 1, 1)
        }
    }

    #[test]
    fn parser_is_closed_and_never_accepts_secret_arguments() {
        let root = TestRoot::new();
        let host = TestHost::new(root.paths(), []);
        for arguments in [
            vec!["init", "--passphrase", "secret"],
            vec!["init", "--vault=personal"],
            vec!["status", "--unsafe-include-secrets"],
            vec!["doctor", "extra"],
            vec!["unlock"],
        ] {
            let output = run(arguments, &host);
            assert_eq!(output.exit_code(), ExitCode::InvalidInput);
            assert_eq!(output.stderr(), "vault-pm: invalid command\n");
        }
        assert!(!root.0.join("config").exists());
    }

    #[test]
    fn init_survives_restart_and_locked_queries_are_redacted() {
        let root = TestRoot::new();
        let paths = root.paths();
        let init_host = TestHost::new(paths.clone(), [b"correct horse battery staple".to_vec()]);
        let initialized = run(["init"], &init_host);
        assert_eq!(
            initialized.exit_code(),
            ExitCode::Success,
            "{initialized:?}"
        );
        assert_eq!(initialized.stdout(), "Vault initialized.\n");
        assert!(initialized.stderr().is_empty());

        let restarted = TestHost::new(paths, []);
        let status = run(["status", "--json"], &restarted);
        assert_eq!(status.exit_code(), ExitCode::Success);
        assert_eq!(status.stdout(), "{\"state\":\"locked\"}\n");
        assert!(!status.stdout().contains("personal"));

        let doctor = run(["doctor"], &restarted);
        assert_eq!(doctor.exit_code(), ExitCode::Locked);
        assert_eq!(doctor.stdout(), "Doctor: authentication_required\n");
        assert!(doctor.stderr().is_empty());
    }

    #[test]
    fn repeated_init_does_not_prompt_or_replace_active_state() {
        let root = TestRoot::new();
        let paths = root.paths();
        let first = TestHost::new(paths.clone(), [b"first passphrase".to_vec()]);
        let initialized = run(["init"], &first);
        assert_eq!(
            initialized.exit_code(),
            ExitCode::Success,
            "{initialized:?}"
        );

        let second = TestHost::new(paths, []);
        let output = run(["init"], &second);
        assert_eq!(output.exit_code(), ExitCode::InvalidInput);
        assert_eq!(output.stderr(), "vault-pm: already initialized\n");
    }

    #[test]
    fn prepared_init_is_rehydrated_without_new_randomness() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"resumable passphrase".to_vec();
        let mut random_bytes = [0_u8; GENERATION_ZERO_RANDOM_BYTES];
        for (index, byte) in random_bytes.iter_mut().enumerate() {
            *byte = u8::try_from(index % 251).unwrap().wrapping_add(1);
        }
        let prepared = prepare_generation_zero(
            Zeroizing::new(passphrase.clone()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 1_700_000_000_000).unwrap(),
            GenerationZeroRandomness::new(random_bytes),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let exact_prepared = prepared.owner_state().encode().unwrap();
        let layout = paths.prepare().unwrap();
        let writer = layout.try_acquire_writer().unwrap();
        let store = application_store(&paths);
        store
            .compare_exchange(locator, None, &exact_prepared)
            .unwrap();
        let config = initial_config(
            &paths,
            locator,
            ConfigName::new(DEFAULT_VAULT_NAME).unwrap(),
            ConfigName::new(DEFAULT_STORAGE_NAME).unwrap(),
        )
        .unwrap();
        writer
            .create_config(render_config(&config).as_bytes())
            .unwrap();
        drop(writer);
        drop(layout);
        drop(prepared);

        let host = TestHost::new(paths.clone(), [passphrase]);
        let resumed = run(["init"], &host);
        assert_eq!(resumed.exit_code(), ExitCode::Success, "{resumed:?}");
        assert_eq!(run(["status"], &host).stdout(), "Status: locked\n");
    }

    #[test]
    fn unconfigured_status_and_doctor_are_stable() {
        let root = TestRoot::new();
        let host = TestHost::new(root.paths(), []);
        let status = run(["status"], &host);
        assert_eq!(status.stdout(), "Status: uninitialized\n", "{status:?}");
        let doctor = run(["doctor"], &host);
        assert_eq!(doctor.exit_code(), ExitCode::InvalidInput);
        assert_eq!(doctor.stdout(), "Doctor: initialization_required\n");
    }

    #[test]
    fn help_has_no_host_side_effects() {
        let root = TestRoot::new();
        let host = TestHost::new(root.paths(), []);
        let output = run(["--help"], &host);
        assert_eq!(output.exit_code(), ExitCode::Success);
        assert_eq!(output.stdout(), USAGE);
        assert!(!root.0.join("config").exists());
    }
}
