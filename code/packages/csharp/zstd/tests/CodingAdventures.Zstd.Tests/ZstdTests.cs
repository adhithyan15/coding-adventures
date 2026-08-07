using System.Diagnostics;
using System.Text;
using Xunit;

namespace CodingAdventures.Zstd.Tests;

public sealed class ZstdTests
{
    [Fact]
    public void EmptyInputRoundTripsAndProducesAFrame()
    {
        var compressed = Zstd.Compress([]);
        Assert.Empty(Zstd.Decompress(compressed));
        Assert.Equal(16, compressed.Length);
        Assert.Equal([0x28, 0xB5, 0x2F, 0xFD], compressed[..4]);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(0x42)]
    [InlineData(0xFF)]
    public void SingleBytesRoundTrip(int value)
    {
        var input = new[] { (byte)value };
        Assert.Equal(input, Zstd.Decompress(Zstd.Compress(input)));
    }

    [Fact]
    public void AllByteValuesRoundTripAsRawData()
    {
        var input = Enumerable.Range(0, 256).Select(value => (byte)value).ToArray();
        var compressed = Zstd.Compress(input);
        Assert.Equal(input, Zstd.Decompress(compressed));
        Assert.Equal(0, BlockTypes(compressed).Single());
    }

    [Theory]
    [InlineData(0)]
    [InlineData(0x41)]
    [InlineData(0xFF)]
    public void RepeatedBytesUseRle(int value)
    {
        var input = Enumerable.Repeat((byte)value, 1024).ToArray();
        var compressed = Zstd.Compress(input);
        Assert.Equal(input, Zstd.Decompress(compressed));
        Assert.True(compressed.Length < 30);
        Assert.Equal(1, BlockTypes(compressed).Single());
    }

    [Fact]
    public void EnglishProseUsesCompressedBlocks()
    {
        var input = Encoding.UTF8.GetBytes(string.Concat(Enumerable.Repeat("the quick brown fox jumps over the lazy dog ", 25)));
        var compressed = Zstd.Compress(input);
        Assert.Equal(input, Zstd.Decompress(compressed));
        Assert.True(compressed.Length < input.Length * 0.8);
        Assert.Equal(2, BlockTypes(compressed).Single());
    }

    [Fact]
    public void DeterministicBinaryDataRoundTrips()
    {
        uint seed = 42;
        var input = new byte[512];
        for (var index = 0; index < input.Length; index++)
        {
            seed = unchecked(seed * 1_664_525 + 1_013_904_223);
            input[index] = (byte)seed;
        }

        Assert.Equal(input, Zstd.Decompress(Zstd.Compress(input)));
    }

    [Fact]
    public void MultiBlockRleFrameRoundTrips()
    {
        var input = Enumerable.Repeat((byte)'x', 200 * 1024).ToArray();
        var compressed = Zstd.Compress(input);
        Assert.Equal(input, Zstd.Decompress(compressed));
        Assert.Equal([1, 1], BlockTypes(compressed));
        Assert.True(compressed.Length < 50);
    }

    [Fact]
    public void MultiBlockCompressedFrameRoundTrips()
    {
        var input = Encoding.ASCII.GetBytes(string.Concat(Enumerable.Repeat("ABCDEFGHIJKLMNOP", 10_000)));
        var compressed = Zstd.Compress(input);
        Assert.Equal(input, Zstd.Decompress(compressed));
        Assert.True(BlockTypes(compressed).Count > 1);
        Assert.All(BlockTypes(compressed), type => Assert.Equal(2, type));
    }

    [Fact]
    public void RepeatDistancePatternCompressesEfficiently()
    {
        var bytes = new List<byte>(Encoding.ASCII.GetBytes("ABCDEFGH"));
        for (var index = 0; index < 10; index++)
        {
            bytes.AddRange(Enumerable.Repeat((byte)'X', 128));
            bytes.AddRange(Encoding.ASCII.GetBytes("ABCDEFGH"));
        }

        var input = bytes.ToArray();
        var compressed = Zstd.Compress(input);
        Assert.Equal(input, Zstd.Decompress(compressed));
        Assert.True(compressed.Length < input.Length * 0.7);
    }

    [Fact]
    public void CompressionIsDeterministic()
    {
        var input = Encoding.ASCII.GetBytes(string.Concat(Enumerable.Repeat("hello zstd world! ", 50)));
        Assert.Equal(Zstd.Compress(input), Zstd.Compress(input));
    }

