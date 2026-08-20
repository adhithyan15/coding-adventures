package com.codingadventures.zip;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HexFormat;
import java.util.zip.DataFormatException;
import java.util.zip.Deflater;
import java.util.zip.Inflater;
import org.junit.jupiter.api.Test;

/** Runs the shared CMP09 raw RFC 1951 corpus against the Java lane. */
final class RawRfc1951ConformanceTest {
    private static final ObjectMapper JSON = new ObjectMapper();
    private static final HexFormat HEX = HexFormat.of();

    @Test
    void closedPortableCorpusPasses() throws Exception {
        JsonNode root = JSON.readTree(Files.readString(findFixture()));
        assertEquals(1, root.path("schema_version").asInt());
        assertEquals("zip-owned-raw-rfc1951-v1", root.path("profile").asText());
        assertEquals(RawRfc1951.MAX_OUTPUT, root.path("limits").path("default_max_output").asInt());
        assertEquals(RawRfc1951.MAX_OUTPUT, root.path("limits").path("hard_max_output").asInt());
        assertEquals(RawRfc1951.ERROR_CODES, JSON.convertValue(root.path("error_ids"),
            JSON.getTypeFactory().constructCollectionType(java.util.List.class, String.class)));
        assertEquals(34, root.path("cases").size());

        for (JsonNode testCase : root.path("cases")) {
            String id = testCase.path("id").asText();
            int limit = testCase.has("max_output")
                ? testCase.path("max_output").asInt()
                : RawRfc1951.MAX_OUTPUT;
            switch (testCase.path("operation").asText()) {
                case "inflate" -> {
                    byte[] input = fromHex(testCase.path("input_hex").asText());
                    byte[] expected = materialize(testCase.path("expected").path("output"));
                    RawRfc1951.InflateResult result = RawRfc1951.rawInflateCounted(input, limit);
                    assertArrayEquals(expected, result.output(), id);
                    assertEquals(testCase.path("expected").path("bytes_consumed").asInt(),
                        result.bytesConsumed(), id);
                    assertArrayEquals(expected, RawRfc1951.rawInflate(input, limit), id);
                }
                case "inflate-error" -> {
                    byte[] input = fromHex(testCase.path("input_hex").asText());
                    String expected = testCase.path("expected").path("error_id").asText();
                    RawRfc1951.RawInflateException error = assertThrows(
                        RawRfc1951.RawInflateException.class,
                        () -> RawRfc1951.rawInflateCounted(input, limit), id);
                    assertEquals(expected, error.code(), id);
                    assertEquals(expected, error.getMessage(), id);
                }
                case "deflate-interoperability" -> {
                    byte[] input = fromHex(testCase.path("input_hex").asText());
                    byte[] expected = materialize(testCase.path("expected").path("output"));
                    assertArrayEquals(expected, jdkCodec("decompress", RawRfc1951.rawDeflate(input)), id);
                }
                case "crc32" -> {
                    long checksum = testCase.has("initial_crc32_hex")
                        ? Long.parseUnsignedLong(testCase.path("initial_crc32_hex").asText(), 16)
                        : 0L;
                    for (JsonNode chunk : testCase.path("chunks_hex")) {
                        checksum = RawRfc1951.crc32(fromHex(chunk.asText()), checksum);
                    }
                    assertEquals(testCase.path("expected").path("crc32_hex").asText(),
                        String.format("%08x", checksum), id);
                }
                default -> throw new AssertionError(id + ": unknown operation");
            }
        }
    }

    @Test
    void foreignFullWindowAndHistoricalWrapperPass() throws Exception {
        byte[] expected = new byte[65_536];
        for (int index = 0; index < 32_768; index++) {
            expected[index] = (byte) ((index * 73 + index / 251) & 0xff);
            expected[index + 32_768] = expected[index];
        }
        assertArrayEquals(expected,
            RawRfc1951.rawInflate(jdkCodec("compress", expected), expected.length));

        byte[] historical = "historical wrapper compatibility".getBytes(StandardCharsets.UTF_8);
        assertArrayEquals(historical, RawRfc1951.rawInflate(RawRfc1951.rawDeflate(historical)));
    }

