//! Exact-message TLS 1.3 retry handshake traffic-key binding.
//!
//! This crate consumes only the existing exact-message retry handshake
//! traffic-secret proof and derives one sender's traffic key and static IV into
//! caller-owned wipe-on-drop storage. It can consume the resulting monotonic
//! record state into bounded reassembly so only authenticated record content
//! becomes policy-authorized handshake messages.

#![forbid(unsafe_code)]

use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk, HkdfPrkImportError};
use core::cell::Cell;
use core::fmt;
use tls13_certificate_message::{
    decode_tls13_certificate_message, Tls13CertificateMessage, Tls13CertificateMessageErrorKind,
    Tls13CertificateMessageLimits,
};
use tls13_certificate_request_message::{
    decode_certificate_request, CertificateRequest, CertificateRequestError,
    CertificateRequestLimits,
};
use tls13_certificate_verify_input::CertificateVerifyRole;
use tls13_certificate_verify_message::{CertificateVerifyMessage, MessageError};
use tls13_certificate_verify_transcript_binding::VerifiedTranscriptBoundCertificateVerify;
use tls13_cipher_suite_policy::{HkdfHash, SelectedCipherSuite};
use tls13_encrypted_extensions_message::{
    decode_encrypted_extensions_message, EncryptedExtensionsMessage,
};
use tls13_extension_block::{
    decode_extension_block, ExtensionBlockError, ExtensionBlockLimits, ExtensionType,
};
use tls13_extension_context_policy::{ExtensionContextPolicyError, ServerExtensionContext};
use tls13_finished::{FinishedError, FinishedRole, VerifiedFinished};
use tls13_handshake::{Tls13HandshakeMessage, Tls13HandshakePolicy};
use tls13_handshake_reassembly::{ReassemblyError, ReassemblyProgress, Tls13HandshakeReassembler};
use tls13_inner_plaintext::Tls13InnerPlaintextPolicy;
use tls13_no_psk_server_flight::{NoPskServerFlight, ServerFlightError, ServerFlightState};
use tls13_record::{Tls13CiphertextPolicy, TlsContentType};
use tls13_record_protection::{
    open_tls13_record, seal_tls13_record, OpenedTls13Record, RecordProtectionError, Tls13Plaintext,
};
use tls13_record_protection_state::{RecordProtectionState, RecordProtectionStateError};
use tls13_retry_handshake_traffic_secrets_binding::{
    BoundRetryHandshakeTrafficSecrets, BoundRetryServerFinishedKey,
    BoundRetryServerTranscriptContinuation, ExactRetryBaseSecrets,
};
use tls13_signature_scheme_list::{
    decode_signature_scheme_list, SignatureSchemeList, SignatureSchemeListError,
    SignatureSchemeListLimits, SIGNATURE_ALGORITHMS_CERT_EXTENSION_TYPE,
    SIGNATURE_ALGORITHMS_EXTENSION_TYPE,
};
use tls13_signature_scheme_policy::{
    AuthorizedSignatureScheme, PolicyError as SignatureSchemePolicyError, SignatureSchemePolicy,
};
use tls13_traffic_key_material::{
    derive_traffic_key_material, TrafficKeyMaterial, TrafficKeyMaterialError,
    TrafficKeyMaterialStorage,
};
use tls13_transcript::{SelectedTls13Transcript, TranscriptError, TranscriptHash};
use x509_certificate::X509Certificate;

/// Sender whose retained handshake traffic secret supplies one key and IV.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandshakeTrafficSender {
    /// Use the exact retained client handshake traffic secret.
    Client,
    /// Use the exact retained server handshake traffic secret.
    Server,
}

/// Stable payload-free failures for exact retry traffic-key binding.
#[derive(Debug, Eq, PartialEq)]
pub enum RetryHandshakeTrafficKeyMaterialBindingError {
    /// Caller-owned output storage retains a different selected suite.
    CipherSuiteMismatch,
    /// The retained hash-length traffic secret could not enter fixed PRK storage.
    TrafficSecretImport(HkdfPrkImportError),
    /// Repository-owned traffic-key or static-IV derivation failed.
    KeyMaterial(TrafficKeyMaterialError),
}

impl fmt::Display for RetryHandshakeTrafficKeyMaterialBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CipherSuiteMismatch => {
                "TLS 1.3 retry traffic secret and key-material storage use different cipher suites"
            }
            Self::TrafficSecretImport(_) => "TLS 1.3 retry handshake traffic-secret import failed",
            Self::KeyMaterial(_) => "TLS 1.3 retry handshake traffic-key derivation failed",
        })
    }
}

impl std::error::Error for RetryHandshakeTrafficKeyMaterialBindingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TrafficSecretImport(error) => Some(error),
            Self::KeyMaterial(error) => Some(error),
            Self::CipherSuiteMismatch => None,
        }
    }
}

/// Stable payload-free failures while issuing exact retry record state.
#[derive(Debug, Eq, PartialEq)]
pub enum RetryHandshakeRecordProtectionStateBindingError {
    /// This issuer already created record state for the requested sender.
    SenderAlreadyIssued,
    /// Repository-owned key-material binding failed.
    KeyMaterial(RetryHandshakeTrafficKeyMaterialBindingError),
    /// Repository-owned record-state construction failed.
    RecordState(RecordProtectionStateError),
}

impl fmt::Display for RetryHandshakeRecordProtectionStateBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SenderAlreadyIssued => {
                "TLS 1.3 retry handshake record state already issued for sender"
            }
            Self::KeyMaterial(_) => "TLS 1.3 retry handshake record-state key derivation failed",
            Self::RecordState(_) => "TLS 1.3 retry handshake record-state composition failed",
        })
    }
}

impl std::error::Error for RetryHandshakeRecordProtectionStateBindingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::KeyMaterial(error) => Some(error),
            Self::RecordState(error) => Some(error),
            Self::SenderAlreadyIssued => None,
        }
    }
}

/// Stable payload-free failures while binding retry handshake records.
#[derive(Debug, Eq, PartialEq)]
pub enum RetryHandshakeRecordBindingError {
    /// Repository-owned TLS 1.3 record protection failed.
    RecordProtection(RecordProtectionError),
    /// A successfully authenticated record was not a handshake record.
    UnexpectedContentType,
}

impl fmt::Display for RetryHandshakeRecordBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RecordProtection(_) => "TLS 1.3 retry handshake record protection failed",
            Self::UnexpectedContentType => {
                "TLS 1.3 retry record authenticated with a non-handshake content type"
            }
        })
    }
}

impl std::error::Error for RetryHandshakeRecordBindingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::RecordProtection(error) => Some(error),
            Self::UnexpectedContentType => None,
        }
    }
}

/// Stable payload-free failures while opening and reassembling retry records.
#[derive(Debug, Eq, PartialEq)]
pub enum RetryHandshakeRecordReassemblyError {
    /// A prior failure permanently closed this reassembly owner.
    OwnerFailed,
    /// Repository-owned record binding failed.
    Record(RetryHandshakeRecordBindingError),
    /// Repository-owned bounded handshake reassembly failed.
    Reassembly(ReassemblyError),
}

impl fmt::Display for RetryHandshakeRecordReassemblyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::OwnerFailed => "TLS 1.3 retry handshake reassembly owner already failed",
            Self::Record(_) => "TLS 1.3 retry record authentication failed during reassembly",
            Self::Reassembly(_) => "TLS 1.3 retry handshake reassembly failed",
        })
    }
}

impl std::error::Error for RetryHandshakeRecordReassemblyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Record(error) => Some(error),
            Self::Reassembly(error) => Some(error),
            Self::OwnerFailed => None,
        }
    }
}

/// Stable payload-free failures while enforcing the retry server-flight order.
#[derive(Debug, Eq, PartialEq)]
pub enum RetryOrderedServerFlightError {
    /// The supplied authenticated record state belongs to the client sender.
    ClientSender,
    /// The supplied reassembler already consumed or failed a record.
    NonFreshReassembler,
    /// Authenticated record opening or bounded reassembly failed.
    Reassembly(RetryHandshakeRecordReassemblyError),
    /// The authenticated message type violated the no-PSK server-flight order.
    Ordering(ServerFlightError),
}

/// Stable payload-free failures while joining ordered flight and transcript provenance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryOrderedServerFlightTranscriptBindingError {
    /// The ordered owner already consumed, buffered, accepted, or failed input.
    NonFreshOrderedFlight,
    /// Ordered records and the Finished-key continuation retain different secrets.
    TrafficSecretProofMismatch,
    /// Ordered records, Finished key, and transcript selected different suites.
    CipherSuiteMismatch,
    /// The continuation no longer matches the shared proof's ServerHello snapshot.
    SnapshotMismatch,
}

impl fmt::Display for RetryOrderedServerFlightTranscriptBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFreshOrderedFlight => {
                "TLS 1.3 ordered server-flight transcript binding received non-fresh state"
            }
            Self::TrafficSecretProofMismatch => {
                "TLS 1.3 ordered server-flight and transcript retain different retry proofs"
            }
            Self::CipherSuiteMismatch => {
                "TLS 1.3 ordered server-flight transcript components use different cipher suites"
            }
            Self::SnapshotMismatch => {
                "TLS 1.3 ordered server-flight transcript does not match the retained snapshot"
            }
        })
    }
}

impl std::error::Error for RetryOrderedServerFlightTranscriptBindingError {}

/// Stable payload-free failures while advancing one authenticated ordered transcript.
#[derive(Debug, Eq, PartialEq)]
pub enum RetryOrderedServerTranscriptAdvanceError {
    /// Authenticated record opening or bounded reassembly failed.
    Reassembly(RetryHandshakeRecordReassemblyError),
    /// The authenticated message type violated the no-PSK server-flight order.
    Ordering(ServerFlightError),
    /// The synchronous caller rejected the exact ordered message.
    MessageRejected,
    /// Repository-owned transcript advancement failed.
    Transcript(TranscriptError),
}

impl fmt::Display for RetryOrderedServerTranscriptAdvanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Reassembly(_) => "TLS 1.3 ordered transcript authenticated reassembly failed",
            Self::Ordering(_) => "TLS 1.3 ordered transcript server-flight ordering failed",
            Self::MessageRejected => "TLS 1.3 ordered transcript message was rejected",
            Self::Transcript(_) => "TLS 1.3 ordered transcript advancement failed",
        })
    }
}

impl std::error::Error for RetryOrderedServerTranscriptAdvanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Reassembly(error) => Some(error),
            Self::Ordering(error) => Some(error),
            Self::Transcript(error) => Some(error),
            Self::MessageRejected => None,
        }
    }
}

impl fmt::Display for RetryOrderedServerFlightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ClientSender => {
                "TLS 1.3 ordered server-flight composition received client sender state"
            }
            Self::NonFreshReassembler => {
                "TLS 1.3 ordered server-flight composition received a non-fresh reassembler"
            }
            Self::Reassembly(_) => "TLS 1.3 ordered server-flight authenticated reassembly failed",
            Self::Ordering(_) => "TLS 1.3 authenticated server-flight ordering failed",
        })
    }
}

impl std::error::Error for RetryOrderedServerFlightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Reassembly(error) => Some(error),
            Self::Ordering(error) => Some(error),
            Self::ClientSender | Self::NonFreshReassembler => None,
        }
    }
}

enum ReassemblyHandlerError<HandlerError> {
    Reassembly(RetryHandshakeRecordReassemblyError),
    Handler(HandlerError),
}

/// One caller-owned handshake plaintext request.
///
/// This value intentionally derives neither `Copy` nor `Clone` so the sealed
/// proof can retain the exact request rather than detached replacement bytes.
pub struct RetryHandshakePlaintext<'content> {
    content: &'content [u8],
    padding_length: usize,
}

impl<'content> RetryHandshakePlaintext<'content> {
    /// Retain exact handshake content and the requested zero-padding length.
    #[must_use]
    pub const fn new(content: &'content [u8], padding_length: usize) -> Self {
        Self {
            content,
            padding_length,
        }
    }

    /// Borrow the exact handshake content.
    #[must_use]
    pub const fn content(&self) -> &'content [u8] {
        self.content
    }

    /// Return the requested zero-padding length.
    #[must_use]
    pub const fn padding_length(&self) -> usize {
        self.padding_length
    }
}

impl fmt::Debug for RetryHandshakePlaintext<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RetryHandshakePlaintext")
            .field("content", &"[REDACTED]")
            .field("padding_length", &self.padding_length)
            .finish()
    }
}

/// One sender's key material attached to exact retry traffic-secret evidence.
///
/// This value intentionally derives neither `Copy` nor `Clone`.
pub struct BoundRetryHandshakeTrafficKeyMaterial<
    'bound,
    'evidence,
    'secret_storage,
    'material,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    traffic_secrets:
        &'bound BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence>,
    sender: HandshakeTrafficSender,
    key_material: TrafficKeyMaterial<'material>,
}

