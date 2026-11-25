package com.codingadventures.testbootstrap.corpusjson;

/** Bounded text for messages built from untrusted fixture data (CWE-400). */
public final class Clip {
  private Clip() {}

  /** Truncates {@code s} to {@code limit} chars, marking the cut with "...". */
  public static String clip(String s, int limit) {
    if (s.length() <= limit) {
      return s;
    }
    int cut = limit;
    // Never split a surrogate pair.
    if (cut > 0 && Character.isHighSurrogate(s.charAt(cut - 1))) {
      cut--;
    }
    return s.substring(0, cut) + "...";
  }
}
