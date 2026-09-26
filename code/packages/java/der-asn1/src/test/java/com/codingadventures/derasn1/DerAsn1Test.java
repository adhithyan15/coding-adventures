package com.codingadventures.derasn1;

import com.codingadventures.dertlv.DerTlv;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.math.BigInteger;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestFactory;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

final class DerAsn1Test {
    private static final ObjectMapper MAPPER = new ObjectMapper();
    private static final JsonNode FIXTURE = readFixture("der-asn1-v1");
    private static final JsonNode UPSTREAM = readFixture("der-tlv-v1");

    @TestFactory
    Stream<DynamicTest> portableConformance() {
        assertEquals(122, FIXTURE.path("cases").size());
        assertEquals(22, FIXTURE.path("error_ids").size());
        var tests = new ArrayList<DynamicTest>();
        for (JsonNode testCase : FIXTURE.path("cases")) {
            tests.add(DynamicTest.dynamicTest(testCase.path("id").asText(), () -> {
                Object actual = runCase(testCase);
                JsonNode normalized = MAPPER.readTree(MAPPER.writeValueAsString(actual));
                assertEquals(testCase.path("expected"), normalized);
                if (testCase.has("redacted_input_hex")) {
                    assertFalse(MAPPER.writeValueAsString(actual)
                        .contains(testCase.path("redacted_input_hex").asText()));
                }
            }));
        }
        return tests.stream();
    }

    @Test
    void nativeContractsCoverViewsLimitsUnsignedOidAndRedaction() {
        var defaults = DerAsn1.Asn1Limits.defaults();
        assertEquals(defaults, DerAsn1.Asn1Limits.defaults());
        assertEquals(defaults.hashCode(), DerAsn1.Asn1Limits.defaults().hashCode());
        assertThrows(IllegalArgumentException.class,
            () -> new DerAsn1.Asn1Limits(DerTlv.Limits.defaults(), -1, 1, 1));
        assertThrows(IllegalArgumentException.class,
            () -> new DerAsn1.Asn1Limits(DerTlv.Limits.defaults(), 1, -1, 1));
        assertThrows(IllegalArgumentException.class,
            () -> new DerAsn1.Asn1Limits(DerTlv.Limits.defaults(), 1, 1, -1));

        byte[] input = hex("060b2a81ffffffffffffffff7f");
        var decoder = new DerAsn1.Asn1Decoder();
        var element = decoder.decodeExact(input);
        var oid = DerAsn1.decodeObjectIdentifier(element);
        assertEquals(List.of(BigInteger.ONE, BigInteger.TWO,
            BigInteger.ONE.shiftLeft(64).subtract(BigInteger.ONE)), oid.arcs());
        assertTrue(oid.equalsArcs(oid.arcs()));
        assertFalse(oid.equalsArcs(List.of(BigInteger.ONE, BigInteger.TWO)));
        input[input.length - 1] = 1;
        assertEquals(1, element.value().get(element.value().remaining() - 1));

        var hostile = new DerAsn1.Asn1Decoder().decodeExact(hex("160261ff"));
        var error = assertThrows(DerAsn1.Error.class,
            () -> DerAsn1.decodeIA5String(hostile));
        assertFalse(error.toString().contains("61ff"));
        assertFalse(error.toString().contains("255"));
    }

    @Test
    void equalLimitsCanShareACursor() {
        var first = new DerAsn1.Asn1Decoder(DerAsn1.Asn1Limits.defaults());
        var root = first.decodeExact(hex("30020500"));
        var cursor = first.sequence(root);
        var second = new DerAsn1.Asn1Decoder(DerAsn1.Asn1Limits.defaults());
        assertNotNull(cursor.read(second));
        assertEquals(1, second.elementsRead());
        assertEquals(0, cursor.remaining().remaining());
    }

    private static Object runCase(JsonNode testCase) {
        if (testCase.has("der_tlv_case_id")) {
            return verifyUpstream(testCase.path("der_tlv_case_id").asText());
        }
        var limits = limits(testCase);
        var decoder = new DerAsn1.Asn1Decoder(limits);
        String operation = testCase.path("operation").asText();
        try {
            var root = decoder.decodeExact(materialize(testCase.path("input")));
            return switch (operation) {
                case "decode-exact" -> map(
                    "outcome", "value",
                    "tag", tag(root),
                    "header_hex", hex(bytes(root.header())),
                    "value_hex", hex(bytes(root.value())),
                    "encoded_hex", hex(bytes(root.encoded())),
                    "depth", root.depth(),
                    "elements_read", decoder.elementsRead());
                case "cursor-script" -> cursorResult(testCase, decoder, root);
                case "sequence", "set" -> {
                    var cursor = operation.equals("sequence")
                        ? decoder.sequence(root) : decoder.set(root);
                    yield map(
                        "outcome", "value",
                        "elements_read", decoder.elementsRead(),
                        "remaining_offset", root.value().remaining() - cursor.remaining().remaining());
                }
                case "explicit" -> {
                    var child = decoder.explicit(root, testCase.path("tag_number").asLong());
                    yield map(
                        "outcome", "value",
                        "tag", tag(child),
                        "value_hex", hex(bytes(child.value())),
                        "depth", child.depth(),
                        "elements_read", decoder.elementsRead());
                }
                default -> primitiveResult(operation, root, limits,
                    testCase.path("tag_number").asLong());
            };
        } catch (DerAsn1.Error error) {
            String scope = operation.equals("explicit")
                    && error.kind() == DerAsn1.Asn1ErrorKind.FRAMING
                ? "container-value" : "operation-input";
            return error(error, scope);
        }
    }

