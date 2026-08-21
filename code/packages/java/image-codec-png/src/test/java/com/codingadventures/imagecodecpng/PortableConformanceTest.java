package com.codingadventures.imagecodecpng;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.fail;

import com.codingadventures.pixelcontainer.PixelContainer;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.awt.image.BufferedImage;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;
import java.util.zip.InflaterInputStream;
import javax.imageio.ImageIO;
import org.junit.jupiter.api.Test;

final class PortableConformanceTest {
    private static final ObjectMapper JSON = new ObjectMapper();
    private static final HexFormat HEX = HexFormat.of();

    private record Chunk(String type, byte[] data) {}

    @Test
    void consumesEveryPortableCaseThroughPublicApis() throws Exception {
        JsonNode document = JSON.readTree(Files.readString(fixturePath()));
        JsonNode cases = document.required("cases");

        assertEquals(1, document.required("schema_version").intValue());
        assertEquals("image-codec-png-v1", document.required("profile").textValue());
        assertEquals(85, cases.size());
        assertEquals(Png.MAX_DIMENSION, document.at("/limits/max_dimension").intValue());
        assertEquals(Png.DEFAULT_MAX_PIXELS, document.at("/limits/default_max_pixels").longValue());

        List<String> expectedErrors = new ArrayList<>();
        document.required("error_ids").forEach(node -> expectedErrors.add(node.textValue()));
        assertEquals(expectedErrors, Png.ERROR_CODES);

        for (JsonNode fixture : cases) {
            String id = fixture.required("id").textValue();
            try {
                switch (fixture.required("operation").textValue()) {
                    case "decode" -> assertDecode(fixture);
                    case "decode-error" -> assertDecodeError(fixture);
                    case "encode" -> assertEncode(fixture);
                    case "encode-error" -> assertEncodeError(fixture);
                    case "adler32" -> assertAdler(fixture);
                    default -> fail("unknown fixture operation for " + id);
                }
            } catch (AssertionError | RuntimeException error) {
                throw new AssertionError("portable case failed: " + id, error);
            }
        }
    }

    private static void assertDecode(JsonNode fixture) {
        PixelContainer actual = decode(fixture);
        JsonNode expected = fixture.required("expected");
        assertEquals(expected.required("width").intValue(), actual.width);
        assertEquals(expected.required("height").intValue(), actual.height);
        assertArrayEquals(hex(expected.required("rgba_hex").textValue()), actual.data);
    }

    private static void assertDecodeError(JsonNode fixture) {
        PngError error = assertThrows(PngError.class, () -> decode(fixture));
        String expected = fixture.at("/expected/error_id").textValue();
        assertEquals(expected, error.code());
        assertEquals(expected, error.getMessage());
    }

    private static PixelContainer decode(JsonNode fixture) {
        byte[] png = hex(fixture.required("png_hex").textValue());
        JsonNode options = fixture.get("options");
        if (options == null) {
            return Png.decodePng(png);
        }
        return Png.decodePng(png, options.required("max_pixels").doubleValue());
    }

    private static void assertEncode(JsonNode fixture) throws IOException {
        JsonNode input = fixture.required("input");
        byte[] encoded = encodeFixture(input);
        JsonNode expected = fixture.required("expected");
        List<Chunk> chunks = parseChunks(encoded);

        List<String> actualTypes = chunks.stream().map(Chunk::type).toList();
        List<String> expectedTypes = new ArrayList<>();
        expected.required("chunk_types").forEach(node -> expectedTypes.add(node.textValue()));
        assertEquals(expectedTypes, actualTypes);
        assertEquals(expected.required("bit_depth").intValue(), encoded[24] & 0xff);
        assertEquals(expected.required("colour_type").intValue(), encoded[25] & 0xff);
        assertEquals(expected.required("interlace").intValue(), encoded[28] & 0xff);

        byte[] filtered = inflateIdat(chunks);
        int width = exactFixtureDimension(input.required("width"));
        int height = exactFixtureDimension(input.required("height"));
        List<Integer> actualFilters = new ArrayList<>();
        int rowSize = width * 4 + 1;
        for (int row = 0; row < height; row++) {
            actualFilters.add(filtered[row * rowSize] & 0xff);
        }
        List<Integer> expectedFilters = new ArrayList<>();
        expected.required("filter_types").forEach(node -> expectedFilters.add(node.intValue()));
        assertEquals(expectedFilters, actualFilters);

        BufferedImage foreign = ImageIO.read(new ByteArrayInputStream(encoded));
        assertNotNull(foreign);
        assertEquals(width, foreign.getWidth());
        assertEquals(height, foreign.getHeight());
        assertArrayEquals(hex(input.required("rgba_hex").textValue()), rgbaBytes(foreign));
    }

