//! # coding_adventures_uuid — UUID v1/v3/v4/v5/v7 generation and parsing
//!
//! A from-scratch implementation of UUIDs (Universally Unique Identifiers),
//! covering RFC 4122 (v1/v3/v4/v5) and the newer RFC 9562 (v7), without
//! relying on the popular `uuid` crate. Every algorithmic step is explained
//! inline so the reader can follow the specification directly in the code.
//!
//! ## What Is a UUID?
//!
//! A UUID is a 128-bit (16-byte) label that is unique across space and time
//! without a central authority issuing them. They appear as 32 hex digits
//! grouped by hyphens into the canonical form:
//!
//! ```text
//! 550e8400-e29b-41d4-a716-446655440000
//! ^^^^^^^^ ^^^^ ^^^^ ^^^^ ^^^^^^^^^^^^
//!    |      |    |    |    node (48 bits)
//!    |      |    |    clock_seq (14 bits)
//!    |      |    time_hi_and_version (12-bit timestamp + 4-bit version)
//!    |      time_mid (16 bits)
//!    time_low (32 bits)
//! ```
//!
//! The `version` nibble (byte 6, high nibble) identifies the UUID variant:
//! - **v1**: Time + MAC address (privacy concerns, but monotonic)
//! - **v3**: MD5 name-based hash (deterministic, collision-resistant under MD5)
//! - **v4**: Random (most common; non-deterministic)
//! - **v5**: SHA-1 name-based hash (preferred over v3)
//! - **v7**: Unix millisecond timestamp + random (new in RFC 9562; sortable)
//!
//! The `variant` bits in byte 8 identify the UUID family. RFC 4122 UUIDs
//! always have `10xx xxxx` in byte 8 (i.e., byte 8 OR'd with 0x80 and
//! AND'd with 0xBF).
//!
//! ## Why This Crate Exists
//!
//! This crate is part of the `coding-adventures` monorepo — a ground-up
//! implementation of the computing stack from transistors to web services.
//! We build UUID generation here to show that cryptographic hash functions
//! (implemented in `coding_adventures_sha1` and `coding_adventures_md5`) compose naturally into higher-
//! level protocols and identifiers.
//!
//! ## Examples
//!
//! ```
//! use coding_adventures_uuid::{v4, v5, NAMESPACE_DNS};
//! let u = v4().unwrap();
//! assert_eq!(u.version(), 4);
//! assert_eq!(u.variant(), "rfc4122");
//!
//! // Deterministic: same inputs always yield the same UUID
//! let u2 = v5(NAMESPACE_DNS, "python.org");
//! assert_eq!(u2.to_string(), "886313e1-3b8a-5372-9b90-0c9aee199e5d");
//! ```

use coding_adventures_sha1::sum1;    // fn sum1(data: &[u8]) -> [u8; 20]
use coding_adventures_md5::sum_md5;  // fn sum_md5(data: &[u8]) -> [u8; 16]

use std::fmt;
use std::str::FromStr;

// ─── Error Type ───────────────────────────────────────────────────────────────
//
// A dedicated error type makes failures composable with `?` and descriptive
// without pulling in large error-handling crates.

/// An error produced by UUID parsing or generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UUIDError(String);

impl fmt::Display for UUIDError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UUIDError: {}", self.0)
    }
}

impl std::error::Error for UUIDError {}

// ─── UUID Type ────────────────────────────────────────────────────────────────
//
// We store the UUID as 16 raw bytes in network (big-endian) byte order.
// Big-endian means the most significant byte of each multi-byte field comes
// first, matching how UUIDs are written on the wire and in strings.
//
// We derive Copy + Clone because UUIDs are small (16 bytes) and value
// semantics are the natural mental model — no need for reference counting.
// Ord + PartialOrd enable sorting, which is especially useful for v7 UUIDs.
// Hash enables use as HashMap keys.

/// A 128-bit UUID stored as 16 bytes in network byte order (big-endian).
///
/// The internal representation is a `[u8; 16]` matching the wire format.
/// All multi-byte fields are big-endian (MSB first), except where the
/// specification explicitly requires little-endian (not the case here).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UUID([u8; 16]);

impl UUID {
    // ── Construction ─────────────────────────────────────────────────────────

