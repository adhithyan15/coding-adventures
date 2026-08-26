package com.codingadventures.deflate;

import com.codingadventures.lzss.LZSS;
import java.io.ByteArrayOutputStream;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.List;
import java.util.zip.DataFormatException;
import java.util.zip.Inflater;

/** Raw RFC 1951 DEFLATE streams with bounded, strict decompression. */
public final class Deflate {
    public static final int DEFAULT_MAX_OUTPUT = 256 * 1024 * 1024;

    private static final int[] LENGTH_BASE = {
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31,
        35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258
    };
    private static final int[] LENGTH_EXTRA = {
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
        3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0
    };
    private static final int[] DISTANCE_BASE = {
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129,
        193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097,
        6145, 8193, 12289, 16385, 24577
    };
    private static final int[] DISTANCE_EXTRA = {
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6,
        6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13
    };
    private static final int[] CL_PERMUTATION = {
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15
    };

    private Deflate() {}

    public static byte[] compress(byte[] data) {
        List<LZSS.Token> tokens = LZSS.encode(data, 32_768, 255, 3);
        long fixedBits = fixedBlockBits(tokens);
        DynamicPlan dynamic = planDynamic(tokens);
        BitWriter writer = new BitWriter();
        if (dynamic.totalBits() < fixedBits) {
            emitDynamicBlock(writer, tokens, dynamic);
        } else {
            emitFixedBlock(writer, tokens);
        }
        return writer.finish();
    }

    public static byte[] decompress(byte[] data) {
        return inflate(data);
    }

    public static byte[] inflate(byte[] data) {
        return inflate(data, DEFAULT_MAX_OUTPUT);
    }

    public static byte[] inflate(byte[] data, int maxOutput) {
        if (maxOutput < 0) {
            throw new IllegalArgumentException("maximum output must be non-negative");
        }
        Inflater inflater = new Inflater(true);
        inflater.setInput(data);
        ByteArrayOutputStream output = new ByteArrayOutputStream(Math.min(maxOutput, 8192));
        byte[] buffer = new byte[8192];
        try {
            while (!inflater.finished()) {
                int count = inflater.inflate(buffer);
                if (count > maxOutput - output.size()) {
                    throw new IllegalArgumentException("DEFLATE output exceeds configured limit");
                }
                output.write(buffer, 0, count);
                if (count == 0 && !inflater.finished()) {
                    if (inflater.needsInput()) {
                        throw new IllegalArgumentException("truncated DEFLATE stream");
                    }
                    throw new IllegalArgumentException("malformed DEFLATE stream");
                }
            }
            if (inflater.getRemaining() != 0) {
                throw new IllegalArgumentException("trailing bytes after DEFLATE stream");
            }
            return output.toByteArray();
        } catch (DataFormatException exception) {
            throw new IllegalArgumentException("malformed DEFLATE stream", exception);
        } finally {
            inflater.end();
        }
    }

    private static void emitFixedBlock(BitWriter writer, List<LZSS.Token> tokens) {
        writer.writeBits(1, 1);
        writer.writeBits(1, 2);
        for (LZSS.Token token : tokens) {
            if (token instanceof LZSS.Literal literal) {
                writeFixedSymbol(writer, literal.value());
            } else if (token instanceof LZSS.Match match) {
                writeLength(writer, match.length());
                writeDistance(writer, match.offset());
            }
        }
        writeFixedSymbol(writer, 256);
    }

    private static void writeLength(BitWriter writer, int length) {
        for (int index = 0; index < LENGTH_BASE.length; index++) {
            int base = LENGTH_BASE[index];
            int extraBits = LENGTH_EXTRA[index];
            int maximum = base + ((1 << extraBits) - 1);
            if (length >= base && length <= maximum) {
                writeFixedSymbol(writer, 257 + index);
                writer.writeBits(length - base, extraBits);
                return;
            }
        }
        throw new IllegalArgumentException("LZSS match length cannot be represented by DEFLATE");
    }

    private static void writeDistance(BitWriter writer, int distance) {
        for (int index = 0; index < DISTANCE_BASE.length; index++) {
            int base = DISTANCE_BASE[index];
            int extraBits = DISTANCE_EXTRA[index];
            int maximum = base + ((1 << extraBits) - 1);
            if (distance >= base && distance <= maximum) {
                writer.writeBits(reverseBits(index, 5), 5);
                writer.writeBits(distance - base, extraBits);
                return;
            }
        }
        throw new IllegalArgumentException("LZSS match distance cannot be represented by DEFLATE");
    }

