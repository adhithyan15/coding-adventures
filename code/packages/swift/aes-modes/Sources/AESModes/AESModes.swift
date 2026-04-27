// AESModes.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// AES Modes of Operation — ECB, CBC, CTR, GCM
// ============================================================================
//
// A block cipher like AES operates on fixed 128-bit (16-byte) blocks. To
// encrypt messages of arbitrary length, you need a **mode of operation** —
// a recipe that describes how to chain multiple block-cipher calls together.
//
// The choice of mode is critical for security:
//
//   Mode   | Security          | Properties
//   -------|-------------------|--------------------------------------------
//   ECB    | BROKEN            | Each block encrypted independently.
//          |                   | Identical plaintext → identical ciphertext.
//   -------|-------------------|--------------------------------------------
//   CBC    | Legacy (padding   | XOR with previous ciphertext before encrypt.
//          | oracle attacks)   | Requires unpredictable IV.
//   -------|-------------------|--------------------------------------------
//   CTR    | Modern, secure    | Stream cipher: encrypt counter, XOR plaintext.
//          |                   | Parallelizable, no padding needed.
//   -------|-------------------|--------------------------------------------
//   GCM    | Modern, secure +  | CTR + GHASH authentication tag.
//          | authenticated     | Gold standard for TLS 1.3.
//
// ============================================================================
// Public API
// ============================================================================
//
//   AESModes.ecbEncrypt(plaintext, key:) -> [UInt8]
//   AESModes.ecbDecrypt(ciphertext, key:) -> [UInt8]
//   AESModes.cbcEncrypt(plaintext, key:, iv:) -> [UInt8]
//   AESModes.cbcDecrypt(ciphertext, key:, iv:) -> [UInt8]
//   AESModes.ctrEncrypt(plaintext, key:, nonce:) -> [UInt8]
//   AESModes.ctrDecrypt(ciphertext, key:, nonce:) -> [UInt8]
//   AESModes.gcmEncrypt(plaintext, key:, iv:, aad:) -> (ciphertext, tag)
//   AESModes.gcmDecrypt(ciphertext, key:, iv:, aad:, tag:) -> [UInt8]

import AES

// ============================================================================
// MARK: - Constants
// ============================================================================

/// AES block size in bytes. AES always operates on 128-bit = 16-byte blocks.
private let blockSize = 16

// ============================================================================
// MARK: - Error Types
// ============================================================================

/// Errors that can occur during AES mode operations.
public enum AESModesError: Error, Sendable {
    case invalidPaddedData(String)
    case invalidPaddingValue(Int)
    case inconsistentPadding
    case invalidIVLength(Int)
    case invalidNonceLength(Int)
    case invalidTagLength(Int)
    case invalidCiphertextLength
    case authenticationFailed
}

// ============================================================================
// MARK: - AESModes Namespace
// ============================================================================
//
// All public functions are grouped under the AESModes enum (used as a
// namespace, since it has no cases). This follows the Swift convention
// of using caseless enums as namespaces (like Never or Optional).

/// AES modes of operation: ECB, CBC, CTR, GCM.
///
/// Each mode wraps the AES block cipher from the `AES` package to provide
/// different security properties for encrypting messages longer than 16 bytes.
public enum AESModes {

    // ========================================================================
    // MARK: - PKCS#7 Padding
    // ========================================================================
    //
    // Block ciphers need input that is an exact multiple of the block size.
    // PKCS#7 padding fills the gap:
    //   - N bytes short → append N copies of byte N
    //   - Already aligned → append full block of 16 bytes each = 0x10
    //
    // To unpad: read last byte N, verify last N bytes all equal N, strip.

    /// Apply PKCS#7 padding. Adds 1–16 bytes so the result is block-aligned.
    public static func pkcs7Pad(_ data: [UInt8]) -> [UInt8] {
        let padLen = blockSize - (data.count % blockSize)
        return data + [UInt8](repeating: UInt8(padLen), count: padLen)
    }

