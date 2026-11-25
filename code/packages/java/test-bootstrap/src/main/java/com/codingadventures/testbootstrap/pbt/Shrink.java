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
   * <p>When the shared budget is already spent the input is returned as is,
   * uncopied: the returned list may alias {@code initial}.
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
    if (maxSteps == 0) {
      return new Shrunk<>(initial, 0); // no shrinking, no copy: may alias the input
    }
    // One budget for the whole shrink, shared with every nested shrinker (a list
    // of lists, a tuple of lists): attempts AND elements copied. The initial copy
    // is charged too; once the budget is spent the input is returned as is,
    // without copying (VER05e section 4.1).
    try (ShrinkBudget budget = ShrinkBudget.enter(maxSteps)) {
      if (!budget.chargeCopy(initial.size())) {
        return new Shrunk<>(initial, 0);
      }
      return sequence(budget, initial, falsifies, maxSteps, minLen, shrinkElem);
    }
  }

  private static <E> Shrunk<List<E>> sequence(
      ShrinkBudget budget,
      List<E> initial,
      Predicate<List<E>> falsifies,
      int maxSteps,
      int minLen,
      Shrinker<E> shrinkElem) {
    final List<E> current = copy(initial);
    int steps = 0;

    // Each candidate is built fresh (after its charge) and handed straight to the
    // property, with no second copy. The property may scribble on it, so an
    // ACCEPTED move is applied to `current` in place from the known move, never
    // by reusing the candidate.
    if (current.size() > minLen && budget.charge(minLen)) {
      if (falsifies.test(new ArrayList<>(current.subList(0, minLen)))) {
        if (minLen == 0) {
          return new Shrunk<>(new ArrayList<>(), 1);
        }
        current.subList(minLen, current.size()).clear();
        steps = 1;
      }
    }
    int floor = Math.max(1, minLen);

    // Phase 1: bisection (left half first, then right half).
    while (current.size() > floor && steps < maxSteps) {
      int mid = current.size() / 2;
      if (mid >= minLen
          && budget.charge(mid)
          && falsifies.test(new ArrayList<>(current.subList(0, mid)))) {
        current.subList(mid, current.size()).clear();
        steps++;
        continue;
      }
      int rightLen = current.size() - mid;
      if (rightLen >= minLen
          && budget.charge(rightLen)
          && falsifies.test(new ArrayList<>(current.subList(mid, current.size())))) {
        current.subList(0, mid).clear();
        steps++;
        continue;
      }
      break;
    }

    // Phase 2: single-element deletion, right to left.
    if (!Hooks.skipDeletionPhase) {
      int i = current.size() - 1;
      while (i >= 0 && current.size() > floor && steps < maxSteps && budget.hasRoom()) {
        // Charge BEFORE copying: a spent budget must not cost another copy.
        if (!budget.charge(current.size() - 1)) {
          break;
        }
        List<E> cand = copy(current);
        cand.remove(i);
        if (falsifies.test(cand)) {
          current.remove(i);
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
        if (remaining <= 0 || !budget.hasRoom()) {
          break;
        }
        final int at = index;
        Predicate<E> withElement =
            candidate -> {
              if (!budget.charge(current.size())) {
                return false;
              }
              List<E> probe = copy(current);
              probe.set(at, candidate);
              return falsifies.test(probe);
            };
        Shrunk<E> r = shrinkElem.shrink(current.get(index), withElement, remaining);
        if (r.steps() > 0) {
          current.set(index, r.value());
          steps += r.steps();
        }
      }
    }
    return new Shrunk<>(current, steps);
  }

  static void checkBudget(int maxSteps) {
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
    try (ShrinkBudget budget = ShrinkBudget.enterChild(maxSteps)) {
      if (!budget.charge(1)) {
        return new Shrunk<>(initial, 0); // starved by an enclosing shrink
      }
      if (prop.test(initial)) {
        throw new NotFalsifyingException();
      }
      return shrinkIntCore(initial, target, lo, hi, v -> budget.charge(1) && !prop.test(v), maxSteps);
    }
  }

  /**
   * Shrinks a falsifying list of integers: empty probe, halves, single deletion,
   * then each element toward {@code elemTarget} (unbounded). {@code prop} is true
   * when the list satisfies the property. When the shared budget is already spent
   * the returned list may alias {@code initial}. The property receives copies, so one
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
    // Public entry point: a child scope, charged before the first copies (the
    // working copy and the copy the first property call receives) and for the
    // initial property evaluation.
    try (ShrinkBudget budget = ShrinkBudget.enterChild(maxSteps)) {
      if (!budget.chargeCopy(2L * initial.size()) || !budget.charge(1)) {
        return new Shrunk<>(initial, 0);
      }
      List<BigInteger> items = copy(initial);
      if (prop.test(copy(items))) {
        throw new NotFalsifyingException();
      }
      Shrinker<BigInteger> walk =
          (value, falsifies, steps) -> intWalk(value, elemTarget, null, null, falsifies, steps);
      // A candidate is already a fresh list, so the property may scribble on it.
      return shrinkSequence(items, c -> !prop.test(c), maxSteps, 0, walk);
    }
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
    try (ShrinkBudget budget = ShrinkBudget.enterChild(maxSteps)) {
      // The clone, the list conversion and the array conversion back.
      if (!budget.chargeCopy(3L * initial.length) || !budget.charge(1)) {
        return new Shrunk<>(initial, 0);
      }
      if (prop.test(initial.clone())) {
        throw new NotFalsifyingException();
      }
      // Each attempt also converts its candidate back to an array: charge that
      // copy (candidate size) on top of the attempt's own charge.
      Shrunk<List<Integer>> r =
          shrinkSequence(
              toList(initial),
              c -> budget.chargeCopy(c.size()) && !prop.test(toArray(c)),
              maxSteps,
              0,
              BYTE_WALK);
      return new Shrunk<>(toArray(r.value()), r.steps());
    }
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
    try (ShrinkBudget budget = ShrinkBudget.enterChild(1000)) {
      SplitMix64 rng = Hooks.newRng.apply(seed);
      BigInteger initialFail = null;
      for (int i = 0; i < 1000 && budget.charge(1); i++) {
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
}
