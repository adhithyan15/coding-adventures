package com.codingadventures.testbootstrap.fuzz;

import com.codingadventures.testbootstrap.prng.SplitMix64;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Function;

/**
 * The deterministic, budgeted coverage-guided campaign (VER06 sections 5 and 9.1,
 * VER06f sections 5 and 6).
 *
 * <p>A campaign is a pure function of (corpus, options, target): triage every
 * seed, then repeatedly pick a seed, give it an energy-scaled number of mutated
 * children, run each child and either admit it (new coverage), record it (crash,
 * timeout, OOM) or drop it (redundant). Nothing here reads a clock, a file or the
 * environment, and nothing starts a thread.
 */
public final class Fuzz {
  private Fuzz() {}

  /** Distinct crash signatures a report holds; further novel ones only increment {@code crashesDropped}. */
  public static final int MAX_CRASHES = 1000;

  /** Seeds accepted by one campaign. */
  public static final int MAX_INITIAL_CORPUS = 10_000;

  /**
   * Bytes of corpus seeds plus crash inputs one campaign retains (VER06f section 6).
   * Go bounds the same quantity only by {@code maxInputLen x 65,536}.
   */
  public static final long MAX_RETAINED_BYTES = 256L << 20;

  /** Stack elements inspected per crash, however deep the recursion (CWE-400). */
  static final int MAX_WALK = 4096;

  /** The code under test, as outcomes: runs {@code data}, reports edges into {@code cov}. */
  @FunctionalInterface
  public interface ExecFunc {
    /** Executes one input. The campaign resets {@code cov} before each call. */
    Outcome run(byte[] data, Bitmap cov);
  }

  // ---------------------------------------------------------------------------
  // The executor
  // ---------------------------------------------------------------------------

  /**
   * Runs a bounded coverage-guided campaign against {@code target}. See
   * {@link #runOutcomes} for the contracts. The coverage tier is
   * {@code caller-reported} when the target called {@code h.edge} at least once
   * during the campaign, {@code black-box-uninstrumented} otherwise.
   */
  public static Report run(List<byte[]> initialCorpus, Options opts, Target target) {
    if (target == null) {
      throw new IllegalArgumentException("target is null");
    }
    opts.validate();
    Handle h = new Handle(opts.stepBudget(), opts.memBudget());
    return runOutcomes(
        initialCorpus,
        opts,
        (data, cov) -> {
          h.cov = cov;
          h.steps = 0;
          h.reserved = 0;
          h.exhausted = null;
          return runTarget(target, data, h);
        });
  }

  /**
   * Runs one execution. Its class and name delimit the target's frames: the stack
   * is cut here, so everything the campaign did above it is never inspected.
   *
   * <p>Only the target runs inside this {@code try}. A real {@link OutOfMemoryError}
   * is an OOM outcome, and nothing is rendered from it (the message and the stack
   * are never read on that path); every other throwable goes to {@link #classify}.
   */
  static Outcome runTarget(Target target, byte[] data, Handle h) {
    // The copy is made OUTSIDE the try: an out-of-memory raised by the engine's own copy
    // is the engine's problem, not a verdict about the target.
    byte[] copy = data.clone();
    try {
      target.run(copy, h);
    } catch (OutOfMemoryError e) {
      return Outcome.oom();
    } catch (Throwable t) { // a target may throw anything; every kind is a crash
      // ... unless its budget had already run out: a target that swallowed the sentinel and
      // then threw something ordinary is still a timeout or an OOM, not a crash.
      return h.exhausted != null ? exhaustedOutcome(h) : classify(t, h);
    }
    // A target that swallowed the sentinel and returned is not a pass either.
    return h.exhausted != null ? exhaustedOutcome(h) : Outcome.pass();
  }

  private static Outcome exhaustedOutcome(Handle h) {
    return h.exhausted.budget == Handle.Budget.STEPS ? Outcome.timeout() : Outcome.oom();
  }

  /** Turns a throwable into an outcome. Budget sentinels of another handle are rethrown to their owner. */
  static Outcome classify(Throwable t, Handle h) {
    if (t instanceof Handle.BudgetExceeded b) {
      if (b.owner != h) {
        throw b; // another campaign's budget: let it unwind to its owner
      }
      return b.budget == Handle.Budget.STEPS ? Outcome.timeout() : Outcome.oom();
    }
    // Rendering an untrusted throwable: class names cannot be overridden, but
    // getStackTrace can, and it can raise ANYTHING (a custom Error, an InternalError).
    // Whatever it raises, the campaign goes on: an out-of-memory is a counted OOM,
    // everything else means "no frames".
    String kind = Signature.normaliseKind(t.getClass().getName());
    List<String> frames;
    try {
      frames = framesOf(t.getStackTrace());
    } catch (OutOfMemoryError e) {
      return Outcome.oom(); // the stack walk itself ran out of memory: a counted OOM
    } catch (Throwable hostile) { // a custom Error from a getStackTrace override must not end the campaign
      frames = List.of();
    }
    return Outcome.crash(kind, frames);
  }

