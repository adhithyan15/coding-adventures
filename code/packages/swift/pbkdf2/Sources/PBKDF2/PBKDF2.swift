// ============================================================================
// PBKDF2.swift — PBKDF2 (Password-Based Key Derivation Function 2)
// RFC 8018 (formerly RFC 2898 / PKCS#5 v2.1)
// ============================================================================
//
// What Is PBKDF2?
// ===============
// PBKDF2 derives a cryptographic key from a password by applying a pseudorandom
// function (PRF) — typically HMAC — `c` times per output block. The iteration
// count `c` is the tunable cost: every brute-force guess requires the same `c`
// PRF calls as the original derivation.
//
// Real-world uses:
// - WPA2 Wi-Fi: PBKDF2-HMAC-SHA1, 4096 iterations
// - Django: PBKDF2-HMAC-SHA256, 720,000 iterations (2024)
// - macOS Keychain: PBKDF2-HMAC-SHA256
//
// Algorithm (RFC 8018 § 5.2)
// ===========================
//
//   DK = T_1 || T_2 || ... (first dkLen bytes)
//
//   T_i = U_1 XOR U_2 XOR ... XOR U_c
//
//   U_1 = PRF(Password, Salt || INT_32_BE(i))
//   U_j = PRF(Password, U_{j-1})   for j = 2..c
//
// INT_32_BE(i) encodes the block counter as a 4-byte big-endian integer
// appended to the salt. This makes each block's first U value unique.
//
// Security Notes
// ==============
// OWASP 2023 minimum iteration counts:
// - HMAC-SHA256: 600,000
// - HMAC-SHA1:   1,300,000
//
// For new systems prefer Argon2id (memory-hard, resists GPU attacks).
//
// ============================================================================

import Foundation
import HMAC

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

/// Errors that PBKDF2 functions can throw.
public enum PBKDF2Error: Error, Equatable {
    /// Password is empty — provides no entropy.
    case emptyPassword
    /// Iterations must be ≥ 1.
    case invalidIterations
    /// Key length must be ≥ 1.
    case invalidKeyLength
}

// ─────────────────────────────────────────────────────────────────────────────
// Core loop
// ─────────────────────────────────────────────────────────────────────────────

