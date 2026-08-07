using System.Security.Cryptography;
using System.Text;
using Cipher = CodingAdventures.ChaCha20Poly1305.ChaCha20Poly1305;

namespace CodingAdventures.ChaCha20Poly1305.Tests;

public sealed class ChaCha20Poly1305Tests
{
    private static readonly byte[] ChaChaKey = Hex(
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");
    private static readonly byte[] ChaChaNonce = Hex("000000000000004a00000000");
    private static readonly byte[] ChaChaPlaintext = Encoding.ASCII.GetBytes(
        "Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.");
    private static readonly byte[] ChaChaCiphertext = Hex(
        "6e2e359a2568f98041ba0728dd0d6981e97e7aec1d4360c20a27afccfd9fae0b" +
        "f91b65c5524733ab8f593dabcd62b3571639d624e65152ab8f530c359f0861d8" +
        "07ca0dbf500d6a6156a38e088a22b65e52bc514d16ccf806818ce91ab7793736" +
        "5af90bbf74a35be6b40b8eedf2785e42874d");
    private static readonly byte[] AeadKey = Hex(
        "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f");
    private static readonly byte[] AeadNonce = Hex("070000004041424344454647");
    private static readonly byte[] AeadAad = Hex("50515253c0c1c2c3c4c5c6c7");
    private static readonly byte[] AeadCiphertext = Hex(
        "d31a8d34648e60db7b86afbc53ef7ec2a4aded51296e08fea9e2b5a736ee62d6" +
        "3dbea45e8ca9671282fafb69da92728b1a71de0a9e060b2905d6a5b67ecd3b36" +
        "92ddbd7f2d778b8c9803aee328091b58fab324e4fad675945585808b4831d7bc" +
        "3ff4def08e4b7a9de576d26586cec64b6116");
    private static readonly byte[] AeadTag = Hex("1ae10b594f09e26a7e902ecbd0600691");

    [Fact]
    public void BlockMatchesRfc8439Section232()
    {
        var block = Cipher.ChaCha20Block(
            ChaChaKey,
            1,
            Hex("000000090000004a00000000"));

        Assert.Equal(64, block.Length);
        Assert.Equal(Hex("10f1e7e4d13b5915500fdd1fa32071c4"), block[..16]);
    }

    [Fact]
    public void StreamCipherMatchesRfc8439Section242()
    {
        Assert.Equal(
            ChaChaCiphertext,
            Cipher.ChaCha20Encrypt(ChaChaPlaintext, ChaChaKey, ChaChaNonce, 1));
    }

    [Fact]
    public void StreamCipherIsSymmetricAcrossMultipleBlocks()
    {
        var plaintext = Enumerable.Range(0, 512).Select(index => (byte)index).ToArray();
        var encrypted = Cipher.ChaCha20Encrypt(plaintext, ChaChaKey, ChaChaNonce, 7);

        Assert.Equal(plaintext, Cipher.ChaCha20Encrypt(encrypted, ChaChaKey, ChaChaNonce, 7));
        Assert.Empty(Cipher.ChaCha20Encrypt([], ChaChaKey, ChaChaNonce));
    }

    [Fact]
    public void Poly1305MatchesRfc8439Section252()
    {
        Assert.Equal(
            Hex("a8061dc1305136c6c22b8baf0c0127a9"),
            Cipher.Poly1305Mac(
                Encoding.ASCII.GetBytes("Cryptographic Forum Research Group"),
                Hex("85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b")));
    }

    [Fact]
    public void Poly1305HandlesEmptyAndPartialBlocks()
    {
        var key = Enumerable.Range(0, 32).Select(index => (byte)index).ToArray();
        Assert.Equal(16, Cipher.Poly1305Mac([], key).Length);
        Assert.NotEqual(Cipher.Poly1305Mac([0], key), Cipher.Poly1305Mac([0, 0], key));
    }

    [Fact]
    public void AeadEncryptMatchesRfc8439Section282()
    {
        var result = Cipher.AeadEncrypt(ChaChaPlaintext, AeadKey, AeadNonce, AeadAad);
        Assert.Equal(AeadCiphertext, result.Ciphertext);
        Assert.Equal(AeadTag, result.Tag);
    }

    [Fact]
    public void AeadDecryptMatchesRfc8439Section282()
    {
        Assert.Equal(
            ChaChaPlaintext,
            Cipher.AeadDecrypt(AeadCiphertext, AeadKey, AeadNonce, AeadAad, AeadTag));
    }

    [Fact]
    public void AeadRoundTripsEmptyAndLargePlaintexts()
    {
        var key = Enumerable.Range(0, 32).Select(index => (byte)index).ToArray();
        var nonce = Enumerable.Range(0, 12).Select(index => (byte)index).ToArray();
        foreach (var plaintext in new[] { Array.Empty<byte>(), Enumerable.Repeat((byte)0x41, 1024).ToArray() })
        {
            var result = Cipher.AeadEncrypt(plaintext, key, nonce, null);
            Assert.Equal(plaintext, Cipher.AeadDecrypt(result.Ciphertext, key, nonce, null, result.Tag));
        }
    }

    [Fact]
    public void TamperedCiphertextIsRejected()
    {
        var result = Cipher.AeadEncrypt(Encoding.ASCII.GetBytes("secret"), AeadKey, AeadNonce, AeadAad);
        result.Ciphertext[0] ^= 1;

        Assert.Throws<CryptographicException>(() =>
            Cipher.AeadDecrypt(result.Ciphertext, AeadKey, AeadNonce, AeadAad, result.Tag));
    }

    [Fact]
    public void TamperedTagAndWrongAadAreRejected()
    {
        var result = Cipher.AeadEncrypt(Encoding.ASCII.GetBytes("secret"), AeadKey, AeadNonce, AeadAad);
        var badTag = result.Tag.ToArray();
        badTag[^1] ^= 1;

        Assert.Throws<CryptographicException>(() =>
            Cipher.AeadDecrypt(result.Ciphertext, AeadKey, AeadNonce, AeadAad, badTag));
        Assert.Throws<CryptographicException>(() =>
            Cipher.AeadDecrypt(result.Ciphertext, AeadKey, AeadNonce, [1, 2, 3], result.Tag));
    }

    [Fact]
    public void InvalidKeyLengthsAreRejected()
    {
        Assert.Throws<ArgumentNullException>(() => Cipher.ChaCha20Encrypt([], null!, ChaChaNonce));
        Assert.Throws<ArgumentException>(() => Cipher.ChaCha20Encrypt([], new byte[31], ChaChaNonce));
        Assert.Throws<ArgumentException>(() => Cipher.Poly1305Mac([], new byte[33]));
    }

    [Fact]
    public void InvalidNonceAndTagLengthsAreRejected()
    {
        Assert.Throws<ArgumentException>(() => Cipher.ChaCha20Encrypt([], ChaChaKey, new byte[11]));
        Assert.Throws<ArgumentException>(() =>
            Cipher.AeadDecrypt([], ChaChaKey, ChaChaNonce, [], new byte[15]));
    }

    [Fact]
    public void NullDataInputsAreRejected()
    {
        Assert.Throws<ArgumentNullException>(() => Cipher.ChaCha20Encrypt(null!, ChaChaKey, ChaChaNonce));
        Assert.Throws<ArgumentNullException>(() => Cipher.Poly1305Mac(null!, ChaChaKey));
        Assert.Throws<ArgumentNullException>(() => Cipher.AeadEncrypt(null!, ChaChaKey, ChaChaNonce));
        Assert.Throws<ArgumentNullException>(() => Cipher.AeadDecrypt(null!, ChaChaKey, ChaChaNonce, [], new byte[16]));
    }

    private static byte[] Hex(string value) => Convert.FromHexString(value);
}
