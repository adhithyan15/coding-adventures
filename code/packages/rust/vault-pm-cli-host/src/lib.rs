//! Controlling-terminal secret input and OS entropy for the vault-pm CLI.

#![deny(missing_docs)]

use coding_adventures_csprng::fill_random;
use coding_adventures_ct_compare::ct_eq;
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Debug, Display, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

#[cfg(unix)]
#[path = "unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

pub mod clipboard;

/// Maximum accepted UTF-8 passphrase or item-secret bytes.
pub const MAX_SECRET_BYTES: usize = 1_024;
/// Maximum accepted UTF-8 bytes for one echoed item field.
pub const MAX_TEXT_BYTES: usize = 2_048;
/// Maximum accepted UTF-8 bytes for one interactive shell command line.
///
/// A command line carries selectors — item identifiers, revision identifiers,
/// field names, and a search query — never a passphrase or a record secret.
/// The bound exists so a pasted blob cannot force an unbounded allocation on
/// the terminal read path, not because long commands are meaningful.
pub const MAX_COMMAND_BYTES: usize = 1_024;

/// Fixed prompt written before each interactive shell command line.
///
/// Like every other prompt in this adapter it is a compile-time constant. The
/// shell never renders a vault name, item title, or previous result into its
/// prompt, so a stored value can never be mistaken for shell chrome.
const SHELL_COMMAND_PROMPT: &str = "vault-pm> ";

/// Stable, payload-free native CLI host failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliHostError {
    /// The process has no usable controlling terminal or console.
    TerminalUnavailable,
    /// The controlling terminal could not be opened or written.
    TerminalAccessFailed,
    /// Echo state could not be read, disabled, or restored safely.
    TerminalModeFailed,
    /// Secret input ended or failed before a complete line was read.
    SecretInputFailed,
    /// The collected secret was empty.
    EmptySecret,
    /// The collected secret exceeded [`MAX_SECRET_BYTES`].
    SecretTooLong,
    /// Echoed text input ended or failed before a complete line was read.
    TextInputFailed,
    /// A required echoed text field was empty.
    EmptyText,
    /// Echoed text was invalid UTF-8, contained controls, or exceeded its bound.
    InvalidText,
    /// Two independently collected new-passphrase values did not match.
    SecretMismatch,
    /// The caller supplied an empty portable-export destination or artifact.
    InvalidExportDestination,
    /// The portable-export destination already exists and was not replaced.
    ExportDestinationExists,
    /// The portable-export destination could not be durably written.
    ExportWriteFailed,
    /// The portable-import source was empty, not a file, or exceeded its bound.
    InvalidImportSource,
    /// The portable-import source could not be completely read.
    ImportReadFailed,
    /// The caller requested an empty entropy buffer.
    InvalidEntropyRequest,
    /// The operating-system CSPRNG failed to fill a requested buffer.
    EntropyUnavailable,
    /// This host has no usable clipboard session or utility (VLT-PM46 §4).
    ClipboardUnavailable,
    /// The value is not the printable ASCII this adapter can round-trip.
    ClipboardValueUnsupported,
    /// A clipboard utility failed, was killed, or could not be started.
    ClipboardWriteFailed,
    /// The clipboard could not be read back for a verified clear.
    ClipboardReadFailed,
    /// The detached verified-clear process could not be started.
    ClipboardClearScheduleFailed,
    /// A clipboard-clear parameter block was absent or malformed.
    InvalidClipboardClearRequest,
    /// No audited implementation exists for the current target.
    UnsupportedPlatform,
}

impl Display for CliHostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TerminalUnavailable => "vault-pm CLI host: terminal unavailable",
            Self::TerminalAccessFailed => "vault-pm CLI host: terminal access failed",
            Self::TerminalModeFailed => "vault-pm CLI host: terminal mode failed",
            Self::SecretInputFailed => "vault-pm CLI host: secret input failed",
            Self::EmptySecret => "vault-pm CLI host: empty secret",
            Self::SecretTooLong => "vault-pm CLI host: secret too long",
            Self::TextInputFailed => "vault-pm CLI host: text input failed",
            Self::EmptyText => "vault-pm CLI host: empty text",
            Self::InvalidText => "vault-pm CLI host: invalid text",
            Self::SecretMismatch => "vault-pm CLI host: secrets do not match",
            Self::InvalidExportDestination => "vault-pm CLI host: invalid export destination",
            Self::ExportDestinationExists => "vault-pm CLI host: export destination exists",
            Self::ExportWriteFailed => "vault-pm CLI host: export write failed",
            Self::InvalidImportSource => "vault-pm CLI host: invalid import source",
            Self::ImportReadFailed => "vault-pm CLI host: import read failed",
            Self::InvalidEntropyRequest => "vault-pm CLI host: invalid entropy request",
            Self::EntropyUnavailable => "vault-pm CLI host: OS entropy unavailable",
            Self::ClipboardUnavailable => "vault-pm CLI host: clipboard unavailable",
            Self::ClipboardValueUnsupported => "vault-pm CLI host: unsupported clipboard value",
            Self::ClipboardWriteFailed => "vault-pm CLI host: clipboard write failed",
            Self::ClipboardReadFailed => "vault-pm CLI host: clipboard read failed",
            Self::ClipboardClearScheduleFailed => {
                "vault-pm CLI host: clipboard clear could not be scheduled"
            }
            Self::InvalidClipboardClearRequest => {
                "vault-pm CLI host: invalid clipboard clear request"
            }
            Self::UnsupportedPlatform => "vault-pm CLI host: unsupported platform",
        })
    }
}

