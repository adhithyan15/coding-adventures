// ============================================================================
// ChaCha20-Poly1305 — Authenticated Encryption (RFC 8439)
// ============================================================================
//
// This module implements ChaCha20-Poly1305 AEAD from scratch in pure Swift.
// It combines two primitives designed by Daniel J. Bernstein:
//
//   1. **ChaCha20** — a stream cipher built from ARX (Add, Rotate, XOR)
//      operations on 32-bit words. No lookup tables, no timing side channels.
//
//   2. **Poly1305** — a one-time MAC that computes a 16-byte authentication
//      tag using polynomial evaluation modulo the prime 2^130 - 5.
//
// Together they form an Authenticated Encryption with Associated Data (AEAD)
// scheme used in TLS 1.3, WireGuard, SSH, and Chrome/Android.
//
// ## Why ChaCha20 instead of AES?
//
// AES relies on hardware AES-NI instructions for speed and resistance to
// cache-timing attacks. ChaCha20 is fast in *pure software* on any CPU —
// it uses only additions, rotations, and XORs (no S-boxes, no lookup tables).
//
// ============================================================================

// ============================================================================
// Section 1: ChaCha20 Stream Cipher
// ============================================================================
//
// ChaCha20 generates a pseudorandom keystream by scrambling a 4x4 matrix
// of 32-bit words through 20 rounds of quarter-round operations.
//
// The state matrix layout:
//
//   ┌──────────┬──────────┬──────────┬──────────┐
//   │ const[0] │ const[1] │ const[2] │ const[3] │  "expand 32-byte k"
//   ├──────────┼──────────┼──────────┼──────────┤
//   │ key[0]   │ key[1]   │ key[2]   │ key[3]   │  256-bit key
//   ├──────────┼──────────┼──────────┼──────────┤
//   │ key[4]   │ key[5]   │ key[6]   │ key[7]   │  (continued)
//   ├──────────┼──────────┼──────────┼──────────┤
//   │ counter  │ nonce[0] │ nonce[1] │ nonce[2] │  32-bit counter + 96-bit nonce
//   └──────────┴──────────┴──────────┴──────────┘
//
// ============================================================================

/// The ChaCha20-Poly1305 authenticated encryption suite.
///
/// This is the main entry point for all ChaCha20-Poly1305 operations.
/// All methods are static — no instance state is needed.
public enum ChaCha20Poly1305 {

    // MARK: - Constants

