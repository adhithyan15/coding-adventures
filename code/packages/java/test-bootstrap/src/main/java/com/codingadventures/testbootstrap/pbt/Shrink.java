package com.codingadventures.testbootstrap.pbt;

import com.codingadventures.testbootstrap.prng.SplitMix64;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.List;
import java.util.function.Predicate;

/**
 * Integrated shrinking (VER05e section 4, VER05 section 4).
 *
 * <p>A property that fails on {@code [913, -4, 77, 0, 2]} has told you
 * nothing; one that fails on {@code [0]} has told you what is wrong. Shrinking
 * walks from the first failing candidate toward the smallest one that still
 * fails. It is "integrated" because the generator that made the value knows its
 * own bounds, so a shrunk {@code integer(5, 10)} never becomes 0.
 *
 * <p>All shrinkers take a predicate that returns {@code true} when the property
 * <em>holds</em> (the corpus convention) and return the minimal value together
 * with the number of <em>accepted</em> moves.
 */
public final class Shrink {
  private Shrink() {}

  private static final BigInteger ONE = BigInteger.ONE;

  private static boolean inBounds(BigInteger v, BigInteger lo, BigInteger hi) {
    return (lo == null || lo.compareTo(v) <= 0) && (hi == null || v.compareTo(hi) <= 0);
  }

  /**
   * Bisects then unit-steps {@code current} toward {@code target}.
   *
   * <p>Each turn tries, in order,
   *
   * <ol>
   *   <li>the bisection candidate {@code current - floor(diff/2)}, diff = current -
   *       target, when |diff| &gt; 1;
   *   <li>the unit candidate {@code current -/+ 1} toward the target.
   * </ol>
   *
   * A candidate is accepted only if it is inside [lo, hi] (null means
   * unbounded), differs from current and still falsifies. Only accepted moves
   * count as steps. No acceptable candidate ends the walk.
   *
   * <p>Worked example (corpus pbt/shrink_int/54321_le_100, property x &lt;= 100):
   * 54321 -&gt; 27161 -&gt; 13581 -&gt; ... -&gt; 101, 15 accepted moves. 101 is the
   * smallest value that still fails; 100 would satisfy the property, so neither
   * the bisection nor the unit candidate from 101 is accepted.
   */
  static Shrunk<BigInteger> intWalk(
      BigInteger current,
      BigInteger target,
      BigInteger lo,
      BigInteger hi,
      Predicate<BigInteger> falsifies,
      int budget) {
    int steps = 0;
    while (steps < budget && current.compareTo(target) != 0) {
      BigInteger diff = current.subtract(target);
      boolean moved = false;
      if (diff.abs().compareTo(ONE) > 0) {
        BigInteger cand = Hooks.bisectCandidate.apply(current, diff);
        if (cand.compareTo(current) != 0 && inBounds(cand, lo, hi) && falsifies.test(cand)) {
          current = cand;
          steps++;
          moved = true;
        }
      }
      if (!moved) {
        BigInteger cand = Hooks.unitCandidate.apply(current, diff);
        if (cand.compareTo(current) != 0 && inBounds(cand, lo, hi) && falsifies.test(cand)) {
          current = cand;
          steps++;
          moved = true;
        }
      }
      if (!moved) {
        break;
      }
    }
    return new Shrunk<>(current, steps);
  }

  /** The target probe followed by the walk (VER05 section 4.2). */
  static Shrunk<BigInteger> shrinkIntCore(
      BigInteger initial,
      BigInteger target,
      BigInteger lo,
      BigInteger hi,
      Predicate<BigInteger> falsifies,
      int maxSteps) {
    if (maxSteps == 0) {
      return new Shrunk<>(initial, 0); // no budget, no shrinking
    }
    if (inBounds(target, lo, hi) && target.compareTo(initial) != 0 && falsifies.test(target)) {
      return new Shrunk<>(target, 1); // the target itself fails: cannot do better
    }
    return intWalk(initial, target, lo, hi, falsifies, maxSteps);
  }

  /** A byte value (held as an Integer 0..255) shrunk toward 0 by the integer walk. */
  static final Shrinker<Integer> BYTE_WALK =
      (value, falsifies, budget) -> {
        Shrunk<BigInteger> r =
            intWalk(
                BigInteger.valueOf(value),
                BigInteger.ZERO,
                BigInteger.ZERO,
                BigInteger.valueOf(255),
                b -> falsifies.test(b.intValue()),
                budget);
        return new Shrunk<>(r.value().intValue(), r.steps());
      };

  private static <E> List<E> copy(List<E> in) {
    return new ArrayList<>(in);
  }

