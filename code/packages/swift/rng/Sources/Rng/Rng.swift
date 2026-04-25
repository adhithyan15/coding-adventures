// ============================================================================
// Rng.swift — Three Classic Pseudorandom Number Generators
// ============================================================================
//
// A pseudorandom number generator (PRNG) is a deterministic algorithm that
// takes a small seed value and produces a long sequence of numbers that
// *looks* random even though it isn't.  Given the same seed you always get
// the same sequence — a property that makes tests reproducible and simulations
// replayable.
//
// This module implements three generators that represent 70 years of PRNG
// evolution:
//
//   ┌──────────────┬───────────┬────────────────────────────────────────────┐
//   │ Generator    │ Year      │ Core idea                                  │
//   ├──────────────┼───────────┼────────────────────────────────────────────┤
//   │ LCG          │ 1948      │ multiply-add recurrence; upper 32 bits out │
//   │ Xorshift64   │ 2003      │ three XOR-shifts; lower 32 bits out        │
//   │ PCG32        │ 2014      │ LCG recurrence + XSH RR output permutation │
//   └──────────────┴───────────┴────────────────────────────────────────────┘
//
// All three share the same interface:
//
//   var g = LCG(seed: 42)
//   let v = g.nextU32()            // UInt32 in [0, 2^32)
//   let u = g.nextU64()            // UInt64 in [0, 2^64)
//   let f = g.nextFloat()          // Double in [0.0, 1.0)
//   let n = g.nextIntInRange(1, 6) // Int64 in [1, 6] inclusive
//
// Constants (Knuth / Numerical Recipes):
//   LCG_MULTIPLIER = 6364136223846793005
//   LCG_INCREMENT  = 1442695040888963407
//
// These satisfy the Hull-Dobell theorem: every 64-bit value appears exactly
// once per cycle (full period 2^64).
//
// Layer: CS03 (computer-science layer 3 — leaf package, zero dependencies)
// Spec:  code/specs/CS03-rng.md
// ============================================================================

// ── Constants ─────────────────────────────────────────────────────────────────

/// Knuth multiplier for the LCG and PCG32 recurrences.
///
/// Together with `lcgIncrement`, this satisfies the Hull-Dobell theorem:
/// GCD(lcgIncrement, 2^64) = 1, so the period is the full 2^64.
private let lcgMultiplier: UInt64 = 6364136223846793005

/// Additive constant for the LCG and PCG32 recurrences.
///
/// Must be odd for full period (Hull-Dobell condition c).  1442695040888963407
/// is odd (ends in 7) and was chosen by Knuth for its good spectral properties.
private let lcgIncrement: UInt64 = 1442695040888963407

/// Divisor used to normalise a UInt32 to [0.0, 1.0).
///
/// Dividing by 2^32 maps [0, 2^32 − 1] onto [0.0, ~0.99999999977).
private let floatDiv: Double = 4294967296.0  // 2^32

// ── LCG ───────────────────────────────────────────────────────────────────────

/// A Linear Congruential Generator (Knuth 1948).
///
/// # How it works
///
/// Each call advances a 64-bit state by:
///
///   state = (state × a + c) mod 2^64
///
/// where a = 6364136223846793005 and c = 1442695040888963407.
/// Swift's `UInt64` arithmetic wraps on overflow, so no explicit mod is needed.
///
/// The *upper* 32 bits are returned as output.  The lower bits have shorter
/// sub-periods and would produce noticeably worse sequences on their own.
///
/// # Strengths and weaknesses
///
///   + Extremely fast: one multiply, one add, one shift
///   + Full period: visits all 2^64 states before repeating
///   − Low-order bits are highly correlated
///   − Consecutive pairs fall on hyperplanes (Marsaglia's lattice test)
///
/// For serious simulations use PCG32 instead.  LCG is excellent for learning
/// and for performance-critical applications where quality matters less.
///
/// # Usage
///
///   var g = LCG(seed: 1)
///   print(g.nextU32())  // 1817669548
///   print(g.nextU32())  // 2187888307
public struct LCG {
    // State is the full 64-bit accumulator.  Declared var because nextU32
    // must mutate it.
    var state: UInt64

    /// Seed the generator.  Any 64-bit value is a valid seed.
    ///
    /// - Parameter seed: starting state; 0 is allowed (unlike Xorshift64)
    public init(seed: UInt64) {
        self.state = seed
    }

    // ── Core advance ──────────────────────────────────────────────────────────

    /// Advance the state one step and return the upper 32 bits.
    ///
    /// The upper-bits trick:
    ///
    ///   output = state >> 32
    ///
    /// Lower bits of an LCG have shorter sub-periods.  Bit 0 alternates 0/1.
    /// Bit 1 has period 4.  Only the upper half has the full period.
    public mutating func nextU32() -> UInt32 {
        state = state &* lcgMultiplier &+ lcgIncrement
        return UInt32(truncatingIfNeeded: state >> 32)
    }

    // ── Derived outputs ───────────────────────────────────────────────────────