    /// Remove PKCS#7 padding. Throws if the padding is invalid.
    public static func pkcs7Unpad(_ data: [UInt8]) throws -> [UInt8] {
        guard !data.isEmpty, data.count % blockSize == 0 else {
            throw AESModesError.invalidPaddedData(
                "Length must be a positive multiple of 16, got \(data.count)")
        }
        let padLen = Int(data.last!)
        guard padLen >= 1, padLen <= blockSize else {
            throw AESModesError.invalidPaddingValue(padLen)
        }
        // Constant-time padding validation: accumulate differences with OR
        // instead of returning early on the first mismatch (prevents timing attacks)
        var padDiff: UInt8 = 0
        for i in (data.count - padLen)..<data.count {
            padDiff |= data[i] ^ UInt8(padLen)
        }
        guard padDiff == 0 else {
            throw AESModesError.inconsistentPadding
        }
        return Array(data[..<(data.count - padLen)])
    }

    // ========================================================================
    // MARK: - ECB (Electronic Codebook) — INSECURE
    // ========================================================================
    //
    // Encrypt each 16-byte block independently:
    //   C[i] = AES_encrypt(P[i], key)
    //
    // *** INSECURE: identical plaintext blocks → identical ciphertext ***
    // The "ECB penguin" demonstrates this — image structure leaks through.

    /// Encrypt with AES in ECB mode (INSECURE — educational only).
    public static func ecbEncrypt(_ plaintext: [UInt8], key: [UInt8]) -> [UInt8] {
        let padded = pkcs7Pad(plaintext)
        var result = [UInt8]()
        result.reserveCapacity(padded.count)

        for i in stride(from: 0, to: padded.count, by: blockSize) {
            let block = Array(padded[i..<(i + blockSize)])
            let encrypted = aesEncryptBlock(block, key: key)
            result.append(contentsOf: encrypted)
        }
        return result
    }

    /// Decrypt with AES in ECB mode (INSECURE — educational only).
    public static func ecbDecrypt(_ ciphertext: [UInt8], key: [UInt8]) throws -> [UInt8] {
        guard !ciphertext.isEmpty, ciphertext.count % blockSize == 0 else {
            throw AESModesError.invalidCiphertextLength
        }
        var result = [UInt8]()
        result.reserveCapacity(ciphertext.count)

        for i in stride(from: 0, to: ciphertext.count, by: blockSize) {
            let block = Array(ciphertext[i..<(i + blockSize)])
            let decrypted = aesDecryptBlock(block, key: key)
            result.append(contentsOf: decrypted)
        }
        return try pkcs7Unpad(result)
    }

    // ========================================================================
    // MARK: - CBC (Cipher Block Chaining) — Legacy
    // ========================================================================
    //
    // Chains blocks:
    //   C[0] = AES_encrypt(P[0] XOR IV, key)
    //   C[i] = AES_encrypt(P[i] XOR C[i-1], key)
    //
    // IV must be unpredictable and never reused with the same key.
    // Vulnerable to padding oracle attacks (POODLE, Lucky 13).

    /// Encrypt with AES in CBC mode.
    public static func cbcEncrypt(
        _ plaintext: [UInt8], key: [UInt8], iv: [UInt8]
    ) throws -> [UInt8] {
        guard iv.count == blockSize else {
            throw AESModesError.invalidIVLength(iv.count)
        }
        let padded = pkcs7Pad(plaintext)
        var result = [UInt8]()
        result.reserveCapacity(padded.count)
        var prev = iv

        for i in stride(from: 0, to: padded.count, by: blockSize) {
            let block = Array(padded[i..<(i + blockSize)])
            let xored = xorBytes(block, prev)
            let encrypted = aesEncryptBlock(xored, key: key)
            result.append(contentsOf: encrypted)
            prev = encrypted
        }
        return result
    }

