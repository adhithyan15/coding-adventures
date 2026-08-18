//! Strict local CLI grammar, rendering, and product composition for vault-pm.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod shell;

mod crash;

/// Whether VLT-PM41 crash injection is compiled into this build.
///
/// A composition root that must never ship a kill switch can turn this into a
/// *compile* error rather than a test:
///
/// ```
/// const _: () = assert!(!coding_adventures_vault_pm_cli::CRASH_INJECTION_COMPILED);
/// ```
///
/// That matters because cargo's `--features <dep>/<feature>` syntax reaches a
/// direct dependency's features even when the root package declares none of
/// its own. Declaring no feature is therefore not enough on its own to keep
/// the instrumentation out of a product binary; refusing to compile with it is.
pub const CRASH_INJECTION_COMPILED: bool = cfg!(feature = "crash-injection");

use coding_adventures_vault_pm_application::{
    complete_generation_zero, open_portable_with_passphrase, portable_import_random_bytes,
    prepare_audited_generation_zero, rehydrate_prepared_init, AddItemRandomnessV1,
    ApiKeyConflictMergeInputV1, ApplicationError, AuditEventViewV1, AuditVerificationV1,
    AuditedAccessRandomnessV1, AuditedGenerationZeroRandomness, BootstrapLocator, BootstrapStore,
    BootstrapStoreError, CardConflictMergeInputV1, DatabaseCredentialConflictMergeInputV1,
    DeleteItemRandomnessV1, GenerationZeroPolicyV1, ItemHistoryViewV1, LocalStateStore,
    LocalStateStoreError, LocalVaultStateV1, LoginEditInputV1, OpaqueConflictMergeInputV1,
    PortableExportPolicyV1, PortableExportRandomnessV1, PortableImportRandomnessV1,
    PortableOpenPolicyV1, ReplaceItemRandomnessV1, ResolveItemConflictRandomnessV1,
    RestoreItemRandomnessV1, RevealedSecretEncodingV1, RevealedSecretV1, SecretDisclosureIntentV1,
    SecretFieldV1, SecureNoteConflictMergeInputV1, TotpConflictMergeInputV1,
    V1ApplicationRepositoryFactory, VaultAccessV1, VaultDoctorStateV1, VaultStatusStateV1,
    ADD_ITEM_RANDOM_BYTES, AUDITED_ACCESS_RANDOM_BYTES, AUDITED_GENERATION_ZERO_RANDOM_BYTES,
    DEFAULT_AUDIT_HISTORY_LIMIT, DEFAULT_ITEM_HISTORY_LIMIT, DELETE_ITEM_RANDOM_BYTES,
    MAX_PORTABLE_EXPORT_ARTIFACT_BYTES, PORTABLE_EXPORT_RANDOM_BYTES, REPLACE_ITEM_RANDOM_BYTES,
    RESOLVE_ITEM_CONFLICT_RANDOM_BYTES, RESTORE_ITEM_RANDOM_BYTES,
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
use coding_adventures_vault_records::{
    AnyRecord, ApiKey, Card, DatabaseCredential, Login, SecureNote, TotpSeed, API_KEY_V1, CARD_V1,
    DATABASE_CREDENTIAL_V1, LOGIN_V1, SECURE_NOTE_V1, TOTP_SEED_V1,
};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt::{self, Debug, Formatter};
use crash::LocalBackend;
use shell::{run_shell, NativeShellTerminal, ShellTerminal};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_VAULT_NAME: &str = "personal";
const DEFAULT_STORAGE_NAME: &str = "local";
const PRODUCTION_KDF_MEMORY_KIB: u32 = 64 * 1024;
const PRODUCTION_KDF_ITERATIONS: u32 = 3;
const PRODUCTION_KDF_LANES: u8 = 1;
const ITEM_OPERATION_RANDOM_BYTES: usize = 32;
const DEFAULT_SEARCH_RESULT_LIMIT: usize = 100;
const USAGE: &str = "Usage:\n  vault-pm init [--vault NAME] [--storage NAME]\n  vault-pm vault create NAME\n  vault-pm [--vault NAME] status [--json]\n  vault-pm [--vault NAME] shell\n  vault-pm [--vault NAME] audit enable\n  vault-pm [--vault NAME] audit verify\n  vault-pm [--vault NAME] audit list\n  vault-pm [--vault NAME] audit show TRACE\n  vault-pm [--vault NAME] doctor [--unlock]\n  vault-pm [--vault NAME] export FILE\n  vault-pm [--vault NAME] import FILE\n  vault-pm --vault NAME restore FILE\n  vault-pm [--vault NAME] restore verify FILE\n  vault-pm [--vault NAME] item add login\n  vault-pm [--vault NAME] item add secure-note\n  vault-pm [--vault NAME] item add card\n  vault-pm [--vault NAME] item add api-key\n  vault-pm [--vault NAME] item add database-credential\n  vault-pm [--vault NAME] item add totp\n  vault-pm [--vault NAME] item edit ITEM\n  vault-pm [--vault NAME] item delete ITEM\n  vault-pm [--vault NAME] item list\n  vault-pm [--vault NAME] item show ITEM\n  vault-pm [--vault NAME] item reveal ITEM FIELD\n  vault-pm [--vault NAME] search QUERY\n  vault-pm [--vault NAME] history list ITEM\n  vault-pm [--vault NAME] history restore ITEM REVISION\n  vault-pm [--vault NAME] conflict list ITEM\n  vault-pm [--vault NAME] conflict reveal ITEM REVISION FIELD\n  vault-pm [--vault NAME] conflict choose ITEM REVISION\n  vault-pm [--vault NAME] conflict merge login ITEM BASE_REVISION\n  vault-pm [--vault NAME] conflict merge secure-note ITEM BASE_REVISION\n  vault-pm [--vault NAME] conflict merge card ITEM BASE_REVISION\n  vault-pm [--vault NAME] conflict merge api-key ITEM BASE_REVISION\n  vault-pm [--vault NAME] conflict merge database-credential ITEM BASE_REVISION\n  vault-pm [--vault NAME] conflict merge totp ITEM BASE_REVISION\n  vault-pm [--vault NAME] conflict merge opaque ITEM BASE_REVISION\n";

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

    /// Prepend the fixed VLT-PM42 repair notice to standard error.
    ///
    /// Standard output is untouched on purpose: anything that parses a
    /// command's output must not have to learn a new line, and the exit class
    /// of a command that happened to also repair the vault is the exit class of
    /// the command.
    fn with_recovery_notice(mut self) -> Self {
        self.stderr.insert_str(0, RECOVERY_NOTICE);
        self
    }
}

/// The one sentence a repaired vault is announced with.
///
/// Fixed at compile time and payload-free: it names no vault, item, revision,
/// object, provider, path, or count.
const RECOVERY_NOTICE: &str = "vault-pm: recovered an interrupted write\n";

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

    /// Collect the number of login URLs from the controlling terminal.
    fn read_login_url_count(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect one required login URL from the controlling terminal.
    fn read_login_url(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect a login password with terminal echo disabled.
    fn read_login_password(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect optional private login notes with terminal echo disabled.
    fn read_login_notes(&self) -> Result<Option<Zeroizing<String>>, HostError>;

    /// Collect a secure-note title from the controlling terminal.
    fn read_secure_note_title(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect a secure-note body with terminal echo disabled.
    fn read_secure_note_body(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect a payment-card title from the controlling terminal.
    fn read_card_title(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect a payment-card holder name from the controlling terminal.
    fn read_card_holder(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect a payment-card number with terminal echo disabled.
    fn read_card_number(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect a payment-card expiry month from the controlling terminal.
    fn read_card_expiry_month(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect a payment-card expiry year from the controlling terminal.
    fn read_card_expiry_year(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect a payment-card verification code with terminal echo disabled.
    fn read_card_cvv(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect an optional payment-card billing postal code.
    fn read_card_billing_postal_code(&self) -> Result<Option<Zeroizing<String>>, HostError>;

    /// Collect an API-key display label from the controlling terminal.
    fn read_api_key_label(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect an API-key service name from the controlling terminal.
    fn read_api_key_service(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect an API-key token with terminal echo disabled.
    fn read_api_key_token(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect an optional comma-separated API-key scope list.
    fn read_api_key_scopes(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect an optional API-key expiry in Unix seconds.
    fn read_api_key_expiry(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect a database-credential display label.
    fn read_database_label(&self) -> Result<Zeroizing<String>, HostError>;
    /// Collect a database engine identifier.
    fn read_database_engine(&self) -> Result<Zeroizing<String>, HostError>;
    /// Collect a database host.
    fn read_database_host(&self) -> Result<Zeroizing<String>, HostError>;
    /// Collect a database TCP port.
    fn read_database_port(&self) -> Result<Zeroizing<String>, HostError>;
    /// Collect an optional database or catalog name.
    fn read_database_name(&self) -> Result<Option<Zeroizing<String>>, HostError>;
    /// Collect a database username.
    fn read_database_username(&self) -> Result<Zeroizing<String>, HostError>;
    /// Collect a database password with terminal echo disabled.
    fn read_database_password(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect a TOTP display label.
    fn read_totp_label(&self) -> Result<Zeroizing<String>, HostError>;
    /// Collect an optional TOTP issuer.
    fn read_totp_issuer(&self) -> Result<Option<Zeroizing<String>>, HostError>;
    /// Collect a TOTP seed as canonical unpadded Base32 with echo disabled.
    fn read_totp_secret(&self) -> Result<Zeroizing<String>, HostError>;
    /// Collect a TOTP HMAC algorithm.
    fn read_totp_algorithm(&self) -> Result<Zeroizing<String>, HostError>;
    /// Collect a TOTP output digit count.
    fn read_totp_digits(&self) -> Result<Zeroizing<String>, HostError>;
    /// Collect a TOTP period in seconds.
    fn read_totp_period(&self) -> Result<Zeroizing<String>, HostError>;

    /// Collect an opaque record's whole canonical-CBOR payload as lowercase
    /// hexadecimal, with terminal echo disabled.
    fn read_opaque_payload(&self) -> Result<Zeroizing<String>, HostError>;

    /// Require explicit interactive confirmation before revealing a secret.
    fn confirm_secret_reveal(&self) -> Result<bool, HostError>;

    /// Deliver one audited UTF-8 secret directly to the controlling terminal.
    fn write_revealed_text(&self, value: &str) -> Result<(), HostError>;

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

    fn read_login_url_count(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::LoginUrlCount)
            .map_err(map_native_cli_host)
    }

    fn read_login_url(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::LoginUrl)
            .map_err(map_native_cli_host)
    }

    fn read_login_password(&self) -> Result<Zeroizing<String>, HostError> {
        self.read_utf8_secret(SecretPrompt::LoginPassword)
    }

    fn read_login_notes(&self) -> Result<Option<Zeroizing<String>>, HostError> {
        ControllingTerminal
            .read_optional_login_notes()
            .map_err(map_native_cli_host)?
            .map(|value| {
                core::str::from_utf8(&value)
                    .map(|text| Zeroizing::new(text.to_owned()))
                    .map_err(|_| HostError::Invalid)
            })
            .transpose()
    }

    fn read_secure_note_title(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::SecureNoteTitle)
            .map_err(map_native_cli_host)
    }

    fn read_secure_note_body(&self) -> Result<Zeroizing<String>, HostError> {
        self.read_utf8_secret(SecretPrompt::SecureNoteBody)
    }

    fn read_card_title(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::CardTitle)
            .map_err(map_native_cli_host)
    }

    fn read_card_holder(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::CardHolder)
            .map_err(map_native_cli_host)
    }

    fn read_card_number(&self) -> Result<Zeroizing<String>, HostError> {
        self.read_utf8_secret(SecretPrompt::CardNumber)
    }

    fn read_card_expiry_month(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::CardExpiryMonth)
            .map_err(map_native_cli_host)
    }

    fn read_card_expiry_year(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::CardExpiryYear)
            .map_err(map_native_cli_host)
    }

    fn read_card_cvv(&self) -> Result<Zeroizing<String>, HostError> {
        self.read_utf8_secret(SecretPrompt::CardCvv)
    }

    fn read_card_billing_postal_code(&self) -> Result<Option<Zeroizing<String>>, HostError> {
        let value = ControllingTerminal
            .read_text(TextPrompt::CardBillingPostalCode)
            .map_err(map_native_cli_host)?;
        Ok((!value.is_empty()).then_some(value))
    }

    fn read_api_key_label(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::ApiKeyLabel)
            .map_err(map_native_cli_host)
    }

    fn read_api_key_service(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::ApiKeyService)
            .map_err(map_native_cli_host)
    }

    fn read_api_key_token(&self) -> Result<Zeroizing<String>, HostError> {
        self.read_utf8_secret(SecretPrompt::ApiKeyToken)
    }

    fn read_api_key_scopes(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::ApiKeyScopes)
            .map_err(map_native_cli_host)
    }

    fn read_api_key_expiry(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::ApiKeyExpiry)
            .map_err(map_native_cli_host)
    }

    fn read_database_label(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::DatabaseLabel)
            .map_err(map_native_cli_host)
    }
    fn read_database_engine(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::DatabaseEngine)
            .map_err(map_native_cli_host)
    }
    fn read_database_host(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::DatabaseHost)
            .map_err(map_native_cli_host)
    }
    fn read_database_port(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::DatabasePort)
            .map_err(map_native_cli_host)
    }
    fn read_database_name(&self) -> Result<Option<Zeroizing<String>>, HostError> {
        let value = ControllingTerminal
            .read_text(TextPrompt::DatabaseName)
            .map_err(map_native_cli_host)?;
        Ok((!value.is_empty()).then_some(value))
    }
    fn read_database_username(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::DatabaseUsername)
            .map_err(map_native_cli_host)
    }
    fn read_database_password(&self) -> Result<Zeroizing<String>, HostError> {
        self.read_utf8_secret(SecretPrompt::DatabasePassword)
    }

    fn read_totp_label(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::TotpLabel)
            .map_err(map_native_cli_host)
    }
    fn read_totp_issuer(&self) -> Result<Option<Zeroizing<String>>, HostError> {
        let value = ControllingTerminal
            .read_text(TextPrompt::TotpIssuer)
            .map_err(map_native_cli_host)?;
        Ok((!value.is_empty()).then_some(value))
    }
    fn read_totp_secret(&self) -> Result<Zeroizing<String>, HostError> {
        self.read_utf8_secret(SecretPrompt::TotpSecret)
    }
    fn read_totp_algorithm(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::TotpAlgorithm)
            .map_err(map_native_cli_host)
    }
    fn read_totp_digits(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::TotpDigits)
            .map_err(map_native_cli_host)
    }
    fn read_totp_period(&self) -> Result<Zeroizing<String>, HostError> {
        ControllingTerminal
            .read_text(TextPrompt::TotpPeriod)
            .map_err(map_native_cli_host)
    }

    fn read_opaque_payload(&self) -> Result<Zeroizing<String>, HostError> {
        self.read_utf8_secret(SecretPrompt::OpaquePayload)
    }

    fn confirm_secret_reveal(&self) -> Result<bool, HostError> {
        ControllingTerminal
            .confirm_secret_reveal()
            .map_err(map_native_cli_host)
    }

    fn write_revealed_text(&self, value: &str) -> Result<(), HostError> {
        ControllingTerminal
            .write_revealed_text(value)
            .map_err(map_native_cli_host)
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
    run_with_terminal(arguments, host, &NativeShellTerminal)
}

/// Parse and execute one argument vector with an injected shell terminal.
///
/// Identical to [`run`] for every command except `shell`, which needs a
/// terminal to read command lines from and render results to. Tests use this
/// entry point to drive a scripted session; the executable uses [`run`], whose
/// terminal is the real controlling terminal.
pub fn run_with_terminal<I, S>(
    arguments: I,
    host: &dyn CliHost,
    terminal: &dyn ShellTerminal,
) -> CliOutput
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let invocation = match parse(arguments) {
        Ok(invocation) => invocation,
        Err(error) => return CliOutput::failure(error),
    };
    if matches!(invocation.command, Command::Help) {
        return CliOutput::success(USAGE);
    }
    // The shell is dispatched before `execute` because `execute` acquires the
    // cross-process writer lock for the whole command. A shell must not hold
    // that lock while it waits at a prompt, and its own per-command
    // invocations acquire it one at a time.
    if matches!(invocation.command, Command::Shell) {
        return match run_shell(host, terminal, invocation.selected_vault) {
            Ok(output) => output,
            Err(error) => CliOutput::failure(error),
        };
    }
    match execute(invocation, host) {
        Ok(output) => output,
        Err(error) => CliOutput::failure(error),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Invocation {
    selected_vault: Option<ConfigName>,
    command: Command,
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Init {
        vault: ConfigName,
        storage: ConfigName,
    },
    VaultCreate {
        vault: ConfigName,
    },
    Status {
        json: bool,
    },
    /// Begin one foreground interactive session over this same grammar.
    Shell,
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
    PortableRestore {
        source: PathBuf,
    },
    PortableRestoreVerify {
        source: PathBuf,
    },
    ItemAddLogin,
    ItemAddSecureNote,
    ItemAddCard,
    ItemAddApiKey,
    ItemAddDatabaseCredential,
    ItemAddTotp,
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
    ItemReveal {
        item_id: ItemId,
        field: SecretFieldV1,
    },
    Search {
        query: SearchQuery,
    },
    HistoryList {
        item_id: ItemId,
    },
    HistoryRestore {
        item_id: ItemId,
        revision_id: RevisionId,
    },
    ConflictList {
        item_id: ItemId,
    },
    ConflictReveal {
        item_id: ItemId,
        revision_id: RevisionId,
        field: SecretFieldV1,
    },
    ConflictChoose {
        item_id: ItemId,
        revision_id: RevisionId,
    },
    ConflictMergeLogin {
        item_id: ItemId,
        base_revision: RevisionId,
    },
    ConflictMergeSecureNote {
        item_id: ItemId,
        base_revision: RevisionId,
    },
    ConflictMergeCard {
        item_id: ItemId,
        base_revision: RevisionId,
    },
    ConflictMergeApiKey {
        item_id: ItemId,
        base_revision: RevisionId,
    },
    ConflictMergeDatabaseCredential {
        item_id: ItemId,
        base_revision: RevisionId,
    },
    ConflictMergeTotp {
        item_id: ItemId,
        base_revision: RevisionId,
    },
    ConflictMergeOpaque {
        item_id: ItemId,
        base_revision: RevisionId,
    },
    Help,
}

struct SearchQuery(Zeroizing<String>);

impl SearchQuery {
    fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }

    fn into_zeroizing(self) -> Zeroizing<String> {
        self.0
    }
}

impl Debug for SearchQuery {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("SearchQuery(<redacted>)")
    }
}

impl PartialEq for SearchQuery {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Eq for SearchQuery {}

fn parse<I, S>(arguments: I) -> Result<Invocation, CliFailure>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut values = Vec::new();
    for value in arguments {
        match value.into().into_string() {
            Ok(value) => values.push(value),
            Err(_) => {
                for value in &mut values {
                    value.zeroize();
                }
                return Err(CliFailure::InvalidCommand);
            }
        }
    }
    let command_index = usize::from(values.first().is_some_and(|value| value == "--vault")) * 2;
    if values
        .get(command_index)
        .is_some_and(|name| name == "search")
    {
        let command = parse_search(&mut values, command_index)?;
        let selected_vault = if command_index == 2 {
            values
                .get(1)
                .cloned()
                .map(ConfigName::new)
                .transpose()
                .map_err(|_| CliFailure::InvalidCommand)?
        } else {
            None
        };
        return Ok(Invocation {
            selected_vault,
            command,
        });
    }
    let Some(name) = values.get(command_index).map(String::as_str) else {
        return Err(CliFailure::InvalidCommand);
    };
    let selected_vault = if command_index == 2 {
        Some(ConfigName::new(values[1].clone()).map_err(|_| CliFailure::InvalidCommand)?)
    } else {
        None
    };
    let tail = &values[command_index + 1..];
    let command = match name {
        "--help" | "-h" | "help" if tail.is_empty() => Ok(Command::Help),
        "init" => parse_init(tail),
        "vault" => parse_vault(tail),
        "status" => parse_status(tail),
        "shell" if tail.is_empty() => Ok(Command::Shell),
        "audit" => parse_audit(tail),
        "doctor" => parse_doctor(tail),
        "export" => parse_export(tail),
        "import" => parse_import(tail),
        "restore" => parse_restore(tail),
        "item" => parse_item(tail),
        "history" => parse_history(tail),
        "conflict" => parse_conflict(tail),
        _ => Err(CliFailure::InvalidCommand),
    }?;
    if selected_vault.is_some()
        && matches!(
            &command,
            Command::Init { .. } | Command::VaultCreate { .. } | Command::Help
        )
    {
        return Err(CliFailure::InvalidCommand);
    }
    if matches!(&command, Command::PortableRestore { .. }) && selected_vault.is_none() {
        return Err(CliFailure::InvalidCommand);
    }
    Ok(Invocation {
        selected_vault,
        command,
    })
}

fn parse_search(values: &mut [String], command_index: usize) -> Result<Command, CliFailure> {
    let query_index = command_index + 1;
    if values.len() != query_index + 1 || values[query_index].starts_with('-') {
        for value in values.iter_mut().skip(query_index) {
            value.zeroize();
        }
        return Err(CliFailure::InvalidCommand);
    }
    let query = core::mem::take(&mut values[query_index]);
    Ok(Command::Search {
        query: SearchQuery::new(query),
    })
}

fn parse_vault(arguments: &[String]) -> Result<Command, CliFailure> {
    match arguments {
        [action, name] if action == "create" => Ok(Command::VaultCreate {
            vault: ConfigName::new(name.clone()).map_err(|_| CliFailure::InvalidCommand)?,
        }),
        _ => Err(CliFailure::InvalidCommand),
    }
}

