package com.codingadventures.imagecodecpng;

import com.codingadventures.pixelcontainer.PixelContainer;
import com.codingadventures.zip.RawRfc1951;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Objects;

/**
 * Pure in-memory IC18 PNG framing, zlib wrapping, filtering, encoding, and decoding.
 * RFC 1951 and CRC-32 are delegated to the repository ZIP package.
 */
public final class Png {
    /** Largest accepted width or height. */
    public static final int MAX_DIMENSION = 16_384;

    /** Default and hard total-pixel ceiling. */
    public static final long DEFAULT_MAX_PIXELS = 32L * 1024L * 1024L;

    /** Closed IC18 error taxonomy in normative order. */
    public static final List<String> ERROR_CODES = List.of(
        "invalid-max-pixels",
        "invalid-image-dimensions",
        "invalid-pixel-data-length",
        "file-too-short",
        "invalid-signature",
        "truncated-chunk",
        "invalid-chunk-type",
        "chunk-crc-mismatch",
        "chunk-before-ihdr",
        "duplicate-ihdr",
        "invalid-ihdr-length",
        "invalid-dimensions",
        "dimension-limit",
        "pixel-limit",
        "unsupported-feature",
        "invalid-plte",
        "invalid-trns",
        "nonconsecutive-idat",
        "invalid-iend",
        "trailing-data",
        "unknown-critical-chunk",
        "missing-required-chunk",
        "invalid-zlib-header",
        "preset-dictionary",
        "inflate-failed",
        "inflated-length-mismatch",
        "idat-cavity",
        "adler-mismatch",
        "invalid-filter"
    );

    private static final byte[] SIGNATURE = {
        (byte) 0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a
    };
    private static final long ADLER_MOD = 65_521L;

    private Png() {}

    /** Compute the RFC 1950 Adler-32 checksum. */
    public static long adler32(byte[] data) {
        Objects.requireNonNull(data, "data");
        long a = 1;
        long b = 0;
        for (int start = 0; start < data.length; start += 5552) {
            int end = Math.min(start + 5552, data.length);
            for (int index = start; index < end; index++) {
                a += data[index] & 0xff;
                b += a;
            }
            a %= ADLER_MOD;
            b %= ADLER_MOD;
        }
        return ((b << 16) | a) & 0xffff_ffffL;
    }

    /** Encode RGBA8 pixels as a bounded colour-type-6 PNG. */
    public static byte[] encodePng(PixelContainer pixels) {
        if (pixels == null || pixels.width <= 0 || pixels.height <= 0
                || pixels.width > MAX_DIMENSION || pixels.height > MAX_DIMENSION) {
            throw fail("invalid-image-dimensions");
        }
        long pixelCount = (long) pixels.width * pixels.height;
        if (pixelCount > DEFAULT_MAX_PIXELS) {
            throw fail("invalid-image-dimensions");
        }
        long expectedPixels = Math.multiplyExact(pixelCount, 4L);
        if (pixels.data == null || pixels.data.length != expectedPixels) {
            throw fail("invalid-pixel-data-length");
        }

        ByteArrayOutputStream output = new ByteArrayOutputStream();
        output.writeBytes(SIGNATURE);
        byte[] ihdr = new byte[13];
        writeU32(ihdr, 0, pixels.width);
        writeU32(ihdr, 4, pixels.height);
        ihdr[8] = 8;
        ihdr[9] = 6;
        appendChunk(output, "IHDR", ihdr);

        int stride = Math.multiplyExact(pixels.width, 4);
        int filteredLength = Math.toIntExact(Math.multiplyExact(
            (long) pixels.height,
            (long) stride + 1L
        ));
        byte[] filtered = new byte[filteredLength];
        byte[] prior = new byte[stride];
        byte[] scratch = new byte[stride];
        byte[] best = new byte[stride];
        for (int row = 0; row < pixels.height; row++) {
            int source = row * stride;
            int destination = row * (stride + 1);
            int filter = chooseFilter(pixels.data, source, prior, scratch, best, 4);
            filtered[destination] = (byte) filter;
            System.arraycopy(best, 0, filtered, destination + 1, stride);
            System.arraycopy(pixels.data, source, prior, 0, stride);
        }

        byte[] deflated;
        try {
            deflated = RawRfc1951.rawDeflate(filtered);
        } catch (IOException error) {
            throw new IllegalStateException("deflate-failed", error);
        }
        byte[] idat = new byte[Math.addExact(deflated.length, 6)];
        idat[0] = 0x78;
        idat[1] = (byte) 0x9c;
        System.arraycopy(deflated, 0, idat, 2, deflated.length);
        writeU32(idat, idat.length - 4, adler32(filtered));
        appendChunk(output, "IDAT", idat);
        appendChunk(output, "IEND", new byte[0]);
        return output.toByteArray();
    }

