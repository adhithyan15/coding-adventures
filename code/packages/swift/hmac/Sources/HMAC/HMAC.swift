// ============================================================================
// HMAC.swift — Hash-based Message Authentication Code
// RFC 2104 / FIPS 198-1
// ============================================================================
//
// What Is HMAC?
// =============
// HMAC takes a secret key and a message and produces a fixed-size
// authentication tag that proves two things simultaneously:
//
//   1. Integrity   — the message was not altered after the tag was created.
//   2. Authenticity — the tag creator possessed the secret key.
//
// Unlike a plain hash (which anyone can compute from a known message), an
// HMAC tag cannot be forged without the key. HMAC is used in:
//
//   - TLS 1.2 PRF (key expansion) and 1.3 HKDF
//   - JWT HS256 / HS512 signature algorithms
//   - WPA2 four-way handshake (PBKDF2-HMAC-SHA1)
//   - TOTP / HOTP one-time passwords (RFC 6238 / 4226)
//   - AWS Signature Version 4
//
// Why Not hash(key || message)?
// ==============================
// Naively prepending the key is vulnerable to the **length extension attack**
// on Merkle-Damgård hash functions (MD5, SHA-1, SHA-256, SHA-512).
//
// A Merkle-Damgård hash outputs its internal state directly as the digest.
// Anyone who knows hash(key || msg) knows the internal state after processing
// (key || msg). They can resume the hash and append more bytes — without
// knowing `key`:
//
//   attacker knows:    D = hash(key || msg)
//   attacker computes: hash(key || msg || padding || extra)
//                      by resuming from state D.
//
// HMAC defeats this with two hash calls under different derived keys:
//
//   HMAC(K, M) = H((K' ⊕ opad) || H((K' ⊕ ipad) || M))
//
// The outer hash wraps the inner result as a new message. An attacker
// cannot "resume" the outer hash without knowing K' ⊕ opad, which
// requires knowing K.
//
// The ipad and opad Constants
// ============================
//   ipad = 0x36 = 0011_0110  (inner pad)
//   opad = 0x5C = 0101_1100  (outer pad)
//
// They differ in exactly 4 bits — the maximum Hamming distance for single-byte
// values where both are XOR'd with the same key byte — making inner_key and
// outer_key as different as possible despite sharing source key K'.
//
// The Algorithm (RFC 2104 §2)
// ============================
//   1. Normalize K to block_size bytes:
//      - len(K) > block_size: K' = H(K), zero-pad to block_size
//      - len(K) ≤ block_size: zero-pad to block_size
//   2. inner_key = K' ⊕ (0x36 × block_size)
//   3. outer_key = K' ⊕ (0x5C × block_size)
//   4. inner     = H(inner_key + message)
//   5. return      H(outer_key + inner)
//
// Block Sizes
// ===========
//   MD5 / SHA-1 / SHA-256: 64-byte blocks
//   SHA-512:               128-byte blocks (64-bit word → 1024-bit schedule)
//
// ============================================================================

import Foundation
import MD5
import SHA1
import SHA256
import SHA512

// ─── ipad / opad constants ────────────────────────────────────────────────────

private let IPAD: UInt8 = 0x36
private let OPAD: UInt8 = 0x5C

// ─── Generic HMAC ─────────────────────────────────────────────────────────────

/// Compute HMAC using any hash function.
///
/// - Parameters:
///   - hashFn:    One-shot hash function: `Data -> Data`
///   - blockSize: Internal block size of `hashFn` in bytes (64 or 128)
///   - key:       Secret key, any length
///   - message:   Data to authenticate, any length
/// - Returns: Authentication tag as `Data` (same length as `hashFn` output)
///
/// Example:
/// ```swift
/// let tag = hmac(hashFn: sha256, blockSize: 64,
///                key: Data(repeating: 0x0b, count: 20),
///                message: Data("Hi There".utf8))
/// ```
public func hmac(hashFn: (Data) -> Data, blockSize: Int, key: Data, message: Data) -> Data {
    // Step 1 — normalize key to exactly blockSize bytes
    let keyPrime = normalizeKey(hashFn: hashFn, blockSize: blockSize, key: key)

    // Step 2 — derive inner and outer padded keys by XOR-ing with ipad / opad
    let innerKey = keyPrime.map { $0 ^ IPAD }
    let outerKey = keyPrime.map { $0 ^ OPAD }

    // Step 3 — nested hashes
    let inner = hashFn(Data(innerKey) + message)
    return hashFn(Data(outerKey) + inner)
}

// ─── Named variants ───────────────────────────────────────────────────────────

