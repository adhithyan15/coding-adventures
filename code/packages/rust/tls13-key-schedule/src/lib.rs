//! Suite-bound fixed-memory TLS 1.3 base-secret schedule.
//!
//! This crate composes only the no-PSK RFC 8446 section 7.1 transitions for a
//! full ephemeral key exchange. It does not perform key exchange, derive
//! traffic secrets, process a handshake, protect records, validate
//! certificates, perform I/O, or provide TLS or HTTPS.

#![deny(unsafe_code)]

use coding_adventures_hkdf::{hkdf_extract_fixed, FixedHkdfHash};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt;
use tls13_cipher_suite_policy::{HkdfHash, SelectedCipherSuite};
use tls13_hkdf_label::derive_secret_from_transcript_hash_fixed;

/// Largest hash output admitted by the selected TLS 1.3 cipher suites.
pub const MAX_SECRET_LENGTH: usize = 48;

/// Closed upper bound for the borrowed ephemeral shared secret.
///
/// This admits the standard TLS 1.3 finite-field and elliptic-curve shared
/// secret sizes through FFDHE8192 while preventing a detached caller from
/// turning one schedule operation into unbounded HMAC work. Exact group and
/// encoding policy remains caller-owned.
pub const MAX_SHARED_SECRET_LENGTH: usize = 1024;

const DERIVED_LABEL: &[u8] = b"derived";
const SHA256_EMPTY_HASH: [u8; 32] = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
];
const SHA384_EMPTY_HASH: [u8; 48] = [
    0x38, 0xb0, 0x60, 0xa7, 0x51, 0xac, 0x96, 0x38, 0x4c, 0xd9, 0x32, 0x7e, 0xb1, 0xb1, 0xe3, 0x6a,
    0x21, 0xfd, 0xb7, 0x11, 0x14, 0xbe, 0x07, 0x43, 0x4c, 0x0c, 0xc7, 0xbf, 0x63, 0xf6, 0xe1, 0xda,
    0x27, 0x4e, 0xde, 0xbf, 0xe7, 0x6f, 0x65, 0xfb, 0xd5, 0x1a, 0xd2, 0xf1, 0x48, 0x98, 0xb9, 0x5b,
];

/// Stable payload-free failures for the bounded base-secret schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyScheduleError {
    /// The shared secret must contain at least one byte.
    EmptySharedSecret,
    /// The shared secret exceeded the reviewed work bound.
    SharedSecretTooLong,
    /// A future suite descriptor exceeded this package's fixed storage.
    UnsupportedParameters,
    /// A repository-owned HKDF or Derive-Secret invariant failed.
    DerivationFailed,
}

impl fmt::Display for KeyScheduleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptySharedSecret => "TLS 1.3 shared secret is empty",
            Self::SharedSecretTooLong => "TLS 1.3 shared secret exceeds the reviewed bound",
            Self::UnsupportedParameters => "TLS 1.3 cipher-suite parameters exceed reviewed bounds",
            Self::DerivationFailed => "TLS 1.3 base-secret derivation failed",
        })
    }
}

impl std::error::Error for KeyScheduleError {}

/// Non-cloneable caller-owned storage for one suite's base secrets.
///
/// Construction stores only public suite policy and zero bytes. Successful
/// derivation returns a borrowed [`BaseSecrets`] view; without that success
/// evidence this owner exposes no secret accessors. All backing storage is
/// wiped on drop.
pub struct BaseSecretStorage {
    selected_suite: SelectedCipherSuite,
    early_secret: Zeroizing<[u8; MAX_SECRET_LENGTH]>,
    handshake_secret: Zeroizing<[u8; MAX_SECRET_LENGTH]>,
    master_secret: Zeroizing<[u8; MAX_SECRET_LENGTH]>,
}

