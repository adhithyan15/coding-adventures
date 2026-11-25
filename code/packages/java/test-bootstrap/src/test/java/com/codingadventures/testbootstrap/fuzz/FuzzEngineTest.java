package com.codingadventures.testbootstrap.fuzz;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Function;
import org.junit.jupiter.api.Test;

/** The executor, minimiser and campaign (VER06f sections 5 and 6). */
class FuzzEngineTest {
  static Options opts(int iterations, long seed) {
    return Options.defaults().withMaxIterations(iterations).withSeed(seed);
  }

  static List<byte[]> seeds(byte[]... s) {
    return Arrays.asList(s);
  }

  static Report run(List<byte[]> corpus, Options o, Target t) {
    return Fuzz.run(corpus, o, t);
  }

  // ---- the executor ----------------------------------------------------------------

  static void boom(byte[] d, Handle h) {
    throw new IllegalStateException("secret " + Arrays.toString(d));
  }

  static Outcome exec(Target t) {
    return exec(t, new byte[] {1, 2, 3});
  }

  static Outcome exec(Target t, byte[] input) {
    Handle h = new Handle(1000, 1 << 20);
    h.cov = new Bitmap();
    return Fuzz.runTarget(t, input, h);
  }

  @Test
  void aThrownExceptionIsACrashWithKindAndInnermostFirstFrames() {
    Outcome o = exec((d, h) -> boom(d, h));
    assertEquals(Outcome.Kind.CRASH, o.kind());
    assertEquals("java.lang.IllegalStateException", o.crashKind());
    assertEquals("FuzzEngineTest.boom", o.frames().get(0));
    assertTrue(o.frames().get(1).startsWith("FuzzEngineTest.lambda$"), o.frames().toString());
    for (String f : o.frames()) {
      assertFalse(f.contains(":"), f);
      assertFalse(f.startsWith("Fuzz.") || f.startsWith("Handle."), "engine frame leaked: " + f);
      assertFalse(f.startsWith("Method") || f.startsWith("Thread"), "JDK frame leaked: " + f);
    }
    assertEquals(2, o.frames().size(), "the stack is cut at the executor: " + o.frames());
  }

  @Test
  void everyKindOfThrowableIsACrashNeverAnEscape() {
    assertEquals("java.lang.Exception", exec((d, h) -> { throw new Exception("checked"); }).crashKind());
    assertEquals("java.lang.AssertionError", exec((d, h) -> { throw new AssertionError("x"); }).crashKind());
    assertEquals("java.lang.ArrayIndexOutOfBoundsException", exec((d, h) -> { int x = d[99]; }).crashKind());
    assertEquals("java.lang.NullPointerException", exec((d, h) -> { Object o = null; o.hashCode(); }).crashKind());
    assertEquals("java.lang.ClassCastException", exec((d, h) -> { Object o = "s"; Integer i = (Integer) o; }).crashKind());
    assertEquals("java.lang.InternalError", exec((d, h) -> { throw new InternalError(); }).crashKind());
    assertEquals(Outcome.Kind.PASS, exec((d, h) -> { }).kind());
  }

  @Test
  void theMessageIsNeverPartOfTheSignatureOrAnyOutput() {
    Target t = (d, h) -> boom(d, h);
    Outcome a = exec(t, new byte[] {1});
    Outcome b = exec(t, new byte[] {9, 9, 9});
    assertEquals(Signature.crashSignature(a.crashKind(), a.frames()), Signature.crashSignature(b.crashKind(), b.frames()));
    assertFalse(a.toString().contains("secret"));
    assertFalse(a.frames().toString().contains("secret"));
  }

  static int recurse(int depth) {
    return recurse(depth + 1) + 1;
  }

  @Test
  void stackOverflowIsACrashAndRecursiveFramesCollapse() {
    Outcome a = exec((d, h) -> recurse(0));
    Outcome b = exec((d, h) -> recurse(0));
    assertEquals("java.lang.StackOverflowError", a.crashKind());
    assertEquals(List.of("FuzzEngineTest.recurse"), a.frames(), "a thousand identical frames collapse to one");
    assertEquals(Signature.crashSignature(a.crashKind(), a.frames()), Signature.crashSignature(b.crashKind(), b.frames()));
    assertTrue(a.frames().size() <= 64);
  }

  static class HostileTrace extends RuntimeException {
    private static final long serialVersionUID = 1L;

    @Override
    public StackTraceElement[] getStackTrace() {
      throw new IllegalStateException("hostile getStackTrace");
    }
  }

  static class NullTrace extends RuntimeException {
    private static final long serialVersionUID = 1L;

    @Override
    public StackTraceElement[] getStackTrace() {
      return null;
    }
  }

  static class RecursiveTrace extends RuntimeException {
    private static final long serialVersionUID = 1L;

    @Override
    public StackTraceElement[] getStackTrace() {
      return getStackTrace();
    }
  }