    [Fact]
    public void HandCraftedRawFrameDecodes()
    {
        byte[] frame = [0x28, 0xB5, 0x2F, 0xFD, 0x20, 0x05, 0x29, 0, 0, (byte)'h', (byte)'e', (byte)'l', (byte)'l', (byte)'o'];
        Assert.Equal(Encoding.ASCII.GetBytes("hello"), Zstd.Decompress(frame));
    }

    [Fact]
    public void HandCraftedRleFrameDecodes()
    {
        byte[] frame = [0x28, 0xB5, 0x2F, 0xFD, 0x20, 0x0A, 0x53, 0, 0, 0x41];
        Assert.Equal(Enumerable.Repeat((byte)'A', 10), Zstd.Decompress(frame));
    }

    [Fact]
    public void MultiSegmentHeadersAndChecksumsAreConsumed()
    {
        // Descriptor 0x04 sets ONLY Content_Checksum_Flag (bit 2, per RFC
        // 8878 §3.1.1.1 — see lessons.md Lesson 95). Bit 5 (Single_Segment)
        // is unset, so a 1-byte window descriptor follows.
        byte[] frame =
        [
            0x28, 0xB5, 0x2F, 0xFD,
            0x04, 0x00,
            0x09, 0, 0, (byte)'x',
            1, 2, 3, 4,
        ];
        Assert.Equal([(byte)'x'], Zstd.Decompress(frame));
    }

    [Fact]
    public void Bit4IsUnusedAndMustNotBeTreatedAsChecksumOrReserved()
    {
        // Descriptor 0x10 sets only bit 4 (Unused_bit). A conformant decoder
        // must ignore it entirely: no checksum trailer, no rejection. This
        // guards against reintroducing the Lesson 95 bit-4/bit-2 mixup.
        byte[] frame =
        [
            0x28, 0xB5, 0x2F, 0xFD,
            0x10, 0x00,
            0x09, 0, 0, (byte)'x',
        ];
        Assert.Equal([(byte)'x'], Zstd.Decompress(frame));
    }

    [Fact]
    public void DictionaryIdAndContentSizeFormsAreSkipped()
    {
        foreach (var descriptor in new byte[] { 0x21, 0x62, 0xA3 })
        {
            var dictionaryBytes = (descriptor & 3) switch { 1 => 1, 2 => 2, _ => 4 };
            var contentBytes = (descriptor >> 6) switch { 0 => 1, 1 => 2, 2 => 4, _ => 8 };
            var frame = new List<byte> { 0x28, 0xB5, 0x2F, 0xFD, descriptor };
            frame.AddRange(new byte[dictionaryBytes + contentBytes]);
            frame.AddRange([1, 0, 0]);
            Assert.Empty(Zstd.Decompress(frame.ToArray()));
        }
    }

    [Fact]
    public void NullInputsAreRejected()
    {
        Assert.Throws<ArgumentNullException>(() => Zstd.Compress(null!));
        Assert.Throws<ArgumentNullException>(() => Zstd.Decompress(null!));
    }

    [Theory]
    [MemberData(nameof(MalformedFrames))]
    public void MalformedFramesAreRejected(byte[] frame)
    {
        Assert.Throws<InvalidDataException>(() => Zstd.Decompress(frame));
    }

    public static TheoryData<byte[]> MalformedFrames => new()
    {
        new byte[] { },
        new byte[] { 0x28, 0xB5, 0x2F },
        Encoding.ASCII.GetBytes("not zstd data"),
        new byte[] { 0x28, 0xB5, 0x2F, 0xFD, 0x2C },
        new byte[] { 0x28, 0xB5, 0x2F, 0xFD, 0x20 },
        new byte[] { 0x28, 0xB5, 0x2F, 0xFD, 0x20, 0, 0x29, 0, 0, (byte)'h' },
        new byte[] { 0x28, 0xB5, 0x2F, 0xFD, 0x20, 0, 0x53, 0, 0 },
        new byte[] { 0x28, 0xB5, 0x2F, 0xFD, 0x20, 0, 0x07, 0, 0 },
        new byte[] { 0x28, 0xB5, 0x2F, 0xFD, 0x20, 0, 0x01, 0, 0, 0xFF },
        new byte[] { 0x28, 0xB5, 0x2F, 0xFD, 0x20, 0, 0x05, 0, 0 },
    };

