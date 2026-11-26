package com.codingadventures.testbootstrap.pbt;

import com.codingadventures.testbootstrap.prng.SplitMix64;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Function;
import java.util.function.Predicate;

/**
 * The generator algebra (VER05 section 3, VER05e section 3).
 *
 * <p>Draw order is the compatibility contract between language lanes: the
 * pbt-v1 vectors pin exact outputs per seed, so reordering a draw breaks
 * conformance.
 *
 * <p>Integers are {@link BigInteger} wherever the corpus or the full 128-bit
 * range is involved (Java has no 128-bit integer).
 */
public final class Gens {
  private Gens() {}

  private static final BigInteger ONE = BigInteger.ONE;
  private static final BigInteger MIN_I128 = ONE.shiftLeft(127).negate();
  private static final BigInteger MAX_I128 = ONE.shiftLeft(127).subtract(ONE);
  private static final BigInteger TWO_127 = ONE.shiftLeft(127);
  private static final BigInteger TWO_128 = ONE.shiftLeft(128);
  private static final BigInteger MAX_U64 = ONE.shiftLeft(64).subtract(ONE);

  // ---------------------------------------------------------------------------
  // Primitive generators
  // ---------------------------------------------------------------------------

  /** Always generates {@code v} and draws nothing. */
  public static <T> Gen<T> constant(T v) {
    return Gen.of(rng -> v, null);
  }

  /** {@code bounded(2) == 1}. Shrinks true to false. */
  public static Gen<Boolean> bool() {
    return Gen.of(
        rng -> rng.bounded(2) == 1,
        (value, falsifies, maxSteps) ->
            value && maxSteps > 0 && falsifies.test(false)
                ? new Shrunk<>(false, 1)
                : new Shrunk<>(value, 0));
  }

  /** Adapts the big-integer walk to a native integer type. */
  private static <T> Shrinker<T> bigShrinker(
      BigInteger lo, BigInteger hi, Function<T, BigInteger> toBig, Function<BigInteger, T> fromBig) {
    BigInteger target = clampZero(lo, hi);
    return (value, falsifies, maxSteps) -> {
      Shrunk<BigInteger> r =
          Shrink.shrinkIntCore(toBig.apply(value), target, lo, hi, b -> falsifies.test(fromBig.apply(b)), maxSteps);
      return new Shrunk<>(fromBig.apply(r.value()), r.steps());
    };
  }

  /** The in-range point nearest zero: the integer shrink target. */
  static BigInteger clampZero(BigInteger lo, BigInteger hi) {
    if (lo.signum() > 0) {
      return lo;
    }
    if (hi.signum() < 0) {
      return hi;
    }
    return BigInteger.ZERO;
  }

  /** [0, 255]: {@code bounded(256)}. */
  public static Gen<Integer> u8() {
    return Gen.of(
        rng -> (int) rng.bounded(256),
        bigShrinker(BigInteger.ZERO, BigInteger.valueOf(255), (Integer i) -> BigInteger.valueOf(i), BigInteger::intValue));
  }

  /** A full 64-bit word (as a non-negative BigInteger): {@code nextU64()}. */
  public static Gen<BigInteger> u64() {
    return Gen.of(
        rng -> SplitMix64.unsigned(rng.nextU64()),
        bigShrinker(BigInteger.ZERO, MAX_U64, b -> b, b -> b));
  }

  static BigInteger sampleI128(SplitMix64 rng) {
    BigInteger hi = SplitMix64.unsigned(rng.nextU64());
    BigInteger lo = SplitMix64.unsigned(rng.nextU64());
    BigInteger v = hi.shiftLeft(64).or(lo);
    if (v.compareTo(TWO_127) >= 0) {
      v = v.subtract(TWO_128);
    }
    return v;
  }

  /**
   * A signed 128-bit integer from two stream words, high word first, read as
   * two's complement:
   *
   * <pre>
   *   words 0x8000...0 : 0   -&gt; -2^127   (top bit set =&gt; subtract 2^128)
   *   words 0x7fff...f : f.. -&gt;  2^127 - 1
   * </pre>
   */
  public static Gen<BigInteger> i128() {
    return Gen.of(Gens::sampleI128, bigShrinker(MIN_I128, MAX_I128, b -> b, b -> b));
  }

