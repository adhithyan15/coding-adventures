package com.codingadventures.testbootstrap.fuzz;

import static com.codingadventures.testbootstrap.corpusjson.Values.CaseException;
import static com.codingadventures.testbootstrap.corpusjson.Values.MAX_FRAGMENT_LEN;
import static com.codingadventures.testbootstrap.corpusjson.Values.MAX_PROBLEM_LEN;
import static com.codingadventures.testbootstrap.corpusjson.Values.MAX_SIZE;
import static com.codingadventures.testbootstrap.corpusjson.Values.same;
import static com.codingadventures.testbootstrap.corpusjson.Values.show;

import com.codingadventures.testbootstrap.corpusjson.Clip;
import com.codingadventures.testbootstrap.corpusjson.StrictJson;
import com.codingadventures.testbootstrap.corpusjson.Values;
import com.codingadventures.testbootstrap.fuzz.Mutate.Endian;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;

/**
 * The production runner for the shared fuzz-v1 corpus
 * ({@code code/specs/fixtures/fuzz-v1/cases.json}, VER06 section 8, VER06f section 8).
 *
 * <p>It takes the raw JSON, strictly decodes it, evaluates every case with this
 * package's own public engine functions and returns human-readable problems. An
 * empty list means "conforms". It does no file I/O (the package is
 * capability-free); the tests read the committed fixture.
 *
 * <p>A runner that cannot report a problem is worse than none, so this one is
 * strict:
 *
 * <ul>
 *   <li>a case has exactly the fields of its op, in canonical encodings (ints are
 *       decimal strings, bytes are even-length lowercase hex);
 *   <li>an unknown op or a malformed value is a PROBLEM, never a silent skip;
 *   <li>every op appears, a failure id is accepted and one rejected, and the case
 *       count equals {@link #FUZZV1_CASE_COUNT}, so deleting cases is itself a
 *       problem;
 *   <li>fixture-controlled sizes are capped (CWE-400): a hostile {@code steps =
 *       10^9} is a reported problem, not a hang;
 *   <li>every problem about a case starts with that case's id.
 * </ul>
 */
public final class Corpus {
  private Corpus() {}

  /** How many cases fuzz-v1 holds; a different count is a problem. */
  public static final int FUZZV1_CASE_COUNT = 25;

  private static final int MAX_CASES = 10_000;

  /**
   * Bounds the aggregate mutate_sequence work of one corpus: the sum over cases of
   * {@code steps x max(len(input), 1)}. Each case is already capped (10,000 steps,
   * 10,000 bytes) but 10,000 such cases would still be 10^12 byte-steps (CWE-400).
   */
  static final long MAX_SEQUENCE_WORK = 1L << 24;

  /** How many cases were fully evaluated (match or not) and every problem found. */
  public record Report(int evaluated, List<String> problems) {}

  /**
   * Strictly decodes a fuzz-v1 document into its case list. The document must be
   * exactly {@code {"schema_version": 1, "cases": [...]}}. Duplicate object keys,
   * trailing data, nesting deeper than 64 and documents over 8 MiB are refused with
   * {@link StrictJson.JsonException}. Cases are returned unvalidated.
   */
  public static List<Object> parse(byte[] data) {
    Object root = StrictJson.parse(data);
    if (!(root instanceof Map<?, ?> doc) || doc.size() != 2) {
      throw new StrictJson.JsonException("fuzz corpus: want exactly {\"schema_version\": 1, \"cases\": [...]}");
    }
    if (!(doc.get("schema_version") instanceof StrictJson.JsonNumber v) || !v.text().equals("1")) {
      throw new StrictJson.JsonException("fuzz corpus: schema_version must be 1");
    }
    if (!(doc.get("cases") instanceof List<?> cases)) {
      throw new StrictJson.JsonException("fuzz corpus: \"cases\" must be an array");
    }
    return new ArrayList<>(cases);
  }

