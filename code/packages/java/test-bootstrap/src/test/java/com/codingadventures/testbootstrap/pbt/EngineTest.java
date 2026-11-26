package com.codingadventures.testbootstrap.pbt;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.codingadventures.testbootstrap.prng.SplitMix64;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.List;
import java.util.function.Predicate;
import org.junit.jupiter.api.Test;

/** Generator algebra and shrinker behaviour (VER05e sections 3 and 4). */
class EngineTest {
  private static BigInteger big(long v) {
    return BigInteger.valueOf(v);
  }

  private static BigInteger twoPow(int n) {
    return BigInteger.ONE.shiftLeft(n);
  }

  @Test
  void integerStaysInRangeAcrossSeeds() {
    Gen<BigInteger> g = Gens.integer(big(-5), big(5));
    for (long seed = 0; seed < 500; seed++) {
      BigInteger v = g.sample(new SplitMix64(seed));
      assertTrue(v.compareTo(big(-5)) >= 0 && v.compareTo(big(5)) <= 0);
    }
    Gen<BigInteger> wide = Gens.integer(twoPow(200).negate(), twoPow(200));
    for (long seed = 0; seed < 200; seed++) {
      BigInteger v = wide.sample(new SplitMix64(seed));
      assertTrue(v.abs().compareTo(twoPow(200)) <= 0);
    }
  }

  @Test
  void integerRejectsBadBounds() {
    assertThrows(IllegalArgumentException.class, () -> Gens.integer(big(2), big(1)));
    assertThrows(IllegalArgumentException.class, () -> Gens.integer(null, big(1)));
    assertThrows(IllegalArgumentException.class, () -> Gens.int64(2, 1));
  }

  @Test
  void int64MatchesIntegerDrawsIncludingTheFullRange() {
    for (long[] range : new long[][] {{-100, 100}, {Long.MIN_VALUE, Long.MAX_VALUE}, {0, 9}, {Long.MAX_VALUE - 3, Long.MAX_VALUE}}) {
      Gen<Long> small = Gens.int64(range[0], range[1]);
      Gen<BigInteger> viaBig = Gens.integer(big(range[0]), big(range[1]));
      for (long seed = 0; seed < 100; seed++) {
        assertEquals(big(small.sample(new SplitMix64(seed))), viaBig.sample(new SplitMix64(seed)));
      }
    }
  }

  @Test
  void i128CoversBothSignsAndStaysInRange() {
    Gen<BigInteger> g = Gens.i128();
    boolean neg = false;
    boolean pos = false;
    for (long seed = 0; seed < 300; seed++) {
      BigInteger v = g.sample(new SplitMix64(seed));
      assertTrue(v.compareTo(twoPow(127).negate()) >= 0 && v.compareTo(twoPow(127)) < 0);
      neg |= v.signum() < 0;
      pos |= v.signum() > 0;
    }
    assertTrue(neg && pos);
  }

  @Test
  void primitivesAndTheirShrinkers() {
    // u8: smallest failing value of x < 10 is 10.
    Shrunk<Integer> u8 = Gens.u8().shrink(200, x -> x >= 10, 1000);
    assertEquals(10, u8.value());
    assertEquals(0, Gens.u8().shrink(200, x -> true, 1000).value(), "the target probe wins when 0 falsifies");
    assertEquals(0, Gens.u8().sample(new SplitMix64(0)) - 175, "seed 0 first u8 is 175 (corpus)");

    Shrunk<BigInteger> u64 = Gens.u64().shrink(twoPow(63), x -> x.compareTo(big(1000)) >= 0, 1000);
    assertEquals(big(1000), u64.value());

    Shrunk<BigInteger> i128 = Gens.i128().shrink(twoPow(100).negate(), x -> x.compareTo(big(-7)) <= 0, 1000);
    assertEquals(big(-7), i128.value());

    Shrunk<Boolean> bool = Gens.bool().shrink(true, b -> true, 5);
    assertEquals(new Shrunk<>(false, 1), bool);
    assertEquals(new Shrunk<>(true, 0), Gens.bool().shrink(true, b -> b, 5), "false does not falsify: stay");
    assertEquals(new Shrunk<>(false, 0), Gens.bool().shrink(false, b -> true, 5));
    assertEquals(new Shrunk<>(true, 0), Gens.bool().shrink(true, b -> true, 0), "no budget, no shrinking");

    Shrunk<Long> i64 = Gens.int64(-50, 50).shrink(-40L, x -> x <= -3, 1000);
    assertEquals(-3L, i64.value());
  }