  /**
   * Shrinks a falsifying sequence (VER05 section 4.3). {@code falsifies} always
   * receives a fresh list it may keep or mutate.
   *
   * <ol start="0">
   *   <li><b>probe.</b> minLen == 0: does the empty list fail? Then it is the
   *       answer, one step. (minLen &gt; 0: probe the length-minLen prefix and
   *       carry on from it.)
   *   <li><b>bisection:</b> keep the left half, else the right half, while one of
   *       them still fails.
   *   <li><b>single deletion,</b> right to left.
   *   <li><b>element-wise,</b> left to right, via {@code shrinkElem} (null means
   *       none).
   * </ol>
   *
   * <p>Worked example (corpus list_sum_le_20, initial [10 5 8 4]): the empty list
   * satisfies the property; both halves ([10 5], [8 4]) satisfy it too; deleting
   * index 3 leaves [10 5 8] (sum 23, still failing); element-wise 10 -&gt; 8 gives
   * the 1-minimal [8 5 8].
   */
  static <E> Shrunk<List<E>> shrinkSequence(
      List<E> initial,
      Predicate<List<E>> falsifies,
      int maxSteps,
      int minLen,
      Shrinker<E> shrinkElem) {
    List<E> current = copy(initial);
    int steps = 0;
    if (maxSteps == 0) {
      return new Shrunk<>(current, 0);
    }
    // Every attempt, accepted or rejected, is charged against one budget: a
    // rejected attempt copies the list but is not a step, so without this an
    // n-element list costs O(n^2) work whatever maxSteps says.
    final long budget = (long) Limits.ATTEMPT_FACTOR * maxSteps + Limits.ATTEMPT_SLACK;
    final long[] attempts = {0};
    Predicate<List<E>> attempt =
        cand -> {
          if (attempts[0] >= budget) {
            return false; // exhausted: nothing more is accepted, every loop ends
          }
          attempts[0]++;
          return falsifies.test(copy(cand));
        };

    if (current.size() > minLen) {
      List<E> probe = copy(current.subList(0, minLen));
      if (attempt.test(probe)) {
        if (minLen == 0) {
          return new Shrunk<>(probe, 1);
        }
        current = probe;
        steps = 1;
      }
    }
    int floor = Math.max(1, minLen);

    // Phase 1: bisection.
    while (current.size() > floor && steps < maxSteps) {
      int mid = current.size() / 2;
      List<E> left = copy(current.subList(0, mid));
      if (left.size() >= minLen && attempt.test(left)) {
        current = left;
        steps++;
        continue;
      }
      List<E> right = copy(current.subList(mid, current.size()));
      if (right.size() >= minLen && attempt.test(right)) {
        current = right;
        steps++;
        continue;
      }
      break;
    }

    // Phase 2: single-element deletion, right to left.
    if (!Hooks.skipDeletionPhase) {
      int i = current.size() - 1;
      while (i >= 0 && current.size() > floor && steps < maxSteps && attempts[0] < budget) {
        List<E> cand = copy(current);
        cand.remove(i);
        if (attempt.test(cand)) {
          current = cand;
          steps++;
          i = Math.min(i, current.size() - 1);
        } else {
          i--;
        }
      }
    }

    // Phase 3: element-wise, left to right.
    if (shrinkElem != null) {
      int size = current.size();
      for (int index = 0; index < size; index++) {
        int remaining = maxSteps - steps;
        if (remaining <= 0 || attempts[0] >= budget) {
          break;
        }
        final List<E> base = current;
        final int at = index;
        Predicate<E> withElement =
            candidate -> {
              if (attempts[0] >= budget) {
                return false;
              }
              attempts[0]++;
              List<E> probe = copy(base);
              probe.set(at, candidate);
              return falsifies.test(probe);
            };
        Shrunk<E> r = shrinkElem.shrink(current.get(index), withElement, remaining);
        if (r.steps() > 0) {
          current = copy(current);
          current.set(index, r.value());
          steps += r.steps();
        }
      }
    }
    return new Shrunk<>(current, steps);
  }

  private static void checkBudget(int maxSteps) {
    if (maxSteps < 0 || maxSteps > Limits.MAX_SHRINK_STEPS) {
      throw new IllegalArgumentException(
          "maxSteps " + maxSteps + " outside [0, " + Limits.MAX_SHRINK_STEPS + "]");
    }
  }

