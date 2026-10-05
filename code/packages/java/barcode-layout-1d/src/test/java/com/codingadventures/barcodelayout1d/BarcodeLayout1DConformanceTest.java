package com.codingadventures.barcodelayout1d;

import com.codingadventures.paintinstructions.PaintInstruction;
import com.codingadventures.paintinstructions.PaintScene;
import com.fasterxml.jackson.core.JsonParser;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestFactory;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.stream.Stream;

import static org.junit.jupiter.api.Assertions.*;

/** The checked-in neutral corpus is executed, not transcribed, in the Java lane. */
class BarcodeLayout1DConformanceTest {
    private static final int MAX_BYTES = 131_072;
    private static final int MAX_DEPTH = 8;
    private static final String CORPUS_SHA =
            "be95aa0381041ef3bd729b36bb4292f7a20692e139b53adca97af4e157cb7388";
    private static final Path CORPUS = Path.of("..", "..", "..", "..", "code", "specs",
            "fixtures", "barcode-layout-1d-v1", "cases.json");
    private static final ObjectMapper JSON = new ObjectMapper();

    static {
        JSON.getFactory().enable(JsonParser.Feature.STRICT_DUPLICATE_DETECTION);
    }

    private static String sha256(byte[] bytes) {
        try {
            return java.util.HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
        } catch (NoSuchAlgorithmException error) {
            throw new AssertionError(error);
        }
    }

    /** A physical preflight makes depth and size fail before JSON tree allocation. */
    private static JsonNode load(byte[] bytes) {
        if (bytes.length > MAX_BYTES) throw new IllegalArgumentException("fixture-size-limit");
        int depth = 0;
        boolean quoted = false;
        boolean escaped = false;
        for (byte value : bytes) {
            char ch = (char) (value & 0xff);
            if (quoted) {
                if (escaped) escaped = false;
                else if (ch == '\\') escaped = true;
                else if (ch == '"') quoted = false;
            } else if (ch == '"') quoted = true;
            else if (ch == '[' || ch == '{') {
                if (++depth > MAX_DEPTH) throw new IllegalArgumentException("fixture-depth-limit");
            } else if (ch == ']' || ch == '}') depth--;
        }
        try {
            JsonNode document = JSON.readTree(bytes);
            if (document == null || !document.isObject() || !document.path("cases").isArray()) {
                throw new IllegalArgumentException("fixture-schema-invalid");
            }
            validateScalars(document);
            return document;
        } catch (IOException error) {
            throw new IllegalArgumentException("fixture-invalid-json", error);
        }
    }

    private static void validateScalars(JsonNode node) {
        if (node.isTextual()) {
            String text = node.textValue();
            for (int i = 0; i < text.length(); i++) {
                char ch = text.charAt(i);
                if (Character.isHighSurrogate(ch)) {
                    if (++i >= text.length() || !Character.isLowSurrogate(text.charAt(i))) {
                        throw new IllegalArgumentException("fixture-invalid-scalar");
                    }
                } else if (Character.isLowSurrogate(ch)) {
                    throw new IllegalArgumentException("fixture-invalid-scalar");
                }
            }
        } else if (node.isContainerNode()) {
            node.elements().forEachRemaining(BarcodeLayout1DConformanceTest::validateScalars);
            if (node.isObject()) node.fieldNames().forEachRemaining(key -> validateScalars(JSON.valueToTree(key)));
        }
    }

    private static JsonNode corpus() throws IOException {
        byte[] bytes;
        try (var input = Files.newInputStream(CORPUS)) {
            bytes = input.readNBytes(MAX_BYTES + 1);
        }
        if (bytes.length > MAX_BYTES) throw new IllegalArgumentException("fixture-size-limit");
        assertEquals(CORPUS_SHA, sha256(bytes));
        JsonNode document = load(bytes);
        assertEquals(56, document.path("cases").size());
        var ids = new HashSet<String>();
        document.path("cases").forEach(row -> assertTrue(ids.add(row.path("id").asText())));
        return document;
    }

    private static String pattern(JsonNode input) {
        if (input.has("pattern")) return input.path("pattern").asText();
        JsonNode repeat = input.path("repeat");
        int count = repeat.path("count").asInt(-1);
        String token = repeat.path("token").asText();
        String suffix = repeat.path("suffix").asText("");
        if (count < 0 || count > 65_569 || token.codePointCount(0, token.length()) > 2
                || suffix.codePointCount(0, suffix.length()) > 1) {
            throw new IllegalArgumentException("fixture-schema-invalid");
        }
        return token.repeat(count) + suffix;
    }