    /** Decode a PNG using the default pixel ceiling. */
    public static PixelContainer decodePng(byte[] data) {
        return decodePngWithLimit(data, DEFAULT_MAX_PIXELS);
    }

    /** Decode a PNG using a caller-lowered pixel ceiling. */
    public static PixelContainer decodePng(byte[] data, double maxPixels) {
        return decodePngWithLimit(data, validateMaxPixels(maxPixels));
    }

    static long validateMaxPixels(Double value) {
        if (value == null) {
            return DEFAULT_MAX_PIXELS;
        }
        if (!Double.isFinite(value) || Math.rint(value) != value
                || value <= 0 || value > DEFAULT_MAX_PIXELS) {
            throw fail("invalid-max-pixels");
        }
        return value.longValue();
    }

    static PixelContainer decodePngWithLimit(byte[] data, long maxPixels) {
        Objects.requireNonNull(data, "data");
        if (data.length < SIGNATURE.length) {
            throw fail("file-too-short");
        }
        if (!Arrays.equals(Arrays.copyOfRange(data, 0, SIGNATURE.length), SIGNATURE)) {
            throw fail("invalid-signature");
        }

        long width = 0;
        long height = 0;
        int bitDepth = 0;
        int colourType = 0;
        boolean sawIhdr = false;
        boolean sawIend = false;
        boolean sawPlte = false;
        boolean sawTrns = false;
        boolean inIdat = false;
        boolean idatEnded = false;
        Integer transparentGrey = null;
        int[] transparentRgb = null;
        List<byte[]> idatParts = new ArrayList<>();

        int position = SIGNATURE.length;
        while (position < data.length) {
            if (data.length - position < 8) {
                throw fail("truncated-chunk");
            }
            long length = readU32(data, position);
            long chunkEnd = (long) position + 12L + length;
            if (chunkEnd > data.length || length > Integer.MAX_VALUE) {
                throw fail("truncated-chunk");
            }
            int size = (int) length;
            int typeStart = position + 4;
            int dataStart = position + 8;
            int dataEnd = dataStart + size;
            byte[] typeBytes = Arrays.copyOfRange(data, typeStart, dataStart);
            if (!validChunkType(typeBytes)) {
                throw fail("invalid-chunk-type");
            }
            byte[] chunkData = Arrays.copyOfRange(data, dataStart, dataEnd);
            long checksum = RawRfc1951.crc32(typeBytes);
            checksum = RawRfc1951.crc32(chunkData, checksum);
            if (checksum != readU32(data, dataEnd)) {
                throw fail("chunk-crc-mismatch");
            }
            String type = new String(typeBytes, StandardCharsets.US_ASCII);
            if (!sawIhdr && !type.equals("IHDR")) {
                throw fail("chunk-before-ihdr");
            }

            switch (type) {
                case "IHDR" -> {
                    if (sawIhdr) {
                        throw fail("duplicate-ihdr");
                    }
                    if (size != 13) {
                        throw fail("invalid-ihdr-length");
                    }
                    width = readU32(chunkData, 0);
                    height = readU32(chunkData, 4);
                    bitDepth = chunkData[8] & 0xff;
                    colourType = chunkData[9] & 0xff;
                    if (width == 0 || height == 0) {
                        throw fail("invalid-dimensions");
                    }
                    if (width > MAX_DIMENSION || height > MAX_DIMENSION) {
                        throw fail("dimension-limit");
                    }
                    if (Math.multiplyExact(width, height) > maxPixels) {
                        throw fail("pixel-limit");
                    }
                    if ((chunkData[10] & 0xff) != 0 || (chunkData[11] & 0xff) != 0
                            || (chunkData[12] & 0xff) != 0) {
                        throw fail("unsupported-feature");
                    }
                    if (bitDepth != 8 || (colourType != 0 && colourType != 2
                            && colourType != 4 && colourType != 6)) {
                        throw fail("unsupported-feature");
                    }
                    sawIhdr = true;
                }
                case "PLTE" -> {
                    if (sawPlte || !idatParts.isEmpty() || sawTrns
                            || (colourType != 2 && colourType != 6)
                            || size < 3 || size > 768 || size % 3 != 0) {
                        throw fail("invalid-plte");
                    }
                    sawPlte = true;
                }
                case "tRNS" -> {
                    if (sawTrns || !idatParts.isEmpty()) {
                        throw fail("invalid-trns");
                    }
                    if (colourType == 0) {
                        if (size != 2 || readU16(chunkData, 0) > 255) {
                            throw fail("invalid-trns");
                        }
                        transparentGrey = readU16(chunkData, 0);
                    } else if (colourType == 2) {
                        if (size != 6) {
                            throw fail("invalid-trns");
                        }
                        transparentRgb = new int[3];
                        for (int index = 0; index < 3; index++) {
                            int sample = readU16(chunkData, index * 2);
                            if (sample > 255) {
                                throw fail("invalid-trns");
                            }
                            transparentRgb[index] = sample;
                        }
                    } else {
                        throw fail("invalid-trns");
                    }
                    sawTrns = true;
                }
                case "IDAT" -> {
                    if (idatEnded) {
                        throw fail("nonconsecutive-idat");
                    }
                    idatParts.add(chunkData);
                    inIdat = true;
                }
                case "IEND" -> {
                    if (size != 0) {
                        throw fail("invalid-iend");
                    }
                    if (chunkEnd != data.length) {
                        throw fail("trailing-data");
                    }
                    sawIend = true;
                    position = (int) chunkEnd;
                    continue;
                }
                case "acTL", "fcTL", "fdAT" -> throw fail("unsupported-feature");
                default -> {
                    if ((typeBytes[0] & 0x20) == 0) {
                        throw fail("unknown-critical-chunk");
                    }
                }
            }

            if (!type.equals("IDAT") && inIdat) {
                inIdat = false;
                idatEnded = true;
            }
            position = (int) chunkEnd;
        }

        if (!sawIhdr || !sawIend || idatParts.isEmpty()) {
            throw fail("missing-required-chunk");
        }
        long zlibLength = 0;
        for (byte[] part : idatParts) {
            zlibLength = Math.addExact(zlibLength, part.length);
        }
        if (zlibLength > data.length || zlibLength > Integer.MAX_VALUE) {
            throw fail("truncated-chunk");
        }
        byte[] zlib = new byte[(int) zlibLength];
        int zlibOffset = 0;
        for (byte[] part : idatParts) {
            System.arraycopy(part, 0, zlib, zlibOffset, part.length);
            zlibOffset += part.length;
        }
        if (zlib.length < 6) {
            throw fail("invalid-zlib-header");
        }
        int cmf = zlib[0] & 0xff;
        int flg = zlib[1] & 0xff;
        if ((cmf & 0x0f) != 8 || (cmf >>> 4) > 7 || ((cmf << 8) | flg) % 31 != 0) {
            throw fail("invalid-zlib-header");
        }
        if ((flg & 0x20) != 0) {
            throw fail("preset-dictionary");
        }

        int channels = switch (colourType) {
            case 0 -> 1;
            case 2 -> 3;
            case 4 -> 2;
            default -> 4;
        };
        long strideLong = Math.multiplyExact(width, channels);
        long expectedLong = Math.multiplyExact(height, Math.addExact(strideLong, 1L));
        if (expectedLong > Integer.MAX_VALUE) {
            throw fail("pixel-limit");
        }
        int expected = (int) expectedLong;
        byte[] deflate = Arrays.copyOfRange(zlib, 2, zlib.length - 4);
        RawRfc1951.InflateResult inflated;
        try {
            inflated = RawRfc1951.rawInflateCounted(deflate, expected);
        } catch (RawRfc1951.RawInflateException error) {
            if (error.code().equals("output-limit-exceeded")) {
                throw fail("inflated-length-mismatch");
            }
            throw fail("inflate-failed");
        }
        if (inflated.output().length != expected) {
            throw fail("inflated-length-mismatch");
        }
        if (inflated.bytesConsumed() != deflate.length) {
            throw fail("idat-cavity");
        }
        if (adler32(inflated.output()) != readU32(zlib, zlib.length - 4)) {
            throw fail("adler-mismatch");
        }

        int stride = Math.toIntExact(strideLong);
        int rowSize = stride + 1;
        for (int row = 0; row < height; row++) {
            if ((inflated.output()[row * rowSize] & 0xff) > 4) {
                throw fail("invalid-filter");
            }
        }

        byte[] rgba = new byte[Math.toIntExact(Math.multiplyExact(width * height, 4L))];
        byte[] prior = new byte[stride];
        for (int rowIndex = 0; rowIndex < height; rowIndex++) {
            int source = rowIndex * rowSize;
            byte[] row = Arrays.copyOfRange(inflated.output(), source + 1, source + rowSize);
            undoFilter(inflated.output()[source] & 0xff, row, prior, channels);
            int destination = rowIndex * Math.toIntExact(width) * 4;
            for (int x = 0; x < width; x++) {
                int from = x * channels;
                int to = destination + x * 4;
                int first = row[from] & 0xff;
                switch (channels) {
                    case 1 -> {
                        rgba[to] = row[from];
                        rgba[to + 1] = row[from];
                        rgba[to + 2] = row[from];
                        rgba[to + 3] = (byte) (transparentGrey != null
                            && first == transparentGrey ? 0 : 255);
                    }
                    case 2 -> {
                        rgba[to] = row[from];
                        rgba[to + 1] = row[from];
                        rgba[to + 2] = row[from];
                        rgba[to + 3] = row[from + 1];
                    }
                    case 3 -> {
                        int green = row[from + 1] & 0xff;
                        int blue = row[from + 2] & 0xff;
                        rgba[to] = row[from];
                        rgba[to + 1] = row[from + 1];
                        rgba[to + 2] = row[from + 2];
                        rgba[to + 3] = (byte) (transparentRgb != null
                            && first == transparentRgb[0]
                            && green == transparentRgb[1]
                            && blue == transparentRgb[2] ? 0 : 255);
                    }
                    default -> System.arraycopy(row, from, rgba, to, 4);
                }
            }
            System.arraycopy(row, 0, prior, 0, stride);
        }
        return new PixelContainer(Math.toIntExact(width), Math.toIntExact(height), rgba);
    }

