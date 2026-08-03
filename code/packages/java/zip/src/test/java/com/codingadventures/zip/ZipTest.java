package com.codingadventures.zip;

// Unit tests for the Java ZIP (CMP09) implementation.
//
// Tests TC-1 through TC-12 mirror the C# reference test suite to ensure
// cross-language behavioural consistency for the ZIP archive format.
//
// Test structure:
//   TC-01  Round-trip single file, stored method.
//   TC-02  Round-trip single file, DEFLATE method.
//   TC-03  Multiple files in one archive.
//   TC-04  Directory entry (name ends with '/').
//   TC-05  CRC mismatch detection (corrupt byte → IOException).
//   TC-06  Random-access read (read a specific entry by name).
//   TC-07  Incompressible data falls back to stored method.
//   TC-08  Empty file (zero bytes).
//   TC-09  Large file (100 KB repetitive data) compressed with DEFLATE.
//   TC-10  Unicode (UTF-8) filename.
//   TC-11  Nested path (e.g., "dir/sub/file.txt").
//   TC-12  Empty archive (no entries).

import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.List;
import java.util.Random;

import static org.junit.jupiter.api.Assertions.*;

/**
 * JUnit 5 tests for {@link Zip}.
 *
 * <p>Each test is self-contained and labelled for human-readable test reports.
 * All tests exercise the public API ({@link Zip.ZipWriter}, {@link Zip.ZipReader},
 * {@link Zip#zip}, {@link Zip#unzip}) rather than internal helpers.</p>
 */
class ZipTest {

    // ─── Helpers ─────────────────────────────────────────────────────────────

    /** Convenience: UTF-8 encode a string. */
    private static byte[] utf8(String s) {
        return s.getBytes(StandardCharsets.UTF_8);
    }

    // ─── TC-01: Round-trip single file — stored ───────────────────────────────

    /**
     * TC-01: A single file stored without compression survives the round trip.
     *
     * <p>We force {@code compress=false} so the archive uses method 0 (Stored).
     * After reading back the entry the bytes must match exactly.</p>
     */
    @Test
    void roundTripSingleFileStored() throws IOException {
        byte[] content = utf8("Hello, ZIP!");
        Zip.ZipWriter w = new Zip.ZipWriter();
        w.addFile("hello.txt", content, false);
        byte[] archive = w.finish();

        Zip.ZipReader r = new Zip.ZipReader(archive);
        assertEquals(1, r.entries().size());
        assertEquals("hello.txt", r.entries().get(0).name());
        assertArrayEquals(content, r.read("hello.txt"));
    }

    // ─── TC-02: Round-trip single file — DEFLATE ─────────────────────────────

    /**
     * TC-02: A single file compressed with DEFLATE survives the round trip.
     *
     * <p>Repetitive data ("ABCABC...") is highly compressible so the archive
     * will use method 8 (DEFLATE).  After reading back, the bytes must match.</p>
     */
    @Test
    void roundTripSingleFileDeflate() throws IOException {
        // Build a 2 KB repetitive payload that DEFLATE will compress well.
        byte[] base = utf8("ABCDEFGHIJ");
        byte[] content = new byte[2048];
        for (int i = 0; i < content.length; i++) {
            content[i] = base[i % base.length];
        }

        Zip.ZipWriter w = new Zip.ZipWriter();
        w.addFile("data.bin", content, true);
        byte[] archive = w.finish();

        Zip.ZipReader r = new Zip.ZipReader(archive);
        assertEquals(1, r.entries().size());
        assertArrayEquals(content, r.read("data.bin"));
    }

    // ─── TC-03: Multiple files in one archive ─────────────────────────────────

    /**
     * TC-03: An archive containing three files preserves all entries in order
     * and returns the correct data for each.
     *
     * <p>This exercises the sequential-write path (multiple Local Headers) and
     * the multi-entry Central Directory parse.</p>
     */
    @Test
    void multipleFiles() throws IOException {
        byte[] data1 = utf8("File one content");
        byte[] data2 = utf8("File two content");
        byte[] data3 = utf8("File three content");

        Zip.ZipWriter w = new Zip.ZipWriter();
        w.addFile("a.txt", data1, false);
        w.addFile("b.txt", data2, false);
        w.addFile("c.txt", data3, false);
        byte[] archive = w.finish();

        Zip.ZipReader r = new Zip.ZipReader(archive);
        assertEquals(3, r.entries().size());
        assertArrayEquals(data1, r.read("a.txt"));
        assertArrayEquals(data2, r.read("b.txt"));
        assertArrayEquals(data3, r.read("c.txt"));
    }

