package com.codingadventures.dertlv;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
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
import static org.junit.jupiter.api.Assertions.assertThrows;

final class DerTlvTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();
    private static final JsonNode FIXTURE = readFixture();

    @TestFactory
    Stream<DynamicTest> portableConformance() {
        var tests = new ArrayList<DynamicTest>();
        for (JsonNode testCase : FIXTURE.path("cases")) {
            tests.add(DynamicTest.dynamicTest(testCase.path("id").asText(), () -> {
                byte[] input = materialize(testCase.path("input"));
                Object actual = testCase.path("operation").asText().equals("cursor")
                    ? runCursor(testCase, input)
                    : runDecode(testCase, input);
                assertEquals(
                    testCase.path("expected"),
                    MAPPER.readTree(MAPPER.writeValueAsString(actual)));
                if (testCase.has("redacted_input_hex")) {
                    String serialized = MAPPER.writeValueAsString(actual);
                    assertEquals(-1, serialized.indexOf(testCase.path("redacted_input_hex").asText()));
                }
            }));
        }
        return tests.stream();
    }

    @Test
    void viewsShareTheCallerArrayAndDefaultsArePublic() {
        byte[] input = {4, 1, 42};
        var element = DerTlv.decodeExact(input);
        input[2] = 127;
        assertArrayEquals(new byte[] {127}, bytes(element.value()));
        assertEquals(4_096, DerTlv.Limits.defaults().maxElements());
        assertEquals(2, new DerTlv.Cursor(new byte[] {5, 0}).remaining().remaining());
    }

    @Test
    void rejectsInvalidLimitsAndNulls() {
        assertThrows(IllegalArgumentException.class, () -> new DerTlv.Limits(1, 1, -1, 1));
        assertThrows(IllegalArgumentException.class, () -> new DerTlv.Limits(1, 1, 1, 0x1_0000_0000L));
        assertThrows(NullPointerException.class, () -> DerTlv.decodeExact(null));
        assertThrows(NullPointerException.class, () -> new DerTlv.Cursor(null));
    }

    private static Object runDecode(JsonNode testCase, byte[] input) {
        try {
            if (testCase.path("operation").asText().equals("decode-one")) {
                var decoded = DerTlv.decodeOne(input, limits(testCase));
                var result = elementProjection(decoded.element(), 0);
                assertEquals(result.get("remainder_offset"), input.length - decoded.remainder().remaining());
                return result;
            }
            return elementProjection(DerTlv.decodeExact(input, limits(testCase)), 0);
        } catch (DerTlv.Error error) {
            return errorProjection(error);
        }
    }

    private static Object runCursor(JsonNode testCase, byte[] input) {
        var cursor = new DerTlv.Cursor(input, limits(testCase));
        var events = new ArrayList<Map<String, Object>>();
        for (JsonNode action : testCase.path("actions")) {
            if (action.asText().equals("finish")) {
                try {
                    cursor.finish();
                    events.add(Map.of("outcome", "finished"));
                } catch (DerTlv.Error error) {
                    events.add(errorProjection(error));
                }
            } else {
                int offset = input.length - cursor.remaining().remaining();
                try {
                    var element = cursor.read();
                    events.add(element == null ? Map.of("outcome", "end") : elementProjection(element, offset));
                } catch (DerTlv.Error error) {
                    events.add(errorProjection(error));
                }
            }
        }
        return Map.of(
            "events", events,
            "elements_read", cursor.elementsRead(),
            "remaining_offset", input.length - cursor.remaining().remaining());
    }

    private static Map<String, Object> elementProjection(DerTlv.Element element, int offset) {
        Map<String, Object> result = new LinkedHashMap<>();
        result.put("outcome", "element");
        result.put("element_offset", offset);
        result.put("tag", Map.of(
            "class", element.tag().tagClass(),
            "constructed", element.tag().constructed(),
            "number", element.tag().number()));
        result.put("header_len", element.header().remaining());
        result.put("encoded_len", element.encoded().remaining());
        result.put("remainder_offset", offset + element.encoded().remaining());
        return result;
    }

    private static Map<String, Object> errorProjection(DerTlv.Error error) {
        return Map.of("outcome", "error", "error_id", error.kind(), "offset", error.offset());
    }

    private static DerTlv.Limits limits(JsonNode testCase) {
        JsonNode overrides = testCase.path("limits");
        JsonNode defaults = FIXTURE.path("defaults");
        return new DerTlv.Limits(
            limit(defaults, overrides, "max_input_len"),
            limit(defaults, overrides, "max_value_len"),
            limit(defaults, overrides, "max_elements"),
            limit(defaults, overrides, "max_tag_number"));
    }

    private static long limit(JsonNode defaults, JsonNode overrides, String name) {
        JsonNode value = overrides.has(name) ? overrides.path(name) : defaults.path(name);
        return value.isTextual() ? Long.MAX_VALUE : value.asLong();
    }

    private static byte[] materialize(JsonNode segments) {
        var output = new java.io.ByteArrayOutputStream();
        for (JsonNode segment : segments) {
            byte[] value = hex(segment.has("hex") ? segment.path("hex").asText() : segment.path("repeat_hex").asText());
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
            output[index] = (byte) Integer.parseInt(value.substring(index * 2, index * 2 + 2), 16);
        }
        return output;
    }

    private static byte[] bytes(java.nio.ByteBuffer buffer) {
        byte[] output = new byte[buffer.remaining()];
        buffer.get(output);
        return output;
    }

    private static JsonNode readFixture() {
        Path path = Path.of("..", "..", "..", "specs", "fixtures", "der-tlv-v1", "cases.json");
        try {
            return MAPPER.readTree(Files.readString(path));
        } catch (IOException error) {
            throw new ExceptionInInitializerError(error);
        }
    }
}
