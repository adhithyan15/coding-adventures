namespace CodingAdventures.Gf256.Tests;

public sealed class Gf256Tests
{
    [Fact]
    public void ExposesExpectedConstants()
    {
        Assert.Equal("0.1.0", Gf256.VERSION);
        Assert.Equal((byte)0, Gf256.ZERO);
        Assert.Equal((byte)1, Gf256.ONE);
        Assert.Equal(0x11d, Gf256.PRIMITIVE_POLYNOMIAL);
    }

    [Fact]
    public void LogAndAlogTablesMatchKnownValues()
    {
        Assert.Equal(256, Gf256.ALOG.Count);
        Assert.Equal(256, Gf256.LOG.Count);
        Assert.Equal(1, Gf256.ALOG[0]);
        Assert.Equal(2, Gf256.ALOG[1]);
        Assert.Equal(29, Gf256.ALOG[8]);
        Assert.Equal(0, Gf256.LOG[1]);
        Assert.Equal(1, Gf256.LOG[2]);

        for (var value = 1; value <= 255; value++)
        {
            Assert.Equal(value, Gf256.ALOG[Gf256.LOG[value]]);
        }
    }

    [Fact]
    public void AddAndSubtractAreXor()
    {
        for (var value = 0; value <= 255; value++)
        {
            Assert.Equal((byte)value, Gf256.Add(0, (byte)value));
            Assert.Equal((byte)0, Gf256.Add((byte)value, (byte)value));
            Assert.Equal(Gf256.Add((byte)value, 0x42), Gf256.Subtract((byte)value, 0x42));
        }

        Assert.Equal((byte)0x99, Gf256.Add(0x53, 0xca));
    }

    [Fact]
    public void MultiplyObeysIdentityZeroAndKnownSpotChecks()
    {
        for (var value = 0; value <= 255; value++)
        {
            Assert.Equal((byte)0, Gf256.Multiply((byte)value, 0));
            Assert.Equal((byte)value, Gf256.Multiply((byte)value, 1));
        }

        Assert.Equal((byte)1, Gf256.Multiply(0x53, 0x8c));
        Assert.Equal(Gf256.Multiply(0x34, 0x56), Gf256.Multiply(0x56, 0x34));
    }

    [Fact]
    public void DivideAndInverseWorkForNonZeroInputs()
    {
        for (var value = 1; value <= 255; value++)
        {
            Assert.Equal((byte)1, Gf256.Divide((byte)value, (byte)value));
            Assert.Equal((byte)value, Gf256.Divide((byte)value, 1));
            Assert.Equal((byte)1, Gf256.Multiply((byte)value, Gf256.Inverse((byte)value)));
        }

        Assert.Equal((byte)0, Gf256.Divide(0, 1));
        Assert.Throws<InvalidOperationException>(() => Gf256.Divide(1, 0));
        Assert.Throws<InvalidOperationException>(() => Gf256.Inverse(0));
    }

    [Fact]
    public void PowerHandlesZeroAndPositiveExponents()
    {
        Assert.Equal((byte)1, Gf256.Power(0, 0));
        Assert.Equal((byte)0, Gf256.Power(0, 5));
        Assert.Equal((byte)1, Gf256.Power(0x53, 0));
        Assert.Equal((byte)0x53, Gf256.Power(0x53, 1));
        Assert.Equal(Gf256.Multiply(0x53, 0x53), Gf256.Power(0x53, 2));
        Assert.Throws<ArgumentOutOfRangeException>(() => Gf256.Power(1, -1));
    }

    [Fact]
    public void ZeroAndOneHelpersReturnFieldIdentities()
    {
        Assert.Equal(Gf256.ZERO, Gf256.Zero());
        Assert.Equal(Gf256.ONE, Gf256.One());
    }

    [Fact]
    public void AlternateFieldSupportsAesPolynomial()
    {
        var aes = Gf256.CreateField(0x11b);

        Assert.Equal(0x11b, aes.Polynomial);
        Assert.Equal((byte)0x01, aes.Multiply(0x53, 0xca));
        Assert.Equal((byte)0xc1, aes.Multiply(0x57, 0x83));
        Assert.Equal((byte)0x53, aes.Divide(0x53, 1));
        Assert.Equal((byte)1, aes.Multiply(0x57, aes.Inverse(0x57)));
        Assert.Throws<InvalidOperationException>(() => aes.Divide(1, 0));
        Assert.Throws<InvalidOperationException>(() => aes.Inverse(0));
    }
}
