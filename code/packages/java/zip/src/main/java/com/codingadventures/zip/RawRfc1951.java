package com.codingadventures.zip;

import java.io.IOException;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * ZIP-owned raw RFC 1951 and CRC-32 primitives.
 *
 * <p>The API deliberately says {@code raw}: these streams contain no ZIP,
 * zlib, or gzip framing. Production is a pure in-memory byte transform and
 * owns no filesystem, process, network, environment, clock, entropy, FFI, or
 * credential authority.</p>
 */
public final class RawRfc1951 {
    /** Default and hard output ceiling: 256 MiB. */
    public static final int MAX_OUTPUT = 256 * 1024 * 1024;

    /** Complete payload-blind failure taxonomy, in fixture order. */
    public static final List<String> ERROR_CODES = List.of(
        "invalid-output-limit",
        "unexpected-eof",
        "reserved-block-type",
        "stored-length-mismatch",
        "huffman-oversubscribed",
        "incomplete-code-length-tree",
        "incomplete-literal-length-tree",
        "incomplete-distance-tree",
        "repeat-without-previous",
        "repeat-overrun",
        "invalid-literal-length-symbol",
        "reserved-distance-symbol",
        "invalid-back-reference",
        "output-limit-exceeded"
    );

    private RawRfc1951() {}

    /** A stable raw-inflate failure whose message contains only its code. */
    public static final class RawInflateException extends IOException {
        private final String code;

        RawInflateException(String code) {
            super(code);
            this.code = code;
        }

        /** Return the stable portable failure identifier. */
        public String code() {
            return code;
        }
    }

    /**
     * Decoded bytes and the exact compressed byte count reached.
     * The output array is newly allocated and owned by the caller.
     */
    public record InflateResult(byte[] output, int bytesConsumed) {}

    /** Compress bytes as raw RFC 1951 without ZIP, zlib, or gzip framing. */
    public static byte[] rawDeflate(byte[] data) throws IOException {
        if (data == null) throw new NullPointerException("data");
        return Zip.DeflateCompressor.compress(data);
    }

    /** Inflate raw RFC 1951 with the default 256 MiB ceiling. */
    public static byte[] rawInflate(byte[] data) throws RawInflateException {
        return rawInflate(data, MAX_OUTPUT);
    }

    /** Inflate raw RFC 1951 with a caller-lowerable output ceiling. */
    public static byte[] rawInflate(byte[] data, int maxOutput) throws RawInflateException {
        return rawInflateCounted(data, maxOutput).output();
    }

    /** Inflate and report the exact final compressed input byte reached. */
    public static InflateResult rawInflateCounted(byte[] data, int maxOutput)
            throws RawInflateException {
        if (data == null) throw new NullPointerException("data");
        return Inflater.inflate(data, maxOutput);
    }

    /** Compute incremental ZIP CRC-32, passing the previous result as initial. */
    public static long crc32(byte[] data, long initial) {
        if (data == null) throw new NullPointerException("data");
        return Zip.Crc32.compute(data, initial) & 0xffffffffL;
    }

    /** Compute ZIP CRC-32 from the standard zero initial value. */
    public static long crc32(byte[] data) {
        return crc32(data, 0L);
    }

    private static final class Inflater {
        private enum Completeness { CODE_LENGTH, LITERAL_LENGTH, DISTANCE }

        private record Tables(HuffmanTable literalLength, HuffmanTable distance) {}

        private static final class HuffmanTable {
            private final Map<Integer, Integer>[] codesByLength;
            private final int maximumLength;

            HuffmanTable(Map<Integer, Integer>[] codesByLength, int maximumLength) {
                this.codesByLength = codesByLength;
                this.maximumLength = maximumLength;
            }

            int decode(Zip.BitReader reader) throws RawInflateException {
                if (maximumLength == 0) fail("unexpected-eof");
                int code = 0;
                for (int length = 1; length <= maximumLength; length++) {
                    int bit = reader.readLsb(1);
                    if (bit < 0) fail("unexpected-eof");
                    code = (code << 1) | bit;
                    Integer symbol = codesByLength[length].get(code);
                    if (symbol != null) return symbol;
                }
                return failResult("unexpected-eof");
            }
        }

        private static final class OutputBuffer {
            private final int maximum;
            private byte[] data;
            private int size;

