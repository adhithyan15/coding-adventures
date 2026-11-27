// ============================================================================
// SplitMix64.swift -- the deterministic PRNG (TF01 section 5.1, VER05 section 2)
// ============================================================================
//
// Every generation and shrinking decision in the engine comes from this one
// generator, so a seed fully determines a run. SplitMix64 is tiny: a 64-bit
// counter that advances by a fixed odd constant (the "golden gamma"), followed
// by a *finalizer* that scrambles the counter into a well-mixed output.
//
//     state' = state + 0x9e3779b97f4a7c15            (mod 2^64)
//     z      = state'
//     z      = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9  (mod 2^64)
//     z      = (z ^ (z >> 27)) * 0x94d049bb133111eb  (mod 2^64)
//     output = z ^ (z >> 31)
//
// WRAPPING ARITHMETIC
// -------------------
// "mod 2^64" is exactly what `UInt64` wrapping operators (`&+`, `&*`) compute.
// This file is the ONE place in the package that uses them. Everywhere else
// plain `+` and `*` are used so that an overflow traps loudly instead of
// silently wrapping and hiding a bug.
//
// Pinned reference vector: seed 0's first output is 0xe220a8397b1dcdaf.
// ============================================================================

/// Errors for caller mistakes in the engine's public functions.
public enum PbtError: Error, Equatable {
    /// A bad argument (range, size, null-like input). The message says which.
    case invalidArgument(String)
    /// A shrinker was handed a start value that does not falsify the property.
    case notFalsifying
}

public struct SplitMix64 {
    public private(set) var state: UInt64

    public init(seed: UInt64) { state = seed }

    /// The next raw 64-bit word.
    public mutating func nextU64() -> UInt64 {
        state = state &+ 0x9e37_79b9_7f4a_7c15
        var z = state
        z = (z ^ (z >> 30)) &* 0xbf58_476d_1ce4_e5b9
        z = (z ^ (z >> 27)) &* 0x94d0_49bb_1331_11eb
        return z ^ (z >> 31)
    }

    /// An unbiased draw uniform on `[0, n)`, for `1 <= n <= 2^64 - 1`.
    ///
    /// WHY REJECTION?  `x % n` over all 2^64 words is slightly biased unless n
    /// divides 2^64: the first `2^64 mod n` residues are hit once more often
    /// than the rest. Rejection sampling throws those surplus words away.
    ///
    /// The number of words to discard from the *bottom* of the range is
    ///
    ///     t = (2^64 - n) mod n          (== 2^64 mod n)
    ///
    /// and in unsigned wrapping arithmetic `2^64 - n` is simply `0 &- n`, so
    /// `t = (0 &- n) % n`. Any word `x >= t` leaves a range of
    /// `2^64 - t`, a multiple of n, so `x % n` is exactly uniform.
    ///
    /// `bounded(1)` still consumes one word: the loop runs once (t = 0).
    public mutating func bounded(_ n: UInt64) throws -> UInt64 {
        if n == 0 { throw PbtError.invalidArgument("bounded(0)") }
        let threshold = (0 &- n) % n
        while true {
            let x = nextU64()
            if x >= threshold { return x % n }
        }
    }

    /// An unbiased draw uniform on `[0, span)` for any `span >= 1` (a `BigInt`).
    ///
    /// * below 2^64: the ordinary `bounded`;
    /// * exactly 2^64: one raw word (every word is already uniform);
    /// * above 2^64: draw `k` words, most significant first, where
    ///   `k = max(2, ceil(bitlen(span - 1) / 64))`, and reject when the k-word
    ///   number is below `t = 2^(64k) mod span` (the same argument as above,
    ///   with 2^(64k) in place of 2^64).
    public mutating func boundedBig(_ span: BigInt) throws -> BigInt {
        if span.signum <= 0 { throw PbtError.invalidArgument("boundedBig needs span >= 1") }
        if span.bitWidth > BigInt.apiMaxBits {
            throw PbtError.invalidArgument("span wider than \(BigInt.apiMaxBits) bits")
        }
        if let small = span.asUInt64 { return BigInt(try bounded(small)) }
        let twoTo64 = BigInt.one.shiftedLeft(64)
        if span == twoTo64 { return BigInt(nextU64()) }
        let k = max(2, ((span - .one).bitWidth + 63) / 64)
        // t = (2^(64k) - span) mod span == 2^(64k) mod span, since subtracting
        // one span does not change the residue.
        let threshold = BigInt.one.shiftedLeft(64 * k).quotientAndRemainder(dividingBy: span).remainder
        while true {
            var x = BigInt.zero
            for _ in 0..<k { x = x.shiftedLeft(64) + BigInt(nextU64()) }
            if x >= threshold { return x.quotientAndRemainder(dividingBy: span).remainder }
        }
    }
}

/// The 64-bit FNV-1a hash: xor a byte in, then multiply by the FNV prime.
public func fnv1a64(_ bytes: [UInt8]) -> UInt64 {
    var h: UInt64 = 0xcbf2_9ce4_8422_2325
    for b in bytes { h = (h ^ UInt64(b)) &* 0x0000_0100_0000_01b3 }
    return h
}

public func fnv1a64(_ text: String) -> UInt64 { fnv1a64(Array(text.utf8)) }

/// `sub_seed(run_seed, id) = SplitMix64(run_seed ^ fnv1a64(utf8(id))).nextU64()`
/// (VER05 section 2.3). Reordering unrelated tests never changes the candidate
/// sequence of an unchanged test id.
public func subSeed(_ runSeed: UInt64, _ propertyId: String) -> UInt64 {
    var rng = SplitMix64(seed: runSeed ^ fnv1a64(propertyId))
    return rng.nextU64()
}