/// Fixed echoed prompts that can never contain caller-controlled text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextPrompt {
    /// Required login display title.
    LoginTitle,
    /// Required secure-note display title.
    SecureNoteTitle,
    /// Required payment-card display title.
    CardTitle,
    /// Required payment-card holder name.
    CardHolder,
    /// Required payment-card expiry month.
    CardExpiryMonth,
    /// Required four-digit payment-card expiry year.
    CardExpiryYear,
    /// Optional payment-card billing postal code.
    CardBillingPostalCode,
    /// Required API-key display label.
    ApiKeyLabel,
    /// Required API-key service name.
    ApiKeyService,
    /// Optional comma-separated API-key scopes.
    ApiKeyScopes,
    /// Optional API-key expiry in Unix seconds.
    ApiKeyExpiry,
    /// Required database-credential display label.
    DatabaseLabel,
    /// Required database engine identifier.
    DatabaseEngine,
    /// Required database host.
    DatabaseHost,
    /// Required database TCP port.
    DatabasePort,
    /// Optional database or catalog name.
    DatabaseName,
    /// Required database username.
    DatabaseUsername,
    /// Required TOTP display label.
    TotpLabel,
    /// Optional TOTP issuer.
    TotpIssuer,
    /// Required TOTP HMAC algorithm.
    TotpAlgorithm,
    /// Required TOTP output digit count.
    TotpDigits,
    /// Required TOTP period in seconds.
    TotpPeriod,
    /// Optional login username or account handle.
    LoginUsername,
    /// Optional primary login URL.
    LoginUrl,
    /// Required count of login URLs collected by the fixed repeated prompt.
    LoginUrlCount,
    /// Explicit confirmation before one audited terminal secret disclosure.
    SecretRevealConfirmation,
    /// Explicit confirmation before one audited clipboard secret disclosure.
    ///
    /// A separate prompt from [`Self::SecretRevealConfirmation`] because the
    /// two describe different consequences. Asking "reveal secret on this
    /// terminal?" and then putting the value somewhere every process in the
    /// session can read would manufacture a record of an agreement nobody
    /// made. VLT-PM46 §3.1.
    SecretCopyConfirmation,
}

impl TextPrompt {
    fn message(self) -> &'static str {
        match self {
            Self::LoginTitle | Self::SecureNoteTitle | Self::CardTitle => "Title: ",
            Self::CardHolder => "Cardholder: ",
            Self::CardExpiryMonth => "Expiry month (1-12): ",
            Self::CardExpiryYear => "Expiry year (YYYY): ",
            Self::CardBillingPostalCode => "Billing postal code (optional): ",
            Self::ApiKeyLabel => "Label: ",
            Self::ApiKeyService => "Service: ",
            Self::ApiKeyScopes => "Scopes (comma-separated, optional): ",
            Self::ApiKeyExpiry => "Expiry Unix seconds (optional): ",
            Self::DatabaseLabel => "Label: ",
            Self::DatabaseEngine => "Engine: ",
            Self::DatabaseHost => "Host: ",
            Self::DatabasePort => "Port: ",
            Self::DatabaseName => "Database (optional): ",
            Self::DatabaseUsername => "Username: ",
            Self::TotpLabel => "Label: ",
            Self::TotpIssuer => "Issuer (optional): ",
            Self::TotpAlgorithm => "Algorithm (SHA1/SHA256/SHA512): ",
            Self::TotpDigits => "Digits (6 or 8): ",
            Self::TotpPeriod => "Period seconds (1-3600): ",
            Self::LoginUsername => "Username: ",
            Self::LoginUrl => "URL: ",
            Self::LoginUrlCount => "URL count (0-16): ",
            Self::SecretRevealConfirmation => {
                "Reveal secret on this terminal? Type yes to continue: "
            }
            Self::SecretCopyConfirmation => {
                "Copy secret to this system's clipboard? Type yes to continue: "
            }
        }
    }

    const fn max_bytes(self) -> usize {
        match self {
            Self::LoginTitle
            | Self::SecureNoteTitle
            | Self::CardTitle
            | Self::CardHolder
            | Self::CardBillingPostalCode
            | Self::ApiKeyLabel
            | Self::ApiKeyService
            | Self::DatabaseLabel
            | Self::DatabaseEngine
            | Self::DatabaseHost
            | Self::DatabaseName
            | Self::DatabaseUsername
            | Self::TotpLabel
            | Self::TotpIssuer => 256,
            Self::TotpAlgorithm => 6,
            Self::TotpDigits => 1,
            Self::TotpPeriod => 4,
            Self::ApiKeyScopes => MAX_TEXT_BYTES,
            Self::ApiKeyExpiry => 20,
            Self::DatabasePort => 5,
            Self::CardExpiryMonth => 2,
            Self::CardExpiryYear => 4,
            Self::LoginUsername => 1_024,
            Self::LoginUrl => MAX_TEXT_BYTES,
            Self::LoginUrlCount => 2,
            Self::SecretRevealConfirmation | Self::SecretCopyConfirmation => 16,
        }
    }

    const fn allows_empty(self) -> bool {
        matches!(
            self,
            Self::LoginUsername
                | Self::CardBillingPostalCode
                | Self::ApiKeyScopes
                | Self::ApiKeyExpiry
                | Self::DatabaseName
                | Self::TotpIssuer
                | Self::SecretRevealConfirmation
                | Self::SecretCopyConfirmation
        )
    }
}

