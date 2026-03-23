/**
 * Prediction statistics — measuring how well a branch predictor performs.
 *
 * Every branch predictor needs a scorecard. When a CPU designer evaluates a
 * predictor, the first question is always: "What's the accuracy?" A predictor
 * that's 95% accurate causes a pipeline flush on only 5% of branches, while
 * a 70% accurate predictor flushes on 30% — potentially halving throughput
 * on a deeply pipelined machine.
 *
 * This module provides PredictionStats, a simple counter-based tracker that
 * records every prediction and computes accuracy metrics.
 *
 * Real-world context:
 * - Intel's Pentium Pro achieved ~90% accuracy with a two-level adaptive predictor
 * - Modern CPUs (since ~2015) achieve 95-99% accuracy using TAGE or perceptron predictors
 * - Even a 1% improvement in accuracy can yield measurable speedups on branch-heavy code
 */

// ─── PredictionStats ──────────────────────────────────────────────────────────
//
// A class that acts as the scoreboard for any predictor. Every time the
// predictor makes a guess, the core calls `record(correct)` to log whether
// the guess was right or wrong.
//
// We track three counters:
//   predictions — total number of branches seen
//   correct     — how many the predictor got right
//   incorrect   — how many it got wrong
//
// From these, we derive:
//   accuracy            — correct / predictions × 100 (as a percentage)
//   mispredictionRate   — incorrect / predictions × 100 (the complement)
//
// Edge case: if no predictions have been made yet, both rates return 0.0
// rather than throwing a division error. This is a design choice — a
// predictor that hasn't seen any branches has no accuracy, not infinite accuracy.

/**
 * Tracks prediction accuracy for a branch predictor.
 *
 * @example
 * ```ts
 * const stats = new PredictionStats();
 * stats.record(true);   // predictor got it right
 * stats.record(true);   // right again
 * stats.record(false);  // wrong this time
 * console.log(stats.accuracy);  // 66.67 (2 out of 3)
 * ```
 *
 * The stats object is usually owned by a predictor and exposed via its
 * `stats` property. The CPU core never creates PredictionStats directly —
 * it just reads the predictor's stats after running a benchmark.
 */
export class PredictionStats {
  // ── Counters ──────────────────────────────────────────────────────────
  //
  // All counters start at zero. Creating a PredictionStats() starts
  // with a clean slate.

  /** Total number of predictions made. */
  predictions: number;

  /** Number of correct predictions. */
  correct: number;

  /** Number of incorrect predictions (mispredictions). */
  incorrect: number;

  constructor(predictions = 0, correct = 0, incorrect = 0) {
    this.predictions = predictions;
    this.correct = correct;
    this.incorrect = incorrect;
  }

  // ── Derived Metrics ───────────────────────────────────────────────────
  //
  // These are getters, not stored fields, because they're computed from
  // the counters. This avoids the classic bug of updating a counter but
  // forgetting to update the derived value.

  /**
   * Prediction accuracy as a percentage (0.0 to 100.0).
   *
   * Returns 0.0 if no predictions have been made yet, because we can't
   * divide by zero, and "no data" is semantically closer to "0% accurate"
   * than "100% accurate" in a benchmarking context.
   *
   * @example
   * ```ts
   * const stats = new PredictionStats(100, 87, 13);
   * // stats.accuracy === 87.0
   * ```
   */
  get accuracy(): number {
    if (this.predictions === 0) {
      return 0.0;
    }
    return (this.correct / this.predictions) * 100.0;
  }

  /**
   * Misprediction rate as a percentage (0.0 to 100.0).
   *
   * This is the complement of accuracy: mispredictionRate = 100 - accuracy.
   * CPU architects often think in terms of misprediction rate because each
   * misprediction causes a pipeline flush — a concrete, measurable cost.
   *
   * @example
   * ```ts
   * const stats = new PredictionStats(100, 87, 13);
   * // stats.mispredictionRate === 13.0
   * ```
   */
  get mispredictionRate(): number {
    if (this.predictions === 0) {
      return 0.0;
    }
    return (this.incorrect / this.predictions) * 100.0;
  }

  // ── Mutation ──────────────────────────────────────────────────────────

  /**
   * Record the outcome of a single prediction.
   *
   * This is the primary API that the CPU core calls after every branch.
   *
   * @param correct - True if the predictor guessed correctly, False otherwise.
   *
   * @example
   * ```ts
   * const stats = new PredictionStats();
   * stats.record(true);
   * // stats.predictions === 1
   * // stats.correct === 1
   * ```
   */
  record(correct: boolean): void {
    this.predictions += 1;
    if (correct) {
      this.correct += 1;
    } else {
      this.incorrect += 1;
    }
  }

  /**
   * Reset all counters to zero.
   *
   * Called when starting a new benchmark or program execution. Without
   * this, stats from a previous run would contaminate the new measurement.
   *
   * @example
   * ```ts
   * const stats = new PredictionStats(50, 40, 10);
   * stats.reset();
   * // stats.predictions === 0
   * // stats.accuracy === 0.0
   * ```
   */
  reset(): void {
    this.predictions = 0;
    this.correct = 0;
    this.incorrect = 0;
  }
}
