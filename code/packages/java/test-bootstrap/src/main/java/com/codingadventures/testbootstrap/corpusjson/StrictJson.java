package com.codingadventures.testbootstrap.corpusjson;

import java.nio.ByteBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * A small, strict JSON reader (VER05e section 6).
 *
 * <p>The JDK ships no JSON library and this package takes no third-party
 * dependencies, so the corpus loader owns its parser. "Strict" is the point: a
 * conformance fixture is <em>data</em>, and the usual lenient parsers quietly
 * paper over exactly the corruption a conformance check exists to notice.
 *
 * <ul>
 *   <li>duplicate object keys are refused (a lenient parser keeps the last one,
 *       so a fixture could say two different things and pass);
 *   <li>trailing data after the document is refused;
 *   <li>nesting deeper than {@link #MAX_DEPTH} is refused (a hostile
 *       million-deep list would overflow the stack of a recursive reader);
 *   <li>documents over {@link #MAX_BYTES} are refused;
 *   <li>invalid UTF-8, raw control characters in strings, leading zeros,
 *       {@code +1}, {@code NaN}, comments and trailing commas are refused.
 * </ul>
 *
 * Values become {@code Map<String,Object>} (insertion ordered), {@code
 * List<Object>}, {@code String}, {@code Boolean}, {@code null} and
 * {@link JsonNumber} (the literal text of a number, never a double: a 30-digit
 * integer must not lose its low digits).
 */
public final class StrictJson {
  /** JSON nesting limit. */
  public static final int MAX_DEPTH = 64;

  /**
   * Total values (objects, arrays, strings, numbers, literals) one document may
   * hold. 8 MiB of {@code 0,} would otherwise become ~4 million boxed nodes
   * (hundreds of MB of heap).
   */
  public static final int MAX_NODES = 200_000;

  /** Document size limit, in bytes. */
  public static final int MAX_BYTES = 8 << 20;

  private StrictJson() {}

  /** A JSON number kept as its literal text. */
  public record JsonNumber(String text) {}

  /** Thrown for any malformed or over-limit document. */
  public static final class JsonException extends RuntimeException {
    private static final long serialVersionUID = 1L;

    public JsonException(String message) {
      super(message);
    }
  }

  /** Parses a UTF-8 document. */
  public static Object parse(byte[] data) {
    if (data.length > MAX_BYTES) {
      throw new JsonException(data.length + " bytes exceeds " + MAX_BYTES);
    }
    String text;
    try {
      text =
          StandardCharsets.UTF_8
              .newDecoder()
              .onMalformedInput(CodingErrorAction.REPORT)
              .onUnmappableCharacter(CodingErrorAction.REPORT)
              .decode(ByteBuffer.wrap(data))
              .toString();
    } catch (CharacterCodingException e) {
      throw new JsonException("document is not valid UTF-8");
    }
    return new Reader(text).document();
  }

  private static final class Reader {
    private final String s;
    private int pos;
    private int nodes;

    Reader(String s) {
      this.s = s;
    }

    Object document() {
      skipWs();
      Object v = value(0);
      skipWs();
      if (pos != s.length()) {
        throw new JsonException("trailing data after the document");
      }
      return v;
    }

    private void skipWs() {
      while (pos < s.length()) {
        char c = s.charAt(pos);
        if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
          pos++;
        } else {
          break;
        }
      }
    }

    private JsonException err(String what) {
      return new JsonException(what + " at offset " + pos);
    }

    private Object value(int depth) {
      if (depth > MAX_DEPTH) {
        throw new JsonException("nesting deeper than " + MAX_DEPTH);
      }
      if (++nodes > MAX_NODES) {
        throw new JsonException("more than " + MAX_NODES + " values in one document");
      }
      if (pos >= s.length()) {
        throw err("unexpected end of input");
      }
      char c = s.charAt(pos);
      switch (c) {
        case '{':
          return object(depth);
        case '[':
          return array(depth);
        case '"':
          return string();
        case 't':
          literal("true");
          return Boolean.TRUE;
        case 'f':
          literal("false");
          return Boolean.FALSE;
        case 'n':
          literal("null");
          return null;
        default:
          if (c == '-' || (c >= '0' && c <= '9')) {
            return number();
          }
          throw err("unexpected character");
      }
    }

    private void literal(String word) {
      if (!s.startsWith(word, pos)) {
        throw err("bad literal");
      }
      pos += word.length();
    }

    private Object object(int depth) {
      pos++; // {
      Map<String, Object> out = new LinkedHashMap<>();
      skipWs();
      if (peek() == '}') {
        pos++;
        return out;
      }
      while (true) {
        skipWs();
        if (peek() != '"') {
          throw err("object key is not a string");
        }
        String key = string();
        if (out.containsKey(key)) {
          throw new JsonException("duplicate JSON key \"" + Clip.clip(key, 64) + "\"");
        }
        skipWs();
        if (peek() != ':') {
          throw err("expected ':'");
        }
        pos++;
        skipWs();
        out.put(key, value(depth + 1));
        skipWs();
        char c = peek();
        pos++;
        if (c == '}') {
          return out;
        }
        if (c != ',') {
          throw err("expected ',' or '}'");
        }
      }
    }

    private Object array(int depth) {
      pos++; // [
      List<Object> out = new ArrayList<>();
      skipWs();
      if (peek() == ']') {
        pos++;
        return out;
      }
      while (true) {
        skipWs();
        out.add(value(depth + 1));
        skipWs();
        char c = peek();
        pos++;
        if (c == ']') {
          return out;
        }
        if (c != ',') {
          throw err("expected ',' or ']'");
        }
      }
    }

    private char peek() {
      if (pos >= s.length()) {
        throw err("unexpected end of input");
      }
      return s.charAt(pos);
    }

    private String string() {
      pos++; // opening quote
      StringBuilder b = new StringBuilder();
      while (true) {
        if (pos >= s.length()) {
          throw err("unterminated string");
        }
        char c = s.charAt(pos++);
        if (c == '"') {
          return b.toString();
        }
        if (c < 0x20) {
          throw err("raw control character in string");
        }
        if (c != '\\') {
          b.append(c);
          continue;
        }
        if (pos >= s.length()) {
          throw err("unterminated escape");
        }
        char e = s.charAt(pos++);
        switch (e) {
          case '"', '\\', '/' -> b.append(e);
          case 'b' -> b.append('\b');
          case 'f' -> b.append('\f');
          case 'n' -> b.append('\n');
          case 'r' -> b.append('\r');
          case 't' -> b.append('\t');
          case 'u' -> {
            if (pos + 4 > s.length()) {
              throw err("short \\u escape");
            }
            int code = 0;
            for (int i = 0; i < 4; i++) {
              int d = Character.digit(s.charAt(pos + i), 16);
              if (d < 0 || s.charAt(pos + i) > 'f') {
                throw err("bad \\u escape");
              }
              code = code * 16 + d;
            }
            pos += 4;
            b.append((char) code);
          }
          default -> throw err("bad escape");
        }
      }
    }

    private Object number() {
      int start = pos;
      if (peek() == '-') {
        pos++;
      }
      char c = peek();
      if (c == '0') {
        pos++;
      } else if (c >= '1' && c <= '9') {
        while (pos < s.length() && Character.isDigit(s.charAt(pos)) && s.charAt(pos) < 0x80) {
          pos++;
        }
      } else {
        throw err("bad number");
      }
      if (pos < s.length() && s.charAt(pos) == '.') {
        pos++;
        digits();
      }
      if (pos < s.length() && (s.charAt(pos) == 'e' || s.charAt(pos) == 'E')) {
        pos++;
        if (pos < s.length() && (s.charAt(pos) == '+' || s.charAt(pos) == '-')) {
          pos++;
        }
        digits();
      }
      return new JsonNumber(s.substring(start, pos));
    }

    private void digits() {
      int begin = pos;
      while (pos < s.length() && s.charAt(pos) >= '0' && s.charAt(pos) <= '9') {
        pos++;
      }
      if (pos == begin) {
        throw err("digits expected");
      }
    }
  }
}
