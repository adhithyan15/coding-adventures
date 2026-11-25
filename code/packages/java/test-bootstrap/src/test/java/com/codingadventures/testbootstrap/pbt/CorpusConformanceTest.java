package com.codingadventures.testbootstrap.pbt;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.codingadventures.testbootstrap.corpusjson.StrictJson;
import com.codingadventures.testbootstrap.prng.SplitMix64;
import java.io.IOException;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

/**
 * Replays the whole committed pbt-v1 corpus and carries its negative controls
 * (VER05e section 6). The fixture is read from its committed path, never a copy.
 */
class CorpusConformanceTest {
  static Path repoFixturePath() {
    Path dir = Path.of("").toAbsolutePath();
    while (dir != null) {
      Path candidate = dir.resolve("code/specs/fixtures/pbt-v1/cases.json");
      if (Files.isRegularFile(candidate)) {
        return candidate;
      }
      dir = dir.getParent();
    }
    throw new AssertionError("could not find code/specs/fixtures/pbt-v1/cases.json above the working directory");
  }

  static List<Object> loadCases() {
    try {
      return Corpus.parse(Files.readAllBytes(repoFixturePath()));
    } catch (IOException e) {
      throw new AssertionError(e);
    }
  }

  @AfterEach
  void resetHooks() {
    Hooks.reset();
  }