  private static final String HARNESS_CLASS = Fuzz.class.getName();
  private static final String HANDLE_CLASS = Handle.class.getName();

  private static boolean isJdk(String cls) {
    return cls.startsWith("java.")
        || cls.startsWith("javax.")
        || cls.startsWith("jdk.")
        || cls.startsWith("sun.")
        || cls.startsWith("com.sun.");
  }

  /**
   * Normalises a captured stack: innermost first, engine and JDK frames dropped, cut
   * at the executor's own frame, consecutive duplicates collapsed, at most 64 kept.
   * At most {@link #MAX_WALK} elements are inspected, so a hostile array cannot
   * make this loop long.
   */
  static List<String> framesOf(StackTraceElement[] stack) {
    List<String> frames = new ArrayList<>();
    String prevRaw = "";
    int limit = Math.min(stack.length, MAX_WALK);
    for (int i = 0; i < limit; i++) {
      StackTraceElement f = stack[i];
      String cls = f.getClassName();
      String method = f.getMethodName();
      if (cls.equals(HARNESS_CLASS) && method.equals("runTarget")) {
        break;
      }
      if (cls.equals(HARNESS_CLASS) || cls.equals(HANDLE_CLASS) || isJdk(cls)) {
        continue; // engine / JDK plumbing, not the target's
      }
      String raw = cls + "#" + method;
      if (frames.isEmpty() || !raw.equals(prevRaw)) {
        prevRaw = raw;
        frames.add(Signature.normaliseFrame(cls, method));
        if (frames.size() >= Signature.MAX_FRAMES) {
          break;
        }
      }
    }
    return Signature.collapse(frames);
  }

  // ---------------------------------------------------------------------------
  // Minimisation (VER06 section 5.3)
  // ---------------------------------------------------------------------------

  private static boolean sameCrash(Outcome o, String sig) {
    return o.kind() == Outcome.Kind.CRASH && Signature.crashSignature(o.crashKind(), o.frames()).equals(sig);
  }

  /** The result of a minimisation: the bytes, executions spent, and whether it finished. */
  record Minimized(byte[] data, int spent, boolean finished) {}

  /**
   * Shrinks {@code data} while it keeps the exact signature. Every candidate
   * execution, accepted or rejected, is counted in {@code spent}, and the budget is
   * checked before each one, so no stream of rejected candidates can run past it.
   */
  static Minimized minimize(byte[] data, String sig, Function<byte[], Outcome> test, int budget) {
    byte[] cur = data.clone();
    int spent = 0;
    if (cur.length == 0) {
      return new Minimized(cur, 0, true);
    }
    // Pass 1: delete blocks, halving the block size. Starting at max(1, len/2)
    // (not len/2) is what makes the empty input reachable from a 1-byte crasher.
    for (int chunk = Math.max(1, cur.length / 2); chunk > 0; chunk /= 2) {
      int offset = 0;
      while (offset < cur.length) {
        if (spent >= budget) {
          return new Minimized(cur, spent, false);
        }
        int end = offset + Math.min(chunk, cur.length - offset);
        byte[] candidate = new byte[cur.length - (end - offset)];
        System.arraycopy(cur, 0, candidate, 0, offset);
        System.arraycopy(cur, end, candidate, offset, cur.length - end);
        spent++;
        if (sameCrash(test.apply(candidate), sig)) {
          cur = candidate; // same offset again: the tail slid left
        } else {
          offset += chunk;
        }
      }
    }
    // Pass 2: zero individual non-essential bytes.
    for (int i = 0; i < cur.length; i++) {
      if (cur[i] == 0) {
        continue;
      }
      if (spent >= budget) {
        return new Minimized(cur, spent, false);
      }
      byte original = cur[i];
      cur[i] = 0;
      spent++;
      if (!sameCrash(test.apply(cur.clone()), sig)) {
        cur[i] = original;
      }
    }
    return new Minimized(cur, spent, true);
  }