impl std::error::Error for CliHostError {}

/// Fixed prompts that can never contain caller-controlled text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretPrompt {
    /// Unlock an initialized local vault.
    Unlock,
    /// Collect the first value for a new vault passphrase.
    NewPassphrase,
    /// Confirm a newly collected vault passphrase.
    ConfirmPassphrase,
    /// Protect a portable export with a distinct passphrase.
    ExportPassphrase,
    /// Confirm the distinct portable-export passphrase.
    ConfirmExportPassphrase,
    /// Open a portable export using its distinct passphrase.
    ImportPassphrase,
    /// Collect a login item's password without terminal echo.
    LoginPassword,
    /// Collect optional private login notes without terminal echo.
    LoginNotes,
    /// Collect a secure-note body without terminal echo.
    SecureNoteBody,
    /// Collect a payment-card number without terminal echo.
    CardNumber,
    /// Collect a payment-card verification code without terminal echo.
    CardCvv,
    /// Collect an API-key token without terminal echo.
    ApiKeyToken,
    /// Collect a database password without terminal echo.
    DatabasePassword,
    /// Collect a TOTP seed in canonical Base32 without terminal echo.
    TotpSecret,
    /// Collect an opaque record's whole canonical-CBOR payload, as lowercase
    /// hexadecimal, without terminal echo.
    ///
    /// An opaque record's schema is unknown to this product, so no part of its
    /// payload can be shown to be non-secret. It is therefore collected with
    /// the same hidden ceremony as a password or a seed rather than with the
    /// echoed text prompts used for named metadata fields.
    OpaquePayload,
}

impl SecretPrompt {
    fn message(self) -> &'static str {
        match self {
            Self::Unlock => "Vault passphrase: ",
            Self::NewPassphrase => "New vault passphrase: ",
            Self::ConfirmPassphrase => "Confirm vault passphrase: ",
            Self::ExportPassphrase => "Export passphrase: ",
            Self::ConfirmExportPassphrase => "Confirm export passphrase: ",
            Self::ImportPassphrase => "Import passphrase: ",
            Self::LoginPassword => "Password: ",
            Self::LoginNotes => "Notes (optional): ",
            Self::SecureNoteBody => "Note: ",
            Self::CardNumber => "Card number: ",
            Self::CardCvv => "CVV: ",
            Self::ApiKeyToken => "Token: ",
            Self::DatabasePassword => "Password: ",
            Self::TotpSecret => "Secret (Base32): ",
            Self::OpaquePayload => "Payload (hex): ",
        }
    }
}

/// Stateless reader that always opens the process controlling terminal.
#[derive(Clone, Copy, Debug, Default)]
pub struct ControllingTerminal;

