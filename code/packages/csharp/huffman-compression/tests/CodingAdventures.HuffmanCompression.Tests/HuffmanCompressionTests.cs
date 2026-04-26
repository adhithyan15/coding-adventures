using System.Buffers.Binary;
using System.Text;

namespace CodingAdventures.HuffmanCompression.Tests;

public sealed class HuffmanCompressionTests
{
    [Theory]
    [InlineData("AAABBC")]
    [InlineData("hello world")]
    [InlineData("Lorem ipsum dolor sit amet, consectetur adipiscing elit.")]
    [InlineData("ABCABCABCABC")]
    [InlineData("line\nline\nline\n")]
    public void CompressAndDecompress_RoundTripTextInputs(string value)
    {
        var data = Encoding.UTF8.GetBytes(value);
        Assert.Equal(data, HuffmanCompression.Decompress(HuffmanCompression.Compress(data)));
    }

    [Fact]
    public void CompressAndDecompress_RoundTripAllByteValues()
    {
        var data = Enumerable.Range(0, 256).Select(value => (byte)value).ToArray();
        Assert.Equal(data, HuffmanCompression.Decompress(HuffmanCompression.Compress(data)));
    }

    [Fact]
    public void CompressAndDecompress_RoundTripRepeatedAllByteValues()
    {
        var data = Repeat(Enumerable.Range(0, 256).Select(value => (byte)value).ToArray(), 10);
        Assert.Equal(data, HuffmanCompression.Decompress(HuffmanCompression.Compress(data)));
    }

    [Fact]
    public void CompressAndDecompress_RoundTripBinaryData()
    {
        var data = new byte[] { 0, 1, 2, 3, 255, 254, 253, 128, 64, 32, 0, 255 };
        Assert.Equal(data, HuffmanCompression.Decompress(HuffmanCompression.Compress(data)));
    }

    [Theory]
    [InlineData(0)]
    [InlineData(65)]
    [InlineData(127)]
    [InlineData(255)]
    public void SingleSymbolInputs_RoundTrip(int symbol)
    {
        var data = Enumerable.Repeat((byte)symbol, 50).ToArray();
        Assert.Equal(data, HuffmanCompression.Decompress(HuffmanCompression.Compress(data)));
    }

    [Fact]
    public void EmptyAndNullCompress_ProduceHeaderOnly()
    {
        Assert.Equal(new byte[8], HuffmanCompression.Compress([]));
        Assert.Equal(new byte[8], HuffmanCompression.Compress(null));
    }

    [Fact]
    public void EmptyAndShortDecompress_ReturnEmpty()
    {
        Assert.Empty(HuffmanCompression.Decompress([]));
        Assert.Empty(HuffmanCompression.Decompress(null));
        Assert.Empty(HuffmanCompression.Decompress([0, 0, 0, 0]));
        Assert.Empty(HuffmanCompression.Decompress(HuffmanCompression.Compress([])));
    }

    [Fact]
    public void Aaabbc_MatchesExactCmp04WireBytes()
    {
        var result = HuffmanCompression.Compress(Encoding.ASCII.GetBytes("AAABBC"));

        Assert.Equal(6u, BinaryPrimitives.ReadUInt32BigEndian(result.AsSpan(0, 4)));
        Assert.Equal(3u, BinaryPrimitives.ReadUInt32BigEndian(result.AsSpan(4, 4)));
        Assert.Equal(
            new byte[]
            {
                0x00, 0x00, 0x00, 0x06,
                0x00, 0x00, 0x00, 0x03,
                0x41, 0x01,
                0x42, 0x02,
                0x43, 0x02,
                0xA8, 0x01
            },
            result);
    }

    [Theory]
    [InlineData(1)]
    [InlineData(5)]
    [InlineData(100)]
    [InlineData(1000)]
    public void WireFormat_StoresOriginalLength(int length)
    {
        var data = Enumerable.Repeat((byte)'A', length).ToArray();
        var compressed = HuffmanCompression.Compress(data);
        Assert.Equal((uint)length, BinaryPrimitives.ReadUInt32BigEndian(compressed.AsSpan(0, 4)));
    }

