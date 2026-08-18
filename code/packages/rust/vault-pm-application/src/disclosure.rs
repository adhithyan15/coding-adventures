use crate::ApplicationError;
use coding_adventures_vault_records::AnyRecord;
use coding_adventures_zeroize::Zeroize;

/// One explicit secret-bearing field understood by the V1 application core.
///
/// Selectors are schema-specific so a host cannot accidentally reinterpret a
/// generic field name across record kinds. Opaque records have no revealable
/// fields because the application cannot classify their payload safely.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretFieldV1 {
    /// The password in a login record.
    LoginPassword,
    /// The optional private notes in a login record.
    LoginNotes,
    /// The body in a secure-note record.
    SecureNoteBody,
    /// The primary account number in a card record.
    CardNumber,
    /// The verification code in a card record.
    CardCvv,
    /// The raw shared-secret bytes in a TOTP record.
    TotpSecret,
    /// The token in an API-key record.
    ApiKeyToken,
    /// The password in a database-credential record.
    DatabasePassword,
}

/// Host-declared destination and authorization ceremony for one secret.
///
/// The application owns policy validation, while the host remains responsible
/// for proving terminal state, emitting the non-interactive warning, and
/// clearing a clipboard value only while it still owns that value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretDisclosureIntentV1 {
    /// Copy through a host clipboard adapter without writing secret stdout.
    ///
    /// Carries the confirmation flag for the same reason
    /// [`Self::InteractiveReveal`] does. Until VLT-PM46 this variant
    /// authorized *unconditionally* and was reachable only from tests, which
    /// made it a trap rather than a policy: the first caller to reach for it —
    /// naturally, the first slice to implement `--copy` — would have silently
    /// deleted the application-layer confirmation gate while appearing to
    /// describe the delivery channel more accurately. A destination is not an
    /// authorization, and putting a secret somewhere every process in a
    /// session can read is not the disclosure that needs *less* consent.
    Clipboard {
        /// Whether the user completed the explicit confirmation ceremony.
        confirmed: bool,
    },
    /// Reveal after a host confirmed an interactive controlling-TTY prompt.
    InteractiveReveal {
        /// Whether the user completed the explicit confirmation ceremony.
        confirmed: bool,
    },
    /// Reveal without a TTY only after both opt-in and warning obligations.
    UnsafeNonInteractiveReveal {
        /// Whether the explicit unsafe secret-output flag was supplied.
        unsafe_include_secrets: bool,
        /// Whether the host emitted its warning to standard error.
        warning_emitted: bool,
    },
}

impl SecretDisclosureIntentV1 {
    pub(crate) const fn authorize(self) -> Result<(), ApplicationError> {
        match self {
            Self::Clipboard { confirmed: true }
            | Self::InteractiveReveal { confirmed: true }
            | Self::UnsafeNonInteractiveReveal {
                unsafe_include_secrets: true,
                warning_emitted: true,
            } => Ok(()),
            Self::Clipboard { confirmed: false }
            | Self::InteractiveReveal { confirmed: false }
            | Self::UnsafeNonInteractiveReveal { .. } => Err(ApplicationError::InvalidInput),
        }
    }
}

/// Encoding of an explicitly revealed secret value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevealedSecretEncodingV1 {
    /// The bytes are valid UTF-8 text.
    Utf8,
    /// The bytes are an opaque binary value whose rendering is a host choice.
    Bytes,
}

/// One owned secret selected for an already-authorized host disclosure.
///
/// This type deliberately implements neither `Debug`, `Display`, nor `Clone`.
/// Its full-capacity byte allocation is observably wiped before release.
pub struct RevealedSecretV1 {
    bytes: Vec<u8>,
    encoding: RevealedSecretEncodingV1,
}

impl RevealedSecretV1 {
    /// Borrow the secret only for the duration of the host disclosure action.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Return whether the value is UTF-8 text or opaque bytes.
    pub const fn encoding(&self) -> RevealedSecretEncodingV1 {
        self.encoding
    }