  /** The closed field set of each op (besides id and op). */
  static final Map<String, List<String>> SHAPES =
      Map.ofEntries(
          Map.entry("mutate_bit_flip", List.of("input", "bit_offset", "count", "output")),
          Map.entry("mutate_byte_flip", List.of("input", "byte_offset", "count", "output")),
          Map.entry("mutate_arithmetic", List.of("input", "byte_offset", "delta", "width", "endian", "output")),
          Map.entry("mutate_boundary", List.of("input", "byte_offset", "width", "val", "endian", "output")),
          Map.entry("mutate_block_delete", List.of("input", "offset", "length", "output")),
          Map.entry("mutate_block_insert", List.of("input", "offset", "insert_bytes", "output")),
          Map.entry("mutate_block_duplicate", List.of("input", "src_offset", "length", "dst_offset", "output")),
          Map.entry("mutate_splice", List.of("target", "donor", "target_split", "donor_split", "output")),
          Map.entry("mutate_sequence", List.of("input", "seed", "steps", "output")),
          Map.entry("corpus_admit", List.of("existing_edges", "trace_edges", "admitted", "new_edges")),
          Map.entry("crash_dedup", List.of("crash_a_kind", "crash_a_frames", "crash_b_kind", "crash_b_frames", "is_duplicate")),
          Map.entry("failure_id_valid", List.of("input", "valid")),
          Map.entry(
              "private_reporting",
              List.of("target", "failure_id", "raw_payload", "sig_hash", "tap_line", "contains_raw_payload")),
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

  /** {@link #evaluate(List, int)} with the fuzz-v1 case count. */
  public static Report evaluate(List<Object> cases) {
    return evaluate(cases, FUZZV1_CASE_COUNT);
  }

  /**
   * Evaluates every case and returns the problems (none means the engine conforms).
   * A negative {@code expected} skips the case-count check (for ad-hoc case lists
   * in tests).
   */
  public static Report evaluate(List<Object> casesIn, int expected) {
    List<String> problems = new ArrayList<>();
    int evaluated = 0;
    List<Object> cases = casesIn;
    if (expected >= 0 && cases.size() != expected) {
      problems.add("fuzz-v1: expected " + expected + " cases, got " + cases.size());
    }
    if (cases.size() > MAX_CASES) {
      problems.add("fuzz-v1: " + cases.size() + " cases exceeds the cap of " + MAX_CASES);
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

    long[] workLeft = {MAX_SEQUENCE_WORK};
    Set<String> opsSeen = new HashSet<>();
    boolean acceptedIdSeen = false;
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
      Eval ev = evaluateCase(cid, op, (Map<String, Object>) c, workLeft);
      problems.addAll(ev.problems);
      if (!ev.ranOk) {
        continue;
      }
      if (op.equals("failure_id_valid")) {
        acceptedIdSeen = acceptedIdSeen || ev.accepted;
        rejectedIdSeen = rejectedIdSeen || !ev.accepted;
      }
      opsSeen.add(op);
      evaluated++;
    }

    TreeSet<String> missingOps = new TreeSet<>(SHAPES.keySet());
    missingOps.removeAll(opsSeen);
    if (!missingOps.isEmpty()) {
      problems.add("fuzz-v1: operations never exercised: " + missingOps);
    }
    if (!acceptedIdSeen) {
      problems.add("fuzz-v1: failure_id_valid needs a registered-id acceptance");
    }
    if (!rejectedIdSeen) {
      problems.add("fuzz-v1: failure_id_valid needs an unregistered-id rejection");
    }
    List<String> unregistered = new ArrayList<>();
    for (String id : FailureIds.ALL) {
      if (!Hooks.isFailureId.test(id)) {
        unregistered.add(id);
      }
    }
    if (!unregistered.isEmpty()) {
      problems.add("fuzz-v1: engine rejects registered ids " + unregistered);
    }
    problems.replaceAll(p -> Clip.clip(p, MAX_PROBLEM_LEN));
    return new Report(evaluated, problems);
  }

  private record Eval(List<String> problems, boolean ranOk, boolean accepted) {}

  // Decodes and checks one case. Any exception (a malformed case or an engine that
  // threw) is reported as a problem.
  private static Eval evaluateCase(String cid, String op, Map<String, Object> c, long[] work) {
    try {
      Map<String, Object> v = new HashMap<>();
      for (Map.Entry<String, Object> e : c.entrySet()) {
        if (!e.getKey().equals("id") && !e.getKey().equals("op")) {
          v.put(e.getKey(), Values.decode(e.getValue()));
        }
      }
      List<String> found = new ArrayList<>();
      boolean accepted = false;
      switch (op) {
        case "corpus_admit" -> found = checkCorpusAdmit(cid, v);
        case "crash_dedup" -> found = checkCrashDedup(cid, v);
        case "failure_id_valid" -> {
          String in = v.get("input") instanceof String s ? s : null;
          accepted = in != null && Hooks.isFailureId.test(in);
          if (!same(accepted, v.get("valid"))) {
            found.add(cid + ": failure id membership disagrees for " + show(v.get("input")));
          }
        }
        case "private_reporting" -> found = checkPrivateReporting(cid, v);
        case "corpus_reject" -> {
          if (!(v.get("text") instanceof String text)) {
            throw new CaseException("text must be a string");
          }
          if (Hooks.tfvValid.test(text)) {
            found.add(cid + ": corpus_reject text was accepted by the strict parser");
          }
        }
        default -> found = checkMutator(cid, op, v, work); // the nine byte-in, byte-out mutators
      }
      return new Eval(found, true, accepted);
    } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
      String detail = e.getMessage() == null ? e.getClass().getSimpleName() : e.getMessage();
      return new Eval(
          List.of(cid + ": " + op + " could not be evaluated: " + Clip.clip(detail, MAX_FRAGMENT_LEN)), false, false);
    }
  }

