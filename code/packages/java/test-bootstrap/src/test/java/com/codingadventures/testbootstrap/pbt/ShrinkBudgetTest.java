package com.codingadventures.testbootstrap.pbt;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigInteger;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Predicate;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

/**
 * Regressions for the shrink budget (VER05e section 4.1), ported from the C#
 * security review of VER05g: work is budgeted, the budget is shared by nested
 * shrinkers, initial copies are charged, and re-entrant shrinks are isolated.
 */
class ShrinkBudgetTest {
  @AfterEach
  void reset() {
    Hooks.reset();
    assertNull(ShrinkBudget.current(), "the ambient budget must be restored");
  }

  private static BigInteger big(long v) {
    return BigInteger.valueOf(v);
  }

  // Attempts alone do not bound the work; elements copied are charged too.
  @Test
  void aMegabyteWitnessWithAStallingPropertyStaysInsideTheWorkBudget() {
    final int n = 1 << 20;
    AtomicLong touched = new AtomicLong();
    int[] calls = {0};
    Gen<byte[]> g = Gens.bytes(0, Limits.MAX_INPUT_BYTES);

    // Every smaller candidate "holds" (is rejected): the worst case for copy work.
    Shrunk<byte[]> r =
        g.shrink(
            new byte[n],
            b -> {
              calls[0]++;
              touched.addAndGet(b.length);
              return b.length == n;
            },
            1000);

    assertEquals(n, r.value().length);
    assertTrue(ShrinkBudget.lastWork <= Limits.MAX_SHRINK_WORK, "charged work " + ShrinkBudget.lastWork);
    assertTrue(touched.get() <= Limits.MAX_SHRINK_WORK, "elements handed to the property " + touched);
    // Without a work budget the attempt budget (16,064) would allow ~16 billion elements.
    assertTrue(calls[0] < 100, "attempts " + calls[0]);
  }

  @Test
  void theRunnerShrinkOfAMegabyteWitnessIsAlsoBounded() {
    final int n = 1 << 20;
    AtomicLong touched = new AtomicLong();
    Verdict v =
        Runner.check(
            1,
            Gens.constant(new byte[n]),
            b -> {
              touched.addAndGet(b.length);
              return b.length != n;
            },
            new Options(1, 1000, CiMode.OFF));
    assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
    assertTrue(touched.get() <= Limits.MAX_SHRINK_WORK + 3L * n, "elements touched " + touched);
  }

  // Accepted moves apply in place, from the move, never from a mutated candidate.
  @Test
  void aMutatingPropertyCannotCorruptBisectionOrProbe() {
    List<BigInteger> start = new ArrayList<>(List.of(big(0), big(7), big(0), big(0), big(0), big(0), big(0), big(0)));
    Shrunk<List<BigInteger>> r =
        Shrink.shrinkSequence(
            start,
            c -> {
              boolean fails = c.contains(big(7));
              c.clear();
              c.add(big(99));
              return fails;
            },
            1000,
            0,
            null);
    assertEquals(List.of(big(7)), r.value());
    assertEquals(3, r.steps());

    Shrunk<List<Integer>> p =
        Gens.list(Gens.u8(), 2, 8)
            .shrink(
                new ArrayList<>(List.of(9, 9, 9, 9, 9)),
                c -> {
                  boolean ok = c.size() >= 2;
                  c.clear();
                  return ok;
                },
                1000);
    assertEquals(List.of(0, 0), p.value());
  }

  private static List<List<Integer>> grid(int rows, int cols) {
    List<List<Integer>> out = new ArrayList<>();
    for (int i = 0; i < rows; i++) {
      out.add(new ArrayList<>(Collections.nCopies(cols, 7)));
    }
    return out;
  }

  private static boolean isGrid(List<List<Integer>> l, int rows, int cols) {
    if (l.size() != rows) {
      return false;
    }
    for (List<Integer> x : l) {
      if (x.size() != cols) {
        return false;
      }
      for (int b : x) {
        if (b != 7) {
          return false;
        }
      }
    }
    return true;
  }

  // One budget for the whole shrink, across nested shrinkers.
  @Test
  void nestedListsShareOneGlobalBudget() {
    Gen<List<List<Integer>>> g = Gens.list(Gens.list(Gens.u8(), 0, 20), 0, 50);
    List<List<Integer>> start = grid(50, 20);
    int[] calls = {0};
    final int maxSteps = 10;

    // Falsifies only for the exact original: every candidate at every level is rejected.
    Shrunk<List<List<Integer>>> r =
        g.shrink(
            start,
            l -> {
              calls[0]++;
              return isGrid(l, 50, 20);
            },
            maxSteps);
    assertEquals(0, r.steps());
    assertTrue(calls[0] <= Limits.attemptBudget(maxSteps), "nested evaluations " + calls[0]);
    assertTrue(ShrinkBudget.lastWork <= Limits.MAX_SHRINK_WORK);

    calls[0] = 0;
    Verdict v =
        Runner.check(
            1,
            Gens.constant(start),
            l -> {
              calls[0]++;
              return !isGrid(l, 50, 20);
            },
            new Options(1, maxSteps, CiMode.OFF));
    assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
    assertTrue(calls[0] <= 1 + Limits.attemptBudget(maxSteps) + 1, "runner nested evaluations " + calls[0]);
  }

