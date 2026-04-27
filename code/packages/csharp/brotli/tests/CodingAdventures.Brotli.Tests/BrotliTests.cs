using System.Buffers.Binary;
using System.Text;
using CodingAdventures.Brotli;

namespace CodingAdventures.Brotli.Tests;

public sealed class BrotliTests
{
    [Fact]
    public void EmptyInputUsesMinimalEncoding()
    {
        var compressed = Brotli.Compress([]);
        Assert.Equal(13, compressed.Length);
        Assert.Equal(0u, BinaryPrimitives.ReadUInt32BigEndian(compressed.AsSpan(0, 4)));
        Assert.Equal((byte)1, compressed[4]);
        Assert.Equal((byte)0, compressed[5]);
        Assert.Equal([], Brotli.Decompress(compressed));
    }

    [Theory]
    [InlineData((byte)0x42)]
    [InlineData((byte)0x00)]
    [InlineData((byte)0xFF)]
    public void SingleByteValuesRoundTrip(byte value)
    {
        var data = new[] { value };
        Assert.Equal(data, Brotli.Decompress(Brotli.Compress(data)));
    }

    [Fact]
    public void RepeatedADataCompressesWell()
    {
        var data = new byte[1024];
        Array.Fill(data, (byte)'A');
        var compressed = Brotli.Compress(data);

        Assert.Equal(data, Brotli.Decompress(compressed));
        Assert.True(compressed.Length < data.Length / 2.0);
        Assert.True(compressed[5] > 0);
    }

    [Fact]
    public void EnglishProseRoundTrips()
    {
        var data = Bytes(string.Concat(Enumerable.Repeat(
            "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs. ",
            16)));

        var compressed = Brotli.Compress(data);
        Assert.Equal(data, Brotli.Decompress(compressed));
        Assert.True(compressed.Length < data.Length * 0.8);
    }

    [Fact]
    public void BinaryDataRoundTrips()
    {
        var data = new byte[512];
        uint state = 0xDEADBEEF;
        for (var index = 0; index < data.Length; index++)
        {
            state = (state >> 1) ^ ((state & 1) == 0 ? 0u : 0xEDB88320u);
            data[index] = (byte)(state & 0xFF);
        }

        Assert.Equal(data, Brotli.Decompress(Brotli.Compress(data)));
    }

    [Fact]
    public void ContextTransitionsRoundTrip()
    {
        var data = Bytes("abc123ABCabc");
        var compressed = Brotli.Compress(data);

        Assert.Equal(data, Brotli.Decompress(compressed));
        Assert.True(compressed[6] > 0);
        Assert.True(compressed[7] > 0);
        Assert.True(compressed[8] > 0);
    }

    [Fact]
    public void LongDistanceMatchRoundTrips()
    {
        var marker = Bytes("XYZABCDEFG");
        var filler = Enumerable.Repeat((byte)'B', 4200).ToArray();
        var data = new byte[marker.Length + filler.Length + marker.Length];
        marker.CopyTo(data, 0);
        filler.CopyTo(data, marker.Length);
        marker.CopyTo(data, marker.Length + filler.Length);

        var compressed = Brotli.Compress(data);
        Assert.Equal(data, Brotli.Decompress(compressed));
        Assert.True(compressed[5] > 0);
    }

    [Fact]
    public void CompressionIsDeterministic()
    {
        var data = Bytes("The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog.");
        var a = Brotli.Compress(data);
        var b = Brotli.Compress(data);
        Assert.Equal(a, b);
    }

    [Fact]
    public void ManualPayloadForSingleADecompresses()
    {
        var payload = new byte[]
        {
            0x00, 0x00, 0x00, 0x01,
            0x01,
            0x00,
            0x01,
            0x00,
            0x00,
            0x00,
            0x3F, 0x01,
            0x00, 0x41, 0x01,
            0x00
        };

        Assert.Equal(Bytes("A"), Brotli.Decompress(payload));
    }

    [Fact]
    public void ManualPayloadForEmptyInputDecompresses()
    {
        var payload = new byte[]
        {
            0x00, 0x00, 0x00, 0x00,
            0x01,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x3F, 0x01,
            0x00
        };

        Assert.Equal([], Brotli.Decompress(payload));
    }

    [Fact]
    public void HeaderStoresOriginalLength()
    {
        var data = Bytes("Hello, Brotli!");
        var compressed = Brotli.Compress(data);
        Assert.Equal((uint)data.Length, BinaryPrimitives.ReadUInt32BigEndian(compressed.AsSpan(0, 4)));
    }

    [Fact]
    public void IccSentinelAlwaysPresent()
    {
        var compressed = Brotli.Compress(Bytes("test"));
        Assert.True(compressed[4] > 0);
    }

    [Fact]
    public void AllDistinctByteValuesRoundTrip()
    {
        var data = Enumerable.Range(0, 256).Select(value => (byte)value).ToArray();
        Assert.Equal(data, Brotli.Decompress(Brotli.Compress(data)));
    }

    private static byte[] Bytes(string value) => Encoding.UTF8.GetBytes(value);
}
