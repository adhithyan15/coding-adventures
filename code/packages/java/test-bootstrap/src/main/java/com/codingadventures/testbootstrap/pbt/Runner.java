package com.codingadventures.testbootstrap.pbt;

import com.codingadventures.testbootstrap.corpusjson.Canon;
import com.codingadventures.testbootstrap.corpusjson.Clip;
import com.codingadventures.testbootstrap.prng.SplitMix64;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Predicate;

/**
 * The honest runner (VER05 section 7.1, VER05e section 5).
 *
 * <p>Verdict honesty:
 *
 * <ul>
 *   <li>zero cases evaluated is {@code property-exhausted}, never a pass;
 *   <li>a generator exception is {@code property-exhausted};
 *   <li>a property exception is {@code property-panic};
 *   <li>the reported minimal witness is re-verified: if the shrinker throws,
 *       returns something that no longer fails, a value the property throws on,
 *       or a nonsensical step count, the original candidate is reported with 0
 *       steps;
 *   <li>redacted mode never emits the witness or the seed.
 * </ul>
 */
public final class Runner {
  private Runner() {}

  /** Runs {@code prop} over {@code options.numTests()} candidates from {@code seed}. */
  public static <T> Verdict check(long seed, Gen<T> gen, Predicate<T> prop, Options options) {
    return check(seed, gen, prop, options, null);
  }

  /**
   * {@link #check(long, Gen, Predicate, Options)} with an explicit shrinker that
   * overrides the generator's own; it exists so a defective shrinker can be
   * tested. {@code prop} returns true when the property holds. Invalid options
   * throw {@link IllegalArgumentException} (no verdict is produced).
   */
  public static <T> Verdict check(
      long seed, Gen<T> gen, Predicate<T> prop, Options options, Shrinker<T> shrink) {
    Options o = options == null ? Options.defaults() : options;
    if (prop == null || gen == null) {
      throw new IllegalArgumentException("null property or generator");
    }
    if (o.numTests() < 0 || o.numTests() > Limits.MAX_CASES) {
      throw new IllegalArgumentException(
          "numTests " + o.numTests() + " outside [0, " + Limits.MAX_CASES + "]");
    }
    if (o.maxShrinkSteps() < 0 || o.maxShrinkSteps() > Limits.MAX_SHRINK_STEPS) {
      throw new IllegalArgumentException(
          "maxShrinkSteps " + o.maxShrinkSteps() + " outside [0, " + Limits.MAX_SHRINK_STEPS + "]");
    }
    boolean redact = o.ci() != CiMode.OFF;

    if (o.numTests() == 0) {
      return new Verdict(
          false,
          FailureIds.EXHAUSTED,
          "no cases were evaluated (numTests is 0); a property that never ran is not a pass",
          0,
          0,
          0,
          null,
          null,
          null,
          null);
    }

    SplitMix64 rng = Hooks.newRng.apply(seed);
    for (int iteration = 1; iteration <= o.numTests(); iteration++) {
      T candidate;
      try {
        candidate = gen.sample(rng);
      } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
        return exhausted(e, iteration, redact);
      }
      boolean holds;
      try {
        holds = prop.test(candidate);
      } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
        return panic(seed, iteration, e, redact);
      }
      if (!holds) {
        return falsified(seed, iteration, candidate, gen, prop, shrink, o, redact);
      }
    }
    return new Verdict(true, null, null, o.numTests(), 0, 0, null, null, null, null);
  }

  /** Text for a throwable that cannot itself throw while being rendered. */
  static String safeString(Object value) {
    try {
      return Clip.clip(String.valueOf(value), 200);
    } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
      return value.getClass().getName();
    }
  }

  private static Verdict exhausted(Throwable e, int iteration, boolean redact) {
    String text;
    if (e instanceof ExhaustedException) {
      text = "filter precondition rejected all candidates at iteration " + iteration;
    } else if (redact) {
      text = "generator error at iteration " + iteration + " (" + e.getClass().getName() + ")";
    } else {
      text = "generator error at iteration " + iteration + ": " + safeString(e);
    }
    return new Verdict(false, FailureIds.EXHAUSTED, text, iteration - 1, iteration, 0, null, null, null, null);
  }

  private static Verdict panic(long seed, int iteration, Throwable e, boolean redact) {
    if (redact) {
      return new Verdict(
          false,
          FailureIds.PANIC,
          "property evaluation threw at iteration " + iteration + " (" + e.getClass().getName() + ")",
          iteration,
          iteration,
          0,
          null,
          null,
          null,
          null);
    }
    return new Verdict(
        false,
        FailureIds.PANIC,
        "property evaluation threw at iteration " + iteration + ": " + safeString(e),
        iteration,
        iteration,
        0,
        seed,
        null,
        null,
        null);
  }

  private static <T> Verdict falsified(
      long seed,
      int iteration,
      T candidate,
      Gen<T> gen,
      Predicate<T> prop,
      Shrinker<T> shrink,
      Options o,
      boolean redact) {
    // A candidate the property throws on is not a counterexample to the claim;
    // it is a different failure, so it is never accepted as "smaller".
    Predicate<T> falsifies =
        v -> {
          try {
            return !prop.test(v);
          } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
            return false;
          }
        };
    // One evaluation budget for the whole shrink, however many shrinkers (tuple
    // components, nested lists) the generator composes; the re-verification below
    // uses the unbudgeted predicate.
    final long budget = Limits.attemptBudget(o.maxShrinkSteps());
    final AtomicLong spent = new AtomicLong();
    Predicate<T> budgeted =
        v -> {
          if (spent.getAndIncrement() >= budget) {
            return false;
          }
          return falsifies.test(v);
        };
    T minimal = candidate;
    int steps = 0;
    // Untrusted shrinker: fall back to the original. The isolated scope gives
    // this verdict its own work budget and restores the caller's on exit, so a
    // property that itself calls check gets the same inner verdict as alone.
    try (ShrinkBudget scope = ShrinkBudget.enterIsolated(o.maxShrinkSteps())) {
      Shrunk<T> shrunk =
          shrink != null
              ? shrink.shrink(candidate, budgeted, o.maxShrinkSteps())
              : gen.shrinkNested(candidate, budgeted, o.maxShrinkSteps());
      // Re-verify: the witness we report must really fail the property.
      if (shrunk.steps() >= 0 && shrunk.steps() <= o.maxShrinkSteps() && falsifies.test(shrunk.value())) {
        minimal = shrunk.value();
        steps = shrunk.steps();
      }
    } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
      // keep the original
    }

    if (redact) {
      String rendered;
      try {
        rendered = Canon.render(minimal);
      } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
        rendered = Canon.UNRENDERABLE; // constant preimage: a verdict is still returned
      }
      String preimage = "iter:" + iteration + ";steps:" + steps + ";shrunk:" + rendered;
      String digest = String.format("%016x", SplitMix64.fnv1a64(preimage));
      return new Verdict(
          false,
          FailureIds.COUNTEREXAMPLE,
          "property falsified after " + iteration + " iterations (shrunk in " + steps
              + " steps, repro-fnv: " + digest + ")",
          iteration,
          iteration,
          steps,
          null,
          null,
          null,
          digest);
    }
    return new Verdict(
        false,
        FailureIds.COUNTEREXAMPLE,
        "property falsified at iteration " + iteration + " (seed=" + Long.toUnsignedString(seed)
            + "): initial=" + Canon.render(candidate) + ", minimal=" + Canon.render(minimal)
            + " after " + steps + " shrink steps",
        iteration,
        iteration,
        steps,
        seed,
        candidate,
        minimal,
        null);
  }
}