impl BaseSecretStorage {
    /// Allocate zeroed storage bound to one policy-selected suite.
    pub fn new(selected_suite: SelectedCipherSuite) -> Result<Self, KeyScheduleError> {
        if fixed_hash(selected_suite.parameters().hkdf_hash()).hash_len() > MAX_SECRET_LENGTH {
            return Err(KeyScheduleError::UnsupportedParameters);
        }
        Ok(Self {
            selected_suite,
            early_secret: Zeroizing::new([0; MAX_SECRET_LENGTH]),
            handshake_secret: Zeroizing::new([0; MAX_SECRET_LENGTH]),
            master_secret: Zeroizing::new([0; MAX_SECRET_LENGTH]),
        })
    }

    /// Return the public selected suite retained by this storage.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.selected_suite
    }

    fn clear(&mut self) {
        self.early_secret[..].zeroize();
        self.handshake_secret[..].zeroize();
        self.master_secret[..].zeroize();
    }
}

impl fmt::Debug for BaseSecretStorage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BaseSecretStorage")
            .field("suite", &self.selected_suite.parameters().suite())
            .field("base_secrets", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Success-only borrowed access to one suite-bound base-secret schedule.
pub struct BaseSecrets<'storage> {
    storage: &'storage BaseSecretStorage,
}

impl BaseSecrets<'_> {
    /// Return the policy-selected suite retained by this schedule.
    #[must_use]
    pub const fn selected_suite(&self) -> SelectedCipherSuite {
        self.storage.selected_suite
    }

    /// Borrow the exact Hash-length Early Secret.
    #[must_use]
    pub fn early_secret(&self) -> &[u8] {
        &self.storage.early_secret[..self.hash_len()]
    }

    /// Borrow the exact Hash-length Handshake Secret.
    #[must_use]
    pub fn handshake_secret(&self) -> &[u8] {
        &self.storage.handshake_secret[..self.hash_len()]
    }

    /// Borrow the exact Hash-length Master Secret.
    #[must_use]
    pub fn master_secret(&self) -> &[u8] {
        &self.storage.master_secret[..self.hash_len()]
    }

    fn hash_len(&self) -> usize {
        fixed_hash(self.selected_suite().parameters().hkdf_hash()).hash_len()
    }
}

impl fmt::Debug for BaseSecrets<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BaseSecrets")
            .field("suite", &self.selected_suite().parameters().suite())
            .field("hash", &self.selected_suite().parameters().hkdf_hash())
            .field("secret_length", &self.hash_len())
            .finish_non_exhaustive()
    }
}

/// Derive the no-PSK TLS 1.3 Early, Handshake, and Master Secrets.
///
/// The selected suite supplies the sole SHA-256 or SHA-384 policy. The caller
/// supplies a non-empty borrowed shared secret produced by a separately
/// reviewed ephemeral key-exchange boundary. This function does not establish
/// that input's entropy, provenance, exact group encoding, or peer binding;
/// later certificate and Finished processing must authenticate the handshake.
///
/// Existing storage is cleared before validation. All three outputs are staged
/// in wipe-on-drop owners and copied into caller storage only after the complete
/// schedule succeeds, then a borrowed success view is returned.
pub fn derive_ephemeral_base_secrets<'storage>(
    shared_secret: &[u8],
    storage: &'storage mut BaseSecretStorage,
) -> Result<BaseSecrets<'storage>, KeyScheduleError> {
    storage.clear();
    if shared_secret.is_empty() {
        return Err(KeyScheduleError::EmptySharedSecret);
    }
    if shared_secret.len() > MAX_SHARED_SECRET_LENGTH {
        return Err(KeyScheduleError::SharedSecretTooLong);
    }

    let hash = fixed_hash(storage.selected_suite.parameters().hkdf_hash());
    let hash_len = hash.hash_len();
    if hash_len > MAX_SECRET_LENGTH {
        return Err(KeyScheduleError::UnsupportedParameters);
    }
    let zeros = [0u8; MAX_SECRET_LENGTH];
    let empty_hash = empty_hash(hash);

    let early = hkdf_extract_fixed(&[], &zeros[..hash_len], hash)
        .map_err(|_| KeyScheduleError::DerivationFailed)?;
    let mut derived = Zeroizing::new([0u8; MAX_SECRET_LENGTH]);
    derive_secret_from_transcript_hash_fixed(
        &early,
        DERIVED_LABEL,
        empty_hash,
        &mut derived[..hash_len],
    )
    .map_err(|_| KeyScheduleError::DerivationFailed)?;

    let handshake = hkdf_extract_fixed(&derived[..hash_len], shared_secret, hash)
        .map_err(|_| KeyScheduleError::DerivationFailed)?;
    derived[..].zeroize();
    derive_secret_from_transcript_hash_fixed(
        &handshake,
        DERIVED_LABEL,
        empty_hash,
        &mut derived[..hash_len],
    )
    .map_err(|_| KeyScheduleError::DerivationFailed)?;

    let master = hkdf_extract_fixed(&derived[..hash_len], &zeros[..hash_len], hash)
        .map_err(|_| KeyScheduleError::DerivationFailed)?;

    storage.early_secret[..hash_len].copy_from_slice(early.as_bytes());
    storage.handshake_secret[..hash_len].copy_from_slice(handshake.as_bytes());
    storage.master_secret[..hash_len].copy_from_slice(master.as_bytes());

    Ok(BaseSecrets { storage })
}