            OutputBuffer(int maximum) {
                this.maximum = maximum;
                this.data = new byte[Math.min(maximum, 8192)];
            }

            int size() {
                return size;
            }

            void add(int value) throws RawInflateException {
                ensure(1);
                data[size++] = (byte) value;
            }

            void copy(int distance, int length) throws RawInflateException {
                if (distance <= 0 || distance > size) fail("invalid-back-reference");
                ensure(length);
                for (int index = 0; index < length; index++) {
                    data[size] = data[size - distance];
                    size++;
                }
            }

            void ensure(int additional) throws RawInflateException {
                if (additional > maximum - size) fail("output-limit-exceeded");
                int required = size + additional;
                if (required <= data.length) return;
                int doubled = data.length == 0 ? 1 : data.length * 2;
                data = Arrays.copyOf(data, Math.min(maximum, Math.max(required, doubled)));
            }

            byte[] toArray() {
                return Arrays.copyOf(data, size);
            }
        }

        static InflateResult inflate(byte[] data, int maximumOutput)
                throws RawInflateException {
            if (maximumOutput < 0 || maximumOutput > MAX_OUTPUT) fail("invalid-output-limit");
            Zip.BitReader reader = new Zip.BitReader(data);
            OutputBuffer output = new OutputBuffer(maximumOutput);

            while (true) {
                int finalBlock = reader.readLsb(1);
                int type = reader.readLsb(2);
                if (finalBlock < 0 || type < 0) fail("unexpected-eof");
                switch (type) {
                    case 0 -> readStored(reader, output);
                    case 1 -> {
                        Tables tables = fixedTables();
                        decodeCompressed(reader, output, tables.literalLength(), tables.distance());
                    }
                    case 2 -> {
                        Tables tables = readDynamicTables(reader);
                        decodeCompressed(reader, output, tables.literalLength(), tables.distance());
                    }
                    default -> fail("reserved-block-type");
                }
                if (finalBlock == 1) return new InflateResult(output.toArray(), reader.position());
            }
        }

        private static void readStored(Zip.BitReader reader, OutputBuffer output)
                throws RawInflateException {
            reader.align();
            int length = reader.readLsb(16);
            int complement = reader.readLsb(16);
            if (length < 0 || complement < 0) fail("unexpected-eof");
            if (length != (complement ^ 0xffff)) fail("stored-length-mismatch");
            output.ensure(length);
            for (int index = 0; index < length; index++) {
                int value = reader.readLsb(8);
                if (value < 0) fail("unexpected-eof");
                output.add(value);
            }
        }

        private static Tables fixedTables() throws RawInflateException {
            int[] literalLengths = new int[288];
            Arrays.fill(literalLengths, 0, 144, 8);
            Arrays.fill(literalLengths, 144, 256, 9);
            Arrays.fill(literalLengths, 256, 280, 7);
            Arrays.fill(literalLengths, 280, 288, 8);
            int[] distanceLengths = new int[32];
            Arrays.fill(distanceLengths, 5);
            return new Tables(
                buildHuffman(literalLengths, Completeness.LITERAL_LENGTH),
                buildHuffman(distanceLengths, Completeness.DISTANCE));
        }

        private static Tables readDynamicTables(Zip.BitReader reader)
                throws RawInflateException {
            int literalCount = reader.readLsb(5);
            int distanceCount = reader.readLsb(5);
            int codeLengthCount = reader.readLsb(4);
            if (literalCount < 0 || distanceCount < 0 || codeLengthCount < 0)
                fail("unexpected-eof");
            literalCount += 257;
            distanceCount += 1;
            codeLengthCount += 4;
            if (literalCount > 286) fail("invalid-literal-length-symbol");

            int[] order = {16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15};
            int[] codeLengths = new int[19];
            for (int index = 0; index < codeLengthCount; index++) {
                int length = reader.readLsb(3);
                if (length < 0) fail("unexpected-eof");
                codeLengths[order[index]] = length;
            }
            HuffmanTable codeLengthTable = buildHuffman(codeLengths, Completeness.CODE_LENGTH);

            int total = literalCount + distanceCount;
            int[] lengths = new int[total];
            int count = 0;
            while (count < total) {
                int symbol = codeLengthTable.decode(reader);
                if (symbol >= 0 && symbol <= 15) {
                    lengths[count++] = symbol;
                } else if (symbol == 16) {
                    if (count == 0) fail("repeat-without-previous");
                    int extra = reader.readLsb(2);
                    if (extra < 0) fail("unexpected-eof");
                    count = repeat(lengths, count, lengths[count - 1], extra + 3);
                } else if (symbol == 17) {
                    int extra = reader.readLsb(3);
                    if (extra < 0) fail("unexpected-eof");
                    count = repeat(lengths, count, 0, extra + 3);
                } else if (symbol == 18) {
                    int extra = reader.readLsb(7);
                    if (extra < 0) fail("unexpected-eof");
                    count = repeat(lengths, count, 0, extra + 11);
                } else {
                    fail("unexpected-eof");
                }
            }

            int[] literalLengths = Arrays.copyOfRange(lengths, 0, literalCount);
            int[] distanceLengths = Arrays.copyOfRange(lengths, literalCount, total);
            if (literalLengths[256] == 0) fail("incomplete-literal-length-tree");
            return new Tables(
                buildHuffman(literalLengths, Completeness.LITERAL_LENGTH),
                buildHuffman(distanceLengths, Completeness.DISTANCE));
        }