  /**
   * A uniform integer over the inclusive range [min, max], any width.
   *
   * <p>The width is span = max - min + 1 and the offset from min is drawn by
   * {@link SplitMix64#boundedBig} (VER05 section 7.2). It shrinks toward the
   * in-range point nearest zero and never leaves [min, max].
   */
  public static Gen<BigInteger> integer(BigInteger min, BigInteger max) {
    if (min == null || max == null || min.compareTo(max) > 0) {
      throw new IllegalArgumentException("integer needs non-null min <= max");
    }
    return Gen.of(rng -> boundedInt(rng, min, max), bigShrinker(min, max, b -> b, b -> b));
  }

  static BigInteger boundedInt(SplitMix64 rng, BigInteger lo, BigInteger hi) {
    BigInteger span = hi.subtract(lo).add(ONE);
    return lo.add(rng.boundedBig(span));
  }

  /** {@link #integer} over native long bounds: the same draws, a native result. */
  public static Gen<Long> int64(long min, long max) {
    if (min > max) {
      throw new IllegalArgumentException("int64 needs min <= max");
    }
    // Long arithmetic wraps, so a full-range span is 0 (meaning 2^64).
    long span = max - min + 1;
    return Gen.of(
        rng -> {
          long offset = span == 0 ? rng.nextU64() : rng.bounded(span);
          return min + offset;
        },
        bigShrinker(BigInteger.valueOf(min), BigInteger.valueOf(max), BigInteger::valueOf, BigInteger::longValue));
  }

  private static void checkLengths(int minLen, int maxLen, int limit, String what) {
    if (minLen < 0 || maxLen < minLen) {
      throw new IllegalArgumentException(what + " needs 0 <= minLen <= maxLen");
    }
    if (maxLen > limit) {
      throw new IllegalArgumentException(what + " maxLen " + maxLen + " exceeds " + limit);
    }
  }

  /**
   * A byte string: {@code L = minLen + bounded(span)}, then L draws of
   * {@code bounded(256)}. Shrinks by sequence shrinking, respecting minLen.
   */
  public static Gen<byte[]> bytes(int minLen, int maxLen) {
    checkLengths(minLen, maxLen, Limits.MAX_INPUT_BYTES, "bytes");
    long span = (long) (maxLen - minLen) + 1;
    return Gen.of(
        rng -> {
          int n = minLen + (int) rng.bounded(span);
          byte[] out = new byte[n];
          for (int i = 0; i < n; i++) {
            out[i] = (byte) rng.bounded(256);
          }
          return out;
        },
        (value, falsifies, maxSteps) -> {
          // The list conversion (and the array one on the way out) are copies
          // too: charge them first, and do nothing once the budget is spent.
          try (ShrinkBudget budget = ShrinkBudget.enter(maxSteps)) {
            if (maxSteps == 0 || !budget.chargeCopy(2L * value.length)) {
              return new Shrunk<>(value, 0);
            }
            Shrunk<List<Integer>> r =
                Shrink.shrinkSequence(
                    Shrink.toList(value),
                    c -> budget.chargeCopy(c.size()) && falsifies.test(Shrink.toArray(c)),
                    maxSteps,
                    minLen,
                    Shrink.BYTE_WALK);
            return new Shrunk<>(Shrink.toArray(r.value()), r.steps());
          }
        });
  }

  // ---------------------------------------------------------------------------
  // Combinators
  // ---------------------------------------------------------------------------

  /** Transforms generated values. Drops shrinking: {@code f} has no inverse. */
  public static <T, U> Gen<U> map(Gen<T> g, Function<T, U> f) {
    return Gen.of(rng -> f.apply(g.sample(rng)), null);
  }

  /** {@link #filter(Gen, Predicate, int)} with the default retry count (100). */
  public static <T> Gen<T> filter(Gen<T> g, Predicate<T> pred) {
    return filter(g, pred, Limits.DEFAULT_MAX_RETRIES);
  }