  @Test
  void clampZeroTargets() {
    assertEquals(big(5), Gens.clampZero(big(5), big(10)));
    assertEquals(big(-2), Gens.clampZero(big(-10), big(-2)));
    assertEquals(BigInteger.ZERO, Gens.clampZero(big(-10), big(10)));
    // integer(5, 10) shrinks toward 5, never 0.
    assertEquals(big(5), Gens.integer(big(5), big(10)).shrink(big(10), x -> true, 100).value());
    assertEquals(big(-2), Gens.integer(big(-10), big(-2)).shrink(big(-10), x -> true, 100).value());
  }

  @Test
  void bytesAndListLengthsAndStreamPosition() {
    Gen<byte[]> g = Gens.bytes(2, 6);
    for (long seed = 0; seed < 200; seed++) {
      int n = g.sample(new SplitMix64(seed)).length;
      assertTrue(n >= 2 && n <= 6);
    }
    // A fixed-length byte string still spends one word on its length: 1 + 4 words.
    SplitMix64 r = new SplitMix64(1);
    Gens.bytes(4, 4).sample(r);
    SplitMix64 ref = new SplitMix64(1);
    for (int i = 0; i < 5; i++) {
      ref.nextU64();
    }
    assertEquals(ref.state(), r.state());

    Gen<List<Integer>> l = Gens.list(Gens.u8(), 0, 3);
    for (long seed = 0; seed < 100; seed++) {
      assertTrue(l.sample(new SplitMix64(seed)).size() <= 3);
    }
    assertThrows(IllegalArgumentException.class, () -> Gens.bytes(3, 2));
    assertThrows(IllegalArgumentException.class, () -> Gens.bytes(-1, 2));
    assertThrows(IllegalArgumentException.class, () -> Gens.bytes(0, Limits.MAX_INPUT_BYTES + 1));
    assertThrows(IllegalArgumentException.class, () -> Gens.list(Gens.u8(), 0, Limits.MAX_LIST_LEN + 1));
  }

  @Test
  void listRespectsMinLenWhileShrinking() {
    Gen<List<Integer>> g = Gens.list(Gens.u8(), 3, 8);
    List<Integer> start = List.of(9, 9, 9, 9, 9, 9);
    // Property: sum < 10 holds only for tiny lists; everything of length >= 3 with a 9 fails.
    Shrunk<List<Integer>> r = g.shrink(start, l -> l.stream().mapToInt(i -> i).sum() >= 1, 1000);
    assertEquals(3, r.value().size());
    assertTrue(r.value().stream().mapToInt(i -> i).sum() >= 1);

    Gen<byte[]> b = Gens.bytes(2, 8);
    Shrunk<byte[]> br = b.shrink(new byte[] {1, 2, 3, 4, 5}, x -> true, 1000);
    assertEquals(2, br.value().length);
    assertArrayEquals(new byte[] {0, 0}, br.value());
  }