  /**
   * Shrinks a falsifying integer toward {@code target} inside [lo, hi] (the
   * corpus calls these min and max).
   *
   * <p>{@code prop.test(x)} is true when the property holds. Steps counts
   * accepted moves only and the target probe is one move. Throws
   * {@link NotFalsifyingException} when {@code initial} satisfies {@code prop},
   * and {@link IllegalArgumentException} for a null argument, lo &gt; hi, an
   * initial outside the bounds, or a budget outside [0, MAX_SHRINK_STEPS]. The
   * result is never outside [lo, hi].
   */
  public static Shrunk<BigInteger> shrinkInt(
      BigInteger initial,
      BigInteger target,
      BigInteger lo,
      BigInteger hi,
      Predicate<BigInteger> prop,
      int maxSteps) {
    if (initial == null || target == null || lo == null || hi == null || prop == null) {
      throw new IllegalArgumentException("null argument");
    }
    checkBudget(maxSteps);
    if (lo.compareTo(hi) > 0) {
      throw new IllegalArgumentException("min exceeds max");
    }
    if (!inBounds(initial, lo, hi)) {
      throw new IllegalArgumentException("initial value must lie within [min, max]");
    }
    if (prop.test(initial)) {
      throw new NotFalsifyingException();
    }
    return shrinkIntCore(initial, target, lo, hi, v -> !prop.test(v), maxSteps);
  }

  /**
   * Shrinks a falsifying list of integers: empty probe, halves, single deletion,
   * then each element toward {@code elemTarget} (unbounded). {@code prop} is true
   * when the list satisfies the property. The property receives copies, so one
   * that scribbles on its argument cannot corrupt the shrinker's state.
   */
  public static Shrunk<List<BigInteger>> shrinkList(
      List<BigInteger> initial,
      BigInteger elemTarget,
      Predicate<List<BigInteger>> prop,
      int maxSteps) {
    if (initial == null || elemTarget == null || prop == null) {
      throw new IllegalArgumentException("null argument");
    }
    checkBudget(maxSteps);
    for (int i = 0; i < initial.size(); i++) {
      if (initial.get(i) == null) {
        throw new IllegalArgumentException("null element at index " + i);
      }
    }
    Predicate<List<BigInteger>> holds = c -> prop.test(copy(c));
    List<BigInteger> items = copy(initial);
    if (holds.test(items)) {
      throw new NotFalsifyingException();
    }
    Shrinker<BigInteger> walk =
        (value, falsifies, budget) -> intWalk(value, elemTarget, null, null, falsifies, budget);
    return shrinkSequence(items, c -> !holds.test(c), maxSteps, 0, walk);
  }

  /**
   * Shrinks falsifying bytes: sequence shrinking over byte values toward 0x00.
   * {@code prop} is true when the bytes satisfy the property.
   */
  public static Shrunk<byte[]> shrinkBytes(byte[] initial, Predicate<byte[]> prop, int maxSteps) {
    if (initial == null || prop == null) {
      throw new IllegalArgumentException("null argument");
    }
    checkBudget(maxSteps);
    if (prop.test(initial.clone())) {
      throw new NotFalsifyingException();
    }
    Shrunk<List<Integer>> r =
        shrinkSequence(
            toList(initial), c -> !prop.test(toArray(c)), maxSteps, 0, BYTE_WALK);
    return new Shrunk<>(toArray(r.value()), r.steps());
  }

  static List<Integer> toList(byte[] in) {
    List<Integer> out = new ArrayList<>(in.length);
    for (byte b : in) {
      out.add(b & 0xff);
    }
    return out;
  }

  static byte[] toArray(List<Integer> in) {
    byte[] out = new byte[in.size()];
    for (int i = 0; i < out.length; i++) {
      out[i] = (byte) (int) in.get(i);
    }
    return out;
  }

  /** The first value and shrunk value of {@link #simulateReplay}. */
  public record Replay(BigInteger initialFail, BigInteger minimal) {}

  /**
   * Replays a failure from nothing but {@code seed}.
   *
   * <p>It draws up to 1000 values {@code bounded(1000)}; the first that
   * falsifies {@code prop} is the initial failure; that value is then shrunk
   * toward 0 over [0, 1000]. The same seed gives the same pair forever.
   * {@code prop} is true when the value is fine. Throws
   * {@link IllegalArgumentException} if no draw falsified {@code prop}.
   */
  public static Replay simulateReplay(long seed, Predicate<BigInteger> prop) {
    SplitMix64 rng = Hooks.newRng.apply(seed);
    BigInteger initialFail = null;
    for (int i = 0; i < 1000; i++) {
      BigInteger v = SplitMix64.unsigned(rng.bounded(1000));
      if (!prop.test(v)) {
        initialFail = v;
        break;
      }
    }
    if (initialFail == null) {
      throw new IllegalArgumentException("property was not falsified within 1000 draws");
    }
    Shrunk<BigInteger> r =
        shrinkInt(initialFail, BigInteger.ZERO, BigInteger.ZERO, BigInteger.valueOf(1000), prop, 1000);
    return new Replay(initialFail, r.value());
  }
}