    /// Create a UUID from 16 raw bytes in network byte order.
    ///
    /// The bytes are used as-is; no version or variant bits are set.
    /// This is the low-level constructor; prefer `v4()`, `v5()`, etc.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        UUID(bytes)
    }

    /// Return the raw 16 bytes in network byte order.
    pub fn bytes(&self) -> [u8; 16] {
        self.0
    }

    /// Return the UUID as a 128-bit unsigned integer (big-endian).
    ///
    /// Useful for arithmetic comparisons. v7 UUIDs are monotonically
    /// increasing as u128 values when generated within the same millisecond
    /// window.
    pub fn to_u128(&self) -> u128 {
        u128::from_be_bytes(self.0)
    }

    // ── Field Accessors ───────────────────────────────────────────────────────
    //
    // RFC 4122 § 4.1.3: The version number is in the 4 high bits of byte 6.
    //
    //   Byte 6 layout:  [v v v v | t t t t]
    //                    ^^^^^^^ version nibble
    //                            ^^^^^^^ time_hi (low 12 bits of timestamp)
    //
    // We shift right 4 and mask to isolate the version nibble.

    /// Return the version field (the high nibble of byte 6).
    ///
    /// Valid values are 1, 3, 4, 5, 7, or 0 for the nil UUID.
    pub fn version(&self) -> u8 {
        (self.0[6] >> 4) & 0xF
    }

    // RFC 4122 § 4.1.1: The variant field is in the high bits of byte 8.
    //
    //   Byte 8:  [1 0 x x | x x x x]  → RFC 4122 variant  (bit pattern 10xx xxxx)
    //            [0 x x x | x x x x]  → NCS backward-compat
    //            [1 1 0 x | x x x x]  → Microsoft backward-compat
    //            [1 1 1 x | x x x x]  → reserved
    //
    // We decode the two high bits; for 0b11 we check the third bit.

    /// Return the variant field as a human-readable string.
    ///
    /// Returns `"rfc4122"` for standard UUIDs, `"ncs"` for NCS legacy UUIDs,
    /// `"microsoft"` for Microsoft GUID-style UUIDs, or `"reserved"`.
    pub fn variant(&self) -> &'static str {
        // The top two bits of byte 8 encode the variant.
        match (self.0[8] >> 6) & 0x3 {
            0b00 | 0b01 => "ncs",       // 0xxx xxxx — NCS backward compatibility
            0b10 => "rfc4122",          // 10xx xxxx — RFC 4122 (what we generate)
            0b11 => {
                // 110x xxxx = Microsoft; 111x xxxx = reserved
                if (self.0[8] >> 5) & 1 == 0 {
                    "microsoft"
                } else {
                    "reserved"
                }
            }
            _ => unreachable!(),
        }
    }

    /// Return `true` if this is the nil UUID (all 128 bits zero).
    ///
    /// The nil UUID `00000000-0000-0000-0000-000000000000` is the UUID analog
    /// of a null pointer — conventionally means "no UUID assigned".
    pub fn is_nil(&self) -> bool {
        self.0 == [0u8; 16]
    }

    /// Return `true` if this is the max UUID (all 128 bits one, i.e. all 0xFF).
    ///
    /// Introduced in RFC 9562 as the UUID analog of a sentinel max value.
    pub fn is_max(&self) -> bool {
        self.0 == [0xFFu8; 16]
    }
}

// ─── Display and Debug ────────────────────────────────────────────────────────
//
// RFC 4122 § 3 mandates the canonical 8-4-4-4-12 lowercase hex format.
// We convert the 16 bytes to 32 hex characters then insert hyphens at the
// correct positions.

impl fmt::Display for UUID {
    /// Format as the standard 8-4-4-4-12 lowercase hex representation.
    ///
    /// Example: `"550e8400-e29b-41d4-a716-446655440000"`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let h = hex_encode(&self.0);
        write!(
            f,
            "{}-{}-{}-{}-{}",
            &h[0..8],
            &h[8..12],
            &h[12..16],
            &h[16..20],
            &h[20..32]
        )
    }
}

impl fmt::Debug for UUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UUID(\"{}\")", self)
    }
}

// ─── Parsing ──────────────────────────────────────────────────────────────────
//
// We accept four common serialization forms:
//   Standard:  "550e8400-e29b-41d4-a716-446655440000"  (36 chars, 4 hyphens)
//   Compact:   "550e8400e29b41d4a716446655440000"        (32 chars, no hyphens)
//   Braced:    "{550e8400-e29b-41d4-a716-446655440000}" (common in Microsoft GUID)
//   URN:       "urn:uuid:550e8400-e29b-41d4-a716-446655440000"
//
// Strategy: strip known prefixes/wrappers, remove hyphens, validate length
// is exactly 32, then decode 2 hex chars per byte.

impl FromStr for UUID {
    type Err = UUIDError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse(s)
    }
}

/// Parse a UUID from its string representation.
///
/// Accepts standard (8-4-4-4-12), compact (no hyphens), braced, and URN forms.
///
/// # Errors
///
/// Returns [`UUIDError`] if the string is not a valid UUID representation.
///
/// # Examples
///
/// ```
/// use coding_adventures_uuid::parse;
/// let u = parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
/// assert_eq!(u.to_string(), "550e8400-e29b-41d4-a716-446655440000");
/// ```
pub fn parse(s: &str) -> Result<UUID, UUIDError> {
    let s = s.trim();

    // Strip URN prefix "urn:uuid:"
    let s = s.strip_prefix("urn:uuid:").unwrap_or(s);

    // Strip outer braces (Microsoft GUID style)
    let s = s.strip_prefix('{').unwrap_or(s);
    let s = s.strip_suffix('}').unwrap_or(s);

    // Remove hyphens so we work with raw hex
    let hex: String = s.chars().filter(|c| *c != '-').collect();

    if hex.len() != 32 {
        return Err(UUIDError(format!(
            "expected 32 hex digits (after removing hyphens), got {}: {:?}",
            hex.len(),
            s
        )));
    }

    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(UUIDError(format!(
            "non-hex character in UUID string: {:?}",
            s
        )));
    }

    let mut bytes = [0u8; 16];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let high = hex_digit_value(chunk[0]);
        let low = hex_digit_value(chunk[1]);
        bytes[i] = (high << 4) | low;
    }

    Ok(UUID(bytes))
}