  @Test
  void filterKeepsShrinkingValid() {
    Gen<BigInteger> even = Gens.filter(Gens.integer(big(0), big(1000)), x -> !x.testBit(0));
    for (long seed = 0; seed < 100; seed++) {
      assertFalse(even.sample(new SplitMix64(seed)).testBit(0));
    }
    // Shrunk candidates that are odd are not valid inputs, so never reported.
    Shrunk<BigInteger> r = even.shrink(big(900), x -> x.compareTo(big(100)) >= 0, 1000);
    assertFalse(r.value().testBit(0));
    assertTrue(r.value().compareTo(big(100)) >= 0);

    Gen<BigInteger> never = Gens.filter(Gens.integer(big(0), big(1)), x -> false, 7);
    ExhaustedException e = assertThrows(ExhaustedException.class, () -> never.sample(new SplitMix64(1)));
    assertTrue(e.getMessage().contains("7"));
    assertThrows(IllegalArgumentException.class, () -> Gens.filter(Gens.bool(), x -> true, 0));
  }

  @Test
  void mapFlatMapConstantAndGenWithoutSampler() {
    Gen<Integer> plusOne = Gens.map(Gens.u8(), x -> x + 1);
    assertEquals(176, plusOne.sample(new SplitMix64(0)));
    assertEquals(new Shrunk<>(176, 0), plusOne.shrink(176, x -> true, 10), "map drops shrinking");

    SplitMix64 r = new SplitMix64(3);
    SplitMix64 untouched = new SplitMix64(3);
    assertEquals("k", Gens.constant("k").sample(r));
    assertEquals(untouched.state(), r.state(), "constant draws nothing");

    Gen<List<Integer>> dependent = Gens.flatMap(Gens.integer(big(1), big(4)), n -> Gens.list(Gens.u8(), n.intValue(), n.intValue()));
    for (long seed = 0; seed < 50; seed++) {
      int size = dependent.sample(new SplitMix64(seed)).size();
      assertTrue(size >= 1 && size <= 4);
    }
    assertThrows(IllegalArgumentException.class, () -> Gen.of(null, null));
  }

  @Test
  void oneOfAndFrequency() {
    Gen<Integer> g = Gens.oneOf(Gens.constant(1), Gens.constant(2));
    boolean one = false;
    boolean two = false;
    for (long seed = 0; seed < 100; seed++) {
      int v = g.sample(new SplitMix64(seed));
      one |= v == 1;
      two |= v == 2;
    }
    assertTrue(one && two);
    assertThrows(IllegalArgumentException.class, Gens::<Integer>oneOf);

    Gen<String> f = Gens.frequency(new Gens.Arm<>(0, Gens.constant("never")), new Gens.Arm<>(1, Gens.constant("a")), new Gens.Arm<>(3, Gens.constant("b")));
    int a = 0;
    int b = 0;
    SplitMix64 r = new SplitMix64(77);
    for (int i = 0; i < 4000; i++) {
      String v = f.sample(r);
      assertFalse(v.equals("never"));
      if (v.equals("a")) {
        a++;
      } else {
        b++;
      }
    }
    assertTrue(b > 2 * a && b < 4 * a, "a=" + a + " b=" + b);

    assertThrows(IllegalArgumentException.class, Gens::<Integer>frequency);
    assertThrows(IllegalArgumentException.class, () -> Gens.frequency(new Gens.Arm<>(0, Gens.constant(1))));
    assertThrows(IllegalArgumentException.class, () -> Gens.frequency(new Gens.Arm<>(-1, Gens.constant(1))));
    assertThrows(IllegalArgumentException.class, () -> Gens.frequency(new Gens.Arm<>((1L << 32) + 1, Gens.constant(1))));
    assertThrows(
        IllegalArgumentException.class,
        () -> Gens.frequency(new Gens.Arm<>(1L << 32, Gens.constant(1)), new Gens.Arm<>(1, Gens.constant(1))));
    // total exactly 2^32 is allowed.
    assertEquals(1, Gens.frequency(new Gens.Arm<>(1L << 32, Gens.constant(1))).sample(new SplitMix64(0)));
  }