    private static int chooseFilter(
            byte[] raw,
            int rawOffset,
            byte[] prior,
            byte[] scratch,
            byte[] best,
            int bytesPerPixel) {
        int bestFilter = 0;
        int bestScore = Integer.MAX_VALUE;
        for (int filter = 0; filter <= 4; filter++) {
            applyFilter(filter, raw, rawOffset, prior, scratch, bytesPerPixel);
            int score = 0;
            for (byte value : scratch) {
                int unsigned = value & 0xff;
                score += unsigned < 128 ? unsigned : 256 - unsigned;
            }
            if (score < bestScore) {
                bestScore = score;
                bestFilter = filter;
                System.arraycopy(scratch, 0, best, 0, scratch.length);
            }
        }
        return bestFilter;
    }

    private static void applyFilter(
            int filter,
            byte[] raw,
            int rawOffset,
            byte[] prior,
            byte[] output,
            int bytesPerPixel) {
        for (int index = 0; index < output.length; index++) {
            int value = raw[rawOffset + index] & 0xff;
            int left = index >= bytesPerPixel ? raw[rawOffset + index - bytesPerPixel] & 0xff : 0;
            int above = prior[index] & 0xff;
            int aboveLeft = index >= bytesPerPixel ? prior[index - bytesPerPixel] & 0xff : 0;
            int prediction = switch (filter) {
                case 1 -> left;
                case 2 -> above;
                case 3 -> (left + above) / 2;
                case 4 -> paeth(left, above, aboveLeft);
                default -> 0;
            };
            output[index] = (byte) (value - prediction);
        }
    }