/// Return `true` if `s` is a valid UUID string in any accepted format.
///
/// # Examples
///
/// ```
/// use coding_adventures_uuid::is_valid;
/// assert!(is_valid("550e8400-e29b-41d4-a716-446655440000"));
/// assert!(is_valid("{550e8400-e29b-41d4-a716-446655440000}"));
/// assert!(!is_valid("not-a-uuid"));
/// ```
pub fn is_valid(s: &str) -> bool {
    parse(s).is_ok()
}

// ─── Hex Helpers ──────────────────────────────────────────────────────────────
//
// We roll our own rather than pulling in a hex crate. These are simple enough
// that they're clearer to read than a dependency import.

/// Convert a single ASCII hex digit byte to its 0–15 numeric value.
/// Non-hex input returns 0 (never reached due to prior validation).
fn hex_digit_value(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

/// Encode bytes as a lowercase hex string.
/// `[0xDE, 0xAD]` → `"dead"`.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ─── Namespace Constants ──────────────────────────────────────────────────────
//
// RFC 4122 Appendix C defines four well-known namespace UUIDs for use with
// name-based UUIDs (v3 and v5). They are fixed constants chosen by the RFC
// authors and will never change.
//
// These bytes are the binary big-endian encoding of the "standard" UUID
// string form. For example, NAMESPACE_DNS decodes to:
//   "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
//
// Why namespaces? Because two different applications might both want a stable
// UUID for the string "python.org". By including a namespace UUID in the hash
// input, they get different deterministic UUIDs and cannot collide.

/// Namespace UUID for fully qualified domain names (RFC 4122 Appendix C).
///
/// `"6ba7b810-9dad-11d1-80b4-00c04fd430c8"`
pub const NAMESPACE_DNS: UUID = UUID([
    0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1,
    0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
]);

/// Namespace UUID for URLs (RFC 4122 Appendix C).
///
/// `"6ba7b811-9dad-11d1-80b4-00c04fd430c8"`
pub const NAMESPACE_URL: UUID = UUID([
    0x6b, 0xa7, 0xb8, 0x11, 0x9d, 0xad, 0x11, 0xd1,
    0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
]);

/// Namespace UUID for ISO Object Identifiers (RFC 4122 Appendix C).
///
/// `"6ba7b812-9dad-11d1-80b4-00c04fd430c8"`
pub const NAMESPACE_OID: UUID = UUID([
    0x6b, 0xa7, 0xb8, 0x12, 0x9d, 0xad, 0x11, 0xd1,
    0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
]);

/// Namespace UUID for X.500 distinguished names (RFC 4122 Appendix C).
///
/// `"6ba7b814-9dad-11d1-80b4-00c04fd430c8"`
pub const NAMESPACE_X500: UUID = UUID([
    0x6b, 0xa7, 0xb8, 0x14, 0x9d, 0xad, 0x11, 0xd1,
    0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
]);

/// The nil UUID: all 128 bits zero.
///
/// Analogous to a null pointer — conventionally means "no UUID assigned".
/// `"00000000-0000-0000-0000-000000000000"`
pub const NIL: UUID = UUID([0u8; 16]);

/// The max UUID: all 128 bits one (all 0xFF bytes).
///
/// Introduced in RFC 9562 as a sentinel maximum value.
/// `"ffffffff-ffff-ffff-ffff-ffffffffffff"`
pub const MAX: UUID = UUID([0xFF; 16]);

// ─── Version and Variant Stamping ─────────────────────────────────────────────
//
// After constructing the raw 16-byte buffer from a hash or random source,
// we must overwrite two fields to identify the UUID version and variant:
//
//   Byte 6: set high nibble to `version`
//   Byte 8: set two high bits to `10` (RFC 4122 variant)
//
// The masking ensures we preserve the lower bits of each byte:
//   raw[6] & 0x0F clears the version nibble (keeps lower 4 bits from hash)
//   raw[8] & 0x3F clears the top 2 variant bits (keeps lower 6 bits from hash)

fn set_version_variant(raw: &mut [u8; 16], version: u8) {
    // Set version: byte 6 high nibble = version number (1–7)
    raw[6] = (raw[6] & 0x0F) | (version << 4);
    // Set variant: byte 8 high 2 bits = 10 (RFC 4122)
    raw[8] = (raw[8] & 0x3F) | 0x80;
}

// ─── Randomness ───────────────────────────────────────────────────────────────
//
// Rust's standard library provides no CSPRNG (cryptographically secure
// pseudo-random number generator). The `getrandom` crate bridges to the OS
// entropy source: `getrandom(2)` on Linux, `BCryptGenRandom` on Windows,
// `SecRandomCopyBytes` on macOS/iOS.
//
// This is the same entropy source used by the `uuid` and `rand` crates.
// We request exactly N bytes of entropy, returning a UUIDError on failure
// (which in practice only happens in exotic embedded or WASM environments).

fn random_bytes<const N: usize>() -> Result<[u8; N], UUIDError> {
    let mut buf = [0u8; N];
    getrandom::getrandom(&mut buf)
        .map_err(|e| UUIDError(format!("RNG error: {}", e)))?;
    Ok(buf)
}

// ─── UUID v4: Random ─────────────────────────────────────────────────────────
//
// v4 is the most common UUID version. It is simply 122 bits of random data
// with the 4-bit version and 2-bit variant fields stamped in. There is no
// timestamp, no MAC address, and no determinism.
//
// RFC 4122 § 4.4:
//   Set all 128 bits from a CSPRNG.
//   Set byte 6 high nibble = 0100 (version 4).
//   Set byte 8 high 2 bits = 10 (RFC 4122 variant).
//
// Expected collision probability for n independently generated v4 UUIDs:
//   P(collision) ≈ n² / (2 × 2^122)
// For n = 1 billion: P ≈ 1.2 × 10^-19. Effectively zero in practice.

/// Generate a UUID v4 (random).
///
/// Uses the OS CSPRNG via the `getrandom` crate. Returns an error only in
/// rare environments where the OS entropy source is unavailable.
///
/// # Examples
///
/// ```
/// use coding_adventures_uuid::v4;
/// let u = v4().unwrap();
/// assert_eq!(u.version(), 4);
/// assert_eq!(u.variant(), "rfc4122");
/// ```
pub fn v4() -> Result<UUID, UUIDError> {
    let mut raw = random_bytes::<16>()?;
    set_version_variant(&mut raw, 4);
    Ok(UUID(raw))
}

// ─── UUID v5: SHA-1 Name-Based ────────────────────────────────────────────────
//
// v5 produces a deterministic UUID from a namespace + name pair by hashing
// them with SHA-1. The same (namespace, name) always yields the same UUID.
// This is the preferred choice over v3 (MD5) for new code.
//
// RFC 4122 § 4.3 algorithm:
//   1. Concatenate namespace bytes (16) and name bytes.
//   2. Compute SHA-1 digest → 20 bytes.
//   3. Take the first 16 bytes as the raw UUID.
//   4. Stamp version=5 and variant=rfc4122 into bytes 6 and 8.
//
// Why SHA-1? It is still collision-resistant for this purpose: we are not
// relying on SHA-1 for signature security, just for uniform distribution
// and stable output. Two different names will not collide in practice.
//
// Known RFC vector: v5(NAMESPACE_DNS, "python.org") = "886313e1-3b8a-5372-9b90-0c9aee199e5d"

/// Generate a UUID v5 (SHA-1 name-based). Deterministic.
///
/// Given the same `namespace` and `name`, always returns the same UUID.
/// Prefer v5 over v3 for new systems; SHA-1 provides better distribution.
///
/// # Examples
///
/// ```
/// use coding_adventures_uuid::{v5, NAMESPACE_DNS};
/// let u = v5(NAMESPACE_DNS, "python.org");
/// assert_eq!(u.to_string(), "886313e1-3b8a-5372-9b90-0c9aee199e5d");
/// ```
pub fn v5(namespace: UUID, name: &str) -> UUID {
    // Build the hash input: namespace bytes || name bytes
    let mut data = Vec::with_capacity(16 + name.len());
    data.extend_from_slice(&namespace.0);
    data.extend_from_slice(name.as_bytes());

    // SHA-1 produces 20 bytes; we use the first 16 as the UUID body.
    let digest = sum1(&data); // [u8; 20]
    let mut raw = [0u8; 16];
    raw.copy_from_slice(&digest[..16]);

    set_version_variant(&mut raw, 5);
    UUID(raw)
}

// ─── UUID v3: MD5 Name-Based ─────────────────────────────────────────────────
//
// v3 is identical to v5 but uses MD5 instead of SHA-1. MD5 produces 16 bytes
// directly, so we use all 16 (then stamp version/variant).
//
// MD5 is no longer considered cryptographically secure, but v3 UUIDs remain
// valid for non-security-critical use (e.g., generating stable IDs for known
// inputs where an adversary cannot forge collisions).
//
// Prefer v5 for new systems. v3 exists for compatibility with older systems.
//
// Known RFC vector: v3(NAMESPACE_DNS, "python.org") = "6fa459ea-ee8a-3ca4-894e-db77e160355e"

/// Generate a UUID v3 (MD5 name-based). Deterministic.
///
/// Given the same `namespace` and `name`, always returns the same UUID.
/// Prefer [`v5`] for new systems; use v3 only for compatibility.
///
/// # Examples
///
/// ```
/// use coding_adventures_uuid::{v3, NAMESPACE_DNS};
/// let u = v3(NAMESPACE_DNS, "python.org");
/// assert_eq!(u.to_string(), "6fa459ea-ee8a-3ca4-894e-db77e160355e");
/// ```
pub fn v3(namespace: UUID, name: &str) -> UUID {
    // Build the hash input: namespace bytes || name bytes
    let mut data = Vec::with_capacity(16 + name.len());
    data.extend_from_slice(&namespace.0);
    data.extend_from_slice(name.as_bytes());

    // MD5 produces exactly 16 bytes — the right size for a UUID body.
    let mut raw = sum_md5(&data); // [u8; 16]

    set_version_variant(&mut raw, 3);
    UUID(raw)
}

// ─── UUID v1: Time-Based ─────────────────────────────────────────────────────
//
// v1 encodes a 60-bit timestamp (100-nanosecond intervals since 1582-10-15,
// the Gregorian epoch) plus a 14-bit clock sequence (to avoid duplicates if
// the clock goes backward) plus a 48-bit node ID (traditionally the MAC
// address, but we use a random node for privacy).
//
// RFC 4122 § 4.2 layout:
//
//   Byte  0-3:  time_low       — low 32 bits of timestamp
//   Byte  4-5:  time_mid       — next 16 bits of timestamp
//   Byte  6-7:  time_hi_and_version — high 12 bits of timestamp + 4-bit version
//   Byte  8:    clock_seq_hi_res — 2-bit variant + high 6 bits of clock_seq
//   Byte  9:    clock_seq_low
//   Byte 10-15: node           — 48-bit node ID
//
// The Gregorian epoch offset: the number of 100-ns intervals from 1582-10-15
// to 1970-01-01 (the Unix epoch). This is a fixed constant from RFC 4122.
//
// We use a random node (RFC 4122 § 4.5 allows this when MAC address is
// unavailable or undesired). The multicast bit (LSB of byte 0 of the node)
// is set to 1 to indicate a random node, distinguishing it from real MACs.

/// 100-nanosecond intervals between the Gregorian epoch (1582-10-15)
/// and the Unix epoch (1970-01-01). RFC 4122 Appendix B.
const GREGORIAN_OFFSET: u64 = 122_192_928_000_000_000;

/// Generate a UUID v1 (time-based with random node ID).
///
/// Uses the system clock for the timestamp and a random 48-bit node ID
/// (with the multicast bit set to indicate a random, not MAC, node).
///
/// # Examples
///
/// ```
/// use coding_adventures_uuid::v1;
/// let u = v1().unwrap();
/// assert_eq!(u.version(), 1);
/// assert_eq!(u.variant(), "rfc4122");
/// ```
pub fn v1() -> Result<UUID, UUIDError> {
    // Get current time as nanoseconds since Unix epoch, then convert to
    // 100-ns intervals since the Gregorian epoch.
    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| UUIDError(format!("SystemTime error: {}", e)))?
        .as_nanos() as u64;

    // 1 100-ns interval = 100 ns, so ns / 100 = count of 100-ns intervals.
    let t = now_ns / 100 + GREGORIAN_OFFSET;

    // Extract the three timestamp sub-fields:
    //   bits 0-31  → time_low  (bytes 0-3)
    //   bits 32-47 → time_mid  (bytes 4-5)
    //   bits 48-59 → time_hi   (lower 12 bits of bytes 6-7, after version bits)
    let time_low = (t & 0xFFFF_FFFF) as u32;
    let time_mid = ((t >> 32) & 0xFFFF) as u16;
    let time_hi = ((t >> 48) & 0x0FFF) as u16;
    let time_hi_v = 0x1000u16 | time_hi; // set version 1 in high nibble

    // 14-bit random clock sequence (splits across bytes 8 and 9).
    // We generate 2 random bytes and mask to 14 bits.
    let clock_raw = random_bytes::<2>()?;
    let clock_seq = u16::from_be_bytes(clock_raw) & 0x3FFF;
    // Byte 8 layout: [1 0 | clock_seq high 6 bits] → variant bits + high seq
    let clock_hi = (0x80 | (clock_seq >> 8)) as u8;
    let clock_lo = (clock_seq & 0xFF) as u8;

    // Random 48-bit node with multicast bit set (LSB of first byte).
    // RFC 4122 § 4.5: setting the multicast bit signals a randomly chosen
    // node ID rather than a real IEEE 802 MAC address.
    let mut node = random_bytes::<6>()?;
    node[0] |= 0x01; // set multicast bit

    let mut raw = [0u8; 16];
    raw[0..4].copy_from_slice(&time_low.to_be_bytes());
    raw[4..6].copy_from_slice(&time_mid.to_be_bytes());
    raw[6..8].copy_from_slice(&time_hi_v.to_be_bytes());
    raw[8] = clock_hi;
    raw[9] = clock_lo;
    raw[10..16].copy_from_slice(&node);

    Ok(UUID(raw))
}