  /**
   * Keeps only values satisfying {@code pred}. After exactly {@code maxRetries}
   * rejected samples sampling throws {@link ExhaustedException}. Shrinking is
   * preserved: a shrink candidate that fails {@code pred} is simply not a valid
   * input, so it is treated as "does not falsify".
   */
  public static <T> Gen<T> filter(Gen<T> g, Predicate<T> pred, int maxRetries) {
    if (maxRetries < 1 || maxRetries > Limits.MAX_FILTER_RETRIES) {
      throw new IllegalArgumentException(
          "filter maxRetries " + maxRetries + " outside [1, " + Limits.MAX_FILTER_RETRIES + "]");
    }
    return Gen.of(
        rng -> {
          // Every retry is charged whenever a budget exists, so stacked filters
          // cannot multiply uncharged retries. A nested check pays an attempt up
          // the chain; a top-level check root pays one element of WORK (its attempt
          // cap, about numTests + 16 * maxSteps + 64, would wrongly exhaust a
          // legitimate rare-but-satisfiable filter). Refused: the sample is
          // exhausted, and the root's refused flag turns a pass into exhausted.
          final ShrinkBudget budget = ShrinkBudget.current();
          final boolean nestedScope = budget != null && budget.isNested();
          for (int i = 0; i < maxRetries; i++) {
            if (budget != null && !(nestedScope ? budget.charge(1) : budget.chargeCopy(1))) {
              throw new ExhaustedException("the enclosing shrink budget was spent while filtering");
            }
            T v = g.sample(rng);
            if (pred.test(v)) {
              return v;
            }
          }
          throw new ExhaustedException("filter rejected " + maxRetries + " samples");
        },
        (value, falsifies, maxSteps) -> {
          // Charge before the user predicate runs: it is property code and may be costly.
          final ShrinkBudget budget = ShrinkBudget.current();
          return g.shrinkNested(
              value, c -> (budget == null || budget.charge(1)) && pred.test(c) && falsifies.test(c), maxSteps);
        });
  }

  /** Dependent generation: sample, then sample from {@code f(value)}. No shrinking. */
  public static <T, U> Gen<U> flatMap(Gen<T> g, Function<T, Gen<U>> f) {
    return Gen.of(rng -> f.apply(g.sample(rng)).sample(rng), null);
  }

  /** Picks one generator uniformly ({@code bounded(len)}, then that arm's draws). No shrinking. */
  @SafeVarargs
  public static <T> Gen<T> oneOf(Gen<T>... gens) {
    if (gens.length == 0) {
      throw new IllegalArgumentException("oneOf needs at least one generator");
    }
    List<Gen<T>> arms = List.of(gens);
    return Gen.of(rng -> arms.get((int) rng.bounded(arms.size())).sample(rng), null);
  }

  /** One weighted alternative of {@link #frequency}. */
  public record Arm<T>(long weight, Gen<T> gen) {}

  /**
   * A weighted choice: {@code pick = bounded(total)}; arms are walked in order,
   * subtracting each weight, so {@code frequency(Arm(1, a), Arm(3, b))} picks b
   * three times in four. A zero-weight arm can never be selected. An empty list,
   * a negative weight, a zero total or a total above 2^32 is rejected.
   */
  @SafeVarargs
  public static <T> Gen<T> frequency(Arm<T>... arms) {
    if (arms.length == 0) {
      throw new IllegalArgumentException("frequency needs at least one arm");
    }
    long total = 0;
    for (Arm<T> a : arms) {
      if (a.weight() < 0 || a.weight() > Limits.MAX_FREQUENCY_TOTAL) {
        throw new IllegalArgumentException("frequency weight outside [0, 2^32]");
      }
      total += a.weight(); // at most len * 2^32: cannot wrap a long
      if (total > Limits.MAX_FREQUENCY_TOTAL) {
        throw new IllegalArgumentException("frequency total weight exceeds 2^32");
      }
    }
    if (total == 0) {
      throw new IllegalArgumentException("frequency total weight must be > 0");
    }
    List<Arm<T>> owned = List.of(arms);
    final long sum = total;
    return Gen.of(
        rng -> {
          long pick = rng.bounded(sum);
          for (Arm<T> a : owned) {
            if (pick < a.weight()) {
              return a.gen().sample(rng);
            }
            pick -= a.weight();
          }
          throw new IllegalStateException("unreachable: pick < total");
        },
        null);
  }

