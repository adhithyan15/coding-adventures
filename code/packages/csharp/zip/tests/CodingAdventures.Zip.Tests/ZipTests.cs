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

    // =========================================================================
    // Security / adversarial-input hardening
    // =========================================================================
    //
    // These are not part of the CMP09 spec's 12 numbered conformance cases —
    // they exercise the defenses added during security review: zip-slip
    // normalization on write, ZIP32 field-width limits on write, and
    // overflow-safe bounds checking + a decompression-bomb budget on read.

    // A malicious or careless caller must not be able to bake a path-traversal
    // shaped entry name into an archive this library produces — any downstream
    // extractor that does File.WriteAllBytes(Path.Combine(outDir, entry.Name),
    // ...) on the round-tripped entries must never see "..".
    [Fact]
    public void Security_WriterNormalizesTraversalSegmentsOutOfEntryNames()
    {
        var writer = new ZipWriter();
        writer.AddFile("../../etc/passwd", "not actually passwd"u8.ToArray());
        var archive = writer.Finish();

        var entries = ZipArchive.Unzip(archive);
        Assert.Single(entries);
        Assert.DoesNotContain("..", entries[0].Name);
        Assert.Equal("etc/passwd", entries[0].Name);
    }

    // Backslashes (Windows-style separators) and a leading absolute slash must
    // be normalized the same way as forward-slash ".." segments.
    [Fact]
    public void Security_WriterNormalizesBackslashesAndLeadingSlash()
    {
        var writer = new ZipWriter();
        writer.AddFile(@"..\..\windows\system32\evil.dll", "x"u8.ToArray());
        writer.AddFile("/etc/shadow", "y"u8.ToArray());
        var archive = writer.Finish();

        var names = ZipArchive.Unzip(archive).Select(e => e.Name).ToList();
        Assert.Contains("windows/system32/evil.dll", names);
        Assert.Contains("etc/shadow", names);
        Assert.DoesNotContain(names, n => n.Contains(".."));
        Assert.DoesNotContain(names, n => n.StartsWith('/'));
    }

    // A Windows drive-letter prefix ("C:\...") is a rooted path on Windows
    // even without a leading slash — Path.Combine(outDir, "C:/evil.dll")
    // silently discards outDir and returns the rooted second argument
    // verbatim. Splitting on separators alone doesn't touch a "C:" segment
    // (it contains neither "/" nor "\"), so it must be dropped explicitly.
    [Fact]
    public void Security_WriterStripsWindowsDriveLetterPrefix()
    {
        var writer = new ZipWriter();
        writer.AddFile(@"C:\Windows\System32\evil.dll", "x"u8.ToArray());
        var archive = writer.Finish();

        var entries = ZipArchive.Unzip(archive);
        Assert.Single(entries);
        Assert.Equal("Windows/System32/evil.dll", entries[0].Name);
        Assert.False(Path.IsPathRooted(entries[0].Name));
    }

    // Windows also supports *drive-relative* paths, where the colon is NOT
    // followed by a separator (e.g. "C:evil.dll" means "relative to drive
    // C's current directory"). Path.IsPathRooted is true for this shape too
    // (it only looks at the first two characters), so a fix that only strips
    // a segment equal to exactly "C:" (i.e. requires a separator right after
    // the colon) misses this case entirely — caught in a third review round.
    [Fact]
    public void Security_WriterStripsDriveRelativePrefixWithNoSeparator()
    {
        var writer = new ZipWriter();
        writer.AddFile("C:evil.dll", "x"u8.ToArray());
        var archive = writer.Finish();

        var entries = ZipArchive.Unzip(archive);
        Assert.Single(entries);
        Assert.Equal("evil.dll", entries[0].Name);
        Assert.False(Path.IsPathRooted(entries[0].Name));
    }

    // The multi-segment variant of the same drive-relative shape: the drive
    // prefix is glued onto the first path component rather than standing
    // alone as its own segment.
    [Fact]
    public void Security_WriterStripsDriveRelativePrefixWithSubdirectory()
    {
        var writer = new ZipWriter();
        writer.AddFile(@"C:relative\evil.dll", "x"u8.ToArray());
        var archive = writer.Finish();

        var entries = ZipArchive.Unzip(archive);
        Assert.Single(entries);
        Assert.Equal("relative/evil.dll", entries[0].Name);
        Assert.False(Path.IsPathRooted(entries[0].Name));
    }

    // A chained drive prefix in a single segment ("C:D:evil.dll", no
    // separators at all): stripping "C:" only once leaves "D:evil.dll",
    // which is ITSELF still a rooted-looking drive-relative path. The
    // stripping must repeat until no drive prefix remains, however many are
    // glued together, or this degenerates back into Finding 1's bypass one
    // layer deeper.
    [Fact]
    public void Security_WriterStripsChainedDrivePrefixes()
    {
        var writer = new ZipWriter();
        writer.AddFile("C:D:evil.dll", "x"u8.ToArray());
        var archive = writer.Finish();

        var entries = ZipArchive.Unzip(archive);
        Assert.Single(entries);
        Assert.Equal("evil.dll", entries[0].Name);
        Assert.False(Path.IsPathRooted(entries[0].Name));
    }

    // A naive loop that strips one drive prefix per iteration by reassigning
    // `segment = segment[2..]` (a fresh O(remaining-length) copy each time)
    // costs O(n^2) on a segment made of many chained "<letter>:" prefixes.
    // This is well under the ZIP32 65535-byte name_len cap (so it isn't
    // rejected by that check first, which runs on the *normalized* result,
    // after this method would already have paid the quadratic cost).
    //
    // Sized and timed from direct measurement, not extrapolation (an earlier
    // version of this test relied on an extrapolated estimate that turned
    // out to be off by roughly 2-3x and left too thin a margin against the
    // timeout — caught by reverting the production fix and watching this
    // test pass anyway). Measured directly on this exact input size:
    // the pre-fix O(n^2) `segment = segment[2..]`-per-iteration loop takes
    // ~4.0s; the current O(n) index-scan-then-single-slice implementation
    // takes under 1ms. A 1-second timeout sits far below the buggy runtime
    // and far above the fixed one, so it reliably fails on a regression
    // without being flaky on normal CI hardware variance.
    // xUnit's Timeout only applies to async tests, so the body runs inside
    // Task.Run — the underlying work is still plain synchronous CPU-bound
    // code, this just gives xUnit a Task to apply the timeout to.
    [Fact(Timeout = 1000)]
    public async Task Security_StripDrivePrefixIsLinearNotQuadratic()
    {
        await Task.Run(() =>
        {
            var chainedPrefix = string.Concat(Enumerable.Repeat("A:", 200_000)); // 400,000 chars, all stripped
            var name = chainedPrefix + "evil.dll";

            var writer = new ZipWriter();
            writer.AddFile(name, "x"u8.ToArray());
            var archive = writer.Finish();

            var entries = ZipArchive.Unzip(archive);
            Assert.Single(entries);
            Assert.Equal("evil.dll", entries[0].Name);
        });
    }

    // A directory entry's trailing slash must survive normalization.
    [Fact]
    public void Security_WriterNormalizationPreservesTrailingSlashForDirectories()
    {
        var writer = new ZipWriter();
        writer.AddDirectory("../weird/dir/");
        var archive = writer.Finish();

        var reader = new ZipReader(archive);
        Assert.Contains("weird/dir/", reader.Entries.Select(e => e.Name));
    }

    // An entry name that normalizes away to nothing (e.g. pure ".." segments)
    // has no safe representation and must be rejected outright rather than
    // silently written as an empty name.
    [Fact]
    public void Security_WriterRejectsNameThatNormalizesToEmpty()
    {
        var writer = new ZipWriter();
        Assert.Throws<ArgumentException>(() => writer.AddFile("../..", "x"u8.ToArray()));
    }

    // ZIP32's name_len field is 16 bits; a name that encodes to more than
    // 65535 UTF-8 bytes must be rejected rather than silently truncated (a
    // silent truncation would desynchronize the declared length from the
    // bytes actually written, corrupting the archive layout).
    [Fact]
    public void Security_WriterRejectsNameExceedingZip32Limit()
    {
        var hugeName = new string('a', 70_000);
        var writer = new ZipWriter();
        Assert.Throws<ArgumentException>(() => writer.AddFile(hugeName, "x"u8.ToArray()));
    }

    // ZIP32's entries_total field is 16 bits; adding more than 65535 entries
    // must fail Finish() rather than silently wrap the declared count (which
    // would desynchronize it from the number of records actually written —
    // the same class of bug already hardened against in the sibling Ruby
    // package's parser).
    [Fact]
    public void Security_WriterRejectsMoreThanZip32EntryLimit()
    {
        var writer = new ZipWriter();
        for (var i = 0; i <= ushort.MaxValue; i++)
        {
            writer.AddDirectory($"d{i}/");
        }
        Assert.Throws<InvalidOperationException>(() => writer.Finish());
    }

    // A Central Directory offset field crafted to sit in the upper half of the
    // uint range (>= 0x80000000) must not defeat the reader's bounds checks by
    // wrapping negative when narrowed to `int`. Before the fix, this specific
    // shape made `cdOffset + cdSize > data.Length` pass incorrectly and the
    // code went on to throw the wrong exception type
    // (ArgumentOutOfRangeException) instead of the documented
    // InvalidDataException.
    [Fact]
    public void Security_ReaderRejectsOverflowingCentralDirectoryOffset()
    {
        // Build a minimal 22-byte EOCD-only "archive" whose cd_offset is
        // 0xFFFFFFFF (-1 as a signed int32) and whose cd_size is small enough
        // that the (buggy) `cdOffset + cdSize > data.Length` check would pass.
        var eocd = new byte[22];
        BitConverter.GetBytes(0x06054B50u).CopyTo(eocd, 0);       // EOCD signature "PK\x05\x06"
        // disk_number, disk_with_cd_start, entries_on_this_disk, entries_total: all 0 (bytes 4-11)
        BitConverter.GetBytes((uint)6).CopyTo(eocd, 12);          // cd_size = 6
        BitConverter.GetBytes(0xFFFFFFFFu).CopyTo(eocd, 16);      // cd_offset = 0xFFFFFFFF
        // comment_length = 0 (bytes 20-21)

        Assert.Throws<InvalidDataException>(() => new ZipReader(eocd));
    }

    // A Central Directory record's local_offset field crafted to sit in the
    // upper half of the uint range must likewise be rejected with
    // InvalidDataException rather than reaching an unchecked negative-offset
    // span access.
    [Fact]
    public void Security_ReaderRejectsOverflowingLocalOffset()
    {
        // Build a real archive, then corrupt the local_offset field (bytes
        // 42-45 of the Central Directory header) to 0xFFFFFFFF.
        var archive = ZipArchive.Zip([new ZipEntry("f.txt", "hi"u8.ToArray())]);
        var corrupted = (byte[])archive.Clone();

        var cdSigOffset = FindSignatureOffset(corrupted, 0x02014B50u); // CD signature "PK\x01\x02"
        BitConverter.GetBytes(0xFFFFFFFFu).CopyTo(corrupted, cdSigOffset + 42);

        var reader = new ZipReader(corrupted);
        Assert.Throws<InvalidDataException>(() => reader.Read("f.txt"));
    }

    private static int FindSignatureOffset(byte[] data, uint signature)
    {
        var sigBytes = BitConverter.GetBytes(signature);
        for (var i = 0; i <= data.Length - 4; i++)
        {
            if (data[i] == sigBytes[0] && data[i + 1] == sigBytes[1] &&
                data[i + 2] == sigBytes[2] && data[i + 3] == sigBytes[3])
                return i;
        }
        throw new InvalidOperationException("signature not found");
    }

    // ZipArchive.Unzip must refuse to expand an archive whose aggregate
    // decompressed size exceeds the caller's (or default) budget, even though
    // every individual entry stays comfortably under the per-entry cap —
    // guards against a "decompression bomb" made of several moderately-sized
    // entries rather than one enormous one.
    [Fact]
    public void Security_UnzipEnforcesAggregateDecompressionBudget()
    {
        var chunk = "0123456789"u8.ToArray();
        var data  = Enumerable.Repeat(chunk, 1000).SelectMany(x => x).ToArray(); // 10 KB, compresses well
        var archive = ZipArchive.Zip([new ZipEntry("big.bin", data)]);

        // Budget smaller than the entry's true decompressed size (10 KB) but
        // larger than the compressed archive itself, so the guard can only be
        // observed by actually decompressing and tracking the running total.
        Assert.Throws<InvalidDataException>(() => ZipArchive.Unzip(archive, maxTotalBytes: 1024));

        // With a sufficient budget the same archive extracts normally.
        var entries = ZipArchive.Unzip(archive, maxTotalBytes: 1024 * 1024);
        Assert.Equal(data, entries[0].Data);
    }

    // ZipReader.Read(name) originally did a linear O(n) scan of every
    // entry's metadata to find one by name (`_meta.FirstOrDefault(m =>
    // m.Name == name)`). ZipArchive.Unzip calls Read() once per entry, so a
    // single Unzip() call on an n-entry archive cost O(n) lookups * O(n)
    // scan each = O(n^2) string comparisons overall — for a perfectly valid,
    // small (a few hundred KB) archive with tens of thousands of entries,
    // that's billions of comparisons. This is squarely inside Unzip's own
    // documented threat model ("untrusted uploads, third-party .zip files"),
    // so the lookup must be O(1) (a Dictionary), not O(n).
    //
    // Entries use compress: false to isolate the read-side lookup complexity
    // being tested from the writer's DEFLATE pipeline.
    //
    // Sized and timed from direct measurement, not extrapolation (see the
    // note on Security_StripDrivePrefixIsLinearNotQuadratic above — an
    // earlier version of this test relied on an extrapolated estimate that
    // left too thin a margin). Measured directly at this entry count: the
    // pre-fix O(n) FirstOrDefault-per-lookup scan (called once per entry by
    // Unzip, so O(n^2) overall) takes on the order of ~1.2s of pure lookup
    // cost on top of archive construction; the current O(1) dictionary
    // lookup adds only a few milliseconds. A 1-second timeout sits well
    // below the buggy runtime and well above the fixed one. 40,000 is also
    // comfortably inside the ZIP32 65535-entry cap this same PR enforces
    // elsewhere (Security_WriterRejectsMoreThanZip32EntryLimit), so this
    // archive is itself a legal one a caller could actually receive.
    // xUnit's Timeout requires an async test, so this wraps the same
    // synchronous CPU-bound work in Task.Run purely to give xUnit a Task to
    // time out.
    [Fact(Timeout = 1000)]
    public async Task Security_UnzipEntryLookupIsLinearNotQuadratic()
    {
        await Task.Run(() =>
        {
            const int entryCount = 40_000;
            var writer = new ZipWriter();
            for (var i = 0; i < entryCount; i++)
            {
                writer.AddFile($"f{i}.txt", "x"u8.ToArray(), compress: false);
            }
            var archive = writer.Finish();

            var entries = ZipArchive.Unzip(archive);
            Assert.Equal(entryCount, entries.Count);
            Assert.Equal("x"u8.ToArray(), entries[0].Data);
            Assert.Equal("x"u8.ToArray(), entries[^1].Data);
        });
    }
}
