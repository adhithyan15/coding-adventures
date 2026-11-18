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
use tls13_cipher_suite_policy::{HkdfHash, SelectedCipherSuite};
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
    BoundRetryHandshakeTrafficSecrets, ExactRetryBaseSecrets,
};
use tls13_traffic_key_material::{
    derive_traffic_key_material, TrafficKeyMaterial, TrafficKeyMaterialError,
    TrafficKeyMaterialStorage,
};

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
