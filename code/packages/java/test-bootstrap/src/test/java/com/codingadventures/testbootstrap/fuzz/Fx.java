package com.codingadventures.testbootstrap.fuzz;

import com.codingadventures.testbootstrap.fuzz.Mutate.Endian;
import java.io.IOException;
import java.math.BigInteger;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** Shared test helpers: the committed fixture, hex, big integers, deep copies. */
final class Fx {
  private Fx() {}

  static Path repoRoot() {
    Path dir = Path.of("").toAbsolutePath();
    while (dir != null) {
      if (Files.isRegularFile(dir.resolve("code/specs/fixtures/fuzz-v1/cases.json"))) {
        return dir;
      }
      dir = dir.getParent();
    }
    throw new AssertionError("could not find code/specs/fixtures/fuzz-v1/cases.json above the working directory");
  }

  static Path fixturePath() {
    return repoRoot().resolve("code/specs/fixtures/fuzz-v1/cases.json");
  }

  static List<Object> loadCases() {
    try {
      return Corpus.parse(Files.readAllBytes(fixturePath()));
    } catch (IOException e) {
      throw new AssertionError(e);
    }
  }

  static byte[] hx(String s) {
    byte[] out = new byte[s.length() / 2];
    for (int i = 0; i < out.length; i++) {
      out[i] = (byte) Integer.parseInt(s.substring(2 * i, 2 * i + 2), 16);
    }
    return out;
  }

  static String hex(byte[] b) {
    StringBuilder s = new StringBuilder();
    for (byte x : b) {
      s.append(String.format("%02x", x));
    }
    return s.toString();
  }

  static BigInteger bg(String s) {
    return new BigInteger(s);
  }

  static Endian endian(String s) {
    return s.equals("big") ? Endian.BIG : Endian.LITTLE;
  }

  static Map<String, Object> intNode(String v) {
    Map<String, Object> m = new LinkedHashMap<>();
    m.put("kind", "int");
    m.put("value", v);
    return m;
  }

  static Map<String, Object> bytesNode(String hex) {
    Map<String, Object> m = new LinkedHashMap<>();
    m.put("kind", "bytes");
    m.put("hex", hex);
    return m;
  }

  @SuppressWarnings("unchecked")
  static Object deepCopy(Object node) {
    if (node instanceof Map<?, ?> m) {
      Map<String, Object> out = new LinkedHashMap<>();
      for (Map.Entry<String, Object> e : ((Map<String, Object>) m).entrySet()) {
        out.put(e.getKey(), deepCopy(e.getValue()));
      }
      return out;
    }
    if (node instanceof List<?> l) {
      List<Object> out = new ArrayList<>();
      for (Object item : l) {
        out.add(deepCopy(item));
      }
      return out;
    }
    return node;
  }

  static List<Object> copyOfCases(List<Object> cases) {
    List<Object> out = new ArrayList<>();
    for (Object c : cases) {
      out.add(deepCopy(c));
    }
    return out;
  }

  @SuppressWarnings("unchecked")
  static Map<String, Object> caseAt(List<Object> cases, int i) {
    return (Map<String, Object>) cases.get(i);
  }

  @SuppressWarnings("unchecked")
  static Map<String, Object> firstOf(List<Object> cases, String op) {
    for (Object c : cases) {
      if (op.equals(((Map<String, Object>) c).get("op"))) {
        return (Map<String, Object>) deepCopy(c);
      }
    }
    throw new AssertionError("no case of op " + op);
  }

  /** The fixture with {@code replacement} swapped in for the case that has its id. */
  static List<Object> swapped(List<Object> cases, Map<String, Object> replacement) {
    List<Object> out = new ArrayList<>();
    for (Object c : cases) {
      boolean target = c instanceof Map<?, ?> m && replacement.get("id").equals(m.get("id"));
      out.add(target ? replacement : deepCopy(c));
    }
    return out;
  }

  static List<String> problems(List<Object> cases) {
    return Corpus.evaluate(cases, -1).problems();
  }

  static boolean anyContains(List<String> problems, String fragment) {
    return problems.stream().anyMatch(p -> p.contains(fragment));
  }
}