    private static void undoFilter(
            int filter,
            byte[] row,
            byte[] prior,
            int bytesPerPixel) {
        for (int index = 0; index < row.length; index++) {
            int left = index >= bytesPerPixel ? row[index - bytesPerPixel] & 0xff : 0;
            int above = prior[index] & 0xff;
            int aboveLeft = index >= bytesPerPixel ? prior[index - bytesPerPixel] & 0xff : 0;
            int prediction = switch (filter) {
                case 0 -> 0;
                case 1 -> left;
                case 2 -> above;
                case 3 -> (left + above) / 2;
                case 4 -> paeth(left, above, aboveLeft);
                default -> throw fail("invalid-filter");
            };
            row[index] = (byte) ((row[index] & 0xff) + prediction);
        }
    }

    private static int paeth(int left, int above, int aboveLeft) {
        int prediction = left + above - aboveLeft;
        int leftDistance = Math.abs(prediction - left);
        int aboveDistance = Math.abs(prediction - above);
        int diagonalDistance = Math.abs(prediction - aboveLeft);
        if (leftDistance <= aboveDistance && leftDistance <= diagonalDistance) {
            return left;
        }
        if (aboveDistance <= diagonalDistance) {
            return above;
        }
        return aboveLeft;
    }

    private static boolean validChunkType(byte[] type) {
        if (type.length != 4 || (type[2] & 0x20) != 0) {
            return false;
        }
        for (byte value : type) {
            int unsigned = value & 0xff;
            if (!((unsigned >= 'A' && unsigned <= 'Z')
                    || (unsigned >= 'a' && unsigned <= 'z'))) {
                return false;
            }
        }
        return true;
    }