    private static void writeFixedSymbol(BitWriter writer, int symbol) {
        int code;
        int bitCount;
        if (symbol <= 143) {
            code = 0x30 + symbol;
            bitCount = 8;
        } else if (symbol <= 255) {
            code = 0x190 + symbol - 144;
            bitCount = 9;
        } else if (symbol <= 279) {
            code = symbol - 256;
            bitCount = 7;
        } else if (symbol <= 287) {
            code = 0xc0 + symbol - 280;
            bitCount = 8;
        } else {
            throw new IllegalArgumentException("invalid fixed Huffman symbol");
        }
        writer.writeBits(reverseBits(code, bitCount), bitCount);
    }

    private static int reverseBits(int value, int bitCount) {
        int result = 0;
        for (int index = 0; index < bitCount; index++) {
            result = (result << 1) | ((value >>> index) & 1);
        }
        return result;
    }

    static long[] candidateBitCosts(byte[] data) {
        List<LZSS.Token> tokens = LZSS.encode(data, 32_768, 255, 3);
        return new long[] {fixedBlockBits(tokens), planDynamic(tokens).totalBits()};
    }

    private static int lengthIndex(int length) {
        for (int index = 0; index < LENGTH_BASE.length; index++) {
            int maximum = LENGTH_BASE[index] + ((1 << LENGTH_EXTRA[index]) - 1);
            if (length >= LENGTH_BASE[index] && length <= maximum) {
                return index;
            }
        }
        throw new IllegalArgumentException("LZSS match length cannot be represented by DEFLATE");
    }

    private static int distanceIndex(int distance) {
        for (int index = 0; index < DISTANCE_BASE.length; index++) {
            int maximum = DISTANCE_BASE[index] + ((1 << DISTANCE_EXTRA[index]) - 1);
            if (distance >= DISTANCE_BASE[index] && distance <= maximum) {
                return index;
            }
        }
        throw new IllegalArgumentException("LZSS match distance cannot be represented by DEFLATE");
    }

    private static int fixedSymbolBits(int symbol) {
        if (symbol <= 143) {
            return 8;
        }
        if (symbol <= 255) {
            return 9;
        }
        if (symbol <= 279) {
            return 7;
        }
        if (symbol <= 287) {
            return 8;
        }
        throw new IllegalArgumentException("invalid fixed Huffman symbol");
    }

    private static long fixedBlockBits(List<LZSS.Token> tokens) {
        long bits = 3;
        for (LZSS.Token token : tokens) {
            if (token instanceof LZSS.Literal literal) {
                bits += fixedSymbolBits(literal.value());
            } else if (token instanceof LZSS.Match match) {
                int lengthIndex = lengthIndex(match.length());
                int distanceIndex = distanceIndex(match.offset());
                bits += fixedSymbolBits(257 + lengthIndex) + LENGTH_EXTRA[lengthIndex];
                bits += 5L + DISTANCE_EXTRA[distanceIndex];
            }
        }
        return bits + fixedSymbolBits(256);
    }

    private static int[] lengthLimitedHuffman(long[] frequencies, int maxLength) {
        int[] lengths = new int[frequencies.length];
        List<Integer> present = new ArrayList<>();
        for (int symbol = 0; symbol < frequencies.length; symbol++) {
            if (frequencies[symbol] > 0) {
                present.add(symbol);
            }
        }
        if (present.isEmpty()) {
            return lengths;
        }
        if (present.size() == 1) {
            lengths[present.get(0)] = 1;
            return lengths;
        }
        if (present.size() > (1 << maxLength)) {
            throw new IllegalArgumentException("alphabet exceeds Huffman length limit");
        }

        List<PackageItem> originals = new ArrayList<>();
        for (int index = 0; index < present.size(); index++) {
            originals.add(new PackageItem(frequencies[present.get(index)], List.of(index)));
        }
        originals.sort(Comparator.comparingLong(PackageItem::weight)
            .thenComparingInt(item -> item.covers().get(0)));
        List<PackageItem> list = new ArrayList<>(originals);
        for (int level = 1; level < maxLength; level++) {
            List<PackageItem> packaged = new ArrayList<>(list.size() / 2);
            for (int index = 0; index + 1 < list.size(); index += 2) {
                PackageItem left = list.get(index);
                PackageItem right = list.get(index + 1);
                List<Integer> covers = new ArrayList<>(left.covers());
                covers.addAll(right.covers());
                packaged.add(new PackageItem(left.weight() + right.weight(), List.copyOf(covers)));
            }
            List<PackageItem> merged = new ArrayList<>(originals.size() + packaged.size());
            int originalIndex = 0;
            int packageIndex = 0;
            while (originalIndex < originals.size() && packageIndex < packaged.size()) {
                if (originals.get(originalIndex).weight() <= packaged.get(packageIndex).weight()) {
                    merged.add(originals.get(originalIndex++));
                } else {
                    merged.add(packaged.get(packageIndex++));
                }
            }
            merged.addAll(originals.subList(originalIndex, originals.size()));
            merged.addAll(packaged.subList(packageIndex, packaged.size()));
            list = merged;
        }

        int[] depths = new int[present.size()];
        int take = 2 * present.size() - 2;
        if (list.size() < take) {
            throw new IllegalStateException("package-merge produced an incomplete final list");
        }
        for (int index = 0; index < take; index++) {
            for (int covered : list.get(index).covers()) {
                depths[covered]++;
            }
        }
        long kraft = 0;
        long limit = 1L << maxLength;
        for (int index = 0; index < present.size(); index++) {
            int depth = depths[index];
            if (depth < 1 || depth > maxLength) {
                throw new IllegalStateException("package-merge produced an invalid code length");
            }
            lengths[present.get(index)] = depth;
            kraft += 1L << (maxLength - depth);
        }
        if (kraft > limit) {
            throw new IllegalStateException("package-merge violated Kraft's inequality");
        }
        return lengths;
    }

