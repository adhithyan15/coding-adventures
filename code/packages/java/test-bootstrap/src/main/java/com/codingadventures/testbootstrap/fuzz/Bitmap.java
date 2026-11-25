package com.codingadventures.testbootstrap.fuzz;

import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;

/**
 * A 65,536-bit execution map and the admission and scheduling rules around it
 * (VER06 section 4, VER06f section 4).
 *
 * <p>Bit {@code e} set means edge {@code e} was executed. Edge ids are reduced
 * modulo 65,536 as <em>unsigned</em> 64-bit numbers. Collisions are the known
 * price of a fixed-size map; AFL pays the same one.
 */
public final class Bitmap {
  /** Number of edges the map distinguishes. */
  public static final int COVERAGE_MAP_SIZE = 65_536;

  /** Size of the map in bytes. */
  public static final int COVERAGE_BYTES = COVERAGE_MAP_SIZE / 8;

  private final byte[] map = new byte[COVERAGE_BYTES];

  // The masked ids in the order first seen, beside the map, so listing the edges
  // of one execution costs O(edges hit), not O(8192).
  private int[] edges = new int[16];
  private int size;

  /** An empty map. */
  public Bitmap() {}

  private static int slotOf(long edge) {
    return (int) (edge & (COVERAGE_MAP_SIZE - 1)); // low 16 bits == unsigned mod 65536
  }

  /** Marks an edge as executed. */
  public void record(long edge) {
    int e = slotOf(edge);
    int idx = e >>> 3;
    int mask = 1 << (e & 7);
    if ((map[idx] & mask) == 0) {
      map[idx] |= (byte) mask;
      if (size == edges.length) {
        edges = Arrays.copyOf(edges, size * 2);
      }
      edges[size++] = e;
    }
  }

  /** Whether an edge is set. */
  public boolean has(long edge) {
    int e = slotOf(edge);
    return (map[e >>> 3] & (1 << (e & 7))) != 0;
  }

  /** The result of {@link #admit}: whether any edge was new and how many were. */
  public record Admission(boolean hasNew, int countNew) {}

  /** ORs the trace into this (cumulative) map; reports whether any edge was new and how many. */
  public Admission admit(int[] trace) {
    int fresh = 0;
    for (int e : trace) {
      if (!has(e)) {
        record(e);
        fresh++;
      }
    }
    return new Admission(fresh > 0, fresh);
  }

  /** Every covered edge in ascending order (a fresh array). */
  public int[] edges() {
    int[] out = Arrays.copyOf(edges, size);
    Arrays.sort(out);
    return out;
  }

  /** The edges in first-seen order (a fresh array); the campaign's per-execution trace. */
  int[] trace() {
    return Arrays.copyOf(edges, size);
  }

  /** The number of distinct covered edges (a population count). */
  public int count() {
    int n = 0;
    for (byte b : map) {
      n += Integer.bitCount(b & 0xFF);
    }
    return n;
  }

  /** ORs another map into this one. */
  public void merge(Bitmap other) {
    for (int i = 0; i < other.size; i++) {
      record(other.edges[i]);
    }
  }

  /** Clears the map in O(edges recorded). */
  public void reset() {
    for (int i = 0; i < size; i++) {
      int e = edges[i];
      map[e >>> 3] &= (byte) ~(1 << (e & 7));
    }
    size = 0;
  }

  /** A copy of the 8,192 bytes of the map (LSB-first within a byte). */
  public byte[] raw() {
    return map.clone();
  }

  /** The result of {@link #corpusAdmit}. */
  public record Novelty(boolean admitted, int newEdges) {}

  /**
   * The pure novelty check: does the trace hold an edge absent from {@code existing}?
   * It counts UNIQUE novel edges (a trace that repeats one novel edge three times
   * adds one). Ids are compared as full unsigned 64-bit words, not reduced.
   *
   * <pre>
   *   corpusAdmit([1 2 3], [2 4]) -&gt; (true, 1)    (edge 4 is new)
   *   corpusAdmit([1 2 3], [1 2]) -&gt; (false, 0)
   * </pre>
   */
  public static Novelty corpusAdmit(long[] existing, long[] trace) {
    Set<Long> have = new HashSet<>(Math.max(16, existing.length * 2));
    for (long e : existing) {
      have.add(e);
    }
    int novel = 0;
    for (long e : trace) {
      if (have.add(e)) { // count each novel edge once
        novel++;
      }
    }
    return new Novelty(novel > 0, novel);
  }

  /**
   * The mutation budget: {@code max(1, round(base * (1 + new / total) / sqrt(len)))}.
   *
   * <p>Short seeds that found a large share of the coverage get the most energy.
   * The edge bonus is 1.0 while nothing is covered (total == 0). Rounding is half
   * AWAY from zero ({@code floor(x + 0.5)}), as in the other lanes.
   *
   * <p>Worked example: base 10, len 4, 3 new of 6 total edges: 10 * 1.5 / 2 = 7.5
   * rounds to 8.
   */
  public static int calculateEnergy(int baseEnergy, int seedLen, int newEdges, int totalEdges) {
    double base = Math.max(1, baseEnergy);
    double bonus = totalEdges > 0 ? 1.0 + (double) newEdges / totalEdges : 1.0;
    double factor = Math.sqrt(Math.max(1, seedLen));
    // Clamp BEFORE the int conversion: no campaign runs more than MAX_ITERATIONS slots.
    double energy = Math.floor(base * bonus / factor + 0.5);
    return Math.max(1, (int) Math.min(energy, Mutate.MAX_ITERATIONS));
  }
}
