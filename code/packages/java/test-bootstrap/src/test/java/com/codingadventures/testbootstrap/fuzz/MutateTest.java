package com.codingadventures.testbootstrap.fuzz;

import static com.codingadventures.testbootstrap.fuzz.Fx.bg;
import static com.codingadventures.testbootstrap.fuzz.Fx.hex;
import static com.codingadventures.testbootstrap.fuzz.Fx.hx;
import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotSame;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.codingadventures.testbootstrap.fuzz.Mutate.Endian;
import com.codingadventures.testbootstrap.prng.SplitMix64;
import java.math.BigInteger;
import java.util.Set;
import java.util.TreeSet;
import org.junit.jupiter.api.Test;

/**
 * Unit tests of the mutator algebra, including the targeted tests for the known
 * weaknesses of the shared corpus (backlog PT-F1): the single splice case has
 * equal splits and the single mutate_sequence case does not pin every constant.
 */
class MutateTest {
  // ---- PT-F1: splice splits ------------------------------------------------------

  @Test
  void spliceUsesTheTargetSplitForThePrefixAndTheDonorSplitForTheSuffix() {
    byte[] t = hx("01020304");
    byte[] d = hx("05060708");
    assertEquals("0108", hex(Mutate.splice(t, d, 1, 3)));
    // Swapping the splits is a different, and here distinguishable, result.
    assertEquals("010203060708", hex(Mutate.splice(t, d, 3, 1)));
    // Swapping target and donor is also visible.
    assertEquals("0506070304", hex(Mutate.splice(d, t, 3, 2)));
    // Unequal buffer lengths make a swapped clamp visible too.
    byte[] longer = hx("0506070809");
    assertEquals("01020304" + "0809", hex(Mutate.splice(t, longer, 4, 3)));
    assertEquals("010203" + "09", hex(Mutate.splice(t, longer, 3, 4)));
    assertEquals("01" + "0506070809", hex(Mutate.splice(t, longer, 1, 0)));
  }

  @Test
  void spliceClampsBothSplitsIndependently() {
    byte[] t = hx("0102");
    byte[] d = hx("0304");
    assertEquals("0304", hex(Mutate.splice(t, d, -5, 0)));
    assertEquals("0102", hex(Mutate.splice(t, d, 99, 99)));
    assertEquals("010204", hex(Mutate.splice(t, d, 99, 1)));
    assertEquals("04", hex(Mutate.splice(new byte[0], d, 7, 1)));
    assertEquals("0102", hex(Mutate.spliceBig(t, d, bg("100000000000000000000000000"), bg("100000000000000000000000000"))));
    assertEquals("0304", hex(Mutate.spliceBig(t, d, bg("-100000000000000000000000000"), bg("-100000000000000000000000000"))));
  }

  // ---- PT-F1: mutate_sequence constants -------------------------------------------

  @Test
  void interestListsAreExactlyTheSpecifiedConstants() {
    assertArrayEquals(new long[] {0, 1, 127, 128, 255}, Mutate.interestValues(1));
    assertArrayEquals(new long[] {0, 1, 255, 256, 32767, 32768, 65535}, Mutate.interestValues(2));
    assertArrayEquals(new long[] {0, 1, 65535, 65536, 2147483647L, 2147483648L, 4294967295L}, Mutate.interestValues(4));
    long[] w8 = Mutate.interestValues(8);
    String[] expected = {"0", "1", "4294967295", "4294967296", "9223372036854775807", "9223372036854775808", "18446744073709551615"};
    for (int i = 0; i < expected.length; i++) {
      assertEquals(expected[i], SplitMix64.unsigned(w8[i]).toString());
    }
    assertNull(Mutate.interestValues(3));
    assertNull(Mutate.interestValues(0));
    Mutate.interestValues(1)[0] = 99; // a copy
    assertEquals(0, Mutate.interestValues(1)[0]);
    assertArrayEquals(new long[] {-35, -8, -1, 1, 8, 35}, Mutate.sequenceDeltas());
  }

  @Test
  void everyInterestValueLandsAtEveryWidthAndEndian() {
    for (int w : new int[] {1, 2, 4, 8}) {
      for (long v : Mutate.interestValues(w)) {
        BigInteger big = SplitMix64.unsigned(v);
        for (Endian e : Endian.values()) {
          byte[] got = Mutate.boundaryInsert(new byte[w + 1], 1, w, v, e);
          byte[] want = new byte[w + 1];
          for (int i = 0; i < w; i++) {
            int shift = 8 * (e == Endian.BIG ? w - 1 - i : i);
            want[1 + i] = big.shiftRight(shift).byteValue();
          }
          assertArrayEquals(want, got, "w=" + w + " v=" + big + " " + e);
        }
      }
    }
  }

