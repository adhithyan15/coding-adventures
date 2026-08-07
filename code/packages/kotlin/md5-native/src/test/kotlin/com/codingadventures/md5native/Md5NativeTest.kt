package com.codingadventures.md5native

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class Md5NativeTest {
    private fun a(s: String) = s.toByteArray(Charsets.UTF_8)

    @Test fun rfcVectors() {
        assertEquals("d41d8cd98f00b204e9800998ecf8427e", Md5Native.hexString(a("")))
        assertEquals("0cc175b9c0f1b6a831c399e269772661", Md5Native.hexString(a("a")))
        assertEquals("900150983cd24fb0d6963f7d28e17f72", Md5Native.hexString(a("abc")))
        assertEquals("f96b697d7cb7938d525a2f31aaf161d0", Md5Native.hexString(a("message digest")))
    }

    @Test fun bytes0To255() {
        val data = ByteArray(256) { it.toByte() }
        assertEquals("e2c865db4162bed963bfaa9ef6ac18f0", Md5Native.hexString(data))
    }

    @Test fun digestIs16Bytes() {
        assertEquals(16, Md5Native.sumMd5(a("")).size)
        assertEquals(16, Md5Native.sumMd5(a("hello world")).size)
    }

    @Test fun streamingMatchesOneShot() {
        Md5Native.Digest().use { h ->
            h.update(a("ab")); h.update(a("c"))
            assertContentEquals(Md5Native.sumMd5(a("abc")), h.digest())
        }
    }

    @Test fun streamingByteAtATime() {
        val data = ByteArray(100) { it.toByte() }
        Md5Native.Digest().use { h ->
            for (x in data) h.update(byteArrayOf(x))
            assertContentEquals(Md5Native.sumMd5(data), h.digest())
        }
    }

    @Test fun streamingEmptyAndNonDestructive() {
        Md5Native.Digest().use { e -> assertContentEquals(Md5Native.sumMd5(a("")), e.digest()) }
        Md5Native.Digest().use { h ->
            h.update(a("abc"))
            assertContentEquals(h.digest(), h.digest())
            h.update(a("d"))
            assertContentEquals(Md5Native.sumMd5(a("abcd")), h.digest())
        }
    }

    @Test fun copyIsIndependent() {
        Md5Native.Digest().use { h ->
            h.update(a("ab"))
            h.copy().use { h2 ->
                h2.update(a("c")); h.update(a("x"))
                assertContentEquals(Md5Native.sumMd5(a("abc")), h2.digest())
                assertContentEquals(Md5Native.sumMd5(a("abx")), h.digest())
            }
        }
    }

    @Test fun usingClosedThrows() {
        val h = Md5Native.Digest()
        h.update(a("abc"))
        h.close()
        assertFailsWith<IllegalStateException> { h.update(a("x")) }
        assertFailsWith<IllegalStateException> { h.digest() }
        h.close()
    }
}
