package com.codingadventures.testbootstrap.fuzz;

import static com.codingadventures.testbootstrap.fuzz.Fx.anyContains;
import static com.codingadventures.testbootstrap.fuzz.Fx.caseAt;
import static com.codingadventures.testbootstrap.fuzz.Fx.deepCopy;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.codingadventures.testbootstrap.corpusjson.StrictJson;
import com.codingadventures.testbootstrap.fuzz.Mutate.Endian;
import com.codingadventures.testbootstrap.prng.SplitMix64;
import java.io.IOException;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Consumer;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

/**
 * Replays the whole committed fuzz-v1 corpus and carries its negative controls
 * (VER06f section 10). The fixture is read from its committed path, never a copy.
 */
class FuzzCorpusConformanceTest {
  @AfterEach
  void resetHooks() {
    Hooks.reset();
  }

  // ---- corruption ------------------------------------------------------------

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

  /** Each variant is one corruption of the case; every one must be noticed. */
  static List<Consumer<Map<String, Object>>> corruptions(String op) {
    return switch (op) {
      case "mutate_bit_flip",
          "mutate_byte_flip",
          "mutate_arithmetic",
          "mutate_boundary",
          "mutate_block_delete",
          "mutate_block_insert",
          "mutate_block_duplicate",
          "mutate_splice",
          "mutate_sequence" -> List.of(c -> c.put("output", corruptValue(c.get("output"))));
      case "corpus_admit" ->
          List.of(c -> c.put("admitted", corruptValue(c.get("admitted"))), c -> c.put("new_edges", corruptValue(c.get("new_edges"))));
      case "crash_dedup" -> List.of(c -> c.put("is_duplicate", corruptValue(c.get("is_duplicate"))));
      case "failure_id_valid" -> List.of(c -> c.put("valid", corruptValue(c.get("valid"))));
      case "private_reporting" ->
          List.of(
              c -> c.put("tap_line", "not ok 1 - ct_compare [fuzz-crash:3a8f19b3]"),
              c -> c.put("contains_raw_payload", corruptValue(c.get("contains_raw_payload"))));
      case "corpus_reject" -> List.of(c -> c.put("text", "tfv 1\n"));
      default -> throw new AssertionError("no corruption defined for op " + op);
    };
  }

  // ---- the positive run ------------------------------------------------------

  @Test
  void corpusConformsToAllTwentyFiveCases() {
    Corpus.Report report = Corpus.evaluate(Fx.loadCases());
    assertEquals(List.of(), report.problems());
    assertEquals(25, report.evaluated(), "exactly 25 cases must be fully evaluated");
    assertEquals(25, Corpus.FUZZV1_CASE_COUNT);
  }

  @Test
  void everyOpIsPresentInTheFixtureAndHasACorruption() {
    java.util.Set<String> ops = new java.util.TreeSet<>();
    for (Object c : Fx.loadCases()) {
      ops.add((String) ((Map<?, ?>) c).get("op"));
    }
    assertEquals(14, ops.size(), "ops: " + ops);
    assertEquals(Corpus.SHAPES.keySet(), new java.util.TreeSet<>(ops));
    for (String op : ops) {
      assertFalse(corruptions(op).isEmpty());
    }
  }

  // ---- negative controls -----------------------------------------------------