/// Generic PBKDF2 — used internally by all public convenience functions.
///
/// - Parameters:
///   - prf:        PRF(key, msg) → Data of length `hLen`
///   - hLen:       Output byte length of `prf`
///   - password:   Secret being stretched — becomes the HMAC key
///   - salt:       Unique random value per credential (≥16 bytes recommended)
///   - iterations: Number of PRF calls per block
///   - keyLength:  Number of derived bytes to produce
private func pbkdf2Core(
    prf: (Data, Data) -> Data,
    hLen: Int,
    password: Data,
    salt: Data,
    iterations: Int,
    keyLength: Int
) throws -> Data {
    guard !password.isEmpty else { throw PBKDF2Error.emptyPassword }
    guard iterations > 0 else { throw PBKDF2Error.invalidIterations }
    guard keyLength > 0 else { throw PBKDF2Error.invalidKeyLength }
    // Upper bounds prevent unbounded CPU/memory from attacker-controlled inputs,
    // and guard against arithmetic overflow in the ceiling computation below.
    // Swift's Int is 64-bit and traps on overflow by default, so we check first.
    guard iterations <= (1 << 31) else { throw PBKDF2Error.invalidIterations }
    guard keyLength <= (1 << 20) else { throw PBKDF2Error.invalidKeyLength }

    // Number of hLen-sized blocks needed.
    // Safe from overflow: keyLength ≤ 2^20 and hLen ≤ 64, so keyLength + hLen - 1 ≤ 2^20 + 63.
    let numBlocks = (keyLength + hLen - 1) / hLen
    var dk = Data(capacity: numBlocks * hLen)

    for i in 1...numBlocks {
        // Seed = Salt || INT_32_BE(i)
        // withUnsafeBytes writes the big-endian UInt32 bytes into a Data buffer.
        var blockIdx = UInt32(i).bigEndian
        let idxData = withUnsafeBytes(of: &blockIdx) { Data($0) }
        let seed = salt + idxData

        // U_1 = PRF(password, seed)
        var u = prf(password, seed)

        // t accumulates the XOR of all U values.
        var t = u

        // U_j = PRF(password, U_{j-1}), XOR into t.
        for _ in 1..<iterations {
            u = prf(password, u)
            for k in 0..<hLen {
                t[t.startIndex + k] ^= u[u.startIndex + k]
            }
        }

        dk.append(t)
    }

    return dk.prefix(keyLength)
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API — concrete PRF variants
// ─────────────────────────────────────────────────────────────────────────────

/// PBKDF2 with HMAC-SHA1 as the PRF.
///
/// `hLen` = 20 bytes (160-bit SHA-1 output).
/// Used in WPA2 (4096 iterations). For new systems prefer `pbkdf2HmacSHA256`.
///
/// RFC 6070 test vector:
/// ```swift
/// let dk = try pbkdf2HmacSHA1(
///     password: Data("password".utf8),
///     salt: Data("salt".utf8),
///     iterations: 1,
///     keyLength: 20
/// )
/// // dk.map { String(format: "%02x", $0) }.joined()
/// // → "0c60c80f961f0e71f3a9b524af6012062fe037a6"
/// ```
public func pbkdf2HmacSHA1(
    password: Data,
    salt: Data,
    iterations: Int,
    keyLength: Int
) throws -> Data {
    try pbkdf2Core(
        prf: { key, msg in hmacSHA1(key: key, message: msg) },
        hLen: 20,
        password: password,
        salt: salt,
        iterations: iterations,
        keyLength: keyLength
    )
}

/// PBKDF2 with HMAC-SHA256 as the PRF.
///
/// `hLen` = 32 bytes (256-bit SHA-256 output).
/// Recommended for new systems (OWASP 2023: ≥ 600,000 iterations).
public func pbkdf2HmacSHA256(
    password: Data,
    salt: Data,
    iterations: Int,
    keyLength: Int
) throws -> Data {
    try pbkdf2Core(
        prf: { key, msg in hmacSHA256(key: key, message: msg) },
        hLen: 32,
        password: password,
        salt: salt,
        iterations: iterations,
        keyLength: keyLength
    )
}

/// PBKDF2 with HMAC-SHA512 as the PRF.
///
/// `hLen` = 64 bytes (512-bit SHA-512 output).
/// Suitable for high-security applications.
public func pbkdf2HmacSHA512(
    password: Data,
    salt: Data,
    iterations: Int,
    keyLength: Int
) throws -> Data {
    try pbkdf2Core(
        prf: { key, msg in hmacSHA512(key: key, message: msg) },
        hLen: 64,
        password: password,
        salt: salt,
        iterations: iterations,
        keyLength: keyLength
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Hex convenience variants
// ─────────────────────────────────────────────────────────────────────────────

private func toHex(_ data: Data) -> String {
    data.map { String(format: "%02x", $0) }.joined()
}

/// Like `pbkdf2HmacSHA1` but returns a lowercase hex string.
public func pbkdf2HmacSHA1Hex(
    password: Data, salt: Data, iterations: Int, keyLength: Int
) throws -> String {
    toHex(try pbkdf2HmacSHA1(password: password, salt: salt, iterations: iterations, keyLength: keyLength))
}

/// Like `pbkdf2HmacSHA256` but returns a lowercase hex string.
public func pbkdf2HmacSHA256Hex(
    password: Data, salt: Data, iterations: Int, keyLength: Int
) throws -> String {
    toHex(try pbkdf2HmacSHA256(password: password, salt: salt, iterations: iterations, keyLength: keyLength))
}

/// Like `pbkdf2HmacSHA512` but returns a lowercase hex string.
public func pbkdf2HmacSHA512Hex(
    password: Data, salt: Data, iterations: Int, keyLength: Int
) throws -> String {
    toHex(try pbkdf2HmacSHA512(password: password, salt: salt, iterations: iterations, keyLength: keyLength))
}
