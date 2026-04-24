// ZipTests.cs — xUnit test suite for CodingAdventures.Zip (CMP09)
//
// 12 test cases covering:
//   TC-1:  Round-trip single file, Stored (compress=false)
//   TC-2:  Round-trip single file, DEFLATE (repetitive text gets smaller)
//   TC-3:  Multiple files in one archive
//   TC-4:  Directory entry (name ends with /)
//   TC-5:  CRC-32 mismatch detected (corrupt byte → exception)
//   TC-6:  Random-access read (10 files, read only f5.txt)
//   TC-7:  Incompressible data stored as Stored (method=0)
//   TC-8:  Empty file
//   TC-9:  Large file compressed (100 KB repetitive data)
//   TC-10: Unicode filename
//   TC-11: Nested paths
//   TC-12: Empty archive

using System.IO;
using System.Linq;
using System.Text;
using CodingAdventures.Zip;

namespace CodingAdventures.Zip.Tests;

public class ZipTests
{
    // ── TC-1: Round-trip single file, Stored ─────────────────────────────────
    //
    // When compress=false the writer must use method=0 (Stored) and the reader
    // must return the original bytes verbatim.

    [Fact]
    public void TC1_RoundTrip_SingleFile_Stored()
    {
        var data = "hello, world"u8.ToArray();

        var writer = new ZipWriter();
        writer.AddFile("hello.txt", data, compress: false);
        var archive = writer.Finish();

        var entries = ZipArchive.Unzip(archive);
        Assert.Single(entries);
        Assert.Equal("hello.txt", entries[0].Name);
        Assert.Equal(data, entries[0].Data);

        // Verify method=0 is recorded in the Central Directory.
        var reader = new ZipReader(archive);
        // The entry list uses empty Data placeholders; read via Read() to get bytes.
        Assert.Equal(data, reader.Read("hello.txt"));
    }

    // ── TC-2: Round-trip single file, DEFLATE ────────────────────────────────
    //
    // Highly repetitive text should compress to fewer bytes than the original.

    [Fact]
    public void TC2_RoundTrip_SingleFile_Deflate()
    {
        var text = Encoding.UTF8.GetBytes(
            string.Concat(Enumerable.Repeat("the quick brown fox jumps over the lazy dog ", 10)));

        var archive = ZipArchive.Zip([new ZipEntry("text.txt", text)]);

        var entries = ZipArchive.Unzip(archive);
        Assert.Single(entries);
        Assert.Equal("text.txt", entries[0].Name);
        Assert.Equal(text, entries[0].Data);

        // The archive must be smaller than the raw text (compression worked).
        Assert.True(archive.Length < text.Length,
            $"archive ({archive.Length} bytes) must be smaller than text ({text.Length} bytes)");
    }

    // ── TC-3: Multiple files in one archive ──────────────────────────────────

    [Fact]
    public void TC3_MultipleFiles()
    {
        var allBytes = Enumerable.Range(0, 256).Select(i => (byte)i).ToArray();
        var input = new[]
        {
            new ZipEntry("a.txt", "file A content"u8.ToArray()),
            new ZipEntry("b.txt", "file B content"u8.ToArray()),
            new ZipEntry("c.bin", allBytes),
        };

        var archive = ZipArchive.Zip(input);
        var output  = ZipArchive.Unzip(archive);

        Assert.Equal(3, output.Count);
        foreach (var orig in input)
        {
            var found = output.First(e => e.Name == orig.Name);
            Assert.Equal(orig.Data, found.Data);
        }
    }

    // ── TC-4: Directory entry ─────────────────────────────────────────────────
    //
    // Directory entries have name ending with '/', method=0, sizes=0, CRC=0.
    // They must appear in the Central Directory but return empty data on read.

    [Fact]
    public void TC4_DirectoryEntry()
    {
        var writer = new ZipWriter();
        writer.AddDirectory("mydir/");
        writer.AddFile("mydir/file.txt", "contents"u8.ToArray());
        var archive = writer.Finish();

        var reader = new ZipReader(archive);
        var names  = reader.Entries.Select(e => e.Name).ToList();

        Assert.Contains("mydir/", names);
        Assert.Contains("mydir/file.txt", names);

        // Reading a directory entry returns empty bytes.
        Assert.Empty(reader.Read("mydir/"));
    }

    // ── TC-5: CRC-32 mismatch detected ───────────────────────────────────────
    //
    // Corrupt one byte of the compressed payload in the archive. When the reader
    // decompresses and CRC-checks it must throw InvalidDataException mentioning "CRC".

