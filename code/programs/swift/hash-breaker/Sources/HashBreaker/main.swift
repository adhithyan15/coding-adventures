// hash-breaker — Demonstrating why MD5 is cryptographically broken.
//
// Three attacks against MD5:
//   1. Known Collision Pairs (Wang & Yu, 2004)
//   2. Length Extension Attack (forge hash without secret)
//   3. Birthday Attack on truncated hash (birthday paradox)

import Foundation
import MD5

// ============================================================================
// Utility functions
// ============================================================================

func hexToBytes(_ hex: String) -> [UInt8] {
    var bytes: [UInt8] = []
    var index = hex.startIndex
    while index < hex.endIndex {
        let nextIndex = hex.index(index, offsetBy: 2)
        let byteStr = String(hex[index..<nextIndex])
        bytes.append(UInt8(byteStr, radix: 16)!)
        index = nextIndex
    }
    return bytes
}

func bytesToHex(_ bytes: [UInt8]) -> String {
    bytes.map { String(format: "%02x", $0) }.joined()
}

func hexDump(_ bytes: [UInt8]) -> String {
    var lines: [String] = []
    var i = 0
    while i < bytes.count {
        let end = min(i + 16, bytes.count)
        let row = bytes[i..<end].map { String(format: "%02x", $0) }.joined()
        lines.append("  \(row)")
        i += 16
    }
    return lines.joined(separator: "\n")
}

// ============================================================================
// ATTACK 1: Known MD5 Collision Pairs (Wang & Yu, 2004)
// ============================================================================
//
// Two 128-byte messages that produce the SAME MD5 hash. The canonical Wang/Yu
// collision pair — the breakthrough that proved MD5 is broken for security.

let collisionAHex =
    "d131dd02c5e6eec4693d9a0698aff95c" +
    "2fcab58712467eab4004583eb8fb7f89" +
    "55ad340609f4b30283e488832571415a" +
    "085125e8f7cdc99fd91dbdf280373c5b" +
    "d8823e3156348f5bae6dacd436c919c6" +
    "dd53e2b487da03fd02396306d248cda0" +
    "e99f33420f577ee8ce54b67080a80d1e" +
    "c69821bcb6a8839396f9652b6ff72a70"

let collisionBHex =
    "d131dd02c5e6eec4693d9a0698aff95c" +
    "2fcab50712467eab4004583eb8fb7f89" +
    "55ad340609f4b30283e4888325f1415a" +
    "085125e8f7cdc99fd91dbd7280373c5b" +
    "d8823e3156348f5bae6dacd436c919c6" +
    "dd53e23487da03fd02396306d248cda0" +
    "e99f33420f577ee8ce54b67080280d1e" +
    "c69821bcb6a8839396f965ab6ff72a70"

func attack1() {
    let sep = String(repeating: "=", count: 72)
    print(sep)
    print("ATTACK 1: Known MD5 Collision Pair (Wang & Yu, 2004)")
    print(sep)
    print()
    print("Two different 128-byte messages that produce the SAME MD5 hash.")
    print("This was the breakthrough that proved MD5 is broken for security.")
    print()

    let bytesA = hexToBytes(collisionAHex)
    let bytesB = hexToBytes(collisionBHex)

    print("Block A (hex):")
    print(hexDump(bytesA))
    print()
    print("Block B (hex):")
    print(hexDump(bytesB))
    print()

    var diffs: [Int] = []
    for i in 0..<bytesA.count {
        if bytesA[i] != bytesB[i] { diffs.append(i) }
    }
    print("Blocks differ at \(diffs.count) byte positions: \(diffs)")
    for pos in diffs {
        print(String(format: "  Byte %d: A=0x%02x  B=0x%02x", pos, bytesA[pos], bytesB[pos]))
    }
    print()

    let hashA = md5Hex(Data(bytesA))
    let hashB = md5Hex(Data(bytesB))
    print("MD5(A) = \(hashA)")
    print("MD5(B) = \(hashB)")
    print("Match?   \(hashA == hashB ? "YES — COLLISION!" : "No (unexpected)")")
    print()
    print("Lesson: MD5 collisions are REAL. Never use MD5 for integrity or auth.")
    print()
}

// ============================================================================
// ATTACK 2: Length Extension Attack
// ============================================================================

// MD5 T-table (sine-derived constants).
let tTable: [UInt32] = (0..<64).map { i in
    UInt32(floor(abs(sin(Double(i + 1))) * pow(2.0, 32.0)))
}

