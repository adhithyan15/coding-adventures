// ============================================================================
// HashFunctions.swift — small hashes, written from their bit operations
// ============================================================================
//
// A hash function folds an arbitrary byte sequence into a fixed-width integer.
// These implementations deliberately use only Swift's standard library: their
// wrapping arithmetic is visible, their byte order is explicit, and none of
// them can acquire filesystem, process, clock, or entropy authority.

/// Stable validation failures shared by the free functions and strategy types.
public enum HashFunctionError: Error, Equatable, Sendable {
    case invalidModulus
    case invalidSipHashKeyLength(Int)
    case invalidOutputBits(Int)
    case emptySamples
    case emptySample(Int)
    case invalidBucketCount(Int)
    case emptyInputs
}

/// A type-erased-friendly interface for choosing a hash at runtime.
///
/// Thirty-two-bit algorithms are widened to `UInt64` without changing their
/// bit pattern. `outputBits` tells analysis code how many bits are meaningful.
public protocol HashFunction: Sendable {
    func hash(_ data: [UInt8]) -> UInt64
    var outputBits: Int { get }
}

public let fnv32OffsetBasis: UInt32 = 0x811C_9DC5
public let fnv32Prime: UInt32 = 0x0100_0193
public let fnv64OffsetBasis: UInt64 = 0xCBF2_9CE4_8422_2325
public let fnv64Prime: UInt64 = 0x0000_0100_0000_01B3
public let polynomialRollingDefaultBase: UInt64 = 31
public let polynomialRollingDefaultModulus: UInt64 = (1 << 61) - 1

private let murmurC1: UInt32 = 0xCC9E_2D51
private let murmurC2: UInt32 = 0x1B87_3593

// MARK: - FNV-1a

/// Fowler–Noll–Vo 1a, truncated naturally to 32 bits after every multiply.
public func fnv1a32(_ data: [UInt8]) -> UInt32 {
    var hash = fnv32OffsetBasis
    for byte in data {
        hash ^= UInt32(byte)
        hash = hash &* fnv32Prime
    }
    return hash
}

public func fnv1a32(_ text: String) -> UInt32 {
    fnv1a32(Array(text.utf8))
}

/// Fowler–Noll–Vo 1a with the standard 64-bit offset and prime.
public func fnv1a64(_ data: [UInt8]) -> UInt64 {
    var hash = fnv64OffsetBasis
    for byte in data {
        hash ^= UInt64(byte)
        hash = hash &* fnv64Prime
    }
    return hash
}

public func fnv1a64(_ text: String) -> UInt64 {
    fnv1a64(Array(text.utf8))
}

public func hashStringFNV1a32(_ text: String) -> UInt32 {
    fnv1a32(text)
}

// MARK: - DJB2 and polynomial rolling

/// Dan Bernstein's `hash * 33 + byte`, with defined 64-bit wrapping.
public func djb2(_ data: [UInt8]) -> UInt64 {
    var hash: UInt64 = 5_381
    for byte in data {
        hash = (hash &* 33) &+ UInt64(byte)
    }
    return hash
}

public func djb2(_ text: String) -> UInt64 {
    djb2(Array(text.utf8))
}

/// Evaluate the repository's polynomial compatibility hash.
///
/// `multipliedFullWidth` retains the complete 128-bit product without relying
/// on a platform-specific integer type. Because both factors are reduced first,
/// the high half is smaller than `modulus`, which is the precondition required
/// by `dividingFullWidth`.
public func polynomialRolling(
    _ data: [UInt8],
    base: UInt64 = polynomialRollingDefaultBase,
    modulus: UInt64 = polynomialRollingDefaultModulus
) throws -> UInt64 {
    guard modulus > 0 else { throw HashFunctionError.invalidModulus }
    return polynomialRollingUnchecked(data, base: base, modulus: modulus)
}

public func polynomialRolling(
    _ text: String,
    base: UInt64 = polynomialRollingDefaultBase,
    modulus: UInt64 = polynomialRollingDefaultModulus
) throws -> UInt64 {
    try polynomialRolling(Array(text.utf8), base: base, modulus: modulus)
}

private func polynomialRollingUnchecked(
    _ data: [UInt8],
    base: UInt64,
    modulus: UInt64
) -> UInt64 {
    let reducedBase = base % modulus
    var hash: UInt64 = 0
    for byte in data {
        let product = hash.multipliedFullWidth(by: reducedBase)
        hash = modulus.dividingFullWidth(product).remainder
        hash = addModulo(hash, UInt64(byte) % modulus, modulus: modulus)
    }
    return hash
}