  @Test
  void aNestedShrinkerStillShrinksWhenTheBudgetIsAmple() {
    Gen<List<List<Integer>>> g = Gens.list(Gens.list(Gens.u8(), 0, 5), 0, 5);
    List<List<Integer>> start = new ArrayList<>();
    start.add(new ArrayList<>(List.of(5, 6)));
    start.add(new ArrayList<>(List.of(7)));
    Shrunk<List<List<Integer>>> r = g.shrink(start, l -> l.size() >= 1, 1000);
    assertEquals(1, r.value().size());
    assertTrue(r.value().get(0).isEmpty());
  }

  @Test
  void aBudgetSpentByWorkEndsEveryLoop() {
    try (ShrinkBudget b = ShrinkBudget.enter(1000, false, 10)) {
      assertTrue(b.charge(6));
      assertFalse(b.charge(6)); // 12 > 10
      assertTrue(b.charge(4));
      assertFalse(b.hasRoom());
      assertEquals(2, b.attempts());
      assertEquals(10, b.work());
      try (ShrinkBudget nested = ShrinkBudget.enter(5)) {
        assertFalse(nested.charge(1)); // joins the same, spent, budget
        assertFalse(nested.hasRoom());
        assertEquals(10, nested.work());
      }
      assertSame(b, ShrinkBudget.current()); // a joining scope leaves the ambient one alone
    }
  }

  @Test
  void chargeCopyChargesWorkButNotAttempts() {
    try (ShrinkBudget b = ShrinkBudget.enter(1000, false, 10)) {
      assertTrue(b.chargeCopy(8));
      assertEquals(0, b.attempts());
      assertFalse(b.chargeCopy(3));
    }
  }

  // A property that runs its own Runner.check gets its own isolated budget.
  private static Verdict innerRun() {
    return Runner.check(
        5,
        Gens.list(Gens.u8(), 0, 50),
        l -> l.stream().mapToInt(Integer::intValue).sum() <= 20,
        new Options(200, 1000, CiMode.ON));
  }

  @Test
  void aReentrantRunnerCheckIsNotStarvedByTheOuterShrink() {
    Verdict standalone = innerRun();
    assertEquals(FailureIds.COUNTEREXAMPLE, standalone.failureId());
    assertTrue(standalone.shrinkSteps() > 3, "the inner shrink must need a real budget");

    List<Verdict> inner = new ArrayList<>();
    Predicate<BigInteger> plain = x -> x.compareTo(big(500)) < 0;
    Predicate<BigInteger> reentrant =
        x -> {
          inner.add(innerRun());
          return x.compareTo(big(500)) < 0;
        };
    Options outer = new Options(100, 1, CiMode.ON); // outer budget: 16 * 1 + 64 = 80 attempts
    Verdict expected = Runner.check(9, Gens.integer(big(0), big(1000)), plain, outer);
    Verdict actual = Runner.check(9, Gens.integer(big(0), big(1000)), reentrant, outer);

    assertFalse(inner.isEmpty());
    for (Verdict v : inner) {
      assertEquals(standalone, v); // same digest, steps and message
    }
    assertEquals(expected, actual); // the outer verdict is unaffected
  }

  @Test
  void aReentrantGenShrinkAndShrinkListGetFreshBudgetsAndRestoreTheOuterOne() {
    Gen<List<Integer>> g = Gens.list(Gens.u8(), 0, 40);
    List<Integer> start = new ArrayList<>(Collections.nCopies(40, 3));
    int[] innerCalls = {0};
    Gen<BigInteger> outerGen = Gens.integer(big(0), big(1000));
    Shrunk<BigInteger> r =
        outerGen.shrink(
            big(900),
            x -> {
              Shrunk<List<Integer>> a = g.shrink(new ArrayList<>(start), l -> l.size() >= 1, 1000);
              Shrunk<List<BigInteger>> b =
                  Shrink.shrinkList(new ArrayList<>(List.of(big(5), big(6), big(7))), BigInteger.ZERO, l -> l.isEmpty(), 1000);
              innerCalls[0]++;
              assertEquals(1, a.value().size());
              assertEquals(List.of(BigInteger.ZERO), b.value());
              return x.compareTo(big(10)) >= 0;
            },
            3);
    assertTrue(innerCalls[0] > 3);
    assertTrue(r.value().compareTo(big(10)) >= 0);
  }

