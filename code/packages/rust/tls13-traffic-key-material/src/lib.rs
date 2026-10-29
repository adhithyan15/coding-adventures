//! Cipher-suite-bound fixed-memory TLS 1.3 traffic key and IV derivation.
//!
//! RFC 8446 section 7.3 derives the record-protection key and static IV from
//! one traffic secret using `HKDF-Expand-Label` with the labels `"key"` and
//! `"iv"` and an empty context. This crate binds those operations to one
//! [`SelectedCipherSuite`], preventing detached hash, label, or output-length
//! inputs from reinterpreting the negotiated policy.
//!
//! It is only a key-schedule prerequisite. It does not negotiate a suite,
//! derive traffic secrets, invoke an AEAD, protect records, process a
//! handshake, validate certificates, perform I/O, or provide TLS or HTTPS.

#![deny(unsafe_code)]

use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt;
use tls13_cipher_suite_policy::{
    AeadAlgorithm, CipherSuiteParameters, HkdfHash, SelectedCipherSuite,
};
use tls13_hkdf_label::{hkdf_expand_label_fixed, HkdfExpandLabelError};

/// Largest traffic-key length among the admitted cipher suites.
pub const MAX_TRAFFIC_KEY_LENGTH: usize = 32;

/// Largest static-IV length among the admitted cipher suites.
pub const MAX_TRAFFIC_IV_LENGTH: usize = 12;

const TRAFFIC_KEY_LABEL: &[u8] = b"key";
const TRAFFIC_IV_LABEL: &[u8] = b"iv";
const EMPTY_CONTEXT: &[u8] = b"";

/// Stable failures for traffic key and static-IV derivation.
///
/// Categories retain no traffic-secret, key, or IV bytes and no
/// caller-supplied lengths.
#[derive(Debug, PartialEq, Eq)]
pub enum TrafficKeyMaterialError {
    /// The traffic secret's fixed HKDF hash did not match the selected suite.
    HashMismatch,
    /// A future suite descriptor exceeded this package's reviewed fixed bounds.
    UnsupportedParameters,
    /// Repository-owned key expansion failed.
    KeyDerivation(HkdfExpandLabelError),
    /// Repository-owned IV expansion failed.
    IvDerivation(HkdfExpandLabelError),
}

impl fmt::Display for TrafficKeyMaterialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HashMismatch => formatter
                .write_str("TLS 1.3 traffic secret hash does not match the selected cipher suite"),
            Self::UnsupportedParameters => {
                formatter.write_str("TLS 1.3 cipher-suite parameters exceed reviewed bounds")
            }
            Self::KeyDerivation(error) => {
                write!(formatter, "TLS 1.3 traffic-key derivation failed: {error}")
            }
            Self::IvDerivation(error) => {
                write!(formatter, "TLS 1.3 static-IV derivation failed: {error}")
            }
        }
    }
}

impl std::error::Error for TrafficKeyMaterialError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::KeyDerivation(error) | Self::IvDerivation(error) => Some(error),
            Self::HashMismatch | Self::UnsupportedParameters => None,
        }
    }
}

/// Non-cloneable caller-owned storage for one selected suite's key material.
///
/// Construction stores only public suite policy and zero bytes. Successful
/// [`derive_traffic_key_material`] calls write key material into this storage
/// and return a borrowed [`TrafficKeyMaterial`] view. The storage itself has no
/// secret accessors, so zero or stale bytes cannot be used as validated key
/// material without fresh success evidence. The complete backing arrays are
/// wiped on drop.
pub struct TrafficKeyMaterialStorage {
    selected_suite: SelectedCipherSuite,
    key: Zeroizing<[u8; MAX_TRAFFIC_KEY_LENGTH]>,
    iv: Zeroizing<[u8; MAX_TRAFFIC_IV_LENGTH]>,
}

