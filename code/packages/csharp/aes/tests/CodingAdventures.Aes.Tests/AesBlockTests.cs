namespace CodingAdventures.Aes.Tests;

public sealed class AesBlockTests
{
    [Theory]
    [InlineData(
        "2b7e151628aed2a6abf7158809cf4f3c",
        "3243f6a8885a308d313198a2e0370734",
        "3925841d02dc09fbdc118597196a0b32")]
    [InlineData(
        "000102030405060708090a0b0c0d0e0f",
        "00112233445566778899aabbccddeeff",
        "69c4e0d86a7b0430d8cdb78070b4c55a")]
    [InlineData(
        "000102030405060708090a0b0c0d0e0f1011121314151617",
        "00112233445566778899aabbccddeeff",
        "dda97ca4864cdfe06eaf70a0ec0d7191")]
    [InlineData(
        "603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4",
        "6bc1bee22e409f96e93d7e117393172a",
        "f3eed1bdb5d2a03c064b5a7e3db181f8")]
    [InlineData(
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        "00112233445566778899aabbccddeeff",
        "8ea2b7ca516745bfeafc49904b496089")]
    public void EncryptAndDecryptMatchKnownVectors(string keyHex, string plainHex, string cipherHex)
    {
        var key = Convert.FromHexString(keyHex);
        var plaintext = Convert.FromHexString(plainHex);
        var ciphertext = Convert.FromHexString(cipherHex);

        Assert.Equal(ciphertext, AesBlock.EncryptBlock(plaintext, key));
        Assert.Equal(plaintext, AesBlock.DecryptBlock(ciphertext, key));
    }

    [Fact]
    public void RoundTripsAcrossKeySizesAndInputs()
    {
        foreach (var keyLength in new[] { 16, 24, 32 })
        {
            var key = Enumerable.Range(0, keyLength).Select(value => (byte)value).ToArray();
            var plaintext = Enumerable.Range(0, 16).Select(value => (byte)(255 - value)).ToArray();

            Assert.Equal(plaintext, AesBlock.DecryptBlock(AesBlock.EncryptBlock(plaintext, key), key));
        }
    }

    [Fact]
    public void ValidationRejectsInvalidInputs()
    {
        var key = new byte[16];
        var block = new byte[16];

        Assert.Throws<ArgumentNullException>(() => AesBlock.EncryptBlock(null!, key));
        Assert.Throws<ArgumentNullException>(() => AesBlock.DecryptBlock(block, null!));
        Assert.Throws<ArgumentException>(() => AesBlock.EncryptBlock(new byte[15], key));
        Assert.Throws<ArgumentException>(() => AesBlock.DecryptBlock(new byte[17], key));
        Assert.Throws<ArgumentException>(() => AesBlock.EncryptBlock(block, new byte[10]));
        Assert.Throws<ArgumentException>(() => AesBlock.DecryptBlock(block, new byte[20]));
    }
}
