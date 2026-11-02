// rng.dart — Three classic pseudorandom number generators
// ========================================================
//
// A pseudorandom number generator (PRNG) takes a seed integer and produces a
// sequence of numbers that appears random but is entirely deterministic. Every
// time you use the same seed you get the identical sequence — which is useful
// for games, simulations, reproducible tests, and procedural generation.
//
// This library implements three algorithms of increasing quality:
//
//   Lcg        — simplest, fast, full period, some statistical weaknesses
//   Xorshift64 — no multiplication, longer period, better quality than LCG
//   Pcg32      — LCG with output permutation, passes all known test suites
//
// All three expose the same five-method API so they are interchangeable.
//
// # Dart integer notes
//
// Dart integers are arbitrary-precision on the VM. They do NOT wrap on
// overflow. So every operation that should stay 64-bit must be masked:
//
//   result = (a * b + c) & _mask64     // force 64-bit range
//
// 32-bit values are similarly masked with & 0xFFFFFFFF.
// Unsigned right shift uses >>> (Dart 2.14+).

// ─── Constants ───────────────────────────────────────────────────────────────

// Knuth / Numerical Recipes LCG constants. Together they satisfy the
// Hull-Dobell theorem for full period 2^64.
const int _lcgMultiplier = 6364136223846793005;
const int _lcgIncrement  = 1442695040888963407;

// Masks to keep arithmetic within 64-bit and 32-bit ranges.
const int _mask64 = 0xFFFFFFFFFFFFFFFF; // 2^64 - 1
const int _mask32 = 0xFFFFFFFF;         // 2^32 - 1

// Divisor used to convert a 32-bit integer to a double in [0.0, 1.0).
const double _floatDiv = 4294967296.0; // 2^32

// ─── Lcg ─────────────────────────────────────────────────────────────────────

/// Linear Congruential Generator (Knuth 1948).
///
/// Recurrence: `state = (state × a + c) mod 2^64`
///
/// The state is a 64-bit integer. Because Dart integers are arbitrary-
/// precision, every step explicitly masks to 64 bits with [_mask64].
///
/// Output is the upper 32 bits of state. The lower bits have shorter
/// sub-periods than the upper bits, so we discard them.
///
/// Trade-offs
/// ----------
/// - Simplest useful PRNG: one multiply, one add.
/// - Full period 2^64: every 64-bit value appears exactly once per cycle.
/// - Consecutive outputs are correlated; fails some statistical tests.
///   Use [Pcg32] when quality matters.
class Lcg {
  int _state;

  /// Create an [Lcg] seeded with [seed]. Any value including 0 is valid.
  Lcg(int seed) : _state = seed & _mask64;

  /// Advance the LCG state and return the upper 32 bits.
  ///
  /// The recurrence is:
  ///
  ///   state ← (state × a + c) mod 2^64
  ///   output ← state >> 32
  ///
  /// The `& _mask64` forces wrap-around at 2^64 because Dart integers do
  /// not overflow.
  int nextU32() {
    _state = ((_state * _lcgMultiplier) + _lcgIncrement) & _mask64;
    return (_state >>> 32) & _mask32;
  }

  /// Return a 64-bit value composed of two consecutive [nextU32] calls.
  ///
  /// Composition: `(hi << 32) | lo` where hi and lo are successive draws.
  int nextU64() {
    final hi = nextU32();
    final lo = nextU32();
    return ((hi << 32) | lo) & _mask64;
  }

  /// Return a double uniformly distributed in `[0.0, 1.0)`.
  ///
  /// Divides the 32-bit draw by 2^32, giving 2^32 evenly spaced values.
  double nextFloat() {
    return nextU32() / _floatDiv;
  }

  /// Return an integer uniformly drawn from `[min, max]` inclusive, by the
  /// rejection sampling of [_intInRange]. Throws [ArgumentError] if
  /// `min > max`, before any draw.
  int nextIntInRange(int min, int max) => _intInRange(nextU32, min, max);
}

