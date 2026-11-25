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
   * there is no shrinker. A public entry point (VER05e section 4.1): it opens a
   * child of the ambient shrink budget (a fresh root outside any shrink) with its
   * own local cap, and charges every evaluation of {@code falsifies}, so however
   * a property or a custom {@link Shrinker} nests calls, the total work stays
   * inside the outermost budget.
   */
  public Shrunk<T> shrink(T value, Predicate<T> falsifies, int maxSteps) {
    Shrink.checkBudget(maxSteps);
    try (ShrinkBudget budget = ShrinkBudget.enterChild(maxSteps)) {
      return run(value, budget.charged(falsifies), maxSteps);
    }
  }

  /**
   * For combinators shrinking their components: joins the enclosing shrink
   * budget instead of opening a new one (a fresh one only when none is open).
   */
  Shrunk<T> shrinkNested(T value, Predicate<T> falsifies, int maxSteps) {
    Shrink.checkBudget(maxSteps);
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