impl<'bound, 'evidence, 'secret_storage, 'material, Evidence>
    BoundRetryHandshakeTrafficKeyMaterial<'bound, 'evidence, 'secret_storage, 'material, Evidence>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Borrow the complete exact-message retry traffic-secret proof.
    #[must_use]
    pub const fn traffic_secrets(
        &self,
    ) -> &'bound BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence> {
        self.traffic_secrets
    }

    /// Return the sender whose retained secret supplied this key material.
    #[must_use]
    pub const fn sender(&self) -> HandshakeTrafficSender {
        self.sender
    }

    /// Return the exact policy-selected suite retained by every component.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.key_material.selected_suite()
    }

    /// Borrow the sender's traffic key and static IV in caller-owned storage.
    #[must_use]
    pub const fn key_material(&self) -> &TrafficKeyMaterial<'material> {
        &self.key_material
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for BoundRetryHandshakeTrafficKeyMaterial<'_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryHandshakeTrafficKeyMaterial")
            .field("traffic_secrets", &"[RETAINED]")
            .field("sender", &self.sender)
            .field("key_material", &"[SECRET]")
            .finish()
    }
}

const CLIENT_STATE_ISSUED: u8 = 1;
const SERVER_STATE_ISSUED: u8 = 2;

impl HandshakeTrafficSender {
    const fn issuance_bit(self) -> u8 {
        match self {
            Self::Client => CLIENT_STATE_ISSUED,
            Self::Server => SERVER_STATE_ISSUED,
        }
    }
}

/// Single-use per-sender issuer for exact retry handshake record state.
///
/// Construction consumes the complete non-cloneable traffic-secret proof.
/// Each sender can obtain state exactly once from this issuer. The interior
/// issuance bits make this type non-`Sync`, so duplicate checks and successful
/// issuance cannot race across threads.
pub struct RetryHandshakeRecordProtectionStateIssuer<
    'evidence,
    'secret_storage,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    traffic_secrets: BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence>,
    issued: Cell<u8>,
}

impl<'evidence, 'secret_storage, Evidence>
    RetryHandshakeRecordProtectionStateIssuer<'evidence, 'secret_storage, Evidence>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Consume exact retry traffic-secret evidence into a fresh issuer.
    #[must_use]
    pub const fn new(
        traffic_secrets: BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence>,
    ) -> Self {
        Self {
            traffic_secrets,
            issued: Cell::new(0),
        }
    }

    /// Borrow the complete exact-message retry traffic-secret proof.
    #[must_use]
    pub const fn traffic_secrets(
        &self,
    ) -> &BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence> {
        &self.traffic_secrets
    }

    /// Report whether this issuer already created state for `sender`.
    #[must_use]
    pub fn has_issued(&self, sender: HandshakeTrafficSender) -> bool {
        self.issued.get() & sender.issuance_bit() != 0
    }

    /// Issue fresh sequence-zero state for one not-yet-issued sender.
    ///
    /// A duplicate sender fails before `storage` is touched. Failed key
    /// derivation or state construction leaves the sender unissued so a caller
    /// can retry with valid storage.
    pub fn issue<'issuer, 'material>(
        &'issuer self,
        sender: HandshakeTrafficSender,
        storage: &'material mut TrafficKeyMaterialStorage,
    ) -> Result<
        BoundRetryHandshakeRecordProtectionState<
            'issuer,
            'evidence,
            'secret_storage,
            'material,
            Evidence,
        >,
        RetryHandshakeRecordProtectionStateBindingError,
    > {
        let issuance_bit = sender.issuance_bit();
        if self.issued.get() & issuance_bit != 0 {
            return Err(RetryHandshakeRecordProtectionStateBindingError::SenderAlreadyIssued);
        }

        let bound =
            bind_retry_handshake_traffic_key_material(&self.traffic_secrets, sender, storage)
                .map_err(RetryHandshakeRecordProtectionStateBindingError::KeyMaterial)?;
        let BoundRetryHandshakeTrafficKeyMaterial { key_material, .. } = bound;
        let state = RecordProtectionState::new(key_material)
            .map_err(RetryHandshakeRecordProtectionStateBindingError::RecordState)?;

        self.issued.set(self.issued.get() | issuance_bit);
        Ok(BoundRetryHandshakeRecordProtectionState {
            issuer: self,
            sender,
            state,
        })
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for RetryHandshakeRecordProtectionStateIssuer<'_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RetryHandshakeRecordProtectionStateIssuer")
            .field("traffic_secrets", &"[RETAINED]")
            .field(
                "client_issued",
                &self.has_issued(HandshakeTrafficSender::Client),
            )
            .field(
                "server_issued",
                &self.has_issued(HandshakeTrafficSender::Server),
            )
            .finish()
    }
}

/// One sender's record state attached to a single-use retry issuer.
///
/// This value intentionally derives neither `Copy` nor `Clone`. The retained
/// record state owns the traffic-key view, while the secret bytes remain in
/// the caller-owned wipe-on-drop storage borrowed by that view.
pub struct BoundRetryHandshakeRecordProtectionState<
    'issuer,
    'evidence,
    'secret_storage,
    'material,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    issuer:
        &'issuer RetryHandshakeRecordProtectionStateIssuer<'evidence, 'secret_storage, Evidence>,
    sender: HandshakeTrafficSender,
    state: RecordProtectionState<'material>,
}

impl<'issuer, 'evidence, 'secret_storage, 'material, Evidence>
    BoundRetryHandshakeRecordProtectionState<
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        Evidence,
    >
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Borrow the complete exact-message retry traffic-secret proof.
    #[must_use]
    pub const fn traffic_secrets(
        &self,
    ) -> &'issuer BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence> {
        &self.issuer.traffic_secrets
    }

    /// Return the sender whose retained secret supplied this record state.
    #[must_use]
    pub const fn sender(&self) -> HandshakeTrafficSender {
        self.sender
    }

    /// Return the exact policy-selected suite retained by every component.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.state.selected_suite()
    }

    /// Borrow the exact retry-bound record-protection state.
    #[must_use]
    pub const fn record_state(&self) -> &RecordProtectionState<'material> {
        &self.state
    }

    /// Mutably borrow the exact retry-bound record-protection state.
    pub const fn record_state_mut(&mut self) -> &mut RecordProtectionState<'material> {
        &mut self.state
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for BoundRetryHandshakeRecordProtectionState<'_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryHandshakeRecordProtectionState")
            .field("issuer", &"[RETAINED]")
            .field("sender", &self.sender)
            .field("record_state", &"[STATE]")
            .finish()
    }
}

/// One sealed handshake record attached to its exact retry state and request.
///
/// The retained shared borrows prevent both state reuse and output mutation
/// while this proof is live. This value intentionally derives neither `Copy`
/// nor `Clone`.
pub struct SealedRetryHandshakeRecord<
    'state,
    'issuer,
    'evidence,
    'secret_storage,
    'material,
    'content,
    'output,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    state: &'state BoundRetryHandshakeRecordProtectionState<
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        Evidence,
    >,
    plaintext: RetryHandshakePlaintext<'content>,
    output: &'output [u8],
}

impl<'state, 'issuer, 'evidence, 'secret_storage, 'material, 'content, 'output, Evidence>
    SealedRetryHandshakeRecord<
        'state,
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'content,
        'output,
        Evidence,
    >
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Borrow the complete exact-message retry traffic-secret proof.
    #[must_use]
    pub const fn traffic_secrets(
        &self,
    ) -> &'issuer BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence> {
        self.state.traffic_secrets()
    }

    /// Return the sender whose monotonic record state sealed this record.
    #[must_use]
    pub const fn sender(&self) -> HandshakeTrafficSender {
        self.state.sender()
    }

    /// Return the policy-selected suite retained by the exact record state.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.state.selected_suite()
    }

    /// Borrow the exact handshake content supplied for sealing.
    #[must_use]
    pub const fn content(&self) -> &'content [u8] {
        self.plaintext.content()
    }

    /// Return the requested zero-padding length.
    #[must_use]
    pub const fn padding_length(&self) -> usize {
        self.plaintext.padding_length()
    }

    /// Borrow the exact sealed record prefix written by the dispatcher.
    #[must_use]
    pub const fn output(&self) -> &'output [u8] {
        self.output
    }

    /// Return the sender-state record count after sealing.
    #[must_use]
    pub const fn records_used(&self) -> u64 {
        self.state.record_state().records_used()
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for SealedRetryHandshakeRecord<'_, '_, '_, '_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SealedRetryHandshakeRecord")
            .field("state", &"[RETAINED]")
            .field("plaintext", &"[REDACTED]")
            .field("output", &"[REDACTED]")
            .finish()
    }
}

/// Seal one exact retry-bound TLS 1.3 handshake record.
///
/// The inner content type is fixed to `handshake`; callers provide no detached
/// content type. Sequence and output behavior follows the repository-owned
/// record dispatcher.
pub fn seal_retry_handshake_record<
    'state,
    'issuer,
    'evidence,
    'secret_storage,
    'material,
    'content,
    'output,
    Evidence,
>(
    state: &'state mut BoundRetryHandshakeRecordProtectionState<
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        Evidence,
    >,
    ciphertext_policy: &Tls13CiphertextPolicy,
    inner_policy: &Tls13InnerPlaintextPolicy,
    plaintext: RetryHandshakePlaintext<'content>,
    cleartext_scratch: &mut [u8],
    output: &'output mut [u8],
) -> Result<
    SealedRetryHandshakeRecord<
        'state,
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'content,
        'output,
        Evidence,
    >,
    RetryHandshakeRecordBindingError,
>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    let used = seal_tls13_record(
        state.record_state_mut(),
        ciphertext_policy,
        inner_policy,
        Tls13Plaintext::new(
            TlsContentType::Handshake,
            plaintext.content(),
            plaintext.padding_length(),
        ),
        cleartext_scratch,
        output,
    )
    .map_err(RetryHandshakeRecordBindingError::RecordProtection)?;

    Ok(SealedRetryHandshakeRecord {
        state: &*state,
        plaintext,
        output: &output[..used],
    })
}

/// One authenticated handshake record attached to exact input and retry state.
///
/// The retained dispatcher owner wipes the authenticated plaintext prefix on
/// drop. Shared borrows prevent state reuse, input mutation, and opened-output
/// mutation while this proof is live. This value intentionally derives neither
/// `Copy` nor `Clone`.
pub struct OpenedRetryHandshakeRecord<
    'state,
    'issuer,
    'evidence,
    'secret_storage,
    'material,
    'input,
    'output,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    state: &'state BoundRetryHandshakeRecordProtectionState<
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        Evidence,
    >,
    input: &'input [u8],
    opened: OpenedTls13Record<'output>,
}

impl<'state, 'issuer, 'evidence, 'secret_storage, 'material, 'input, 'output, Evidence>
    OpenedRetryHandshakeRecord<
        'state,
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'input,
        'output,
        Evidence,
    >
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Borrow the complete exact-message retry traffic-secret proof.
    #[must_use]
    pub const fn traffic_secrets(
        &self,
    ) -> &'issuer BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence> {
        self.state.traffic_secrets()
    }

    /// Return the sender whose monotonic record state opened this record.
    #[must_use]
    pub const fn sender(&self) -> HandshakeTrafficSender {
        self.state.sender()
    }

    /// Return the policy-selected suite retained by the exact record state.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.state.selected_suite()
    }

    /// Borrow the exact ciphertext record supplied for opening.
    #[must_use]
    pub const fn input(&self) -> &'input [u8] {
        self.input
    }

    /// Borrow authenticated handshake content.
    #[must_use]
    pub fn content(&self) -> &[u8] {
        self.opened.content()
    }

    /// Return the authenticated zero-padding length.
    #[must_use]
    pub const fn padding_length(&self) -> usize {
        self.opened.padding_length()
    }

    /// Return the sender-state record count after opening.
    #[must_use]
    pub const fn records_used(&self) -> u64 {
        self.state.record_state().records_used()
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for OpenedRetryHandshakeRecord<'_, '_, '_, '_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenedRetryHandshakeRecord")
            .field("state", &"[RETAINED]")
            .field("input", &"[REDACTED]")
            .field("opened", &"[REDACTED]")
            .finish()
    }
}

/// Authenticate and open one exact retry-bound TLS 1.3 handshake record.
///
/// Authentication completes before the inner content type is inspected. An
/// authenticated non-handshake record is rejected and its opened owner wipes
/// the plaintext prefix before this function returns.
pub fn open_retry_handshake_record<
    'state,
    'issuer,
    'evidence,
    'secret_storage,
    'material,
    'input,
    'output,
    Evidence,
>(
    state: &'state mut BoundRetryHandshakeRecordProtectionState<
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        Evidence,
    >,
    ciphertext_policy: &Tls13CiphertextPolicy,
    inner_policy: &Tls13InnerPlaintextPolicy,
    input: &'input [u8],
    output: &'output mut [u8],
) -> Result<
    OpenedRetryHandshakeRecord<
        'state,
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'input,
        'output,
        Evidence,
    >,
    RetryHandshakeRecordBindingError,
>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    let opened = open_tls13_record(
        state.record_state_mut(),
        ciphertext_policy,
        inner_policy,
        input,
        output,
    )
    .map_err(RetryHandshakeRecordBindingError::RecordProtection)?;
    if opened.content_type() != TlsContentType::Handshake {
        return Err(RetryHandshakeRecordBindingError::UnexpectedContentType);
    }

    Ok(OpenedRetryHandshakeRecord {
        state: &*state,
        input,
        opened,
    })
}

