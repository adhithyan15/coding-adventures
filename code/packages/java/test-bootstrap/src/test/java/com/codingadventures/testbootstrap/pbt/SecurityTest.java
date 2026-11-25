package com.codingadventures.testbootstrap.pbt;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

/** CWE-400 caps and bounded reports for hostile fixtures (VER05e section 6). */
class SecurityTest {
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

  private static Map<String, Object> genBytes(String id, String count, String maxLen, List<Object> outputs) {
    Map<String, Object> c = new LinkedHashMap<>();
    c.put("id", id);
    c.put("op", "gen_bytes");
    c.put("min_len", intNode("0"));
    c.put("max_len", intNode(maxLen));
    c.put("seed", intNode("1"));
    c.put("count", intNode(count));
    c.put("outputs", outputs);
    return c;
  }

  private static List<String> problems(Map<String, Object> c) {
    return Corpus.evaluate(new ArrayList<>(List.of(c)), -1).problems().stream().filter(p -> p.startsWith(c.get("id") + ":")).toList();
  }

  @Test
  void generationBudgetBoundsCountTimesWidth() {
    // 10,000 x 10,000 passes the per-field caps (each <= 10,000) but not the total budget.
    List<String> p = problems(genBytes("hostile/bytes", "10000", "10000", new ArrayList<>(java.util.Collections.nCopies(10000, bytesNode("")))));
    assertEquals(1, p.size(), p.toString());
    assertTrue(p.get(0).contains("generation budget"));

    Map<String, Object> bounded = new LinkedHashMap<>();
    bounded.put("id", "hostile/bounded");
    bounded.put("op", "gen_bounded");
    bounded.put("min", intNode("0"));
    bounded.put("max", intNode("9".repeat(1000)));
    bounded.put("seed", intNode("1"));
    bounded.put("count", intNode("10000"));
    bounded.put("outputs", new ArrayList<>(java.util.Collections.nCopies(10000, intNode("0"))));
    List<String> q = problems(bounded);
    assertEquals(1, q.size(), q.toString());
    assertTrue(q.get(0).contains("generation budget"));
  }

  @Test
  void sizesAboveTheCapAreProblemsNotHangs() {
    assertTrue(problems(genBytes("hostile/count", "1000000000", "1", new ArrayList<>())).get(0).contains("outside [0, 10000]"));
    assertTrue(problems(genBytes("hostile/neg", "-1", "1", new ArrayList<>())).get(0).contains("outside"));
    assertTrue(problems(genBytes("hostile/maxlen", "1", "1000000000", new ArrayList<>(List.of(bytesNode(""))))).get(0).contains("outside"));
    // outputs length must be exactly count, checked before generating anything.
    assertTrue(problems(genBytes("hostile/outputs", "3", "1", new ArrayList<>())).get(0).contains("exactly count"));

    Map<String, Object> filter = new LinkedHashMap<>();
    filter.put("id", "hostile/filter");
    filter.put("op", "filter_exhausted");
    filter.put("max_retries", intNode("1000000000"));
    filter.put("verdict", "fail");
    filter.put("failure_id", "property-exhausted");
    assertTrue(problems(filter).get(0).contains("outside"));
    filter.put("max_retries", intNode("0"));
    assertTrue(problems(filter).get(0).contains("at least 1"));
  }

  @Test
  void problemTextIsBoundedForHostileFixtures() {
    String huge = "A".repeat(2_000_000);
    Map<String, Object> c = new LinkedHashMap<>();
    c.put("id", huge);
    c.put("op", huge);
    List<String> p = Corpus.evaluate(new ArrayList<>(List.of(c)), -1).problems();
    for (String line : p) {
      assertTrue(line.length() <= 403, "problem of " + line.length() + " chars");
    }
    Map<String, Object> d = genBytes("x", "1", "1", new ArrayList<>(List.of(bytesNode("00"))));
    d.put("seed", intNode("9".repeat(900)));
    d.put("min_len", huge);
    for (String line : Corpus.evaluate(new ArrayList<>(List.of(d)), -1).problems()) {
      assertTrue(line.length() <= 403);
    }
    Map<String, Object> e = new LinkedHashMap<>();
    e.put("id", "e");
    e.put("op", "shrink_int");
    for (String f : List.of("initial", "target", "min", "max", "expected_minimal", "expected_steps")) {
      e.put(f, intNode("1"));
    }
    e.put("property", huge);
    for (String line : Corpus.evaluate(new ArrayList<>(List.of(e)), -1).problems()) {
      assertTrue(line.length() <= 403);
    }
  }

