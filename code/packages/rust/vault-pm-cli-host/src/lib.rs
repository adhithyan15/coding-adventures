//! Controlling-terminal secret input and OS entropy for the vault-pm CLI.

#![deny(missing_docs)]

use coding_adventures_csprng::fill_random;
use coding_adventures_ct_compare::ct_eq;
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Debug, Display, Formatter};

#[cfg(unix)]
#[path = "unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

/// Maximum accepted UTF-8 passphrase bytes.
pub const MAX_SECRET_BYTES: usize = 1_024;

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
    /// Two independently collected new-passphrase values did not match.
    SecretMismatch,
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
            Self::SecretMismatch => "vault-pm CLI host: secrets do not match",
            Self::InvalidEntropyRequest => "vault-pm CLI host: invalid entropy request",
            Self::EntropyUnavailable => "vault-pm CLI host: OS entropy unavailable",
            Self::UnsupportedPlatform => "vault-pm CLI host: unsupported platform",
        })
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
    /// Open a portable export using its distinct passphrase.
    ImportPassphrase,
}

impl SecretPrompt {
    fn message(self) -> &'static str {
        match self {
            Self::Unlock => "Vault passphrase: ",
            Self::NewPassphrase => "New vault passphrase: ",
            Self::ConfirmPassphrase => "Confirm vault passphrase: ",
            Self::ExportPassphrase => "Export passphrase: ",
            Self::ImportPassphrase => "Import passphrase: ",
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
            SecretPrompt::ImportPassphrase.message(),
            "Import passphrase: "
        );
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