impl ControllingTerminal {
    /// Read one bounded, non-empty secret with terminal echo disabled.
    pub fn read_secret(&self, prompt: SecretPrompt) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
        #[cfg(any(unix, windows))]
        {
            let secret = platform::read_secret(prompt.message(), MAX_SECRET_BYTES)?;
            validate_secret(secret)
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = prompt;
            Err(CliHostError::UnsupportedPlatform)
        }
    }

    /// Read optional private login notes with terminal echo disabled.
    pub fn read_optional_login_notes(&self) -> Result<Option<Zeroizing<Vec<u8>>>, CliHostError> {
        #[cfg(any(unix, windows))]
        {
            let secret =
                platform::read_secret(SecretPrompt::LoginNotes.message(), MAX_SECRET_BYTES)?;
            validate_optional_secret(secret)
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(CliHostError::UnsupportedPlatform)
        }
    }

    /// Read and constant-time compare a new passphrase and confirmation.
    pub fn read_new_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
        confirm_new_passphrase(|prompt| self.read_secret(prompt))
    }

    /// Read and constant-time compare a portable-export passphrase and confirmation.
    pub fn read_export_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
        confirm_export_passphrase(|prompt| self.read_secret(prompt))
    }

    /// Read one bounded UTF-8 field without changing the terminal's echo mode.
    pub fn read_text(&self, prompt: TextPrompt) -> Result<Zeroizing<String>, CliHostError> {
        #[cfg(any(unix, windows))]
        {
            let bytes = platform::read_text(prompt.message(), prompt.max_bytes())?;
            validate_text(bytes, prompt.allows_empty())
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = prompt;
            Err(CliHostError::UnsupportedPlatform)
        }
    }

    /// Read one bounded interactive shell command line, or report end of input.
    ///
    /// The line is collected from the same controlling terminal every prompt
    /// uses — never from process standard input. A redirected or piped stdin
    /// therefore cannot drive an unlocked shell session, exactly as it cannot
    /// satisfy a passphrase prompt.
    ///
    /// Echo is left in the terminal's ordinary line-discipline state: a command
    /// line is not a secret, and hiding it would make the shell unusable. The
    /// caller is responsible for keeping secret-bearing input on the hidden
    /// [`Self::read_secret`] path.
    ///
    /// Returns:
    ///
    /// | Terminal event | Result |
    /// |---|---|
    /// | complete line | `Ok(Some(line))`, terminator removed |
    /// | empty line | `Ok(Some(""))` |
    /// | end of input (`Ctrl-D`, closed terminal) | `Ok(None)` |
    /// | invalid UTF-8, control character, or oversize | `Err(..)` |
    ///
    /// End of input is a value rather than an error because a foreground shell
    /// must be able to end its session cleanly, while an unexpected read
    /// failure must still fail closed.
    pub fn read_command_line(&self) -> Result<Option<Zeroizing<String>>, CliHostError> {
        #[cfg(any(unix, windows))]
        {
            match platform::read_line_or_eof(SHELL_COMMAND_PROMPT, MAX_COMMAND_BYTES)? {
                Some(bytes) => validate_text(bytes, true).map(Some),
                None => Ok(None),
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(CliHostError::UnsupportedPlatform)
        }
    }

    /// Require an exact echoed `yes` before a terminal secret disclosure.
    pub fn confirm_secret_reveal(&self) -> Result<bool, CliHostError> {
        let answer = self.read_text(TextPrompt::SecretRevealConfirmation)?;
        Ok(answer.as_str() == "yes")
    }

    /// Require an exact echoed `yes` before a clipboard secret disclosure.
    ///
    /// Same rule, different sentence: the person is agreeing to put the value
    /// where every process in their session can read it, not to see it on this
    /// terminal, and the prompt says so (VLT-PM46 §3.1).
    pub fn confirm_secret_copy(&self) -> Result<bool, CliHostError> {
        let answer = self.read_text(TextPrompt::SecretCopyConfirmation)?;
        Ok(answer.as_str() == "yes")
    }

    /// Write one quoted and control-escaped secret directly to the controlling
    /// terminal, never through ordinary process standard output.
    pub fn write_revealed_text(&self, value: &str) -> Result<(), CliHostError> {
        let escaped = escaped_revealed_text(value);
        #[cfg(any(unix, windows))]
        {
            platform::write_revealed_text(&escaped)
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = escaped;
            Err(CliHostError::UnsupportedPlatform)
        }
    }
}

fn escaped_revealed_text(value: &str) -> Zeroizing<String> {
    Zeroizing::new(format!("{value:?}"))
}

