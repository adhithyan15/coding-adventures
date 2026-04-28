namespace CodingAdventures.VigenereCipher.Tests;

public sealed class VigenereCipherTests
{
    private const string LongEnglishText =
        "THE ART OF ENCIPHERING AND DECIPHERING MESSAGES HAS A LONG AND STORIED " +
        "HISTORY STRETCHING BACK TO ANCIENT TIMES. THE SPARTANS USED THE SCYTALE, " +
        "JULIUS CAESAR USED A SIMPLE SHIFT CIPHER, AND DURING THE RENAISSANCE THE " +
        "VIGENERE CIPHER WAS CONSIDERED UNBREAKABLE FOR NEARLY THREE HUNDRED YEARS. " +
        "CHARLES BABBAGE BROKE THE CIPHER IN THE EIGHTEEN FIFTIES USING THE INDEX.";

    [Fact]
    public void EncryptAndDecryptKnownVectors()
    {
        Assert.Equal("LXFOPVEFRNHR", VigenereCipher.Encrypt("ATTACKATDAWN", "LEMON"));
        Assert.Equal("ATTACKATDAWN", VigenereCipher.Decrypt("LXFOPVEFRNHR", "LEMON"));
        Assert.Equal("Rijvs, Uyvjn!", VigenereCipher.Encrypt("Hello, World!", "key"));
        Assert.Equal("Hello, World!", VigenereCipher.Decrypt("Rijvs, Uyvjn!", "key"));
    }

    [Fact]
    public void EncryptPreservesCaseAndPunctuation()
    {
        Assert.Equal("A B", VigenereCipher.Encrypt("A A", "AB"));
        Assert.Equal("123 !@#", VigenereCipher.Encrypt("123 !@#", "key"));
        Assert.Equal("Cafe\u0301 costs $3.50", VigenereCipher.Decrypt(VigenereCipher.Encrypt("Cafe\u0301 costs $3.50", "KEY"), "KEY"));
        Assert.Equal("Hj", VigenereCipher.Encrypt("Hi", "ABCDEFGHIJ"));
    }

    [Fact]
    public void RoundTripsAcrossKeys()
    {
        var texts = new[]
        {
            "ATTACKATDAWN",
            "Hello, World!",
            "The quick brown fox jumps over the lazy dog.",
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "",
        };
        var keys = new[] { "KEY", "LEMON", "A", "SECRETKEY", "MiXeD" };

        foreach (var text in texts)
        {
            foreach (var key in keys)
            {
                Assert.Equal(text, VigenereCipher.Decrypt(VigenereCipher.Encrypt(text, key), key));
            }
        }
    }

    [Fact]
    public void InvalidKeysAreRejected()
    {
        Assert.Throws<ArgumentException>(() => VigenereCipher.Encrypt("hello", ""));
        Assert.Throws<ArgumentException>(() => VigenereCipher.Encrypt("hello", "key1"));
        Assert.Throws<ArgumentException>(() => VigenereCipher.Decrypt("hello", "ke y"));
        Assert.Throws<ArgumentNullException>(() => VigenereCipher.Encrypt("hello", null!));
        Assert.Throws<ArgumentOutOfRangeException>(() => VigenereCipher.FindKey("ABC", 0));
    }

    [Fact]
    public void FindsKeyLengths()
    {
        Assert.Equal(1, VigenereCipher.FindKeyLength("A"));
        Assert.Equal(5, VigenereCipher.FindKeyLength(VigenereCipher.Encrypt(LongEnglishText, "LEMON")));
        Assert.Equal(3, VigenereCipher.FindKeyLength(VigenereCipher.Encrypt(LongEnglishText, "KEY")));
    }

    [Fact]
    public void RecoversKeys()
    {
        Assert.Equal("LEMON", VigenereCipher.FindKey(VigenereCipher.Encrypt(LongEnglishText, "LEMON"), 5));
        Assert.Equal("KEY", VigenereCipher.FindKey(VigenereCipher.Encrypt(LongEnglishText, "KEY"), 3));
        var lowSignalKey = VigenereCipher.FindKey("A", 2);
        Assert.Equal(2, lowSignalKey.Length);
        Assert.EndsWith("A", lowSignalKey);
    }

    [Fact]
    public void BreakCipherRecoversKeyAndPlaintext()
    {
        var ciphertext = VigenereCipher.Encrypt(LongEnglishText, "LEMON");
        var result = VigenereCipher.BreakCipher(ciphertext);

        Assert.Equal("LEMON", result.Key);
        Assert.Equal(LongEnglishText, result.Plaintext);
    }

    [Fact]
    public void EnglishFrequenciesMatchExpectations()
    {
        Assert.Equal(26, VigenereCipher.EnglishFrequencies.Count);
        Assert.InRange(VigenereCipher.EnglishFrequencies.Sum(), 0.99, 1.01);
        Assert.True(VigenereCipher.EnglishFrequencies[4] > VigenereCipher.EnglishFrequencies[25]);
    }
}