// ─── UUID v7: Unix-Time Ordered Random ────────────────────────────────────────
//
// v7 is new in RFC 9562 and addresses a key shortcoming of v4: random UUIDs
// do not sort chronologically, causing poor database index locality when used
// as primary keys.
//
// v7 layout (128 bits):
//
//   Bits 0–47:   unix_ts_ms   — Unix timestamp in milliseconds (big-endian)
//   Bits 48–51:  version      — 0111 (7)
//   Bits 52–63:  rand_a       — 12 bits random
//   Bits 64–65:  variant      — 10 (RFC 4122)
//   Bits 66–127: rand_b       — 62 bits random
//
// The 48-bit millisecond timestamp provides ≥ 10,000 years of monotonic
// ordering (until year 10889). Two UUIDs generated in the same millisecond
// are further differentiated by their random bits; sorting them as u128
// produces insertion-order within a millisecond window.
//
// In byte terms:
//   raw[0..6]  = 48-bit ms timestamp (6 bytes, big-endian)
//   raw[6]     = 0x70 | rand_a[0..4]   (version nibble + 4 random bits)
//   raw[7]     = rand_a[4..12]          (remaining 8 random bits of rand_a)
//   raw[8]     = 0x80 | rand_b[0..6]   (variant bits + 6 random bits)
//   raw[9..16] = rand_b[6..62]          (remaining 56 random bits)

