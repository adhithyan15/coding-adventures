//! Cipher-suite-bound TLS 1.3 traffic key and record nonce state.
//!
//! One [`RecordProtectionState`] consumes one success-only
//! [`TrafficKeyMaterial`] view and creates the matching monotonic record nonce
//! sequence directly from the same selected cipher suite. Callers cannot
//! detach the key, static IV, nonce length, AEAD routing identifier, or record
//! ceiling from one another.
//!
//! The state represents one traffic key in one direction. It has no reset or
//! key-update operation. Every successful [`RecordProtectionState::reserve_next`]
//! consumes the sequence before returning a borrowed reservation, so a later
//! AEAD failure cannot make that nonce available again.
//!
//! This crate does not invoke an AEAD, protect a record, derive a traffic
//! secret, process a handshake, validate certificates, perform I/O, or provide
//! TLS or HTTPS.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_cipher_suite_policy::{AeadAlgorithm, SelectedCipherSuite};
use tls13_record_nonce::{Error as NonceError, NoncePolicy, RecordNonceSequence, ReservedNonce};
use tls13_traffic_key_material::{TrafficKeyMaterial, MAX_TRAFFIC_IV_LENGTH};

/// Stable, payload-free failures for record-protection state composition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordProtectionStateError {
    /// The selected suite's retained policy exceeded the nonce-state bounds.
    UnsupportedSuiteParameters,
    /// The caller-authorized per-key record ceiling has been reached.
    SequenceExhausted,
    /// Bound components disagreed despite originating from one material view.
    InvariantViolation,
}

impl fmt::Display for RecordProtectionStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnsupportedSuiteParameters => {
                "TLS 1.3 suite parameters exceed record-protection bounds"
            }
            Self::SequenceExhausted => "TLS 1.3 record sequence exhausted",
            Self::InvariantViolation => "TLS 1.3 record-protection invariant failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RecordProtectionStateError {}

/// Non-cloneable record-protection state for one traffic key and direction.
///
/// The state owns the success-only key-material view, but the secret bytes stay
/// in the caller-owned `TrafficKeyMaterialStorage` borrowed by that view. The
/// fixed nonce buffer contains only the latest reserved public per-record
/// nonce. A reservation borrows this entire state, preventing another sequence
/// value from being consumed while the matching key and nonce are in use.
pub struct RecordProtectionState<'material> {
    material: TrafficKeyMaterial<'material>,
    sequence: RecordNonceSequence,
    nonce: [u8; MAX_TRAFFIC_IV_LENGTH],
}

impl<'material> RecordProtectionState<'material> {
    /// Bind one validated traffic-key view to fresh sequence-zero nonce state.
    ///
    /// IV length and record ceiling are taken only from `material`'s retained
    /// selected suite. There is deliberately no caller-supplied nonce policy.
    pub fn new(
        material: TrafficKeyMaterial<'material>,
    ) -> Result<Self, RecordProtectionStateError> {
        let parameters = material.parameters();
        let policy = NoncePolicy::new(parameters.iv_length(), material.record_limit())
            .map_err(|_| RecordProtectionStateError::UnsupportedSuiteParameters)?;

        Ok(Self {
            material,
            sequence: RecordNonceSequence::new(policy),
            nonce: [0; MAX_TRAFFIC_IV_LENGTH],
        })
    }

    /// Return the exact policy-selected suite retained by this state.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.material.selected_suite()
    }

    /// Return the number of record sequence values already consumed.
    #[must_use]
    pub const fn records_used(&self) -> u64 {
        self.sequence.records_used()
    }

    /// Return the caller-authorized number of reservations still available.
    #[must_use]
    pub const fn remaining_records(&self) -> u64 {
        self.sequence.remaining_records()
    }

    /// Consume the next sequence and return its inseparable key/nonce view.
    ///
    /// The nonce is derived into state-owned fixed storage. On success, the
    /// sequence has already advanced and the returned borrow prevents another
    /// reservation until this one is released. On failure, neither sequence
    /// state nor the retained nonce buffer changes.
    pub fn reserve_next(
        &mut self,
    ) -> Result<RecordProtectionReservation<'_, 'material>, RecordProtectionStateError> {
        let iv_length = self.material.parameters().iv_length();
        let reserved = self
            .sequence
            .reserve_next_nonce(self.material.iv(), &mut self.nonce[..iv_length])
            .map_err(map_nonce_error)?;

        Ok(RecordProtectionReservation {
            state: self,
            reserved,
        })
    }
}

