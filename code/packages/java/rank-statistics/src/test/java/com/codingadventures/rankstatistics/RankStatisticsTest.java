package com.codingadventures.rankstatistics;

import static org.junit.jupiter.api.Assertions.*;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Random;
import org.junit.jupiter.api.Test;

final class RankStatisticsTest {
    private static final ObjectMapper JSON = new ObjectMapper();

    @Test void medianEdgesAndDoesNotMutate() {
        double[] values = {4, 1, 3, 2};
        assertEquals(2.5, RankStatistics.median(values));
        assertArrayEquals(new double[] {4, 1, 3, 2}, values);
        double d = Math.nextUp(0.0);
        assertEquals(2 * d, RankStatistics.median(new double[] {d, 2 * d}));
        assertEquals(-2 * d, RankStatistics.median(new double[] {-2 * d, -d}));
        assertEquals(Double.MIN_NORMAL, RankStatistics.median(new double[] {Math.nextDown(Double.MIN_NORMAL), Double.MIN_NORMAL}));
        assertEquals(-Double.MIN_NORMAL, RankStatistics.median(new double[] {-Double.MIN_NORMAL, -Math.nextDown(Double.MIN_NORMAL)}));
        assertEquals(0, RankStatistics.median(new double[] {-Double.MAX_VALUE, Double.MAX_VALUE}));
        assertEquals(Double.MAX_VALUE, RankStatistics.median(new double[] {Double.MAX_VALUE, Double.MAX_VALUE}));
        assertThrows(IllegalArgumentException.class, () -> RankStatistics.median(new double[] {}));
        assertThrows(IllegalArgumentException.class, () -> RankStatistics.median(null));
        assertThrows(IllegalArgumentException.class, () -> RankStatistics.median(new double[] {Double.NaN}));
        assertNull(RankStatistics.medianInterval(new double[] {1.0}, 95));
        assertThrows(IllegalArgumentException.class, () -> RankStatistics.medianInterval(new double[] {1.0}, 0));
    }

    @Test void exactRankMatchesIndependentSignEnumeration() {
        for (int n = 1; n <= 12; n++) for (int confidence = 1; confidence <= 99; confidence++) {
            int expected = 0;
            for (int k = 1; k <= n / 2; k++) {
                long tail = 0;
                for (long mask = 0; mask < (1L << n); mask++) if (Long.bitCount(mask) < k) tail++;
                if (200 * tail <= (100 - confidence) * (1L << n)) expected = k;
                else break;
            }
            assertEquals(expected, RankStatistics.exactMedianRank(n, confidence));
        }
    }

    @Test void deterministicGeneratedSamplesRespectOrderInvariants() {
        Random random = new Random(0x5eedL);
        for (int trial = 0; trial < 500; trial++) {
            double[] sample = new double[1 + random.nextInt(96)];
            for (int i = 0; i < sample.length; i++) sample[i] = random.nextInt(2001) - 1000;
            double[] original = sample.clone();
            double median = RankStatistics.median(sample);
            RankStatistics.Interval interval = RankStatistics.medianInterval(sample, 90);
            assertTrue(median >= -1000 && median <= 1000);
            if (interval != null) assertTrue(interval.low() <= median && median <= interval.high());
            assertArrayEquals(original, sample);
        }
    }

    @Test void sharedFixtureIsPresentAndConforms() throws Exception {
        Path local = Path.of("testdata", "microbenchmark-pairs-v1.json");
        Path canonical = Path.of("../../../specs/fixtures/microbenchmark-pairs-v1.json");
        assertArrayEquals(Files.readAllBytes(canonical), Files.readAllBytes(local));
        JsonNode cases = JSON.readTree(local.toFile()).path("cases");
        assertFalse(cases.isMissingNode());
        int confidence = JSON.readTree(local.toFile()).path("confidence_percent").intValue();
        for (JsonNode sample : cases) {
            JsonNode candidates = sample.path("candidate_ns");
            double[] ratios = new double[candidates.size()];
            for (int i = 0; i < ratios.length; i++) {
                ratios[i] = candidates.get(i).longValue() / (double) sample.path("baseline_ns").longValue();
            }
            assertEquals(sample.path("ratio").doubleValue(), RankStatistics.median(ratios), 1e-12);
            RankStatistics.Interval interval = RankStatistics.medianInterval(ratios, confidence);
            JsonNode want = sample.path("interval");
            if (want.isNull()) assertNull(interval);
            else {
                assertNotNull(interval);
                assertEquals(want.get(0).doubleValue(), interval.low(), 1e-12);
                assertEquals(want.get(1).doubleValue(), interval.high(), 1e-12);
            }
        }
    }
}
