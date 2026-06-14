namespace CodingAdventures.AesModes.Tests;

public sealed class AesModesTests
{
    private static readonly byte[] NistKey = Convert.FromHexString("2b7e151628aed2a6abf7158809cf4f3c");
    private static readonly byte[] NistPlaintext = Convert.FromHexString(
        "6bc1bee22e409f96e93d7e117393172a" +
        "ae2d8a571e03ac9c9eb76fac45af8e51" +
        "30c81c46a35ce411e5fbc1191a0a52ef" +
        "f69f2445df4f9b17ad2b417be66c3710");

    [Fact]
    public void Pkcs7PadAndUnpadRoundTrips()
    {
        var padded = AesModes.Pkcs7Pad("hello"u8.ToArray());

        Assert.Equal(16, padded.Length);
        Assert.Equal(11, padded[^1]);
        Assert.Equal("hello"u8.ToArray(), AesModes.Pkcs7Unpad(padded));
        Assert.Equal(32, AesModes.Pkcs7Pad(new byte[16]).Length);
    }

    [Fact]
    public void Pkcs7UnpadRejectsInvalidPadding()
    {
        Assert.Throws<ArgumentException>(() => AesModes.Pkcs7Unpad(Array.Empty<byte>()));
        Assert.Throws<ArgumentException>(() => AesModes.Pkcs7Unpad("short"u8.ToArray()));
        Assert.Throws<ArgumentException>(() => AesModes.Pkcs7Unpad(Convert.FromHexString("30313233343536373839616263646500")));
        Assert.Throws<ArgumentException>(() => AesModes.Pkcs7Unpad(Convert.FromHexString("30313233343536373839616263030203")));
    }

    [Fact]
    public void XorBytesRequiresEqualLengths()
    {
        Assert.Equal(new byte[] { 0x0f, 0xf0 }, AesModes.XorBytes(new byte[] { 0xff, 0x00 }, new byte[] { 0xf0, 0xf0 }));
        Assert.Throws<ArgumentException>(() => AesModes.XorBytes(new byte[] { 1 }, Array.Empty<byte>()));
    }

    [Fact]
    public void EcbMatchesNistBlocksAndRoundTrips()
    {
        var ciphertext = AesModes.EcbEncrypt(NistPlaintext, NistKey);

        Assert.Equal("3ad77bb40d7a3660a89ecaf32466ef97", Convert.ToHexString(ciphertext[..16]).ToLowerInvariant());
        Assert.Equal("f5d3d58503b9699de785895a96fdbaaf", Convert.ToHexString(ciphertext[16..32]).ToLowerInvariant());
        Assert.Equal(NistPlaintext, AesModes.EcbDecrypt(ciphertext, NistKey));
    }

    [Fact]
    public void EcbShowsIdenticalBlockLeak()
    {
        var plaintext = Enumerable.Repeat((byte)'A', 48).ToArray();
        var ciphertext = AesModes.EcbEncrypt(plaintext, NistKey);

        Assert.Equal(ciphertext[..16], ciphertext[16..32]);
        Assert.Equal(ciphertext[16..32], ciphertext[32..48]);
        Assert.Throws<ArgumentException>(() => AesModes.EcbDecrypt("short"u8.ToArray(), NistKey));
    }

    [Fact]
    public void CbcMatchesNistBlocksAndRoundTrips()
    {
        var iv = Convert.FromHexString("000102030405060708090a0b0c0d0e0f");
        var ciphertext = AesModes.CbcEncrypt(NistPlaintext, NistKey, iv);

        Assert.Equal("7649abac8119b246cee98e9b12e9197d", Convert.ToHexString(ciphertext[..16]).ToLowerInvariant());
        Assert.Equal("5086cb9b507219ee95db113a917678b2", Convert.ToHexString(ciphertext[16..32]).ToLowerInvariant());
        Assert.Equal(NistPlaintext, AesModes.CbcDecrypt(ciphertext, NistKey, iv));
    }

    [Fact]
    public void CbcRejectsInvalidInputs()
    {
        Assert.Throws<ArgumentException>(() => AesModes.CbcEncrypt("test"u8.ToArray(), NistKey, "short"u8.ToArray()));
        Assert.Throws<ArgumentException>(() => AesModes.CbcDecrypt("short"u8.ToArray(), NistKey, new byte[16]));
    }

    [Fact]
    public void CtrRoundTripsWithoutPadding()
    {
        var nonce = new byte[12];
        var plaintext = "CTR keeps the same length."u8.ToArray();
        var ciphertext = AesModes.CtrEncrypt(plaintext, NistKey, nonce);

        Assert.Equal(plaintext.Length, ciphertext.Length);
        Assert.Equal(plaintext, AesModes.CtrDecrypt(ciphertext, NistKey, nonce));
        Assert.Empty(AesModes.CtrEncrypt(Array.Empty<byte>(), NistKey, nonce));
        Assert.Throws<ArgumentException>(() => AesModes.CtrEncrypt(plaintext, NistKey, new byte[16]));
    }

    [Fact]
    public void GcmMatchesNistTestCaseAndRoundTripsWithAad()
    {
        var key = Convert.FromHexString("feffe9928665731c6d6a8f9467308308");
        var iv = Convert.FromHexString("cafebabefacedbaddecaf888");
        var plaintext = Convert.FromHexString(
            "d9313225f88406e5a55909c5aff5269a" +
            "86a7a9531534f7da2e4c303d8a318a72" +
            "1c3c0c95956809532fcf0e2449a6b525" +
            "b16aedf5aa0de657ba637b391aafd255");
        var expectedCiphertext = Convert.FromHexString(
            "42831ec2217774244b7221b784d0d49c" +
            "e3aa212f2c02a4e035c17e2329aca12e" +
            "21d514b25466931c7d8f6a5aac84aa05" +
            "1ba30b396a0aac973d58e091473f5985");
        var expectedTag = Convert.FromHexString("4d5c2af327cd64a62cf35abd2ba6fab4");

        var (ciphertext, tag) = AesModes.GcmEncrypt(plaintext, key, iv);

        Assert.Equal(expectedCiphertext, ciphertext);
        Assert.Equal(expectedTag, tag);
        Assert.Equal(plaintext, AesModes.GcmDecrypt(ciphertext, key, iv, null, tag));

        var aad = "metadata"u8.ToArray();
        var roundTrip = AesModes.GcmEncrypt("secret"u8.ToArray(), key, iv, aad);
        Assert.Equal("secret"u8.ToArray(), AesModes.GcmDecrypt(roundTrip.Ciphertext, key, iv, aad, roundTrip.Tag));
    }

    [Fact]
    public void GcmRejectsTamperingAndInvalidLengths()
    {
        var key = Convert.FromHexString("feffe9928665731c6d6a8f9467308308");
        var iv = Convert.FromHexString("cafebabefacedbaddecaf888");
        var (ciphertext, tag) = AesModes.GcmEncrypt("secret"u8.ToArray(), key, iv);

        ciphertext[0] ^= 1;
        Assert.Throws<InvalidOperationException>(() => AesModes.GcmDecrypt(ciphertext, key, iv, null, tag));
        Assert.Throws<ArgumentException>(() => AesModes.GcmEncrypt("test"u8.ToArray(), key, new byte[16]));
        Assert.Throws<ArgumentException>(() => AesModes.GcmDecrypt(Array.Empty<byte>(), key, iv, null, new byte[8]));
    }
}
