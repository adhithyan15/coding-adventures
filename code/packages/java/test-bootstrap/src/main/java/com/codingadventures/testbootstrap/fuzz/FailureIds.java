package com.codingadventures.testbootstrap.fuzz;

import java.util.List;

/**
 * The closed vocabulary of fuzz failure ids (VER06 section 6). Only these five
 * strings may appear in a public failure line.
 *
 * <p>{@code fuzz-leak} and {@code fuzz-panic} are registered but this engine
 * cannot produce them (VER06f section 9, item 7).
 */
public final class FailureIds {
  private FailureIds() {}

  /** A target crashed. */
  public static final String CRASH = "fuzz-crash";

  /** A target exhausted its step budget. */
  public static final String TIMEOUT = "fuzz-timeout";

  /** A target exhausted its memory budget. */
  public static final String OOM = "fuzz-oom";

  /** Registered; unproducible here. */
  public static final String LEAK = "fuzz-leak";

  /** Registered; unproducible here. */
  public static final String PANIC = "fuzz-panic";

  /** Every registered id. */
  public static final List<String> ALL = List.of(CRASH, TIMEOUT, OOM, LEAK, PANIC);

  /**
   * Whether {@code candidate} is one of the five registered ids.
   *
   * <pre>
   *   isFuzzFailureId("fuzz-crash")           -&gt; true
   *   isFuzzFailureId("fuzz-arbitrary-error") -&gt; false
   * </pre>
   */
  public static boolean isFuzzFailureId(String candidate) {
    return candidate != null && ALL.contains(candidate);
  }
}
