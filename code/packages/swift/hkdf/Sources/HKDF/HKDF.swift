// ============================================================================
// HKDF — HMAC-based Extract-and-Expand Key Derivation Function (RFC 5869)
// ============================================================================
//
// What Is Key Derivation?
// =======================
// Many cryptographic protocols start with some "input keying material" (IKM)
// that is not directly suitable as a cryptographic key. The IKM might come
// from a Diffie-Hellman exchange, a password, a random seed, or any other
// source of entropy. A Key Derivation Function (KDF) transforms this raw
// material into one or more cryptographically strong keys.
//
// HKDF is the most widely used KDF in modern cryptography. It appears in:
//
//   - TLS 1.3 (the key schedule is built entirely on HKDF)
//   - Signal Protocol (Double Ratchet key derivation)
//   - WireGuard (handshake key derivation)
//   - Noise Protocol Framework
//   - Web Crypto API (deriveBits / deriveKey)
//
// Why Two Phases?
// ===============
// HKDF splits key derivation into two distinct phases:
//
//   1. **Extract** — concentrate the entropy from the IKM into a fixed-size
//      pseudorandom key (PRK). This step "cleans up" non-uniform input.
//
//   2. **Expand** — stretch the PRK into as many output bytes as needed,
//      using an "info" string for domain separation.
//
// Visual Overview
// ===============
//
//   Input Keying Material (IKM)
//          |
//          v
//   +--------------+
//   |   Extract    |  PRK = HMAC(salt, IKM)
//   |  (compress)  |
//   +--------------+
//          |
//          v
//   Pseudorandom Key (PRK)   [exactly HashLen bytes]
//          |
//          v
//   +--------------+
//   |   Expand     |  OKM = T(1) || T(2) || ... || T(N)
//   |  (stretch)   |  where T(i) = HMAC(PRK, T(i-1) || info || i)
//   +--------------+
//          |
//          v
//   Output Keying Material (OKM)  [L bytes, up to 255 * HashLen]
//
// ============================================================================

import Foundation
import HMAC

// ============================================================================
// Section 1: Hash Algorithm Configuration
// ============================================================================
//
// HKDF is parameterized by a hash function. We support SHA-256 (32-byte
// output) and SHA-512 (64-byte output). The hash algorithm determines:
//
//   - HashLen: the output size of the hash (and the PRK size)
//   - The maximum output length: 255 * HashLen bytes
//   - The default salt: HashLen zero bytes when no salt is provided
//
// ============================================================================

/// Supported hash algorithms for HKDF.
public enum HashAlgorithm {
    case sha256
    case sha512

    /// The output length of the hash function in bytes.
    /// SHA-256 produces 32 bytes; SHA-512 produces 64 bytes.
    var hashLength: Int {
        switch self {
        case .sha256: return 32
        case .sha512: return 64
        }
    }
}

/// Dispatch to the correct HMAC function based on the hash algorithm.
///
/// Each HMAC variant wraps its hash function per RFC 2104:
///   HMAC(K, M) = H((K ^ opad) || H((K ^ ipad) || M))
private func hmacFunction(_ algorithm: HashAlgorithm, key: Data, message: Data) -> Data {
    switch algorithm {
    case .sha256:
        return hmacSHA256(key: key, message: message)
    case .sha512:
        return hmacSHA512(key: key, message: message)
    }
}

// ============================================================================
// Section 2: HKDF-Extract
// ============================================================================
//
// The Extract phase takes potentially non-uniform input keying material (IKM)
// and produces a fixed-length pseudorandom key (PRK).
//
//   PRK = HMAC-Hash(salt, IKM)
//
// The salt is the HMAC key; IKM is the message. If salt is empty,
// we use HashLen zero bytes per RFC 5869 Section 2.2.
//
// ============================================================================

/// HKDF-Extract: compress input keying material into a pseudorandom key.
///
/// - Parameters:
///   - salt: Optional salt value (non-secret random value). Empty means
///           HashLen zero bytes.
///   - ikm: Input keying material — the raw secret to derive from.
///   - hash: Hash algorithm (default: `.sha256`).
/// - Returns: PRK — a pseudorandom key of HashLen bytes.
public func hkdfExtract(salt: Data, ikm: Data, hash: HashAlgorithm = .sha256) -> Data {
    // If the salt is empty, RFC 5869 says to use HashLen zero bytes.
    let effectiveSalt = salt.isEmpty ? Data(count: hash.hashLength) : salt
    return hmacFunction(hash, key: effectiveSalt, message: ikm)
}

