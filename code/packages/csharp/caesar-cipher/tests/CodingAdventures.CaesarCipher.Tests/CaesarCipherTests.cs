namespace CodingAdventures.CaesarCipher.Tests;

public sealed class CaesarCipherTests
{
    [Fact]
    public void EncryptAndDecryptHandleKnownExamples()
    {
        Assert.Equal("KHOOR", CaesarCipher.Encrypt("HELLO", 3));
        Assert.Equal("khoor", CaesarCipher.Encrypt("hello", 3));
        Assert.Equal("Khoor, Zruog!", CaesarCipher.Encrypt("Hello, World!", 3));
        Assert.Equal("HELLO", CaesarCipher.Decrypt("KHOOR", 3));
        Assert.Equal("Hello, World!", CaesarCipher.Decrypt("Khoor, Zruog!", 3));
    }

    [Fact]
    public void EncryptRespectsWrappingAndNegativeShifts()
    {
        Assert.Equal("abc XYZ 123!", CaesarCipher.Encrypt("abc XYZ 123!", 0));
        Assert.Equal("Wrap around test", CaesarCipher.Encrypt("Wrap around test", 26));
        Assert.Equal("Double wrap", CaesarCipher.Encrypt("Double wrap", 52));
        Assert.Equal("ZAB", CaesarCipher.Encrypt("ABC", -1));
        Assert.Equal("ZAB", CaesarCipher.Encrypt("ABC", -27));
        Assert.Equal("ABC", CaesarCipher.Encrypt("XYZ", 3));
    }

    [Fact]
    public void RoundTripsAcrossManyShifts()
    {
        var original = "The Quick Brown Fox Jumps Over The Lazy Dog! 123";
        for (var shift = -30; shift <= 30; shift++)
        {
            var encrypted = CaesarCipher.Encrypt(original, shift);
            var decrypted = CaesarCipher.Decrypt(encrypted, shift);
            Assert.Equal(original, decrypted);
        }
    }

    [Fact]
    public void Rot13IsSelfInverse()
    {
        var text = "The Quick Brown Fox! 123";
        Assert.Equal(text, CaesarCipher.Rot13(CaesarCipher.Rot13(text)));
        Assert.Equal(CaesarCipher.Encrypt(text, 13), CaesarCipher.Rot13(text));
    }

    [Fact]
    public void NonAsciiCharactersPassThrough()
    {
        const string original = "Cafe\u0301 costs $3.50 \u2764";
        var encrypted = CaesarCipher.Encrypt(original, 7);
        var decrypted = CaesarCipher.Decrypt(encrypted, 7);
        Assert.Equal(original, decrypted);
    }

    [Fact]
    public void BruteForceReturnsAllCandidates()
    {
        var results = CaesarCipher.BruteForce("KHOOR");
        Assert.Equal(25, results.Count);
        Assert.Equal(3, results[2].Shift);
        Assert.Equal("HELLO", results[2].Plaintext);
        Assert.All(results, result => Assert.InRange(result.Shift, 1, 25));
    }

    [Fact]
    public void BruteForceHandlesEmptyAndNonAlphabeticInput()
    {
        var emptyResults = CaesarCipher.BruteForce("");
        Assert.Equal(25, emptyResults.Count);
        Assert.All(emptyResults, result => Assert.Equal("", result.Plaintext));

        var punctuationResults = CaesarCipher.BruteForce("123!!!");
        Assert.All(punctuationResults, result => Assert.Equal("123!!!", result.Plaintext));
    }

    [Fact]
    public void FrequencyAnalysisFindsKnownShifts()
    {
        var plaintext = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG";
        var ciphertext = CaesarCipher.Encrypt(plaintext, 3);
        var (shift, decoded) = CaesarCipher.FrequencyAnalysis(ciphertext);
        Assert.Equal(3, shift);
        Assert.Equal(plaintext, decoded);

        plaintext = "IN CRYPTOGRAPHY A CAESAR CIPHER ALSO KNOWN AS SHIFT CIPHER IS ONE OF THE SIMPLEST AND MOST WIDELY KNOWN ENCRYPTION TECHNIQUES";
        ciphertext = CaesarCipher.Encrypt(plaintext, 17);
        (shift, decoded) = CaesarCipher.FrequencyAnalysis(ciphertext);
        Assert.Equal(17, shift);
        Assert.Equal(plaintext, decoded);
    }

    [Fact]
    public void FrequencyAnalysisHandlesLowSignalInputs()
    {
        var (shift, decoded) = CaesarCipher.FrequencyAnalysis("");
        Assert.InRange(shift, 1, 25);
        Assert.Equal("", decoded);

        (shift, decoded) = CaesarCipher.FrequencyAnalysis("12345!@#$%");
        Assert.Equal(1, shift);
        Assert.Equal("12345!@#$%", decoded);

        (shift, _) = CaesarCipher.FrequencyAnalysis("EEEEEEEEEEE");
        Assert.InRange(shift, 1, 25);
    }

    [Fact]
    public void EnglishFrequencyTableMatchesExpectations()
    {
        Assert.Equal(26, CaesarCipher.EnglishFrequencies.Count);
        Assert.InRange(CaesarCipher.EnglishFrequencies.Sum(), 0.99, 1.01);

        var eFrequency = CaesarCipher.EnglishFrequencies[4];
        for (var i = 0; i < CaesarCipher.EnglishFrequencies.Count; i++)
        {
            Assert.True(CaesarCipher.EnglishFrequencies[i] > 0.0);
            if (i != 4)
            {
                Assert.True(eFrequency > CaesarCipher.EnglishFrequencies[i]);
            }
        }
    }
}