    fn text(value: &str) -> Self {
        Self {
            bytes: value.as_bytes().to_vec(),
            encoding: RevealedSecretEncodingV1::Utf8,
        }
    }

    fn bytes(value: &[u8]) -> Self {
        Self {
            bytes: value.to_vec(),
            encoding: RevealedSecretEncodingV1::Bytes,
        }
    }
}

impl Zeroize for RevealedSecretV1 {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for RevealedSecretV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

pub(crate) fn select_secret(
    record: &AnyRecord,
    field: SecretFieldV1,
) -> Result<RevealedSecretV1, ApplicationError> {
    match (record, field) {
        (AnyRecord::Login(value), SecretFieldV1::LoginPassword) => {
            Ok(RevealedSecretV1::text(&value.password))
        }
        (AnyRecord::Login(value), SecretFieldV1::LoginNotes) => value
            .notes
            .as_deref()
            .map(RevealedSecretV1::text)
            .ok_or(ApplicationError::NotFound),
        (AnyRecord::SecureNote(value), SecretFieldV1::SecureNoteBody) => {
            Ok(RevealedSecretV1::text(&value.body))
        }
        (AnyRecord::Card(value), SecretFieldV1::CardNumber) => {
            Ok(RevealedSecretV1::text(&value.number))
        }
        (AnyRecord::Card(value), SecretFieldV1::CardCvv) => Ok(RevealedSecretV1::text(&value.cvv)),
        (AnyRecord::TotpSeed(value), SecretFieldV1::TotpSecret) => {
            Ok(RevealedSecretV1::bytes(&value.secret))
        }
        (AnyRecord::ApiKey(value), SecretFieldV1::ApiKeyToken) => {
            Ok(RevealedSecretV1::text(&value.token))
        }
        (AnyRecord::DatabaseCredential(value), SecretFieldV1::DatabasePassword) => {
            Ok(RevealedSecretV1::text(&value.password))
        }
        _ => Err(ApplicationError::InvalidInput),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_vault_records::{
        ApiKey, Card, DatabaseCredential, Login, SecureNote, TotpSeed,
    };

    #[test]
    fn disclosure_policy_requires_complete_reveal_ceremonies() {
        assert_eq!(
            SecretDisclosureIntentV1::Clipboard { confirmed: true }.authorize(),
            Ok(())
        );
        // The arm that used to authorize unconditionally.
        assert_eq!(
            SecretDisclosureIntentV1::Clipboard { confirmed: false }.authorize(),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            SecretDisclosureIntentV1::InteractiveReveal { confirmed: true }.authorize(),
            Ok(())
        );
        assert_eq!(
            SecretDisclosureIntentV1::InteractiveReveal { confirmed: false }.authorize(),
            Err(ApplicationError::InvalidInput)
        );
        for (unsafe_include_secrets, warning_emitted, expected) in [
            (false, false, Err(ApplicationError::InvalidInput)),
            (true, false, Err(ApplicationError::InvalidInput)),
            (false, true, Err(ApplicationError::InvalidInput)),
            (true, true, Ok(())),
        ] {
            assert_eq!(
                SecretDisclosureIntentV1::UnsafeNonInteractiveReveal {
                    unsafe_include_secrets,
                    warning_emitted,
                }
                .authorize(),
                expected
            );
        }
    }

    #[test]
    fn every_first_party_secret_field_is_selected_exactly() {
        let cases = [
            (
                AnyRecord::Login(Login {
                    title: "login".into(),
                    username: "user".into(),
                    password: "login-secret".into(),
                    urls: vec![],
                    notes: None,
                }),
                SecretFieldV1::LoginPassword,
                b"login-secret".as_slice(),
                RevealedSecretEncodingV1::Utf8,
            ),
            (
                AnyRecord::Login(Login {
                    title: "login".into(),
                    username: "user".into(),
                    password: "login-secret".into(),
                    urls: vec![],
                    notes: Some("login-notes-secret".into()),
                }),
                SecretFieldV1::LoginNotes,
                b"login-notes-secret".as_slice(),
                RevealedSecretEncodingV1::Utf8,
            ),
            (
                AnyRecord::SecureNote(SecureNote {
                    title: "note".into(),
                    body: "note-secret".into(),
                }),
                SecretFieldV1::SecureNoteBody,
                b"note-secret".as_slice(),
                RevealedSecretEncodingV1::Utf8,
            ),
            (
                AnyRecord::Card(Card {
                    title: "card".into(),
                    holder: "holder".into(),
                    number: "4111111111111111".into(),
                    expiry_month: 1,
                    expiry_year: 2030,
                    cvv: "123".into(),
                    billing_zip: None,
                }),
                SecretFieldV1::CardNumber,
                b"4111111111111111".as_slice(),
                RevealedSecretEncodingV1::Utf8,
            ),
            (
                AnyRecord::Card(Card {
                    title: "card".into(),
                    holder: "holder".into(),
                    number: "4111111111111111".into(),
                    expiry_month: 1,
                    expiry_year: 2030,
                    cvv: "123".into(),
                    billing_zip: None,
                }),
                SecretFieldV1::CardCvv,
                b"123".as_slice(),
                RevealedSecretEncodingV1::Utf8,
            ),
            (
                AnyRecord::TotpSeed(TotpSeed {
                    label: "totp".into(),
                    issuer: None,
                    secret: vec![0, 1, 2, 255],
                    algorithm: "SHA1".into(),
                    digits: 6,
                    period: 30,
                }),
                SecretFieldV1::TotpSecret,
                &[0, 1, 2, 255],
                RevealedSecretEncodingV1::Bytes,
            ),
            (
                AnyRecord::ApiKey(ApiKey {
                    label: "api".into(),
                    service: "service".into(),
                    token: "api-secret".into(),
                    scopes: vec![],
                    expires_at: None,
                }),
                SecretFieldV1::ApiKeyToken,
                b"api-secret".as_slice(),
                RevealedSecretEncodingV1::Utf8,
            ),
            (
                AnyRecord::DatabaseCredential(DatabaseCredential {
                    label: "database".into(),
                    engine: "postgres".into(),
                    host: "localhost".into(),
                    port: 5432,
                    database: None,
                    username: "user".into(),
                    password: "database-secret".into(),
                    lease_id: None,
                    expires_at: None,
                }),
                SecretFieldV1::DatabasePassword,
                b"database-secret".as_slice(),
                RevealedSecretEncodingV1::Utf8,
            ),
        ];

        for (record, field, expected, encoding) in cases {
            let mut revealed = select_secret(&record, field).unwrap();
            assert_eq!(revealed.as_bytes(), expected);
            assert_eq!(revealed.encoding(), encoding);
            revealed.zeroize();
            assert!(revealed.as_bytes().is_empty());
        }
    }

    #[test]
    fn wrong_schema_and_opaque_fields_fail_closed() {
        let login = AnyRecord::Login(Login {
            title: "login".into(),
            username: "user".into(),
            password: "secret".into(),
            urls: vec![],
            notes: None,
        });
        assert!(matches!(
            select_secret(&login, SecretFieldV1::CardCvv),
            Err(ApplicationError::InvalidInput)
        ));
        assert!(matches!(
            select_secret(&login, SecretFieldV1::LoginNotes),
            Err(ApplicationError::NotFound)
        ));

        let opaque = AnyRecord::Opaque {
            content_type: "future/secret/v1".into(),
            payload_bytes: vec![1, 2, 3],
        };
        assert!(matches!(
            select_secret(&opaque, SecretFieldV1::ApiKeyToken),
            Err(ApplicationError::InvalidInput)
        ));
    }
}
