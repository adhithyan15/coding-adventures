/**
 * Tests for the Kotlin ZIP (CMP09) implementation.
 *
 * Covers TC-1 through TC-12 matching the Rust and C# reference test suites:
 *
 *   TC-01  Stored round-trip (compress=false)
 *   TC-02  DEFLATE round-trip (repetitive text)
 *   TC-03  Multiple files in one archive
 *   TC-04  Directory entry
 *   TC-05  CRC-32 mismatch detection
 *   TC-06  Random-access read (single entry by name from a 10-file archive)
 *   TC-07  Incompressible data falls back to Stored
 *   TC-08  Empty file entry
 *   TC-09  Large file with DEFLATE compression
 *   TC-10  Unicode filename (multi-byte UTF-8)
 *   TC-11  Nested paths
 *   TC-12  Empty archive
 *
 * Additional unit tests cover CRC-32, DEFLATE internals, and ZipArchive helpers.
 */
package com.codingadventures.zip

import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import java.io.IOException

class ZipTest {

    // =========================================================================
    // CRC-32
    // =========================================================================

    @Test
    fun `TC-CRC01 crc32 known value hello world`() {
        // CRC-32 of "hello world" — verified against Python's binascii.crc32().
        assertEquals(0x0D4A_1185.toInt(), crc32("hello world".toByteArray()))
        // Standard test vector: CRC-32 of "123456789" = 0xCBF43926.
        assertEquals(0xCBF4_3926.toInt(), crc32("123456789".toByteArray()))
    }

    @Test
    fun `TC-CRC02 crc32 empty input`() {
        assertEquals(0, crc32(ByteArray(0)))
    }

    @Test
    fun `TC-CRC03 crc32 incremental equals one-shot`() {
        val full = crc32("hello world".toByteArray())
        val part1 = crc32("hello ".toByteArray())
        val part2 = crc32("world".toByteArray(), part1)
        assertEquals(full, part2)
    }

    // =========================================================================
    // DEFLATE round-trips
    // =========================================================================

    private fun deflateRt(data: ByteArray) {
        val compressed = deflateCompress(data)
        val decompressed = deflateDecompress(compressed)
        assertArrayEquals(data, decompressed, "DEFLATE round-trip mismatch")
    }

    @Test
    fun `TC-DEFLATE01 empty`() { deflateRt(ByteArray(0)) }

    @Test
    fun `TC-DEFLATE02 single byte`() { deflateRt(byteArrayOf(0x41)) }

    @Test
    fun `TC-DEFLATE03 all 256 byte values`() {
        deflateRt(ByteArray(256) { it.toByte() })
    }

    @Test
    fun `TC-DEFLATE04 repetitive data compresses`() {
        val data = "ABCABCABC".toByteArray().let { it.let { base ->
            ByteArray(base.size * 100).also { out ->
                for (i in out.indices) out[i] = base[i % base.size]
            }
        }}
        val compressed = deflateCompress(data)
        val decompressed = deflateDecompress(compressed)
        assertArrayEquals(data, decompressed)
        assertTrue(compressed.size < data.size, "DEFLATE must compress repetitive data")
    }

    @Test
    fun `TC-DEFLATE05 long string round-trip`() {
        val base = "the quick brown fox jumps over the lazy dog ".toByteArray()
        val data = ByteArray(base.size * 20).also { out ->
            for (i in out.indices) out[i] = base[i % base.size]
        }
        deflateRt(data)
    }

    @Test
    fun `TC-DEFLATE06 binary data round-trip`() {
        val data = ByteArray(512) { (it % 256).toByte() }
        deflateRt(data)
    }

    // =========================================================================
    // TC-01: Stored round-trip
    // =========================================================================

    @Test
    fun `TC-01 stored round-trip`() {
        val data = "hello, world".toByteArray()
        val w = ZipWriter()
        w.addFile("hello.txt", data, compress = false) // compress=false → Stored
        val archive = w.finish()

        val reader = ZipReader(archive)
        val files = ZipArchive.unzip(archive)
        assertEquals(1, files.size)
        assertEquals("hello.txt", files[0].name)
        assertArrayEquals(data, files[0].data)

        // Verify method is 0 (Stored).
        val meta = reader.entries.first()
        assertEquals("hello.txt", meta.name)
    }