    /// Return a 64-bit value by concatenating two consecutive 32-bit outputs.
    ///
    ///   result = (hi << 32) | lo
    ///
    /// Two calls are needed because a single LCG step only mixes 64 bits of
    /// state into 32 bits of output.
    public mutating func nextU64() -> UInt64 {
        let hi = UInt64(nextU32())
        let lo = UInt64(nextU32())
        return (hi << 32) | lo
    }

    /// Return a Double uniformly distributed in [0.0, 1.0).
    ///
    /// Divide the 32-bit output by 2^32.  The maximum representable value is
    /// (2^32 − 1) / 2^32 ≈ 0.99999999977, which is strictly less than 1.
    public mutating func nextFloat() -> Double {
        return Double(nextU32()) / floatDiv
    }

    /// Return a uniform random Int64 in the closed interval [min, max].
    ///
    /// # Rejection sampling
    ///
    /// A naïve `rawValue % rangeSize` produces modulo bias whenever 2^32 is
    /// not divisible by rangeSize.  Example: range = 3.  There are 2^32 / 3
    /// full chunks plus a remainder of 1.  Values 0 and 1 are slightly more
    /// likely to appear.
    ///
    /// Rejection sampling fixes this by discarding draws below:
    ///
    ///   threshold = (-rangeSize) mod rangeSize  (arithmetic mod 2^32)
    ///
    /// Any raw value ≥ threshold is accepted; the expected number of extra
    /// draws per call is less than 2 for any range size.
    ///
    /// - Parameters:
    ///   - min: lower bound (inclusive)
    ///   - max: upper bound (inclusive); must be ≥ min
    /// - Returns: uniformly distributed value in [min, max]
    public mutating func nextIntInRange(min: Int64, max: Int64) -> Int64 {
        precondition(min <= max, "nextIntInRange requires min <= max")
        let rangeSize = UInt64(bitPattern: max - min + 1)
        let threshold = (0 &- rangeSize) % rangeSize
        while true {
            let r = UInt64(nextU32())
            if r >= threshold {
                return min + Int64(bitPattern: r % rangeSize)
            }
        }
    }
}

// ── Xorshift64 ────────────────────────────────────────────────────────────────

/// A Xorshift64 generator (Marsaglia 2003).
///
/// # How it works
///
/// Xorshift generators apply a series of XOR-shift operations that together
/// form a maximal-period linear feedback shift register over GF(2):
///
///   x ^= x << 13
///   x ^= x >> 7
///   x ^= x << 17
///
/// Each step uses three shift amounts chosen so the resulting 64×64 binary
/// matrix is primitive — meaning the sequence visits all 2^64 − 1 non-zero
/// states before repeating.  (Zero is a fixed point and must be avoided.)
///
/// The *lower* 32 bits are returned as output.  Unlike LCG, the quality is
/// more uniform across bit positions, but Xorshift fails some statistical
/// tests that PCG passes.
///
/// # Zero-seed protection
///
/// If seed = 0 the state would never change (0 XOR anything = anything,
/// but all bits start as 0 so every shift produces 0 again).  We replace
/// seed 0 with 1.
///
/// # Usage
///
///   var g = Xorshift64(seed: 1)
///   print(g.nextU32())  // 1082269761
///   print(g.nextU32())  // 201397313
public struct Xorshift64 {
    var state: UInt64

    /// Seed the generator.
    ///
    /// - Parameter seed: starting state; 0 is silently replaced with 1
    public init(seed: UInt64) {
        self.state = seed == 0 ? 1 : seed
    }

    // ── Core advance ──────────────────────────────────────────────────────────

    /// Apply the three XOR-shift steps and return the lower 32 bits.
    ///
    /// Shift amounts 13, 7, 17 were found by Marsaglia through exhaustive
    /// search: they are the unique triple (from the canonical 81 listed in his
    /// 2003 paper) with period 2^64 − 1.
    public mutating func nextU32() -> UInt32 {
        var x = state
        x ^= x << 13
        x ^= x >> 7
        x ^= x << 17
        state = x
        return UInt32(truncatingIfNeeded: x)
    }

    // ── Derived outputs ───────────────────────────────────────────────────────

    /// Return a 64-bit value composed of two consecutive 32-bit outputs.
    public mutating func nextU64() -> UInt64 {
        let hi = UInt64(nextU32())
        let lo = UInt64(nextU32())
        return (hi << 32) | lo
    }

    /// Return a Double uniformly distributed in [0.0, 1.0).
    public mutating func nextFloat() -> Double {
        return Double(nextU32()) / floatDiv
    }

    /// Return a uniform random Int64 in [min, max] using rejection sampling.
    ///
    /// See `LCG.nextIntInRange` for a full explanation of rejection sampling.
    public mutating func nextIntInRange(min: Int64, max: Int64) -> Int64 {
        precondition(min <= max, "nextIntInRange requires min <= max")
        let rangeSize = UInt64(bitPattern: max - min + 1)
        let threshold = (0 &- rangeSize) % rangeSize
        while true {
            let r = UInt64(nextU32())
            if r >= threshold {
                return min + Int64(bitPattern: r % rangeSize)
            }
        }
    }
}

