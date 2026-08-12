//! Strict local CLI grammar, rendering, and product composition for vault-pm.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_storage_fs::FsStorageBackend;
use coding_adventures_vault_pm_application::{
    complete_generation_zero, open_portable_with_passphrase, portable_import_random_bytes,
    prepare_generation_zero, rehydrate_prepared_init, AddItemRandomnessV1, ApplicationError,
    AuditEventViewV1, AuditVerificationV1, AuditedAccessRandomnessV1, BootstrapLocator,
    BootstrapStore, BootstrapStoreError, DeleteItemRandomnessV1, GenerationZeroPolicyV1,
    GenerationZeroRandomness, ItemHistoryViewV1, LocalStateStore, LocalStateStoreError,
    LocalVaultStateV1, LoginEditInputV1, PortableExportPolicyV1, PortableExportRandomnessV1,
    PortableImportRandomnessV1, PortableOpenPolicyV1, ReplaceItemRandomnessV1,
    RestoreItemRandomnessV1, V1ApplicationRepositoryFactory, VaultAccessV1, VaultDoctorStateV1,
    VaultStatusStateV1, ADD_ITEM_RANDOM_BYTES, AUDITED_ACCESS_RANDOM_BYTES,
    DEFAULT_AUDIT_HISTORY_LIMIT, DEFAULT_ITEM_HISTORY_LIMIT, DELETE_ITEM_RANDOM_BYTES,
    GENERATION_ZERO_RANDOM_BYTES, MAX_PORTABLE_EXPORT_ARTIFACT_BYTES, PORTABLE_EXPORT_RANDOM_BYTES,
    REPLACE_ITEM_RANDOM_BYTES, RESTORE_ITEM_RANDOM_BYTES,
};
use coding_adventures_vault_pm_application_storage_core::StorageCoreApplicationStore;
use coding_adventures_vault_pm_cli_host::{
    read_portable_export, write_portable_export, CliHostError, ControllingTerminal, OsEntropy,
    SecretPrompt, TextPrompt,
};
use coding_adventures_vault_pm_config::{
    parse_config, render_config, ConfigName, CredentialRef, StorageConfigV1, StorageKind,
    StorageLocation, VaultConfigV1, VaultLocator as ConfigVaultLocator, VaultPmConfigV1,
    DEFAULT_AUTO_LOCK_SECONDS, DEFAULT_CLIPBOARD_CLEAR_SECONDS,
};
use coding_adventures_vault_pm_domain::{
    ContentType, ItemDocument, ItemId, LwwRegister, ObservedSet, OperationId, RedactedItemView,
    RedactedRecordView, RevisionId,
};
use coding_adventures_vault_pm_local_host::{LocalHostError, LocalVaultPaths, LocalWriterGuard};
use coding_adventures_vault_pm_storage_storage_core::StorageCoreObjectStore;
use coding_adventures_vault_records::{AnyRecord, Login, SecureNote, LOGIN_V1, SECURE_NOTE_V1};
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Debug, Formatter};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_VAULT_NAME: &str = "personal";
const DEFAULT_STORAGE_NAME: &str = "local";
const PRODUCTION_KDF_MEMORY_KIB: u32 = 64 * 1024;
const PRODUCTION_KDF_ITERATIONS: u32 = 3;
const PRODUCTION_KDF_LANES: u8 = 1;
const ITEM_OPERATION_RANDOM_BYTES: usize = 32;
const USAGE: &str = "Usage:\n  vault-pm init [--vault NAME] [--storage NAME]\n  vault-pm status [--json]\n  vault-pm audit enable\n  vault-pm audit verify\n  vault-pm audit list\n  vault-pm audit show TRACE\n  vault-pm doctor [--unlock]\n  vault-pm export FILE\n  vault-pm import FILE\n  vault-pm item add login\n  vault-pm item add secure-note\n  vault-pm item edit ITEM\n  vault-pm item delete ITEM\n  vault-pm item list\n  vault-pm item show ITEM\n  vault-pm history list ITEM\n  vault-pm history restore ITEM REVISION\n";

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

    /// Collect the existing passphrase for recovery or one-shot unlock.
    fn read_existing_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError>;

    /// Collect a login title from the controlling terminal.
    fn read_login_title(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect a login username from the controlling terminal.
    fn read_login_username(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect an optional primary login URL from the controlling terminal.
    fn read_login_url(&self) -> Result<Option<Zeroizing<String>>, HostError>;

    /// Collect a login password with terminal echo disabled.
    fn read_login_password(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect a secure-note title from the controlling terminal.
    fn read_secure_note_title(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect a secure-note body with terminal echo disabled.
    fn read_secure_note_body(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect and confirm a distinct portable-export passphrase without echo.
    fn read_export_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError>;

    /// Durably create one explicit encrypted portable-export destination.
    fn write_portable_export(&self, destination: &Path, artifact: &[u8]) -> Result<(), HostError>;

    /// Read one explicit encrypted portable artifact under the V1 size ceiling.
    fn read_portable_export(&self, source: &Path) -> Result<Vec<u8>, HostError>;

    /// Collect the passphrase for an existing portable artifact without echo.
    fn read_import_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError>;

    /// Fill the entire generation-zero randomness block.
    fn fill_entropy(&self, output: &mut [u8]) -> Result<(), HostError>;

    /// Return the advisory current Unix time in milliseconds.
    fn now_ms(&self) -> Result<u64, HostError>;

    /// Return the bounded Argon2id policy for a new vault.
    fn generation_zero_kdf(&self) -> (u32, u32, u8);

    /// Return the bounded Argon2id policy for a portable export.
    fn portable_export_kdf(&self) -> (u32, u32, u8) {
        self.generation_zero_kdf()
    }

    /// Return the maximum accepted Argon2id policy for opening an artifact.
    fn portable_open_kdf(&self) -> (u32, u32, u8) {
        self.portable_export_kdf()
    }
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

    fn read_login_title(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::LoginTitle)
            .map_err(map_native_cli_host)
    }

    fn read_login_username(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::LoginUsername)
            .map_err(map_native_cli_host)
    }

    fn read_login_url(&self) -> Result<Option<Zeroizing<String>>, HostError> {
        let value = ControllingTerminal
            .read_text(TextPrompt::LoginUrl)
            .map_err(map_native_cli_host)?;
        Ok((!value.is_empty()).then_some(value))
    }

    fn read_login_password(&self) -> Result<Zeroizing<String>, HostError> {
        self.read_utf8_secret(SecretPrompt::LoginPassword)
    }

    fn read_secure_note_title(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::SecureNoteTitle)
            .map_err(map_native_cli_host)
    }

    fn read_secure_note_body(&self) -> Result<Zeroizing<String>, HostError> {
        self.read_utf8_secret(SecretPrompt::SecureNoteBody)
    }

    fn read_export_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
        ControllingTerminal
            .read_export_passphrase()
            .map_err(map_native_cli_host)
    }

    fn write_portable_export(&self, destination: &Path, artifact: &[u8]) -> Result<(), HostError> {
        write_portable_export(destination, artifact).map_err(map_native_cli_host)
    }

    fn read_portable_export(&self, source: &Path) -> Result<Vec<u8>, HostError> {
        read_portable_export(source, MAX_PORTABLE_EXPORT_ARTIFACT_BYTES)
            .map_err(map_native_cli_host)
    }

    fn read_import_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
        ControllingTerminal
            .read_secret(SecretPrompt::ImportPassphrase)
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

impl NativeCliHost {
    fn read_utf8_secret(&self, prompt: SecretPrompt) -> Result<Zeroizing<String>, HostError> {
        let bytes = ControllingTerminal
            .read_secret(prompt)
            .map_err(map_native_cli_host)?;
        core::str::from_utf8(&bytes).map_err(|_| HostError::Invalid)?;
        Ok(Zeroizing::new(
            String::from_utf8(bytes.into_inner()).expect("UTF-8 was validated before ownership"),
        ))
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
    AuditEnable,
    AuditVerify,
    AuditList,
    AuditShow {
        trace_id: OperationId,
    },
    Doctor {
        unlock: bool,
    },
    PortableExport {
        destination: PathBuf,
    },
    PortableImport {
        source: PathBuf,
    },
    ItemAddLogin,
    ItemAddSecureNote,
    ItemEdit {
        item_id: ItemId,
    },
    ItemDelete {
        item_id: ItemId,
    },
    ItemList,
    ItemShow {
        item_id: ItemId,
    },
    HistoryList {
        item_id: ItemId,
    },
    HistoryRestore {
        item_id: ItemId,
        revision_id: RevisionId,
    },
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
        "audit" => parse_audit(&values[1..]),
        "doctor" => parse_doctor(&values[1..]),
        "export" => parse_export(&values[1..]),
        "import" => parse_import(&values[1..]),
        "item" => parse_item(&values[1..]),
        "history" => parse_history(&values[1..]),
        _ => Err(CliFailure::InvalidCommand),
    }
}

fn parse_import(arguments: &[String]) -> Result<Command, CliFailure> {
    match arguments {
        [source] if !source.is_empty() => Ok(Command::PortableImport {
            source: PathBuf::from(source),
        }),
        _ => Err(CliFailure::InvalidCommand),
    }
}

fn parse_export(arguments: &[String]) -> Result<Command, CliFailure> {
    match arguments {
        [destination] if !destination.is_empty() => Ok(Command::PortableExport {
            destination: PathBuf::from(destination),
        }),
        _ => Err(CliFailure::InvalidCommand),
    }
}

fn parse_audit(arguments: &[String]) -> Result<Command, CliFailure> {
    match arguments {
        [action] if action == "enable" => Ok(Command::AuditEnable),
        [action] if action == "verify" => Ok(Command::AuditVerify),
        [action] if action == "list" => Ok(Command::AuditList),
        [action, trace] if action == "show" => Ok(Command::AuditShow {
            trace_id: OperationId::from_user_string(trace)
                .map_err(|_| CliFailure::InvalidCommand)?,
        }),
        _ => Err(CliFailure::InvalidCommand),
    }
}

fn parse_history(arguments: &[String]) -> Result<Command, CliFailure> {
    match arguments {
        [action, item] if action == "list" => Ok(Command::HistoryList {
            item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
        }),
        [action, item, revision] if action == "restore" => Ok(Command::HistoryRestore {
            item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
            revision_id: RevisionId::from_user_string(revision)
                .map_err(|_| CliFailure::InvalidCommand)?,
        }),
        _ => Err(CliFailure::InvalidCommand),
    }
}

fn parse_item(arguments: &[String]) -> Result<Command, CliFailure> {
    match arguments {
        [action, kind] if action == "add" && kind == "login" => Ok(Command::ItemAddLogin),
        [action, kind] if action == "add" && kind == "secure-note" => {
            Ok(Command::ItemAddSecureNote)
        }
        [action, item] if action == "edit" => Ok(Command::ItemEdit {
            item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
        }),
        [action, item] if action == "delete" => Ok(Command::ItemDelete {
            item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
        }),
        [action] if action == "list" => Ok(Command::ItemList),
        [action, item] if action == "show" => Ok(Command::ItemShow {
            item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
        }),
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

fn parse_doctor(arguments: &[String]) -> Result<Command, CliFailure> {
    match arguments {
        [] => Ok(Command::Doctor { unlock: false }),
        [argument] if argument == "--unlock" => Ok(Command::Doctor { unlock: true }),
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
        Command::AuditEnable => audit_enable(host, prepared.paths(), &writer),
        Command::AuditVerify => audit_verify(host, prepared.paths(), &writer),
        Command::AuditList => audit_list(host, prepared.paths(), &writer),
        Command::AuditShow { trace_id } => audit_show(host, prepared.paths(), &writer, trace_id),
        Command::Doctor { unlock } => doctor(host, prepared.paths(), &writer, unlock),
        Command::PortableExport { destination } => {
            portable_export(host, prepared.paths(), &writer, &destination)
        }
        Command::PortableImport { source } => {
            portable_import(host, prepared.paths(), &writer, &source)
        }
        Command::ItemAddLogin => item_add_login(host, prepared.paths(), &writer),
        Command::ItemAddSecureNote => item_add_secure_note(host, prepared.paths(), &writer),
        Command::ItemEdit { item_id } => item_edit_login(host, prepared.paths(), &writer, item_id),
        Command::ItemDelete { item_id } => item_delete(host, prepared.paths(), &writer, item_id),
        Command::ItemList => item_list(host, prepared.paths(), &writer),
        Command::ItemShow { item_id } => item_show(host, prepared.paths(), &writer, item_id),
        Command::HistoryList { item_id } => history_list(host, prepared.paths(), &writer, item_id),
        Command::HistoryRestore {
            item_id,
            revision_id,
        } => history_restore(host, prepared.paths(), &writer, item_id, revision_id),
        Command::Help => unreachable!("help returns before host access"),
    }
}

fn authenticated_access(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
) -> Result<(VaultAccessV1, StorageCoreApplicationStore<FsStorageBackend>), CliFailure> {
    let exact_config = writer
        .load_config()
        .map_err(map_local_host)?
        .ok_or(CliFailure::InvalidCommand)?;
    let config = decode_config(&exact_config)?;
    let vault = configured_vault(paths, &config)?;
    let locator = application_locator(vault.locator());
    let application_store = application_store(paths);
    let repository_factory = repository_factory(paths);
    let mut access = VaultAccessV1::locked(locator);
    let passphrase = host.read_existing_passphrase().map_err(map_host)?;
    access
        .unlock(
            passphrase,
            &application_store,
            &application_store,
            &repository_factory,
        )
        .map_err(map_application)?;
    Ok((access, application_store))
}

fn audited_access_inputs(
    host: &dyn CliHost,
) -> Result<(u64, AuditedAccessRandomnessV1), CliFailure> {
    let wall_time_ms = host.now_ms().map_err(map_host)?;
    let mut random = [0_u8; AUDITED_ACCESS_RANDOM_BYTES];
    host.fill_entropy(&mut random).map_err(map_host)?;
    Ok((wall_time_ms, AuditedAccessRandomnessV1::new(random)))
}

fn portable_export(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    destination: &Path,
) -> Result<CliOutput, CliFailure> {
    let exact_config = writer
        .load_config()
        .map_err(map_local_host)?
        .ok_or(CliFailure::InvalidCommand)?;
    let config = decode_config(&exact_config)?;
    let vault = configured_vault(paths, &config)?;
    let locator = application_locator(vault.locator());
    let application_store = application_store(paths);
    let exact_bootstrap = application_store
        .load_latest(locator)
        .map_err(map_bootstrap_store)?
        .ok_or(CliFailure::Integrity)?;
    let (wall_time_ms, audit_randomness) = audited_access_inputs(host)?;
    let mut export_random = [0_u8; PORTABLE_EXPORT_RANDOM_BYTES];
    host.fill_entropy(&mut export_random).map_err(map_host)?;
    let (memory_kib, iterations, lanes) = host.portable_export_kdf();
    let policy =
        PortableExportPolicyV1::new(memory_kib, iterations, lanes).map_err(map_application)?;

    let repository_factory = repository_factory(paths);
    let mut access = VaultAccessV1::locked(locator);
    let vault_passphrase = host.read_existing_passphrase().map_err(map_host)?;
    access
        .unlock(
            vault_passphrase,
            &application_store,
            &application_store,
            &repository_factory,
        )
        .map_err(map_application)?;
    let audit_enabled = access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled();
    let export_passphrase = match host.read_export_passphrase() {
        Ok(passphrase) => passphrase,
        Err(error) => {
            if audit_enabled {
                access
                    .into_unlocked()
                    .map_err(map_application)?
                    .record_audited_portable_export_host_failure(
                        wall_time_ms,
                        audit_randomness,
                        &application_store,
                    )
                    .map_err(map_application)?;
            }
            return Err(map_host(error));
        }
    };
    let export_randomness = PortableExportRandomnessV1::new(export_random);
    let artifact = if audit_enabled {
        access
            .into_unlocked()
            .map_err(map_application)?
            .audited_export_portable_with_passphrase(
                &exact_bootstrap,
                export_passphrase,
                policy,
                export_randomness,
                wall_time_ms,
                audit_randomness,
                &application_store,
            )
            .map_err(map_application)?
            .into_operation()
            .map_err(map_application)?
    } else {
        let operation = access
            .as_unlocked()
            .map_err(map_application)?
            .export_portable_with_passphrase(
                &exact_bootstrap,
                export_passphrase,
                policy,
                export_randomness,
            );
        access.lock();
        operation.map_err(map_application)?
    };
    host.write_portable_export(destination, artifact.as_bytes())
        .map_err(map_host)?;
    Ok(CliOutput::success("Portable export written.\n"))
}

struct PortableImportContext {
    access: VaultAccessV1,
    application_store: StorageCoreApplicationStore<FsStorageBackend>,
    wall_time_ms: u64,
    failure_randomness: AuditedAccessRandomnessV1,
}

impl PortableImportContext {
    fn fail(self, error: CliFailure) -> Result<CliOutput, CliFailure> {
        self.access
            .into_unlocked()
            .map_err(map_application)?
            .record_audited_portable_import_host_failure(
                self.wall_time_ms,
                self.failure_randomness,
                &self.application_store,
            )
            .map_err(map_application)?;
        Err(error)
    }

    fn complete(
        self,
        snapshot: coding_adventures_vault_pm_application::OpenedPortableSnapshotV1,
        randomness: PortableImportRandomnessV1,
        item_count: usize,
        candidate_count: usize,
    ) -> Result<CliOutput, CliFailure> {
        self.access
            .into_unlocked()
            .map_err(map_application)?
            .audited_import_opened_portable_snapshot(
                snapshot,
                self.wall_time_ms,
                randomness,
                self.failure_randomness,
                &self.application_store,
            )
            .map_err(map_application)?;
        Ok(CliOutput::success(format!(
            "Portable import complete: items={item_count} candidates={candidate_count}.\n"
        )))
    }
}

fn portable_import(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    source: &Path,
) -> Result<CliOutput, CliFailure> {
    let (memory_kib, iterations, lanes) = host.portable_open_kdf();
    let open_policy =
        PortableOpenPolicyV1::new(memory_kib, iterations, lanes).map_err(map_application)?;
    let (wall_time_ms, failure_randomness) = audited_access_inputs(host)?;
    let (mut access, application_store) = authenticated_access(host, paths, writer)?;
    if !access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled()
    {
        access.lock();
        return Err(CliFailure::InvalidCommand);
    }
    let context = PortableImportContext {
        access,
        application_store,
        wall_time_ms,
        failure_randomness,
    };
    let artifact = match host.read_portable_export(source) {
        Ok(artifact) => artifact,
        Err(error) => return context.fail(map_host(error)),
    };
    let passphrase = match host.read_import_passphrase() {
        Ok(passphrase) => passphrase,
        Err(error) => return context.fail(map_host(error)),
    };
    let snapshot = match open_portable_with_passphrase(&artifact, passphrase, open_policy) {
        Ok(snapshot) => snapshot,
        Err(error) => return context.fail(map_application(error)),
    };
    let item_count = snapshot.item_count();
    let candidate_count = snapshot.candidate_count();
    let random_bytes = match portable_import_random_bytes(&snapshot) {
        Ok(count) => count,
        Err(error) => return context.fail(map_application(error)),
    };
    let mut random = vec![0_u8; random_bytes];
    if let Err(error) = host.fill_entropy(&mut random) {
        return context.fail(map_host(error));
    }
    let randomness = match PortableImportRandomnessV1::new(random, &snapshot) {
        Ok(randomness) => randomness,
        Err(error) => return context.fail(map_application(error)),
    };
    context.complete(snapshot, randomness, item_count, candidate_count)
}

struct ItemCreateContext {
    access: VaultAccessV1,
    application_store: StorageCoreApplicationStore<FsStorageBackend>,
    now_ms: u64,
    randomness: AddItemRandomnessV1,
    item_id: ItemId,
    favorite_operation: OperationId,
    failure_randomness: AuditedAccessRandomnessV1,
    audit_enabled: bool,
}

impl ItemCreateContext {
    fn document(
        &self,
        content_type: &'static str,
        record: AnyRecord,
    ) -> Result<ItemDocument, CliFailure> {
        ItemDocument::new(
            self.item_id,
            ContentType::new(content_type).map_err(|_| CliFailure::Internal)?,
            self.now_ms,
            self.now_ms,
            LwwRegister::new(false, self.now_ms, self.favorite_operation),
            ObservedSet::new(),
            ObservedSet::new(),
            record,
            ObservedSet::new(),
        )
        .map_err(|_| CliFailure::InvalidCommand)
    }

    fn fail(self, error: CliFailure) -> Result<CliOutput, CliFailure> {
        if self.audit_enabled {
            self.access
                .into_unlocked()
                .map_err(map_application)?
                .record_audited_item_create_host_failure(
                    self.randomness,
                    self.now_ms,
                    self.failure_randomness,
                    &self.application_store,
                )
                .map_err(map_application)?;
        }
        Err(error)
    }

    fn complete(self, document: ItemDocument) -> Result<CliOutput, CliFailure> {
        self.access
            .into_unlocked()
            .map_err(map_application)?
            .add_item(
                document,
                self.now_ms,
                self.randomness,
                &self.application_store,
            )
            .map_err(map_application)?;
        Ok(CliOutput::success(format!(
            "Item added: {}\n",
            self.item_id.to_user_string()
        )))
    }
}

fn prepare_item_create(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
) -> Result<ItemCreateContext, CliFailure> {
    let now_ms = host.now_ms().map_err(map_host)?;
    let mut mutation_random = [0_u8; ADD_ITEM_RANDOM_BYTES];
    host.fill_entropy(&mut mutation_random).map_err(map_host)?;
    let randomness = AddItemRandomnessV1::new(mutation_random);
    let item_id = randomness.item_id();
    let mut operation_random = [0_u8; ITEM_OPERATION_RANDOM_BYTES];
    host.fill_entropy(&mut operation_random).map_err(map_host)?;
    let mut failure_random = [0_u8; AUDITED_ACCESS_RANDOM_BYTES];
    host.fill_entropy(&mut failure_random).map_err(map_host)?;
    let failure_randomness = AuditedAccessRandomnessV1::new(failure_random);
    let (access, application_store) = authenticated_access(host, paths, writer)?;
    let audit_enabled = access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled();
    Ok(ItemCreateContext {
        access,
        application_store,
        now_ms,
        randomness,
        item_id,
        favorite_operation: OperationId::new(operation_random),
        failure_randomness,
        audit_enabled,
    })
}

fn item_add_login(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
) -> Result<CliOutput, CliFailure> {
    let context = prepare_item_create(host, paths, writer)?;
    let input = (|| {
        Ok::<_, HostError>((
            host.read_login_title()?,
            host.read_login_username()?,
            host.read_login_password()?,
            host.read_login_url()?,
        ))
    })();
    let (title, username, password, url) = match input {
        Ok(input) => input,
        Err(error) => return context.fail(map_host(error)),
    };
    let document = context.document(
        LOGIN_V1,
        AnyRecord::Login(Login {
            title: title.into_inner(),
            username: username.into_inner(),
            password: password.into_inner(),
            urls: url.into_iter().map(Zeroizing::into_inner).collect(),
            notes: None,
        }),
    );
    let document = match document {
        Ok(document) => document,
        Err(error) => return context.fail(error),
    };
    context.complete(document)
}

fn item_add_secure_note(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
) -> Result<CliOutput, CliFailure> {
    let context = prepare_item_create(host, paths, writer)?;
    let input = (|| {
        Ok::<_, HostError>((
            host.read_secure_note_title()?,
            host.read_secure_note_body()?,
        ))
    })();
    let (title, body) = match input {
        Ok(input) => input,
        Err(error) => return context.fail(map_host(error)),
    };
    let document = context.document(
        SECURE_NOTE_V1,
        AnyRecord::SecureNote(SecureNote {
            title: title.into_inner(),
            body: body.into_inner(),
        }),
    );
    let document = match document {
        Ok(document) => document,
        Err(error) => return context.fail(error),
    };
    context.complete(document)
}

fn item_list(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
) -> Result<CliOutput, CliFailure> {
    let (mut access, application_store) = authenticated_access(host, paths, writer)?;
    let result = if access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled()
    {
        let (wall_time_ms, randomness) = audited_access_inputs(host)?;
        access
            .into_unlocked()
            .map_err(map_application)?
            .audited_list_items(wall_time_ms, randomness, &application_store)
            .map_err(map_application)?
            .into_operation()
    } else {
        let result = access
            .as_unlocked()
            .and_then(|session| session.list_items());
        access.lock();
        result
    };
    let items = result.map_err(map_application)?;
    if items.is_empty() {
        return Ok(CliOutput::success("No items.\n"));
    }
    let mut output = String::new();
    for item in items {
        output.push_str(&item.item_id.to_user_string());
        output.push('\t');
        output.push_str(item.schema.as_str());
        output.push('\t');
        output.push_str(&quoted(record_title(&item.record)));
        output.push('\n');
    }
    Ok(CliOutput::success(output))
}

fn item_edit_login(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    item_id: ItemId,
) -> Result<CliOutput, CliFailure> {
    let (access, application_store) = authenticated_access(host, paths, writer)?;
    let audit_enabled = access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled();
    if audit_enabled {
        let (wall_time_ms, failure_randomness) = audited_access_inputs(host)?;
        let preparation = access
            .into_unlocked()
            .map_err(map_application)?
            .prepare_audited_login_edit(
                item_id,
                wall_time_ms,
                failure_randomness,
                &application_store,
            )
            .map_err(map_application)?
            .into_preparation()
            .map_err(map_application)?;
        let input = match read_login_edit_input(host) {
            Ok(input) => input,
            Err(error) => {
                preparation
                    .record_audited_host_failure(&application_store)
                    .map_err(map_application)?;
                return Err(map_host(error));
            }
        };
        let mut mutation_random = [0_u8; REPLACE_ITEM_RANDOM_BYTES];
        if let Err(error) = host.fill_entropy(&mut mutation_random) {
            preparation
                .record_audited_host_failure(&application_store)
                .map_err(map_application)?;
            return Err(map_host(error));
        }
        preparation
            .complete_audited(
                input,
                ReplaceItemRandomnessV1::new(mutation_random),
                &application_store,
            )
            .map_err(map_application)?
            .into_operation()
            .map_err(map_application)?;
    } else {
        let preparation = access
            .into_unlocked()
            .map_err(map_application)?
            .prepare_login_edit(item_id)
            .map_err(map_application)?;
        let input = read_login_edit_input(host).map_err(map_host)?;
        let wall_time_ms = host.now_ms().map_err(map_host)?;
        let mut mutation_random = [0_u8; REPLACE_ITEM_RANDOM_BYTES];
        host.fill_entropy(&mut mutation_random).map_err(map_host)?;
        preparation
            .complete(
                input,
                wall_time_ms,
                ReplaceItemRandomnessV1::new(mutation_random),
                &application_store,
            )
            .map_err(map_application)?;
    }
    Ok(CliOutput::success(format!(
        "Item updated: {}\n",
        item_id.to_user_string()
    )))
}

fn read_login_edit_input(host: &dyn CliHost) -> Result<LoginEditInputV1, HostError> {
    Ok(LoginEditInputV1::new(
        host.read_login_title()?,
        host.read_login_username()?,
        host.read_login_password()?,
        host.read_login_url()?,
    ))
}

fn item_show(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    item_id: ItemId,
) -> Result<CliOutput, CliFailure> {
    let (mut access, application_store) = authenticated_access(host, paths, writer)?;
    let item = if access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled()
    {
        let (wall_time_ms, randomness) = audited_access_inputs(host)?;
        access
            .into_unlocked()
            .map_err(map_application)?
            .audited_get_item(item_id, wall_time_ms, randomness, &application_store)
            .map_err(map_application)?
            .into_operation()
            .map_err(map_application)?
    } else {
        let result = access
            .as_unlocked()
            .and_then(|session| session.get_item(item_id));
        access.lock();
        result
            .map_err(map_application)?
            .ok_or(CliFailure::NotFound)?
    };
    render_item(item)
}

fn item_delete(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    item_id: ItemId,
) -> Result<CliOutput, CliFailure> {
    let (access, application_store) = authenticated_access(host, paths, writer)?;
    let audit_enabled = access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled();
    let (now_ms, failure_randomness) = if audit_enabled {
        let (wall_time_ms, randomness) = audited_access_inputs(host)?;
        (wall_time_ms, Some(randomness))
    } else {
        (host.now_ms().map_err(map_host)?, None)
    };
    let mut mutation_random = [0_u8; DELETE_ITEM_RANDOM_BYTES];
    host.fill_entropy(&mut mutation_random).map_err(map_host)?;
    let session = access.into_unlocked().map_err(map_application)?;
    if let Some(failure_randomness) = failure_randomness {
        session
            .audited_delete_current_item(
                item_id,
                now_ms,
                now_ms,
                DeleteItemRandomnessV1::new(mutation_random),
                failure_randomness,
                &application_store,
            )
            .map_err(map_application)?
            .into_operation()
            .map_err(map_application)?;
    } else {
        session
            .delete_current_item(
                item_id,
                now_ms,
                now_ms,
                DeleteItemRandomnessV1::new(mutation_random),
                &application_store,
            )
            .map_err(map_application)?;
    }
    Ok(CliOutput::success(format!(
        "Item deleted: {}\n",
        item_id.to_user_string()
    )))
}

fn history_list(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    item_id: ItemId,
) -> Result<CliOutput, CliFailure> {
    let (mut access, application_store) = authenticated_access(host, paths, writer)?;
    let result = if access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled()
    {
        let (wall_time_ms, randomness) = audited_access_inputs(host)?;
        access
            .into_unlocked()
            .map_err(map_application)?
            .audited_item_history(
                item_id,
                DEFAULT_ITEM_HISTORY_LIMIT,
                wall_time_ms,
                randomness,
                &application_store,
            )
            .map_err(map_application)?
            .into_operation()
    } else {
        let result = access
            .as_unlocked()
            .and_then(|session| session.item_history(item_id, DEFAULT_ITEM_HISTORY_LIMIT));
        access.lock();
        result
    };
    let history = result.map_err(map_application)?;
    if history.is_empty() {
        return Err(CliFailure::NotFound);
    }
    render_history(history)
}

fn render_history(history: Vec<ItemHistoryViewV1>) -> Result<CliOutput, CliFailure> {
    let mut output = String::new();
    for revision in history {
        output.push_str(&revision.revision_id().to_user_string());
        if let Some(item) = revision.redacted_item() {
            output.push_str("\tlive\tparents=");
            output.push_str(&revision.causal_parent_count().to_string());
            output.push_str("\tupdated=");
            output.push_str(&revision.advisory_time_ms().to_string());
            output.push('\t');
            output.push_str(item.schema.as_str());
            output.push('\t');
            output.push_str(&quoted(record_title(&item.record)));
        } else {
            output.push_str("\tdeleted\tparents=");
            output.push_str(&revision.causal_parent_count().to_string());
            output.push_str("\tdeleted=");
            output.push_str(&revision.advisory_time_ms().to_string());
        }
        output.push('\n');
    }
    Ok(CliOutput::success(output))
}

fn history_restore(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    item_id: ItemId,
    revision_id: RevisionId,
) -> Result<CliOutput, CliFailure> {
    let (access, application_store) = authenticated_access(host, paths, writer)?;
    let audit_enabled = access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled();
    let (now_ms, failure_randomness) = if audit_enabled {
        let (wall_time_ms, randomness) = audited_access_inputs(host)?;
        (wall_time_ms, Some(randomness))
    } else {
        (host.now_ms().map_err(map_host)?, None)
    };
    let mut mutation_random = [0_u8; RESTORE_ITEM_RANDOM_BYTES];
    host.fill_entropy(&mut mutation_random).map_err(map_host)?;
    let session = access.into_unlocked().map_err(map_application)?;
    if let Some(failure_randomness) = failure_randomness {
        session
            .audited_restore_item_for_item(
                item_id,
                revision_id,
                DEFAULT_ITEM_HISTORY_LIMIT,
                now_ms,
                RestoreItemRandomnessV1::new(mutation_random),
                failure_randomness,
                &application_store,
            )
            .map_err(map_application)?
            .into_operation()
            .map_err(map_application)?;
    } else {
        session
            .restore_item_for_item(
                item_id,
                revision_id,
                DEFAULT_ITEM_HISTORY_LIMIT,
                now_ms,
                RestoreItemRandomnessV1::new(mutation_random),
                &application_store,
            )
            .map_err(map_application)?;
    }
    Ok(CliOutput::success(format!(
        "Item restored: {}\n",
        item_id.to_user_string()
    )))
}

fn record_title(record: &RedactedRecordView) -> &str {
    match record {
        RedactedRecordView::Login { title, .. }
        | RedactedRecordView::SecureNote { title, .. }
        | RedactedRecordView::Card { title, .. } => title,
        RedactedRecordView::TotpSeed { label, .. }
        | RedactedRecordView::ApiKey { label, .. }
        | RedactedRecordView::DatabaseCredential { label, .. } => label,
        RedactedRecordView::Opaque { content_type, .. } => content_type.as_str(),
    }
}

fn render_item(item: RedactedItemView) -> Result<CliOutput, CliFailure> {
    let mut output = format!(
        "Item: {}\nType: {}\n",
        item.item_id.to_user_string(),
        item.schema.as_str(),
    );
    match &item.record {
        RedactedRecordView::Login {
            title,
            username,
            urls,
            has_notes,
            ..
        } => {
            output.push_str("Title: ");
            output.push_str(&quoted(title));
            output.push_str("\nUsername: ");
            output.push_str(&quoted(username));
            output.push('\n');
            if urls.is_empty() {
                output.push_str("URL: none\n");
            } else {
                for url in urls {
                    output.push_str("URL: ");
                    output.push_str(&quoted(url));
                    output.push('\n');
                }
            }
            output.push_str("Password: <redacted>\nNotes: ");
            output.push_str(if *has_notes { "present\n" } else { "absent\n" });
        }
        RedactedRecordView::SecureNote { title, .. } => {
            output.push_str("Title: ");
            output.push_str(&quoted(title));
            output.push_str("\nBody: <redacted>\n");
        }
        _ => return Err(CliFailure::Unsupported),
    }
    output.push_str(if item.favorite {
        "Favorite: yes\n"
    } else {
        "Favorite: no\n"
    });
    output.push_str(&format!("Updated: {}\n", item.updated_at_ms));
    Ok(CliOutput::success(output))
}

fn quoted(value: &str) -> String {
    format!("{value:?}")
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

fn audit_enable(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
) -> Result<CliOutput, CliFailure> {
    let (mut access, application_store) = authenticated_access(host, paths, writer)?;
    if access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled()
    {
        access.lock();
        return Ok(CliOutput::success("Audit: already enabled.\n"));
    }
    let (wall_time_ms, randomness) = audited_access_inputs(host)?;
    access
        .into_unlocked()
        .map_err(map_application)?
        .activate_audit_epoch(wall_time_ms, randomness, &application_store)
        .map_err(map_application)?;
    Ok(CliOutput::success("Audit: enabled.\n"))
}

fn audit_verify(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
) -> Result<CliOutput, CliFailure> {
    let exact_config = writer
        .load_config()
        .map_err(map_local_host)?
        .ok_or(CliFailure::InvalidCommand)?;
    let config = decode_config(&exact_config)?;
    let vault = configured_vault(paths, &config)?;
    let locator = application_locator(vault.locator());
    let application_store = application_store(paths);
    let repository_factory = repository_factory(paths);
    let mut access = VaultAccessV1::locked(locator);
    let passphrase = host.read_existing_passphrase().map_err(map_host)?;
    access
        .unlock(
            passphrase,
            &application_store,
            &application_store,
            &repository_factory,
        )
        .map_err(map_application)?;
    let result = if access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled()
    {
        let (wall_time_ms, randomness) = audited_access_inputs(host)?;
        access
            .into_unlocked()
            .map_err(map_application)?
            .audited_verify(wall_time_ms, randomness, &application_store)
            .map_err(map_application)?
            .into_operation()
    } else {
        let result = access
            .as_unlocked()
            .and_then(|session| session.audit_verify());
        access.lock();
        result
    };
    let report = result.map_err(map_application)?;
    Ok(render_audit(report))
}

fn audit_list(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
) -> Result<CliOutput, CliFailure> {
    let (access, application_store) = authenticated_access(host, paths, writer)?;
    let (wall_time_ms, randomness) = audited_access_inputs(host)?;
    let events = access
        .into_unlocked()
        .map_err(map_application)?
        .audited_audit_history(
            DEFAULT_AUDIT_HISTORY_LIMIT,
            wall_time_ms,
            randomness,
            &application_store,
        )
        .map_err(map_application)?
        .into_operation()
        .map_err(map_application)?;
    Ok(render_audit_events(events))
}

fn audit_show(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    trace_id: OperationId,
) -> Result<CliOutput, CliFailure> {
    let (access, application_store) = authenticated_access(host, paths, writer)?;
    let (wall_time_ms, randomness) = audited_access_inputs(host)?;
    let event = access
        .into_unlocked()
        .map_err(map_application)?
        .audited_audit_event(trace_id, wall_time_ms, randomness, &application_store)
        .map_err(map_application)?
        .into_operation()
        .map_err(map_application)?
        .ok_or(CliFailure::NotFound)?;
    Ok(render_audit_events(vec![event]))
}

fn doctor(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    unlock: bool,
) -> Result<CliOutput, CliFailure> {
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
    let mut access = VaultAccessV1::locked(locator);
    if unlock {
        let repository_factory = repository_factory(paths);
        let passphrase = host.read_existing_passphrase().map_err(map_host)?;
        access
            .unlock(
                passphrase,
                &application_store,
                &application_store,
                &repository_factory,
            )
            .map_err(map_application)?;
    }
    let report = if unlock
        && access
            .as_unlocked()
            .map_err(map_application)?
            .audit_enabled()
    {
        let (wall_time_ms, randomness) = audited_access_inputs(host)?;
        access
            .into_unlocked()
            .map_err(map_application)?
            .audited_doctor(
                &application_store,
                &application_store,
                wall_time_ms,
                randomness,
            )
            .map_err(map_application)?
            .into_operation()
            .map_err(map_application)?
    } else {
        let report = access.doctor(&application_store, &application_store);
        access.lock();
        report
    };
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

fn render_audit(report: AuditVerificationV1) -> CliOutput {
    CliOutput::success(format!(
        "Audit: verified (announcements={} commits={} catalogs={} revisions={} items={} audit_events={})\n",
        report.announcement_count(),
        report.commit_count(),
        report.catalog_count(),
        report.revision_count(),
        report.item_count(),
        report.audit_event_count(),
    ))
}

fn render_audit_events(events: Vec<AuditEventViewV1>) -> CliOutput {
    let mut output = String::new();
    for event in events {
        output.push_str(&event.trace_id().to_user_string());
        output.push_str("\tcounter=");
        output.push_str(&event.device_counter().to_string());
        output.push_str("\taction=");
        output.push_str(event.action().label());
        output.push_str("\toutcome=");
        output.push_str(event.outcome().label());
        output.push_str("\ttime=");
        output.push_str(&event.timestamp_ms().to_string());
        if let Some(item_id) = event.item_id() {
            output.push_str("\titem=");
            output.push_str(&item_id.to_user_string());
        }
        if let Some(revision_id) = event.selected_revision() {
            output.push_str("\tselected=");
            output.push_str(&revision_id.to_user_string());
        }
        if let Some(revision_id) = event.result_revision() {
            output.push_str("\tresult=");
            output.push_str(&revision_id.to_user_string());
        }
        output.push('\n');
    }
    CliOutput::success(output)
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
        | CliHostError::EmptyText
        | CliHostError::InvalidText
        | CliHostError::InvalidExportDestination
        | CliHostError::ExportDestinationExists
        | CliHostError::InvalidImportSource
        | CliHostError::InvalidEntropyRequest => HostError::Invalid,
        CliHostError::UnsupportedPlatform => HostError::Unsupported,
        CliHostError::TerminalUnavailable
        | CliHostError::TerminalAccessFailed
        | CliHostError::TerminalModeFailed
        | CliHostError::SecretInputFailed
        | CliHostError::TextInputFailed
        | CliHostError::ExportWriteFailed
        | CliHostError::ImportReadFailed
        | CliHostError::EntropyUnavailable => HostError::Unavailable,
    }
}

fn map_bootstrap_store(error: BootstrapStoreError) -> CliFailure {
    match error {
        BootstrapStoreError::Unavailable => CliFailure::Provider,
        BootstrapStoreError::Conflict | BootstrapStoreError::Corruption => CliFailure::Integrity,
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

    fn first_storage_record_with_body_magic(
        root: &std::path::Path,
        body_magic: &[u8],
    ) -> Option<PathBuf> {
        for entry in fs::read_dir(root).ok()? {
            let entry = entry.ok()?;
            let path = entry.path();
            if entry.file_type().ok()?.is_dir() {
                if let Some(found) = first_storage_record_with_body_magic(&path, body_magic) {
                    return Some(found);
                }
            } else if let Ok(bytes) = fs::read(&path) {
                let meta_len = bytes
                    .get(5..9)
                    .and_then(|exact| <[u8; 4]>::try_from(exact).ok())
                    .map(u32::from_be_bytes)? as usize;
                let body_start = 9_usize.checked_add(meta_len)?;
                let magic_end = body_start.checked_add(body_magic.len())?;
                if bytes.get(body_start..magic_end) == Some(body_magic) {
                    return Some(path);
                }
            }
        }
        None
    }

    struct TestHost {
        paths: LocalVaultPaths,
        secrets: Mutex<VecDeque<Vec<u8>>>,
        texts: Mutex<VecDeque<String>>,
        entropy_seed: u8,
    }

    impl TestHost {
        fn new(paths: LocalVaultPaths, secrets: impl IntoIterator<Item = Vec<u8>>) -> Self {
            Self {
                paths,
                secrets: Mutex::new(secrets.into_iter().collect()),
                texts: Mutex::new(VecDeque::new()),
                entropy_seed: 1,
            }
        }

        fn with_entropy_seed(
            paths: LocalVaultPaths,
            secrets: impl IntoIterator<Item = Vec<u8>>,
            entropy_seed: u8,
        ) -> Self {
            Self {
                paths,
                secrets: Mutex::new(secrets.into_iter().collect()),
                texts: Mutex::new(VecDeque::new()),
                entropy_seed,
            }
        }

        fn with_texts(
            paths: LocalVaultPaths,
            secrets: impl IntoIterator<Item = Vec<u8>>,
            texts: impl IntoIterator<Item = String>,
        ) -> Self {
            Self {
                paths,
                secrets: Mutex::new(secrets.into_iter().collect()),
                texts: Mutex::new(texts.into_iter().collect()),
                entropy_seed: 1,
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

        fn text(&self) -> Result<Zeroizing<String>, HostError> {
            self.texts
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

        fn read_login_title(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_login_username(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_login_url(&self) -> Result<Option<Zeroizing<String>>, HostError> {
            self.text()
                .map(|value| (!value.is_empty()).then_some(value))
        }

        fn read_login_password(&self) -> Result<Zeroizing<String>, HostError> {
            let value = self.secret()?;
            let text = core::str::from_utf8(&value).map_err(|_| HostError::Invalid)?;
            Ok(Zeroizing::new(text.to_owned()))
        }

        fn read_secure_note_title(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_secure_note_body(&self) -> Result<Zeroizing<String>, HostError> {
            let value = self.secret()?;
            let text = core::str::from_utf8(&value).map_err(|_| HostError::Invalid)?;
            Ok(Zeroizing::new(text.to_owned()))
        }

        fn read_export_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
            self.secret()
        }

        fn write_portable_export(
            &self,
            destination: &Path,
            artifact: &[u8],
        ) -> Result<(), HostError> {
            write_portable_export(destination, artifact).map_err(map_native_cli_host)
        }

        fn read_portable_export(&self, source: &Path) -> Result<Vec<u8>, HostError> {
            read_portable_export(source, MAX_PORTABLE_EXPORT_ARTIFACT_BYTES)
                .map_err(map_native_cli_host)
        }

        fn read_import_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
            self.secret()
        }

        fn fill_entropy(&self, output: &mut [u8]) -> Result<(), HostError> {
            for (index, byte) in output.iter_mut().enumerate() {
                *byte = u8::try_from(index % 251)
                    .unwrap()
                    .wrapping_add(self.entropy_seed);
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

    fn activate_test_audit_epoch(paths: &LocalVaultPaths, passphrase: Vec<u8>) {
        let prepared = paths.prepare().unwrap();
        let writer = prepared.try_acquire_writer().unwrap();
        let host = TestHost::new(paths.clone(), [passphrase]);
        let (access, application_store) = authenticated_access(&host, paths, &writer).unwrap();
        let (wall_time_ms, randomness) = audited_access_inputs(&host).unwrap();
        access
            .into_unlocked()
            .unwrap()
            .activate_audit_epoch(wall_time_ms, randomness, &application_store)
            .unwrap();
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
            vec!["doctor", "--unlock", "extra"],
            vec!["export"],
            vec!["export", "backup.vpm", "extra"],
            vec!["export", "--passphrase", "secret"],
            vec!["import"],
            vec!["import", "backup.vpm", "extra"],
            vec!["import", "--passphrase", "secret"],
            vec!["audit"],
            vec!["audit", "enable", "extra"],
            vec!["audit", "verify", "extra"],
            vec!["audit", "list", "extra"],
            vec!["audit", "show", "not-a-trace"],
            vec!["audit", "show", "not-a-trace", "extra"],
            vec!["item", "add", "login", "--password", "secret"],
            vec!["item", "add", "secure-note", "--body", "secret"],
            vec!["item", "edit", "not-an-item-id"],
            vec!["item", "delete", "not-an-item-id"],
            vec!["item", "list", "extra"],
            vec!["item", "show", "not-an-item-id"],
            vec!["history"],
            vec!["history", "list", "not-an-item-id"],
            vec!["history", "list", "not-an-item-id", "extra"],
            vec!["history", "restore", "not-an-item-id", "not-a-revision"],
            vec!["unlock"],
        ] {
            let output = run(arguments, &host);
            assert_eq!(output.exit_code(), ExitCode::InvalidInput);
            assert_eq!(output.stderr(), "vault-pm: invalid command\n");
        }
        assert!(!root.0.join("config").exists());
    }

    #[test]
    fn audit_enable_installs_one_durable_epoch_and_is_idempotent() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"audit migration passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let enable_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let enabled = run(["audit", "enable"], &enable_host);
        assert_eq!(enabled.exit_code(), ExitCode::Success, "{enabled:?}");
        assert_eq!(enabled.stdout(), "Audit: enabled.\n");

        let repeated_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let repeated = run(["audit", "enable"], &repeated_host);
        assert_eq!(repeated.exit_code(), ExitCode::Success, "{repeated:?}");
        assert_eq!(repeated.stdout(), "Audit: already enabled.\n");

        let verify_host = TestHost::new(paths, [passphrase]);
        let verified = run(["audit", "verify"], &verify_host);
        assert_eq!(verified.exit_code(), ExitCode::Success, "{verified:?}");
        assert!(verified.stdout().contains("commits=2"), "{verified:?}");
        assert!(verified.stdout().contains("audit_events=1"), "{verified:?}");
    }

    #[test]
    fn audit_history_is_trace_selectable_and_logs_missing_lookups() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"traceable audit passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);
        activate_test_audit_epoch(&paths, passphrase.clone());

        let list_host = TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 2);
        let listed = run(["audit", "list"], &list_host);
        assert_eq!(listed.exit_code(), ExitCode::Success, "{listed:?}");
        let rows = listed.stdout().lines().collect::<Vec<_>>();
        assert_eq!(rows.len(), 2, "{listed:?}");
        assert!(rows[0].contains("\tcounter=3\taction=audit_read\toutcome=succeeded\t"));
        assert!(rows[1].contains("\tcounter=2\taction=audit_epoch_start\toutcome=succeeded\t"));
        let listed_trace = rows[0].split('\t').next().unwrap();
        assert!(OperationId::from_user_string(listed_trace).is_ok());
        assert!(!listed.stdout().contains("traceable audit passphrase"));
        assert!(!listed.stdout().contains("personal"));

        let show_host = TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 3);
        let shown = run(["audit", "show", listed_trace], &show_host);
        assert_eq!(shown.exit_code(), ExitCode::Success, "{shown:?}");
        assert_eq!(shown.stdout(), format!("{}\n", rows[0]));

        let missing_trace = OperationId::new([0xff; 32]).to_user_string();
        let missing_host = TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 4);
        let missing = run(["audit", "show", missing_trace.as_str()], &missing_host);
        assert_eq!(missing.exit_code(), ExitCode::NotFound, "{missing:?}");
        assert!(missing.stdout().is_empty());
        assert_eq!(missing.stderr(), "vault-pm: not found\n");

        let verify_host = TestHost::with_entropy_seed(paths, [passphrase], 5);
        let verified = run(["audit", "verify"], &verify_host);
        assert_eq!(verified.exit_code(), ExitCode::Success, "{verified:?}");
        assert!(verified.stdout().contains("audit_events=4"), "{verified:?}");
    }

    #[test]
    fn item_identity_parsers_require_the_canonical_item_id() {
        let item_id = ItemId::new([0x5a; 16]);
        let canonical = item_id.to_user_string();
        assert_eq!(
            parse(["item", "show", canonical.as_str()]),
            Ok(Command::ItemShow { item_id })
        );
        assert_eq!(
            parse(["item", "show", canonical.to_lowercase().as_str()]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse(["item", "edit", canonical.as_str()]),
            Ok(Command::ItemEdit { item_id })
        );
        assert_eq!(
            parse(["item", "delete", canonical.as_str()]),
            Ok(Command::ItemDelete { item_id })
        );
        assert_eq!(
            parse(["history", "list", canonical.as_str()]),
            Ok(Command::HistoryList { item_id })
        );
        let revision_id = RevisionId::new([0x6b; 32]);
        let revision = revision_id.to_user_string();
        assert_eq!(
            parse(["history", "restore", canonical.as_str(), revision.as_str(),]),
            Ok(Command::HistoryRestore {
                item_id,
                revision_id,
            })
        );
        assert_eq!(
            parse([
                "history",
                "restore",
                canonical.as_str(),
                revision.to_lowercase().as_str(),
            ]),
            Err(CliFailure::InvalidCommand)
        );
    }

    #[test]
    fn audit_show_parser_requires_the_canonical_trace_id() {
        let trace_id = OperationId::new([0x7c; 32]);
        let canonical = trace_id.to_user_string();
        assert_eq!(
            parse(["audit", "show", canonical.as_str()]),
            Ok(Command::AuditShow { trace_id })
        );
        assert_eq!(
            parse(["audit", "show", canonical.to_lowercase().as_str()]),
            Err(CliFailure::InvalidCommand)
        );
    }

    #[test]
    fn portable_export_parser_accepts_exactly_one_explicit_destination() {
        assert_eq!(
            parse(["export", "backup.vpm"]),
            Ok(Command::PortableExport {
                destination: PathBuf::from("backup.vpm")
            })
        );
        assert_eq!(parse(["export"]), Err(CliFailure::InvalidCommand));
        assert_eq!(
            parse(["export", "backup.vpm", "extra"]),
            Err(CliFailure::InvalidCommand)
        );
    }

    #[test]
    fn portable_import_parser_accepts_exactly_one_explicit_source() {
        assert_eq!(
            parse(["import", "backup.vpm"]),
            Ok(Command::PortableImport {
                source: PathBuf::from("backup.vpm")
            })
        );
        assert_eq!(parse(["import"]), Err(CliFailure::InvalidCommand));
        assert_eq!(
            parse(["import", "backup.vpm", "extra"]),
            Err(CliFailure::InvalidCommand)
        );
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
    fn authenticated_audit_and_doctor_unlock_for_one_operation() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"correct horse battery staple".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let audit_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let audit = run(["audit", "verify"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert_eq!(
            audit.stdout(),
            "Audit: verified (announcements=1 commits=1 catalogs=1 revisions=0 items=0 audit_events=0)\n"
        );
        assert!(audit.stderr().is_empty());

        let doctor_host = TestHost::new(paths.clone(), [passphrase]);
        let doctor = run(["doctor", "--unlock"], &doctor_host);
        assert_eq!(doctor.exit_code(), ExitCode::Success, "{doctor:?}");
        assert_eq!(doctor.stdout(), "Doctor: healthy\n");
        assert!(doctor.stderr().is_empty());

        let locked = TestHost::new(paths, []);
        assert_eq!(run(["status"], &locked).stdout(), "Status: locked\n");
    }

    #[test]
    fn active_epoch_routes_direct_cli_reads_through_durable_audit_events() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"audited cli passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);
        let add_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), b"audited item secret".to_vec()],
            [
                "Audited CLI item".to_owned(),
                "user@example.test".to_owned(),
                String::new(),
            ],
        );
        assert_eq!(
            run(["item", "add", "login"], &add_host).exit_code(),
            ExitCode::Success
        );
        let item_id =
            ItemId::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]).to_user_string();
        activate_test_audit_epoch(&paths, passphrase.clone());

        for arguments in [
            vec!["item", "list"],
            vec!["item", "show", item_id.as_str()],
            vec!["history", "list", item_id.as_str()],
        ] {
            let host = TestHost::new(paths.clone(), [passphrase.clone()]);
            let output = run(arguments, &host);
            assert_eq!(output.exit_code(), ExitCode::Success, "{output:?}");
            assert!(!output.stdout().contains("audited item secret"));
        }

        let audit_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let audit = run(["audit", "verify"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert!(audit.stdout().contains("commits=6"), "{audit:?}");
        assert!(audit.stdout().contains("audit_events=4"), "{audit:?}");

        let doctor_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let doctor = run(["doctor", "--unlock"], &doctor_host);
        assert_eq!(doctor.exit_code(), ExitCode::Success, "{doctor:?}");

        let final_host = TestHost::new(paths, [passphrase]);
        let final_audit = run(["audit", "verify"], &final_host);
        assert_eq!(
            final_audit.exit_code(),
            ExitCode::Success,
            "{final_audit:?}"
        );
        assert!(
            final_audit.stdout().contains("commits=8"),
            "{final_audit:?}"
        );
        assert!(
            final_audit.stdout().contains("audit_events=6"),
            "{final_audit:?}"
        );
    }

    #[test]
    fn active_epoch_records_item_create_prompt_failure_before_returning() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"audited create failure passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);
        activate_test_audit_epoch(&paths, passphrase.clone());

        let failed_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let failed = run(["item", "add", "login"], &failed_host);
        assert_eq!(failed.exit_code(), ExitCode::Provider, "{failed:?}");
        assert!(failed.stdout().is_empty());

        let list_host = TestHost::with_entropy_seed(paths, [passphrase], 2);
        let audit = run(["audit", "list"], &list_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        let expected_item =
            ItemId::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]).to_user_string();
        assert!(
            audit.stdout().contains(&format!(
                "action=item_create\toutcome=failed\ttime=1700000000000\titem={expected_item}"
            )),
            "{audit:?}"
        );
        assert!(!audit.stdout().contains("audited create failure passphrase"));
    }

    #[test]
    fn portable_export_is_encrypted_audited_and_never_overwrites() {
        let root = TestRoot::new();
        let paths = root.paths();
        let vault_passphrase = b"portable source passphrase".to_vec();
        let export_passphrase = b"distinct export passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [vault_passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);
        activate_test_audit_epoch(&paths, vault_passphrase.clone());

        let destination = root.0.join("backup.vpm");
        let export_host = TestHost::new(
            paths.clone(),
            [vault_passphrase.clone(), export_passphrase.clone()],
        );
        let exported = run(
            [
                "export",
                destination.to_str().expect("UTF-8 test destination"),
            ],
            &export_host,
        );
        assert_eq!(exported.exit_code(), ExitCode::Success, "{exported:?}");
        assert_eq!(exported.stdout(), "Portable export written.\n");
        let artifact = fs::read(&destination).unwrap();
        assert!(!artifact.is_empty());
        assert!(!artifact
            .windows(vault_passphrase.len())
            .any(|window| window == vault_passphrase));
        assert!(!artifact
            .windows(export_passphrase.len())
            .any(|window| window == export_passphrase));
        let opened = coding_adventures_vault_pm_application::open_portable_with_passphrase(
            &artifact,
            Zeroizing::new(export_passphrase.clone()),
            coding_adventures_vault_pm_application::PortableOpenPolicyV1::new(8 * 1024, 1, 1)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(opened.item_count(), 0);
        assert_eq!(opened.candidate_count(), 0);

        let repeated_host = TestHost::new(
            paths.clone(),
            [vault_passphrase.clone(), export_passphrase.clone()],
        );
        let repeated = run(
            [
                "export",
                destination.to_str().expect("UTF-8 test destination"),
            ],
            &repeated_host,
        );
        assert_eq!(repeated.exit_code(), ExitCode::InvalidInput, "{repeated:?}");
        assert_eq!(fs::read(&destination).unwrap(), artifact);

        let audit_host = TestHost::with_entropy_seed(paths, [vault_passphrase], 8);
        let audit = run(["audit", "list"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert_eq!(audit.stdout().matches("action=portable_export").count(), 2);
        assert_eq!(audit.stdout().matches("outcome=succeeded").count(), 4);
        assert!(!audit.stdout().contains("distinct export passphrase"));
        assert!(!audit.stdout().contains("backup.vpm"));
    }

    #[test]
    fn active_epoch_records_export_prompt_failure_before_returning() {
        let root = TestRoot::new();
        let paths = root.paths();
        let vault_passphrase = b"portable failure passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [vault_passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);
        activate_test_audit_epoch(&paths, vault_passphrase.clone());

        let destination = root.0.join("never-written.vpm");
        let failed_host = TestHost::new(paths.clone(), [vault_passphrase.clone()]);
        let failed = run(
            [
                "export",
                destination.to_str().expect("UTF-8 test destination"),
            ],
            &failed_host,
        );
        assert_eq!(failed.exit_code(), ExitCode::Provider, "{failed:?}");
        assert!(!destination.exists());

        let audit_host = TestHost::with_entropy_seed(paths, [vault_passphrase], 9);
        let audit = run(["audit", "list"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert!(audit
            .stdout()
            .contains("action=portable_export\toutcome=failed"));
        assert!(!audit.stdout().contains("never-written.vpm"));
        assert!(!audit.stdout().contains("portable failure passphrase"));
    }

    #[test]
    fn portable_import_requires_auditing_logs_failure_then_restores_independently() {
        let source_root = TestRoot::new();
        let source_paths = source_root.paths();
        let source_passphrase = b"portable source vault passphrase".to_vec();
        let export_passphrase = b"portable artifact passphrase".to_vec();
        let init_source = TestHost::new(source_paths.clone(), [source_passphrase.clone()]);
        assert_eq!(run(["init"], &init_source).exit_code(), ExitCode::Success);
        let add_source = TestHost::with_texts(
            source_paths.clone(),
            [
                source_passphrase.clone(),
                b"restored secure note body".to_vec(),
            ],
            ["Restored secure note".to_owned()],
        );
        assert_eq!(
            run(["item", "add", "secure-note"], &add_source).exit_code(),
            ExitCode::Success
        );
        activate_test_audit_epoch(&source_paths, source_passphrase.clone());
        let artifact_path = source_root.0.join("restore-source.vpm");
        let export_host = TestHost::new(
            source_paths.clone(),
            [source_passphrase.clone(), export_passphrase.clone()],
        );
        assert_eq!(
            run(
                [
                    "export",
                    artifact_path.to_str().expect("UTF-8 test artifact path"),
                ],
                &export_host,
            )
            .exit_code(),
            ExitCode::Success
        );

        let target_root = TestRoot::new();
        let target_paths = target_root.paths();
        let target_passphrase = b"independent target passphrase".to_vec();
        let init_target =
            TestHost::with_entropy_seed(target_paths.clone(), [target_passphrase.clone()], 23);
        assert_eq!(run(["init"], &init_target).exit_code(), ExitCode::Success);

        let pre_audit = TestHost::new(
            target_paths.clone(),
            [target_passphrase.clone(), export_passphrase.clone()],
        );
        let rejected = run(
            [
                "import",
                artifact_path.to_str().expect("UTF-8 test artifact path"),
            ],
            &pre_audit,
        );
        assert_eq!(rejected.exit_code(), ExitCode::InvalidInput, "{rejected:?}");
        activate_test_audit_epoch(&target_paths, target_passphrase.clone());

        let wrong_host = TestHost::new(
            target_paths.clone(),
            [
                target_passphrase.clone(),
                b"wrong artifact passphrase".to_vec(),
            ],
        );
        let wrong = run(
            [
                "import",
                artifact_path.to_str().expect("UTF-8 test artifact path"),
            ],
            &wrong_host,
        );
        assert_eq!(wrong.exit_code(), ExitCode::Locked, "{wrong:?}");
        assert!(wrong.stdout().is_empty());

        let import_host = TestHost::with_entropy_seed(
            target_paths.clone(),
            [target_passphrase.clone(), export_passphrase.clone()],
            41,
        );
        let imported = run(
            [
                "import",
                artifact_path.to_str().expect("UTF-8 test artifact path"),
            ],
            &import_host,
        );
        assert_eq!(imported.exit_code(), ExitCode::Success, "{imported:?}");
        assert_eq!(
            imported.stdout(),
            "Portable import complete: items=1 candidates=1.\n"
        );

        let list_host = TestHost::new(target_paths.clone(), [target_passphrase.clone()]);
        let listed = run(["item", "list"], &list_host);
        assert_eq!(listed.exit_code(), ExitCode::Success, "{listed:?}");
        assert!(listed
            .stdout()
            .contains("vault/note/v1\t\"Restored secure note\""));
        assert!(!listed.stdout().contains("restored secure note body"));

        let audit_host = TestHost::with_entropy_seed(target_paths, [target_passphrase], 17);
        let audit = run(["audit", "list"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert!(audit
            .stdout()
            .contains("action=portable_import\toutcome=failed"));
        assert!(audit
            .stdout()
            .contains("action=portable_import\toutcome=succeeded"));
        assert!(!audit.stdout().contains("restore-source.vpm"));
        assert!(!audit.stdout().contains("portable artifact passphrase"));

        let source_list = TestHost::new(source_paths, [source_passphrase]);
        let source_items = run(["item", "list"], &source_list);
        assert_eq!(
            source_items.exit_code(),
            ExitCode::Success,
            "{source_items:?}"
        );
        assert!(source_items.stdout().contains("Restored secure note"));
    }

    #[test]
    fn active_epoch_delete_keeps_revision_capability_inside_one_mutation() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"audited delete passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);
        let add_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), b"delete secret".to_vec()],
            [
                "Delete me".to_owned(),
                "user@example.test".to_owned(),
                String::new(),
            ],
        );
        assert_eq!(
            run(["item", "add", "login"], &add_host).exit_code(),
            ExitCode::Success
        );
        let item_id =
            ItemId::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]).to_user_string();
        activate_test_audit_epoch(&paths, passphrase.clone());

        let delete_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let deleted = run(["item", "delete", item_id.as_str()], &delete_host);
        assert_eq!(deleted.exit_code(), ExitCode::Success, "{deleted:?}");
        assert_eq!(deleted.stdout(), format!("Item deleted: {item_id}\n"));

        let repeated_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let repeated = run(["item", "delete", item_id.as_str()], &repeated_host);
        assert_eq!(repeated.exit_code(), ExitCode::NotFound, "{repeated:?}");
        assert_eq!(repeated.stderr(), "vault-pm: not found\n");

        let audit_host = TestHost::new(paths, [passphrase]);
        let audit = run(["audit", "verify"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert!(audit.stdout().contains("commits=5"), "{audit:?}");
        assert!(audit.stdout().contains("audit_events=3"), "{audit:?}");
    }

    #[test]
    fn active_epoch_restore_binds_item_and_revision_inside_one_mutation() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"audited restore passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);
        let add_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), b"restore secret".to_vec()],
            [
                "Restore me".to_owned(),
                "user@example.test".to_owned(),
                String::new(),
            ],
        );
        assert_eq!(
            run(["item", "add", "login"], &add_host).exit_code(),
            ExitCode::Success
        );
        let item_id =
            ItemId::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]).to_user_string();
        let history_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let history = run(["history", "list", item_id.as_str()], &history_host);
        assert_eq!(history.exit_code(), ExitCode::Success, "{history:?}");
        let original_revision = history
            .stdout()
            .lines()
            .next()
            .unwrap()
            .split('\t')
            .next()
            .unwrap()
            .to_owned();
        let delete_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(
            run(["item", "delete", item_id.as_str()], &delete_host).exit_code(),
            ExitCode::Success
        );
        let deleted_history_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let deleted_history = run(["history", "list", item_id.as_str()], &deleted_history_host);
        assert_eq!(deleted_history.exit_code(), ExitCode::Success);
        let tombstone_revision = deleted_history
            .stdout()
            .lines()
            .next()
            .unwrap()
            .split('\t')
            .next()
            .unwrap()
            .to_owned();
        activate_test_audit_epoch(&paths, passphrase.clone());

        let tombstone_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let tombstone = run(
            [
                "history",
                "restore",
                item_id.as_str(),
                tombstone_revision.as_str(),
            ],
            &tombstone_host,
        );
        assert_eq!(
            tombstone.exit_code(),
            ExitCode::InvalidInput,
            "{tombstone:?}"
        );

        let restore_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let restored = run(
            [
                "history",
                "restore",
                item_id.as_str(),
                original_revision.as_str(),
            ],
            &restore_host,
        );
        assert_eq!(restored.exit_code(), ExitCode::Success, "{restored:?}");
        assert_eq!(restored.stdout(), format!("Item restored: {item_id}\n"));

        let audit_host = TestHost::new(paths, [passphrase]);
        let audit = run(["audit", "verify"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert!(audit.stdout().contains("commits=6"), "{audit:?}");
        assert!(audit.stdout().contains("audit_events=3"), "{audit:?}");
    }

    #[test]
    fn active_epoch_edit_records_prompt_failure_before_success() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"audited edit passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);
        let add_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), b"original secret".to_vec()],
            [
                "Edit me".to_owned(),
                "original@example.test".to_owned(),
                String::new(),
            ],
        );
        assert_eq!(
            run(["item", "add", "login"], &add_host).exit_code(),
            ExitCode::Success
        );
        let item_id =
            ItemId::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]).to_user_string();
        activate_test_audit_epoch(&paths, passphrase.clone());

        let failed_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let failed = run(["item", "edit", item_id.as_str()], &failed_host);
        assert_eq!(failed.exit_code(), ExitCode::Provider, "{failed:?}");
        assert!(failed.stdout().is_empty());

        let replacement = b"replacement secret".to_vec();
        let edit_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), replacement],
            [
                "Edited".to_owned(),
                "edited@example.test".to_owned(),
                String::new(),
            ],
        );
        let edited = run(["item", "edit", item_id.as_str()], &edit_host);
        assert_eq!(edited.exit_code(), ExitCode::Success, "{edited:?}");
        assert_eq!(edited.stdout(), format!("Item updated: {item_id}\n"));
        assert!(!edited.stdout().contains("replacement secret"));

        let audit_host = TestHost::new(paths, [passphrase]);
        let audit = run(["audit", "verify"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert!(audit.stdout().contains("commits=5"), "{audit:?}");
        assert!(audit.stdout().contains("audit_events=3"), "{audit:?}");
    }

    #[test]
    fn login_add_list_and_show_survive_restart_without_rendering_password() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"correct horse battery staple".to_vec();
        let password = b"item password must stay secret".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let add_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), password.clone()],
            [
                "Example account".to_string(),
                "ada@example.test".to_string(),
                "https://example.test".to_string(),
            ],
        );
        let added = run(["item", "add", "login"], &add_host);
        assert_eq!(added.exit_code(), ExitCode::Success, "{added:?}");
        let expected_id =
            ItemId::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]).to_user_string();
        assert_eq!(added.stdout(), format!("Item added: {expected_id}\n"));
        assert!(!added.stdout().contains("item password"));

        let list_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let listed = run(["item", "list"], &list_host);
        assert_eq!(listed.exit_code(), ExitCode::Success, "{listed:?}");
        assert_eq!(
            listed.stdout(),
            format!("{expected_id}\t{LOGIN_V1}\t\"Example account\"\n")
        );
        assert!(!listed.stdout().contains("item password"));

        let show_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let shown = run(["item", "show", expected_id.as_str()], &show_host);
        assert_eq!(shown.exit_code(), ExitCode::Success, "{shown:?}");
        assert_eq!(
            shown.stdout(),
            format!(
                "Item: {expected_id}\nType: {LOGIN_V1}\nTitle: \"Example account\"\nUsername: \"ada@example.test\"\nURL: \"https://example.test\"\nPassword: <redacted>\nNotes: absent\nFavorite: no\nUpdated: 1700000000000\n"
            )
        );
        assert!(!shown
            .stdout()
            .contains(core::str::from_utf8(&password).unwrap()));

        let updated_password = b"replacement password stays secret".to_vec();
        let edit_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), updated_password.clone()],
            [
                "Updated account".to_string(),
                "grace@example.test".to_string(),
                String::new(),
            ],
        );
        let edited = run(["item", "edit", expected_id.as_str()], &edit_host);
        assert_eq!(edited.exit_code(), ExitCode::Success, "{edited:?}");
        assert_eq!(edited.stdout(), format!("Item updated: {expected_id}\n"));
        assert!(!edited.stdout().contains("replacement password"));

        let updated_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let updated = run(["item", "show", expected_id.as_str()], &updated_host);
        assert_eq!(updated.exit_code(), ExitCode::Success, "{updated:?}");
        assert_eq!(
            updated.stdout(),
            format!(
                "Item: {expected_id}\nType: {LOGIN_V1}\nTitle: \"Updated account\"\nUsername: \"grace@example.test\"\nURL: none\nPassword: <redacted>\nNotes: absent\nFavorite: no\nUpdated: 1700000000000\n"
            )
        );
        assert!(!updated
            .stdout()
            .contains(core::str::from_utf8(&updated_password).unwrap()));

        let history_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let history = run(["history", "list", expected_id.as_str()], &history_host);
        assert_eq!(history.exit_code(), ExitCode::Success, "{history:?}");
        let lines = history.stdout().lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\tlive\tparents=1\tupdated=1700000000000\t"));
        assert!(lines[0].ends_with("vault/login/v1\t\"Updated account\""));
        assert!(lines[1].contains("\tlive\tparents=0\tupdated=1700000000000\t"));
        assert!(lines[1].ends_with("vault/login/v1\t\"Example account\""));
        let original_revision = lines[1].split('\t').next().unwrap().to_owned();
        for line in lines {
            let revision = line.split('\t').next().unwrap();
            assert!(RevisionId::from_user_string(revision).is_ok());
        }
        for secret in [&password, &updated_password] {
            assert!(!history
                .stdout()
                .contains(core::str::from_utf8(secret).unwrap()));
        }

        let delete_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let deleted = run(["item", "delete", expected_id.as_str()], &delete_host);
        assert_eq!(deleted.exit_code(), ExitCode::Success, "{deleted:?}");
        assert_eq!(deleted.stdout(), format!("Item deleted: {expected_id}\n"));

        let deleted_show_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let deleted_show = run(["item", "show", expected_id.as_str()], &deleted_show_host);
        assert_eq!(deleted_show.exit_code(), ExitCode::NotFound);

        let repeated_delete_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let repeated_delete = run(
            ["item", "delete", expected_id.as_str()],
            &repeated_delete_host,
        );
        assert_eq!(repeated_delete.exit_code(), ExitCode::NotFound);
        assert!(repeated_delete.stdout().is_empty());

        let deleted_history_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let deleted_history = run(
            ["history", "list", expected_id.as_str()],
            &deleted_history_host,
        );
        assert_eq!(deleted_history.exit_code(), ExitCode::Success);
        let deleted_lines = deleted_history.stdout().lines().collect::<Vec<_>>();
        assert_eq!(deleted_lines.len(), 3);
        assert!(deleted_lines[0].contains("\tdeleted\tparents=1\tdeleted="));
        let tombstone_revision = deleted_lines[0].split('\t').next().unwrap();

        let tombstone_restore_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let tombstone_restore = run(
            [
                "history",
                "restore",
                expected_id.as_str(),
                tombstone_revision,
            ],
            &tombstone_restore_host,
        );
        assert_eq!(tombstone_restore.exit_code(), ExitCode::InvalidInput);
        assert!(tombstone_restore.stdout().is_empty());

        let restore_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let restored = run(
            [
                "history",
                "restore",
                expected_id.as_str(),
                original_revision.as_str(),
            ],
            &restore_host,
        );
        assert_eq!(restored.exit_code(), ExitCode::Success, "{restored:?}");
        assert_eq!(restored.stdout(), format!("Item restored: {expected_id}\n"));

        let restored_show_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let restored_show = run(["item", "show", expected_id.as_str()], &restored_show_host);
        assert_eq!(restored_show.exit_code(), ExitCode::Success);
        assert!(restored_show
            .stdout()
            .contains("Title: \"Example account\""));
        assert!(restored_show.stdout().contains("Password: <redacted>"));
        for secret in [&password, &updated_password] {
            assert!(!restored_show
                .stdout()
                .contains(core::str::from_utf8(secret).unwrap()));
        }

        let audit_host = TestHost::new(paths, [passphrase]);
        let audit = run(["audit", "verify"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert!(audit.stdout().contains("revisions=4 items=1"));
    }

    #[test]
    fn secure_note_create_list_show_and_audit_never_render_the_body() {
        assert_eq!(
            parse(["item", "add", "secure-note"]),
            Ok(Command::ItemAddSecureNote)
        );
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"secure note passphrase".to_vec();
        let body = b"recovery phrase that must remain hidden".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);
        activate_test_audit_epoch(&paths, passphrase.clone());

        let add_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), body.clone()],
            ["Recovery note".to_owned()],
        );
        let added = run(["item", "add", "secure-note"], &add_host);
        assert_eq!(added.exit_code(), ExitCode::Success, "{added:?}");
        let item_id =
            ItemId::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]).to_user_string();
        assert_eq!(added.stdout(), format!("Item added: {item_id}\n"));
        assert!(!added.stdout().contains("recovery phrase"));

        let list_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let listed = run(["item", "list"], &list_host);
        assert_eq!(listed.exit_code(), ExitCode::Success, "{listed:?}");
        assert_eq!(
            listed.stdout(),
            format!("{item_id}\t{SECURE_NOTE_V1}\t\"Recovery note\"\n")
        );

        let show_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let shown = run(["item", "show", item_id.as_str()], &show_host);
        assert_eq!(shown.exit_code(), ExitCode::Success, "{shown:?}");
        assert_eq!(
            shown.stdout(),
            format!(
                "Item: {item_id}\nType: {SECURE_NOTE_V1}\nTitle: \"Recovery note\"\nBody: <redacted>\nFavorite: no\nUpdated: 1700000000000\n"
            )
        );
        assert!(!shown
            .stdout()
            .contains(core::str::from_utf8(&body).unwrap()));

        let audit_host = TestHost::with_entropy_seed(paths, [passphrase], 2);
        let audit = run(["audit", "list"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert!(
            audit.stdout().contains(&format!(
                "action=item_create\toutcome=succeeded\ttime=1700000000000\titem={item_id}"
            )),
            "{audit:?}"
        );
        assert!(!audit.stdout().contains("Recovery note"));
        assert!(!audit
            .stdout()
            .contains(core::str::from_utf8(&body).unwrap()));
    }

    #[test]
    fn item_reads_fail_closed_for_missing_items_and_wrong_passphrases() {
        let root = TestRoot::new();
        let paths = root.paths();
        let init_host = TestHost::new(paths.clone(), [b"correct passphrase".to_vec()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let empty = TestHost::new(paths.clone(), [b"correct passphrase".to_vec()]);
        let list = run(["item", "list"], &empty);
        assert_eq!(list.exit_code(), ExitCode::Success);
        assert_eq!(list.stdout(), "No items.\n");

        let wrong = TestHost::new(paths.clone(), [b"wrong passphrase".to_vec()]);
        let list = run(["item", "list"], &wrong);
        assert_eq!(list.exit_code(), ExitCode::Locked);
        assert!(list.stdout().is_empty());

        let missing_id = ItemId::new([0x55; 16]).to_user_string();
        let wrong = TestHost::new(paths.clone(), [b"wrong passphrase".to_vec()]);
        let edit = run(["item", "edit", missing_id.as_str()], &wrong);
        assert_eq!(edit.exit_code(), ExitCode::Locked);
        assert!(edit.stdout().is_empty());

        let wrong = TestHost::new(paths.clone(), [b"wrong passphrase".to_vec()]);
        let history = run(["history", "list", missing_id.as_str()], &wrong);
        assert_eq!(history.exit_code(), ExitCode::Locked);
        assert!(history.stdout().is_empty());

        let wrong = TestHost::new(paths.clone(), [b"wrong passphrase".to_vec()]);
        let delete = run(["item", "delete", missing_id.as_str()], &wrong);
        assert_eq!(delete.exit_code(), ExitCode::Locked);
        assert!(delete.stdout().is_empty());

        let missing_revision = RevisionId::new([0x56; 32]).to_user_string();
        let wrong = TestHost::new(paths.clone(), [b"wrong passphrase".to_vec()]);
        let restore = run(
            [
                "history",
                "restore",
                missing_id.as_str(),
                missing_revision.as_str(),
            ],
            &wrong,
        );
        assert_eq!(restore.exit_code(), ExitCode::Locked);
        assert!(restore.stdout().is_empty());

        let correct = TestHost::new(paths, [b"correct passphrase".to_vec()]);
        let show = run(["item", "show", missing_id.as_str()], &correct);
        assert_eq!(show.exit_code(), ExitCode::NotFound);
        assert_eq!(show.stderr(), "vault-pm: not found\n");

        let correct = TestHost::new(root.paths(), [b"correct passphrase".to_vec()]);
        let edit = run(["item", "edit", missing_id.as_str()], &correct);
        assert_eq!(edit.exit_code(), ExitCode::NotFound);
        assert_eq!(edit.stderr(), "vault-pm: not found\n");

        let correct = TestHost::new(root.paths(), [b"correct passphrase".to_vec()]);
        let history = run(["history", "list", missing_id.as_str()], &correct);
        assert_eq!(history.exit_code(), ExitCode::NotFound);
        assert_eq!(history.stderr(), "vault-pm: not found\n");

        let correct = TestHost::new(root.paths(), [b"correct passphrase".to_vec()]);
        let delete = run(["item", "delete", missing_id.as_str()], &correct);
        assert_eq!(delete.exit_code(), ExitCode::NotFound);
        assert_eq!(delete.stderr(), "vault-pm: not found\n");

        let correct = TestHost::new(root.paths(), [b"correct passphrase".to_vec()]);
        let restore = run(
            [
                "history",
                "restore",
                missing_id.as_str(),
                missing_revision.as_str(),
            ],
            &correct,
        );
        assert_eq!(restore.exit_code(), ExitCode::NotFound);
        assert_eq!(restore.stderr(), "vault-pm: not found\n");
    }

    #[test]
    fn authenticated_verification_rejects_the_wrong_passphrase() {
        let root = TestRoot::new();
        let paths = root.paths();
        let init_host = TestHost::new(paths.clone(), [b"correct passphrase".to_vec()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let wrong = TestHost::new(paths, [b"wrong passphrase".to_vec()]);
        let output = run(["audit", "verify"], &wrong);
        assert_eq!(output.exit_code(), ExitCode::Locked);
        assert!(output.stdout().is_empty());
        assert_eq!(output.stderr(), "vault-pm: authentication required\n");
    }

    #[test]
    fn authenticated_verification_fails_closed_on_repository_tampering() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"correct passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let object = first_storage_record_with_body_magic(paths.object_root(), b"VPO1")
            .expect("generation zero encrypted repository object");
        let mut bytes = fs::read(&object).unwrap();
        let last = bytes.last_mut().expect("non-empty storage record");
        *last ^= 0x01;
        fs::write(object, bytes).unwrap();

        let audit_host = TestHost::new(paths, [passphrase]);
        let output = run(["audit", "verify"], &audit_host);
        assert_eq!(output.exit_code(), ExitCode::Integrity, "{output:?}");
        assert!(output.stdout().is_empty());
        assert_eq!(output.stderr(), "vault-pm: integrity check failed\n");
    }

    #[test]
    fn audit_history_fails_closed_without_output_on_repository_tampering() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"audit history tamper passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);
        activate_test_audit_epoch(&paths, passphrase.clone());

        let object = first_storage_record_with_body_magic(paths.object_root(), b"VPO1")
            .expect("encrypted repository object after audit activation");
        let mut bytes = fs::read(&object).unwrap();
        let last = bytes.last_mut().expect("non-empty storage record");
        *last ^= 0x01;
        fs::write(object, bytes).unwrap();

        let audit_host = TestHost::new(paths, [passphrase]);
        let output = run(["audit", "list"], &audit_host);
        assert_eq!(output.exit_code(), ExitCode::Integrity, "{output:?}");
        assert!(output.stdout().is_empty());
        assert_eq!(output.stderr(), "vault-pm: integrity check failed\n");
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
        let full_doctor = run(["doctor", "--unlock"], &host);
        assert_eq!(full_doctor.exit_code(), ExitCode::InvalidInput);
        assert_eq!(full_doctor.stdout(), "Doctor: initialization_required\n");
        let audit = run(["audit", "verify"], &host);
        assert_eq!(audit.exit_code(), ExitCode::InvalidInput);
        assert_eq!(audit.stderr(), "vault-pm: invalid command\n");
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
