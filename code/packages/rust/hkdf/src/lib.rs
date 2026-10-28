//! # coding_adventures_hkdf
//!
//! HKDF (HMAC-based Extract-and-Expand Key Derivation Function) — RFC 5869.
//!
//! ## What Is HKDF?
//!
//! HKDF derives one or more cryptographically strong keys from a single piece of
//! input keying material (IKM). It is the standard key derivation function used in
//! TLS 1.3, Signal Protocol, WireGuard, and many other modern protocols.
//!
//! ## Why Do We Need Key Derivation?
//!
//! Raw key material — from a Diffie-Hellman exchange, a password, or a random
//! source — is not always suitable for direct use as a cryptographic key. It may
//! have non-uniform distribution, wrong length, or insufficient entropy
//! concentration. HKDF solves all three through a two-phase approach.
//!
//! ## Phase 1: Extract
//!
//! Extract takes the raw IKM and concentrates its entropy into a fixed-length
//! pseudorandom key (PRK):
//!
//! ```text
//! PRK = HMAC-Hash(salt, IKM)
//!
//! +------+     +------+
//! | salt |---->|      |
//! +------+     | HMAC |----> PRK (HashLen bytes)
//! | IKM  |---->|      |
//! +------+     +------+
//! ```
//!
//! The salt is the HMAC *key* and the IKM is the HMAC *message*. This ordering
//! follows RFC 5869 Section 2.2 exactly.
//!
//! ## Phase 2: Expand
//!
//! Expand produces as many output bytes as needed by chaining HMAC calls:
//!
//! ```text
//! T(0) = empty
//! T(i) = HMAC-Hash(PRK, T(i-1) || info || i)   for i = 1..N
//! OKM  = first L bytes of T(1) || T(2) || ... || T(N)
//! ```
//!
//! The counter byte is a single octet (0x01..0xFF), so the maximum output is
//! 255 × HashLen bytes.
//!
//! ## Example
//!
//! ```
//! use coding_adventures_hkdf::{hkdf, HashAlgorithm};
//!
//! let salt = b"my-salt";
//! let ikm = b"input-keying-material";
//! let info = b"application-context";
//! let okm = hkdf(salt, ikm, info, 32, HashAlgorithm::Sha256).unwrap();
//! assert_eq!(okm.len(), 32);
//! ```

use coding_adventures_blake2b::{blake2b as b2b_hash, Blake2bOptions};
use coding_adventures_hmac::{
    hmac, hmac_sha256, hmac_sha256_segments, hmac_sha384, hmac_sha384_segments, HmacError,
};
use coding_adventures_sha256::sha256;
use coding_adventures_sha512::sum512;
use coding_adventures_zeroize::{Zeroize, Zeroizing};

// ─── Hash Algorithm Selection ────────────────────────────────────────────────

/// Supported hash algorithms for HKDF.
///
/// Each variant carries the hash function's output length (HashLen) and
/// internal block size, which are needed by both Extract and Expand.
///
/// ## BLAKE2b variant
///
/// `Blake2b` uses BLAKE2b's native keyed-hash mode (Blake2b-MAC) as the
/// pseudorandom function (PRF) instead of HMAC-SHA-256/SHA-512.  BLAKE2b
/// in keyed mode is at least as secure as HMAC-BLAKE2b and is the
/// recommended construction when BLAKE2b is the underlying primitive.
///
/// The output length is fixed at **32 bytes** (256-bit) to match the PRK
/// size expected by the Signal Protocol stack built on this library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// SHA-256: 32-byte output, 64-byte block.
    Sha256,
    /// SHA-512: 64-byte output, 128-byte block.
    Sha512,
    /// BLAKE2b in keyed mode: 32-byte output.
    ///
    /// Keyed BLAKE2b serves as the PRF in place of HMAC.  The key (PRK in
    /// Extract, PRK again in Expand) is passed via BLAKE2b's built-in key
    /// parameter; no outer/inner-pad construction is needed.
    Blake2b,
}

impl HashAlgorithm {
    /// The output length of the hash function in bytes.
    ///
    /// This determines:
    /// - The length of the PRK from Extract
    /// - The size of each T(i) block in Expand
    /// - The default salt length when none is provided
    pub fn hash_len(self) -> usize {
        match self {
            HashAlgorithm::Sha256 => 32,
            HashAlgorithm::Sha512 => 64,
            HashAlgorithm::Blake2b => 32,
        }
    }

    /// Compute the PRF (HMAC or keyed-BLAKE2b) for this algorithm.
    ///
    /// For SHA-256 and SHA-512 this wraps the RFC 2104 HMAC construction.
    /// For BLAKE2b this uses the algorithm's native keyed-hash mode, which
    /// produces an equivalent pseudorandom function without the extra
    /// inner/outer padding steps.
    fn prf(self, key: &[u8], message: &[u8]) -> Vec<u8> {
        match self {
            HashAlgorithm::Sha256 => {
                hmac(|d| sha256(d).to_vec(), 64, key, message)
            }
            HashAlgorithm::Sha512 => {
                hmac(|d| sum512(d).to_vec(), 128, key, message)
            }
            HashAlgorithm::Blake2b => {
                // BLAKE2b-keyed: key is 1–64 bytes, output 32 bytes.
                // If key is empty (e.g. zero-salt case) we fall back to
                // an unkeyed hash over key||message to stay deterministic.
                let opts = if key.is_empty() {
                    // Treat as unkeyed hash of zero-salt||message.
                    // Concatenate a 32-byte zero key prefix with the message
                    // so behaviour is consistent with the HMAC zero-key path.
                    let mut data = vec![0u8; 32];
                    data.extend_from_slice(message);
                    return b2b_hash(&data, &Blake2bOptions::new().digest_size(32))
                        .expect("blake2b digest_size=32 is always valid");
                } else {
                    // Clamp key to 64 bytes maximum (BLAKE2b limit).
                    let key_bytes = if key.len() > 64 { &key[..64] } else { key };
                    Blake2bOptions::new()
                        .key(key_bytes)
                        .digest_size(32)
                };
                b2b_hash(message, &opts)
                    .expect("blake2b key/digest_size are within valid ranges")
            }
        }
    }

