namespace CodingAdventures.AtbashCipher.Tests;

public sealed class AtbashCipherTests
{
    [Fact]
    public void EncryptMatchesClassicExamples()
    {
        Assert.Equal("SVOOL", AtbashCipher.Encrypt("HELLO"));
        Assert.Equal("svool", AtbashCipher.Encrypt("hello"));
        Assert.Equal("Svool, Dliow! 123", AtbashCipher.Encrypt("Hello, World! 123"));
    }

    [Fact]
    public void EncryptPreservesCaseAndPassthroughCharacters()
    {
        Assert.Equal("ZYX", AtbashCipher.Encrypt("ABC"));
        Assert.Equal("zyx", AtbashCipher.Encrypt("abc"));
        Assert.Equal("ZyXwVu", AtbashCipher.Encrypt("AbCdEf"));
        Assert.Equal("12345", AtbashCipher.Encrypt("12345"));
        Assert.Equal("!@#$%", AtbashCipher.Encrypt("!@#$%"));
        Assert.Equal("Z\nY\tX", AtbashCipher.Encrypt("A\nB\tC"));
    }

    [Fact]
    public void EncryptTransformsEntireAlphabet()
    {
        Assert.Equal("ZYXWVUTSRQPONMLKJIHGFEDCBA", AtbashCipher.Encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ"));
        Assert.Equal("zyxwvutsrqponmlkjihgfedcba", AtbashCipher.Encrypt("abcdefghijklmnopqrstuvwxyz"));
    }

    [Fact]
    public void CipherIsSelfInverse()
    {
        var text = "The quick brown fox jumps over the lazy dog! 42";
        Assert.Equal(text, AtbashCipher.Encrypt(AtbashCipher.Encrypt(text)));
        Assert.Equal(text, AtbashCipher.Decrypt(AtbashCipher.Encrypt(text)));
        Assert.Equal(AtbashCipher.Encrypt(text), AtbashCipher.Decrypt(text));
    }

    [Fact]
    public void HandlesEdgeCases()
    {
        Assert.Equal("", AtbashCipher.Encrypt(""));
        Assert.Equal("5", AtbashCipher.Encrypt("5"));
        Assert.Equal("Z", AtbashCipher.Encrypt("A"));
        Assert.Equal("A", AtbashCipher.Encrypt("Z"));
        Assert.Equal("N", AtbashCipher.Encrypt("M"));
        Assert.Equal("M", AtbashCipher.Encrypt("N"));
    }

    [Fact]
    public void NoLetterMapsToItself()
    {
        for (var offset = 0; offset < 26; offset++)
        {
            var upper = ((char)('A' + offset)).ToString();
            var lower = ((char)('a' + offset)).ToString();
            Assert.NotEqual(upper, AtbashCipher.Encrypt(upper));
            Assert.NotEqual(lower, AtbashCipher.Encrypt(lower));
        }
    }
}