  // ---------------------------------------------------------------------------
  // Typed field access
  // ---------------------------------------------------------------------------

  private static CaseException caseErr(String message) {
    return new CaseException(message);
  }

  private static byte[] bytesOf(Object v, String what) {
    if (v instanceof byte[] b) {
      return b;
    }
    throw caseErr(what + " must be bytes");
  }

  private static String textOf(Object v, String what) {
    if (v instanceof String s) {
      return s;
    }
    throw caseErr(what + " must be a string");
  }

  private static boolean boolOf(Object v, String what) {
    if (v instanceof Boolean b) {
      return b;
    }
    throw caseErr(what + " must be a bool");
  }

  private static BigInteger intOf(Object v, String what) {
    if (v instanceof BigInteger b) {
      return b;
    }
    throw caseErr(what + " must be an int");
  }

  /** An int that drives a loop or an allocation: 0 &lt;= n &lt;= MAX_SIZE. */
  private static int sizeOf(Object v, String what) {
    BigInteger b = intOf(v, what);
    if (b.signum() < 0 || b.compareTo(BigInteger.valueOf(MAX_SIZE)) > 0) {
      throw caseErr(what + " " + show(b) + " outside [0, " + MAX_SIZE + "]");
    }
    return b.intValue();
  }

  /**
   * Maps a corpus width to an int; anything outside [0, 64] becomes -1, which the
   * mutators treat as "not one of 1, 2, 4, 8" (input unchanged).
   */
  private static int widthOf(Object v) {
    BigInteger b = intOf(v, "width");
    if (b.signum() < 0 || b.compareTo(BigInteger.valueOf(64)) > 0) {
      return -1;
    }
    return b.intValue();
  }

  private static Endian endianOf(Object v) {
    String s = textOf(v, "endian");
    return switch (s) {
      case "little" -> Endian.LITTLE;
      case "big" -> Endian.BIG;
      default -> throw caseErr("endian must be 'little' or 'big', got " + show(s));
    };
  }

  private static List<String> textsOf(Object v, String what) {
    if (!(v instanceof List<?> list)) {
      throw caseErr(what + " must be a list of strings");
    }
    List<String> out = new ArrayList<>(list.size());
    for (Object item : list) {
      if (!(item instanceof String s)) {
        throw caseErr(what + " must be a list of strings");
      }
      out.add(s);
    }
    return out;
  }

  private static final BigInteger TWO_64 = BigInteger.ONE.shiftLeft(64);

  /** A list of edge ids; each must fit in an unsigned 64-bit word (carried in a long). */
  private static long[] edgesOf(Object v, String what) {
    if (!(v instanceof List<?> list)) {
      throw caseErr(what + " must be a list of ints");
    }
    long[] out = new long[list.size()];
    for (int i = 0; i < out.length; i++) {
      if (!(list.get(i) instanceof BigInteger b)) {
        throw caseErr(what + " must be a list of ints");
      }
      if (b.signum() < 0 || b.compareTo(TWO_64) >= 0) {
        throw caseErr(what + " element " + show(b) + " outside [0, 2^64)");
      }
      out[i] = b.longValue();
    }
    return out;
  }

  // ---------------------------------------------------------------------------
  // Per-op checks
  // ---------------------------------------------------------------------------