fn parse_restore(arguments: &[String]) -> Result<Command, CliFailure> {
    match arguments {
        [source] if !source.is_empty() => Ok(Command::PortableRestore {
            source: PathBuf::from(source),
        }),
        [action, source] if action == "verify" && !source.is_empty() => {
            Ok(Command::PortableRestoreVerify {
                source: PathBuf::from(source),
            })
        }
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

fn parse_conflict(arguments: &[String]) -> Result<Command, CliFailure> {
    match arguments {
        [action, item] if action == "list" => Ok(Command::ConflictList {
            item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
        }),
        [action, item, revision, field] if action == "reveal" => Ok(Command::ConflictReveal {
            item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
            revision_id: RevisionId::from_user_string(revision)
                .map_err(|_| CliFailure::InvalidCommand)?,
            field: parse_secret_field(field)?,
        }),
        [action, item, revision] if action == "choose" => Ok(Command::ConflictChoose {
            item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
            revision_id: RevisionId::from_user_string(revision)
                .map_err(|_| CliFailure::InvalidCommand)?,
        }),
        [action, kind, item, revision] if action == "merge" && kind == "login" => {
            Ok(Command::ConflictMergeLogin {
                item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
                base_revision: RevisionId::from_user_string(revision)
                    .map_err(|_| CliFailure::InvalidCommand)?,
            })
        }
        [action, kind, item, revision] if action == "merge" && kind == "secure-note" => {
            Ok(Command::ConflictMergeSecureNote {
                item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
                base_revision: RevisionId::from_user_string(revision)
                    .map_err(|_| CliFailure::InvalidCommand)?,
            })
        }
        [action, kind, item, revision] if action == "merge" && kind == "card" => {
            Ok(Command::ConflictMergeCard {
                item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
                base_revision: RevisionId::from_user_string(revision)
                    .map_err(|_| CliFailure::InvalidCommand)?,
            })
        }
        [action, kind, item, revision] if action == "merge" && kind == "api-key" => {
            Ok(Command::ConflictMergeApiKey {
                item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
                base_revision: RevisionId::from_user_string(revision)
                    .map_err(|_| CliFailure::InvalidCommand)?,
            })
        }
        [action, kind, item, revision] if action == "merge" && kind == "database-credential" => {
            Ok(Command::ConflictMergeDatabaseCredential {
                item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
                base_revision: RevisionId::from_user_string(revision)
                    .map_err(|_| CliFailure::InvalidCommand)?,
            })
        }
        [action, kind, item, revision] if action == "merge" && kind == "totp" => {
            Ok(Command::ConflictMergeTotp {
                item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
                base_revision: RevisionId::from_user_string(revision)
                    .map_err(|_| CliFailure::InvalidCommand)?,
            })
        }
        [action, kind, item, revision] if action == "merge" && kind == "opaque" => {
            Ok(Command::ConflictMergeOpaque {
                item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
                base_revision: RevisionId::from_user_string(revision)
                    .map_err(|_| CliFailure::InvalidCommand)?,
            })
        }
        _ => Err(CliFailure::InvalidCommand),
    }
}

fn parse_item(arguments: &[String]) -> Result<Command, CliFailure> {
    match arguments {
        [action, kind] if action == "add" && kind == "login" => Ok(Command::ItemAddLogin),
        [action, kind] if action == "add" && kind == "secure-note" => {
            Ok(Command::ItemAddSecureNote)
        }
        [action, kind] if action == "add" && kind == "card" => Ok(Command::ItemAddCard),
        [action, kind] if action == "add" && kind == "api-key" => Ok(Command::ItemAddApiKey),
        [action, kind] if action == "add" && kind == "database-credential" => {
            Ok(Command::ItemAddDatabaseCredential)
        }
        [action, kind] if action == "add" && kind == "totp" => Ok(Command::ItemAddTotp),
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
        [action, item, field] if action == "reveal" => Ok(Command::ItemReveal {
            item_id: ItemId::from_user_string(item).map_err(|_| CliFailure::InvalidCommand)?,
            field: parse_secret_field(field)?,
        }),
        _ => Err(CliFailure::InvalidCommand),
    }
}

fn parse_secret_field(value: &str) -> Result<SecretFieldV1, CliFailure> {
    match value {
        "login-password" => Ok(SecretFieldV1::LoginPassword),
        "login-notes" => Ok(SecretFieldV1::LoginNotes),
        "secure-note-body" => Ok(SecretFieldV1::SecureNoteBody),
        "card-number" => Ok(SecretFieldV1::CardNumber),
        "card-cvv" => Ok(SecretFieldV1::CardCvv),
        "api-key-token" => Ok(SecretFieldV1::ApiKeyToken),
        "database-password" => Ok(SecretFieldV1::DatabasePassword),
        "totp-secret" => Ok(SecretFieldV1::TotpSecret),
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

fn execute(invocation: Invocation, host: &dyn CliHost) -> Result<CliOutput, CliFailure> {
    let paths = host.paths().map_err(map_host)?;
    let prepared = paths.prepare().map_err(map_local_host)?;
    let writer = prepared.try_acquire_writer().map_err(map_local_host)?;
    let Invocation {
        selected_vault,
        command,
    } = invocation;
    let selected_vault = selected_vault.as_ref();

    // VLT-PM42 §6. A repair nobody mentions is indistinguishable from no
    // repair, so the composition root watches the durable lifecycle state
    // across the command it is about to run. Both reads happen inside the
    // cross-process writer lock the command already holds, which is what makes
    // the inference sound: no other local writer can move the state between
    // them, so `RecoveryRequired` before and anything else after means *this*
    // command finished an interrupted publication.
    let before = observed_vault_state(prepared.paths(), &writer, selected_vault);
    let result = dispatch(command, host, prepared.paths(), &writer, selected_vault);
    let after = observed_vault_state(prepared.paths(), &writer, selected_vault);
    if !observed_a_repair(before, after) {
        return result;
    }
    // The notice is attached to a failed command too. A repair that happened
    // is worth saying even when the verb that triggered it went on to report
    // `not found`, and rendering the failure here produces exactly the output
    // the caller would have rendered from the error.
    Ok(match result {
        Ok(output) => output.with_recovery_notice(),
        Err(error) => CliOutput::failure(error).with_recovery_notice(),
    })
}

/// Whether two observations prove *this* command finished an interrupted
/// publication.
///
/// The whole truth table, because the interesting cases are the ones that must
/// stay quiet:
///
/// | before | after | repair announced | why |
/// |---|---|---|---|
/// | `RecoveryRequired` | anything else, observed | **yes** | only this command held the writer lock, so only this command can have moved it |
/// | `RecoveryRequired` | `RecoveryRequired` | no | still wedged; nothing was finished |
/// | `RecoveryRequired` | `None` | no | the observation could not be taken, which is not evidence of anything |
/// | anything else | anything | no | there was nothing to repair |
///
/// The third row is the one worth stating out loud. `None` means the state
/// could not be read — an owner-state file that just became unreadable, a store
/// that went away mid-command — and `None != Some(RecoveryRequired)` is
/// perfectly true while proving nothing. Reading it as "no longer recovery
/// required" would announce a repair on a vault that is still wedged, which is
/// precisely the false claim [`observed_vault_state`] exists to avoid. Both
/// ends therefore fail toward silence.
const fn observed_a_repair(
    before: Option<VaultStatusStateV1>,
    after: Option<VaultStatusStateV1>,
) -> bool {
    matches!(before, Some(VaultStatusStateV1::RecoveryRequired))
        && matches!(after, Some(state) if !matches!(state, VaultStatusStateV1::RecoveryRequired))
}

/// Read the durable lifecycle state of the vault this invocation targets.
///
/// Deliberately total, and deliberately silent about why it failed: this
/// observation exists only to decide whether one sentence is added to standard
/// error, so every difficulty — no configuration yet, a selector naming a vault
/// that does not exist, an unreadable owner-state file — must degrade to
/// `None`, "say nothing", rather than change what the command itself reports.
///
/// `None` therefore biases the notice toward silence. That is the safe
/// direction: a missing notice loses a courtesy, while a wrong one would be a
/// false claim about a person's vault. The caller must honour that bias in
/// *both* observations — see [`execute`], where an unobservable after-state is
/// treated as "say nothing" rather than as "no longer recovery required".
fn observed_vault_state(
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
) -> Option<VaultStatusStateV1> {
    let exact_config = writer.load_config().ok()??;
    let config = decode_config(&exact_config).ok()?;
    let vault = configured_vault(paths, &config, selected_vault).ok()?;
    let locator = application_locator(vault.locator());
    let application_store = application_store(paths);
    VaultAccessV1::locked(locator)
        .status(&application_store)
        .ok()
        .map(|report| report.state())
}

fn dispatch(
    command: Command,
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
) -> Result<CliOutput, CliFailure> {
    match command {
        Command::Init { vault, storage } => init(host, paths, writer, vault, storage),
        Command::VaultCreate { vault } => vault_create(host, paths, writer, vault),
        Command::Status { json } => status(paths, writer, selected_vault, json),
        Command::AuditEnable => audit_enable(host, paths, writer, selected_vault),
        Command::AuditVerify => audit_verify(host, paths, writer, selected_vault),
        Command::AuditList => audit_list(host, paths, writer, selected_vault),
        Command::AuditShow { trace_id } => {
            audit_show(host, paths, writer, selected_vault, trace_id)
        }
        Command::Doctor { unlock } => doctor(host, paths, writer, selected_vault, unlock),
        Command::PortableExport { destination } => {
            portable_export(host, paths, writer, selected_vault, &destination)
        }
        Command::PortableImport { source } => {
            portable_import(host, paths, writer, selected_vault, &source)
        }
        Command::PortableRestore { source } => {
            portable_restore(host, paths, writer, selected_vault, &source)
        }
        Command::PortableRestoreVerify { source } => {
            portable_restore_verify(host, paths, writer, selected_vault, &source)
        }
        Command::ItemAddLogin => item_add_login(host, paths, writer, selected_vault),
        Command::ItemAddSecureNote => item_add_secure_note(host, paths, writer, selected_vault),
        Command::ItemAddCard => item_add_card(host, paths, writer, selected_vault),
        Command::ItemAddApiKey => item_add_api_key(host, paths, writer, selected_vault),
        Command::ItemAddDatabaseCredential => {
            item_add_database_credential(host, paths, writer, selected_vault)
        }
        Command::ItemAddTotp => item_add_totp(host, paths, writer, selected_vault),
        Command::ItemEdit { item_id } => {
            item_edit_login(host, paths, writer, selected_vault, item_id)
        }
        Command::ItemDelete { item_id } => {
            item_delete(host, paths, writer, selected_vault, item_id)
        }
        Command::ItemList => item_list(host, paths, writer, selected_vault),
        Command::ItemShow { item_id } => item_show(host, paths, writer, selected_vault, item_id),
        Command::ItemReveal { item_id, field } => {
            item_reveal(host, paths, writer, selected_vault, item_id, field)
        }
        Command::Search { query } => item_search(host, paths, writer, selected_vault, query),
        Command::HistoryList { item_id } => {
            history_list(host, paths, writer, selected_vault, item_id)
        }
        Command::HistoryRestore {
            item_id,
            revision_id,
        } => history_restore(host, paths, writer, selected_vault, item_id, revision_id),
        Command::ConflictList { item_id } => {
            conflict_list(host, paths, writer, selected_vault, item_id)
        }
        Command::ConflictReveal {
            item_id,
            revision_id,
            field,
        } => conflict_reveal(
            host,
            paths,
            writer,
            selected_vault,
            item_id,
            revision_id,
            field,
        ),
        Command::ConflictChoose {
            item_id,
            revision_id,
        } => conflict_choose(host, paths, writer, selected_vault, item_id, revision_id),
        Command::ConflictMergeLogin {
            item_id,
            base_revision,
        } => conflict_merge_login(host, paths, writer, selected_vault, item_id, base_revision),
        Command::ConflictMergeSecureNote {
            item_id,
            base_revision,
        } => {
            conflict_merge_secure_note(host, paths, writer, selected_vault, item_id, base_revision)
        }
        Command::ConflictMergeCard {
            item_id,
            base_revision,
        } => conflict_merge_card(host, paths, writer, selected_vault, item_id, base_revision),
        Command::ConflictMergeApiKey {
            item_id,
            base_revision,
        } => conflict_merge_api_key(host, paths, writer, selected_vault, item_id, base_revision),
        Command::ConflictMergeDatabaseCredential {
            item_id,
            base_revision,
        } => conflict_merge_database_credential(
            host,
            paths,
            writer,
            selected_vault,
            item_id,
            base_revision,
        ),
        Command::ConflictMergeTotp {
            item_id,
            base_revision,
        } => conflict_merge_totp(host, paths, writer, selected_vault, item_id, base_revision),
        Command::ConflictMergeOpaque {
            item_id,
            base_revision,
        } => conflict_merge_opaque(host, paths, writer, selected_vault, item_id, base_revision),
        Command::Help | Command::Shell => {
            unreachable!("help and shell return before writer acquisition")
        }
    }
}

fn authenticated_access(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
) -> Result<(VaultAccessV1, StorageCoreApplicationStore<LocalBackend>), CliFailure> {
    let exact_config = writer
        .load_config()
        .map_err(map_local_host)?
        .ok_or(CliFailure::InvalidCommand)?;
    let config = decode_config(&exact_config)?;
    let vault = configured_vault(paths, &config, selected_vault)?;
    let locator = application_locator(vault.locator());
    let application_store = application_store(paths);
    let repository_factory = configured_repository_factory(&config, vault)?;
    let mut access = VaultAccessV1::locked(locator);
    let passphrase = host.read_existing_passphrase().map_err(map_host)?;
    // VLT-PM42. A process killed inside a mutation publication leaves an exact
    // journal that this open finishes with the passphrase it just collected.
    // Before that, every command on this path answered a crash with exit 2
    // `invalid command` — telling a person their command was wrong about a
    // vault that was intact and one replay from healthy.
    access
        .unlock_recovering_pending_publication(
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

struct PortableRestoreAuditInputs {
    import: (u64, AuditedAccessRandomnessV1),
    verify: (u64, AuditedAccessRandomnessV1),
}

fn portable_restore_audit_inputs(
    host: &dyn CliHost,
) -> Result<PortableRestoreAuditInputs, CliFailure> {
    let import_wall_time_ms = host.now_ms().map_err(map_host)?;
    let verify_wall_time_ms = host.now_ms().map_err(map_host)?;
    let mut combined = [0_u8; AUDITED_ACCESS_RANDOM_BYTES * 2];
    host.fill_entropy(&mut combined).map_err(map_host)?;
    let mut import_random = [0_u8; AUDITED_ACCESS_RANDOM_BYTES];
    let mut verify_random = [0_u8; AUDITED_ACCESS_RANDOM_BYTES];
    import_random.copy_from_slice(&combined[..AUDITED_ACCESS_RANDOM_BYTES]);
    verify_random.copy_from_slice(&combined[AUDITED_ACCESS_RANDOM_BYTES..]);
    Ok(PortableRestoreAuditInputs {
        import: (
            import_wall_time_ms,
            AuditedAccessRandomnessV1::new(import_random),
        ),
        verify: (
            verify_wall_time_ms,
            AuditedAccessRandomnessV1::new(verify_random),
        ),
    })
}

fn portable_export(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    destination: &Path,
) -> Result<CliOutput, CliFailure> {
    let exact_config = writer
        .load_config()
        .map_err(map_local_host)?
        .ok_or(CliFailure::InvalidCommand)?;
    let config = decode_config(&exact_config)?;
    let vault = configured_vault(paths, &config, selected_vault)?;
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

    let repository_factory = configured_repository_factory(&config, vault)?;
    let mut access = VaultAccessV1::locked(locator);
    let vault_passphrase = host.read_existing_passphrase().map_err(map_host)?;
    // VLT-PM42. An export must describe a settled vault, so an interrupted
    // publication is finished before the artifact is computed.
    access
        .unlock_recovering_pending_publication(
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
    crash::around_export_artifact(|| host.write_portable_export(destination, artifact.as_bytes()))
        .map_err(map_host)?;
    Ok(CliOutput::success("Portable export written.\n"))
}

struct PortableImportContext {
    access: VaultAccessV1,
    application_store: StorageCoreApplicationStore<LocalBackend>,
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
    selected_vault: Option<&ConfigName>,
    source: &Path,
) -> Result<CliOutput, CliFailure> {
    let (memory_kib, iterations, lanes) = host.portable_open_kdf();
    let open_policy =
        PortableOpenPolicyV1::new(memory_kib, iterations, lanes).map_err(map_application)?;
    let (wall_time_ms, failure_randomness) = audited_access_inputs(host)?;
    let (mut access, application_store) =
        authenticated_access(host, paths, writer, selected_vault)?;
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

struct PortableRestoreVerifyContext {
    access: VaultAccessV1,
    application_store: StorageCoreApplicationStore<LocalBackend>,
    wall_time_ms: u64,
    randomness: AuditedAccessRandomnessV1,
}

impl PortableRestoreVerifyContext {
    fn fail(self, error: CliFailure) -> Result<CliOutput, CliFailure> {
        self.access
            .into_unlocked()
            .map_err(map_application)?
            .record_audited_portable_restore_verify_host_failure(
                self.wall_time_ms,
                self.randomness,
                &self.application_store,
            )
            .map_err(map_application)?;
        Err(error)
    }

    fn complete(
        self,
        expectation: coding_adventures_vault_pm_application::PortableRestoreExpectationV1,
    ) -> Result<coding_adventures_vault_pm_application::PortableRestoreVerificationV1, CliFailure>
    {
        self.access
            .into_unlocked()
            .map_err(map_application)?
            .audited_verify_portable_restore(
                expectation,
                self.wall_time_ms,
                self.randomness,
                &self.application_store,
            )
            .map_err(map_application)?
            .into_operation()
            .map_err(map_application)
    }
}

fn portable_restore(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    source: &Path,
) -> Result<CliOutput, CliFailure> {
    let selected_vault = selected_vault.ok_or(CliFailure::InvalidCommand)?;
    let exact_config = writer
        .load_config()
        .map_err(map_local_host)?
        .ok_or(CliFailure::InvalidCommand)?;
    let config = decode_config(&exact_config)?;
    if config.default_vault() == selected_vault {
        return Err(CliFailure::InvalidCommand);
    }

    let (memory_kib, iterations, lanes) = host.portable_open_kdf();
    let open_policy =
        PortableOpenPolicyV1::new(memory_kib, iterations, lanes).map_err(map_application)?;
    let PortableRestoreAuditInputs {
        import: (import_wall_time_ms, import_failure_randomness),
        verify: (verify_wall_time_ms, verify_randomness),
    } = portable_restore_audit_inputs(host)?;
    let (mut access, application_store) =
        authenticated_access(host, paths, writer, Some(selected_vault))?;
    if !access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled()
    {
        access.lock();
        return Err(CliFailure::InvalidCommand);
    }
    let import_context = PortableImportContext {
        access,
        application_store,
        wall_time_ms: import_wall_time_ms,
        failure_randomness: import_failure_randomness,
    };
    let artifact = match host.read_portable_export(source) {
        Ok(artifact) => artifact,
        Err(error) => return import_context.fail(map_host(error)),
    };
    let passphrase = match host.read_import_passphrase() {
        Ok(passphrase) => passphrase,
        Err(error) => return import_context.fail(map_host(error)),
    };
    let snapshot = match open_portable_with_passphrase(&artifact, passphrase, open_policy) {
        Ok(snapshot) => snapshot,
        Err(error) => return import_context.fail(map_application(error)),
    };
    let expectation = match snapshot.prepare_restore_verification() {
        Ok(expectation) => expectation,
        Err(error) => return import_context.fail(map_application(error)),
    };
    let item_count = snapshot.item_count();
    let candidate_count = snapshot.candidate_count();
    let random_bytes = match portable_import_random_bytes(&snapshot) {
        Ok(count) => count,
        Err(error) => return import_context.fail(map_application(error)),
    };
    let mut random = vec![0_u8; random_bytes];
    if let Err(error) = host.fill_entropy(&mut random) {
        return import_context.fail(map_host(error));
    }
    let randomness = match PortableImportRandomnessV1::new(random, &snapshot) {
        Ok(randomness) => randomness,
        Err(error) => return import_context.fail(map_application(error)),
    };
    import_context.complete(snapshot, randomness, item_count, candidate_count)?;

    let (mut access, application_store) =
        authenticated_access(host, paths, writer, Some(selected_vault))?;
    if !access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled()
    {
        access.lock();
        return Err(CliFailure::InvalidCommand);
    }
    let report = PortableRestoreVerifyContext {
        access,
        application_store,
        wall_time_ms: verify_wall_time_ms,
        randomness: verify_randomness,
    }
    .complete(expectation)?;
    Ok(CliOutput::success(format!(
        "Portable restore completed and verified: items={} candidates={} conflicts={}.\n",
        report.item_count(),
        report.candidate_count(),
        report.conflicted_item_count(),
    )))
}

fn portable_restore_verify(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    source: &Path,
) -> Result<CliOutput, CliFailure> {
    let (memory_kib, iterations, lanes) = host.portable_open_kdf();
    let open_policy =
        PortableOpenPolicyV1::new(memory_kib, iterations, lanes).map_err(map_application)?;
    let (wall_time_ms, randomness) = audited_access_inputs(host)?;
    let (mut access, application_store) =
        authenticated_access(host, paths, writer, selected_vault)?;
    if !access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled()
    {
        access.lock();
        return Err(CliFailure::InvalidCommand);
    }
    let context = PortableRestoreVerifyContext {
        access,
        application_store,
        wall_time_ms,
        randomness,
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
    let expectation = match snapshot.prepare_restore_verification() {
        Ok(expectation) => expectation,
        Err(error) => return context.fail(map_application(error)),
    };
    let report = context.complete(expectation)?;
    Ok(CliOutput::success(format!(
        "Portable restore verified: items={} candidates={} conflicts={}.\n",
        report.item_count(),
        report.candidate_count(),
        report.conflicted_item_count(),
    )))
}

struct ItemCreateContext {
    access: VaultAccessV1,
    application_store: StorageCoreApplicationStore<LocalBackend>,
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
    selected_vault: Option<&ConfigName>,
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
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
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
    selected_vault: Option<&ConfigName>,
) -> Result<CliOutput, CliFailure> {
    let context = prepare_item_create(host, paths, writer, selected_vault)?;
    let input = match read_login_form(host) {
        Ok(input) => input,
        Err(error) => return context.fail(map_host(error)),
    };
    let document = context.document(
        LOGIN_V1,
        AnyRecord::Login(Login {
            title: input.title.into_inner(),
            username: input.username.into_inner(),
            password: input.password.into_inner(),
            urls: input.urls.into_iter().map(Zeroizing::into_inner).collect(),
            notes: input.notes.map(Zeroizing::into_inner),
        }),
    );
    let document = match document {
        Ok(document) => document,
        Err(error) => return context.fail(error),
    };
    context.complete(document)
}

struct LoginFormV1 {
    title: Zeroizing<String>,
    username: Zeroizing<String>,
    password: Zeroizing<String>,
    urls: Vec<Zeroizing<String>>,
    notes: Option<Zeroizing<String>>,
}

fn read_login_form(host: &dyn CliHost) -> Result<LoginFormV1, HostError> {
    let title = host.read_login_title()?;
    let username = host.read_login_username()?;
    let password = host.read_login_password()?;
    let count = host.read_login_url_count()?;
    let count = parse_login_url_count(&count).map_err(|_| HostError::Invalid)?;
    let mut urls = Vec::with_capacity(count);
    for _ in 0..count {
        urls.push(host.read_login_url()?);
    }
    let notes = host.read_login_notes()?;
    Ok(LoginFormV1 {
        title,
        username,
        password,
        urls,
        notes,
    })
}

fn parse_login_url_count(value: &str) -> Result<usize, CliFailure> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(CliFailure::InvalidCommand);
    }
    let count = value
        .parse::<usize>()
        .map_err(|_| CliFailure::InvalidCommand)?;
    (count <= 16 && count.to_string() == value)
        .then_some(count)
        .ok_or(CliFailure::InvalidCommand)
}

fn item_add_secure_note(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
) -> Result<CliOutput, CliFailure> {
    let context = prepare_item_create(host, paths, writer, selected_vault)?;
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

fn item_add_card(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
) -> Result<CliOutput, CliFailure> {
    let context = prepare_item_create(host, paths, writer, selected_vault)?;
    let input = (|| {
        Ok::<_, HostError>((
            host.read_card_title()?,
            host.read_card_holder()?,
            host.read_card_number()?,
            host.read_card_expiry_month()?,
            host.read_card_expiry_year()?,
            host.read_card_cvv()?,
            host.read_card_billing_postal_code()?,
        ))
    })();
    let (title, holder, number, expiry_month, expiry_year, cvv, billing_zip) = match input {
        Ok(input) => input,
        Err(error) => return context.fail(map_host(error)),
    };
    if validate_ascii_digits(&number, 8, 19).is_err() || validate_ascii_digits(&cvv, 3, 4).is_err()
    {
        return context.fail(CliFailure::InvalidCommand);
    }
    let expiry_month = match parse_card_expiry_month(&expiry_month) {
        Ok(value) => value,
        Err(error) => return context.fail(error),
    };
    let expiry_year = match parse_card_expiry_year(&expiry_year) {
        Ok(value) => value,
        Err(error) => return context.fail(error),
    };
    let document = context.document(
        CARD_V1,
        AnyRecord::Card(Card {
            title: title.into_inner(),
            holder: holder.into_inner(),
            number: number.into_inner(),
            expiry_month,
            expiry_year,
            cvv: cvv.into_inner(),
            billing_zip: billing_zip.map(Zeroizing::into_inner),
        }),
    );
    let document = match document {
        Ok(document) => document,
        Err(error) => return context.fail(error),
    };
    context.complete(document)
}

fn item_add_api_key(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
) -> Result<CliOutput, CliFailure> {
    let context = prepare_item_create(host, paths, writer, selected_vault)?;
    let input = (|| {
        Ok::<_, HostError>((
            host.read_api_key_label()?,
            host.read_api_key_service()?,
            host.read_api_key_token()?,
            host.read_api_key_scopes()?,
            host.read_api_key_expiry()?,
        ))
    })();
    let (label, service, token, scopes, expiry) = match input {
        Ok(input) => input,
        Err(error) => return context.fail(map_host(error)),
    };
    let scopes = match parse_api_key_scopes(&scopes) {
        Ok(scopes) => scopes,
        Err(error) => return context.fail(error),
    };
    let expires_at = match parse_optional_unix_seconds(&expiry) {
        Ok(expires_at) => expires_at,
        Err(error) => return context.fail(error),
    };
    let document = context.document(
        API_KEY_V1,
        AnyRecord::ApiKey(ApiKey {
            label: label.into_inner(),
            service: service.into_inner(),
            token: token.into_inner(),
            scopes,
            expires_at,
        }),
    );
    let document = match document {
        Ok(document) => document,
        Err(error) => return context.fail(error),
    };
    context.complete(document)
}

fn parse_api_key_scopes(value: &str) -> Result<Vec<String>, CliFailure> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    let mut seen = BTreeSet::new();
    let mut scopes = Vec::new();
    for scope in value.split(',') {
        if scopes.len() == 64
            || scope.is_empty()
            || scope.trim() != scope
            || scope.len() > 256
            || !seen.insert(scope)
        {
            return Err(CliFailure::InvalidCommand);
        }
        scopes.push(scope.to_owned());
    }
    Ok(scopes)
}

fn parse_optional_unix_seconds(value: &str) -> Result<Option<u64>, CliFailure> {
    if value.is_empty() {
        return Ok(None);
    }
    if value.starts_with('0') || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(CliFailure::InvalidCommand);
    }
    let seconds = value
        .parse::<u64>()
        .map_err(|_| CliFailure::InvalidCommand)?;
    (seconds != 0 && seconds.to_string() == value)
        .then_some(Some(seconds))
        .ok_or(CliFailure::InvalidCommand)
}

fn item_add_database_credential(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
) -> Result<CliOutput, CliFailure> {
    let context = prepare_item_create(host, paths, writer, selected_vault)?;
    let input = (|| {
        Ok::<_, HostError>((
            host.read_database_label()?,
            host.read_database_engine()?,
            host.read_database_host()?,
            host.read_database_port()?,
            host.read_database_name()?,
            host.read_database_username()?,
            host.read_database_password()?,
        ))
    })();
    let (label, engine, database_host, port, database, username, password) = match input {
        Ok(input) => input,
        Err(error) => return context.fail(map_host(error)),
    };
    if validate_database_engine(&engine).is_err() {
        return context.fail(CliFailure::InvalidCommand);
    }
    let port = match parse_database_port(&port) {
        Ok(port) => port,
        Err(error) => return context.fail(error),
    };
    let document = context.document(
        DATABASE_CREDENTIAL_V1,
        AnyRecord::DatabaseCredential(DatabaseCredential {
            label: label.into_inner(),
            engine: engine.into_inner(),
            host: database_host.into_inner(),
            port,
            database: database.map(Zeroizing::into_inner),
            username: username.into_inner(),
            password: password.into_inner(),
            lease_id: None,
            expires_at: None,
        }),
    );
    let document = match document {
        Ok(document) => document,
        Err(error) => return context.fail(error),
    };
    context.complete(document)
}

fn validate_database_engine(value: &str) -> Result<(), CliFailure> {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return Err(CliFailure::InvalidCommand);
    };
    if value.len() > 32
        || !first.is_ascii_lowercase()
        || !bytes
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_".contains(&byte))
    {
        return Err(CliFailure::InvalidCommand);
    }
    Ok(())
}

fn parse_database_port(value: &str) -> Result<u16, CliFailure> {
    if value.starts_with('0') || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(CliFailure::InvalidCommand);
    }
    let port = value
        .parse::<u16>()
        .map_err(|_| CliFailure::InvalidCommand)?;
    (port != 0 && port.to_string() == value)
        .then_some(port)
        .ok_or(CliFailure::InvalidCommand)
}

fn item_add_totp(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
) -> Result<CliOutput, CliFailure> {
    let context = prepare_item_create(host, paths, writer, selected_vault)?;
    let input = (|| {
        Ok::<_, HostError>((
            host.read_totp_label()?,
            host.read_totp_issuer()?,
            host.read_totp_secret()?,
            host.read_totp_algorithm()?,
            host.read_totp_digits()?,
            host.read_totp_period()?,
        ))
    })();
    let (label, issuer, secret, algorithm, digits, period) = match input {
        Ok(input) => input,
        Err(error) => return context.fail(map_host(error)),
    };
    let secret = match decode_totp_base32(&secret) {
        Ok(secret) => secret,
        Err(error) => return context.fail(error),
    };
    if !matches!(algorithm.as_str(), "SHA1" | "SHA256" | "SHA512") {
        return context.fail(CliFailure::InvalidCommand);
    }
    let digits = match digits.as_str() {
        "6" => 6,
        "8" => 8,
        _ => return context.fail(CliFailure::InvalidCommand),
    };
    let period = match parse_totp_period(&period) {
        Ok(period) => period,
        Err(error) => return context.fail(error),
    };
    let document = context.document(
        TOTP_SEED_V1,
        AnyRecord::TotpSeed(TotpSeed {
            label: label.into_inner(),
            issuer: issuer.map(Zeroizing::into_inner),
            secret: secret.into_inner(),
            algorithm: algorithm.into_inner(),
            digits,
            period,
        }),
    );
    let document = match document {
        Ok(document) => document,
        Err(error) => return context.fail(error),
    };
    context.complete(document)
}

fn decode_totp_base32(value: &str) -> Result<Zeroizing<Vec<u8>>, CliFailure> {
    if value.is_empty() || value.len() > 256 {
        return Err(CliFailure::InvalidCommand);
    }
    let mut output = Zeroizing::new(Vec::with_capacity(value.len() * 5 / 8));
    let mut buffer = 0_u16;
    let mut bits = 0_u8;
    for byte in value.bytes() {
        let digit = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'2'..=b'7' => byte - b'2' + 26,
            _ => return Err(CliFailure::InvalidCommand),
        };
        buffer = (buffer << 5) | u16::from(digit);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1_u16 << bits) - 1;
        }
    }
    if output.is_empty()
        || (bits != 0 && buffer != 0)
        || encode_totp_base32(&output).as_str() != value
    {
        return Err(CliFailure::InvalidCommand);
    }
    Ok(output)
}

fn encode_totp_base32(value: &[u8]) -> Zeroizing<String> {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut output = Zeroizing::new(String::with_capacity((value.len() * 8).div_ceil(5)));
    let mut buffer = 0_u16;
    let mut bits = 0_u8;
    for byte in value {
        buffer = (buffer << 8) | u16::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            output.push(ALPHABET[((buffer >> bits) & 0x1f) as usize] as char);
            buffer &= (1_u16 << bits) - 1;
        }
    }
    if bits != 0 {
        output.push(ALPHABET[((buffer << (5 - bits)) & 0x1f) as usize] as char);
    }
    output
}