impl TrafficKeyMaterialStorage {
    /// Allocate fixed zeroed storage for one already-selected suite.
    pub fn new(selected_suite: SelectedCipherSuite) -> Result<Self, TrafficKeyMaterialError> {
        let parameters = selected_suite.parameters();
        if parameters.key_length() > MAX_TRAFFIC_KEY_LENGTH
            || parameters.iv_length() > MAX_TRAFFIC_IV_LENGTH
        {
            return Err(TrafficKeyMaterialError::UnsupportedParameters);
        }

        Ok(Self {
            selected_suite,
            key: Zeroizing::new([0; MAX_TRAFFIC_KEY_LENGTH]),
            iv: Zeroizing::new([0; MAX_TRAFFIC_IV_LENGTH]),
        })
    }

    /// Return the public policy-selected suite retained by this storage.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.selected_suite
    }

    fn clear(&mut self) {
        self.key[..].zeroize();
        self.iv[..].zeroize();
    }
}

impl fmt::Debug for TrafficKeyMaterialStorage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TrafficKeyMaterialStorage")
            .field("suite", &self.selected_suite.parameters().suite())
            .field("key_material", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Success-only borrowed access to one derived traffic key and static IV.
///
/// This value contains only a shared reference. Secret bytes remain in the
/// caller-owned [`TrafficKeyMaterialStorage`] and never cross the ABI return
/// slot by value. The mutable borrow required for another derivation cannot
/// coexist with this view.
pub struct TrafficKeyMaterial<'storage> {
    storage: &'storage TrafficKeyMaterialStorage,
}

impl TrafficKeyMaterial<'_> {
    /// Return the policy-selected suite retained by this key material.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.storage.selected_suite
    }

    /// Return the immutable standard parameters retained by the selected suite.
    #[must_use]
    pub const fn parameters(&self) -> CipherSuiteParameters {
        self.selected_suite().parameters()
    }

    /// Return the AEAD routing identifier retained by the selected suite.
    #[must_use]
    pub const fn aead(&self) -> AeadAlgorithm {
        self.parameters().aead()
    }

    /// Borrow the exact suite-selected traffic-key bytes.
    #[must_use]
    pub fn key(&self) -> &[u8] {
        &self.storage.key[..self.parameters().key_length()]
    }

    /// Borrow the exact suite-selected static-IV bytes.
    #[must_use]
    pub fn iv(&self) -> &[u8] {
        &self.storage.iv[..self.parameters().iv_length()]
    }

    /// Return the caller-authorized maximum record count for this key.
    #[must_use]
    pub const fn record_limit(&self) -> u64 {
        self.selected_suite().record_limit()
    }
}

impl fmt::Debug for TrafficKeyMaterial<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TrafficKeyMaterial")
            .field("suite", &self.parameters().suite())
            .field("aead", &self.aead())
            .field("hash", &self.parameters().hkdf_hash())
            .field("key_length", &self.parameters().key_length())
            .field("iv_length", &self.parameters().iv_length())
            .field("record_limit", &self.record_limit())
            .finish_non_exhaustive()
    }
}

