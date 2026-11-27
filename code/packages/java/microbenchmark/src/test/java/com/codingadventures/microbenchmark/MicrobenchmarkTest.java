package com.codingadventures.microbenchmark;

import static org.junit.jupiter.api.Assertions.*;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.ByteArrayOutputStream;
import java.io.OutputStream;
import java.io.PrintStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.atomic.AtomicLong;
import org.junit.jupiter.api.Test;

final class MicrobenchmarkTest {
    private static final ObjectMapper JSON = new ObjectMapper();
    private static final Config QUICK = new Config(5, 95, 1.05, 91, 1, 1);

    private static Clock ticks() { AtomicLong clock = new AtomicLong(); return () -> clock.getAndAdd(100); }
    private static Report report(Config config) throws Exception {
        return Microbenchmark.collect(config, 7, n -> 7 * n, n -> 7 * n, ticks());
    }

    @Test void analysisBoundariesAndInvalidObservations() {
        List<Pair> fast = new ArrayList<>();
        for (int i = 0; i < 6; i++) fast.add(new Pair(100, 80, i < 3 ? "baseline-first" : "candidate-first"));
        assertEquals("faster", Microbenchmark.analyze(fast, 95, 1.05).verdict());
        assertThrows(IllegalArgumentException.class, () -> Microbenchmark.analyze(List.of(), 95, 1.05));
        assertThrows(IllegalArgumentException.class, () -> Microbenchmark.analyze(List.of(new Pair(0, 1, "baseline-first")), 95, 1.05));
        assertThrows(IllegalArgumentException.class, () -> Microbenchmark.analyze(List.of(new Pair(1, 1, "unknown")), 95, 1.05));
        assertThrows(IllegalArgumentException.class, () -> Microbenchmark.analyze(fast, 100, 1.05));
        assertThrows(IllegalArgumentException.class, () -> Microbenchmark.analyze(fast, 95, Double.NaN));
        assertThrows(IllegalArgumentException.class, () -> Microbenchmark.analyze(
                java.util.Collections.nCopies(10_001, new Pair(1, 1, "baseline-first")), 95, 1.05));
    }