/// Exact retry record state composed with bounded authenticated reassembly.
///
/// This owner consumes the non-cloneable sender state so every fragment copied
/// into its reassembler first passed authentication under one monotonic record
/// sequence. Caller-owned reassembly storage is cleared by the composed
/// repository-owned primitive. This value intentionally derives neither
/// `Copy` nor `Clone`.
pub struct RetryHandshakeRecordReassembler<
    'issuer,
    'evidence,
    'secret_storage,
    'material,
    'reassembly,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    state: BoundRetryHandshakeRecordProtectionState<
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        Evidence,
    >,
    reassembler: Tls13HandshakeReassembler<'reassembly>,
    message_in_progress: bool,
    failed: bool,
}

impl<'issuer, 'evidence, 'secret_storage, 'material, 'reassembly, Evidence>
    RetryHandshakeRecordReassembler<
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        Evidence,
    >
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Consume exact sender state into a bounded authenticated reassembler.
    pub fn new(
        state: BoundRetryHandshakeRecordProtectionState<
            'issuer,
            'evidence,
            'secret_storage,
            'material,
            Evidence,
        >,
        handshake_policy: Tls13HandshakePolicy,
        reassembly_storage: &'reassembly mut [u8],
    ) -> Result<Self, RetryHandshakeRecordReassemblyError> {
        let reassembler = Tls13HandshakeReassembler::new(handshake_policy, reassembly_storage)
            .map_err(RetryHandshakeRecordReassemblyError::Reassembly)?;
        Ok(Self {
            state,
            reassembler,
            message_in_progress: false,
            failed: false,
        })
    }

    /// Borrow the complete exact-message retry traffic-secret proof.
    #[must_use]
    pub const fn traffic_secrets(
        &self,
    ) -> &'issuer BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence> {
        self.state.traffic_secrets()
    }

    /// Return the sender authenticated by this owner's monotonic state.
    #[must_use]
    pub const fn sender(&self) -> HandshakeTrafficSender {
        self.state.sender()
    }

    /// Return the policy-selected suite retained by the exact record state.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.state.selected_suite()
    }

    /// Return whether a partial handshake message awaits another record.
    #[must_use]
    pub const fn message_in_progress(&self) -> bool {
        self.message_in_progress
    }

    /// Return whether a failure permanently closed this owner.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    /// Return the record count already consumed by this owner.
    #[must_use]
    pub const fn records_used(&self) -> u64 {
        self.state.record_state().records_used()
    }

    /// Authenticate one record and drain all of its handshake content.
    ///
    /// `on_message` is invoked synchronously once for each complete
    /// policy-authorized message. Its borrowed message cannot escape the call.
    /// A final fragment remains in bounded caller-owned reassembly storage for
    /// the next authenticated record.
    pub fn open_and_reassemble<'owner, 'input, F>(
        &'owner mut self,
        ciphertext_policy: &Tls13CiphertextPolicy,
        inner_policy: &Tls13InnerPlaintextPolicy,
        input: &'input [u8],
        opened_output: &mut [u8],
        mut on_message: F,
    ) -> Result<
        ReassembledRetryHandshakeRecord<
            'owner,
            'issuer,
            'evidence,
            'secret_storage,
            'material,
            'reassembly,
            'input,
            Evidence,
        >,
        RetryHandshakeRecordReassemblyError,
    >
    where
        F: for<'message> FnMut(Tls13HandshakeMessage<'message>),
    {
        match self.open_and_reassemble_with(
            ciphertext_policy,
            inner_policy,
            input,
            opened_output,
            |message| {
                on_message(message);
                Ok::<(), core::convert::Infallible>(())
            },
        ) {
            Ok(result) => Ok(result),
            Err(ReassemblyHandlerError::Reassembly(error)) => Err(error),
            Err(ReassemblyHandlerError::Handler(uninhabited)) => match uninhabited {},
        }
    }

    fn open_and_reassemble_with<'owner, 'input, F, HandlerError>(
        &'owner mut self,
        ciphertext_policy: &Tls13CiphertextPolicy,
        inner_policy: &Tls13InnerPlaintextPolicy,
        input: &'input [u8],
        opened_output: &mut [u8],
        mut on_message: F,
    ) -> Result<
        ReassembledRetryHandshakeRecord<
            'owner,
            'issuer,
            'evidence,
            'secret_storage,
            'material,
            'reassembly,
            'input,
            Evidence,
        >,
        ReassemblyHandlerError<HandlerError>,
    >
    where
        F: for<'message> FnMut(Tls13HandshakeMessage<'message>) -> Result<(), HandlerError>,
    {
        if self.failed {
            return Err(ReassemblyHandlerError::Reassembly(
                RetryHandshakeRecordReassemblyError::OwnerFailed,
            ));
        }

        let opened = match open_tls13_record(
            self.state.record_state_mut(),
            ciphertext_policy,
            inner_policy,
            input,
            opened_output,
        ) {
            Ok(opened) => opened,
            Err(error) => {
                self.fail();
                return Err(ReassemblyHandlerError::Reassembly(
                    RetryHandshakeRecordReassemblyError::Record(
                        RetryHandshakeRecordBindingError::RecordProtection(error),
                    ),
                ));
            }
        };
        if opened.content_type() != TlsContentType::Handshake {
            self.fail();
            return Err(ReassemblyHandlerError::Reassembly(
                RetryHandshakeRecordReassemblyError::Record(
                    RetryHandshakeRecordBindingError::UnexpectedContentType,
                ),
            ));
        }

        let authenticated_content_length = opened.content().len();
        let mut offset = 0;
        let mut completed_messages = 0;
        while offset < authenticated_content_length {
            let progress = match self
                .reassembler
                .push_authenticated_record_content(&opened.content()[offset..])
            {
                Ok(progress) => progress,
                Err(error) => {
                    self.fail();
                    return Err(ReassemblyHandlerError::Reassembly(
                        RetryHandshakeRecordReassemblyError::Reassembly(error),
                    ));
                }
            };
            let consumed = progress.consumed();
            debug_assert!(consumed > 0);
            offset += consumed;
            match progress {
                ReassemblyProgress::Incomplete { .. } => {
                    self.message_in_progress = true;
                }
                ReassemblyProgress::Complete { message, .. } => {
                    self.message_in_progress = false;
                    completed_messages += 1;
                    let handler_result =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            on_message(message)
                        }));
                    match handler_result {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            self.fail();
                            return Err(ReassemblyHandlerError::Handler(error));
                        }
                        Err(payload) => {
                            self.fail();
                            std::panic::resume_unwind(payload);
                        }
                    }
                }
            }
        }
        drop(opened);

        Ok(ReassembledRetryHandshakeRecord {
            owner: &*self,
            input,
            authenticated_content_length,
            completed_messages,
            message_in_progress: self.message_in_progress,
        })
    }

    fn fail(&mut self) {
        self.reassembler.reset();
        self.message_in_progress = false;
        self.failed = true;
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for RetryHandshakeRecordReassembler<'_, '_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RetryHandshakeRecordReassembler")
            .field("state", &"[RETAINED]")
            .field("reassembler", &"[REDACTED]")
            .field("message_in_progress", &self.message_in_progress)
            .field("failed", &self.failed)
            .finish()
    }
}

/// Result of draining one authenticated retry-handshake record.
///
/// The retained shared owner borrow prevents another record from advancing the
/// sequence before this exact input result is consumed. This value
/// intentionally derives neither `Copy` nor `Clone`.
pub struct ReassembledRetryHandshakeRecord<
    'owner,
    'issuer,
    'evidence,
    'secret_storage,
    'material,
    'reassembly,
    'input,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    owner: &'owner RetryHandshakeRecordReassembler<
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        Evidence,
    >,
    input: &'input [u8],
    authenticated_content_length: usize,
    completed_messages: usize,
    message_in_progress: bool,
}

impl<'owner, 'issuer, 'evidence, 'secret_storage, 'material, 'reassembly, 'input, Evidence>
    ReassembledRetryHandshakeRecord<
        'owner,
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        'input,
        Evidence,
    >
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Borrow the complete exact-message retry traffic-secret proof.
    #[must_use]
    pub const fn traffic_secrets(
        &self,
    ) -> &'issuer BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence> {
        self.owner.traffic_secrets()
    }

    /// Return the sender whose state authenticated this exact input.
    #[must_use]
    pub const fn sender(&self) -> HandshakeTrafficSender {
        self.owner.sender()
    }

    /// Return the exact selected suite retained by the owner.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.owner.selected_suite()
    }

    /// Borrow the exact ciphertext record supplied to this operation.
    #[must_use]
    pub const fn input(&self) -> &'input [u8] {
        self.input
    }

    /// Return the authenticated inner handshake-content length.
    #[must_use]
    pub const fn authenticated_content_length(&self) -> usize {
        self.authenticated_content_length
    }

    /// Return how many complete messages the handler received.
    #[must_use]
    pub const fn completed_messages(&self) -> usize {
        self.completed_messages
    }

    /// Return whether the owner retained a final incomplete message.
    #[must_use]
    pub const fn message_in_progress(&self) -> bool {
        self.message_in_progress
    }

    /// Return the sender-state record count after opening this input.
    #[must_use]
    pub const fn records_used(&self) -> u64 {
        self.owner.records_used()
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for ReassembledRetryHandshakeRecord<'_, '_, '_, '_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReassembledRetryHandshakeRecord")
            .field("owner", &"[RETAINED]")
            .field("input", &"[REDACTED]")
            .field(
                "authenticated_content_length",
                &self.authenticated_content_length,
            )
            .field("completed_messages", &self.completed_messages)
            .field("message_in_progress", &self.message_in_progress)
            .finish()
    }
}

/// One authenticated message accepted by the no-PSK server-flight policy.
///
/// The transition happens before this wrapper reaches the caller. The borrowed
/// message cannot escape the higher-ranked callback that receives it. This
/// value intentionally derives neither `Copy` nor `Clone`.
pub struct OrderedRetryServerFlightMessage<'message> {
    message: Tls13HandshakeMessage<'message>,
    state: ServerFlightState,
    client_authentication_requested: bool,
}

impl<'message> OrderedRetryServerFlightMessage<'message> {
    /// Borrow the exact authenticated, policy-authorized handshake message.
    #[must_use]
    pub const fn message(&self) -> &Tls13HandshakeMessage<'message> {
        &self.message
    }

    /// Return the server-flight state after accepting this message.
    #[must_use]
    pub const fn state(&self) -> ServerFlightState {
        self.state
    }

    /// Return whether the accepted prefix requested client authentication.
    #[must_use]
    pub const fn client_authentication_requested(&self) -> bool {
        self.client_authentication_requested
    }
}

impl fmt::Debug for OrderedRetryServerFlightMessage<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OrderedRetryServerFlightMessage")
            .field("message", &"[REDACTED]")
            .field("state", &self.state)
            .field(
                "client_authentication_requested",
                &self.client_authentication_requested,
            )
            .finish()
    }
}

/// Authenticated retry reassembly composed with no-PSK server-flight order.
///
/// Construction consumes a server-sender owner and creates the ordering state
/// internally at `ExpectEncryptedExtensions`; detached state cannot be
/// supplied or resumed. This value intentionally derives neither `Copy` nor
/// `Clone`.
pub struct RetryOrderedServerFlight<
    'issuer,
    'evidence,
    'secret_storage,
    'material,
    'reassembly,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    reassembler: RetryHandshakeRecordReassembler<
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        Evidence,
    >,
    flight: NoPskServerFlight,
}