/// Derive one selected suite's record-protection key and static IV.
///
/// The selected suite supplies the sole hash, key length, IV length, labels,
/// and empty-context policy. `traffic_secret` must already be a valid TLS 1.3
/// traffic secret imported into [`HkdfPrk`]; this function does not establish
/// its entropy, provenance, or place in a complete key schedule.
///
/// Existing bytes in `storage` are wiped before validation. The key and IV are
/// then derived into separate wipe-on-drop staging owners. Every exit wipes
/// those original staging slots; success first copies the exact prefixes into
/// caller-owned storage and returns a borrowed view. No secret-bearing value
/// crosses the ABI return slot by value.
///
/// # Example
///
/// ```
/// use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
/// use tls13_cipher_suite_policy::{
///     CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry,
/// };
/// use tls13_traffic_key_material::{
///     derive_traffic_key_material, TrafficKeyMaterialStorage,
/// };
///
/// let entry = CipherSuitePolicyEntry::new(
///     CipherSuite::TlsAes128GcmSha256,
///     1024,
/// )?;
/// let selected = CipherSuitePolicy::new(&[entry])?.select(0x1301)?;
/// let secret = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0x42; 32])?;
/// let mut storage = TrafficKeyMaterialStorage::new(selected)?;
/// let material = derive_traffic_key_material(&secret, &mut storage)?;
/// assert_eq!(material.key().len(), 16);
/// assert_eq!(material.iv().len(), 12);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
// Keep one uniquely named Rust-ABI symbol for the optimized-IR security gate.
// This is not a promised C ABI; it only prevents the release compiler from
// discarding or renaming the boundary that the regression test inspects.
#[allow(unsafe_code)]
#[unsafe(export_name = "coding_adventures_tls13_derive_traffic_key_material_ir_audit")]
pub fn derive_traffic_key_material<'storage>(
    traffic_secret: &HkdfPrk,
    storage: &'storage mut TrafficKeyMaterialStorage,
) -> Result<TrafficKeyMaterial<'storage>, TrafficKeyMaterialError> {
    storage.clear();
    let parameters = storage.selected_suite.parameters();
    let expected_hash = fixed_hash(parameters.hkdf_hash());
    if traffic_secret.algorithm() != expected_hash {
        return Err(TrafficKeyMaterialError::HashMismatch);
    }

    let mut key_staging = Zeroizing::new([0; MAX_TRAFFIC_KEY_LENGTH]);
    let mut iv_staging = Zeroizing::new([0; MAX_TRAFFIC_IV_LENGTH]);

    hkdf_expand_label_fixed(
        traffic_secret,
        TRAFFIC_KEY_LABEL,
        EMPTY_CONTEXT,
        &mut key_staging[..parameters.key_length()],
    )
    .map_err(TrafficKeyMaterialError::KeyDerivation)?;
    hkdf_expand_label_fixed(
        traffic_secret,
        TRAFFIC_IV_LABEL,
        EMPTY_CONTEXT,
        &mut iv_staging[..parameters.iv_length()],
    )
    .map_err(TrafficKeyMaterialError::IvDerivation)?;

    storage.key[..parameters.key_length()].copy_from_slice(&key_staging[..parameters.key_length()]);
    storage.iv[..parameters.iv_length()].copy_from_slice(&iv_staging[..parameters.iv_length()]);

    Ok(TrafficKeyMaterial { storage })
}

