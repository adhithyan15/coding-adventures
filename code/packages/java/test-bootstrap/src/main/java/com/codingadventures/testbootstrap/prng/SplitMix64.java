package com.codingadventures.testbootstrap.prng;

import java.math.BigInteger;
import java.nio.charset.StandardCharsets;

/**
 * Deterministic randomness shared by every test engine in the Java lane
 * (specification: VER05e section 2).
 *
 * <p>Why a hand-written PRNG instead of {@code java.util.Random}? A property
 * test is only useful if a failure can be replayed. "Seed 42 failed" must mean
 * the same sequence of candidates on every machine, every JDK release and every
 * other language lane. SplitMix64 is five lines of 64-bit arithmetic that any
 * language can reproduce bit for bit, and the shared {@code pbt-v1} corpus pins
 * its first outputs.
 *
 * <h2>SplitMix64</h2>
 *
 * The state is one 64-bit word. Each step adds the golden-ratio constant and
 * scrambles the sum with two xor-shift-multiply rounds:
 *
 * <pre>
 *   state  += 0x9e3779b97f4a7c15
 *   z       = state
 *   z       = (z ^ (z >>> 30)) * 0xbf58476d1ce4e5b9
 *   z       = (z ^ (z >>> 27)) * 0x94d049bb133111eb
 *   output  = z ^ (z >>> 31)
 * </pre>
 *
 * Java's {@code long} wraps modulo 2^64, which is exactly the arithmetic the
 * algorithm assumes. The one trap is the shift: {@code >>} is an arithmetic
 * (sign-extending) shift, so every right shift here is the unsigned {@code >>>}.
 * Pinned vector: seed 0's first output is {@code 0xe220a8397b1dcdaf}.
 *
 * <h2>Unsigned words in a signed language</h2>
 *
 * Java has no unsigned 64-bit type. A {@code long} holds the same 64 bits, but
 * {@code <}, {@code %} and {@code /} read them as signed. Every comparison and
 * remainder on a stream word therefore goes through
 * {@link Long#compareUnsigned} / {@link Long#remainderUnsigned}.
 *
 * <h2>Bounded draws without bias</h2>
 *
 * {@code x % n} is biased unless n divides 2^64: the residues below 2^64 mod n
 * occur once more often than the rest. Rejection sampling removes the bias. With
 * the threshold {@code t = (2^64 - n) mod n}, every raw value below t is thrown
 * away; the 2^64 - t values that remain split into exactly equal residue
 * classes.
 *
 * <p>Worked example, n = 3: 2^64 = 3 * 6148914691236517205 + 1, so t = 1. Only the
 * raw value 0 is rejected; the other 2^64 - 1 values fall evenly into residues
 * 0, 1, 2.
 *
 * <p>Not safe for concurrent use.
 */
public final class SplitMix64 {
  private static final long GOLDEN_GAMMA = 0x9e3779b97f4a7c15L;
  private static final long MIX_MUL_1 = 0xbf58476d1ce4e5b9L;
  private static final long MIX_MUL_2 = 0x94d049bb133111ebL;
  private static final long FNV_OFFSET_BASIS = 0xcbf29ce484222325L;
  private static final long FNV_PRIME = 0x00000100000001b3L;

  private static final BigInteger ONE = BigInteger.ONE;
  private static final BigInteger TWO_64 = ONE.shiftLeft(64);

  private long state;

  /** A generator whose stream is fully determined by {@code seed}. */
  public SplitMix64(long seed) {
    this.state = seed;
  }

  /** The current internal state (the next {@link #nextU64} starts from it). */
  public long state() {
    return state;
  }

  /** Restores a state previously returned by {@link #state()}. */
  public void setState(long newState) {
    this.state = newState;
  }

  /** Advances the stream and returns the next 64 random bits (as a signed long). */
  public long nextU64() {
    state += GOLDEN_GAMMA;
    long z = state;
    z = (z ^ (z >>> 30)) * MIX_MUL_1;
    z = (z ^ (z >>> 27)) * MIX_MUL_2;
    return z ^ (z >>> 31);
  }

