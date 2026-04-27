package com.codingadventures.pixelcontainer

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlin.test.assertSame
import kotlin.test.assertContentEquals

class PixelContainerTest {

    @Test
    fun defaultConstructorAllocatesCorrectlySizedBuffer() {
        val c = PixelContainer(4, 3)
        assertEquals(4, c.width)
        assertEquals(3, c.height)
        assertEquals(4 * 3 * 4, c.data.size)
    }

    @Test
    fun defaultConstructorZeroesAllPixels() {
        val c = PixelContainer(5, 5)
        assertTrue(c.data.all { it == 0.toByte() })
    }

    @Test
    fun explicitBufferIsRetainedByReference() {
        val buf = ByteArray(2 * 2 * 4)
        val c = PixelContainer(2, 2, buf)
        assertSame(buf, c.data)
    }

    @Test
    fun zeroSizedContainerHasEmptyBuffer() {
        val c = PixelContainer(0, 0)
        assertEquals(0, c.data.size)
    }

    @Test
    fun oneByOneContainerHasFourBytes() {
        val c = PixelContainer(1, 1)
        assertEquals(4, c.data.size)
    }

    @Test
    fun setPixelAndReadBack() {
        val c = PixelContainer(3, 3)
        PixelOps.setPixel(c, 1, 1, 10, 20, 30, 40)
        val px = PixelOps.pixelAt(c, 1, 1)
        assertContentEquals(intArrayOf(10, 20, 30, 40), px)
    }

    @Test
    fun setPixelAllChannelsFull() {
        val c = PixelContainer(1, 1)
        PixelOps.setPixel(c, 0, 0, 255, 255, 255, 255)
        assertContentEquals(intArrayOf(255, 255, 255, 255), PixelOps.pixelAt(c, 0, 0))
    }

    @Test
    fun setPixelTruncatesHighBits() {
        val c = PixelContainer(1, 1)
        PixelOps.setPixel(c, 0, 0, 0x1FF, 0x2AA, -1, 0x100)
        val px = PixelOps.pixelAt(c, 0, 0)
        assertContentEquals(intArrayOf(255, 0xAA, 255, 0), px)
    }

    @Test
    fun pixelAtOutOfBoundsReturnsTransparentBlack() {
        val c = PixelContainer(2, 2)
        PixelOps.fillPixels(c, 100, 100, 100, 100)
        assertContentEquals(intArrayOf(0, 0, 0, 0), PixelOps.pixelAt(c, -1, 0))
        assertContentEquals(intArrayOf(0, 0, 0, 0), PixelOps.pixelAt(c, 0, -1))
        assertContentEquals(intArrayOf(0, 0, 0, 0), PixelOps.pixelAt(c, 2, 0))
        assertContentEquals(intArrayOf(0, 0, 0, 0), PixelOps.pixelAt(c, 0, 2))
        assertContentEquals(intArrayOf(0, 0, 0, 0), PixelOps.pixelAt(c, 100, 100))
    }

    @Test
    fun setPixelOutOfBoundsIsNoOp() {
        val c = PixelContainer(2, 2)
        PixelOps.fillPixels(c, 5, 6, 7, 8)
        val before = c.data.copyOf()
        PixelOps.setPixel(c, -1, 0, 99, 99, 99, 99)
        PixelOps.setPixel(c, 0, -1, 99, 99, 99, 99)
        PixelOps.setPixel(c, 2, 0, 99, 99, 99, 99)
        PixelOps.setPixel(c, 0, 2, 99, 99, 99, 99)
        assertContentEquals(before, c.data)
    }

    @Test
    fun fillPixelsOverwritesEveryPixel() {
        val c = PixelContainer(3, 4)
        PixelOps.fillPixels(c, 1, 2, 3, 4)
        for (y in 0 until 4)
            for (x in 0 until 3)
                assertContentEquals(intArrayOf(1, 2, 3, 4), PixelOps.pixelAt(c, x, y))
    }

    @Test
    fun fillPixelsHandlesZeroSizeContainer() {
        val c = PixelContainer(0, 0)
        PixelOps.fillPixels(c, 1, 2, 3, 4)
        assertEquals(0, c.data.size)
    }

    @Test
    fun rowMajorLayout() {
        val c = PixelContainer(3, 2)
        // pixel (0,1) should be at byte offset (1*3+0)*4 = 12
        PixelOps.setPixel(c, 0, 1, 11, 22, 33, 44)
        assertEquals(11, c.data[12].toInt() and 0xFF)
        assertEquals(22, c.data[13].toInt() and 0xFF)
        assertEquals(33, c.data[14].toInt() and 0xFF)
        assertEquals(44, c.data[15].toInt() and 0xFF)
    }

    @Test
    fun channelInterleaveOrderIsRGBA() {
        val c = PixelContainer(1, 1)
        PixelOps.setPixel(c, 0, 0, 0x11, 0x22, 0x33, 0x44)
        assertEquals(0x11.toByte(), c.data[0])
        assertEquals(0x22.toByte(), c.data[1])
        assertEquals(0x33.toByte(), c.data[2])
        assertEquals(0x44.toByte(), c.data[3])
    }