  @Test
  void aSingleStepSequenceFromOneZeroByteProducesExactlyTheSpecifiedValues() {
    Set<Integer> seen = new TreeSet<>();
    for (long seed = 0; seed < 6000; seed++) {
      byte[] out = Mutate.mutateSequence(new byte[] {0}, seed, 1, 1); // maxLen 1: no growth
      assertEquals(1, out.length);
      seen.add(out[0] & 0xFF);
    }
    Set<Integer> want = new TreeSet<>();
    for (int k = 0; k < 8; k++) {
      want.add(1 << k); // bit flips
    }
    want.add(0xFF); // byte flip, and delta -1
    for (int d : new int[] {-35, -8, -1, 1, 8, 35}) {
      want.add(d & 0xFF); // arithmetic deltas on 0
    }
    for (int v : new int[] {0, 1, 127, 128, 255}) {
      want.add(v); // boundary values
    }
    assertEquals(want, seen);
  }

  @Test
  void arithmeticDeltasAreAddedExactly() {
    Set<String> seen = new TreeSet<>();
    for (long seed = 0; seed < 6000; seed++) {
      seen.add(hex(Mutate.mutateSequence(hx("80"), seed, 1, 1)));
    }
    // 0x80 + {-35,-8,-1,1,8,35} = 5d 78 7f 81 88 a3, plus flips and boundary values.
    for (String s : new String[] {"5d", "78", "7f", "81", "88", "a3"}) {
      assertTrue(seen.contains(s), s + " missing from " + seen);
    }
    for (String s : new String[] {"5c", "5e", "77", "79", "a2", "a4"}) {
      assertFalse(seen.contains(s), s + " must not appear: " + seen);
    }
  }

  @Test
  void sequenceReferenceValuesFromTheOracle() {
    assertEquals("00ddb08a3e0c0753ff7ffd0100c6fd", hex(Mutate.mutateSequence(hx("b9892d5158ae8d3dfdfa"), 42, 40, 1048576)));
    assertEquals("72bfca0b54", hex(Mutate.mutateSequence(hx("72cb54"), 42, 4, 1048576)));
    assertEquals("012bff013b", hex(Mutate.mutateSequence(hx("5d296d413b"), -1L, 8, 5)));
    assertEquals("7f82", hex(Mutate.mutateSequence(hx("4a8b6329b5"), 0, 16, 1)));
    assertEquals("03ff00000000bf080000", hex(Mutate.mutateSequence(hx("0000000000000000"), 42, 4, 1 << 20)));
  }

  // ---- flips ----------------------------------------------------------------------

  @Test
  void flipsApplyByParityForEnormousCounts() {
    byte[] data = {0, 0};
    assertEquals("feff", hex(Mutate.bitFlip(data, 0, 17)));
    assertEquals(2, Mutate.bitFlip(data, 3, 1_000_000_000_000_000_000L).length);
    assertEquals("0002", hex(Mutate.bitFlip(data, 9, 1)));
    assertEquals("0100", hex(Mutate.bitFlip(data, 0, 1)));
    assertEquals("3c00", hex(Mutate.bitFlip(data, 2, 4)));
    assertEquals("0001", hex(Mutate.bitFlip(data, -8, 1))); // floor modulo: -8 is bit 8
    assertEquals("0080", hex(Mutate.bitFlip(data, -1, 1)));
    assertEquals("ffff", hex(Mutate.bitFlip(data, 0, 16)));
    assertEquals("0000", hex(Mutate.bitFlip(data, 0, 32)));
    assertEquals("0000", hex(Mutate.bitFlip(data, 5, 0)));
    assertEquals("0000", hex(Mutate.bitFlip(data, 5, -3)));
    assertEquals("", hex(Mutate.bitFlip(new byte[0], 5, 3)));
    assertEquals("edcba9", hex(Mutate.byteFlip(hx("123456"), 0, 3)));
    assertEquals("12cba9", hex(Mutate.byteFlip(hx("123456"), 1, 2)));
    assertEquals("12cba9", hex(Mutate.byteFlip(hx("123456"), -2, 2)));
    assertEquals("1234a9", hex(Mutate.byteFlip(hx("123456"), 0, 5)));
    assertEquals("123456", hex(Mutate.byteFlip(hx("123456"), 7, 0)));
    assertEquals("", hex(Mutate.byteFlip(new byte[0], 1, 1)));
    // Big forms agree, including a count far beyond a long.
    BigInteger huge = bg("1000000000000000000000000000001");
    assertEquals(hex(Mutate.bitFlip(data, 1, 1)), hex(Mutate.bitFlipBig(data, bg("1"), BigInteger.ONE)));
    assertEquals(2, Mutate.bitFlipBig(data, huge, huge).length);
    assertEquals("0000", hex(Mutate.bitFlipBig(new byte[2], BigInteger.ZERO, BigInteger.ZERO)));
    assertEquals("", hex(Mutate.byteFlipBig(new byte[0], huge, huge)));
  }