// MD5 per-round shift amounts.
let shifts: [UInt32] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
]

/// Inline MD5 compression for the length extension attack.
func md5Compress(state: (UInt32, UInt32, UInt32, UInt32), block: [UInt8]) -> (UInt32, UInt32, UInt32, UInt32) {
    var m = [UInt32](repeating: 0, count: 16)
    for i in 0..<16 {
        m[i] = UInt32(block[i * 4])
            | (UInt32(block[i * 4 + 1]) << 8)
            | (UInt32(block[i * 4 + 2]) << 16)
            | (UInt32(block[i * 4 + 3]) << 24)
    }

    var (a, b, c, d) = state
    let (a0, b0, c0, d0) = state

    for i in 0..<64 {
        var f: UInt32
        var g: Int
        if i < 16 {
            f = (b & c) | (~b & d)
            g = i
        } else if i < 32 {
            f = (d & b) | (~d & c)
            g = (5 * i + 1) % 16
        } else if i < 48 {
            f = b ^ c ^ d
            g = (3 * i + 5) % 16
        } else {
            f = c ^ (b | ~d)
            g = (7 * i) % 16
        }

        let temp = d
        d = c
        c = b
        let sum = a &+ f &+ tTable[i] &+ m[g]
        b = b &+ (sum << shifts[i] | sum >> (32 - shifts[i]))
        a = temp
    }

    return (a0 &+ a, b0 &+ b, c0 &+ c, d0 &+ d)
}

func md5Padding(messageLen: Int) -> [UInt8] {
    let remainder = messageLen % 64
    var padLen = (55 - remainder) % 64
    if padLen < 0 { padLen += 64 }
    var padding = [UInt8](repeating: 0, count: 1 + padLen + 8)
    padding[0] = 0x80
    let bitLen = UInt64(messageLen) * 8
    for i in 0..<8 {
        padding[1 + padLen + i] = UInt8((bitLen >> (i * 8)) & 0xFF)
    }
    return padding
}

func readLE32(_ bytes: [UInt8], _ offset: Int) -> UInt32 {
    UInt32(bytes[offset])
        | (UInt32(bytes[offset + 1]) << 8)
        | (UInt32(bytes[offset + 2]) << 16)
        | (UInt32(bytes[offset + 3]) << 24)
}

func writeLE32(_ value: UInt32) -> [UInt8] {
    [
        UInt8(value & 0xFF),
        UInt8((value >> 8) & 0xFF),
        UInt8((value >> 16) & 0xFF),
        UInt8((value >> 24) & 0xFF),
    ]
}

func attack2() {
    let sep = String(repeating: "=", count: 72)
    print(sep)
    print("ATTACK 2: Length Extension Attack")
    print(sep)
    print()
    print("Given md5(secret + message) and len(secret + message), we can forge")
    print("md5(secret + message + padding + evil_data) WITHOUT knowing the secret!")
    print()

    let secret = Array("supersecretkey!!".utf8)
    let message = Array("amount=100&to=alice".utf8)
    let originalData = secret + message
    let originalHash = md5(Data(originalData))
    let originalBytes = [UInt8](originalHash)
    let originalHex = bytesToHex(originalBytes)

    print("Secret (unknown to attacker): \"supersecretkey!!\"")
    print("Message:                      \"amount=100&to=alice\"")
    print("MAC = md5(secret || message): \(originalHex)")
    print("Length of (secret || message): \(originalData.count) bytes")
    print()

    let evilData = Array("&amount=1000000&to=mallory".utf8)
    print("Evil data to append: \"&amount=1000000&to=mallory\"")
    print()

    // Step 1: Extract state from hash
    let a = readLE32(originalBytes, 0)
    let b = readLE32(originalBytes, 4)
    let c = readLE32(originalBytes, 8)
    let d = readLE32(originalBytes, 12)

    print("Step 1: Extract MD5 internal state from the hash")
    print(String(format: "  A = 0x%08x, B = 0x%08x, C = 0x%08x, D = 0x%08x", a, b, c, d))
    print()

    // Step 2: Compute padding
    let padding = md5Padding(messageLen: originalData.count)
    print("Step 2: Compute MD5 padding for the original message")
    print("  Padding (\(padding.count) bytes): \(bytesToHex(padding))")
    print()

    let processedLen = originalData.count + padding.count
    print("Step 3: Total bytes processed so far: \(processedLen)")
    print()

    // Step 4: Forge
    let forgedInput = evilData + md5Padding(messageLen: processedLen + evilData.count)
    var state = (a, b, c, d)
    var offset = 0
    while offset + 64 <= forgedInput.count {
        let block = Array(forgedInput[offset..<offset + 64])
        state = md5Compress(state: state, block: block)
        offset += 64
    }

    let forgedBytes = writeLE32(state.0) + writeLE32(state.1) + writeLE32(state.2) + writeLE32(state.3)
    let forgedHex = bytesToHex(forgedBytes)

    print("Step 4: Initialize hasher with extracted state, feed evil_data")
    print("  Forged hash: \(forgedHex)")
    print()

    // Step 5: Verify
    let actualFull = originalData + padding + evilData
    let actualHex = md5Hex(Data(actualFull))

    print("Step 5: Verify — compute actual md5(secret || message || padding || evil_data)")
    print("  Actual hash: \(actualHex)")
    print("  Match?       \(forgedHex == actualHex ? "YES — FORGED!" : "No (bug)")")
    print()
    print("The attacker forged a valid MAC without knowing the secret!")
    print()
    print("Why HMAC fixes this:")
    print("  HMAC = md5(key XOR opad || md5(key XOR ipad || message))")
    print("  The outer hash prevents length extension because the attacker")
    print("  cannot extend past the outer md5() boundary.")
    print()
}

