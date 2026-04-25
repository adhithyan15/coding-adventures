namespace CodingAdventures.Rng;

// Rng.cs — Three classic pseudorandom number generators in one file
// =================================================================
//
// A pseudorandom number generator (PRNG) is a deterministic function that
// maps one "state" number to the next, producing a sequence that looks
// random but is fully reproducible given the same seed. This matters for
// games, simulations, cryptography tests, and anywhere you need "random"
// data that you can replay.
//
// All three generators here share the same five-method API so you can swap
// them out without changing calling code:
//
//   NextU32()            — 32-bit unsigned integer in [0, 2^32)
//   NextU64()            — 64-bit unsigned integer in [0, 2^64)
//   NextFloat()          — double in [0.0, 1.0)
//   NextIntInRange(a, b) — long in [a, b] inclusive
//
// Quality ranking (roughly): PCG32 > Xorshift64 > LCG
// Speed ranking  (roughly): LCG ~ Xorshift64 > PCG32

// ── Shared constants ─────────────────────────────────────────────────────────

internal static class RngConstants
{
    // Knuth / Numerical Recipes LCG constants.
    // These satisfy the Hull-Dobell theorem so the LCG visits every possible
    // 64-bit value exactly once before repeating — a "full period 2^64" cycle.
    internal const ulong LcgMultiplier = 6364136223846793005UL;
    internal const ulong LcgIncrement  = 1442695040888963407UL;

    // Divide a u32 by 2^32 to get a float in [0.0, 1.0).
    // 4294967296.0 == 2^32.
    internal const double FloatDiv = 4294967296.0;
}

// ── LCG ──────────────────────────────────────────────────────────────────────

/// <summary>
/// Linear Congruential Generator (Knuth 1948).
///
/// Recurrence: state = (state × a + c) mod 2^64
///
/// The state is a 64-bit word. Arithmetic wraps automatically because C#
/// ulong arithmetic is unchecked by default — overflows mod 2^64.
///
/// Output: upper 32 bits of state. The lower bits have shorter sub-periods
/// than the upper bits, so we discard them.
///
/// Trade-offs
/// ----------
/// + Simplest useful PRNG — just one multiply and one add.
/// + Full period 2^64: every 64-bit value appears exactly once per cycle.
/// − Consecutive outputs are correlated; fails some statistical tests.
///   Use PCG32 when quality matters.
/// </summary>
public sealed class Lcg
{
    private ulong _state;

    /// <summary>
    /// Seed the LCG. Any value, including 0, is a valid seed.
    /// </summary>
    public Lcg(ulong seed)
    {
        _state = seed;
    }

    /// <summary>
    /// Advance the state and return the upper 32 bits.
    ///
    /// The recurrence is:
    ///
    ///   state ← state × a + c   (mod 2^64)
    ///   output ← state >> 32
    ///
    /// Taking the upper half discards the low-period lower bits.
    /// </summary>
    public uint NextU32()
    {
        _state = (_state * RngConstants.LcgMultiplier) + RngConstants.LcgIncrement;
        return (uint)(_state >> 32);
    }

    /// <summary>
    /// Return a 64-bit value from two consecutive 32-bit draws.
    ///
    /// Composition: (hi &lt;&lt; 32) | lo where hi and lo are successive NextU32 results.
    /// </summary>
    public ulong NextU64()
    {
        ulong hi = (ulong)NextU32();
        ulong lo = (ulong)NextU32();
        return (hi << 32) | lo;
    }

    /// <summary>
    /// Return a double uniformly distributed in [0.0, 1.0).
    ///
    /// Divides the 32-bit draw by 2^32. This gives exactly 2^32 evenly spaced
    /// values in [0, 1) with a spacing of about 2.3 × 10^-10.
    /// </summary>
    public double NextFloat()
    {
        return (double)NextU32() / RngConstants.FloatDiv;
    }

    /// <summary>
    /// Return a long uniformly drawn from [min, max] inclusive.
    ///
    /// Rejection sampling eliminates modulo bias. Naïve (value % range) skews
    /// the result when 2^32 is not divisible by range because some outputs are
    /// reachable by more raw values than others.
    ///
    /// The threshold trick: discard any draw that falls in the partial bucket
    /// at the bottom of the range. The threshold is (-range) % range mod 2^32.
    /// Expected extra draws: less than 2 for all range sizes.
    /// </summary>
    public long NextIntInRange(long min, long max)
    {
        if (min > max)
            throw new ArgumentOutOfRangeException(nameof(min), $"NextIntInRange requires min <= max, got {min} > {max}");
        ulong rangeSize = (ulong)(max - min + 1);
        ulong threshold = (ulong)(-(long)rangeSize) % rangeSize;
        while (true)
        {
            ulong r = (ulong)NextU32();
            if (r >= threshold)
            {
                return min + (long)(r % rangeSize);
            }
        }
    }
}

// ── Xorshift64 ────────────────────────────────────────────────────────────────

/// <summary>
/// Xorshift64 (Marsaglia 2003).
///
/// Three XOR-shift operations scramble 64-bit state with no multiplication:
///
///   x ^= x &lt;&lt; 13
///   x ^= x >> 7
///   x ^= x &lt;&lt; 17
///
/// Each shift "smears" high bits into low bits or vice versa. The three
/// specific constants (13, 7, 17) were found by exhaustive search to give
/// a maximal-length linear feedback register over GF(2) — meaning the
/// sequence visits every non-zero 64-bit state exactly once before repeating.
///
/// Period: 2^64 − 1. State 0 is a fixed point (XOR of 0 is always 0), so
/// seed 0 is replaced with 1.
///
/// Output: lower 32 bits.
/// </summary>
public sealed class Xorshift64
{
    private ulong _state;

