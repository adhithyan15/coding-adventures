// lib.rs -- IEEE 802.15.4 security primitives
// ================================================================
//
// This package turns parsed 802.15.4 security metadata into the small pieces
// higher security layers need: nonce construction, replay windows, and key
// lookup. It deliberately does not implement AES-CCM* yet.

#![deny(unsafe_code)]

use coding_adventures_aes::encrypt_block;
use coding_adventures_ct_compare::ct_eq;
use ieee802154_core::{
    Address, AuxiliarySecurityHeader, FrameCounter, KeyIdentifier, MacFrame, SecurityLevel,
};
use std::collections::BTreeMap;
use std::fmt;

pub const AES_128_KEY_LEN: usize = 16;
pub const CCM_STAR_NONCE_LEN: usize = 13;
const AES_BLOCK_LEN: usize = 16;
const CCM_L: usize = 2;
const CCM_L_PRIME: u8 = (CCM_L as u8) - 1;

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
    pub fn new(bytes: [u8; CCM_STAR_NONCE_LEN]) -> Self {
        Self(bytes)
    }

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

    pub fn decrypt_and_verify(&self, key: Aes128Key) -> Result<Vec<u8>, SecurityError> {
        ccm_star_decrypt(
            key,
            self.nonce,
            &self.authenticated_data,
            &self.encrypted_payload,
            &self.mic,
            self.context.security_level,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CcmStarOutput {
    pub payload: Vec<u8>,
    pub mic: Vec<u8>,
}

pub fn ccm_star_encrypt(
    key: Aes128Key,
    nonce: CcmStarNonce,
    authenticated_data: &[u8],
    plaintext: &[u8],
    security_level: SecurityLevel,
) -> Result<CcmStarOutput, SecurityError> {
    let mic_len = security_level.mic_len();
    validate_mic_len(mic_len)?;

    let mic = if mic_len == 0 {
        Vec::new()
    } else {
        encrypted_mic(
            key,
            nonce,
            authenticated_data,
            plaintext,
            security_level,
            mic_len,
        )?
    };

    let payload = if security_level.encrypts() {
        ctr_crypt(key, nonce, plaintext)?
    } else {
        plaintext.to_vec()
    };

    Ok(CcmStarOutput { payload, mic })
}

pub fn ccm_star_decrypt(
    key: Aes128Key,
    nonce: CcmStarNonce,
    authenticated_data: &[u8],
    encrypted_payload: &[u8],
    mic: &[u8],
    security_level: SecurityLevel,
) -> Result<Vec<u8>, SecurityError> {
    let mic_len = security_level.mic_len();
    validate_mic_len(mic_len)?;
    if mic.len() != mic_len {
        return Err(SecurityError::InvalidMicLength {
            expected: mic_len,
            actual: mic.len(),
        });
    }

    let plaintext = if security_level.encrypts() {
        ctr_crypt(key, nonce, encrypted_payload)?
    } else {
        encrypted_payload.to_vec()
    };

    if mic_len != 0 {
        let expected = encrypted_mic(
            key,
            nonce,
            authenticated_data,
            &plaintext,
            security_level,
            mic_len,
        )?;
        if !ct_eq(&expected, mic) {
            return Err(SecurityError::AuthenticationFailed);
        }
    }

    Ok(plaintext)
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutgoingFrameCounterKey {
    pub source_address: u64,
    pub key_identifier: NormalizedKeyIdentifier,
}

impl OutgoingFrameCounterKey {
    pub fn new(source_address: u64, key_identifier: NormalizedKeyIdentifier) -> Self {
        Self {
            source_address,
            key_identifier,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingFrameCounterLease {
    pub source_address: u64,
    pub key_identifier: NormalizedKeyIdentifier,
    pub frame_counter: u32,
}

impl OutgoingFrameCounterLease {
    pub fn frame_counter(&self) -> FrameCounter {
        FrameCounter::Counter32(self.frame_counter)
    }

    pub fn security_context(&self, security_level: SecurityLevel) -> SecurityContext {
        SecurityContext {
            source_address: self.source_address,
            frame_counter: self.frame_counter,
            security_level,
            key_identifier: self.key_identifier.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OutgoingFrameCounterStore {
    next_by_key: BTreeMap<OutgoingFrameCounterKey, u64>,
}

impl OutgoingFrameCounterStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn restore_next_counter(&mut self, key: OutgoingFrameCounterKey, next_counter: u32) {
        self.next_by_key.insert(key, next_counter as u64);
    }

    pub fn peek_next_counter(&self, key: &OutgoingFrameCounterKey) -> Option<u32> {
        let next = self.next_by_key.get(key).copied().unwrap_or(0);
        u32::try_from(next).ok()
    }

    pub fn reserve_next(
        &mut self,
        source_address: u64,
        key_identifier: NormalizedKeyIdentifier,
    ) -> Result<OutgoingFrameCounterLease, SecurityError> {
        self.reserve_next_for_key(OutgoingFrameCounterKey::new(source_address, key_identifier))
    }

    pub fn reserve_next_for_key(
        &mut self,
        key: OutgoingFrameCounterKey,
    ) -> Result<OutgoingFrameCounterLease, SecurityError> {
        let next = self.next_by_key.get(&key).copied().unwrap_or(0);
        let frame_counter =
            u32::try_from(next).map_err(|_| SecurityError::OutgoingFrameCounterExhausted)?;
        self.next_by_key.insert(key.clone(), next + 1);

        Ok(OutgoingFrameCounterLease {
            source_address: key.source_address,
            key_identifier: key.key_identifier,
            frame_counter,
        })
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
    InvalidMicLength { expected: usize, actual: usize },
    UnsupportedMicLength(usize),
    MessageTooLong { len: usize },
    AuthenticatedDataTooLong { len: usize },
    CryptoFailed(String),
    AuthenticationFailed,
    OutgoingFrameCounterExhausted,
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
            Self::InvalidMicLength { expected, actual } => {
                write!(f, "invalid MIC length: expected {expected}, got {actual}")
            }
            Self::UnsupportedMicLength(len) => write!(f, "unsupported MIC length: {len}"),
            Self::MessageTooLong { len } => write!(f, "message is too long for L=2 CCM*: {len}"),
            Self::AuthenticatedDataTooLong { len } => {
                write!(f, "authenticated data is too long for this encoder: {len}")
            }
            Self::CryptoFailed(message) => write!(f, "crypto operation failed: {message}"),
            Self::AuthenticationFailed => write!(f, "authentication failed"),
            Self::OutgoingFrameCounterExhausted => {
                write!(f, "outgoing frame counter space is exhausted")
            }
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

fn encrypted_mic(
    key: Aes128Key,
    nonce: CcmStarNonce,
    authenticated_data: &[u8],
    plaintext: &[u8],
    security_level: SecurityLevel,
    mic_len: usize,
) -> Result<Vec<u8>, SecurityError> {
    let tag = cbc_mac(
        key,
        nonce,
        authenticated_data,
        plaintext,
        security_level,
        mic_len,
    )?;
    let s0 = encrypt_block(&counter_block(nonce, 0), key.expose_for_crypto())
        .map_err(SecurityError::CryptoFailed)?;
    Ok(tag
        .iter()
        .zip(s0.iter())
        .map(|(tag_byte, stream_byte)| tag_byte ^ stream_byte)
        .collect())
}

fn cbc_mac(
    key: Aes128Key,
    nonce: CcmStarNonce,
    authenticated_data: &[u8],
    plaintext: &[u8],
    security_level: SecurityLevel,
    mic_len: usize,
) -> Result<Vec<u8>, SecurityError> {
    if plaintext.len() > u16::MAX as usize {
        return Err(SecurityError::MessageTooLong {
            len: plaintext.len(),
        });
    }

    let mut x = [0u8; AES_BLOCK_LEN];
    for block in ccm_mac_blocks(
        nonce,
        authenticated_data,
        plaintext,
        security_level,
        mic_len,
    )? {
        let xored = xor_block(&x, &block);
        x = encrypt_block(&xored, key.expose_for_crypto()).map_err(SecurityError::CryptoFailed)?;
    }

    Ok(x[..mic_len].to_vec())
}

fn ccm_mac_blocks(
    nonce: CcmStarNonce,
    authenticated_data: &[u8],
    plaintext: &[u8],
    security_level: SecurityLevel,
    mic_len: usize,
) -> Result<Vec<[u8; AES_BLOCK_LEN]>, SecurityError> {
    let mut blocks = Vec::new();
    blocks.push(b0_block(
        nonce,
        !authenticated_data.is_empty(),
        plaintext.len(),
        security_level,
        mic_len,
    )?);

    if !authenticated_data.is_empty() {
        blocks.extend(padded_blocks(&encode_authenticated_data(
            authenticated_data,
        )?));
    }

    if !plaintext.is_empty() {
        blocks.extend(padded_blocks(plaintext));
    }

    Ok(blocks)
}

fn b0_block(
    nonce: CcmStarNonce,
    has_authenticated_data: bool,
    message_len: usize,
    security_level: SecurityLevel,
    mic_len: usize,
) -> Result<[u8; AES_BLOCK_LEN], SecurityError> {
    if message_len > u16::MAX as usize {
        return Err(SecurityError::MessageTooLong { len: message_len });
    }

    let mut block = [0u8; AES_BLOCK_LEN];
    block[0] = b0_flags(has_authenticated_data, security_level, mic_len)?;
    block[1..14].copy_from_slice(nonce.as_bytes());
    block[14..16].copy_from_slice(&(message_len as u16).to_be_bytes());
    Ok(block)
}

fn b0_flags(
    has_authenticated_data: bool,
    security_level: SecurityLevel,
    mic_len: usize,
) -> Result<u8, SecurityError> {
    validate_mic_len(mic_len)?;
    let adata = if has_authenticated_data { 0x40 } else { 0x00 };
    let mic_bits = if mic_len == 0 {
        0
    } else {
        (((mic_len - 2) / 2) as u8) << 3
    };
    Ok(adata | mic_bits | CCM_L_PRIME | ccm_star_auth_only_bit(security_level))
}

fn ccm_star_auth_only_bit(_security_level: SecurityLevel) -> u8 {
    0
}

fn encode_authenticated_data(authenticated_data: &[u8]) -> Result<Vec<u8>, SecurityError> {
    if authenticated_data.len() >= 0xff00 {
        return Err(SecurityError::AuthenticatedDataTooLong {
            len: authenticated_data.len(),
        });
    }

    let mut encoded = Vec::with_capacity(2 + authenticated_data.len());
    encoded.extend_from_slice(&(authenticated_data.len() as u16).to_be_bytes());
    encoded.extend_from_slice(authenticated_data);
    Ok(encoded)
}

fn padded_blocks(bytes: &[u8]) -> Vec<[u8; AES_BLOCK_LEN]> {
    let mut blocks = Vec::new();
    for chunk in bytes.chunks(AES_BLOCK_LEN) {
        let mut block = [0u8; AES_BLOCK_LEN];
        block[..chunk.len()].copy_from_slice(chunk);
        blocks.push(block);
    }
    blocks
}

fn ctr_crypt(key: Aes128Key, nonce: CcmStarNonce, input: &[u8]) -> Result<Vec<u8>, SecurityError> {
    let mut output = Vec::with_capacity(input.len());
    for (block_index, chunk) in input.chunks(AES_BLOCK_LEN).enumerate() {
        let counter = (block_index + 1) as u16;
        let stream = encrypt_block(&counter_block(nonce, counter), key.expose_for_crypto())
            .map_err(SecurityError::CryptoFailed)?;
        for (i, byte) in chunk.iter().enumerate() {
            output.push(byte ^ stream[i]);
        }
    }
    Ok(output)
}

fn counter_block(nonce: CcmStarNonce, counter: u16) -> [u8; AES_BLOCK_LEN] {
    let mut block = [0u8; AES_BLOCK_LEN];
    block[0] = CCM_L_PRIME;
    block[1..14].copy_from_slice(nonce.as_bytes());
    block[14..16].copy_from_slice(&counter.to_be_bytes());
    block
}

fn validate_mic_len(mic_len: usize) -> Result<(), SecurityError> {
    match mic_len {
        0 | 4 | 8 | 16 => Ok(()),
        other => Err(SecurityError::UnsupportedMicLength(other)),
    }
}

fn xor_block(left: &[u8; AES_BLOCK_LEN], right: &[u8; AES_BLOCK_LEN]) -> [u8; AES_BLOCK_LEN] {
    let mut out = [0u8; AES_BLOCK_LEN];
    for i in 0..AES_BLOCK_LEN {
        out[i] = left[i] ^ right[i];
    }
    out
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
    fn outgoing_frame_counter_store_reserves_monotonic_leases_per_key() {
        let mut store = OutgoingFrameCounterStore::new();
        let source_address = 0x8877_6655_4433_2211;
        let first_key = NormalizedKeyIdentifier::KeyIndex(1);
        let second_key = NormalizedKeyIdentifier::KeyIndex(2);

        let first = store
            .reserve_next(source_address, first_key.clone())
            .unwrap();
        let second = store
            .reserve_next(source_address, first_key.clone())
            .unwrap();
        let other_key_first = store
            .reserve_next(source_address, second_key.clone())
            .unwrap();
        let other_source_first = store
            .reserve_next(0x0102_0304_0506_0708, first_key.clone())
            .unwrap();

        assert_eq!(first.frame_counter, 0);
        assert_eq!(first.frame_counter(), FrameCounter::Counter32(0));
        assert_eq!(second.frame_counter, 1);
        assert_eq!(other_key_first.frame_counter, 0);
        assert_eq!(other_source_first.frame_counter, 0);
        assert_eq!(
            store.peek_next_counter(&OutgoingFrameCounterKey::new(source_address, first_key)),
            Some(2)
        );

        let context = second.security_context(SecurityLevel::EncMic32);
        assert_eq!(context.frame_counter, 1);
        assert_eq!(context.key_identifier, NormalizedKeyIdentifier::KeyIndex(1));
    }

    #[test]
    fn outgoing_frame_counter_store_restores_persisted_next_counter() {
        let mut store = OutgoingFrameCounterStore::new();
        let key = OutgoingFrameCounterKey::new(
            0x8877_6655_4433_2211,
            NormalizedKeyIdentifier::KeySource4 {
                source: [1, 2, 3, 4],
                index: 7,
            },
        );
        store.restore_next_counter(key.clone(), 41);

        let lease = store.reserve_next_for_key(key.clone()).unwrap();

        assert_eq!(lease.frame_counter, 41);
        assert_eq!(store.peek_next_counter(&key), Some(42));
    }

    #[test]
    fn outgoing_frame_counter_store_rejects_after_counter_space_is_exhausted() {
        let mut store = OutgoingFrameCounterStore::new();
        let key = OutgoingFrameCounterKey::new(1, NormalizedKeyIdentifier::Implicit);
        store.restore_next_counter(key.clone(), u32::MAX);

        let final_lease = store.reserve_next_for_key(key.clone()).unwrap();

        assert_eq!(final_lease.frame_counter, u32::MAX);
        assert_eq!(store.peek_next_counter(&key), None);
        assert_eq!(
            store.reserve_next_for_key(key),
            Err(SecurityError::OutgoingFrameCounterExhausted)
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

    #[test]
    fn ccm_star_encrypts_rfc3610_packet_vector_1() {
        let key = Aes128Key::new([
            0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xcb, 0xcc, 0xcd,
            0xce, 0xcf,
        ]);
        let nonce = CcmStarNonce::new([
            0x00, 0x00, 0x00, 0x03, 0x02, 0x01, 0x00, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5,
        ]);
        let authenticated_data = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        let plaintext = [
            0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15,
            0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        ];

        let output = ccm_star_encrypt(
            key,
            nonce,
            &authenticated_data,
            &plaintext,
            SecurityLevel::EncMic64,
        )
        .unwrap();

        assert_eq!(
            output.payload,
            vec![
                0x58, 0x8c, 0x97, 0x9a, 0x61, 0xc6, 0x63, 0xd2, 0xf0, 0x66, 0xd0, 0xc2, 0xc0, 0xf9,
                0x89, 0x80, 0x6d, 0x5f, 0x6b, 0x61, 0xda, 0xc3, 0x84,
            ]
        );
        assert_eq!(
            output.mic,
            vec![0x17, 0xe8, 0xd1, 0x2c, 0xfd, 0xf9, 0x26, 0xe0]
        );
    }

    #[test]
    fn ccm_star_decrypts_rfc3610_packet_vector_1() {
        let key = Aes128Key::new([
            0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xcb, 0xcc, 0xcd,
            0xce, 0xcf,
        ]);
        let nonce = CcmStarNonce::new([
            0x00, 0x00, 0x00, 0x03, 0x02, 0x01, 0x00, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5,
        ]);
        let authenticated_data = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        let ciphertext = [
            0x58, 0x8c, 0x97, 0x9a, 0x61, 0xc6, 0x63, 0xd2, 0xf0, 0x66, 0xd0, 0xc2, 0xc0, 0xf9,
            0x89, 0x80, 0x6d, 0x5f, 0x6b, 0x61, 0xda, 0xc3, 0x84,
        ];
        let mic = [0x17, 0xe8, 0xd1, 0x2c, 0xfd, 0xf9, 0x26, 0xe0];

        let plaintext = ccm_star_decrypt(
            key,
            nonce,
            &authenticated_data,
            &ciphertext,
            &mic,
            SecurityLevel::EncMic64,
        )
        .unwrap();

        assert_eq!(
            plaintext,
            vec![
                0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15,
                0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
            ]
        );
    }

    #[test]
    fn ccm_star_rejects_modified_mic() {
        let key = Aes128Key::new([0; AES_128_KEY_LEN]);
        let nonce = CcmStarNonce::new([0; CCM_STAR_NONCE_LEN]);
        let encrypted =
            ccm_star_encrypt(key, nonce, b"header", b"payload", SecurityLevel::EncMic32).unwrap();
        let mut bad_mic = encrypted.mic.clone();
        bad_mic[0] ^= 0x01;

        assert_eq!(
            ccm_star_decrypt(
                key,
                nonce,
                b"header",
                &encrypted.payload,
                &bad_mic,
                SecurityLevel::EncMic32,
            ),
            Err(SecurityError::AuthenticationFailed)
        );
    }
}