// ============================================================================
// ATTACK 3: Birthday Attack (Truncated Hash)
// ============================================================================

/// Simple xorshift32 PRNG for reproducible results.
struct Xorshift32 {
    var state: UInt32

    init(seed: UInt32) {
        state = seed
    }

    mutating func next() -> UInt32 {
        state ^= state << 13
        state ^= state >> 17
        state ^= state << 5
        return state
    }
}

func attack3() {
    let sep = String(repeating: "=", count: 72)
    print(sep)
    print("ATTACK 3: Birthday Attack on Truncated MD5 (32-bit)")
    print(sep)
    print()
    print("The birthday paradox: with N possible hash values, expect a collision")
    print("after ~sqrt(N) random inputs. For 32-bit hash: sqrt(2^32) = 2^16 = 65536.")
    print()

    var rng = Xorshift32(seed: 42)
    var seen: [String: [UInt8]] = [:]

    for attempts in 1... {
        var msg = [UInt8](repeating: 0, count: 8)
        for i in 0..<8 {
            msg[i] = UInt8(rng.next() & 0xFF)
        }

        let hash = [UInt8](md5(Data(msg)))
        let truncated = bytesToHex(Array(hash[0..<4]))

        if let other = seen[truncated] {
            if other != msg {
                print("COLLISION FOUND after \(attempts) attempts!")
                print()
                print("  Message 1: \(bytesToHex(other))")
                print("  Message 2: \(bytesToHex(msg))")
                print("  Truncated MD5 (4 bytes): \(truncated)")
                print("  Full MD5 of msg1: \(md5Hex(Data(other)))")
                print("  Full MD5 of msg2: \(md5Hex(Data(msg)))")
                print()
                print("  Expected ~65536 attempts (2^16), got \(attempts)")
                print(String(format: "  Ratio: %.2fx the theoretical expectation", Double(attempts) / 65536.0))
                break
            }
        } else {
            seen[truncated] = msg
        }
    }

    print()
    print("This is a GENERIC attack — it works against any hash function.")
    print("The defense is a longer hash: SHA-256 has 2^128 birthday bound,")
    print("while MD5 has only 2^64 (and dedicated attacks are even faster).")
    print()
}

// ============================================================================
// Main
// ============================================================================

print()
print("======================================================================")
print("           MD5 HASH BREAKER — Why MD5 Is Broken")
print("======================================================================")
print("  Three attacks showing MD5 must NEVER be used for security:")
print("    1. Known collision pairs (Wang & Yu, 2004)")
print("    2. Length extension attack (forge MAC without secret)")
print("    3. Birthday attack on truncated hash (birthday paradox)")
print("======================================================================")
print()

attack1()
attack2()
attack3()

print(String(repeating: "=", count: 72))
print("CONCLUSION")
print(String(repeating: "=", count: 72))
print()
print("MD5 is broken in three distinct ways:")
print("  1. COLLISION RESISTANCE: known pairs exist (and can be generated)")
print("  2. LENGTH EXTENSION: Merkle-Damgard structure leaks internal state")
print("  3. BIRTHDAY BOUND: only 2^64 (and dedicated attacks beat even that)")
print()
print("Use SHA-256 or SHA-3 for security. Use HMAC (not raw hash) for MACs.")
print()