  @Test
  void aHostileStackTraceStillYieldsACrashWithNoFrames() {
    for (Target t : new Target[] {(d, h) -> { throw new HostileTrace(); }, (d, h) -> { throw new NullTrace(); }, (d, h) -> { throw new RecursiveTrace(); }}) {
      Outcome o = exec(t);
      assertEquals(Outcome.Kind.CRASH, o.kind());
      assertEquals(List.of(), o.frames());
      assertTrue(o.crashKind().startsWith("com.codingadventures.testbootstrap.fuzz.FuzzEngineTest$"), o.crashKind());
    }
  }

  static class InternalErrorTrace extends RuntimeException {
    private static final long serialVersionUID = 1L;

    @Override
    public StackTraceElement[] getStackTrace() {
      throw new InternalError("hostile");
    }
  }

  static class CustomError extends Error {
    private static final long serialVersionUID = 1L;
  }

  static class CustomErrorTrace extends RuntimeException {
    private static final long serialVersionUID = 1L;

    @Override
    public StackTraceElement[] getStackTrace() {
      throw new CustomError();
    }
  }

  static class OomTrace extends RuntimeException {
    private static final long serialVersionUID = 1L;

    @Override
    public StackTraceElement[] getStackTrace() {
      throw new OutOfMemoryError();
    }
  }

  @Test
  void aStackTraceThatThrowsAnyErrorNeverEndsTheCampaign() {
    for (Target t : new Target[] {(d, h) -> { throw new InternalErrorTrace(); }, (d, h) -> { throw new CustomErrorTrace(); }}) {
      Outcome o = exec(t);
      assertEquals(Outcome.Kind.CRASH, o.kind());
      assertEquals(List.of(), o.frames());
      Report r = run(seeds(new byte[] {1}), opts(30, 1).withMinimizeBudget(10), t);
      assertEquals(1, r.crashes().size());
      assertTrue(r.iterations() == 0 || r.iterations() == 30);
    }
    // An out-of-memory while walking the stack is a counted OOM, not an escape.
    assertEquals(Outcome.Kind.OOM, exec((d, h) -> { throw new OomTrace(); }).kind());
    Report r = run(seeds(new byte[] {1}), opts(30, 1), (d, h) -> { throw new OomTrace(); });
    assertEquals(1, r.ooms());
  }

  @Test
  void budgetExhaustionIsStickyAndSharedBetweenTickAndReserve() {
    // A swallowed MEMORY breach followed by reserve(1): still stopped, reported as OOM, not PASS.
    Handle h1 = new Handle(1000, 100);
    h1.cov = new Bitmap();
    Outcome oom =
        Fuzz.runTarget(
            (d, hh) -> {
              try {
                hh.reserve(1000);
              } catch (Throwable swallowed) {
                // keeps going
              }
              hh.reserve(1);
            },
            new byte[0],
            h1);
    assertEquals(Outcome.Kind.OOM, oom.kind());
    // ... and a tick after a memory breach is stopped as the memory breach.
    Handle h2 = new Handle(1000, 100);
    Outcome oom2 =
        Fuzz.runTarget(
            (d, hh) -> {
              try {
                hh.reserve(1000);
              } catch (Throwable swallowed) {
                // keeps going
              }
              hh.tick();
            },
            new byte[0],
            h2);
    assertEquals(Outcome.Kind.OOM, oom2.kind());
    // A swallowed STEPS breach followed by reserve(1): a timeout.
    Handle h3 = new Handle(1, 100);
    Outcome slow =
        Fuzz.runTarget(
            (d, hh) -> {
              hh.tick();
              try {
                hh.tick();
              } catch (Throwable swallowed) {
                // keeps going
              }
              hh.reserve(1);
            },
            new byte[0],
            h3);
    assertEquals(Outcome.Kind.TIMEOUT, slow.kind());
    // The first breach wins: a steps breach then a memory breach stays a timeout.
    Handle h4 = new Handle(0, 100);
    assertEquals(
        Outcome.Kind.TIMEOUT,
        Fuzz.runTarget((d, hh) -> { try { hh.tick(); } catch (Throwable s) { } hh.reserve(1000); }, new byte[0], h4).kind());
    // Each execution of a campaign starts fresh.
    Report r = run(seeds(new byte[] {0}), opts(50, 1).withMemBudget(10), (d, hh) -> { if (d.length > 0 && d[0] != 0) { hh.reserve(100); } else { hh.reserve(5); hh.tick(); } });
    assertTrue(r.ooms() > 0);
    assertTrue(r.admittedSeeds() >= 1, "the clean seed ran with a fresh budget");
  }

