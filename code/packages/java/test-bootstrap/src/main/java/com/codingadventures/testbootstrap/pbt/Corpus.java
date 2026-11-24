package com.codingadventures.testbootstrap.pbt;

import static com.codingadventures.testbootstrap.corpusjson.Values.CaseException;
import static com.codingadventures.testbootstrap.corpusjson.Values.MAX_FRAGMENT_LEN;
import static com.codingadventures.testbootstrap.corpusjson.Values.MAX_PROBLEM_LEN;
import static com.codingadventures.testbootstrap.corpusjson.Values.MAX_SIZE;
import static com.codingadventures.testbootstrap.corpusjson.Values.same;
import static com.codingadventures.testbootstrap.corpusjson.Values.show;

import com.codingadventures.testbootstrap.corpusjson.Canon;
import com.codingadventures.testbootstrap.corpusjson.Clip;
import com.codingadventures.testbootstrap.corpusjson.StrictJson;
import com.codingadventures.testbootstrap.corpusjson.Values;
import com.codingadventures.testbootstrap.tfv.Tfv;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;
import java.util.function.Predicate;

/**
 * The production runner for the shared pbt-v1 corpus
 * ({@code code/specs/fixtures/pbt-v1/cases.json}, VER05 section 6).
 *
 * <p>It takes the raw JSON, strictly decodes it, evaluates every case with this
 * package's own public engine functions and returns human-readable problems. An
 * empty list means "conforms". It does no file I/O (the package is
 * capability-free); the test that reads the committed fixture lives in the test
 * tree.
 *
 * <p>Why a runner that can fail matters: a conformance check that cannot report
 * a problem is worse than none. So the runner is strict:
 *
 * <ul>
 *   <li>a case must have exactly the fields of its op, with canonical encodings
 *       (ints are decimal strings, bytes are even-length lowercase hex);
 *   <li>an unknown op, unknown property name or malformed value is a problem,
 *       never a silent skip;
 *   <li>every op and all four failure ids must appear, at least one unregistered
 *       id must be rejected, and the number of cases must equal
 *       {@link #PBTV1_CASE_COUNT}, so deleting cases from the list is itself a
 *       problem;
 *   <li>every problem about a case starts with that case's id.
 * </ul>
 */
public final class Corpus {
  private Corpus() {}

  /** How many cases pbt-v1 holds; a different count is a problem. */
  public static final int PBTV1_CASE_COUNT = 25;

  private static final int MAX_CASES = 10_000;

  /**
   * Bounds the total bytes one gen case may generate and hold: count times the
   * per-item width. The separate caps on count and width alone would still allow
   * 10,000 x 10,000 bytes (CWE-400).
   */
  private static final int MAX_GEN_BYTES = 1 << 20;

  /** How many cases were fully evaluated (match or not) and every problem found. */
  public record Report(int evaluated, List<String> problems) {}

  /**
   * Strictly decodes a pbt-v1 document into its case list. The document must be
   * exactly {@code {"schema_version": 1, "cases": [...]}}. Duplicate object keys,
   * trailing data, nesting deeper than 64 and documents over 8 MiB are refused
   * with {@link StrictJson.JsonException}. Cases are returned unvalidated.
   */
  public static List<Object> parse(byte[] data) {
    Object root = StrictJson.parse(data);
    if (!(root instanceof Map<?, ?> doc) || doc.size() != 2) {
      throw new StrictJson.JsonException("pbt corpus: want exactly {\"schema_version\": 1, \"cases\": [...]}");
    }
    if (!(doc.get("schema_version") instanceof StrictJson.JsonNumber v) || !v.text().equals("1")) {
      throw new StrictJson.JsonException("pbt corpus: schema_version must be 1");
    }
    if (!(doc.get("cases") instanceof List<?> cases)) {
      throw new StrictJson.JsonException("pbt corpus: \"cases\" must be an array");
    }
    return new ArrayList<>(cases);
  }

  // ---------------------------------------------------------------------------
  // Properties named by the corpus. true means "holds".
  // ---------------------------------------------------------------------------

  private static final BigInteger B100 = BigInteger.valueOf(100);
  private static final BigInteger B50 = BigInteger.valueOf(50);
  private static final BigInteger BM10 = BigInteger.valueOf(-10);
  private static final BigInteger B10 = BigInteger.TEN;
  private static final BigInteger B20 = BigInteger.valueOf(20);

