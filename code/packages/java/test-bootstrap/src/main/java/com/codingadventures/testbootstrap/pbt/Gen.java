package com.codingadventures.testbootstrap.pbt;

import com.codingadventures.testbootstrap.prng.SplitMix64;
import java.util.function.Predicate;

/**
 * A generator of values of type {@code T}: it draws from a PRNG stream and knows
 * how to shrink what it drew (VER05e section 3).
 *
 * <p>Sampling throws {@link ExhaustedException} because generation can fail on
 * data (a filter that never accepts); every combinator propagates it. The
 * shrinker is optional: {@code null} means "values are reported unshrunk".
 */
public final class Gen<T> {
  /** Draws one value from the stream. */
  @FunctionalInterface
  public interface Sampler<T> {
    T sample(SplitMix64 rng);
  }

  private final Sampler<T> sampler;
  private final Shrinker<T> shrinker;

  private Gen(Sampler<T> sampler, Shrinker<T> shrinker) {
    this.sampler = sampler;
    this.shrinker = shrinker;
  }

  /** Builds a generator from a sampler and an optional shrinker. */
  public static <T> Gen<T> of(Sampler<T> sampler, Shrinker<T> shrinker) {
    if (sampler == null) {
      throw new IllegalArgumentException("a generator needs a sampler");
    }
    return new Gen<>(sampler, shrinker);
  }

  /** Draws one value. */
  public T sample(SplitMix64 rng) {
    return sampler.sample(rng);
  }

  /**
   * Shrinks {@code value} while {@code falsifies} holds; {@code (value, 0)} if
   * there is no shrinker. A top-level entry point: it opens an isolated shrink
   * budget (VER05e section 4.1), so a property that itself shrinks neither eats
   * nor is starved by an enclosing shrink.
   */
  public Shrunk<T> shrink(T value, Predicate<T> falsifies, int maxSteps) {
    try (ShrinkBudget budget = ShrinkBudget.enterIsolated(maxSteps)) {
      return run(value, falsifies, maxSteps);
    }
  }

  /**
   * For combinators shrinking their components: joins the enclosing shrink
   * budget instead of opening a new one (a fresh one only when none is open).
   */
  Shrunk<T> shrinkNested(T value, Predicate<T> falsifies, int maxSteps) {
    try (ShrinkBudget budget = ShrinkBudget.enter(maxSteps)) {
      return run(value, falsifies, maxSteps);
    }
  }

  private Shrunk<T> run(T value, Predicate<T> falsifies, int maxSteps) {
    if (shrinker == null) {
      return new Shrunk<>(value, 0);
    }
    return shrinker.shrink(value, falsifies, maxSteps);
  }
}