    private static void appendChunk(ByteArrayOutputStream output, String type, byte[] data) {
        byte[] typeBytes = type.getBytes(StandardCharsets.US_ASCII);
        writeU32(output, data.length);
        output.writeBytes(typeBytes);
        output.writeBytes(data);
        long checksum = RawRfc1951.crc32(typeBytes);
        checksum = RawRfc1951.crc32(data, checksum);
        writeU32(output, checksum);
    }

    private static int readU16(byte[] data, int offset) {
        return ((data[offset] & 0xff) << 8) | (data[offset + 1] & 0xff);
    }

    private static long readU32(byte[] data, int offset) {
        return ((long) (data[offset] & 0xff) << 24)
            | ((long) (data[offset + 1] & 0xff) << 16)
            | ((long) (data[offset + 2] & 0xff) << 8)
            | (data[offset + 3] & 0xffL);
    }

    private static void writeU32(byte[] data, int offset, long value) {
        data[offset] = (byte) (value >>> 24);
        data[offset + 1] = (byte) (value >>> 16);
        data[offset + 2] = (byte) (value >>> 8);
        data[offset + 3] = (byte) value;
    }

    private static void writeU32(ByteArrayOutputStream output, long value) {
        output.write((byte) (value >>> 24));
        output.write((byte) (value >>> 16));
        output.write((byte) (value >>> 8));
        output.write((byte) value);
    }

    private static PngError fail(String code) {
        return new PngError(code);
    }
}
