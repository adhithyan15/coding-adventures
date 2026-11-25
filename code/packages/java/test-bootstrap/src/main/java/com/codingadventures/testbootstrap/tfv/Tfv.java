package com.codingadventures.testbootstrap.tfv;

import java.nio.ByteBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * Validates TFV v1 text, the line format that carries test corpora between
 * languages (TF01 section 2).
 *
 * <p>This is a <em>validator</em>, not a reader: its one job in this package is
 * to let the corpus runner check the pbt-v1 {@code corpus_reject} cases with a
 * real parser instead of skipping them.
 *
 * <h2>The format in one screen</h2>
 *
 * <pre>
 *   tfv 1
 *   # comment
 *   &lt;id&gt; R&lt;n&gt; K&lt;field&gt; &lt;value&gt; K&lt;field&gt; &lt;value&gt; ...
 * </pre>
 *
 * A value is one of
 *
 * <pre>
 *   N  T  F                 null, true, false
 *   I&lt;int&gt;                  canonical decimal integer, at most 1000 digits
 *   D&lt;16 hex&gt;               IEEE-754 bit pattern, lowercase
 *   X&lt;hex&gt;  S&lt;hex&gt;          bytes / UTF-8 text as even-length lowercase hex
 *   L&lt;n&gt; v1 ... vn          list
 *   M&lt;n&gt; k1 v1 ... kn vn    map; no two keys deep-equal
 * </pre>
 *
 * Tokens are separated by exactly one ASCII space. Anything else is rejected:
 * uppercase or odd-length hex, {@code I-0}, {@code I007}, a count that disagrees
 * with the tokens, trailing tokens, doubled spaces, a duplicate field or record
 * id, invalid UTF-8 inside S, a carriage return in a record line, nesting deeper
 * than 64 containers, and integers of more than 1000 digits.
 *
 * <p>Worked example. {@code "tfv 1\nr R1 Kop S706274 Kdata XFF\n"} fails: XFF is
 * uppercase hex (and odd length). {@code "bad header\n"} fails: the first line
 * must be exactly {@code tfv 1}.
 */
public final class Tfv {
  /** The exact first line of every TFV file. */
  public static final String HEADER = "tfv 1";

  /** Deepest container nesting accepted (outermost is 1). */
  public static final int MAX_DEPTH = 64;

  /** Digits of an integer token (sign excluded). */
  public static final int MAX_INT_DIGITS = 1000;

  /** Bytes (chars) of a document. */
  public static final int MAX_INPUT = 1 << 20;

  private Tfv() {}

  /** Every rejection. */
  public static final class InvalidTfvException extends RuntimeException {
    private static final long serialVersionUID = 1L;

    public InvalidTfvException(String message) {
      super("invalid TFV: " + message);
    }
  }

  private static String q(String s) {
    final int limit = 64;
    String shown = s.length() > limit ? s.substring(0, limit) + "..." : s;
    return "\"" + shown.replace("\\", "\\\\").replace("\"", "\\\"") + "\"";
  }

  /** True when {@code text} is a well-formed TFV v1 document. Never throws. */
  public static boolean isValid(String text) {
    try {
      validate(text);
      return true;
    } catch (InvalidTfvException e) {
      return false;
    }
  }

  /**
   * Returns normally when {@code text} is a well-formed TFV v1 document and
   * throws {@link InvalidTfvException} otherwise. Reserves no memory from a
   * count in the input.
   */
  public static void validate(String text) {
    if (text.length() > MAX_INPUT) {
      throw new InvalidTfvException("input of " + text.length() + " chars exceeds " + MAX_INPUT);
    }
    try {
      StandardCharsets.UTF_8
          .newEncoder()
          .onMalformedInput(CodingErrorAction.REPORT)
          .onUnmappableCharacter(CodingErrorAction.REPORT)
          .encode(java.nio.CharBuffer.wrap(text));
    } catch (CharacterCodingException e) {
      throw new InvalidTfvException("document is not valid UTF-8");
    }
    String[] lines = text.split("\n", -1);
    int count = lines.length;
    if (lines[count - 1].isEmpty()) {
      count--; // the final newline is optional
    }
    if (count == 0 || !lines[0].equals(HEADER)) {
      throw new InvalidTfvException("the first line must be exactly " + q(HEADER));
    }
    Set<String> seenIds = new HashSet<>();
    for (int n = 1; n < count; n++) {
      String line = lines[n];
      if (line.isEmpty() || line.charAt(0) == '#') {
        continue; // blank and comment lines are skipped whole
      }
      String id;
      try {
        id = validateRecord(line);
      } catch (InvalidTfvException e) {
        throw new InvalidTfvException("line " + (n + 1) + ": " + e.getMessage().substring("invalid TFV: ".length()));
      }
      if (!seenIds.add(id)) {
        throw new InvalidTfvException("line " + (n + 1) + ": duplicate record id " + q(id));
      }
    }
  }

