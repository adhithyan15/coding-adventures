package com.codingadventures.canonicalcbor;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

/** Executes every language-neutral CBR01 vector against the Java lane. */
class PortableConformanceTest {
    private static final ObjectMapper JSON = new ObjectMapper();
    private static final HexFormat HEX = HexFormat.of();

    @Test
    void exactPortableBytesMatchSharedOracle() throws Exception {
        JsonNode root = JSON.readTree(Files.readString(findFixture()));
        assertEquals(1, root.path("schema_version").asInt());
        assertEquals("rfc8949-section-4.2.3-length-first", root.path("profile").asText());
        assertEquals(CanonicalCbor.MAX_NESTING_DEPTH,
                root.path("limits").path("max_nesting_depth").asInt());
        assertEquals(CanonicalCbor.MAX_ENCODED_BYTES,
                root.path("limits").path("max_encoded_bytes").asInt());
        assertEquals(55, root.path("cases").size());

        for (JsonNode testCase : root.path("cases")) {
            String id = testCase.path("id").asText();
            String operation = testCase.path("operation").asText();
            String input = testCase.path("input").asText();
            String expected = testCase.path("expected").asText();
            switch (operation) {
                case "round-trip" -> {
                    CborValue value = CanonicalCbor.decode(fromHex(input));
                    assertArrayEquals(fromHex(expected), CanonicalCbor.encodeChecked(value), id);
                }
                case "decode-error" -> {
                    byte[] wire = input.startsWith("nested-array-wire:")
                            ? nestedArrayWire(Integer.parseInt(input.substring(input.lastIndexOf(':') + 1)))
                            : fromHex(input);
                    assertError(expected, () -> CanonicalCbor.decode(wire), id);
                }
                case "encode-map" -> assertArrayEquals(
                        fromHex(expected), CanonicalCbor.encodeChecked(mapValue(input)), id);
                case "generated-round-trip" -> assertArrayEquals(
                        generatedWire(expected), CanonicalCbor.encodeChecked(generatedValue(input)), id);
                case "encode-error" -> {
                    CborValue value = input.equals("duplicate-map-key")
                            ? mapValue("6161=00;6161=01") : generatedValue(input);
                    assertError(expected, () -> CanonicalCbor.encodeChecked(value), id);
                    ByteArrayOutputStream destination = new ByteArrayOutputStream();
                    destination.write(0xaa);
                    assertError(expected, () -> CanonicalCbor.encodeIntoChecked(value, destination), id);
                    assertArrayEquals(new byte[]{(byte) 0xaa}, destination.toByteArray(), id);
                }
                default -> throw new AssertionError(id + ": unknown operation");
            }
        }
    }

    @Test
    void unsignedMaximumUsesAllEightArgumentBytes() throws Exception {
        CborValue value = new CborValue.Unsigned(-1L);
        assertArrayEquals(fromHex("1bffffffffffffffff"), CanonicalCbor.encodeChecked(value));
        assertEquals(value, CanonicalCbor.decode(fromHex("1bffffffffffffffff")));
    }

    @Test
    void errorsNeverReflectPayloadBytes() {
        CborException error = assertThrows(CborException.class,
                () -> CanonicalCbor.decode(fromHex("63e298")));
        assertEquals("length-too-large", error.id());
        assertTrue(error.getMessage().startsWith("canonical-cbor:"));
        assertTrue(!error.getMessage().contains("e298"));
    }

    @Test
    void publicValuesDefendBytesAndCheckedAppendPublishesAtomically() throws Exception {
        byte[] source = new byte[]{1, 2, 3};
        CborValue.Bytes value = new CborValue.Bytes(source);
        source[0] = 9;
        assertArrayEquals(new byte[]{1, 2, 3}, value.value());
        assertEquals(new CborValue.Bytes(new byte[]{1, 2, 3}), value);
        assertEquals(new CborValue.Bytes(new byte[]{1, 2, 3}).hashCode(), value.hashCode());
        assertEquals("Bytes[length=3]", value.toString());

        ByteArrayOutputStream destination = new ByteArrayOutputStream();
        destination.write(0xaa);
        CanonicalCbor.encodeIntoChecked(new CborValue.Unsigned(24), destination);
        assertArrayEquals(fromHex("aa1818"), destination.toByteArray());
        assertThrows(IllegalArgumentException.class, () -> new CborException("unknown-id"));
    }

    private static void assertError(String id, ThrowingAction action, String caseId) {
        CborException error = assertThrows(CborException.class, action::run, caseId);
        assertEquals(id, error.id(), caseId);
        assertTrue(error.getMessage().startsWith("canonical-cbor:"), caseId);
    }

    private static CborValue mapValue(String specification) throws CborException {
        List<CborValue.MapEntry> entries = new ArrayList<>();
        for (String fragment : specification.split(";")) {
            String[] pair = fragment.split("=", 2);
            entries.add(new CborValue.MapEntry(
                    CanonicalCbor.decode(fromHex(pair[0])),
                    CanonicalCbor.decode(fromHex(pair[1]))));
        }
        return new CborValue.Map(entries);
    }

    private static CborValue generatedValue(String specification) {
        if (specification.startsWith("nested-array:")) {
            int depth = Integer.parseInt(specification.substring(specification.lastIndexOf(':') + 1));
            CborValue value = CborValue.Null.INSTANCE;
            for (int i = 0; i < depth; i++) {
                value = new CborValue.Array(List.of(value));
            }
            return value;
        }
        String[] parts = specification.split(":");
        byte[] bytes = new byte[Integer.parseInt(parts[1])];
        java.util.Arrays.fill(bytes, fromHex(parts[2])[0]);
        return new CborValue.Bytes(bytes);
    }

    private static byte[] generatedWire(String specification) {
        if (specification.startsWith("wire:nested-array:")) {
            return nestedArrayWire(Integer.parseInt(specification.substring(specification.lastIndexOf(':') + 1)));
        }
        String[] parts = specification.split(":");
        int length = Integer.parseInt(parts[2]);
        ByteArrayOutputStream output = new ByteArrayOutputStream(length + 9);
        if (length <= 23) {
            output.write(0x40 | length);
        } else if (length <= 0xff) {
            output.write(0x58);
            output.write(length);
        } else if (length <= 0xffff) {
            output.write(0x59);
            output.write(length >>> 8);
            output.write(length);
        } else {
            output.write(0x5a);
            output.write(length >>> 24);
            output.write(length >>> 16);
            output.write(length >>> 8);
            output.write(length);
        }
        byte repeated = fromHex(parts[3])[0];
        for (int i = 0; i < length; i++) {
            output.write(repeated);
        }
        return output.toByteArray();
    }

    private static byte[] nestedArrayWire(int depth) {
        byte[] wire = new byte[depth + 1];
        java.util.Arrays.fill(wire, 0, depth, (byte) 0x81);
        wire[depth] = (byte) 0xf6;
        return wire;
    }

    private static byte[] fromHex(String value) {
        return HEX.parseHex(value);
    }

    private static Path findFixture() throws IOException {
        Path directory = Path.of("").toAbsolutePath();
        while (directory != null) {
            Path candidate = directory.resolve("code/specs/fixtures/canonical-cbor-v1/cases.json");
            if (Files.isRegularFile(candidate)) {
                return candidate;
            }
            directory = directory.getParent();
        }
        throw new IOException("canonical-cbor-v1 fixture not found");
    }

    @FunctionalInterface
    private interface ThrowingAction {
        void run() throws Exception;
    }
}