  @Test
  void flipsMatchANaiveLoopForEveryOffsetAndSmallCount() {
    byte[] data = hx("a53c");
    for (int off = -20; off < 20; off++) {
      for (int count = 0; count < 40; count++) {
        byte[] naive = data.clone();
        for (int i = 0; i < count; i++) {
          int pos = Math.floorMod(off + i, 16);
          naive[pos / 8] ^= (byte) (1 << (pos % 8));
        }
        assertArrayEquals(naive, Mutate.bitFlip(data, off, count), "bit off=" + off + " count=" + count);
        assertArrayEquals(naive, Mutate.bitFlipBig(data, BigInteger.valueOf(off), BigInteger.valueOf(count)));
        byte[] naiveBytes = data.clone();
        for (int i = 0; i < count; i++) {
          naiveBytes[Math.floorMod(off + i, 2)] ^= (byte) 0xFF;
        }
        assertArrayEquals(naiveBytes, Mutate.byteFlip(data, off, count), "byte off=" + off + " count=" + count);
      }
    }
  }

  // ---- arithmetic and boundary -----------------------------------------------------

  @Test
  void arithmeticWrapsAtEveryWidthAndEndian() {
    assertEquals("00", hex(Mutate.byteArithmetic(hx("ff"), 0, 1, 1, Endian.LITTLE)));
    assertEquals("0001", hex(Mutate.byteArithmetic(hx("ff00"), 0, 1, 2, Endian.LITTLE)));
    assertEquals("0100", hex(Mutate.byteArithmetic(hx("00ff"), 0, 1, 2, Endian.BIG)));
    assertEquals("00000000", hex(Mutate.byteArithmetic(hx("ffffffff"), 0, 1, 4, Endian.BIG)));
    assertEquals("0000000000000000", hex(Mutate.byteArithmetic(hx("ffffffffffffffff"), 0, 1, 8, Endian.LITTLE)));
    assertEquals("ffffffffffffffff", hex(Mutate.byteArithmetic(hx("0000000000000000"), 0, -1, 8, Endian.BIG)));
    assertEquals("fe", hex(Mutate.byteArithmetic(hx("00"), 0, -2, 1, Endian.LITTLE)));
    assertEquals("0000ff", hex(Mutate.byteArithmetic(hx("0000fe"), -1, 1, 1, Endian.LITTLE))); // offset wraps over 3 positions
    assertEquals("ff0000", hex(Mutate.byteArithmetic(hx("fe0000"), 3, 1, 1, Endian.LITTLE)));
    assertEquals("ffff", hex(Mutate.byteArithmetic(hx("ffff"), 0, 0x10000, 2, Endian.LITTLE))); // delta mod 2^16
    assertEquals("ab", hex(Mutate.byteArithmetic(hx("ab"), 0, 5, 3, Endian.LITTLE))); // bad width
    assertEquals("ab", hex(Mutate.byteArithmetic(hx("ab"), 0, 5, 2, Endian.LITTLE))); // too short
    assertEquals("ab", hex(Mutate.byteArithmetic(hx("ab"), 0, 5, 0, Endian.LITTLE)));
    assertEquals("ab", hex(Mutate.byteArithmeticBig(hx("ab"), BigInteger.ZERO, BigInteger.ONE, 9, Endian.BIG)));
    assertEquals("", hex(Mutate.byteArithmetic(new byte[0], 0, 1, 1, Endian.LITTLE)));
    assertEquals(
        hex(Mutate.byteArithmetic(hx("0123456789abcdef"), 0, -3, 8, Endian.BIG)),
        hex(Mutate.byteArithmeticBig(hx("0123456789abcdef"), BigInteger.ZERO, bg("18446744073709551613"), 8, Endian.BIG)));
  }