// ─── Xorshift64 ──────────────────────────────────────────────────────────────

/// Xorshift64 (Marsaglia 2003).
///
/// Three XOR-shift operations scramble 64-bit state with no multiplication:
///
///   x ^= x << 13
///   x ^= x >>> 7
///   x ^= x << 17
///
/// Each shift "smears" bits from one end of the word toward the other. The
/// specific constants (13, 7, 17) were found by exhaustive search to produce
/// a maximal-period linear feedback register over GF(2) — the sequence visits
/// every non-zero 64-bit state exactly once before repeating.
///
/// Period: 2^64 − 1. State 0 is a fixed point (XOR of 0 is always 0), so
/// seed 0 is replaced with 1.
///
/// Output: lower 32 bits.
class Xorshift64 {
  int _state;

  /// Create an [Xorshift64] seeded with [seed].
  /// Seed 0 is replaced with 1 to avoid the zero fixed point.
  Xorshift64(int seed) : _state = (seed == 0 ? 1 : seed) & _mask64;

  /// Apply the three XOR-shifts and return the lower 32 bits.
  ///
  ///   x ^= x << 13   — bleed high bits downward
  ///   x ^= x >>> 7   — bleed low bits upward
  ///   x ^= x << 17   — bleed high bits downward again
  ///
  /// Each intermediate value is masked to 64 bits to simulate hardware wrap.
  int nextU32() {
    int x = _state;
    x = (x ^ (x << 13)) & _mask64;
    x = (x ^ (x >>> 7)) & _mask64;
    x = (x ^ (x << 17)) & _mask64;
    _state = x;
    return x & _mask32;
  }

  /// Return a 64-bit value composed of two consecutive [nextU32] calls.
  int nextU64() {
    final hi = nextU32();
    final lo = nextU32();
    return ((hi << 32) | lo) & _mask64;
  }

  /// Return a double in `[0.0, 1.0)`.
  double nextFloat() {
    return nextU32() / _floatDiv;
  }

  /// Return an integer uniformly drawn from `[min, max]` inclusive, by the
  /// rejection sampling of [_intInRange]. Throws [ArgumentError] if
  /// `min > max`, before any draw.
  int nextIntInRange(int min, int max) => _intInRange(nextU32, min, max);
}

// ─── Pcg32 ───────────────────────────────────────────────────────────────────

/// PCG32 — Permuted Congruential Generator (O'Neill 2014).
///
/// Uses the same LCG recurrence as [Lcg] but adds an XSH RR output
/// permutation before returning ("XOR-Shift High / Random Rotate"):
///
///  1. Capture old_state before the LCG advance.
///  2. Advance: `state = (old_state × a + c) mod 2^64`
///  3. `xorshifted = ((old_state >>> 18) ^ old_state) >>> 27`
///     — mix two different views of the high bits down to 32 bits.
///  4. `rot = old_state >>> 59`
///     — extract a 5-bit rotation amount from the top of old_state.
///  5. `output = rotr32(xorshifted, rot)`
///     — rotate right so all bits contribute to the output.
///
/// The rotation step makes the output function non-linear, breaking the
/// correlations that cause plain LCG to fail statistical tests. PCG32 passes
/// TestU01 BigCrush and PractRand with only 8 bytes of state.
///
/// Initialization: start from state=0, advance once, add seed, advance again.
class Pcg32 {
  int _state;
  final int _increment;

  /// Create a [Pcg32] seeded with [seed].
  ///
  /// The increment is fixed to `lcgIncrement | 1` (must be odd for full
  /// period). Two warm-up advances scatter seed bits through all 64 state
  /// bits before the first [nextU32] call.
  Pcg32(int seed)
      : _state = 0,
        _increment = (_lcgIncrement | 1) & _mask64 {
    // initseq warm-up: advance, mix in seed, advance again.
    _state = ((_state * _lcgMultiplier) + _increment) & _mask64;
    _state = (_state + seed) & _mask64;
    _state = ((_state * _lcgMultiplier) + _increment) & _mask64;
  }