        private static int repeat(int[] target, int start, int value, int count)
                throws RawInflateException {
            if (count > target.length - start) fail("repeat-overrun");
            Arrays.fill(target, start, start + count, value);
            return start + count;
        }

        @SuppressWarnings("unchecked")
        private static HuffmanTable buildHuffman(int[] lengths, Completeness completeness)
                throws RawInflateException {
            int[] counts = new int[16];
            for (int length : lengths) {
                if (length > 15) fail("huffman-oversubscribed");
                if (length > 0) counts[length]++;
            }
            int left = 1;
            for (int length = 1; length <= 15; length++) {
                left = left * 2 - counts[length];
                if (left < 0) fail("huffman-oversubscribed");
            }
            int symbolCount = Arrays.stream(counts).sum();
            if (left != 0) {
                switch (completeness) {
                    case CODE_LENGTH -> fail("incomplete-code-length-tree");
                    case LITERAL_LENGTH -> fail("incomplete-literal-length-tree");
                    case DISTANCE -> {
                        if (symbolCount != 0 && !(symbolCount == 1 && counts[1] == 1))
                            fail("incomplete-distance-tree");
                    }
                }
            }

            int[] nextCode = new int[16];
            int code = 0;
            for (int length = 1; length <= 15; length++) {
                code = (code + counts[length - 1]) << 1;
                nextCode[length] = code;
            }
            Map<Integer, Integer>[] tables = new Map[16];
            for (int index = 0; index < tables.length; index++) tables[index] = new HashMap<>();
            int maximumLength = 0;
            for (int symbol = 0; symbol < lengths.length; symbol++) {
                int length = lengths[symbol];
                if (length == 0) continue;
                tables[length].put(nextCode[length]++, symbol);
                maximumLength = Math.max(maximumLength, length);
            }
            return new HuffmanTable(tables, maximumLength);
        }

        private static void decodeCompressed(
                Zip.BitReader reader,
                OutputBuffer output,
                HuffmanTable literalLength,
                HuffmanTable distance) throws RawInflateException {
            while (true) {
                int symbol = literalLength.decode(reader);
                if (symbol >= 0 && symbol <= 255) {
                    output.add(symbol);
                } else if (symbol == 256) {
                    return;
                } else if (symbol >= 257 && symbol <= 285) {
                    int[] lengthRow = Zip.DeflateTable.LENGTH[symbol - 257];
                    int extraLength = reader.readLsb(lengthRow[1]);
                    if (extraLength < 0) fail("unexpected-eof");
                    int length = lengthRow[0] + extraLength;
                    int distanceSymbol = distance.decode(reader);
                    if (distanceSymbol >= 30) fail("reserved-distance-symbol");
                    int[] distanceRow = Zip.DeflateTable.DIST[distanceSymbol];
                    int extraDistance = reader.readLsb(distanceRow[1]);
                    if (extraDistance < 0) fail("unexpected-eof");
                    output.copy(distanceRow[0] + extraDistance, length);
                } else {
                    fail("invalid-literal-length-symbol");
                }
            }
        }

        private static void fail(String code) throws RawInflateException {
            throw new RawInflateException(code);
        }

        private static <T> T failResult(String code) throws RawInflateException {
            throw new RawInflateException(code);
        }
    }
}