  @Test
  void badPropertiesAndTypesAreProblems() {
    Map<String, Object> e = new LinkedHashMap<>();
    e.put("id", "e");
    e.put("op", "shrink_int");
    for (String f : List.of("initial", "target", "min", "max", "expected_minimal", "expected_steps")) {
      e.put(f, intNode("1"));
    }
    e.put("property", "no_such_property");
    assertTrue(problems(e).get(0).contains("unknown property"));
    e.put("property", 5);
    assertFalse(problems(e).isEmpty());
    e.put("property", "int_le_100");
    e.put("initial", true); // not an int
    assertTrue(problems(e).get(0).contains("must be an int"));
    e.put("initial", intNode("1")); // does not falsify: caller bug surfaces as a problem
    assertTrue(problems(e).get(0).contains("could not be evaluated"));

    Map<String, Object> l = new LinkedHashMap<>();
    l.put("id", "l");
    l.put("op", "shrink_list");
    l.put("initial", "not a list");
    l.put("elem_target", intNode("0"));
    l.put("property", "list_sum_le_20");
    l.put("expected_minimal", new ArrayList<>());
    l.put("expected_steps", intNode("0"));
    assertTrue(problems(l).get(0).contains("must be a list"));
    l.put("initial", new ArrayList<>(List.of(true)));
    assertTrue(problems(l).get(0).contains("must be an int"));
    l.put("initial", new ArrayList<>());
    l.put("property", "nope");
    assertTrue(problems(l).get(0).contains("unknown property"));
    l.put("property", "list_sum_le_20");
    l.put("elem_target", true);
    l.put("initial", new ArrayList<>(List.of(intNode("99"))));
    assertTrue(problems(l).get(0).contains("must be an int"));

    Map<String, Object> b = new LinkedHashMap<>();
    b.put("id", "b");
    b.put("op", "shrink_bytes");
    b.put("initial", intNode("1"));
    b.put("property", "bytes_len_lt_5");
    b.put("expected_minimal", bytesNode(""));
    b.put("expected_steps", intNode("0"));
    assertTrue(problems(b).get(0).contains("must be bytes"));
    b.put("property", "nope");
    assertTrue(problems(b).get(0).contains("unknown property"));
    b.put("property", "bytes_no_zero_byte");
    b.put("initial", bytesNode("010203"));
    assertTrue(problems(b).get(0).contains("could not be evaluated"));
    b.put("initial", bytesNode("010200"));
    List<String> ok = problems(b);
    assertTrue(ok.stream().anyMatch(s -> s.contains("minimal mismatch")), ok.toString());

    Map<String, Object> r = new LinkedHashMap<>();
    r.put("id", "r");
    r.put("op", "replay_consistency");
    r.put("seed", true);
    r.put("property", "int_le_100");
    r.put("initial_fail", intNode("1"));
    r.put("expected_minimal", intNode("1"));
    assertTrue(problems(r).get(0).contains("seed must be an int"));
    r.put("seed", intNode("-42"));
    r.put("property", "nope");
    assertTrue(problems(r).get(0).contains("unknown property"));
    r.put("property", "int_le_100");
    assertFalse(problems(r).isEmpty());
    r.put("seed", intNode("1"));
    r.put("property", "int_is_even");
    assertFalse(problems(r).isEmpty());

    Map<String, Object> g = new LinkedHashMap<>();
    g.put("id", "g");
    g.put("op", "gen_primitive");
    g.put("type", true);
    g.put("seed", intNode("1"));
    g.put("count", intNode("0"));
    g.put("outputs", new ArrayList<>());
    assertTrue(problems(g).get(0).contains("type must be a string"));
    g.put("type", "float");
    g.put("count", intNode("1"));
    g.put("outputs", new ArrayList<>(List.of(true)));
    assertTrue(problems(g).get(0).contains("unknown primitive type"));
    g.put("count", true);
    assertTrue(problems(g).get(0).contains("count must be an int"));

    Map<String, Object> t = new LinkedHashMap<>();
    t.put("id", "t");
    t.put("op", "corpus_reject");
    t.put("text", true);
    assertTrue(problems(t).get(0).contains("text must be a string"));
    t.put("text", "tfv 1\n");
    assertTrue(problems(t).get(0).contains("accepted by the strict parser"));

    Map<String, Object> f = new LinkedHashMap<>();
    f.put("id", "f");
    f.put("op", "failure_id_valid");
    f.put("input", true);
    f.put("valid", false);
    assertEquals(List.of(), problems(f));
    f.put("valid", true);
    assertFalse(problems(f).isEmpty());
  }

  @Test
  void anEvaluatorThatThrowsIsAProblemForThatCase() {
    Hooks.newRng = seed -> { throw new IllegalStateException("engine exploded"); };
    try {
      Map<String, Object> g = new LinkedHashMap<>();
      g.put("id", "boom");
      g.put("op", "gen_primitive");
      g.put("type", "bool");
      g.put("seed", intNode("1"));
      g.put("count", intNode("1"));
      g.put("outputs", new ArrayList<>(List.of(true)));
      List<String> p = problems(g);
      assertEquals(1, p.size());
      assertTrue(p.get(0).contains("could not be evaluated"));
      assertTrue(p.get(0).contains("engine exploded"));
    } finally {
      Hooks.reset();
    }
  }

  @Test
  void deepHandBuiltTreesAreProblemsNotCrashes() {
    Object deep = new ArrayList<>();
    Object cur = deep;
    for (int i = 0; i < 50_000; i++) {
      List<Object> next = new ArrayList<>();
      @SuppressWarnings("unchecked")
      List<Object> c = (List<Object>) cur;
      c.add(next);
      cur = next;
    }
    Map<String, Object> g = genBytes("deep", "1", "1", new ArrayList<>(List.of(deep)));
    List<String> p = problems(g);
    assertEquals(1, p.size(), p.toString());
    assertTrue(p.get(0).contains("could not be evaluated"));
  }

  @Test
  void hugeDocumentsAreRefusedBeforeEvaluation() {
    List<Object> many = new ArrayList<>(java.util.Collections.nCopies(10_001, "x"));
    List<String> p = Corpus.evaluate(many, -1).problems();
    assertTrue(p.stream().anyMatch(s -> s.contains("exceeds the cap")));
  }
}