  @Test
  void aTargetThatSwallowsTheSentinelThenReturnsOrThrowsIsStillTimeoutOrOom() {
    Target swallowSteps = (d, h) -> { try { h.tick(); h.tick(); } catch (Throwable s) { } };
    Target swallowMem = (d, h) -> { try { h.reserve(1000); } catch (Throwable s) { } };
    Target stepsThenThrow = (d, h) -> { try { h.tick(); h.tick(); } catch (Throwable s) { throw new IllegalStateException("ordinary"); } };
    Target memThenThrow = (d, h) -> { try { h.reserve(1000); } catch (Throwable s) { throw new IllegalStateException("ordinary"); } };
    for (Target t : new Target[] {swallowSteps, stepsThenThrow}) {
      Handle h = new Handle(1, 100);
      assertEquals(Outcome.Kind.TIMEOUT, Fuzz.runTarget(t, new byte[0], h).kind());
    }
    for (Target t : new Target[] {swallowMem, memThenThrow}) {
      Handle h = new Handle(10, 100);
      assertEquals(Outcome.Kind.OOM, Fuzz.runTarget(t, new byte[0], h).kind());
    }
    // And in a campaign: counted as timeouts/OOMs, never as passes or crashes.
    Report a = run(seeds(new byte[] {1}), opts(5, 1).withStepBudget(1), stepsThenThrow);
    assertTrue(a.timeouts() > 0);
    assertTrue(a.crashes().isEmpty());
    Report b = run(seeds(new byte[] {1}), opts(5, 1).withMemBudget(10), memThenThrow);
    assertTrue(b.ooms() > 0);
    assertTrue(b.crashes().isEmpty());
    // An ordinary throw with budget left is still a crash.
    assertEquals(Outcome.Kind.CRASH, Fuzz.runTarget((d, h) -> { throw new IllegalStateException(); }, new byte[0], new Handle(10, 100)).kind());
  }

  @Test
  void framesOfInspectsAtMostMaxWalkElementsAndKeepsSixtyFour() {
    StackTraceElement[] many = new StackTraceElement[10_000];
    for (int i = 0; i < many.length; i++) {
      many[i] = new StackTraceElement("com.acme.C" + i, "m", "C.java", i);
    }
    List<String> frames = Fuzz.framesOf(many);
    assertEquals(64, frames.size());
    assertEquals("C0.m", frames.get(0));
    // Identical frames collapse and the walk is bounded.
    StackTraceElement[] same = new StackTraceElement[1_000_000];
    Arrays.fill(same, new StackTraceElement("com.acme.R", "m", "R.java", 1));
    assertEquals(List.of("R.m"), Fuzz.framesOf(same));
    // JDK, engine and harness frames are dropped; the cut is at the executor.
    StackTraceElement[] mixed = {
      new StackTraceElement("java.util.ArrayList", "get", "A.java", 1),
      new StackTraceElement("jdk.internal.util.Preconditions", "check", "P.java", 1),
      new StackTraceElement("sun.x.Y", "z", "Y.java", 1),
      new StackTraceElement("javax.x.Y", "z", "Y.java", 1),
      new StackTraceElement("com.sun.x.Y", "z", "Y.java", 1),
      new StackTraceElement(Handle.class.getName(), "tick", "H.java", 1),
      new StackTraceElement("com.acme.Target", "go", "T.java", 1),
      new StackTraceElement(Fuzz.class.getName(), "classify", "F.java", 1),
      new StackTraceElement("com.acme.Other", "go", "T.java", 1),
      new StackTraceElement(Fuzz.class.getName(), "runTarget", "F.java", 1),
      new StackTraceElement("com.acme.Above", "never", "T.java", 1),
    };
    assertEquals(List.of("Target.go", "Other.go"), Fuzz.framesOf(mixed));
  }

  @Test
  void differentFailuresGetDifferentSignatures() {
    Outcome a = exec((d, h) -> { throw new IllegalStateException(); });
    Outcome b = exec((d, h) -> { throw new IllegalArgumentException(); });
    Outcome c = exec((d, h) -> boom(d, h));
    assertNotEquals(Signature.crashSignature(a.crashKind(), a.frames()), Signature.crashSignature(b.crashKind(), b.frames()));
    assertNotEquals(Signature.crashSignature(a.crashKind(), a.frames()), Signature.crashSignature(c.crashKind(), c.frames()));
  }

  @Test
  void aRealOutOfMemoryErrorIsAnOomOutcomeAndNothingIsRendered() {
    Outcome o = exec((d, h) -> { throw new OutOfMemoryError("Java heap space: " + Arrays.toString(d)); });
    assertEquals(Outcome.Kind.OOM, o.kind());
    assertEquals("", o.crashKind());
    assertEquals(List.of(), o.frames());
  }

  @Test
  void tickIsADeterministicStepBudgetThatCannotBeSwallowed() {
    Handle h = new Handle(3, 100);
    h.cov = new Bitmap();
    h.tick();
    h.tick();
    h.tick(); // the third is allowed
    assertThrows(Handle.BudgetExceeded.class, h::tick);
    assertThrows(Handle.BudgetExceeded.class, h::tick); // and every later one
    // A target that swallows every Throwable and keeps ticking cannot outrun the budget:
    // it gets nothing but sentinels back, so it can only finish by returning, never by looping on.
    int[] swallowed = {0};
    Outcome o =
        exec(
            (d, hh) -> {
              for (int i = 0; i < 5000; i++) {
                try {
                  hh.tick();
                } catch (Throwable ignored) {
                  swallowed[0]++;
                }
              }
            });
    assertEquals(Outcome.Kind.TIMEOUT, o.kind(), "a target that swallowed the sentinel and returned is still a timeout");
    assertEquals(4000, swallowed[0], "1000 ticks pass, every later one is refused, none is ever accepted");
    Handle strict = new Handle(10, 100);
    strict.cov = new Bitmap();
    Outcome t = Fuzz.runTarget((d, hh) -> { for (;;) { hh.tick(); } }, new byte[0], strict);
    assertEquals(Outcome.Kind.TIMEOUT, t.kind());
    assertEquals(10, strict.steps);
    Outcome zero = Fuzz.runTarget((d, hh) -> hh.tick(), new byte[0], new Handle(0, 0));
    assertEquals(Outcome.Kind.TIMEOUT, zero.kind());
  }