    // ─── TC-9: real `zstd` CLI interoperability ────────────────────────────
    //
    // Every prior test in this file only ever round-trips through OUR OWN
    // encoder/decoder pair. That is necessary but never sufficient: an
    // encoder and decoder that systematically agree with each other on a
    // WRONG wire-format convention (bit order, table-construction algorithm)
    // pass every internal round-trip test while producing output no other
    // implementation can read. That is exactly the shape of the bug fixed
    // alongside this test — see lessons.md Lesson 96 and CHANGELOG.md.
    // Skipped (not failed) when the `zstd` binary isn't on PATH, since dev/CI
    // environments vary.
    [Fact]
    public void Tc9CliInterop()
    {
        if (!IsZstdCliAvailable())
        {
            return;
        }

        var text = string.Concat(Enumerable.Repeat("the quick brown fox jumps over the lazy dog ", 25));
        var original = Encoding.UTF8.GetBytes(text);

        // Direction 1: compress with ours, decompress with the real `zstd -d`.
        var ourCompressed = Zstd.Compress(original);
        var oursZst = Path.GetTempFileName();
        try
        {
            File.WriteAllBytes(oursZst, ourCompressed);
            var decodedByCli = RunZstd(["-d", "-q", "-c", oursZst]);
            Assert.Equal(original, decodedByCli);
        }
        finally
        {
            File.Delete(oursZst);
        }

        // Direction 2: compress with real `zstd`, decompress with ours.
        var theirsInput = Path.GetTempFileName();
        try
        {
            File.WriteAllBytes(theirsInput, original);
            var theirCompressed = RunZstd(["-q", "-c", theirsInput]);
            var decodedByUs = Zstd.Decompress(theirCompressed);
            Assert.Equal(original, decodedByUs);
        }
        finally
        {
            File.Delete(theirsInput);
        }
    }

    /// <summary>
    /// Real `zstd` CLI interop on an input large enough to push the
    /// compressor's single-block sequence count well past a handful of
    /// sequences, exercising the FSE sequences codec across many
    /// state-transition steps (not just the boundary case). A codec that
    /// gets the per-sequence field order or last-sequence special-case wrong
    /// (Lesson 96) fails on multi-sequence input even though a
    /// one-or-two-sequence smoke test can pass by coincidence.
    /// </summary>
    [Fact]
    public void RepeatingPatternCliInterop()
    {
        if (!IsZstdCliAvailable())
        {
            return;
        }

        var cycle = "ABCDEF"u8.ToArray();
        var original = new byte[9000];
        for (var index = 0; index < original.Length; index++)
        {
            original[index] = cycle[index % cycle.Length];
        }

        var ourCompressed = Zstd.Compress(original);
        var oursZst = Path.GetTempFileName();
        try
        {
            File.WriteAllBytes(oursZst, ourCompressed);
            var decodedByCli = RunZstd(["-d", "-q", "-c", oursZst]);
            Assert.Equal(original, decodedByCli);
        }
        finally
        {
            File.Delete(oursZst);
        }
    }

    /// <summary>
    /// Real `zstd` CLI interop proving Repeated-Offset (R1/R2/R3) sequence
    /// decoding (RFC 8878 §3.1.1.3.2.1.1, lessons.md Lesson 98).
    /// </summary>
    /// <remarks>
    /// This package's OWN encoder never emits an offset code below 2 (the
    /// minimum LZSS match offset is 1, so raw offset = offset + 3 is always
    /// &gt;= 4) — the "no repeat-offset shortcuts" simplification documented
    /// in the module doc comment on <c>EncodeSequences</c>/<c>DecompressBlock</c>
    /// is entirely an encoder-side choice. So neither this package's own
    /// Compress()/Decompress() round trip, nor <see cref="Tc9CliInterop"/>'s
    /// prose corpus, nor <see cref="RepeatingPatternCliInterop"/>'s single
    /// long repeat (which the real CLI encodes as one giant match, never
    /// needing to reuse a distance across sequences), ever exercises the
    /// repeat-offset DECODE path — a self-consistency gap this test closes
    /// by compressing with the real CLI, whose encoder uses repeat offsets
    /// constantly.
    ///
    /// <para>
    /// The input alternates a fixed 8-byte literal run with a long run of a
    /// single repeated byte, over and over — "repeated back-to-back matches
    /// at the same distance" in the literal sense: each `X` run is a match
    /// against the immediately preceding byte (distance 1), and the real
    /// `zstd` CLI's encoder represents a run like that as a sequence whose
    /// Offset_Value is a repeat-offset code (reusing distance 1 across
    /// repeats) rather than re-encoding the explicit offset every time. On
    /// the pre-fix decoder (this exact scenario, minus the register
    /// plumbing) this reliably threw
    /// <c>InvalidDataException: match offset exceeds decoded output</c>,
    /// because `rawOffset - 3` underflows for any Offset_Value &lt;= 3 —
    /// confirmed against the pre-fix code in this session (and independently
    /// in `c/zstd`, PR #9941, whose fuzz harness found the same gap first
    /// with a simpler single-repeated-byte input).
    /// </para>
    /// </remarks>
    [Fact]
    public void RepeatOffsetCliInterop()
    {
        if (!IsZstdCliAvailable())
        {
            return;
        }

        var bytes = new List<byte>(Encoding.ASCII.GetBytes("ABCDEFGH"));
        for (var index = 0; index < 20; index++)
        {
            bytes.AddRange(Enumerable.Repeat((byte)'X', 128));
            bytes.AddRange(Encoding.ASCII.GetBytes("ABCDEFGH"));
        }

        var original = bytes.ToArray();
        var theirsInput = Path.GetTempFileName();
        try
        {
            File.WriteAllBytes(theirsInput, original);
            var theirCompressed = RunZstd(["-q", "-c", theirsInput]);
            var decodedByUs = Zstd.Decompress(theirCompressed);
            Assert.Equal(original, decodedByUs);
        }
        finally
        {
            File.Delete(theirsInput);
        }
    }