    @Test void replayEnforcesTextTreeAndFileBudgetsBeforePairConversion() throws Exception {
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(" ".repeat(ReportCodec.MAXIMUM_JSON_BYTES + 1)));
        String tooDeep = "[".repeat(129) + "0" + "]".repeat(129);
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(tooDeep));
        String tooManyValues = "{\"x\":[" + "0,".repeat(65_535) + "0]}";
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(tooManyValues));

        Path oversized = Files.createTempFile("java-microbenchmark-replay", ".json");
        try {
            Files.write(oversized, new byte[ReportCodec.MAXIMUM_JSON_BYTES + 1]);
            assertThrows(IllegalArgumentException.class, () -> ReportCodec.readFile(oversized));
        } finally {
            Files.deleteIfExists(oversized);
        }
    }

    @Test void configValidatesEveryPublicRange() {
        Config.defaults().validate();
        assertThrows(IllegalArgumentException.class, () -> new Config(0, 95, 1.05, 0, 1, 1).validate());
        assertThrows(IllegalArgumentException.class, () -> new Config(10_001, 95, 1.05, 0, 1, 1).validate());
        assertThrows(IllegalArgumentException.class, () -> new Config(1, 0, 1.05, 0, 1, 1).validate());
        assertThrows(IllegalArgumentException.class, () -> new Config(1, 100, 1.05, 0, 1, 1).validate());
        assertThrows(IllegalArgumentException.class, () -> new Config(1, 95, 1.0, 0, 1, 1).validate());
        assertThrows(IllegalArgumentException.class, () -> new Config(1, 95, Double.POSITIVE_INFINITY, 0, 1, 1).validate());
        assertThrows(IllegalArgumentException.class, () -> new Config(1, 95, 1.05, 0, 0, 1).validate());
        assertThrows(IllegalArgumentException.class, () -> new Config(1, 95, 1.05, 0, 1, 0).validate());
    }

    @Test void collectorCalibratesBalancesAndReproducesSeededOrder() throws Exception {
        Report first = report(QUICK);
        Report second = report(QUICK);
        assertEquals(1, first.batchSize());
        assertEquals(first.pairs().stream().map(Pair::order).toList(), second.pairs().stream().map(Pair::order).toList());
        long base = first.pairs().stream().filter(p -> p.order().equals("baseline-first")).count();
        assertEquals(3, base);
        assertEquals(5, first.pairs().size());
        Config hugeBatch = new Config(1, 95, 1.05, Long.MIN_VALUE, 1, Long.MAX_VALUE);
        assertEquals(Long.MIN_VALUE, hugeBatch.seed());
        assertEquals(Long.MAX_VALUE, hugeBatch.maxBatch());
    }

    @Test void collectorRejectsFailuresAndInvalidClock() {
        assertThrows(IllegalArgumentException.class, () -> Microbenchmark.collect(QUICK, 7, n -> 8 * n, n -> 7 * n, ticks()));
        assertThrows(Exception.class, () -> Microbenchmark.collect(QUICK, 7, n -> { throw new Exception("callback"); }, n -> 7 * n, ticks()));
        AtomicLong backwards = new AtomicLong(10);
        Clock regressing = () -> backwards.getAndAdd(-1);
        assertThrows(IllegalArgumentException.class, () -> Microbenchmark.collect(QUICK, 7, n -> 7 * n, n -> 7 * n, regressing));
        assertThrows(IllegalArgumentException.class, () -> Microbenchmark.collect(QUICK, 7, n -> 7 * n, n -> 7 * n, () -> -1));
        Config exhausted = new Config(2, 95, 1.05, 0, 100, 1);
        assertThrows(IllegalArgumentException.class, () -> Microbenchmark.collect(exhausted, 7, n -> 7 * n, n -> 7 * n, () -> 5));
        Config overflow = new Config(1, 95, 1.05, 0, 200, 2);
        assertThrows(ArithmeticException.class, () -> Microbenchmark.collect(overflow, Long.MAX_VALUE, n -> Long.MAX_VALUE, n -> Long.MAX_VALUE, ticks()));
    }

    @Test void reportRoundTripPreservesSigned64AndRejectsTampering() throws Exception {
        Config boundary = new Config(1, 95, 1.05, Long.MIN_VALUE, 1, Long.MAX_VALUE);
        Report raw = new Report(boundary, Long.MAX_VALUE, List.of(new Pair(Long.MAX_VALUE, Long.MAX_VALUE, "baseline-first")),
                Microbenchmark.analyze(List.of(new Pair(Long.MAX_VALUE, Long.MAX_VALUE, "baseline-first")), 95, 1.05));
        String encoded = ReportCodec.toJson(raw, List.of("bench"), "r", "Java 21", "Windows", "x86_64");
        assertTrue(encoded.contains("-9223372036854775808"));
        assertTrue(encoded.contains("9223372036854775807"));
        Report decoded = ReportCodec.replay(encoded);
        assertEquals(Long.MIN_VALUE, decoded.config().seed());
        assertEquals(Long.MAX_VALUE, decoded.pairs().getFirst().baselineNs());
        assertEquals(Long.MAX_VALUE, decoded.batchSize());
        long beyondSafeInteger = 9_007_199_254_740_993L;
        Pair precisePair = new Pair(beyondSafeInteger, beyondSafeInteger, "baseline-first");
        Report preciseReport = new Report(new Config(1, 95, 1.05, 0, 1, 1), 1, List.of(precisePair), Microbenchmark.analyze(List.of(precisePair), 95, 1.05));
        String preciseJson = ReportCodec.toJson(preciseReport, List.of("bench"), "r", "Java", "Windows", "x64");
        assertTrue(preciseJson.contains("9007199254740993"));
        assertEquals(beyondSafeInteger, ReportCodec.replay(preciseJson).pairs().getFirst().baselineNs());

        Report valid = report(QUICK);
        String balanced = ReportCodec.toJson(valid, List.of("bench"), "r", "Java 21", "Windows", "x86_64");
        JsonNode node;
        try { node = JSON.readTree(balanced); } catch (Exception e) { throw new AssertionError(e); }
        com.fasterxml.jackson.databind.node.ArrayNode rows = (com.fasterxml.jackson.databind.node.ArrayNode) node.path("pairs");
        for (int i = 0; i < rows.size(); i++) ((com.fasterxml.jackson.databind.node.ObjectNode) rows.get(i)).put("order", "baseline-first");
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(node.toString()));
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(encoded.replace("9223372036854775807", "9223372036854775808")));
        String genericWorkload = encoded.replace("indexed-array-sum-4096", "dependent-link-order");
        com.fasterxml.jackson.databind.node.ObjectNode genericNode;
        try { genericNode = (com.fasterxml.jackson.databind.node.ObjectNode) JSON.readTree(genericWorkload); }
        catch (Exception e) { throw new AssertionError(e); }
        genericNode.put("profile", "native-go");
        assertEquals(decoded.pairs(), ReportCodec.replay(genericNode.toString()).pairs());
        genericNode.put("workload", "\u2003");
        assertDoesNotThrow(() -> ReportCodec.replay(genericNode.toString()));
        genericNode.put("workload", new String(Character.toChars(0x1F680)));
        assertDoesNotThrow(() -> ReportCodec.replay(genericNode.toString()));
        genericNode.put("workload", "\uFFFD");
        assertDoesNotThrow(() -> ReportCodec.replay(genericNode.toString()));
        genericNode.put("workload", "literal" + (char) 92 + "uD800");
        assertDoesNotThrow(() -> ReportCodec.replay(genericNode.toString()));
        for (String surrogate : List.of("\\" + "uD800", "\\" + "uDFFF")) {
            String escapedSurrogate = encoded.replace("indexed-array-sum-4096", surrogate);
            assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(escapedSurrogate));
        }
        genericNode.put("workload", "");
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(genericNode.toString()));
        genericNode.put("workload", " \t\n\r\f" + (char) 11);
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(genericNode.toString()));
        genericNode.put("workload", "\0");
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(genericNode.toString()));
        genericNode.put("workload", "dependent\0link");
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(genericNode.toString()));
        genericNode.putNull("workload");
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(genericNode.toString()));
        genericNode.put("workload", 1);
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(genericNode.toString()));
        JsonNode extraConfig;
        try { extraConfig = JSON.readTree(encoded); } catch (Exception e) { throw new AssertionError(e); }
        ((com.fasterxml.jackson.databind.node.ObjectNode) extraConfig.path("config")).put("extra", 1);
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(extraConfig.toString()));
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay("{\"x\":1,\"x\":2}"));
        assertThrows(IllegalArgumentException.class, () -> ReportCodec.replay(encoded + "{}"));
        JsonNode profileNode;
        try { profileNode = JSON.readTree(balanced); } catch (Exception e) { throw new AssertionError(e); }
        ((com.fasterxml.jackson.databind.node.ObjectNode) profileNode).put("profile", "portable");
        assertDoesNotThrow(() -> ReportCodec.replay(profileNode.toString()));
    }

    @Test void fixtureCopyAndAllPortableCasesConform() throws Exception {
        Path local = Path.of("testdata", "microbenchmark-pairs-v1.json");
        Path canonical = Path.of("../../../specs/fixtures/microbenchmark-pairs-v1.json");
        assertArrayEquals(Files.readAllBytes(canonical), Files.readAllBytes(local));
        JsonNode root = JSON.readTree(local.toFile());
        for (JsonNode sample : root.path("cases")) {
            List<Pair> pairs = new ArrayList<>();
            for (JsonNode duration : sample.path("candidate_ns")) pairs.add(new Pair(sample.path("baseline_ns").longValue(), duration.longValue(), "baseline-first"));
            Analysis actual = Microbenchmark.analyze(pairs, root.path("confidence_percent").intValue(), root.path("minimum_factor").doubleValue());
            assertEquals(sample.path("ratio").doubleValue(), actual.ratio(), 1e-12, sample.path("name").asText());
            assertEquals(sample.path("verdict").asText(), actual.verdict());
            JsonNode interval = sample.path("interval");
            if (interval.isNull()) assertFalse(actual.hasInterval());
            else { assertEquals(interval.get(0).doubleValue(), actual.low(), 1e-12); assertEquals(interval.get(1).doubleValue(), actual.high(), 1e-12); }
        }
    }

    @Test void nativeCliProducesAndReplaysFortyPairsAndPropagatesBrokenSink() throws Exception {
        ByteArrayOutputStream stdout = new ByteArrayOutputStream();
        var status = ArrayMicrobenchmark.run(new String[] {"--pairs", "40", "--target-ns", "1", "--max-batch", "1", "--revision", "test"}, stdout, System.err, ticks());
        assertEquals(0, status);
        Report parsed = ReportCodec.replay(stdout.toString(java.nio.charset.StandardCharsets.UTF_8));
        assertEquals(40, parsed.pairs().size());
        assertEquals(20, parsed.pairs().stream().filter(p -> p.order().equals("baseline-first")).count());
        ByteArrayOutputStream allOptions = new ByteArrayOutputStream();
        assertEquals(0, ArrayMicrobenchmark.run(new String[] {"--pairs", "2", "--confidence", "90", "--factor", "1.1", "--seed", "-9223372036854775808", "--target-ns", "1", "--max-batch", "1", "--revision", "opts"}, allOptions, System.err, ticks()));
        assertEquals(2, ReportCodec.replay(allOptions.toString(java.nio.charset.StandardCharsets.UTF_8)).pairs().size());
        ByteArrayOutputStream replayOut = new ByteArrayOutputStream();
        Path reportFile = Files.createTempFile("java-microbenchmark", ".json");
        try {
            Files.writeString(reportFile, stdout.toString(java.nio.charset.StandardCharsets.UTF_8));
            assertEquals(0, ArrayMicrobenchmark.run(new String[] {"--replay", reportFile.toString()}, replayOut, System.err, ticks()));
            assertTrue(replayOut.toString(java.nio.charset.StandardCharsets.UTF_8).contains("replay valid: 40 pairs"));
        } finally { Files.deleteIfExists(reportFile); }
        assertEquals(0, ArrayMicrobenchmark.run(new String[] {"--help"}, new ByteArrayOutputStream(), System.err, ticks()));
        assertEquals(1, ArrayMicrobenchmark.run(new String[] {"--unknown", "x"}, new ByteArrayOutputStream(), System.err, ticks()));
        assertEquals(1, ArrayMicrobenchmark.run(new String[] {"--seed"}, new ByteArrayOutputStream(), System.err, ticks()));
        OutputStream broken = new OutputStream() { @Override public void write(int b) throws java.io.IOException { throw new java.io.IOException("broken"); } };
        assertEquals(1, ArrayMicrobenchmark.run(new String[] {"--pairs", "1", "--target-ns", "1", "--max-batch", "1"}, broken, System.err, ticks()));
        PrintStream brokenPrint = new PrintStream(broken);
        assertEquals(1, ArrayMicrobenchmark.run(new String[] {"--pairs", "1", "--target-ns", "1", "--max-batch", "1"}, brokenPrint, System.err, ticks()));
    }
}