  private static boolean isIdChar(char c) {
    return (c >= 'a' && c <= 'z')
        || (c >= 'A' && c <= 'Z')
        || (c >= '0' && c <= '9')
        || c == '.'
        || c == '_'
        || c == '/'
        || c == '-';
  }

  private static boolean validId(String s) {
    if (s.isEmpty()) {
      return false;
    }
    for (int i = 0; i < s.length(); i++) {
      if (!isIdChar(s.charAt(i))) {
        return false;
      }
    }
    return true;
  }

  private static boolean validFieldName(String s) {
    if (s.isEmpty() || s.charAt(0) < 'a' || s.charAt(0) > 'z') {
      return false;
    }
    for (int i = 1; i < s.length(); i++) {
      char c = s.charAt(i);
      if (!((c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '_')) {
        return false;
      }
    }
    return true;
  }

  private static final class Reader {
    private final String[] tokens;
    private int pos;

    Reader(String[] tokens, int start) {
      this.tokens = tokens;
      this.pos = start;
    }

    String next() {
      if (pos >= tokens.length) {
        throw new InvalidTfvException("record ends early");
      }
      return tokens[pos++];
    }

    // Returns a deep-equality key: equal for two values exactly when TF01
    // section 3.1 calls them deep-equal. depth is the number of enclosing
    // containers.
    String value(int depth) {
      String tok = next();
      char tag = tok.charAt(0);
      String body = tok.substring(1);
      switch (tag) {
        case 'N', 'T', 'F':
          if (!body.isEmpty()) {
            throw new InvalidTfvException("unexpected body in " + q(tok));
          }
          return String.valueOf(tag);
        case 'I':
          validInt(body);
          return tok;
        case 'D':
          {
            byte[] raw = lowerHexBytes(body);
            if (body.length() != 16 || raw == null) {
              throw new InvalidTfvException("float needs exactly 16 lowercase hex digits: " + q(tok));
            }
            // Every NaN equals every NaN; -0 and +0 stay distinct.
            int exp = ((raw[0] & 0x7f) << 4) | ((raw[1] & 0xff) >> 4);
            boolean mantissaZero = (raw[1] & 0x0f) == 0;
            for (int i = 2; i < raw.length; i++) {
              mantissaZero = mantissaZero && raw[i] == 0;
            }
            return exp == 0x7ff && !mantissaZero ? "Dnan" : tok;
          }
        case 'X':
          if (lowerHexBytes(body) == null) {
            throw new InvalidTfvException("bytes need even-length lowercase hex: " + q(tok));
          }
          return tok;
        case 'S':
          {
            byte[] raw = lowerHexBytes(body);
            if (raw == null) {
              throw new InvalidTfvException("text needs even-length lowercase hex: " + q(tok));
            }
            try {
              StandardCharsets.UTF_8
                  .newDecoder()
                  .onMalformedInput(CodingErrorAction.REPORT)
                  .onUnmappableCharacter(CodingErrorAction.REPORT)
                  .decode(ByteBuffer.wrap(raw));
            } catch (CharacterCodingException e) {
              throw new InvalidTfvException("text is not strict UTF-8: " + q(tok));
            }
            return tok;
          }
        case 'L', 'M':
          return container(tag, body, depth);
        default:
          throw new InvalidTfvException("unknown tag in " + q(tok));
      }
    }

    private String container(char tag, String body, int depth) {
      if (depth + 1 > MAX_DEPTH) {
        throw new InvalidTfvException("nesting deeper than " + MAX_DEPTH);
      }
      int count = canonicalNat(body);
      if (tag == 'L') {
        StringBuilder b = new StringBuilder("L[");
        for (int i = 0; i < count; i++) {
          b.append(value(depth + 1)).append(',');
        }
        return b.append(']').toString();
      }
      Set<String> keys = new HashSet<>();
      List<String> entries = new ArrayList<>();
      for (int i = 0; i < count; i++) {
        String k = value(depth + 1);
        String v = value(depth + 1);
        if (!keys.add(k)) {
          throw new InvalidTfvException("duplicate map key");
        }
        entries.add("(" + k + "=>" + v + ")");
      }
      Collections.sort(entries);
      return "M{" + String.join("", entries) + "}";
    }
  }

  private static String validateRecord(String line) {
    if (line.indexOf('\r') >= 0) {
      throw new InvalidTfvException("carriage return in a record line");
    }
    String[] tokens = line.split(" ", -1);
    for (String t : tokens) {
      if (t.isEmpty()) {
        throw new InvalidTfvException("leading, trailing or doubled space");
      }
    }
    if (tokens.length < 2) {
      throw new InvalidTfvException("a record needs an id and R<n>");
    }
    String id = tokens[0];
    if (!validId(id)) {
      throw new InvalidTfvException("bad record id " + q(id));
    }
    if (tokens[1].charAt(0) != 'R') {
      throw new InvalidTfvException("expected R<n>, got " + q(tokens[1]));
    }
    int count = canonicalNat(tokens[1].substring(1));
    Reader r = new Reader(tokens, 2);
    Set<String> seen = new HashSet<>();
    for (int i = 0; i < count; i++) {
      String keyTok = r.next();
      if (keyTok.charAt(0) != 'K' || !validFieldName(keyTok.substring(1))) {
        throw new InvalidTfvException("bad field key " + q(keyTok));
      }
      if (!seen.add(keyTok)) {
        throw new InvalidTfvException("duplicate field " + q(keyTok.substring(1)));
      }
      r.value(0);
    }
    if (r.pos != tokens.length) {
      throw new InvalidTfvException("trailing tokens after the last field");
    }
    return id;
  }

  // 0 or digits without a leading zero. A count too large for the tokens present
  // is rejected later as "record ends early", and no allocation depends on it.
  private static int canonicalNat(String s) {
    if (s.isEmpty()) {
      throw new InvalidTfvException("empty count");
    }
    for (int i = 0; i < s.length(); i++) {
      if (s.charAt(i) < '0' || s.charAt(i) > '9') {
        throw new InvalidTfvException("non-digit in count " + q(s));
      }
    }
    if (s.length() > 1 && s.charAt(0) == '0') {
      throw new InvalidTfvException("leading zero in count " + q(s));
    }
    if (s.length() > 9) {
      throw new InvalidTfvException("count " + q(s) + " too large");
    }
    return Integer.parseInt(s);
  }

  private static byte[] lowerHexBytes(String s) {
    if (s.length() % 2 != 0) {
      return null;
    }
    byte[] out = new byte[s.length() / 2];
    for (int i = 0; i < s.length(); i += 2) {
      int hi = hexVal(s.charAt(i));
      int lo = hexVal(s.charAt(i + 1));
      if (hi < 0 || lo < 0) {
        return null;
      }
      out[i / 2] = (byte) (hi << 4 | lo);
    }
    return out;
  }

  private static int hexVal(char c) {
    if (c >= '0' && c <= '9') {
      return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
      return c - 'a' + 10;
    }
    return -1;
  }

  private static void validInt(String body) {
    boolean neg = body.startsWith("-");
    String digits = neg ? body.substring(1) : body;
    if (digits.isEmpty()) {
      throw new InvalidTfvException("empty integer");
    }
    for (int i = 0; i < digits.length(); i++) {
      if (digits.charAt(i) < '0' || digits.charAt(i) > '9') {
        throw new InvalidTfvException("non-digit in integer " + q(body));
      }
    }
    if (digits.charAt(0) == '0' && (digits.length() > 1 || neg)) {
      throw new InvalidTfvException("non-canonical integer " + q(body));
    }
    if (digits.length() > MAX_INT_DIGITS) {
      throw new InvalidTfvException("integer of " + digits.length() + " digits exceeds " + MAX_INT_DIGITS);
    }
  }
}