    private static HuffmanCode[] canonicalCodes(int[] lengths) {
        HuffmanCode[] codes = new HuffmanCode[lengths.length];
        Arrays.fill(codes, new HuffmanCode(0, 0));
        int maxLength = Arrays.stream(lengths).max().orElse(0);
        if (maxLength == 0) {
            return codes;
        }
        int[] counts = new int[maxLength + 1];
        for (int length : lengths) {
            if (length > 0) {
                counts[length]++;
            }
        }
        int[] nextCode = new int[maxLength + 1];
        int code = 0;
        for (int bits = 1; bits <= maxLength; bits++) {
            code = (code + counts[bits - 1]) << 1;
            nextCode[bits] = code;
        }
        for (int symbol = 0; symbol < lengths.length; symbol++) {
            int length = lengths[symbol];
            if (length > 0) {
                codes[symbol] = new HuffmanCode(nextCode[length]++, length);
            }
        }
        return codes;
    }

    private static List<CodeLengthItem> runLengthEncode(int[] lengths) {
        List<CodeLengthItem> items = new ArrayList<>();
        int index = 0;
        while (index < lengths.length) {
            int current = lengths[index];
            int run = 1;
            while (index + run < lengths.length && lengths[index + run] == current) {
                run++;
            }
            int remaining = run;
            if (current == 0) {
                while (remaining >= 11) {
                    int count = Math.min(remaining, 138);
                    items.add(new CodeLengthItem(18, 7, count - 11));
                    remaining -= count;
                }
                while (remaining >= 3) {
                    int count = Math.min(remaining, 10);
                    items.add(new CodeLengthItem(17, 3, count - 3));
                    remaining -= count;
                }
                while (remaining-- > 0) {
                    items.add(new CodeLengthItem(0, 0, 0));
                }
            } else {
                items.add(new CodeLengthItem(current, 0, 0));
                remaining--;
                while (remaining >= 3) {
                    int count = Math.min(remaining, 6);
                    items.add(new CodeLengthItem(16, 2, count - 3));
                    remaining -= count;
                }
                while (remaining-- > 0) {
                    items.add(new CodeLengthItem(current, 0, 0));
                }
            }
            index += run;
        }
        return items;
    }