    /// Decrypt with AES in CBC mode.
    public static func cbcDecrypt(
        _ ciphertext: [UInt8], key: [UInt8], iv: [UInt8]
    ) throws -> [UInt8] {
        guard iv.count == blockSize else {
            throw AESModesError.invalidIVLength(iv.count)
        }
        guard !ciphertext.isEmpty, ciphertext.count % blockSize == 0 else {
            throw AESModesError.invalidCiphertextLength
        }
        var result = [UInt8]()
        result.reserveCapacity(ciphertext.count)
        var prev = iv

        for i in stride(from: 0, to: ciphertext.count, by: blockSize) {
            let block = Array(ciphertext[i..<(i + blockSize)])
            let decrypted = aesDecryptBlock(block, key: key)
            let plain = xorBytes(decrypted, prev)
            result.append(contentsOf: plain)
            prev = block
        }
        return try pkcs7Unpad(result)
    }

    // ========================================================================
    // MARK: - CTR (Counter Mode) — Modern
    // ========================================================================
    //
    // Stream cipher: encrypt counter, XOR with plaintext:
    //   keystream[i] = AES_encrypt(nonce || counter_i, key)
    //   C[i] = P[i] XOR keystream[i]
    //
    // Nonce is 12 bytes, counter is 4-byte big-endian starting at 1.
    // No padding needed. Encryption = decryption.
    //
    // CRITICAL: Never reuse a nonce with the same key.

    /// Encrypt with AES in CTR mode.
    public static func ctrEncrypt(
        _ plaintext: [UInt8], key: [UInt8], nonce: [UInt8]
    ) throws -> [UInt8] {
        guard nonce.count == 12 else {
            throw AESModesError.invalidNonceLength(nonce.count)
        }
        var result = [UInt8]()
        result.reserveCapacity(plaintext.count)
        var counter: UInt32 = 1

        var offset = 0
        while offset < plaintext.count {
            let counterBlock = buildCounterBlock(nonce: nonce, counter: counter)
            let keystream = aesEncryptBlock(counterBlock, key: key)
            let remaining = min(blockSize, plaintext.count - offset)
            for j in 0..<remaining {
                result.append(plaintext[offset + j] ^ keystream[j])
            }
            counter &+= 1
            offset += blockSize
        }
        return result
    }

    /// Decrypt with AES in CTR mode. Same as encryption (stream cipher property).
    public static func ctrDecrypt(
        _ ciphertext: [UInt8], key: [UInt8], nonce: [UInt8]
    ) throws -> [UInt8] {
        return try ctrEncrypt(ciphertext, key: key, nonce: nonce)
    }

    // ========================================================================
    // MARK: - GCM (Galois/Counter Mode) — Authenticated
    // ========================================================================
    //
    // CTR encryption + GHASH authentication tag. Provides AEAD:
    //   1. H = AES_encrypt(0^128, key)
    //   2. J0 = IV || 0x00000001
    //   3. CTR-encrypt plaintext at J0+1
    //   4. Tag = GHASH(H, AAD, CT) XOR AES_encrypt(J0, key)

    /// Encrypt with AES-GCM. Returns (ciphertext, 16-byte tag).
    public static func gcmEncrypt(
        _ plaintext: [UInt8],
        key: [UInt8],
        iv: [UInt8],
        aad: [UInt8] = []
    ) throws -> (ciphertext: [UInt8], tag: [UInt8]) {
        guard iv.count == 12 else {
            throw AESModesError.invalidIVLength(iv.count)
        }

        // Step 1: H = AES_encrypt(0^128, key)
        let h = aesEncryptBlock([UInt8](repeating: 0, count: 16), key: key)

        // Step 2: J0 = IV || 0x00000001
        var j0 = [UInt8](repeating: 0, count: 16)
        for i in 0..<12 { j0[i] = iv[i] }
        j0[15] = 1

        // Step 3: CTR-encrypt starting at J0+1
        var ciphertext = [UInt8]()
        ciphertext.reserveCapacity(plaintext.count)
        var counter = j0
        var offset = 0
        while offset < plaintext.count {
            counter = incrementCounter(counter)
            let keystream = aesEncryptBlock(counter, key: key)
            let remaining = min(blockSize, plaintext.count - offset)
            for j in 0..<remaining {
                ciphertext.append(plaintext[offset + j] ^ keystream[j])
            }
            offset += blockSize
        }

        // Step 4: Tag = GHASH(H, AAD, CT) XOR AES_encrypt(J0, key)
        let ghashResult = ghash(h: h, aad: aad, ciphertext: ciphertext)
        let encJ0 = aesEncryptBlock(j0, key: key)
        let tag = xorBytes(ghashResult, encJ0)

        return (ciphertext, tag)
    }

