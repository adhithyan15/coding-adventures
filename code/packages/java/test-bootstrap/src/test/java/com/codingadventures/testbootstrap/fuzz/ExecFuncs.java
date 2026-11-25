package com.codingadventures.testbootstrap.fuzz;

/** Tiny helpers shared by the engine tests. */
final class ExecFuncs {
  private ExecFuncs() {}

  /** A mutable counter usable from a lambda. */
  static final class Counter {
    int runs;
  }
}