  // Cross-thread: a thread created inside a scope inherits the budget.
  @Test
  void threadsCreatedInsideAScopeShareOneGlobalBudget() throws Exception {
    Gen<List<Integer>> lists = Gens.list(Gens.u8(), 0, 50);
    List<Integer> start = new ArrayList<>(Collections.nCopies(50, 7));
    AtomicLong calls = new AtomicLong();
    final int maxSteps = 10;
    Gen<BigInteger> parent =
        Gen.of(
            rng -> BigInteger.ZERO,
            (value, falsifies, steps) -> {
              List<Thread> threads = new ArrayList<>();
              for (int t = 0; t < 12; t++) {
                Thread th =
                    new Thread(
                        () ->
                            lists.shrinkNested(
                                new ArrayList<>(start),
                                l -> {
                                  calls.incrementAndGet();
                                  return l.size() == 50;
                                },
                                maxSteps));
                threads.add(th);
                th.start();
              }
              for (Thread th : threads) {
                try {
                  th.join();
                } catch (InterruptedException e) {
                  throw new IllegalStateException(e);
                }
              }
              return new Shrunk<>(value, 0);
            });
    parent.shrink(BigInteger.ZERO, v -> true, maxSteps);
    assertTrue(calls.get() <= Limits.attemptBudget(maxSteps), "evaluations " + calls + " across threads");
    assertTrue(calls.get() > 50, "the threads must really have shrunk");
  }

  // Initial copies are charged and skipped once the budget is spent.
  @Test
  void tuplePassesOverALargeListDoNotCopyAfterTheBudgetIsSpent() {
    ShrinkBudget.defaultMaxWork = 2_000_000; // small allowance so the test is quick
    final int n = 10_000;
    // Elements in DISTINCT list objects the property saw = elements actually copied
    // (the stateful coordinate re-hands the same list without copying it).
    java.util.Map<Object, Boolean> seen = new java.util.IdentityHashMap<>();
    long[] evaluated = {0};
    Gen<BigInteger> oneStep =
        Gen.of(
            rng -> big(1_000_000),
            (value, falsifies, max) ->
                value.signum() > 0 && max > 0 && falsifies.test(value.subtract(BigInteger.ONE))
                    ? new Shrunk<>(value.subtract(BigInteger.ONE), 1)
                    : new Shrunk<>(value, 0));
    Gen<Gens.Pair<List<Long>, BigInteger>> g = Gens.tuple2(Gens.list(Gens.int64(0, 9), 0, n), oneStep);
    Gens.Pair<List<Long>, BigInteger> start =
        new Gens.Pair<>(new ArrayList<>(Collections.nCopies(n, 0L)), big(1_000_000));

    Shrunk<Gens.Pair<List<Long>, BigInteger>> r =
        g.shrink(
            start,
            p -> {
              if (seen.put(p.first(), Boolean.TRUE) == null) {
                evaluated[0] += p.first().size();
              }
              return p.first().size() == n;
            },
            1000);

    assertTrue(r.steps() >= 900, "the stateful component must force many passes: " + r.steps());
    assertTrue(ShrinkBudget.lastWork <= 2_000_000, "charged work " + ShrinkBudget.lastWork);
    // Candidate lists handed to the property are what the budget charged, so they stay bounded too.
    assertTrue(evaluated[0] <= 2_000_000L + n, "elements handed to the property " + evaluated[0]);
  }

  @Test
  void bytesAndListEntryPointsChargeTheirInitialCopyAndReturnTheInputWhenSpent() {
    ShrinkBudget.defaultMaxWork = 5;
    byte[] data = new byte[100];
    int[] calls = {0};
    Shrunk<byte[]> b =
        Gens.bytes(0, 200)
            .shrink(
                data,
                x -> {
                  calls[0]++;
                  return true;
                },
                1000);
    assertSame(data, b.value());
    assertEquals(0, b.steps());
    List<BigInteger> items = new ArrayList<>(Collections.nCopies(100, big(1)));
    Shrunk<List<BigInteger>> l = Shrink.shrinkList(items, BigInteger.ZERO, x -> false, 1000);
    assertEquals(100, l.value().size());
    assertEquals(0, l.steps());
    Shrunk<byte[]> pb = Shrink.shrinkBytes(data, x -> false, 1000);
    assertSame(data, pb.value());
    assertEquals(0, calls[0]);
  }

