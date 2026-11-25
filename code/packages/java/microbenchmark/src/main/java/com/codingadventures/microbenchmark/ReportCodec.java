package com.codingadventures.microbenchmark;

import com.fasterxml.jackson.core.JsonFactory;
import com.fasterxml.jackson.core.StreamReadFeature;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

/** Exact signed-64-bit JSON encoder and defensive report replay validator. */
public final class ReportCodec {
    private ReportCodec() {}
    private static final ObjectMapper JSON = new ObjectMapper(JsonFactory.builder()
            .enable(StreamReadFeature.STRICT_DUPLICATE_DETECTION).build())
            .enable(DeserializationFeature.FAIL_ON_TRAILING_TOKENS);
    private static final Set<String> TOP_KEYS = Set.of("workload", "invocation", "revision", "runtime", "os", "arch", "config", "batch_size", "pairs", "analysis");
    private static final Set<String> CONFIG_KEYS = Set.of("pairs", "confidence_percent", "minimum_factor", "seed", "target_ns", "max_batch");

    static Map<String, Object> toDocument(Report report, List<String> invocation,
                                          String revision, String runtime, String os, String arch) {
        Map<String, Object> root = new LinkedHashMap<>();
        root.put("workload", "indexed-array-sum-4096");
        root.put("invocation", List.copyOf(invocation));
        root.put("revision", revision); root.put("runtime", runtime); root.put("os", os); root.put("arch", arch);
        root.put("config", Map.of("pairs", report.config().pairs(), "confidence_percent", report.config().confidencePercent(),
                "minimum_factor", report.config().minimumFactor(), "seed", report.config().seed(),
                "target_ns", report.config().targetNs(), "max_batch", report.config().maxBatch()));
        root.put("batch_size", report.batchSize());
        List<Map<String, Object>> rows = new ArrayList<>();
        for (Pair pair : report.pairs()) rows.add(Map.of("baseline_ns", pair.baselineNs(), "candidate_ns", pair.candidateNs(), "order", pair.order()));
        root.put("pairs", rows);
        Analysis a = report.analysis();
        Map<String, Object> analysis = new LinkedHashMap<>();
        analysis.put("ratio", a.ratio());
        analysis.put("interval", a.hasInterval() ? List.of(a.low(), a.high()) : null);
        analysis.put("verdict", a.verdict());
        root.put("analysis", analysis);
        return root;
    }

    public static String toJson(Report report, List<String> invocation, String revision,
                                String runtime, String os, String arch) {
        try { return JSON.writerWithDefaultPrettyPrinter().writeValueAsString(toDocument(report, invocation, revision, runtime, os, arch)) + "\n"; }
        catch (IOException e) { throw new IllegalArgumentException("report could not be encoded", e); }
    }

    /** Validates metadata, exact integer ranges, schedule balance, and recomputed analysis. */
    public static Report replay(String text) {
        try {
            JsonNode root = JSON.readTree(text);
            require(root != null && root.isObject(), "report must be an object");
            Set<String> keys = fieldNames(root);
            Set<String> expected = new LinkedHashSet<>(TOP_KEYS);
            if (keys.contains("profile")) expected.add("profile");
            require(keys.equals(expected), "report fields do not match the portable schema");
            String workload = string(root, "workload");
            require(isValidWorkload(workload), "workload must contain a non-ASCII-whitespace character and no NUL");
            require(root.path("invocation").isArray(), "invocation must be an array");
            for (JsonNode item : root.path("invocation")) require(item.isTextual(), "invocation entries must be strings");
            for (String name : List.of("revision", "runtime", "os", "arch")) string(root, name);
            if (keys.contains("profile")) string(root, "profile");

            JsonNode cfg = root.path("config");
            require(cfg.isObject() && fieldNames(cfg).equals(CONFIG_KEYS), "invalid config fields");
            Config config = new Config(integer(cfg, "pairs", 1, 10_000), integer(cfg, "confidence_percent", 1, 99),
                    number(cfg, "minimum_factor"), integer64(cfg, "seed", Long.MIN_VALUE, Long.MAX_VALUE),
                    integer64(cfg, "target_ns", 1, Long.MAX_VALUE), integer64(cfg, "max_batch", 1, Long.MAX_VALUE));
            config.validate();
            long batchSize = integer64(root, "batch_size", 1, config.maxBatch());
            JsonNode rows = root.path("pairs");
            require(rows.isArray() && rows.size() == config.pairs(), "pair count does not match config");
            List<Pair> pairs = new ArrayList<>(rows.size());
            int baseFirst = 0;
            for (JsonNode row : rows) {
                require(row.isObject() && fieldNames(row).equals(Set.of("baseline_ns", "candidate_ns", "order")), "invalid pair fields");
                String order = string(row, "order");
                if ("baseline-first".equals(order)) baseFirst++;
                pairs.add(new Pair(integer64(row, "baseline_ns", 1, Long.MAX_VALUE), integer64(row, "candidate_ns", 1, Long.MAX_VALUE), order));
            }
            require(baseFirst == (config.pairs() + 1) / 2, "pair order schedule is unbalanced");
            Analysis expectedAnalysis = Microbenchmark.analyze(pairs, config.confidencePercent(), config.minimumFactor());
            validateAnalysis(root.path("analysis"), expectedAnalysis);
            return new Report(config, batchSize, pairs, expectedAnalysis);
        } catch (IOException e) { throw new IllegalArgumentException("invalid JSON report", e); }
    }