/// Durably create one explicit portable-export destination without replacing it.
///
/// V1 accepts only a non-empty encrypted artifact and a non-empty path. The
/// final path is opened with create-new semantics, so an existing file,
/// directory, or symbolic link is never followed or replaced. Unix hosts also
/// request owner-read/write mode at creation. If writing or synchronization
/// fails, the incomplete newly-created file is removed on a best-effort basis.
pub fn write_portable_export(destination: &Path, artifact: &[u8]) -> Result<(), CliHostError> {
    if destination.as_os_str().is_empty() || artifact.is_empty() {
        return Err(CliHostError::InvalidExportDestination);
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(destination).map_err(map_export_open_error)?;
    if file
        .write_all(artifact)
        .and_then(|()| file.sync_all())
        .is_err()
    {
        drop(file);
        let _ = fs::remove_file(destination);
        return Err(CliHostError::ExportWriteFailed);
    }
    Ok(())
}

/// Read one explicit encrypted portable artifact under an exact host ceiling.
///
/// The source must be a non-empty regular file. Metadata is checked before
/// allocation and the reader itself is capped at `max_bytes + 1`, so a file
/// that grows concurrently cannot force an unbounded allocation. Artifact
/// authentication and parsing remain application responsibilities.
pub fn read_portable_export(source: &Path, max_bytes: usize) -> Result<Vec<u8>, CliHostError> {
    if source.as_os_str().is_empty() || max_bytes == 0 {
        return Err(CliHostError::InvalidImportSource);
    }
    let file = File::open(source).map_err(|_| CliHostError::ImportReadFailed)?;
    let metadata = file
        .metadata()
        .map_err(|_| CliHostError::ImportReadFailed)?;
    if !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > u64::try_from(max_bytes).unwrap_or(u64::MAX)
    {
        return Err(CliHostError::InvalidImportSource);
    }
    let capacity =
        usize::try_from(metadata.len()).map_err(|_| CliHostError::InvalidImportSource)?;
    let limit = u64::try_from(max_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut artifact = Vec::with_capacity(capacity);
    file.take(limit)
        .read_to_end(&mut artifact)
        .map_err(|_| CliHostError::ImportReadFailed)?;
    if artifact.is_empty() || artifact.len() > max_bytes {
        return Err(CliHostError::InvalidImportSource);
    }
    Ok(artifact)
}

/// Stateless cryptographic entropy adapter backed by the repository OS CSPRNG.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsEntropy;

impl OsEntropy {
    /// Completely overwrite a non-empty caller buffer with fresh OS entropy.
    pub fn fill(&self, output: &mut [u8]) -> Result<(), CliHostError> {
        if output.is_empty() {
            return Err(CliHostError::InvalidEntropyRequest);
        }
        fill_random(output).map_err(|_| CliHostError::EntropyUnavailable)
    }
}

fn validate_secret(secret: Zeroizing<Vec<u8>>) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
    if secret.is_empty() {
        Err(CliHostError::EmptySecret)
    } else if secret.len() > MAX_SECRET_BYTES {
        Err(CliHostError::SecretTooLong)
    } else {
        Ok(secret)
    }
}

fn validate_optional_secret(
    secret: Zeroizing<Vec<u8>>,
) -> Result<Option<Zeroizing<Vec<u8>>>, CliHostError> {
    if secret.len() > MAX_SECRET_BYTES {
        Err(CliHostError::SecretTooLong)
    } else if secret.is_empty() {
        Ok(None)
    } else {
        Ok(Some(secret))
    }
}

fn validate_text(
    bytes: Zeroizing<Vec<u8>>,
    allows_empty: bool,
) -> Result<Zeroizing<String>, CliHostError> {
    if bytes.is_empty() && !allows_empty {
        return Err(CliHostError::EmptyText);
    }
    let text = core::str::from_utf8(&bytes).map_err(|_| CliHostError::InvalidText)?;
    if text.chars().any(char::is_control) {
        return Err(CliHostError::InvalidText);
    }
    Ok(Zeroizing::new(text.to_owned()))
}

fn confirm_new_passphrase<F>(mut read: F) -> Result<Zeroizing<Vec<u8>>, CliHostError>
where
    F: FnMut(SecretPrompt) -> Result<Zeroizing<Vec<u8>>, CliHostError>,
{
    let first = read(SecretPrompt::NewPassphrase)?;
    let confirmation = read(SecretPrompt::ConfirmPassphrase)?;
    if ct_eq(&first, &confirmation) {
        Ok(first)
    } else {
        Err(CliHostError::SecretMismatch)
    }
}

fn confirm_export_passphrase<F>(mut read: F) -> Result<Zeroizing<Vec<u8>>, CliHostError>
where
    F: FnMut(SecretPrompt) -> Result<Zeroizing<Vec<u8>>, CliHostError>,
{
    let first = read(SecretPrompt::ExportPassphrase)?;
    let confirmation = read(SecretPrompt::ConfirmExportPassphrase)?;
    if ct_eq(&first, &confirmation) {
        Ok(first)
    } else {
        Err(CliHostError::SecretMismatch)
    }
}

fn map_export_open_error(error: io::Error) -> CliHostError {
    if error.kind() == io::ErrorKind::AlreadyExists {
        CliHostError::ExportDestinationExists
    } else {
        CliHostError::ExportWriteFailed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_prompts_and_errors_are_payload_free() {
        assert_eq!(SecretPrompt::Unlock.message(), "Vault passphrase: ");
        assert_eq!(
            SecretPrompt::NewPassphrase.message(),
            "New vault passphrase: "
        );
        assert_eq!(
            SecretPrompt::ConfirmPassphrase.message(),
            "Confirm vault passphrase: "
        );
        assert_eq!(
            SecretPrompt::ExportPassphrase.message(),
            "Export passphrase: "
        );
        assert_eq!(
            SecretPrompt::ConfirmExportPassphrase.message(),
            "Confirm export passphrase: "
        );
        assert_eq!(
            SecretPrompt::ImportPassphrase.message(),
            "Import passphrase: "
        );
        assert_eq!(SecretPrompt::LoginPassword.message(), "Password: ");
        assert_eq!(SecretPrompt::LoginNotes.message(), "Notes (optional): ");
        assert_eq!(SecretPrompt::SecureNoteBody.message(), "Note: ");
        assert_eq!(SecretPrompt::CardNumber.message(), "Card number: ");
        assert_eq!(SecretPrompt::CardCvv.message(), "CVV: ");
        assert_eq!(SecretPrompt::ApiKeyToken.message(), "Token: ");
        assert_eq!(SecretPrompt::DatabasePassword.message(), "Password: ");
        assert_eq!(SecretPrompt::TotpSecret.message(), "Secret (Base32): ");
        assert_eq!(SecretPrompt::OpaquePayload.message(), "Payload (hex): ");
        assert_eq!(TextPrompt::LoginTitle.message(), "Title: ");
        assert_eq!(TextPrompt::SecureNoteTitle.message(), "Title: ");
        assert_eq!(TextPrompt::CardTitle.message(), "Title: ");
        assert_eq!(TextPrompt::CardHolder.message(), "Cardholder: ");
        assert_eq!(
            TextPrompt::CardExpiryMonth.message(),
            "Expiry month (1-12): "
        );
        assert_eq!(TextPrompt::CardExpiryYear.message(), "Expiry year (YYYY): ");
        assert_eq!(
            TextPrompt::CardBillingPostalCode.message(),
            "Billing postal code (optional): "
        );
        assert_eq!(TextPrompt::ApiKeyLabel.message(), "Label: ");
        assert_eq!(TextPrompt::ApiKeyService.message(), "Service: ");
        assert_eq!(
            TextPrompt::ApiKeyScopes.message(),
            "Scopes (comma-separated, optional): "
        );
        assert_eq!(
            TextPrompt::ApiKeyExpiry.message(),
            "Expiry Unix seconds (optional): "
        );
        assert_eq!(TextPrompt::DatabaseLabel.message(), "Label: ");
        assert_eq!(TextPrompt::DatabaseEngine.message(), "Engine: ");
        assert_eq!(TextPrompt::DatabaseHost.message(), "Host: ");
        assert_eq!(TextPrompt::DatabasePort.message(), "Port: ");
        assert_eq!(TextPrompt::DatabaseName.message(), "Database (optional): ");
        assert_eq!(TextPrompt::DatabaseUsername.message(), "Username: ");
        assert_eq!(TextPrompt::TotpLabel.message(), "Label: ");
        assert_eq!(TextPrompt::TotpIssuer.message(), "Issuer (optional): ");
        assert_eq!(
            TextPrompt::TotpAlgorithm.message(),
            "Algorithm (SHA1/SHA256/SHA512): "
        );
        assert_eq!(TextPrompt::TotpDigits.message(), "Digits (6 or 8): ");
        assert_eq!(
            TextPrompt::TotpPeriod.message(),
            "Period seconds (1-3600): "
        );
        assert_eq!(TextPrompt::LoginUsername.message(), "Username: ");
        assert_eq!(TextPrompt::LoginUrl.message(), "URL: ");
        assert_eq!(TextPrompt::LoginUrlCount.message(), "URL count (0-16): ");
        assert_eq!(
            TextPrompt::SecretRevealConfirmation.message(),
            "Reveal secret on this terminal? Type yes to continue: "
        );
        // The clipboard prompt names the clipboard. VLT-PM46 §3.1: a consent
        // ceremony that misdescribes what it consents to is worse than none.
        assert_eq!(
            TextPrompt::SecretCopyConfirmation.message(),
            "Copy secret to this system's clipboard? Type yes to continue: "
        );
        assert_ne!(
            TextPrompt::SecretCopyConfirmation.message(),
            TextPrompt::SecretRevealConfirmation.message()
        );
        assert_eq!(TextPrompt::SecretCopyConfirmation.max_bytes(), 16);
        assert!(!TextPrompt::LoginTitle.allows_empty());
        assert!(!TextPrompt::SecureNoteTitle.allows_empty());
        assert!(!TextPrompt::CardTitle.allows_empty());
        assert!(!TextPrompt::CardHolder.allows_empty());
        assert!(!TextPrompt::CardExpiryMonth.allows_empty());
        assert!(!TextPrompt::CardExpiryYear.allows_empty());
        assert!(TextPrompt::CardBillingPostalCode.allows_empty());
        assert!(!TextPrompt::ApiKeyLabel.allows_empty());
        assert!(!TextPrompt::ApiKeyService.allows_empty());
        assert!(TextPrompt::ApiKeyScopes.allows_empty());
        assert!(TextPrompt::ApiKeyExpiry.allows_empty());
        assert!(!TextPrompt::DatabaseLabel.allows_empty());
        assert!(!TextPrompt::DatabaseEngine.allows_empty());
        assert!(!TextPrompt::DatabaseHost.allows_empty());
        assert!(!TextPrompt::DatabasePort.allows_empty());
        assert!(TextPrompt::DatabaseName.allows_empty());
        assert!(!TextPrompt::DatabaseUsername.allows_empty());
        assert!(!TextPrompt::TotpLabel.allows_empty());
        assert!(TextPrompt::TotpIssuer.allows_empty());
        assert!(!TextPrompt::TotpAlgorithm.allows_empty());
        assert!(!TextPrompt::TotpDigits.allows_empty());
        assert!(!TextPrompt::TotpPeriod.allows_empty());
        assert!(TextPrompt::LoginUsername.allows_empty());
        assert!(!TextPrompt::LoginUrl.allows_empty());
        assert!(!TextPrompt::LoginUrlCount.allows_empty());
        assert!(TextPrompt::SecretRevealConfirmation.allows_empty());
        assert!(TextPrompt::SecretCopyConfirmation.allows_empty());
        let expected = [
            (
                CliHostError::TerminalUnavailable,
                "vault-pm CLI host: terminal unavailable",
            ),
            (
                CliHostError::TerminalAccessFailed,
                "vault-pm CLI host: terminal access failed",
            ),
            (
                CliHostError::TerminalModeFailed,
                "vault-pm CLI host: terminal mode failed",
            ),
            (
                CliHostError::SecretInputFailed,
                "vault-pm CLI host: secret input failed",
            ),
            (CliHostError::EmptySecret, "vault-pm CLI host: empty secret"),
            (
                CliHostError::SecretTooLong,
                "vault-pm CLI host: secret too long",
            ),
            (
                CliHostError::SecretMismatch,
                "vault-pm CLI host: secrets do not match",
            ),
            (
                CliHostError::InvalidExportDestination,
                "vault-pm CLI host: invalid export destination",
            ),
            (
                CliHostError::ExportDestinationExists,
                "vault-pm CLI host: export destination exists",
            ),
            (
                CliHostError::ExportWriteFailed,
                "vault-pm CLI host: export write failed",
            ),
            (
                CliHostError::InvalidImportSource,
                "vault-pm CLI host: invalid import source",
            ),
            (
                CliHostError::ImportReadFailed,
                "vault-pm CLI host: import read failed",
            ),
            (
                CliHostError::TextInputFailed,
                "vault-pm CLI host: text input failed",
            ),
            (CliHostError::EmptyText, "vault-pm CLI host: empty text"),
            (CliHostError::InvalidText, "vault-pm CLI host: invalid text"),
            (
                CliHostError::InvalidEntropyRequest,
                "vault-pm CLI host: invalid entropy request",
            ),
            (
                CliHostError::EntropyUnavailable,
                "vault-pm CLI host: OS entropy unavailable",
            ),
            (
                CliHostError::UnsupportedPlatform,
                "vault-pm CLI host: unsupported platform",
            ),
            (
                CliHostError::ClipboardUnavailable,
                "vault-pm CLI host: clipboard unavailable",
            ),
            (
                CliHostError::ClipboardValueUnsupported,
                "vault-pm CLI host: unsupported clipboard value",
            ),
            (
                CliHostError::ClipboardWriteFailed,
                "vault-pm CLI host: clipboard write failed",
            ),
            (
                CliHostError::ClipboardReadFailed,
                "vault-pm CLI host: clipboard read failed",
            ),
            (
                CliHostError::ClipboardClearScheduleFailed,
                "vault-pm CLI host: clipboard clear could not be scheduled",
            ),
            (
                CliHostError::InvalidClipboardClearRequest,
                "vault-pm CLI host: invalid clipboard clear request",
            ),
        ];
        for (error, display) in expected {
            assert_eq!(error.to_string(), display);
        }
    }

    #[test]
    fn revealed_text_is_quoted_and_control_escaped() {
        assert_eq!(&*escaped_revealed_text("plain secret"), "\"plain secret\"");
        assert_eq!(
            &*escaped_revealed_text("line\n\"terminal\u{1b}"),
            "\"line\\n\\\"terminal\\u{1b}\""
        );
    }

    #[test]
    fn validation_enforces_nonempty_bounded_secrets() {
        assert!(matches!(
            validate_secret(Zeroizing::new(Vec::new())),
            Err(CliHostError::EmptySecret)
        ));
        assert!(matches!(
            validate_secret(Zeroizing::new(vec![b'x'; MAX_SECRET_BYTES + 1])),
            Err(CliHostError::SecretTooLong)
        ));
        assert_eq!(
            &*validate_secret(Zeroizing::new(vec![b'x'; MAX_SECRET_BYTES])).unwrap(),
            &vec![b'x'; MAX_SECRET_BYTES]
        );
        assert!(validate_optional_secret(Zeroizing::new(Vec::new()))
            .unwrap()
            .is_none());
        assert_eq!(
            &**validate_optional_secret(Zeroizing::new(vec![b'x'; MAX_SECRET_BYTES]))
                .unwrap()
                .unwrap(),
            &vec![b'x'; MAX_SECRET_BYTES]
        );
        assert!(matches!(
            validate_optional_secret(Zeroizing::new(vec![b'x'; MAX_SECRET_BYTES + 1])),
            Err(CliHostError::SecretTooLong)
        ));
    }

    #[test]
    fn text_validation_enforces_utf8_controls_and_empty_policy() {
        assert!(matches!(
            validate_text(Zeroizing::new(Vec::new()), false),
            Err(CliHostError::EmptyText)
        ));
        assert_eq!(
            &*validate_text(Zeroizing::new(Vec::new()), true).unwrap(),
            ""
        );
        assert!(matches!(
            validate_text(Zeroizing::new(vec![0xff]), false),
            Err(CliHostError::InvalidText)
        ));
        assert!(matches!(
            validate_text(Zeroizing::new(b"line\nbreak".to_vec()), false),
            Err(CliHostError::InvalidText)
        ));
        assert_eq!(
            &*validate_text(Zeroizing::new("Ada 🐎".as_bytes().to_vec()), false).unwrap(),
            "Ada 🐎"
        );
    }

    #[test]
    fn confirmation_uses_two_fixed_prompts_and_rejects_mismatch() {
        let mut prompts = Vec::new();
        let matched = confirm_new_passphrase(|prompt| {
            prompts.push(prompt);
            Ok(Zeroizing::new(b"same secret".to_vec()))
        })
        .unwrap();
        assert_eq!(&*matched, b"same secret");
        assert_eq!(
            prompts,
            [SecretPrompt::NewPassphrase, SecretPrompt::ConfirmPassphrase]
        );

        let mut call = 0;
        assert!(matches!(
            confirm_new_passphrase(|_| {
                call += 1;
                Ok(Zeroizing::new(if call == 1 {
                    b"first".to_vec()
                } else {
                    b"second".to_vec()
                }))
            }),
            Err(CliHostError::SecretMismatch)
        ));

        let mut export_prompts = Vec::new();
        let export = confirm_export_passphrase(|prompt| {
            export_prompts.push(prompt);
            Ok(Zeroizing::new(b"portable secret".to_vec()))
        })
        .unwrap();
        assert_eq!(&*export, b"portable secret");
        assert_eq!(
            export_prompts,
            [
                SecretPrompt::ExportPassphrase,
                SecretPrompt::ConfirmExportPassphrase
            ]
        );
    }

    #[test]
    fn portable_export_writer_is_durable_private_and_never_overwrites() {
        let destination = std::env::temp_dir().join(format!(
            "vault-pm-cli-host-export-{}.vpm",
            std::process::id()
        ));
        let _ = fs::remove_file(&destination);
        write_portable_export(&destination, b"encrypted artifact").unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"encrypted artifact");
        assert_eq!(
            write_portable_export(&destination, b"replacement").unwrap_err(),
            CliHostError::ExportDestinationExists
        );
        assert_eq!(fs::read(&destination).unwrap(), b"encrypted artifact");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        fs::remove_file(destination).unwrap();
    }

    #[test]
    fn portable_import_reader_is_regular_nonempty_and_bounded() {
        let source = std::env::temp_dir().join(format!(
            "vault-pm-cli-host-import-{}.vpm",
            std::process::id()
        ));
        let _ = fs::remove_file(&source);
        fs::write(&source, b"encrypted artifact").unwrap();
        assert_eq!(
            read_portable_export(&source, b"encrypted artifact".len()).unwrap(),
            b"encrypted artifact"
        );
        assert_eq!(
            read_portable_export(&source, b"encrypted artifact".len() - 1).unwrap_err(),
            CliHostError::InvalidImportSource
        );
        assert_eq!(
            read_portable_export(
                source.parent().expect("temporary import parent"),
                b"encrypted artifact".len(),
            )
            .unwrap_err(),
            CliHostError::InvalidImportSource
        );
        fs::write(&source, b"").unwrap();
        assert_eq!(
            read_portable_export(&source, 64).unwrap_err(),
            CliHostError::InvalidImportSource
        );
        fs::remove_file(source).unwrap();
    }

    #[test]
    fn os_entropy_fills_exact_caller_buffer_and_rejects_empty_requests() {
        let entropy = OsEntropy;
        let mut first = [0u8; 64];
        let mut second = [0u8; 64];
        entropy.fill(&mut first).unwrap();
        entropy.fill(&mut second).unwrap();
        assert!(first.iter().any(|byte| *byte != 0));
        assert_ne!(first, second);
        assert_eq!(
            entropy.fill(&mut []).unwrap_err(),
            CliHostError::InvalidEntropyRequest
        );
    }
}