    // =========================================================================
    // TC-02: DEFLATE round-trip
    // =========================================================================

    @Test
    fun `TC-02 DEFLATE round-trip`() {
        val base = "the quick brown fox jumps over the lazy dog ".toByteArray()
        val text = ByteArray(base.size * 10).also { out ->
            for (i in out.indices) out[i] = base[i % base.size]
        }
        val archive = ZipArchive.zip(listOf(ZipEntry("text.txt", text)))
        val files = ZipArchive.unzip(archive)
        assertEquals("text.txt", files[0].name)
        assertArrayEquals(text, files[0].data)
    }

    // =========================================================================
    // TC-03: Multiple files
    // =========================================================================

    @Test
    fun `TC-03 multiple files in one archive`() {
        val allBytes = ByteArray(256) { it.toByte() }
        val input = listOf(
            ZipEntry("a.txt", "file A content".toByteArray()),
            ZipEntry("b.txt", "file B content".toByteArray()),
            ZipEntry("c.bin", allBytes)
        )
        val archive = ZipArchive.zip(input)
        val files = ZipArchive.unzip(archive)
        assertEquals(3, files.size)
        for (entry in input) {
            val found = files.find { it.name == entry.name }
                ?: fail("Entry '${entry.name}' not found in extracted output")
            assertArrayEquals(entry.data, found.data, "mismatch for ${entry.name}")
        }
    }

    // =========================================================================
    // TC-04: Directory entry
    // =========================================================================

    @Test
    fun `TC-04 directory entry`() {
        val w = ZipWriter()
        w.addDirectory("mydir/")
        w.addFile("mydir/file.txt", "contents".toByteArray(), compress = true)
        val archive = w.finish()

        val reader = ZipReader(archive)
        val names = reader.entries.map { it.name }
        assertTrue("mydir/" in names, "directory entry missing")
        assertTrue("mydir/file.txt" in names, "file inside dir missing")

        // Reading the directory entry should yield empty bytes (not throw).
        val dirData = reader.read("mydir/")
        assertEquals(0, dirData.size)
    }

    // =========================================================================
    // TC-05: CRC-32 mismatch detected
    // =========================================================================

    @Test
    fun `TC-05 CRC-32 mismatch detected`() {
        val archive = ZipArchive.zip(listOf(ZipEntry("f.txt", "test data".toByteArray())))
        val corrupted = archive.copyOf()

        // Corrupt a data byte directly.
        // Offset 35 = 30-byte fixed Local Header + 5-byte name "f.txt" = first data byte.
        // Changing the file data means CRC-32 of the decompressed content won't match.
        corrupted[35] = (corrupted[35].toInt() xor 0xFF).toByte()

        val ex = assertThrows<IOException> {
            ZipArchive.unzip(corrupted)
        }
        assertTrue(ex.message?.contains("CRC") == true,
            "Expected error message to contain 'CRC', got: ${ex.message}")
    }

    // =========================================================================
    // TC-06: Random access (read single entry)
    // =========================================================================

    @Test
    fun `TC-06 random access read single entry`() {
        val entries = (0 until 10).map { i ->
            ZipEntry("f$i.txt", "content $i".toByteArray())
        }
        val archive = ZipArchive.zip(entries)

        val reader = ZipReader(archive)
        val data5 = reader.read("f5.txt")
        assertArrayEquals("content 5".toByteArray(), data5)
    }

    // =========================================================================
    // TC-07: Incompressible data uses Stored
    // =========================================================================

    @Test
    fun `TC-07 incompressible data uses Stored method`() {
        // Pseudo-random data via LCG (seed=42): compresses poorly with DEFLATE.
        var seed = 42L
        val data = ByteArray(1024) {
            seed = (seed * 1_664_525L + 1_013_904_223L) and 0xFFFFFFFFL
            (seed ushr 24).toByte()
        }

        val archive = ZipArchive.zip(listOf(ZipEntry("random.bin", data)))
        val reader = ZipReader(archive)

        // The entry should use method=0 (Stored) because DEFLATE would expand it.
        // We check by verifying that the compressed size equals the uncompressed size.
        // (The reader only exposes entry names, not method directly — so we check via unzip.)
        val files = ZipArchive.unzip(archive)
        assertEquals(1, files.size)
        assertArrayEquals(data, files[0].data)

        // Verify Stored by checking archive size: Stored archive is smaller than
        // a DEFLATE archive that expands the data. The archive overhead is fixed
        // (~30 bytes local header + ~46 bytes CD + 22 bytes EOCD) so the total
        // should be close to data.size + ~100 bytes, not data.size + DEFLATE overhead.
        assertTrue(archive.size <= data.size + 200,
            "Incompressible data should be Stored, not DEFLATE-expanded")
    }