    /// Backward-compatible alias used internally.
    fn hmac(self, key: &[u8], message: &[u8]) -> Vec<u8> {
        self.prf(key, message)
    }
}

/// Repository-owned fixed-output hashes admitted by the fixed-memory Extract
/// and Expand boundaries.
///
/// This is deliberately separate from [`HashAlgorithm`]. The legacy enum also
/// contains allocation-backed compatibility algorithms that are not admitted
/// for production TLS secret handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedHkdfHash {
    /// HMAC-SHA-256 with a 32-byte PRK.
    Sha256,
    /// HMAC-SHA-384 with a 48-byte PRK.
    Sha384,
}

impl FixedHkdfHash {
    /// The exact PRK and RFC 5869 default-salt length for this hash.
    pub const fn hash_len(self) -> usize {
        match self {
            Self::Sha256 => 32,
            Self::Sha384 => 48,
        }
    }

    /// HMAC's inner and outer hash block length for this algorithm.
    const fn block_len(self) -> usize {
        match self {
            Self::Sha256 => 64,
            Self::Sha384 => 128,
        }
    }

    /// Largest complete hash input representable by the FIPS length field.
    const fn max_message_bytes(self) -> u128 {
        match self {
            Self::Sha256 => u64::MAX as u128 / 8,
            Self::Sha384 => u128::MAX / 8,
        }
    }
}

/// Wipe-on-drop pseudorandom key produced by [`hkdf_extract_fixed`].
///
/// The owner uses one fixed 48-byte backing array and exposes only the selected
/// hash's 32-byte or 48-byte prefix. Debug formatting reports metadata, never
/// PRK bytes. Callers should retain this owner rather than copying its secret
/// slice into longer-lived storage.
pub struct HkdfPrk {
    algorithm: FixedHkdfHash,
    bytes: Zeroizing<[u8; 48]>,
}

impl HkdfPrk {
    /// The hash algorithm that produced this PRK.
    pub const fn algorithm(&self) -> FixedHkdfHash {
        self.algorithm
    }

    /// Borrow the selected hash's exact PRK bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.algorithm.hash_len()]
    }
}

impl std::fmt::Debug for HkdfPrk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HkdfPrk")
            .field("algorithm", &self.algorithm)
            .field("length", &self.algorithm.hash_len())
            .finish_non_exhaustive()
    }
}

// ─── Error Type ──────────────────────────────────────────────────────────────

/// Errors that can occur during HKDF operations.
#[derive(Debug, PartialEq, Eq)]
pub enum HkdfError {
    /// The requested output length exceeds 255 × HashLen.
    ///
    /// The Expand phase uses a single-byte counter (0x01..0xFF), so it can
    /// produce at most 255 HMAC blocks of HashLen bytes each.
    OutputTooLong { requested: usize, maximum: usize },
    /// The requested output length is zero or negative.
    OutputTooShort,
    /// A fixed-output Extract or Expand input exceeds the selected hash's
    /// encoded message-length bound.
    InputTooLong,
    /// The fixed-output PRF rejected its key. Public Extract inputs cannot
    /// cause this because an omitted salt is replaced with HashLen zero bytes;
    /// this category preserves the underlying invariant without panicking.
    PrfRejectedKey,
}

impl std::fmt::Display for HkdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HkdfError::OutputTooLong { requested, maximum } => {
                write!(
                    f,
                    "HKDF output length {requested} exceeds maximum {maximum} (255 * HashLen)"
                )
            }
            HkdfError::OutputTooShort => {
                write!(f, "HKDF output length must be positive")
            }
            HkdfError::InputTooLong => {
                write!(f, "HKDF input exceeds the selected hash length bound")
            }
            HkdfError::PrfRejectedKey => {
                write!(f, "HKDF pseudorandom function rejected its key")
            }
        }
    }
}

impl std::error::Error for HkdfError {}

// ─── Extract ─────────────────────────────────────────────────────────────────

fn map_fixed_hmac_error(error: HmacError) -> HkdfError {
    match error {
        HmacError::InputTooLong => HkdfError::InputTooLong,
        HmacError::EmptyKey => HkdfError::PrfRejectedKey,
    }
}

