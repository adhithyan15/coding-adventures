using System.Text;
using CodingAdventures.Lz78;

namespace CodingAdventures.Lz78.Tests;

public sealed class Lz78Tests
{
    private static byte[] EncodeText(string value) => Encoding.UTF8.GetBytes(value);

    private static string DecodeText(byte[] value) => Encoding.UTF8.GetString(value);

    [Fact]
    public void EmptyInput_ProducesNoTokens()
    {
        Assert.Empty(Lz78.Encode([]));
        Assert.Empty(Lz78.Decode([], 0));
    }

    [Fact]
    public void SingleByte_ProducesSingleLiteralToken()
    {
        var tokens = Lz78.Encode(EncodeText("A"));
        Assert.Equal([new Lz78Token(0, (byte)'A')], tokens);
        Assert.Equal("A", DecodeText(Lz78.Decode(tokens, 1)));
    }

    [Fact]
    public void Aabcbbabc_MatchesSpecVector()
    {
        var tokens = Lz78.Encode(EncodeText("AABCBBABC"));
        Assert.Equal(
            [
                new Lz78Token(0, (byte)'A'),
                new Lz78Token(1, (byte)'B'),
                new Lz78Token(0, (byte)'C'),
                new Lz78Token(0, (byte)'B'),
                new Lz78Token(4, (byte)'A'),
                new Lz78Token(4, (byte)'C')
            ],
            tokens);
        Assert.Equal("AABCBBABC", DecodeText(Lz78.Decompress(Lz78.Compress(EncodeText("AABCBBABC")))));
    }

    [Fact]
    public void Ababab_UsesFlushToken()
    {
        var tokens = Lz78.Encode(EncodeText("ABABAB"));
        Assert.Equal(
            [
                new Lz78Token(0, (byte)'A'),
                new Lz78Token(0, (byte)'B'),
                new Lz78Token(1, (byte)'B'),
                new Lz78Token(3, 0)
            ],
            tokens);
    }

    [Theory]
    [InlineData("")]
    [InlineData("A")]
    [InlineData("ABCDE")]
    [InlineData("AAAAAAA")]
    [InlineData("ABABABAB")]
    [InlineData("hello world")]
    public void CompressAndDecompress_RoundTripAsciiInputs(string value)
    {
        var data = EncodeText(value);
        Assert.Equal(data, Lz78.Decompress(Lz78.Compress(data)));
    }

    [Fact]
    public void BinaryInputs_RoundTrip()
    {
        var data = new byte[] { 0, 0, 0, 255, 255, 0, 1, 2, 0, 1, 2 };
        Assert.Equal(data, Lz78.Decompress(Lz78.Compress(data)));
    }

    [Fact]
    public void MaxDictionarySize_IsRespected()
    {
        var tokens = Lz78.Encode(EncodeText("ABCABCABCABCABC"), maxDictSize: 10);
        Assert.All(tokens, token => Assert.InRange(token.DictIndex, 0, 9));
    }

    [Fact]
    public void MaxDictionarySizeOne_ForcesAllLiterals()
    {
        var tokens = Lz78.Encode(EncodeText("AAAA"), maxDictSize: 1);
        Assert.All(tokens, token => Assert.Equal(0, token.DictIndex));
    }

    [Fact]
    public void SerialiseAndDeserialise_AreSymmetric()
    {
        var tokens = new List<Lz78Token>
        {
            new(0, (byte)'A'),
            new(1, (byte)'B')
        };

        var serialised = Lz78.SerialiseTokens(tokens, 3);
        var (deserialised, originalLength) = Lz78.DeserialiseTokens(serialised);

        Assert.Equal(3, originalLength);
        Assert.Equal(tokens, deserialised);
    }

    [Fact]
    public void Decode_RejectsUnknownDictionaryIndex()
    {
        var error = Assert.Throws<InvalidOperationException>(() => Lz78.Decode([new Lz78Token(1, (byte)'A')]));
        Assert.Contains("does not exist", error.Message);
    }
}