  @Test
  void reserveAccountsBytesWithoutOverflow() {
    Handle h = new Handle(10, 100);
    h.reserve(60);
    h.reserve(40);
    assertEquals(100, h.reserved);
    assertThrows(Handle.BudgetExceeded.class, () -> h.reserve(1));
    assertEquals(100, h.reserved);
    Handle z = new Handle(10, 100);
    z.reserve(0);
    z.reserve(-5);
    assertEquals(0, z.reserved);
    Handle g = new Handle(10, 100);
    assertThrows(Handle.BudgetExceeded.class, () -> g.reserve(Long.MAX_VALUE));
    assertEquals(0, g.reserved);
    Outcome o = Fuzz.runTarget((d, hh) -> hh.reserve(101), new byte[0], new Handle(10, 100));
    assertEquals(Outcome.Kind.OOM, o.kind());
    assertEquals(Outcome.Kind.PASS, Fuzz.runTarget((d, hh) -> hh.reserve(100), new byte[0], new Handle(10, 100)).kind());
  }

  @Test
  void aForeignBudgetSentinelIsRethrownToItsOwner() {
    Handle mine = new Handle(1, 1);
    Handle other = new Handle(1, 1);
    Handle.BudgetExceeded foreign = new Handle.BudgetExceeded(other, Handle.Budget.STEPS);
    assertSame(foreign, assertThrows(Handle.BudgetExceeded.class, () -> Fuzz.classify(foreign, mine)));
    assertEquals(Outcome.Kind.TIMEOUT, Fuzz.classify(new Handle.BudgetExceeded(mine, Handle.Budget.STEPS), mine).kind());
    assertEquals(Outcome.Kind.OOM, Fuzz.classify(new Handle.BudgetExceeded(mine, Handle.Budget.MEMORY), mine).kind());
  }

  @Test
  void nestedCampaignsDoNotClaimEachOthersTimeouts() {
    Options inner = opts(5, 1).withStepBudget(1_000_000);
    Options outer = opts(5, 1).withStepBudget(5);
    Report r =
        run(
            seeds(new byte[] {1}),
            outer,
            (d, oh) -> {
              Fuzz.run(seeds(new byte[] {1}), inner, (dd, ih) -> { });
              for (int i = 0; i < 100; i++) {
                oh.tick(); // the OUTER budget runs out here
              }
            });
    assertTrue(r.timeouts() > 0);
    assertTrue(r.crashes().isEmpty());
  }

  @Test
  void theTargetReceivesItsOwnCopyOfTheInput() {
    byte[] original = {1, 2, 3};
    Report r = run(seeds(original), opts(20, 1), (d, h) -> Arrays.fill(d, (byte) 9));
    assertArrayEquals(new byte[] {1, 2, 3}, original);
    assertArrayEquals(new byte[] {1, 2, 3}, r.corpus().get(0));
  }

  @Test
  void edgeIdIsAStableFnvOfTheLabel() {
    assertEquals(Handle.edgeId("a"), Handle.edgeId("a"));
    assertNotEquals(Handle.edgeId("a"), Handle.edgeId("b"));
    assertEquals(0xaf63dc4c8601ec8cL, Handle.edgeId("a"));
  }

  // ---- minimisation --------------------------------------------------------------------

  static Outcome crashOn(boolean crash, String kind) {
    return crash ? Outcome.crash(kind, List.of("f")) : Outcome.pass();
  }

  @Test
  void theMinimiserReachesTheEmptyInput() {
    String sig = Signature.crashSignature("k", List.of("f"));
    byte[] out = Fuzz.minimizeCrashingInput(new byte[] {7}, sig, d -> crashOn(true, "k"), 100);
    assertEquals(0, out.length);
    assertEquals(0, Fuzz.minimizeCrashingInput(new byte[0], sig, d -> crashOn(true, "k"), 0).length);
    assertEquals(0, Fuzz.minimizeCrashingInput(new byte[40], sig, d -> crashOn(true, "k"), 1000).length);
  }

  @Test
  void theMinimiserKeepsTheExactSignatureAndZeroesBytes() {
    String sig = Signature.crashSignature("k", List.of("f"));
    // Crashes iff byte 3 is 0x42 and the input is at least 4 long.
    Function<byte[], Outcome> test = d -> crashOn(d.length >= 4 && d[3] == 0x42, "k");
    byte[] out = Fuzz.minimizeCrashingInput(new byte[] {1, 2, 3, 0x42, 5, 6, 7, 8}, sig, test, 1000);
    assertArrayEquals(new byte[] {0, 0, 0, 0x42}, out);
  }