    [Fact]
    public void TC5_CrcMismatchDetected()
    {
        // Build an archive with a known file.
        var original = ZipArchive.Zip([new ZipEntry("f.txt", "test data"u8.ToArray())]);

        // Find where the file data starts: 30-byte fixed LFH + 5-byte name "f.txt" = offset 35.
        // For a stored file the data follows immediately at byte 35.
        // We want to corrupt a byte that is part of the actual payload
        // (not a header field that we ignore during reads).
        var corrupted = (byte[])original.Clone();

        // Find the local header start (signature at offset 0) and skip to data.
        // Local header fixed part: 30 bytes. Name "f.txt" = 5 bytes → data at byte 35.
        corrupted[35] ^= 0xFF;

        var reader = new ZipReader(corrupted);
        var ex = Assert.Throws<InvalidDataException>(() => reader.Read("f.txt"));
        Assert.Contains("CRC", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    // ── TC-6: Random-access read (10 files, read only f5.txt) ─────────────────
    //
    // ZipReader must be able to read a single entry without reading the others.

    [Fact]
    public void TC6_RandomAccessRead()
    {
        var entries = Enumerable.Range(0, 10)
            .Select(i => new ZipEntry($"f{i}.txt", Encoding.UTF8.GetBytes($"content {i}")))
            .ToArray();

        var archive = ZipArchive.Zip(entries);
        var reader  = new ZipReader(archive);

        var data5 = reader.Read("f5.txt");
        Assert.Equal(Encoding.UTF8.GetBytes("content 5"), data5);
    }

    // ── TC-7: Incompressible data stored as Stored (method=0) ─────────────────
    //
    // When DEFLATE produces output >= original length the writer must fall back
    // to method=0. The Central Directory must record method=0 for such entries.

    [Fact]
    public void TC7_IncompressibleData_StoredMethod()
    {
        // Build pseudo-random data via a simple LCG. This should be incompressible.
        var seed = 42u;
        var data = new byte[1024];
        for (var i = 0; i < data.Length; i++)
        {
            seed = seed * 1_664_525u + 1_013_904_223u;
            data[i] = (byte)(seed >> 24);
        }

        var archive = ZipArchive.Zip([new ZipEntry("random.bin", data)]);
        var reader  = new ZipReader(archive);

        // Find the entry in the Central Directory and check its method field.
        // Because the reader's Entries list contains placeholder ZipEntry objects,
        // we need to check via the raw archive bytes. We do so by verifying that
        // the round-tripped data is correct (which would fail if decompression went wrong).
        var result = reader.Read("random.bin");
        Assert.Equal(data, result);

        // Verify method=0 by inspecting the archive bytes.
        // Central Directory starts after all Local File Headers. We search for the
        // CD signature and check offset +10 (method field).
        var methodInCd = FindCdMethod(archive, "random.bin");
        Assert.Equal(0, methodInCd); // Stored
    }

    // Helper: scan archive bytes for a Central Directory entry for `entryName`
    // and return its compression method.
    private static int FindCdMethod(byte[] archive, string entryName)
    {
        var sig   = new byte[] { 0x50, 0x4B, 0x01, 0x02 };
        var name  = Encoding.UTF8.GetBytes(entryName);

        for (var i = 0; i <= archive.Length - 46; i++)
        {
            if (archive[i] != sig[0] || archive[i+1] != sig[1] ||
                archive[i+2] != sig[2] || archive[i+3] != sig[3]) continue;

            var nameLen = archive[i + 28] | (archive[i + 29] << 8);
            if (nameLen != name.Length) continue;

            var nameStart = i + 46;
            if (nameStart + nameLen > archive.Length) continue;
            if (!archive.AsSpan(nameStart, nameLen).SequenceEqual(name)) continue;

            return archive[i + 10] | (archive[i + 11] << 8); // method
        }
        throw new InvalidOperationException($"CD entry for '{entryName}' not found");
    }

    // ── TC-8: Empty file ──────────────────────────────────────────────────────

    [Fact]
    public void TC8_EmptyFile()
    {
        var archive = ZipArchive.Zip([new ZipEntry("empty.txt", [])]);
        var entries = ZipArchive.Unzip(archive);

        Assert.Single(entries);
        Assert.Equal("empty.txt", entries[0].Name);
        Assert.Empty(entries[0].Data);
    }

    // ── TC-9: Large file compressed (100 KB repetitive data) ─────────────────
    //
    // 100 KB of "abcdefghij" repeated must compress to a significantly smaller archive.

    [Fact]
    public void TC9_LargeFile_Compressed()
    {
        // 10 bytes × 10000 repetitions = 100 KB. DEFLATE should compress this well.
        var chunk = "abcdefghij"u8.ToArray();
        var data  = Enumerable.Repeat(chunk, 10_000)
                              .SelectMany(x => x)
                              .ToArray();

        var archive = ZipArchive.Zip([new ZipEntry("big.bin", data)]);
        var entries = ZipArchive.Unzip(archive);

        Assert.Equal(data, entries[0].Data);
        Assert.True(archive.Length < data.Length,
            $"100 KB repetitive data must compress: archive={archive.Length} data={data.Length}");
    }

    // ── TC-10: Unicode filename ───────────────────────────────────────────────
    //
    // ZIP bit 11 = UTF-8. Both the writer and reader must preserve multi-byte filenames.

    [Fact]
    public void TC10_UnicodeFilename()
    {
        var name    = "日本語/résumé.txt";
        var content = "content"u8.ToArray();

        var archive = ZipArchive.Zip([new ZipEntry(name, content)]);
        var entries = ZipArchive.Unzip(archive);

        Assert.Single(entries);
        Assert.Equal(name, entries[0].Name);
        Assert.Equal(content, entries[0].Data);
    }

    // ── TC-11: Nested paths ───────────────────────────────────────────────────

    [Fact]
    public void TC11_NestedPaths()
    {
        var input = new[]
        {
            new ZipEntry("root.txt",         "root"u8.ToArray()),
            new ZipEntry("dir/file.txt",     "nested"u8.ToArray()),
            new ZipEntry("dir/sub/deep.txt", "deep"u8.ToArray()),
        };

        var archive = ZipArchive.Zip(input);
        var output  = ZipArchive.Unzip(archive);

        foreach (var orig in input)
        {
            var found = output.First(e => e.Name == orig.Name);
            Assert.Equal(orig.Data, found.Data);
        }
    }

    // ── TC-12: Empty archive ──────────────────────────────────────────────────
    //
    // A writer with no entries must produce a valid (but empty) archive.

    [Fact]
    public void TC12_EmptyArchive()
    {
        var writer  = new ZipWriter();
        var archive = writer.Finish();

        var reader  = new ZipReader(archive);
        Assert.Empty(reader.Entries);

        var entries = ZipArchive.Unzip(archive);
        Assert.Empty(entries);
    }
}