// ── PCG32 ─────────────────────────────────────────────────────────────────────

/// A Permuted Congruential Generator (O'Neill 2014).
///
/// # How it works
///
/// PCG32 uses the same fast LCG recurrence as the LCG struct but adds an
/// *output permutation* — a function that scrambles the state before returning
/// it.  This breaks the correlations that plague raw LCGs.
///
/// The permutation is called XSH RR (XOR-Shift High / Random Rotate):
///
///   1. xorshifted = ((old >> 18) ^ old) >> 27   // fold high bits down
///   2. rot        = old >> 59                    // 5-bit rotation amount
///   3. output     = rotr32(xorshifted, rot)      // rotate by state-dependent amount
///
/// The rotation uses the top 5 bits of the state to determine how far to
/// rotate the 32-bit xorshifted value.  Since the rotation amount itself
/// changes each step, an attacker cannot predict future values from current
/// output — even though the underlying LCG is entirely predictable.
///
/// PCG32 passes TestU01 BigCrush (the most stringent public statistical
/// test suite) with all 160 tests.  LCG fails dozens; Xorshift64 fails some.
///
/// # Initialisation warm-up
///
/// Low seeds (especially 0 and 1) would produce poor first outputs if we
/// simply set state = seed.  We warm up using the same "initseq" procedure
/// as the reference C implementation:
///
///   1. Advance once from state=0 (mixes in the increment)
///   2. Add seed to state
///   3. Advance once more (scatters seed bits throughout)
///
/// # Usage
///
///   var g = PCG32(seed: 1)
///   print(g.nextU32())  // 1412771199
///   print(g.nextU32())  // 1791099446
public struct PCG32 {
    var state: UInt64
    let increment: UInt64

    /// Seed the generator with the initseq warm-up procedure.
    ///
    /// - Parameter seed: any 64-bit value
    public init(seed: UInt64) {
        // The increment must be odd for full period (Hull-Dobell condition c).
        // We use lcgIncrement which is already odd.
        let inc = lcgIncrement | 1
        self.increment = inc
        self.state = 0

        // Step 1: advance once to mix in the increment before adding seed.
        self.state = self.state &* lcgMultiplier &+ inc
        // Step 2: add seed to spread initial seed bits across state.
        self.state = self.state &+ seed
        // Step 3: advance once more to scatter the seed bits throughout.
        self.state = self.state &* lcgMultiplier &+ inc
    }

    // ── Core advance ──────────────────────────────────────────────────────────

    /// Advance state and return the XSH RR permuted 32-bit output.
    ///
    /// The "output before advance" pattern (we capture `oldState` before
    /// updating) is standard in PCG — the permutation of the *old* state is
    /// more efficient to compute with full 64-bit precision.
    public mutating func nextU32() -> UInt32 {
        let oldState = state
        // Advance the LCG.
        state = oldState &* lcgMultiplier &+ increment

        // XSH RR permutation ─────────────────────────────────────────────────
        //
        // Step 1 — XOR-Shift High: fold the top bits down into a 32-bit value.
        //   xorshifted = ((old >> 18) ^ old) >> 27
        //
        // The >> 18 shifts the high bits towards the middle; XOR-ing with the
        // original mixes them with the surrounding bits; >> 27 takes the top
        // 37 bits and distills them into the top 32-bit region.
        //
        // Step 2 — rotation amount from the very top 5 bits.
        //   rot = old >> 59    (gives 0..31)
        //
        // Step 3 — rotate right by `rot` using Swift's &>> / &<< overflow ops.
        let xorshifted = UInt32(truncatingIfNeeded: ((oldState >> 18) ^ oldState) >> 27)
        let rot = UInt32(truncatingIfNeeded: oldState >> 59)
        return (xorshifted &>> rot) | (xorshifted &<< (32 &- rot))
    }

    // ── Derived outputs ───────────────────────────────────────────────────────

    /// Return a 64-bit value composed of two consecutive 32-bit outputs.
    public mutating func nextU64() -> UInt64 {
        let hi = UInt64(nextU32())
        let lo = UInt64(nextU32())
        return (hi << 32) | lo
    }

    /// Return a Double uniformly distributed in [0.0, 1.0).
    public mutating func nextFloat() -> Double {
        return Double(nextU32()) / floatDiv
    }

    /// Return a uniform random Int64 in [min, max] using rejection sampling.
    ///
    /// See `LCG.nextIntInRange` for a full explanation of rejection sampling.
    public mutating func nextIntInRange(min: Int64, max: Int64) -> Int64 {
        precondition(min <= max, "nextIntInRange requires min <= max")
        let rangeSize = UInt64(bitPattern: max - min + 1)
        let threshold = (0 &- rangeSize) % rangeSize
        while true {
            let r = UInt64(nextU32())
            if r >= threshold {
                return min + Int64(bitPattern: r % rangeSize)
            }
        }
    }
}
