namespace CodingAdventures.ScytaleCipher.Tests;

public sealed class ScytaleCipherTests
{
    [Fact]
    public void EncryptMatchesReferenceExamples()
    {
        Assert.Equal("HLWLEOODL R ", ScytaleCipher.Encrypt("HELLO WORLD", 3));
        Assert.Equal("ACEBDF", ScytaleCipher.Encrypt("ABCDEF", 2));
        Assert.Equal("ADBECF", ScytaleCipher.Encrypt("ABCDEF", 3));
        Assert.Equal("ABCD", ScytaleCipher.Encrypt("ABCD", 4));
        Assert.Equal(string.Empty, ScytaleCipher.Encrypt(string.Empty, 2));
    }

    [Fact]
    public void DecryptStripsPaddingAndMatchesReferenceExamples()
    {
        Assert.Equal("HELLO WORLD", ScytaleCipher.Decrypt("HLWLEOODL R ", 3));
        Assert.Equal("ABCDEF", ScytaleCipher.Decrypt("ACEBDF", 2));
        Assert.Equal("HELLO", ScytaleCipher.Decrypt(ScytaleCipher.Encrypt("HELLO", 3), 3));
        Assert.Equal(string.Empty, ScytaleCipher.Decrypt(string.Empty, 2));
    }

    [Fact]
    public void RoundTripsAcrossValidKeys()
    {
        var text = "The quick brown fox jumps over the lazy dog!";
        for (var key = 2; key <= text.Length / 2; key++)
        {
            Assert.Equal(text, ScytaleCipher.Decrypt(ScytaleCipher.Encrypt(text, key), key));
        }
    }

    [Fact]
    public void BruteForceReturnsCandidatePlaintexts()
    {
        var ciphertext = ScytaleCipher.Encrypt("HELLO WORLD", 3);
        var results = ScytaleCipher.BruteForce(ciphertext);

        Assert.Contains(results, result => result.Key == 3 && result.Text == "HELLO WORLD");
        Assert.Equal([2, 3, 4, 5], ScytaleCipher.BruteForce("ABCDEFGHIJ").Select(result => result.Key));
        Assert.Empty(ScytaleCipher.BruteForce("ABC"));
    }

    [Fact]
    public void InvalidInputsThrow()
    {
        Assert.Throws<ArgumentNullException>(() => ScytaleCipher.Encrypt(null!, 2));
        Assert.Throws<ArgumentNullException>(() => ScytaleCipher.Decrypt(null!, 2));
        Assert.Throws<ArgumentNullException>(() => ScytaleCipher.BruteForce(null!));
        Assert.Throws<ArgumentOutOfRangeException>(() => ScytaleCipher.Encrypt("HELLO", 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => ScytaleCipher.Decrypt("HI", 3));
    }
}
