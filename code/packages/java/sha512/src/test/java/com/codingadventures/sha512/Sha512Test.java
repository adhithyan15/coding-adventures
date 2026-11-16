package com.codingadventures.sha512;

import org.junit.jupiter.api.Test;

import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CharsetDecoder;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.Random;
import java.util.regex.Pattern;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

class Sha512Test {

    private static final Pattern ID_PATTERN = Pattern.compile("^[a-z0-9]+(?:-[a-z0-9]+)*$");
    private static final Pattern HEX_LOWER_PATTERN = Pattern.compile("^[0-9a-f]*$");
    private static final Pattern DIGEST_HEX_PATTERN = Pattern.compile("^[0-9a-f]{128}$");
    private static final Pattern WORD64_HEX_PATTERN = Pattern.compile("^[0-9a-f]{16}$");
    private static final Pattern CHUNKS_CSV_PATTERN = Pattern.compile("^[1-9][0-9]*(?:,[1-9][0-9]*)*$");

    private File findFixturesFile() {
        File current = new File(System.getProperty("user.dir"));
        for (int i = 0; i < 10; i++) {
            if (current == null) {
                break;
            }
            File candidate = new File(current, "code/specs/fixtures/sha512-v1/cases.txt");
            if (candidate.exists() && candidate.isFile()) {
                return candidate;
            }
            File candidate2 = new File(current, "fixtures/sha512-v1/cases.txt");
            if (candidate2.exists() && candidate2.isFile()) {
                return candidate2;
            }
            current = current.getParentFile();
        }
        throw new IllegalStateException("Could not find sha512-v1/cases.txt");
    }