impl<'issuer, 'evidence, 'secret_storage, 'material, 'reassembly, Evidence>
    RetryOrderedServerFlight<'issuer, 'evidence, 'secret_storage, 'material, 'reassembly, Evidence>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Consume one fresh authenticated server-sender reassembly owner.
    pub fn new(
        reassembler: RetryHandshakeRecordReassembler<
            'issuer,
            'evidence,
            'secret_storage,
            'material,
            'reassembly,
            Evidence,
        >,
    ) -> Result<Self, RetryOrderedServerFlightError> {
        if reassembler.sender() != HandshakeTrafficSender::Server {
            return Err(RetryOrderedServerFlightError::ClientSender);
        }
        if reassembler.records_used() != 0
            || reassembler.message_in_progress()
            || reassembler.is_failed()
        {
            return Err(RetryOrderedServerFlightError::NonFreshReassembler);
        }
        Ok(Self {
            reassembler,
            flight: NoPskServerFlight::new(),
        })
    }

    /// Borrow the complete exact-message retry traffic-secret proof.
    #[must_use]
    pub const fn traffic_secrets(
        &self,
    ) -> &'issuer BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence> {
        self.reassembler.traffic_secrets()
    }

    /// Return the policy-selected suite retained by the exact record state.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.reassembler.selected_suite()
    }

    /// Return the next expected server-flight position.
    #[must_use]
    pub const fn state(&self) -> ServerFlightState {
        self.flight.state()
    }

    /// Return whether one accepted request requires client authentication.
    #[must_use]
    pub const fn client_authentication_requested(&self) -> bool {
        self.flight.client_authentication_requested()
    }

    /// Return whether the complete public server-flight sequence was accepted.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.flight.is_complete()
    }

    /// Return whether a partial handshake message awaits another record.
    #[must_use]
    pub const fn message_in_progress(&self) -> bool {
        self.reassembler.message_in_progress()
    }

    /// Return whether a terminal failure permanently closed this owner.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.reassembler.is_failed()
    }

    /// Return the record count already consumed by this owner.
    #[must_use]
    pub const fn records_used(&self) -> u64 {
        self.reassembler.records_used()
    }

    /// Authenticate one record, enforce message order, and lend accepted messages.
    ///
    /// A message reaches `on_message` only after the internal no-PSK policy
    /// accepts its type. Ordering failure clears and permanently closes the
    /// composed reassembler. Earlier valid messages from the same record may
    /// already have reached the callback; caller effects are not transactional.
    pub fn open_and_accept<'owner, 'input, F>(
        &'owner mut self,
        ciphertext_policy: &Tls13CiphertextPolicy,
        inner_policy: &Tls13InnerPlaintextPolicy,
        input: &'input [u8],
        opened_output: &mut [u8],
        mut on_message: F,
    ) -> Result<
        OrderedRetryServerFlightRecord<
            'owner,
            'issuer,
            'evidence,
            'secret_storage,
            'material,
            'reassembly,
            'input,
            Evidence,
        >,
        RetryOrderedServerFlightError,
    >
    where
        F: for<'message> FnMut(OrderedRetryServerFlightMessage<'message>),
    {
        let reassembler = &mut self.reassembler;
        let flight = &mut self.flight;
        let reassembled = match reassembler.open_and_reassemble_with(
            ciphertext_policy,
            inner_policy,
            input,
            opened_output,
            |message| {
                let state = flight
                    .accept(message.handshake_type())
                    .map_err(RetryOrderedServerFlightError::Ordering)?;
                on_message(OrderedRetryServerFlightMessage {
                    message,
                    state,
                    client_authentication_requested: flight.client_authentication_requested(),
                });
                Ok(())
            },
        ) {
            Ok(result) => result,
            Err(ReassemblyHandlerError::Reassembly(error)) => {
                return Err(RetryOrderedServerFlightError::Reassembly(error));
            }
            Err(ReassemblyHandlerError::Handler(error)) => return Err(error),
        };

        Ok(OrderedRetryServerFlightRecord {
            reassembled,
            state: flight.state(),
            client_authentication_requested: flight.client_authentication_requested(),
        })
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for RetryOrderedServerFlight<'_, '_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RetryOrderedServerFlight")
            .field("reassembler", &"[RETAINED]")
            .field("state", &self.flight.state())
            .field(
                "client_authentication_requested",
                &self.flight.client_authentication_requested(),
            )
            .field("failed", &self.reassembler.is_failed())
            .finish()
    }
}

/// Opaque identity of one single-use server record-state issuer.
///
/// This receipt is copied across synchronous callback lifetimes so downstream
/// repository-owned compositions can reject messages from a different ordered
/// record owner. Its private value is never exposed or formatted. The retained
/// lifetime keeps the issuer borrowed for as long as any identity copy exists,
/// preventing allocator or stack-slot address reuse from substituting a later
/// issuer.
#[derive(Clone, Copy)]
pub struct OrderedRetryServerFlightIdentity<'issuer> {
    address: usize,
    issuer_borrow: core::marker::PhantomData<&'issuer ()>,
}

impl<'left, 'right> PartialEq<OrderedRetryServerFlightIdentity<'right>>
    for OrderedRetryServerFlightIdentity<'left>
{
    fn eq(&self, other: &OrderedRetryServerFlightIdentity<'right>) -> bool {
        self.address == other.address
    }
}

impl Eq for OrderedRetryServerFlightIdentity<'_> {}

impl<'issuer> OrderedRetryServerFlightIdentity<'issuer> {
    fn from_issuer<Issuer: ?Sized>(issuer: &'issuer Issuer) -> Self {
        Self {
            address: core::ptr::from_ref(issuer).addr(),
            issuer_borrow: core::marker::PhantomData,
        }
    }
}

impl fmt::Debug for OrderedRetryServerFlightIdentity<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OrderedRetryServerFlightIdentity([REDACTED])")
    }
}

/// One authenticated ordered message awaiting caller authorization to be hashed.
///
/// The snapshot is the exact transcript state immediately before this message.
/// The borrowed message cannot escape the higher-ranked callback that receives
/// this value. This value intentionally derives neither `Copy` nor `Clone`.
pub struct PendingRetryServerTranscriptMessage<'message, 'retry_evidence, 'issuer> {
    message: Tls13HandshakeMessage<'message>,
    retry_evidence: &'retry_evidence dyn ExactRetryBaseSecrets,
    ordered_server_flight_identity: OrderedRetryServerFlightIdentity<'issuer>,
    transcript_snapshot: TranscriptHash,
    state: ServerFlightState,
    client_authentication_requested: bool,
}

impl<'message, 'retry_evidence, 'issuer>
    PendingRetryServerTranscriptMessage<'message, 'retry_evidence, 'issuer>
{
    /// Borrow the exact authenticated, policy-ordered handshake message.
    #[must_use]
    pub const fn message(&self) -> &Tls13HandshakeMessage<'message> {
        &self.message
    }

    /// Return the opaque identity of the single-use server record-state issuer.
    #[must_use]
    pub const fn ordered_server_flight_identity(
        &self,
    ) -> OrderedRetryServerFlightIdentity<'issuer> {
        self.ordered_server_flight_identity
    }

    /// Return the transcript snapshot immediately before this message.
    #[must_use]
    pub const fn transcript_snapshot(&self) -> TranscriptHash {
        self.transcript_snapshot
    }

    /// Return the server-flight state after accepting this message's type.
    #[must_use]
    pub const fn state(&self) -> ServerFlightState {
        self.state
    }

    /// Return whether this ordered prefix requests client authentication.
    #[must_use]
    pub const fn client_authentication_requested(&self) -> bool {
        self.client_authentication_requested
    }

    /// Decode and policy-bind this exact pending `EncryptedExtensions` message.
    pub fn validate_encrypted_extensions(
        &self,
        limits: ExtensionBlockLimits,
    ) -> Result<
        BoundRetryEncryptedExtensions<'_, 'message, 'retry_evidence, 'issuer>,
        RetryEncryptedExtensionsValidationError,
    > {
        if self.message.handshake_type() != tls13_handshake::Tls13HandshakeType::EncryptedExtensions
        {
            return Err(RetryEncryptedExtensionsValidationError::WrongMessageType);
        }
        if self.state != ServerFlightState::ExpectCertificateRequestOrCertificate {
            return Err(RetryEncryptedExtensionsValidationError::WrongFlightState);
        }
        let encrypted_extensions = decode_encrypted_extensions_message(self.message.body(), limits)
            .map_err(RetryEncryptedExtensionsValidationError::Structure)?;
        self.retry_evidence
            .extension_context_policy()
            .validate_server_message(
                ServerExtensionContext::EncryptedExtensions,
                encrypted_extensions.extensions(),
                self.retry_evidence.final_client_hello_extensions(),
            )
            .map_err(RetryEncryptedExtensionsValidationError::Context)?;
        Ok(BoundRetryEncryptedExtensions {
            pending: self,
            encrypted_extensions,
        })
    }

    /// Decode and policy-bind this exact pending main-handshake
    /// `CertificateRequest` message.
    pub fn validate_certificate_request(
        &self,
        limits: CertificateRequestLimits,
        signature_limits: SignatureSchemeListLimits,
    ) -> Result<
        BoundRetryCertificateRequest<'_, 'message, 'retry_evidence, 'issuer>,
        RetryCertificateRequestValidationError,
    > {
        if self.message.handshake_type() != tls13_handshake::Tls13HandshakeType::CertificateRequest
        {
            return Err(RetryCertificateRequestValidationError::WrongMessageType);
        }
        if self.state != ServerFlightState::ExpectCertificate {
            return Err(RetryCertificateRequestValidationError::WrongFlightState);
        }
        if !self.client_authentication_requested {
            return Err(RetryCertificateRequestValidationError::ClientAuthenticationNotRequested);
        }
        let certificate_request = decode_certificate_request(self.message.body(), limits)
            .map_err(RetryCertificateRequestValidationError::Structure)?;
        if !certificate_request.context().is_empty() {
            return Err(RetryCertificateRequestValidationError::NonEmptyContext);
        }
        self.retry_evidence
            .extension_context_policy()
            .validate_server_message(
                ServerExtensionContext::CertificateRequest,
                certificate_request.extensions(),
                self.retry_evidence.final_client_hello_extensions(),
            )
            .map_err(RetryCertificateRequestValidationError::Context)?;
        let signature_algorithms = certificate_request
            .extensions()
            .find(ExtensionType::from_wire_value(
                SIGNATURE_ALGORITHMS_EXTENSION_TYPE,
            ))
            .ok_or(RetryCertificateRequestValidationError::MissingSignatureAlgorithms)
            .and_then(|entry| {
                decode_signature_scheme_list(entry.data(), signature_limits)
                    .map_err(RetryCertificateRequestValidationError::SignatureAlgorithms)
            })?;
        let signature_algorithms_cert = certificate_request
            .extensions()
            .find(ExtensionType::from_wire_value(
                SIGNATURE_ALGORITHMS_CERT_EXTENSION_TYPE,
            ))
            .map(|entry| {
                decode_signature_scheme_list(entry.data(), signature_limits)
                    .map_err(RetryCertificateRequestValidationError::SignatureAlgorithmsCert)
            })
            .transpose()?;
        Ok(BoundRetryCertificateRequest {
            pending: self,
            certificate_request,
            signature_algorithms,
            signature_algorithms_cert,
        })
    }

    /// Decode and policy-bind this exact pending main-handshake server
    /// `Certificate` message.
    pub fn validate_server_certificate(
        &self,
        limits: Tls13CertificateMessageLimits,
        extension_limits: ExtensionBlockLimits,
    ) -> Result<
        BoundRetryServerCertificate<'_, 'message, 'retry_evidence, 'issuer>,
        RetryServerCertificateValidationError,
    > {
        if self.message.handshake_type() != tls13_handshake::Tls13HandshakeType::Certificate {
            return Err(RetryServerCertificateValidationError::WrongMessageType);
        }
        if self.state != ServerFlightState::ExpectCertificateVerify {
            return Err(RetryServerCertificateValidationError::WrongFlightState);
        }
        let certificate = decode_tls13_certificate_message(self.message.body(), limits)
            .map_err(|error| RetryServerCertificateValidationError::Structure(error.kind()))?;
        if !certificate.request_context().is_empty() {
            return Err(RetryServerCertificateValidationError::NonEmptyContext);
        }
        if certificate.certificate_count() == 0 {
            return Err(RetryServerCertificateValidationError::EmptyCertificateList);
        }
        let policy = self.retry_evidence.extension_context_policy();
        let final_client_hello = self.retry_evidence.final_client_hello_extensions();
        for entry in certificate.entries() {
            let extensions = decode_extension_block(entry.encoded_extensions(), extension_limits)
                .map_err(RetryServerCertificateValidationError::EntryExtensions)?;
            policy
                .validate_server_message(
                    ServerExtensionContext::Certificate,
                    &extensions,
                    final_client_hello,
                )
                .map_err(RetryServerCertificateValidationError::EntryContext)?;
        }
        Ok(BoundRetryServerCertificate {
            pending: self,
            certificate,
        })
    }

    /// Decode and policy-bind this exact pending server `CertificateVerify`.
    pub fn validate_server_certificate_verify(
        &self,
        signature_limits: SignatureSchemeListLimits,
        signature_policy: SignatureSchemePolicy<'_>,
    ) -> Result<
        BoundRetryServerCertificateVerify<'_, 'message, 'retry_evidence, 'issuer>,
        RetryServerCertificateVerifyValidationError,
    > {
        if self.message.handshake_type() != tls13_handshake::Tls13HandshakeType::CertificateVerify {
            return Err(RetryServerCertificateVerifyValidationError::WrongMessageType);
        }
        if self.state != ServerFlightState::ExpectFinished {
            return Err(RetryServerCertificateVerifyValidationError::WrongFlightState);
        }
        let message = CertificateVerifyMessage::decode(self.message.body())
            .map_err(RetryServerCertificateVerifyValidationError::Structure)?;
        let offered_signature_algorithms = self
            .retry_evidence
            .final_client_hello_extensions()
            .find(ExtensionType::from_wire_value(
                SIGNATURE_ALGORITHMS_EXTENSION_TYPE,
            ))
            .ok_or(RetryServerCertificateVerifyValidationError::MissingSignatureAlgorithms)
            .and_then(|entry| {
                decode_signature_scheme_list(entry.data(), signature_limits)
                    .map_err(RetryServerCertificateVerifyValidationError::SignatureAlgorithms)
            })?;
        if !offered_signature_algorithms.contains(message.algorithm_wire_id()) {
            return Err(RetryServerCertificateVerifyValidationError::UnofferedSignatureScheme);
        }
        let authorized_scheme = signature_policy
            .select(message.algorithm_wire_id())
            .map_err(RetryServerCertificateVerifyValidationError::Policy)?;
        Ok(BoundRetryServerCertificateVerify {
            pending: self,
            message,
            offered_signature_algorithms,
            authorized_scheme,
        })
    }
}

