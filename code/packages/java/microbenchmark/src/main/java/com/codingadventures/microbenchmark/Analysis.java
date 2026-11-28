package com.codingadventures.microbenchmark;

/** Median candidate/baseline ratio and its exact-rank confidence interval. */
public record Analysis(double ratio, Double low, Double high, String verdict) {
    public boolean hasInterval() { return low != null && high != null; }
}