  @Test
  void tuples() {
    Gen<Gens.Pair<BigInteger, BigInteger>> pair = Gens.tuple2(Gens.integer(big(0), big(1000)), Gens.integer(big(0), big(1000)));
    Predicate<Gens.Pair<BigInteger, BigInteger>> falsifies = p -> p.first().add(p.second()).compareTo(big(50)) >= 0;
    Shrunk<Gens.Pair<BigInteger, BigInteger>> r = pair.shrink(new Gens.Pair<>(big(700), big(900)), falsifies, 1000);
    assertTrue(falsifies.test(r.value()));
    assertEquals(big(50), r.value().first().add(r.value().second()), "1-minimal: the sum lands exactly on the threshold");
    assertTrue(r.steps() > 0);
    assertEquals(0, pair.shrink(new Gens.Pair<>(big(5), big(5)), x -> true, 0).steps());

    Gen<Gens.Triple<Boolean, BigInteger, List<Integer>>> triple =
        Gens.tuple3(Gens.bool(), Gens.integer(big(0), big(100)), Gens.list(Gens.u8(), 0, 4));
    Shrunk<Gens.Triple<Boolean, BigInteger, List<Integer>>> t =
        triple.shrink(new Gens.Triple<>(true, big(80), List.of(7, 7, 7)), x -> true, 1000);
    assertEquals(new Gens.Triple<>(false, BigInteger.ZERO, List.<Integer>of()), t.value());
    Gens.Triple<Boolean, BigInteger, List<Integer>> sampled = triple.sample(new SplitMix64(4));
    assertTrue(sampled.second().signum() >= 0);
    Shrunk<Gens.Triple<Boolean, BigInteger, List<Integer>>> partial =
        triple.shrink(new Gens.Triple<>(true, big(80), List.of(7, 7, 7)), x -> x.second().compareTo(big(10)) >= 0, 1000);
    assertEquals(big(10), partial.value().second());
    assertEquals(0, triple.shrink(sampled, x -> true, 0).steps());
  }

  @Test
  void corpusEntryPointErrors() {
    assertThrows(IllegalArgumentException.class, () -> Gens.genPrimitive(0, "bool", -1));
    assertThrows(IllegalArgumentException.class, () -> Gens.genPrimitive(0, "float", 1));
    assertThrows(IllegalArgumentException.class, () -> Gens.genBounded(0, big(2), big(1), 1));
    assertThrows(IllegalArgumentException.class, () -> Gens.genBounded(0, big(1), big(2), -1));
    assertThrows(IllegalArgumentException.class, () -> Gens.genBytes(0, 3, 2, 1));
    assertThrows(IllegalArgumentException.class, () -> Gens.genBytes(0, 0, 1000, Limits.MAX_GEN_BYTES_TOTAL));
    assertEquals(0, Gens.genPrimitive(0, "u8", 0).size());
    assertEquals(3, Gens.genBytes(1, 0, 0, 3).size());
  }

  @Test
  void monotoneThresholdsLandExactlyOnT() {
    // VER05 section 7.3: for "fails iff x >= T" the shrunk witness is exactly T.
    for (long t = 1; t <= 120; t++) {
      final long threshold = t;
      for (long start = t; start <= 400; start += 7) {
        Shrunk<BigInteger> r =
            Shrink.shrinkInt(big(start), BigInteger.ZERO, big(-1000), big(1000), x -> x.compareTo(big(threshold)) < 0, 1000);
        assertEquals(big(t), r.value(), "t=" + t + " start=" + start);
      }
      for (long start = -t; start >= -400; start -= 7) {
        Shrunk<BigInteger> r =
            Shrink.shrinkInt(big(start), BigInteger.ZERO, big(-1000), big(1000), x -> x.compareTo(big(-threshold)) > 0, 1000);
        assertEquals(big(-t), r.value(), "t=-" + t + " start=" + start);
      }
    }
  }