  private static final Map<String, Predicate<BigInteger>> INT_PROPS =
      Map.of(
          "int_le_100", x -> x.compareTo(B100) <= 0,
          "int_in_range_neg10_50", x -> x.compareTo(BM10) >= 0 && x.compareTo(B50) <= 0,
          "int_is_even", x -> !x.testBit(0), // two's complement: correct for negatives
          "int_abs_le_10", x -> x.abs().compareTo(B10) <= 0,
          "refinement_x_le_50", x -> x.compareTo(B50) <= 0);

  private static final Map<String, Predicate<List<BigInteger>>> LIST_PROPS =
      Map.of(
          "list_no_consecutive_duplicates",
          l -> {
            for (int i = 0; i + 1 < l.size(); i++) {
              if (l.get(i).compareTo(l.get(i + 1)) == 0) {
                return false;
              }
            }
            return true;
          },
          "list_sum_le_20",
          l -> {
            BigInteger sum = BigInteger.ZERO;
            for (BigInteger v : l) {
              sum = sum.add(v);
            }
            return sum.compareTo(B20) <= 0;
          });

  private static final Map<String, Predicate<byte[]>> BYTES_PROPS =
      Map.of(
          "bytes_len_lt_5", b -> b.length < 5,
          "bytes_no_zero_byte",
          b -> {
            for (byte x : b) {
              if (x == 0) {
                return false;
              }
            }
            return true;
          });

  /** The closed field set of each op (besides id and op). */
  private static final Map<String, List<String>> SHAPES =
      Map.ofEntries(
          Map.entry("gen_primitive", List.of("type", "seed", "count", "outputs")),
          Map.entry("gen_bounded", List.of("min", "max", "seed", "count", "outputs")),
          Map.entry("gen_bytes", List.of("min_len", "max_len", "seed", "count", "outputs")),
          Map.entry("shrink_int", List.of("initial", "target", "min", "max", "property", "expected_minimal", "expected_steps")),
          Map.entry("shrink_refinement", List.of("initial", "target", "min", "max", "property", "expected_minimal", "expected_steps")),
          Map.entry("shrink_list", List.of("initial", "elem_target", "property", "expected_minimal", "expected_steps")),
          Map.entry("shrink_bytes", List.of("initial", "property", "expected_minimal", "expected_steps")),
          Map.entry("replay_consistency", List.of("seed", "property", "initial_fail", "expected_minimal")),
          Map.entry("filter_exhausted", List.of("max_retries", "verdict", "failure_id")),
          Map.entry("failure_id_valid", List.of("input", "valid")),
          Map.entry("corpus_reject", List.of("text")));

  private static boolean fieldSetMatches(Map<?, ?> c, String op) {
    List<String> fields = SHAPES.get(op);
    if (c.size() != fields.size() + 2 || !c.containsKey("id") || !c.containsKey("op")) {
      return false;
    }
    for (String f : fields) {
      if (!c.containsKey(f)) {
        return false;
      }
    }
    return true;
  }

  /** {@link #evaluate(List, int)} with the pbt-v1 case count. */
  public static Report evaluate(List<Object> cases) {
    return evaluate(cases, PBTV1_CASE_COUNT);
  }