  /**
   * A uniform value in [0, n) where {@code n} is read as an <em>unsigned</em>
   * 64-bit integer, 1 &lt;= n &lt;= 2^64-1. The result is also an unsigned word.
   *
   * <p>It always draws at least one word, even for n == 1 (where the answer is
   * 0): that is the VER05 section 2.2 loop run once, and it keeps the stream
   * position of every generator identical across language lanes. n == 0 is a
   * caller bug. A span of exactly 2^64 cannot be written as a long: use
   * {@link #nextU64()}.
   */
  public long bounded(long n) {
    if (n == 0) {
      throw new IllegalArgumentException("bounded(0) has no valid result");
    }
    // -n is 2^64 - n modulo 2^64, so this is (2^64 - n) mod n, computed unsigned.
    // (Python's "(-n) % n" is a different, wrong number; Java's signed "%"
    // would be wrong too, which is why this is remainderUnsigned.)
    long threshold = Long.remainderUnsigned(-n, n);
    while (true) {
      long x = nextU64();
      if (Long.compareUnsigned(x, threshold) >= 0) {
        return Long.remainderUnsigned(x, n);
      }
    }
  }

  /**
   * A uniform value in [0, span) for any span &gt;= 1.
   *
   * <pre>
   *   span &lt;= 2^64-1   bounded(span)
   *   span == 2^64     one raw word
   *   span  &gt; 2^64     k-word rejection sampling
   * </pre>
   *
   * For a wide span take k = max(2, ceil(bitlen(span-1)/64)) words, first draw
   * most significant, giving raw uniform on [0, 2^(64k)). A plain raw % span
   * would over-weight the low residues (for span = 3*2^126 the low third would
   * be drawn half the time instead of a third), so raw values below
   * t = (2^(64k) - span) mod span are redrawn. The acceptance probability is at
   * least one half.
   */
  public BigInteger boundedBig(BigInteger span) {
    if (span == null || span.signum() <= 0) {
      throw new IllegalArgumentException("boundedBig needs span >= 1");
    }
    int cmp = span.compareTo(TWO_64);
    if (cmp < 0) {
      return unsigned(bounded(span.longValue()));
    }
    if (cmp == 0) {
      return unsigned(nextU64());
    }
    int words = (span.subtract(ONE).bitLength() + 63) / 64;
    if (words < 2) {
      words = 2;
    }
    BigInteger modulus = ONE.shiftLeft(64 * words);
    BigInteger threshold = modulus.subtract(span).mod(span);
    while (true) {
      BigInteger raw = BigInteger.ZERO;
      for (int i = 0; i < words; i++) {
        raw = raw.shiftLeft(64).or(unsigned(nextU64()));
      }
      if (raw.compareTo(threshold) >= 0) {
        return raw.mod(span);
      }
    }
  }

  /** The non-negative BigInteger whose 64 bits are {@code word}. */
  public static BigInteger unsigned(long word) {
    BigInteger v = BigInteger.valueOf(word);
    return word < 0 ? v.add(TWO_64) : v;
  }

  /**
   * 64-bit FNV-1a: start from the offset basis; for each byte xor it in, then
   * multiply by the FNV prime (mod 2^64).
   *
   * <pre>
   *   fnv1a64("")  = 0xcbf29ce484222325
   *   fnv1a64("a") = 0xaf63dc4c8601ec8c
   * </pre>
   */
  public static long fnv1a64(byte[] data) {
    long h = FNV_OFFSET_BASIS;
    for (byte b : data) {
      h = (h ^ (b & 0xffL)) * FNV_PRIME;
    }
    return h;
  }

  /** FNV-1a over the UTF-8 bytes of {@code s}. */
  public static long fnv1a64(String s) {
    return fnv1a64(s.getBytes(StandardCharsets.UTF_8));
  }

  /**
   * The isolated seed of one property inside a run (VER05 section 2.3):
   *
   * <pre>
   *   subSeed(run, id) = SplitMix64(run XOR fnv1a64(utf8(id))).nextU64()
   * </pre>
   *
   * Two properties in one run never share a stream, and adding or reordering
   * properties never changes another property's draws. The final nextU64 is a
   * scramble: without it two ids whose hashes differ in one bit would produce
   * seeds differing in one bit, so their streams would be strongly related.
   */
  public static long subSeed(long runSeed, String id) {
    return new SplitMix64(runSeed ^ fnv1a64(id)).nextU64();
  }
}