    /// The four 32-bit constants that form the first row of the ChaCha20 state.
    /// They spell out "expand 32-byte k" in ASCII — a nothing-up-my-sleeve number
    /// chosen by Bernstein to fill the state deterministically.
    private static let constants: [UInt32] = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574]

    // MARK: - Byte Manipulation Helpers

    /// Read a 32-bit little-endian word from a byte array.
    ///
    /// ChaCha20 works with 32-bit words stored in little-endian byte order.
    /// For example, bytes [0x04, 0x03, 0x02, 0x01] become the word 0x01020304.
    private static func readU32LE(_ data: [UInt8], offset: Int) -> UInt32 {
        return UInt32(data[offset])
            | (UInt32(data[offset + 1]) << 8)
            | (UInt32(data[offset + 2]) << 16)
            | (UInt32(data[offset + 3]) << 24)
    }

    /// Write a 32-bit little-endian word into a byte array.
    private static func writeU32LE(_ data: inout [UInt8], offset: Int, value: UInt32) {
        data[offset]     = UInt8(value & 0xFF)
        data[offset + 1] = UInt8((value >> 8) & 0xFF)
        data[offset + 2] = UInt8((value >> 16) & 0xFF)
        data[offset + 3] = UInt8((value >> 24) & 0xFF)
    }

    // MARK: - ChaCha20 Quarter Round

    /// The ChaCha20 quarter round — the fundamental mixing operation.
    ///
    /// It takes four 32-bit words (a, b, c, d) from the state and scrambles
    /// them through four ARX steps:
    ///
    ///   a += b;  d ^= a;  d <<<= 16    (big rotation — coarse mixing)
    ///   c += d;  b ^= c;  b <<<= 12    (medium rotation)
    ///   a += b;  d ^= a;  d <<<= 8     (small rotation)
    ///   c += d;  b ^= c;  b <<<= 7     (smallest rotation)
    ///
    /// The rotation amounts (16, 12, 8, 7) were chosen by Bernstein to
    /// maximize diffusion. Swift's `&+` operator gives wrapping addition.
    private static func quarterRound(
        _ state: inout [UInt32], _ a: Int, _ b: Int, _ c: Int, _ d: Int
    ) {
        state[a] = state[a] &+ state[b]
        state[d] ^= state[a]
        state[d] = (state[d] << 16) | (state[d] >> 16)

        state[c] = state[c] &+ state[d]
        state[b] ^= state[c]
        state[b] = (state[b] << 12) | (state[b] >> 20)

        state[a] = state[a] &+ state[b]
        state[d] ^= state[a]
        state[d] = (state[d] << 8) | (state[d] >> 24)

        state[c] = state[c] &+ state[d]
        state[b] ^= state[c]
        state[b] = (state[b] << 7) | (state[b] >> 25)
    }

    // MARK: - ChaCha20 Block Function

    /// Generate one 64-byte ChaCha20 keystream block.
    ///
    /// The 20 rounds consist of 10 iterations, each performing:
    ///   - 4 "column" quarter rounds (down the columns of the 4x4 matrix)
    ///   - 4 "diagonal" quarter rounds (along the diagonals)
    ///
    /// After all rounds, the original state is added back (mod 2^32) to
    /// prevent an attacker from inverting the rounds.
    private static func chacha20Block(
        key: [UInt8], nonce: [UInt8], counter: UInt32
    ) -> [UInt8] {
        // Initialize the 4x4 state matrix
        var state = [UInt32](repeating: 0, count: 16)

        // Row 0: constants
        state[0] = constants[0]
        state[1] = constants[1]
        state[2] = constants[2]
        state[3] = constants[3]

        // Row 1-2: key (8 words = 32 bytes)
        for i in 0..<8 {
            state[4 + i] = readU32LE(key, offset: i * 4)
        }

        // Row 3: counter + nonce
        state[12] = counter
        state[13] = readU32LE(nonce, offset: 0)
        state[14] = readU32LE(nonce, offset: 4)
        state[15] = readU32LE(nonce, offset: 8)

        // Save original state for the final addition
        let original = state

        // 20 rounds (10 double-rounds)
        for _ in 0..<10 {
            // Column rounds
            quarterRound(&state, 0, 4,  8, 12)
            quarterRound(&state, 1, 5,  9, 13)
            quarterRound(&state, 2, 6, 10, 14)
            quarterRound(&state, 3, 7, 11, 15)

            // Diagonal rounds
            quarterRound(&state, 0, 5, 10, 15)
            quarterRound(&state, 1, 6, 11, 12)
            quarterRound(&state, 2, 7,  8, 13)
            quarterRound(&state, 3, 4,  9, 14)
        }

        // Add original state back
        for i in 0..<16 {
            state[i] = state[i] &+ original[i]
        }

        // Serialize to 64 bytes (little-endian)
        var output = [UInt8](repeating: 0, count: 64)
        for i in 0..<16 {
            writeU32LE(&output, offset: i * 4, value: state[i])
        }

        return output
    }

    // MARK: - ChaCha20 Encrypt

    /// ChaCha20 stream cipher encryption (and decryption — they're the same).
    ///
    /// ChaCha20 is a stream cipher: it generates a pseudorandom keystream
    /// and XORs it with the plaintext. Since XOR is its own inverse,
    /// encryption and decryption are the same operation.
    ///
    /// - Parameters:
    ///   - plaintext: The data to encrypt (or ciphertext to decrypt).
    ///   - key: 32-byte (256-bit) secret key.
    ///   - nonce: 12-byte (96-bit) nonce (number used once).
    ///   - counter: Starting block counter (usually 0 or 1).
    /// - Returns: The ciphertext (or plaintext if decrypting).
    public static func chacha20Encrypt(
        plaintext: [UInt8], key: [UInt8], nonce: [UInt8], counter: UInt32
    ) -> [UInt8] {
        precondition(key.count == 32, "Key must be 32 bytes")
        precondition(nonce.count == 12, "Nonce must be 12 bytes")

        var output = [UInt8](repeating: 0, count: plaintext.count)
        var offset = 0
        var currentCounter = counter

        while offset < plaintext.count {
            let block = chacha20Block(key: key, nonce: nonce, counter: currentCounter)
            currentCounter = currentCounter &+ 1

            let remaining = plaintext.count - offset
            let bytesToProcess = min(64, remaining)

            for i in 0..<bytesToProcess {
                output[offset + i] = plaintext[offset + i] ^ block[i]
            }

            offset += bytesToProcess
        }

        return output
    }

    // MARK: - Poly1305 MAC
    // ========================================================================
    //
    // Poly1305 is a one-time MAC: given a 32-byte key and a message, it
    // produces a 16-byte authentication tag.
    //
    // The math: treat 16-byte message chunks as numbers and evaluate a
    // polynomial modulo p = 2^130 - 5:
    //
    //   acc = 0
    //   for each chunk c:
    //     acc = (acc + c_with_0x01) * r  mod  p
    //   tag = (acc + s) mod 2^128
    //
    // Swift doesn't have native 130-bit integers, so we use a 5-limb
    // representation with 26 bits per limb. This is the standard approach
    // used in production Poly1305 implementations — each limb fits in a
    // UInt32, and products of two limbs fit in a UInt64, avoiding all
    // overflow issues.
    //
    // ========================================================================

    /// Compute a Poly1305 MAC tag for a message.
    ///
    /// The key is split into two halves:
    ///   - r (bytes 0-15): the multiplier, with certain bits "clamped" to zero
    ///   - s (bytes 16-31): added at the end to hide the internal state
    ///
    /// Clamping r ensures it has a specific algebraic structure that makes
    /// the MAC provably secure.
    ///
    /// We use a radix-2^26 representation with 5 limbs for all 130-bit values.
    /// This means each "digit" holds up to 26 bits (values 0 to 2^26 - 1).
    /// The advantage: 26-bit * 26-bit = 52-bit, which fits in UInt64 with
    /// plenty of room for accumulation without overflow.
    ///
    /// - Parameters:
    ///   - message: The data to authenticate.
    ///   - key: 32-byte one-time key (NEVER reuse!).
    /// - Returns: 16-byte authentication tag.
    public static func poly1305Mac(message: [UInt8], key: [UInt8]) -> [UInt8] {
        precondition(key.count == 32, "Key must be 32 bytes")

        // --- Extract and clamp r ---
        var rBytes = Array(key[0..<16])
        rBytes[3]  &= 0x0F
        rBytes[7]  &= 0x0F
        rBytes[11] &= 0x0F
        rBytes[15] &= 0x0F
        rBytes[4]  &= 0xFC
        rBytes[8]  &= 0xFC
        rBytes[12] &= 0xFC

        // Convert r to 5 limbs of 26 bits each (radix 2^26)
        let r0 = (readU32LE(rBytes, offset: 0)) & 0x3FFFFFF
        let r1 = (readU32LE(rBytes, offset: 3) >> 2) & 0x3FFFFFF
        let r2 = (readU32LE(rBytes, offset: 6) >> 4) & 0x3FFFFFF
        let r3 = (readU32LE(rBytes, offset: 9) >> 6) & 0x3FFFFFF
        let r4 = (readU32LE(rBytes, offset: 12) >> 8) & 0x3FFFFFF

        // Precompute 5*r for the reduction step.
        // When we multiply and reduce mod 2^130 - 5, terms that overflow
        // past 2^130 get multiplied by 5 and folded back in.
        let s1 = r1 &* 5
        let s2 = r2 &* 5
        let s3 = r3 &* 5
        let s4 = r4 &* 5

        // --- Extract s (last 16 bytes) ---
        // s is just a 128-bit number added at the end
        var sBytes = Array(key[16..<32])

        // --- Accumulator: 5 limbs of 26 bits ---
        var h0: UInt32 = 0
        var h1: UInt32 = 0
        var h2: UInt32 = 0
        var h3: UInt32 = 0
        var h4: UInt32 = 0

        // --- Process message in 16-byte chunks ---
        var offset = 0
        while offset < message.count {
            let end = min(offset + 16, message.count)
            let chunkLen = end - offset

            // Read chunk into a 17-byte buffer (with the 0x01 high byte)
            var buf = [UInt8](repeating: 0, count: 17)
            for j in 0..<chunkLen {
                buf[j] = message[offset + j]
            }
            buf[chunkLen] = 1  // append 0x01 byte

            // Convert to 5 limbs and add to accumulator
            let t0 = UInt32(buf[0]) | (UInt32(buf[1]) << 8) | (UInt32(buf[2]) << 16) | ((UInt32(buf[3]) & 0x03) << 24)
            let t1 = (UInt32(buf[3]) >> 2) | (UInt32(buf[4]) << 6) | (UInt32(buf[5]) << 14) | ((UInt32(buf[6]) & 0x0F) << 22)
            let t2 = (UInt32(buf[6]) >> 4) | (UInt32(buf[7]) << 4) | (UInt32(buf[8]) << 12) | ((UInt32(buf[9]) & 0x3F) << 20)
            let t3 = (UInt32(buf[9]) >> 6) | (UInt32(buf[10]) << 2) | (UInt32(buf[11]) << 10) | (UInt32(buf[12]) << 18)
            let t4 = UInt32(buf[13]) | (UInt32(buf[14]) << 8) | (UInt32(buf[15]) << 16) | (UInt32(buf[16]) << 24)

            h0 = h0 &+ t0
            h1 = h1 &+ t1
            h2 = h2 &+ t2
            h3 = h3 &+ t3
            h4 = h4 &+ t4

            // --- Multiply: h = h * r  mod  (2^130 - 5) ---
            //
            // Using schoolbook multiplication in radix 2^26:
            //   h*r = (h0 + h1*B + h2*B^2 + h3*B^3 + h4*B^4) *
            //         (r0 + r1*B + r2*B^2 + r3*B^3 + r4*B^4)
            //
            // where B = 2^26. Terms that would land at B^5 or higher
            // (i.e., >= 2^130) are reduced using 2^130 ≡ 5 (mod p).
            // This means h_i * r_j where i+j >= 5 gets multiplied by 5.
            //
            // We precomputed s_k = 5*r_k for this purpose.

            let d0 = UInt64(h0) &* UInt64(r0)
                &+ UInt64(h1) &* UInt64(s4)
                &+ UInt64(h2) &* UInt64(s3)
                &+ UInt64(h3) &* UInt64(s2)
                &+ UInt64(h4) &* UInt64(s1)

            let d1 = UInt64(h0) &* UInt64(r1)
                &+ UInt64(h1) &* UInt64(r0)
                &+ UInt64(h2) &* UInt64(s4)
                &+ UInt64(h3) &* UInt64(s3)
                &+ UInt64(h4) &* UInt64(s2)

            let d2 = UInt64(h0) &* UInt64(r2)
                &+ UInt64(h1) &* UInt64(r1)
                &+ UInt64(h2) &* UInt64(r0)
                &+ UInt64(h3) &* UInt64(s4)
                &+ UInt64(h4) &* UInt64(s3)

            let d3 = UInt64(h0) &* UInt64(r3)
                &+ UInt64(h1) &* UInt64(r2)
                &+ UInt64(h2) &* UInt64(r1)
                &+ UInt64(h3) &* UInt64(r0)
                &+ UInt64(h4) &* UInt64(s4)

            let d4 = UInt64(h0) &* UInt64(r4)
                &+ UInt64(h1) &* UInt64(r3)
                &+ UInt64(h2) &* UInt64(r2)
                &+ UInt64(h3) &* UInt64(r1)
                &+ UInt64(h4) &* UInt64(r0)

            // --- Carry propagation ---
            // Each d_i can be up to about 5 * 2^26 * 2^26 = 5 * 2^52,
            // which fits in a UInt64. We propagate carries from low to high.
            var c: UInt64
            c = d0 >> 26; h0 = UInt32(d0 & 0x3FFFFFF)
            var e1 = d1 &+ c; c = e1 >> 26; h1 = UInt32(e1 & 0x3FFFFFF)
            var e2 = d2 &+ c; c = e2 >> 26; h2 = UInt32(e2 & 0x3FFFFFF)
            var e3 = d3 &+ c; c = e3 >> 26; h3 = UInt32(e3 & 0x3FFFFFF)
            var e4 = d4 &+ c; c = e4 >> 26; h4 = UInt32(e4 & 0x3FFFFFF)
            // Wrap carry from h4 back to h0 with factor 5 (since 2^130 ≡ 5)
            h0 = h0 &+ UInt32(c &* 5)
            c = UInt64(h0 >> 26); h0 &= 0x3FFFFFF
            h1 = h1 &+ UInt32(c)

            offset += 16
        }

        // --- Final reduction modulo p = 2^130 - 5 ---
        // After processing all chunks, h might be in [0, 2p).
        // We need to fully reduce to [0, p).
        //
        // Carry propagation
        var c: UInt32
        c = h1 >> 26; h1 &= 0x3FFFFFF
        h2 &+= c; c = h2 >> 26; h2 &= 0x3FFFFFF
        h3 &+= c; c = h3 >> 26; h3 &= 0x3FFFFFF
        h4 &+= c; c = h4 >> 26; h4 &= 0x3FFFFFF
        h0 &+= c &* 5; c = h0 >> 26; h0 &= 0x3FFFFFF
        h1 &+= c

        // Compute g = h + 5 - 2^130. If h >= p, then g >= 0 (no borrow).
        // If h < p, then g underflows (bit 31 of g4 is set after subtracting 2^26).
        var g0 = h0 &+ 5; c = g0 >> 26; g0 &= 0x3FFFFFF
        var g1 = h1 &+ c; c = g1 >> 26; g1 &= 0x3FFFFFF
        var g2 = h2 &+ c; c = g2 >> 26; g2 &= 0x3FFFFFF
        var g3 = h3 &+ c; c = g3 >> 26; g3 &= 0x3FFFFFF
        let g4 = h4 &+ c &- (1 << 26)  // subtract 2^26 (= 2^130 in the radix-2^26 world)

        // If h >= p: g4 is small (0 to ~0x3FFFFFF), bit 31 = 0, mask = 0xFFFFFFFF
        // If h < p:  g4 underflowed, bit 31 = 1, mask = 0x00000000
        let mask = (g4 >> 31) &- 1  // 0xFFFFFFFF if h >= p, 0 if h < p
        let notMask = ~mask
        h0 = (h0 & notMask) | (g0 & mask)
        h1 = (h1 & notMask) | (g1 & mask)
        h2 = (h2 & notMask) | (g2 & mask)
        h3 = (h3 & notMask) | (g3 & mask)
        h4 = (h4 & notMask) | ((g4 & 0x3FFFFFF) & mask)  // mask g4 to 26 bits too

        // --- Reassemble h into 4 x 32-bit words ---
        // The 5 limbs of 26 bits pack into 130 bits. We extract 4 x 32-bit
        // words by computing overlapping 64-bit values and masking to 32 bits.
        let w0 = UInt32((UInt64(h0) | (UInt64(h1) << 26)) & 0xFFFFFFFF)
        let w1 = UInt32(((UInt64(h1) >> 6) | (UInt64(h2) << 20)) & 0xFFFFFFFF)
        let w2 = UInt32(((UInt64(h2) >> 12) | (UInt64(h3) << 14)) & 0xFFFFFFFF)
        let w3 = UInt32(((UInt64(h3) >> 18) | (UInt64(h4) << 8)) & 0xFFFFFFFF)

        // --- Add s (tag = h + s mod 2^128) ---
        var f0 = UInt64(w0) &+ UInt64(readU32LE(sBytes, offset: 0))
        var fc = f0 >> 32
        var f1 = UInt64(w1) &+ UInt64(readU32LE(sBytes, offset: 4)) &+ fc
        fc = f1 >> 32
        var f2 = UInt64(w2) &+ UInt64(readU32LE(sBytes, offset: 8)) &+ fc
        fc = f2 >> 32
        var f3 = UInt64(w3) &+ UInt64(readU32LE(sBytes, offset: 12)) &+ fc

        // --- Write 16-byte tag (little-endian) ---
        var result = [UInt8](repeating: 0, count: 16)
        writeU32LE(&result, offset: 0, value: UInt32(f0 & 0xFFFFFFFF))
        writeU32LE(&result, offset: 4, value: UInt32(f1 & 0xFFFFFFFF))
        writeU32LE(&result, offset: 8, value: UInt32(f2 & 0xFFFFFFFF))
        writeU32LE(&result, offset: 12, value: UInt32(f3 & 0xFFFFFFFF))

        return result
    }

    // MARK: - AEAD Construction
    // ========================================================================
    //
    // The AEAD construction (RFC 8439 Section 2.8):
    //
    //   1. Derive a one-time Poly1305 key from ChaCha20(key, nonce, counter=0)
    //   2. Encrypt plaintext with ChaCha20 starting at counter=1
    //   3. Build MAC input: AAD || pad || ciphertext || pad || lengths
    //   4. Compute tag = Poly1305(poly_key, mac_input)
    //
    // ========================================================================

    /// Pad data length to a 16-byte boundary.
    private static func pad16(_ length: Int) -> [UInt8] {
        let remainder = length % 16
        if remainder == 0 { return [] }
        return [UInt8](repeating: 0, count: 16 - remainder)
    }

    /// Encode a number as 8 bytes little-endian (le64).
    private static func le64(_ value: UInt64) -> [UInt8] {
        var result = [UInt8](repeating: 0, count: 8)
        var v = value
        for i in 0..<8 {
            result[i] = UInt8(v & 0xFF)
            v >>= 8
        }
        return result
    }

    /// Build the Poly1305 MAC input for AEAD (RFC 8439 Section 2.8).
    ///
    ///   AAD || pad16(AAD) || ciphertext || pad16(CT) || le64(len_AAD) || le64(len_CT)
    private static func buildMacData(aad: [UInt8], ciphertext: [UInt8]) -> [UInt8] {
        var data = [UInt8]()
        data.append(contentsOf: aad)
        data.append(contentsOf: pad16(aad.count))
        data.append(contentsOf: ciphertext)
        data.append(contentsOf: pad16(ciphertext.count))
        data.append(contentsOf: le64(UInt64(aad.count)))
        data.append(contentsOf: le64(UInt64(ciphertext.count)))
        return data
    }

    /// AEAD encryption: encrypt plaintext and produce an authentication tag.
    ///
    /// - Parameters:
    ///   - plaintext: Data to encrypt.
    ///   - key: 32-byte secret key.
    ///   - nonce: 12-byte nonce (must be unique per encryption with same key).
    ///   - aad: Associated data to authenticate but not encrypt.
    /// - Returns: A tuple of (ciphertext, 16-byte tag).
    public static func aeadEncrypt(
        plaintext: [UInt8], key: [UInt8], nonce: [UInt8], aad: [UInt8]
    ) -> (ciphertext: [UInt8], tag: [UInt8]) {
        precondition(key.count == 32, "Key must be 32 bytes")
        precondition(nonce.count == 12, "Nonce must be 12 bytes")

        // Step 1: Generate one-time Poly1305 key using counter=0
        let polyBlock = chacha20Block(key: key, nonce: nonce, counter: 0)
        let polyKey = Array(polyBlock[0..<32])

        // Step 2: Encrypt plaintext with ChaCha20 starting at counter=1
        let ciphertext = chacha20Encrypt(
            plaintext: plaintext, key: key, nonce: nonce, counter: 1
        )

        // Step 3: Build MAC input and compute tag
        let macData = buildMacData(aad: aad, ciphertext: ciphertext)
        let tag = poly1305Mac(message: macData, key: polyKey)

        return (ciphertext, tag)
    }

    /// AEAD decryption: verify the tag and decrypt the ciphertext.
    ///
    /// Returns the decrypted plaintext, or `nil` if the tag is invalid
    /// (indicating the ciphertext was tampered with).
    ///
    /// - Parameters:
    ///   - ciphertext: Encrypted data.
    ///   - key: 32-byte secret key.
    ///   - nonce: 12-byte nonce (same as used for encryption).
    ///   - aad: Associated data (same as used for encryption).
    ///   - tag: 16-byte authentication tag to verify.
    /// - Returns: The decrypted plaintext, or `nil` if authentication fails.
    public static func aeadDecrypt(
        ciphertext: [UInt8], key: [UInt8], nonce: [UInt8], aad: [UInt8], tag: [UInt8]
    ) -> [UInt8]? {
        precondition(key.count == 32, "Key must be 32 bytes")
        precondition(nonce.count == 12, "Nonce must be 12 bytes")
        precondition(tag.count == 16, "Tag must be 16 bytes")

        // Step 1: Generate one-time Poly1305 key
        let polyBlock = chacha20Block(key: key, nonce: nonce, counter: 0)
        let polyKey = Array(polyBlock[0..<32])

        // Step 2: Recompute the tag
        let macData = buildMacData(aad: aad, ciphertext: ciphertext)
        let computedTag = poly1305Mac(message: macData, key: polyKey)

        // Step 3: Constant-time tag comparison
        var diff: UInt8 = 0
        for i in 0..<16 {
            diff |= computedTag[i] ^ tag[i]
        }
        if diff != 0 {
            return nil
        }

        // Step 4: Decrypt
        return chacha20Encrypt(
            plaintext: ciphertext, key: key, nonce: nonce, counter: 1
        )
    }
}