fn parse_totp_period(value: &str) -> Result<u32, CliFailure> {
    if value.starts_with('0') || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(CliFailure::InvalidCommand);
    }
    let period = value
        .parse::<u32>()
        .map_err(|_| CliFailure::InvalidCommand)?;
    (period != 0 && period <= 3_600 && period.to_string() == value)
        .then_some(period)
        .ok_or(CliFailure::InvalidCommand)
}

fn validate_ascii_digits(value: &str, min: usize, max: usize) -> Result<(), CliFailure> {
    if (min..=max).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(())
    } else {
        Err(CliFailure::InvalidCommand)
    }
}

fn parse_card_expiry_month(value: &str) -> Result<u8, CliFailure> {
    let month = value
        .parse::<u8>()
        .map_err(|_| CliFailure::InvalidCommand)?;
    if (1..=12).contains(&month) && month.to_string() == value {
        Ok(month)
    } else {
        Err(CliFailure::InvalidCommand)
    }
}

fn parse_card_expiry_year(value: &str) -> Result<u16, CliFailure> {
    if value.len() != 4 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(CliFailure::InvalidCommand);
    }
    let year = value
        .parse::<u16>()
        .map_err(|_| CliFailure::InvalidCommand)?;
    (year != 0)
        .then_some(year)
        .ok_or(CliFailure::InvalidCommand)
}

fn item_list(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
) -> Result<CliOutput, CliFailure> {
    let (mut access, application_store) =
        authenticated_access(host, paths, writer, selected_vault)?;
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
    Ok(render_item_rows(items, "No items.\n"))
}

fn item_search(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    query: SearchQuery,
) -> Result<CliOutput, CliFailure> {
    let (mut access, application_store) =
        authenticated_access(host, paths, writer, selected_vault)?;
    let query = query.into_zeroizing();
    let result = if access
        .as_unlocked()
        .map_err(map_application)?
        .audit_enabled()
    {
        let (wall_time_ms, randomness) = audited_access_inputs(host)?;
        access
            .into_unlocked()
            .map_err(map_application)?
            .audited_search_items(
                query,
                None,
                DEFAULT_SEARCH_RESULT_LIMIT,
                wall_time_ms,
                randomness,
                &application_store,
            )
            .map_err(map_application)?
            .into_operation()
    } else {
        let result = access
            .as_unlocked()
            .and_then(|session| session.search_items(query, None, DEFAULT_SEARCH_RESULT_LIMIT));
        access.lock();
        result
    };
    let items = result.map_err(map_application)?;
    Ok(render_item_rows(items, "No matches.\n"))
}

