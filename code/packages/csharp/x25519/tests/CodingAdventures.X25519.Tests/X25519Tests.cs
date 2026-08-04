using X25519Algorithm = CodingAdventures.X25519.Curve25519;

namespace CodingAdventures.X25519.Tests;

public sealed class X25519Tests
{
    [Theory]
    [InlineData(
        "a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4",
        "e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c",
        "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552")]
    [InlineData(
        "4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d",
        "e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493",
        "95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957")]
    public void ComputeMatchesRfc7748Vectors(string scalar, string u, string expected)
    {
        Assert.Equal(Hex(expected), X25519Algorithm.Compute(Hex(scalar), Hex(u)));
    }

    [Fact]
    public void ComputeBaseMatchesAliceAndBobPublicKeys()
    {
        Assert.Equal(
            Hex("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"),
            X25519Algorithm.ComputeBase(Hex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")));
        Assert.Equal(
            Hex("de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f"),
            X25519Algorithm.ComputeBase(Hex("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")));
    }

    [Fact]
    public void BothPartiesDeriveTheRfcSharedSecret()
    {
        var alicePrivate = Hex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
        var bobPrivate = Hex("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
        var alicePublic = X25519Algorithm.ComputeBase(alicePrivate);
        var bobPublic = X25519Algorithm.ComputeBase(bobPrivate);
        var expected = Hex("4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742");

        Assert.Equal(expected, X25519Algorithm.Compute(alicePrivate, bobPublic));
        Assert.Equal(expected, X25519Algorithm.Compute(bobPrivate, alicePublic));
    }

    [Fact]
    public void GenerateKeypairAliasesBaseMultiplication()
    {
        var privateKey = Enumerable.Range(0, 32).Select(value => (byte)value).ToArray();
        Assert.Equal(X25519Algorithm.ComputeBase(privateKey), X25519Algorithm.GenerateKeypair(privateKey));
    }

    [Fact]
    public void IteratedRfcVectorMatchesAfterOneThousandRounds()
    {
        var scalar = new byte[32];
        var u = new byte[32];
        scalar[0] = 9;
        u[0] = 9;

        for (var iteration = 0; iteration < 1_000; iteration++)
        {
            var next = X25519Algorithm.Compute(scalar, u);
            u = scalar;
            scalar = next;
        }

        Assert.Equal(
            Hex("684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51"),
            scalar);
    }

    [Fact]
    public void DecodeMasksTheHighBitOfTheUCoordinate()
    {
        var scalar = Enumerable.Repeat((byte)0x42, 32).ToArray();
        var canonical = X25519Algorithm.BasePoint;
        var highBitSet = canonical.ToArray();
        highBitSet[31] = 0x80;

        Assert.Equal(
            X25519Algorithm.Compute(scalar, canonical),
            X25519Algorithm.Compute(scalar, highBitSet));
    }

    [Fact]
    public void RejectsNullAndWrongLengthInputs()
    {
        Assert.Throws<ArgumentNullException>(() => X25519Algorithm.Compute(null!, new byte[32]));
        Assert.Throws<ArgumentNullException>(() => X25519Algorithm.Compute(new byte[32], null!));
        Assert.Throws<ArgumentException>(() => X25519Algorithm.Compute(new byte[31], new byte[32]));
        Assert.Throws<ArgumentException>(() => X25519Algorithm.Compute(new byte[32], new byte[33]));
    }

    [Fact]
    public void RejectsLowOrderAllZeroOutput()
    {
        Assert.Throws<InvalidOperationException>(() =>
            X25519Algorithm.Compute(Enumerable.Repeat((byte)0x11, 32).ToArray(), new byte[32]));
    }

    [Fact]
    public void BasePointReturnsAnIndependentCopy()
    {
        var first = X25519Algorithm.BasePoint;
        first[0] = 0;
        var second = X25519Algorithm.BasePoint;

        Assert.Equal(32, second.Length);
        Assert.Equal(9, second[0]);
        Assert.All(second.Skip(1), value => Assert.Equal(0, value));
    }

    private static byte[] Hex(string value) => Convert.FromHexString(value);
}