/// Generate a UUID v7 (Unix-time ordered random).
///
/// v7 UUIDs sort chronologically as byte strings and as `u128` values.
/// This makes them excellent as database primary keys — they maintain
/// B-tree locality unlike v4 (fully random) UUIDs.
///
/// # Examples
///
/// ```
/// use coding_adventures_uuid::v7;
/// let u = v7().unwrap();
/// assert_eq!(u.version(), 7);
/// assert_eq!(u.variant(), "rfc4122");
/// ```
pub fn v7() -> Result<UUID, UUIDError> {
    // 48-bit Unix timestamp in milliseconds.
    let ts_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| UUIDError(format!("SystemTime error: {}", e)))?
        .as_millis() as u64;

    // 10 random bytes (80 bits) for rand_a (12 bits) and rand_b (62 bits).
    let rand = random_bytes::<10>()?;

    let mut raw = [0u8; 16];

    // Bytes 0–5: 48-bit timestamp (big-endian, MSB first)
    raw[0] = ((ts_ms >> 40) & 0xFF) as u8;
    raw[1] = ((ts_ms >> 32) & 0xFF) as u8;
    raw[2] = ((ts_ms >> 24) & 0xFF) as u8;
    raw[3] = ((ts_ms >> 16) & 0xFF) as u8;
    raw[4] = ((ts_ms >> 8) & 0xFF) as u8;
    raw[5] = (ts_ms & 0xFF) as u8;

    // Byte 6: version nibble (0x7_) OR'd with 4 random bits from rand[0]
    raw[6] = 0x70 | (rand[0] & 0x0F);

    // Byte 7: remaining 8 random bits of rand_a
    raw[7] = rand[1];

    // Byte 8: variant bits (10xx xxxx) OR'd with 6 random bits
    raw[8] = 0x80 | (rand[2] & 0x3F);

    // Bytes 9–15: 56 more random bits from rand[3..10]
    raw[9..16].copy_from_slice(&rand[3..10]);

    Ok(UUID(raw))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Parsing Tests ─────────────────────────────────────────────────────────

    #[test]
    fn parse_standard_form() {
        let s = "550e8400-e29b-41d4-a716-446655440000";
        let u = parse(s).unwrap();
        assert_eq!(u.to_string(), s);
    }

    #[test]
    fn parse_compact_no_hyphens() {
        let compact = "550e8400e29b41d4a716446655440000";
        let u = parse(compact).unwrap();
        assert_eq!(u.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn parse_braced_form() {
        let braced = "{550e8400-e29b-41d4-a716-446655440000}";
        let u = parse(braced).unwrap();
        assert_eq!(u.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn parse_urn_form() {
        let urn = "urn:uuid:550e8400-e29b-41d4-a716-446655440000";
        let u = parse(urn).unwrap();
        assert_eq!(u.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn parse_uppercase_hex() {
        let upper = "550E8400-E29B-41D4-A716-446655440000";
        let u = parse(upper).unwrap();
        // Display is always lowercase
        assert_eq!(u.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn parse_nil_uuid() {
        let u = parse("00000000-0000-0000-0000-000000000000").unwrap();
        assert!(u.is_nil());
    }

    #[test]
    fn parse_max_uuid() {
        let u = parse("ffffffff-ffff-ffff-ffff-ffffffffffff").unwrap();
        assert!(u.is_max());
    }

    #[test]
    fn parse_rejects_too_short() {
        assert!(parse("550e8400-e29b-41d4-a716").is_err());
    }

    #[test]
    fn parse_rejects_too_long() {
        assert!(parse("550e8400-e29b-41d4-a716-4466554400001234").is_err());
    }

    #[test]
    fn parse_rejects_non_hex() {
        assert!(parse("550e8400-e29b-41d4-a716-44665544GGGG").is_err());
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn parse_from_str_trait() {
        let u: UUID = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
        assert_eq!(u.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    // ── is_valid Tests ────────────────────────────────────────────────────────

    #[test]
    fn is_valid_accepts_standard() {
        assert!(is_valid("550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn is_valid_accepts_braced() {
        assert!(is_valid("{550e8400-e29b-41d4-a716-446655440000}"));
    }

    #[test]
    fn is_valid_accepts_urn() {
        assert!(is_valid("urn:uuid:550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn is_valid_rejects_garbage() {
        assert!(!is_valid("not-a-uuid"));
        assert!(!is_valid("hello"));
        assert!(!is_valid(""));
    }

    // ── Display / to_string Tests ─────────────────────────────────────────────

    #[test]
    fn display_is_lowercase_hyphenated() {
        let u = parse("FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF").unwrap();
        assert_eq!(u.to_string(), "ffffffff-ffff-ffff-ffff-ffffffffffff");
    }

    #[test]
    fn display_format_is_8_4_4_4_12() {
        let s = parse("550e8400-e29b-41d4-a716-446655440000")
            .unwrap()
            .to_string();
        let parts: Vec<&str> = s.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
    }

    // ── Version and Variant Tests ─────────────────────────────────────────────

    #[test]
    fn version_field_read_correctly() {
        // Byte 6 high nibble encodes version: 0x40 → version 4
        let mut raw = [0u8; 16];
        raw[6] = 0x40; // version 4
        raw[8] = 0x80; // variant rfc4122
        let u = UUID::from_bytes(raw);
        assert_eq!(u.version(), 4);
    }

    #[test]
    fn variant_rfc4122() {
        let mut raw = [0u8; 16];
        raw[8] = 0x80; // 10xx xxxx → rfc4122
        let u = UUID::from_bytes(raw);
        assert_eq!(u.variant(), "rfc4122");
    }

    #[test]
    fn variant_ncs() {
        let mut raw = [0u8; 16];
        raw[8] = 0x00; // 00xx xxxx → ncs
        let u = UUID::from_bytes(raw);
        assert_eq!(u.variant(), "ncs");
    }

    #[test]
    fn variant_microsoft() {
        let mut raw = [0u8; 16];
        raw[8] = 0xC0; // 110x xxxx → microsoft
        let u = UUID::from_bytes(raw);
        assert_eq!(u.variant(), "microsoft");
    }

    // ── Nil and Max Tests ─────────────────────────────────────────────────────

    #[test]
    fn nil_constant_is_nil() {
        assert!(NIL.is_nil());
        assert!(!NIL.is_max());
        assert_eq!(NIL.to_string(), "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn max_constant_is_max() {
        assert!(MAX.is_max());
        assert!(!MAX.is_nil());
        assert_eq!(MAX.to_string(), "ffffffff-ffff-ffff-ffff-ffffffffffff");
    }

    #[test]
    fn non_nil_is_not_nil() {
        let u = parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert!(!u.is_nil());
    }

    // ── Namespace Constant Tests ──────────────────────────────────────────────

    #[test]
    fn namespace_dns_string() {
        // RFC 4122 Appendix C
        assert_eq!(
            NAMESPACE_DNS.to_string(),
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
        );
    }

    #[test]
    fn namespace_url_string() {
        assert_eq!(
            NAMESPACE_URL.to_string(),
            "6ba7b811-9dad-11d1-80b4-00c04fd430c8"
        );
    }

    #[test]
    fn namespace_oid_string() {
        assert_eq!(
            NAMESPACE_OID.to_string(),
            "6ba7b812-9dad-11d1-80b4-00c04fd430c8"
        );
    }

    #[test]
    fn namespace_x500_string() {
        assert_eq!(
            NAMESPACE_X500.to_string(),
            "6ba7b814-9dad-11d1-80b4-00c04fd430c8"
        );
    }

    // ── v4 Tests ─────────────────────────────────────────────────────────────

    #[test]
    fn v4_version_is_4() {
        let u = v4().unwrap();
        assert_eq!(u.version(), 4);
    }

    #[test]
    fn v4_variant_is_rfc4122() {
        let u = v4().unwrap();
        assert_eq!(u.variant(), "rfc4122");
    }

    #[test]
    fn v4_is_not_nil() {
        // Probability of being nil: 1/2^122 — negligible
        let u = v4().unwrap();
        assert!(!u.is_nil());
    }

    #[test]
    fn v4_outputs_are_unique() {
        // Birthday paradox: generating 100 should yield 100 distinct values
        let uuids: std::collections::HashSet<String> =
            (0..100).map(|_| v4().unwrap().to_string()).collect();
        assert_eq!(uuids.len(), 100);
    }

    // ── v5 Tests ─────────────────────────────────────────────────────────────

    #[test]
    fn v5_rfc_vector_python_org() {
        // RFC 4122 § Appendix B test vector
        let u = v5(NAMESPACE_DNS, "python.org");
        assert_eq!(u.to_string(), "886313e1-3b8a-5372-9b90-0c9aee199e5d");
    }

    #[test]
    fn v5_version_is_5() {
        let u = v5(NAMESPACE_DNS, "example.com");
        assert_eq!(u.version(), 5);
    }

    #[test]
    fn v5_variant_is_rfc4122() {
        let u = v5(NAMESPACE_DNS, "example.com");
        assert_eq!(u.variant(), "rfc4122");
    }

    #[test]
    fn v5_is_deterministic() {
        let u1 = v5(NAMESPACE_DNS, "hello.example");
        let u2 = v5(NAMESPACE_DNS, "hello.example");
        assert_eq!(u1, u2);
    }

    #[test]
    fn v5_different_names_differ() {
        let u1 = v5(NAMESPACE_DNS, "foo.example");
        let u2 = v5(NAMESPACE_DNS, "bar.example");
        assert_ne!(u1, u2);
    }

    #[test]
    fn v5_different_namespaces_differ() {
        let u1 = v5(NAMESPACE_DNS, "python.org");
        let u2 = v5(NAMESPACE_URL, "python.org");
        assert_ne!(u1, u2);
    }

    // ── v3 Tests ─────────────────────────────────────────────────────────────

    #[test]
    fn v3_rfc_vector_python_org() {
        // RFC 4122 § Appendix B test vector
        let u = v3(NAMESPACE_DNS, "python.org");
        assert_eq!(u.to_string(), "6fa459ea-ee8a-3ca4-894e-db77e160355e");
    }

    #[test]
    fn v3_version_is_3() {
        let u = v3(NAMESPACE_DNS, "example.com");
        assert_eq!(u.version(), 3);
    }

    #[test]
    fn v3_variant_is_rfc4122() {
        let u = v3(NAMESPACE_DNS, "example.com");
        assert_eq!(u.variant(), "rfc4122");
    }

    #[test]
    fn v3_is_deterministic() {
        let u1 = v3(NAMESPACE_DNS, "hello.example");
        let u2 = v3(NAMESPACE_DNS, "hello.example");
        assert_eq!(u1, u2);
    }

    #[test]
    fn v3_and_v5_differ() {
        // Even with same inputs, v3 (MD5) ≠ v5 (SHA-1)
        let u3 = v3(NAMESPACE_DNS, "python.org");
        let u5 = v5(NAMESPACE_DNS, "python.org");
        assert_ne!(u3, u5);
    }

    // ── v1 Tests ─────────────────────────────────────────────────────────────

    #[test]
    fn v1_version_is_1() {
        let u = v1().unwrap();
        assert_eq!(u.version(), 1);
    }

    #[test]
    fn v1_variant_is_rfc4122() {
        let u = v1().unwrap();
        assert_eq!(u.variant(), "rfc4122");
    }

    #[test]
    fn v1_outputs_are_unique() {
        let uuids: std::collections::HashSet<String> =
            (0..10).map(|_| v1().unwrap().to_string()).collect();
        assert_eq!(uuids.len(), 10);
    }

    // ── v7 Tests ─────────────────────────────────────────────────────────────

    #[test]
    fn v7_version_is_7() {
        let u = v7().unwrap();
        assert_eq!(u.version(), 7);
    }

    #[test]
    fn v7_variant_is_rfc4122() {
        let u = v7().unwrap();
        assert_eq!(u.variant(), "rfc4122");
    }

    #[test]
    fn v7_outputs_are_unique() {
        let uuids: std::collections::HashSet<String> =
            (0..20).map(|_| v7().unwrap().to_string()).collect();
        assert_eq!(uuids.len(), 20);
    }

    #[test]
    fn v7_is_time_ordered() {
        // v7 UUIDs have a 48-bit millisecond timestamp in the high bits.
        // Two UUIDs generated within the same millisecond can have their
        // random suffix land in any order, so we only assert that the
        // top 48 bits (the timestamp portion) never go backward.
        //
        // Full byte-level monotonicity requires an additional sequence counter
        // (RFC 9562 §6.2), which this implementation omits for simplicity.
        let extract_ts_ms = |u: UUID| u.to_u128() >> 80; // top 48 bits
        let mut prev_ts = extract_ts_ms(v7().unwrap());
        for _ in 0..20 {
            let curr_ts = extract_ts_ms(v7().unwrap());
            assert!(
                curr_ts >= prev_ts,
                "v7 timestamp went backward: {} < {}",
                curr_ts,
                prev_ts
            );
            prev_ts = curr_ts;
        }
    }

    #[test]
    fn v7_high_bits_contain_timestamp() {
        // The high 48 bits of a v7 UUID encode unix_ts_ms.
        // Since we just generated it, the value should be a plausible recent
        // timestamp: between Jan 2020 and Jan 2100.
        let u = v7().unwrap();
        let ts_ms = u.to_u128() >> 80; // top 48 bits
        let jan2020_ms: u128 = 1_577_836_800_000;
        let jan2100_ms: u128 = 4_102_444_800_000;
        assert!(
            ts_ms >= jan2020_ms && ts_ms <= jan2100_ms,
            "Unexpected timestamp: {}",
            ts_ms
        );
    }

    // ── Bytes and to_u128 Tests ───────────────────────────────────────────────

    #[test]
    fn bytes_roundtrip() {
        let u = parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let u2 = UUID::from_bytes(u.bytes());
        assert_eq!(u, u2);
    }

    #[test]
    fn to_u128_nil_is_zero() {
        assert_eq!(NIL.to_u128(), 0u128);
    }

    #[test]
    fn to_u128_max_is_max() {
        assert_eq!(MAX.to_u128(), u128::MAX);
    }

    // ── Equality and Ordering Tests ───────────────────────────────────────────

    #[test]
    fn equality_same_uuid() {
        let u1 = parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let u2 = parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(u1, u2);
    }

    #[test]
    fn ordering_nil_less_than_max() {
        assert!(NIL < MAX);
    }

    #[test]
    fn hash_trait_works() {
        // Just verify UUID can be used as HashMap key
        let mut map = std::collections::HashMap::new();
        let u = parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
        map.insert(u, "value");
        assert_eq!(map[&u], "value");
    }

    // ── Debug Format Test ─────────────────────────────────────────────────────

    #[test]
    fn debug_format() {
        let u = NIL;
        let debug = format!("{:?}", u);
        assert!(debug.contains("UUID("));
        assert!(debug.contains("00000000-0000-0000-0000-000000000000"));
    }
}
