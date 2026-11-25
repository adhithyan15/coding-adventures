package com.codingadventures.testbootstrap.pbt;

import java.util.function.Predicate;

/**
 * Shrinks a failing value. {@code falsifies.test(v)} is true while {@code v}
 * still makes the property fail; the shrinker returns a value that still
 * falsifies (or the original) and the number of accepted moves.
 */
@FunctionalInterface
public interface Shrinker<T> {
  Shrunk<T> shrink(T value, Predicate<T> falsifies, int maxSteps);
}