impl fmt::Debug for RecordProtectionState<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RecordProtectionState")
            .field("suite", &self.material.parameters().suite())
            .field("aead", &self.material.aead())
            .field("records_used", &self.records_used())
            .field("record_limit", &self.material.record_limit())
            .field("key_material", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// One consumed sequence bound to its exact traffic key, nonce, and AEAD data.
///
/// This value is intentionally neither `Clone` nor `Copy`. Its state borrow
/// prevents concurrent reservations and keeps secret key bytes in the original
/// caller-owned key-material storage.
pub struct RecordProtectionReservation<'reservation, 'material> {
    state: &'reservation RecordProtectionState<'material>,
    reserved: ReservedNonce,
}

impl RecordProtectionReservation<'_, '_> {
    /// Return the selected suite shared by every value in this reservation.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.state.material.selected_suite()
    }

    /// Return the AEAD routing identifier fixed by the selected suite.
    #[must_use]
    pub const fn aead(&self) -> AeadAlgorithm {
        self.state.material.aead()
    }

    /// Borrow the exact suite-selected traffic key.
    #[must_use]
    pub fn key(&self) -> &[u8] {
        self.state.material.key()
    }

    /// Borrow the nonce derived for this consumed sequence.
    #[must_use]
    pub fn nonce(&self) -> &[u8] {
        let iv_length = self.state.material.parameters().iv_length();
        &self.state.nonce[..iv_length]
    }

    /// Return the selected suite's exact authentication-tag length.
    #[must_use]
    pub const fn tag_length(&self) -> usize {
        self.state.material.parameters().tag_length()
    }

    /// Return the record sequence number already consumed for this nonce.
    #[must_use]
    pub const fn sequence_number(&self) -> u64 {
        self.reserved.sequence_number()
    }
}