    private static void assertEncodeError(JsonNode fixture) {
        PngError error = assertThrows(PngError.class, () -> encodeFixture(fixture.required("input")));
        String expected = fixture.at("/expected/error_id").textValue();
        assertEquals(expected, error.code());
        assertEquals(expected, error.getMessage());
    }

    private static byte[] encodeFixture(JsonNode input) {
        int width = exactFixtureDimension(input.required("width"));
        int height = exactFixtureDimension(input.required("height"));
        PixelContainer pixels = new PixelContainer(
            width,
            height,
            hex(input.required("rgba_hex").textValue())
        );
        return Png.encodePng(pixels);
    }

    private static int exactFixtureDimension(JsonNode node) {
        if (!node.isNumber()) {
            throw new PngError("invalid-image-dimensions");
        }
        double value = node.doubleValue();
        if (!Double.isFinite(value) || Math.rint(value) != value
                || value < Integer.MIN_VALUE || value > Integer.MAX_VALUE) {
            throw new PngError("invalid-image-dimensions");
        }
        return (int) value;
    }

    private static void assertAdler(JsonNode fixture) {
        long actual = Png.adler32(hex(fixture.required("input_hex").textValue()));
        assertEquals(fixture.at("/expected/adler32_hex").textValue(), "%08x".formatted(actual));
    }

    private static List<Chunk> parseChunks(byte[] png) {
        List<Chunk> chunks = new ArrayList<>();
        int offset = 8;
        while (offset < png.length) {
            long length = readU32(png, offset);
            long end = offset + 12L + length;
            if (length > Integer.MAX_VALUE || end > png.length) {
                fail("encoder produced a truncated chunk");
            }
            int size = (int) length;
            String type = new String(png, offset + 4, 4, java.nio.charset.StandardCharsets.US_ASCII);
            chunks.add(new Chunk(type, java.util.Arrays.copyOfRange(png, offset + 8, offset + 8 + size)));
            offset = (int) end;
        }
        return chunks;
    }

    private static byte[] inflateIdat(List<Chunk> chunks) throws IOException {
        ByteArrayOutputStream idat = new ByteArrayOutputStream();
        for (Chunk chunk : chunks) {
            if (chunk.type().equals("IDAT")) {
                idat.writeBytes(chunk.data());
            }
        }
        try (InflaterInputStream inflater = new InflaterInputStream(
                new ByteArrayInputStream(idat.toByteArray()))) {
            return inflater.readAllBytes();
        }
    }

    private static byte[] rgbaBytes(BufferedImage image) {
        byte[] rgba = new byte[Math.multiplyExact(Math.multiplyExact(image.getWidth(), image.getHeight()), 4)];
        int offset = 0;
        for (int y = 0; y < image.getHeight(); y++) {
            for (int x = 0; x < image.getWidth(); x++) {
                int argb = image.getRGB(x, y);
                rgba[offset++] = (byte) (argb >>> 16);
                rgba[offset++] = (byte) (argb >>> 8);
                rgba[offset++] = (byte) argb;
                rgba[offset++] = (byte) (argb >>> 24);
            }
        }
        return rgba;
    }

    private static long readU32(byte[] data, int offset) {
        return ((long) (data[offset] & 0xff) << 24)
            | ((long) (data[offset + 1] & 0xff) << 16)
            | ((long) (data[offset + 2] & 0xff) << 8)
            | (data[offset + 3] & 0xffL);
    }

    private static byte[] hex(String value) {
        return HEX.parseHex(value);
    }

    private static Path fixturePath() {
        Path current = Path.of("").toAbsolutePath();
        while (current != null) {
            Path candidate = current.resolve(
                "code/specs/fixtures/image-codec-png-v1/cases.json"
            );
            if (Files.isRegularFile(candidate)) {
                return candidate;
            }
            current = current.getParent();
        }
        throw new AssertionError("could not locate IC18 portable fixture corpus");
    }
}