    @Test
    void zipReaderRequiresExactContainerBoundaries() throws Exception {
        byte[] compressed = fromHex("0dc28911c0200c03b0d8f97028ec3f6ed129cab7dd96a0c2445bdb93809663a5d303f6b265e20c2b79ea03379d227e");
        byte[] plain = fromHex("0406030b000e070909010906010a04070007000000000501010908030108050302030401000401000207090009020a0a020605020d060c01020b020302090201");
        assertArrayEquals(plain, new Zip.ZipReader(rawZip("dynamic.bin", compressed, plain, plain.length, 8))
            .read("dynamic.bin"));

        byte[] cavity = java.util.Arrays.copyOf(compressed, compressed.length + 2);
        cavity[cavity.length - 2] = (byte) 0xde;
        cavity[cavity.length - 1] = (byte) 0xad;
        assertMessage("zip: compressed payload contains trailing bytes", () ->
            new Zip.ZipReader(rawZip("cavity.bin", cavity, plain, plain.length, 8)).read("cavity.bin"));
        assertMessage("zip: uncompressed size does not match the directory", () ->
            new Zip.ZipReader(rawZip("size.bin", compressed, plain, plain.length + 1, 8)).read("size.bin"));
        assertMessage("zip: stored entry sizes do not match", () ->
            new Zip.ZipReader(rawZip("stored.bin", plain, plain, plain.length + 1, 0)).read("stored.bin"));
        assertMessage("zip: raw inflate failed: reserved-block-type", () ->
            new Zip.ZipReader(rawZip("malformed.bin", new byte[]{0x07}, new byte[0], 0, 8))
                .read("malformed.bin"));

        Zip.ZipWriter writer = new Zip.ZipWriter();
        writer.addFile("a.bin", new byte[4], false);
        writer.addFile("b.bin", new byte[4], false);
        assertMessage("zip: aggregate decompressed size exceeds the 7-byte limit (decompression bomb guard)",
            () -> Zip.unzip(writer.finish(), 7));

        byte[] invalidLargeEntry = rawZip(
            "preflight.bin", new byte[]{0x07}, new byte[0], 8, 8);
        assertMessage("zip: aggregate decompressed size exceeds the 7-byte limit (decompression bomb guard)",
            () -> Zip.unzip(invalidLargeEntry, 7));
    }

    private static void assertMessage(String expected, ThrowingAction action) {
        IOException error = assertThrows(IOException.class, action::run);
        assertEquals(expected, error.getMessage());
    }

    private static byte[] materialize(JsonNode output) {
        if (output.has("hex")) return fromHex(output.path("hex").asText());
        byte value = fromHex(output.path("repeat_hex").asText())[0];
        byte[] result = new byte[output.path("count").asInt()];
        java.util.Arrays.fill(result, value);
        return result;
    }

    private static byte[] fromHex(String value) {
        return HEX.parseHex(value);
    }

    private static Path findFixture() throws IOException {
        for (Path directory = Path.of("").toAbsolutePath(); directory != null; directory = directory.getParent()) {
            Path candidate = directory.resolve("code/specs/fixtures/zip-raw-rfc1951-v1/cases.json");
            if (Files.isRegularFile(candidate)) return candidate;
        }
        throw new IOException("zip-raw-rfc1951-v1 fixture not found");
    }

    private static byte[] jdkCodec(String mode, byte[] input) throws DataFormatException {
        byte[] buffer = new byte[4096];
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        if (mode.equals("compress")) {
            Deflater codec = new Deflater(9, true);
            try {
                codec.setInput(input);
                codec.finish();
                while (!codec.finished()) output.write(buffer, 0, codec.deflate(buffer));
            } finally {
                codec.end();
            }
            return output.toByteArray();
        }

        Inflater codec = new Inflater(true);
        try {
            // JDK's nowrap inflater requires one dummy byte after a raw stream.
            codec.setInput(java.util.Arrays.copyOf(input, input.length + 1));
            while (!codec.finished()) {
                int count = codec.inflate(buffer);
                if (count == 0 && (codec.needsInput() || codec.needsDictionary())) {
                    throw new DataFormatException("JDK raw inflater did not reach end-of-stream");
                }
                output.write(buffer, 0, count);
            }
        } finally {
            codec.end();
        }
        return output.toByteArray();
    }

    private static byte[] rawZip(String name, byte[] compressed, byte[] plain, int declaredSize, int method) {
        ByteArrayOutputStream archive = new ByteArrayOutputStream();
        byte[] nameBytes = name.getBytes(StandardCharsets.UTF_8);
        long checksum = RawRfc1951.crc32(plain);
        u32(archive, 0x04034b50L); u16(archive, 20); u16(archive, 0x0800); u16(archive, method);
        u16(archive, 0); u16(archive, 0); u32(archive, checksum); u32(archive, compressed.length);
        u32(archive, declaredSize); u16(archive, nameBytes.length); u16(archive, 0); archive.writeBytes(nameBytes); archive.writeBytes(compressed);
        int centralOffset = archive.size();
        u32(archive, 0x02014b50L); u16(archive, 0x031e); u16(archive, 20); u16(archive, 0x0800); u16(archive, method);
        u16(archive, 0); u16(archive, 0); u32(archive, checksum); u32(archive, compressed.length); u32(archive, declaredSize);
        u16(archive, nameBytes.length); u16(archive, 0); u16(archive, 0); u16(archive, 0); u16(archive, 0);
        u32(archive, 0); u32(archive, 0); archive.writeBytes(nameBytes);
        int centralSize = archive.size() - centralOffset;
        u32(archive, 0x06054b50L); u16(archive, 0); u16(archive, 0); u16(archive, 1); u16(archive, 1);
        u32(archive, centralSize); u32(archive, centralOffset); u16(archive, 0);
        return archive.toByteArray();
    }

    private static void u16(ByteArrayOutputStream out, int value) {
        out.write(value); out.write(value >>> 8);
    }

    private static void u32(ByteArrayOutputStream out, long value) {
        u16(out, (int) value); u16(out, (int) (value >>> 16));
    }

    @FunctionalInterface
    private interface ThrowingAction { void run() throws Exception; }
}