  /**
   * Evaluates every case and returns the problems (none means the engine
   * conforms). A negative {@code expected} skips the case-count check (for
   * ad-hoc case lists in tests).
   */
  public static Report evaluate(List<Object> casesIn, int expected) {
    List<String> problems = new ArrayList<>();
    int evaluated = 0;
    List<Object> cases = casesIn;
    if (expected >= 0 && cases.size() != expected) {
      problems.add("pbt-v1: expected " + expected + " cases, got " + cases.size());
    }
    if (cases.size() > MAX_CASES) {
      problems.add("pbt-v1: " + cases.size() + " cases exceeds the cap of " + MAX_CASES);
      cases = cases.subList(0, MAX_CASES);
    }

    // Ids are compared as strings only: a non-string id is a reported problem.
    Set<String> seenIds = new HashSet<>();
    for (Object raw : cases) {
      String id = raw instanceof Map<?, ?> m && m.get("id") instanceof String s ? s : null;
      if (id == null || id.isEmpty() || !seenIds.add(id)) {
        problems.add("duplicate or missing case id");
        break;
      }
    }

    Set<String> opsSeen = new HashSet<>();
    Set<String> registeredSeen = new HashSet<>();
    boolean rejectedIdSeen = false;

    for (Object raw : cases) {
      if (!(raw instanceof Map<?, ?> c)) {
        problems.add("case is not an object: " + show(raw));
        continue;
      }
      if (!(c.get("id") instanceof String rawId) || rawId.isEmpty()) {
        problems.add("case missing a string id: " + show(c));
        continue;
      }
      String cid = Clip.clip(rawId, 100);
      String op = c.get("op") instanceof String s ? s : "";
      if (!SHAPES.containsKey(op)) {
        problems.add(cid + ": unknown op " + show(c.get("op")));
        continue;
      }
      if (!fieldSetMatches(c, op)) {
        List<String> keys = new ArrayList<>();
        for (Object k : c.keySet()) {
          keys.add(String.valueOf(k));
        }
        keys.sort(null);
        List<String> want = new ArrayList<>(List.of("id", "op"));
        want.addAll(SHAPES.get(op));
        problems.add(cid + ": fields " + show(keys) + " != " + want);
        continue;
      }
      @SuppressWarnings("unchecked")
      Eval ev = evaluateCase(cid, op, (Map<String, Object>) c);
      problems.addAll(ev.problems);
      if (!ev.ranOk) {
        continue;
      }
      if (op.equals("failure_id_valid")) {
        if (ev.registered) {
          registeredSeen.add(ev.candidate);
        } else {
          rejectedIdSeen = true;
        }
      }
      opsSeen.add(op);
      evaluated++;
    }

    TreeSet<String> missingOps = new TreeSet<>(SHAPES.keySet());
    missingOps.removeAll(opsSeen);
    if (!missingOps.isEmpty()) {
      problems.add("pbt-v1: operations never exercised: " + missingOps);
    }
    List<String> missingIds = new ArrayList<>();
    for (String id : FailureIds.ALL) {
      if (!registeredSeen.contains(id)) {
        missingIds.add(id);
      }
    }
    if (!missingIds.isEmpty()) {
      problems.add("pbt-v1: registered failure ids missing: " + missingIds);
    }
    if (!rejectedIdSeen) {
      problems.add("pbt-v1: failure_id_valid needs an unregistered-id rejection");
    }
    problems.replaceAll(p -> Clip.clip(p, MAX_PROBLEM_LEN));
    return new Report(evaluated, problems);
  }

  private record Eval(List<String> problems, boolean ranOk, String candidate, boolean registered) {}

  // Decodes and checks one case. Any exception (a malformed case or an engine
  // that threw) is reported as a problem.
  private static Eval evaluateCase(String cid, String op, Map<String, Object> c) {
    try {
      Map<String, Object> v = new java.util.HashMap<>();
      for (Map.Entry<String, Object> e : c.entrySet()) {
        if (!e.getKey().equals("id") && !e.getKey().equals("op")) {
          v.put(e.getKey(), Values.decode(e.getValue()));
        }
      }
      List<String> found = new ArrayList<>();
      String candidate = "";
      boolean registered = false;
      switch (op) {
        case "gen_primitive", "gen_bounded", "gen_bytes" -> found = checkGen(cid, op, v);
        case "shrink_int", "shrink_refinement" -> found = checkShrinkInt(cid, op, v);
        case "shrink_list" -> found = checkShrinkList(cid, v);
        case "shrink_bytes" -> found = checkShrinkBytes(cid, v);
        case "replay_consistency" -> found = checkReplay(cid, v);
        case "filter_exhausted" -> found = checkFilterExhausted(cid, v);
        case "failure_id_valid" -> {
          String in = v.get("input") instanceof String s ? s : null;
          boolean got = in != null && FailureIds.isPbtFailureId(in);
          if (!same(got, v.get("valid"))) {
            found.add(cid + ": failure id membership disagrees for " + show(v.get("input")));
          }
          candidate = in == null ? "" : in;
          registered = got;
        }
        default -> { // corpus_reject
          if (!(v.get("text") instanceof String text)) {
            throw new CaseException("text must be a string");
          }
          if (Tfv.isValid(text)) {
            found.add(cid + ": corpus_reject text was accepted by the strict parser");
          }
        }
      }
      return new Eval(found, true, candidate, registered);
    } catch (RuntimeException | StackOverflowError e) {
      return new Eval(
          List.of(cid + ": " + op + " could not be evaluated: " + Clip.clip(Runner.safeString(e), MAX_FRAGMENT_LEN)),
          false,
          "",
          false);
    }
  }

  private static CaseException caseErr(String message) {
    return new CaseException(message);
  }