  /**
   * A list: {@code L = minLen + bounded(span)}, then L elements in order.
   * Shrinks by sequence shrinking, respecting minLen, with the element
   * generator's shrinker for the element-wise phase.
   */
  public static <T> Gen<List<T>> list(Gen<T> elem, int minLen, int maxLen) {
    checkLengths(minLen, maxLen, Limits.MAX_LIST_LEN, "list");
    long span = (long) (maxLen - minLen) + 1;
    return Gen.of(
        rng -> {
          int n = minLen + (int) rng.bounded(span);
          List<T> out = new ArrayList<>(Math.min(n, 1024));
          for (int i = 0; i < n; i++) {
            out.add(elem.sample(rng));
          }
          return out;
        },
        (value, falsifies, maxSteps) ->
            Shrink.shrinkSequence(value, falsifies, maxSteps, minLen, elem::shrinkNested));
  }

  /** The value of {@link #tuple2}. */
  public record Pair<A, B>(A first, B second) {}

  /** The value of {@link #tuple3}. */
  public record Triple<A, B, C>(A first, B second, C third) {}

  /**
   * One shrink-wide evaluation budget for a tuple: the repeat-until-no-progress
   * loop calls each component shrinker once per pass, and each of those has its
   * own local budget, so without a shared one the work is quadratic in maxSteps.
   * Every evaluation of the property, in any component or pass, is charged here;
   * when it is spent nothing further is accepted.
   */
  private static <V> Predicate<V> globalBudget(Predicate<V> falsifies, int maxSteps) {
    final long budget = Limits.attemptBudget(maxSteps);
    final AtomicLong spent = new AtomicLong();
    return v -> {
      if (spent.getAndIncrement() >= budget) {
        return false;
      }
      return falsifies.test(v);
    };
  }

  /** Samples a then b, and shrinks one coordinate at a time. */
  public static <A, B> Gen<Pair<A, B>> tuple2(Gen<A> ga, Gen<B> gb) {
    return Gen.of(
        rng -> {
          A a = ga.sample(rng);
          B b = gb.sample(rng);
          return new Pair<>(a, b);
        },
        (value, rawFalsifies, maxSteps) -> {
          Predicate<Pair<A, B>> falsifies = globalBudget(rawFalsifies, maxSteps);
          A first = value.first();
          B second = value.second();
          int steps = 0;
          // Repeat passes until a whole pass makes no progress: shrinking one
          // coordinate can unlock more shrinking of the other. Every pass that
          // continues accepted at least one move, so the budget bounds it.
          boolean progress = true;
          while (progress && steps < maxSteps) {
            progress = false;
            if (maxSteps - steps > 0) {
              final B fixedB = second;
              Shrunk<A> r = ga.shrinkNested(first, a -> falsifies.test(new Pair<>(a, fixedB)), maxSteps - steps);
              if (r.steps() > 0) {
                first = r.value();
                steps += r.steps();
                progress = true;
              }
            }
            if (maxSteps - steps > 0) {
              final A fixedA = first;
              Shrunk<B> r = gb.shrinkNested(second, b -> falsifies.test(new Pair<>(fixedA, b)), maxSteps - steps);
              if (r.steps() > 0) {
                second = r.value();
                steps += r.steps();
                progress = true;
              }
            }
          }
          return new Shrunk<>(new Pair<>(first, second), steps);
        });
  }

  /** Samples a, b, c in order and shrinks one coordinate at a time. */
  public static <A, B, C> Gen<Triple<A, B, C>> tuple3(Gen<A> ga, Gen<B> gb, Gen<C> gc) {
    return Gen.of(
        rng -> {
          A a = ga.sample(rng);
          B b = gb.sample(rng);
          C c = gc.sample(rng);
          return new Triple<>(a, b, c);
        },
        (value, rawFalsifies, maxSteps) -> {
          Predicate<Triple<A, B, C>> falsifies = globalBudget(rawFalsifies, maxSteps);
          A first = value.first();
          B second = value.second();
          C third = value.third();
          int steps = 0;
          boolean progress = true;
          while (progress && steps < maxSteps) {
            progress = false;
            if (maxSteps - steps > 0) {
              final B fb = second;
              final C fc = third;
              Shrunk<A> r = ga.shrinkNested(first, a -> falsifies.test(new Triple<>(a, fb, fc)), maxSteps - steps);
              if (r.steps() > 0) {
                first = r.value();
                steps += r.steps();
                progress = true;
              }
            }
            if (maxSteps - steps > 0) {
              final A fa = first;
              final C fc = third;
              Shrunk<B> r = gb.shrinkNested(second, b -> falsifies.test(new Triple<>(fa, b, fc)), maxSteps - steps);
              if (r.steps() > 0) {
                second = r.value();
                steps += r.steps();
                progress = true;
              }
            }
            if (maxSteps - steps > 0) {
              final A fa = first;
              final B fb = second;
              Shrunk<C> r = gc.shrinkNested(third, c -> falsifies.test(new Triple<>(fa, fb, c)), maxSteps - steps);
              if (r.steps() > 0) {
                third = r.value();
                steps += r.steps();
                progress = true;
              }
            }
          }
          return new Shrunk<>(new Triple<>(first, second, third), steps);
        });
  }

