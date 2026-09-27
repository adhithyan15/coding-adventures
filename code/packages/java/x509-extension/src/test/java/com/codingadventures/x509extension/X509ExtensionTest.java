package com.codingadventures.x509extension;

import com.codingadventures.derasn1.DerAsn1;
import com.codingadventures.dertlv.DerTlv;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.math.BigInteger;
import java.nio.ByteBuffer;
import java.nio.ReadOnlyBufferException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestFactory;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;

final class X509ExtensionTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();
    private static final JsonNode CONTRACT = readFixture("x509-extension-v1");
    private static final JsonNode UPSTREAM = readFixture("der-asn1-v1");

    @TestFactory
    Stream<DynamicTest> portableConformance() {
        assertEquals(48, CONTRACT.path("cases").size());
        assertEquals(8, CONTRACT.path("error_ids").size());
        var tests = new ArrayList<DynamicTest>();
        for (JsonNode testCase : CONTRACT.path("cases")) {
            tests.add(DynamicTest.dynamicTest(testCase.path("id").asText(), () -> {
                var decoder = new DerAsn1.Asn1Decoder(limits(testCase));
                var root = decoder.decodeExact(materialize(testCase.path("input")));
                Object actual;
                if (testCase.path("operation").asText().equals("extension-script")) {
                    var events = new ArrayList<Map<String, Object>>();
                    for (JsonNode ignored : testCase.path("actions")) {
                        events.add(attempt(decoder, root));
                    }
                    actual = map("outcome", "script", "events", events);
                } else {
                    actual = attempt(decoder, root);
                }
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
    void nativeContractBlocksForgingSnapshotsBytesAndRedactsErrors() {
        assertEquals(0, X509Extension.Value.class.getConstructors().length);

        byte[] input = hex("30090603551d1104023000");
        var decoder = new DerAsn1.Asn1Decoder();
        var root = decoder.decodeExact(input);
        var value = X509Extension.decodeX509Extension(decoder, root);
        input[9] = (byte) 0xff;
        assertEquals("3000", hex(bytes(value.extensionValue())));
        assertThrows(ReadOnlyBufferException.class,
            () -> value.extensionValue().put(0, (byte) 0xff));
        assertEquals("3000", hex(bytes(value.extensionValue())));

        var hostileDecoder = new DerAsn1.Asn1Decoder();
        var hostileRoot = hostileDecoder.decodeExact(hex("30080601800403deadbe"));
        var error = assertThrows(X509Extension.Error.class,
            () -> X509Extension.decodeX509Extension(hostileDecoder, hostileRoot));
        assertEquals(X509Extension.ErrorKind.INVALID_EXTENSION_ID, error.kind());
        assertEquals(DerAsn1.Asn1ErrorKind.NON_MINIMAL_OBJECT_IDENTIFIER,
            error.asn1Kind());
        assertEquals(4, error.offset());
        assertEquals(2, hostileDecoder.elementsRead());
        assertFalse(error.toString().contains("deadbe"));
    }

    private static Map<String, Object> attempt(
            DerAsn1.Asn1Decoder decoder, DerAsn1.Asn1Element root) {
        try {
            var value = X509Extension.decodeX509Extension(decoder, root);
            return map(
                "outcome", "value",
                "extension_id_arcs_decimal",
                    value.extensionId().arcs().stream().map(BigInteger::toString).toList(),
                "critical", value.critical(),
                "extension_value_hex", hex(bytes(value.extensionValue())),
                "elements_read", decoder.elementsRead());
        } catch (X509Extension.Error error) {
            var result = map(
                "outcome", "error",
                "error_id", error.kind().id(),
                "offset", error.offset(),
                "offset_scope", "extension-element",
                "elements_read", decoder.elementsRead());
            if (error.asn1Kind() != null) {
                result.put("asn1_error_id", error.asn1Kind().id());
            }
            if (error.framingKind() != null) {
                result.put("framing_error_id", error.framingKind());
            }
            return result;
        }
    }

    private static DerAsn1.Asn1Limits limits(JsonNode testCase) {
        JsonNode defaults = UPSTREAM.path("defaults");
        JsonNode overrides = testCase.path("limits");
        return new DerAsn1.Asn1Limits(
            derLimits(defaults.path("der"), overrides.path("der")),
            integerLimit(defaults, overrides, "max_depth"),
            longLimit(defaults, overrides, "max_total_elements"),
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