    /// <summary>
    /// Seed the generator. Seed 0 is replaced with 1 to avoid the zero fixed point.
    /// </summary>
    public Xorshift64(ulong seed)
    {
        _state = seed == 0 ? 1UL : seed;
    }

    /// <summary>
    /// Apply three XOR-shifts to the state and return the lower 32 bits.
    ///
    /// The shifts are applied to the state in place:
    ///
    ///   x ^= x &lt;&lt; 13   — bleed high bits downward
    ///   x ^= x >> 7    — bleed low bits upward
    ///   x ^= x &lt;&lt; 17   — bleed high bits downward again
    /// </summary>
    public uint NextU32()
    {
        ulong x = _state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        _state = x;
        return (uint)x;
    }

    /// <summary>Return a 64-bit value from two consecutive 32-bit draws.</summary>
    public ulong NextU64()
    {
        ulong hi = (ulong)NextU32();
        ulong lo = (ulong)NextU32();
        return (hi << 32) | lo;
    }

    /// <summary>Return a double in [0.0, 1.0).</summary>
    public double NextFloat()
    {
        return (double)NextU32() / RngConstants.FloatDiv;
    }

    /// <summary>Return a long in [min, max] inclusive via rejection sampling.</summary>
    public long NextIntInRange(long min, long max)
    {
        if (min > max)
            throw new ArgumentOutOfRangeException(nameof(min), $"NextIntInRange requires min <= max, got {min} > {max}");
        ulong rangeSize = (ulong)(max - min + 1);
        ulong threshold = (ulong)(-(long)rangeSize) % rangeSize;
        while (true)
        {
            ulong r = (ulong)NextU32();
            if (r >= threshold)
            {
                return min + (long)(r % rangeSize);
            }
        }
    }
}

// ── PCG32 ─────────────────────────────────────────────────────────────────────

/// <summary>
/// PCG32 — Permuted Congruential Generator (O'Neill 2014).
///
/// Uses the same LCG recurrence as <see cref="Lcg"/> but adds an output
/// permutation step called "XSH RR" (XOR-Shift High / Random Rotate):
///
///   1. Capture old_state before the LCG advance.
///   2. Advance: state = old_state × a + c
///   3. xorshifted = ((old_state >> 18) ^ old_state) >> 27
///      — mix bits from two different positions in the 64-bit state.
///   4. rot = old_state >> 59
///      — extract a 5-bit rotation amount from the top of old_state.
///   5. output = rotr32(xorshifted, rot)
///      — rotate right to scatter all bits through the 32-bit output.
///
/// The rotation makes the output function non-linear, which breaks the
/// correlations that trip up plain LCG. PCG32 passes all known statistical
/// test suites (TestU01 BigCrush, PractRand) with only 8 bytes of state.
///
/// Initialization (the "initseq" warm-up):
///   Start from state=0, advance once to absorb the increment, add the seed,
///   then advance once more so seed bits scatter throughout state.
/// </summary>
public sealed class Pcg32
{
    private ulong _state;
    private readonly ulong _increment;

    /// <summary>
    /// Seed the PCG32 generator.
    ///
    /// The increment is fixed to <c>LcgIncrement | 1</c> (always odd, which
    /// is required for full period). Two warm-up advances scatter the seed
    /// through all 64 state bits before the first NextU32 call.
    /// </summary>
    public Pcg32(ulong seed)
    {
        _increment = RngConstants.LcgIncrement | 1UL;
        _state = 0;
        _state = (_state * RngConstants.LcgMultiplier) + _increment;
        _state += seed;
        _state = (_state * RngConstants.LcgMultiplier) + _increment;
    }

    /// <summary>
    /// Advance the LCG state and return the XSH RR permuted 32-bit output.
    ///
    /// XSH RR mixes two views of the old state:
    /// - the high bits (after XOR-shift) form the value to rotate
    /// - the top 5 bits determine by how much to rotate
    ///
    /// .NET 6+ provides <see cref="uint.RotateRight"/> so we use it directly.
    /// </summary>
    public uint NextU32()
    {
        ulong oldState = _state;
        _state = (oldState * RngConstants.LcgMultiplier) + _increment;

        uint xorshifted = (uint)(((oldState >> 18) ^ oldState) >> 27);
        int  rot        = (int)(oldState >> 59);
        return uint.RotateRight(xorshifted, rot);
    }

    /// <summary>Return a 64-bit value from two consecutive 32-bit draws.</summary>
    public ulong NextU64()
    {
        ulong hi = (ulong)NextU32();
        ulong lo = (ulong)NextU32();
        return (hi << 32) | lo;
    }

    /// <summary>Return a double in [0.0, 1.0).</summary>
    public double NextFloat()
    {
        return (double)NextU32() / RngConstants.FloatDiv;
    }

    /// <summary>Return a long in [min, max] inclusive via rejection sampling.</summary>
    public long NextIntInRange(long min, long max)
    {
        if (min > max)
            throw new ArgumentOutOfRangeException(nameof(min), $"NextIntInRange requires min <= max, got {min} > {max}");
        ulong rangeSize = (ulong)(max - min + 1);
        ulong threshold = (ulong)(-(long)rangeSize) % rangeSize;
        while (true)
        {
            ulong r = (ulong)NextU32();
            if (r >= threshold)
            {
                return min + (long)(r % rangeSize);
            }
        }
    }
}