  @Test
  void theMinimiserNeverTradesOneCrashForAnother() {
    String sig = Signature.crashSignature("k", List.of("f"));
    // Shorter inputs crash with a DIFFERENT kind: they must be rejected.
    Function<byte[], Outcome> test = d -> d.length >= 6 ? crashOn(true, "k") : (d.length > 0 ? crashOn(true, "other") : Outcome.pass());
    byte[] out = Fuzz.minimizeCrashingInput(new byte[] {1, 2, 3, 4, 5, 6}, sig, test, 1000);
    assertEquals(6, out.length);
    // A passing candidate is never accepted either.
    assertEquals(6, Fuzz.minimizeCrashingInput(new byte[6], sig, d -> d.length == 6 ? crashOn(true, "k") : Outcome.pass(), 1000).length);
  }

  @Test
  void everyCandidateExecutionCountsAgainstTheBudgetRejectedOnesIncluded() {
    String sig = Signature.crashSignature("k", List.of("f"));
    AtomicInteger runs = new AtomicInteger();
    // Nothing ever reproduces: every candidate is rejected.
    Function<byte[], Outcome> never =
        d -> {
          runs.incrementAndGet();
          return Outcome.pass();
        };
    byte[] out = Fuzz.minimizeCrashingInput(new byte[500], sig, never, 25);
    assertEquals(25, runs.get(), "exactly the budget, rejected attempts included");
    assertEquals(500, out.length);
    runs.set(0);
    byte[] nonzero = new byte[500];
    Arrays.fill(nonzero, (byte) 1);
    Fuzz.Minimized m = Fuzz.minimize(nonzero, sig, never, 600);
    assertTrue(runs.get() <= 600);
    assertEquals(runs.get(), m.spent());
    runs.set(0);
    assertFalse(Fuzz.minimize(nonzero, sig, never, 30).finished());
    assertEquals(30, runs.get());
    // A budget that runs out during pass 2 is also reported unfinished.
    runs.set(0);
    byte[] small = {1, 1};
    Fuzz.Minimized pass2 = Fuzz.minimize(small, sig, never, 3);
    assertFalse(pass2.finished());
    assertEquals(3, runs.get());
  }

  @Test
  void theMinimiserSkipsBytesThatAreAlreadyZeroAndValidatesItsArguments() {
    String sig = Signature.crashSignature("k", List.of("f"));
    AtomicInteger runs = new AtomicInteger();
    Fuzz.minimize(
        new byte[] {0, 0, 0},
        sig,
        d -> {
          runs.incrementAndGet();
          return Outcome.pass();
        },
        100);
    assertEquals(3, runs.get(), "only pass 1 runs for an all-zero input");
    assertThrows(IllegalArgumentException.class, () -> Fuzz.minimizeCrashingInput(new byte[1], sig, d -> Outcome.pass(), -1));
    assertThrows(IllegalArgumentException.class, () -> Fuzz.minimizeCrashingInput(new byte[1], sig, d -> Outcome.pass(), Options.MAX_MINIMIZE_BUDGET + 1));
    assertThrows(IllegalArgumentException.class, () -> Fuzz.minimizeCrashingInput(new byte[1], sig, null, 1));
    assertThrows(IllegalArgumentException.class, () -> Fuzz.minimizeCrashingInput(new byte[Mutate.MAX_INPUT_BYTES + 1], sig, d -> Outcome.pass(), 1));
  }

  // ---- campaigns -------------------------------------------------------------------------

  static Target staircase(boolean instrumented) {
    byte[] want = {(byte) 128, (byte) 255, 127, 1}; // boundary values: one step each, but all four at once is hopeless
    return (d, h) -> {
      for (int i = 0; i < want.length; i++) {
        if (d.length <= i || d[i] != want[i]) {
          return;
        }
        if (instrumented) {
          h.edge(Handle.edgeId("stair" + i));
        }
      }
      throw new IllegalStateException("cracked");
    };
  }

  @Test
  void coverageFeedbackCracksAStaircaseBlindMutationDoesNot() {
    int[] cracked = new int[2];
    for (boolean instrumented : new boolean[] {true, false}) {
      for (long seed = 0; seed < 8; seed++) {
        Report r = run(seeds(new byte[4]), opts(8000, seed).withMinimizeBudget(0), staircase(instrumented));
        if (!r.crashes().isEmpty()) {
          cracked[instrumented ? 0 : 1]++;
        }
      }
    }
    assertTrue(cracked[0] >= 4, "coverage guidance cracked only " + cracked[0] + " of 8 seeds");
    assertEquals(0, cracked[1], "blind mutation cracked " + cracked[1] + " of 8: the staircase is not a feedback test");
  }

  @Test
  void aCampaignIsDeterministicAndSeedSensitive() {
    Report a = run(seeds(new byte[4]), opts(2000, 3), staircase(true));
    Report b = run(seeds(new byte[4]), opts(2000, 3), staircase(true));
    assertEquals(a.toString(), b.toString());
    assertEquals(a.corpus().size(), b.corpus().size());
    for (int i = 0; i < a.corpus().size(); i++) {
      assertArrayEquals(a.corpus().get(i), b.corpus().get(i));
    }
    Report c = run(seeds(new byte[4]), opts(2000, 4), staircase(true));
    boolean differs = c.corpus().size() != a.corpus().size();
    for (int i = 0; !differs && i < a.corpus().size(); i++) {
      differs = !Arrays.equals(a.corpus().get(i), c.corpus().get(i));
    }
    assertTrue(differs, "a different seed must explore differently");
  }