  /** A corpus seed reduced modulo 2^64, as the reference masks it (negative seeds wrap). */
  private static long seedOf(Object v) {
    if (!(v instanceof BigInteger b)) {
      throw caseErr("seed must be an int");
    }
    return b.longValue(); // the low 64 bits of the two's-complement form
  }

  private static BigInteger intOf(Object v, String what) {
    if (!(v instanceof BigInteger b)) {
      throw caseErr(what + " must be an int");
    }
    return b;
  }

  /** An int that drives a loop or an allocation: 0 <= n <= MAX_SIZE. */
  private static int sizeOf(Object v, String what) {
    BigInteger b = intOf(v, what);
    if (b.signum() < 0 || b.compareTo(BigInteger.valueOf(MAX_SIZE)) > 0) {
      throw caseErr(what + " " + show(b) + " outside [0, " + MAX_SIZE + "]");
    }
    return b.intValue();
  }

  private static String propName(Object v) {
    if (!(v instanceof String s)) {
      throw caseErr("unknown property " + show(v));
    }
    return s;
  }

  private static List<String> checkGen(String cid, String op, Map<String, Object> v) {
    long seed = seedOf(v.get("seed"));
    int count = sizeOf(v.get("count"), "count");
    // The expected outputs are in hand already: a case whose list length is not
    // count cannot conform, so refuse it before generating anything.
    if (!(v.get("outputs") instanceof List<?> expected) || expected.size() != count) {
      throw caseErr("outputs must be a list of exactly count (" + count + ") values");
    }
    List<Object> got;
    switch (op) {
      case "gen_primitive" -> {
        if (!(v.get("type") instanceof String name)) {
          throw caseErr("type must be a string");
        }
        got = Gens.genPrimitive(seed, name, count);
      }
      case "gen_bounded" -> {
        BigInteger lo = intOf(v.get("min"), "min");
        BigInteger hi = intOf(v.get("max"), "max");
        // Gens.genBounded enforces the count x width generation budget itself.
        got = new ArrayList<>(Gens.genBounded(seed, lo, hi, count));
      }
      default -> {
        int minLen = sizeOf(v.get("min_len"), "min_len");
        int maxLen = sizeOf(v.get("max_len"), "max_len");
        if ((long) count * maxLen > MAX_GEN_BYTES) {
          throw caseErr("count " + count + " x max_len " + maxLen + " exceeds the generation budget of " + MAX_GEN_BYTES + " bytes");
        }
        got = new ArrayList<>(Gens.genBytes(seed, minLen, maxLen, count));
      }
    }
    if (!same(got, v.get("outputs"))) {
      return List.of(cid + ": " + op + " outputs mismatch");
    }
    return List.of();
  }

  private static List<String> checkShrinkInt(String cid, String op, Map<String, Object> v) {
    String name = propName(v.get("property"));
    Predicate<BigInteger> prop = INT_PROPS.get(name);
    if (prop == null) {
      throw caseErr("unknown property " + show(name) + " for " + op);
    }
    BigInteger initial = intOf(v.get("initial"), "initial");
    BigInteger target = intOf(v.get("target"), "target");
    BigInteger lo = intOf(v.get("min"), "min");
    BigInteger hi = intOf(v.get("max"), "max");
    Shrunk<BigInteger> r = Shrink.shrinkInt(initial, target, lo, hi, prop, 1000);
    List<String> problems = new ArrayList<>();
    if (!same(r.value(), v.get("expected_minimal"))) {
      problems.add(cid + ": " + op + " minimal mismatch: got " + show(r.value()) + ", corpus " + show(v.get("expected_minimal")));
    }
    if (!same(BigInteger.valueOf(r.steps()), v.get("expected_steps"))) {
      problems.add(cid + ": " + op + " step count mismatch: got " + r.steps() + ", corpus " + show(v.get("expected_steps")));
    }
    // Engine invariants, independent of the corpus's expected values.
    if (prop.test(r.value())) {
      problems.add(cid + ": " + op + " minimal " + show(r.value()) + " does not falsify the property");
    }
    if (r.value().compareTo(lo) < 0 || r.value().compareTo(hi) > 0) {
      problems.add(cid + ": " + op + " minimal " + show(r.value()) + " left [" + show(lo) + ", " + show(hi) + "]");
    }
    return problems;
  }