  @Test
  void boundaryInsertsExactBytes() {
    assertEquals("00000080", hex(Mutate.boundaryInsert(hx("00000000"), 0, 4, 0x80000000L, Endian.LITTLE)));
    assertEquals("80000000", hex(Mutate.boundaryInsert(hx("00000000"), 0, 4, 0x80000000L, Endian.BIG)));
    assertEquals("00ffff", hex(Mutate.boundaryInsert(hx("000000"), 1, 2, 65535, Endian.BIG)));
    assertEquals("ffff", hex(Mutate.boundaryInsert(hx("0000"), 0, 2, -1L, Endian.LITTLE)));
    assertEquals("7fffffffffffffff", hex(Mutate.boundaryInsert(new byte[8], 0, 8, Long.MAX_VALUE, Endian.BIG)));
    assertEquals("0000000000000080", hex(Mutate.boundaryInsert(new byte[8], 0, 8, Long.MIN_VALUE, Endian.LITTLE)));
    assertEquals("ff", hex(Mutate.boundaryInsertBig(hx("00"), BigInteger.ZERO, 1, bg("-1"), Endian.LITTLE)));
    assertEquals("00", hex(Mutate.boundaryInsertBig(hx("ab"), BigInteger.ZERO, 1, bg("256"), Endian.LITTLE)));
    assertEquals("ab", hex(Mutate.boundaryInsert(hx("ab"), 0, 3, 1, Endian.LITTLE)));
    assertEquals("ab", hex(Mutate.boundaryInsert(hx("ab"), 0, 2, 1, Endian.LITTLE)));
    assertEquals("ab", hex(Mutate.boundaryInsertBig(hx("ab"), BigInteger.ZERO, 0, BigInteger.ONE, Endian.LITTLE)));
    assertEquals("00ab", hex(Mutate.boundaryInsert(hx("0000"), -1, 1, 0xab, Endian.LITTLE))); // -1 wraps to position 1
  }

  // ---- blocks -----------------------------------------------------------------------

  @Test
  void blockOperationsClampAndWrap() {
    assertEquals("01020506", hex(Mutate.blockDelete(hx("010203040506"), 2, 2)));
    assertEquals("0102", hex(Mutate.blockDelete(hx("01020304"), 2, 100)));
    assertEquals("0304", hex(Mutate.blockDelete(hx("01020304"), -4, 2)));
    assertEquals("01020304", hex(Mutate.blockDelete(hx("01020304"), 1, 0)));
    assertEquals("", hex(Mutate.blockDelete(new byte[0], 0, 1)));
    assertEquals("", hex(Mutate.blockDeleteBig(hx("0102"), BigInteger.ZERO, bg("100000000000000000000000000"))));
    assertEquals("01020304", hex(Mutate.blockDeleteBig(hx("01020304"), BigInteger.ZERO, bg("-1"))));
    assertEquals("0102ff03", hex(Mutate.blockInsert(hx("010203"), 2, hx("ff"))));
    assertEquals("010203ff", hex(Mutate.blockInsert(hx("010203"), 3, hx("ff"))));
    assertEquals("ff010203", hex(Mutate.blockInsert(hx("010203"), 4, hx("ff"))));
    assertEquals("ff", hex(Mutate.blockInsert(new byte[0], 9, hx("ff"))));
    assertEquals("ff", hex(Mutate.blockInsertBig(new byte[0], bg("9"), hx("ff"))));
    assertEquals("01ff0203", hex(Mutate.blockInsertBig(hx("010203"), bg("1000000000000000000000000001"), hx("ff"))));
    assertEquals("01010203", hex(Mutate.blockDuplicate(hx("010203"), 0, 1, 1)));
    assertEquals("0102030203", hex(Mutate.blockDuplicate(hx("010203"), 1, 100, 3)));
    assertEquals("010203", hex(Mutate.blockDuplicate(hx("010203"), 1, 0, 3)));
    assertEquals("", hex(Mutate.blockDuplicate(new byte[0], 1, 1, 1)));
    assertEquals("01010203", hex(Mutate.blockDuplicateBig(hx("010203"), bg("0"), bg("1"), bg("1"))));
    assertEquals("010203", hex(Mutate.blockDuplicateBig(hx("010203"), bg("0"), bg("-1"), bg("1"))));
    assertEquals("010203010203", hex(Mutate.blockDuplicateBig(hx("010203"), bg("0"), bg("100000000000000000000000000"), bg("3"))));
  }

