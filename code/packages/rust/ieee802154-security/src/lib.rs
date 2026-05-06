// lib.rs -- IEEE 802.15.4 security primitives
// ================================================================
//
// This package turns parsed 802.15.4 security metadata into the small pieces
// higher security layers need: nonce construction, replay windows, and key
// lookup. It deliberately does not implement AES-CCM* yet.

#![deny(unsafe_code)]

use ieee802154_core::{
    Address, AuxiliarySecurityHeader, FrameCounter, KeyIdentifier, MacFrame, SecurityLevel,
};
use std::collections::BTreeMap;
use std::fmt;

pub const AES_128_KEY_LEN: usize = 16;
pub const CCM_STAR_NONCE_LEN: usize = 13;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityContext {
    pub source_address: u64,
    pub frame_counter: u32,
    pub security_level: SecurityLevel,
    pub key_identifier: NormalizedKeyIdentifier,
}

impl SecurityContext {
    pub fn nonce(&self) -> CcmStarNonce {
        build_ccm_star_nonce(self.source_address, self.frame_counter, self.security_level)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CcmStarNonce([u8; CCM_STAR_NONCE_LEN]);

impl CcmStarNonce {
    pub fn as_bytes(&self) -> &[u8; CCM_STAR_NONCE_LEN] {
        &self.0
    }
}

pub fn build_ccm_star_nonce(
    source_address: u64,
    frame_counter: u32,
    security_level: SecurityLevel,
) -> CcmStarNonce {
    let mut nonce = [0u8; CCM_STAR_NONCE_LEN];
    nonce[..8].copy_from_slice(&source_address.to_le_bytes());
    nonce[8..12].copy_from_slice(&frame_counter.to_le_bytes());
    nonce[12] = security_level_bits(security_level);
    CcmStarNonce(nonce)
}

pub fn security_context_from_parts(
    source: Address,
    header: &AuxiliarySecurityHeader,
) -> Result<SecurityContext, SecurityError> {
    let source_address = match source {
        Address::Extended(value) => value,
        Address::Short(_) => return Err(SecurityError::ExtendedSourceAddressRequired),
    };

    let frame_counter = match header.frame_counter {
        Some(FrameCounter::Counter32(value)) => value,
        Some(FrameCounter::Counter40(_)) => return Err(SecurityError::UnsupportedFrameCounterSize),
        None => return Err(SecurityError::FrameCounterRequired),
    };

    Ok(SecurityContext {
        source_address,
        frame_counter,
        security_level: header.security_control.security_level,
        key_identifier: NormalizedKeyIdentifier::from_key_identifier(header.key_identifier),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecuredFrameParts {
    pub context: SecurityContext,
    pub nonce: CcmStarNonce,
    pub authenticated_data: Vec<u8>,
    pub encrypted_payload: Vec<u8>,
    pub mic: Vec<u8>,
}

impl SecuredFrameParts {
    pub fn from_frame(frame: &MacFrame) -> Result<Self, SecurityError> {
        let source = frame.source.ok_or(SecurityError::SourceAddressRequired)?;
        let header = frame
            .auxiliary_security_header
            .as_ref()
            .ok_or(SecurityError::AuxiliarySecurityHeaderRequired)?;
        let context = security_context_from_parts(source, header)?;
        let mic_len = context.security_level.mic_len();

        if frame.payload.len() < mic_len {
            return Err(SecurityError::PayloadShorterThanMic {
                payload_len: frame.payload.len(),
                mic_len,
            });
        }

        let encrypted_len = frame.payload.len() - mic_len;
        let encrypted_payload = frame.payload[..encrypted_len].to_vec();
        let mic = frame.payload[encrypted_len..].to_vec();
        let authenticated_data = authenticated_data_without_payload(frame)?;
        let nonce = context.nonce();

        Ok(Self {
            context,
            nonce,
            authenticated_data,
            encrypted_payload,
            mic,
        })
    }

    pub fn has_encrypted_payload(&self) -> bool {
        self.context.security_level.encrypts()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NormalizedKeyIdentifier {
    Implicit,
    KeyIndex(u8),
    KeySource4 { source: [u8; 4], index: u8 },
    KeySource8 { source: [u8; 8], index: u8 },
}

impl NormalizedKeyIdentifier {
    pub fn from_key_identifier(identifier: KeyIdentifier) -> Self {
        match identifier {
            KeyIdentifier::Implicit => Self::Implicit,
            KeyIdentifier::KeyIndex(index) => Self::KeyIndex(index),
            KeyIdentifier::KeySource4 { source, index } => Self::KeySource4 { source, index },
            KeyIdentifier::KeySource8 { source, index } => Self::KeySource8 { source, index },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aes128Key([u8; AES_128_KEY_LEN]);

impl Aes128Key {
    pub fn new(bytes: [u8; AES_128_KEY_LEN]) -> Self {
        Self(bytes)
    }

    pub fn expose_for_crypto(&self) -> &[u8; AES_128_KEY_LEN] {
        &self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryKeyStore {
    keys: BTreeMap<NormalizedKeyIdentifier, Aes128Key>,
}

impl InMemoryKeyStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, identifier: NormalizedKeyIdentifier, key: Aes128Key) {
        self.keys.insert(identifier, key);
    }

    pub fn lookup(&self, identifier: &NormalizedKeyIdentifier) -> Result<Aes128Key, SecurityError> {
        self.keys
            .get(identifier)
            .copied()
            .ok_or(SecurityError::UnknownKeyIdentifier)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayDecision {
    Accept,
    RejectReplay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayWindow {
    highest_seen: Option<u64>,
}

impl ReplayWindow {
    pub fn new() -> Self {
        Self { highest_seen: None }
    }

    pub fn highest_seen(&self) -> Option<u64> {
        self.highest_seen
    }

    pub fn check_and_update(&mut self, frame_counter: u64) -> ReplayDecision {
        match self.highest_seen {
            None => {
                self.highest_seen = Some(frame_counter);
                ReplayDecision::Accept
            }
            Some(highest) if frame_counter > highest => {
                self.highest_seen = Some(frame_counter);
                ReplayDecision::Accept
            }
            Some(_) => ReplayDecision::RejectReplay,
        }
    }
}

impl Default for ReplayWindow {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    SourceAddressRequired,
    AuxiliarySecurityHeaderRequired,
    ExtendedSourceAddressRequired,
    FrameCounterRequired,
    UnsupportedFrameCounterSize,
    PayloadShorterThanMic { payload_len: usize, mic_len: usize },
    UnknownKeyIdentifier,
    FrameEncodingFailed(String),
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceAddressRequired => write!(f, "source address is required"),
            Self::AuxiliarySecurityHeaderRequired => {
                write!(f, "auxiliary security header is required")
            }
            Self::ExtendedSourceAddressRequired => {
                write!(
                    f,
                    "IEEE 802.15.4 security requires an extended source address"
                )
            }
            Self::FrameCounterRequired => {
                write!(f, "frame counter is required for nonce construction")
            }
            Self::UnsupportedFrameCounterSize => {
                write!(f, "40-bit frame counters are not supported yet")
            }
            Self::PayloadShorterThanMic {
                payload_len,
                mic_len,
            } => write!(
                f,
                "payload is shorter than required MIC: payload {payload_len} bytes, MIC {mic_len} bytes"
            ),
            Self::UnknownKeyIdentifier => write!(f, "unknown key identifier"),
            Self::FrameEncodingFailed(message) => write!(f, "frame encoding failed: {message}"),
        }
    }
}

impl std::error::Error for SecurityError {}

fn security_level_bits(security_level: SecurityLevel) -> u8 {
    match security_level {
        SecurityLevel::None => 0,
        SecurityLevel::Mic32 => 1,
        SecurityLevel::Mic64 => 2,
        SecurityLevel::Mic128 => 3,
        SecurityLevel::Enc => 4,
        SecurityLevel::EncMic32 => 5,
        SecurityLevel::EncMic64 => 6,
        SecurityLevel::EncMic128 => 7,
    }
}

fn authenticated_data_without_payload(frame: &MacFrame) -> Result<Vec<u8>, SecurityError> {
    let mut clone = frame.clone();
    clone.payload.clear();
    clone.fcs = None;
    clone
        .encode()
        .map_err(|err| SecurityError::FrameEncodingFailed(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ieee802154_core::{
        AddressMode, FrameControl, FrameType, FrameVersion, KeyIdentifierMode, SecurityControl,
    };

    fn secured_header() -> AuxiliarySecurityHeader {
        AuxiliarySecurityHeader {
            security_control: SecurityControl {
                security_level: SecurityLevel::EncMic32,
                key_identifier_mode: KeyIdentifierMode::KeyIndex,
                frame_counter_suppression: false,
                frame_counter_size_5: false,
            },
            frame_counter: Some(FrameCounter::Counter32(0x0102_0304)),
            key_identifier: KeyIdentifier::KeyIndex(2),
        }
    }

    #[test]
    fn builds_ccm_star_nonce() {
        let nonce =
            build_ccm_star_nonce(0x8877_6655_4433_2211, 0xaabb_ccdd, SecurityLevel::EncMic64);

        assert_eq!(
            nonce.as_bytes(),
            &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0xdd, 0xcc, 0xbb, 0xaa, 0x06,]
        );
    }

    #[test]
    fn builds_context_from_extended_source_and_aux_header() {
        let context = security_context_from_parts(
            Address::Extended(0x8877_6655_4433_2211),
            &secured_header(),
        )
        .unwrap();

        assert_eq!(context.source_address, 0x8877_6655_4433_2211);
        assert_eq!(context.frame_counter, 0x0102_0304);
        assert_eq!(context.security_level, SecurityLevel::EncMic32);
        assert_eq!(context.key_identifier, NormalizedKeyIdentifier::KeyIndex(2));
    }

    #[test]
    fn rejects_short_source_for_nonce_context() {
        assert_eq!(
            security_context_from_parts(Address::Short(0x1234), &secured_header()),
            Err(SecurityError::ExtendedSourceAddressRequired)
        );
    }

    #[test]
    fn rejects_40_bit_frame_counter_until_supported() {
        let mut header = secured_header();
        header.frame_counter = Some(FrameCounter::Counter40(0x0001_0203_0405));

        assert_eq!(
            security_context_from_parts(Address::Extended(1), &header),
            Err(SecurityError::UnsupportedFrameCounterSize)
        );
    }

    #[test]
    fn replay_window_accepts_only_monotonic_counters() {
        let mut window = ReplayWindow::new();

        assert_eq!(window.check_and_update(10), ReplayDecision::Accept);
        assert_eq!(window.check_and_update(10), ReplayDecision::RejectReplay);
        assert_eq!(window.check_and_update(9), ReplayDecision::RejectReplay);
        assert_eq!(window.check_and_update(11), ReplayDecision::Accept);
        assert_eq!(window.highest_seen(), Some(11));
    }

    #[test]
    fn in_memory_key_store_round_trips_keys() {
        let identifier = NormalizedKeyIdentifier::KeySource4 {
            source: [1, 2, 3, 4],
            index: 7,
        };
        let key = Aes128Key::new([0x42; AES_128_KEY_LEN]);
        let mut store = InMemoryKeyStore::new();
        store.insert(identifier.clone(), key);

        assert_eq!(store.lookup(&identifier).unwrap(), key);
        assert_eq!(
            store.lookup(&NormalizedKeyIdentifier::Implicit),
            Err(SecurityError::UnknownKeyIdentifier)
        );
    }

    #[test]
    fn extracts_secured_frame_parts_with_encrypted_payload_and_mic() {
        let frame = MacFrame {
            frame_control: FrameControl {
                frame_type: FrameType::Data,
                security_enabled: true,
                frame_pending: false,
                ack_request: false,
                pan_id_compression: true,
                sequence_number_suppression: false,
                information_elements_present: false,
                destination_address_mode: AddressMode::Short,
                frame_version: FrameVersion::Ieee8021542006,
                source_address_mode: AddressMode::Extended,
            },
            sequence_number: Some(0x7a),
            destination_pan_id: Some(0x1234),
            destination: Some(Address::Short(0x5678)),
            source_pan_id: Some(0x1234),
            source: Some(Address::Extended(0x8877_6655_4433_2211)),
            auxiliary_security_header: Some(secured_header()),
            payload: vec![0xaa, 0xbb, 0xcc, 0xdd, 0x10, 0x11, 0x12, 0x13],
            fcs: Some(0xbeef),
        };

        let parts = SecuredFrameParts::from_frame(&frame).unwrap();

        assert!(parts.has_encrypted_payload());
        assert_eq!(parts.context.source_address, 0x8877_6655_4433_2211);
        assert_eq!(parts.encrypted_payload, vec![0xaa, 0xbb, 0xcc, 0xdd]);
        assert_eq!(parts.mic, vec![0x10, 0x11, 0x12, 0x13]);
        assert_eq!(
            parts.nonce.as_bytes(),
            &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x04, 0x03, 0x02, 0x01, 0x05,]
        );
        assert_eq!(
            parts.authenticated_data,
            vec![
                0x49, 0xd8, // frame control
                0x7a, // sequence number
                0x34, 0x12, // destination PAN id
                0x78, 0x56, // destination short address
                0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, // source extended address
                0x0d, // security control
                0x04, 0x03, 0x02, 0x01, // frame counter
                0x02, // key index
            ]
        );
    }

    #[test]
    fn mic_only_frame_has_authenticated_payload_and_empty_encrypted_payload() {
        let mut header = secured_header();
        header.security_control.security_level = SecurityLevel::Mic64;
        let frame = MacFrame {
            frame_control: FrameControl {
                frame_type: FrameType::Data,
                security_enabled: true,
                frame_pending: false,
                ack_request: false,
                pan_id_compression: true,
                sequence_number_suppression: false,
                information_elements_present: false,
                destination_address_mode: AddressMode::Short,
                frame_version: FrameVersion::Ieee8021542006,
                source_address_mode: AddressMode::Extended,
            },
            sequence_number: Some(1),
            destination_pan_id: Some(0x1234),
            destination: Some(Address::Short(0x5678)),
            source_pan_id: Some(0x1234),
            source: Some(Address::Extended(0x8877_6655_4433_2211)),
            auxiliary_security_header: Some(header),
            payload: vec![0, 1, 2, 3, 4, 5, 6, 7],
            fcs: None,
        };

        let parts = SecuredFrameParts::from_frame(&frame).unwrap();

        assert!(!parts.has_encrypted_payload());
        assert!(parts.encrypted_payload.is_empty());
        assert_eq!(parts.mic, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn rejects_payload_shorter_than_mic() {
        let mut frame = MacFrame {
            frame_control: FrameControl {
                frame_type: FrameType::Data,
                security_enabled: true,
                frame_pending: false,
                ack_request: false,
                pan_id_compression: true,
                sequence_number_suppression: false,
                information_elements_present: false,
                destination_address_mode: AddressMode::Short,
                frame_version: FrameVersion::Ieee8021542006,
                source_address_mode: AddressMode::Extended,
            },
            sequence_number: Some(1),
            destination_pan_id: Some(0x1234),
            destination: Some(Address::Short(0x5678)),
            source_pan_id: Some(0x1234),
            source: Some(Address::Extended(0x8877_6655_4433_2211)),
            auxiliary_security_header: Some(secured_header()),
            payload: vec![1, 2, 3],
            fcs: None,
        };
        frame
            .auxiliary_security_header
            .as_mut()
            .unwrap()
            .security_control
            .security_level = SecurityLevel::EncMic32;

        assert_eq!(
            SecuredFrameParts::from_frame(&frame),
            Err(SecurityError::PayloadShorterThanMic {
                payload_len: 3,
                mic_len: 4,
            })
        );
    }
}
