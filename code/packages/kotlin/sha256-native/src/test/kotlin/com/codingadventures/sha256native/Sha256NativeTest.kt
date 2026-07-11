package com.codingadventures.sha256native

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class Sha256NativeTest {
    private fun a(s: String) = s.toByteArray(Charsets.UTF_8)

    @Test fun fipsVectors() {
        assertEquals("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", Sha256Native.sha256Hex(a("")))
        assertEquals("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", Sha256Native.sha256Hex(a("abc")))
        assertEquals("248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            Sha256Native.sha256Hex(a("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")))
    }

    @Test fun millionA() {
        val data = ByteArray(1_000_000) { 'a'.code.toByte() }
        assertEquals("cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0", Sha256Native.sha256Hex(data))
    }

    @Test fun digestIs32Bytes() {
        assertEquals(32, Sha256Native.sha256(a("")).size)
        assertEquals(32, Sha256Native.sha256(a("hello world")).size)
    }

    @Test fun streamingMatchesOneShot() {
        Sha256Native.Hasher().use { h ->
            h.update(a("ab")); h.update(a("c"))
            assertContentEquals(Sha256Native.sha256(a("abc")), h.digest())
        }
    }

    @Test fun streamingByteAtATime() {
        val data = ByteArray(100) { it.toByte() }
        Sha256Native.Hasher().use { h ->
            for (x in data) h.update(byteArrayOf(x))
            assertContentEquals(Sha256Native.sha256(data), h.digest())
        }
    }

    @Test fun streamingEmptyAndNonDestructive() {
        Sha256Native.Hasher().use { e -> assertContentEquals(Sha256Native.sha256(a("")), e.digest()) }
        Sha256Native.Hasher().use { h ->
            h.update(a("abc"))
            assertContentEquals(h.digest(), h.digest())
            h.update(a("d"))
            assertContentEquals(Sha256Native.sha256(a("abcd")), h.digest())
        }
    }

    @Test fun copyIsIndependent() {
        Sha256Native.Hasher().use { h ->
            h.update(a("ab"))
            h.copy().use { h2 ->
                h2.update(a("c")); h.update(a("x"))
                assertContentEquals(Sha256Native.sha256(a("abc")), h2.digest())
                assertContentEquals(Sha256Native.sha256(a("abx")), h.digest())
            }
        }
    }

    @Test fun usingClosedThrows() {
        val h = Sha256Native.Hasher()
        h.update(a("abc"))
        h.close()
        assertFailsWith<IllegalStateException> { h.update(a("x")) }
        assertFailsWith<IllegalStateException> { h.digest() }
        h.close()
    }
}