  // ---------------------------------------------------------------------------
  // Corpus entry points (pbt-v1)
  // ---------------------------------------------------------------------------

  private static void checkCount(int count) {
    if (count < 0 || count > Limits.MAX_GEN_COUNT) {
      throw new IllegalArgumentException("count " + count + " outside [0, " + Limits.MAX_GEN_COUNT + "]");
    }
  }

  /**
   * {@code count} primitives of {@code typeName} ("bool", "u8", "u64", "i128")
   * drawn from {@code seed}. Booleans are Boolean; every integer is BigInteger.
   */
  public static List<Object> genPrimitive(long seed, String typeName, int count) {
    checkCount(count);
    SplitMix64 rng = Hooks.newRng.apply(seed);
    List<Object> out = new ArrayList<>(Math.min(count, 1024));
    for (int i = 0; i < count; i++) {
      switch (typeName) {
        case "bool" -> out.add(rng.bounded(2) == 1);
        case "u8" -> out.add(SplitMix64.unsigned(rng.bounded(256)));
        case "u64" -> out.add(SplitMix64.unsigned(rng.nextU64()));
        case "i128" -> out.add(sampleI128(rng));
        default -> throw new IllegalArgumentException("unknown primitive type \"" + typeName + "\"");
      }
    }
    return out;
  }

  /** {@code count} uniform integers over [lo, hi] drawn from {@code seed}. */
  public static List<BigInteger> genBounded(long seed, BigInteger lo, BigInteger hi, int count) {
    if (lo == null || hi == null || lo.compareTo(hi) > 0) {
      throw new IllegalArgumentException("need non-null lo <= hi");
    }
    checkCount(count);
    // Each draw is an integer about as wide as the span; count x width bounds
    // what the returned list can hold (a 1,000-digit span x 10^6 draws would not).
    long width = Math.max(1, (hi.subtract(lo).bitLength() + 7) / 8);
    if (count * width > Limits.MAX_GEN_BYTES_TOTAL) {
      throw new IllegalArgumentException(
          "count " + count + " x " + width + "-byte span exceeds the generation budget of "
              + Limits.MAX_GEN_BYTES_TOTAL + " bytes");
    }
    SplitMix64 rng = Hooks.newRng.apply(seed);
    List<BigInteger> out = new ArrayList<>(Math.min(count, 1024));
    for (int i = 0; i < count; i++) {
      out.add(boundedInt(rng, lo, hi));
    }
    return out;
  }

  /** {@code count} byte strings of length in [minLen, maxLen] from {@code seed}. */
  public static List<byte[]> genBytes(long seed, int minLen, int maxLen, int count) {
    if (minLen < 0 || maxLen < minLen || maxLen > Limits.MAX_INPUT_BYTES) {
      throw new IllegalArgumentException("need 0 <= minLen <= maxLen <= " + Limits.MAX_INPUT_BYTES);
    }
    checkCount(count); // applies even when maxLen == 0: empty strings still cost one slot each
    if (maxLen > 0 && count > Limits.MAX_GEN_BYTES_TOTAL / maxLen) {
      throw new IllegalArgumentException(
          "count " + count + " x maxLen " + maxLen + " exceeds the total of " + Limits.MAX_GEN_BYTES_TOTAL + " bytes");
    }
    Gen<byte[]> gen = bytes(minLen, maxLen);
    SplitMix64 rng = Hooks.newRng.apply(seed);
    List<byte[]> out = new ArrayList<>(Math.min(count, 1024));
    for (int i = 0; i < count; i++) {
      out.add(gen.sample(rng));
    }
    return out;
  }
}