  @Test
  void noMutatorModifiesItsArgumentOrReturnsItsAlias() {
    byte[] d = hx("0102030405060708");
    byte[] keep = d.clone();
    byte[] ins = hx("aa");
    byte[][] results = {
      Mutate.bitFlip(d, 3, 5), Mutate.bitFlip(d, 3, 0), Mutate.byteFlip(d, 1, 2), Mutate.byteFlip(d, 1, 0),
      Mutate.byteArithmetic(d, 1, 5, 2, Endian.BIG), Mutate.byteArithmetic(d, 1, 5, 3, Endian.BIG),
      Mutate.boundaryInsert(d, 1, 2, 9, Endian.LITTLE), Mutate.boundaryInsert(d, 1, 3, 9, Endian.LITTLE),
      Mutate.blockDelete(d, 1, 2), Mutate.blockDelete(d, 1, 0), Mutate.blockInsert(d, 1, ins),
      Mutate.blockDuplicate(d, 1, 2, 3), Mutate.blockDuplicate(d, 1, 0, 3), Mutate.splice(d, ins, 1, 0),
      Mutate.mutateSequence(d, 7, 0, 100), Mutate.mutateSequence(d, 7, -3, 100),
    };
    assertArrayEquals(keep, d);
    for (byte[] r : results) {
      assertNotSame(d, r);
    }
    assertArrayEquals(keep, Mutate.mutateSequence(d, 7, 0, 100));
    assertArrayEquals(keep, Mutate.bitFlip(d, 3, 0));
    assertNotSame(ins, Mutate.blockInsert(new byte[0], 0, ins));
  }

  // ---- mutateSequence mechanics -------------------------------------------------------

  @Test
  void mutateSequenceIsDeterministicAndRefillsAnEmptyBuffer() {
    byte[] a = Mutate.mutateSequence(hx("0102030405"), 99, 30, 1 << 20);
    assertArrayEquals(a, Mutate.mutateSequence(hx("0102030405"), 99, 30, 1 << 20));
    assertTrue(Mutate.mutateSequence(new byte[0], 5, 1, 64).length >= 1, "an empty buffer is refilled to 00 first");
    assertEquals(0, Mutate.mutateSequence(new byte[0], 5, 0, 64).length);
  }

  @Test
  void growthPastMaxLenIsSkipped() {
    for (long seed = 0; seed < 200; seed++) {
      byte[] out = Mutate.mutateSequence(hx("01020304"), seed, 60, 6);
      assertTrue(out.length <= 6, "length " + out.length + " for seed " + seed);
    }
  }

  @Test
  void mutateSequenceIsCapped() {
    assertThrows(IllegalArgumentException.class, () -> Mutate.mutateSequence(new byte[1], 1, Mutate.MAX_ITERATIONS + 1, 10));
    assertThrows(IllegalArgumentException.class, () -> Mutate.mutateSequence(new byte[Mutate.MAX_INPUT_BYTES + 1], 1, 1, 10));
    // The work cap counts every byte a step copies: a tiny cap binds on the first steps.
    assertThrows(IllegalArgumentException.class, () -> Mutate.mutateSequence(new byte[100], new SplitMix64(1), 50, 1000, 250));
    // The initial copy is charged too: 100 bytes cost 100 before any step runs.
    assertEquals(100, Mutate.mutateSequence(new byte[100], new SplitMix64(1), 0, 1000, 100).length);
    assertThrows(Mutate.WorkExhausted.class, () -> Mutate.mutateSequence(new byte[100], new SplitMix64(1), 0, 1000, 99));
    assertTrue(Mutate.mutateSequence(new byte[100], new SplitMix64(1), 2, 1000, 300).length > 0);
    assertThrows(IllegalArgumentException.class, () -> Mutate.mutateSequence(new byte[100], new SplitMix64(1), 2, 1000, 299));
    // The refusal reports the work done before it, so an aggregate can charge a failed call.
    Mutate.WorkExhausted e = assertThrows(Mutate.WorkExhausted.class, () -> Mutate.mutateSequence(new byte[100], new SplitMix64(1), 50, 1000, 250));
    assertEquals(200, e.work);
    assertEquals(Mutate.MAX_INPUT_BYTES, Mutate.mutateSequence(new byte[Mutate.MAX_INPUT_BYTES], 1, 0, 10).length);
    assertTrue(Mutate.mutateSequence(new byte[Mutate.MAX_INPUT_BYTES], 1, 4, 10).length > 0);
  }

  @Test
  void bigIntegerReductionsSaturateInsteadOfOverflowing() {
    assertEquals(Long.MAX_VALUE, Mutate.satLong(bg("99999999999999999999999")));
    assertEquals(Long.MIN_VALUE, Mutate.satLong(bg("-99999999999999999999999")));
    assertEquals(-5, Mutate.satLong(bg("-5")));
    assertEquals(-1L, Mutate.low64(bg("-1")));
    assertEquals(0L, Mutate.low64(bg("18446744073709551616")));
    assertEquals(5, Mutate.spliceLength(4, 4, 3, 2));
  }
}