  private static List<String> checkMutator(String cid, String op, Map<String, Object> v, long[] work) {
    byte[] got;
    switch (op) {
      case "mutate_bit_flip" -> {
        got = Hooks.bitFlip.apply(bytesOf(v.get("input"), "input"), intOf(v.get("bit_offset"), "bit_offset"), intOf(v.get("count"), "count"));
      }
      case "mutate_byte_flip" -> {
        got = Hooks.byteFlip.apply(bytesOf(v.get("input"), "input"), intOf(v.get("byte_offset"), "byte_offset"), intOf(v.get("count"), "count"));
      }
      case "mutate_arithmetic" -> {
        byte[] in = bytesOf(v.get("input"), "input");
        BigInteger o = intOf(v.get("byte_offset"), "byte_offset");
        BigInteger d = intOf(v.get("delta"), "delta");
        int w = widthOf(v.get("width"));
        Endian en = endianOf(v.get("endian"));
        got = Hooks.byteArithmetic.apply(in, o, d, w, en);
      }
      case "mutate_boundary" -> {
        byte[] in = bytesOf(v.get("input"), "input");
        BigInteger o = intOf(v.get("byte_offset"), "byte_offset");
        int w = widthOf(v.get("width"));
        BigInteger val = intOf(v.get("val"), "val");
        Endian en = endianOf(v.get("endian"));
        got = Hooks.boundaryInsert.apply(in, o, w, val, en);
      }
      case "mutate_block_delete" -> {
        got = Hooks.blockDelete.apply(bytesOf(v.get("input"), "input"), intOf(v.get("offset"), "offset"), intOf(v.get("length"), "length"));
      }
      case "mutate_block_insert" -> {
        byte[] in = bytesOf(v.get("input"), "input");
        BigInteger o = intOf(v.get("offset"), "offset");
        byte[] ins = bytesOf(v.get("insert_bytes"), "insert_bytes");
        got = Hooks.blockInsert.apply(in, o, ins);
      }
      case "mutate_block_duplicate" -> {
        byte[] in = bytesOf(v.get("input"), "input");
        BigInteger s = intOf(v.get("src_offset"), "src_offset");
        BigInteger l = intOf(v.get("length"), "length");
        BigInteger d = intOf(v.get("dst_offset"), "dst_offset");
        got = Hooks.blockDuplicate.apply(in, s, l, d);
      }
      case "mutate_splice" -> {
        byte[] tgt = bytesOf(v.get("target"), "target");
        byte[] donor = bytesOf(v.get("donor"), "donor");
        BigInteger ts = intOf(v.get("target_split"), "target_split");
        BigInteger ds = intOf(v.get("donor_split"), "donor_split");
        got = Hooks.splice.apply(tgt, donor, ts, ds);
        List<String> problems = compare(cid, op, got, v);
        long want = Mutate.spliceLength(tgt.length, donor.length, Mutate.satLong(ts), Mutate.satLong(ds));
        if (got.length != want) {
          problems.add(cid + ": splice length is not prefix + suffix");
        }
        return problems;
      }
      default -> { // mutate_sequence
        byte[] in = bytesOf(v.get("input"), "input");
        BigInteger seed = intOf(v.get("seed"), "seed");
        int steps = sizeOf(v.get("steps"), "steps");
        // Charge the REAL work: the budget handed down is what is left of the aggregate cap,
        // and what the call actually copied (block inserts and duplicates grow the buffer,
        // so steps x initial length would undercount) is deducted afterwards.
        // A call that hits its cap is charged too (the work it did before refusing), and an
        // exhausted aggregate fails every later case at once without running it: otherwise each
        // failing case would be handed the full budget again.
        if (work[0] <= 0) {
          throw caseErr("mutate_sequence work budget of " + MAX_SEQUENCE_WORK + " byte-steps exhausted");
        }
        Mutate.Counted done;
        try {
          done = Hooks.mutateSequence.apply(in, Mutate.low64(seed), steps, Mutate.MAX_INPUT_BYTES, work[0]);
        } catch (Mutate.WorkExhausted e) {
          work[0] -= Math.max(e.work, 0);
          if (work[0] > 0) {
            work[0] = 0; // the aggregate is breached: nothing is left for any later case
          }
          throw caseErr("mutate_sequence work budget of " + MAX_SEQUENCE_WORK + " byte-steps exhausted");
        }
        work[0] -= done.work();
        got = done.data();
      }
    }
    List<String> problems = compare(cid, op, got, v);
    // Engine invariant, independent of the corpus's expected bytes.
    switch (op) {
      case "mutate_bit_flip", "mutate_byte_flip", "mutate_arithmetic", "mutate_boundary" -> {
        if (got.length != ((byte[]) v.get("input")).length) {
          problems.add(cid + ": " + op + " changed the length of its input");
        }
      }
      default -> { }
    }
    return problems;
  }

