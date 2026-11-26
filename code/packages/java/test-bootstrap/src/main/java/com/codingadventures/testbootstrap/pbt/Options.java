package com.codingadventures.testbootstrap.pbt;

import java.util.function.Function;

/**
 * Configures {@link Runner#check}.
 *
 * @param numTests how many cases to run. 0 is legal and is a failure: a property
 *     that never ran proves nothing.
 * @param maxShrinkSteps the shrink budget, 0..{@link Limits#MAX_SHRINK_STEPS}
 * @param ci redaction mode
 */
public record Options(int numTests, int maxShrinkSteps, CiMode ci) {
  /** 100 cases, 1000 shrink steps and CI redaction. */
  public static Options defaults() {
    return new Options(100, 1000, CiMode.UNSET);
  }

  /** The same options with another CI mode. */
  public Options withCi(CiMode mode) {
    return new Options(numTests, maxShrinkSteps, mode);
  }

  /**
   * Pure CI detection. Pass {@code System::getenv}: it reports true when CI is
   * set to anything except empty, "0" or "false" (trimmed, any case). The package
   * calls no environment function itself.
   */
  public static boolean ciFromLookup(Function<String, String> lookup) {
    String value = lookup.apply("CI");
    if (value == null) {
      return false;
    }
    String v = value.trim().toLowerCase(java.util.Locale.ROOT);
    return !(v.isEmpty() || v.equals("0") || v.equals("false"));
  }
}
