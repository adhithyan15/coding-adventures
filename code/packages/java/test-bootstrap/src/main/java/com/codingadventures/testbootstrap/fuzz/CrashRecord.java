package com.codingadventures.testbootstrap.fuzz;

import java.util.List;

/**
 * One deduplicated crash. The minimal input is PRIVATE (VER06 section 6.3): it is
 * reachable only through {@link #minimalInput()}, never through {@link #toString()},
 * and public output goes through {@link Signature#formatTapFailure} only.
 *
 * <p>This is a class, not a record, precisely so the generated {@code toString},
 * {@code equals} and accessor of a record cannot print the bytes.
 */
public final class CrashRecord {
  private final String signature;
  private final String kind;
  private final List<String> frames;
  private int hits;
  private boolean minimized;
  private byte[] minimal;
  private boolean inputDropped;

  CrashRecord(String signature, String kind, List<String> frames, byte[] minimal, boolean minimized, boolean dropped) {
    this.signature = signature;
    this.kind = kind;
    this.frames = List.copyOf(frames);
    this.hits = 1;
    this.minimized = minimized;
    this.minimal = minimal;
    this.inputDropped = dropped;
  }

  /** The 8-hex-digit signature. */
  public String signature() {
    return signature;
  }

  /** The normalised crash kind (a class name). */
  public String kind() {
    return kind;
  }

  /** The normalised frames, innermost first. */
  public List<String> frames() {
    return frames;
  }

  /** How many campaign executions hit this signature (triage included). */
  public int hits() {
    return hits;
  }

  /**
   * False when minimisation was skipped or cut short by a budget, so the minimal
   * input is only the shortest input seen, not a fixed point.
   */
  public boolean minimized() {
    return minimized;
  }

  /** True when the retained-bytes cap forced the input to be dropped. */
  public boolean inputDropped() {
    return inputDropped;
  }

  /** A copy of the shortest input known to reproduce the signature. PRIVATE: never log it. */
  public byte[] minimalInput() {
    return minimal.clone();
  }

  int minimalLength() {
    return minimal.length;
  }

  void hit() {
    hits++;
  }

  void replaceMinimal(byte[] shorter) {
    minimal = shorter;
  }

  /** Signature, kind, frames and counts; never input bytes. */
  @Override
  public String toString() {
    return "CrashRecord{sig=" + signature + " kind=" + kind + " frames=" + frames + " hits=" + hits
        + " minimized=" + minimized + "}";
  }
}
