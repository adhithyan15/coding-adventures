package com.codingadventures.pixelcontainer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Tests cover container construction, the full read/write surface, edge
 * cases (out-of-bounds, zero-size, large values), and the {@link ImageCodec}
 * interface contract via a stub implementation.
 */
class PixelContainerTest {

    @Test
    void constructorZeroesData() {
        PixelContainer c = new PixelContainer(3, 2);
        assertEquals(3, c.width);
        assertEquals(2, c.height);
        assertEquals(3 * 2 * 4, c.data.length);
        for (byte b : c.data) assertEquals(0, b);
    }

    @Test
    void constructorWithDataWrapsArray() {
        byte[] raw = new byte[16];
        raw[0] = 1; raw[1] = 2; raw[2] = 3; raw[3] = 4;
        PixelContainer c = new PixelContainer(2, 2, raw);
        assertSame(raw, c.data);
        assertEquals(2, c.width);
        assertEquals(2, c.height);
    }

    @Test
    void factoryCreatesTransparentBlack() {
        PixelContainer c = PixelOps.create(4, 4);
        assertEquals(64, c.data.length);
        int[] px = PixelOps.pixelAt(c, 0, 0);
        assertArrayEquals(new int[]{0, 0, 0, 0}, px);
    }

    @Test
    void setPixelThenReadRoundTrip() {
        PixelContainer c = PixelOps.create(2, 2);
        PixelOps.setPixel(c, 1, 1, 10, 20, 30, 40);
        assertArrayEquals(new int[]{10, 20, 30, 40}, PixelOps.pixelAt(c, 1, 1));
    }

    @Test
    void setPixelMasksChannelValues() {
        PixelContainer c = PixelOps.create(1, 1);
        PixelOps.setPixel(c, 0, 0, 0x1FF, 0x2AA, 0x355, 0xFFF);
        assertArrayEquals(new int[]{0xFF, 0xAA, 0x55, 0xFF}, PixelOps.pixelAt(c, 0, 0));
    }

    @Test
    void setPixelHandlesFullUnsignedRange() {
        PixelContainer c = PixelOps.create(1, 1);
        PixelOps.setPixel(c, 0, 0, 255, 255, 255, 255);
        assertArrayEquals(new int[]{255, 255, 255, 255}, PixelOps.pixelAt(c, 0, 0));
    }

    @Test
    void pixelAtOutOfBoundsNegativeX() {
        PixelContainer c = PixelOps.create(2, 2);
        assertArrayEquals(new int[]{0, 0, 0, 0}, PixelOps.pixelAt(c, -1, 0));
    }

    @Test
    void pixelAtOutOfBoundsNegativeY() {
        PixelContainer c = PixelOps.create(2, 2);
        assertArrayEquals(new int[]{0, 0, 0, 0}, PixelOps.pixelAt(c, 0, -1));
    }

    @Test
    void pixelAtOutOfBoundsXTooLarge() {
        PixelContainer c = PixelOps.create(2, 2);
        assertArrayEquals(new int[]{0, 0, 0, 0}, PixelOps.pixelAt(c, 2, 0));
    }

    @Test
    void pixelAtOutOfBoundsYTooLarge() {
        PixelContainer c = PixelOps.create(2, 2);
        assertArrayEquals(new int[]{0, 0, 0, 0}, PixelOps.pixelAt(c, 0, 2));
    }

    @Test
    void setPixelOutOfBoundsIsNoOp() {
        PixelContainer c = PixelOps.create(2, 2);
        PixelOps.setPixel(c, -1, 0, 1, 2, 3, 4);
        PixelOps.setPixel(c, 0, -1, 1, 2, 3, 4);
        PixelOps.setPixel(c, 2, 0, 1, 2, 3, 4);
        PixelOps.setPixel(c, 0, 2, 1, 2, 3, 4);
        for (byte b : c.data) assertEquals(0, b);
    }

    @Test
    void fillPixelsFillsEveryPixel() {
        PixelContainer c = PixelOps.create(3, 3);
        PixelOps.fillPixels(c, 200, 100, 50, 255);
        for (int y = 0; y < 3; y++) {
            for (int x = 0; x < 3; x++) {
                assertArrayEquals(new int[]{200, 100, 50, 255}, PixelOps.pixelAt(c, x, y));
            }
        }
    }