/// HMAC-MD5: 16-byte authentication tag (RFC 2202).
///
/// HMAC-MD5 remains secure as a MAC even though MD5 is broken for collision
/// resistance — MAC security and collision resistance are different properties.
/// It still appears in legacy TLS cipher suites.
public func hmacMD5(key: Data, message: Data) -> Data {
    precondition(!key.isEmpty, "HMAC key must not be empty")
    return hmac(hashFn: md5, blockSize: 64, key: key, message: message)
}

/// HMAC-SHA1: 20-byte authentication tag (RFC 2202).
///
/// Used in WPA2 (PBKDF2-HMAC-SHA1), older TLS/SSH, and TOTP/HOTP.
/// SHA-1 is collision-broken but HMAC-SHA1 remains secure as a MAC.
public func hmacSHA1(key: Data, message: Data) -> Data {
    precondition(!key.isEmpty, "HMAC key must not be empty")
    return hmac(hashFn: sha1, blockSize: 64, key: key, message: message)
}

/// HMAC-SHA256: 32-byte authentication tag (RFC 4231).
///
/// The modern default for TLS 1.3, JWT HS256, AWS Signature V4, and
/// PBKDF2-HMAC-SHA256. Uses the 64-byte SHA-256 block size.
public func hmacSHA256(key: Data, message: Data) -> Data {
    precondition(!key.isEmpty, "HMAC key must not be empty")
    return hmac(hashFn: sha256, blockSize: 64, key: key, message: message)
}

/// HMAC-SHA512: 64-byte authentication tag (RFC 4231).
///
/// Used in JWT HS512 and high-security configurations.
/// SHA-512 uses a 128-byte block (64-bit words, 1024-bit schedule),
/// so the ipad/opad key derivation uses 128 bytes.
public func hmacSHA512(key: Data, message: Data) -> Data {
    precondition(!key.isEmpty, "HMAC key must not be empty")
    return hmac(hashFn: sha512, blockSize: 128, key: key, message: message)
}

// ─── Hex-string variants ──────────────────────────────────────────────────────

/// HMAC-MD5 as a 32-character lowercase hex string.
public func hmacMD5Hex(key: Data, message: Data) -> String {
    hmacMD5(key: key, message: message).map { String(format: "%02x", $0) }.joined()
}

/// HMAC-SHA1 as a 40-character lowercase hex string.
public func hmacSHA1Hex(key: Data, message: Data) -> String {
    hmacSHA1(key: key, message: message).map { String(format: "%02x", $0) }.joined()
}

/// HMAC-SHA256 as a 64-character lowercase hex string.
public func hmacSHA256Hex(key: Data, message: Data) -> String {
    hmacSHA256(key: key, message: message).map { String(format: "%02x", $0) }.joined()
}

/// HMAC-SHA512 as a 128-character lowercase hex string.
public func hmacSHA512Hex(key: Data, message: Data) -> String {
    hmacSHA512(key: key, message: message).map { String(format: "%02x", $0) }.joined()
}

// ─── Constant-time tag verification ──────────────────────────────────────────

/// Compare two HMAC tags in constant time.
///
/// Use this instead of `==` when verifying a received HMAC tag against an
/// expected one. Swift's `Data ==` short-circuits on the first differing byte,
/// leaking timing information about how many bytes match. Over many requests
/// an attacker can exploit these timing differences to reconstruct the expected
/// tag — a **timing attack**.
///
/// This function XOR-accumulates all byte differences without short-circuiting.
/// The result is the same regardless of where the first mismatch occurs.
///
/// - Parameters:
///   - expected: The tag produced locally using the secret key
///   - actual:   The tag received from an untrusted source
/// - Returns: `true` iff `expected` and `actual` are byte-for-byte identical
public func hmacVerify(expected: Data, actual: Data) -> Bool {
    guard expected.count == actual.count else { return false }
    var diff: UInt8 = 0
    for (a, b) in zip(expected, actual) {
        diff |= a ^ b
    }
    return diff == 0
}

// ─── Private helpers ──────────────────────────────────────────────────────────

/// Normalize key to exactly `blockSize` bytes.
/// Long keys are hashed with `hashFn`. All keys are zero-padded on the right.
private func normalizeKey(hashFn: (Data) -> Data, blockSize: Int, key: Data) -> Data {
    let effective = key.count > blockSize ? hashFn(key) : key
    var result = Data(count: blockSize) // zero-initialized
    result.replaceSubrange(0 ..< min(effective.count, blockSize), with: effective.prefix(blockSize))
    return result
}