    private String readBoundedCorpus(File file, int maxBytes) throws IOException {
        byte[] buf = new byte[maxBytes + 1];
        int total = 0;
        try (FileInputStream in = new FileInputStream(file)) {
            int read;
            while ((read = in.read(buf, total, buf.length - total)) > 0) {
                total += read;
                if (total > maxBytes) {
                    throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_OVERSIZED);
                }
            }
        }
        if (total < 1 || total > maxBytes) {
            throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_OVERSIZED);
        }
        CharsetDecoder decoder = StandardCharsets.UTF_8.newDecoder()
            .onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT);
        try {
            return decoder.decode(ByteBuffer.wrap(buf, 0, total)).toString();
        } catch (CharacterCodingException e) {
            throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED, e);
        }
    }

    private void parseCorpus(String content) {
        String[] lines = content.split("\r?\n");
        int rowCount = 0;
        int vCount = 0;
        int lCount = 0;

        for (String line : lines) {
            String trimmed = line.trim();
            if (trimmed.isEmpty() || trimmed.startsWith("#")) {
                continue;
            }
            rowCount++;
            String[] parts = trimmed.split("\\|", -1);
            if (parts.length != 5) {
                throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED);
            }
            String tag = parts[0];
            String id = parts[1];
            if (!ID_PATTERN.matcher(id).matches()) {
                throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED);
            }

            if ("V".equals(tag)) {
                vCount++;
                String inputHex = parts[2];
                String chunkCsv = parts[3];
                String expectedDigestHex = parts[4];

                if (!HEX_LOWER_PATTERN.matcher(inputHex).matches() || (inputHex.length() % 2 != 0) || inputHex.length() > 8192) {
                    throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED);
                }
                if (!DIGEST_HEX_PATTERN.matcher(expectedDigestHex).matches()) {
                    throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED);
                }
                byte[] input = inputHex.isEmpty() ? new byte[0] : HexFormat.of().parseHex(inputHex);
                if (input.length == 0) {
                    if (!chunkCsv.isEmpty()) {
                        throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED);
                    }
                } else {
                    if (chunkCsv.isEmpty() || !CHUNKS_CSV_PATTERN.matcher(chunkCsv).matches()) {
                        throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED);
                    }
                    String[] chunks = chunkCsv.split(",");
                    if (chunks.length > 64) {
                        throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED);
                    }
                    int sum = 0;
                    for (String chunkStr : chunks) {
                        sum += Integer.parseInt(chunkStr);
                    }
                    if (sum != input.length) {
                        throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED);
                    }
                }

                // Verify one-shot and streaming
                byte[] digest = Sha512.sha512(input);
                assertEquals(64, digest.length);
                assertEquals(expectedDigestHex, HexFormat.of().formatHex(digest));
                assertEquals(expectedDigestHex, Sha512.sha512Hex(input));

                Sha512.Hasher hasher = Sha512.create();
                if (!chunkCsv.isEmpty()) {
                    int offset = 0;
                    for (String chunkStr : chunkCsv.split(",")) {
                        int size = Integer.parseInt(chunkStr);
                        hasher.update(Arrays.copyOfRange(input, offset, offset + size));
                        offset += size;
                    }
                } else {
                    hasher.update(input);
                }
                assertEquals(expectedDigestHex, hasher.hexDigest());
            } else if ("L".equals(tag)) {
                lCount++;
                String highStr = parts[2];
                String lowStr = parts[3];
                String expected = parts[4];

                if (!WORD64_HEX_PATTERN.matcher(highStr).matches() || !WORD64_HEX_PATTERN.matcher(lowStr).matches()) {
                    throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED);
                }
                long high = Long.parseUnsignedLong(highStr, 16);
                long low = Long.parseUnsignedLong(lowStr, 16);

                if ("ERROR".equals(expected)) {
                    Sha512Exception ex = assertThrows(Sha512Exception.class, () -> Sha512.encodeBitLength(high, low));
                    assertEquals(Sha512Messages.ERR_MESSAGE_TOO_LONG, ex.getMessage());
                    assertEquals(Sha512ErrorCode.MESSAGE_TOO_LONG, ex.getCode());
                } else {
                    if (!HEX_LOWER_PATTERN.matcher(expected).matches() || expected.length() != 32) {
                        throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED);
                    }
                    byte[] encoded = Sha512.encodeBitLength(high, low);
                    assertEquals(16, encoded.length);
                    assertEquals(expected, HexFormat.of().formatHex(encoded));
                }
            } else {
                throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED);
            }
        }

        if (rowCount < 1 || rowCount > 64) {
            throw new IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED);
        }
        assertEquals(9, vCount, "Corpus must have exactly 9 V rows");
        assertEquals(6, lCount, "Corpus must have exactly 6 L rows");
    }

    @Test
    void testNeutralConformanceCorpus() throws IOException {
        File fixtureFile = findFixturesFile();
        String content = readBoundedCorpus(fixtureFile, 4096);
        parseCorpus(content);
    }

    @Test
    void testCorpusReaderFailsClosedOnOversized() throws IOException {
        File dummy = File.createTempFile("oversized", ".txt");
        dummy.deleteOnExit();
        Files.write(dummy.toPath(), new byte[4097]);
        assertThrows(IllegalArgumentException.class, () -> readBoundedCorpus(dummy, 4096));

        File empty = File.createTempFile("empty", ".txt");
        empty.deleteOnExit();
        assertThrows(IllegalArgumentException.class, () -> readBoundedCorpus(empty, 4096));
    }

    @Test
    void testCorpusReaderFailsClosedOnMalformedRows() {
        // Unknown tag
        assertThrows(IllegalArgumentException.class, () -> parseCorpus("X|unknown|1234|1|digest\n"));
        // Wrong column count
        assertThrows(IllegalArgumentException.class, () -> parseCorpus("V|id|1234\n"));
        // Uppercase hex in V input
        assertThrows(IllegalArgumentException.class, () -> parseCorpus("V|id|AB|1|" + "0".repeat(128) + "\n"));
        // Uppercase hex in V digest
        assertThrows(IllegalArgumentException.class, () -> parseCorpus("V|id|aa|1|" + "A".repeat(128) + "\n"));
        // Chunk sizes not summing to input size
        assertThrows(IllegalArgumentException.class, () -> parseCorpus("V|id|aabb|1|" + "0".repeat(128) + "\n"));
        // Invalid ID characters
        assertThrows(IllegalArgumentException.class, () -> parseCorpus("V|ID_UPPER|aa|1|" + "0".repeat(128) + "\n"));
        // Uppercase in L row word
        assertThrows(IllegalArgumentException.class, () -> parseCorpus("L|id|000000000000000A|0000000000000000|00000000000000000000000000000000\n"));
        // Short L row word
        assertThrows(IllegalArgumentException.class, () -> parseCorpus("L|id|0|0|ERROR\n"));
    }

    // ─── 6-Class SHA-512 Resilience Showcase ──────────────────────────────────

    @Test
    void testCveClass1CounterWraparoundPast61Bits() {
        long lowOverflow = 0x2000_0000_0000_0001L; // > 2^61 bytes
        byte[] encoded = Sha512.encodeBitLength(0L, lowOverflow);
        assertEquals("00000000000000010000000000000008", HexFormat.of().formatHex(encoded));

        byte[] encodedHigh = Sha512.encodeBitLength(1L, 0L);
        assertEquals("00000000000000080000000000000000", HexFormat.of().formatHex(encodedHigh));
    }

    @Test
    void testCveClass2FipsOverflowFailureAtomicity() {
        Sha512.Hasher hasher = Sha512.create();
        hasher.update(new byte[] {1, 2, 3});
        String hexBefore = hasher.hexDigest();

        Sha512Exception ex = assertThrows(Sha512Exception.class, () -> {
            Sha512.encodeBitLength(0x2000_0000_0000_0000L, 0L);
        });
        assertEquals(Sha512ErrorCode.MESSAGE_TOO_LONG, ex.getCode());

        // Hasher state must remain intact
        assertEquals(hexBefore, hasher.hexDigest());
        hasher.update(new byte[] {4, 5, 6});
        assertNotEquals(hexBefore, hasher.hexDigest());

        // Exercise Hasher.update counter carry
        Sha512.Hasher carryHasher = Sha512.create();
        carryHasher.setByteCountersForTesting(0L, -2L); // 0xFFFF_FFFF_FFFF_FFFE unsigned
        carryHasher.update(new byte[] {1, 2, 3}); // wraps low to 1, carries to high = 1L
        assertNotNull(carryHasher.hexDigest());

        // Exercise Hasher.update 128-bit domain overflow failure atomicity
        Sha512.Hasher overflowHasher = Sha512.create();
        overflowHasher.update(new byte[] {9, 8, 7});
        overflowHasher.setByteCountersForTesting(0x1FFF_FFFF_FFFF_FFFFL, -1L); // max valid FIPS bytes
        String snap = overflowHasher.hexDigest();

        Sha512Exception ofEx = assertThrows(Sha512Exception.class, () -> overflowHasher.update(new byte[] {1}));
        assertEquals(Sha512ErrorCode.MESSAGE_TOO_LONG, ofEx.getCode());
        assertEquals(snap, overflowHasher.hexDigest()); // state preserved
    }

    @Test
    void testCveClass3PaddingBoundarySplits() throws NoSuchAlgorithmException {
        MessageDigest std = MessageDigest.getInstance("SHA-512");

        for (int boundary : new int[] {111, 112, 127, 128, 129, 239, 240, 255, 256, 257}) {
            byte[] data = new byte[boundary];
            for (int i = 0; i < boundary; i++) {
                data[i] = (byte) (i * 31 + 7);
            }
            byte[] expected = std.digest(data);
            byte[] actual = Sha512.sha512(data);
            assertArrayEquals(expected, actual, "Failed on boundary " + boundary);
        }
    }

    @Test
    void testCveClass4BufferAliasingAndContinuation() {
        byte[] input = new byte[] {10, 20, 30, 40};
        Sha512.Hasher hasher = Sha512.create();
        hasher.update(input);

        input[0] = 99; // Mutate caller array
        String snapshot1 = hasher.hexDigest();

        hasher.update(new byte[] {50, 60});
        String snapshot2 = hasher.hexDigest();
        assertNotEquals(snapshot1, snapshot2);

        Sha512.Hasher copy = hasher.copy();
        assertEquals(snapshot2, copy.hexDigest());
        copy.update(new byte[] {70});
        assertNotEquals(hasher.hexDigest(), copy.hexDigest());
    }

    @Test
    void testCveClass5FailClosedOnNullAndOverflowSafeBounds() {
        Sha512.Hasher hasher = Sha512.create();
        hasher.update(new byte[] {1, 2, 3});
        String before = hasher.hexDigest();

        // Null pointer check
        NullPointerException npe = assertThrows(NullPointerException.class, () -> hasher.update(null, 0, 0));
        assertEquals(Sha512Messages.ERR_NULL_DATA, npe.getMessage());

        // Integer overflow attack (offset = 1, length = MAX_VALUE)
        byte[] testData = new byte[] {1, 2, 3, 4};
        IndexOutOfBoundsException ioobe = assertThrows(
            IndexOutOfBoundsException.class,
            () -> hasher.update(testData, 1, Integer.MAX_VALUE)
        );
        assertEquals(Sha512Messages.ERR_INDEX_OUT_OF_BOUNDS, ioobe.getMessage());
        assertEquals(before, hasher.hexDigest()); // Hasher state must be completely unmodified!

        // Negative offset
        assertThrows(IndexOutOfBoundsException.class, () -> hasher.update(testData, -1, 1));
        // Offset beyond length
        assertThrows(IndexOutOfBoundsException.class, () -> hasher.update(testData, 5, 1));
        // Length beyond array bounds
        assertThrows(IndexOutOfBoundsException.class, () -> hasher.update(testData, 0, 10));
    }

    @Test
    void testCveClass6PayloadBlindErrorDiagnostics() {
        Sha512Exception ex = assertThrows(Sha512Exception.class, () -> {
            Sha512.encodeBitLength(0xE000_0000_0000_0000L, 0L);
        });
        assertEquals(Sha512Messages.ERR_MESSAGE_TOO_LONG, ex.getMessage());
        assertEquals(Sha512ErrorCode.MESSAGE_TOO_LONG, ex.getCode());
        assertFalse(ex.getMessage().contains("0xE000"));
    }

    // ─── Multi-chunk Streaming Cross-check & 10,000-Iteration Fuzz ─────────────

    @Test
    void testMultiChunkAgainstJavaSecurityMessageDigest() throws NoSuchAlgorithmException {
        MessageDigest std = MessageDigest.getInstance("SHA-512");
        Random prng = new Random(0x5EED512L);

        for (int len : new int[] {0, 1, 16, 64, 127, 128, 129, 256, 1000, 4096}) {
            byte[] data = new byte[len];
            prng.nextBytes(data);

            std.reset();
            std.update(data);
            byte[] expected = std.digest();

            Sha512.Hasher hasher = Sha512.create();
            int offset = 0;
            while (offset < len) {
                int chunkSize = Math.min(len - offset, prng.nextInt(64) + 1);
                hasher.update(data, offset, chunkSize);
                offset += chunkSize;
            }
            byte[] actual = hasher.digest();
            assertArrayEquals(expected, actual, "Mismatch on length " + len);
        }
    }

    @Test
    void testDeterministicFuzzLoop10kIterations() throws NoSuchAlgorithmException {
        MessageDigest std = MessageDigest.getInstance("SHA-512");
        Random prng = new Random(0xDEADBEEF512L);

        for (int iter = 0; iter < 10_000; iter++) {
            int len = prng.nextInt(300);
            byte[] data = new byte[len];
            prng.nextBytes(data);

            std.reset();
            std.update(data);
            byte[] expected = std.digest();

            Sha512.Hasher hasher = Sha512.create();
            int offset = 0;
            while (offset < len) {
                int chunkSize = Math.min(len - offset, prng.nextInt(50) + 1);
                hasher.update(data, offset, chunkSize);
                offset += chunkSize;
            }
            assertArrayEquals(expected, hasher.digest(), "Fuzz iteration failed at " + iter);
        }
    }

    @Test
    void testCopyConstructorNullCheck() {
        assertThrows(NullPointerException.class, () -> new Sha512.Hasher(null));
    }

    @Test
    void testMessagesCoverageAndEmptyUpdate() {
        for (Sha512ErrorCode code : Sha512ErrorCode.values()) {
            assertNotNull(Sha512Messages.messageFor(code));
        }
        assertEquals(Sha512Messages.ERR_UNKNOWN, Sha512Messages.messageFor(null));

        Sha512.Hasher hasher = Sha512.create();
        hasher.update(new byte[] {1, 2, 3});
        hasher.update(new byte[] {4, 5, 6}, 0, 0); // length == 0 branch
        assertEquals(Sha512.sha512Hex(new byte[] {1, 2, 3}), hasher.hexDigest());
    }
}