    @Test
    void fillPixelsMasksToByte() {
        PixelContainer c = PixelOps.create(1, 1);
        PixelOps.fillPixels(c, 0x1FF, 0x200, 0x3AB, 0x4CD);
        assertArrayEquals(new int[]{0xFF, 0x00, 0xAB, 0xCD}, PixelOps.pixelAt(c, 0, 0));
    }

    @Test
    void zeroSizeContainer() {
        PixelContainer c = PixelOps.create(0, 0);
        assertEquals(0, c.data.length);
        assertArrayEquals(new int[]{0, 0, 0, 0}, PixelOps.pixelAt(c, 0, 0));
    }

    @Test
    void writeDoesNotBleedIntoNeighbours() {
        PixelContainer c = PixelOps.create(3, 1);
        PixelOps.setPixel(c, 1, 0, 100, 150, 200, 250);
        assertArrayEquals(new int[]{0, 0, 0, 0},       PixelOps.pixelAt(c, 0, 0));
        assertArrayEquals(new int[]{100, 150, 200, 250}, PixelOps.pixelAt(c, 1, 0));
        assertArrayEquals(new int[]{0, 0, 0, 0},       PixelOps.pixelAt(c, 2, 0));
    }

    @Test
    void rowMajorOrdering() {
        PixelContainer c = PixelOps.create(2, 2);
        // byte indices 0..3 = pixel (0,0), 4..7 = (1,0), 8..11 = (0,1), 12..15 = (1,1)
        c.data[8] = (byte) 255;
        assertArrayEquals(new int[]{255, 0, 0, 0}, PixelOps.pixelAt(c, 0, 1));
    }

    @Test
    void overwriteExistingPixel() {
        PixelContainer c = PixelOps.create(1, 1);
        PixelOps.setPixel(c, 0, 0, 1, 2, 3, 4);
        PixelOps.setPixel(c, 0, 0, 10, 20, 30, 40);
        assertArrayEquals(new int[]{10, 20, 30, 40}, PixelOps.pixelAt(c, 0, 0));
    }

    @Test
    void fillOverwritesPriorWrites() {
        PixelContainer c = PixelOps.create(2, 2);
        PixelOps.setPixel(c, 0, 0, 1, 2, 3, 4);
        PixelOps.fillPixels(c, 9, 9, 9, 9);
        assertArrayEquals(new int[]{9, 9, 9, 9}, PixelOps.pixelAt(c, 0, 0));
    }

    @Test
    void dataLengthMatchesDimensions() {
        PixelContainer c = PixelOps.create(7, 5);
        assertEquals(7 * 5 * 4, c.data.length);
    }

    @Test
    void rectangularShapesWorkBothWays() {
        PixelContainer wide = PixelOps.create(5, 1);
        PixelContainer tall = PixelOps.create(1, 5);
        PixelOps.setPixel(wide, 4, 0, 1, 2, 3, 4);
        PixelOps.setPixel(tall, 0, 4, 1, 2, 3, 4);
        assertArrayEquals(new int[]{1, 2, 3, 4}, PixelOps.pixelAt(wide, 4, 0));
        assertArrayEquals(new int[]{1, 2, 3, 4}, PixelOps.pixelAt(tall, 0, 4));
    }

    @Test
    void imageCodecContract() {
        ImageCodec codec = new ImageCodec() {
            public String mimeType() { return "image/test"; }
            public byte[] encode(PixelContainer c) { return c.data.clone(); }
            public PixelContainer decode(byte[] data) {
                return new PixelContainer(1, data.length / 4, data);
            }
        };
        PixelContainer c = PixelOps.create(1, 2);
        PixelOps.setPixel(c, 0, 1, 10, 20, 30, 40);
        byte[] bytes = codec.encode(c);
        PixelContainer round = codec.decode(bytes);
        assertEquals("image/test", codec.mimeType());
        assertArrayEquals(new int[]{10, 20, 30, 40}, PixelOps.pixelAt(round, 0, 1));
    }

    @Test
    void fillPixelsOnZeroSize() {
        PixelContainer c = PixelOps.create(0, 5);
        // Width*height == 0 → no bytes to fill; should not throw.
        PixelOps.fillPixels(c, 1, 2, 3, 4);
        assertEquals(0, c.data.length);
    }
}
