package com.codingadventures.testbootstrap.fuzz;

/**
 * The code under test. It receives its own copy of the input and a {@link Handle}
 * for reporting coverage and observing budgets. Any throwable that escapes is a
 * crash (except the engine's own budget sentinels).
 */
@FunctionalInterface
public interface Target {
  /** Runs the code under test on one input. */
  void run(byte[] data, Handle h) throws Exception;
}
