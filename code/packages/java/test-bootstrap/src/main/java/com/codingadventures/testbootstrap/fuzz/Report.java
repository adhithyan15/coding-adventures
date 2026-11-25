package com.codingadventures.testbootstrap.fuzz;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * What a campaign found. Every execution lands in exactly one bucket.
 * {@link #iterations()} counts MUTATION executions (triage is not counted);
 * crashes, timeouts and OOMs count triage and mutation executions; anything that
 * passed is in the corpus or was redundant.
 *
 * <p>The seed and the corpus are PRIVATE (VER06 section 6): fields, not record
 * components, so no default rendering can reach them.
 */
public final class Report {
  /** Edges were reported by the target. */
  public static final String TIER_REPORTED = "caller-reported";

  /** No edge was collected, so there was no feedback. */
  public static final String TIER_BLACK_BOX = "black-box-uninstrumented";

  private final int iterations;
  private final int admittedSeeds;
  private final int totalEdges;
  private final List<CrashRecord> crashes;
  private final int timeouts;
  private final int ooms;
  private final int crashesDropped;
  private final int admissionsDropped;
  private final String coverageTier;
  private final long seed;
  private final List<byte[]> corpus;

  Report(
      int iterations,
      int admittedSeeds,
      int totalEdges,
      List<CrashRecord> crashes,
      int timeouts,
      int ooms,
      int crashesDropped,
      int admissionsDropped,
      String coverageTier,
      long seed,
      List<byte[]> corpus) {
    this.iterations = iterations;
    this.admittedSeeds = admittedSeeds;
    this.totalEdges = totalEdges;
    this.crashes = Collections.unmodifiableList(new ArrayList<>(crashes));
    this.timeouts = timeouts;
    this.ooms = ooms;
    this.crashesDropped = crashesDropped;
    this.admissionsDropped = admissionsDropped;
    this.coverageTier = coverageTier;
    this.seed = seed;
    this.corpus = corpus;
  }

  /** Mutation executions run. */
  public int iterations() {
    return iterations;
  }

  /** Seeds in the final corpus. */
  public int admittedSeeds() {
    return admittedSeeds;
  }

  /** Distinct covered edges. */
  public int totalEdges() {
    return totalEdges;
  }

  /** One record per distinct signature (at most {@link Fuzz#MAX_CRASHES}). */
  public List<CrashRecord> crashes() {
    return crashes;
  }

  /** Executions that exhausted their step budget. */
  public int timeouts() {
    return timeouts;
  }

  /** Executions that exhausted their memory budget. */
  public int ooms() {
    return ooms;
  }

  /** Novel signatures beyond {@link Fuzz#MAX_CRASHES}, so a cap never silently loses a failure. */
  public int crashesDropped() {
    return crashesDropped;
  }

  /** Corpus admissions refused by the retained-bytes cap (VER06f section 6). */
  public int admissionsDropped() {
    return admissionsDropped;
  }

  /** {@link #TIER_REPORTED} or {@link #TIER_BLACK_BOX}: what was actually collected. */
  public String coverageTier() {
    return coverageTier;
  }

  /**
   * Whether the campaign can be read as "found nothing": at least one mutation
   * iteration ran and no execution, triage included, crashed, timed out or ran out
   * of memory. A campaign that never got past triage, or ran zero iterations,
   * fuzzed nothing.
   */
  public boolean isClean() {
    return iterations > 0 && crashes.isEmpty() && timeouts == 0 && ooms == 0 && crashesDropped == 0;
  }

  /** The number of distinct crash signatures seen. */
  public int uniqueCrashes() {
    return crashes.size() + crashesDropped;
  }

  /** The campaign seed. PRIVATE. */
  public long seed() {
    return seed;
  }

  /** Copies of the admitted seeds. PRIVATE, like {@link CrashRecord#minimalInput()}. */
  public List<byte[]> corpus() {
    List<byte[]> out = new ArrayList<>(corpus.size());
    for (byte[] c : corpus) {
      out.add(c.clone());
    }
    return out;
  }

  /** Counts, never seeds or bytes. */
  @Override
  public String toString() {
    return "Report{iterations=" + iterations + " admitted=" + admittedSeeds + " edges=" + totalEdges
        + " crashes=" + crashes.size() + " timeouts=" + timeouts + " ooms=" + ooms
        + " dropped=" + crashesDropped + " admissionsDropped=" + admissionsDropped
        + " tier=" + coverageTier + " clean=" + isClean() + "}";
  }
}
