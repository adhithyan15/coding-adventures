package com.codingadventures.rankstatistics;

import java.math.BigInteger;
import java.util.Arrays;

/** Finite-sample median operations with an exact distribution-free rank cutoff. */
public final class RankStatistics {
    private RankStatistics() {}

    /** A closed interval whose endpoints are observations from the sample. */
    public record Interval(double low, double high) {}

    private static double[] sortedFinite(double[] values) {
        if (values == null) throw new IllegalArgumentException("sample must not be null");
        if (values.length == 0) throw new IllegalArgumentException("sample must not be empty");
        double[] copy = values.clone();
        for (double value : copy) {
            if (!Double.isFinite(value)) throw new IllegalArgumentException("sample values must be finite");
        }
        Arrays.sort(copy);
        return copy;
    }

    private static double midpoint(double low, double high) {
        double sum = low + high;
        // Adding first preserves subnormal ties; only an overflowing sum needs the half-operands fallback.
        return Double.isFinite(sum) ? sum / 2.0 : low / 2.0 + high / 2.0;
    }

    /** Returns the median without changing the caller's array. */
    public static double median(double[] values) {
        double[] ordered = sortedFinite(values);
        int middle = ordered.length / 2;
        return (ordered.length & 1) == 1 ? ordered[middle] : midpoint(ordered[middle - 1], ordered[middle]);
    }

    /** Returns an exact two-sided interval, or null when no rank qualifies. */
    public static Interval medianInterval(double[] values, int confidencePercent) {
        if (confidencePercent < 1 || confidencePercent > 99) {
            throw new IllegalArgumentException("confidence must be from 1 through 99");
        }
        double[] ordered = sortedFinite(values);
        int rank = exactMedianRank(ordered.length, confidencePercent);
        return rank == 0 ? null : new Interval(ordered[rank - 1], ordered[ordered.length - rank]);
    }

    /**
     * Largest qualifying one-based rank; zero means the interval is unavailable.
     * For each j, coefficient is C(n,j), so tail accumulates exactly the
     * lower binomial tail. The inequality is multiplied through by 100*2^n;
     * BigInteger means a rounded probability can never move a boundary rank.
     */
    static int exactMedianRank(int n, int confidencePercent) {
        BigInteger maxTail = BigInteger.valueOf(100L - confidencePercent).shiftLeft(n);
        BigInteger coefficient = BigInteger.ONE;
        BigInteger tail = BigInteger.ZERO;
        BigInteger twoHundred = BigInteger.valueOf(200);
        int rank = 0;
        for (int j = 0; j <= n / 2; j++) {
            tail = tail.add(coefficient);
            if (tail.multiply(twoHundred).compareTo(maxTail) > 0) break;
            rank = j + 1;
            coefficient = coefficient.multiply(BigInteger.valueOf(n - (long) j))
                    .divide(BigInteger.valueOf(j + 1L));
        }
        return rank;
    }
}