/// Fixed-memory HKDF-Extract for repository-owned SHA-256 and SHA-384.
///
/// Implements RFC 5869 Section 2.2 without heap allocation:
///
/// ```text
/// PRK = HMAC-Hash(salt, IKM)
/// ```
///
/// An empty `salt` is replaced by exactly HashLen zero bytes. The algorithm is
/// selected explicitly through [`FixedHkdfHash`], and the returned [`HkdfPrk`]
/// wipes its fixed backing storage on drop. Temporary HMAC outputs are also
/// wipe-on-drop. The IKM is borrowed without allocating an aggregate input
/// buffer; the wiped hash state may retain one bounded partial block.
///
/// This function does not implement HKDF-Expand or a TLS key schedule.
pub fn hkdf_extract_fixed(
    salt: &[u8],
    ikm: &[u8],
    algorithm: FixedHkdfHash,
) -> Result<HkdfPrk, HkdfError> {
    let default_salt = [0u8; 48];
    let effective_salt = if salt.is_empty() {
        &default_salt[..algorithm.hash_len()]
    } else {
        salt
    };

    let mut prk = Zeroizing::new([0u8; 48]);
    match algorithm {
        FixedHkdfHash::Sha256 => {
            let tag =
                Zeroizing::new(hmac_sha256(effective_salt, ikm).map_err(map_fixed_hmac_error)?);
            prk[..32].copy_from_slice(&tag[..]);
        }
        FixedHkdfHash::Sha384 => {
            let tag =
                Zeroizing::new(hmac_sha384(effective_salt, ikm).map_err(map_fixed_hmac_error)?);
            prk[..48].copy_from_slice(&tag[..]);
        }
    }

    Ok(HkdfPrk {
        algorithm,
        bytes: prk,
    })
}

/// HKDF-Extract: concentrate entropy from IKM into a pseudorandom key.
///
/// Implements RFC 5869 Section 2.2:
///
/// ```text
/// PRK = HMAC-Hash(salt, IKM)
/// ```
///
/// If `salt` is empty, a string of `HashLen` zero bytes is used (per the RFC).
///
/// # Parameters
/// - `salt`: Optional salt value. Pass `&[]` for no salt.
/// - `ikm`: Input keying material.
/// - `algorithm`: Which hash function to use.
///
/// # Returns
/// The pseudorandom key (PRK), exactly `HashLen` bytes.
///
/// # Example
/// ```
/// use coding_adventures_hkdf::{hkdf_extract, HashAlgorithm};
/// let ikm = hex::decode("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b").unwrap();
/// let salt = hex::decode("000102030405060708090a0b0c").unwrap();
/// let prk = hkdf_extract(&salt, &ikm, HashAlgorithm::Sha256);
/// assert_eq!(
///     hex::encode(&prk),
///     "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"
/// );
/// ```
pub fn hkdf_extract(salt: &[u8], ikm: &[u8], algorithm: HashAlgorithm) -> Vec<u8> {
    // RFC 5869 Section 2.2: "if not provided, [salt] is set to a string
    // of HashLen zeros."
    //
    // When salt is empty, we create a vector of HashLen zero bytes.
    // HMAC will normalize this key to block_size bytes internally,
    // but the important thing is that we provide a deterministic,
    // non-empty key to HMAC.
    let effective_salt: Vec<u8>;
    let salt_ref = if salt.is_empty() {
        effective_salt = vec![0u8; algorithm.hash_len()];
        &effective_salt
    } else {
        salt
    };

    // Note: salt is the HMAC *key*, IKM is the *message*.
    // This follows RFC 5869 exactly.
    algorithm.hmac(salt_ref, ikm)
}

// ─── Expand ──────────────────────────────────────────────────────────────────

/// Return whether every inner HMAC input in fixed-memory Expand fits the
/// selected hash's encoded message-length field.
///
/// The HMAC stream first consumes one normalized key block, followed by the
/// logical message `T(i-1) || info || i`. Only blocks after the first include
/// `T(i-1)`, so a one-block expansion preserves the slightly larger valid
/// `info` boundary allowed by the standard. This public-length preflight occurs
/// before any caller output is changed.
fn fixed_expand_input_fits(algorithm: FixedHkdfHash, info_len: usize, block_count: usize) -> bool {
    let previous_len = if block_count > 1 {
        algorithm.hash_len()
    } else {
        0
    };
    let input_len = algorithm.block_len() as u128 + previous_len as u128 + info_len as u128 + 1;
    input_len <= algorithm.max_message_bytes()
}

/// A borrowed HMAC result tag that scrubs the original result storage on drop.
///
/// Protecting the borrowed array, rather than moving it into a second owner,
/// ensures the ABI return slot written by HMAC is the storage that gets wiped.
struct FixedExpandTag<'a, const N: usize>(&'a mut [u8; N]);

impl<const N: usize> Drop for FixedExpandTag<'_, N> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Copy one successful fixed-output HMAC block into caller output and the
/// wipe-on-drop previous-block owner. The tag remains in its original HMAC
/// result storage, which is scrubbed on every exit before its lifetime ends.
fn copy_fixed_expand_block<const N: usize>(
    result: &mut Result<[u8; N], HmacError>,
    previous: &mut [u8; 48],
    output: &mut [u8],
    written: usize,
    take: usize,
) -> Result<(), HmacError> {
    match result {
        Ok(tag) => {
            let tag = FixedExpandTag(tag);
            output[written..written + take].copy_from_slice(&tag.0[..take]);
            previous[..N].copy_from_slice(&tag.0[..]);
            Ok(())
        }
        Err(HmacError::EmptyKey) => Err(HmacError::EmptyKey),
        Err(HmacError::InputTooLong) => Err(HmacError::InputTooLong),
    }
}

