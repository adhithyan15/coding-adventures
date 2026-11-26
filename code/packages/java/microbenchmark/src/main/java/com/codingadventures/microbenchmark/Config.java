package com.codingadventures.microbenchmark;

/** Immutable choices fixed before paired samples are collected. */
public record Config(int pairs, int confidencePercent, double minimumFactor,
                     long seed, long targetNs, long maxBatch) {
    public static Config defaults() { return new Config(40, 95, 1.05, 0, 100_000, 65_536); }

    public void validate() {
        if (pairs < 1 || pairs > 10_000) throw new IllegalArgumentException("pairs must be from 1 through 10000");
        if (confidencePercent < 1 || confidencePercent > 99) throw new IllegalArgumentException("confidence must be from 1 through 99");
        if (!Double.isFinite(minimumFactor) || minimumFactor <= 1.0) throw new IllegalArgumentException("factor must be finite and greater than 1");
        if (targetNs <= 0 || maxBatch <= 0) throw new IllegalArgumentException("target and max batch must be positive signed 64-bit integers");
    }
}