  @Test
  void budgetLengthCapAndEmptyCorpus() {
    int[] longest = {0};
    List<Integer> firstLens = new ArrayList<>();
    Report r =
        run(
            List.of(),
            opts(500, 9).withMaxInputLen(6),
            (d, h) -> {
              if (firstLens.size() < 1) {
                firstLens.add(d.length);
                assertArrayEquals(new byte[] {0}, d);
              }
              longest[0] = Math.max(longest[0], d.length);
              h.edge(d.length > 0 ? (d[0] & 0xFF) : 1000);
            });
    assertEquals(500, r.iterations());
    assertTrue(longest[0] <= 6, "mutated candidates are capped: " + longest[0]);
    assertTrue(r.isClean());
    // Initial seeds run as given, even above the cap.
    int[] seen = {0};
    run(seeds(new byte[100]), opts(3, 1).withMaxInputLen(4), (d, h) -> seen[0] = Math.max(seen[0], d.length));
    assertEquals(100, seen[0]);
    // A zero cap makes every candidate empty.
    Report z = run(seeds(new byte[3]), opts(20, 1).withMaxInputLen(0), (d, h) -> { });
    assertEquals(20, z.iterations());
  }

  @Test
  void crashingSeedsAreNotMutatedAndNeverClean() {
    AtomicInteger runs = new AtomicInteger();
    Report r =
        run(
            seeds(new byte[] {1}, new byte[] {2}),
            opts(100, 1),
            (d, h) -> {
              runs.incrementAndGet();
              throw new IllegalStateException();
            });
    assertEquals(0, r.iterations());
    assertFalse(r.isClean());
    assertEquals(1, r.crashes().size());
    assertEquals(2, r.crashes().get(0).hits());
    for (Target t : new Target[] {(d, h) -> { for (;;) { h.tick(); } }, (d, h) -> h.reserve(Long.MAX_VALUE)}) {
      Report f = run(seeds(new byte[1]), opts(100, 1).withStepBudget(5), t);
      assertEquals(0, f.iterations());
      assertFalse(f.isClean());
    }
  }

  @Test
  void everyExecutionLandsInABucketAndIsCleanIsHonest() {
    // Pass-only: clean.
    Report clean = run(seeds(new byte[2]), opts(50, 1), (d, h) -> h.edge(1));
    assertTrue(clean.isClean());
    assertEquals(Report.TIER_REPORTED, clean.coverageTier());
    // Zero iterations are never clean.
    assertFalse(run(seeds(new byte[2]), opts(0, 1), (d, h) -> { }).isClean());
    // A timeout alone, an OOM alone, a crash alone each make it unclean.
    Target slow = (d, h) -> { if (d.length != 1 || d[0] != 0) { for (;;) { h.tick(); } } };
    Report t = run(seeds(new byte[] {0}), opts(200, 1).withStepBudget(10), slow);
    assertTrue(t.timeouts() > 0);
    assertFalse(t.isClean());
    Target greedy = (d, h) -> { if (d.length != 1 || d[0] != 0) { h.reserve(1 << 20); } };
    Report m = run(seeds(new byte[] {0}), opts(200, 1).withMemBudget(10), greedy);
    assertTrue(m.ooms() > 0);
    assertFalse(m.isClean());
    Target bad = (d, h) -> { if (d.length != 1 || d[0] != 0) { throw new IllegalStateException(); } };
    Report c = run(seeds(new byte[] {0}), opts(200, 1), bad);
    assertFalse(c.crashes().isEmpty());
    assertFalse(c.isClean());
    assertEquals(c.crashes().size(), c.uniqueCrashes());
  }

  @Test
  void dedupKeepsOneRecordPerSignatureWithCorrectHitsAndTheShorterInput() {
    ExecFuncs.Counter counter = new ExecFuncs.Counter();
    Fuzz.ExecFunc exec =
        (d, cov) -> {
          counter.runs++;
          return d.length == 0 ? Outcome.pass() : Outcome.crash("k", List.of("f"));
        };
    Report r = Fuzz.runOutcomes(seeds(new byte[] {5, 5, 5, 5}, new byte[] {6, 6}), opts(50, 1).withMinimizeBudget(0), exec);
    assertEquals(1, r.crashes().size());
    assertEquals(2, r.crashes().get(0).hits());
    assertEquals(2, r.crashes().get(0).minimalInput().length, "the shorter input is kept on a re-hit");
    assertFalse(r.crashes().get(0).minimized());
    // With minimisation the record reaches the empty-adjacent fixed point: here the empty input passes, so one byte.
    Report m = Fuzz.runOutcomes(seeds(new byte[] {5, 5, 5, 5}), opts(50, 1), exec);
    assertEquals(1, m.crashes().get(0).minimalInput().length);
    assertTrue(m.crashes().get(0).minimized());
  }

  @Test
  void aLongerReHitNeverReplacesTheShorterInput() {
    Fuzz.ExecFunc exec = (d, cov) -> Outcome.crash("k", List.of("f"));
    Report r = Fuzz.runOutcomes(seeds(new byte[] {1}, new byte[] {2, 2, 2}), opts(0, 1), exec);
    assertEquals(1, r.crashes().get(0).minimalInput().length);
    assertEquals(2, r.crashes().get(0).hits());
  }

