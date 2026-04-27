using System.Buffers.Binary;
using System.Text;
using CodingAdventures.Deflate;

namespace CodingAdventures.Deflate.Tests;

public sealed class DeflateTests
{
    [Fact]
    public void EmptyInputUsesMinimalHeader()
    {
        var compressed = Deflate.Compress([]);
        Assert.Equal(12, compressed.Length);
        Assert.Equal(0u, BinaryPrimitives.ReadUInt32BigEndian(compressed.AsSpan(0, 4)));
        Assert.Equal((ushort)1, BinaryPrimitives.ReadUInt16BigEndian(compressed.AsSpan(4, 2)));
        Assert.Equal((ushort)0, BinaryPrimitives.ReadUInt16BigEndian(compressed.AsSpan(6, 2)));
        Assert.Equal([], Deflate.Decompress(compressed));
    }

    [Theory]
    [InlineData((byte)0x00)]
    [InlineData((byte)0xFF)]
    public void SingleByteValuesRoundTrip(byte value)
    {
        var data = new[] { value };
        Assert.Equal(data, Deflate.Decompress(Deflate.Compress(data)));
    }

    [Fact]
    public void LiteralOnlyExampleHasNoDistanceTree()
    {
        var data = Bytes("AAABBC");
        var compressed = Deflate.Compress(data);
        Assert.Equal(data, Deflate.Decompress(compressed));
        Assert.Equal((ushort)0, BinaryPrimitives.ReadUInt16BigEndian(compressed.AsSpan(6, 2)));
    }

    [Fact]
    public void SpecMatchExampleHasDistanceTree()
    {
        var data = Bytes("AABCBBABC");
        var compressed = Deflate.Compress(data);
        Assert.Equal((uint)9, BinaryPrimitives.ReadUInt32BigEndian(compressed.AsSpan(0, 4)));
        Assert.True(BinaryPrimitives.ReadUInt16BigEndian(compressed.AsSpan(6, 2)) > 0);
        Assert.Equal(data, Deflate.Decompress(compressed));
    }

    [Theory]
    [InlineData("AAAAAAA")]
    [InlineData("ABABABABABAB")]
    [InlineData("ABCABCABCABC")]
    [InlineData("hello hello hello world")]
    [InlineData("AABABC")]
    public void MatchHeavyExamplesRoundTrip(string value)
    {
        var data = Bytes(value);
        Assert.Equal(data, Deflate.Decompress(Deflate.Compress(data)));
    }

    [Fact]
    public void LongRepetitiveTextRoundTrips()
    {
        var data = Bytes(string.Concat(Enumerable.Repeat("the quick brown fox jumps over the lazy dog ", 10)));
        Assert.Equal(data, Deflate.Decompress(Deflate.Compress(data)));
    }

    [Fact]
    public void BinaryDataRoundTrips()
    {
        var data = new byte[1000];
        for (var index = 0; index < data.Length; index++)
        {
            data[index] = (byte)(index % 256);
        }

        Assert.Equal(data, Deflate.Decompress(Deflate.Compress(data)));
    }

    [Fact]
    public void RepetitiveDataCompressesBelowHalf()
    {
        var baseBytes = Bytes("ABCABC");
        var data = new byte[baseBytes.Length * 100];
        for (var index = 0; index < 100; index++)
        {
            baseBytes.CopyTo(data, index * baseBytes.Length);
        }

        var compressed = Deflate.Compress(data);
        Assert.True(compressed.Length < data.Length / 2.0);
    }

    private static byte[] Bytes(string value) => Encoding.UTF8.GetBytes(value);
}