  // ---- deep copies and corruption ------------------------------------------

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
    return node; // String, Boolean, JsonNumber, null: immutable
  }

  /** Same shape, different value. */
  @SuppressWarnings("unchecked")
  static Object corruptValue(Object node) {
    if (node instanceof Boolean b) {
      return !b;
    }
    if (node instanceof Map<?, ?> raw) {
      Map<String, Object> m = (Map<String, Object>) raw;
      if ("int".equals(m.get("kind"))) {
        m.put("value", new BigInteger((String) m.get("value")).add(BigInteger.ONE).toString());
      } else {
        String hex = (String) m.get("hex");
        m.put("hex", hex.isEmpty() ? "ff" : hex.substring(2));
      }
      return m;
    }
    throw new AssertionError("cannot corrupt " + node);
  }

  @SuppressWarnings("unchecked")
  static void corruptFirstOutput(Map<String, Object> c, String field) {
    List<Object> list = (List<Object>) c.get(field);
    list.set(0, corruptValue(list.get(0)));
  }

  /** Each variant is one corruption of the case; every one must be noticed. */
  @SuppressWarnings("unchecked")
  static List<java.util.function.Consumer<Map<String, Object>>> corruptions(String op) {
    return switch (op) {
      case "gen_primitive", "gen_bounded", "gen_bytes" -> List.of(c -> corruptFirstOutput(c, "outputs"));
      case "shrink_int", "shrink_refinement", "shrink_bytes" ->
          List.of(
              c -> c.put("expected_minimal", corruptValue(c.get("expected_minimal"))),
              c -> c.put("expected_steps", corruptValue(c.get("expected_steps"))));
      case "shrink_list" ->
          List.of(
              c -> corruptFirstOutput(c, "expected_minimal"),
              c -> c.put("expected_steps", corruptValue(c.get("expected_steps"))));
      case "replay_consistency" ->
          List.of(
              c -> c.put("initial_fail", corruptValue(c.get("initial_fail"))),
              c -> c.put("expected_minimal", corruptValue(c.get("expected_minimal"))));
      case "filter_exhausted" ->
          List.of(c -> c.put("failure_id", "property-panic"), c -> c.put("verdict", "pass"));
      case "failure_id_valid" -> List.of(c -> c.put("valid", corruptValue(c.get("valid"))));
      case "corpus_reject" -> List.of(c -> c.put("text", "tfv 1\nr R1 Kop S706274\n"));
      default -> throw new AssertionError("no corruption defined for op " + op);
    };
  }

  // ---- the positive run ------------------------------------------------------

  @Test
  void corpusConformsToAllTwentyFiveCases() {
    Corpus.Report report = Corpus.evaluate(loadCases());
    assertEquals(List.of(), report.problems());
    assertEquals(25, report.evaluated(), "exactly 25 cases must be fully evaluated");
    assertEquals(25, Corpus.PBTV1_CASE_COUNT);
  }

  @Test
  void everyOpIsPresentInTheFixture() {
    java.util.Set<String> ops = new java.util.TreeSet<>();
    for (Object c : loadCases()) {
      ops.add((String) ((Map<?, ?>) c).get("op"));
    }
    assertEquals(11, ops.size(), "ops: " + ops);
  }

  // ---- negative controls -----------------------------------------------------

  /**
   * Per-op, per-case: corrupt the expected value of EVERY real case in every way
   * its op allows and require the run to fail and name exactly that case. A
   * negative control on a single case proves only that case's path.
   */
  @Test
  void corruptingAnyCaseMakesTheRunFailAndNamesThatCase() {
    List<Object> pristine = loadCases();
    int corrupted = 0;
    for (int i = 0; i < pristine.size(); i++) {
      @SuppressWarnings("unchecked")
      Map<String, Object> original = (Map<String, Object>) pristine.get(i);
      String id = (String) original.get("id");
      String op = (String) original.get("op");
      for (var corruption : corruptions(op)) {
        List<Object> cases = new ArrayList<>();
        for (Object c : pristine) {
          cases.add(deepCopy(c));
        }
        @SuppressWarnings("unchecked")
        Map<String, Object> victim = (Map<String, Object>) cases.get(i);
        corruption.accept(victim);
        assertFalse(deepCopy(original).equals(victim), id + ": the corruption must change the case");
        Corpus.Report report = Corpus.evaluate(cases);
        assertFalse(report.problems().isEmpty(), id + " (" + op + "): corruption went unnoticed");
        for (String p : report.problems()) {
          assertTrue(p.startsWith(id + ":"), id + ": problem names another case or none: " + p);
        }
        corrupted++;
      }
    }
    assertTrue(corrupted >= 25, "every case must have been corrupted at least once, got " + corrupted);
  }

  @Test
  void droppingAnyCaseIsAProblem() {
    List<Object> pristine = loadCases();
    for (int i = 0; i < pristine.size(); i++) {
      List<Object> cases = new ArrayList<>(pristine);
      cases.remove(i);
      Corpus.Report report = Corpus.evaluate(cases);
      assertFalse(report.problems().isEmpty(), "dropping case " + i + " went unnoticed");
      assertTrue(report.problems().get(0).contains("expected 25 cases, got 24"));
    }
    assertFalse(Corpus.evaluate(new ArrayList<>()).problems().isEmpty());
    assertFalse(Corpus.evaluate(pristine.subList(0, 8)).problems().isEmpty());
  }

  @Test
  void aTruncatedDocumentFailsToParse() throws IOException {
    byte[] data = Files.readAllBytes(repoFixturePath());
    byte[] half = java.util.Arrays.copyOf(data, data.length / 2);
    assertThrows(StrictJson.JsonException.class, () -> Corpus.parse(half));
  }

  @Test
  void duplicatedCaseIdsAreAProblem() {
    List<Object> cases = new ArrayList<>(loadCases());
    cases.set(1, deepCopy(cases.get(0)));
    assertTrue(Corpus.evaluate(cases).problems().contains("duplicate or missing case id"));
  }

  @Test
  void anUnknownOpAndABadFieldSetAreProblems() {
    List<Object> cases = new ArrayList<>();
    for (Object c : loadCases()) {
      cases.add(deepCopy(c));
    }
    @SuppressWarnings("unchecked")
    Map<String, Object> first = (Map<String, Object>) cases.get(0);
    String id = (String) first.get("id");
    first.put("op", "gen_everything");
    assertTrue(problemFor(cases, id, "unknown op"));
    first.put("op", "gen_primitive");
    first.put("surprise", "field");
    assertTrue(problemFor(cases, id, "fields"));
    first.remove("surprise");
    first.remove("count");
    assertTrue(problemFor(cases, id, "fields"));
  }

  private static boolean problemFor(List<Object> cases, String id, String fragment) {
    return Corpus.evaluate(cases).problems().stream().anyMatch(p -> p.startsWith(id + ":") && p.contains(fragment));
  }

  @Test
  void aNonObjectCaseAndANonStringIdAreProblemsNotCrashes() {
    List<Object> cases = new ArrayList<>(loadCases());
    cases.set(0, "not an object");
    assertTrue(Corpus.evaluate(cases).problems().stream().anyMatch(p -> p.contains("case is not an object")));
    cases.set(0, new LinkedHashMap<String, Object>(Map.of("id", 7, "op", "x")));
    assertTrue(Corpus.evaluate(cases).problems().stream().anyMatch(p -> p.contains("missing a string id")));
  }

  @Test
  void coverageGatesNeedEveryOpAndEveryFailureId() {
    // A corpus with only failure_id cases misses ops and ids.
    List<Object> only = new ArrayList<>();
    for (Object c : loadCases()) {
      if ("failure_id_valid".equals(((Map<?, ?>) c).get("op"))
          && !"unregistered-id".equals(((Map<?, ?>) c).get("input"))
          && !"property-panic".equals(((Map<?, ?>) c).get("input"))) {
        only.add(c);
      }
    }
    List<String> problems = Corpus.evaluate(only, -1).problems();
    assertTrue(problems.stream().anyMatch(p -> p.contains("operations never exercised")));
    assertTrue(problems.stream().anyMatch(p -> p.contains("registered failure ids missing")));
    assertTrue(problems.stream().anyMatch(p -> p.contains("unregistered-id rejection")));
  }

  // ---- engine sabotage ---------------------------------------------------------

  @Test
  void sabotagedBisectionIsCaught() {
    Hooks.bisectCandidate = (current, diff) -> current.subtract(diff.shiftRight(2).max(BigInteger.ONE));
    assertFalse(Corpus.evaluate(loadCases()).problems().isEmpty());
  }

  @Test
  void sabotagedUnitStepIsCaught() {
    Hooks.unitCandidate = (current, diff) -> current.subtract(BigInteger.valueOf(2L * diff.signum()));
    assertFalse(Corpus.evaluate(loadCases()).problems().isEmpty());
  }

  @Test
  void skippingTheDeletionPhaseIsCaught() {
    Hooks.skipDeletionPhase = true;
    List<String> problems = Corpus.evaluate(loadCases()).problems();
    assertFalse(problems.isEmpty());
    assertTrue(problems.stream().anyMatch(p -> p.startsWith("pbt/shrink_")), problems.toString());
  }

  @Test
  void aShiftedRngIsCaught() {
    Hooks.newRng = seed -> new SplitMix64(seed + 1);
    List<String> problems = Corpus.evaluate(loadCases()).problems();
    assertTrue(problems.stream().anyMatch(p -> p.startsWith("pbt/gen_")), problems.toString());
    assertTrue(problems.stream().anyMatch(p -> p.startsWith("pbt/replay/")), problems.toString());
  }

  @Test
  void hooksResetToProduction() {
    Hooks.skipDeletionPhase = true;
    Hooks.reset();
    assertEquals(List.of(), Corpus.evaluate(loadCases()).problems());
  }

  // ---- strict reader -----------------------------------------------------------

  @Test
  void parseCorpusIsStrict() {
    String deep = "[".repeat(100) + "]".repeat(100);
    Map<String, String> bad = new LinkedHashMap<>();
    bad.put("not json", "nope");
    bad.put("empty", "");
    bad.put("trailing data", "{\"schema_version\":1,\"cases\":[]} {}");
    bad.put("wrong version", "{\"schema_version\":2,\"cases\":[]}");
    bad.put("string version", "{\"schema_version\":\"1\",\"cases\":[]}");
    bad.put("missing cases", "{\"schema_version\":1}");
    bad.put("extra key", "{\"schema_version\":1,\"cases\":[],\"x\":1}");
    bad.put("cases not array", "{\"schema_version\":1,\"cases\":{}}");
    bad.put("not an object", "[1]");
    bad.put("duplicate key", "{\"schema_version\":1,\"cases\":[],\"cases\":[]}");
    bad.put("nested duplicate", "{\"schema_version\":1,\"cases\":[{\"id\":\"a\",\"id\":\"b\"}]}");
    bad.put("truncated", "{\"schema_version\":1,\"cases\":[");
    bad.put("deep nesting", "{\"schema_version\":1,\"cases\":" + deep + "}");
    bad.put("unterminated obj", "{\"schema_version\":1,\"cases\":[{\"id\":\"a\"");
    bad.put("mismatched close", "{\"schema_version\":1,\"cases\":[}");
    bad.put("oversized", "{\"schema_version\":1,\"cases\":[],\"pad\":\"" + "a".repeat(StrictJson.MAX_BYTES) + "\"}");
    for (Map.Entry<String, String> e : bad.entrySet()) {
      assertThrows(
          StrictJson.JsonException.class,
          () -> Corpus.parse(e.getValue().getBytes(StandardCharsets.UTF_8)),
          e.getKey() + ": accepted");
    }
    List<Object> cases =
        Corpus.parse("{\"schema_version\":1,\"cases\":[1,\"a\",null,true,{\"x\":[]}]}".getBytes(StandardCharsets.UTF_8));
    assertEquals(5, cases.size(), "the reader leaves case-shape judgement to the evaluator");
    assertNotNull(cases.get(0));
  }
}
