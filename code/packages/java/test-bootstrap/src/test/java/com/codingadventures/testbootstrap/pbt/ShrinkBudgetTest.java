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
    try (ShrinkBudget b = ShrinkBudget.enterChild(1000, 10)) {
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
    try (ShrinkBudget b = ShrinkBudget.enterChild(1000, 10)) {
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
    Options outer = new Options(100, 10_000, CiMode.ON); // room: an inner check is standalone-like
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
            1000);
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
            new Options(100, 1000, CiMode.OFF));
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

  // Property context is per thread: a worker blocked inside the property must not make
  // the shrinker thread's own Gen.shrink calls isolate.
  @Test
  void aWorkerInsideThePropertyDoesNotMakeTheShrinkerThreadIsolate() throws Exception {
    Gen<List<Integer>> inner = Gens.list(Gens.u8(), 0, 50);
    List<Integer> start = new ArrayList<>(Collections.nCopies(50, 7));
    final int maxSteps = 10;
    AtomicLong calls = new AtomicLong();
    java.util.concurrent.CountDownLatch inProperty = new java.util.concurrent.CountDownLatch(1);
    java.util.concurrent.CountDownLatch release = new java.util.concurrent.CountDownLatch(1);
    Thread[] worker = new Thread[1];
    Shrinker<List<Integer>> custom =
        (value, falsifies, steps) -> {
          worker[0] = new Thread(() -> falsifies.test(new ArrayList<>(value)));
          worker[0].start();
          try {
            inProperty.await();
            for (int pass = 0; pass < 200; pass++) {
              inner.shrink(new ArrayList<>(value), falsifies, steps);
            }
          } catch (InterruptedException e) {
            throw new IllegalStateException(e);
          } finally {
            release.countDown();
          }
          return new Shrunk<>(value, 0);
        };
    Gen<List<Integer>> outer = Gen.of(rng -> new ArrayList<>(start), custom);
    outer.shrink(
        new ArrayList<>(start),
        l -> {
          calls.incrementAndGet();
          if (Thread.currentThread() == worker[0]) {
            inProperty.countDown();
            try {
              release.await();
            } catch (InterruptedException e) {
              throw new IllegalStateException(e);
            }
          }
          return l.size() == 50;
        },
        maxSteps);
    worker[0].join();
    // +1: the worker's own uncharged call
    assertTrue(calls.get() <= Limits.attemptBudget(maxSteps) + 1, "evaluations " + calls);
  }


  // ---------------------------------------------------------------------------
  // The budget tree (redesign): the bound holds whatever the classification.
  // ---------------------------------------------------------------------------

  /** Runs {@code body} in a root scope with a small work cap; returns {attempts, work} of the root. */
  private static long[] rootTotals(long cap, Runnable body) {
    ShrinkBudget.defaultMaxWork = cap;
    body.run();
    return new long[] {ShrinkBudget.lastAttempts, ShrinkBudget.lastWork};
  }

  private static Predicate<List<Integer>> sizeIs(int n, AtomicLong calls) {
    return l -> {
      calls.incrementAndGet();
      return l.size() == n;
    };
  }

  // 1. A property calling Runner.check on every attempt, at nesting depths 1, 2 and 3.
  private static void nested(int depth, AtomicLong evals, int maxSteps) {
    Runner.check(
        1,
        Gens.list(Gens.u8(), 0, 40),
        l -> {
          if (ShrinkBudget.current() != null) {
            evals.incrementAndGet(); // only evaluations inside the outermost shrink
          }
          if (depth > 1) {
            nested(depth - 1, evals, maxSteps);
          }
          return l.size() < 5;
        },
        new Options(100, maxSteps, CiMode.OFF));
  }

  @Test
  void aPropertyCallingRunnerCheckKeepsTheRootTotalsWithinTheRootCap() {
    for (int depth = 1; depth <= 3; depth++) {
      AtomicLong evals = new AtomicLong();
      final int maxSteps = 5;
      final int d = depth;
      // the outermost check's shrink scope is the root, and it closes last
      long[] t = rootTotals(1_000_000, () -> nested(d, evals, maxSteps));
      long cap = 100 + Limits.attemptBudget(maxSteps); // numTests + shrink cap
      assertTrue(t[0] <= cap, "depth " + d + " attempts " + t[0]);
      assertTrue(t[1] <= 1_000_000, "depth " + d + " work " + t[1]);
      assertTrue(evals.get() > 0);
      // Every charged evaluation reaches the root (<= cap); only each check's free
      // first case is uncharged, one per check call, one level down at most.
      assertTrue(evals.get() <= 3 * cap * d, "depth " + d + " evaluations " + evals);
      Hooks.reset();
    }
  }

  // 2. A custom shrinker recursing into Gen.shrink / shrinkList, and a filter predicate doing so.
  @Test
  void aRecursingCustomShrinkerStaysInsideTheRootCap() {
    Gen<List<Integer>> inner = Gens.list(Gens.u8(), 0, 50);
    List<Integer> start = new ArrayList<>(Collections.nCopies(50, 7));
    AtomicLong calls = new AtomicLong();
    AtomicLong extra = new AtomicLong();
    final int maxSteps = 10;
    Shrinker<List<Integer>> custom =
        (value, falsifies, steps) -> {
          for (int pass = 0; pass < 100; pass++) {
            inner.shrink(new ArrayList<>(value), falsifies, steps);
            // a property of the shrinker's own: charged only through the budget chain
            inner.shrink(new ArrayList<>(value), sizeIs(50, extra), steps);
            Shrink.shrinkList(
                new ArrayList<>(List.of(big(1), big(2), big(3))), BigInteger.ZERO, l -> l.isEmpty(), steps);
          }
          return new Shrunk<>(value, 0);
        };
    Gen<List<Integer>> outer = Gen.of(rng -> new ArrayList<>(start), custom);
    long[] t = rootTotals(1_000_000, () -> outer.shrink(new ArrayList<>(start), sizeIs(50, calls), maxSteps));
    assertTrue(t[0] <= Limits.attemptBudget(maxSteps), "root attempts " + t[0]);
    assertTrue(t[1] <= 1_000_000, "root work " + t[1]);
    assertTrue(calls.get() <= Limits.attemptBudget(maxSteps), "evaluations " + calls);
    assertTrue(calls.get() + extra.get() <= Limits.attemptBudget(maxSteps), "all evaluations " + (calls.get() + extra.get()));
  }

  @Test
  void aFilterPredicateThatShrinksStaysInsideTheRootCap() {
    Gen<List<Integer>> g = Gens.list(Gens.u8(), 0, 200);
    List<Integer> start = new ArrayList<>(Collections.nCopies(200, 3));
    AtomicLong innerEvals = new AtomicLong();
    final int maxSteps = 5;
    Gen<BigInteger> filtered =
        Gens.filter(
            Gens.integer(big(0), big(1000)),
            x -> {
              g.shrink(new ArrayList<>(start), sizeIs(200, innerEvals), 1000);
              return true;
            },
            100);
    long[] t = rootTotals(5_000_000, () -> filtered.shrink(big(900), x -> x.compareTo(big(10)) >= 0, maxSteps));
    assertTrue(innerEvals.get() > 0, "the predicate must have shrunk");
    assertTrue(t[0] <= Limits.attemptBudget(maxSteps), "root attempts " + t[0]);
    assertTrue(t[1] <= 5_000_000, "root work " + t[1]);
    // Each inner shrink alone could use ~400 attempts; the root allows 144 in all.
    assertTrue(innerEvals.get() <= Limits.attemptBudget(maxSteps), "inner evaluations " + innerEvals);
  }

  // 3. A thread spawned in a scope charges the root, also after its spawning inner scope closed.
  @Test
  void aThreadSpawnedInAScopeChargesTheRootEvenAfterTheInnerScopeClosed() throws Exception {
    try (ShrinkBudget root = ShrinkBudget.enterChild(10)) {
      Thread[] t = new Thread[1];
      java.util.concurrent.CountDownLatch go = new java.util.concurrent.CountDownLatch(1);
      boolean[] charged = new boolean[1];
      try (ShrinkBudget inner = ShrinkBudget.enterChild(10)) {
        t[0] =
            new Thread(
                () -> {
                  try {
                    go.await();
                  } catch (InterruptedException e) {
                    return;
                  }
                  ShrinkBudget cur = ShrinkBudget.current();
                  charged[0] = cur != null && cur.charge(1);
                });
        t[0].start();
      }
      long before = root.attempts();
      go.countDown();
      t[0].join();
      assertTrue(charged[0]);
      assertEquals(before + 1, root.attempts());
    }
  }

  // 4. A thread outliving the root gets a fresh working root; a reused worker is not starved.
  @Test
  void aThreadOutlivingTheRootGetsAFreshWorkingRootEachTime() throws Exception {
    java.util.concurrent.CountDownLatch go = new java.util.concurrent.CountDownLatch(1);
    Thread[] t = new Thread[1];
    List<Integer> sizes = new ArrayList<>();
    try (ShrinkBudget root = ShrinkBudget.enterChild(1)) {
      t[0] =
          new Thread(
              () -> {
                try {
                  go.await();
                } catch (InterruptedException e) {
                  return;
                }
                Gen<List<Integer>> g = Gens.list(Gens.u8(), 0, 50);
                for (int round = 0; round < 3; round++) { // the same inherited thread, reused
                  sizes.add(g.shrink(new ArrayList<>(Collections.nCopies(50, 7)), l -> l.size() >= 1, 1000).value().size());
                }
              });
      t[0].start();
    }
    go.countDown();
    t[0].join();
    assertEquals(List.of(1, 1, 1), sizes);
  }

  // 5. A passing inner check with a huge numTests under an outer shrink is bounded by case charging.
  @Test
  void aPassingInnerCheckWithHugeNumTestsIsBoundedUnderAnOuterShrink() {
    AtomicLong innerCases = new AtomicLong();
    final int maxSteps = 5;
    long[] t =
        rootTotals(
            1_000_000,
            () ->
                Gens.integer(big(0), big(1000))
                    .shrink(
                        big(900),
                        x -> {
                          Runner.check(
                              1,
                              Gens.integer(big(0), big(9)),
                              y -> {
                                innerCases.incrementAndGet();
                                return true;
                              },
                              new Options(Limits.MAX_CASES, 10, CiMode.OFF));
                          return x.compareTo(big(10)) >= 0;
                        },
                        maxSteps));
    assertTrue(innerCases.get() <= Limits.attemptBudget(maxSteps) + 64, "inner cases " + innerCases);
    assertTrue(t[0] <= Limits.attemptBudget(maxSteps), "root attempts " + t[0]);
  }

  // 6. shrinkInt and simulateReplay inside a property are charged.
  @Test
  void shrinkIntAndSimulateReplayInsideAPropertyAreCharged() {
    AtomicLong evals = new AtomicLong();
    final int maxSteps = 5;
    long[] t =
        rootTotals(
            1_000_000,
            () ->
                Gens.integer(big(0), big(1000))
                    .shrink(
                        big(900),
                        x -> {
                          Shrink.shrinkInt(big(500), BigInteger.ZERO, BigInteger.ZERO, big(1000), v -> {
                            evals.incrementAndGet();
                            return v.compareTo(big(100)) < 0;
                          }, 1000);
                          Shrink.simulateReplay(3, v -> {
                            evals.incrementAndGet();
                            return v.compareTo(big(100)) < 0;
                          });
                          return x.compareTo(big(10)) >= 0;
                        },
                        maxSteps));
    assertTrue(evals.get() > 0);
    assertTrue(t[0] <= Limits.attemptBudget(maxSteps), "root attempts " + t[0]);
    assertTrue(evals.get() <= Limits.attemptBudget(maxSteps), "inner evaluations " + evals);
  }

  // 7. The inner verdict equals the standalone verdict when the outer budget has room
  //    (see aReentrantRunnerCheckIsNotStarvedByTheOuterShrink).
  // 8. A starved nested check reports property-exhausted without evaluating anything;
  //    the TOP-LEVEL check, which has no ambient, always evaluates its first case.
  @Test
  void aStarvedNestedCheckIsExhaustedButATopLevelCheckStillRuns() {
    int[] evaluated = {0};
    try (ShrinkBudget root = ShrinkBudget.enterChild(0)) { // 64 attempts
      while (root.charge(1)) {
        // spend it all
      }
      long before = root.attempts();
      Verdict nested =
          Runner.check(
              1,
              Gens.list(Gens.u8(), 5, 5),
              l -> {
                evaluated[0]++;
                return false;
              },
              new Options(10, 1000, CiMode.OFF));
      assertEquals(FailureIds.EXHAUSTED, nested.failureId());
      assertFalse(nested.passed());
      assertEquals(0, evaluated[0]);
      assertEquals(before, root.attempts()); // all or nothing: unchanged
    }
    Verdict top =
        Runner.check(
            1, Gens.list(Gens.u8(), 5, 5), l -> false, new Options(10, 1000, CiMode.OFF));
    assertEquals(FailureIds.COUNTEREXAMPLE, top.failureId());
    assertTrue(top.minimal() != null);
  }

  // A branching property (m = 2 nested checks per evaluation, depth 4) stays inside the root cap.
  private static void branch(int level, AtomicLong counted) {
    Runner.check(
        1,
        Gens.integer(big(0), big(9)),
        x -> {
          if (level > 1) {
            counted.incrementAndGet(); // evaluations made by NESTED checks
          }
          if (level < 4) {
            branch(level + 1, counted);
            branch(level + 1, counted);
          }
          return x.compareTo(big(3)) < 0;
        },
        new Options(10, 5, CiMode.OFF));
  }

  @Test
  void aBranchingPropertyKeepsTotalEvaluationsWithinTheRootCap() {
    AtomicLong counted = new AtomicLong();
    long[] t = rootTotals(1_000_000, () -> branch(1, counted));
    long cap = 10 + Limits.attemptBudget(5);
    assertTrue(counted.get() > 0);
    assertTrue(t[0] <= cap, "root attempts " + t[0]);
    // every nested evaluation is preceded by a charge that reaches the root
    assertTrue(counted.get() <= cap, "nested evaluations " + counted + " (2^depth would be far more)");
  }

  // A nested Runner.check during the outer case loop (generation phase, before any shrink)
  // is charged to the outer root.
  @Test
  void aNestedCheckInsideTheOuterCaseLoopChargesTheOuterRoot() {
    AtomicLong innerCases = new AtomicLong();
    long[] t =
        rootTotals(
            1_000_000,
            () ->
                Runner.check(
                    1,
                    Gens.integer(big(0), big(9)),
                    x -> {
                      Runner.check(
                          2,
                          Gens.integer(big(0), big(9)),
                          y -> {
                            innerCases.incrementAndGet();
                            return true;
                          },
                          new Options(200, 10, CiMode.OFF));
                      return true; // the outer property never fails: no shrink at all
                    },
                    new Options(5, 5, CiMode.OFF)));
    long cap = 5 + Limits.attemptBudget(5);
    assertTrue(t[0] > 0 && t[0] <= cap, "root attempts " + t[0]);
    assertTrue(innerCases.get() <= cap, "inner cases " + innerCases + " (uncharged: 5 x 200)");
  }

  @Test
  void aRefusedTakeLeavesEveryCounterUnchanged() {
    try (ShrinkBudget root = ShrinkBudget.enterChild(1000, 100)) {
      try (ShrinkBudget child = ShrinkBudget.enterChild(1000, 1_000_000)) {
        assertTrue(child.charge(60));
        assertFalse(child.charge(60)); // the root has only 40 work left
        assertEquals(1, child.attempts());
        assertEquals(60, child.work());
        assertEquals(1, root.attempts());
        assertEquals(60, root.work());
      }
    }
  }

  // 9. A Long.MAX_VALUE cost is refused without wrapping.
  @Test
  void aHugeCostIsRefusedWithoutWrapping() {
    try (ShrinkBudget b = ShrinkBudget.enterChild(1000, 100)) {
      assertTrue(b.charge(60));
      assertFalse(b.charge(Long.MAX_VALUE));
      assertFalse(b.chargeCopy(Long.MAX_VALUE));
      assertEquals(60, b.work());
    }
  }

  // 10. Concurrent charging from N threads never exceeds the cap.
  @Test
  void concurrentChargingNeverExceedsTheCap() throws Exception {
    try (ShrinkBudget b = ShrinkBudget.enterChild(1000, 5000)) {
      AtomicLong ok = new AtomicLong();
      List<Thread> ts = new ArrayList<>();
      for (int i = 0; i < 16; i++) {
        Thread t =
            new Thread(
                () -> {
                  for (int k = 0; k < 1000; k++) {
                    if (b.charge(1)) {
                      ok.incrementAndGet();
                    }
                  }
                });
        ts.add(t);
        t.start();
      }
      for (Thread t : ts) {
        t.join();
      }
      assertEquals(5000, ok.get());
      assertEquals(5000, b.work());
      assertEquals(5000, b.attempts());
    }
  }

  // 11. AMBIENT is restored after a property or shrinker throws; nesting beyond the cap joins.
  @Test
  void theAmbientBudgetIsRestoredAfterThrowsAndDeepNestingJoins() {
    Gen<BigInteger> thrower =
        Gen.of(rng -> BigInteger.ZERO, (v, f, s) -> {
          throw new IllegalStateException("shrinker boom");
        });
    try {
      thrower.shrink(BigInteger.ZERO, x -> true, 5);
    } catch (IllegalStateException expected) {
      // fine
    }
    assertNull(ShrinkBudget.current());
    try {
      Gens.integer(big(0), big(10)).shrink(big(5), x -> {
        throw new IllegalStateException("property boom");
      }, 5);
    } catch (IllegalStateException expected) {
      // fine
    }
    assertNull(ShrinkBudget.current());

    List<ShrinkBudget> opened = new ArrayList<>();
    for (int i = 0; i < ShrinkBudget.MAX_DEPTH + 6; i++) {
      opened.add(ShrinkBudget.enterChild(10));
    }
    for (int i = 0; i < opened.size(); i++) {
      assertEquals(i >= ShrinkBudget.MAX_DEPTH, opened.get(i).isJoined(), "scope " + i);
    }
    for (int i = opened.size() - 1; i >= 0; i--) {
      opened.get(i).close();
    }
    assertNull(ShrinkBudget.current());
  }

  // 12. The pool-thread escape is pinned as documented behaviour (VER05e section 4.1).
  @Test
  void aThreadThatPredatesTheScopeHasNoAmbientBudget() throws Exception {
    java.util.concurrent.ExecutorService pool = java.util.concurrent.Executors.newSingleThreadExecutor();
    try {
      pool.submit(() -> {}).get(); // the worker exists before any scope
      try (ShrinkBudget root = ShrinkBudget.enterChild(10)) {
        assertNull(pool.submit(ShrinkBudget::current).get()); // documented limitation
        assertTrue(ShrinkBudget.current() != null);
      }
    } finally {
      pool.shutdown();
    }
  }

  // Sampling a large collection inside a nested check is work.
  @Test
  void aNestedCheckSamplingLargeListsIsChargedAsWork() {
    List<Verdict> inner = new ArrayList<>();
    rootTotals(
        10_000,
        () ->
            Gens.integer(big(0), big(1000))
                .shrink(
                    big(900),
                    x -> {
                      inner.add(
                          Runner.check(1, Gens.list(Gens.u8(), 2000, 2000), l -> true, new Options(50, 10, CiMode.OFF)));
                      return x.compareTo(big(10)) >= 0;
                    },
                    1000));
    assertFalse(inner.isEmpty());
    assertTrue(inner.stream().anyMatch(v -> FailureIds.EXHAUSTED.equals(v.failureId())));
  }

  // maxSteps is validated on the public and nested paths; maxSteps 0 returns the input uncopied.
  @Test
  void maxStepsIsValidatedAndZeroStepsReturnsTheInputUncopied() {
    Gen<List<Integer>> g = Gens.list(Gens.u8(), 0, 5);
    org.junit.jupiter.api.Assertions.assertThrows(IllegalArgumentException.class, () -> g.shrink(new ArrayList<>(), l -> true, -1));
    org.junit.jupiter.api.Assertions.assertThrows(IllegalArgumentException.class, () -> g.shrinkNested(new ArrayList<>(), l -> true, -1));
    List<Integer> in = new ArrayList<>(List.of(1, 2, 3));
    assertSame(in, Shrink.shrinkSequence(in, l -> true, 0, 0, null).value());
  }

  // LOW 1: when the chain refuses the re-verification charge, the shrunk value is not
  // adopted: the original witness is reported with 0 steps.
  @Test
  void aRefusedReVerificationReportsTheOriginalWitnessWithZeroSteps() {
    try (ShrinkBudget root = ShrinkBudget.enterChild(0)) { // 64 attempts
      // A shrinker that finds a smaller failing value but spends the whole chain first.
      Shrinker<Integer> spender =
          (value, falsifies, steps) -> {
            while (ShrinkBudget.current().charge(1)) {
              // spend until the chain refuses
            }
            return new Shrunk<>(3, 1);
          };
      Verdict v =
          Runner.check(1, Gens.constant(100), x -> false, new Options(1, 5, CiMode.OFF), spender);
      assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
      assertEquals(100, v.minimal());
      assertEquals(0, v.shrinkSteps());
    }
    // With room, the same shrinker's result IS adopted (3 still fails the property).
    Shrinker<Integer> plain = (value, falsifies, steps) -> new Shrunk<>(3, 1);
    Verdict ok = Runner.check(1, Gens.constant(100), x -> false, new Options(1, 5, CiMode.OFF), plain);
    assertEquals(3, ok.minimal());
    assertEquals(1, ok.shrinkSteps());
  }

  // LOW 4: filter charges before its user predicate runs, so under a spent budget the
  // predicate is not invoked at all.
  @Test
  void aFilterPredicateIsNotInvokedUnderASpentBudget() {
    int[] calls = {0};
    Gen<BigInteger> filtered =
        Gens.filter(
            Gens.integer(big(0), big(1000)),
            x -> {
              calls[0]++;
              return true;
            },
            100);
    try (ShrinkBudget root = ShrinkBudget.enterChild(0)) {
      while (root.charge(1)) {
        // spend it all
      }
      Shrunk<BigInteger> r = filtered.shrink(big(900), x -> true, 1000);
      assertEquals(0, r.steps());
      assertEquals(0, calls[0]);
    }
    // With room the predicate runs and shrinking works.
    Shrunk<BigInteger> r = filtered.shrink(big(900), x -> x.compareTo(big(10)) >= 0, 1000);
    assertTrue(calls[0] > 0);
    assertEquals(big(10), r.value());
  }

  // The filter SAMPLER's retries are attempts charged up the chain inside a nested check.
  @Test
  void aFilterSamplerRejectingEverythingStaysInsideTheRootCapInANestedCheck() {
    AtomicLong predCalls = new AtomicLong();
    Gen<BigInteger> rejecting =
        Gens.filter(
            Gens.integer(big(0), big(1000)),
            x -> {
              predCalls.incrementAndGet();
              return false;
            },
            Limits.MAX_FILTER_RETRIES);
    final int maxSteps = 5;
    rootTotals(
        1_000_000,
        () ->
            Gens.integer(big(0), big(1000))
                .shrink(
                    big(900),
                    x -> {
                      Runner.check(1, rejecting, y -> true, new Options(3, 10, CiMode.OFF));
                      return x.compareTo(big(10)) >= 0;
                    },
                    maxSteps));
    assertTrue(predCalls.get() > 0);
    assertTrue(predCalls.get() <= Limits.attemptBudget(maxSteps), "sampler retries " + predCalls);
  }

  @Test
  void maxRetriesIsCappedByALimit() {
    Gen<BigInteger> g = Gens.integer(big(0), big(9));
    Gens.filter(g, x -> true, Limits.MAX_FILTER_RETRIES);
    org.junit.jupiter.api.Assertions.assertThrows(
        IllegalArgumentException.class, () -> Gens.filter(g, x -> true, Limits.MAX_FILTER_RETRIES + 1));
  }

  // A top-level check whose nested check was starved is exhausted, never passed.
  @Test
  void aStarvedNestedCheckNeverLetsTheTopLevelCheckPass() {
    Verdict v =
        Runner.check(
            1,
            Gens.integer(big(0), big(9)),
            x -> {
              Runner.check(2, Gens.integer(big(0), big(9)), y -> true, new Options(5000, 0, CiMode.OFF));
              return true; // the property itself never fails
            },
            new Options(3, 0, CiMode.OFF)); // root cap: 3 + 64 attempts
    assertFalse(v.passed());
    assertEquals(FailureIds.EXHAUSTED, v.failureId());
    // Without starvation the same shape passes.
    Verdict ok =
        Runner.check(
            1,
            Gens.integer(big(0), big(9)),
            x -> {
              Runner.check(2, Gens.integer(big(0), big(9)), y -> true, new Options(5, 0, CiMode.OFF));
              return true;
            },
            new Options(3, 0, CiMode.OFF));
    assertTrue(ok.passed());
  }

  // Generation work counts one level deeper (a list of lists).
  @Test
  void sampledListsOfListsAreChargedByTheirContents() {
    List<Verdict> inner = new ArrayList<>();
    rootTotals(
        20_000,
        () ->
            Gens.integer(big(0), big(1000))
                .shrink(
                    big(900),
                    x -> {
                      inner.add(
                          Runner.check(
                              1,
                              Gens.list(Gens.list(Gens.u8(), 100, 100), 100, 100),
                              l -> true,
                              new Options(50, 10, CiMode.OFF)));
                      return x.compareTo(big(10)) >= 0;
                    },
                    1000));
    // 100 + 100 x 100 = 10,100 per case: the second case no longer fits in 20,000.
    assertEquals(FailureIds.EXHAUSTED, inner.get(0).failureId()); // the very first inner check
  }

  // A witness that cannot be rendered still yields a verdict (non-redacted path too), and a
  // hostile collection cannot break the nested work estimate either.
  @Test
  void anUnrenderableWitnessStillYieldsAVerdictWithoutRedaction() {
    List<Integer> bad =
        new java.util.AbstractList<>() {
          @Override
          public Integer get(int i) {
            throw new IllegalStateException("hostile accessor");
          }

          @Override
          public int size() {
            return 1;
          }
        };
    Verdict v = Runner.check(1, Gens.constant(bad), x -> false, new Options(1, 10, CiMode.OFF));
    assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
    assertTrue(v.message().contains(com.codingadventures.testbootstrap.corpusjson.Canon.UNRENDERABLE));
    Verdict nested = rootCheckNested(bad);
    assertEquals(FailureIds.COUNTEREXAMPLE, nested.failureId());
  }

  private static Verdict rootCheckNested(List<Integer> bad) {
    Verdict[] out = new Verdict[1];
    rootTotals(
        1_000_000,
        () ->
            Gens.integer(big(0), big(1000))
                .shrink(
                    big(900),
                    x -> {
                      out[0] = Runner.check(1, Gens.constant(bad), y -> false, new Options(1, 10, CiMode.OFF));
                      return x.compareTo(big(10)) >= 0;
                    },
                    5));
    return out[0];
  }

  // Stacked top-level filters cannot multiply uncharged retries: each retry is one element of work.
  private static Gen<BigInteger> stacked(AtomicLong calls) {
    Gen<BigInteger> g = Gens.integer(big(0), big(1_000_000));
    for (int m : new int[] {31, 37, 41}) {
      g =
          Gens.filter(
              g,
              x -> {
                calls.incrementAndGet();
                return x.mod(big(m)).signum() == 0;
              },
              Limits.MAX_FILTER_RETRIES);
    }
    return g;
  }

  @Test
  void stackedTopLevelFiltersAreBoundedByTheWorkAllowance() {
    ShrinkBudget.defaultMaxWork = 2_000;
    AtomicLong calls = new AtomicLong();
    Verdict v = Runner.check(1, stacked(calls), x -> true, new Options(100, 10, CiMode.OFF));
    assertFalse(v.passed());
    assertEquals(FailureIds.EXHAUSTED, v.failureId());
    assertTrue(calls.get() <= 2_000 + 10, "predicate calls " + calls);
  }

  @Test
  void anOrdinaryFilterStillPassesAtTheDefaultAllowance() {
    AtomicLong calls = new AtomicLong();
    Gen<BigInteger> rare =
        Gens.filter(
            Gens.integer(big(0), big(1_000_000)),
            x -> {
              calls.incrementAndGet();
              return x.mod(big(500)).signum() == 0; // about 1 in 500
            },
            Limits.MAX_FILTER_RETRIES);
    Verdict v = Runner.check(1, rare, x -> true, new Options(100, 10, CiMode.OFF));
    assertTrue(v.passed(), v.message());
    assertTrue(calls.get() > 100);
  }
}
