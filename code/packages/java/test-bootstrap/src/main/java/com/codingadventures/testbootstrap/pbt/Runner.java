package com.codingadventures.testbootstrap.pbt;

import com.codingadventures.testbootstrap.corpusjson.Canon;
import com.codingadventures.testbootstrap.corpusjson.Clip;
import com.codingadventures.testbootstrap.prng.SplitMix64;
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

    // A check nested in a property (an ambient scope exists) pays one attempt at
    // entry, one per case after the first and the size of every sampled collection, all charged up
    // the chain: so neither nesting depth, branching nor numTests multiplies the
    // outermost budget. A starved nested check reports property-exhausted without
    // evaluating anything (never a pass). A top-level check always evaluates its
    // first case.
    final ShrinkBudget outer = ShrinkBudget.current();
    final boolean nested = outer != null;
    if (nested && !outer.charge(1)) {
      return spent(0);
    }
    // The WHOLE check runs in one scope (case loop included): a root with no
    // enclosing budget, a child otherwise, so a nested check during generation is
    // charged to it too.
    try (ShrinkBudget scope = ShrinkBudget.enterCheck(o.numTests(), o.maxShrinkSteps())) {
      SplitMix64 rng = Hooks.newRng.apply(seed);
      for (int iteration = 1; iteration <= o.numTests(); iteration++) {
        if (nested && iteration > 1 && !scope.charge(1)) {
          return spent(iteration);
        }
        T candidate;
        try {
          candidate = gen.sample(rng);
        } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
          return exhausted(e, iteration, redact);
        }
        if (nested && !scope.chargeCopy(sizeOf(candidate))) {
          return spent(iteration);
        }
        boolean holds;
        try {
          holds = prop.test(candidate);
        } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
          return panic(seed, iteration, e, redact);
        }
        if (!holds) {
          return falsified(seed, iteration, candidate, gen, prop, shrink, o, redact, scope);
        }
      }
      if (!nested && scope.wasRefused()) {
        // A nested check or generator inside this run was starved, so the property
        // was not fully evaluated: never report that as a pass.
        return new Verdict(
            false,
            FailureIds.EXHAUSTED,
            "a nested check was starved by the shrink budget; the property was not fully evaluated",
            o.numTests(),
            o.numTests(),
            0,
            null,
            null,
            null,
            null);
      }
      return new Verdict(true, null, null, o.numTests(), 0, 0, null, null, null, null);
    }
  }

  /** The enclosing shrink budget was spent: exhausted, never a pass. */
  private static Verdict spent(int iteration) {
    return new Verdict(
        false,
        FailureIds.EXHAUSTED,
        "the enclosing shrink budget was spent at iteration " + iteration,
        Math.max(0, iteration - 1),
        iteration,
        0,
        null,
        null,
        null,
        null);
  }

  /** The most elements the work estimate of one sampled value will visit. */
  private static final int WALK_CAP = 10_000;

  /**
   * An approximate size of a sampled value in elements (work): a collection or array
   * counts its length plus, one level deeper, the length of each element that is
   * itself a collection, array or string (visiting at most {@link #WALK_CAP}
   * elements); a string counts its length. Generation work is approximate by design.
   */
  private static long sizeOf(Object v) {
    try {
      return sizeOfUnchecked(v);
    } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
      return 0; // a hostile collection: the estimate is best effort
    }
  }

  private static long sizeOfUnchecked(Object v) {
    long n = shallow(v);
    if (n == 0) {
      return 0;
    }
    long total = n;
    int visited = 0;
    if (v instanceof java.util.Collection<?> c) {
      for (Object e : c) {
        if (visited++ >= WALK_CAP) {
          break;
        }
        total += shallow(e);
      }
    } else if (v instanceof Object[] arr) {
      for (Object e : arr) {
        if (visited++ >= WALK_CAP) {
          break;
        }
        total += shallow(e);
      }
    }
    return total;
  }

  private static long shallow(Object v) {
    if (v instanceof java.util.Collection<?> c) {
      return c.size();
    }
    if (v instanceof CharSequence cs) {
      return cs.length();
    }
    if (v != null && v.getClass().isArray()) {
      return java.lang.reflect.Array.getLength(v);
    }
    return 0;
  }

  /** {@link Canon#render} that cannot throw: a verdict is still returned. */
  private static String render(Object value) {
    try {
      return Canon.render(value);
    } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
      return Canon.UNRENDERABLE;
    }
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
      boolean redact,
      ShrinkBudget checkScope) {
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
    T minimal = candidate;
    int steps = 0;
    // Untrusted shrinker: fall back to the original. The shrink scope is a child
    // of the check scope (itself a child of any enclosing shrink): every evaluation
    // is charged to the whole chain, so nothing a property or shrinker nests can
    // exceed the outermost budget.
    try {
      Shrunk<T> shrunk;
      try (ShrinkBudget scope = ShrinkBudget.enterChild(o.maxShrinkSteps())) {
        Predicate<T> budgeted = scope.charged(falsifies);
        shrunk =
            shrink != null
                ? shrink.shrink(candidate, budgeted, o.maxShrinkSteps())
                : gen.shrinkNested(candidate, budgeted, o.maxShrinkSteps());
      }
      // Re-verify: the witness we report must really fail the property. One
      // attempt is charged to the check scope (not the spent shrink scope); if the
      // chain refuses it the shrunk value is not adopted: the original witness is
      // reported with 0 steps.
      if (checkScope.charge(1)
          && shrunk.steps() >= 0
          && shrunk.steps() <= o.maxShrinkSteps()
          && falsifies.test(shrunk.value())) {
        minimal = shrunk.value();
        steps = shrunk.steps();
      }
    } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
      // keep the original
    }

    if (redact) {
      String rendered = render(minimal); // constant preimage when unrenderable
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
            + "): initial=" + render(candidate) + ", minimal=" + render(minimal)
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