/// Fixed-memory HKDF-Expand for repository-owned SHA-256 and SHA-384.
///
/// Implements RFC 5869 Section 2.3 into caller-provided storage:
///
/// ```text
/// T(0) = empty
/// T(i) = HMAC-Hash(PRK, T(i-1) || info || i)
/// OKM  = first L bytes of T(1) || ... || T(N)
/// ```
///
/// The algorithm is bound to `prk`, preventing an Extract/Expand mismatch.
/// Borrowed message segments avoid an aggregate HMAC-message allocation, one
/// fixed 48-byte owner holds `T(i-1)`, and each temporary HMAC tag is wiped on
/// drop. The function itself allocates no heap storage. On success, `output`
/// belongs to the caller and is not wiped. Public length failures occur before
/// `output` changes; an unexpected HMAC failure wipes all caller output before
/// returning.
///
/// This function does not implement `HKDF-Expand-Label` or a TLS key schedule.
///
/// # Errors
///
/// Returns [`HkdfError::OutputTooShort`] when `output` is empty,
/// [`HkdfError::OutputTooLong`] when it exceeds 255 times HashLen, or
/// [`HkdfError::InputTooLong`] when an inner HMAC input would exceed the
/// selected hash's encoded message-length bound.
pub fn hkdf_expand_fixed(prk: &HkdfPrk, info: &[u8], output: &mut [u8]) -> Result<(), HkdfError> {
    let algorithm = prk.algorithm();
    let hash_len = algorithm.hash_len();
    let maximum = 255 * hash_len;

    if output.is_empty() {
        return Err(HkdfError::OutputTooShort);
    }
    if output.len() > maximum {
        return Err(HkdfError::OutputTooLong {
            requested: output.len(),
            maximum,
        });
    }

    let block_count = output.len().div_ceil(hash_len);
    if !fixed_expand_input_fits(algorithm, info.len(), block_count) {
        return Err(HkdfError::InputTooLong);
    }

    let mut previous = Zeroizing::new([0u8; 48]);
    let mut written = 0;

    for block_index in 0..block_count {
        let previous_block = if block_index == 0 {
            &[][..]
        } else {
            &previous[..hash_len]
        };
        let counter = [(block_index + 1) as u8];
        let segments = [previous_block, info, &counter[..]];
        let remaining = output.len() - written;
        let take = remaining.min(hash_len);

        let result = match algorithm {
            FixedHkdfHash::Sha256 => {
                let mut tag = hmac_sha256_segments(prk.as_bytes(), &segments);
                copy_fixed_expand_block(&mut tag, &mut previous, output, written, take)
            }
            FixedHkdfHash::Sha384 => {
                let mut tag = hmac_sha384_segments(prk.as_bytes(), &segments);
                copy_fixed_expand_block(&mut tag, &mut previous, output, written, take)
            }
        };

        if let Err(error) = result {
            output.zeroize();
            return Err(map_fixed_hmac_error(error));
        }
        written += take;
    }

    Ok(())
}

/// HKDF-Expand: derive output keying material from a pseudorandom key.
///
/// Implements RFC 5869 Section 2.3:
///
/// ```text
/// N = ceil(L / HashLen)
/// T(0) = empty
/// T(i) = HMAC-Hash(PRK, T(i-1) || info || i)   for i = 1..N
/// OKM  = first L bytes of T(1) || ... || T(N)
/// ```
///
/// # Parameters
/// - `prk`: Pseudorandom key (typically from `hkdf_extract`).
/// - `info`: Context string binding the derived key to its purpose.
/// - `length`: Desired output length in bytes. Must be 1..=255*HashLen.
/// - `algorithm`: Which hash function to use.
///
/// # Errors
/// Returns `HkdfError::OutputTooLong` if length > 255 * HashLen.
/// Returns `HkdfError::OutputTooShort` if length == 0.
///
/// # Example
/// ```
/// use coding_adventures_hkdf::{hkdf_expand, HashAlgorithm};
/// let prk = hex::decode(
///     "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"
/// ).unwrap();
/// let info = hex::decode("f0f1f2f3f4f5f6f7f8f9").unwrap();
/// let okm = hkdf_expand(&prk, &info, 42, HashAlgorithm::Sha256).unwrap();
/// assert_eq!(
///     hex::encode(&okm),
///     "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
/// );
/// ```
pub fn hkdf_expand(
    prk: &[u8],
    info: &[u8],
    length: usize,
    algorithm: HashAlgorithm,
) -> Result<Vec<u8>, HkdfError> {
    let hash_len = algorithm.hash_len();
    let max_length = 255 * hash_len;

    // Validate output length.
    if length == 0 {
        return Err(HkdfError::OutputTooShort);
    }
    if length > max_length {
        return Err(HkdfError::OutputTooLong {
            requested: length,
            maximum: max_length,
        });
    }

    // Number of HMAC blocks needed: ceil(length / hash_len).
    let n = length.div_ceil(hash_len);

    // Build OKM block by block.
    //
    // Each block T(i) = HMAC-Hash(PRK, T(i-1) || info || counter_byte)
    // where PRK is the HMAC key and the concatenation is the HMAC message.
    // T(0) is the empty slice.
    let mut okm = Vec::with_capacity(n * hash_len);
    let mut t_prev: Vec<u8> = Vec::new(); // T(0) = empty

    for i in 1..=n {
        // Build the HMAC message: T(i-1) || info || counter_byte
        let mut message = Vec::with_capacity(t_prev.len() + info.len() + 1);
        message.extend_from_slice(&t_prev);
        message.extend_from_slice(info);
        // The counter is a single byte, 1-indexed. Since n <= 255 and
        // i starts at 1, the cast to u8 is always safe.
        message.push(i as u8);

        t_prev = algorithm.hmac(prk, &message);
        okm.extend_from_slice(&t_prev);
    }

    // Truncate to exactly the requested length.
    okm.truncate(length);
    Ok(okm)
}