    // ─── TC-04: Directory entry ────────────────────────────────────────────────

    /**
     * TC-04: A directory entry appears in the entries list with an empty data
     * array and a name ending with '/'.
     *
     * <p>Directory entries are stored with method 0, zero bytes of data, and
     * Unix mode 0o040755 in the external attributes.</p>
     */
    @Test
    void directoryEntry() throws IOException {
        Zip.ZipWriter w = new Zip.ZipWriter();
        w.addDirectory("mydir/");
        w.addFile("mydir/file.txt", utf8("content"), false);
        byte[] archive = w.finish();

        Zip.ZipReader r = new Zip.ZipReader(archive);
        assertEquals(2, r.entries().size());

        // First entry should be the directory.
        Zip.ZipEntry dir = r.entries().get(0);
        assertEquals("mydir/", dir.name());
        assertTrue(dir.name().endsWith("/"), "directory entry name must end with '/'");

        // File entry should be readable.
        assertArrayEquals(utf8("content"), r.read("mydir/file.txt"));
    }

    // ─── TC-05: CRC mismatch detection ────────────────────────────────────────

    /**
     * TC-05: Corrupting a byte in the file data region triggers an
     * {@link IOException} with a "CRC-32 mismatch" message on read.
     *
     * <p>This is the primary integrity mechanism of the ZIP format — the CRC
     * stored in the header must match the CRC of the decompressed content.
     * We flip one byte after the Local File Header to simulate corruption.</p>
     */
    @Test
    void crcMismatchDetected() throws IOException {
        byte[] content = utf8("This content will be corrupted.");
        Zip.ZipWriter w = new Zip.ZipWriter();
        w.addFile("corrupt.txt", content, false);
        byte[] archive = w.finish();

        // Locate the start of file data and flip one byte.
        // Local File Header is 30 bytes + name length "corrupt.txt" = 11 bytes → data at offset 41.
        // Flip a byte well inside the data region.
        int dataOffset = 30 + "corrupt.txt".length() + 5; // a few bytes into the data
        archive[dataOffset] ^= 0xFF;

        // Parse should succeed (EOCD / Central Directory are not corrupted).
        Zip.ZipReader r = new Zip.ZipReader(archive);

        // read() must throw because CRC-32 will not match.
        IOException ex = assertThrows(IOException.class, () -> r.read("corrupt.txt"));
        assertTrue(ex.getMessage().toLowerCase().contains("crc"),
            "exception message should mention CRC: " + ex.getMessage());
    }

    // ─── TC-06: Random-access read ────────────────────────────────────────────

    /**
     * TC-06: The reader can retrieve any entry by name without reading others.
     *
     * <p>This tests the random-access property of ZIP: the Central Directory
     * stores local offsets so we can jump directly to any entry.</p>
     */
    @Test
    void randomAccessRead() throws IOException {
        Zip.ZipWriter w = new Zip.ZipWriter();
        w.addFile("first.txt",  utf8("first"),  false);
        w.addFile("second.txt", utf8("second"), false);
        w.addFile("third.txt",  utf8("third"),  false);
        byte[] archive = w.finish();

        Zip.ZipReader r = new Zip.ZipReader(archive);
        // Read the middle entry directly.
        assertArrayEquals(utf8("second"), r.read("second.txt"));
        // Read the last entry.
        assertArrayEquals(utf8("third"), r.read("third.txt"));
        // Read the first entry last.
        assertArrayEquals(utf8("first"), r.read("first.txt"));
    }

    // ─── TC-07: Incompressible data stored ────────────────────────────────────

    /**
     * TC-07: Truly random (incompressible) data causes the writer to fall back
     * to stored method, and the archive is still readable.
     *
     * <p>Pseudo-random bytes cannot be made smaller by DEFLATE.  The auto-
     * compression policy stores them verbatim when compressed >= original.</p>
     */
    @Test
    void incompressibleDataStoredMethod() throws IOException {
        // Generate 4 KB of pseudo-random bytes.
        byte[] random = new byte[4096];
        new Random(42L).nextBytes(random);

        Zip.ZipWriter w = new Zip.ZipWriter();
        w.addFile("rand.bin", random, true); // compress=true but will fall back
        byte[] archive = w.finish();

        Zip.ZipReader r = new Zip.ZipReader(archive);
        assertArrayEquals(random, r.read("rand.bin"));
    }

    // ─── TC-08: Empty file ────────────────────────────────────────────────────