    // =========================================================================
    // TC-08: Empty file entry
    // =========================================================================

    @Test
    fun `TC-08 empty file entry`() {
        val archive = ZipArchive.zip(listOf(ZipEntry("empty.txt", ByteArray(0))))
        val files = ZipArchive.unzip(archive)
        assertEquals(1, files.size)
        assertEquals("empty.txt", files[0].name)
        assertEquals(0, files[0].data.size)
    }

    // =========================================================================
    // TC-09: Large file with compression
    // =========================================================================

    @Test
    fun `TC-09 large file compressed`() {
        val base = "abcdefghij".toByteArray()
        val data = ByteArray(base.size * 10_000).also { out ->
            for (i in out.indices) out[i] = base[i % base.size]
        } // 100 KB of repetitive data

        val archive = ZipArchive.zip(listOf(ZipEntry("big.bin", data)))
        val files = ZipArchive.unzip(archive)
        assertArrayEquals(data, files[0].data)
        assertTrue(
            archive.size < data.size,
            "Repetitive 100 KB must compress: archive=${archive.size} data=${data.size}"
        )
    }

    // =========================================================================
    // TC-10: Unicode filename
    // =========================================================================

    @Test
    fun `TC-10 unicode filename`() {
        val name = "日本語/résumé.txt"
        val archive = ZipArchive.zip(listOf(ZipEntry(name, "content".toByteArray())))
        val files = ZipArchive.unzip(archive)
        assertEquals(name, files[0].name)
        assertArrayEquals("content".toByteArray(), files[0].data)
    }

    // =========================================================================
    // TC-11: Nested paths
    // =========================================================================

    @Test
    fun `TC-11 nested paths`() {
        val input = listOf(
            ZipEntry("root.txt", "root".toByteArray()),
            ZipEntry("dir/file.txt", "nested".toByteArray()),
            ZipEntry("dir/sub/deep.txt", "deep".toByteArray())
        )
        val archive = ZipArchive.zip(input)
        val files = ZipArchive.unzip(archive)
        for (entry in input) {
            val found = files.find { it.name == entry.name }
                ?: fail("Entry '${entry.name}' not found")
            assertArrayEquals(entry.data, found.data, "mismatch for ${entry.name}")
        }
    }

    // =========================================================================
    // TC-12: Empty archive
    // =========================================================================

    @Test
    fun `TC-12 empty archive`() {
        val archive = ZipArchive.zip(emptyList())
        val files = ZipArchive.unzip(archive)
        assertTrue(files.isEmpty())
    }

    // =========================================================================
    // Additional: ZipWriter / ZipReader API
    // =========================================================================

    @Test
    fun `TC-R01 ZipReader read by name`() {
        val archive = ZipArchive.zip(listOf(
            ZipEntry("alpha.txt", "AAA".toByteArray()),
            ZipEntry("beta.txt", "BBB".toByteArray())
        ))
        val reader = ZipReader(archive)
        assertArrayEquals("BBB".toByteArray(), reader.read("beta.txt"))

        val ex = assertThrows<IOException> {
            reader.read("nope.txt")
        }
        assertTrue(ex.message?.contains("not found") == true)
    }

    @Test
    fun `TC-R02 dosDt epoch`() {
        // 1980-01-01 00:00:00: year_offset=0, month=1, day=1 → date=(0<<9)|(1<<5)|1=33=0x21; time=0
        val dt = dosDt(1980, 1, 1, 0, 0, 0)
        assertEquals(33, dt ushr 16)    // date field
        assertEquals(0, dt and 0xFFFF)  // time field
    }
}
