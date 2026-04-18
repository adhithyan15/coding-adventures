using System.Buffers.Binary;
using System.Text;
using CodingAdventures.Lzss;

namespace CodingAdventures.Lzss.Tests;

public sealed class LzssTests
{
    private static byte[] EncodeText(string value) => Encoding.UTF8.GetBytes(value);

    [Fact]
    public void EmptyInput_ProducesNoTokens()
    {
        Assert.Empty(Lzss.Encode([]));
    }

    [Fact]
    public void SingleByte_ProducesLiteral()
    {
        Assert.Equal([new LzssLiteral((byte)'A')], Lzss.Encode(EncodeText("A")));
    }

    [Fact]
    public void Aabcbbabc_MatchesSpecVectorTail()
    {
        var tokens = Lzss.Encode(EncodeText("AABCBBABC"));
        Assert.Equal(7, tokens.Count);
        Assert.Equal(new LzssMatch(5, 3), tokens[^1]);
    }

    [Fact]
    public void Ababab_UsesMatchToken()
    {
        Assert.Equal(
            [new LzssLiteral((byte)'A'), new LzssLiteral((byte)'B'), new LzssMatch(2, 4)],
            Lzss.Encode(EncodeText("ABABAB")));
    }

    [Fact]
    public void AllSameBytes_UseSelfReferentialMatch()
    {
        Assert.Equal(
            [new LzssLiteral((byte)'A'), new LzssMatch(1, 6)],
            Lzss.Encode(EncodeText("AAAAAAA")));
    }

    [Fact]
    public void Decode_HandlesOverlappingMatches()
    {
        var output = Lzss.Decode([new LzssLiteral((byte)'A'), new LzssMatch(1, 6)], 7);
        Assert.Equal(EncodeText("AAAAAAA"), output);
    }

    [Theory]
    [InlineData("")]
    [InlineData("A")]
    [InlineData("ABCDE")]
    [InlineData("AAAAAAA")]
    [InlineData("ABABAB")]
    [InlineData("AABCBBABC")]
    [InlineData("hello world")]
    public void CompressAndDecompress_RoundTripAsciiInputs(string value)
    {
        var data = EncodeText(value);
        Assert.Equal(data, Lzss.Decompress(Lzss.Compress(data)));
    }

    [Fact]
    public void BinaryAndRepeatedData_RoundTrip()
    {
        var data = new byte[300];
        for (var index = 0; index < data.Length; index++)
        {
            data[index] = (byte)(index % 3);
        }

        Assert.Equal(data, Lzss.Decompress(Lzss.Compress(data)));
    }

    [Fact]
    public void SerialiseAndDeserialise_AreSymmetric()
    {
        var tokens = new List<LzssToken>
        {
            new LzssLiteral((byte)'A'),
            new LzssLiteral((byte)'B'),
            new LzssMatch(2, 4)
        };

        var bytes = Lzss.SerialiseTokens(tokens, 6);
        var (recovered, originalLength) = Lzss.DeserialiseTokens(bytes);

        Assert.Equal(6, originalLength);
        Assert.Equal(tokens, recovered);
    }

    [Fact]
    public void Deserialise_CapsCraftedLargeBlockCount()
    {
        var bad = new byte[16];
        BinaryPrimitives.WriteUInt32BigEndian(bad.AsSpan(4, 4), 0x40000000);
        var output = Lzss.Decompress(bad);
        Assert.NotNull(output);
    }

    [Fact]
    public void Decode_RejectsOffsetsBeforeOutputBuffer()
    {
        var error = Assert.Throws<InvalidOperationException>(() => Lzss.Decode([new LzssMatch(4, 1)]));
        Assert.Contains("before the output buffer", error.Message);
    }
}