  @Test
  void shrinkIntExtremesDoNotOverflow() {
    BigInteger min = twoPow(127).negate();
    BigInteger max = twoPow(127).subtract(BigInteger.ONE);
    // Everything fails: an opposite-extreme target wins the probe in one step.
    assertEquals(new Shrunk<>(max, 1), Shrink.shrinkInt(min, max, min, max, x -> false, 1000));
    assertEquals(new Shrunk<>(min, 1), Shrink.shrinkInt(max, min, min, max, x -> false, 1000));
    // Extreme start, in-range target: the walk crosses 2^127 without overflow.
    assertEquals(big(-100), Shrink.shrinkInt(min, BigInteger.ZERO, min, max, x -> x.compareTo(big(-100)) > 0, 1000).value());
    assertEquals(big(100), Shrink.shrinkInt(max, BigInteger.ZERO, min, max, x -> x.compareTo(big(100)) < 0, 1000).value());
    // Walking away from an out-of-range target never leaves [min, max].
    Shrunk<BigInteger> r = Shrink.shrinkInt(min, max, min, max, x -> x.compareTo(big(5)) > 0, 1000);
    assertTrue(r.value().compareTo(min) >= 0 && r.value().compareTo(max) <= 0);
  }

  @Test
  void shrinkIntBudgetAndErrors() {
    Predicate<BigInteger> le100 = x -> x.compareTo(big(100)) <= 0;
    Shrunk<BigInteger> none = Shrink.shrinkInt(big(54321), BigInteger.ZERO, BigInteger.ZERO, big(100000), le100, 0);
    assertEquals(new Shrunk<>(big(54321), 0), none);
    Shrunk<BigInteger> two = Shrink.shrinkInt(big(54321), BigInteger.ZERO, BigInteger.ZERO, big(100000), le100, 2);
    assertEquals(2, two.steps());
    Shrunk<BigInteger> probe = Shrink.shrinkInt(big(500), BigInteger.ZERO, BigInteger.ZERO, big(1000), x -> false, 10);
    assertEquals(new Shrunk<>(BigInteger.ZERO, 1), probe, "the target probe is one accepted move");

    assertThrows(NotFalsifyingException.class, () -> Shrink.shrinkInt(big(5), BigInteger.ZERO, BigInteger.ZERO, big(10), le100, 10));
    assertThrows(IllegalArgumentException.class, () -> Shrink.shrinkInt(big(500), BigInteger.ZERO, BigInteger.ZERO, big(10), le100, 10));
    assertThrows(IllegalArgumentException.class, () -> Shrink.shrinkInt(big(500), BigInteger.ZERO, big(10), big(0), le100, 10));
    assertThrows(IllegalArgumentException.class, () -> Shrink.shrinkInt(null, BigInteger.ZERO, BigInteger.ZERO, big(10), le100, 10));
    assertThrows(IllegalArgumentException.class, () -> Shrink.shrinkInt(big(500), BigInteger.ZERO, BigInteger.ZERO, big(1000), le100, -1));
    assertThrows(IllegalArgumentException.class, () -> Shrink.shrinkInt(big(500), BigInteger.ZERO, BigInteger.ZERO, big(1000), le100, Limits.MAX_SHRINK_STEPS + 1));
  }

  @Test
  void shrinkersDoNotAliasTheCaller() {
    List<BigInteger> start = new ArrayList<>(List.of(big(10), big(5), big(8), big(4)));
    Shrunk<List<BigInteger>> r =
        Shrink.shrinkList(
            start,
            BigInteger.ZERO,
            l -> {
              BigInteger sum = BigInteger.ZERO;
              for (BigInteger v : l) {
                sum = sum.add(v);
              }
              boolean ok = sum.compareTo(big(20)) <= 0;
              l.clear(); // a property that scribbles on its argument
              return ok;
            },
            1000);
    assertEquals(List.of(big(8), big(5), big(8)), r.value());
    assertEquals(4, start.size(), "the caller's list is untouched");

    byte[] bytes = {1, 2, 3, 4, 5, 6, 7, 8};
    Shrunk<byte[]> b =
        Shrink.shrinkBytes(
            bytes,
            x -> {
              boolean ok = x.length < 5;
              java.util.Arrays.fill(x, (byte) 9);
              return ok;
            },
            1000);
    assertEquals(5, b.value().length);
    assertArrayEquals(new byte[] {1, 2, 3, 4, 5, 6, 7, 8}, bytes);
  }