    private static Object verifyUpstream(String id) {
        JsonNode referenced = null;
        for (JsonNode candidate : UPSTREAM.path("cases")) {
            if (candidate.path("id").asText().equals(id)) {
                referenced = candidate;
                break;
            }
        }
        if (referenced == null) {
            throw new IllegalStateException("missing upstream case " + id);
        }
        var derLimits = derLimits(UPSTREAM.path("defaults"), referenced.path("limits"));
        var decoder = new DerAsn1.Asn1Decoder(
            new DerAsn1.Asn1Limits(derLimits, 32, 16_384, 128));
        JsonNode expected = referenced.path("expected");
        byte[] input = materialize(referenced.path("input"));
        try {
            var element = decoder.decodeExact(input);
            assertEquals("element", expected.path("outcome").asText(), id);
            assertEquals(expected.path("tag").path("class").asText(),
                element.tag().tagClass(), id);
            assertEquals(expected.path("tag").path("constructed").asBoolean(),
                element.tag().constructed(), id);
            assertEquals(expected.path("tag").path("number").asLong(),
                element.tag().number(), id);
            assertEquals(expected.path("header_len").asInt(), element.header().remaining(), id);
            assertEquals(expected.path("encoded_len").asInt(), element.encoded().remaining(), id);
            assertEquals(input.length, element.encoded().remaining(), id);
        } catch (DerAsn1.Error error) {
            assertEquals("error", expected.path("outcome").asText(), id);
            assertNotNull(error.framingKind(), id);
            assertEquals(expected.path("error_id").asText(), error.framingKind(), id);
            assertEquals(expected.path("offset").asInt(), error.offset(), id);
        }
        return Map.of("outcome", "upstream");
    }

    private static Object primitiveResult(
            String operation,
            DerAsn1.Asn1Element element,
            DerAsn1.Asn1Limits limits,
            long tagNumber) {
        return switch (operation) {
            case "decode-boolean" -> map(
                "outcome", "value",
                "boolean", DerAsn1.decodeBoolean(element),
                "elements_read", 1);
            case "decode-integer", "integer-to-u64" -> {
                var integer = DerAsn1.decodeInteger(element);
                var result = map(
                    "outcome", "value",
                    "signed_hex", hex(bytes(integer.signedBytes())),
                    "negative", integer.isNegative());
                if (operation.equals("integer-to-u64")) {
                    result.put("u64_decimal", integer.toU64().toString());
                }
                yield result;
            }
            case "decode-bit-string" -> {
                var bits = DerAsn1.decodeBitString(element);
                yield map(
                    "outcome", "value",
                    "bytes_hex", hex(bytes(bits.bytes())),
                    "unused_bits", bits.unusedBits(),
                    "bit_length", bits.bitLength());
            }
            case "decode-octet-string" -> map(
                "outcome", "value",
                "bytes_hex", hex(bytes(DerAsn1.decodeOctetString(element))));
            case "decode-implicit-octet-string" -> map(
                "outcome", "value",
                "bytes_hex", hex(bytes(DerAsn1.decodeImplicitOctetString(element, tagNumber))));
            case "decode-ia5-string" -> map(
                "outcome", "value", "text", DerAsn1.decodeIA5String(element));
            case "decode-implicit-ia5-string" -> map(
                "outcome", "value", "text", DerAsn1.decodeImplicitIA5String(element, tagNumber));
            case "decode-null" -> {
                DerAsn1.decodeNull(element);
                yield map("outcome", "value");
            }
            case "decode-object-identifier", "decode-implicit-object-identifier" -> {
                var oid = operation.equals("decode-object-identifier")
                    ? DerAsn1.decodeObjectIdentifier(element, limits)
                    : DerAsn1.decodeImplicitObjectIdentifier(element, tagNumber, limits);
                yield map(
                    "outcome", "value",
                    "bytes_hex", hex(bytes(oid.encoded())),
                    "arcs_decimal", oid.arcs().stream().map(BigInteger::toString).toList(),
                    "arc_count", oid.arcCount());
            }
            default -> throw new IllegalStateException("unsupported operation " + operation);
        };
    }

