package com.codingadventures.testbootstrap.corpusjson;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class StrictJsonTest {
  private static Object parse(String s) {
    return StrictJson.parse(s.getBytes(StandardCharsets.UTF_8));
  }

  @Test
  void parsesEveryValueKind() {
    Object v = parse(" {\"a\": [1, -2.5e+3, true, false, null, \"x\\n\\u00e9\\\"\\\\\\/\\b\\f\\r\\t\"], \"b\": {}, \"c\": []} ");
    Map<?, ?> m = (Map<?, ?>) v;
    List<?> a = (List<?>) m.get("a");
    assertEquals(new StrictJson.JsonNumber("1"), a.get(0));
    assertEquals(new StrictJson.JsonNumber("-2.5e+3"), a.get(1));
    assertEquals(Boolean.TRUE, a.get(2));
    assertEquals(Boolean.FALSE, a.get(3));
    assertNull(a.get(4));
    assertEquals("x\n\u00e9\"\\/\b\f\r\t", a.get(5));
    assertEquals(Map.of(), m.get("b"));
    assertEquals(List.of(), m.get("c"));
    assertEquals(List.copyOf(m.keySet()), List.of("a", "b", "c"), "insertion order is kept");
  }

  @Test
  void numbersKeepTheirFullDigits() {
    StrictJson.JsonNumber n = (StrictJson.JsonNumber) parse("123456789012345678901234567890");
    assertEquals(new BigInteger("123456789012345678901234567890"), new BigInteger(n.text()));
    assertEquals("0", ((StrictJson.JsonNumber) parse("0")).text());
    assertEquals("0.5", ((StrictJson.JsonNumber) parse("0.5")).text());
    assertEquals("1E2", ((StrictJson.JsonNumber) parse("1E2")).text());
    assertEquals("1e-2", ((StrictJson.JsonNumber) parse("1e-2")).text());
  }

  @Test
  void refusesEverythingLenientParsersAccept() {
    String[] bad = {
      "", "   ", "nope", "tru", "nul", "fals", "[1,]", "[,1]", "{\"a\":1,}", "{,}", "{\"a\"}", "{\"a\":}", "{a:1}", "{1:1}",
      "[1 2]", "{\"a\":1 \"b\":2}", "'a'", "\"unterminated", "\"bad \\q escape\"", "\"\\u12\"", "\"\\u12zz\"", "\"\\",
      "\"raw\ttab\"", "\"raw\nnewline\"", "01", "-", "-01", "1.", "1.e2", ".5", "+1", "1e", "1e+", "NaN", "Infinity",
      "[1] [2]", "[1]x", "/*c*/[1]", "{\"a\":1,\"a\":2}", "[", "{", "[1", "{\"a\":1", "]", "}", "\"\\uD83D\\u12G4\"", "\"\\U0041\""
    };
    for (String s : bad) {
      assertThrows(StrictJson.JsonException.class, () -> parse(s), "accepted: " + s);
    }
  }

  @Test
  void nestingAndSizeAreCapped() {
    parse("[".repeat(StrictJson.MAX_DEPTH) + "]".repeat(StrictJson.MAX_DEPTH));
    StrictJson.JsonException e =
        assertThrows(StrictJson.JsonException.class, () -> parse("[".repeat(StrictJson.MAX_DEPTH + 5) + "]".repeat(StrictJson.MAX_DEPTH + 5)));
    assertTrue(e.getMessage().contains("nesting"));
    assertThrows(StrictJson.JsonException.class, () -> parse("{\"a\":".repeat(100000)));
    assertThrows(StrictJson.JsonException.class, () -> StrictJson.parse(new byte[StrictJson.MAX_BYTES + 1]));
  }

  @Test
  void invalidUtf8IsRefused() {
    assertThrows(StrictJson.JsonException.class, () -> StrictJson.parse(new byte[] {'"', (byte) 0xff, '"'}));
    assertThrows(StrictJson.JsonException.class, () -> StrictJson.parse(new byte[] {'"', (byte) 0xc3, '"'}));
  }

  @Test
  void duplicateKeysAreNamedAndClipped() {
    String longKey = "k".repeat(500);
    StrictJson.JsonException e = assertThrows(StrictJson.JsonException.class, () -> parse("{\"" + longKey + "\":1,\"" + longKey + "\":2}"));
    assertTrue(e.getMessage().contains("duplicate"));
    assertTrue(e.getMessage().length() < 200);
  }

  @Test
  void clip() {
    assertEquals("abc", Clip.clip("abc", 3));
    assertEquals("ab...", Clip.clip("abcdef", 2));
    // Never splits a surrogate pair.
    String s = "a\uD83D\uDE00b";
    assertEquals("a...", Clip.clip(s, 2));
    assertFalse(Clip.clip(s, 3).contains("\uD83D") && !Clip.clip(s, 3).contains("\uDE00"));
  }

  // Security regression (review item 4): 8 MiB of tiny tokens amplified to ~300 MB of heap.
  @Test
  void totalValueCountIsCapped() {
    int limit = StrictJson.MAX_NODES;
    // One array plus (limit - 1) numbers is exactly at the limit.
    StringBuilder ok = new StringBuilder("[");
    for (int i = 0; i < limit - 1; i++) {
      ok.append(i == 0 ? "0" : ",0");
    }
    assertEquals(limit - 1, ((List<?>) parse(ok.append("]").toString())).size());
    StringBuilder over = new StringBuilder("[");
    for (int i = 0; i < limit; i++) {
      over.append(i == 0 ? "0" : ",0");
    }
    StrictJson.JsonException e = assertThrows(StrictJson.JsonException.class, () -> parse(over.append("]").toString()));
    assertTrue(e.getMessage().contains("values in one document"));
    assertTrue(e.getMessage().length() < 100);
    // The same for a maximal-size document of tiny tokens.
    byte[] big = ("[" + "0,".repeat(4_000_000) + "0]").getBytes(StandardCharsets.UTF_8);
    assertThrows(StrictJson.JsonException.class, () -> StrictJson.parse(big));
  }
}
