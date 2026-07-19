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
        byte[] frame =
        [
            0x28, 0xB5, 0x2F, 0xFD,
            0x10, 0x00,
            0x09, 0, 0, (byte)'x',
            1, 2, 3, 4,
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
