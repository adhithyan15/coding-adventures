package com.codingadventures.testbootstrap.pbt;

import java.util.List;

/** The closed failure vocabulary (VER05 section 5.1). */
public final class FailureIds {
  private FailureIds() {}

  /** The property was falsified; a minimal witness was found. */
  public static final String COUNTEREXAMPLE = "property-counterexample";

  /** Nothing (or no valid input) was evaluated. */
  public static final String EXHAUSTED = "property-exhausted";

  /** Registered but never produced by this lane. */
  public static final String TIMEOUT = "property-timeout";

  /** The property itself threw. */
  public static final String PANIC = "property-panic";

  /** The closed set of ids, in registration order. */
  public static final List<String> ALL = List.of(COUNTEREXAMPLE, EXHAUSTED, TIMEOUT, PANIC);

  /**
   * True only for one of the four registered ids.
   *
   * <pre>
   *   isPbtFailureId("property-panic")   -> true
   *   isPbtFailureId("unregistered-id")  -> false
   * </pre>
   */
  public static boolean isPbtFailureId(String candidate) {
    return candidate != null && ALL.contains(candidate);
  }
}
