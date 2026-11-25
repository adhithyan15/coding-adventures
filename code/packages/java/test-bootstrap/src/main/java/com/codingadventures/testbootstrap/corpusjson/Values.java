package com.codingadventures.testbootstrap.corpusjson;

import java.math.BigInteger;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;

/**
 * Strict value decoding and comparison for corpus cases (VER05e section 6).
 *
 * <p>Case encoding (JSON):
 *
 * <pre>
 *   {"kind": "int",   "value": "-12"}      an integer (any width, decimal text)
 *   {"kind": "bytes", "hex": "00ff"}       a byte string
 *   true / false                           a bool
 *   [ ... ]                                a list of the above
 *   "text"                                 a string (types, property names)
 * </pre>
 *
 * Decoded values are {@code Boolean}, {@code String}, {@link BigInteger},
 * {@code byte[]} and {@code List<Object>}.
 */
public final class Values {
  /** Bounds every fixture-controlled size: counts, lengths, list and bytes elements. */
  public static final int MAX_SIZE = 10_000;

  /** Bounds the decimal digits of any integer in a case. */
  public static final int MAX_INT_DIGITS = 1_000;

  /** Bounds value nesting so decoding recursion is finite. */
  public static final int MAX_DEPTH = 64;

  /** Bounds one reported problem. */
  public static final int MAX_PROBLEM_LEN = 400;

  /** Bounds one fixture-derived fragment inside a problem. */
  public static final int MAX_FRAGMENT_LEN = 200;

  private Values() {}

  /** A problem with one case's encoding; its text is already clipped. */
  public static final class CaseException extends RuntimeException {
    private static final long serialVersionUID = 1L;

    public CaseException(String message) {
      super(Clip.clip(message, MAX_PROBLEM_LEN));
    }
  }

  /** Bounded rendering of a fixture-derived value for a message. */
  public static String show(Object v) {
    return Clip.clip(Canon.render(v), MAX_FRAGMENT_LEN);
  }

  /**
   * Decimal digits only, a single leading "-" at most, no leading zeros, no "-0",
   * nothing a lenient parser would also swallow (underscores, spaces, "+").
   */
  public static boolean isCanonicalInt(String text) {
    String digits = text.startsWith("-") ? text.substring(1) : text;
    if (digits.isEmpty()) {
      return false;
    }
    for (int i = 0; i < digits.length(); i++) {
      char c = digits.charAt(i);
      if (c < '0' || c > '9') {
        return false;
      }
    }
    if (digits.charAt(0) == '0') {
      return text.equals("0");
    }
    return true;
  }

  /** Even length, digits and a-f only. */
  public static boolean isLowercaseHex(String text) {
    if (text.length() % 2 != 0) {
      return false;
    }
    for (int i = 0; i < text.length(); i++) {
      char c = text.charAt(i);
      if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'))) {
        return false;
      }
    }
    return true;
  }

  private static byte[] hexToBytes(String text) {
    byte[] out = new byte[text.length() / 2];
    for (int i = 0; i < out.length; i++) {
      out[i] = (byte) (Character.digit(text.charAt(2 * i), 16) << 4 | Character.digit(text.charAt(2 * i + 1), 16));
    }
    return out;
  }

  /** Turns a parsed JSON node into a strictly decoded value. */
  public static Object decode(Object node) {
    return decode(node, 0);
  }

  // Runners may be handed a hand-built tree that never went through the parser,
  // so the depth cap lives here too: a deeper input is a problem, not a stack
  // overflow.
  private static Object decode(Object node, int depth) {
    if (depth > MAX_DEPTH) {
      throw new CaseException("value nesting deeper than " + MAX_DEPTH);
    }
    if (node instanceof Boolean || node instanceof String) {
      return node;
    }
    if (node instanceof List<?> list) {
      if (list.size() > MAX_SIZE) {
        throw new CaseException("list exceeds " + MAX_SIZE + " elements");
      }
      List<Object> out = new ArrayList<>(list.size());
      for (Object item : list) {
        out.add(decode(item, depth + 1));
      }
      return out;
    }
    if (node instanceof Map<?, ?> map) {
      Object kind = map.get("kind");
      if ("int".equals(kind) && map.size() == 2) {
        Object text = map.get("value");
        if (!(text instanceof String t) || !isCanonicalInt(t)) {
          throw new CaseException("non-canonical int " + show(text));
        }
        String digits = t.startsWith("-") ? t.substring(1) : t;
        if (digits.length() > MAX_INT_DIGITS) {
          throw new CaseException("int exceeds " + MAX_INT_DIGITS + " digits");
        }
        return new BigInteger(t);
      }
      if ("bytes".equals(kind) && map.size() == 2) {
        Object text = map.get("hex");
        if (!(text instanceof String t) || !isLowercaseHex(t)) {
          throw new CaseException("bytes hex must be even-length lowercase: " + show(text));
        }
        if (t.length() > 2 * MAX_SIZE) {
          throw new CaseException("bytes exceed " + MAX_SIZE + " bytes");
        }
        return hexToBytes(t);
      }
      throw new CaseException("unrecognised value node " + show(map));
    }
    throw new CaseException("unsupported JSON value " + show(node));
  }

  /** Equality that does not conflate true with 1, lists with bytes, or an int with its text. */
  public static boolean same(Object a, Object b) {
    return same(a, b, 0);
  }

  private static boolean same(Object a, Object b, int depth) {
    if (depth > MAX_DEPTH) {
      return false; // decode() caps real inputs; deeper is never "the same"
    }
    if (a instanceof Boolean x) {
      return b instanceof Boolean y && x.equals(y);
    }
    if (a instanceof String x) {
      return b instanceof String y && x.equals(y);
    }
    if (a instanceof BigInteger x) {
      return b instanceof BigInteger y && x.compareTo(y) == 0;
    }
    if (a instanceof byte[] x) {
      return b instanceof byte[] y && Arrays.equals(x, y);
    }
    if (a instanceof List<?> x) {
      if (!(b instanceof List<?> y) || x.size() != y.size()) {
        return false;
      }
      for (int i = 0; i < x.size(); i++) {
        if (!same(x.get(i), y.get(i), depth + 1)) {
          return false;
        }
      }
      return true;
    }
    return false;
  }
}
