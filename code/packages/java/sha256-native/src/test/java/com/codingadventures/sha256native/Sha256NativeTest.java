package com.codingadventures.sha256native;

import org.junit.jupiter.api.Test;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;
import static org.junit.jupiter.api.Assertions.*;

/** Exercises the Rust SHA-256 through JNI, asserting FIPS 180-4 answers. */
class Sha256NativeTest {

    private static byte[] a(String s) {
        return s.getBytes(StandardCharsets.UTF_8);
    }

    @Test
    void fipsVectors() {
        assertEquals("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            Sha256Native.sha256Hex(a("")));
        assertEquals("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            Sha256Native.sha256Hex(a("abc")));
        assertEquals("248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            Sha256Native.sha256Hex(a("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")));
    }

    @Test
    void millionA() {
        byte[] data = new byte[1_000_000];
        Arrays.fill(data, (byte) 'a');
        assertEquals("cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
            Sha256Native.sha256Hex(data));
    }

    @Test
    void digestIs32Bytes() {
        assertEquals(32, Sha256Native.sha256(a("")).length);
        assertEquals(32, Sha256Native.sha256(a("hello world")).length);
    }

    @Test
    void blockBoundariesDistinct() {
        Set<String> seen = new HashSet<>();
        for (int n : new int[]{55, 56, 63, 64, 127, 128}) {
            byte[] d = Sha256Native.sha256(new byte[n]);
            assertEquals(32, d.length);
            seen.add(Sha256Native.toHex(d));
        }
        assertEquals(6, seen.size());
    }

    @Test
    void streamingMatchesOneShot() {
        try (Sha256Native.Hasher h = new Sha256Native.Hasher()) {
            h.update(a("ab"));
            h.update(a("c"));
            assertArrayEquals(Sha256Native.sha256(a("abc")), h.digest());
        }
    }

    @Test
    void streamingByteAtATime() {
        byte[] data = new byte[100];
        for (int i = 0; i < 100; i++) data[i] = (byte) i;
        try (Sha256Native.Hasher h = new Sha256Native.Hasher()) {
            for (byte b : data) h.update(new byte[]{b});
            assertArrayEquals(Sha256Native.sha256(data), h.digest());
        }
    }

    @Test
    void streamingEmptyAndNonDestructive() {
        try (Sha256Native.Hasher e = new Sha256Native.Hasher()) {
            assertArrayEquals(Sha256Native.sha256(a("")), e.digest());
        }
        try (Sha256Native.Hasher h = new Sha256Native.Hasher()) {
            h.update(a("abc"));
            assertArrayEquals(h.digest(), h.digest());
            h.update(a("d"));
            assertArrayEquals(Sha256Native.sha256(a("abcd")), h.digest());
        }
    }

    @Test
    void copyIsIndependent() {
        try (Sha256Native.Hasher h = new Sha256Native.Hasher()) {
            h.update(a("ab"));
            try (Sha256Native.Hasher h2 = h.copy()) {
                h2.update(a("c"));
                h.update(a("x"));
                assertArrayEquals(Sha256Native.sha256(a("abc")), h2.digest());
                assertArrayEquals(Sha256Native.sha256(a("abx")), h.digest());
            }
        }
    }

    @Test
    void usingClosedHasherThrows() {
        Sha256Native.Hasher h = new Sha256Native.Hasher();
        h.update(a("abc"));
        h.close();
        assertThrows(IllegalStateException.class, () -> h.update(a("x")));
        assertThrows(IllegalStateException.class, h::digest);
        h.close(); // idempotent
    }
}