    private static List<BarcodeLayout1DV1.Run> runs(JsonNode input) {
        var result = new ArrayList<BarcodeLayout1DV1.Run>();
        if (input.has("runs")) {
            input.path("runs").forEach(row -> result.add(new BarcodeLayout1DV1.Run(
                    row.path("color").asText(), row.path("modules").asInt(),
                    row.path("sourceLabel").asText(), row.path("sourceIndex").asLong(),
                    row.path("role").asText())));
            return result;
        }
        JsonNode repeated = input.path("repeatRuns");
        int count = repeated.path("count").asInt(-1);
        if (count < 0 || count > 40_980) throw new IllegalArgumentException("fixture-schema-invalid");
        for (int i = 0; i < count; i++) {
            String color = repeated.path("firstColor").asText();
            if ((i & 1) == 1) color = color.equals("bar") ? "space" : "bar";
            result.add(new BarcodeLayout1DV1.Run(color, repeated.path("modules").asInt(),
                    repeated.path("sourceLabel").asText(), repeated.path("sourceIndex").asLong(),
                    repeated.path("role").asText()));
        }
        return result;
    }

    private static List<BarcodeLayout1DV1.Symbol> symbols(JsonNode input) {
        if (input.has("symbols")) {
            var result = new ArrayList<BarcodeLayout1DV1.Symbol>();
            input.path("symbols").forEach(row -> result.add(new BarcodeLayout1DV1.Symbol(
                    row.path("label").asText(), row.path("modules").asInt(),
                    row.path("sourceIndex").asLong(), row.path("role").asText())));
            return result;
        }
        if (!input.has("repeatSymbols")) return null;
        JsonNode repeated = input.path("repeatSymbols");
        int count = repeated.path("count").asInt(-1);
        if (count < 0 || count > 40_980) throw new IllegalArgumentException("fixture-schema-invalid");
        var result = new ArrayList<BarcodeLayout1DV1.Symbol>();
        for (int i = 0; i < count; i++) result.add(new BarcodeLayout1DV1.Symbol(
                repeated.path("label").asText(), repeated.path("modules").asInt(),
                i, repeated.path("role").asText()));
        return result;
    }

    private static int integer(JsonNode row, String key, int fallback) {
        return row.has(key) ? row.path(key).asInt() : fallback;
    }

    private static String string(JsonNode row, String key, String fallback) {
        return row.has(key) ? row.path(key).asText() : fallback;
    }

    private static Object execute(JsonNode row) {
        JsonNode input = row.path("input");
        return switch (row.path("operation").asText()) {
            case "expand-binary" -> BarcodeLayout1DV1.expandBinary(pattern(input),
                    input.path("sourceLabel").asText(), input.path("sourceIndex").asLong(),
                    input.path("role").asText());
            case "expand-width" -> BarcodeLayout1DV1.expandWidth(pattern(input),
                    string(input, "narrowMarker", "N"), string(input, "wideMarker", "W"),
                    integer(input, "narrowModules", 1), integer(input, "wideModules", 3),
                    string(input, "startingColor", "bar"), input.path("sourceLabel").asText(),
                    input.path("sourceIndex").asLong(), input.path("role").asText());
            case "compute-layout" -> BarcodeLayout1DV1.computeLayout(runs(input),
                    input.path("quietZoneModules").asInt(), symbols(input));
            case "project-scene" -> {
                JsonNode render = input.path("renderConfig");
                Map<String, String> metadata = new HashMap<>();
                input.path("metadata").fields().forEachRemaining(entry ->
                        metadata.put(entry.getKey(), entry.getValue().asText()));
                var options = new BarcodeLayout1DV1.SceneOptions(
                        integer(render, "moduleWidth", 4), integer(render, "barHeight", 120),
                        string(render, "foreground", "#000000"),
                        string(render, "background", "#ffffff"),
                        string(input, "label", "1D barcode"), metadata,
                        input.has("humanReadableText") ? input.path("humanReadableText").asText() : null,
                        render.path("includeHumanReadableText").asBoolean(false), symbols(input));
                yield BarcodeLayout1DV1.projectScene(runs(input),
                        input.path("quietZoneModules").asInt(), options);
            }
            default -> throw new AssertionError(row.path("operation").asText());
        };
    }