    private static DynamicPlan planDynamic(List<LZSS.Token> tokens) {
        long[] literalLengthFrequencies = new long[286];
        long[] distanceFrequencies = new long[30];
        literalLengthFrequencies[256] = 1;
        for (LZSS.Token token : tokens) {
            if (token instanceof LZSS.Literal literal) {
                literalLengthFrequencies[literal.value()]++;
            } else if (token instanceof LZSS.Match match) {
                literalLengthFrequencies[257 + lengthIndex(match.length())]++;
                distanceFrequencies[distanceIndex(match.offset())]++;
            }
        }
        int[] literalLengthFull = lengthLimitedHuffman(literalLengthFrequencies, 15);
        int[] distanceFull = lengthLimitedHuffman(distanceFrequencies, 15);
        if (Arrays.stream(distanceFull).noneMatch(length -> length > 0)) {
            distanceFull[0] = 1;
        }

        int literalLengthCount = 286;
        while (literalLengthCount > 257 && literalLengthFull[literalLengthCount - 1] == 0) {
            literalLengthCount--;
        }
        int distanceCount = 30;
        while (distanceCount > 1 && distanceFull[distanceCount - 1] == 0) {
            distanceCount--;
        }
        int[] literalLengths = Arrays.copyOf(literalLengthFull, literalLengthCount);
        int[] distanceLengths = Arrays.copyOf(distanceFull, distanceCount);
        HuffmanCode[] literalCodes = canonicalCodes(literalLengthFull);
        HuffmanCode[] distanceCodes = canonicalCodes(distanceFull);

        int[] combined = Arrays.copyOf(literalLengths, literalLengths.length + distanceLengths.length);
        System.arraycopy(distanceLengths, 0, combined, literalLengths.length, distanceLengths.length);
        List<CodeLengthItem> runLengths = runLengthEncode(combined);
        long[] codeLengthFrequencies = new long[19];
        for (CodeLengthItem item : runLengths) {
            codeLengthFrequencies[item.symbol()]++;
        }
        int[] codeLengthLengths = lengthLimitedHuffman(codeLengthFrequencies, 7);
        HuffmanCode[] codeLengthCodes = canonicalCodes(codeLengthLengths);
        int codeLengthCount = 19;
        while (codeLengthCount > 4 && codeLengthLengths[CL_PERMUTATION[codeLengthCount - 1]] == 0) {
            codeLengthCount--;
        }

        long totalBits = 3 + 5 + 5 + 4 + 3L * codeLengthCount;
        for (CodeLengthItem item : runLengths) {
            totalBits += codeLengthLengths[item.symbol()] + item.extraBits();
        }
        for (LZSS.Token token : tokens) {
            if (token instanceof LZSS.Literal literal) {
                totalBits += literalLengthFull[literal.value()];
            } else if (token instanceof LZSS.Match match) {
                int lengthIndex = lengthIndex(match.length());
                int distanceIndex = distanceIndex(match.offset());
                totalBits += literalLengthFull[257 + lengthIndex] + LENGTH_EXTRA[lengthIndex];
                totalBits += distanceFull[distanceIndex] + DISTANCE_EXTRA[distanceIndex];
            }
        }
        totalBits += literalLengthFull[256];
        return new DynamicPlan(
            literalLengths,
            distanceLengths,
            literalCodes,
            distanceCodes,
            codeLengthLengths,
            codeLengthCodes,
            codeLengthCount,
            List.copyOf(runLengths),
            totalBits
        );
    }

    private static void emitDynamicBlock(
        BitWriter writer, List<LZSS.Token> tokens, DynamicPlan plan
    ) {
        writer.writeBits(1, 1);
        writer.writeBits(2, 2);
        writer.writeBits(plan.literalLengths().length - 257, 5);
        writer.writeBits(plan.distanceLengths().length - 1, 5);
        writer.writeBits(plan.codeLengthCount() - 4, 4);
        for (int index = 0; index < plan.codeLengthCount(); index++) {
            writer.writeBits(plan.codeLengthLengths()[CL_PERMUTATION[index]], 3);
        }
        for (CodeLengthItem item : plan.runLengths()) {
            writer.writeHuffman(plan.codeLengthCodes()[item.symbol()]);
            writer.writeBits(item.extraValue(), item.extraBits());
        }
        for (LZSS.Token token : tokens) {
            if (token instanceof LZSS.Literal literal) {
                writer.writeHuffman(plan.literalCodes()[literal.value()]);
            } else if (token instanceof LZSS.Match match) {
                int lengthIndex = lengthIndex(match.length());
                writer.writeHuffman(plan.literalCodes()[257 + lengthIndex]);
                writer.writeBits(match.length() - LENGTH_BASE[lengthIndex], LENGTH_EXTRA[lengthIndex]);
                int distanceIndex = distanceIndex(match.offset());
                writer.writeHuffman(plan.distanceCodes()[distanceIndex]);
                writer.writeBits(
                    match.offset() - DISTANCE_BASE[distanceIndex], DISTANCE_EXTRA[distanceIndex]
                );
            }
        }
        writer.writeHuffman(plan.literalCodes()[256]);
    }

    private record PackageItem(long weight, List<Integer> covers) {}

    private record HuffmanCode(int code, int bits) {}

    private record CodeLengthItem(int symbol, int extraBits, int extraValue) {}

    private record DynamicPlan(
        int[] literalLengths,
        int[] distanceLengths,
        HuffmanCode[] literalCodes,
        HuffmanCode[] distanceCodes,
        int[] codeLengthLengths,
        HuffmanCode[] codeLengthCodes,
        int codeLengthCount,
        List<CodeLengthItem> runLengths,
        long totalBits
    ) {}

    private static final class BitWriter {
        private final ByteArrayOutputStream output = new ByteArrayOutputStream();
        private int currentByte;
        private int bitOffset;

        private void writeBits(int value, int count) {
            for (int index = 0; index < count; index++) {
                currentByte |= ((value >>> index) & 1) << bitOffset;
                bitOffset++;
                if (bitOffset == 8) {
                    output.write(currentByte);
                    currentByte = 0;
                    bitOffset = 0;
                }
            }
        }

        private void writeHuffman(HuffmanCode code) {
            writeBits(reverseBits(code.code(), code.bits()), code.bits());
        }

        private byte[] finish() {
            if (bitOffset != 0) {
                output.write(currentByte);
            }
            return output.toByteArray();
        }
    }
}