impl fmt::Debug for PendingRetryServerTranscriptMessage<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PendingRetryServerTranscriptMessage")
            .field("message", &"[REDACTED]")
            .field("transcript_snapshot", &"[RETAINED]")
            .field("state", &self.state)
            .field(
                "client_authentication_requested",
                &self.client_authentication_requested,
            )
            .finish()
    }
}

/// Stable payload-free failures for exact retry `EncryptedExtensions` validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryEncryptedExtensionsValidationError {
    /// The pending ordered message was not `EncryptedExtensions`.
    WrongMessageType,
    /// The ordered flight was not immediately after `EncryptedExtensions`.
    WrongFlightState,
    /// The exact authenticated body was structurally invalid.
    Structure(ExtensionBlockError),
    /// Placement or final ClientHello offer policy rejected one extension.
    Context(ExtensionContextPolicyError),
}

impl fmt::Display for RetryEncryptedExtensionsValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongMessageType => "TLS 1.3 retry message is not EncryptedExtensions",
            Self::WrongFlightState => {
                "TLS 1.3 retry EncryptedExtensions has the wrong ordered-flight state"
            }
            Self::Structure(_) => "TLS 1.3 retry EncryptedExtensions structure is invalid",
            Self::Context(_) => "TLS 1.3 retry EncryptedExtensions policy rejected an extension",
        })
    }
}

impl std::error::Error for RetryEncryptedExtensionsValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Structure(error) => Some(error),
            Self::Context(error) => Some(error),
            Self::WrongMessageType | Self::WrongFlightState => None,
        }
    }
}

/// Exact authenticated `EncryptedExtensions` bound to retry offer/context policy.
///
/// This proof intentionally derives neither `Copy` nor `Clone` and retains the
/// pending ordered message beside the exact structural parse.
pub struct BoundRetryEncryptedExtensions<'pending, 'message, 'retry_evidence, 'issuer> {
    pending: &'pending PendingRetryServerTranscriptMessage<'message, 'retry_evidence, 'issuer>,
    encrypted_extensions: EncryptedExtensionsMessage<'message>,
}

impl<'pending, 'message, 'retry_evidence, 'issuer>
    BoundRetryEncryptedExtensions<'pending, 'message, 'retry_evidence, 'issuer>
{
    /// Borrow the exact pending ordered message and pre-message snapshot.
    #[must_use]
    pub const fn pending(
        &self,
    ) -> &'pending PendingRetryServerTranscriptMessage<'message, 'retry_evidence, 'issuer> {
        self.pending
    }

    /// Borrow the exact structurally decoded and policy-authorized body.
    #[must_use]
    pub const fn encrypted_extensions(&self) -> &EncryptedExtensionsMessage<'message> {
        &self.encrypted_extensions
    }
}

impl fmt::Debug for BoundRetryEncryptedExtensions<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryEncryptedExtensions")
            .field("pending", &"[RETAINED]")
            .field("encrypted_extensions", &"[REDACTED]")
            .finish()
    }
}

/// Stable payload-free failures for exact retry `CertificateRequest` validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryCertificateRequestValidationError {
    /// The pending ordered message was not `CertificateRequest`.
    WrongMessageType,
    /// The ordered flight was not immediately after `CertificateRequest`.
    WrongFlightState,
    /// The ordered flight did not record a client-authentication request.
    ClientAuthenticationNotRequested,
    /// The exact authenticated body was structurally invalid.
    Structure(CertificateRequestError),
    /// A main-handshake `CertificateRequest` carried a nonempty context.
    NonEmptyContext,
    /// Placement policy rejected one request extension.
    Context(ExtensionContextPolicyError),
    /// The required `signature_algorithms` extension was absent.
    MissingSignatureAlgorithms,
    /// The required `signature_algorithms` list was structurally invalid.
    SignatureAlgorithms(SignatureSchemeListError),
    /// The optional `signature_algorithms_cert` list was structurally invalid.
    SignatureAlgorithmsCert(SignatureSchemeListError),
}

impl fmt::Display for RetryCertificateRequestValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongMessageType => "TLS 1.3 retry message is not CertificateRequest",
            Self::WrongFlightState => {
                "TLS 1.3 retry CertificateRequest has the wrong ordered-flight state"
            }
            Self::ClientAuthenticationNotRequested => {
                "TLS 1.3 retry flight did not request client authentication"
            }
            Self::Structure(_) => "TLS 1.3 retry CertificateRequest structure is invalid",
            Self::NonEmptyContext => {
                "TLS 1.3 main-handshake CertificateRequest context is not empty"
            }
            Self::Context(_) => "TLS 1.3 retry CertificateRequest policy rejected an extension",
            Self::MissingSignatureAlgorithms => {
                "TLS 1.3 CertificateRequest omitted signature_algorithms"
            }
            Self::SignatureAlgorithms(_) => {
                "TLS 1.3 CertificateRequest signature_algorithms is invalid"
            }
            Self::SignatureAlgorithmsCert(_) => {
                "TLS 1.3 CertificateRequest signature_algorithms_cert is invalid"
            }
        })
    }
}

impl std::error::Error for RetryCertificateRequestValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Structure(error) => Some(error),
            Self::Context(error) => Some(error),
            Self::SignatureAlgorithms(error) | Self::SignatureAlgorithmsCert(error) => Some(error),
            Self::WrongMessageType
            | Self::WrongFlightState
            | Self::ClientAuthenticationNotRequested
            | Self::NonEmptyContext
            | Self::MissingSignatureAlgorithms => None,
        }
    }
}

/// Exact authenticated main-handshake `CertificateRequest` bound to retry policy.
///
/// This proof intentionally derives neither `Copy` nor `Clone`. It retains the
/// pending ordered message, exact decoded request, required handshake-signature
/// list, and optional certificate-signature list.
pub struct BoundRetryCertificateRequest<'pending, 'message, 'retry_evidence, 'issuer> {
    pending: &'pending PendingRetryServerTranscriptMessage<'message, 'retry_evidence, 'issuer>,
    certificate_request: CertificateRequest<'message>,
    signature_algorithms: SignatureSchemeList<'message>,
    signature_algorithms_cert: Option<SignatureSchemeList<'message>>,
}

impl<'pending, 'message, 'retry_evidence, 'issuer>
    BoundRetryCertificateRequest<'pending, 'message, 'retry_evidence, 'issuer>
{
    /// Borrow the exact pending ordered message and pre-message snapshot.
    #[must_use]
    pub const fn pending(
        &self,
    ) -> &'pending PendingRetryServerTranscriptMessage<'message, 'retry_evidence, 'issuer> {
        self.pending
    }

    /// Borrow the exact structurally decoded and placement-authorized request.
    #[must_use]
    pub const fn certificate_request(&self) -> &CertificateRequest<'message> {
        &self.certificate_request
    }

    /// Borrow the required exact handshake-signature scheme list.
    #[must_use]
    pub const fn signature_algorithms(&self) -> &SignatureSchemeList<'message> {
        &self.signature_algorithms
    }

    /// Borrow the optional exact certificate-signature scheme list.
    #[must_use]
    pub const fn signature_algorithms_cert(&self) -> Option<&SignatureSchemeList<'message>> {
        self.signature_algorithms_cert.as_ref()
    }
}

impl fmt::Debug for BoundRetryCertificateRequest<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryCertificateRequest")
            .field("pending", &"[RETAINED]")
            .field("certificate_request", &"[REDACTED]")
            .field("signature_algorithms", &"[REDACTED]")
            .field("signature_algorithms_cert", &"[REDACTED]")
            .finish()
    }
}

/// Stable payload-free failures for exact retry server `Certificate` validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryServerCertificateValidationError {
    /// The pending ordered message was not `Certificate`.
    WrongMessageType,
    /// The ordered flight was not immediately after server `Certificate`.
    WrongFlightState,
    /// The exact authenticated body was structurally invalid.
    Structure(Tls13CertificateMessageErrorKind),
    /// A main-handshake server `Certificate` carried a nonempty context.
    NonEmptyContext,
    /// The server supplied no certificate entries.
    EmptyCertificateList,
    /// One exact entry extension vector was structurally invalid.
    EntryExtensions(ExtensionBlockError),
    /// Placement or final ClientHello offer policy rejected one entry extension.
    EntryContext(ExtensionContextPolicyError),
}

impl fmt::Display for RetryServerCertificateValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongMessageType => "TLS 1.3 retry message is not Certificate",
            Self::WrongFlightState => {
                "TLS 1.3 retry server Certificate has the wrong ordered-flight state"
            }
            Self::Structure(_) => "TLS 1.3 retry server Certificate structure is invalid",
            Self::NonEmptyContext => {
                "TLS 1.3 main-handshake server Certificate context is not empty"
            }
            Self::EmptyCertificateList => "TLS 1.3 server Certificate list is empty",
            Self::EntryExtensions(_) => "TLS 1.3 server Certificate entry extensions are invalid",
            Self::EntryContext(_) => {
                "TLS 1.3 server Certificate policy rejected an entry extension"
            }
        })
    }
}

impl std::error::Error for RetryServerCertificateValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EntryExtensions(error) => Some(error),
            Self::EntryContext(error) => Some(error),
            Self::WrongMessageType
            | Self::WrongFlightState
            | Self::Structure(_)
            | Self::NonEmptyContext
            | Self::EmptyCertificateList => None,
        }
    }
}

/// Exact authenticated nonempty main-handshake server `Certificate` proof.
///
/// This proof intentionally derives neither `Copy` nor `Clone`. It retains the
/// pending ordered message and exact decoded certificate body after every entry
/// extension vector passed retained placement and offer policy.
pub struct BoundRetryServerCertificate<'pending, 'message, 'retry_evidence, 'issuer> {
    pending: &'pending PendingRetryServerTranscriptMessage<'message, 'retry_evidence, 'issuer>,
    certificate: Tls13CertificateMessage<'message>,
}

impl<'pending, 'message, 'retry_evidence, 'issuer>
    BoundRetryServerCertificate<'pending, 'message, 'retry_evidence, 'issuer>
{
    /// Borrow the exact pending ordered message and pre-message snapshot.
    #[must_use]
    pub const fn pending(
        &self,
    ) -> &'pending PendingRetryServerTranscriptMessage<'message, 'retry_evidence, 'issuer> {
        self.pending
    }

    /// Borrow the exact structurally decoded and policy-authorized certificate.
    #[must_use]
    pub const fn certificate(&self) -> &Tls13CertificateMessage<'message> {
        &self.certificate
    }
}

impl fmt::Debug for BoundRetryServerCertificate<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryServerCertificate")
            .field("pending", &"[RETAINED]")
            .field("certificate", &"[REDACTED]")
            .finish()
    }
}

/// Stable payload-free failures while binding an authenticated retry server
/// certificate entry to one repository-decoded X.509 certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryServerCertificateLeafBindingError {
    /// The decoded certificate does not retain the exact first entry bytes.
    LeafCertificateMismatch,
}

impl fmt::Display for RetryServerCertificateLeafBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("TLS 1.3 retry server Certificate leaf and decoded X.509 certificate differ")
    }
}

impl std::error::Error for RetryServerCertificateLeafBindingError {}

/// Exact authenticated retry server-certificate proof bound to its decoded leaf.
///
/// This proof intentionally derives neither `Copy` nor `Clone`. It retains the
/// complete ordered server-certificate proof beside the exact repository-owned
/// X.509 decode whose complete DER bytes equal the first certificate entry.
pub struct BoundRetryServerCertificateLeaf<
    'binding,
    'pending,
    'message,
    'retry_evidence,
    'issuer,
    'certificate,
    'encoded,
> {
    server_certificate:
        &'binding BoundRetryServerCertificate<'pending, 'message, 'retry_evidence, 'issuer>,
    leaf_certificate: &'certificate X509Certificate<'encoded>,
}

impl<'binding, 'pending, 'message, 'retry_evidence, 'issuer, 'certificate, 'encoded>
    BoundRetryServerCertificateLeaf<
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'certificate,
        'encoded,
    >
{
    /// Borrow the complete authenticated ordered server-certificate proof.
    #[must_use]
    pub const fn server_certificate(
        &self,
    ) -> &'binding BoundRetryServerCertificate<'pending, 'message, 'retry_evidence, 'issuer> {
        self.server_certificate
    }

    /// Borrow the exact repository-decoded leaf certificate.
    #[must_use]
    pub const fn leaf_certificate(&self) -> &'certificate X509Certificate<'encoded> {
        self.leaf_certificate
    }
}

impl fmt::Debug for BoundRetryServerCertificateLeaf<'_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryServerCertificateLeaf")
            .field("server_certificate", &"[RETAINED]")
            .field("leaf_certificate", &"[REDACTED]")
            .finish()
    }
}

/// Bind the exact first authenticated retry Certificate entry to one exact
/// repository-decoded X.509 certificate.
pub fn bind_retry_server_certificate_leaf<
    'binding,
    'pending,
    'message,
    'retry_evidence,
    'issuer,
    'certificate,
    'encoded,