/// Add two already-reduced values without overflowing before the modulus.
private func addModulo(_ lhs: UInt64, _ rhs: UInt64, modulus: UInt64) -> UInt64 {
    let distanceToWrap = modulus - rhs
    if lhs >= distanceToWrap {
        return lhs - distanceToWrap
    }
    return lhs + rhs
}

// MARK: - MurmurHash3 x86 32-bit

/// MurmurHash3's 32-bit variant, including all 1–3 byte tail shapes.
public func murmur3_32(_ data: [UInt8], seed: UInt32 = 0) -> UInt32 {
    var hash = seed
    var offset = 0

    while data.count - offset >= 4 {
        var block = UInt32(data[offset])
        block |= UInt32(data[offset + 1]) << 8
        block |= UInt32(data[offset + 2]) << 16
        block |= UInt32(data[offset + 3]) << 24
        block = block &* murmurC1
        block = block.rotatedLeft(by: 15)
        block = block &* murmurC2

        hash ^= block
        hash = hash.rotatedLeft(by: 13)
        hash = (hash &* 5) &+ 0xE654_6B64
        offset += 4
    }

    var tail: UInt32 = 0
    let remaining = data.count - offset
    if remaining >= 3 { tail ^= UInt32(data[offset + 2]) << 16 }
    if remaining >= 2 { tail ^= UInt32(data[offset + 1]) << 8 }
    if remaining >= 1 {
        tail ^= UInt32(data[offset])
        tail = tail &* murmurC1
        tail = tail.rotatedLeft(by: 15)
        tail = tail &* murmurC2
        hash ^= tail
    }

    hash ^= UInt32(truncatingIfNeeded: data.count)
    return murmurFinalMix(hash)
}

public func murmur3_32(_ text: String, seed: UInt32 = 0) -> UInt32 {
    murmur3_32(Array(text.utf8), seed: seed)
}

private func murmurFinalMix(_ input: UInt32) -> UInt32 {
    var hash = input
    hash ^= hash >> 16
    hash = hash &* 0x85EB_CA6B
    hash ^= hash >> 13
    hash = hash &* 0xC2B2_AE35
    hash ^= hash >> 16
    return hash
}

private extension UInt32 {
    func rotatedLeft(by count: UInt32) -> UInt32 {
        (self << count) | (self >> (32 - count))
    }
}

// MARK: - SipHash-2-4

private let sipHashV0: UInt64 = 0x736F_6D65_7073_6575
private let sipHashV1: UInt64 = 0x646F_7261_6E64_6F6D
private let sipHashV2: UInt64 = 0x6C79_6765_6E65_7261
private let sipHashV3: UInt64 = 0x7465_6462_7974_6573

/// SipHash-2-4 using an exact 128-bit key supplied by the caller.
public func sipHash24(_ data: [UInt8], key: [UInt8]) throws -> UInt64 {
    guard key.count == 16 else {
        throw HashFunctionError.invalidSipHashKeyLength(key.count)
    }
    return sipHash24Unchecked(data, key: key)
}

public func hashStringSipHash(_ text: String, key: [UInt8]) throws -> UInt64 {
    try sipHash24(Array(text.utf8), key: key)
}

private func sipHash24Unchecked(_ data: [UInt8], key: [UInt8]) -> UInt64 {
    let k0 = readLittleEndian64(key, at: 0)
    let k1 = readLittleEndian64(key, at: 8)
    var state = SipState(
        v0: sipHashV0 ^ k0,
        v1: sipHashV1 ^ k1,
        v2: sipHashV2 ^ k0,
        v3: sipHashV3 ^ k1
    )

    var offset = 0
    while data.count - offset >= 8 {
        let message = readLittleEndian64(data, at: offset)
        state.v3 ^= message
        state.rounds(2)
        state.v0 ^= message
        offset += 8
    }

    // The high byte carries length modulo 256; the lower seven bytes carry the
    // final partial word in little-endian order.
    var finalWord = UInt64(truncatingIfNeeded: data.count & 0xFF) << 56
    for index in 0..<(data.count - offset) {
        finalWord |= UInt64(data[offset + index]) << UInt64(index * 8)
    }

    state.v3 ^= finalWord
    state.rounds(2)
    state.v0 ^= finalWord
    state.v2 ^= 0xFF
    state.rounds(4)
    return state.v0 ^ state.v1 ^ state.v2 ^ state.v3
}

private struct SipState {
    var v0: UInt64
    var v1: UInt64
    var v2: UInt64
    var v3: UInt64

    mutating func rounds(_ count: Int) {
        for _ in 0..<count { round() }
    }

    private mutating func round() {
        v0 = v0 &+ v1
        v1 = v1.rotatedLeft(by: 13) ^ v0
        v0 = v0.rotatedLeft(by: 32)
        v2 = v2 &+ v3
        v3 = v3.rotatedLeft(by: 16) ^ v2
        v0 = v0 &+ v3
        v3 = v3.rotatedLeft(by: 21) ^ v0
        v2 = v2 &+ v1
        v1 = v1.rotatedLeft(by: 17) ^ v2
        v2 = v2.rotatedLeft(by: 32)
    }
}

