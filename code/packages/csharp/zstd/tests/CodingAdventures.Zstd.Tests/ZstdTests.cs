// ZStd (CMP07) — xunit test suite.
//
// Every test round-trips data through Zstd.Compress → Zstd.Decompress and
// checks bit-for-bit equality with the original. Several tests also verify
// specific compression-ratio guarantees to ensure the LZ77 + FSE pipeline
// is actually compressing and not just copying.

using System.Text;
using CodingAdventures.Zstd;

namespace CodingAdventures.Zstd.Tests;

public class ZstdTests
{
    // ── Helper ────────────────────────────────────────────────────────────────

    // Round-trip: compress then decompress.
    private static byte[] Rt(byte[] data) =>
        Zstd.Decompress(Zstd.Compress(data));

    // ── TC-1: empty input ─────────────────────────────────────────────────────

    [Fact]
    public void Tc1_Empty()
    {
        // An empty input must produce a valid ZStd frame and decompress back
        // to empty bytes without panic or error.
        Assert.Equal(Array.Empty<byte>(), Rt(Array.Empty<byte>()));
    }

    // ── TC-2: literal-only short input ────────────────────────────────────────

    [Fact]
    public void Tc2_Literal()
    {
        // The smallest non-empty input: one byte, and a short string.
        Assert.Equal(new byte[] { 0x42 }, Rt(new byte[] { 0x42 }));
        byte[] hello = Encoding.UTF8.GetBytes("hello");
        Assert.Equal(hello, Rt(hello));
    }

    // ── TC-3: all 256 byte values ─────────────────────────────────────────────

    [Fact]
    public void Tc3_AllBytes()
    {
        // Every possible byte value 0x00..=0xFF in order. This exercises
        // literal encoding of non-ASCII and zero bytes.
        var input = new byte[256];
        for (int i = 0; i < 256; i++) input[i] = (byte)i;
        Assert.Equal(input, Rt(input));
    }

    // ── TC-4: RLE block ───────────────────────────────────────────────────────

    [Fact]
    public void Tc4_RleBlock()
    {
        // 1024 identical bytes should be detected as an RLE block.
        // Expected compressed size: 4 (magic) + 1 (FHD) + 8 (FCS) + 3 (block header)
        //                         + 1 (RLE byte) = 17 bytes < 30.
        var input = new byte[1024];
        Array.Fill(input, (byte)'A');
        byte[] compressed = Zstd.Compress(input);
        Assert.Equal(input, Zstd.Decompress(compressed));
        Assert.True(
            compressed.Length < 30,
            $"RLE of 1024 bytes compressed to {compressed.Length} (expected < 30)");
    }

    // ── TC-5: English prose ───────────────────────────────────────────────────

    [Fact]
    public void Tc5_Prose()
    {
        // Repeated English text has strong LZ77 matches. Must achieve ≥ 20%
        // compression (output ≤ 80% of input size).
        string text = string.Concat(Enumerable.Repeat("the quick brown fox jumps over the lazy dog ", 25));
        byte[] input = Encoding.UTF8.GetBytes(text);
        byte[] compressed = Zstd.Compress(input);
        Assert.Equal(input, Zstd.Decompress(compressed));
        int threshold = input.Length * 80 / 100;
        Assert.True(
            compressed.Length < threshold,
            $"prose: compressed {compressed.Length} bytes (input {input.Length}), expected < {threshold} (80%)");
    }

    // ── TC-6: pseudo-random data (deterministic) ──────────────────────────────

    [Fact]
    public void Tc6_Random()
    {
        // 1000 bytes incrementing mod 251 (deterministic "random").
        // No significant compression expected, but round-trip must be exact.
        var input = new byte[1000];
        for (int i = 0; i < 1000; i++) input[i] = (byte)(i % 251);
        Assert.Equal(input, Rt(input));
    }

    // ── TC-7: multi-block (300 KB) ────────────────────────────────────────────

    [Fact]
    public void Tc7_Multiblock()
    {
        // 300 KB > MaxBlockSize (128 KB), so this requires at least 2 blocks.
        // Use a repeating pattern so it's actually compressible.
        var pattern = Encoding.UTF8.GetBytes("Hello World ZStd! ");
        var input = new byte[300 * 1024];
        for (int i = 0; i < input.Length; i++) input[i] = pattern[i % pattern.Length];
        Assert.Equal(input, Rt(input));
    }

    // ── TC-8: repeat-offset pattern ───────────────────────────────────────────

    [Fact]
    public void Tc8_RepeatOffset()
    {
        // "abcabc..." 1000 bytes — strong LZ77 match potential.
        var input = new byte[1000];
        for (int i = 0; i < 1000; i++) input[i] = (byte)('a' + (i % 3));
        Assert.Equal(input, Rt(input));
    }

    // ── TC-9: deterministic output ────────────────────────────────────────────

    [Fact]
    public void Tc9_Deterministic()
    {
        // Compressing the same data twice must produce identical bytes.
        // This is required for reproducible builds and cache invalidation.
        byte[] data = Encoding.UTF8.GetBytes(string.Concat(Enumerable.Repeat("hello, ZStd world! ", 50)));
        Assert.Equal(Zstd.Compress(data), Zstd.Compress(data));
    }

