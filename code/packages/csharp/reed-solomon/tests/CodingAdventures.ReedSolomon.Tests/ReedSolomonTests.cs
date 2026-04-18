using System.Text;
using CodingAdventures.ReedSolomon;
using FieldMath = CodingAdventures.Gf256.Gf256;

namespace CodingAdventures.ReedSolomon.Tests;

internal static class Helpers
{
    public static byte[] Bytes(string value) => Encoding.UTF8.GetBytes(value);

    public static byte[] Corrupt(byte[] codeword, int[] positions, byte mask)
    {
        var clone = (byte[])codeword.Clone();
        foreach (var position in positions)
        {
            clone[position] ^= mask;
        }

        return clone;
    }
}

public sealed class BuildGeneratorTests
{
    [Fact]
    public void NCheckTwoMatchesKnownVector()
    {
        Assert.Equal([8, 6, 1], ReedSolomon.BuildGenerator(2));
    }

    [Fact]
    public void GeneratorIsMonicAndHasExpectedDegree()
    {
        var generator = ReedSolomon.BuildGenerator(8);
        Assert.Equal(9, generator.Length);
        Assert.Equal(1, generator[^1]);
    }

    [Fact]
    public void GeneratorRootsAreConsecutiveAlphaPowers()
    {
        var nCheck = 4;
        var generator = ReedSolomon.BuildGenerator(nCheck);

        for (var powerIndex = 1; powerIndex <= nCheck; powerIndex++)
        {
            var root = FieldMath.Power(2, powerIndex);
            byte value = 0;

            for (var index = generator.Length - 1; index >= 0; index--)
            {
                value = FieldMath.Add(FieldMath.Multiply(value, root), generator[index]);
            }

            Assert.Equal(0, value);
        }
    }

    [Fact]
    public void OddNCheckThrows()
    {
        Assert.Throws<InvalidInputError>(() => ReedSolomon.BuildGenerator(3));
    }
}

public sealed class EncodeAndSyndromeTests
{
    [Fact]
    public void EncodeIsSystematic()
    {
        var message = Helpers.Bytes("hello RS");
        var codeword = ReedSolomon.Encode(message, 4);
        Assert.Equal(message, codeword[..message.Length]);
    }

    [Fact]
    public void ValidCodewordHasZeroSyndromes()
    {
        var codeword = ReedSolomon.Encode(Helpers.Bytes("syndromes"), 6);
        Assert.All(ReedSolomon.Syndromes(codeword, 6), value => Assert.Equal(0, value));
    }

    [Fact]
    public void EmptyMessageEncodesToZeroSyndromeCodeword()
    {
        var codeword = ReedSolomon.Encode([], 4);
        Assert.Equal(4, codeword.Length);
        Assert.All(ReedSolomon.Syndromes(codeword, 4), value => Assert.Equal(0, value));
    }

    [Fact]
    public void MaxLengthCodewordIsAccepted()
    {
        var codeword = ReedSolomon.Encode([0x42], 254);
        Assert.Equal(255, codeword.Length);
        Assert.All(ReedSolomon.Syndromes(codeword, 254), value => Assert.Equal(0, value));
    }

    [Fact]
    public void OversizedCodewordThrows()
    {
        var message = new byte[240];
        var error = Assert.Throws<InvalidInputError>(() => ReedSolomon.Encode(message, 20));
        Assert.Contains("exceeds GF(256) block size limit", error.Message);
    }
}

public sealed class DecodeTests
{
    [Fact]
    public void DecodeReturnsOriginalOnCleanCodeword()
    {
        var message = Helpers.Bytes("Reed-Solomon coding is beautiful");
        var recovered = ReedSolomon.Decode(ReedSolomon.Encode(message, 8), 8);
        Assert.Equal(message, recovered);
    }

    [Fact]
    public void T1CorrectsSingleError()
    {
        var message = Helpers.Bytes("abc");
        var codeword = Helpers.Corrupt(ReedSolomon.Encode(message, 2), [1], 0x5A);
        Assert.Equal(message, ReedSolomon.Decode(codeword, 2));
    }

    [Fact]
    public void T2CorrectsTwoErrors()
    {
        var message = Helpers.Bytes("four check bytes");
        var codeword = Helpers.Corrupt(ReedSolomon.Encode(message, 4), [0, 5], 0xAA);
        Assert.Equal(message, ReedSolomon.Decode(codeword, 4));
    }

    [Fact]
    public void T4CorrectsFourErrors()
    {
        var message = Helpers.Bytes("eight check bytes give t=4");
        var codeword = ReedSolomon.Encode(message, 8);
        codeword = Helpers.Corrupt(codeword, [0], 0xFF);
        codeword = Helpers.Corrupt(codeword, [3], 0xAA);
        codeword = Helpers.Corrupt(codeword, [10], 0x55);
        codeword = Helpers.Corrupt(codeword, [14], 0x0F);

        Assert.Equal(message, ReedSolomon.Decode(codeword, 8));
    }

    [Fact]
    public void ErrorsInCheckBytesAreCorrected()
    {
        var message = Helpers.Bytes("check byte error");
        var codeword = ReedSolomon.Encode(message, 4);
        codeword[message.Length] ^= 0x33;
        Assert.Equal(message, ReedSolomon.Decode(codeword, 4));
    }

    [Fact]
    public void TooManyErrorsThrows()
    {
        var message = Helpers.Bytes("too many errors");
        var codeword = Helpers.Corrupt(ReedSolomon.Encode(message, 4), [0, 2, 4], 0x77);
        Assert.Throws<TooManyErrorsError>(() => ReedSolomon.Decode(codeword, 4));
    }

    [Fact]
    public void TooShortReceivedThrows()
    {
        Assert.Throws<InvalidInputError>(() => ReedSolomon.Decode([1, 2, 3], 4));
    }
}

public sealed class ErrorLocatorTests
{
    [Fact]
    public void ErrorLocatorForCleanCodewordIsOne()
    {
        var lambda = ReedSolomon.ErrorLocator(ReedSolomon.Syndromes(ReedSolomon.Encode(Helpers.Bytes("clean"), 4), 4));
        Assert.Equal([1], lambda);
    }

    [Fact]
    public void ErrorLocatorDegreeMatchesCorrectableErrors()
    {
        var message = Helpers.Bytes("locator");
        var codeword = Helpers.Corrupt(ReedSolomon.Encode(message, 4), [0, 5], 0x12);
        var lambda = ReedSolomon.ErrorLocator(ReedSolomon.Syndromes(codeword, 4));
        Assert.Equal(3, lambda.Length);
        Assert.Equal(1, lambda[0]);
    }
}

public sealed class ValidationTests
{
    [Fact]
    public void DecodeRejectsOddNCheck()
    {
        Assert.Throws<InvalidInputError>(() => ReedSolomon.Decode(Helpers.Bytes("abc"), 3));
    }

    [Fact]
    public void EncodeRejectsZeroNCheck()
    {
        Assert.Throws<InvalidInputError>(() => ReedSolomon.Encode(Helpers.Bytes("abc"), 0));
    }

    [Fact]
    public void SingleByteRoundTrips()
    {
        Assert.Equal([0x42], ReedSolomon.Decode(ReedSolomon.Encode([0x42], 2), 2));
    }

    [Fact]
    public void BinaryPayloadRoundTrips()
    {
        var message = new byte[50];
        for (var index = 0; index < message.Length; index++)
        {
            message[index] = (byte)((index * 37 + 13) & 0xFF);
        }

        Assert.Equal(message, ReedSolomon.Decode(ReedSolomon.Encode(message, 10), 10));
    }

}