    /// <summary>
    /// Same proof as <see cref="RepeatOffsetCliInterop"/>, using a single
    /// long run of one repeated byte — the exact minimal repro that
    /// surfaced this gap in `c/zstd` (PR #9941, lessons.md Lesson 98): the
    /// real CLI encodes 4713+ bytes of one repeated byte as a Compressed
    /// (not RLE) block containing one sequence with Offset_Value=1 — "reuse
    /// Repeated_Offset1", which starts at its default value of 1.
    /// </summary>
    [Fact]
    public void SingleByteRunCliInterop()
    {
        if (!IsZstdCliAvailable())
        {
            return;
        }

        var original = Enumerable.Repeat((byte)'Z', 4713).ToArray();
        var theirsInput = Path.GetTempFileName();
        try
        {
            File.WriteAllBytes(theirsInput, original);
            var theirCompressed = RunZstd(["-q", "-c", theirsInput]);
            var decodedByUs = Zstd.Decompress(theirCompressed);
            Assert.Equal(original, decodedByUs);
        }
        finally
        {
            File.Delete(theirsInput);
        }
    }

    private static bool IsZstdCliAvailable()
    {
        try
        {
            using var process = Process.Start(new ProcessStartInfo("zstd", "--version")
            {
                RedirectStandardOutput = true,
                RedirectStandardError = true,
            });
            if (process is null)
            {
                return false;
            }

            return process.WaitForExit(10_000) && process.ExitCode == 0;
        }
        catch (Exception ex) when (ex is System.ComponentModel.Win32Exception or InvalidOperationException)
        {
            return false;
        }
    }

    private static byte[] RunZstd(IEnumerable<string> args)
    {
        var startInfo = new ProcessStartInfo("zstd")
        {
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
        };
        foreach (var arg in args)
        {
            startInfo.ArgumentList.Add(arg);
        }

        using var process = Process.Start(startInfo) ?? throw new InvalidOperationException("failed to start zstd");

        // Both stdout and stderr are redirected, so both must be drained
        // concurrently with (not after) WaitForExit: if `zstd` writes enough
        // to stderr to fill the OS pipe buffer while nothing is reading it,
        // the child blocks on that write and the parent deadlocks blocked on
        // reading stdout. Draining both asynchronously up front avoids that.
        using var stdout = new MemoryStream();
        var stdoutTask = process.StandardOutput.BaseStream.CopyToAsync(stdout);
        var stderrTask = process.StandardError.ReadToEndAsync();

        if (!process.WaitForExit(30_000))
        {
            process.Kill();
            throw new InvalidOperationException("zstd CLI timed out");
        }

        stdoutTask.GetAwaiter().GetResult();
        var stderr = stderrTask.GetAwaiter().GetResult();

        if (process.ExitCode != 0)
        {
            throw new InvalidOperationException($"zstd CLI failed (exit {process.ExitCode}): {stderr}");
        }

        return stdout.ToArray();
    }

    private static List<int> BlockTypes(byte[] frame)
    {
        var position = 13;
        var result = new List<int>();
        var last = false;
        while (!last)
        {
            var header = frame[position] | (frame[position + 1] << 8) | (frame[position + 2] << 16);
            position += 3;
            last = (header & 1) != 0;
            var type = (header >> 1) & 3;
            var size = header >> 3;
            result.Add(type);
            position += type == 1 ? 1 : size;
        }

        return result;
    }
}