>(
    server_certificate: &'binding BoundRetryServerCertificate<
        'pending,
        'message,
        'retry_evidence,
        'issuer,
    >,
    leaf_certificate: &'certificate X509Certificate<'encoded>,
) -> Result<
    BoundRetryServerCertificateLeaf<
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'certificate,
        'encoded,
    >,
    RetryServerCertificateLeafBindingError,
> {
    let first_entry = server_certificate
        .certificate()
        .entries()
        .next()
        .ok_or(RetryServerCertificateLeafBindingError::LeafCertificateMismatch)?;
    if first_entry.certificate_data() != leaf_certificate.encoded() {
        return Err(RetryServerCertificateLeafBindingError::LeafCertificateMismatch);
    }
    Ok(BoundRetryServerCertificateLeaf {
        server_certificate,
        leaf_certificate,
    })
}

/// Stable payload-free failures for exact retry server `CertificateVerify` validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryServerCertificateVerifyValidationError {
    /// The pending ordered message was not `CertificateVerify`.
    WrongMessageType,
    /// The ordered flight was not immediately after server `CertificateVerify`.
    WrongFlightState,
    /// The exact authenticated body was structurally invalid.
    Structure(MessageError),
    /// The final ClientHello omitted its required `signature_algorithms` offer.
    MissingSignatureAlgorithms,
    /// The retained final offer was structurally invalid.
    SignatureAlgorithms(SignatureSchemeListError),
    /// The peer selected a scheme absent from the retained final offer.
    UnofferedSignatureScheme,
    /// The caller's data-driven repository policy rejected the selected scheme.
    Policy(SignatureSchemePolicyError),
}

impl fmt::Display for RetryServerCertificateVerifyValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongMessageType => "TLS 1.3 retry message is not CertificateVerify",
            Self::WrongFlightState => {
                "TLS 1.3 retry CertificateVerify has the wrong ordered-flight state"
            }
            Self::Structure(_) => "TLS 1.3 retry CertificateVerify structure is invalid",
            Self::MissingSignatureAlgorithms => {
                "TLS 1.3 final ClientHello omitted signature_algorithms"
            }
            Self::SignatureAlgorithms(_) => {
                "TLS 1.3 final ClientHello signature_algorithms is invalid"
            }
            Self::UnofferedSignatureScheme => {
                "TLS 1.3 server CertificateVerify selected an unoffered signature scheme"
            }
            Self::Policy(_) => {
                "TLS 1.3 server CertificateVerify signature scheme is not authorized"
            }
        })
    }
}

impl std::error::Error for RetryServerCertificateVerifyValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Structure(error) => Some(error),
            Self::SignatureAlgorithms(error) => Some(error),
            Self::Policy(error) => Some(error),
            Self::WrongMessageType
            | Self::WrongFlightState
            | Self::MissingSignatureAlgorithms
            | Self::UnofferedSignatureScheme => None,
        }
    }
}

/// Exact authenticated server `CertificateVerify` bound to offer and policy.
///
/// This proof intentionally derives neither `Copy` nor `Clone`. It retains the
/// pending pre-message transcript snapshot, exact decoded message, exact final
/// ClientHello offer, and policy-authorized scheme. Cryptographic verification
/// remains a separate required boundary.
pub struct BoundRetryServerCertificateVerify<'pending, 'message, 'retry_evidence, 'issuer> {
    pending: &'pending PendingRetryServerTranscriptMessage<'message, 'retry_evidence, 'issuer>,
    message: CertificateVerifyMessage<'message>,
    offered_signature_algorithms: SignatureSchemeList<'retry_evidence>,
    authorized_scheme: AuthorizedSignatureScheme,
}

impl<'pending, 'message, 'retry_evidence, 'issuer>
    BoundRetryServerCertificateVerify<'pending, 'message, 'retry_evidence, 'issuer>
{
    /// Borrow the exact pending ordered message and pre-message snapshot.
    #[must_use]
    pub const fn pending(
        &self,
    ) -> &'pending PendingRetryServerTranscriptMessage<'message, 'retry_evidence, 'issuer> {
        self.pending
    }

    /// Borrow the exact structurally decoded `CertificateVerify` body.
    #[must_use]
    pub const fn message(&self) -> &CertificateVerifyMessage<'message> {
        &self.message
    }

    /// Borrow the exact retained final ClientHello signature offer.
    #[must_use]
    pub const fn offered_signature_algorithms(&self) -> &SignatureSchemeList<'retry_evidence> {
        &self.offered_signature_algorithms
    }

    /// Return the exact offered scheme authorized by repository policy.
    #[must_use]
    pub const fn authorized_scheme(&self) -> AuthorizedSignatureScheme {
        self.authorized_scheme
    }
}

impl fmt::Debug for BoundRetryServerCertificateVerify<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryServerCertificateVerify")
            .field("pending", &"[RETAINED]")
            .field("message", &"[REDACTED]")
            .field("offered_signature_algorithms", &"[REDACTED]")
            .field("authorized_scheme", &"[RETAINED]")
            .finish()
    }
}

/// Stable payload-free failures while binding ordered retry evidence to an
/// exact repository-owned signature verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryServerCertificateVerifySignatureBindingError {
    /// The signature verdict authenticated the client role instead of server.
    WrongEndpointRole,
    /// The signature verdict used a different selected-transcript snapshot.
    TranscriptSnapshotMismatch,
    /// The signature verdict used a different policy-authorized scheme.
    SignatureSchemeMismatch,
    /// The signature verdict authenticated a different decoded message.
    MessageMismatch,
}

impl fmt::Display for RetryServerCertificateVerifySignatureBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongEndpointRole => {
                "TLS 1.3 retry CertificateVerify signature verdict has the wrong endpoint role"
            }
            Self::TranscriptSnapshotMismatch => {
                "TLS 1.3 retry CertificateVerify signature verdict has a different transcript snapshot"
            }
            Self::SignatureSchemeMismatch => {
                "TLS 1.3 retry CertificateVerify signature verdict has a different authorized scheme"
            }
            Self::MessageMismatch => {
                "TLS 1.3 retry CertificateVerify signature verdict has a different message"
            }
        })
    }
}

impl std::error::Error for RetryServerCertificateVerifySignatureBindingError {}

/// Exact ordered retry `CertificateVerify` retained with its RSA-PSS verdict.
///
/// This proof intentionally derives neither `Copy` nor `Clone`. It retains the
/// complete offer- and policy-bound ordered-message proof beside the exact
/// repository-owned selected-transcript signature verdict after role,
/// transcript snapshot, authorized scheme, and decoded message equality all
/// pass.
pub struct VerifiedBoundRetryServerCertificateVerifySignature<
    'binding,
    'pending,
    'message,
    'retry_evidence,
    'issuer,
    'verification,
    'hashed,
    'bound,
    'transcript,
    'output,
    'verified_message,
    'verified_body,
    'key,
> {
    binding:
        &'binding BoundRetryServerCertificateVerify<'pending, 'message, 'retry_evidence, 'issuer>,
    verification: &'verification VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'verified_message,
        'verified_body,
        'key,
    >,
}

impl<
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'verification,
        'hashed,
        'bound,
        'transcript,
        'output,
        'verified_message,
        'verified_body,
        'key,
    >
    VerifiedBoundRetryServerCertificateVerifySignature<
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'verification,
        'hashed,
        'bound,
        'transcript,
        'output,
        'verified_message,
        'verified_body,
        'key,
    >
{
    /// Borrow the exact authenticated ordered-message proof.
    #[must_use]
    pub const fn binding(
        &self,
    ) -> &'binding BoundRetryServerCertificateVerify<'pending, 'message, 'retry_evidence, 'issuer>
    {
        self.binding
    }

    /// Borrow the exact repository-owned RSA-PSS signature verdict.
    #[must_use]
    pub const fn verification(
        &self,
    ) -> &'verification VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'verified_message,
        'verified_body,
        'key,
    > {
        self.verification
    }
}

impl fmt::Debug
    for VerifiedBoundRetryServerCertificateVerifySignature<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedBoundRetryServerCertificateVerifySignature")
    }
}

/// Bind one exact authenticated ordered retry `CertificateVerify` proof to
/// one exact repository-owned selected-transcript RSA-PSS verdict.
///
/// Every comparison is public-data only. Failure returns no composed evidence
/// and retains no peer payload in its stable error category.
#[allow(clippy::too_many_arguments)]
pub fn bind_retry_server_certificate_verify_signature<
    'binding,
    'pending,
    'message,
    'retry_evidence,
    'issuer,
    'verification,
    'hashed,
    'bound,
    'transcript,
    'output,
    'verified_message,
    'verified_body,
    'key,
>(
    binding: &'binding BoundRetryServerCertificateVerify<
        'pending,
        'message,
        'retry_evidence,
        'issuer,
    >,
    verification: &'verification VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'verified_message,
        'verified_body,
        'key,
    >,
) -> Result<
    VerifiedBoundRetryServerCertificateVerifySignature<
        'binding,
        'pending,
        'message,
        'retry_evidence,
        'issuer,
        'verification,
        'hashed,
        'bound,
        'transcript,
        'output,
        'verified_message,
        'verified_body,
        'key,
    >,
    RetryServerCertificateVerifySignatureBindingError,
> {
    let hashed_input = verification.hashed_input();
    let bound_input = hashed_input.bound_input();
    if bound_input.input().role() != CertificateVerifyRole::Server {
        return Err(RetryServerCertificateVerifySignatureBindingError::WrongEndpointRole);
    }
    if bound_input.snapshot() != binding.pending().transcript_snapshot() {
        return Err(RetryServerCertificateVerifySignatureBindingError::TranscriptSnapshotMismatch);
    }
    if hashed_input.hashed_input().authorized_scheme() != binding.authorized_scheme() {
        return Err(RetryServerCertificateVerifySignatureBindingError::SignatureSchemeMismatch);
    }
    if verification.verification().signature().message() != binding.message() {
        return Err(RetryServerCertificateVerifySignatureBindingError::MessageMismatch);
    }

    Ok(VerifiedBoundRetryServerCertificateVerifySignature {
        binding,
        verification,
    })
}

/// Stable payload-free failures while authenticating one exact ordered retry
/// server `Finished` message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderedRetryServerFinishedVerificationError {
    /// The authenticated ordered message was not `Finished`.
    WrongMessageType,
    /// The ordered flight had not reached its post-Finished complete state.
    WrongFlightState,
    /// The ordered message and Finished-key proof retain different retry evidence.
    RetryEvidenceMismatch,
    /// The Finished-key proof unexpectedly retained a non-server key.
    WrongEndpointRole,
    /// The ordered message and Finished-key proof retain different suites.
    CipherSuiteMismatch,
    /// The peer Finished body length was not the selected suite's hash length.
    PeerVerifyDataLengthMismatch,
    /// Repository-owned Finished computation failed before comparison.
    FinishedComputation,
    /// The exact authenticated peer verify-data did not match.
    VerifyDataMismatch,
}

impl fmt::Display for OrderedRetryServerFinishedVerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongMessageType => "ordered retry server message is not Finished",
            Self::WrongFlightState => "ordered retry server Finished has the wrong flight state",
            Self::RetryEvidenceMismatch => {
                "ordered retry server Finished and key proof retain different retry evidence"
            }
            Self::WrongEndpointRole => {
                "ordered retry server Finished key retained the wrong endpoint role"
            }
            Self::CipherSuiteMismatch => {
                "ordered retry server Finished and key proof retain different cipher suites"
            }
            Self::PeerVerifyDataLengthMismatch => {
                "ordered retry server Finished verify-data length is invalid"
            }
            Self::FinishedComputation => "ordered retry server Finished computation failed",
            Self::VerifyDataMismatch => {
                "ordered retry server Finished verify-data did not authenticate"
            }
        })
    }
}

impl std::error::Error for OrderedRetryServerFinishedVerificationError {}

/// Exact authenticated ordered retry server `Finished` evidence.
///
/// The result retains the single-use ordered record-owner identity, complete
/// retry-derived server Finished-key proof, exact pre-message transcript
/// snapshot, and repository-owned verdict. It intentionally derives neither
/// `Copy` nor `Clone` and retains no callback-scoped message borrow.
pub struct VerifiedOrderedRetryServerFinished<
    'proof,
    'finished_bound,
    'key_storage,
    'evidence,
    'secret_storage,
    'issuer,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    ordered_server_flight_identity: OrderedRetryServerFlightIdentity<'issuer>,
    server_finished_key: &'proof BoundRetryServerFinishedKey<
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        Evidence,
    >,
    transcript_snapshot: TranscriptHash,
    client_authentication_requested: bool,
    verification: VerifiedFinished,
}