  @Test
  void theCrashTableIsCappedAndCountsDroppedSignatures() {
    List<byte[]> many = new ArrayList<>();
    for (int i = 0; i < Fuzz.MAX_CRASHES + 25; i++) {
      many.add(new byte[] {(byte) (i >> 8), (byte) i});
    }
    Fuzz.ExecFunc exec = (d, cov) -> Outcome.crash("k" + ((d[0] & 0xFF) << 8 | (d[1] & 0xFF)), List.of("f"));
    Report r = Fuzz.runOutcomes(many, opts(10, 1).withMinimizeBudget(0), exec);
    assertEquals(Fuzz.MAX_CRASHES, r.crashes().size());
    assertEquals(25, r.crashesDropped());
    assertEquals(Fuzz.MAX_CRASHES + 25, r.uniqueCrashes());
    assertFalse(r.isClean());
    assertThrows(UnsupportedOperationException.class, () -> r.crashes().add(null));
  }

  @Test
  void coverageAdmissionGrowsTheCorpusAndTiersAreHonest() {
    Report r = run(seeds(new byte[] {0}), opts(500, 1), (d, h) -> { if (d.length > 0) { h.edge(d[0] & 0xFF); } });
    assertTrue(r.admittedSeeds() > 1);
    assertTrue(r.totalEdges() > 1);
    assertEquals(Report.TIER_REPORTED, r.coverageTier());
    Report blind = run(seeds(new byte[] {0}), opts(100, 1), (d, h) -> { });
    assertEquals(Report.TIER_BLACK_BOX, blind.coverageTier());
    assertEquals(1, blind.admittedSeeds(), "the first passing seed is admitted even with no coverage");
    assertEquals(0, blind.totalEdges());
    assertTrue(blind.isClean());
  }

  @Test
  void minimisationWorkIsBoundedByMaxIterationsAcrossTheCampaign() {
    AtomicInteger execs = new AtomicInteger();
    List<byte[]> many = new ArrayList<>();
    for (int i = 0; i < 100; i++) {
      byte[] s = new byte[100];
      Arrays.fill(s, (byte) (i + 1));
      many.add(s);
    }
    Fuzz.ExecFunc safe =
        (d, cov) -> {
          execs.incrementAndGet();
          return Outcome.crash("k" + (d.length == 0 ? 0 : d[0]), List.of("f"));
        };
    Report r = Fuzz.runOutcomes(many, opts(5, 1), safe);
    int probes = execs.get() - many.size();
    assertTrue(probes >= 1 && probes <= 5, probes + " minimisation probes for maxIterations 5");
    int unfinished = 0;
    for (CrashRecord c : r.crashes()) {
      if (!c.minimized()) {
        unfinished++;
      }
    }
    assertTrue(unfinished > 0);
    assertEquals(100, r.crashes().size());
    // maxIterations 0: nothing is minimised at all.
    execs.set(0);
    Report z = Fuzz.runOutcomes(many, opts(0, 1), safe);
    assertEquals(many.size(), execs.get());
    for (CrashRecord c : z.crashes()) {
      assertFalse(c.minimized());
    }
    // The per-crash budget caps one crash too.
    execs.set(0);
    Fuzz.runOutcomes(seeds(new byte[500]), opts(1000, 1).withMinimizeBudget(7), (d, cov) -> {
      execs.incrementAndGet();
      return Outcome.crash("k", List.of("f"));
    });
    assertTrue(execs.get() - 1 <= 7, execs.get() + " probes for minimizeBudget 7");
  }

  @Test
  void optionsAndArgumentsAreValidated() {
    Target ok = (d, h) -> { };
    List<byte[]> one = seeds(new byte[1]);
    Options d = Options.defaults();
    for (Options bad : new Options[] {
      d.withMaxIterations(-1), d.withMaxIterations(Mutate.MAX_ITERATIONS + 1),
      d.withMaxInputLen(-1), d.withMaxInputLen(Mutate.MAX_INPUT_BYTES + 1),
      d.withBaseEnergy(-1), d.withBaseEnergy(Mutate.MAX_ITERATIONS + 1),
      d.withMinimizeBudget(-1), d.withMinimizeBudget(Options.MAX_MINIMIZE_BUDGET + 1),
      d.withStepBudget(-1), d.withStepBudget(Options.MAX_STEP_BUDGET + 1),
      d.withMemBudget(-1), d.withMemBudget(Options.MAX_MEM_BUDGET + 1),
    }) {
      assertThrows(IllegalArgumentException.class, () -> run(one, bad, ok));
      assertThrows(IllegalArgumentException.class, () -> Fuzz.runOutcomes(one, bad, (x, y) -> Outcome.pass()));
    }
    d.withMaxIterations(Mutate.MAX_ITERATIONS).withMaxInputLen(Mutate.MAX_INPUT_BYTES).validate();
    assertThrows(IllegalArgumentException.class, () -> run(one, d, null));
    assertThrows(IllegalArgumentException.class, () -> Fuzz.runOutcomes(one, d, null));
    List<byte[]> tooMany = new ArrayList<>();
    for (int i = 0; i <= Fuzz.MAX_INITIAL_CORPUS; i++) {
      tooMany.add(new byte[0]);
    }
    assertThrows(IllegalArgumentException.class, () -> run(tooMany, d, ok));
    assertThrows(IllegalArgumentException.class, () -> run(seeds(new byte[Mutate.MAX_INPUT_BYTES + 1]), d, ok));
    assertThrows(IllegalArgumentException.class, () -> Fuzz.runOutcomes(one, d, (x, y) -> null));
    assertThrows(IllegalArgumentException.class, () -> Fuzz.runOutcomes(one, d, (x, y) -> new Outcome(null, "", List.of())));
    assertEquals(1000, d.maxIterations());
    assertEquals(42, d.seed());
    assertEquals(4096, d.maxInputLen());
    assertEquals(10, d.baseEnergy());
    assertEquals(2000, d.minimizeBudget());
    assertEquals(200_000, d.stepBudget());
    assertEquals(64 << 20, d.memBudget());
  }