  @Test
  void listAndBytesShrinkErrors() {
    assertThrows(NotFalsifyingException.class, () -> Shrink.shrinkList(List.of(big(1)), BigInteger.ZERO, l -> true, 10));
    assertThrows(NotFalsifyingException.class, () -> Shrink.shrinkBytes(new byte[] {1}, b -> true, 10));
    assertThrows(IllegalArgumentException.class, () -> Shrink.shrinkList(null, BigInteger.ZERO, l -> false, 10));
    assertThrows(IllegalArgumentException.class, () -> Shrink.shrinkList(List.of(big(1)), null, l -> false, 10));
    assertThrows(IllegalArgumentException.class, () -> Shrink.shrinkList(java.util.Arrays.asList(big(1), null), BigInteger.ZERO, l -> false, 10));
    assertThrows(IllegalArgumentException.class, () -> Shrink.shrinkList(List.of(big(1)), BigInteger.ZERO, l -> false, -1));
    assertThrows(IllegalArgumentException.class, () -> Shrink.shrinkBytes(null, b -> false, 10));
    assertThrows(IllegalArgumentException.class, () -> Shrink.shrinkBytes(new byte[1], b -> false, -1));
    Shrunk<List<BigInteger>> zero = Shrink.shrinkList(List.of(big(1), big(2)), BigInteger.ZERO, l -> false, 0);
    assertEquals(new Shrunk<>(List.of(big(1), big(2)), 0), zero);
  }

  /** A sequence shrink result is 1-minimal: deleting any single element makes the property hold. */
  @Test
  void sequenceShrinkingIsOneMinimalByDeletion() {
    Predicate<List<BigInteger>> sumLe = l -> l.stream().reduce(BigInteger.ZERO, BigInteger::add).compareTo(big(30)) <= 0;
    for (long seed = 0; seed < 60; seed++) {
      List<BigInteger> start = Gens.list(Gens.integer(big(0), big(40)), 4, 12).sample(new SplitMix64(seed));
      if (sumLe.test(start)) {
        continue;
      }
      List<BigInteger> m = Shrink.shrinkList(start, BigInteger.ZERO, sumLe, 1000).value();
      assertFalse(sumLe.test(m));
      for (int i = 0; i < m.size(); i++) {
        List<BigInteger> cut = new ArrayList<>(m);
        cut.remove(i);
        assertTrue(sumLe.test(cut), "deleting index " + i + " of " + m + " must make the property hold");
      }
    }
  }

  @Test
  void simulateReplay() {
    Shrink.Replay r = Shrink.simulateReplay(42, x -> x.compareTo(big(100)) <= 0);
    assertEquals(big(413), r.initialFail());
    assertEquals(big(101), r.minimal());
    assertEquals(r, Shrink.simulateReplay(42, x -> x.compareTo(big(100)) <= 0), "replay is deterministic");
    assertThrows(IllegalArgumentException.class, () -> Shrink.simulateReplay(1, x -> true));
  }

  @Test
  void sequenceShrinkMinLenPrefixProbe() {
    // minLen 2: the 2-element prefix falsifies, so it is accepted as one step.
    Shrunk<List<Integer>> r = Gens.list(Gens.u8(), 2, 6).shrink(List.of(5, 6, 7, 8, 9), l -> l.size() >= 2, 100);
    assertEquals(2, r.value().size());
    assertEquals(List.of(0, 0), r.value());
  }