  /**
   * Shrinks {@code data} while it keeps the EXACT crash signature: pass 1 deletes
   * blocks, pass 2 zeroes bytes. A candidate that crashes differently is rejected,
   * so minimisation never trades one bug for another. {@code budget} caps the
   * executions of {@code test}; it must be in [0, {@link Options#MAX_MINIMIZE_BUDGET}]
   * and {@code data} at most {@link Mutate#MAX_INPUT_BYTES}.
   */
  public static byte[] minimizeCrashingInput(byte[] data, String sig, Function<byte[], Outcome> test, int budget) {
    if (budget < 0 || budget > Options.MAX_MINIMIZE_BUDGET) {
      throw new IllegalArgumentException("budget must be in [0, " + Options.MAX_MINIMIZE_BUDGET + "]");
    }
    if (test == null) {
      throw new IllegalArgumentException("test is null");
    }
    if (data.length > Mutate.MAX_INPUT_BYTES) {
      throw new IllegalArgumentException("data exceeds " + Mutate.MAX_INPUT_BYTES + " bytes");
    }
    return minimize(data, sig, test, budget).data();
  }

  // ---------------------------------------------------------------------------
  // The campaign
  // ---------------------------------------------------------------------------

  private record SeedEntry(byte[] data, int newEdges, int generation) {}

  private static final class Campaign {
    final ExecFunc exec;
    final Options opts;
    final SplitMix64 rng;
    final Bitmap step = new Bitmap();
    final Bitmap cumulative = new Bitmap();
    final List<SeedEntry> corpus = new ArrayList<>();
    final List<CrashRecord> crashes = new ArrayList<>();
    final Map<String, Integer> bySig = new HashMap<>();
    int timeouts;
    int ooms;
    int dropped;
    int admissionsDropped;
    int minimizeLeft;
    long retained;
    final long retainedCap;

    Campaign(ExecFunc exec, Options opts, long retainedCap) {
      this.exec = exec;
      this.opts = opts;
      this.rng = new SplitMix64(opts.seed());
      this.minimizeLeft = opts.maxIterations();
      this.retainedCap = retainedCap;
    }

    /** Runs one input; the edges of the run are left in {@code step}. */
    Outcome execute(byte[] data) {
      step.reset();
      Outcome out = exec.run(data, step);
      if (out == null || out.kind() == null) {
        throw new IllegalArgumentException("exec returned no outcome");
      }
      return out;
    }

    /** The minimiser's re-run: coverage is discarded and nothing counted. */
    Outcome probe(byte[] data) {
      return execute(data);
    }

    /** Puts one non-passing execution in its bucket. */
    void file(Outcome out, byte[] data) {
      switch (out.kind()) {
        case TIMEOUT -> timeouts++;
        case OOM -> ooms++;
        case CRASH -> recordCrash(out, data);
        default -> { }
      }
    }

    /**
     * Puts a passing input in the corpus iff it covered a new edge. The first passing
     * seed is always admitted, even with no coverage, so there is something to mutate
     * in black-box mode.
     */
    void admit(byte[] data, int generation) {
      Bitmap.Admission a = cumulative.admit(step.trace());
      if (a.hasNew() || (generation == 0 && corpus.isEmpty())) {
        if (retained + data.length > retainedCap) {
          admissionsDropped++;
          return;
        }
        retained += data.length;
        corpus.add(new SeedEntry(data.clone(), a.countNew(), generation));
      }
    }

    void recordCrash(Outcome out, byte[] data) {
      String sig = Signature.crashSignature(out.crashKind(), out.frames());
      Integer at = bySig.get(sig);
      if (at != null) {
        CrashRecord rec = crashes.get(at);
        rec.hit();
        if (!rec.inputDropped() && data.length < rec.minimalLength()) {
          retained -= rec.minimalLength() - data.length;
          rec.replaceMinimal(data.clone());
        }
        return;
      }
      if (crashes.size() >= MAX_CRASHES) {
        dropped++;
        return;
      }
      int allowance = Math.min(opts.minimizeBudget(), minimizeLeft);
      Minimized m = minimize(data, sig, this::probe, allowance);
      minimizeLeft -= m.spent();
      byte[] kept = m.data();
      boolean droppedInput = false;
      if (retained + kept.length > retainedCap) {
        kept = new byte[0];
        droppedInput = true;
      } else {
        retained += kept.length;
      }
      bySig.put(sig, crashes.size());
      crashes.add(new CrashRecord(sig, out.crashKind(), out.frames(), kept, m.finished() && !droppedInput, droppedInput));
    }
  }