fn render_item_rows(items: Vec<RedactedItemView>, empty: &'static str) -> CliOutput {
    if items.is_empty() {
        return CliOutput::success(empty);
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
    CliOutput::success(output)
}

fn item_edit_login(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
) -> Result<CliOutput, CliFailure> {
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
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
    let input = read_login_form(host)?;
    Ok(LoginEditInputV1::new(
        input.title,
        input.username,
        input.password,
        input.urls,
        input.notes,
    ))
}

fn item_show(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
) -> Result<CliOutput, CliFailure> {
    let (mut access, application_store) =
        authenticated_access(host, paths, writer, selected_vault)?;
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

fn item_reveal(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
    field: SecretFieldV1,
) -> Result<CliOutput, CliFailure> {
    let (wall_time_ms, randomness) = audited_access_inputs(host)?;
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
    let (confirmed, confirmation_error) = match host.confirm_secret_reveal() {
        Ok(confirmed) => (confirmed, None),
        Err(error) => (false, Some(error)),
    };
    let disclosed = access
        .into_unlocked()
        .map_err(map_application)?
        .audited_reveal_current_item_field(
            item_id,
            field,
            SecretDisclosureIntentV1::InteractiveReveal { confirmed },
            wall_time_ms,
            randomness,
            &application_store,
        )
        .map_err(map_application)?
        .into_operation();
    if let Some(error) = confirmation_error {
        if !matches!(disclosed, Err(ApplicationError::InvalidInput)) {
            return Err(CliFailure::Internal);
        }
        return Err(map_host(error));
    }
    let secret = disclosed.map_err(map_application)?;
    deliver_revealed_secret(host, field, secret)?;
    Ok(CliOutput::success(""))
}

fn deliver_revealed_secret(
    host: &dyn CliHost,
    field: SecretFieldV1,
    secret: RevealedSecretV1,
) -> Result<(), CliFailure> {
    match (field, secret.encoding()) {
        (_, RevealedSecretEncodingV1::Utf8) => {
            let value =
                core::str::from_utf8(secret.as_bytes()).map_err(|_| CliFailure::Integrity)?;
            host.write_revealed_text(value).map_err(map_host)?;
        }
        (SecretFieldV1::TotpSecret, RevealedSecretEncodingV1::Bytes) => {
            let value = encode_totp_base32(secret.as_bytes());
            host.write_revealed_text(&value).map_err(map_host)?;
        }
        (_, RevealedSecretEncodingV1::Bytes) => return Err(CliFailure::Unsupported),
    }
    drop(secret);
    Ok(())
}

fn item_delete(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
) -> Result<CliOutput, CliFailure> {
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
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
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
) -> Result<CliOutput, CliFailure> {
    let (mut access, application_store) =
        authenticated_access(host, paths, writer, selected_vault)?;
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

fn conflict_list(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
) -> Result<CliOutput, CliFailure> {
    let (wall_time_ms, randomness) = audited_access_inputs(host)?;
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
    let candidates = access
        .into_unlocked()
        .map_err(map_application)?
        .audited_conflict_candidates(item_id, wall_time_ms, randomness, &application_store)
        .map_err(map_application)?
        .into_operation()
        .map_err(map_application)?;
    render_history(candidates)
}

fn conflict_reveal(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
    revision_id: RevisionId,
    field: SecretFieldV1,
) -> Result<CliOutput, CliFailure> {
    let (wall_time_ms, randomness) = audited_access_inputs(host)?;
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
    let (confirmed, confirmation_error) = match host.confirm_secret_reveal() {
        Ok(confirmed) => (confirmed, None),
        Err(error) => (false, Some(error)),
    };
    let disclosed = access
        .into_unlocked()
        .map_err(map_application)?
        .audited_reveal_conflict_candidate_field(
            item_id,
            revision_id,
            field,
            SecretDisclosureIntentV1::InteractiveReveal { confirmed },
            wall_time_ms,
            randomness,
            &application_store,
        )
        .map_err(map_application)?
        .into_operation();
    if let Some(error) = confirmation_error {
        if !matches!(disclosed, Err(ApplicationError::InvalidInput)) {
            return Err(CliFailure::Internal);
        }
        return Err(map_host(error));
    }
    let secret = disclosed.map_err(map_application)?;
    deliver_revealed_secret(host, field, secret)?;
    Ok(CliOutput::success(""))
}

fn conflict_choose(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
    revision_id: RevisionId,
) -> Result<CliOutput, CliFailure> {
    let (wall_time_ms, failure_randomness) = audited_access_inputs(host)?;
    let mut mutation_random = [0_u8; RESOLVE_ITEM_CONFLICT_RANDOM_BYTES];
    host.fill_entropy(&mut mutation_random).map_err(map_host)?;
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
    access
        .into_unlocked()
        .map_err(map_application)?
        .audited_resolve_item_conflict_for_item(
            item_id,
            revision_id,
            wall_time_ms,
            ResolveItemConflictRandomnessV1::new(mutation_random),
            failure_randomness,
            &application_store,
        )
        .map_err(map_application)?
        .into_operation()
        .map_err(map_application)?;
    Ok(CliOutput::success(format!(
        "Conflict resolved: {}\n",
        item_id.to_user_string()
    )))
}

fn conflict_merge_login(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
    base_revision: RevisionId,
) -> Result<CliOutput, CliFailure> {
    let (wall_time_ms, failure_randomness) = audited_access_inputs(host)?;
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
    let preparation = access
        .into_unlocked()
        .map_err(map_application)?
        .prepare_audited_login_conflict_merge(
            item_id,
            base_revision,
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
    let mut mutation_random = [0_u8; RESOLVE_ITEM_CONFLICT_RANDOM_BYTES];
    if let Err(error) = host.fill_entropy(&mut mutation_random) {
        preparation
            .record_audited_host_failure(&application_store)
            .map_err(map_application)?;
        return Err(map_host(error));
    }
    preparation
        .complete_audited(
            input,
            ResolveItemConflictRandomnessV1::new(mutation_random),
            &application_store,
        )
        .map_err(map_application)?
        .into_operation()
        .map_err(map_application)?;
    Ok(CliOutput::success(format!(
        "Conflict merged: {}\n",
        item_id.to_user_string()
    )))
}

fn conflict_merge_secure_note(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
    base_revision: RevisionId,
) -> Result<CliOutput, CliFailure> {
    let (wall_time_ms, failure_randomness) = audited_access_inputs(host)?;
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
    let preparation = access
        .into_unlocked()
        .map_err(map_application)?
        .prepare_audited_secure_note_conflict_merge(
            item_id,
            base_revision,
            wall_time_ms,
            failure_randomness,
            &application_store,
        )
        .map_err(map_application)?
        .into_preparation()
        .map_err(map_application)?;
    let input = (|| {
        Ok::<_, HostError>(SecureNoteConflictMergeInputV1::new(
            host.read_secure_note_title()?,
            host.read_secure_note_body()?,
        ))
    })();
    let input = match input {
        Ok(input) => input,
        Err(error) => {
            preparation
                .record_audited_host_failure(&application_store)
                .map_err(map_application)?;
            return Err(map_host(error));
        }
    };
    let mut mutation_random = [0_u8; RESOLVE_ITEM_CONFLICT_RANDOM_BYTES];
    if let Err(error) = host.fill_entropy(&mut mutation_random) {
        preparation
            .record_audited_host_failure(&application_store)
            .map_err(map_application)?;
        return Err(map_host(error));
    }
    preparation
        .complete_audited(
            input,
            ResolveItemConflictRandomnessV1::new(mutation_random),
            &application_store,
        )
        .map_err(map_application)?
        .into_operation()
        .map_err(map_application)?;
    Ok(CliOutput::success(format!(
        "Conflict merged: {}\n",
        item_id.to_user_string()
    )))
}

fn conflict_merge_card(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
    base_revision: RevisionId,
) -> Result<CliOutput, CliFailure> {
    let (wall_time_ms, failure_randomness) = audited_access_inputs(host)?;
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
    let preparation = access
        .into_unlocked()
        .map_err(map_application)?
        .prepare_audited_card_conflict_merge(
            item_id,
            base_revision,
            wall_time_ms,
            failure_randomness,
            &application_store,
        )
        .map_err(map_application)?
        .into_preparation()
        .map_err(map_application)?;
    let input = (|| {
        Ok::<_, HostError>(CardConflictMergeInputV1::new(
            host.read_card_title()?,
            host.read_card_holder()?,
            host.read_card_number()?,
            host.read_card_expiry_month()?,
            host.read_card_expiry_year()?,
            host.read_card_cvv()?,
            host.read_card_billing_postal_code()?,
        ))
    })();
    let input = match input {
        Ok(input) => input,
        Err(error) => {
            preparation
                .record_audited_host_failure(&application_store)
                .map_err(map_application)?;
            return Err(map_host(error));
        }
    };
    let mut mutation_random = [0_u8; RESOLVE_ITEM_CONFLICT_RANDOM_BYTES];
    if let Err(error) = host.fill_entropy(&mut mutation_random) {
        preparation
            .record_audited_host_failure(&application_store)
            .map_err(map_application)?;
        return Err(map_host(error));
    }
    preparation
        .complete_audited(
            input,
            ResolveItemConflictRandomnessV1::new(mutation_random),
            &application_store,
        )
        .map_err(map_application)?
        .into_operation()
        .map_err(map_application)?;
    Ok(CliOutput::success(format!(
        "Conflict merged: {}\n",
        item_id.to_user_string()
    )))
}

fn conflict_merge_api_key(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
    base_revision: RevisionId,
) -> Result<CliOutput, CliFailure> {
    let (wall_time_ms, failure_randomness) = audited_access_inputs(host)?;
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
    let preparation = access
        .into_unlocked()
        .map_err(map_application)?
        .prepare_audited_api_key_conflict_merge(
            item_id,
            base_revision,
            wall_time_ms,
            failure_randomness,
            &application_store,
        )
        .map_err(map_application)?
        .into_preparation()
        .map_err(map_application)?;
    // The scope and expiry lines travel to the application exactly as typed:
    // the audited preparation, not the CLI, is what turns them into a record,
    // so an invalid form is recorded before its error is ever returned.
    let input = (|| {
        Ok::<_, HostError>(ApiKeyConflictMergeInputV1::new(
            host.read_api_key_label()?,
            host.read_api_key_service()?,
            host.read_api_key_token()?,
            host.read_api_key_scopes()?,
            host.read_api_key_expiry()?,
        ))
    })();
    let input = match input {
        Ok(input) => input,
        Err(error) => {
            preparation
                .record_audited_host_failure(&application_store)
                .map_err(map_application)?;
            return Err(map_host(error));
        }
    };
    let mut mutation_random = [0_u8; RESOLVE_ITEM_CONFLICT_RANDOM_BYTES];
    if let Err(error) = host.fill_entropy(&mut mutation_random) {
        preparation
            .record_audited_host_failure(&application_store)
            .map_err(map_application)?;
        return Err(map_host(error));
    }
    preparation
        .complete_audited(
            input,
            ResolveItemConflictRandomnessV1::new(mutation_random),
            &application_store,
        )
        .map_err(map_application)?
        .into_operation()
        .map_err(map_application)?;
    Ok(CliOutput::success(format!(
        "Conflict merged: {}\n",
        item_id.to_user_string()
    )))
}

fn conflict_merge_database_credential(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
    base_revision: RevisionId,
) -> Result<CliOutput, CliFailure> {
    let (wall_time_ms, failure_randomness) = audited_access_inputs(host)?;
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
    let preparation = access
        .into_unlocked()
        .map_err(map_application)?
        .prepare_audited_database_credential_conflict_merge(
            item_id,
            base_revision,
            wall_time_ms,
            failure_randomness,
            &application_store,
        )
        .map_err(map_application)?
        .into_preparation()
        .map_err(map_application)?;
    // The engine and port lines travel to the application exactly as typed:
    // the audited preparation, not the CLI, is what turns them into a record,
    // so an invalid form is recorded before its error is ever returned.
    let input = (|| {
        Ok::<_, HostError>(DatabaseCredentialConflictMergeInputV1::new(
            host.read_database_label()?,
            host.read_database_engine()?,
            host.read_database_host()?,
            host.read_database_port()?,
            host.read_database_name()?,
            host.read_database_username()?,
            host.read_database_password()?,
        ))
    })();
    let input = match input {
        Ok(input) => input,
        Err(error) => {
            preparation
                .record_audited_host_failure(&application_store)
                .map_err(map_application)?;
            return Err(map_host(error));
        }
    };
    let mut mutation_random = [0_u8; RESOLVE_ITEM_CONFLICT_RANDOM_BYTES];
    if let Err(error) = host.fill_entropy(&mut mutation_random) {
        preparation
            .record_audited_host_failure(&application_store)
            .map_err(map_application)?;
        return Err(map_host(error));
    }
    preparation
        .complete_audited(
            input,
            ResolveItemConflictRandomnessV1::new(mutation_random),
            &application_store,
        )
        .map_err(map_application)?
        .into_operation()
        .map_err(map_application)?;
    Ok(CliOutput::success(format!(
        "Conflict merged: {}\n",
        item_id.to_user_string()
    )))
}

fn conflict_merge_totp(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
    base_revision: RevisionId,
) -> Result<CliOutput, CliFailure> {
    let (wall_time_ms, failure_randomness) = audited_access_inputs(host)?;
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
    let preparation = access
        .into_unlocked()
        .map_err(map_application)?
        .prepare_audited_totp_conflict_merge(
            item_id,
            base_revision,
            wall_time_ms,
            failure_randomness,
            &application_store,
        )
        .map_err(map_application)?
        .into_preparation()
        .map_err(map_application)?;
    // The Base32 seed line and every parameter travel to the application
    // exactly as typed: the audited preparation, not the CLI, is what decodes
    // and turns them into a record, so an invalid form is recorded before its
    // error is ever returned.
    let input = (|| {
        Ok::<_, HostError>(TotpConflictMergeInputV1::new(
            host.read_totp_label()?,
            host.read_totp_issuer()?,
            host.read_totp_secret()?,
            host.read_totp_algorithm()?,
            host.read_totp_digits()?,
            host.read_totp_period()?,
        ))
    })();
    let input = match input {
        Ok(input) => input,
        Err(error) => {
            preparation
                .record_audited_host_failure(&application_store)
                .map_err(map_application)?;
            return Err(map_host(error));
        }
    };
    let mut mutation_random = [0_u8; RESOLVE_ITEM_CONFLICT_RANDOM_BYTES];
    if let Err(error) = host.fill_entropy(&mut mutation_random) {
        preparation
            .record_audited_host_failure(&application_store)
            .map_err(map_application)?;
        return Err(map_host(error));
    }
    preparation
        .complete_audited(
            input,
            ResolveItemConflictRandomnessV1::new(mutation_random),
            &application_store,
        )
        .map_err(map_application)?
        .into_operation()
        .map_err(map_application)?;
    Ok(CliOutput::success(format!(
        "Conflict merged: {}\n",
        item_id.to_user_string()
    )))
}

fn conflict_merge_opaque(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
    base_revision: RevisionId,
) -> Result<CliOutput, CliFailure> {
    let (wall_time_ms, failure_randomness) = audited_access_inputs(host)?;
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
    let preparation = access
        .into_unlocked()
        .map_err(map_application)?
        .prepare_audited_opaque_conflict_merge(
            item_id,
            base_revision,
            wall_time_ms,
            failure_randomness,
            &application_store,
        )
        .map_err(map_application)?
        .into_preparation()
        .map_err(map_application)?;
    // One hidden prompt is the whole form: an opaque record has no schema this
    // product can turn into a field list, and the content type it will keep is
    // the base's rather than anything the terminal could offer. The line
    // travels to the application exactly as typed, so the audited preparation
    // decodes it and an invalid payload is recorded before its error returns.
    let input = host.read_opaque_payload();
    let input = match input {
        Ok(payload) => OpaqueConflictMergeInputV1::new(payload),
        Err(error) => {
            preparation
                .record_audited_host_failure(&application_store)
                .map_err(map_application)?;
            return Err(map_host(error));
        }
    };
    let mut mutation_random = [0_u8; RESOLVE_ITEM_CONFLICT_RANDOM_BYTES];
    if let Err(error) = host.fill_entropy(&mut mutation_random) {
        preparation
            .record_audited_host_failure(&application_store)
            .map_err(map_application)?;
        return Err(map_host(error));
    }
    preparation
        .complete_audited(
            input,
            ResolveItemConflictRandomnessV1::new(mutation_random),
            &application_store,
        )
        .map_err(map_application)?
        .into_operation()
        .map_err(map_application)?;
    Ok(CliOutput::success(format!(
        "Conflict merged: {}\n",
        item_id.to_user_string()
    )))
}

fn history_restore(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    item_id: ItemId,
    revision_id: RevisionId,
) -> Result<CliOutput, CliFailure> {
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
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
        RedactedRecordView::Card {
            title,
            holder,
            last_four,
            expiry_month,
            expiry_year,
            has_billing_zip,
            ..
        } => {
            output.push_str("Title: ");
            output.push_str(&quoted(title));
            output.push_str("\nCardholder: ");
            output.push_str(&quoted(holder));
            output.push_str("\nLast four: ");
            output.push_str(&quoted(last_four));
            output.push_str(&format!("\nExpiry: {expiry_month:02}/{expiry_year:04}\n"));
            output.push_str("Card number: <redacted>\nCVV: <redacted>\n");
            output.push_str("Billing postal code: ");
            output.push_str(if *has_billing_zip {
                "present\n"
            } else {
                "absent\n"
            });
        }
        RedactedRecordView::TotpSeed {
            label,
            issuer,
            algorithm,
            digits,
            period,
            ..
        } => {
            output.push_str("Label: ");
            output.push_str(&quoted(label));
            output.push_str("\nIssuer: ");
            match issuer {
                Some(issuer) => output.push_str(&quoted(issuer)),
                None => output.push_str("none"),
            }
            output.push_str("\nAlgorithm: ");
            output.push_str(algorithm);
            output.push_str(&format!(
                "\nDigits: {digits}\nPeriod: {period}\nSecret: <redacted>\n"
            ));
        }
        RedactedRecordView::ApiKey {
            label,
            service,
            scopes,
            expires_at,
            ..
        } => {
            output.push_str("Label: ");
            output.push_str(&quoted(label));
            output.push_str("\nService: ");
            output.push_str(&quoted(service));
            output.push('\n');
            if scopes.is_empty() {
                output.push_str("Scopes: none\n");
            } else {
                for scope in scopes {
                    output.push_str("Scope: ");
                    output.push_str(&quoted(scope));
                    output.push('\n');
                }
            }
            output.push_str("Expiry: ");
            match expires_at {
                Some(seconds) => output.push_str(&seconds.to_string()),
                None => output.push_str("none"),
            }
            output.push_str("\nToken: <redacted>\n");
        }
        RedactedRecordView::DatabaseCredential {
            label,
            engine,
            host,
            port,
            database,
            username,
            expires_at,
            has_lease_id,
            ..
        } => {
            output.push_str("Label: ");
            output.push_str(&quoted(label));
            output.push_str("\nEngine: ");
            output.push_str(&quoted(engine));
            output.push_str("\nHost: ");
            output.push_str(&quoted(host));
            output.push_str(&format!("\nPort: {port}\nDatabase: "));
            match database {
                Some(database) => output.push_str(&quoted(database)),
                None => output.push_str("none"),
            }
            output.push_str("\nUsername: ");
            output.push_str(&quoted(username));
            output.push_str("\nLease: ");
            output.push_str(if *has_lease_id { "present" } else { "absent" });
            output.push_str("\nExpiry: ");
            match expires_at {
                Some(seconds) => output.push_str(&seconds.to_string()),
                None => output.push_str("none"),
            }
            output.push_str("\nPassword: <redacted>\n");
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
    let mut random_bytes = [0_u8; AUDITED_GENERATION_ZERO_RANDOM_BYTES];
    host.fill_entropy(&mut random_bytes).map_err(map_host)?;
    let (memory_kib, iterations, lanes) = host.generation_zero_kdf();
    let policy = GenerationZeroPolicyV1::new(
        memory_kib,
        iterations,
        lanes,
        host.now_ms().map_err(map_host)?,
    )
    .map_err(map_application)?;
    let prepared = prepare_audited_generation_zero(
        passphrase,
        policy,
        AuditedGenerationZeroRandomness::new(random_bytes),
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
    crash::around_config_create(|| writer.create_config(render_config(&config).as_bytes()))
        .map_err(map_local_host)?;

    let repository_factory = repository_factory(paths.object_root());
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
    let vault = configured_vault(paths, &config, None)?;
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
        return match state {
            LocalVaultStateV1::Active(_) => Err(CliFailure::AlreadyInitialized),
            LocalVaultStateV1::PendingPublication { .. } => {
                resume_pending_publication(host, &config, vault, locator, &application_store)
            }
            LocalVaultStateV1::PreparedInit(_) => unreachable!(),
        };
    };
    let passphrase = host.read_existing_passphrase().map_err(map_host)?;
    let prepared = rehydrate_prepared_init(passphrase, state).map_err(map_application)?;
    let repository_factory = configured_repository_factory(&config, vault)?;
    complete_generation_zero(
        prepared,
        &application_store,
        &application_store,
        &repository_factory,
    )
    .map_err(map_application)?;
    Ok(CliOutput::success("Vault initialized.\n"))
}

/// Finish an interrupted mutation publication found by a resume path.
///
/// VLT-PM42. `init` and `vault create` both mean "finish whatever was
/// interrupted here", and both used to refuse a `PendingPublication` with the
/// conflict class. That refusal answered the right observation the wrong way:
/// the vault's *creation* had indeed finished, so there was no generation zero
/// left to resume — a later mutation had been cut short instead. A pending
/// publication is the same promise one generation on, and these are the verbs
/// a stuck person retries.
///
/// The repaired vault is opened before success is reported, so "recovered"
/// means a real authenticated open of the repaired durable bytes succeeded,
/// not merely that a write returned.
fn resume_pending_publication(
    host: &dyn CliHost,
    config: &VaultPmConfigV1,
    vault: &VaultConfigV1,
    locator: BootstrapLocator,
    application_store: &StorageCoreApplicationStore<LocalBackend>,
) -> Result<CliOutput, CliFailure> {
    let repository_factory = configured_repository_factory(config, vault)?;
    let passphrase = host.read_existing_passphrase().map_err(map_host)?;
    let mut access = VaultAccessV1::locked(locator);
    access
        .unlock_recovering_pending_publication(
            passphrase,
            application_store,
            application_store,
            &repository_factory,
        )
        .map_err(map_application)?;
    access.lock();
    Ok(CliOutput::success("Vault recovered.\n"))
}

fn vault_create(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    vault_name: ConfigName,
) -> Result<CliOutput, CliFailure> {
    let exact_config = writer
        .load_config()
        .map_err(map_local_host)?
        .ok_or(CliFailure::InvalidCommand)?;
    let config = decode_config(&exact_config)?;
    if config.select_vault(Some(&vault_name)).is_some() {
        return resume_vault_create(host, paths, &config, &vault_name);
    }

    let source = configured_vault(paths, &config, None)?;
    let passphrase = host.read_new_passphrase().map_err(map_host)?;
    let mut random_bytes = [0_u8; AUDITED_GENERATION_ZERO_RANDOM_BYTES];
    host.fill_entropy(&mut random_bytes).map_err(map_host)?;
    let (memory_kib, iterations, lanes) = host.generation_zero_kdf();
    let policy = GenerationZeroPolicyV1::new(
        memory_kib,
        iterations,
        lanes,
        host.now_ms().map_err(map_host)?,
    )
    .map_err(map_application)?;
    let prepared = prepare_audited_generation_zero(
        passphrase,
        policy,
        AuditedGenerationZeroRandomness::new(random_bytes),
    )
    .map_err(map_application)?;
    let locator = prepared.bootstrap_locator();
    let exact_prepared = prepared.owner_state().encode().map_err(map_application)?;
    let application_store = application_store(paths);

    // The exact encrypted creation trace and retry journal become durable
    // before the new random locator is made discoverable in configuration.
    application_store
        .compare_exchange(locator, None, &exact_prepared)
        .map_err(map_local_state)?;

    let target_root = target_object_root(paths, locator.as_bytes());
    let target_location = target_root.to_str().ok_or(CliFailure::Unsupported)?;
    let target_storage_name = target_storage_name(locator.as_bytes())?;
    let target_storage = StorageConfigV1::new(
        StorageKind::Filesystem,
        StorageLocation::new(target_location).map_err(|_| CliFailure::Unsupported)?,
        CredentialRef::none(),
    );
    let target = VaultConfigV1::new(
        ConfigVaultLocator::new(*locator.as_bytes()),
        target_storage_name.clone(),
        Vec::new(),
        source.auto_lock_seconds(),
        source.clipboard_clear_seconds(),
    )
    .map_err(|_| CliFailure::Internal)?;
    let mut vaults = config.vaults().clone();
    if vaults.insert(vault_name, target).is_some() {
        return Err(CliFailure::Conflict);
    }
    let mut storage = config.storage().clone();
    if storage
        .insert(target_storage_name, target_storage)
        .is_some()
    {
        return Err(CliFailure::Conflict);
    }
    let replacement = VaultPmConfigV1::new(config.default_vault().clone(), vaults, storage)
        .map_err(|_| CliFailure::Internal)?;
    let rendered = render_config(&replacement);
    crash::around_config_replace(|| {
        writer.compare_exchange_config(&exact_config, rendered.as_bytes())
    })
    .map_err(map_local_host)?;

    let repository_factory = repository_factory(&target_root);
    complete_generation_zero(
        prepared,
        &application_store,
        &application_store,
        &repository_factory,
    )
    .map_err(map_application)?;
    Ok(CliOutput::success("Vault target created.\n"))
}

fn resume_vault_create(
    host: &dyn CliHost,
    paths: &LocalVaultPaths,
    config: &VaultPmConfigV1,
    vault_name: &ConfigName,
) -> Result<CliOutput, CliFailure> {
    let vault = configured_vault(paths, config, Some(vault_name))?;
    let locator = application_locator(vault.locator());
    let application_store = application_store(paths);
    let exact_state = application_store
        .load(locator)
        .map_err(map_local_state)?
        .ok_or(CliFailure::Integrity)?;
    let state = LocalVaultStateV1::decode(&exact_state).map_err(map_application)?;
    let LocalVaultStateV1::PreparedInit(_) = state else {
        return match state {
            LocalVaultStateV1::Active(_) => Err(CliFailure::AlreadyInitialized),
            LocalVaultStateV1::PendingPublication { .. } => {
                resume_pending_publication(host, config, vault, locator, &application_store)
            }
            LocalVaultStateV1::PreparedInit(_) => unreachable!(),
        };
    };
    let passphrase = host.read_existing_passphrase().map_err(map_host)?;
    let prepared = rehydrate_prepared_init(passphrase, state).map_err(map_application)?;
    let repository_factory = configured_repository_factory(config, vault)?;
    complete_generation_zero(
        prepared,
        &application_store,
        &application_store,
        &repository_factory,
    )
    .map_err(map_application)?;
    Ok(CliOutput::success("Vault target created.\n"))
}

fn status(
    paths: &LocalVaultPaths,
    writer: &LocalWriterGuard,
    selected_vault: Option<&ConfigName>,
    json: bool,
) -> Result<CliOutput, CliFailure> {
    let Some(exact_config) = writer.load_config().map_err(map_local_host)? else {
        return Ok(render_status_label("uninitialized", json));
    };
    let config = decode_config(&exact_config)?;
    let vault = configured_vault(paths, &config, selected_vault)?;
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
    selected_vault: Option<&ConfigName>,
) -> Result<CliOutput, CliFailure> {
    let (mut access, application_store) =
        authenticated_access(host, paths, writer, selected_vault)?;
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
    selected_vault: Option<&ConfigName>,
) -> Result<CliOutput, CliFailure> {
    let exact_config = writer
        .load_config()
        .map_err(map_local_host)?
        .ok_or(CliFailure::InvalidCommand)?;
    let config = decode_config(&exact_config)?;
    let vault = configured_vault(paths, &config, selected_vault)?;
    let locator = application_locator(vault.locator());
    let application_store = application_store(paths);
    let repository_factory = configured_repository_factory(&config, vault)?;
    let mut access = VaultAccessV1::locked(locator);
    let passphrase = host.read_existing_passphrase().map_err(map_host)?;
    // VLT-PM42. `audit verify` publishes an audit-only commit of its own
    // through the very path a crash interrupts, so it finishes an outstanding
    // publication before starting another.
    access
        .unlock_recovering_pending_publication(
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
    selected_vault: Option<&ConfigName>,
) -> Result<CliOutput, CliFailure> {
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
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
    selected_vault: Option<&ConfigName>,
    trace_id: OperationId,
) -> Result<CliOutput, CliFailure> {
    let (access, application_store) = authenticated_access(host, paths, writer, selected_vault)?;
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
    selected_vault: Option<&ConfigName>,
    unlock: bool,
) -> Result<CliOutput, CliFailure> {
    let Some(exact_config) = writer.load_config().map_err(map_local_host)? else {
        return Ok(doctor_output(
            "initialization_required",
            ExitCode::InvalidInput,
        ));
    };
    let config = decode_config(&exact_config)?;
    let vault = configured_vault(paths, &config, selected_vault)?;
    let locator = application_locator(vault.locator());
    let application_store = application_store(paths);
    let mut access = VaultAccessV1::locked(locator);
    // VLT-PM42. `doctor` is the one command that reports without repairing,
    // and `--unlock` does not change that: a person who wants to look at a
    // wedged vault before touching it must be able to. So an interrupted
    // publication short-circuits the authenticated half entirely — no
    // passphrase is collected, nothing is published, and the read-only
    // diagnostic answers `recovery_required` with exit class 5. Only the
    // *classification* changes: this case used to inherit the refused open's
    // misleading exit 2 `invalid command`.
    let unlock = unlock
        && !matches!(
            access
                .status(&application_store)
                .map_err(map_application)?
                .state(),
            VaultStatusStateV1::RecoveryRequired
        );
    if unlock {
        let repository_factory = configured_repository_factory(&config, vault)?;
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
    selected_vault: Option<&ConfigName>,
) -> Result<&'a VaultConfigV1, CliFailure> {
    let vault = config.select_vault(selected_vault).ok_or_else(|| {
        if selected_vault.is_some() {
            CliFailure::NotFound
        } else {
            CliFailure::Integrity
        }
    })?;
    if !vault.remote_stores().is_empty() {
        return Err(CliFailure::Unsupported);
    }
    let storage = config
        .storage()
        .get(vault.local_store())
        .ok_or(CliFailure::Integrity)?;
    if config.vaults().values().any(|candidate| {
        candidate.locator() != vault.locator()
            && config
                .storage()
                .get(candidate.local_store())
                .is_some_and(|candidate_storage| {
                    candidate_storage.kind() == StorageKind::Filesystem
                        && candidate_storage.location() == storage.location()
                })
    }) {
        return Err(CliFailure::Unsupported);
    }
    let root_location = paths
        .object_root()
        .to_str()
        .ok_or(CliFailure::Unsupported)?;
    let target_location = target_object_root(paths, vault.locator().as_bytes());
    let target_location = target_location.to_str().ok_or(CliFailure::Unsupported)?;
    if storage.kind() != StorageKind::Filesystem
        || (storage.location().as_str() != root_location
            && storage.location().as_str() != target_location)
        || storage.credential_ref().as_str() != "none"
    {
        return Err(CliFailure::Unsupported);
    }
    Ok(vault)
}

fn application_locator(locator: ConfigVaultLocator) -> BootstrapLocator {
    BootstrapLocator::new(*locator.as_bytes())
}

fn application_store(paths: &LocalVaultPaths) -> StorageCoreApplicationStore<LocalBackend> {
    StorageCoreApplicationStore::new(crash::backend(paths.application_state_root()))
}

fn repository_factory(
    object_root: &Path,
) -> V1ApplicationRepositoryFactory<StorageCoreObjectStore<LocalBackend>> {
    V1ApplicationRepositoryFactory::new(StorageCoreObjectStore::new(crash::backend(object_root)))
}

fn configured_repository_factory(
    config: &VaultPmConfigV1,
    vault: &VaultConfigV1,
) -> Result<V1ApplicationRepositoryFactory<StorageCoreObjectStore<LocalBackend>>, CliFailure> {
    let storage = config
        .storage()
        .get(vault.local_store())
        .ok_or(CliFailure::Integrity)?;
    Ok(repository_factory(Path::new(storage.location().as_str())))
}

fn target_object_root(paths: &LocalVaultPaths, locator: &[u8; 32]) -> PathBuf {
    paths
        .object_root()
        .join("targets")
        .join(locator_hex(locator))
}

fn target_storage_name(locator: &[u8; 32]) -> Result<ConfigName, CliFailure> {
    let encoded = locator_hex(locator);
    ConfigName::new(format!("v{}", &encoded[..63])).map_err(|_| CliFailure::Internal)
}

fn locator_hex(locator: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in locator {
        encoded.push(char::from(HEX[usize::from(*byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(*byte & 0x0f)]));
    }
    encoded
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
    use coding_adventures_vault_pm_application::{
        prepare_generation_zero, GenerationZeroRandomness, GENERATION_ZERO_RANDOM_BYTES,
    };
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

    /// Advisory wall time reported by a [`TestHost`] whose clock never moves.
    const FIXED_TEST_TIME_MS: u64 = 1_700_000_000_000;

    struct TestHost {
        paths: LocalVaultPaths,
        secrets: Mutex<VecDeque<Vec<u8>>>,
        texts: Mutex<VecDeque<String>>,
        revealed: Mutex<Vec<Zeroizing<Vec<u8>>>>,
        entropy_seed: u8,
        entropy_available: bool,
        /// Wall time returned by the next [`CliHost::now_ms`] call.
        clock_ms: AtomicU64,
        /// Milliseconds the clock advances after each reading.
        ///
        /// Zero — the default every existing test uses — makes the clock a
        /// constant, so nothing that does not opt in can observe elapsed time.
        clock_step_ms: u64,
    }

    impl TestHost {
        fn new(paths: LocalVaultPaths, secrets: impl IntoIterator<Item = Vec<u8>>) -> Self {
            Self::build(paths, secrets, [], 1, true, 0)
        }

        fn with_entropy_seed(
            paths: LocalVaultPaths,
            secrets: impl IntoIterator<Item = Vec<u8>>,
            entropy_seed: u8,
        ) -> Self {
            Self::build(paths, secrets, [], entropy_seed, true, 0)
        }

        fn with_texts(
            paths: LocalVaultPaths,
            secrets: impl IntoIterator<Item = Vec<u8>>,
            texts: impl IntoIterator<Item = String>,
        ) -> Self {
            Self::build(paths, secrets, texts, 1, true, 0)
        }

        fn with_texts_and_entropy_seed(
            paths: LocalVaultPaths,
            secrets: impl IntoIterator<Item = Vec<u8>>,
            texts: impl IntoIterator<Item = String>,
            entropy_seed: u8,
        ) -> Self {
            Self::build(paths, secrets, texts, entropy_seed, true, 0)
        }

        fn without_entropy(
            paths: LocalVaultPaths,
            secrets: impl IntoIterator<Item = Vec<u8>>,
        ) -> Self {
            Self::build(paths, secrets, [], 1, false, 0)
        }

        /// Build a host whose clock advances by `clock_step_ms` per reading.
        fn with_clock_step(
            paths: LocalVaultPaths,
            secrets: impl IntoIterator<Item = Vec<u8>>,
            clock_step_ms: u64,
        ) -> Self {
            Self::build(paths, secrets, [], 1, true, clock_step_ms)
        }

        fn build(
            paths: LocalVaultPaths,
            secrets: impl IntoIterator<Item = Vec<u8>>,
            texts: impl IntoIterator<Item = String>,
            entropy_seed: u8,
            entropy_available: bool,
            clock_step_ms: u64,
        ) -> Self {
            Self {
                paths,
                secrets: Mutex::new(secrets.into_iter().collect()),
                texts: Mutex::new(texts.into_iter().collect()),
                revealed: Mutex::new(Vec::new()),
                entropy_seed,
                entropy_available,
                clock_ms: AtomicU64::new(FIXED_TEST_TIME_MS),
                clock_step_ms,
            }
        }

        /// Return how many scripted secrets remain unconsumed.
        fn remaining_secrets(&self) -> usize {
            self.secrets.lock().unwrap().len()
        }

        /// Move the advisory clock forward without reading it.
        ///
        /// This models wall time passing while the process is blocked on
        /// something other than a clock call — a terminal read, for instance.
        fn advance_clock(&self, milliseconds: u64) {
            self.clock_ms.fetch_add(milliseconds, Ordering::Relaxed);
        }

        /// Move the advisory clock backwards, as an NTP step or a manual
        /// correction can do to real wall time.
        fn rewind_clock(&self, milliseconds: u64) {
            self.clock_ms.fetch_sub(milliseconds, Ordering::Relaxed);
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

        fn revealed_equals(&self, expected: &[u8]) -> bool {
            let revealed = self.revealed.lock().unwrap();
            matches!(revealed.as_slice(), [value] if value.as_slice() == expected)
        }

        fn revealed_count(&self) -> usize {
            self.revealed.lock().unwrap().len()
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

        fn read_login_url_count(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_login_url(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_login_password(&self) -> Result<Zeroizing<String>, HostError> {
            let value = self.secret()?;
            let text = core::str::from_utf8(&value).map_err(|_| HostError::Invalid)?;
            Ok(Zeroizing::new(text.to_owned()))
        }

        fn read_login_notes(&self) -> Result<Option<Zeroizing<String>>, HostError> {
            let value = self.secret()?;
            if value.is_empty() {
                return Ok(None);
            }
            let text = core::str::from_utf8(&value).map_err(|_| HostError::Invalid)?;
            Ok(Some(Zeroizing::new(text.to_owned())))
        }

        fn read_secure_note_title(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_secure_note_body(&self) -> Result<Zeroizing<String>, HostError> {
            let value = self.secret()?;
            let text = core::str::from_utf8(&value).map_err(|_| HostError::Invalid)?;
            Ok(Zeroizing::new(text.to_owned()))
        }

        fn read_card_title(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_card_holder(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_card_number(&self) -> Result<Zeroizing<String>, HostError> {
            let value = self.secret()?;
            let text = core::str::from_utf8(&value).map_err(|_| HostError::Invalid)?;
            Ok(Zeroizing::new(text.to_owned()))
        }

        fn read_card_expiry_month(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_card_expiry_year(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_card_cvv(&self) -> Result<Zeroizing<String>, HostError> {
            let value = self.secret()?;
            let text = core::str::from_utf8(&value).map_err(|_| HostError::Invalid)?;
            Ok(Zeroizing::new(text.to_owned()))
        }

        fn read_card_billing_postal_code(&self) -> Result<Option<Zeroizing<String>>, HostError> {
            self.text()
                .map(|value| (!value.is_empty()).then_some(value))
        }

        fn read_api_key_label(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_api_key_service(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_api_key_token(&self) -> Result<Zeroizing<String>, HostError> {
            let value = self.secret()?;
            let text = core::str::from_utf8(&value).map_err(|_| HostError::Invalid)?;
            Ok(Zeroizing::new(text.to_owned()))
        }

        fn read_api_key_scopes(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_api_key_expiry(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_database_label(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }
        fn read_database_engine(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }
        fn read_database_host(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }
        fn read_database_port(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }
        fn read_database_name(&self) -> Result<Option<Zeroizing<String>>, HostError> {
            self.text()
                .map(|value| (!value.is_empty()).then_some(value))
        }
        fn read_database_username(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }
        fn read_database_password(&self) -> Result<Zeroizing<String>, HostError> {
            let value = self.secret()?;
            let text = core::str::from_utf8(&value).map_err(|_| HostError::Invalid)?;
            Ok(Zeroizing::new(text.to_owned()))
        }

        fn read_totp_label(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }
        fn read_totp_issuer(&self) -> Result<Option<Zeroizing<String>>, HostError> {
            self.text()
                .map(|value| (!value.is_empty()).then_some(value))
        }
        fn read_totp_secret(&self) -> Result<Zeroizing<String>, HostError> {
            let value = self.secret()?;
            let text = core::str::from_utf8(&value).map_err(|_| HostError::Invalid)?;
            Ok(Zeroizing::new(text.to_owned()))
        }
        fn read_totp_algorithm(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }
        fn read_totp_digits(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }
        fn read_totp_period(&self) -> Result<Zeroizing<String>, HostError> {
            self.text()
        }

        fn read_opaque_payload(&self) -> Result<Zeroizing<String>, HostError> {
            let value = self.secret()?;
            let text = core::str::from_utf8(&value).map_err(|_| HostError::Invalid)?;
            Ok(Zeroizing::new(text.to_owned()))
        }

        fn confirm_secret_reveal(&self) -> Result<bool, HostError> {
            self.text().map(|answer| answer.as_str() == "yes")
        }

        fn write_revealed_text(&self, value: &str) -> Result<(), HostError> {
            self.revealed
                .lock()
                .unwrap()
                .push(Zeroizing::new(value.as_bytes().to_vec()));
            Ok(())
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
            if !self.entropy_available {
                return Err(HostError::Unavailable);
            }
            for (index, byte) in output.iter_mut().enumerate() {
                *byte = u8::try_from(index % 251)
                    .unwrap()
                    .wrapping_add(self.entropy_seed);
            }
            Ok(())
        }

        fn now_ms(&self) -> Result<u64, HostError> {
            Ok(self
                .clock_ms
                .fetch_add(self.clock_step_ms, Ordering::Relaxed))
        }

        fn generation_zero_kdf(&self) -> (u32, u32, u8) {
            (8 * 1024, 1, 1)
        }
    }

    fn default_invocation(command: Command) -> Result<Invocation, CliFailure> {
        Ok(Invocation {
            selected_vault: None,
            command,
        })
    }

    fn activate_test_audit_epoch(paths: &LocalVaultPaths, passphrase: Vec<u8>) {
        let prepared = paths.prepare().unwrap();
        let writer = prepared.try_acquire_writer().unwrap();
        let host = TestHost::new(paths.clone(), [passphrase]);
        let (mut access, application_store) =
            authenticated_access(&host, paths, &writer, None).unwrap();
        if access.as_unlocked().unwrap().audit_enabled() {
            access.lock();
            return;
        }
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
            vec!["vault"],
            vec!["vault", "create"],
            vec!["vault", "create", "work", "extra"],
            vec!["vault", "create", "bad.name"],
            vec!["--vault"],
            vec!["--vault", "work"],
            vec!["--vault", "work", "--vault", "personal", "status"],
            vec!["--vault", "work", "init"],
            vec!["--vault", "work", "vault", "create", "other"],
            vec!["--vault", "work", "help"],
            vec!["status", "--unsafe-include-secrets"],
            vec!["doctor", "extra"],
            vec!["doctor", "--unlock", "extra"],
            vec!["export"],
            vec!["export", "backup.vpm", "extra"],
            vec!["export", "--passphrase", "secret"],
            vec!["import"],
            vec!["import", "backup.vpm", "extra"],
            vec!["import", "--passphrase", "secret"],
            vec!["restore"],
            vec!["restore", "backup.vpm"],
            vec!["restore", "verify"],
            vec!["restore", "verify", "backup.vpm", "extra"],
            vec!["restore", "verify", "--passphrase", "secret"],
            vec!["audit"],
            vec!["audit", "enable", "extra"],
            vec!["audit", "verify", "extra"],
            vec!["audit", "list", "extra"],
            vec!["audit", "show", "not-a-trace"],
            vec!["audit", "show", "not-a-trace", "extra"],
            vec!["item", "add", "login", "--password", "secret"],
            vec!["item", "add", "secure-note", "--body", "secret"],
            vec!["item", "add", "card", "--number", "4242424242424242"],
            vec!["item", "edit", "not-an-item-id"],
            vec!["item", "delete", "not-an-item-id"],
            vec!["item", "list", "extra"],
            vec!["item", "show", "not-an-item-id"],
            vec!["item", "reveal", "not-an-item-id", "login-password"],
            vec!["item", "reveal", "not-an-item-id", "unknown-field"],
            vec!["search"],
            vec!["search", "portal", "extra"],
            vec!["search", "--query"],
            vec!["history"],
            vec!["history", "list", "not-an-item-id"],
            vec!["history", "list", "not-an-item-id", "extra"],
            vec!["history", "restore", "not-an-item-id", "not-a-revision"],
            vec!["conflict"],
            vec!["conflict", "list", "not-an-item-id"],
            vec![
                "conflict",
                "reveal",
                "not-an-item-id",
                "not-a-revision",
                "login-password",
            ],
            vec!["conflict", "choose", "not-an-item-id", "not-a-revision"],
            vec![
                "conflict",
                "merge",
                "login",
                "not-an-item-id",
                "not-a-revision",
            ],
            vec!["unlock"],
        ] {
            let output = run(arguments, &host);
            assert_eq!(output.exit_code(), ExitCode::InvalidInput);
            assert_eq!(output.stderr(), "vault-pm: invalid command\n");
        }
        assert!(!root.0.join("config").exists());
    }

    #[test]
    fn search_parser_owns_and_redacts_exactly_one_query() {
        let parsed = parse(["search", "private portal metadata"]);
        assert_eq!(
            parsed,
            default_invocation(Command::Search {
                query: SearchQuery::new("private portal metadata".to_owned()),
            })
        );
        let diagnostic = format!("{parsed:?}");
        assert!(diagnostic.contains("SearchQuery(<redacted>)"));
        assert!(!diagnostic.contains("private portal metadata"));

        assert_eq!(
            parse(["--vault", "work", "search", "portal"]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::Search {
                    query: SearchQuery::new("portal".to_owned()),
                },
            })
        );
        assert_eq!(
            parse(["search", ""]),
            default_invocation(Command::Search {
                query: SearchQuery::new(String::new()),
            })
        );
        assert_eq!(
            parse(["--vault", "bad.name", "search", "metadata"]),
            Err(CliFailure::InvalidCommand)
        );
    }

    #[cfg(unix)]
    #[test]
    fn search_parser_rejects_non_unicode_after_wiping_prior_owned_values() {
        use std::os::unix::ffi::OsStringExt;

        assert_eq!(
            parse([
                OsString::from("search"),
                OsString::from("metadata query"),
                OsString::from_vec(vec![0xff]),
            ]),
            Err(CliFailure::InvalidCommand)
        );
    }

    #[test]
    fn parser_separates_named_creation_from_command_scoped_selection() {
        let work = ConfigName::new("work".to_owned()).unwrap();
        assert_eq!(
            parse(["vault", "create", "work"]),
            default_invocation(Command::VaultCreate {
                vault: work.clone(),
            })
        );
        assert_eq!(
            parse(["--vault", "work", "status", "--json"]),
            Ok(Invocation {
                selected_vault: Some(work),
                command: Command::Status { json: true },
            })
        );
    }

    #[test]
    fn audit_enable_reports_audit_first_generation_zero_as_idempotently_enabled() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"audit migration passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let enable_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let enabled = run(["audit", "enable"], &enable_host);
        assert_eq!(enabled.exit_code(), ExitCode::Success, "{enabled:?}");
        assert_eq!(enabled.stdout(), "Audit: already enabled.\n");

        let repeated_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let repeated = run(["audit", "enable"], &repeated_host);
        assert_eq!(repeated.exit_code(), ExitCode::Success, "{repeated:?}");
        assert_eq!(repeated.stdout(), "Audit: already enabled.\n");

        let verify_host = TestHost::new(paths, [passphrase]);
        let verified = run(["audit", "verify"], &verify_host);
        assert_eq!(verified.exit_code(), ExitCode::Success, "{verified:?}");
        assert!(verified.stdout().contains("commits=1"), "{verified:?}");
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
        assert!(rows[0].contains("\tcounter=2\taction=audit_read\toutcome=succeeded\t"));
        assert!(rows[1].contains("\tcounter=1\taction=vault_initialize\toutcome=succeeded\t"));
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
            default_invocation(Command::ItemShow { item_id })
        );
        assert_eq!(
            parse(["item", "show", canonical.to_lowercase().as_str()]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse(["item", "reveal", canonical.as_str(), "login-password",]),
            default_invocation(Command::ItemReveal {
                item_id,
                field: SecretFieldV1::LoginPassword,
            })
        );
        assert_eq!(
            parse([
                "--vault",
                "work",
                "item",
                "reveal",
                canonical.as_str(),
                "secure-note-body",
            ]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ItemReveal {
                    item_id,
                    field: SecretFieldV1::SecureNoteBody,
                },
            })
        );
        assert_eq!(
            parse([
                "item",
                "reveal",
                canonical.to_lowercase().as_str(),
                "login-password",
            ]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse(["item", "reveal", canonical.as_str(), "password"]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse(["item", "edit", canonical.as_str()]),
            default_invocation(Command::ItemEdit { item_id })
        );
        assert_eq!(
            parse(["item", "delete", canonical.as_str()]),
            default_invocation(Command::ItemDelete { item_id })
        );
        assert_eq!(
            parse(["history", "list", canonical.as_str()]),
            default_invocation(Command::HistoryList { item_id })
        );
        let revision_id = RevisionId::new([0x6b; 32]);
        let revision = revision_id.to_user_string();
        assert_eq!(
            parse(["history", "restore", canonical.as_str(), revision.as_str(),]),
            default_invocation(Command::HistoryRestore {
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
    fn conflict_parsers_require_canonical_item_and_revision_selectors() {
        let item_id = ItemId::new([0x5c; 16]);
        let revision_id = RevisionId::new([0x6d; 32]);
        let item = item_id.to_user_string();
        let revision = revision_id.to_user_string();
        assert_eq!(
            parse(["conflict", "list", item.as_str()]),
            default_invocation(Command::ConflictList { item_id })
        );
        assert_eq!(
            parse([
                "conflict",
                "reveal",
                item.as_str(),
                revision.as_str(),
                "login-password",
            ]),
            default_invocation(Command::ConflictReveal {
                item_id,
                revision_id,
                field: SecretFieldV1::LoginPassword,
            })
        );
        assert_eq!(
            parse([
                "--vault",
                "work",
                "conflict",
                "choose",
                item.as_str(),
                revision.as_str(),
            ]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ConflictChoose {
                    item_id,
                    revision_id,
                },
            })
        );
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "login",
                item.as_str(),
                revision.as_str(),
            ]),
            default_invocation(Command::ConflictMergeLogin {
                item_id,
                base_revision: revision_id,
            })
        );
        assert_eq!(
            parse([
                "--vault",
                "work",
                "conflict",
                "merge",
                "login",
                item.as_str(),
                revision.as_str(),
            ]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ConflictMergeLogin {
                    item_id,
                    base_revision: revision_id,
                },
            })
        );
        assert_eq!(
            parse([
                "--vault",
                "work",
                "conflict",
                "reveal",
                item.as_str(),
                revision.as_str(),
                "secure-note-body",
            ]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ConflictReveal {
                    item_id,
                    revision_id,
                    field: SecretFieldV1::SecureNoteBody,
                },
            })
        );
        assert_eq!(
            parse(["conflict", "list", item.to_lowercase().as_str()]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse([
                "conflict",
                "choose",
                item.as_str(),
                revision.to_lowercase().as_str(),
            ]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse([
                "conflict",
                "reveal",
                item.as_str(),
                revision.to_lowercase().as_str(),
                "login-password",
            ]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse([
                "conflict",
                "reveal",
                item.as_str(),
                revision.as_str(),
                "password",
            ]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "secure-note",
                item.as_str(),
                revision.as_str(),
            ]),
            default_invocation(Command::ConflictMergeSecureNote {
                item_id,
                base_revision: revision_id,
            })
        );
        assert_eq!(
            parse([
                "--vault",
                "work",
                "conflict",
                "merge",
                "secure-note",
                item.as_str(),
                revision.as_str(),
            ]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ConflictMergeSecureNote {
                    item_id,
                    base_revision: revision_id,
                },
            })
        );
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "card",
                item.as_str(),
                revision.as_str(),
            ]),
            default_invocation(Command::ConflictMergeCard {
                item_id,
                base_revision: revision_id,
            })
        );
        assert_eq!(
            parse([
                "--vault",
                "work",
                "conflict",
                "merge",
                "card",
                item.as_str(),
                revision.as_str(),
            ]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ConflictMergeCard {
                    item_id,
                    base_revision: revision_id,
                },
            })
        );
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "api-key",
                item.as_str(),
                revision.as_str(),
            ]),
            default_invocation(Command::ConflictMergeApiKey {
                item_id,
                base_revision: revision_id,
            })
        );
        assert_eq!(
            parse([
                "--vault",
                "work",
                "conflict",
                "merge",
                "api-key",
                item.as_str(),
                revision.as_str(),
            ]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ConflictMergeApiKey {
                    item_id,
                    base_revision: revision_id,
                },
            })
        );
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "database-credential",
                item.as_str(),
                revision.as_str(),
            ]),
            default_invocation(Command::ConflictMergeDatabaseCredential {
                item_id,
                base_revision: revision_id,
            })
        );
        assert_eq!(
            parse([
                "--vault",
                "work",
                "conflict",
                "merge",
                "database-credential",
                item.as_str(),
                revision.as_str(),
            ]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ConflictMergeDatabaseCredential {
                    item_id,
                    base_revision: revision_id,
                },
            })
        );
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "totp",
                item.as_str(),
                revision.as_str()
            ]),
            default_invocation(Command::ConflictMergeTotp {
                item_id,
                base_revision: revision_id,
            })
        );
        assert_eq!(
            parse([
                "--vault",
                "work",
                "conflict",
                "merge",
                "totp",
                item.as_str(),
                revision.as_str(),
            ]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ConflictMergeTotp {
                    item_id,
                    base_revision: revision_id,
                },
            })
        );
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "opaque",
                item.as_str(),
                revision.as_str()
            ]),
            default_invocation(Command::ConflictMergeOpaque {
                item_id,
                base_revision: revision_id,
            })
        );
        assert_eq!(
            parse([
                "--vault",
                "work",
                "conflict",
                "merge",
                "opaque",
                item.as_str(),
                revision.as_str(),
            ]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ConflictMergeOpaque {
                    item_id,
                    base_revision: revision_id,
                },
            })
        );
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "login",
                item.as_str(),
                revision.to_lowercase().as_str(),
            ]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "secure-note",
                item.as_str(),
                revision.to_lowercase().as_str(),
            ]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "api-key",
                item.as_str(),
                revision.to_lowercase().as_str(),
            ]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse(["conflict", "merge", "api-key", item.as_str()]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "database-credential",
                item.as_str(),
                revision.to_lowercase().as_str(),
            ]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse(["conflict", "merge", "database-credential", item.as_str()]),
            Err(CliFailure::InvalidCommand)
        );
        // The schema selector is closed: no abbreviation stands in for it.
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "database",
                item.as_str(),
                revision.as_str(),
            ]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "totp",
                item.as_str(),
                revision.to_lowercase().as_str(),
            ]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse(["conflict", "merge", "totp", item.as_str()]),
            Err(CliFailure::InvalidCommand)
        );
        // The TOTP selector is closed too: neither the record name nor a
        // longer spelling stands in for it.
        for alias in ["totp-seed", "otp", "TOTP"] {
            assert_eq!(
                parse(["conflict", "merge", alias, item.as_str(), revision.as_str()]),
                Err(CliFailure::InvalidCommand)
            );
        }
        assert_eq!(
            parse([
                "conflict",
                "merge",
                "opaque",
                item.as_str(),
                revision.to_lowercase().as_str(),
            ]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse(["conflict", "merge", "opaque", item.as_str()]),
            Err(CliFailure::InvalidCommand)
        );
        // So is the opaque selector. "custom" and "unknown" describe the same
        // records but are not the accepted spelling.
        for alias in ["custom", "unknown", "Opaque"] {
            assert_eq!(
                parse(["conflict", "merge", alias, item.as_str(), revision.as_str()]),
                Err(CliFailure::InvalidCommand)
            );
        }
    }

    #[test]
    fn audit_show_parser_requires_the_canonical_trace_id() {
        let trace_id = OperationId::new([0x7c; 32]);
        let canonical = trace_id.to_user_string();
        assert_eq!(
            parse(["audit", "show", canonical.as_str()]),
            default_invocation(Command::AuditShow { trace_id })
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
            default_invocation(Command::PortableExport {
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
            default_invocation(Command::PortableImport {
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
    fn portable_restore_verify_parser_accepts_exactly_one_explicit_source() {
        assert_eq!(
            parse(["restore", "verify", "backup.vpm"]),
            default_invocation(Command::PortableRestoreVerify {
                source: PathBuf::from("backup.vpm")
            })
        );
        assert_eq!(
            parse(["restore", "verify"]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse(["restore", "verify", "backup.vpm", "extra"]),
            Err(CliFailure::InvalidCommand)
        );
    }

    #[test]
    fn portable_restore_parser_requires_an_explicit_named_target() {
        let work = ConfigName::new("work".to_owned()).unwrap();
        assert_eq!(
            parse(["--vault", "work", "restore", "backup.vpm"]),
            Ok(Invocation {
                selected_vault: Some(work),
                command: Command::PortableRestore {
                    source: PathBuf::from("backup.vpm"),
                },
            })
        );
        assert_eq!(
            parse(["restore", "backup.vpm"]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse(["--vault", "work", "restore", "backup.vpm", "extra"]),
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
    fn named_target_creation_preserves_default_and_selection_is_isolated() {
        let root = TestRoot::new();
        let paths = root.paths();
        let personal_passphrase = b"personal target passphrase".to_vec();
        let work_passphrase = b"work target passphrase".to_vec();
        let init_host =
            TestHost::with_entropy_seed(paths.clone(), [personal_passphrase.clone()], 1);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let layout = paths.prepare().unwrap();
        let writer = layout.try_acquire_writer().unwrap();
        let exact_before_failure = writer.load_config().unwrap().unwrap();
        drop(writer);
        drop(layout);
        let failed_host =
            TestHost::without_entropy(paths.clone(), [b"unused target passphrase".to_vec()]);
        let failed = run(["vault", "create", "failed"], &failed_host);
        assert_eq!(failed.exit_code(), ExitCode::Provider, "{failed:?}");
        let layout = paths.prepare().unwrap();
        let writer = layout.try_acquire_writer().unwrap();
        assert_eq!(writer.load_config().unwrap().unwrap(), exact_before_failure);
        drop(writer);
        drop(layout);

        let create_host = TestHost::with_entropy_seed(paths.clone(), [work_passphrase.clone()], 19);
        let created = run(["vault", "create", "work"], &create_host);
        assert_eq!(created.exit_code(), ExitCode::Success, "{created:?}");
        assert_eq!(created.stdout(), "Vault target created.\n");

        let layout = paths.prepare().unwrap();
        let writer = layout.try_acquire_writer().unwrap();
        let exact = writer.load_config().unwrap().unwrap();
        let config = decode_config(&exact).unwrap();
        let personal_name = ConfigName::new("personal".to_owned()).unwrap();
        let work_name = ConfigName::new("work".to_owned()).unwrap();
        assert_eq!(config.default_vault(), &personal_name);
        assert_eq!(config.vaults().len(), 2);
        let personal = config.select_vault(Some(&personal_name)).unwrap();
        let work = config.select_vault(Some(&work_name)).unwrap();
        assert_ne!(personal.locator(), work.locator());
        assert_ne!(personal.local_store(), work.local_store());
        let work_storage = config.storage().get(work.local_store()).unwrap();
        assert_eq!(work_storage.kind(), StorageKind::Filesystem);
        assert_eq!(
            Path::new(work_storage.location().as_str()),
            target_object_root(&paths, work.locator().as_bytes())
        );
        drop(writer);
        drop(layout);

        let locked = TestHost::new(paths.clone(), []);
        assert_eq!(
            run(["--vault", "work", "status"], &locked).stdout(),
            "Status: locked\n"
        );
        let unknown = run(["--vault", "missing", "status"], &locked);
        assert_eq!(unknown.exit_code(), ExitCode::NotFound, "{unknown:?}");

        let verify_work = TestHost::with_entropy_seed(paths.clone(), [work_passphrase.clone()], 20);
        let verified = run(["--vault", "work", "audit", "verify"], &verify_work);
        assert_eq!(verified.exit_code(), ExitCode::Success, "{verified:?}");
        assert!(verified.stdout().contains("audit_events=1"));

        let add_work = TestHost::with_texts(
            paths.clone(),
            [
                work_passphrase.clone(),
                b"work-only secret".to_vec(),
                Vec::new(),
            ],
            [
                "Work-only login".to_owned(),
                "work@example.test".to_owned(),
                "0".to_owned(),
            ],
        );
        let added = run(["--vault", "work", "item", "add", "login"], &add_work);
        assert_eq!(added.exit_code(), ExitCode::Success, "{added:?}");

        let list_personal =
            TestHost::with_entropy_seed(paths.clone(), [personal_passphrase.clone()], 21);
        let personal_items = run(["item", "list"], &list_personal);
        assert_eq!(personal_items.stdout(), "No items.\n", "{personal_items:?}");

        let list_work = TestHost::with_entropy_seed(paths.clone(), [work_passphrase.clone()], 22);
        let work_items = run(["--vault", "work", "item", "list"], &list_work);
        assert_eq!(work_items.exit_code(), ExitCode::Success, "{work_items:?}");
        assert!(work_items.stdout().contains("Work-only login"));

        let search_personal =
            TestHost::with_entropy_seed(paths.clone(), [personal_passphrase.clone()], 24);
        let personal_matches = run(["search", "Work-only login"], &search_personal);
        assert_eq!(
            personal_matches.exit_code(),
            ExitCode::Success,
            "{personal_matches:?}"
        );
        assert_eq!(personal_matches.stdout(), "No matches.\n");

        let search_work = TestHost::with_entropy_seed(paths.clone(), [work_passphrase.clone()], 25);
        let work_matches = run(
            ["--vault", "work", "search", "Work-only login"],
            &search_work,
        );
        assert_eq!(
            work_matches.exit_code(),
            ExitCode::Success,
            "{work_matches:?}"
        );
        assert!(work_matches.stdout().contains("Work-only login"));

        let audit_work = TestHost::with_entropy_seed(paths.clone(), [work_passphrase.clone()], 23);
        let events = run(["--vault", "work", "audit", "list"], &audit_work);
        assert_eq!(events.exit_code(), ExitCode::Success, "{events:?}");
        assert!(events
            .stdout()
            .contains("\tcounter=1\taction=vault_initialize\toutcome=succeeded\t"));

        let duplicate = TestHost::new(paths, []);
        let repeated = run(["vault", "create", "work"], &duplicate);
        assert_eq!(repeated.exit_code(), ExitCode::InvalidInput);
        assert_eq!(repeated.stderr(), "vault-pm: already initialized\n");
    }

    #[test]
    fn selected_uncomposed_provider_fails_closed_without_config_payloads() {
        let root = TestRoot::new();
        let paths = root.paths();
        let init_host = TestHost::new(paths.clone(), [b"provider test passphrase".to_vec()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let layout = paths.prepare().unwrap();
        let writer = layout.try_acquire_writer().unwrap();
        let exact = writer.load_config().unwrap().unwrap();
        let config = decode_config(&exact).unwrap();
        let remote_store = ConfigName::new("remote".to_owned()).unwrap();
        let mut storage = config.storage().clone();
        storage.insert(
            remote_store.clone(),
            StorageConfigV1::new(
                StorageKind::GoogleDrive,
                StorageLocation::new("private-provider-folder").unwrap(),
                CredentialRef::new("keychain:private-provider-token").unwrap(),
            ),
        );
        let mut vaults = config.vaults().clone();
        vaults.insert(
            ConfigName::new("remote".to_owned()).unwrap(),
            VaultConfigV1::new(
                ConfigVaultLocator::new([0x91; 32]),
                remote_store,
                Vec::new(),
                DEFAULT_AUTO_LOCK_SECONDS,
                DEFAULT_CLIPBOARD_CLEAR_SECONDS,
            )
            .unwrap(),
        );
        vaults.insert(
            ConfigName::new("shared".to_owned()).unwrap(),
            VaultConfigV1::new(
                ConfigVaultLocator::new([0x92; 32]),
                config.select_vault(None).unwrap().local_store().clone(),
                Vec::new(),
                DEFAULT_AUTO_LOCK_SECONDS,
                DEFAULT_CLIPBOARD_CLEAR_SECONDS,
            )
            .unwrap(),
        );
        let replacement =
            VaultPmConfigV1::new(config.default_vault().clone(), vaults, storage).unwrap();
        writer
            .compare_exchange_config(&exact, render_config(&replacement).as_bytes())
            .unwrap();
        drop(writer);
        drop(layout);

        let host = TestHost::new(paths, []);
        let output = run(["--vault", "remote", "status"], &host);
        assert_eq!(output.exit_code(), ExitCode::Unsupported, "{output:?}");
        assert_eq!(output.stderr(), "vault-pm: unsupported capability\n");
        assert!(!output.stderr().contains("private-provider"));
        assert!(!output.stderr().contains("keychain"));

        let shared = run(["--vault", "shared", "status"], &host);
        assert_eq!(shared.exit_code(), ExitCode::Unsupported, "{shared:?}");
        assert_eq!(shared.stderr(), "vault-pm: unsupported capability\n");
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
            "Audit: verified (announcements=1 commits=1 catalogs=1 revisions=0 items=0 audit_events=1)\n"
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
            [
                passphrase.clone(),
                b"audited item secret".to_vec(),
                Vec::new(),
            ],
            [
                "Audited CLI item".to_owned(),
                "user@example.test".to_owned(),
                "0".to_owned(),
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
            vec!["search", "user@example.test"],
        ] {
            let host = TestHost::new(paths.clone(), [passphrase.clone()]);
            let output = run(arguments, &host);
            assert_eq!(output.exit_code(), ExitCode::Success, "{output:?}");
            assert!(!output.stdout().contains("audited item secret"));
        }

        for invalid_query in [String::new(), "line\nbreak".to_owned(), "x".repeat(257)] {
            let invalid_host = TestHost::new(paths.clone(), [passphrase.clone()]);
            let invalid = run(["search".to_owned(), invalid_query], &invalid_host);
            assert_eq!(invalid.exit_code(), ExitCode::InvalidInput, "{invalid:?}");
            assert!(invalid.stdout().is_empty());
        }

        let secret_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let secret = run(["search", "audited item secret"], &secret_host);
        assert_eq!(secret.exit_code(), ExitCode::Success, "{secret:?}");
        assert_eq!(secret.stdout(), "No matches.\n");
        assert!(!secret.stdout().contains("audited item secret"));

        let audit_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let audit = run(["audit", "verify"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert!(audit.stdout().contains("commits=10"), "{audit:?}");
        assert!(audit.stdout().contains("audit_events=10"), "{audit:?}");

        let list_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let listed = run(["audit", "list"], &list_host);
        assert_eq!(listed.exit_code(), ExitCode::Success, "{listed:?}");
        assert!(listed
            .stdout()
            .contains("action=item_search\toutcome=succeeded"));
        assert!(listed
            .stdout()
            .contains("action=item_search\toutcome=failed"));
        for forbidden in [
            "user@example.test",
            "audited item secret",
            "private portal metadata",
        ] {
            assert!(!listed.stdout().contains(forbidden), "{listed:?}");
        }

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
            final_audit.stdout().contains("commits=13"),
            "{final_audit:?}"
        );
        assert!(
            final_audit.stdout().contains("audit_events=13"),
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

        let invalid_count_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), b"uncommitted secret".to_vec()],
            [
                "Invalid URL count".to_owned(),
                "invalid@example.test".to_owned(),
                "17".to_owned(),
            ],
        );
        let invalid_count = run(["item", "add", "login"], &invalid_count_host);
        assert_eq!(
            invalid_count.exit_code(),
            ExitCode::InvalidInput,
            "{invalid_count:?}"
        );
        assert!(invalid_count.stdout().is_empty());

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
        assert_eq!(
            audit
                .stdout()
                .matches("action=item_create\toutcome=failed")
                .count(),
            2,
            "{audit:?}"
        );
        assert!(!audit.stdout().contains("audited create failure passphrase"));
        assert!(!audit.stdout().contains("uncommitted secret"));
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
    fn portable_import_logs_failure_then_restores_audit_first_target_independently() {
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

        let mismatched_target_host = TestHost::with_entropy_seed(
            target_paths.clone(),
            [target_passphrase.clone(), export_passphrase.clone()],
            33,
        );
        let mismatched_target = run(
            [
                "restore",
                "verify",
                artifact_path.to_str().expect("UTF-8 test artifact path"),
            ],
            &mismatched_target_host,
        );
        assert_eq!(
            mismatched_target.exit_code(),
            ExitCode::Integrity,
            "{mismatched_target:?}"
        );
        assert!(mismatched_target.stdout().is_empty());

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

        let wrong_verify_host = TestHost::with_entropy_seed(
            target_paths.clone(),
            [
                target_passphrase.clone(),
                b"wrong verification passphrase".to_vec(),
            ],
            51,
        );
        let wrong_verify = run(
            [
                "restore",
                "verify",
                artifact_path.to_str().expect("UTF-8 test artifact path"),
            ],
            &wrong_verify_host,
        );
        assert_eq!(
            wrong_verify.exit_code(),
            ExitCode::Locked,
            "{wrong_verify:?}"
        );
        assert!(wrong_verify.stdout().is_empty());

        let verify_host = TestHost::with_entropy_seed(
            target_paths.clone(),
            [target_passphrase.clone(), export_passphrase.clone()],
            61,
        );
        let verified = run(
            [
                "restore",
                "verify",
                artifact_path.to_str().expect("UTF-8 test artifact path"),
            ],
            &verify_host,
        );
        assert_eq!(verified.exit_code(), ExitCode::Success, "{verified:?}");
        assert_eq!(
            verified.stdout(),
            "Portable restore verified: items=1 candidates=1 conflicts=0.\n"
        );

        let list_host = TestHost::new(target_paths.clone(), [target_passphrase.clone()]);
        let listed = run(["item", "list"], &list_host);
        assert_eq!(listed.exit_code(), ExitCode::Success, "{listed:?}");
        assert!(listed
            .stdout()
            .contains("vault/note/v1\t\"Restored secure note\""));
        assert!(!listed.stdout().contains("restored secure note body"));

        let audit_host = TestHost::with_entropy_seed(target_paths, [target_passphrase], 71);
        let audit = run(["audit", "list"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert!(audit
            .stdout()
            .contains("action=portable_import\toutcome=failed"));
        assert!(audit
            .stdout()
            .contains("action=portable_import\toutcome=succeeded"));
        assert!(audit
            .stdout()
            .contains("action=portable_restore_verify\toutcome=failed"));
        assert!(audit
            .stdout()
            .contains("action=portable_restore_verify\toutcome=succeeded"));
        assert!(!audit.stdout().contains("restore-source.vpm"));
        assert!(!audit.stdout().contains("portable artifact passphrase"));
        assert!(!audit.stdout().contains("wrong verification passphrase"));

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
    fn portable_restore_composes_import_and_independent_verification_on_named_target() {
        let root = TestRoot::new();
        let paths = root.paths();
        let source_passphrase = b"composed restore source passphrase".to_vec();
        let target_passphrase = b"composed restore target passphrase".to_vec();
        let artifact_passphrase = b"composed restore artifact passphrase".to_vec();
        let init_source = TestHost::new(paths.clone(), [source_passphrase.clone()]);
        assert_eq!(run(["init"], &init_source).exit_code(), ExitCode::Success);
        let add_source = TestHost::with_texts(
            paths.clone(),
            [
                source_passphrase.clone(),
                b"composed restore note body".to_vec(),
            ],
            ["Composed restore note".to_owned()],
        );
        assert_eq!(
            run(["item", "add", "secure-note"], &add_source).exit_code(),
            ExitCode::Success
        );
        let artifact_path = root.0.join("composed-restore.vpm");
        let export_host = TestHost::new(
            paths.clone(),
            [source_passphrase.clone(), artifact_passphrase.clone()],
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
        let exact_artifact = fs::read(&artifact_path).unwrap();

        let create_target =
            TestHost::with_entropy_seed(paths.clone(), [target_passphrase.clone()], 79);
        let created = run(["vault", "create", "restore"], &create_target);
        assert_eq!(created.exit_code(), ExitCode::Success, "{created:?}");

        let selected_default = TestHost::new(
            paths.clone(),
            [
                source_passphrase.clone(),
                artifact_passphrase.clone(),
                source_passphrase.clone(),
            ],
        );
        let rejected = run(
            [
                "--vault",
                "personal",
                "restore",
                artifact_path.to_str().expect("UTF-8 test artifact path"),
            ],
            &selected_default,
        );
        assert_eq!(rejected.exit_code(), ExitCode::InvalidInput, "{rejected:?}");

        let unused_secret = b"must remain unused".to_vec();
        let restore_host = TestHost::with_entropy_seed(
            paths.clone(),
            [
                target_passphrase.clone(),
                artifact_passphrase.clone(),
                target_passphrase.clone(),
                unused_secret.clone(),
            ],
            83,
        );
        let restored = run(
            [
                "--vault",
                "restore",
                "restore",
                artifact_path.to_str().expect("UTF-8 test artifact path"),
            ],
            &restore_host,
        );
        assert_eq!(restored.exit_code(), ExitCode::Success, "{restored:?}");
        assert_eq!(
            restored.stdout(),
            "Portable restore completed and verified: items=1 candidates=1 conflicts=0.\n"
        );
        assert_eq!(&*restore_host.secret().unwrap(), unused_secret.as_slice());
        assert_eq!(fs::read(&artifact_path).unwrap(), exact_artifact);

        let target_list = TestHost::new(paths.clone(), [target_passphrase.clone()]);
        let listed = run(["--vault", "restore", "item", "list"], &target_list);
        assert_eq!(listed.exit_code(), ExitCode::Success, "{listed:?}");
        assert!(listed.stdout().contains("Composed restore note"));
        assert!(!listed.stdout().contains("composed restore note body"));

        let target_audit = TestHost::with_entropy_seed(paths.clone(), [target_passphrase], 89);
        let audit = run(["--vault", "restore", "audit", "list"], &target_audit);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert!(audit
            .stdout()
            .contains("action=portable_import\toutcome=succeeded"));
        assert!(audit
            .stdout()
            .contains("action=portable_restore_verify\toutcome=succeeded"));
        assert!(!audit.stdout().contains("composed-restore.vpm"));
        assert!(!audit
            .stdout()
            .contains("composed restore artifact passphrase"));
        assert!(!audit
            .stdout()
            .contains("composed restore target passphrase"));

        let source_list = TestHost::new(paths, [source_passphrase]);
        let source_items = run(["item", "list"], &source_list);
        assert_eq!(
            source_items.exit_code(),
            ExitCode::Success,
            "{source_items:?}"
        );
        assert!(source_items.stdout().contains("Composed restore note"));
    }

    #[test]
    fn portable_restore_interruption_after_import_uses_standalone_verification_retry() {
        let root = TestRoot::new();
        let paths = root.paths();
        let source_passphrase = b"retry source passphrase".to_vec();
        let target_passphrase = b"retry target passphrase".to_vec();
        let artifact_passphrase = b"retry artifact passphrase".to_vec();
        let init_source = TestHost::new(paths.clone(), [source_passphrase.clone()]);
        assert_eq!(run(["init"], &init_source).exit_code(), ExitCode::Success);
        let add_source = TestHost::with_texts(
            paths.clone(),
            [source_passphrase.clone(), b"retry note body".to_vec()],
            ["Retry note".to_owned()],
        );
        assert_eq!(
            run(["item", "add", "secure-note"], &add_source).exit_code(),
            ExitCode::Success
        );
        let artifact_path = root.0.join("retry-restore.vpm");
        let export_host = TestHost::new(
            paths.clone(),
            [source_passphrase, artifact_passphrase.clone()],
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
        let create_target =
            TestHost::with_entropy_seed(paths.clone(), [target_passphrase.clone()], 97);
        assert_eq!(
            run(["vault", "create", "retry"], &create_target).exit_code(),
            ExitCode::Success
        );

        let interrupted_host = TestHost::with_entropy_seed(
            paths.clone(),
            [
                target_passphrase.clone(),
                artifact_passphrase.clone(),
                b"wrong second unlock".to_vec(),
            ],
            101,
        );
        let interrupted = run(
            [
                "--vault",
                "retry",
                "restore",
                artifact_path.to_str().expect("UTF-8 test artifact path"),
            ],
            &interrupted_host,
        );
        assert_eq!(interrupted.exit_code(), ExitCode::Locked, "{interrupted:?}");
        assert!(interrupted.stdout().is_empty());

        let repeated_import = TestHost::with_entropy_seed(
            paths.clone(),
            [target_passphrase.clone(), artifact_passphrase.clone()],
            103,
        );
        let repeated = run(
            [
                "--vault",
                "retry",
                "import",
                artifact_path.to_str().expect("UTF-8 test artifact path"),
            ],
            &repeated_import,
        );
        assert_eq!(repeated.exit_code(), ExitCode::InvalidInput, "{repeated:?}");
        assert!(repeated.stdout().is_empty());

        let retry_host = TestHost::with_entropy_seed(
            paths.clone(),
            [target_passphrase.clone(), artifact_passphrase],
            107,
        );
        let verified = run(
            [
                "--vault",
                "retry",
                "restore",
                "verify",
                artifact_path.to_str().expect("UTF-8 test artifact path"),
            ],
            &retry_host,
        );
        assert_eq!(verified.exit_code(), ExitCode::Success, "{verified:?}");
        assert_eq!(
            verified.stdout(),
            "Portable restore verified: items=1 candidates=1 conflicts=0.\n"
        );

        let audit_host = TestHost::with_entropy_seed(paths, [target_passphrase], 109);
        let audit = run(["--vault", "retry", "audit", "list"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert_eq!(
            audit
                .stdout()
                .matches("action=portable_import\toutcome=succeeded")
                .count(),
            1
        );
        assert!(audit
            .stdout()
            .contains("action=portable_import\toutcome=failed"));
        assert!(audit
            .stdout()
            .contains("action=portable_restore_verify\toutcome=succeeded"));
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
            [passphrase.clone(), b"delete secret".to_vec(), Vec::new()],
            [
                "Delete me".to_owned(),
                "user@example.test".to_owned(),
                "0".to_owned(),
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
        assert!(audit.stdout().contains("commits=4"), "{audit:?}");
        assert!(audit.stdout().contains("audit_events=4"), "{audit:?}");
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
            [passphrase.clone(), b"restore secret".to_vec(), Vec::new()],
            [
                "Restore me".to_owned(),
                "user@example.test".to_owned(),
                "0".to_owned(),
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
        assert!(audit.stdout().contains("commits=7"), "{audit:?}");
        assert!(audit.stdout().contains("audit_events=7"), "{audit:?}");
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
            [passphrase.clone(), b"original secret".to_vec(), Vec::new()],
            [
                "Edit me".to_owned(),
                "original@example.test".to_owned(),
                "0".to_owned(),
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

        let invalid_count_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), b"uncommitted replacement".to_vec()],
            [
                "Invalid edit".to_owned(),
                "invalid-edit@example.test".to_owned(),
                "17".to_owned(),
            ],
        );
        let invalid_count = run(["item", "edit", item_id.as_str()], &invalid_count_host);
        assert_eq!(
            invalid_count.exit_code(),
            ExitCode::InvalidInput,
            "{invalid_count:?}"
        );
        assert!(invalid_count.stdout().is_empty());

        let replacement = b"replacement secret".to_vec();
        let edit_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), replacement, Vec::new()],
            [
                "Edited".to_owned(),
                "edited@example.test".to_owned(),
                "0".to_owned(),
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
        assert!(audit.stdout().contains("audit_events=5"), "{audit:?}");
        assert!(!audit.stdout().contains("uncommitted replacement"));
    }

    #[test]
    fn login_url_count_accepts_only_canonical_values_within_the_bound() {
        for value in ["0", "1", "9", "10", "16"] {
            assert_eq!(parse_login_url_count(value), Ok(value.parse().unwrap()));
        }
        for value in ["", "00", "01", "+1", "-1", " 1", "1 ", "17", "999"] {
            assert_eq!(
                parse_login_url_count(value),
                Err(CliFailure::InvalidCommand),
                "unexpected URL count result for {value:?}"
            );
        }
    }

    #[test]
    fn login_add_list_and_show_survive_restart_without_rendering_password() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"correct horse battery staple".to_vec();
        let password = b"item password must stay secret".to_vec();
        let notes = b"private recovery details stay secret".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let add_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), password.clone(), notes.clone()],
            [
                "Example account".to_string(),
                "ada@example.test".to_string(),
                "2".to_string(),
                "https://example.test".to_string(),
                "https://accounts.example.test/login".to_string(),
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
                "Item: {expected_id}\nType: {LOGIN_V1}\nTitle: \"Example account\"\nUsername: \"ada@example.test\"\nURL: \"https://example.test\"\nURL: \"https://accounts.example.test/login\"\nPassword: <redacted>\nNotes: present\nFavorite: no\nUpdated: 1700000000000\n"
            )
        );
        assert!(!shown
            .stdout()
            .contains(core::str::from_utf8(&password).unwrap()));
        assert!(!shown
            .stdout()
            .contains(core::str::from_utf8(&notes).unwrap()));

        let updated_password = b"replacement password stays secret".to_vec();
        let edit_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), updated_password.clone(), Vec::new()],
            [
                "Updated account".to_string(),
                "grace@example.test".to_string(),
                "0".to_string(),
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
        for secret in [&password, &updated_password, &notes] {
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
        for secret in [&password, &updated_password, &notes] {
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
            default_invocation(Command::ItemAddSecureNote)
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
    fn card_create_failures_and_success_are_audited_without_secret_rendering() {
        assert_eq!(
            parse(["item", "add", "card"]),
            default_invocation(Command::ItemAddCard)
        );
        assert_eq!(
            parse(["--vault", "work", "item", "add", "card"]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ItemAddCard,
            })
        );
        assert_eq!(parse_card_expiry_month("1"), Ok(1));
        assert_eq!(parse_card_expiry_month("12"), Ok(12));
        assert_eq!(
            parse_card_expiry_month("01"),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(parse_card_expiry_year("2030"), Ok(2030));
        assert_eq!(
            parse_card_expiry_year("0000"),
            Err(CliFailure::InvalidCommand)
        );

        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"card create passphrase".to_vec();
        let number = b"4242424242424242".to_vec();
        let cvv = b"123".to_vec();
        let postal_code = "94107";
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let unavailable_host = TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 11);
        let unavailable = run(["item", "add", "card"], &unavailable_host);
        assert_eq!(
            unavailable.exit_code(),
            ExitCode::Provider,
            "{unavailable:?}"
        );
        assert!(unavailable.stdout().is_empty());

        let invalid_utf8_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone(), vec![0xff]],
            ["Personal Visa".to_owned(), "Ada Lovelace".to_owned()],
            19,
        );
        let invalid_utf8 = run(["item", "add", "card"], &invalid_utf8_host);
        assert_eq!(
            invalid_utf8.exit_code(),
            ExitCode::InvalidInput,
            "{invalid_utf8:?}"
        );
        assert!(invalid_utf8.stdout().is_empty());

        let invalid_attempt =
            |seed, attempted_number: &[u8], attempted_cvv: &[u8], month: &str, year: &str| {
                let host = TestHost::with_texts_and_entropy_seed(
                    paths.clone(),
                    [
                        passphrase.clone(),
                        attempted_number.to_vec(),
                        attempted_cvv.to_vec(),
                    ],
                    [
                        "Personal Visa".to_owned(),
                        "Ada Lovelace".to_owned(),
                        month.to_owned(),
                        year.to_owned(),
                        postal_code.to_owned(),
                    ],
                    seed,
                );
                let output = run(["item", "add", "card"], &host);
                assert_eq!(output.exit_code(), ExitCode::InvalidInput, "{output:?}");
                assert!(output.stdout().is_empty());
            };
        invalid_attempt(23, b"42", &cvv, "12", "2030");
        invalid_attempt(31, &number, b"12", "12", "2030");
        invalid_attempt(43, &number, &cvv, "01", "2030");
        invalid_attempt(47, &number, &cvv, "12", "0000");

        let add_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone(), number.clone(), cvv.clone()],
            [
                "Personal Visa".to_owned(),
                "Ada Lovelace".to_owned(),
                "12".to_owned(),
                "2030".to_owned(),
                postal_code.to_owned(),
            ],
            59,
        );
        let added = run(["item", "add", "card"], &add_host);
        assert_eq!(added.exit_code(), ExitCode::Success, "{added:?}");
        let item = added
            .stdout()
            .strip_prefix("Item added: ")
            .and_then(|value| value.strip_suffix('\n'))
            .unwrap();
        assert!(ItemId::from_user_string(item).is_ok());

        let list_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let listed = run(["item", "list"], &list_host);
        assert_eq!(listed.exit_code(), ExitCode::Success, "{listed:?}");
        assert_eq!(
            listed.stdout(),
            format!("{item}\t{CARD_V1}\t\"Personal Visa\"\n")
        );

        let show_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let shown = run(["item", "show", item], &show_host);
        assert_eq!(shown.exit_code(), ExitCode::Success, "{shown:?}");
        assert_eq!(
            shown.stdout(),
            format!(
                "Item: {item}\nType: {CARD_V1}\nTitle: \"Personal Visa\"\nCardholder: \"Ada Lovelace\"\nLast four: \"4242\"\nExpiry: 12/2030\nCard number: <redacted>\nCVV: <redacted>\nBilling postal code: present\nFavorite: no\nUpdated: 1700000000000\n"
            )
        );
        for secret in [&number, &cvv] {
            assert!(!shown
                .stdout()
                .contains(core::str::from_utf8(secret).unwrap()));
        }
        assert!(!shown.stdout().contains(postal_code));

        let number_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone()],
            ["yes".to_owned()],
            61,
        );
        let revealed_number = run(["item", "reveal", item, "card-number"], &number_host);
        assert_eq!(
            revealed_number.exit_code(),
            ExitCode::Success,
            "{revealed_number:?}"
        );
        assert!(revealed_number.stdout().is_empty());
        assert!(number_host.revealed_equals(&number));

        let cvv_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone()],
            ["yes".to_owned()],
            67,
        );
        let revealed_cvv = run(["item", "reveal", item, "card-cvv"], &cvv_host);
        assert_eq!(
            revealed_cvv.exit_code(),
            ExitCode::Success,
            "{revealed_cvv:?}"
        );
        assert!(revealed_cvv.stdout().is_empty());
        assert!(cvv_host.revealed_equals(&cvv));

        let audit_host = TestHost::with_entropy_seed(paths, [passphrase], 71);
        let audit = run(["audit", "list"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert_eq!(
            audit
                .stdout()
                .lines()
                .filter(|line| line.contains("action=item_create\toutcome=failed"))
                .count(),
            6,
            "{audit:?}"
        );
        assert!(audit.stdout().lines().any(|line| {
            line.contains("action=item_create\toutcome=succeeded")
                && line.contains(&format!("\titem={item}"))
        }));
        for value in [
            "Personal Visa",
            "Ada Lovelace",
            core::str::from_utf8(&number).unwrap(),
            core::str::from_utf8(&cvv).unwrap(),
            postal_code,
        ] {
            assert!(!audit.stdout().contains(value));
        }
    }

    #[test]
    fn api_key_create_failures_and_success_are_audited_without_token_rendering() {
        assert_eq!(
            parse(["item", "add", "api-key"]),
            default_invocation(Command::ItemAddApiKey)
        );
        assert_eq!(
            parse(["--vault", "work", "item", "add", "api-key"]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ItemAddApiKey,
            })
        );
        assert_eq!(
            parse(["item", "add", "api-key", "secret-in-argv"]),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(parse_api_key_scopes(""), Ok(Vec::new()));
        assert_eq!(
            parse_api_key_scopes("read:issues,write:comments"),
            Ok(vec!["read:issues".to_owned(), "write:comments".to_owned()])
        );
        for invalid in [
            "read:issues, read:users",
            "read:issues,read:issues",
            ",read",
        ] {
            assert_eq!(
                parse_api_key_scopes(invalid),
                Err(CliFailure::InvalidCommand)
            );
        }
        assert_eq!(
            parse_api_key_scopes(&"x".repeat(257)),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(
            parse_api_key_scopes(&vec!["scope"; 65].join(",")),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(parse_optional_unix_seconds(""), Ok(None));
        assert_eq!(
            parse_optional_unix_seconds("1893456000"),
            Ok(Some(1_893_456_000))
        );
        for invalid in ["0", "01", "+1", "1.0", "18446744073709551616"] {
            assert_eq!(
                parse_optional_unix_seconds(invalid),
                Err(CliFailure::InvalidCommand)
            );
        }

        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"api key create passphrase".to_vec();
        let token = b"vlt_e2e_4d0a6b7335c9f428f00b8f75265f19d7".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let unavailable_host = TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 11);
        let unavailable = run(["item", "add", "api-key"], &unavailable_host);
        assert_eq!(
            unavailable.exit_code(),
            ExitCode::Provider,
            "{unavailable:?}"
        );
        assert!(unavailable.stdout().is_empty());

        let invalid_utf8_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone(), vec![0xff]],
            ["Automation key".to_owned(), "api.example.test".to_owned()],
            19,
        );
        let invalid_utf8 = run(["item", "add", "api-key"], &invalid_utf8_host);
        assert_eq!(
            invalid_utf8.exit_code(),
            ExitCode::InvalidInput,
            "{invalid_utf8:?}"
        );
        assert!(invalid_utf8.stdout().is_empty());

        let invalid_attempt = |seed, scopes: &str, expiry: &str| {
            let host = TestHost::with_texts_and_entropy_seed(
                paths.clone(),
                [passphrase.clone(), token.clone()],
                [
                    "Automation key".to_owned(),
                    "api.example.test".to_owned(),
                    scopes.to_owned(),
                    expiry.to_owned(),
                ],
                seed,
            );
            let output = run(["item", "add", "api-key"], &host);
            assert_eq!(output.exit_code(), ExitCode::InvalidInput, "{output:?}");
            assert!(output.stdout().is_empty());
        };
        invalid_attempt(23, "read:issues, read:users", "1893456000");
        invalid_attempt(31, "read:issues", "01");

        let add_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone(), token.clone()],
            [
                "Automation key".to_owned(),
                "api.example.test".to_owned(),
                "read:issues,write:comments".to_owned(),
                "1893456000".to_owned(),
            ],
            43,
        );
        let added = run(["item", "add", "api-key"], &add_host);
        assert_eq!(added.exit_code(), ExitCode::Success, "{added:?}");
        let item = added
            .stdout()
            .strip_prefix("Item added: ")
            .and_then(|value| value.strip_suffix('\n'))
            .unwrap();
        assert!(ItemId::from_user_string(item).is_ok());

        let list_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let listed = run(["item", "list"], &list_host);
        assert_eq!(listed.exit_code(), ExitCode::Success, "{listed:?}");
        assert_eq!(
            listed.stdout(),
            format!("{item}\t{API_KEY_V1}\t\"Automation key\"\n")
        );
        assert!(!listed
            .stdout()
            .contains(core::str::from_utf8(&token).unwrap()));

        let show_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let shown = run(["item", "show", item], &show_host);
        assert_eq!(shown.exit_code(), ExitCode::Success, "{shown:?}");
        assert_eq!(
            shown.stdout(),
            format!(
                "Item: {item}\nType: {API_KEY_V1}\nLabel: \"Automation key\"\nService: \"api.example.test\"\nScope: \"read:issues\"\nScope: \"write:comments\"\nExpiry: 1893456000\nToken: <redacted>\nFavorite: no\nUpdated: 1700000000000\n"
            )
        );
        assert!(!shown
            .stdout()
            .contains(core::str::from_utf8(&token).unwrap()));

        let reveal_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone()],
            ["yes".to_owned()],
            47,
        );
        let revealed = run(["item", "reveal", item, "api-key-token"], &reveal_host);
        assert_eq!(revealed.exit_code(), ExitCode::Success, "{revealed:?}");
        assert!(revealed.stdout().is_empty());
        assert!(reveal_host.revealed_equals(&token));

        let audit_host = TestHost::with_entropy_seed(paths, [passphrase], 59);
        let audit = run(["audit", "list"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert_eq!(
            audit
                .stdout()
                .lines()
                .filter(|line| line.contains("action=item_create\toutcome=failed"))
                .count(),
            4,
            "{audit:?}"
        );
        assert!(audit.stdout().lines().any(|line| {
            line.contains("action=item_create\toutcome=succeeded")
                && line.contains(&format!("\titem={item}"))
        }));
        for value in [
            "Automation key",
            "api.example.test",
            "read:issues",
            "write:comments",
            "1893456000",
            core::str::from_utf8(&token).unwrap(),
        ] {
            assert!(!audit.stdout().contains(value));
        }
    }

    #[test]
    fn database_create_failures_and_success_are_audited_without_password_rendering() {
        assert_eq!(
            parse(["item", "add", "database-credential"]),
            default_invocation(Command::ItemAddDatabaseCredential)
        );
        assert_eq!(
            parse(["--vault", "work", "item", "add", "database-credential"]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ItemAddDatabaseCredential,
            })
        );
        assert_eq!(
            parse(["item", "add", "database-credential", "password"]),
            Err(CliFailure::InvalidCommand)
        );
        for valid in ["postgres", "mysql8", "cockroach-db", "sql_server"] {
            assert_eq!(validate_database_engine(valid), Ok(()));
        }
        for invalid in ["", "Postgres", "9postgres", "postgres.db"] {
            assert_eq!(
                validate_database_engine(invalid),
                Err(CliFailure::InvalidCommand)
            );
        }
        assert_eq!(
            validate_database_engine(&"p".repeat(33)),
            Err(CliFailure::InvalidCommand)
        );
        assert_eq!(parse_database_port("5432"), Ok(5432));
        for invalid in ["", "0", "05432", "+5432", "65536"] {
            assert_eq!(
                parse_database_port(invalid),
                Err(CliFailure::InvalidCommand)
            );
        }

        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"database create passphrase".to_vec();
        let password = b"db_e2e_9f82ac14d76943ffac06b43a7d9c58de".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let unavailable_host = TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 11);
        assert_eq!(
            run(["item", "add", "database-credential"], &unavailable_host).exit_code(),
            ExitCode::Provider
        );

        let texts = || {
            [
                "Production reporting".to_owned(),
                "postgres".to_owned(),
                "db.internal.example".to_owned(),
                "5432".to_owned(),
                "analytics".to_owned(),
                "reporter".to_owned(),
            ]
        };
        let invalid_utf8_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone(), vec![0xff]],
            texts(),
            19,
        );
        assert_eq!(
            run(["item", "add", "database-credential"], &invalid_utf8_host).exit_code(),
            ExitCode::InvalidInput
        );

        let invalid_attempt = |seed, engine: &str, port: &str| {
            let mut values = texts();
            values[1] = engine.to_owned();
            values[3] = port.to_owned();
            let host = TestHost::with_texts_and_entropy_seed(
                paths.clone(),
                [passphrase.clone(), password.clone()],
                values,
                seed,
            );
            let output = run(["item", "add", "database-credential"], &host);
            assert_eq!(output.exit_code(), ExitCode::InvalidInput, "{output:?}");
            assert!(output.stdout().is_empty());
        };
        invalid_attempt(23, "Postgres", "5432");
        invalid_attempt(31, "postgres", "05432");

        let add_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone(), password.clone(), Vec::new()],
            texts(),
            43,
        );
        let added = run(["item", "add", "database-credential"], &add_host);
        assert_eq!(added.exit_code(), ExitCode::Success, "{added:?}");
        let item = added
            .stdout()
            .strip_prefix("Item added: ")
            .and_then(|value| value.strip_suffix('\n'))
            .unwrap();

        let listed = run(
            ["item", "list"],
            &TestHost::new(paths.clone(), [passphrase.clone()]),
        );
        assert_eq!(
            listed.stdout(),
            format!("{item}\t{DATABASE_CREDENTIAL_V1}\t\"Production reporting\"\n")
        );

        let shown = run(
            ["item", "show", item],
            &TestHost::new(paths.clone(), [passphrase.clone()]),
        );
        assert_eq!(shown.exit_code(), ExitCode::Success, "{shown:?}");
        assert_eq!(
            shown.stdout(),
            format!(
                "Item: {item}\nType: {DATABASE_CREDENTIAL_V1}\nLabel: \"Production reporting\"\nEngine: \"postgres\"\nHost: \"db.internal.example\"\nPort: 5432\nDatabase: \"analytics\"\nUsername: \"reporter\"\nLease: absent\nExpiry: none\nPassword: <redacted>\nFavorite: no\nUpdated: 1700000000000\n"
            )
        );
        assert!(!shown
            .stdout()
            .contains(core::str::from_utf8(&password).unwrap()));

        let reveal_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone()],
            ["yes".to_owned()],
            47,
        );
        let revealed = run(["item", "reveal", item, "database-password"], &reveal_host);
        assert_eq!(revealed.exit_code(), ExitCode::Success, "{revealed:?}");
        assert!(revealed.stdout().is_empty());
        assert!(reveal_host.revealed_equals(&password));

        let audit = run(
            ["audit", "list"],
            &TestHost::with_entropy_seed(paths, [passphrase], 59),
        );
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert_eq!(
            audit
                .stdout()
                .lines()
                .filter(|line| line.contains("action=item_create\toutcome=failed"))
                .count(),
            4,
            "{audit:?}"
        );
        for value in [
            "Production reporting",
            "postgres",
            "db.internal.example",
            "analytics",
            "reporter",
            core::str::from_utf8(&password).unwrap(),
        ] {
            assert!(!audit.stdout().contains(value));
        }
    }

    #[test]
    fn totp_create_failures_and_success_are_audited_without_seed_rendering() {
        assert_eq!(
            parse(["item", "add", "totp"]),
            default_invocation(Command::ItemAddTotp)
        );
        assert_eq!(
            parse(["--vault", "work", "item", "add", "totp"]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("work".to_owned()).unwrap()),
                command: Command::ItemAddTotp,
            })
        );
        assert_eq!(
            parse(["item", "add", "totp", "SECRET"]),
            Err(CliFailure::InvalidCommand)
        );
        for (encoded, decoded) in [
            ("MY", &b"f"[..]),
            ("MZXQ", &b"fo"[..]),
            ("MZXW6", &b"foo"[..]),
            ("MZXW6YQ", &b"foob"[..]),
            ("MZXW6YTB", &b"fooba"[..]),
            ("MZXW6YTBOI", &b"foobar"[..]),
        ] {
            let value = decode_totp_base32(encoded).unwrap();
            assert_eq!(value.as_slice(), decoded);
            assert_eq!(encode_totp_base32(&value).as_str(), encoded);
        }
        for invalid in ["", "A", "AAA", "AB", "my", "MY=", "M1"] {
            assert!(matches!(
                decode_totp_base32(invalid),
                Err(CliFailure::InvalidCommand)
            ));
        }
        assert!(matches!(
            decode_totp_base32(&"A".repeat(257)),
            Err(CliFailure::InvalidCommand)
        ));
        assert_eq!(parse_totp_period("30"), Ok(30));
        for invalid in ["", "0", "030", "+30", "3601"] {
            assert_eq!(parse_totp_period(invalid), Err(CliFailure::InvalidCommand));
        }

        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"totp create passphrase".to_vec();
        let seed = b"GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let unavailable_host = TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 11);
        assert_eq!(
            run(["item", "add", "totp"], &unavailable_host).exit_code(),
            ExitCode::Provider
        );

        let texts = || {
            [
                "GitHub ada@example.com".to_owned(),
                "GitHub".to_owned(),
                "SHA1".to_owned(),
                "6".to_owned(),
                "30".to_owned(),
            ]
        };
        let invalid_utf8_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone(), vec![0xff]],
            texts(),
            19,
        );
        assert_eq!(
            run(["item", "add", "totp"], &invalid_utf8_host).exit_code(),
            ExitCode::InvalidInput
        );

        let invalid_attempt =
            |seed_value, secret_value: &[u8], algorithm: &str, digits: &str, period: &str| {
                let mut values = texts();
                values[2] = algorithm.to_owned();
                values[3] = digits.to_owned();
                values[4] = period.to_owned();
                let host = TestHost::with_texts_and_entropy_seed(
                    paths.clone(),
                    [passphrase.clone(), secret_value.to_vec()],
                    values,
                    seed_value,
                );
                let output = run(["item", "add", "totp"], &host);
                assert_eq!(output.exit_code(), ExitCode::InvalidInput, "{output:?}");
                assert!(output.stdout().is_empty());
            };
        invalid_attempt(23, b"my", "SHA1", "6", "30");
        invalid_attempt(29, &seed, "sha1", "6", "30");
        invalid_attempt(31, &seed, "SHA1", "7", "30");
        invalid_attempt(37, &seed, "SHA1", "6", "030");

        let add_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone(), seed.clone()],
            texts(),
            43,
        );
        let added = run(["item", "add", "totp"], &add_host);
        assert_eq!(added.exit_code(), ExitCode::Success, "{added:?}");
        let item = added
            .stdout()
            .strip_prefix("Item added: ")
            .and_then(|value| value.strip_suffix('\n'))
            .unwrap();

        let listed = run(
            ["item", "list"],
            &TestHost::new(paths.clone(), [passphrase.clone()]),
        );
        assert_eq!(
            listed.stdout(),
            format!("{item}\t{TOTP_SEED_V1}\t\"GitHub ada@example.com\"\n")
        );

        let shown = run(
            ["item", "show", item],
            &TestHost::new(paths.clone(), [passphrase.clone()]),
        );
        assert_eq!(shown.exit_code(), ExitCode::Success, "{shown:?}");
        assert_eq!(
            shown.stdout(),
            format!(
                "Item: {item}\nType: {TOTP_SEED_V1}\nLabel: \"GitHub ada@example.com\"\nIssuer: \"GitHub\"\nAlgorithm: SHA1\nDigits: 6\nPeriod: 30\nSecret: <redacted>\nFavorite: no\nUpdated: 1700000000000\n"
            )
        );
        assert!(!shown
            .stdout()
            .contains(core::str::from_utf8(&seed).unwrap()));

        let reveal_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone()],
            ["yes".to_owned()],
            47,
        );
        let revealed = run(["item", "reveal", item, "totp-secret"], &reveal_host);
        assert_eq!(revealed.exit_code(), ExitCode::Success, "{revealed:?}");
        assert!(revealed.stdout().is_empty());
        assert!(reveal_host.revealed_equals(&seed));

        let audit = run(
            ["audit", "list"],
            &TestHost::with_entropy_seed(paths, [passphrase], 59),
        );
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert_eq!(
            audit
                .stdout()
                .lines()
                .filter(|line| line.contains("action=item_create\toutcome=failed"))
                .count(),
            6,
            "{audit:?}"
        );
        for value in [
            "GitHub ada@example.com",
            "GitHub",
            "SHA1",
            core::str::from_utf8(&seed).unwrap(),
            "12345678901234567890",
        ] {
            assert!(!audit.stdout().contains(value));
        }
    }

    #[test]
    fn interactive_secret_reveal_audits_before_direct_terminal_delivery() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"secret reveal passphrase".to_vec();
        let password = b"line one\n\"terminal-safe\"".to_vec();
        let notes = b"private login note 8c54d782".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let add_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone(), password.clone(), notes.clone()],
            [
                "Revealable login".to_owned(),
                "ada@example.test".to_owned(),
                "0".to_owned(),
            ],
            41,
        );
        let added = run(["item", "add", "login"], &add_host);
        assert_eq!(added.exit_code(), ExitCode::Success, "{added:?}");
        let item = added
            .stdout()
            .strip_prefix("Item added: ")
            .and_then(|value| value.strip_suffix('\n'))
            .unwrap();
        let item_id = ItemId::from_user_string(item).unwrap();

        let denied_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone()],
            ["no".to_owned()],
            53,
        );
        let denied = run(["item", "reveal", item, "login-password"], &denied_host);
        assert_eq!(denied.exit_code(), ExitCode::InvalidInput, "{denied:?}");
        assert!(denied.stdout().is_empty());
        assert_eq!(denied_host.revealed_count(), 0);

        let unavailable_host = TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 67);
        let unavailable = run(
            ["item", "reveal", item, "login-password"],
            &unavailable_host,
        );
        assert_eq!(
            unavailable.exit_code(),
            ExitCode::Provider,
            "{unavailable:?}"
        );
        assert!(unavailable.stdout().is_empty());
        assert_eq!(unavailable_host.revealed_count(), 0);

        let wrong_field_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone()],
            ["yes".to_owned()],
            79,
        );
        let wrong_field = run(
            ["item", "reveal", item, "secure-note-body"],
            &wrong_field_host,
        );
        assert_eq!(
            wrong_field.exit_code(),
            ExitCode::InvalidInput,
            "{wrong_field:?}"
        );
        assert!(wrong_field.stdout().is_empty());
        assert_eq!(wrong_field_host.revealed_count(), 0);

        let reveal_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone()],
            ["yes".to_owned()],
            89,
        );
        let revealed = run(["item", "reveal", item, "login-password"], &reveal_host);
        assert_eq!(revealed.exit_code(), ExitCode::Success, "{revealed:?}");
        assert!(revealed.stdout().is_empty());
        assert!(revealed.stderr().is_empty());
        assert!(reveal_host.revealed_equals(&password));

        let notes_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone()],
            ["yes".to_owned()],
            93,
        );
        let revealed_notes = run(["item", "reveal", item, "login-notes"], &notes_host);
        assert_eq!(
            revealed_notes.exit_code(),
            ExitCode::Success,
            "{revealed_notes:?}"
        );
        assert!(revealed_notes.stdout().is_empty());
        assert!(notes_host.revealed_equals(&notes));

        let locked_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [b"wrong passphrase".to_vec()],
            ["yes".to_owned()],
            97,
        );
        let locked = run(["item", "reveal", item, "login-password"], &locked_host);
        assert_eq!(locked.exit_code(), ExitCode::Locked, "{locked:?}");
        assert_eq!(locked_host.revealed_count(), 0);

        let audit_host = TestHost::with_entropy_seed(paths, [passphrase], 101);
        let audit = run(["audit", "list"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert_eq!(
            audit
                .stdout()
                .lines()
                .filter(|line| {
                    line.contains("action=item_read\toutcome=denied")
                        && line.contains(&format!("\titem={}", item_id.to_user_string()))
                })
                .count(),
            2,
            "{audit:?}"
        );
        assert!(audit.stdout().lines().any(|line| {
            line.contains("action=item_read\toutcome=failed")
                && line.contains(&format!("\titem={}", item_id.to_user_string()))
        }));
        assert!(audit.stdout().lines().any(|line| {
            line.contains("action=item_read\toutcome=succeeded")
                && line.contains(&format!("\titem={}", item_id.to_user_string()))
                && line.contains("\tselected=")
        }));
        assert!(!audit.stdout().contains("Revealable login"));
        assert!(!audit
            .stdout()
            .contains(core::str::from_utf8(&password).unwrap()));
        assert!(!audit
            .stdout()
            .contains(core::str::from_utf8(&notes).unwrap()));
        assert!(!audit.stdout().contains("secret reveal passphrase"));
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
    fn conflict_commands_audit_unconflicted_and_missing_candidate_failures() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"conflict command passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);
        let add_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.clone(), b"conflict-safe note body".to_vec()],
            ["Conflict-safe note".to_owned()],
        );
        let added = run(["item", "add", "secure-note"], &add_host);
        assert_eq!(added.exit_code(), ExitCode::Success, "{added:?}");
        let item = added
            .stdout()
            .strip_prefix("Item added: ")
            .and_then(|value| value.strip_suffix('\n'))
            .unwrap();
        let item_id = ItemId::from_user_string(item).unwrap();
        let revision_id = RevisionId::new([0x77; 32]);

        let list_host = TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 113);
        let listed = run(["conflict", "list", item], &list_host);
        assert_eq!(listed.exit_code(), ExitCode::Conflict, "{listed:?}");
        assert!(listed.stdout().is_empty());

        let revision = revision_id.to_user_string();
        let denied_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone()],
            ["no".to_owned()],
            119,
        );
        let denied = run(
            [
                "conflict",
                "reveal",
                item,
                revision.as_str(),
                "secure-note-body",
            ],
            &denied_host,
        );
        assert_eq!(denied.exit_code(), ExitCode::InvalidInput, "{denied:?}");
        assert!(denied.stdout().is_empty());
        assert_eq!(denied_host.revealed_count(), 0);

        let reveal_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone()],
            ["yes".to_owned()],
            123,
        );
        let revealed = run(
            [
                "conflict",
                "reveal",
                item,
                revision.as_str(),
                "secure-note-body",
            ],
            &reveal_host,
        );
        assert_eq!(revealed.exit_code(), ExitCode::Conflict, "{revealed:?}");
        assert!(revealed.stdout().is_empty());
        assert_eq!(reveal_host.revealed_count(), 0);

        let merge_host = TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 125);
        let merged = run(
            ["conflict", "merge", "login", item, revision.as_str()],
            &merge_host,
        );
        assert_eq!(merged.exit_code(), ExitCode::Conflict, "{merged:?}");
        assert!(merged.stdout().is_empty());

        let note_merge_host = TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 126);
        let note_merged = run(
            ["conflict", "merge", "secure-note", item, revision.as_str()],
            &note_merge_host,
        );
        assert_eq!(
            note_merged.exit_code(),
            ExitCode::Conflict,
            "{note_merged:?}"
        );
        assert!(note_merged.stdout().is_empty());

        // The authored API-key merge must also stop at the unconflicted
        // precondition, before any label, token, scope, or expiry is asked for.
        let api_key_merge_host =
            TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 128);
        let api_key_merged = run(
            ["conflict", "merge", "api-key", item, revision.as_str()],
            &api_key_merge_host,
        );
        assert_eq!(
            api_key_merged.exit_code(),
            ExitCode::Conflict,
            "{api_key_merged:?}"
        );
        assert!(api_key_merged.stdout().is_empty());

        // The authored database-credential merge must also stop at the
        // unconflicted precondition, before any connection field or password
        // is asked for.
        let database_merge_host =
            TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 129);
        let database_merged = run(
            [
                "conflict",
                "merge",
                "database-credential",
                item,
                revision.as_str(),
            ],
            &database_merge_host,
        );
        assert_eq!(
            database_merged.exit_code(),
            ExitCode::Conflict,
            "{database_merged:?}"
        );
        assert!(database_merged.stdout().is_empty());

        // The authored TOTP merge must also stop at the unconflicted
        // precondition, before any parameter or Base32 seed is asked for.
        let totp_merge_host = TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 130);
        let totp_merged = run(
            ["conflict", "merge", "totp", item, revision.as_str()],
            &totp_merge_host,
        );
        assert_eq!(
            totp_merged.exit_code(),
            ExitCode::Conflict,
            "{totp_merged:?}"
        );
        assert!(totp_merged.stdout().is_empty());

        // So must the authored opaque-record merge, before its hidden payload
        // prompt. Nothing about the ceremony changes because the record has no
        // schema: the precondition is checked first either way.
        let opaque_merge_host =
            TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 131);
        let opaque_merged = run(
            ["conflict", "merge", "opaque", item, revision.as_str()],
            &opaque_merge_host,
        );
        assert_eq!(
            opaque_merged.exit_code(),
            ExitCode::Conflict,
            "{opaque_merged:?}"
        );
        assert!(opaque_merged.stdout().is_empty());

        let choose_host = TestHost::with_entropy_seed(paths.clone(), [passphrase.clone()], 127);
        let chosen = run(
            [
                "conflict",
                "choose",
                item,
                revision_id.to_user_string().as_str(),
            ],
            &choose_host,
        );
        assert_eq!(chosen.exit_code(), ExitCode::Conflict, "{chosen:?}");
        assert!(chosen.stdout().is_empty());

        let audit_host = TestHost::with_entropy_seed(paths, [passphrase], 131);
        let audit = run(["audit", "list"], &audit_host);
        assert_eq!(audit.exit_code(), ExitCode::Success, "{audit:?}");
        assert!(audit.stdout().lines().any(|line| {
            line.contains("action=item_history_read\toutcome=failed")
                && line.contains(&format!("\titem={}", item_id.to_user_string()))
        }));
        assert!(audit.stdout().lines().any(|line| {
            line.contains("action=item_conflict_resolve\toutcome=failed")
                && line.contains(&format!("\titem={}", item_id.to_user_string()))
        }));
        assert!(audit.stdout().lines().any(|line| {
            line.contains("action=item_read\toutcome=denied")
                && line.contains(&format!("\titem={}", item_id.to_user_string()))
        }));
        assert!(audit.stdout().lines().any(|line| {
            line.contains("action=item_read\toutcome=failed")
                && line.contains(&format!("\titem={}", item_id.to_user_string()))
        }));
        assert!(audit.stdout().lines().any(|line| {
            line.contains("action=item_conflict_merge\toutcome=failed")
                && line.contains(&format!("\titem={}", item_id.to_user_string()))
        }));
        assert!(!audit.stdout().contains("Conflict-safe note"));
        assert!(!audit.stdout().contains("conflict-safe note body"));
        assert!(!audit.stdout().contains("conflict command passphrase"));
        assert!(!audit.stdout().contains("secure-note-body"));
        assert!(!audit.stdout().contains(revision.as_str()));
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
    fn named_target_retry_rehydrates_the_original_audited_journal_without_entropy() {
        let root = TestRoot::new();
        let paths = root.paths();
        let personal_passphrase = b"existing personal passphrase".to_vec();
        let target_passphrase = b"resumable target passphrase".to_vec();
        let init_host = TestHost::new(paths.clone(), [personal_passphrase]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);

        let mut random = [0_u8; AUDITED_GENERATION_ZERO_RANDOM_BYTES];
        for (index, byte) in random.iter_mut().enumerate() {
            *byte = u8::try_from(index % 251).unwrap().wrapping_add(31);
        }
        let prepared = prepare_audited_generation_zero(
            Zeroizing::new(target_passphrase.clone()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 1_700_000_000_000).unwrap(),
            AuditedGenerationZeroRandomness::new(random),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let exact_prepared = prepared.owner_state().encode().unwrap();

        let layout = paths.prepare().unwrap();
        let writer = layout.try_acquire_writer().unwrap();
        let application_store = application_store(&paths);
        application_store
            .compare_exchange(locator, None, &exact_prepared)
            .unwrap();
        let exact_config = writer.load_config().unwrap().unwrap();
        let config = decode_config(&exact_config).unwrap();
        let source = configured_vault(&paths, &config, None).unwrap();
        let target_root = target_object_root(&paths, locator.as_bytes());
        let target_storage_name = target_storage_name(locator.as_bytes()).unwrap();
        let target = VaultConfigV1::new(
            ConfigVaultLocator::new(*locator.as_bytes()),
            target_storage_name.clone(),
            Vec::new(),
            source.auto_lock_seconds(),
            source.clipboard_clear_seconds(),
        )
        .unwrap();
        let mut vaults = config.vaults().clone();
        vaults.insert(ConfigName::new("work".to_owned()).unwrap(), target);
        let mut storage = config.storage().clone();
        storage.insert(
            target_storage_name,
            StorageConfigV1::new(
                StorageKind::Filesystem,
                StorageLocation::new(target_root.to_str().unwrap()).unwrap(),
                CredentialRef::none(),
            ),
        );
        let replacement =
            VaultPmConfigV1::new(config.default_vault().clone(), vaults, storage).unwrap();
        writer
            .compare_exchange_config(&exact_config, render_config(&replacement).as_bytes())
            .unwrap();
        drop(writer);
        drop(layout);
        drop(prepared);

        let resume_host = TestHost::without_entropy(paths.clone(), [target_passphrase.clone()]);
        let resumed = run(["vault", "create", "work"], &resume_host);
        assert_eq!(resumed.exit_code(), ExitCode::Success, "{resumed:?}");
        assert_eq!(resumed.stdout(), "Vault target created.\n");

        let verify_host = TestHost::with_entropy_seed(paths, [target_passphrase], 41);
        let verified = run(["--vault", "work", "audit", "verify"], &verify_host);
        assert_eq!(verified.exit_code(), ExitCode::Success, "{verified:?}");
        assert!(verified.stdout().contains("audit_events=1"));
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

    // -----------------------------------------------------------------------
    // VLT-PM42 — a vault wedged by an interrupted publication
    //
    // VLT-PM41 proved with a real killed process that a crash inside the shared
    // mutation publication path leaves an exact, replayable journal, and that
    // nothing in the product ever replayed it. These tests wedge a real on-disk
    // vault the same way — by letting the write-ahead journal become durable
    // and then taking the provider away — and then drive the ordinary command
    // surface across it.
    // -----------------------------------------------------------------------

    /// Leave the default vault's durable owner state as an exact
    /// `PendingPublication`.
    ///
    /// The session is opened through the ordinary application boundary over a
    /// *faulting view of the very same object root* the CLI itself uses, so
    /// everything on disk afterwards is what a crash leaves: the journal is
    /// real, its signed bytes are real, and the objects it names are the ones
    /// the real repository is missing.
    fn wedge_by_an_interrupted_publication(paths: &LocalVaultPaths, passphrase: &[u8]) {
        use coding_adventures_vault_pm_application::open_active_vault;
        use coding_adventures_vault_pm_storage::{
            FaultAction, FaultEffect, FaultInjectingObjectStore, StoreOperation,
        };
        use std::sync::Arc;

        let (locator, object_root) = configured_vault_location(paths);
        let application_store = application_store(paths);
        let faulting = Arc::new(FaultInjectingObjectStore::new(StorageCoreObjectStore::new(
            crash::backend(&object_root),
        )));
        let factory = V1ApplicationRepositoryFactory::from_shared(Arc::clone(&faulting));
        let session = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &application_store,
            &application_store,
            &factory,
        )
        .expect("the fixture vault must open");
        // The commit reaches the provider and the provider then stops
        // answering — the ambiguity a kill between the journal write and the
        // owner-state advance produces.
        faulting
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::CommitPutThenNetwork,
            })
            .unwrap();
        let interrupted = session.audited_list_items(
            FIXED_TEST_TIME_MS,
            AuditedAccessRandomnessV1::new([0x5c; AUDITED_ACCESS_RANDOM_BYTES]),
            &application_store,
        );
        assert!(
            matches!(interrupted, Err(ApplicationError::StorageUnavailable)),
            "the fixture must interrupt a publication, not complete it",
        );
    }

    /// Resolve the default vault's application locator and object root.
    ///
    /// The writer lock is acquired and released inside this function, because
    /// every command the tests run afterwards acquires it for itself.
    fn configured_vault_location(paths: &LocalVaultPaths) -> (BootstrapLocator, PathBuf) {
        let prepared = paths.prepare().unwrap();
        let writer = prepared.try_acquire_writer().unwrap();
        let exact_config = writer.load_config().unwrap().unwrap();
        let config = decode_config(&exact_config).unwrap();
        let vault = configured_vault(prepared.paths(), &config, None).unwrap();
        let location = config
            .storage()
            .get(vault.local_store())
            .unwrap()
            .location()
            .as_str()
            .to_owned();
        (
            application_locator(vault.locator()),
            PathBuf::from(location),
        )
    }

    /// One initialized vault holding one login, and its item identifier.
    fn vault_with_one_login(paths: &LocalVaultPaths, passphrase: &[u8]) -> String {
        let init_host = TestHost::new(paths.clone(), [passphrase.to_vec()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);
        let add_host = TestHost::with_texts(
            paths.clone(),
            [passphrase.to_vec(), b"first note body".to_vec()],
            ["First note".to_owned()],
        );
        let added = run(["item", "add", "secure-note"], &add_host);
        assert_eq!(added.exit_code(), ExitCode::Success, "{added:?}");
        added
            .stdout()
            .lines()
            .find_map(|line| line.strip_prefix("Item added: "))
            .expect("item-add identifier")
            .to_owned()
    }

    #[test]
    fn a_repair_is_announced_only_when_both_observations_prove_one() {
        use VaultStatusStateV1::{Absent, Locked, Prepared, RecoveryRequired, Unlocked};

        // The only row that speaks: wedged before, demonstrably not wedged
        // after. Every state the projection can report counts as "not wedged".
        for after in [Absent, Prepared, Locked, Unlocked] {
            assert!(
                observed_a_repair(Some(RecoveryRequired), Some(after)),
                "{after:?}"
            );
        }

        // Still wedged, so nothing was finished.
        assert!(!observed_a_repair(
            Some(RecoveryRequired),
            Some(RecoveryRequired)
        ));

        // An observation that could not be taken proves nothing, and
        // `None != Some(RecoveryRequired)` is exactly the true-but-meaningless
        // comparison that would otherwise announce a repair on a vault that is
        // still wedged.
        assert!(!observed_a_repair(Some(RecoveryRequired), None));

        // Nothing to repair in the first place.
        for before in [None, Some(Absent), Some(Prepared), Some(Locked)] {
            assert!(!observed_a_repair(before, Some(Locked)), "{before:?}");
            assert!(!observed_a_repair(before, None), "{before:?}");
        }
    }

    #[test]
    fn an_ordinary_command_finishes_an_interrupted_publication_and_says_so() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"recovery correct horse battery staple".to_vec();
        let item = vault_with_one_login(&paths, &passphrase);
        wedge_by_an_interrupted_publication(&paths, &passphrase);

        // Before VLT-PM42 this was exit 2, `vault-pm: invalid command`, for
        // every command that opens the vault, forever.
        let list_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let listed = run(["item", "list"], &list_host);
        assert_eq!(listed.exit_code(), ExitCode::Success, "{listed:?}");
        assert!(listed.stdout().contains(&item), "{listed:?}");
        assert_eq!(listed.stderr(), RECOVERY_NOTICE, "{listed:?}");

        // The repair happened once. The next command finds an ordinary vault
        // and says nothing.
        let again_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let again = run(["item", "list"], &again_host);
        assert_eq!(again.exit_code(), ExitCode::Success, "{again:?}");
        assert!(again.stderr().is_empty(), "{again:?}");

        // And the vault is ordinary in the only sense that matters: it still
        // takes writes, and its whole audit chain still verifies.
        // A distinct entropy seed, because this host's randomness is
        // deterministic and reusing the fixture's would mint the identifiers
        // the first note already owns.
        let add_host = TestHost::with_texts_and_entropy_seed(
            paths.clone(),
            [passphrase.clone(), b"second note body".to_vec()],
            ["Second note".to_owned()],
            29,
        );
        let added = run(["item", "add", "secure-note"], &add_host);
        assert_eq!(added.exit_code(), ExitCode::Success, "{added:?}");
        let verify_host = TestHost::new(paths, [passphrase]);
        let verified = run(["audit", "verify"], &verify_host);
        assert_eq!(verified.exit_code(), ExitCode::Success, "{verified:?}");
    }

    #[test]
    fn the_read_only_diagnostics_report_a_wedged_vault_without_repairing_it() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"diagnostic correct horse battery staple".to_vec();
        vault_with_one_login(&paths, &passphrase);
        wedge_by_an_interrupted_publication(&paths, &passphrase);

        // `status` and `doctor` answer without a passphrase, and leave the
        // vault exactly as wedged as they found it — the property a person
        // restoring a pre-mutation backup depends on.
        for _ in 0..2 {
            let status = run(["status"], &TestHost::new(paths.clone(), []));
            assert_eq!(status.stdout(), "Status: recovery_required\n", "{status:?}");
            assert!(status.stderr().is_empty(), "{status:?}");

            let doctor = run(["doctor"], &TestHost::new(paths.clone(), []));
            assert_eq!(doctor.exit_code(), ExitCode::Conflict, "{doctor:?}");
            assert_eq!(doctor.stdout(), "Doctor: recovery_required\n");
            assert!(doctor.stderr().is_empty(), "{doctor:?}");
        }

        // `--unlock` does not turn a diagnostic into a repair. It collects no
        // passphrase — the host below would panic if asked for one — and it
        // now reports the state instead of inheriting the refused open's
        // misleading exit 2 `invalid command`.
        let unlock_doctor = run(["doctor", "--unlock"], &TestHost::new(paths.clone(), []));
        assert_eq!(
            unlock_doctor.exit_code(),
            ExitCode::Conflict,
            "{unlock_doctor:?}"
        );
        assert_eq!(unlock_doctor.stdout(), "Doctor: recovery_required\n");
        assert!(unlock_doctor.stderr().is_empty(), "{unlock_doctor:?}");

        // Still wedged, so the repair below is this test's, not a diagnostic's.
        let recover_host = TestHost::new(paths.clone(), [passphrase]);
        let recovered = run(["item", "list"], &recover_host);
        assert_eq!(recovered.exit_code(), ExitCode::Success, "{recovered:?}");
        assert_eq!(recovered.stderr(), RECOVERY_NOTICE, "{recovered:?}");
    }

    #[test]
    fn init_finishes_an_interrupted_publication_instead_of_refusing_it() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"resume correct horse battery staple".to_vec();
        vault_with_one_login(&paths, &passphrase);
        wedge_by_an_interrupted_publication(&paths, &passphrase);

        // `init` is what a stuck person retries. It used to answer the
        // conflict class; it now finishes what was interrupted.
        let resume_host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let resumed = run(["init"], &resume_host);
        assert_eq!(resumed.exit_code(), ExitCode::Success, "{resumed:?}");
        assert_eq!(resumed.stdout(), "Vault recovered.\n");
        assert_eq!(resumed.stderr(), RECOVERY_NOTICE, "{resumed:?}");

        assert_eq!(
            run(["status"], &TestHost::new(paths.clone(), [])).stdout(),
            "Status: locked\n"
        );
        // A healthy vault is still refused, so the repair is the only thing
        // this path learned to do.
        let repeat = run(["init"], &TestHost::new(paths, [passphrase]));
        assert_eq!(repeat.exit_code(), ExitCode::InvalidInput, "{repeat:?}");
        assert_eq!(repeat.stderr(), "vault-pm: already initialized\n");
    }

    #[test]
    fn a_wrong_passphrase_leaves_a_wedged_vault_wedged() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"closed correct horse battery staple".to_vec();
        vault_with_one_login(&paths, &passphrase);
        wedge_by_an_interrupted_publication(&paths, &passphrase);

        // Recovery authenticates before it publishes, so a wrong secret buys
        // nothing and destroys nothing.
        let wrong_host = TestHost::new(paths.clone(), [b"not the passphrase".to_vec()]);
        let refused = run(["item", "list"], &wrong_host);
        assert_eq!(refused.exit_code(), ExitCode::Locked, "{refused:?}");
        assert_eq!(refused.stderr(), "vault-pm: authentication required\n");
        assert_eq!(
            run(["status"], &TestHost::new(paths.clone(), [])).stdout(),
            "Status: recovery_required\n"
        );

        let right_host = TestHost::new(paths, [passphrase]);
        let recovered = run(["item", "list"], &right_host);
        assert_eq!(recovered.exit_code(), ExitCode::Success, "{recovered:?}");
    }

    #[test]
    fn a_recovering_command_that_then_fails_still_reports_the_repair() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"partial correct horse battery staple".to_vec();
        vault_with_one_login(&paths, &passphrase);
        wedge_by_an_interrupted_publication(&paths, &passphrase);

        // The repair is worth saying even when the verb that triggered it went
        // on to report something else, and the verb keeps its own exit class.
        let host = TestHost::new(paths.clone(), [passphrase.clone()]);
        let missing_id = ItemId::new([0x55; 16]).to_user_string();
        let missing = run(["item", "show", missing_id.as_str()], &host);
        assert_eq!(missing.exit_code(), ExitCode::NotFound, "{missing:?}");
        assert_eq!(
            missing.stderr(),
            format!("{RECOVERY_NOTICE}vault-pm: not found\n"),
        );
        assert_eq!(
            run(["status"], &TestHost::new(paths, [])).stdout(),
            "Status: locked\n"
        );
    }

    #[test]
    fn a_portable_export_finishes_an_interrupted_publication_first() {
        let root = TestRoot::new();
        let paths = root.paths();
        let passphrase = b"export correct horse battery staple".to_vec();
        vault_with_one_login(&paths, &passphrase);
        wedge_by_an_interrupted_publication(&paths, &passphrase);

        let destination = root.0.join("recovered-export.vpm");
        let export_host = TestHost::new(
            paths.clone(),
            [passphrase, b"distinct export passphrase".to_vec()],
        );
        let exported = run(
            ["export", destination.to_str().expect("UTF-8 path")],
            &export_host,
        );
        assert_eq!(exported.exit_code(), ExitCode::Success, "{exported:?}");
        assert_eq!(exported.stderr(), RECOVERY_NOTICE, "{exported:?}");
        assert!(destination.exists());
        assert_eq!(
            run(["status"], &TestHost::new(paths, [])).stdout(),
            "Status: locked\n"
        );
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

    /// A scripted stand-in for the controlling terminal a real session reads.
    ///
    /// It answers command lines from a queue and records what the shell chose
    /// to render, so a test can assert on the exact transcript a user would
    /// have seen. An exhausted queue reports end of input, which is how a
    /// scripted session ends without an explicit `exit`.
    struct ScriptedTerminal<'host> {
        lines: Mutex<VecDeque<String>>,
        rendered: Mutex<Vec<CliOutput>>,
        readable: bool,
        /// Host whose clock advances while this terminal waits for input.
        idle_host: Option<&'host TestHost>,
        /// Milliseconds each blocked read consumes.
        idle_ms: u64,
    }

    impl<'host> ScriptedTerminal<'host> {
        fn new(lines: impl IntoIterator<Item = &'static str>) -> Self {
            Self {
                lines: Mutex::new(lines.into_iter().map(str::to_owned).collect()),
                rendered: Mutex::new(Vec::new()),
                readable: true,
                idle_host: None,
                idle_ms: 0,
            }
        }

        /// A terminal whose user takes `idle_ms` to type each line.
        fn idle(
            lines: impl IntoIterator<Item = &'static str>,
            host: &'host TestHost,
            idle_ms: u64,
        ) -> Self {
            Self {
                idle_host: Some(host),
                idle_ms,
                ..Self::new(lines)
            }
        }

        /// A terminal that cannot be read at all, as when `/dev/tty` is absent.
        fn unreadable() -> Self {
            Self {
                lines: Mutex::new(VecDeque::new()),
                rendered: Mutex::new(Vec::new()),
                readable: false,
                idle_host: None,
                idle_ms: 0,
            }
        }

        fn transcript(&self) -> String {
            self.rendered
                .lock()
                .unwrap()
                .iter()
                .map(|output| format!("{}{}", output.stdout(), output.stderr()))
                .collect()
        }

        fn exit_codes(&self) -> Vec<ExitCode> {
            self.rendered
                .lock()
                .unwrap()
                .iter()
                .map(CliOutput::exit_code)
                .collect()
        }
    }

    impl ShellTerminal for ScriptedTerminal<'_> {
        fn read_command_line(&self) -> Result<Option<Zeroizing<String>>, HostError> {
            if !self.readable {
                return Err(HostError::Unavailable);
            }
            // A real terminal read blocks for as long as nobody types, so the
            // scripted one models wall time passing *inside* the read rather
            // than only between commands.
            if let Some(host) = self.idle_host {
                host.advance_clock(self.idle_ms);
            }
            Ok(self.lines.lock().unwrap().pop_front().map(Zeroizing::new))
        }

        fn write_output(&self, output: &CliOutput) -> Result<(), HostError> {
            self.rendered.lock().unwrap().push(output.clone());
            Ok(())
        }
    }

    /// Initialize a vault and return its host root, ready for a shell session.
    fn initialized_shell_root() -> TestRoot {
        let root = TestRoot::new();
        let init_host = TestHost::new(root.paths(), [b"shell session passphrase".to_vec()]);
        assert_eq!(run(["init"], &init_host).exit_code(), ExitCode::Success);
        root
    }

    #[test]
    fn shell_grammar_accepts_only_a_bare_verb() {
        assert_eq!(parse(["shell"]), default_invocation(Command::Shell));
        assert_eq!(
            parse(["--vault", "personal", "shell"]),
            Ok(Invocation {
                selected_vault: Some(ConfigName::new("personal").unwrap()),
                command: Command::Shell,
            })
        );
        for arguments in [
            vec!["shell", "--json"],
            vec!["shell", "item", "list"],
            vec!["shell", "--passphrase", "secret"],
        ] {
            assert_eq!(
                parse(arguments.clone()),
                Err(CliFailure::InvalidCommand),
                "{arguments:?}"
            );
        }
        assert!(USAGE.contains("vault-pm [--vault NAME] shell\n"));
        // Dispatch repeats this refusal rather than trusting classification,
        // because a delegated `shell` would recurse over the real terminal.
        for refused in ["init", "vault", "shell", "--vault"] {
            assert!(shell::is_refused(refused), "{refused}");
        }
        for delegated in ["item", "status", "search", "conflict", "audit"] {
            assert!(!shell::is_refused(delegated), "{delegated}");
        }
    }

    #[test]
    fn shell_unlocks_once_for_many_commands() {
        let root = initialized_shell_root();
        // Exactly one passphrase is scripted for three authenticated commands.
        let host = TestHost::new(root.paths(), [b"shell session passphrase".to_vec()]);
        let terminal = ScriptedTerminal::new(["item list", "status", "item list", "exit"]);
        let output = run_with_terminal(["shell"], &host, &terminal);

        assert_eq!(output.exit_code(), ExitCode::Success);
        assert_eq!(output.stdout(), "");
        assert_eq!(output.stderr(), "");
        assert_eq!(host.remaining_secrets(), 0);
        // `status` still reports `locked`, and that is correct rather than a
        // gap: it projects durable owner state, and between commands the vault
        // really is locked. The session retains an authenticator, not an
        // unlocked vault, so nothing about the stored state has changed.
        assert_eq!(
            terminal.transcript(),
            "No items.\nStatus: locked\nNo items.\n"
        );
        assert_eq!(
            terminal.exit_codes(),
            [ExitCode::Success, ExitCode::Success, ExitCode::Success]
        );
    }

    #[test]
    fn shell_lock_forces_the_next_command_to_reauthenticate() {
        let root = initialized_shell_root();
        let host = TestHost::new(
            root.paths(),
            [
                b"shell session passphrase".to_vec(),
                b"shell session passphrase".to_vec(),
            ],
        );
        let terminal = ScriptedTerminal::new(["item list", "lock", "item list", "quit"]);
        let output = run_with_terminal(["shell"], &host, &terminal);

        assert_eq!(output.exit_code(), ExitCode::Success);
        // Both scripted passphrases were consumed: one before `lock`, one after.
        assert_eq!(host.remaining_secrets(), 0);
        assert_eq!(terminal.transcript(), "No items.\nLocked.\nNo items.\n");
    }

    #[test]
    fn shell_ends_on_end_of_input_without_an_explicit_verb() {
        let root = initialized_shell_root();
        let host = TestHost::new(root.paths(), [b"shell session passphrase".to_vec()]);
        let terminal = ScriptedTerminal::new(["item list"]);
        let output = run_with_terminal(["shell"], &host, &terminal);
        assert_eq!(output.exit_code(), ExitCode::Success);
        assert_eq!(terminal.transcript(), "No items.\n");
    }

    #[test]
    fn shell_wipes_the_authenticator_after_a_rejected_attempt() {
        let root = initialized_shell_root();
        let host = TestHost::new(
            root.paths(),
            [
                b"wrong passphrase".to_vec(),
                b"shell session passphrase".to_vec(),
            ],
        );
        let terminal = ScriptedTerminal::new(["item list", "item list", "exit"]);
        let output = run_with_terminal(["shell"], &host, &terminal);

        assert_eq!(output.exit_code(), ExitCode::Success);
        // A rejected passphrase is not retained, so the second command asks
        // again and succeeds instead of failing forever.
        assert_eq!(host.remaining_secrets(), 0);
        assert_eq!(
            terminal.exit_codes(),
            [ExitCode::Locked, ExitCode::Success],
            "{}",
            terminal.transcript()
        );
        assert_eq!(
            terminal.transcript(),
            "vault-pm: authentication required\nNo items.\n"
        );
    }

    #[test]
    fn shell_reauthenticates_after_the_configured_idle_bound() {
        let root = initialized_shell_root();
        // Each clock reading advances 400 seconds, so the 300-second default
        // auto-lock bound has always elapsed by the next command boundary.
        let host = TestHost::with_clock_step(
            root.paths(),
            [
                b"shell session passphrase".to_vec(),
                b"shell session passphrase".to_vec(),
            ],
            400_000,
        );
        let terminal = ScriptedTerminal::new(["item list", "item list", "exit"]);
        let output = run_with_terminal(["shell"], &host, &terminal);

        assert_eq!(output.exit_code(), ExitCode::Success);
        assert_eq!(host.remaining_secrets(), 0);
        assert_eq!(terminal.transcript(), "No items.\nNo items.\n");
    }

    #[test]
    fn shell_reauthenticates_when_the_wait_at_the_prompt_exceeds_the_bound() {
        let root = initialized_shell_root();
        // The clock itself never moves when it is read; all the elapsed time
        // happens inside the blocked terminal read, which is where an
        // unattended session actually spends it. A bound checked only before
        // the prompt was printed would see zero elapsed time here and hand the
        // stale authenticator to the command an attacker typed.
        let host = TestHost::new(
            root.paths(),
            [
                b"shell session passphrase".to_vec(),
                b"shell session passphrase".to_vec(),
            ],
        );
        let terminal = ScriptedTerminal::idle(["item list", "item list", "exit"], &host, 400_000);
        let output = run_with_terminal(["shell"], &host, &terminal);

        assert_eq!(output.exit_code(), ExitCode::Success);
        assert_eq!(host.remaining_secrets(), 0);
        assert_eq!(terminal.transcript(), "No items.\nNo items.\n");
    }

    #[test]
    fn shell_refuses_lifecycle_and_reselection_verbs() {
        let root = initialized_shell_root();
        let host = TestHost::new(root.paths(), []);
        let terminal = ScriptedTerminal::new([
            "init",
            "vault create work",
            "shell",
            "--vault work item list",
            "\"unterminated",
            "a b c d e f g h i",
            "exit",
        ]);
        let output = run_with_terminal(["shell"], &host, &terminal);

        assert_eq!(output.exit_code(), ExitCode::Success);
        assert_eq!(terminal.exit_codes(), [ExitCode::InvalidInput; 6]);
        assert_eq!(
            terminal.transcript(),
            "vault-pm: invalid command\n".repeat(6)
        );
        // No passphrase was ever collected, so nothing was retained to wipe.
        assert_eq!(host.remaining_secrets(), 0);
    }

    #[test]
    fn shell_help_and_blank_lines_need_no_authentication() {
        let root = initialized_shell_root();
        let host = TestHost::new(root.paths(), []);
        let terminal = ScriptedTerminal::new(["", "   ", "help", "exit"]);
        let output = run_with_terminal(["shell"], &host, &terminal);

        assert_eq!(output.exit_code(), ExitCode::Success);
        let transcript = terminal.transcript();
        assert!(transcript.starts_with("Shell:\n  lock "));
        assert!(transcript.contains("  exit    end the session"));
        assert!(transcript.ends_with(USAGE));
        assert_eq!(terminal.exit_codes(), [ExitCode::Success]);
    }

    #[test]
    fn shell_refuses_to_start_without_configuration_or_a_terminal() {
        let empty = TestRoot::new();
        let unconfigured = TestHost::new(empty.paths(), []);
        let unconfigured_output = run_with_terminal(
            ["shell"],
            &unconfigured,
            &ScriptedTerminal::new(["item list"]),
        );
        assert_eq!(unconfigured_output.exit_code(), ExitCode::InvalidInput);
        assert_eq!(unconfigured_output.stderr(), "vault-pm: invalid command\n");

        let root = initialized_shell_root();
        let host = TestHost::new(root.paths(), []);
        let missing_vault = run_with_terminal(
            ["--vault", "absent", "shell"],
            &host,
            &ScriptedTerminal::new([]),
        );
        assert_eq!(missing_vault.exit_code(), ExitCode::NotFound);

        let unreadable = ScriptedTerminal::unreadable();
        let no_terminal = run_with_terminal(["shell"], &host, &unreadable);
        assert_eq!(no_terminal.exit_code(), ExitCode::Provider);
        assert_eq!(no_terminal.stderr(), "vault-pm: storage unavailable\n");
        assert!(unreadable.transcript().is_empty());
    }

    #[test]
    fn shell_tokenizer_is_closed_and_quote_aware() {
        assert_eq!(
            shell::tokenize("item show ABC").unwrap(),
            ["item", "show", "ABC"]
        );
        assert_eq!(
            shell::tokenize("  item \t list  ").unwrap(),
            ["item", "list"]
        );
        assert_eq!(
            shell::tokenize("search \"two words\"").unwrap(),
            ["search", "two words"]
        );
        assert_eq!(shell::tokenize("search \"\"").unwrap(), ["search", ""]);
        assert!(shell::tokenize("").unwrap().is_empty());
        for rejected in [
            "search \"unterminated",
            "search \"a\"b",
            "one two three four five six seven eight nine",
        ] {
            assert_eq!(
                shell::tokenize(rejected),
                Err(CliFailure::InvalidCommand),
                "{rejected}"
            );
        }
    }

    #[test]
    fn shell_idle_bound_fails_closed_on_an_unusable_clock() {
        let root = TestRoot::new();
        let host = TestHost::new(root.paths(), [b"retained value".to_vec()]);
        let session = shell::ShellSession::new(300_000);
        assert_eq!(&*session.authenticator(&host).unwrap(), b"retained value");
        assert_eq!(format!("{session:?}"), "ShellSession(<retained>)");

        // Wall time is advisory, not monotonic. A backwards step must expire the
        // authenticator rather than report zero elapsed time forever.
        host.rewind_clock(1);
        session.enforce_idle_bound(&host);
        assert_eq!(format!("{session:?}"), "ShellSession(<locked>)");

        // A forward step inside the bound retains it; past the bound expires it.
        assert!(session.authenticator(&host).is_err());
        let host = TestHost::new(root.paths(), [b"retained value".to_vec()]);
        let session = shell::ShellSession::new(300_000);
        session.authenticator(&host).unwrap();
        host.advance_clock(299_999);
        session.enforce_idle_bound(&host);
        assert_eq!(format!("{session:?}"), "ShellSession(<retained>)");
        host.advance_clock(1);
        session.enforce_idle_bound(&host);
        assert_eq!(format!("{session:?}"), "ShellSession(<locked>)");
    }

    /// A host whose every answer names the method that produced it.
    ///
    /// The point is to make a mis-wired delegation impossible to miss. The
    /// session host forwards roughly fifty [`CliHost`] methods by hand, and the
    /// dangerous failure is not a missing method — that would not compile — but
    /// a copy-paste swap between two methods with the same signature, say
    /// `read_card_cvv` answering with the card number. Because each answer here
    /// is its own method's name, any such swap shows up as a wrong value rather
    /// than as silently plausible output.
    struct EchoingHost {
        paths: LocalVaultPaths,
        calls: Mutex<Vec<&'static str>>,
    }

    /// Implement the `CliHost` methods whose answer is just a bounded string.
    macro_rules! echoing_text {
        ($($method:ident),* $(,)?) => {
            $(
                fn $method(&self) -> Result<Zeroizing<String>, HostError> {
                    Ok(self.echo(stringify!($method)))
                }
            )*
        };
    }

    /// Implement the `CliHost` methods whose answer is an optional string.
    macro_rules! echoing_optional_text {
        ($($method:ident),* $(,)?) => {
            $(
                fn $method(&self) -> Result<Option<Zeroizing<String>>, HostError> {
                    Ok(Some(self.echo(stringify!($method))))
                }
            )*
        };
    }

    /// Implement the `CliHost` methods whose answer is owned secret bytes.
    macro_rules! echoing_secret {
        ($($method:ident),* $(,)?) => {
            $(
                fn $method(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
                    Ok(Zeroizing::new(stringify!($method).as_bytes().to_vec()))
                }
            )*
        };
    }

    impl EchoingHost {
        fn new(paths: LocalVaultPaths) -> Self {
            Self {
                paths,
                calls: Mutex::new(Vec::new()),
            }
        }

        fn echo(&self, method: &'static str) -> Zeroizing<String> {
            Zeroizing::new(method.to_owned())
        }

        fn record(&self, method: &'static str) {
            self.calls.lock().unwrap().push(method);
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl CliHost for EchoingHost {
        echoing_text!(
            read_login_title,
            read_login_username,
            read_login_url_count,
            read_login_url,
            read_login_password,
            read_secure_note_title,
            read_secure_note_body,
            read_card_title,
            read_card_holder,
            read_card_number,
            read_card_expiry_month,
            read_card_expiry_year,
            read_card_cvv,
            read_api_key_label,
            read_api_key_service,
            read_api_key_token,
            read_api_key_scopes,
            read_api_key_expiry,
            read_database_label,
            read_database_engine,
            read_database_host,
            read_database_port,
            read_database_username,
            read_database_password,
            read_totp_label,
            read_totp_secret,
            read_totp_algorithm,
            read_totp_digits,
            read_totp_period,
            read_opaque_payload,
        );
        echoing_optional_text!(
            read_login_notes,
            read_card_billing_postal_code,
            read_database_name,
            read_totp_issuer,
        );
        echoing_secret!(
            read_new_passphrase,
            read_existing_passphrase,
            read_export_passphrase,
            read_import_passphrase,
        );

        fn paths(&self) -> Result<LocalVaultPaths, HostError> {
            self.record("paths");
            Ok(self.paths.clone())
        }

        fn confirm_secret_reveal(&self) -> Result<bool, HostError> {
            self.record("confirm_secret_reveal");
            Ok(true)
        }

        fn write_revealed_text(&self, value: &str) -> Result<(), HostError> {
            assert_eq!(value, "revealed");
            self.record("write_revealed_text");
            Ok(())
        }

        fn write_portable_export(
            &self,
            destination: &Path,
            artifact: &[u8],
        ) -> Result<(), HostError> {
            assert_eq!(destination, Path::new("destination"));
            assert_eq!(artifact, b"artifact");
            self.record("write_portable_export");
            Ok(())
        }

        fn read_portable_export(&self, source: &Path) -> Result<Vec<u8>, HostError> {
            assert_eq!(source, Path::new("source"));
            self.record("read_portable_export");
            Ok(b"read_portable_export".to_vec())
        }

        fn fill_entropy(&self, output: &mut [u8]) -> Result<(), HostError> {
            self.record("fill_entropy");
            output.fill(0xa5);
            Ok(())
        }

        fn now_ms(&self) -> Result<u64, HostError> {
            self.record("now_ms");
            Ok(FIXED_TEST_TIME_MS)
        }

        fn generation_zero_kdf(&self) -> (u32, u32, u8) {
            self.record("generation_zero_kdf");
            (1, 2, 3)
        }

        fn portable_export_kdf(&self) -> (u32, u32, u8) {
            self.record("portable_export_kdf");
            (4, 5, 6)
        }

        fn portable_open_kdf(&self) -> (u32, u32, u8) {
            self.record("portable_open_kdf");
            (7, 8, 9)
        }
    }

    #[test]
    fn shell_session_host_delegates_every_authority_except_the_unlock_prompt() {
        let root = TestRoot::new();
        let inner = EchoingHost::new(root.paths());
        let session = shell::ShellSession::new(300_000);
        let host = shell::SessionHost {
            inner: &inner,
            session: &session,
        };

        // Every echoed answer must arrive from the identically named method.
        macro_rules! assert_text {
            ($($method:ident),* $(,)?) => {
                $(assert_eq!(&*host.$method().unwrap(), stringify!($method));)*
            };
        }
        macro_rules! assert_optional_text {
            ($($method:ident),* $(,)?) => {
                $(assert_eq!(
                    &*host.$method().unwrap().expect(stringify!($method)),
                    stringify!($method)
                );)*
            };
        }
        assert_text!(
            read_login_title,
            read_login_username,
            read_login_url_count,
            read_login_url,
            read_login_password,
            read_secure_note_title,
            read_secure_note_body,
            read_card_title,
            read_card_holder,
            read_card_number,
            read_card_expiry_month,
            read_card_expiry_year,
            read_card_cvv,
            read_api_key_label,
            read_api_key_service,
            read_api_key_token,
            read_api_key_scopes,
            read_api_key_expiry,
            read_database_label,
            read_database_engine,
            read_database_host,
            read_database_port,
            read_database_username,
            read_database_password,
            read_totp_label,
            read_totp_secret,
            read_totp_algorithm,
            read_totp_digits,
            read_totp_period,
            read_opaque_payload,
        );
        assert_optional_text!(
            read_login_notes,
            read_card_billing_postal_code,
            read_database_name,
            read_totp_issuer,
        );
        assert_eq!(
            &*host.read_new_passphrase().unwrap(),
            b"read_new_passphrase"
        );
        assert_eq!(
            &*host.read_export_passphrase().unwrap(),
            b"read_export_passphrase"
        );
        assert_eq!(
            &*host.read_import_passphrase().unwrap(),
            b"read_import_passphrase"
        );
        assert_eq!(
            host.paths().unwrap().config_root(),
            root.paths().config_root()
        );
        assert!(host.confirm_secret_reveal().unwrap());
        host.write_revealed_text("revealed").unwrap();
        host.write_portable_export(Path::new("destination"), b"artifact")
            .unwrap();
        assert_eq!(
            host.read_portable_export(Path::new("source")).unwrap(),
            b"read_portable_export"
        );
        let mut entropy = [0_u8; 4];
        host.fill_entropy(&mut entropy).unwrap();
        assert_eq!(entropy, [0xa5; 4]);
        assert_eq!(host.now_ms().unwrap(), FIXED_TEST_TIME_MS);
        assert_eq!(host.generation_zero_kdf(), (1, 2, 3));
        assert_eq!(host.portable_export_kdf(), (4, 5, 6));
        assert_eq!(host.portable_open_kdf(), (7, 8, 9));
        assert_eq!(
            inner.calls(),
            [
                "paths",
                "confirm_secret_reveal",
                "write_revealed_text",
                "write_portable_export",
                "read_portable_export",
                "fill_entropy",
                "now_ms",
                "generation_zero_kdf",
                "portable_export_kdf",
                "portable_open_kdf",
            ]
        );

        // The unlock prompt is the one method that behaves differently: it is
        // collected once and then answered from the session.
        assert_eq!(
            &*host.read_existing_passphrase().unwrap(),
            b"read_existing_passphrase"
        );
        assert_eq!(format!("{session:?}"), "ShellSession(<retained>)");
        assert_eq!(
            &*host.read_existing_passphrase().unwrap(),
            b"read_existing_passphrase"
        );
    }

    #[test]
    fn shell_session_debug_never_reveals_the_authenticator() {
        let session = shell::ShellSession::new(1_000);
        assert_eq!(format!("{session:?}"), "ShellSession(<locked>)");
        let host = TestHost::new(TestRoot::new().paths(), [b"retained value".to_vec()]);
        let collected = session.authenticator(&host).unwrap();
        assert_eq!(&*collected, b"retained value");
        assert_eq!(format!("{session:?}"), "ShellSession(<retained>)");
        // A second call reuses the retained value instead of consuming another.
        assert_eq!(&*session.authenticator(&host).unwrap(), b"retained value");
        assert_eq!(host.remaining_secrets(), 0);
        session.lock();
        assert_eq!(format!("{session:?}"), "ShellSession(<locked>)");
        assert!(matches!(
            session.authenticator(&host),
            Err(HostError::Unavailable)
        ));
    }
}
