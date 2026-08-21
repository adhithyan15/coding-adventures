package com.codingadventures.imagecodecpng;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.codingadventures.pixelcontainer.ImageCodec;
import com.codingadventures.pixelcontainer.PixelContainer;
import com.codingadventures.zip.RawRfc1951;
import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.List;
import org.junit.jupiter.api.Test;

final class PngTest {
    @Test
    void codecImplementsPixelContainerContract() {
        ImageCodec codec = new PngCodec();
        assertEquals("image/png", codec.mimeType());

        PixelContainer pixels = new PixelContainer(1, 1, new byte[] {1, 2, 3, 4});
        assertArrayEquals(pixels.data, codec.decode(codec.encode(pixels)).data);

        PngCodec limited = new PngCodec(1.0);
        assertArrayEquals(pixels.data, limited.decode(limited.encode(pixels)).data);
    }

    @Test
    void callerPixelLimitIsValidatedWithoutCoercion() {
        List<Double> invalid = List.of(
            0.0,
            -1.0,
            1.5,
            (double) Png.DEFAULT_MAX_PIXELS + 1.0,
            Double.NaN,
            Double.POSITIVE_INFINITY,
            Double.NEGATIVE_INFINITY
        );
        for (double value : invalid) {
            requireCode("invalid-max-pixels", () -> new PngCodec(value));
            requireCode("invalid-max-pixels", () -> Png.decodePng(new byte[0], value));
        }
    }

    @Test
    void encoderValidatesExplicitPixelContainerStateBeforeAllocating() {
        requireCode("invalid-image-dimensions", () -> Png.encodePng(null));
        requireCode("invalid-image-dimensions", () ->
            Png.encodePng(new PixelContainer(0, 1, new byte[0])));
        requireCode("invalid-image-dimensions", () ->
            Png.encodePng(new PixelContainer(Png.MAX_DIMENSION + 1, 1, new byte[0])));
        requireCode("invalid-image-dimensions", () ->
            Png.encodePng(new PixelContainer(8192, 4097, new byte[0])));
        requireCode("invalid-pixel-data-length", () ->
            Png.encodePng(new PixelContainer(1, 1, new byte[] {1, 2, 3})));
    }

    @Test
    void errorTaxonomyIsImmutableAndPayloadBlind() {
        assertEquals(29, Png.ERROR_CODES.size());
        assertThrows(UnsupportedOperationException.class, () -> Png.ERROR_CODES.set(0, "changed"));
        PngError error = new PngError("invalid-filter");
        assertEquals("invalid-filter", error.code());
        assertEquals("invalid-filter", error.getMessage());
    }

    @Test
    void apngRefusalPreservesCrcAndFirstChunkPrecedence() {
        byte[] encoded = Png.encodePng(new PixelContainer(1, 1));
        byte[] valid = chunk("acTL", new byte[0]);
        requireCode("unsupported-feature", () -> Png.decodePng(insert(encoded, 33, valid)));

        byte[] corrupt = valid.clone();
        corrupt[corrupt.length - 1] ^= 1;
        requireCode("chunk-crc-mismatch", () -> Png.decodePng(insert(encoded, 33, corrupt)));
        requireCode("chunk-before-ihdr", () -> Png.decodePng(insert(encoded, 8, valid)));
    }

    @Test
    void adlerMatchesPublishedBoundaryVector() {
        assertEquals(0x11e60398L, Png.adler32("Wikipedia".getBytes(StandardCharsets.US_ASCII)));
        byte[] boundary = new byte[5553];
        for (int index = 0; index < boundary.length; index++) {
            boundary[index] = (byte) index;
        }
        assertEquals(0x2ccab2efL, Png.adler32(boundary));
    }

    private static byte[] chunk(String type, byte[] payload) {
        byte[] typeBytes = type.getBytes(StandardCharsets.US_ASCII);
        long crc = RawRfc1951.crc32(typeBytes);
        crc = RawRfc1951.crc32(payload, crc);
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        writeU32(out, payload.length);
        out.writeBytes(typeBytes);
        out.writeBytes(payload);
        writeU32(out, crc);
        return out.toByteArray();
    }

    private static byte[] insert(byte[] original, int offset, byte[] inserted) {
        ByteArrayOutputStream out = new ByteArrayOutputStream(original.length + inserted.length);
        out.write(original, 0, offset);
        out.writeBytes(inserted);
        out.write(original, offset, original.length - offset);
        return out.toByteArray();
    }

    private static void writeU32(ByteArrayOutputStream out, long value) {
        out.write((byte) (value >>> 24));
        out.write((byte) (value >>> 16));
        out.write((byte) (value >>> 8));
        out.write((byte) value);
    }

    private static void requireCode(String expected, Runnable action) {
        PngError error = assertThrows(PngError.class, action::run);
        assertEquals(expected, error.code());
        assertEquals(expected, error.getMessage());
    }
}