impl<
        'proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'issuer,
        Evidence: ExactRetryBaseSecrets + ?Sized,
    >
    VerifiedOrderedRetryServerFinished<
        'proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'issuer,
        Evidence,
    >
{
    /// Return the opaque identity of the single-use server record-state issuer.
    #[must_use]
    pub const fn ordered_server_flight_identity(
        &self,
    ) -> OrderedRetryServerFlightIdentity<'issuer> {
        self.ordered_server_flight_identity
    }

    /// Borrow the complete retry-derived server Finished-key proof.
    #[must_use]
    pub const fn server_finished_key(
        &self,
    ) -> &BoundRetryServerFinishedKey<
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        Evidence,
    > {
        self.server_finished_key
    }

    /// Return the exact transcript snapshot immediately before `Finished`.
    #[must_use]
    pub const fn transcript_snapshot(&self) -> TranscriptHash {
        self.transcript_snapshot
    }

    /// Return whether the ordered server prefix requested client authentication.
    #[must_use]
    pub const fn client_authentication_requested(&self) -> bool {
        self.client_authentication_requested
    }

    /// Borrow the repository-owned exact peer Finished verdict.
    #[must_use]
    pub const fn verification(&self) -> &VerifiedFinished {
        &self.verification
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for VerifiedOrderedRetryServerFinished<'_, '_, '_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedOrderedRetryServerFinished")
            .field("ordered_server_flight_identity", &"[REDACTED]")
            .field("server_finished_key", &"[SECRET]")
            .field("transcript_snapshot", &"[RETAINED]")
            .field(
                "client_authentication_requested",
                &self.client_authentication_requested,
            )
            .field("verification", &"[RETAINED]")
            .finish()
    }
}

/// Authenticate one exact ordered retry server `Finished` message.
///
/// The pending message is admitted only after the record, reassembly, and
/// server-flight ordering layers accepted it. This boundary additionally
/// requires exact retry-evidence object identity, a server-role Finished key,
/// and suite equality before verifying the exact body against the exact
/// pre-message transcript snapshot. The returned verdict retains those peer
/// bytes in fixed memory but no callback-scoped message borrow.
pub fn verify_ordered_retry_server_finished<
    'proof,
    'finished_bound,
    'key_storage,
    'evidence,
    'secret_storage,
    'issuer,
    Evidence,
>(
    pending: &PendingRetryServerTranscriptMessage<'_, '_, 'issuer>,
    server_finished_key: &'proof BoundRetryServerFinishedKey<
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        Evidence,
    >,
) -> Result<
    VerifiedOrderedRetryServerFinished<
        'proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'issuer,
        Evidence,
    >,
    OrderedRetryServerFinishedVerificationError,
>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    if pending.message.handshake_type() != tls13_handshake::Tls13HandshakeType::Finished {
        return Err(OrderedRetryServerFinishedVerificationError::WrongMessageType);
    }
    if pending.state != ServerFlightState::Complete {
        return Err(OrderedRetryServerFinishedVerificationError::WrongFlightState);
    }
    let key_evidence = server_finished_key
        .handshake_traffic_secrets()
        .evidence()
        .as_exact_retry_base_secrets();
    if !core::ptr::eq(pending.retry_evidence, key_evidence) {
        return Err(OrderedRetryServerFinishedVerificationError::RetryEvidenceMismatch);
    }
    let finished_key = server_finished_key.finished_key();
    if finished_key.role() != FinishedRole::Server {
        return Err(OrderedRetryServerFinishedVerificationError::WrongEndpointRole);
    }
    if finished_key.selected_suite() != pending.retry_evidence.selected_suite() {
        return Err(OrderedRetryServerFinishedVerificationError::CipherSuiteMismatch);
    }
    let verification = finished_key
        .verify_peer(
            pending.transcript_snapshot.as_bytes(),
            pending.message.body(),
        )
        .map_err(map_ordered_retry_server_finished_error)?;

    Ok(VerifiedOrderedRetryServerFinished {
        ordered_server_flight_identity: pending.ordered_server_flight_identity,
        server_finished_key,
        transcript_snapshot: pending.transcript_snapshot,
        client_authentication_requested: pending.client_authentication_requested,
        verification,
    })
}

fn map_ordered_retry_server_finished_error(
    error: FinishedError,
) -> OrderedRetryServerFinishedVerificationError {
    match error {
        FinishedError::PeerVerifyDataLengthMismatch => {
            OrderedRetryServerFinishedVerificationError::PeerVerifyDataLengthMismatch
        }
        FinishedError::VerifyDataMismatch => {
            OrderedRetryServerFinishedVerificationError::VerifyDataMismatch
        }
        FinishedError::HashMismatch
        | FinishedError::TranscriptHashLengthMismatch
        | FinishedError::OutputLengthMismatch
        | FinishedError::KeyDerivation(_)
        | FinishedError::Mac(_) => OrderedRetryServerFinishedVerificationError::FinishedComputation,
    }
}

/// Fresh authenticated ordered server-flight state joined to its exact retry transcript.
///
/// This value intentionally derives neither `Copy` nor `Clone`. It exposes no
/// mutable transcript or record-state access; a later composition must consume
/// this complete wrapper to advance either side without detaching provenance.
pub struct BoundRetryOrderedServerFlightTranscript<
    'continuation,
    'finished_bound,
    'key_storage,
    'issuer,
    'evidence,
    'secret_storage,
    'material,
    'reassembly,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    ordered_flight: RetryOrderedServerFlight<
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        Evidence,
    >,
    server_finished_key: &'continuation BoundRetryServerFinishedKey<
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        Evidence,
    >,
    transcript: SelectedTls13Transcript,
}

impl<
        'continuation,
        'finished_bound,
        'key_storage,
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        Evidence: ExactRetryBaseSecrets + ?Sized,
    >
    BoundRetryOrderedServerFlightTranscript<
        'continuation,
        'finished_bound,
        'key_storage,
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        Evidence,
    >
{
    /// Borrow the fresh authenticated ordered server-flight owner.
    #[must_use]
    pub const fn ordered_flight(
        &self,
    ) -> &RetryOrderedServerFlight<
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        Evidence,
    > {
        &self.ordered_flight
    }

    /// Borrow the exact retry-derived server Finished-key proof.
    #[must_use]
    pub const fn server_finished_key(
        &self,
    ) -> &BoundRetryServerFinishedKey<
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        Evidence,
    > {
        self.server_finished_key
    }

    /// Borrow the reconstructed exact retry transcript through ServerHello.
    #[must_use]
    pub const fn transcript(&self) -> &SelectedTls13Transcript {
        &self.transcript
    }

    /// Return the suite selected by all three retained components.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.ordered_flight.selected_suite()
    }

    /// Authenticate one record, authorize ordered messages, and advance the transcript.
    ///
    /// Each callback receives the exact transcript snapshot from immediately
    /// before its message. Returning `true` authorizes hashing that message's
    /// exact header-plus-body encoding; returning `false` permanently closes
    /// the record owner without hashing the rejected message. Record,
    /// reassembly, ordering, transcript, and handler-unwind failures are also
    /// terminal. Earlier callback effects are not transactional.
    pub fn open_accept_and_advance<'owner, 'input, F>(
        &'owner mut self,
        ciphertext_policy: &Tls13CiphertextPolicy,
        inner_policy: &Tls13InnerPlaintextPolicy,
        input: &'input [u8],
        opened_output: &mut [u8],
        mut authorize_message: F,
    ) -> Result<
        AdvancedRetryServerTranscriptRecord<
            'owner,
            'continuation,
            'finished_bound,
            'key_storage,
            'issuer,
            'evidence,
            'secret_storage,
            'material,
            'reassembly,
            'input,
            Evidence,
        >,
        RetryOrderedServerTranscriptAdvanceError,
    >
    where
        F: for<'message> FnMut(
            PendingRetryServerTranscriptMessage<'message, 'evidence, 'issuer>,
        ) -> bool,
    {
        let ordered_server_flight_identity = OrderedRetryServerFlightIdentity::from_issuer(
            self.ordered_flight.reassembler.state.issuer,
        );
        let retry_evidence = self
            .server_finished_key
            .handshake_traffic_secrets()
            .evidence()
            .as_exact_retry_base_secrets();
        let Self {
            ordered_flight,
            server_finished_key,
            transcript,
        } = self;
        let reassembler = &mut ordered_flight.reassembler;
        let flight = &mut ordered_flight.flight;
        let mut committed_finished = false;
        let reassembled = match reassembler.open_and_reassemble_with(
            ciphertext_policy,
            inner_policy,
            input,
            opened_output,
            |message| {
                let state = flight
                    .accept(message.handshake_type())
                    .map_err(RetryOrderedServerTranscriptAdvanceError::Ordering)?;
                let encoded = message.encoded();
                let pending = PendingRetryServerTranscriptMessage {
                    message,
                    retry_evidence,
                    ordered_server_flight_identity,
                    transcript_snapshot: transcript.snapshot(),
                    state,
                    client_authentication_requested: flight.client_authentication_requested(),
                };
                if !authorize_message(pending) {
                    return Err(RetryOrderedServerTranscriptAdvanceError::MessageRejected);
                }
                transcript
                    .update(encoded)
                    .map_err(RetryOrderedServerTranscriptAdvanceError::Transcript)?;
                // Only the ExpectFinished -> Complete transition can reach
                // this state inside the handler. Record success is checked
                // separately below; a later failure publishes no receipt.
                committed_finished = state == ServerFlightState::Complete;
                Ok(())
            },
        ) {
            Ok(result) => result,
            Err(ReassemblyHandlerError::Reassembly(error)) => {
                return Err(RetryOrderedServerTranscriptAdvanceError::Reassembly(error));
            }
            Err(ReassemblyHandlerError::Handler(error)) => return Err(error),
        };

        Ok(AdvancedRetryServerTranscriptRecord {
            reassembled,
            server_finished_key: *server_finished_key,
            committed_finished,
            transcript_snapshot: transcript.snapshot(),
            state: flight.state(),
            client_authentication_requested: flight.client_authentication_requested(),
        })
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for BoundRetryOrderedServerFlightTranscript<'_, '_, '_, '_, '_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryOrderedServerFlightTranscript")
            .field("ordered_flight", &"[RETAINED]")
            .field("server_finished_key", &"[SECRET]")
            .field("transcript", &"[RETAINED]")
            .finish()
    }
}

/// Failure to bind verified server Finished to its committed transcript.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderedRetryServerFinishedCommitError {
    /// This successful record did not append the ordered Finished encoding.
    FinishedNotCommitted,
    /// The record ended with an incomplete handshake message after Finished.
    TrailingHandshakeFragment,
    /// The record and Finished verdict came from different server-state issuers.
    RecordOwnerMismatch,
    /// The record and Finished verdict retain different Finished-key proofs.
    FinishedKeyMismatch,
}

impl fmt::Display for OrderedRetryServerFinishedCommitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::FinishedNotCommitted => "record did not commit ordered retry server Finished",
            Self::TrailingHandshakeFragment => {
                "record contains a handshake fragment after Finished"
            }
            Self::RecordOwnerMismatch => {
                "record and Finished verdict retain different ordered owners"
            }
            Self::FinishedKeyMismatch => "record and Finished verdict retain different key proofs",
        })
    }
}

impl std::error::Error for OrderedRetryServerFinishedCommitError {}

/// Evidence that the authenticated server Finished was appended at a record boundary.
///
/// This owns the exact verification result and the transcript snapshot
/// immediately after its header and body were hashed. It retains no record or
/// callback buffer and is neither copyable nor cloneable.
pub struct CommittedOrderedRetryServerFinished<
    'proof,
    'finished_bound,
    'key_storage,
    'evidence,
    'secret_storage,
    'issuer,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    finished: VerifiedOrderedRetryServerFinished<
        'proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'issuer,
        Evidence,
    >,
    transcript_snapshot: TranscriptHash,
}

impl<'proof, 'finished_bound, 'key_storage, 'evidence, 'secret_storage, 'issuer, Evidence>
    CommittedOrderedRetryServerFinished<
        'proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'issuer,
        Evidence,
    >
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Borrow the full authenticated Finished proof, including its pre-message snapshot.
    #[must_use]
    pub const fn finished(
        &self,
    ) -> &VerifiedOrderedRetryServerFinished<
        'proof,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        'issuer,
        Evidence,
    > {
        &self.finished
    }

    /// Return the exact transcript snapshot through the authenticated server Finished.
    #[must_use]
    pub const fn transcript_snapshot(&self) -> TranscriptHash {
        self.transcript_snapshot
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for CommittedOrderedRetryServerFinished<'_, '_, '_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CommittedOrderedRetryServerFinished([REDACTED])")
    }
}

/// Join one fresh ordered server-flight owner to its exact retry continuation.
///
/// Freshness, exact traffic-secret object identity, suite equality, and the
/// retained ServerHello snapshot are checked before either non-copyable input
/// is exposed through the result. No detached transcript, key, record state,
/// proof, suite, hash, or resumed flight position is accepted.
pub fn bind_retry_ordered_server_flight_transcript<
    'continuation,
    'finished_bound,
    'key_storage,
    'issuer,
    'evidence,
    'secret_storage,
    'material,
    'reassembly,
    Evidence,
>(
    ordered_flight: RetryOrderedServerFlight<
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        Evidence,
    >,
    continuation: BoundRetryServerTranscriptContinuation<
        'continuation,
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        Evidence,
    >,
) -> Result<
    BoundRetryOrderedServerFlightTranscript<
        'continuation,
        'finished_bound,
        'key_storage,
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        Evidence,
    >,
    RetryOrderedServerFlightTranscriptBindingError,
