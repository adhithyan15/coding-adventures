package com.codingadventures.testbootstrap.fuzz;

/**
 * Bounds a campaign; every field is validated against its ceiling (CWE-400:
 * nothing a caller controls is unbounded).
 *
 * <p>{@code minimizeBudget} is the most target executions spent minimising ONE
 * novel crash. On top of it the whole campaign spends at most {@code maxIterations}
 * minimisation executions in total, so minimisation can at most double a campaign's
 * work however many distinct crashes it finds.
 *
 * @param maxIterations mutation executions, in [0, {@link Mutate#MAX_ITERATIONS}]
 * @param seed the campaign seed (an unsigned word in a long)
 * @param maxInputLen the longest mutated candidate, in [0, {@link Mutate#MAX_INPUT_BYTES}]
 * @param baseEnergy the energy base, in [0, {@link Mutate#MAX_ITERATIONS}]
 * @param minimizeBudget executions per crash minimisation, in [0, {@link #MAX_MINIMIZE_BUDGET}]
 * @param stepBudget {@code tick()} calls per execution, in [0, {@link #MAX_STEP_BUDGET}]
 * @param memBudget {@code reserve()} bytes per execution, in [0, {@link #MAX_MEM_BUDGET}]
 */
public record Options(
    int maxIterations, long seed, int maxInputLen, int baseEnergy, int minimizeBudget, int stepBudget, int memBudget) {
  /** Ceiling of the step budget a target can observe. */
  public static final int MAX_STEP_BUDGET = 100_000_000;

  /** Ceiling of the memory budget a target can observe (1 GiB). */
  public static final int MAX_MEM_BUDGET = 1 << 30;

  /** Ceiling of the executions spent minimising one crash. */
  public static final int MAX_MINIMIZE_BUDGET = 100_000;

  /** Default per-crash minimisation budget. */
  public static final int DEFAULT_MINIMIZE_BUDGET = 2000;

  /** Default step budget. */
  public static final int DEFAULT_STEP_BUDGET = 200_000;

  /** Default memory budget (64 MiB). */
  public static final int DEFAULT_MEM_BUDGET = 64 << 20;

  /** The standard campaign: 1000 iterations, seed 42, inputs up to 4096 bytes. */
  public static Options defaults() {
    return new Options(1000, 42, 4096, 10, DEFAULT_MINIMIZE_BUDGET, DEFAULT_STEP_BUDGET, DEFAULT_MEM_BUDGET);
  }

  /** A copy with a different iteration count. */
  public Options withMaxIterations(int v) {
    return new Options(v, seed, maxInputLen, baseEnergy, minimizeBudget, stepBudget, memBudget);
  }

  /** A copy with a different seed. */
  public Options withSeed(long v) {
    return new Options(maxIterations, v, maxInputLen, baseEnergy, minimizeBudget, stepBudget, memBudget);
  }

  /** A copy with a different candidate length cap. */
  public Options withMaxInputLen(int v) {
    return new Options(maxIterations, seed, v, baseEnergy, minimizeBudget, stepBudget, memBudget);
  }

  /** A copy with a different energy base. */
  public Options withBaseEnergy(int v) {
    return new Options(maxIterations, seed, maxInputLen, v, minimizeBudget, stepBudget, memBudget);
  }

  /** A copy with a different per-crash minimisation budget. */
  public Options withMinimizeBudget(int v) {
    return new Options(maxIterations, seed, maxInputLen, baseEnergy, v, stepBudget, memBudget);
  }

  /** A copy with a different step budget. */
  public Options withStepBudget(int v) {
    return new Options(maxIterations, seed, maxInputLen, baseEnergy, minimizeBudget, v, memBudget);
  }

  /** A copy with a different memory budget. */
  public Options withMemBudget(int v) {
    return new Options(maxIterations, seed, maxInputLen, baseEnergy, minimizeBudget, stepBudget, v);
  }

  /** Checks every field against its range; throws {@link IllegalArgumentException}. */
  public void validate() {
    check("maxIterations", maxIterations, Mutate.MAX_ITERATIONS);
    check("maxInputLen", maxInputLen, Mutate.MAX_INPUT_BYTES);
    check("baseEnergy", baseEnergy, Mutate.MAX_ITERATIONS);
    check("minimizeBudget", minimizeBudget, MAX_MINIMIZE_BUDGET);
    check("stepBudget", stepBudget, MAX_STEP_BUDGET);
    check("memBudget", memBudget, MAX_MEM_BUDGET);
  }

  private static void check(String name, int v, int hi) {
    if (v < 0 || v > hi) {
      throw new IllegalArgumentException(name + " must be in [0, " + hi + "]");
    }
  }
}
