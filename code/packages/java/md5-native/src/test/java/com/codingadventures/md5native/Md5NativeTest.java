package com.codingadventures.md5native;

import org.junit.jupiter.api.Test;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;
import static org.junit.jupiter.api.Assertions.*;

class Md5NativeTest {
    private static byte[] a(String s) {
        return s.getBytes(StandardCharsets.UTF_8);
    }

    @Test
    void rfcVectors() {
        assertEquals("d41d8cd98f00b204e9800998ecf8427e", Md5Native.hexString(a("")));
        assertEquals("0cc175b9c0f1b6a831c399e269772661", Md5Native.hexString(a("a")));
        assertEquals("900150983cd24fb0d6963f7d28e17f72", Md5Native.hexString(a("abc")));
        assertEquals("f96b697d7cb7938d525a2f31aaf161d0", Md5Native.hexString(a("message digest")));
    }

    @Test
    void bytes0To255() {
        byte[] data = new byte[256];
        for (int i = 0; i < 256; i++) data[i] = (byte) i;
        assertEquals("e2c865db4162bed963bfaa9ef6ac18f0", Md5Native.hexString(data));
    }

    @Test
    void digestIs16Bytes() {
        assertEquals(16, Md5Native.sumMd5(a("")).length);
        assertEquals(16, Md5Native.sumMd5(a("hello world")).length);
    }

    @Test
    void blockBoundariesDistinct() {
        Set<String> seen = new HashSet<>();
        for (int n : new int[]{0, 55, 56, 63, 64, 127, 128}) {
            seen.add(Md5Native.hexString(new byte[n]));
        }
        assertEquals(7, seen.size());
    }

    @Test
    void streamingMatchesOneShot() {
        try (Md5Native.Digest h = new Md5Native.Digest()) {
            h.update(a("ab"));
            h.update(a("c"));
            assertArrayEquals(Md5Native.sumMd5(a("abc")), h.digest());
        }
    }

    @Test
    void streamingByteAtATime() {
        byte[] data = new byte[100];
        for (int i = 0; i < 100; i++) data[i] = (byte) i;
        try (Md5Native.Digest h = new Md5Native.Digest()) {
            for (byte b : data) h.update(new byte[]{b});
            assertArrayEquals(Md5Native.sumMd5(data), h.digest());
        }
    }

    @Test
    void streamingEmptyAndNonDestructive() {
        try (Md5Native.Digest e = new Md5Native.Digest()) {
            assertArrayEquals(Md5Native.sumMd5(a("")), e.digest());
        }
        try (Md5Native.Digest h = new Md5Native.Digest()) {
            h.update(a("abc"));
            assertArrayEquals(h.digest(), h.digest());
            h.update(a("d"));
            assertArrayEquals(Md5Native.sumMd5(a("abcd")), h.digest());
        }
    }

    @Test
    void copyIsIndependent() {
        try (Md5Native.Digest h = new Md5Native.Digest()) {
            h.update(a("ab"));
            try (Md5Native.Digest h2 = h.copy()) {
                h2.update(a("c"));
                h.update(a("x"));
                assertArrayEquals(Md5Native.sumMd5(a("abc")), h2.digest());
                assertArrayEquals(Md5Native.sumMd5(a("abx")), h.digest());
            }
        }
    }

    @Test
    void usingClosedThrows() {
        Md5Native.Digest h = new Md5Native.Digest();
        h.update(a("abc"));
        h.close();
        assertThrows(IllegalStateException.class, () -> h.update(a("x")));
        assertThrows(IllegalStateException.class, h::digest);
        h.close();
    }
}
