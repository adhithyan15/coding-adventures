package com.codingadventures.testbootstrap.prng;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigInteger;
import org.junit.jupiter.api.Test;

/**
 * Vectors below were produced by the independent reference
 * code/scripts/pbt_conformance.py (via the Go lane's pinned copies), never by
 * this package.
 */
class SplitMix64Test {
  private static long u(String decimal) {
    return Long.parseUnsignedLong(decimal);
  }

  @Test
  void pinnedVector() {
    SplitMix64 r = new SplitMix64(0);
    assertEquals(0xe220a8397b1dcdafL, r.nextU64());
    assertEquals(0x6e789e6aa1b965f4L, r.nextU64());
    assertEquals(0x06c45d188009454fL, r.nextU64());
  }

  @Test
  void stateRoundTrip() {
    SplitMix64 r = new SplitMix64(7);
    r.nextU64();
    long saved = r.state();
    long a = r.nextU64();
    r.setState(saved);
    assertEquals(a, r.nextU64());
  }

  @Test
  void boundedSmallVector() {
    SplitMix64 r = new SplitMix64(5);
    for (long want : new long[] {3, 5, 2, 2, 3}) {
      assertEquals(want, r.bounded(7));
    }
  }

  // A span of 2^63+1 rejects raw values below (2^64 - n) mod n = 2^63 - 1,
  // about half of all draws. A threshold that is always 0 would change these.
  @Test
  void boundedWideThresholdIsApplied() {
    long n = (1L << 63) + 1; // unsigned 2^63 + 1
    SplitMix64 r = new SplitMix64(5);
    for (String want :
        new String[] {
          "4654242949169100535", "8957066056171264800", "204786321411665706",
          "1908141438795372386", "6903704994719435514"
        }) {
      assertEquals(u(want), r.bounded(n));
    }
  }

  @Test
  void boundedOneConsumesOneWord() {
    SplitMix64 r = new SplitMix64(5);
    for (int i = 0; i < 3; i++) {
      assertEquals(0, r.bounded(1));
    }
    assertEquals(u("15755400384260043844"), r.state());
  }

  @Test
  void boundedZeroIsRejected() {
    assertThrows(IllegalArgumentException.class, () -> new SplitMix64(1).bounded(0));
  }

  @Test
  void boundedStaysInRangeAcrossSeeds() {
    long[] spans = {1, 2, 3, 7, 255, 256, 1000, (1L << 32) + 1, (1L << 63) + 1, -1L};
    for (long seed = 0; seed < 2000; seed++) {
      SplitMix64 r = new SplitMix64(seed);
      for (long n : spans) {
        assertTrue(Long.compareUnsigned(r.bounded(n), n) < 0, "seed " + seed + " n " + Long.toUnsignedString(n));
      }
    }
  }

  @Test
  void boundedIsRoughlyUniform() {
    SplitMix64 r = new SplitMix64(99);
    int[] counts = new int[3];
    final int draws = 30000;
    for (int i = 0; i < draws; i++) {
      counts[(int) r.bounded(3)]++;
    }
    for (int c : counts) {
      assertTrue(Math.abs(c - draws / 3) < 600, "count " + c);
    }
  }

  // The finalizer of SplitMix64 is a bijection, so we can compute the seed whose
  // next output is any chosen word. That lets a test hand bounded() a raw word
  // sitting exactly on the rejection threshold, which the corpus never does
  // (its spans are tiny, so the threshold almost never fires).
  private static long seedWhoseNextOutputIs(long output) {
    BigInteger mod = BigInteger.ONE.shiftLeft(64);
    long inv1 = BigInteger.valueOf(0xbf58476d1ce4e5b9L).add(mod).mod(mod).modInverse(mod).longValue();
    long inv2 = BigInteger.valueOf(0x94d049bb133111ebL).add(mod).mod(mod).modInverse(mod).longValue();
    long z = output;
    z ^= z >>> 31 ^ z >>> 62;
    z *= inv2;
    z ^= z >>> 27 ^ z >>> 54;
    z *= inv1;
    z ^= z >>> 30 ^ z >>> 60;
    long seed = z - 0x9e3779b97f4a7c15L;
    assertEquals(output, new SplitMix64(seed).nextU64(), "inversion must round-trip");
    return seed;
  }

  @Test
  void rejectionBranchFiresExactlyBelowTheThreshold() {
    // n = 3: threshold (2^64 - 3) mod 3 = 1, so only the raw word 0 is rejected.
    long seed = seedWhoseNextOutputIs(0);
    SplitMix64 r = new SplitMix64(seed);
    SplitMix64 ref = new SplitMix64(seed);
    ref.nextU64(); // the rejected word
    long second = ref.nextU64();
    assertEquals(Long.remainderUnsigned(second, 3), r.bounded(3));
    assertEquals(ref.state(), r.state(), "a rejected word is consumed, then exactly one more");

    // The raw word 1 is the first accepted one: no second draw.
    SplitMix64 accepted = new SplitMix64(seedWhoseNextOutputIs(1));
    assertEquals(1, accepted.bounded(3));
    assertEquals(seedWhoseNextOutputIs(1) + 0x9e3779b97f4a7c15L, accepted.state());
  }

