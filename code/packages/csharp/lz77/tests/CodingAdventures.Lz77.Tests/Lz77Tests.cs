using System.Text;
using CodingAdventures.Lz77;

namespace CodingAdventures.Lz77.Tests;

public sealed class Lz77Tests
{
    private static byte[] EncodeText(string value) => Encoding.UTF8.GetBytes(value);

    private static string DecodeText(byte[] value) => Encoding.UTF8.GetString(value);

    [Fact]
    public void EmptyInput_RoundTripsAsNoTokens()
    {
        Assert.Empty(Lz77.Encode([]));
        Assert.Empty(Lz77.Decode([]));
    }

    [Fact]
    public void AllIdenticalBytes_UseOverlapBackreference()
    {
        var tokens = Lz77.Encode(EncodeText("AAAAAAA"));

        Assert.Equal(2, tokens.Count);
        Assert.Equal(new Lz77Token(0, 0, (byte)'A'), tokens[0]);
        Assert.Equal(1, tokens[1].Offset);
        Assert.Equal(5, tokens[1].Length);
        Assert.Equal((byte)'A', tokens[1].NextChar);
        Assert.Equal("AAAAAAA", DecodeText(Lz77.Decode(tokens)));
    }

    [Fact]
    public void RepeatedPair_UsesSingleBackreference()
    {
        var tokens = Lz77.Encode(EncodeText("ABABABAB"));

        Assert.Equal(3, tokens.Count);
        Assert.Equal(new Lz77Token(0, 0, (byte)'A'), tokens[0]);
        Assert.Equal(new Lz77Token(0, 0, (byte)'B'), tokens[1]);
        Assert.Equal(2, tokens[2].Offset);
        Assert.Equal(5, tokens[2].Length);
        Assert.Equal((byte)'B', tokens[2].NextChar);
        Assert.Equal("ABABABAB", DecodeText(Lz77.Decode(tokens)));
    }

    [Fact]
    public void Aabcbbabc_DefaultMinMatchLeavesAllLiterals()
    {
        var tokens = Lz77.Encode(EncodeText("AABCBBABC"));

        Assert.Equal(9, tokens.Count);
        Assert.All(tokens, token =>
        {
            Assert.Equal(0, token.Offset);
            Assert.Equal(0, token.Length);
        });
    }

    [Theory]
    [InlineData("")]
    [InlineData("A")]
    [InlineData("ABCDE")]
    [InlineData("hello world")]
    [InlineData("ABABABABAB")]
    public void CompressAndDecompress_RoundTripKnownInputs(string value)
    {
        var data = EncodeText(value);
        Assert.Equal(data, Lz77.Decompress(Lz77.Compress(data)));
    }

    [Fact]
    public void Decode_UsesInitialBufferForStreamingStyleBackreferences()
    {
        var result = Lz77.Decode([new Lz77Token(2, 3, (byte)'Z')], [(byte)'A', (byte)'B']);
        Assert.Equal("ABABAZ", DecodeText(result));
    }

    [Fact]
    public void Encode_RespectsWindowAndMaxMatchLimits()
    {
        var data = new byte[1000];
        Array.Fill(data, (byte)'A');
        var tokens = Lz77.Encode(data, windowSize: 100, maxMatch: 50);

        Assert.All(tokens, token =>
        {
            Assert.InRange(token.Offset, 0, 100);
            Assert.InRange(token.Length, 0, 50);
        });
    }

    [Fact]
    public void SerialiseAndDeserialise_AreInverseForTeachingFormat()
    {
        var tokens = new List<Lz77Token>
        {
            new(0, 0, (byte)'A'),
            new(2, 5, (byte)'B'),
            new(1, 3, (byte)'C')
        };

        var serialised = Lz77.SerialiseTokens(tokens);
        var deserialised = Lz77.DeserialiseTokens(serialised);

        Assert.Equal(tokens, deserialised);
    }

    [Fact]
    public void Decode_RejectsOffsetsBeforeTheOutputBuffer()
    {
        var error = Assert.Throws<InvalidOperationException>(() => Lz77.Decode([new Lz77Token(4, 1, (byte)'A')]));
        Assert.Contains("before the output buffer", error.Message);
    }
}