    @Test
    fun independentPixelWritesDoNotCorrupt() {
        val c = PixelContainer(4, 4)
        PixelOps.setPixel(c, 0, 0, 1, 2, 3, 4)
        PixelOps.setPixel(c, 3, 3, 9, 8, 7, 6)
        assertContentEquals(intArrayOf(1, 2, 3, 4), PixelOps.pixelAt(c, 0, 0))
        assertContentEquals(intArrayOf(9, 8, 7, 6), PixelOps.pixelAt(c, 3, 3))
        // all others still zero
        for (y in 0 until 4)
            for (x in 0 until 4)
                if (!((x == 0 && y == 0) || (x == 3 && y == 3)))
                    assertContentEquals(intArrayOf(0, 0, 0, 0), PixelOps.pixelAt(c, x, y))
    }

    @Test
    fun fillThenOverwriteSinglePixel() {
        val c = PixelContainer(2, 2)
        PixelOps.fillPixels(c, 10, 20, 30, 40)
        PixelOps.setPixel(c, 1, 1, 200, 150, 100, 50)
        assertContentEquals(intArrayOf(200, 150, 100, 50), PixelOps.pixelAt(c, 1, 1))
        assertContentEquals(intArrayOf(10, 20, 30, 40), PixelOps.pixelAt(c, 0, 0))
    }

    @Test
    fun explicitBufferPreservesContent() {
        val buf = ByteArray(4)
        buf[0] = 50; buf[1] = 60; buf[2] = 70; buf[3] = 80
        val c = PixelContainer(1, 1, buf)
        assertContentEquals(intArrayOf(50, 60, 70, 80), PixelOps.pixelAt(c, 0, 0))
    }

    @Test
    fun imageCodecInterfaceContractShape() {
        val codec = object : ImageCodec {
            override val mimeType = "image/test"
            override fun encode(container: PixelContainer): ByteArray = container.data.copyOf()
            override fun decode(data: ByteArray): PixelContainer =
                PixelContainer(1, data.size / 4, data.copyOf())
        }
        val src = PixelContainer(1, 2)
        PixelOps.setPixel(src, 0, 0, 1, 2, 3, 4)
        PixelOps.setPixel(src, 0, 1, 5, 6, 7, 8)
        val enc = codec.encode(src)
        val dec = codec.decode(enc)
        assertEquals("image/test", codec.mimeType)
        assertEquals(1, dec.width)
        assertEquals(2, dec.height)
        assertContentEquals(intArrayOf(1, 2, 3, 4), PixelOps.pixelAt(dec, 0, 0))
        assertContentEquals(intArrayOf(5, 6, 7, 8), PixelOps.pixelAt(dec, 0, 1))
    }

    @Test
    fun largeContainerAllocationSucceeds() {
        val c = PixelContainer(256, 256)
        assertEquals(256 * 256 * 4, c.data.size)
    }

    @Test
    fun sequentialWritesInScanOrder() {
        val c = PixelContainer(2, 2)
        var k = 0
        for (y in 0 until 2) for (x in 0 until 2)
            PixelOps.setPixel(c, x, y, k++, k++, k++, k++)
        assertEquals(0, PixelOps.pixelAt(c, 0, 0)[0])
        assertEquals(4, PixelOps.pixelAt(c, 1, 0)[0])
        assertEquals(8, PixelOps.pixelAt(c, 0, 1)[0])
        assertEquals(12, PixelOps.pixelAt(c, 1, 1)[0])
    }

    @Test
    fun nonSquareDimensions() {
        val c = PixelContainer(7, 3)
        assertEquals(7 * 3 * 4, c.data.size)
        PixelOps.setPixel(c, 6, 2, 100, 100, 100, 100)
        assertContentEquals(intArrayOf(100, 100, 100, 100), PixelOps.pixelAt(c, 6, 2))
    }

    @Test
    fun alphaChannelIndependenceFromRGB() {
        val c = PixelContainer(1, 1)
        PixelOps.setPixel(c, 0, 0, 0, 0, 0, 128)
        assertContentEquals(intArrayOf(0, 0, 0, 128), PixelOps.pixelAt(c, 0, 0))
    }

    @Test
    fun containersAreIndependent() {
        val a = PixelContainer(2, 2)
        val b = PixelContainer(2, 2)
        PixelOps.fillPixels(a, 1, 1, 1, 1)
        assertTrue(b.data.all { it == 0.toByte() })
    }

    @Test
    fun pixelOpsSingletonIsStateless() {
        // Repeated calls must behave identically.
        val c = PixelContainer(1, 1)
        PixelOps.setPixel(c, 0, 0, 10, 20, 30, 40)
        val first = PixelOps.pixelAt(c, 0, 0)
        val second = PixelOps.pixelAt(c, 0, 0)
        assertContentEquals(first, second)
    }

    @Test
    fun imageCodecImplementsMimeType() {
        val codec = object : ImageCodec {
            override val mimeType = "image/png"
            override fun encode(container: PixelContainer) = ByteArray(0)
            override fun decode(data: ByteArray) = PixelContainer(0, 0)
        }
        assertNotNull(codec.mimeType)
        assertEquals("image/png", codec.mimeType)
    }
}