const fn fixed_hash(hash: HkdfHash) -> FixedHkdfHash {
    match hash {
        HkdfHash::Sha256 => FixedHkdfHash::Sha256,
        HkdfHash::Sha384 => FixedHkdfHash::Sha384,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tls13_cipher_suite_policy::{CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry};

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
    fn derives_rfc8448_aes128_handshake_key_and_iv() {
        let secret =
            decode_hex::<32>("b3eddb126e067f35a780b3abf45e2d8f3b1a950738f52e9600746a0e27a55a21");
        let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &secret).unwrap();
        let suite = selected(CipherSuite::TlsAes128GcmSha256, 4096);
        let mut storage = TrafficKeyMaterialStorage::new(suite).unwrap();

        let material = derive_traffic_key_material(&prk, &mut storage).unwrap();

        assert_eq!(
            material.key(),
            decode_hex::<16>("dbfaa693d1762c5b666af5d950258d01")
        );
        assert_eq!(material.iv(), decode_hex::<12>("5bd3c71b836e0b76bb73265f"));
        assert_eq!(material.selected_suite(), suite);
        assert_eq!(material.record_limit(), 4096);
        assert_eq!(material.aead(), AeadAlgorithm::Aes128Gcm);
        assert!(storage.key[16..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn derives_independent_aes256_sha384_key_and_iv() {
        let secret = core::array::from_fn::<_, 48, _>(|index| index as u8);
        let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha384, &secret).unwrap();
        let suite = selected(CipherSuite::TlsAes256GcmSha384, 2048);
        let mut storage = TrafficKeyMaterialStorage::new(suite).unwrap();

        let material = derive_traffic_key_material(&prk, &mut storage).unwrap();

        assert_eq!(
            material.key(),
            decode_hex::<32>("6877d022f1c61d24ebb7487c16752d9a4798e40431c75b39320e537c90e23225")
        );
        assert_eq!(material.iv(), decode_hex::<12>("42822531a0fe88648fc09e9f"));
        assert_eq!(material.aead(), AeadAlgorithm::Aes256Gcm);
        assert_eq!(material.parameters().hkdf_hash(), HkdfHash::Sha384);
    }

    #[test]
    fn derives_chacha_key_with_suite_selected_length() {
        let secret =
            decode_hex::<32>("b3eddb126e067f35a780b3abf45e2d8f3b1a950738f52e9600746a0e27a55a21");
        let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &secret).unwrap();
        let suite = selected(CipherSuite::TlsChacha20Poly1305Sha256, 8192);
        let mut storage = TrafficKeyMaterialStorage::new(suite).unwrap();

        let material = derive_traffic_key_material(&prk, &mut storage).unwrap();

        assert_eq!(
            material.key(),
            decode_hex::<32>("73bfffe9212112f34b54106f2be9617a394d95c8f360452bd4ef2be66b9d8392")
        );
        assert_eq!(material.iv(), decode_hex::<12>("5bd3c71b836e0b76bb73265f"));
        assert_eq!(material.aead(), AeadAlgorithm::Chacha20Poly1305);
    }

    #[test]
    fn rejects_suite_and_secret_hash_mismatches() {
        let sha256 = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0x11; 32]).unwrap();
        let sha384 = HkdfPrk::import_exact(FixedHkdfHash::Sha384, &[0x22; 48]).unwrap();
        let mut sha384_storage =
            TrafficKeyMaterialStorage::new(selected(CipherSuite::TlsAes256GcmSha384, 1)).unwrap();
        let mut sha256_storage =
            TrafficKeyMaterialStorage::new(selected(CipherSuite::TlsAes128GcmSha256, 1)).unwrap();

        assert_eq!(
            derive_traffic_key_material(&sha256, &mut sha384_storage).unwrap_err(),
            TrafficKeyMaterialError::HashMismatch
        );
        assert_eq!(
            derive_traffic_key_material(&sha384, &mut sha256_storage).unwrap_err(),
            TrafficKeyMaterialError::HashMismatch
        );
    }

    #[test]
    fn failed_rederivation_wipes_older_key_material() {
        let suite = selected(CipherSuite::TlsAes128GcmSha256, 2);
        let mut storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let matching = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0x33; 32]).unwrap();
        let mismatch = HkdfPrk::import_exact(FixedHkdfHash::Sha384, &[0x44; 48]).unwrap();

        {
            let first = derive_traffic_key_material(&matching, &mut storage).unwrap();
            assert!(first.key().iter().any(|byte| *byte != 0));
            assert!(first.iv().iter().any(|byte| *byte != 0));
        }

        assert_eq!(
            derive_traffic_key_material(&mismatch, &mut storage).unwrap_err(),
            TrafficKeyMaterialError::HashMismatch
        );
        assert!(storage.key.iter().all(|byte| *byte == 0));
        assert!(storage.iv.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn debug_and_errors_disclose_only_public_metadata() {
        let secret = [0xa5; 32];
        let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &secret).unwrap();
        let suite = selected(CipherSuite::TlsAes128GcmSha256, 77);
        let mut storage = TrafficKeyMaterialStorage::new(suite).unwrap();
        let storage_debug = format!("{storage:?}");
        assert!(storage_debug.contains("key_material: \"[REDACTED]\""));
        let material = derive_traffic_key_material(&prk, &mut storage).unwrap();
        let debug = format!("{material:?}");

        assert!(debug.contains("TlsAes128GcmSha256"));
        assert!(debug.contains("record_limit: 77"));
        assert!(!debug.contains(&format!("{:02x?}", secret)));
        assert!(!debug.contains(&format!("{:02x?}", material.key())));
        assert_eq!(
            TrafficKeyMaterialError::HashMismatch.to_string(),
            "TLS 1.3 traffic secret hash does not match the selected cipher suite"
        );
        assert_eq!(
            TrafficKeyMaterialError::UnsupportedParameters.to_string(),
            "TLS 1.3 cipher-suite parameters exceed reviewed bounds"
        );
    }
}
