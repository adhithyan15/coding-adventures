package com.codingadventures.testbootstrap.corpusjson;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigInteger;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class ValuesTest {
  private static Map<String, Object> intNode(String v) {
    Map<String, Object> m = new LinkedHashMap<>();
    m.put("kind", "int");
    m.put("value", v);
    return m;
  }

  private static Map<String, Object> bytesNode(String hex) {
    Map<String, Object> m = new LinkedHashMap<>();
    m.put("kind", "bytes");
    m.put("hex", hex);
    return m;
  }

  @Test
  void canonicalIntAndHex() {
    for (String ok : new String[] {"0", "7", "-7", "10", "-10", "123456789012345678901234567890"}) {
      assertTrue(Values.isCanonicalInt(ok), ok);
    }
    for (String bad : new String[] {"", "-", "-0", "00", "01", "+1", "1_0", " 1", "1 ", "1.0", "0x1", "\u0661"}) {
      assertFalse(Values.isCanonicalInt(bad), bad);
    }
    assertTrue(Values.isLowercaseHex(""));
    assertTrue(Values.isLowercaseHex("00ff"));
    for (String bad : new String[] {"0", "FF", "0G", "zz", "0x"}) {
      assertFalse(Values.isLowercaseHex(bad), bad);
    }
  }

  @Test
  void decodesEveryKind() {
    assertEquals(BigInteger.valueOf(-12), Values.decode(intNode("-12")));
    assertArrayEquals(new byte[] {0, (byte) 0xff}, (byte[]) Values.decode(bytesNode("00ff")));
    assertEquals(true, Values.decode(true));
    assertEquals("text", Values.decode("text"));
    assertEquals(List.of(BigInteger.ONE, true), Values.decode(List.of(intNode("1"), true)));
  }

  @Test
  void refusesMalformedNodes() {
    List<Object> bad = new ArrayList<>();
    bad.add(intNode("01"));
    bad.add(intNode("+1"));
    bad.add(bytesNode("FF"));
    bad.add(bytesNode("f"));
    bad.add(Map.of("kind", "int"));
    bad.add(Map.of("kind", "int", "value", "1", "extra", "x"));
    bad.add(Map.of("kind", "int", "value", 5));
    bad.add(Map.of("kind", "bytes", "hex", 5));
    bad.add(Map.of("kind", "float", "value", "1"));
    bad.add(Map.of());
    bad.add(intNode("1".repeat(Values.MAX_INT_DIGITS + 1)));
    bad.add(bytesNode("00".repeat(Values.MAX_SIZE + 1)));
    bad.add(new ArrayList<>(java.util.Collections.nCopies(Values.MAX_SIZE + 1, true)));
    bad.add(7);
    bad.add(null);
    bad.add(new StrictJson.JsonNumber("1"));
    for (Object node : bad) {
      assertThrows(Values.CaseException.class, () -> Values.decode(node), String.valueOf(node).substring(0, Math.min(40, String.valueOf(node).length())));
    }
    // The limit itself is fine.
    Values.decode(intNode("-" + "1".repeat(Values.MAX_INT_DIGITS)));
  }

  @Test
  void deepHandBuiltTreesAreProblemsNotStackOverflows() {
    Object deep = new ArrayList<>();
    Object cur = deep;
    for (int i = 0; i < 100_000; i++) {
      List<Object> next = new ArrayList<>();
      @SuppressWarnings("unchecked")
      List<Object> c = (List<Object>) cur;
      c.add(next);
      cur = next;
    }
    assertThrows(Values.CaseException.class, () -> Values.decode(deep));
  }

  @Test
  void sameDoesNotConflateKinds() {
    assertTrue(Values.same(true, true));
    assertFalse(Values.same(true, false));
    assertFalse(Values.same(true, BigInteger.ONE));
    assertFalse(Values.same(BigInteger.ONE, true));
    assertTrue(Values.same("a", "a"));
    assertFalse(Values.same("1", BigInteger.ONE));
    assertTrue(Values.same(BigInteger.TEN, new BigInteger("10")));
    assertFalse(Values.same(BigInteger.TEN, BigInteger.ONE));
    assertTrue(Values.same(new byte[] {1}, new byte[] {1}));
    assertFalse(Values.same(new byte[] {1}, List.of(BigInteger.ONE)));
    assertFalse(Values.same(List.of(BigInteger.ONE), new byte[] {1}));
    assertTrue(Values.same(List.of(BigInteger.ONE, "x"), List.of(BigInteger.ONE, "x")));
    assertFalse(Values.same(List.of(BigInteger.ONE), List.of(BigInteger.ONE, BigInteger.ONE)));
    assertFalse(Values.same(List.of(BigInteger.ONE), List.of(BigInteger.TWO)));
    assertFalse(Values.same(null, null), "an unknown kind is never the same");
    assertFalse(Values.same(1.5, 1.5));
    Object deepA = new ArrayList<>();
    Object deepB = new ArrayList<>();
    Object ca = deepA;
    Object cb = deepB;
    for (int i = 0; i < 200; i++) {
      List<Object> na = new ArrayList<>();
      List<Object> nb = new ArrayList<>();
      @SuppressWarnings("unchecked")
      List<Object> la = (List<Object>) ca;
      @SuppressWarnings("unchecked")
      List<Object> lb = (List<Object>) cb;
      la.add(na);
      lb.add(nb);
      ca = na;
      cb = nb;
    }
    assertFalse(Values.same(deepA, deepB), "depth is capped, deeper is never the same");
  }

  @Test
  void problemTextIsBounded() {
    String huge = "x".repeat(100_000);
    assertTrue(Values.show(huge).length() <= Values.MAX_FRAGMENT_LEN + 3);
    assertTrue(new Values.CaseException(huge).getMessage().length() <= Values.MAX_PROBLEM_LEN + 3);
  }
}