    private static Object cursorResult(
            JsonNode testCase,
            DerAsn1.Asn1Decoder decoder,
            DerAsn1.Asn1Element root) {
        var cursor = decoder.sequence(root);
        int total = cursor.remaining().remaining();
        var events = new ArrayList<Map<String, Object>>();
        for (JsonNode raw : testCase.path("actions")) {
            String action = raw.asText();
            if (action.equals("finish")) {
                try {
                    cursor.finish();
                    events.add(map("outcome", "finished"));
                } catch (DerAsn1.Error error) {
                    events.add(error(error, "container-value"));
                }
                continue;
            }
            if (!action.equals("read") && !action.equals("read-with-different-limits")
                    && !action.equals("read-nested-sequence")) {
                throw new IllegalStateException("unsupported cursor action " + action);
            }
            var active = decoder;
            if (action.equals("read-with-different-limits")) {
                var current = decoder.limits();
                active = new DerAsn1.Asn1Decoder(new DerAsn1.Asn1Limits(
                    current.der(), current.maxDepth(), current.maxTotalElements() + 1,
                    current.maxOidArcs()));
            }
            try {
                var child = cursor.read(active);
                if (action.equals("read-nested-sequence")) {
                    if (child == null) throw new IllegalStateException("nested child required");
                    var nested = decoder.sequence(child);
                    var grandchild = nested.read(decoder);
                    if (grandchild == null) throw new IllegalStateException("nested grandchild required");
                    nested.finish();
                    events.add(map("outcome", "value", "tag", tag(grandchild), "depth", grandchild.depth()));
                    continue;
                }
                events.add(child == null
                    ? map("outcome", "end")
                    : map("outcome", "value", "tag", tag(child), "depth", child.depth()));
            } catch (DerAsn1.Error error) {
                events.add(error(error, "container-value"));
            }
        }
        return map(
            "outcome", "value",
            "elements_read", decoder.elementsRead(),
            "remaining_offset", total - cursor.remaining().remaining(),
            "events", events);
    }

    private static DerAsn1.Asn1Limits limits(JsonNode testCase) {
        JsonNode defaults = FIXTURE.path("defaults");
        JsonNode overrides = testCase.path("limits");
        return new DerAsn1.Asn1Limits(
            derLimits(defaults.path("der"), overrides.path("der")),
            integerLimit(defaults, overrides, "max_depth"),
            integerLimit(defaults, overrides, "max_total_elements"),
            integerLimit(defaults, overrides, "max_oid_arcs"));
    }

    private static DerTlv.Limits derLimits(JsonNode defaults, JsonNode overrides) {
        return new DerTlv.Limits(
            longLimit(defaults, overrides, "max_input_len"),
            longLimit(defaults, overrides, "max_value_len"),
            longLimit(defaults, overrides, "max_elements"),
            longLimit(defaults, overrides, "max_tag_number"));
    }

    private static int integerLimit(JsonNode defaults, JsonNode overrides, String name) {
        JsonNode value = overrides.has(name) ? overrides.path(name) : defaults.path(name);
        return value.asInt();
    }

    private static long longLimit(JsonNode defaults, JsonNode overrides, String name) {
        JsonNode value = overrides.has(name) ? overrides.path(name) : defaults.path(name);
        return value.isTextual() ? Long.MAX_VALUE : value.asLong();
    }

    private static Map<String, Object> tag(DerAsn1.Asn1Element element) {
        return map(
            "class", element.tag().tagClass(),
            "constructed", element.tag().constructed(),
            "number", element.tag().number());
    }

    private static Map<String, Object> error(DerAsn1.Error error, String scope) {
        var result = map(
            "outcome", "error",
            "error_id", error.kind().id(),
            "offset", error.offset(),
            "offset_scope", scope);
        if (error.framingKind() != null) {
            result.put("framing_error_id", error.framingKind());
        }
        return result;
    }

    private static Map<String, Object> map(Object... pairs) {
        var result = new LinkedHashMap<String, Object>();
        for (int index = 0; index < pairs.length; index += 2) {
            result.put((String) pairs[index], pairs[index + 1]);
        }
        return result;
    }

    private static byte[] materialize(JsonNode segments) {
        var output = new ByteArrayOutputStream();
        for (JsonNode segment : segments) {
            byte[] value = hex(segment.has("hex")
                ? segment.path("hex").asText() : segment.path("repeat_hex").asText());
            int count = segment.has("hex") ? 1 : segment.path("count").asInt();
            for (int index = 0; index < count; index++) {
                output.writeBytes(value);
            }
        }
        return output.toByteArray();
    }

    private static byte[] hex(String value) {
        byte[] output = new byte[value.length() / 2];
        for (int index = 0; index < output.length; index++) {
            output[index] = (byte) Integer.parseInt(
                value.substring(index * 2, index * 2 + 2), 16);
        }
        return output;
    }

    private static String hex(byte[] value) {
        var output = new StringBuilder(value.length * 2);
        for (byte octet : value) {
            output.append(String.format("%02x", octet & 0xff));
        }
        return output.toString();
    }

    private static byte[] bytes(ByteBuffer buffer) {
        var copy = buffer.duplicate();
        byte[] output = new byte[copy.remaining()];
        copy.get(output);
        return output;
    }

    private static JsonNode readFixture(String name) {
        Path path = Path.of("..", "..", "..", "specs", "fixtures", name, "cases.json");
        try {
            return MAPPER.readTree(Files.readString(path));
        } catch (IOException error) {
            throw new ExceptionInInitializerError(error);
        }
    }
}
