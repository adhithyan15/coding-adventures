//! Controlling-terminal secret input and OS entropy for the vault-pm CLI.

#![deny(missing_docs)]

use coding_adventures_csprng::fill_random;
use coding_adventures_ct_compare::ct_eq;
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Debug, Display, Formatter};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

#[cfg(unix)]
#[path = "unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

/// Maximum accepted UTF-8 passphrase or item-secret bytes.
pub const MAX_SECRET_BYTES: usize = 1_024;
/// Maximum accepted UTF-8 bytes for one echoed item field.
pub const MAX_TEXT_BYTES: usize = 2_048;

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
    /// The caller requested an empty entropy buffer.
    InvalidEntropyRequest,
    /// The operating-system CSPRNG failed to fill a requested buffer.
    EntropyUnavailable,
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
            Self::InvalidEntropyRequest => "vault-pm CLI host: invalid entropy request",
            Self::EntropyUnavailable => "vault-pm CLI host: OS entropy unavailable",
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
    /// Optional login username or account handle.
    LoginUsername,
    /// Optional primary login URL.
    LoginUrl,
}

impl TextPrompt {
    fn message(self) -> &'static str {
        match self {
            Self::LoginTitle | Self::SecureNoteTitle => "Title: ",
            Self::LoginUsername => "Username: ",
            Self::LoginUrl => "URL (optional): ",
        }
    }

    const fn max_bytes(self) -> usize {
        match self {
            Self::LoginTitle | Self::SecureNoteTitle => 256,
            Self::LoginUsername => 1_024,
            Self::LoginUrl => MAX_TEXT_BYTES,
        }
    }

    const fn allows_empty(self) -> bool {
        matches!(self, Self::LoginUsername | Self::LoginUrl)
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
    /// Collect a secure-note body without terminal echo.
    SecureNoteBody,
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
            Self::SecureNoteBody => "Note: ",
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
        assert_eq!(SecretPrompt::SecureNoteBody.message(), "Note: ");
        assert_eq!(TextPrompt::LoginTitle.message(), "Title: ");
        assert_eq!(TextPrompt::SecureNoteTitle.message(), "Title: ");
        assert_eq!(TextPrompt::LoginUsername.message(), "Username: ");
        assert_eq!(TextPrompt::LoginUrl.message(), "URL (optional): ");
        assert!(!TextPrompt::LoginTitle.allows_empty());
        assert!(!TextPrompt::SecureNoteTitle.allows_empty());
        assert!(TextPrompt::LoginUsername.allows_empty());
        assert!(TextPrompt::LoginUrl.allows_empty());
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
        ];
        for (error, display) in expected {
            assert_eq!(error.to_string(), display);
        }
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