  /**
   * The corpus never accepts a half in bisection (its properties fail on length
   * or sum), so a shrinker that forgot the left or the right half would still
   * pass it. These pin both halves with exact step counts: three bisections take
   * 8 elements to 1, where per-element deletion would take seven steps.
   */
  @Test
  void bisectionAcceptsTheLeftHalfThenTheRightHalf() {
    List<BigInteger> ones = new ArrayList<>(java.util.Collections.nCopies(7, BigInteger.ONE));

    List<BigInteger> leftHeavy = new ArrayList<>();
    leftHeavy.add(big(9));
    leftHeavy.addAll(ones);
    Shrunk<List<BigInteger>> left = Shrink.shrinkList(leftHeavy, BigInteger.ZERO, l -> l.isEmpty() || !l.get(0).equals(big(9)), 1000);
    assertEquals(List.of(big(9)), left.value());
    assertEquals(3, left.steps());

    List<BigInteger> rightHeavy = new ArrayList<>(ones);
    rightHeavy.add(big(9));
    Shrunk<List<BigInteger>> right = Shrink.shrinkList(rightHeavy, BigInteger.ZERO, l -> l.isEmpty() || !l.get(l.size() - 1).equals(big(9)), 1000);
    assertEquals(List.of(big(9)), right.value());
    assertEquals(3, right.steps());

    // The same through the generator-attached shrinker, with the byte walk.
    Shrunk<byte[]> bytes = Gens.bytes(0, 8).shrink(new byte[] {9, 1, 1, 1, 1, 1, 1, 1}, b -> b.length > 0 && b[0] == 9, 1000);
    assertArrayEquals(new byte[] {9}, bytes.value());
    assertEquals(3, bytes.steps());
  }

  // Security regression (review item 1): count was unbounded up to
  // Integer.MAX_VALUE, and genBytes skipped its budget entirely when maxLen == 0.
  @Test
  void generationCountIsCappedEvenForEmptyStrings() {
    assertThrows(IllegalArgumentException.class, () -> Gens.genBytes(0, 0, 0, Integer.MAX_VALUE));
    assertThrows(IllegalArgumentException.class, () -> Gens.genBytes(0, 0, 0, Limits.MAX_GEN_COUNT + 1));
    assertThrows(IllegalArgumentException.class, () -> Gens.genPrimitive(0, "bool", Integer.MAX_VALUE));
    assertThrows(IllegalArgumentException.class, () -> Gens.genPrimitive(0, "u8", Limits.MAX_GEN_COUNT + 1));
    assertThrows(IllegalArgumentException.class, () -> Gens.genBounded(0, big(0), big(1), Integer.MAX_VALUE));
    assertThrows(IllegalArgumentException.class, () -> Gens.genBounded(0, big(0), big(1), Limits.MAX_GEN_COUNT + 1));
    assertEquals(Limits.MAX_GEN_COUNT, Gens.genBytes(0, 0, 0, Limits.MAX_GEN_COUNT).size());
    assertEquals(Limits.MAX_GEN_COUNT, Gens.genPrimitive(0, "bool", Limits.MAX_GEN_COUNT).size());
  }

  // Security regression (review item 2): rejected attempts copy the list but
  // are not accepted steps, so they need their own budget.
  @Test
  void sequenceShrinkingChargesRejectedAttemptsAgainstABudget() {
    final int n = 20_000;
    List<BigInteger> start = new ArrayList<>(java.util.Collections.nCopies(n, BigInteger.ONE));
    java.util.concurrent.atomic.AtomicInteger calls = new java.util.concurrent.atomic.AtomicInteger();
    // Holds for every list except the full-length one: every smaller candidate is rejected.
    Shrunk<List<BigInteger>> r =
        Shrink.shrinkList(start, BigInteger.ZERO, l -> { calls.incrementAndGet(); return l.size() != n; }, 10);
    assertEquals(n, r.value().size());
    assertEquals(0, r.steps());
    long budget = (long) Limits.ATTEMPT_FACTOR * 10 + Limits.ATTEMPT_SLACK;
    assertTrue(calls.get() <= budget + 2, "evaluations " + calls.get() + " exceed budget " + budget);
    // The same through the generator-attached bytes shrinker.
    java.util.concurrent.atomic.AtomicInteger byteCalls = new java.util.concurrent.atomic.AtomicInteger();
    Shrunk<byte[]> b = Gens.bytes(0, 50_000).shrink(new byte[50_000], x -> { byteCalls.incrementAndGet(); return x.length == 50_000; }, 5);
    assertEquals(50_000, b.value().length);
    assertTrue(byteCalls.get() <= Limits.ATTEMPT_FACTOR * 5L + Limits.ATTEMPT_SLACK);
  }