  // ---- retained bytes cap --------------------------------------------------------------------

  @Test
  void theRetainedBytesCapRefusesAdmissionsAndCrashInputsVisibly() {
    // Every distinct first byte is a new edge, so everything would be admitted.
    Fuzz.ExecFunc cover = (d, cov) -> { cov.record(d.length == 0 ? 0 : (d[0] & 0xFF) + 1L); return Outcome.pass(); };
    Report r = Fuzz.runOutcomes(seeds(new byte[] {1, 2, 3, 4}), opts(300, 1), cover, 6);
    assertTrue(r.admissionsDropped() > 0);
    long total = 0;
    for (byte[] c : r.corpus()) {
      total += c.length;
    }
    assertTrue(total <= 6, "retained " + total);
    Report big = Fuzz.runOutcomes(seeds(new byte[] {1, 2, 3, 4}), opts(300, 1), cover, 1L << 20);
    assertEquals(0, big.admissionsDropped());
    // A crash whose input does not fit keeps no input and says so.
    Fuzz.ExecFunc crash = (d, cov) -> Outcome.crash("k", List.of("f"));
    Report c = Fuzz.runOutcomes(seeds(new byte[] {1, 2, 3, 4}), opts(0, 1), crash, 2);
    assertTrue(c.crashes().get(0).inputDropped());
    assertEquals(0, c.crashes().get(0).minimalInput().length);
    assertFalse(c.crashes().get(0).minimized());
    // A dropped record is not replaced by a later shorter input; a kept one is.
    Report d = Fuzz.runOutcomes(seeds(new byte[] {1, 2, 3, 4}, new byte[] {9}), opts(0, 1), crash, 2);
    assertEquals(2, d.crashes().get(0).hits());
    assertTrue(d.crashes().get(0).inputDropped());
  }

  // ---- privacy of every rendering -------------------------------------------------------------

  @Test
  void noRenderingOfAReportOrARecordShowsSeedsOrBytes() {
    byte[] secret = {(byte) 0xde, (byte) 0xad, (byte) 0xbe, (byte) 0xef, 0x13, 0x37};
    Report r =
        run(
            seeds(secret),
            opts(50, 0x5eedL).withMinimizeBudget(0),
            (d, h) -> {
              h.edge(d.length);
              if (d.length >= 6) {
                throw new IllegalStateException("payload " + Arrays.toString(d));
              }
            });
    assertFalse(r.crashes().isEmpty());
    List<String> renderings = new ArrayList<>();
    renderings.add(r.toString());
    renderings.add(String.valueOf(r));
    renderings.add(String.format("%s|%s", r, r.crashes()));
    for (CrashRecord c : r.crashes()) {
      renderings.add(c.toString());
      renderings.add(c.frames().toString());
    }
    renderings.addAll(Signature.tapFailures(r, "tgt", 1));
    for (String s : renderings) {
      assertFalse(s.contains("deadbeef"), s);
      assertFalse(s.contains("222"), s);
      assertFalse(s.contains("payload"), s);
      assertFalse(s.contains(String.valueOf(0x5eedL)), s);
      assertFalse(s.contains(Long.toString(0x5eedL, 16)), s);
      assertFalse(s.contains("[-34"), s);
    }
  }

  @Test
  void accessorsReturnCopies() {
    Report r = run(seeds(new byte[] {1, 2, 3}), opts(5, 1).withMinimizeBudget(0), (d, h) -> { if (d.length == 3 && d[0] == 1) { throw new IllegalStateException(); } });
    CrashRecord c = r.crashes().get(0);
    byte[] m = c.minimalInput();
    Arrays.fill(m, (byte) 99);
    assertArrayEquals(new byte[] {1, 2, 3}, c.minimalInput());
    r.corpus().forEach(x -> Arrays.fill(x, (byte) 7));
    assertEquals(1, r.crashes().size());
    assertEquals(1, r.seed());
    assertEquals("java.lang.IllegalStateException", c.kind());
    assertEquals(c.frames(), List.copyOf(c.frames()));
    assertNotNull(c.signature());
    assertEquals(8, c.signature().length());
  }
}