const fn fixed_hash(hash: HkdfHash) -> FixedHkdfHash {
    match hash {
        HkdfHash::Sha256 => FixedHkdfHash::Sha256,
        HkdfHash::Sha384 => FixedHkdfHash::Sha384,
    }
}

fn empty_hash(hash: FixedHkdfHash) -> &'static [u8] {
    match hash {
        FixedHkdfHash::Sha256 => &SHA256_EMPTY_HASH,
        FixedHkdfHash::Sha384 => &SHA384_EMPTY_HASH,
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

    fn selected(suite: CipherSuite) -> SelectedCipherSuite {
        let entry = CipherSuitePolicyEntry::new(suite, 4096).unwrap();
        CipherSuitePolicy::new(&[entry])
            .unwrap()
            .select(suite.wire_id())
            .unwrap()
    }

    #[test]
    fn derives_rfc8448_sha256_base_secrets() {
        let shared_secret =
            decode_hex::<32>("8bd4054fb55b9d63fdfbacf9f04b9f0d35e6d63f537563efd46272900f89492d");
        let mut storage =
            BaseSecretStorage::new(selected(CipherSuite::TlsAes128GcmSha256)).unwrap();

        let secrets = derive_ephemeral_base_secrets(&shared_secret, &mut storage).unwrap();

        assert_eq!(
            secrets.early_secret(),
            decode_hex::<32>("33ad0a1c607ec03b09e6cd9893680ce210adf300aa1f2660e1b22e10f170f92a")
        );
        assert_eq!(
            secrets.handshake_secret(),
            decode_hex::<32>("1dc826e93606aa6fdc0aadc12f741b01046aa6b99f691ed221a9f0ca043fbeac")
        );
        assert_eq!(
            secrets.master_secret(),
            decode_hex::<32>("18df06843d13a08bf2a449844c5f8a478001bc4d4c627984d5a41da8d0402919")
        );
    }

    #[test]
    fn binds_same_sha256_schedule_to_chacha_suite() {
        let shared_secret = [0x42; 32];
        let suite = selected(CipherSuite::TlsChacha20Poly1305Sha256);
        let mut storage = BaseSecretStorage::new(suite).unwrap();
        let secrets = derive_ephemeral_base_secrets(&shared_secret, &mut storage).unwrap();

        assert_eq!(secrets.selected_suite(), suite);
        assert_eq!(secrets.early_secret().len(), 32);
        assert_eq!(secrets.master_secret().len(), 32);
    }

    #[test]
    fn derives_sha384_base_secrets() {
        let shared_secret = core::array::from_fn::<_, 48, _>(|index| index as u8);
        let mut storage =
            BaseSecretStorage::new(selected(CipherSuite::TlsAes256GcmSha384)).unwrap();
        let secrets = derive_ephemeral_base_secrets(&shared_secret, &mut storage).unwrap();

        // Independently generated with Python's standard-library SHA-384 and
        // HMAC implementations from the RFC 8446 section 7.1 construction.
        assert_eq!(
            secrets.early_secret(),
            decode_hex::<48>(
                "7ee8206f5570023e6dc7519eb1073bc4e791ad37b5c382aa10ba18e2357e7169\
                 71f9362f2c2fe2a76bfd78dfec4ea9b5"
            )
        );
        assert_eq!(
            secrets.handshake_secret(),
            decode_hex::<48>(
                "deb1becd84738bdcbf73bdd28bd2209d2fe51b84986a4fbb63d7e3902c00a7d8\
                 67db2d8e1af869b952dcab5d7f482cab"
            )
        );
        assert_eq!(
            secrets.master_secret(),
            decode_hex::<48>(
                "78f71f66fe0d83f65f92e4665b4e2acccb43632e32ebf128e19b5d95c60d499\
                 0721065aea51bb55a9eb9f81cd7cbf3e1"
            )
        );
    }

    #[test]
    fn rejects_invalid_shared_secret_bounds_and_clears_older_secrets() {
        let mut storage =
            BaseSecretStorage::new(selected(CipherSuite::TlsAes128GcmSha256)).unwrap();
        {
            let secrets = derive_ephemeral_base_secrets(&[0x55; 32], &mut storage).unwrap();
            assert!(secrets.master_secret().iter().any(|byte| *byte != 0));
        }

        assert_eq!(
            derive_ephemeral_base_secrets(&[], &mut storage).unwrap_err(),
            KeyScheduleError::EmptySharedSecret
        );
        assert!(storage.early_secret.iter().all(|byte| *byte == 0));
        assert!(storage.handshake_secret.iter().all(|byte| *byte == 0));
        assert!(storage.master_secret.iter().all(|byte| *byte == 0));

        let too_long = [0x77; MAX_SHARED_SECRET_LENGTH + 1];
        assert_eq!(
            derive_ephemeral_base_secrets(&too_long, &mut storage).unwrap_err(),
            KeyScheduleError::SharedSecretTooLong
        );
    }

    #[test]
    fn accepts_shared_secret_at_closed_upper_bound() {
        let shared_secret = [0xa5; MAX_SHARED_SECRET_LENGTH];
        let mut storage =
            BaseSecretStorage::new(selected(CipherSuite::TlsAes128GcmSha256)).unwrap();
        let secrets = derive_ephemeral_base_secrets(&shared_secret, &mut storage).unwrap();
        assert_eq!(secrets.master_secret().len(), 32);
    }

    #[test]
    fn debug_and_errors_disclose_only_public_metadata() {
        let suite = selected(CipherSuite::TlsAes256GcmSha384);
        let mut storage = BaseSecretStorage::new(suite).unwrap();
        let storage_debug = format!("{storage:?}");
        assert!(storage_debug.contains("base_secrets: \"[REDACTED]\""));
        let secrets = derive_ephemeral_base_secrets(&[0x9b; 48], &mut storage).unwrap();
        let debug = format!("{secrets:?}");

        assert!(debug.contains("TlsAes256GcmSha384"));
        assert!(debug.contains("secret_length: 48"));
        assert!(!debug.contains("9b"));
        assert_eq!(
            KeyScheduleError::SharedSecretTooLong.to_string(),
            "TLS 1.3 shared secret exceeds the reviewed bound"
        );
        assert_eq!(
            KeyScheduleError::DerivationFailed.to_string(),
            "TLS 1.3 base-secret derivation failed"
        );
    }
}