  /**
   * Runs a bounded coverage-guided campaign over {@code exec}.
   *
   * <p>Contracts (VER06 section 9.1), each pinned by a test:
   *
   * <ul>
   *   <li>a pure function of (corpus, options, exec): same inputs, same report;
   *   <li>with a live corpus {@code iterations == opts.maxIterations()} exactly, and
   *       no mutated candidate exceeds {@code opts.maxInputLen()} (initial seeds run
   *       as given, up to {@link Mutate#MAX_INPUT_BYTES} each);
   *   <li>an empty corpus is replaced by one {@code 00} seed; seeds that crash, time
   *       out or exhaust memory are never mutated, so a corpus of crashers fuzzes
   *       nothing (iterations == 0);
   *   <li>every execution lands in crashes, timeouts, ooms or the corpus; {@code
   *       isClean} needs at least one iteration and no failure at all;
   *   <li>one {@link CrashRecord} per signature; a novel crash is minimised.
   * </ul>
   *
   * <p>Per iteration the stream draws: {@code bounded(len(corpus))} for the seed; per
   * energy slot {@code 1 + bounded(4)} steps, one {@code nextU64} sub-seed, then, if
   * {@code len(corpus) > 1}, {@code bounded(4)} and on 0 a donor index and two split
   * draws (as the Go, Rust and Python lanes).
   */
  public static Report runOutcomes(List<byte[]> initialCorpus, Options opts, ExecFunc exec) {
    return runOutcomes(initialCorpus, opts, exec, MAX_RETAINED_BYTES);
  }

  /** {@link #runOutcomes(List, Options, ExecFunc)} with an explicit retained-bytes cap (the test seam). */
  static Report runOutcomes(List<byte[]> initialCorpus, Options opts, ExecFunc exec, long retainedCap) {
    opts.validate();
    if (exec == null) {
      throw new IllegalArgumentException("exec is null");
    }
    if (initialCorpus.size() > MAX_INITIAL_CORPUS) {
      throw new IllegalArgumentException("initial corpus exceeds " + MAX_INITIAL_CORPUS + " seeds");
    }
    for (int i = 0; i < initialCorpus.size(); i++) {
      if (initialCorpus.get(i).length > Mutate.MAX_INPUT_BYTES) {
        throw new IllegalArgumentException("initial corpus seed " + i + " exceeds " + Mutate.MAX_INPUT_BYTES + " bytes");
      }
    }
    List<byte[]> seeds = initialCorpus.isEmpty() ? List.of(new byte[] {0}) : initialCorpus;
    Campaign c = new Campaign(exec, opts, retainedCap);

    // 1. Triage: run every seed once.
    for (byte[] data : seeds) {
      Outcome out = c.execute(data);
      if (out.kind() == Outcome.Kind.PASS) {
        c.admit(data, 0);
      } else {
        c.file(out, data);
      }
    }

    // 2. The mutation loop.
    int iterations = 0;
    while (iterations < opts.maxIterations() && !c.corpus.isEmpty()) {
      SeedEntry seed = c.corpus.get((int) c.rng.bounded(c.corpus.size()));
      int energy = Bitmap.calculateEnergy(opts.baseEnergy(), seed.data().length, seed.newEdges(), c.cumulative.count());
      for (int slot = 0; slot < energy && iterations < opts.maxIterations(); slot++) {
        iterations++;
        int steps = 1 + (int) c.rng.bounded(4);
        long sub = c.rng.nextU64();
        byte[] candidate = Mutate.mutateSequence(seed.data(), sub, steps, Mutate.MAX_INPUT_BYTES); // steps <= 4
        if (c.corpus.size() > 1 && c.rng.bounded(4) == 0) {
          byte[] donor = c.corpus.get((int) c.rng.bounded(c.corpus.size())).data();
          long ts = c.rng.bounded(candidate.length + 1L);
          long ds = c.rng.bounded(donor.length + 1L);
          candidate = Mutate.splice(candidate, donor, ts, ds);
        }
        if (candidate.length > opts.maxInputLen()) {
          candidate = java.util.Arrays.copyOf(candidate, opts.maxInputLen());
        }
        Outcome out = c.execute(candidate);
        if (out.kind() == Outcome.Kind.PASS) {
          c.admit(candidate, seed.generation() + 1);
        } else {
          c.file(out, candidate);
        }
      }
    }

    int total = c.cumulative.count();
    List<byte[]> corpus = new ArrayList<>(c.corpus.size());
    for (SeedEntry s : c.corpus) {
      corpus.add(s.data());
    }
    return new Report(
        iterations,
        c.corpus.size(),
        total,
        c.crashes,
        c.timeouts,
        c.ooms,
        c.dropped,
        c.admissionsDropped,
        total > 0 ? Report.TIER_REPORTED : Report.TIER_BLACK_BOX,
        opts.seed(),
        corpus);
  }
}
