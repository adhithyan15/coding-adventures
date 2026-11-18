//! Exact-message TLS 1.3 HelloRetryRequest transcript composition.
//!
//! This crate constructs a transcript only from the four decoded messages
//! retained by [`BoundServerHelloRetrySelection`]. It accepts no detached
//! message bytes or cipher-suite policy and grants no handshake or transport
//! authority.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_server_hello_retry_selection::BoundServerHelloRetrySelection;
use tls13_transcript::{
    PendingTls13Transcript, SelectedTls13Transcript, TranscriptError, TranscriptPolicy,
};

const CLIENT_HELLO_HANDSHAKE_TYPE: u8 = 1;
const SERVER_HELLO_HANDSHAKE_TYPE: u8 = 2;

/// One selected retry transcript attached to its exact retained messages.
///
/// This value intentionally derives neither `Copy` nor `Clone`.
pub struct BoundRetryTranscript<
    'bound,
    'retry_client,
    'retry_cipher_offer,
    'retry_server,
    'retry_version_offer,
    'retry_rules,
    'final_client,
    'final_cipher_offer,
    'final_server,
    'final_version_offer,
    'final_rules,
> {
    messages: &'bound BoundServerHelloRetrySelection<
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    >,
    transcript: SelectedTls13Transcript,
}

impl<
        'bound,
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    >
    BoundRetryTranscript<
        'bound,
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    >
{
    /// Borrow the exact four-message selection proof.
    #[must_use]
    pub const fn messages(
        &self,
    ) -> &'bound BoundServerHelloRetrySelection<
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    > {
        self.messages
    }

    /// Borrow the selected repository-owned transcript state.
    #[must_use]
    pub const fn transcript(&self) -> &SelectedTls13Transcript {
        &self.transcript
    }
}

impl fmt::Debug for BoundRetryTranscript<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryTranscript")
            .field("messages", &"[RETAINED]")
            .field("transcript", &"[RETAINED]")
            .finish()
    }
}

/// Construct the exact selected transcript for one retained retry sequence.
///
/// The transcript is local until every update succeeds, so a policy or hash
/// failure cannot expose a partially advanced state.
pub fn bind_retry_transcript<
    'bound,
    'retry_client,
    'retry_cipher_offer,
    'retry_server,
    'retry_version_offer,
    'retry_rules,
    'final_client,
    'final_cipher_offer,
    'final_server,
    'final_version_offer,
    'final_rules,
>(
    messages: &'bound BoundServerHelloRetrySelection<
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    >,
    policy: TranscriptPolicy,
) -> Result<
    BoundRetryTranscript<
        'bound,
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    >,
    TranscriptError,
> {
    let retry = messages.retry().selection();
    let final_selection = messages.server_hello().selection();

    let mut pending = PendingTls13Transcript::new(policy);
    update_pending_handshake(
        &mut pending,
        CLIENT_HELLO_HANDSHAKE_TYPE,
        retry.client_hello().body(),
    )?;

    let mut transcript = pending.select(retry.cipher_suite());
    transcript.apply_hello_retry_request()?;
    update_selected_handshake(
        &mut transcript,
        SERVER_HELLO_HANDSHAKE_TYPE,
        retry.server_hello().encoded(),
    )?;
    update_selected_handshake(
        &mut transcript,
        CLIENT_HELLO_HANDSHAKE_TYPE,
        final_selection.client_hello().body(),
    )?;
    update_selected_handshake(
        &mut transcript,
        SERVER_HELLO_HANDSHAKE_TYPE,
        final_selection.server_hello().encoded(),
    )?;

    Ok(BoundRetryTranscript {
        messages,
        transcript,
    })
}

fn update_pending_handshake(
    transcript: &mut PendingTls13Transcript,
    message_type: u8,
    body: &[u8],
) -> Result<(), TranscriptError> {
    let header = handshake_header(message_type, body.len());
    transcript.update(&header)?;
    transcript.update(body)
}

fn update_selected_handshake(
    transcript: &mut SelectedTls13Transcript,
    message_type: u8,
    body: &[u8],
) -> Result<(), TranscriptError> {
    let header = handshake_header(message_type, body.len());
    transcript.update(&header)?;
    transcript.update(body)
}

fn handshake_header(message_type: u8, body_length: usize) -> [u8; 4] {
    let body_length = u32::try_from(body_length)
        .expect("decoded TLS 1.3 handshake body length always fits in 24 bits");
    debug_assert!(body_length <= 0x00ff_ffff);
    let length = body_length.to_be_bytes();
    [message_type, length[1], length[2], length[3]]
}
