package com.codingadventures.itf;

import com.fasterxml.jackson.core.JsonParser;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.TestFactory;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;
import java.util.Objects;
import java.util.stream.Collectors;
import java.util.stream.Stream;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Standard conformance test suite reading the canonical barcode-symbologies-v1 cases.json fixture.
 */
class ItfConformanceTest {
    private static final int MAX_FIXTURE_BYTES = 131_072;
    private static final ObjectMapper JSON = new ObjectMapper();

    static {
        JSON.getFactory().enable(JsonParser.Feature.STRICT_DUPLICATE_DETECTION);
    }

    private static String sha256(String value) {
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            byte[] hash = digest.digest(value.getBytes(StandardCharsets.UTF_8));
            return HexFormat.of().formatHex(hash);
        } catch (NoSuchAlgorithmException e) {
            throw new AssertionError(e);
        }
    }

    private static Path locateCasesFixture() {
        Path path = Path.of("code", "specs", "fixtures", "barcode-symbologies-v1", "cases.json");
        Path current = Path.of("").toAbsolutePath();
        while (current != null) {
            Path candidate = current.resolve(path);
            if (Files.isRegularFile(candidate)) {
                return candidate;
            }
            candidate = current.resolve("specs/fixtures/barcode-symbologies-v1/cases.json");
            if (Files.isRegularFile(candidate)) {
                return candidate;
            }
            current = current.getParent();
        }
        throw new IllegalStateException("Could not find barcode-symbologies-v1 cases.json");
    }

    private static JsonNode loadCasesJson() throws IOException {
        Path fixturePath = locateCasesFixture();
        byte[] bytes;
        try (InputStream in = Files.newInputStream(fixturePath)) {
            bytes = in.readNBytes(MAX_FIXTURE_BYTES + 1);
        }
        if (bytes.length > MAX_FIXTURE_BYTES) {
            throw new IllegalStateException("Fixture exceeds max bounded byte limit: " + bytes.length);
        }
        return JSON.readTree(bytes);
    }

    private static String materializeInput(JsonNode inputNode) {
        if (inputNode.has("text")) {
            return inputNode.get("text").asText();
        }
        JsonNode repeat = inputNode.get("repeat");
        String text = repeat.get("text").asText();
        int count = repeat.get("count").asInt();
        return text.repeat(count);
    }

    private static List<Integer> computeRunLengths(String modules) {
        List<Integer> runs = new ArrayList<>();
        if (modules.isEmpty()) {
            return runs;
        }
        char current = modules.charAt(0);
        int count = 1;
        for (int i = 1; i < modules.length(); i++) {
            char ch = modules.charAt(i);
            if (ch == current) {
                count++;
            } else {
                runs.add(count);
                current = ch;
                count = 1;
            }
        }
        runs.add(count);
        return runs;
    }

    @TestFactory
    Stream<DynamicTest> testCanonicalItfCases() throws IOException {
        JsonNode root = loadCasesJson();
        JsonNode casesNode = root.get("cases");
        List<DynamicTest> tests = new ArrayList<>();

        for (JsonNode testCase : casesNode) {
            if (!Objects.equals(testCase.path("symbology").asText(), "itf")) {
                continue;
            }

            String id = testCase.get("id").asText();
            tests.add(DynamicTest.dynamicTest(id, () -> {
                String input = materializeInput(testCase.get("input"));
                JsonNode expected = testCase.get("expected");

                if (expected.has("error")) {
                    String expectedError = expected.get("error").asText();
                    InvalidItfInputException ex = assertThrows(
                            InvalidItfInputException.class,
                            () -> Itf.normalizeItf(input)
                    );
                    assertEquals(expectedError, ex.getErrorId());
                    return;
                }

                String normalized = Itf.normalizeItf(input);
                List<EncodedPair> encodedPairs = Itf.encodeItf(input);

                StringBuilder modulesBuilder = new StringBuilder(ItfConstants.START_PATTERN);
                for (EncodedPair pair : encodedPairs) {
                    modulesBuilder.append(pair.binaryPattern());
                }
                modulesBuilder.append(ItfConstants.STOP_PATTERN);
                String modules = modulesBuilder.toString();

                List<Integer> runs = computeRunLengths(modules);

                if (expected.has("normalized")) {
                    assertEquals(expected.get("normalized").asText(), normalized);
                    assertEquals(expected.get("modules").asText(), modules);

                    List<Integer> expectedRuns = new ArrayList<>();
                    for (JsonNode runNode : expected.get("run_lengths")) {
                        expectedRuns.add(runNode.asInt());
                    }
                    assertEquals(expectedRuns, runs);
                } else {
                    assertEquals(expected.get("normalized_sha256").asText(), sha256(normalized));
                    assertEquals(expected.get("run_count").asInt(), runs.size());

                    String compactRuns = "[" + runs.stream().map(Object::toString).collect(Collectors.joining(",")) + "]";
                    assertEquals(expected.get("run_lengths_sha256").asText(), sha256(compactRuns));
                }

                assertEquals(expected.get("module_count").asInt(), modules.length());
                assertEquals(expected.get("module_sha256").asText(), sha256(modules));
            }));
        }

        assertTrue(tests.size() >= 10, "Expected at least 10 ITF conformance cases");
        return tests.stream();
    }
}
