package com.codingadventures.microbenchmark;

import com.fasterxml.jackson.core.JsonFactory;
import com.fasterxml.jackson.core.StreamReadConstraints;
import com.fasterxml.jackson.core.StreamReadFeature;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.nio.CharBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

/** Exact signed-64-bit JSON encoder and defensive report replay validator. */
public final class ReportCodec {
    private ReportCodec() {}
    static final int MAXIMUM_JSON_BYTES = 4 * 1024 * 1024;
    private static final int MAXIMUM_JSON_NODES = 65_536;
    private static final ObjectMapper JSON = new ObjectMapper(JsonFactory.builder()
            .streamReadConstraints(StreamReadConstraints.builder()
                    .maxNestingDepth(128).maxStringLength(MAXIMUM_JSON_BYTES).build())
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
            checkTextSize(text);
            JsonNode root = JSON.readTree(text);
            require(root != null && root.isObject(), "report must be an object");
            validateNodeBudget(root);
            Set<String> keys = fieldNames(root);
            Set<String> expected = new LinkedHashSet<>(TOP_KEYS);
            if (keys.contains("profile")) expected.add("profile");
            require(keys.equals(expected), "report fields do not match the portable schema");
            String workload = metadataString(root, "workload");
            require(isValidWorkload(workload), "workload must contain a non-ASCII-whitespace character and no NUL");
            require(root.path("invocation").isArray(), "invocation must be an array");
            for (JsonNode item : root.path("invocation")) {
                require(item.isTextual(), "invocation entries must be strings");
                require(!item.textValue().contains("\0"), "invocation entries must not contain NUL");
            }
            for (String name : List.of("revision", "runtime", "os", "arch")) metadataString(root, name);
            if (keys.contains("profile")) metadataString(root, "profile");

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

    /** Reads at most the replay byte budget from a caller-selected report path. */
    static String readFile(Path path) throws IOException {
        try (InputStream input = Files.newInputStream(path)) {
            byte[] bytes = input.readNBytes(MAXIMUM_JSON_BYTES + 1);
            require(bytes.length <= MAXIMUM_JSON_BYTES, "report exceeds the 4 MiB JSON limit");
            try {
                return StandardCharsets.UTF_8.newDecoder()
                        .onMalformedInput(CodingErrorAction.REPORT)
                        .onUnmappableCharacter(CodingErrorAction.REPORT)
                        .decode(ByteBuffer.wrap(bytes)).toString();
            } catch (CharacterCodingException e) {
                throw new IllegalArgumentException("report must be valid UTF-8", e);
            }
        }
    }

    private static void checkTextSize(String text) {
        require(text != null, "report JSON must not be null");
        if (text.length() > MAXIMUM_JSON_BYTES) throw new IllegalArgumentException("report exceeds the 4 MiB JSON limit");
        try {
            ByteBuffer encoded = StandardCharsets.UTF_8.newEncoder()
                    .onMalformedInput(CodingErrorAction.REPORT)
                    .onUnmappableCharacter(CodingErrorAction.REPORT)
                    .encode(CharBuffer.wrap(text));
            require(encoded.remaining() <= MAXIMUM_JSON_BYTES, "report exceeds the 4 MiB JSON limit");
        } catch (CharacterCodingException e) {
            throw new IllegalArgumentException("report JSON must be valid Unicode", e);
        }
    }

    private static void validateNodeBudget(JsonNode root) {
        List<JsonNode> pending = new ArrayList<>();
        pending.add(root);
        int nodes = 0;
        while (!pending.isEmpty()) {
            JsonNode node = pending.remove(pending.size() - 1);
            if (++nodes > MAXIMUM_JSON_NODES) throw new IllegalArgumentException("report exceeds the JSON value limit");
            var children = node.elements();
            while (children.hasNext()) {
                if (nodes + pending.size() >= MAXIMUM_JSON_NODES)
                    throw new IllegalArgumentException("report exceeds the JSON value limit");
                pending.add(children.next());
            }
        }
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
    private static String metadataString(JsonNode node, String key) {
        String value = string(node, key);
        require(!value.isEmpty(), key + " must not be empty");
        require(!value.contains("\0"), key + " must not contain NUL");
        return value;
    }
    private static long integer64(JsonNode node, String key, long min, long max) {
        JsonNode value = node.path(key);
        require(value.isIntegralNumber() && value.canConvertToLong(), key + " must be an exact signed 64-bit integer");
        long result = value.longValue(); require(result >= min && result <= max, key + " is outside its allowed range"); return result;
    }
    private static int integer(JsonNode node, String key, int min, int max) { long n = integer64(node, key, min, max); return (int)n; }
    private static double number(JsonNode node, String key) { JsonNode value = node.path(key); require(value.isNumber() && Double.isFinite(value.doubleValue()), key + " must be a finite number"); return value.doubleValue(); }
    private static void require(boolean condition, String message) { if (!condition) throw new IllegalArgumentException(message); }
}