    // ── RtRepeatedPattern ─────────────────────────────────────────────────────

    [Fact]
    public void RtRepeatedPattern()
    {
        // "Hello World! " × 1000
        byte[] data = Encoding.UTF8.GetBytes(string.Concat(Enumerable.Repeat("Hello World! ", 1000)));
        Assert.Equal(data, Rt(data));
    }

    // ── RtBinaryData ─────────────────────────────────────────────────────────

    [Fact]
    public void RtBinaryData()
    {
        // bytes 0..255 repeated 10×
        var input = new byte[2560];
        for (int i = 0; i < 2560; i++) input[i] = (byte)(i % 256);
        Assert.Equal(input, Rt(input));
    }

    // ── TestRevBitWriterRevBitReaderRoundtrip ─────────────────────────────────

    [Fact]
    public void TestRevBitWriterRevBitReaderRoundtrip()
    {
        // The backward bit stream stores bits so the LAST-written bits are
        // read FIRST by the decoder. This mirrors how ZStd's sequence codec
        // writes the initial FSE states last (so the decoder reads them first).
        //
        // Write order:  A=0b101 (3 bits), B=0b11001100 (8 bits), C=0b1 (1 bit)
        // Read order:   C first, then B, then A  (reversed)
        var bw = new Zstd.RevBitWriter();
        bw.AddBits(0b101UL, 3);       // A — written first → read last
        bw.AddBits(0b11001100UL, 8);  // B
        bw.AddBits(0b1UL, 1);         // C — written last → read first
        bw.Flush();
        byte[] buf = bw.Finish();

        var br = new Zstd.RevBitReader(buf);
        Assert.Equal(0b1UL,        br.ReadBits(1)); // C: last written, first read
        Assert.Equal(0b11001100UL, br.ReadBits(8)); // B
        Assert.Equal(0b101UL,      br.ReadBits(3)); // A: first written, last read
    }

    // ── TestFseDecodeTableCoverage ────────────────────────────────────────────

    [Fact]
    public void TestFseDecodeTableCoverage()
    {
        // Every slot in the decode table should be reachable (sym is valid).
        // We use the LL table as the representative case.
        short[] llNorm = [4,3,2,2,2,2,2,2,2,2,2,2,2,1,1,1, 2,2,2,2,2,2,2,2,2,3,2,1,1,1,1,1, -1,-1,-1,-1];
        var dt = Zstd.BuildDecodeTable(llNorm, 6);
        Assert.Equal(1 << 6, dt.Length);
        foreach (var cell in dt)
        {
            Assert.True(cell.Sym < llNorm.Length,
                $"sym={cell.Sym} out of range (norm.Length={llNorm.Length})");
        }
    }

    // ── TestLlToCodeSmall ─────────────────────────────────────────────────────

    [Fact]
    public void TestLlToCodeSmall()
    {
        // First 16 LL code mappings are identity: ll=0 → code 0, ll=1 → code 1, etc.
        for (int i = 0; i < 16; i++)
        {
            int got = Zstd.LlToCode((uint)i);
            Assert.True(got == i, $"LL code for literal length {i}: expected {i} got {got}");
        }
    }

    // ── TestMlToCodeSmall ─────────────────────────────────────────────────────

    [Fact]
    public void TestMlToCodeSmall()
    {
        // ML code 0 maps to match length 3 (minimum match in ZStd).
        Assert.Equal(0, Zstd.MlToCode(3));
        // Lengths 3..34 are identity-mapped: ml=3 → code 0, ml=4 → code 1, ...
        for (int i = 3; i < 35; i++)
        {
            int got = Zstd.MlToCode((uint)i);
            int expected = i - 3;
            Assert.True(got == expected, $"ML code for match length {i}: expected {expected} got {got}");
        }
    }

    // ── Additional round-trip tests ───────────────────────────────────────────

    [Fact]
    public void RtAllZeros()
    {
        var input = new byte[1000];
        Assert.Equal(input, Rt(input));
    }

    [Fact]
    public void RtAllFf()
    {
        var input = new byte[1000];
        Array.Fill(input, (byte)0xFF);
        Assert.Equal(input, Rt(input));
    }

    [Fact]
    public void RtHelloWorld()
    {
        byte[] hw = Encoding.UTF8.GetBytes("hello world");
        Assert.Equal(hw, Rt(hw));
    }

    [Fact]
    public void RtLargeRle()
    {
        // 200 KB all-same bytes — requires 2 RLE blocks.
        var input = new byte[200 * 1024];
        Array.Fill(input, (byte)'x');
        Assert.Equal(input, Rt(input));
    }

    [Fact]
    public void RtRepeatAbc()
    {
        // "ABCDEF" repeating 500 times.
        var pattern = new byte[] { (byte)'A', (byte)'B', (byte)'C', (byte)'D', (byte)'E', (byte)'F' };
        var input = new byte[3000];
        for (int i = 0; i < 3000; i++) input[i] = pattern[i % 6];
        Assert.Equal(input, Rt(input));
    }
}