  /**
   * Per-op, per-case: corrupt the expected value of EVERY real case in every way
   * its op allows and require the run to fail and name exactly that case.
   */
  @Test
  void corruptingAnyCaseMakesTheRunFailAndNamesThatCase() {
    List<Object> pristine = Fx.loadCases();
    int corrupted = 0;
    for (int i = 0; i < pristine.size(); i++) {
      Map<String, Object> original = caseAt(pristine, i);
      String id = (String) original.get("id");
      String op = (String) original.get("op");
      for (Consumer<Map<String, Object>> corruption : corruptions(op)) {
        List<Object> cases = Fx.copyOfCases(pristine);
        Map<String, Object> victim = caseAt(cases, i);
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
    List<Object> pristine = Fx.loadCases();
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
  void droppingEveryCaseOfAnOpIsAProblemEvenWithTheCountDisabled() {
    List<Object> pristine = Fx.loadCases();
    for (String op : Corpus.SHAPES.keySet()) {
      List<Object> cases = new ArrayList<>();
      for (Object c : pristine) {
        if (!op.equals(((Map<?, ?>) c).get("op"))) {
          cases.add(c);
        }
      }
      assertTrue(anyContains(Corpus.evaluate(cases, -1).problems(), "operations never exercised"), op);
    }
  }

  @Test
  void aTruncatedDocumentFailsToParse() throws IOException {
    byte[] data = Files.readAllBytes(Fx.fixturePath());
    byte[] half = Arrays.copyOf(data, data.length / 2);
    assertThrows(StrictJson.JsonException.class, () -> Corpus.parse(half));
  }

  @Test
  void duplicatedCaseIdsAreAProblem() {
    List<Object> cases = new ArrayList<>(Fx.loadCases());
    cases.set(1, deepCopy(cases.get(0)));
    assertTrue(Corpus.evaluate(cases).problems().contains("duplicate or missing case id"));
  }

  @Test
  void anUnknownOpAMissingAndAnExtraFieldAreProblems() {
    List<Object> cases = Fx.copyOfCases(Fx.loadCases());
    Map<String, Object> first = caseAt(cases, 0);
    String id = (String) first.get("id");
    first.put("op", "gen_everything");
    assertTrue(problemFor(cases, id, "unknown op"));
    first.put("op", "mutate_bit_flip");
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
    List<Object> cases = new ArrayList<>(Fx.loadCases());
    cases.set(0, "not an object");
    assertTrue(anyContains(Corpus.evaluate(cases).problems(), "case is not an object"));
    cases.set(0, new LinkedHashMap<String, Object>(Map.of("id", 7, "op", "x")));
    assertTrue(anyContains(Corpus.evaluate(cases).problems(), "missing a string id"));
  }

  @Test
  void nonCanonicalAndWronglyTypedFieldsAreProblemsNamingTheCase() {
    List<Object> pristine = Fx.loadCases();
    String[][] bad = {
      // op, field, replacement kind
      {"mutate_bit_flip", "bit_offset", "int:007"},
      {"mutate_bit_flip", "bit_offset", "int:+1"},
      {"mutate_bit_flip", "input", "bytes:0G"},
      {"mutate_bit_flip", "input", "bytes:ABCD"},
      {"mutate_bit_flip", "input", "bytes:abc"},
      {"mutate_bit_flip", "count", "bytes:00"},
      {"mutate_bit_flip", "input", "int:1"},
      {"mutate_arithmetic", "endian", "str:middle"},
      {"mutate_arithmetic", "endian", "bool:true"},
      {"mutate_arithmetic", "width", "str:4"},
      {"mutate_sequence", "steps", "int:10001"},
      {"mutate_sequence", "steps", "int:-1"},
      {"mutate_sequence", "seed", "str:42"},
      {"mutate_splice", "donor", "int:1"},
      {"corpus_admit", "trace_edges", "int:1"},
      {"corpus_admit", "trace_edges", "list:str"},
      {"corpus_admit", "trace_edges", "list:neg"},
      {"corpus_admit", "trace_edges", "list:big"},
      {"corpus_admit", "admitted", "int:1"},
      {"crash_dedup", "crash_a_frames", "str:x"},
      {"crash_dedup", "crash_a_frames", "list:int"},
      {"crash_dedup", "crash_a_kind", "int:1"},
      {"crash_dedup", "is_duplicate", "str:true"},
      {"private_reporting", "raw_payload", "str:x"},
      {"private_reporting", "contains_raw_payload", "str:x"},
      {"private_reporting", "sig_hash", "str:NOTHEX00"},
      {"private_reporting", "failure_id", "str:fuzz-bogus"},
      {"private_reporting", "tap_line", "int:1"},
      {"corpus_reject", "text", "int:1"},
    };
    for (String[] b : bad) {
      List<Object> cases = Fx.copyOfCases(pristine);
      Map<String, Object> victim = Fx.firstOf(cases, b[0]);
      String id = (String) victim.get("id");
      Object replacement;
      String[] kv = b[2].split(":", 2);
      replacement =
          switch (kv[0]) {
            case "int" -> Fx.intNode(kv[1]);
            case "bytes" -> Fx.bytesNode(kv[1]);
            case "bool" -> Boolean.parseBoolean(kv[1]);
            case "str" -> kv[1];
            default ->
                switch (kv[1]) {
                  case "str" -> new ArrayList<Object>(List.of("x"));
                  case "int" -> new ArrayList<Object>(List.of(Fx.intNode("1")));
                  case "neg" -> new ArrayList<Object>(List.of(Fx.intNode("-1")));
                  default -> new ArrayList<Object>(List.of(Fx.intNode("18446744073709551616")));
                };
          };
      victim.put(b[1], replacement);
      List<String> problems = Corpus.evaluate(Fx.swapped(cases, victim)).problems();
      assertTrue(problems.stream().anyMatch(p -> p.startsWith(id + ":")), Arrays.toString(b) + ": " + problems);
    }
  }

  @Test
  void anUnsupportedWidthLeavesTheInputUnchangedNotAProblem() {
    List<Object> cases = Fx.copyOfCases(Fx.loadCases());
    Map<String, Object> c = Fx.firstOf(cases, "mutate_arithmetic");
    c.put("width", Fx.intNode("3"));
    c.put("output", c.get("input"));
    assertEquals(List.of(), Fx.problems(Fx.swapped(cases, c)));
    c.put("width", Fx.intNode("99999"));
    assertEquals(List.of(), Fx.problems(Fx.swapped(cases, c)));
  }

  @Test
  void coverageGatesNeedAnAcceptedAndARejectedFailureId() {
    List<Object> only = new ArrayList<>();
    for (Object c : Fx.loadCases()) {
      if ("fuzz-crash".equals(((Map<?, ?>) c).get("input"))) {
        only.add(c);
      }
    }
    assertTrue(anyContains(Corpus.evaluate(only, -1).problems(), "unregistered-id rejection"));
    only.clear();
    for (Object c : Fx.loadCases()) {
      if ("fuzz-arbitrary-error".equals(((Map<?, ?>) c).get("input"))) {
        only.add(c);
      }
    }
    assertTrue(anyContains(Corpus.evaluate(only, -1).problems(), "registered-id acceptance"));
  }

  // ---- engine sabotage ---------------------------------------------------------

  private static final BigInteger ONE = BigInteger.ONE;

  private static void assertCaught(String name) {
    List<String> problems = Corpus.evaluate(Fx.loadCases()).problems();
    assertFalse(problems.isEmpty(), "sabotage was not caught: " + name);
    Hooks.reset();
    assertEquals(List.of(), Corpus.evaluate(Fx.loadCases()).problems(), "hooks must reset after " + name);
  }

  @Test
  void everySabotagedEntryPointIsCaught() {
    Hooks.bitFlip = (d, o, c) -> Mutate.bitFlipBig(d, o.add(ONE), c);
    assertCaught("bit_flip offset+1");
    Hooks.byteFlip = (d, o, c) -> Mutate.byteFlipBig(d, o, c.add(ONE));
    assertCaught("byte_flip count+1");
    Hooks.byteArithmetic = (d, o, dl, w, e) -> Mutate.byteArithmeticBig(d, o, dl, w, e == Endian.BIG ? Endian.LITTLE : Endian.BIG);
    assertCaught("arithmetic endian swapped");
    Hooks.boundaryInsert = (d, o, w, v, e) -> Mutate.boundaryInsertBig(d, o, w, v.xor(ONE), e);
    assertCaught("boundary value xor 1");
    Hooks.blockDelete = (d, o, l) -> Mutate.blockDeleteBig(d, o, l.add(ONE));
    assertCaught("block_delete length+1");
    Hooks.blockInsert = (d, o, ins) -> Mutate.blockInsertBig(d, o.add(ONE), ins);
    assertCaught("block_insert offset+1");
    Hooks.blockDuplicate = (d, s, l, dst) -> Mutate.blockDuplicateBig(d, s, l, dst.add(ONE));
    assertCaught("block_duplicate dst+1");
    Hooks.splice = (a, b, ts, ds) -> Mutate.spliceBig(a, b, ts.add(ONE), ds);
    assertCaught("splice target split+1");
    Hooks.mutateSequence = (d, seed, n, m, cap) -> Mutate.mutateSequenceCounted(d, seed + 1, n, m, cap);
    assertCaught("mutate_sequence seed+1");
    Hooks.corpusAdmit = (e, t) -> new Bitmap.Novelty(true, 1);
    assertCaught("corpus_admit always admits");
    Hooks.isDuplicate = (a, b, c, d) -> true;
    assertCaught("dedup always duplicate");
    Hooks.isDuplicate = (a, b, c, d) -> false;
    assertCaught("dedup never duplicate");
    Hooks.isFailureId = s -> true;
    assertCaught("every failure id valid");
    Hooks.formatTap = (n, t, f, s) -> Signature.formatTapFailure(n, t, f, s) + " " + t;
    assertCaught("tap line leaks the target");
    Hooks.tfvValid = s -> true;
    assertCaught("tfv accepts everything");
  }

  @Test
  void aShiftedStreamAndASabotagedConstantAreCaughtOnTheSequenceCase() {
    Hooks.mutateSequence = (d, seed, n, m, cap) -> new Mutate.Counted(Mutate.mutateSequence(d, new SplitMix64(seed + 1), n, m), 0);
    assertTrue(anyContains(Corpus.evaluate(Fx.loadCases()).problems(), "fuzz/mutate_sequence/seed42_steps4"));
  }

  @Test
  void engineInvariantsAreCheckedIndependentlyOfExpectedValues() {
    List<Object> all = Fx.loadCases();
    // A flip that changes the length is reported even when the corpus agrees with it.
    Map<String, Object> c = Fx.firstOf(all, "mutate_bit_flip");
    Hooks.bitFlip = (d, o, n) -> Arrays.copyOf(d, d.length + 1);
    c.put("output", Fx.bytesNode(((Map<?, ?>) c.get("input")).get("hex") + "00"));
    assertTrue(anyContains(Fx.problems(Fx.swapped(all, c)), "changed the length"));
    Hooks.reset();

    c = Fx.firstOf(all, "mutate_splice");
    Hooks.splice = (a, b, ts, ds) -> new byte[] {1};
    c.put("output", Fx.bytesNode("01"));
    assertTrue(anyContains(Fx.problems(Fx.swapped(all, c)), "prefix + suffix"));
    Hooks.reset();

    c = Fx.firstOf(all, "corpus_admit"); // the first corpus_admit case is the novel-edge one
    assertEquals(Boolean.TRUE, c.get("admitted"));
    Hooks.corpusAdmit = (e, t) -> new Bitmap.Novelty(false, 0);
    c.put("admitted", false);
    c.put("new_edges", Fx.intNode("0"));
    assertTrue(anyContains(Fx.problems(Fx.swapped(all, c)), "set difference"));
    Hooks.reset();

    c = Fx.firstOf(Fx.loadCases(), "private_reporting");
    String want = (String) c.get("tap_line");
    Hooks.formatTap = (n, t, f, s) -> want + "\n";
    assertTrue(anyContains(Fx.problems(Fx.swapped(Fx.loadCases(), c)), "more than one line"));
    Hooks.reset();

    Hooks.isFailureId = s -> s.equals("fuzz-crash");
    assertTrue(anyContains(Corpus.evaluate(Fx.loadCases()).problems(), "rejects registered ids"));
  }

  @Test
  void aThrowingEngineIsAClippedProblemNotACrash() {
    Hooks.bitFlip = (d, o, n) -> {
      throw new IllegalStateException("engine bug " + "x".repeat(5000));
    };
    List<String> ps = Corpus.evaluate(Fx.loadCases()).problems();
    assertTrue(anyContains(ps, "engine bug"));
    for (String p : ps) {
      assertTrue(p.length() <= 403, "an unclipped problem of " + p.length());
    }
    Hooks.reset();
    Hooks.bitFlip = (d, o, n) -> {
      throw new IllegalStateException();
    };
    assertTrue(anyContains(Corpus.evaluate(Fx.loadCases()).problems(), "IllegalStateException"));
    Hooks.reset();
    Hooks.formatTap = (n, t, f, s) -> {
      throw new IllegalArgumentException("nope");
    };
    assertTrue(anyContains(Corpus.evaluate(Fx.loadCases()).problems(), "refused"));
  }

  @Test
  void aMutatorCaseWithANonBytesOutputIsAProblem() {
    List<Object> all = Fx.loadCases();
    Map<String, Object> c = Fx.firstOf(all, "mutate_bit_flip");
    c.put("output", Fx.intNode("1"));
    assertTrue(anyContains(Fx.problems(Fx.swapped(all, c)), "output must be bytes"));
  }

  @Test
  void hooksResetToProduction() {
    Hooks.bitFlip = (d, o, n) -> new byte[0];
    Hooks.reset();
    assertEquals(List.of(), Corpus.evaluate(Fx.loadCases()).problems());
  }

  // ---- caps ----------------------------------------------------------------------

  @Test
  void oversizedValuesAreProblemsNotHangs() {
    List<Object> all = Fx.loadCases();
    Map<String, Object> c = Fx.firstOf(all, "mutate_sequence");
    c.put("steps", Fx.intNode("1000000000"));
    assertTrue(anyContains(Fx.problems(Fx.swapped(all, c)), "outside [0, 10000]"));
    // 10,000 steps over a 10,000-byte input, repeated: the aggregate budget trips.
    List<Object> many = new ArrayList<>();
    for (int i = 0; i < 400; i++) {
      Map<String, Object> d = Fx.firstOf(all, "mutate_sequence");
      d.put("id", "seq/" + i);
      d.put("steps", Fx.intNode("10000"));
      d.put("input", Fx.bytesNode("00".repeat(10_000)));
      many.add(d);
    }
    assertTrue(anyContains(Corpus.evaluate(many, -1).problems(), "work budget"));
  }

  /**
   * The aggregate budget charges the work a sequence really did. Block insert and
   * duplicate grow the buffer, so steps x initial length undercounts: a corpus of tiny
   * inputs with many steps passes the old estimate while copying far more.
   */
  @Test
  void theAggregateSequenceBudgetChargesActualWorkNotStepsTimesInitialLength() {
    List<Object> all = Fx.loadCases();
    List<Object> many = new ArrayList<>();
    long real = 0;
    long estimate = 0;
    int steps = 10_000;
    for (int i = 0; real <= Corpus.MAX_SEQUENCE_WORK; i++) {
      long w = Mutate.mutateSequenceCounted(new byte[2], i, steps, Mutate.MAX_INPUT_BYTES, Long.MAX_VALUE).work();
      real += w;
      estimate += (long) steps * 2;
      Map<String, Object> d = Fx.firstOf(all, "mutate_sequence");
      d.put("id", "seq/" + i);
      d.put("steps", Fx.intNode(Integer.toString(steps)));
      d.put("seed", Fx.intNode(Integer.toString(i)));
      d.put("input", Fx.bytesNode("0000"));
      d.put("output", Fx.bytesNode("00"));
      many.add(d);
    }
    assertTrue(estimate < Corpus.MAX_SEQUENCE_WORK, "the old steps x length estimate would have allowed this corpus: " + estimate);
    String last = "seq/" + (many.size() - 1) + ":";
    assertTrue(Corpus.evaluate(many, -1).problems().stream().anyMatch(p -> p.startsWith(last) && p.contains("work budget")));
  }

  /**
   * A case that hits its own cap must still be charged: otherwise every failing case is
   * handed the full aggregate budget again (10,000 cases x 16 MiB). Total work really done
   * stays within the cap and the later cases fail fast without running.
   */
  @Test
  void casesThatHitTheCapAreChargedAndLaterOnesFailWithoutRunning() {
    List<Object> all = Fx.loadCases();
    List<Object> many = new ArrayList<>();
    for (int i = 0; i < 300; i++) {
      Map<String, Object> d = Fx.firstOf(all, "mutate_sequence");
      d.put("id", "seq/" + i);
      d.put("steps", Fx.intNode("10000"));
      d.put("seed", Fx.intNode(Integer.toString(i)));
      d.put("input", Fx.bytesNode("00".repeat(10_000)));
      many.add(d);
    }
    long[] total = {0};
    int[] calls = {0};
    Hooks.mutateSequence =
        (d, seed, n, m, cap) -> {
          calls[0]++;
          try {
            Mutate.Counted c = Mutate.mutateSequenceCounted(d, seed, n, m, cap);
            total[0] += c.work();
            return c;
          } catch (Mutate.WorkExhausted e) {
            total[0] += e.work;
            throw e;
          }
        };
    List<String> problems = Corpus.evaluate(many, -1).problems();
    assertTrue(total[0] <= Corpus.MAX_SEQUENCE_WORK, "actual work " + total[0] + " exceeded the aggregate cap");
    assertTrue(calls[0] < 50, "once the cap is breached no later case runs, but " + calls[0] + " of 300 did");
    long budgetProblems = problems.stream().filter(p -> p.contains("work budget")).count();
    assertEquals(300 - (calls[0] - 1), budgetProblems, "the breaching case and every later one fail on the budget");
  }

  @Test
  void tooManyCasesAreCappedAndReported() {
    List<Object> all = Fx.loadCases();
    List<Object> many = new ArrayList<>();
    for (int i = 0; i < 10_001; i++) {
      Map<String, Object> d = Fx.firstOf(all, "failure_id_valid");
      d.put("id", "f/" + i);
      many.add(d);
    }
    assertTrue(anyContains(Corpus.evaluate(many, -1).problems(), "exceeds the cap"));
  }

  @Test
  void problemTextIsClipped() {
    List<Object> all = Fx.loadCases();
    Map<String, Object> c = Fx.firstOf(all, "mutate_bit_flip");
    c.put("id", "i".repeat(5000));
    c.put("output", Fx.bytesNode("ab".repeat(5000)));
    for (String p : Fx.problems(Fx.swapped(all, c))) {
      assertTrue(p.length() <= 403, p.length() + " chars");
    }
    List<Object> bogus = new ArrayList<>(all);
    bogus.add(new ArrayList<>(List.of("z".repeat(5000))));
    for (String p : Fx.problems(bogus)) {
      assertTrue(p.length() <= 403, p.length() + " chars");
    }
  }

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
    bad.put("oversized", "{\"schema_version\":1,\"cases\":[],\"pad\":\"" + "a".repeat(StrictJson.MAX_BYTES) + "\"}");
    for (Map.Entry<String, String> e : bad.entrySet()) {
      assertThrows(
          StrictJson.JsonException.class,
          () -> Corpus.parse(e.getValue().getBytes(StandardCharsets.UTF_8)),
          e.getKey() + ": accepted");
    }
    assertEquals(
        5, Corpus.parse("{\"schema_version\":1,\"cases\":[1,\"a\",null,true,{\"x\":[]}]}".getBytes(StandardCharsets.UTF_8)).size());
  }

  @Test
  void aHandBuiltTooDeepValueIsAProblemNotAStackOverflow() {
    List<Object> all = Fx.loadCases();
    Map<String, Object> c = Fx.firstOf(all, "mutate_bit_flip");
    Object nested = new ArrayList<Object>();
    for (int i = 0; i < 100_000; i++) {
      List<Object> outer = new ArrayList<>();
      outer.add(nested);
      nested = outer;
    }
    c.put("count", nested);
    assertTrue(anyContains(Fx.problems(Fx.swapped(all, c)), "could not be evaluated"));
  }
}