>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    if ordered_flight.state() != ServerFlightState::ExpectEncryptedExtensions
        || ordered_flight.records_used() != 0
        || ordered_flight.message_in_progress()
        || ordered_flight.is_failed()
        || ordered_flight.client_authentication_requested()
    {
        return Err(RetryOrderedServerFlightTranscriptBindingError::NonFreshOrderedFlight);
    }
    if !core::ptr::eq(
        ordered_flight.traffic_secrets(),
        continuation
            .server_finished_key()
            .handshake_traffic_secrets(),
    ) {
        return Err(RetryOrderedServerFlightTranscriptBindingError::TrafficSecretProofMismatch);
    }
    let selected_suite = ordered_flight.selected_suite();
    if continuation.transcript().selected_suite() != selected_suite
        || continuation
            .server_finished_key()
            .finished_key()
            .selected_suite()
            != selected_suite
    {
        return Err(RetryOrderedServerFlightTranscriptBindingError::CipherSuiteMismatch);
    }
    if continuation.transcript().snapshot()
        != ordered_flight
            .traffic_secrets()
            .evidence()
            .transcript_hash()
    {
        return Err(RetryOrderedServerFlightTranscriptBindingError::SnapshotMismatch);
    }

    let (server_finished_key, transcript) = continuation.into_parts();
    Ok(BoundRetryOrderedServerFlightTranscript {
        ordered_flight,
        server_finished_key,
        transcript,
    })
}

/// One authenticated ordered record committed to its exact retry transcript.
///
/// The retained reassembly result keeps the joined owner and exact ciphertext
/// borrowed while the Finished-key proof and post-record transcript snapshot
/// remain attached. This value intentionally derives neither `Copy` nor
/// `Clone`.
pub struct AdvancedRetryServerTranscriptRecord<
    'owner,
    'continuation,
    'finished_bound,
    'key_storage,
    'issuer,
    'evidence,
    'secret_storage,
    'material,
    'reassembly,
    'input,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    reassembled: ReassembledRetryHandshakeRecord<
        'owner,
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        'input,
        Evidence,
    >,
    server_finished_key: &'continuation BoundRetryServerFinishedKey<
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        Evidence,
    >,
    committed_finished: bool,
    transcript_snapshot: TranscriptHash,
    state: ServerFlightState,
    client_authentication_requested: bool,
}

impl<
        'owner,
        'continuation,
        'finished_bound,
        'key_storage,
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        'input,
        Evidence,
    >
    AdvancedRetryServerTranscriptRecord<
        'owner,
        'continuation,
        'finished_bound,
        'key_storage,
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        'input,
        Evidence,
    >
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Borrow the complete exact-message retry traffic-secret proof.
    #[must_use]
    pub const fn traffic_secrets(
        &self,
    ) -> &'issuer BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence> {
        self.reassembled.traffic_secrets()
    }

    /// Borrow the exact retry-derived server Finished-key proof.
    #[must_use]
    pub const fn server_finished_key(
        &self,
    ) -> &BoundRetryServerFinishedKey<
        'finished_bound,
        'key_storage,
        'evidence,
        'secret_storage,
        Evidence,
    > {
        self.server_finished_key
    }

    /// Borrow the exact ciphertext record supplied to this operation.
    #[must_use]
    pub const fn input(&self) -> &'input [u8] {
        self.reassembled.input()
    }

    /// Return the authenticated inner handshake-content length.
    #[must_use]
    pub const fn authenticated_content_length(&self) -> usize {
        self.reassembled.authenticated_content_length()
    }

    /// Return how many ordered messages were authorized and hashed.
    #[must_use]
    pub const fn completed_messages(&self) -> usize {
        self.reassembled.completed_messages()
    }

    /// Return whether the owner retained a final incomplete message.
    #[must_use]
    pub const fn message_in_progress(&self) -> bool {
        self.reassembled.message_in_progress()
    }

    /// Return the sender-state record count after opening this input.
    #[must_use]
    pub const fn records_used(&self) -> u64 {
        self.reassembled.records_used()
    }

    /// Return the server-flight state after this record.
    #[must_use]
    pub const fn state(&self) -> ServerFlightState {
        self.state
    }

    /// Return whether the accepted prefix requested client authentication.
    #[must_use]
    pub const fn client_authentication_requested(&self) -> bool {
        self.client_authentication_requested
    }

    /// Return the transcript snapshot after all authorized complete messages.
    #[must_use]
    pub const fn transcript_snapshot(&self) -> TranscriptHash {
        self.transcript_snapshot
    }

    /// Return whether this record completed the public server-flight sequence.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self.state, ServerFlightState::Complete)
    }

    /// Join exact Finished verification to the record that appended its encoding.
    ///
    /// A single-use issuer can produce only one monotonic ordered server flight,
    /// with one Finished transition. Thus a successfully committed transition
    /// and a verified Finished from that issuer must refer to the same message.
    /// The exact retained key proof must also match. RFC 8446 section 5.1
    /// requires Finished to end at the record boundary: a trailing pending
    /// fragment disqualifies this record, even if the public order is complete.
    /// Both the record and the verdict are consumed even on failure. Verifying
    /// the same Finished twice cannot create a second committed transition.
    ///
    /// ```compile_fail,E0382
    /// use tls13_retry_handshake_traffic_key_material_binding::{
    ///     AdvancedRetryServerTranscriptRecord, VerifiedOrderedRetryServerFinished,
    /// };
    /// use tls13_retry_handshake_traffic_secrets_binding::ExactRetryBaseSecrets;
    ///
    /// fn commit_twice<E: ExactRetryBaseSecrets + ?Sized + 'static>(
    ///     record: AdvancedRetryServerTranscriptRecord<
    ///         'static, 'static, 'static, 'static, 'static,
    ///         'static, 'static, 'static, 'static, 'static, E,
    ///     >,
    ///     verdicts: [VerifiedOrderedRetryServerFinished<
    ///         'static, 'static, 'static, 'static, 'static, 'static, E,
    ///     >; 2],
    /// ) {
    ///     let [first, second] = verdicts;
    ///     let _ = record.bind_committed_finished(first);
    ///     let _ = record.bind_committed_finished(second);
    /// }
    /// ```
    pub fn bind_committed_finished<'proof, 'verified_bound, 'verified_key, 'verified_issuer>(
        self,
        finished: VerifiedOrderedRetryServerFinished<
            'proof,
            'verified_bound,
            'verified_key,
            'evidence,
            'secret_storage,
            'verified_issuer,
            Evidence,
        >,
    ) -> Result<
        CommittedOrderedRetryServerFinished<
            'proof,
            'verified_bound,
            'verified_key,
            'evidence,
            'secret_storage,
            'verified_issuer,
            Evidence,
        >,
        OrderedRetryServerFinishedCommitError,
    > {
        use OrderedRetryServerFinishedCommitError as Error;
        if !self.committed_finished {
            return Err(Error::FinishedNotCommitted);
        }
        if self.message_in_progress() {
            return Err(Error::TrailingHandshakeFragment);
        }
        if OrderedRetryServerFlightIdentity::from_issuer(self.reassembled.owner.state.issuer)
            != finished.ordered_server_flight_identity()
        {
            return Err(Error::RecordOwnerMismatch);
        }
        if !core::ptr::eq(self.server_finished_key(), finished.server_finished_key()) {
            return Err(Error::FinishedKeyMismatch);
        }

        Ok(CommittedOrderedRetryServerFinished {
            finished,
            transcript_snapshot: self.transcript_snapshot,
        })
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for AdvancedRetryServerTranscriptRecord<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdvancedRetryServerTranscriptRecord")
            .field("reassembled", &"[RETAINED]")
            .field("input", &"[REDACTED]")
            .field("server_finished_key", &"[SECRET]")
            .field("transcript_snapshot", &"[RETAINED]")
            .field("state", &self.state)
            .field(
                "client_authentication_requested",
                &self.client_authentication_requested,
            )
            .finish()
    }
}

/// Result of accepting one authenticated record under server-flight order.
///
/// The retained reassembly result keeps the exact input and ordered owner
/// borrowed until this evidence is consumed. This value intentionally derives
/// neither `Copy` nor `Clone`.
pub struct OrderedRetryServerFlightRecord<
    'owner,
    'issuer,
    'evidence,
    'secret_storage,
    'material,
    'reassembly,
    'input,
    Evidence: ExactRetryBaseSecrets + ?Sized,
> {
    reassembled: ReassembledRetryHandshakeRecord<
        'owner,
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        'input,
        Evidence,
    >,
    state: ServerFlightState,
    client_authentication_requested: bool,
}

impl<'owner, 'issuer, 'evidence, 'secret_storage, 'material, 'reassembly, 'input, Evidence>
    OrderedRetryServerFlightRecord<
        'owner,
        'issuer,
        'evidence,
        'secret_storage,
        'material,
        'reassembly,
        'input,
        Evidence,
    >
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    /// Borrow the complete exact-message retry traffic-secret proof.
    #[must_use]
    pub const fn traffic_secrets(
        &self,
    ) -> &'issuer BoundRetryHandshakeTrafficSecrets<'evidence, 'secret_storage, Evidence> {
        self.reassembled.traffic_secrets()
    }

    /// Return the exact selected suite retained by the owner.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.reassembled.selected_suite()
    }

    /// Borrow the exact ciphertext record supplied to this operation.
    #[must_use]
    pub const fn input(&self) -> &'input [u8] {
        self.reassembled.input()
    }

    /// Return the authenticated inner handshake-content length.
    #[must_use]
    pub const fn authenticated_content_length(&self) -> usize {
        self.reassembled.authenticated_content_length()
    }

    /// Return how many ordered messages the handler received.
    #[must_use]
    pub const fn completed_messages(&self) -> usize {
        self.reassembled.completed_messages()
    }

    /// Return whether the owner retained a final incomplete message.
    #[must_use]
    pub const fn message_in_progress(&self) -> bool {
        self.reassembled.message_in_progress()
    }

    /// Return the sender-state record count after opening this input.
    #[must_use]
    pub const fn records_used(&self) -> u64 {
        self.reassembled.records_used()
    }

    /// Return the server-flight state after this record.
    #[must_use]
    pub const fn state(&self) -> ServerFlightState {
        self.state
    }

    /// Return whether the accepted prefix requested client authentication.
    #[must_use]
    pub const fn client_authentication_requested(&self) -> bool {
        self.client_authentication_requested
    }

    /// Return whether this record completed the public server-flight sequence.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self.state, ServerFlightState::Complete)
    }
}

impl<Evidence: ExactRetryBaseSecrets + ?Sized> fmt::Debug
    for OrderedRetryServerFlightRecord<'_, '_, '_, '_, '_, '_, '_, Evidence>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OrderedRetryServerFlightRecord")
            .field("reassembled", &"[RETAINED]")
            .field("input", &"[REDACTED]")
            .field("state", &self.state)
            .field(
                "client_authentication_requested",
                &self.client_authentication_requested,
            )
            .finish()
    }
}

/// Bind one exact retry handshake traffic secret to its traffic key and IV.
///
/// Suite substitution fails before `storage` can be cleared or written. The
/// traffic secret is selected only from the retained exact-message evidence;
/// no detached secret, suite, hash, output length, label, or context is
/// accepted.
pub fn bind_retry_handshake_traffic_key_material<
    'bound,
    'evidence,
    'secret_storage,
    'material,
    Evidence,
>(
    traffic_secrets: &'bound BoundRetryHandshakeTrafficSecrets<
        'evidence,
        'secret_storage,
        Evidence,
    >,
    sender: HandshakeTrafficSender,
    storage: &'material mut TrafficKeyMaterialStorage,
) -> Result<
    BoundRetryHandshakeTrafficKeyMaterial<'bound, 'evidence, 'secret_storage, 'material, Evidence>,
    RetryHandshakeTrafficKeyMaterialBindingError,
>
where
    Evidence: ExactRetryBaseSecrets + ?Sized,
{
    let retained = traffic_secrets.traffic_secrets();
    if storage.selected_suite() != retained.selected_suite() {
        return Err(RetryHandshakeTrafficKeyMaterialBindingError::CipherSuiteMismatch);
    }

    let secret = match sender {
        HandshakeTrafficSender::Client => retained.client_secret(),
        HandshakeTrafficSender::Server => retained.server_secret(),
    };
    let prk = HkdfPrk::import_exact(fixed_hash(retained.selected_suite()), secret)
        .map_err(RetryHandshakeTrafficKeyMaterialBindingError::TrafficSecretImport)?;
    let key_material = derive_traffic_key_material(&prk, storage)
        .map_err(RetryHandshakeTrafficKeyMaterialBindingError::KeyMaterial)?;

    Ok(BoundRetryHandshakeTrafficKeyMaterial {
        traffic_secrets,
        sender,
        key_material,
    })
}

const fn fixed_hash(selected_suite: SelectedCipherSuite) -> FixedHkdfHash {
    match selected_suite.parameters().hkdf_hash() {
        HkdfHash::Sha256 => FixedHkdfHash::Sha256,
        HkdfHash::Sha384 => FixedHkdfHash::Sha384,
    }
}