    [Fact]
    public void WireFormat_StoresSortedCodeLengths()
    {
        var result = HuffmanCompression.Compress(Encoding.ASCII.GetBytes("AAABBC"));
        var symbolCount = (int)BinaryPrimitives.ReadUInt32BigEndian(result.AsSpan(4, 4));
        var previousLength = 0;
        var previousSymbol = -1;

        for (var index = 0; index < symbolCount; index++)
        {
            var symbol = result[8 + (index * 2)];
            var length = result[8 + (index * 2) + 1];
            Assert.True(length > previousLength || (length == previousLength && symbol > previousSymbol));
            previousLength = length;
            previousSymbol = symbol;
        }
    }

    [Fact]
    public void SingleByteInput_UsesOneBitCode()
    {
        var result = HuffmanCompression.Compress(Encoding.ASCII.GetBytes("A"));

        Assert.Equal(1u, BinaryPrimitives.ReadUInt32BigEndian(result.AsSpan(0, 4)));
        Assert.Equal(1u, BinaryPrimitives.ReadUInt32BigEndian(result.AsSpan(4, 4)));
        Assert.Equal((byte)'A', result[8]);
        Assert.Equal(1, result[9]);
        Assert.Equal(0, result[10]);
        Assert.Equal(11, result.Length);
    }

    [Fact]
    public void CompressibleInputs_Shrink()
    {
        var data = Enumerable.Repeat((byte)'A', 900)
            .Concat(Enumerable.Repeat((byte)'B', 100))
            .ToArray();
        var compressed = HuffmanCompression.Compress(data);

        Assert.True(compressed.Length < data.Length);
    }

    [Fact]
    public void UniformSmallInput_CanBeLargerThanOriginal()
    {
        var data = Enumerable.Range(0, 256).Select(value => (byte)value).ToArray();
        var compressed = HuffmanCompression.Compress(data);

        Assert.True(compressed.Length > data.Length);
    }

    [Fact]
    public void Compression_IsDeterministic()
    {
        var data = Encoding.ASCII.GetBytes("the quick brown fox jumps over the lazy dog");
        Assert.Equal(HuffmanCompression.Compress(data), HuffmanCompression.Compress(data));
    }

    [Fact]
    public void Decompress_ThrowsWhenBitStreamIsExhausted()
    {
        var truncated = new byte[11];
        BinaryPrimitives.WriteUInt32BigEndian(truncated.AsSpan(0, 4), 100);
        BinaryPrimitives.WriteUInt32BigEndian(truncated.AsSpan(4, 4), 1);
        truncated[8] = (byte)'A';
        truncated[9] = 1;
        truncated[10] = 0;

        var error = Assert.Throws<InvalidOperationException>(() => HuffmanCompression.Decompress(truncated));
        Assert.Contains("exhausted", error.Message);
    }

    [Fact]
    public void Decompress_ThrowsWhenTableIsTruncated()
    {
        var truncated = new byte[9];
        BinaryPrimitives.WriteUInt32BigEndian(truncated.AsSpan(0, 4), 1);
        BinaryPrimitives.WriteUInt32BigEndian(truncated.AsSpan(4, 4), 1);
        truncated[8] = (byte)'A';

        var error = Assert.Throws<InvalidOperationException>(() => HuffmanCompression.Decompress(truncated));
        Assert.Contains("code-length table", error.Message);
    }

    [Fact]
    public void Decompress_RejectsZeroLengthCodes()
    {
        var malformed = new byte[11];
        BinaryPrimitives.WriteUInt32BigEndian(malformed.AsSpan(0, 4), 1);
        BinaryPrimitives.WriteUInt32BigEndian(malformed.AsSpan(4, 4), 1);
        malformed[8] = (byte)'A';
        malformed[9] = 0;
        malformed[10] = 0;

        var error = Assert.Throws<InvalidOperationException>(() => HuffmanCompression.Decompress(malformed));
        Assert.Contains("positive", error.Message);
    }

    private static byte[] Repeat(byte[] source, int times)
    {
        var output = new byte[source.Length * times];
        for (var index = 0; index < times; index++)
        {
            source.CopyTo(output.AsSpan(index * source.Length));
        }

        return output;
    }
}