  private static String hex(byte[] b) {
    StringBuilder s = new StringBuilder(b.length * 2);
    for (byte x : b) {
      s.append(Character.forDigit((x >> 4) & 0xf, 16)).append(Character.forDigit(x & 0xf, 16));
    }
    return s.toString();
  }

  private static List<String> compare(String cid, String op, byte[] got, Map<String, Object> v) {
    List<String> problems = new ArrayList<>();
    if (!(v.get("output") instanceof byte[] want)) {
      problems.add(cid + ": " + op + " output must be bytes");
      return problems;
    }
    if (!Arrays.equals(got, want)) {
      problems.add(
          cid + ": " + op + " expected " + Clip.clip(hex(want), MAX_FRAGMENT_LEN) + ", got "
              + Clip.clip(hex(got), MAX_FRAGMENT_LEN));
    }
    return problems;
  }

  private static List<String> checkCorpusAdmit(String cid, Map<String, Object> v) {
    long[] existing = edgesOf(v.get("existing_edges"), "existing_edges");
    long[] trace = edgesOf(v.get("trace_edges"), "trace_edges");
    boolean admitted = boolOf(v.get("admitted"), "admitted");
    BigInteger newEdges = intOf(v.get("new_edges"), "new_edges");
    Bitmap.Novelty got = Hooks.corpusAdmit.apply(existing, trace);
    List<String> problems = new ArrayList<>();
    if (got.admitted() != admitted) {
      problems.add(cid + ": corpus_admit admitted " + got.admitted() + ", corpus " + admitted);
    }
    if (!same(BigInteger.valueOf(got.newEdges()), newEdges)) {
      problems.add(cid + ": corpus_admit new_edges " + got.newEdges() + ", corpus " + show(newEdges));
    }
    // Invariant: admission is exactly "some trace edge is unseen".
    Set<Long> have = new HashSet<>();
    for (long e : existing) {
      have.add(e);
    }
    boolean novel = false;
    for (long e : trace) {
      if (!have.contains(e)) {
        novel = true;
        break;
      }
    }
    if (got.admitted() != novel) {
      problems.add(cid + ": corpus_admit disagrees with set difference");
    }
    return problems;
  }

  private static List<String> checkCrashDedup(String cid, Map<String, Object> v) {
    String ka = textOf(v.get("crash_a_kind"), "crash_a_kind");
    List<String> fa = textsOf(v.get("crash_a_frames"), "crash_a_frames");
    String kb = textOf(v.get("crash_b_kind"), "crash_b_kind");
    List<String> fb = textsOf(v.get("crash_b_frames"), "crash_b_frames");
    boolean want = boolOf(v.get("is_duplicate"), "is_duplicate");
    boolean got = Hooks.isDuplicate.apply(ka, fa, kb, fb);
    List<String> problems = new ArrayList<>();
    if (got != want) {
      problems.add(cid + ": crash_dedup is_duplicate " + got + ", corpus " + want);
    }
    return problems;
  }

  private static List<String> checkPrivateReporting(String cid, Map<String, Object> v) {
    byte[] raw = bytesOf(v.get("raw_payload"), "raw_payload");
    String target = textOf(v.get("target"), "target");
    String fid = textOf(v.get("failure_id"), "failure_id");
    String sig = textOf(v.get("sig_hash"), "sig_hash");
    String tap = textOf(v.get("tap_line"), "tap_line");
    boolean wantLeak = boolOf(v.get("contains_raw_payload"), "contains_raw_payload");
    String line;
    try {
      line = Hooks.formatTap.apply(1, target, fid, sig);
    } catch (IllegalArgumentException e) {
      throw caseErr("format_tap_failure refused: " + e.getMessage());
    }
    List<String> problems = new ArrayList<>();
    if (!line.equals(tap)) {
      problems.add(
          cid + ": tap_line expected " + Clip.clip(tap, MAX_FRAGMENT_LEN) + ", got " + Clip.clip(line, MAX_FRAGMENT_LEN));
    }
    boolean leaked = raw.length > 0 && line.contains(hex(raw));
    if (leaked != wantLeak) {
      problems.add(cid + ": privacy verdict wrong, payload leaked: " + leaked);
    }
    if (line.indexOf('\r') >= 0 || line.indexOf('\n') >= 0) {
      problems.add(cid + ": failure line spans more than one line");
    }
    return problems;
  }
}