private extension UInt64 {
    func rotatedLeft(by count: UInt64) -> UInt64 {
        (self << count) | (self >> (64 - count))
    }
}

private func readLittleEndian64(_ bytes: [UInt8], at offset: Int) -> UInt64 {
    var value: UInt64 = 0
    for index in 0..<8 {
        value |= UInt64(bytes[offset + index]) << UInt64(index * 8)
    }
    return value
}

// MARK: - Strategy values

public struct FNV1a32: HashFunction {
    public init() {}
    public func hash(_ data: [UInt8]) -> UInt64 { UInt64(fnv1a32(data)) }
    public let outputBits = 32
}

public struct FNV1a64: HashFunction {
    public init() {}
    public func hash(_ data: [UInt8]) -> UInt64 { fnv1a64(data) }
    public let outputBits = 64
}

public struct DJB2Hash: HashFunction {
    public init() {}
    public func hash(_ data: [UInt8]) -> UInt64 { djb2(data) }
    public let outputBits = 64
}

public struct PolynomialRollingHash: HashFunction {
    public let base: UInt64
    public let modulus: UInt64
    public let outputBits = 64

    public init() {
        base = polynomialRollingDefaultBase
        modulus = polynomialRollingDefaultModulus
    }

    public init(base: UInt64, modulus: UInt64) throws {
        guard modulus > 0 else { throw HashFunctionError.invalidModulus }
        self.base = base
        self.modulus = modulus
    }

    public func hash(_ data: [UInt8]) -> UInt64 {
        polynomialRollingUnchecked(data, base: base, modulus: modulus)
    }
}

public struct Murmur3Hash: HashFunction {
    public let seed: UInt32
    public init(seed: UInt32 = 0) { self.seed = seed }
    public func hash(_ data: [UInt8]) -> UInt64 {
        UInt64(murmur3_32(data, seed: seed))
    }
    public let outputBits = 32
}

public struct SipHash24: HashFunction {
    private let key: [UInt8]
    public let outputBits = 64

    public init(key: [UInt8]) throws {
        guard key.count == 16 else {
            throw HashFunctionError.invalidSipHashKeyLength(key.count)
        }
        self.key = key
    }

    public func hash(_ data: [UInt8]) -> UInt64 {
        sipHash24Unchecked(data, key: key)
    }
}

// MARK: - Quality analysis

/// Measure the fraction of the declared low-order output bits changed by
/// one-bit input mutations. Samples are explicit so the operation remains
/// deterministic and never obtains ambient entropy.
public func avalancheScore(
    hashFunction: ([UInt8]) -> UInt64,
    outputBits: Int,
    samples: [[UInt8]]
) throws -> Double {
    guard (1...64).contains(outputBits) else {
        throw HashFunctionError.invalidOutputBits(outputBits)
    }
    guard !samples.isEmpty else { throw HashFunctionError.emptySamples }

    let outputMask = outputBits == 64
        ? UInt64.max
        : (UInt64(1) << outputBits) - 1
    var changedBits = 0.0
    var examinedBits = 0.0
    for (sampleIndex, sample) in samples.enumerated() {
        guard !sample.isEmpty else { throw HashFunctionError.emptySample(sampleIndex) }
        let original = hashFunction(sample)
        for byteIndex in sample.indices {
            for bitOffset in 0..<8 {
                var flipped = sample
                flipped[byteIndex] ^= UInt8(1 << bitOffset)
                let changed = (original ^ hashFunction(flipped)) & outputMask
                changedBits += Double(changed.nonzeroBitCount)
                examinedBits += Double(outputBits)
            }
        }
    }
    return changedBits / examinedBits
}

/// Return the chi-squared statistic for the caller's inputs across buckets.
public func distributionTest(
    hashFunction: ([UInt8]) -> UInt64,
    inputs: [[UInt8]],
    numBuckets: Int
) throws -> Double {
    guard numBuckets > 0 else {
        throw HashFunctionError.invalidBucketCount(numBuckets)
    }
    guard !inputs.isEmpty else { throw HashFunctionError.emptyInputs }

    var counts = [UInt64](repeating: 0, count: numBuckets)
    for input in inputs {
        let bucket = Int(hashFunction(input) % UInt64(numBuckets))
        counts[bucket] += 1
    }

    let expected = Double(inputs.count) / Double(numBuckets)
    return counts.reduce(0) { result, observed in
        let difference = Double(observed) - expected
        return result + difference * difference / expected
    }
}