impl fmt::Debug for RecordProtectionReservation<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RecordProtectionReservation")
            .field("suite", &self.state.material.parameters().suite())
            .field("aead", &self.aead())
            .field("sequence_number", &self.sequence_number())
            .field("key_length", &self.key().len())
            .field("nonce_length", &self.nonce().len())
            .field("tag_length", &self.tag_length())
            .field("key_material", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

fn map_nonce_error(error: NonceError) -> RecordProtectionStateError {
    match error {
        NonceError::SequenceExhausted => RecordProtectionStateError::SequenceExhausted,
        NonceError::InvalidIvLength | NonceError::InvalidRecordLimit => {
            RecordProtectionStateError::UnsupportedSuiteParameters
        }
        NonceError::StaticIvLengthMismatch | NonceError::OutputLengthMismatch => {
            RecordProtectionStateError::InvariantViolation
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
    use tls13_cipher_suite_policy::{CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry};
    use tls13_traffic_key_material::{derive_traffic_key_material, TrafficKeyMaterialStorage};

    fn decode_hex<const N: usize>(value: &str) -> [u8; N] {
        assert_eq!(value.len(), N * 2);
        let mut decoded = [0; N];
        for (index, byte) in decoded.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
        }
        decoded
    }

    fn selected(suite: CipherSuite, record_limit: u64) -> SelectedCipherSuite {
        let entry = CipherSuitePolicyEntry::new(suite, record_limit).unwrap();
        CipherSuitePolicy::new(&[entry])
            .unwrap()
            .select(suite.wire_id())
            .unwrap()
    }

    #[test]
    fn rfc8448_key_and_iv_remain_bound_to_consumed_sequences() {
        let secret =
            decode_hex::<32>("b3eddb126e067f35a780b3abf45e2d8f3b1a950738f52e9600746a0e27a55a21");
        let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &secret).unwrap();
        let suite = selected(CipherSuite::TlsAes128GcmSha256, 2);
        let mut storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let material = derive_traffic_key_material(&prk, &mut storage).unwrap();
        let mut state = RecordProtectionState::new(material).unwrap();

        {
            let first = state.reserve_next().unwrap();
            assert_eq!(first.sequence_number(), 0);
            assert_eq!(
                first.key(),
                decode_hex::<16>("dbfaa693d1762c5b666af5d950258d01")
            );
            assert_eq!(first.nonce(), decode_hex::<12>("5bd3c71b836e0b76bb73265f"));
            assert_eq!(first.aead(), AeadAlgorithm::Aes128Gcm);
            assert_eq!(first.tag_length(), 16);
            assert_eq!(first.selected_suite(), suite);
        }

        let second = state.reserve_next().unwrap();
        assert_eq!(second.sequence_number(), 1);
        assert_eq!(second.nonce(), decode_hex::<12>("5bd3c71b836e0b76bb73265e"));
    }

    #[test]
    fn exhaustion_consumes_no_additional_sequence() {
        let suite = selected(CipherSuite::TlsChacha20Poly1305Sha256, 1);
        let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0x42; 32]).unwrap();
        let mut storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let material = derive_traffic_key_material(&prk, &mut storage).unwrap();
        let mut state = RecordProtectionState::new(material).unwrap();

        assert_eq!(state.reserve_next().unwrap().sequence_number(), 0);
        assert_eq!(state.records_used(), 1);
        assert_eq!(state.remaining_records(), 0);
        assert_eq!(
            state.reserve_next().unwrap_err(),
            RecordProtectionStateError::SequenceExhausted
        );
        assert_eq!(state.records_used(), 1);
    }

    #[test]
    fn suite_is_the_only_source_of_routing_and_lengths() {
        let cases = [
            (
                CipherSuite::TlsAes128GcmSha256,
                FixedHkdfHash::Sha256,
                AeadAlgorithm::Aes128Gcm,
                16,
            ),
            (
                CipherSuite::TlsAes256GcmSha384,
                FixedHkdfHash::Sha384,
                AeadAlgorithm::Aes256Gcm,
                32,
            ),
            (
                CipherSuite::TlsChacha20Poly1305Sha256,
                FixedHkdfHash::Sha256,
                AeadAlgorithm::Chacha20Poly1305,
                32,
            ),
        ];

        for (suite_id, hash, aead, key_length) in cases {
            let suite = selected(suite_id, 3);
            let secret = [0x5a; 48];
            let hash_length = match hash {
                FixedHkdfHash::Sha256 => 32,
                FixedHkdfHash::Sha384 => 48,
            };
            let prk = HkdfPrk::import_exact(hash, &secret[..hash_length]).unwrap();
            let mut storage = TrafficKeyMaterialStorage::new(suite).unwrap();
            let material = derive_traffic_key_material(&prk, &mut storage).unwrap();
            let mut state = RecordProtectionState::new(material).unwrap();
            let reservation = state.reserve_next().unwrap();

            assert_eq!(reservation.selected_suite(), suite);
            assert_eq!(reservation.aead(), aead);
            assert_eq!(reservation.key().len(), key_length);
            assert_eq!(reservation.nonce().len(), 12);
            assert_eq!(reservation.tag_length(), 16);
        }
    }

    #[test]
    fn diagnostics_reveal_only_public_metadata() {
        let suite = selected(CipherSuite::TlsAes128GcmSha256, 2);
        let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0x42; 32]).unwrap();
        let mut storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let material = derive_traffic_key_material(&prk, &mut storage).unwrap();
        let mut state = RecordProtectionState::new(material).unwrap();

        let state_debug = format!("{state:?}");
        assert!(state_debug.contains("[REDACTED]"));
        assert!(!state_debug.contains("42424242"));

        let reservation_debug = format!("{:?}", state.reserve_next().unwrap());
        assert!(reservation_debug.contains("[REDACTED]"));
        assert!(!reservation_debug.contains("42424242"));
    }

    #[test]
    fn error_categories_are_stable_and_payload_free() {
        let errors = [
            RecordProtectionStateError::UnsupportedSuiteParameters,
            RecordProtectionStateError::SequenceExhausted,
            RecordProtectionStateError::InvariantViolation,
        ];

        for error in errors {
            let debug = format!("{error:?}");
            let display = error.to_string();
            assert!(!debug.contains('['));
            assert!(!display.contains('['));
            assert!(!display.contains("key"));
            assert!(!display.contains("nonce"));
        }
    }
}
