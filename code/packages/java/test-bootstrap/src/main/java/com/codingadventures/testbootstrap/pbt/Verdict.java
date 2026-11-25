package com.codingadventures.testbootstrap.pbt;

/**
 * The outcome of {@link Runner#check}. In redacted mode {@code seed},
 * {@code counterexample} and {@code minimal} are null; only the message,
 * iteration, step count and digest remain.
 */
public record Verdict(
    boolean passed,
    String failureId,
    String message,
    int casesRun,
    int iteration,
    int shrinkSteps,
    Long seed,
    Object counterexample,
    Object minimal,
    String digest) {

  /** The negation of {@link #passed()}. */
  public boolean failed() {
    return !passed;
  }

  /** Throws an {@link AssertionError} carrying the failure id unless the verdict is a pass. */
  public void assertPassed() {
    if (!passed) {
      throw new AssertionError("[" + failureId + "] " + message);
    }
  }
}