    /**
     * TC-08: A file with zero bytes is stored and retrieved correctly.
     *
     * <p>Empty files are stored with method 0 (no DEFLATE needed), CRC 0x00000000,
     * compressed and uncompressed sizes both 0.</p>
     */
    @Test
    void emptyFile() throws IOException {
        Zip.ZipWriter w = new Zip.ZipWriter();
        w.addFile("empty.txt", new byte[0], true);
        byte[] archive = w.finish();

        Zip.ZipReader r = new Zip.ZipReader(archive);
        assertEquals(1, r.entries().size());
        assertArrayEquals(new byte[0], r.read("empty.txt"));
    }

    // ─── TC-09: Large file compressed ─────────────────────────────────────────

    /**
     * TC-09: A large (100 KB) repetitive file compresses with DEFLATE and
     * round-trips correctly.
     *
     * <p>100 KB of cycling "HELLO WORLD " is trivially compressible.  This
     * verifies the LZSS + Huffman pipeline handles multi-kilobyte payloads
     * without data loss or truncation.</p>
     */
    @Test
    void largeFileCompressed() throws IOException {
        byte[] pattern = utf8("HELLO WORLD ");
        byte[] large = new byte[100 * 1024];
        for (int i = 0; i < large.length; i++) {
            large[i] = pattern[i % pattern.length];
        }

        Zip.ZipWriter w = new Zip.ZipWriter();
        w.addFile("large.txt", large, true);
        byte[] archive = w.finish();

        // Archive should be significantly smaller than the original.
        assertTrue(archive.length < large.length,
            "compressed archive should be smaller than original data");

        Zip.ZipReader r = new Zip.ZipReader(archive);
        assertArrayEquals(large, r.read("large.txt"));
    }

    // ─── TC-10: Unicode filename ──────────────────────────────────────────────

    /**
     * TC-10: A file with a Unicode (UTF-8) filename is stored and retrieved
     * by its exact name.
     *
     * <p>The ZIP writer sets GP flag bit 11 to signal UTF-8 filenames.
     * The reader decodes names as UTF-8.  This test uses Japanese characters
     * that occupy multiple bytes in UTF-8.</p>
     */
    @Test
    void unicodeFilename() throws IOException {
        String name = "日本語/ファイル.txt"; // Japanese path
        byte[] content = utf8("内容");      // "content" in Japanese

        Zip.ZipWriter w = new Zip.ZipWriter();
        w.addFile(name, content, false);
        byte[] archive = w.finish();

        Zip.ZipReader r = new Zip.ZipReader(archive);
        assertEquals(1, r.entries().size());
        assertEquals(name, r.entries().get(0).name());
        assertArrayEquals(content, r.read(name));
    }

    // ─── TC-11: Nested paths ──────────────────────────────────────────────────

    /**
     * TC-11: Files with nested path separators are stored and retrieved with
     * their full paths intact.
     *
     * <p>ZIP stores paths as opaque UTF-8 strings; nested directories are
     * represented by '/' characters in the name.  No special handling is
     * needed beyond treating the name as a string.</p>
     */
    @Test
    void nestedPaths() throws IOException {
        Zip.ZipWriter w = new Zip.ZipWriter();
        w.addFile("a/b/c.txt",   utf8("deep"), false);
        w.addFile("a/d.txt",     utf8("mid"),  false);
        w.addFile("top.txt",     utf8("top"),  false);
        byte[] archive = w.finish();

        Zip.ZipReader r = new Zip.ZipReader(archive);
        assertEquals(3, r.entries().size());
        assertArrayEquals(utf8("deep"), r.read("a/b/c.txt"));
        assertArrayEquals(utf8("mid"),  r.read("a/d.txt"));
        assertArrayEquals(utf8("top"),  r.read("top.txt"));
    }

    // ─── TC-12: Empty archive ─────────────────────────────────────────────────

    /**
     * TC-12: An archive with no entries is valid: it contains only the EOCD
     * record and can be parsed successfully with an empty entry list.
     *
     * <p>Many tools create empty ZIPs (e.g., when archiving an empty directory
     * tree).  The EOCD is the minimal valid ZIP archive.</p>
     */
    @Test
    void emptyArchive() throws IOException {
        Zip.ZipWriter w = new Zip.ZipWriter();
        byte[] archive = w.finish();

        Zip.ZipReader r = new Zip.ZipReader(archive);
        assertEquals(0, r.entries().size());

        // Convenience round-trip via zip/unzip with empty list.
        byte[] archive2 = Zip.zip(List.of());
        List<Zip.ZipEntry> entries = Zip.unzip(archive2);
        assertEquals(0, entries.size());
    }
}