    /// Decrypt with AES-GCM. Verifies tag before returning plaintext.
    public static func gcmDecrypt(
        _ ciphertext: [UInt8],
        key: [UInt8],
        iv: [UInt8],
        aad: [UInt8],
        tag: [UInt8]
    ) throws -> [UInt8] {
        guard iv.count == 12 else {
            throw AESModesError.invalidIVLength(iv.count)
        }
        guard tag.count == 16 else {
            throw AESModesError.invalidTagLength(tag.count)
        }

        // Compute H and J0
        let h = aesEncryptBlock([UInt8](repeating: 0, count: 16), key: key)
        var j0 = [UInt8](repeating: 0, count: 16)
        for i in 0..<12 { j0[i] = iv[i] }
        j0[15] = 1

        // Verify tag BEFORE decrypting (constant-time comparison)
        let ghashResult = ghash(h: h, aad: aad, ciphertext: ciphertext)
        let encJ0 = aesEncryptBlock(j0, key: key)
        var diff: UInt8 = 0
        for i in 0..<16 {
            diff |= (ghashResult[i] ^ encJ0[i]) ^ tag[i]
        }
        guard diff == 0 else {
            throw AESModesError.authenticationFailed
        }

        // CTR-decrypt
        var plaintext = [UInt8]()
        plaintext.reserveCapacity(ciphertext.count)
        var counter = j0
        var offset = 0
        while offset < ciphertext.count {
            counter = incrementCounter(counter)
            let keystream = aesEncryptBlock(counter, key: key)
            let remaining = min(blockSize, ciphertext.count - offset)
            for j in 0..<remaining {
                plaintext.append(ciphertext[offset + j] ^ keystream[j])
            }
            offset += blockSize
        }
        return plaintext
    }
}

// ============================================================================
// MARK: - Private Helpers
// ============================================================================

/// XOR two byte arrays of equal length.
private func xorBytes(_ a: [UInt8], _ b: [UInt8]) -> [UInt8] {
    return zip(a, b).map { $0 ^ $1 }
}

/// Build a 16-byte CTR counter block: [nonce (12 bytes)] [counter (4 bytes BE)]
private func buildCounterBlock(nonce: [UInt8], counter: UInt32) -> [UInt8] {
    var block = [UInt8](repeating: 0, count: 16)
    for i in 0..<12 { block[i] = nonce[i] }
    block[12] = UInt8((counter >> 24) & 0xFF)
    block[13] = UInt8((counter >> 16) & 0xFF)
    block[14] = UInt8((counter >> 8) & 0xFF)
    block[15] = UInt8(counter & 0xFF)
    return block
}