  // Security regression (re-review 1): every pass of a tuple's repeat-until-no-progress
  // loop gave each component a fresh budget, ~8 * maxSteps^2 evaluations in total.
  @Test
  void tupleShrinkingSharesOneGlobalBudget() {
    // Component B makes exactly one accepted step per call, so the loop repeats
    // once per step; component A rejects every candidate but costs a full local
    // budget each pass.
    Gen<BigInteger> oneStep =
        Gen.of(
            rng -> big(1_000_000),
            (value, falsifies, max) ->
                value.signum() > 0 && max > 0 && falsifies.test(value.subtract(BigInteger.ONE))
                    ? new Shrunk<>(value.subtract(BigInteger.ONE), 1)
                    : new Shrunk<>(value, 0));
    final int n = 5_000;
    Gen<List<Integer>> list = Gens.list(Gens.u8(), 0, n);
    List<Integer> start = new ArrayList<>(java.util.Collections.nCopies(n, 1));
    java.util.concurrent.atomic.AtomicInteger calls = new java.util.concurrent.atomic.AtomicInteger();
    final int maxSteps = 100;
    Shrunk<Gens.Pair<List<Integer>, BigInteger>> r =
        Gens.tuple2(list, oneStep)
            .shrink(
                new Gens.Pair<>(start, big(1_000_000)),
                p -> { calls.incrementAndGet(); return p.first().size() == n; },
                maxSteps);
    long budget = (long) Limits.ATTEMPT_FACTOR * maxSteps + Limits.ATTEMPT_SLACK;
    assertTrue(calls.get() <= budget, "evaluations " + calls.get() + " exceed the global budget " + budget);
    assertEquals(n, r.value().first().size());

    // The same for tuple3, and through the runner (one budget per shrink).
    calls.set(0);
    Gens.tuple3(list, oneStep, Gens.bool())
        .shrink(new Gens.Triple<>(start, big(1_000_000), true), t -> { calls.incrementAndGet(); return t.first().size() == n; }, maxSteps);
    assertTrue(calls.get() <= budget, "tuple3 evaluations " + calls.get());
  }

  // Re-review 2: phase 2 hands its fresh candidate straight to the property; a property
  // that scribbles on it must not reach the shrinker's state.
  @Test
  void deletionPhaseSurvivesAPropertyThatMutatesItsArgument() {
    Shrunk<List<BigInteger>> r =
        Shrink.shrinkList(
            new ArrayList<>(List.of(big(7), big(1), big(1), big(1), big(7))),
            BigInteger.ZERO,
            l -> {
              boolean holds = l.stream().filter(v -> v.equals(big(7))).count() < 2;
              l.replaceAll(v -> big(99));
              return holds;
            },
            1000);
    assertEquals(List.of(big(7), big(7)), r.value());
  }

  // Re-review 4: the public genBounded needs the same count x width budget Corpus had.
  @Test
  void genBoundedHasACountTimesWidthBudget() {
    BigInteger wide = BigInteger.TEN.pow(1000);
    assertThrows(IllegalArgumentException.class, () -> Gens.genBounded(0, BigInteger.ZERO, wide, 100_000));
    assertEquals(1000, Gens.genBounded(0, BigInteger.ZERO, wide, 1000).size());
    // The span, not the bounds, sets the width: huge but narrow bounds are cheap.
    assertEquals(1_000_000, Gens.genBounded(0, wide, wide.add(big(1)), 1_000_000).size() );
  }
}