// ============================================================================
// Section 3: HKDF-Expand
// ============================================================================
//
// The Expand phase takes the PRK and produces output keying material of any
// desired length, up to 255 * HashLen bytes.
//
//   T(0) = empty
//   T(1) = HMAC(PRK, T(0) || info || 0x01)
//   T(2) = HMAC(PRK, T(1) || info || 0x02)
//   ...
//   T(N) = HMAC(PRK, T(N-1) || info || N)
//
//   OKM = first L bytes of T(1) || ... || T(N)
//
// The counter is a single byte (1..255), so N <= 255.
//
// ============================================================================

/// Errors that can occur during HKDF operations.
public enum HKDFError: Error, CustomStringConvertible {
    case lengthTooSmall(Int)
    case lengthTooLarge(requested: Int, maximum: Int)

    public var description: String {
        switch self {
        case .lengthTooSmall(let length):
            return "HKDF-Expand: length must be > 0, got \(length)"
        case .lengthTooLarge(let requested, let maximum):
            return "HKDF-Expand: length \(requested) exceeds maximum \(maximum)"
        }
    }
}

/// HKDF-Expand: stretch a pseudorandom key into output keying material.
///
/// - Parameters:
///   - prk: Pseudorandom key (at least HashLen bytes, typically from hkdfExtract).
///   - info: Context and application-specific information (can be empty).
///   - length: Desired output length in bytes (1 to 255 * HashLen).
///   - hash: Hash algorithm (default: `.sha256`).
/// - Returns: OKM — output keying material of exactly `length` bytes.
/// - Throws: `HKDFError` if length is out of range.
public func hkdfExpand(prk: Data, info: Data, length: Int, hash: HashAlgorithm = .sha256) throws -> Data {
    let hashLen = hash.hashLength

    guard length > 0 else {
        throw HKDFError.lengthTooSmall(length)
    }

    let maxLength = 255 * hashLen
    guard length <= maxLength else {
        throw HKDFError.lengthTooLarge(requested: length, maximum: maxLength)
    }

    // N = ceil(L / HashLen)
    let n = (length + hashLen - 1) / hashLen

    // Build OKM by chaining HMAC blocks.
    // T(0) is the empty Data — used as "previous block" for the first iteration.
    var previous = Data()
    var okm = Data()
    okm.reserveCapacity(n * hashLen)

    for i in 1...n {
        // Build the HMAC input: T(i-1) || info || counter_byte
        // The counter is a single octet with value i (1-indexed).
        var input = Data(capacity: previous.count + info.count + 1)
        input.append(previous)
        input.append(info)
        input.append(UInt8(i))

        let block = hmacFunction(hash, key: prk, message: input)
        okm.append(block)

        // T(i) becomes the "previous" for the next iteration.
        previous = block
    }

    // Return exactly L bytes (truncating the last block if needed).
    return okm.prefix(length)
}

// ============================================================================
// Section 4: Combined HKDF (Extract + Expand)
// ============================================================================
//
// Most callers want the full HKDF pipeline: Extract then Expand. This
// convenience function chains both steps.
//
// ============================================================================

/// Full HKDF: extract-then-expand in one call.
///
/// - Parameters:
///   - salt: Optional salt (non-secret random value). Empty = HashLen zeros.
///   - ikm: Input keying material (the raw secret).
///   - info: Context/application info for domain separation.
///   - length: Desired output length in bytes.
///   - hash: Hash algorithm (default: `.sha256`).
/// - Returns: OKM — derived keying material of exactly `length` bytes.
/// - Throws: `HKDFError` if length is out of range.
public func hkdf(salt: Data, ikm: Data, info: Data, length: Int, hash: HashAlgorithm = .sha256) throws -> Data {
    let prk = hkdfExtract(salt: salt, ikm: ikm, hash: hash)
    return try hkdfExpand(prk: prk, info: info, length: length, hash: hash)
}
