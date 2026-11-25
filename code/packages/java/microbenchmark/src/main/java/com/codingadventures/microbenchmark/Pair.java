package com.codingadventures.microbenchmark;

/** A pair of positive raw durations, retaining which implementation ran first. */
public record Pair(long baselineNs, long candidateNs, String order) {}