  /// Advance the LCG and return the XSH RR permuted 32-bit output.
  ///
  /// The rotation formula in Dart:
  ///
  ///   `((xorshifted >>> rot) | (xorshifted << (32 - rot))) & _mask32`
  ///
  /// which equals `rotr32(xorshifted, rot)` from the C reference.
  int nextU32() {
    final oldState = _state;
    _state = ((oldState * _lcgMultiplier) + _increment) & _mask64;

    // XSH RR permutation on old_state.
    final xorshifted = (((oldState >>> 18) ^ oldState) >>> 27) & _mask32;
    final rot        = oldState >>> 59; // top 5 bits

    return ((xorshifted >>> rot) | (xorshifted << (32 - rot))) & _mask32;
  }

  /// Return a 64-bit value composed of two consecutive [nextU32] calls.
  int nextU64() {
    final hi = nextU32();
    final lo = nextU32();
    return ((hi << 32) | lo) & _mask64;
  }

  /// Return a double in `[0.0, 1.0)`.
  double nextFloat() {
    return nextU32() / _floatDiv;
  }

  /// Return an integer uniformly drawn from `[min, max]` inclusive, by the
  /// rejection sampling of [_intInRange]. Throws [ArgumentError] if
  /// `min > max`, before any draw.
  int nextIntInRange(int min, int max) => _intInRange(nextU32, min, max);
}

// ─── Bounded draws (spec rng.md, "Rejection Sampling", v2) ───────────────────

final BigInt _twoTo64 = BigInt.one << 64;

/// A uniform integer in `[min, max]`, drawn from a stream of 32-bit outputs,
/// identically in every language of the repository.
///
/// `span = max − min + 1` lies between 1 and 2^64. Taking `r % span` of a raw
/// draw over-samples small offsets whenever the number of possible draws is
/// not a multiple of span; rejecting the `threshold` lowest draws removes
/// exactly the excess. Three regimes, chosen by span alone:
///
/// ```text
///   span <= 2^32    draw 32 bits,  threshold = (2^32 − span) mod span
///   span <  2^64    draw 64 bits,  threshold = (2^64 − span) mod span
///   span == 2^64    the whole int domain: every u64 is a valid offset
/// ```
///
/// Dart's `int` is signed 64-bit with no unsigned compare or divide, so the
/// 32-bit regime (where every value is non-negative) uses `int`, and the wide
/// regimes use [BigInt], where `span` and the 64-bit draw are exact.
///
/// v1 took the threshold modulo 2^64 while drawing 32 bits, and computed
/// `max - min + 1` in wrapping `int`: biased, never more than 2^32 − 1 above
/// `min`, wrong for spans past 2^63, and for spans such as 2^63 + 1 every draw
/// was rejected, so it never returned.
int _intInRange(int Function() next32, int min, int max) {
  if (min > max) {
    throw ArgumentError('nextIntInRange requires min <= max, got $min > $max');
  }
  final span = BigInt.from(max) - BigInt.from(min) + BigInt.one;
  if (span <= BigInt.from(1 << 32)) {
    final s = span.toInt();
    final threshold = ((1 << 32) - s) % s;
    while (true) {
      final r = next32();
      if (r >= threshold) return min + r % s;
    }
  }
  BigInt next64() {
    final hi = BigInt.from(next32());
    return (hi << 32) | BigInt.from(next32());
  }

  if (span < _twoTo64) {
    final threshold = (_twoTo64 - span) % span;
    while (true) {
      final r = next64();
      if (r >= threshold) return (BigInt.from(min) + r % span).toInt();
    }
  }
  return (BigInt.from(min) + next64()).toInt();
}