  private static List<String> checkShrinkList(String cid, Map<String, Object> v) {
    String name = propName(v.get("property"));
    Predicate<List<BigInteger>> prop = LIST_PROPS.get(name);
    if (prop == null) {
      throw caseErr("unknown property " + show(name) + " for shrink_list");
    }
    if (!(v.get("initial") instanceof List<?> items)) {
      throw caseErr("initial must be a list of ints");
    }
    List<BigInteger> initial = new ArrayList<>();
    for (Object item : items) {
      initial.add(intOf(item, "element"));
    }
    BigInteger elemTarget = intOf(v.get("elem_target"), "elem_target");
    Shrunk<List<BigInteger>> r = Shrink.shrinkList(initial, elemTarget, prop, 1000);
    List<String> problems = new ArrayList<>();
    if (!same(new ArrayList<Object>(r.value()), v.get("expected_minimal"))) {
      problems.add(cid + ": shrink_list minimal mismatch: got " + Canon.render(r.value()) + ", corpus " + Clip.clip(Canon.render(v.get("expected_minimal")), MAX_FRAGMENT_LEN));
    }
    if (!same(BigInteger.valueOf(r.steps()), v.get("expected_steps"))) {
      problems.add(cid + ": shrink_list steps mismatch: got " + r.steps() + ", corpus " + show(v.get("expected_steps")));
    }
    if (prop.test(r.value())) {
      problems.add(cid + ": shrink_list minimal " + show(r.value()) + " does not falsify the property");
    }
    return problems;
  }

  private static List<String> checkShrinkBytes(String cid, Map<String, Object> v) {
    String name = propName(v.get("property"));
    Predicate<byte[]> prop = BYTES_PROPS.get(name);
    if (prop == null) {
      throw caseErr("unknown property " + show(name) + " for shrink_bytes");
    }
    if (!(v.get("initial") instanceof byte[] initial)) {
      throw caseErr("initial must be bytes");
    }
    Shrunk<byte[]> r = Shrink.shrinkBytes(initial, prop, 1000);
    List<String> problems = new ArrayList<>();
    if (!same(r.value(), v.get("expected_minimal"))) {
      problems.add(cid + ": shrink_bytes minimal mismatch: got " + show(r.value()) + ", corpus " + show(v.get("expected_minimal")));
    }
    if (!same(BigInteger.valueOf(r.steps()), v.get("expected_steps"))) {
      problems.add(cid + ": shrink_bytes steps mismatch: got " + r.steps() + ", corpus " + show(v.get("expected_steps")));
    }
    if (prop.test(r.value())) {
      problems.add(cid + ": shrink_bytes minimal " + show(r.value()) + " does not falsify the property");
    }
    return problems;
  }

  private static List<String> checkReplay(String cid, Map<String, Object> v) {
    long seed = seedOf(v.get("seed"));
    String name = propName(v.get("property"));
    Predicate<BigInteger> prop = INT_PROPS.get(name);
    if (prop == null) {
      throw caseErr("unknown property " + show(name) + " for replay_consistency");
    }
    Shrink.Replay r = Shrink.simulateReplay(seed, prop);
    List<String> problems = new ArrayList<>();
    if (!same(r.initialFail(), v.get("initial_fail"))) {
      problems.add(cid + ": replay initial_fail mismatch: got " + show(r.initialFail()) + ", corpus " + show(v.get("initial_fail")));
    }
    if (!same(r.minimal(), v.get("expected_minimal"))) {
      problems.add(cid + ": replay minimal mismatch: got " + show(r.minimal()) + ", corpus " + show(v.get("expected_minimal")));
    }
    return problems;
  }

  private static List<String> checkFilterExhausted(String cid, Map<String, Object> v) {
    int maxRetries = sizeOf(v.get("max_retries"), "max_retries");
    if (maxRetries < 1) {
      throw caseErr("max_retries must be at least 1");
    }
    int[] calls = {0};
    Gen<BigInteger> gen =
        Gens.filter(
            Gens.integer(BigInteger.ZERO, BigInteger.TEN),
            x -> {
              calls[0]++;
              return false;
            },
            maxRetries);
    Verdict verdict = Runner.check(0, gen, x -> true, new Options(1, 1000, CiMode.ON));
    List<String> problems = new ArrayList<>();
    if (!same("fail", v.get("verdict")) || verdict.passed()) {
      problems.add(cid + ": an unsatisfiable filter must be a fail verdict");
    }
    if (!same(verdict.failureId(), v.get("failure_id"))) {
      problems.add(cid + ": failure id \"" + verdict.failureId() + "\", corpus " + show(v.get("failure_id")));
    }
    if (calls[0] != maxRetries) {
      problems.add(cid + ": filter sampled " + calls[0] + " times, expected " + maxRetries);
    }
    return problems;
  }
}
