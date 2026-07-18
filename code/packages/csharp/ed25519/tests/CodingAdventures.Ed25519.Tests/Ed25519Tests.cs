using Ed25519Algorithm = CodingAdventures.Ed25519.Ed25519;

namespace CodingAdventures.Ed25519.Tests;

public sealed class Ed25519Tests
{
    [Theory]
    [InlineData(
        "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        "",
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b")]
    [InlineData(
        "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
        "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
        "72",
        "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00")]
    [InlineData(
        "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7",
        "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
        "af82",
        "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a")]
    public void MatchesRfc8032Vectors(
        string seedHex,
        string publicKeyHex,
        string messageHex,
        string signatureHex)
    {
        var seed = Hex(seedHex);
        var message = Hex(messageHex);
        var expectedPublicKey = Hex(publicKeyHex);
        var expectedSignature = Hex(signatureHex);

        var (publicKey, secretKey) = Ed25519Algorithm.GenerateKeypair(seed);

        Assert.Equal(expectedPublicKey, publicKey);
        Assert.Equal(seed.Concat(publicKey), secretKey);
        Assert.Equal(expectedSignature, Ed25519Algorithm.Sign(message, secretKey));
        Assert.True(Ed25519Algorithm.Verify(message, expectedSignature, publicKey));
    }

    [Fact]
    public void KeyGenerationAndSigningAreDeterministic()
    {
        var seed = Enumerable.Range(0, 32).Select(value => (byte)value).ToArray();
        var message = "deterministic"u8.ToArray();
        var first = Ed25519Algorithm.GenerateKeypair(seed);
        var second = Ed25519Algorithm.GenerateKeypair(seed);

        Assert.Equal(first.PublicKey, second.PublicKey);
        Assert.Equal(first.SecretKey, second.SecretKey);
        Assert.Equal(
            Ed25519Algorithm.Sign(message, first.SecretKey),
            Ed25519Algorithm.Sign(message, first.SecretKey));
    }

    [Fact]
    public void VerificationRejectsTamperingWrongMessagesAndWrongKeys()
    {
        var seed = Enumerable.Range(0, 32).Select(value => (byte)value).ToArray();
        var otherSeed = Enumerable.Range(32, 32).Select(value => (byte)value).ToArray();
        var (publicKey, secretKey) = Ed25519Algorithm.GenerateKeypair(seed);
        var (otherPublicKey, _) = Ed25519Algorithm.GenerateKeypair(otherSeed);
        var message = "hello"u8.ToArray();
        var signature = Ed25519Algorithm.Sign(message, secretKey);

        var tamperedR = signature.ToArray();
        tamperedR[0] ^= 1;
        var tamperedS = signature.ToArray();
        tamperedS[32] ^= 1;

        Assert.False(Ed25519Algorithm.Verify("world"u8.ToArray(), signature, publicKey));
        Assert.False(Ed25519Algorithm.Verify(message, signature, otherPublicKey));
        Assert.False(Ed25519Algorithm.Verify(message, tamperedR, publicKey));
        Assert.False(Ed25519Algorithm.Verify(message, tamperedS, publicKey));
    }

    [Fact]
    public void VerificationRejectsMalformedScalarsPointsAndLengths()
    {
        var seed = new byte[32];
        var (publicKey, secretKey) = Ed25519Algorithm.GenerateKeypair(seed);
        var message = "hello"u8.ToArray();
        var signature = Ed25519Algorithm.Sign(message, secretKey);

        var outOfRangeS = signature.ToArray();
        Hex("edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010")
            .CopyTo(outOfRangeS, 32);
        var invalidR = signature.ToArray();
        Enumerable.Repeat((byte)0xff, 32).ToArray().CopyTo(invalidR, 0);
        var negativeZeroR = signature.ToArray();
        Array.Clear(negativeZeroR, 0, 32);
        negativeZeroR[0] = 1;
        negativeZeroR[31] = 0x80;

        Assert.False(Ed25519Algorithm.Verify(message, outOfRangeS, publicKey));
        Assert.False(Ed25519Algorithm.Verify(message, invalidR, publicKey));
        Assert.False(Ed25519Algorithm.Verify(message, negativeZeroR, publicKey));
        Assert.False(Ed25519Algorithm.Verify(message, signature, Enumerable.Repeat((byte)0xff, 32).ToArray()));
        Assert.False(Ed25519Algorithm.Verify(message, null, publicKey));
        Assert.False(Ed25519Algorithm.Verify(message, new byte[63], publicKey));
        Assert.False(Ed25519Algorithm.Verify(message, signature, null));
        Assert.False(Ed25519Algorithm.Verify(message, signature, new byte[31]));
    }

    [Fact]
    public void PublicInputsAreValidatedAndSecretKeyMustMatchItsSeed()
    {
        Assert.Throws<ArgumentNullException>(() => Ed25519Algorithm.GenerateKeypair(null!));
        Assert.Throws<ArgumentException>(() => Ed25519Algorithm.GenerateKeypair(new byte[31]));
        Assert.Throws<ArgumentNullException>(() => Ed25519Algorithm.Sign(null!, new byte[64]));
        Assert.Throws<ArgumentNullException>(() => Ed25519Algorithm.Sign([], null!));
        Assert.Throws<ArgumentException>(() => Ed25519Algorithm.Sign([], new byte[63]));
        Assert.Throws<ArgumentNullException>(() => Ed25519Algorithm.Verify(null!, new byte[64], new byte[32]));

        var (_, secretKey) = Ed25519Algorithm.GenerateKeypair(new byte[32]);
        secretKey[63] ^= 1;
        Assert.Throws<ArgumentException>(() => Ed25519Algorithm.Sign([], secretKey));
    }

    private static byte[] Hex(string value) => Convert.FromHexString(value);
}