    private static void validateAnalysis(JsonNode node, Analysis want) {
        require(node.isObject() && fieldNames(node).equals(Set.of("ratio", "interval", "verdict")), "invalid analysis fields");
        double ratio = number(node, "ratio");
        require(Double.doubleToLongBits(ratio) == Double.doubleToLongBits(want.ratio()), "analysis ratio does not match raw pairs");
        require(want.verdict().equals(string(node, "verdict")), "analysis verdict does not match raw pairs");
        JsonNode interval = node.path("interval");
        if (!want.hasInterval()) require(interval.isNull(), "analysis interval should be null");
        else require(interval.isArray() && interval.size() == 2 && interval.get(0).isNumber() && interval.get(1).isNumber()
                && Double.doubleToLongBits(interval.get(0).doubleValue()) == Double.doubleToLongBits(want.low())
                && Double.doubleToLongBits(interval.get(1).doubleValue()) == Double.doubleToLongBits(want.high()), "analysis interval does not match raw pairs");
    }
    private static Set<String> fieldNames(JsonNode node) { Set<String> names = new LinkedHashSet<>(); node.fieldNames().forEachRemaining(names::add); return names; }
    private static boolean isValidWorkload(String workload) {
        if (workload.indexOf('\0') >= 0) return false;
        boolean hasNonWhitespace = false;
        for (int index = 0; index < workload.length(); index++) {
            char character = workload.charAt(index);
            if (Character.isHighSurrogate(character)) {
                if (index + 1 >= workload.length() || !Character.isLowSurrogate(workload.charAt(index + 1))) return false;
                index++;
                hasNonWhitespace = true;
            } else if (Character.isLowSurrogate(character)) {
                return false;
            } else if (character != ' ' && (character < '\t' || character > '\r')) {
                hasNonWhitespace = true;
            }
        }
        return hasNonWhitespace;
    }
    private static String string(JsonNode node, String key) { JsonNode value = node.path(key); require(value.isTextual(), key + " must be a string"); return value.textValue(); }
    private static long integer64(JsonNode node, String key, long min, long max) {
        JsonNode value = node.path(key);
        require(value.isIntegralNumber() && value.canConvertToLong(), key + " must be an exact signed 64-bit integer");
        long result = value.longValue(); require(result >= min && result <= max, key + " is outside its allowed range"); return result;
    }
    private static int integer(JsonNode node, String key, int min, int max) { long n = integer64(node, key, min, max); return (int)n; }
    private static double number(JsonNode node, String key) { JsonNode value = node.path(key); require(value.isNumber() && Double.isFinite(value.doubleValue()), key + " must be a finite number"); return value.doubleValue(); }
    private static void require(boolean condition, String message) { if (!condition) throw new IllegalArgumentException(message); }
}
