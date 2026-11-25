package com.codingadventures.testbootstrap.pbt;

/**
 * Bounds that keep every loop and allocation finite (CWE-400). A caller or a
 * fixture can ask for more only by being refused.
 */
public final class Limits {
  private Limits() {}

  /** The most cases {@link Runner#check} will run. */
  public static final int MAX_CASES = 100_000;

  /** The largest shrink budget. */
  public static final int MAX_SHRINK_STEPS = 10_000;

  /** The longest {@link Gens#bytes} generator. */
  public static final int MAX_INPUT_BYTES = 1 << 20;

  /** The longest {@link Gens#list} generator. */
  public static final int MAX_LIST_LEN = 100_000;

  /** The most values any corpus entry point (genPrimitive, genBounded, genBytes) will generate. */
  public static final int MAX_GEN_COUNT = 1_000_000;

  /**
   * Total attempt budget of sequence shrinking is {@code ATTEMPT_FACTOR * maxSteps
   * + ATTEMPT_SLACK}: rejected attempts cost a list copy each but are not
   * accepted moves, so they need their own bound (VER05e section 4).
   */
  public static final int ATTEMPT_FACTOR = 16;

  /** See {@link #ATTEMPT_FACTOR}. */
  public static final int ATTEMPT_SLACK = 64;

  /** Bounds count x maxLen for {@link Gens#genBytes}. */
  public static final int MAX_GEN_BYTES_TOTAL = 1 << 20;

  /**
   * The largest total weight of {@link Gens#frequency}: it draws with a bounded
   * 64-bit draw, and the VER05 limit is 2^32.
   */
  public static final long MAX_FREQUENCY_TOTAL = 1L << 32;

  /** How many samples a filter tries before giving up. */
  public static final int DEFAULT_MAX_RETRIES = 100;
}