  // A custom Shrinker that shrinks sub-values through the public Gen.shrink joins the
  // enclosing budget; it must not open a fresh one per pass.
  @Test
  void aCustomShrinkerCallingGenShrinkPerPassStaysInsideOneGlobalBudget() {
    Gen<List<Integer>> inner = Gens.list(Gens.u8(), 0, 50);
    List<Integer> start = new ArrayList<>(Collections.nCopies(50, 7));
    final int maxSteps = 10;
    int[] calls = {0};
    Shrinker<List<Integer>> custom =
        (value, falsifies, steps) -> {
          for (int pass = 0; pass < 200; pass++) {
            inner.shrink(new ArrayList<>(value), falsifies, steps);
          }
          return new Shrunk<>(value, 0);
        };
    Gen<List<Integer>> outer = Gen.of(rng -> new ArrayList<>(start), custom);
    outer.shrink(
        new ArrayList<>(start),
        l -> {
          calls[0]++;
          return l.size() == 50; // every smaller candidate is rejected
        },
        maxSteps);
    assertTrue(calls[0] <= Limits.attemptBudget(maxSteps), "evaluations " + calls[0]);
    assertTrue(ShrinkBudget.lastWork <= Limits.MAX_SHRINK_WORK);

    // The same through Runner.check with an explicit shrinker.
    calls[0] = 0;
    Verdict v =
        Runner.check(
            1,
            outer,
            l -> {
              calls[0]++;
              return l.size() != 50;
            },
            new Options(1, maxSteps, CiMode.OFF),
            custom);
    assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
    assertTrue(calls[0] <= Limits.attemptBudget(maxSteps) + 2, "runner evaluations " + calls[0]);
  }

  // Re-entrancy from inside a property stays isolated for Gen.shrink too.
  @Test
  void aGenShrinkInsideAPropertyIsIsolatedAndMatchesAStandaloneRun() {
    Gen<List<Integer>> g = Gens.list(Gens.u8(), 0, 50);
    List<Integer> start = new ArrayList<>(Collections.nCopies(50, 3));
    Shrunk<List<Integer>> standalone = g.shrink(new ArrayList<>(start), l -> l.size() >= 5, 1000);
    assertEquals(5, standalone.value().size());
    List<Shrunk<List<Integer>>> inner = new ArrayList<>();
    Verdict v =
        Runner.check(
            3,
            Gens.integer(big(0), big(1000)),
            x -> {
              inner.add(g.shrink(new ArrayList<>(start), l -> l.size() >= 5, 1000));
              return x.compareTo(big(500)) < 0;
            },
            new Options(100, 1, CiMode.OFF));
    assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
    assertFalse(inner.isEmpty());
    for (Shrunk<List<Integer>> r : inner) {
      assertEquals(standalone.value(), r.value());
      assertEquals(standalone.steps(), r.steps());
    }
  }

  // A thread inheriting a budget that outlives its scope must not keep the spent budget.
  @Test
  void aThreadOutlivingItsScopeShrinksNormallyAfterwards() throws Exception {
    Gen<List<Integer>> lists = Gens.list(Gens.u8(), 0, 50);
    java.util.concurrent.CountDownLatch go = new java.util.concurrent.CountDownLatch(1);
    List<Shrunk<List<Integer>>> out = new ArrayList<>();
    Thread[] holder = new Thread[1];
    Gen<BigInteger> parent =
        Gen.of(
            rng -> BigInteger.ZERO,
            (value, falsifies, steps) -> {
              holder[0] =
                  new Thread(
                      () -> {
                        try {
                          go.await();
                        } catch (InterruptedException e) {
                          return;
                        }
                        out.add(lists.shrinkNested(new ArrayList<>(Collections.nCopies(50, 7)), l -> l.size() >= 1, 1000));
                        out.add(lists.shrink(new ArrayList<>(Collections.nCopies(50, 7)), l -> l.size() >= 1, 1000));
                      });
              holder[0].start();
              return new Shrunk<>(value, 0);
            });
    parent.shrink(BigInteger.ZERO, v -> true, 10); // the scope closes here, with the thread alive
    go.countDown();
    holder[0].join();
    assertEquals(2, out.size());
    for (Shrunk<List<Integer>> r : out) {
      assertEquals(1, r.value().size());
    }
  }

  // Byte shrinking converts every candidate back to an array: that copy is charged too.
  @Test
  void bytePathsChargeTwiceTheCandidateSizePerAttempt() {
    final int n = 4096;
    ShrinkBudget.defaultMaxWork = 1_000_000;
    int[] calls = {0};
    Shrink.shrinkBytes(
        new byte[n],
        b -> {
          calls[0]++;
          return b.length == n ? false : true; // only the original falsifies
        },
        1000);
    // Deletions cost 2 * (n - 1) = 8190 each against 1M of work: about 120 evaluations.
    // Without the array-conversion charge it would be about 245.
    assertTrue(calls[0] < 150, "evaluations " + calls[0]);
    assertTrue(ShrinkBudget.lastWork <= 1_000_000, "charged work " + ShrinkBudget.lastWork);
  }
}