// ─── Combined: Extract-then-Expand ──────────────────────────────────────────

/// HKDF: derive keying material from input keying material.
///
/// This is the standard "extract-then-expand" usage (RFC 5869 Section 2):
///
/// ```text
/// OKM = HKDF-Expand(HKDF-Extract(salt, IKM), info, L)
/// ```
///
/// # Parameters
/// - `salt`: Optional salt. Pass `&[]` for no salt.
/// - `ikm`: Input keying material.
/// - `info`: Context string.
/// - `length`: Desired output length in bytes.
/// - `algorithm`: Which hash function to use.
///
/// # Errors
/// Returns `HkdfError` if the output length is invalid.
///
/// # Example
/// ```
/// use coding_adventures_hkdf::{hkdf, HashAlgorithm};
/// // RFC 5869 Test Case 1
/// let ikm = hex::decode("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b").unwrap();
/// let salt = hex::decode("000102030405060708090a0b0c").unwrap();
/// let info = hex::decode("f0f1f2f3f4f5f6f7f8f9").unwrap();
/// let okm = hkdf(&salt, &ikm, &info, 42, HashAlgorithm::Sha256).unwrap();
/// assert_eq!(
///     hex::encode(&okm),
///     "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
/// );
/// ```
pub fn hkdf(
    salt: &[u8],
    ikm: &[u8],
    info: &[u8],
    length: usize,
    algorithm: HashAlgorithm,
) -> Result<Vec<u8>, HkdfError> {
    let prk = hkdf_extract(salt, ikm, algorithm);
    hkdf_expand(&prk, info, length, algorithm)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: decode a hex string to bytes.
    fn h(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    /// Helper: encode bytes to hex string.
    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn fixed_extract_sha256_matches_rfc5869_tc1() {
        let ikm = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
        let salt = h("000102030405060708090a0b0c");
        let prk = hkdf_extract_fixed(&salt, &ikm, FixedHkdfHash::Sha256).unwrap();

        assert_eq!(prk.algorithm(), FixedHkdfHash::Sha256);
        assert_eq!(
            hex(prk.as_bytes()),
            "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"
        );
    }

    #[test]
    fn fixed_extract_sha384_matches_rfc4231_tc2_prf() {
        let prk = hkdf_extract_fixed(
            b"Jefe",
            b"what do ya want for nothing?",
            FixedHkdfHash::Sha384,
        )
        .unwrap();

        assert_eq!(prk.algorithm(), FixedHkdfHash::Sha384);
        assert_eq!(
            hex(prk.as_bytes()),
            "af45d2e376484031617f78d2b58a6b1b9c7ef464f5a01b47e42ec3736322445e8e2240ca5e69e2c78b3239ecfab21649"
        );
    }

    #[test]
    fn fixed_extract_uses_hash_len_zero_default_salt() {
        let sha256 = hkdf_extract_fixed(&[], b"ikm", FixedHkdfHash::Sha256).unwrap();
        let legacy = hkdf_extract(&[], b"ikm", HashAlgorithm::Sha256);
        assert_eq!(sha256.as_bytes(), legacy.as_slice());

        let sha384 = hkdf_extract_fixed(&[], b"ikm", FixedHkdfHash::Sha384).unwrap();
        let expected_sha384 = hmac_sha384(&[0u8; 48], b"ikm").unwrap();
        assert_eq!(sha384.as_bytes(), expected_sha384.as_slice());
    }

    #[test]
    fn fixed_prk_debug_redacts_secret_bytes() {
        let prk = hkdf_extract_fixed(b"salt", b"secret-ikm", FixedHkdfHash::Sha256).unwrap();
        let rendered = format!("{prk:?}");

        assert!(rendered.contains("Sha256"));
        assert!(rendered.contains("length: 32"));
        assert!(!rendered.contains(&hex(prk.as_bytes())));
    }

    #[test]
    fn fixed_expand_sha256_matches_rfc5869_tc1() {
        let ikm = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
        let salt = h("000102030405060708090a0b0c");
        let info = h("f0f1f2f3f4f5f6f7f8f9");
        let prk = hkdf_extract_fixed(&salt, &ikm, FixedHkdfHash::Sha256).unwrap();
        let mut output = [0u8; 42];

        hkdf_expand_fixed(&prk, &info, &mut output).unwrap();

        assert_eq!(
            hex(&output),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
        );
    }

    #[test]
    fn fixed_expand_sha384_matches_independent_vector() {
        let ikm = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
        let salt = h("000102030405060708090a0b0c");
        let info = h("f0f1f2f3f4f5f6f7f8f9");
        let prk = hkdf_extract_fixed(&salt, &ikm, FixedHkdfHash::Sha384).unwrap();
        let mut output = [0u8; 82];

        hkdf_expand_fixed(&prk, &info, &mut output).unwrap();

        assert_eq!(
            hex(&output),
            "9b5097a86038b805309076a44b3a9f38063e25b516dcbf369f394cfab43685f748b6457763e4f0204fc5d95d1da3e62587b22eb8943d0fab6bb631a2fe9df1a68c6ce5d56116a52005b3f122b88b39b7251f"
        );
    }

    #[test]
    fn fixed_expand_sha256_empty_info_matches_rfc5869_tc3() {
        let ikm = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
        let prk = hkdf_extract_fixed(&[], &ikm, FixedHkdfHash::Sha256).unwrap();
        let mut output = [0u8; 42];

        hkdf_expand_fixed(&prk, &[], &mut output).unwrap();

        assert_eq!(
            hex(&output),
            "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"
        );
    }

    fn reference_expand_sha384(prk: &[u8], info: &[u8], length: usize) -> Vec<u8> {
        let mut output = Vec::with_capacity(length);
        let mut previous = Vec::new();

        for counter in 1..=length.div_ceil(48) {
            let mut message = Vec::with_capacity(previous.len() + info.len() + 1);
            message.extend_from_slice(&previous);
            message.extend_from_slice(info);
            message.push(counter as u8);
            previous = hmac_sha384(prk, &message).unwrap().to_vec();
            output.extend_from_slice(&previous);
        }
        output.truncate(length);
        output
    }

    #[test]
    fn fixed_expand_matches_compatibility_references_across_output_boundaries() {
        let info = b"fixed expand boundary context";
        for algorithm in [FixedHkdfHash::Sha256, FixedHkdfHash::Sha384] {
            let prk = hkdf_extract_fixed(b"salt", b"ikm", algorithm).unwrap();
            let hash_len = algorithm.hash_len();
            for length in [
                1,
                hash_len - 1,
                hash_len,
                hash_len + 1,
                2 * hash_len,
                2 * hash_len + 1,
                255 * hash_len,
            ] {
                let mut actual = vec![0xA5; length];
                hkdf_expand_fixed(&prk, info, &mut actual).unwrap();
                let expected = match algorithm {
                    FixedHkdfHash::Sha256 => {
                        hkdf_expand(prk.as_bytes(), info, length, HashAlgorithm::Sha256).unwrap()
                    }
                    FixedHkdfHash::Sha384 => reference_expand_sha384(prk.as_bytes(), info, length),
                };
                assert_eq!(actual, expected, "{algorithm:?} output length {length}");
            }
        }
    }

    #[test]
    fn fixed_expand_length_errors_do_not_modify_caller_output() {
        for algorithm in [FixedHkdfHash::Sha256, FixedHkdfHash::Sha384] {
            let prk = hkdf_extract_fixed(b"salt", b"ikm", algorithm).unwrap();
            let mut empty = [];
            assert_eq!(
                hkdf_expand_fixed(&prk, b"info", &mut empty),
                Err(HkdfError::OutputTooShort)
            );

            let maximum = 255 * algorithm.hash_len();
            let mut oversized = vec![0xA5; maximum + 1];
            assert_eq!(
                hkdf_expand_fixed(&prk, b"info", &mut oversized),
                Err(HkdfError::OutputTooLong {
                    requested: maximum + 1,
                    maximum,
                })
            );
            assert!(oversized.iter().all(|byte| *byte == 0xA5));
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // RFC 5869 Appendix A — Test Vectors
    // ═══════════════════════════════════════════════════════════════════════

    // ── Test Case 1: Basic SHA-256 ─────────────────────────────────────

    #[test]
    fn tc1_extract() {
        let ikm = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
        let salt = h("000102030405060708090a0b0c");
        let prk = hkdf_extract(&salt, &ikm, HashAlgorithm::Sha256);
        assert_eq!(
            hex(&prk),
            "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"
        );
    }

    #[test]
    fn tc1_expand() {
        let prk = h("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5");
        let info = h("f0f1f2f3f4f5f6f7f8f9");
        let okm = hkdf_expand(&prk, &info, 42, HashAlgorithm::Sha256).unwrap();
        assert_eq!(
            hex(&okm),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
        );
    }

    #[test]
    fn tc1_combined() {
        let ikm = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
        let salt = h("000102030405060708090a0b0c");
        let info = h("f0f1f2f3f4f5f6f7f8f9");
        let okm = hkdf(&salt, &ikm, &info, 42, HashAlgorithm::Sha256).unwrap();
        assert_eq!(
            hex(&okm),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
        );
    }

    // ── Test Case 2: Longer inputs ─────────────────────────────────────

    #[test]
    fn tc2_extract() {
        let ikm: Vec<u8> = (0x00..=0x4fu8).collect();  // 80 bytes
        let salt: Vec<u8> = (0x60..=0xafu8).collect();  // 80 bytes
        let prk = hkdf_extract(&salt, &ikm, HashAlgorithm::Sha256);
        assert_eq!(
            hex(&prk),
            "06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244"
        );
    }

    #[test]
    fn tc2_expand() {
        let prk = h("06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244");
        let info: Vec<u8> = (0xb0..=0xffu8).collect();  // 80 bytes
        let okm = hkdf_expand(&prk, &info, 82, HashAlgorithm::Sha256).unwrap();
        assert_eq!(
            hex(&okm),
            "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87"
        );
    }

    #[test]
    fn tc2_combined() {
        let ikm: Vec<u8> = (0x00..=0x4fu8).collect();
        let salt: Vec<u8> = (0x60..=0xafu8).collect();
        let info: Vec<u8> = (0xb0..=0xffu8).collect();
        let okm = hkdf(&salt, &ikm, &info, 82, HashAlgorithm::Sha256).unwrap();
        assert_eq!(
            hex(&okm),
            "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87"
        );
    }

    // ── Test Case 3: Empty salt and info ───────────────────────────────

    #[test]
    fn tc3_extract() {
        let ikm = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
        let prk = hkdf_extract(&[], &ikm, HashAlgorithm::Sha256);
        assert_eq!(
            hex(&prk),
            "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04"
        );
    }

    #[test]
    fn tc3_expand() {
        let prk = h("19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04");
        let okm = hkdf_expand(&prk, &[], 42, HashAlgorithm::Sha256).unwrap();
        assert_eq!(
            hex(&okm),
            "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"
        );
    }

    #[test]
    fn tc3_combined() {
        let ikm = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
        let okm = hkdf(&[], &ikm, &[], 42, HashAlgorithm::Sha256).unwrap();
        assert_eq!(
            hex(&okm),
            "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Edge Cases
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn expand_exactly_hash_len() {
        let prk = h("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5");
        let okm = hkdf_expand(&prk, b"test", 32, HashAlgorithm::Sha256).unwrap();
        assert_eq!(okm.len(), 32);
    }

    #[test]
    fn expand_one_byte() {
        let prk = vec![0x01u8; 32];
        let okm = hkdf_expand(&prk, &[], 1, HashAlgorithm::Sha256).unwrap();
        assert_eq!(okm.len(), 1);
    }

    #[test]
    fn expand_max_length_sha256() {
        let prk = vec![0x01u8; 32];
        let okm = hkdf_expand(&prk, &[], 255 * 32, HashAlgorithm::Sha256).unwrap();
        assert_eq!(okm.len(), 8160);
    }

    #[test]
    fn expand_exceeds_max_length() {
        let prk = vec![0x01u8; 32];
        let result = hkdf_expand(&prk, &[], 255 * 32 + 1, HashAlgorithm::Sha256);
        assert!(matches!(result, Err(HkdfError::OutputTooLong { .. })));
    }

    #[test]
    fn expand_zero_length() {
        let prk = vec![0x01u8; 32];
        let result = hkdf_expand(&prk, &[], 0, HashAlgorithm::Sha256);
        assert!(matches!(result, Err(HkdfError::OutputTooShort)));
    }

    #[test]
    fn sha512_basic() {
        let ikm = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
        let salt = h("000102030405060708090a0b0c");
        let prk = hkdf_extract(&salt, &ikm, HashAlgorithm::Sha512);
        assert_eq!(prk.len(), 64);
        let okm = hkdf_expand(&prk, b"info", 64, HashAlgorithm::Sha512).unwrap();
        assert_eq!(okm.len(), 64);
    }

    #[test]
    fn sha512_empty_salt() {
        let ikm = vec![0xabu8; 32];
        let prk = hkdf_extract(&[], &ikm, HashAlgorithm::Sha512);
        assert_eq!(prk.len(), 64);
    }

    #[test]
    fn sha512_max_length() {
        let prk = vec![0x01u8; 64];
        let okm = hkdf_expand(&prk, &[], 255 * 64, HashAlgorithm::Sha512).unwrap();
        assert_eq!(okm.len(), 16320);
    }

    #[test]
    fn sha512_exceeds_max() {
        let prk = vec![0x01u8; 64];
        let result = hkdf_expand(&prk, &[], 255 * 64 + 1, HashAlgorithm::Sha512);
        assert!(matches!(result, Err(HkdfError::OutputTooLong { .. })));
    }

    #[test]
    fn different_info_different_okm() {
        let prk = vec![0x01u8; 32];
        let okm1 = hkdf_expand(&prk, b"purpose-a", 32, HashAlgorithm::Sha256).unwrap();
        let okm2 = hkdf_expand(&prk, b"purpose-b", 32, HashAlgorithm::Sha256).unwrap();
        assert_ne!(okm1, okm2);
    }

    #[test]
    fn different_salt_different_prk() {
        let ikm = vec![0x01u8; 32];
        let prk1 = hkdf_extract(b"salt-1", &ikm, HashAlgorithm::Sha256);
        let prk2 = hkdf_extract(b"salt-2", &ikm, HashAlgorithm::Sha256);
        assert_ne!(prk1, prk2);
    }

    #[test]
    fn deterministic() {
        let okm1 = hkdf(b"salt", b"ikm", b"info", 42, HashAlgorithm::Sha256).unwrap();
        let okm2 = hkdf(b"salt", b"ikm", b"info", 42, HashAlgorithm::Sha256).unwrap();
        assert_eq!(okm1, okm2);
    }

    #[test]
    fn round_trip_extract_expand() {
        let salt = b"my-salt";
        let ikm = b"my-input-keying-material";
        let info = b"my-context";
        let length = 48;

        let combined = hkdf(salt, ikm, info, length, HashAlgorithm::Sha256).unwrap();
        let prk = hkdf_extract(salt, ikm, HashAlgorithm::Sha256);
        let manual = hkdf_expand(&prk, info, length, HashAlgorithm::Sha256).unwrap();
        assert_eq!(combined, manual);
    }

    #[test]
    fn error_display() {
        let err = HkdfError::OutputTooLong {
            requested: 9000,
            maximum: 8160,
        };
        assert!(err.to_string().contains("9000"));
        assert!(err.to_string().contains("8160"));

        let err = HkdfError::OutputTooShort;
        assert!(err.to_string().contains("positive"));

        let err = HkdfError::InputTooLong;
        assert!(err.to_string().contains("length bound"));

        let err = HkdfError::PrfRejectedKey;
        assert!(err.to_string().contains("rejected its key"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // BLAKE2b-keyed HKDF — structural / round-trip tests
    //
    // No official RFC test vectors exist for BLAKE2b-HKDF because this
    // construction is specific to this library (it uses BLAKE2b's native
    // keyed mode rather than HMAC-BLAKE2b).  We validate structural
    // properties instead: correct length, determinism, domain separation
    // via info strings, and consistency of combined vs. manual two-step call.
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn blake2b_extract_returns_32_bytes() {
        let salt = b"signal-salt";
        let ikm  = b"shared-secret-from-ecdh";
        let prk  = hkdf_extract(salt, ikm, HashAlgorithm::Blake2b);
        assert_eq!(prk.len(), 32, "BLAKE2b PRK must be exactly 32 bytes");
    }

    #[test]
    fn blake2b_expand_correct_length() {
        let prk = vec![0x42u8; 32];
        for len in [1, 16, 32, 64, 128] {
            let okm = hkdf_expand(&prk, b"info", len, HashAlgorithm::Blake2b).unwrap();
            assert_eq!(okm.len(), len, "expected {len} bytes from expand");
        }
    }

    #[test]
    fn blake2b_deterministic() {
        let okm1 = hkdf(b"salt", b"ikm", b"info", 32, HashAlgorithm::Blake2b).unwrap();
        let okm2 = hkdf(b"salt", b"ikm", b"info", 32, HashAlgorithm::Blake2b).unwrap();
        assert_eq!(okm1, okm2, "HKDF must be deterministic");
    }

    #[test]
    fn blake2b_info_provides_domain_separation() {
        let prk  = vec![0x01u8; 32];
        let okm1 = hkdf_expand(&prk, b"x3dh-root-key",    32, HashAlgorithm::Blake2b).unwrap();
        let okm2 = hkdf_expand(&prk, b"double-ratchet-ck", 32, HashAlgorithm::Blake2b).unwrap();
        assert_ne!(okm1, okm2, "different info strings must yield different OKM");
    }

    #[test]
    fn blake2b_different_salts_different_prk() {
        let ikm  = b"same-ikm";
        let prk1 = hkdf_extract(b"salt-alice", ikm, HashAlgorithm::Blake2b);
        let prk2 = hkdf_extract(b"salt-bob",   ikm, HashAlgorithm::Blake2b);
        assert_ne!(prk1, prk2);
    }

    #[test]
    fn blake2b_empty_salt_uses_zero_key() {
        // An empty salt must not panic; it falls through to the zero-key path.
        let prk = hkdf_extract(&[], b"ikm", HashAlgorithm::Blake2b);
        assert_eq!(prk.len(), 32);
    }

    #[test]
    fn blake2b_combined_equals_manual_two_step() {
        let salt   = b"session-salt";
        let ikm    = b"dh-output-bytes";
        let info   = b"coding_adventures_x3dh_v1";
        let length = 48;

        let combined = hkdf(salt, ikm, info, length, HashAlgorithm::Blake2b).unwrap();
        let prk      = hkdf_extract(salt, ikm, HashAlgorithm::Blake2b);
        let manual   = hkdf_expand(&prk, info, length, HashAlgorithm::Blake2b).unwrap();
        assert_eq!(combined, manual);
    }

    #[test]
    fn blake2b_max_output() {
        let prk = vec![0x01u8; 32];
        let okm = hkdf_expand(&prk, &[], 255 * 32, HashAlgorithm::Blake2b).unwrap();
        assert_eq!(okm.len(), 255 * 32);
    }

    #[test]
    fn blake2b_exceeds_max_output() {
        let prk    = vec![0x01u8; 32];
        let result = hkdf_expand(&prk, &[], 255 * 32 + 1, HashAlgorithm::Blake2b);
        assert!(matches!(result, Err(HkdfError::OutputTooLong { .. })));
    }

    #[test]
    fn blake2b_output_differs_from_sha256_for_same_inputs() {
        // Sanity check: the algorithm selection actually matters.
        let salt = b"salt";
        let ikm  = b"ikm";
        let info = b"info";
        let sha  = hkdf(salt, ikm, info, 32, HashAlgorithm::Sha256).unwrap();
        let b2   = hkdf(salt, ikm, info, 32, HashAlgorithm::Blake2b).unwrap();
        assert_ne!(sha, b2, "SHA-256 and BLAKE2b HKDF must produce different output");
    }
}