  @Test
  void wideThresholdBoundaryIsExact() {
    long n = (1L << 63) + 1;
    long threshold = (1L << 63) - 1; // (2^64 - n) mod n
    // word == threshold is accepted and reduced mod n.
    SplitMix64 on = new SplitMix64(seedWhoseNextOutputIs(threshold));
    assertEquals(threshold, on.bounded(n));
    // word == threshold - 1 is rejected: a second word is consumed.
    long seed = seedWhoseNextOutputIs(threshold - 1);
    SplitMix64 below = new SplitMix64(seed);
    below.bounded(n);
    assertNotEquals(seed + 0x9e3779b97f4a7c15L, below.state());
    // A top-bit-set word is far above the threshold (unsigned compare, not signed).
    long high = 0xfedcba9876543210L;
    SplitMix64 top = new SplitMix64(seedWhoseNextOutputIs(high));
    assertEquals(Long.remainderUnsigned(high, n), top.bounded(n));
  }

  @Test
  void boundedBigSmallSpansMatchBounded() {
    SplitMix64 a = new SplitMix64(11);
    SplitMix64 b = new SplitMix64(11);
    for (int i = 0; i < 50; i++) {
      assertEquals(SplitMix64.unsigned(b.bounded(1000)), a.boundedBig(BigInteger.valueOf(1000)));
    }
  }

  @Test
  void boundedBigFullWordSpanIsRawWord() {
    SplitMix64 a = new SplitMix64(3);
    SplitMix64 b = new SplitMix64(3);
    assertEquals(SplitMix64.unsigned(b.nextU64()), a.boundedBig(BigInteger.ONE.shiftLeft(64)));
  }

  @Test
  void boundedBigWideVectors() {
    SplitMix64 r = new SplitMix64(9);
    BigInteger span = BigInteger.ONE.shiftLeft(130).add(BigInteger.valueOf(7));
    for (String want :
        new String[] {
          "255448235011303640280899557894340534535",
          "89342299821301620937131352229074699094",
          "334684220971821213071345962061252959748"
        }) {
      assertEquals(new BigInteger(want), r.boundedBig(span));
    }
    r = new SplitMix64(9);
    span = BigInteger.valueOf(3).shiftLeft(126);
    for (String want :
        new String[] {
          "232196006557297741543928818731175635554",
          "90284548084330202170444353266735210592",
          "89342299821301620962466552557356346366"
        }) {
      assertEquals(new BigInteger(want), r.boundedBig(span));
    }
  }

  // VER05 section 7.2: raw % span over-weights the low third of a 3*2^126 span.
  @Test
  void boundedBigThirdsAreUnbiased() {
    BigInteger span = BigInteger.valueOf(3).shiftLeft(126);
    BigInteger third = BigInteger.ONE.shiftLeft(126);
    SplitMix64 r = new SplitMix64(2024);
    int[] counts = new int[3];
    final int draws = 30000;
    for (int i = 0; i < draws; i++) {
      BigInteger v = r.boundedBig(span);
      assertTrue(v.signum() >= 0 && v.compareTo(span) < 0);
      counts[v.divide(third).intValue()]++;
    }
    for (int c : counts) {
      assertTrue(Math.abs(c - draws / 3) < 700, "third count " + c);
    }
  }

  @Test
  void boundedBigRejectsBadSpan() {
    for (BigInteger span : new BigInteger[] {null, BigInteger.ZERO, BigInteger.valueOf(-5)}) {
      assertThrows(IllegalArgumentException.class, () -> new SplitMix64(1).boundedBig(span));
    }
  }

  @Test
  void fnvVectors() {
    assertEquals(0xcbf29ce484222325L, SplitMix64.fnv1a64(""));
    assertEquals(0xaf63dc4c8601ec8cL, SplitMix64.fnv1a64("a"));
    assertEquals(0x85944171f73967e8L, SplitMix64.fnv1a64("foobar"));
    assertEquals(0x85944171f73967e8L, SplitMix64.fnv1a64("foobar".getBytes(java.nio.charset.StandardCharsets.UTF_8)));
    // Bytes above 0x7f must be hashed as unsigned values (a sign-extended byte is a classic bug).
    assertEquals(SplitMix64.fnv1a64(new byte[] {(byte) 0xc3, (byte) 0xa9}), SplitMix64.fnv1a64("é"));
    assertNotEquals(SplitMix64.fnv1a64(new byte[] {(byte) 0xff}), SplitMix64.fnv1a64(new byte[] {(byte) 0x7f}));
  }

  @Test
  void subSeedVectors() {
    assertEquals(0x338262d8f096398fL, SplitMix64.subSeed(0, "x"));
    assertEquals(0xcca3a0330363d462L, SplitMix64.subSeed(42, "prop/ct_compare_eq_reflexive"));
    assertEquals(0x30ef66c3e79dda4eL, SplitMix64.subSeed(-1L, ""));
    // Without the final scramble the answer would be run XOR hash (TS/Dart lanes).
    assertNotEquals(SplitMix64.fnv1a64("x"), SplitMix64.subSeed(0, "x"));
  }

  @Test
  void unsignedConversion() {
    assertEquals(BigInteger.ONE.shiftLeft(64).subtract(BigInteger.ONE), SplitMix64.unsigned(-1L));
    assertEquals(BigInteger.valueOf(5), SplitMix64.unsigned(5));
  }
}
