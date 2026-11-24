package com.codingadventures.testbootstrap.fuzz;

import java.util.List;

/**
 * How one execution ended. {@code crashKind} and {@code frames} are meaningful
 * only for {@link Kind#CRASH}; they never carry a message or input bytes.
 */
public record Outcome(Kind kind, String crashKind, List<String> frames) {
  /** The four outcomes of one execution. */
  public enum Kind {
    /** The target returned normally. */
    PASS,
    /** The target threw. */
    CRASH,
    /** The target exhausted its step budget. */
    TIMEOUT,
    /** The target exhausted its memory budget (or raised a real OutOfMemoryError). */
    OOM
  }

  /** The target returned. */
  public static Outcome pass() {
    return new Outcome(Kind.PASS, "", List.of());
  }

  /** The step budget ran out. */
  public static Outcome timeout() {
    return new Outcome(Kind.TIMEOUT, "", List.of());
  }

  /** The memory budget ran out. */
  public static Outcome oom() {
    return new Outcome(Kind.OOM, "", List.of());
  }

  /** A crash with an already-normalised kind and frames. */
  public static Outcome crash(String kind, List<String> frames) {
    return new Outcome(Kind.CRASH, kind, List.copyOf(frames));
  }
}
