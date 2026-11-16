// The bits of every embedded constant are asserted against the fixture
// manifest (PHY00a 6.5 e): the constants are built from bit patterns, and
// the decimal literals the source still writes (PI, the tan and atan
// thresholds) are checked to parse to the pinned bits.

namespace CodingAdventures.Trig.Tests;

public sealed class ConstantsTests
{
    private static ulong Bits(double v) => BitConverter.DoubleToUInt64Bits(v);

    private static ulong FromManifest(string name)
    {
        using var m = ConformanceTests.Manifest();
        return Convert.ToUInt64(m.RootElement.GetProperty("constants").GetProperty(name).GetString(), 16);
    }

    [Fact]
    public void ConstantsMatchTheManifest()
    {
        Assert.Equal(FromManifest("PI"), Bits(Trig.PI));
        Assert.Equal(FromManifest("TWO_PI"), Bits(Trig.TwoPi));
        Assert.Equal(FromManifest("HALF_PI"), Bits(Trig.HalfPi));
        Assert.Equal(FromManifest("INV_PIO2"), Bits(Trig.InvPio2));
        for (var i = 0; i < 5; i++)
        {
            Assert.Equal(FromManifest("PIO2_" + (i + 1)), Bits(Trig.Pio2[i]));
        }

        Assert.Equal(FromManifest("MAGIC"), Bits(Trig.Magic));
        Assert.Equal(FromManifest("LIMIT"), Bits(Trig.Limit));
        Assert.Equal(FromManifest("TINY"), Bits(Trig.Tiny));
        for (var n = 1; n <= 9; n++)
        {
            Assert.Equal(FromManifest("S" + n), Bits(Trig.S[n]));
        }

        for (var n = 2; n <= 9; n++)
        {
            Assert.Equal(FromManifest("C" + n), Bits(Trig.C[n]));
        }

        Assert.Equal(FromManifest("RAD_PER_DEG"), Bits(Trig.RadPerDeg));
        Assert.Equal(FromManifest("DEG_PER_RAD"), Bits(Trig.DegPerRad));
    }

    [Fact]
    public void DecimalLiteralsHaveThePinnedBits()
    {
        Assert.Equal(0x3CD203AF9EE75616UL, Bits(1e-15));
        Assert.Equal(0x7FE1CCF385EBC8A0UL, Bits(1.0e308));
        Assert.Equal(0x3C670EF54646D497UL, Bits(1e-17));
        Assert.Equal(0x01A56E1FC2F8F359UL, Bits(1e-300));
        Assert.Equal(0x3E40000000000000UL, Bits(7.450580596923828e-9));
    }

    [Fact]
    public void DegreeConstantsEqualTheDivisions()
    {
        Assert.Equal(Trig.PI / 180.0, Trig.RadPerDeg);
        Assert.Equal(180.0 / Trig.PI, Trig.DegPerRad);
    }
}
