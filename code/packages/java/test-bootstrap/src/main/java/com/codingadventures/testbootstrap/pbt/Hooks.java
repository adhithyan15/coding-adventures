package com.codingadventures.testbootstrap.pbt;

import com.codingadventures.testbootstrap.prng.SplitMix64;
import java.math.BigInteger;
import java.util.function.BinaryOperator;
import java.util.function.LongFunction;

/**
 * Sabotage hooks. These are the only production indirections added for the
 * negative controls of VER05e section 6: a test swaps one, runs the corpus, and
 * the runner must report problems. Production code never reassigns them, and
 * the class is package-private so no consumer can.
 */
final class Hooks {
  private Hooks() {}

  /** The bisection move: {@code current - floor(diff / 2)}. */
  static volatile BinaryOperator<BigInteger> bisectCandidate =
      (current, diff) -> current.subtract(diff.shiftRight(1)); // shiftRight(1) floors

  /** The unit move: one step toward the target. */
  static volatile BinaryOperator<BigInteger> unitCandidate =
      (current, diff) -> current.subtract(BigInteger.valueOf(diff.signum()));

  /** Disables the single-deletion phase of sequence shrinking. */
  static volatile boolean skipDeletionPhase = false;

  /** Builds the stream for a seed. */
  static volatile LongFunction<SplitMix64> newRng = SplitMix64::new;

  /** Restores every hook to its production value. */
  static void reset() {
    bisectCandidate = (current, diff) -> current.subtract(diff.shiftRight(1));
    unitCandidate = (current, diff) -> current.subtract(BigInteger.valueOf(diff.signum()));
    skipDeletionPhase = false;
    newRng = SplitMix64::new;
    ShrinkBudget.defaultMaxWork = Limits.MAX_SHRINK_WORK;
  }
}