/// Multiply two 128-bit values in GF(2^128) with GCM reducing polynomial.
///
/// The reducing polynomial R has high byte 0xE1:
///   R = x^128 + x^7 + x^2 + x + 1
///
/// Algorithm (NIST SP 800-38D, Algorithm 1):
///   Z = 0, V = Y
///   for each bit i of X (MSB first):
///     if bit i set: Z ^= V
///     carry = LSB of V
///     V >>= 1
///     if carry: V[0] ^= 0xE1
private func gf128Mul(_ x: [UInt8], _ y: [UInt8]) -> [UInt8] {
    var z = [UInt8](repeating: 0, count: 16)
    var v = y

    for i in 0..<128 {
        let byteIdx = i / 8
        let bitIdx = 7 - (i % 8)
        if (x[byteIdx] >> bitIdx) & 1 == 1 {
            for j in 0..<16 { z[j] ^= v[j] }
        }

        let carry = v[15] & 1

        // Right-shift V by 1 bit
        var j = 15
        while j > 0 {
            v[j] = (v[j] >> 1) | ((v[j - 1] & 1) << 7)
            j -= 1
        }
        v[0] >>= 1

        if carry == 1 {
            v[0] ^= 0xE1
        }
    }
    return z
}

/// GHASH: universal hash function for GCM.
///
/// Processes AAD and ciphertext through GF(2^128) polynomial evaluation:
///   Y[0] = 0^128
///   Y[i] = (Y[i-1] XOR block[i]) * H
private func ghash(h: [UInt8], aad: [UInt8], ciphertext: [UInt8]) -> [UInt8] {
    var y = [UInt8](repeating: 0, count: 16)

    // Process data in 16-byte blocks with zero-padding on the last block
    func processBlocks(_ data: [UInt8]) {
        var offset = 0
        while offset < data.count {
            var block = [UInt8](repeating: 0, count: 16)
            let remaining = min(16, data.count - offset)
            for j in 0..<remaining {
                block[j] = data[offset + j]
            }
            let xored = xorBytes(y, block)
            y = gf128Mul(xored, h)
            offset += 16
        }
    }

    if !aad.isEmpty { processBlocks(aad) }
    if !ciphertext.isEmpty { processBlocks(ciphertext) }

    // Length block: [len(AAD)*8 as u64 BE || len(CT)*8 as u64 BE]
    var lenBlock = [UInt8](repeating: 0, count: 16)
    let aadBits = UInt64(aad.count) * 8
    let ctBits = UInt64(ciphertext.count) * 8
    // AAD length in bits (big-endian u64, bytes 0-7)
    let aadBitsBytes = withUnsafeBytes(of: aadBits.bigEndian) { Array($0) }
    let ctBitsBytes = withUnsafeBytes(of: ctBits.bigEndian) { Array($0) }
    for i in 0..<8 { lenBlock[i] = aadBitsBytes[i] }
    for i in 0..<8 { lenBlock[8 + i] = ctBitsBytes[i] }

    let xored = xorBytes(y, lenBlock)
    y = gf128Mul(xored, h)

    return y
}

/// Increment the 32-bit counter in the last 4 bytes of a 16-byte block (BE).
private func incrementCounter(_ block: [UInt8]) -> [UInt8] {
    var result = block
    var i = 15
    while i >= 12 {
        result[i] = result[i] &+ 1
        if result[i] != 0 { break }
        i -= 1
    }
    return result
}

// ============================================================================
// MARK: - Hex Utilities
// ============================================================================

/// Convert a hex string to a byte array.
public func fromHex(_ hex: String) -> [UInt8] {
    let chars = Array(hex)
    var bytes = [UInt8]()
    bytes.reserveCapacity(chars.count / 2)
    var i = 0
    while i < chars.count {
        let hi = UInt8(String(chars[i]), radix: 16)!
        let lo = UInt8(String(chars[i + 1]), radix: 16)!
        bytes.append((hi << 4) | lo)
        i += 2
    }
    return bytes
}

/// Convert a byte array to a lowercase hex string.
public func toHex(_ bytes: [UInt8]) -> String {
    let hexChars: [Character] = Array("0123456789abcdef")
    var result = ""
    result.reserveCapacity(bytes.count * 2)
    for b in bytes {
        result.append(hexChars[Int(b >> 4)])
        result.append(hexChars[Int(b & 0x0F)])
    }
    return result
}
