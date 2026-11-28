package com.codingadventures.microbenchmark;

import com.codingadventures.rankstatistics.RankStatistics;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Random;

/** Paired collector and VER07b analysis. Timing thresholds never decide tests. */
public final class Microbenchmark {
    private Microbenchmark() {}
    private static final int MAXIMUM_PAIRS = 10_000;
    private static final String BASE_FIRST = "baseline-first";
    private static final String CANDIDATE_FIRST = "candidate-first";

    /**
     * Converts each signed duration to binary64 before dividing, then delegates
     * order statistics to the sibling rank-statistics module. The verdict tests
     * the confidence interval against the two strict speed boundaries.
     */
    public static Analysis analyze(List<Pair> pairs, int confidencePercent, double factor) {
        if (pairs == null || pairs.isEmpty()) throw new IllegalArgumentException("at least one pair is required");
        if (pairs.size() > MAXIMUM_PAIRS) throw new IllegalArgumentException("pairs must be from 1 through 10000");
        if (confidencePercent < 1 || confidencePercent > 99) throw new IllegalArgumentException("confidence must be from 1 through 99");
        if (!Double.isFinite(factor) || factor <= 1.0) throw new IllegalArgumentException("factor must be finite and greater than 1");
        double[] ratios = new double[pairs.size()];
        for (int i = 0; i < pairs.size(); i++) {
            Pair p = pairs.get(i);
            if (p == null || ( !BASE_FIRST.equals(p.order()) && !CANDIDATE_FIRST.equals(p.order()))) throw new IllegalArgumentException("pair has invalid order");
            if (p.baselineNs() <= 0 || p.candidateNs() <= 0) throw new IllegalArgumentException("pair durations must be positive signed 64-bit integers");
            ratios[i] = (double) p.candidateNs() / (double) p.baselineNs();
        }
        double median = RankStatistics.median(ratios);
        RankStatistics.Interval interval = RankStatistics.medianInterval(ratios, confidencePercent);
        if (interval == null) return new Analysis(median, null, null, "inconclusive");
        String verdict = "inconclusive";
        double fast = 1.0 / factor;
        if (interval.high() < fast) verdict = "faster";
        else if (interval.low() > factor) verdict = "slower";
        else if (interval.low() >= fast && interval.high() <= factor) verdict = "equivalent";
        return new Analysis(median, interval.low(), interval.high(), verdict);
    }

    /**
     * Warms both callbacks, doubles one shared batch size until both variants
     * meet the target, then measures adjacent shuffled pairs. A seeded local
     * PRNG makes each lane's schedule reproducible; every failure discards the
     * partial work and returns by exception.
     */
    public static Report collect(Config config, long expectedChecksum, Batch baseline,
                                 Batch candidate, Clock clock) throws Exception {
        config.validate();
        if (baseline == null || candidate == null || clock == null) throw new IllegalArgumentException("baseline, candidate, and clock are required");
        State state = new State(config, expectedChecksum, clock);
        state.verify(baseline, 1);
        state.verify(candidate, 1);
        long size = 1;
        while (true) {
            long b = state.measure(baseline, size, true);
            long c = state.measure(candidate, size, true);
            if (b >= config.targetNs() && c >= config.targetNs()) break;
            if (size == config.maxBatch()) throw new IllegalArgumentException("calibration limit exhausted");
            size = size > config.maxBatch() / 2 ? config.maxBatch() : size * 2;
        }
        List<String> schedule = new ArrayList<>(config.pairs());
        for (int i = 0; i < config.pairs(); i++) schedule.add(i < (config.pairs() + 1) / 2 ? BASE_FIRST : CANDIDATE_FIRST);
        Collections.shuffle(schedule, new Random(config.seed()));
        List<Pair> pairs = new ArrayList<>(config.pairs());
        for (String order : schedule) {
            long b;
            long c;
            if (BASE_FIRST.equals(order)) { b = state.measure(baseline, size, false); c = state.measure(candidate, size, false); }
            else { c = state.measure(candidate, size, false); b = state.measure(baseline, size, false); }
            pairs.add(new Pair(b, c, order));
        }
        return new Report(config, size, pairs, analyze(pairs, config.confidencePercent(), config.minimumFactor()));
    }

    private static final class State {
        private final Config config;
        private final long expected;
        private final Clock clock;
        private long last = -1;
        State(Config config, long expected, Clock clock) { this.config = config; this.expected = expected; this.clock = clock; }
        private long wanted(long n) { return Math.multiplyExact(expected, n); }
        void verify(Batch batch, long n) throws Exception {
            if (batch.run(n) != wanted(n)) throw new IllegalArgumentException("checksum mismatch");
        }
        private long read() {
            long tick = clock.now();
            if (tick < 0 || (last >= 0 && tick < last)) throw new IllegalArgumentException("clock must be nonnegative and nonregressing");
            last = tick;
            return tick;
        }
        long measure(Batch batch, long n, boolean calibration) throws Exception {
            long wanted = wanted(n);
            long start = read();
            long got = batch.run(n);
            long end = read();
            if (got != wanted) throw new IllegalArgumentException("checksum mismatch");
            if (end == start && !calibration) throw new IllegalArgumentException("measured duration must be positive");
            return end - start;
        }
    }

    /** Normalize the platform's arbitrary System.nanoTime origin to zero. */
    public static Clock monotonicClock() {
        long origin = System.nanoTime();
        return () -> System.nanoTime() - origin;
    }
}