    private static Map<String, Object> runMap(BarcodeLayout1DV1.Run run) {
        var map = new TreeMap<String, Object>();
        map.put("color", run.color());
        map.put("modules", run.modules());
        map.put("role", run.role());
        map.put("sourceIndex", Math.toIntExact(run.sourceIndex()));
        map.put("sourceLabel", run.sourceLabel());
        return map;
    }

    private static Object project(Object actual) {
        if (actual instanceof List<?> list) {
            return list.stream().map(item -> runMap((BarcodeLayout1DV1.Run) item)).toList();
        }
        if (actual instanceof BarcodeLayout1DV1.Layout layout) {
            return Map.of("leftQuietZoneModules", layout.leftQuietZoneModules(),
                    "rightQuietZoneModules", layout.rightQuietZoneModules(),
                    "contentModules", layout.contentModules(), "totalModules", layout.totalModules(),
                    "symbolLayouts", layout.symbolLayouts().stream().map(symbol -> Map.of(
                            "label", symbol.label(), "startModule", symbol.startModule(),
                            "endModule", symbol.endModule(), "sourceIndex", symbol.sourceIndex(),
                            "role", symbol.role())).toList());
        }
        PaintScene scene = (PaintScene) actual;
        var rectangles = scene.instructions.stream().map(instruction -> {
            var rect = (PaintInstruction.PaintRect) instruction;
            return Map.of("x", rect.x, "y", rect.y, "width", rect.width,
                    "height", rect.height, "fill", rect.fill, "metadata", rect.metadata);
        }).toList();
        return Map.of("width", scene.width, "height", scene.height,
                "background", scene.background, "rectangles", rectangles, "metadata", scene.metadata);
    }

    @TestFactory
    Stream<DynamicTest> allNeutralCases() throws IOException {
        var tests = new ArrayList<DynamicTest>();
        for (JsonNode row : corpus().path("cases")) {
            tests.add(DynamicTest.dynamicTest(row.path("id").asText(), () -> {
                JsonNode expected = row.path("expected");
                if (expected.has("error")) {
                    var error = assertThrows(BarcodeLayout1DV1.V1Error.class, () -> execute(row));
                    assertEquals(expected.path("error").asText(), error.errorId());
                    return;
                }
                Object actual = project(execute(row));
                if (expected.has("runDigest")) {
                    JsonNode digest = expected.path("runDigest");
                    var projected = (List<?>) actual;
                    assertEquals(digest.path("runCount").asInt(), projected.size());
                    assertEquals(digest.path("contentModules").asInt(), projected.stream()
                            .mapToInt(item -> ((Number) ((Map<?, ?>) item).get("modules")).intValue()).sum());
                    assertEquals(digest.path("firstRun"), JSON.valueToTree(projected.get(0)));
                    assertEquals(digest.path("lastRun"), JSON.valueToTree(projected.get(projected.size() - 1)));
                    assertEquals(digest.path("runsSha256").asText(), sha256(JSON.writeValueAsBytes(projected)));
                    return;
                }
                String key = expected.has("runs") ? "runs" : expected.has("layout") ? "layout" : "scene";
                assertEquals(expected.path(key), JSON.valueToTree(actual));
            }));
        }
        return tests.stream();
    }

    @Test
    void hostileFixturesFailBeforeDispatch() {
        assertThrows(IllegalArgumentException.class, () -> load(new byte[MAX_BYTES + 1]));
        assertThrows(IllegalArgumentException.class,
                () -> load("[[[[[[[[[0]]]]]]]]]".getBytes(StandardCharsets.UTF_8)));
        assertThrows(IllegalArgumentException.class,
                () -> load("{\"cases\":[],\"cases\":[]}".getBytes(StandardCharsets.UTF_8)));
        assertThrows(IllegalArgumentException.class,
                () -> load("{\"cases\":[],\"bad\":\"\\uD800\"}".getBytes(StandardCharsets.UTF_8)));
        assertThrows(IllegalArgumentException.class, () -> load(new byte[]{(byte) 0xff}));
        assertThrows(IllegalArgumentException.class,
                () -> pattern(JSON.valueToTree(Map.of("repeat", Map.of("token", "1", "count", 65_570)))));
    }
}
