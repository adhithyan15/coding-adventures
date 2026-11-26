using System.Globalization;

namespace CodingAdventures.LogicGates.Tests;

/// <summary>LGE-CS-HARDEN: null-safe names and culture-invariant messages.</summary>
public class HardeningTests
{
    [Fact]
    public void NullParameterNameIsRejected()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => new InvalidBitException(null!, 2));
        Assert.Equal("parameterName", ex.ParamName);
    }

    [Fact]
    public void ParameterNameIsNeverNull()
    {
        var ex = new InvalidBitException("a", 7);
        Assert.Equal("a", ex.ParameterName);
        Assert.Equal("a", ex.ParamName);
        Assert.Equal(7, ex.Value);
    }

    [Fact]
    public void AllNaryGatesRejectNullArray()
    {
        Assert.Throws<ArgumentNullException>(() => LogicGates.AndN(null!));
        Assert.Throws<ArgumentNullException>(() => LogicGates.OrN(null!));
        Assert.Throws<ArgumentNullException>(() => LogicGates.XorN(null!));
    }

    private static void UnderCulture(Action body)
    {
        var saved = CultureInfo.CurrentCulture;
        try
        {
            var culture = (CultureInfo)CultureInfo.InvariantCulture.Clone();
            culture.NumberFormat.NegativeSign = "−NEG−";
            culture.NumberFormat.NumberGroupSeparator = "'";
            CultureInfo.CurrentCulture = culture;
            body();
        }
        finally
        {
            CultureInfo.CurrentCulture = saved;
        }
    }

    [Fact]
    public void BitMessageIgnoresCurrentCulture()
    {
        UnderCulture(() =>
        {
            var ex = Assert.ThrowsAny<InvalidBitException>(() => LogicGates.And(1, -5));
            Assert.Contains("got -5.", ex.Message);
            Assert.Contains("Actual value was -5.", ex.Message);
            Assert.DoesNotContain("NEG", ex.Message);
            Assert.Equal(-5, ex.Value);
        });
    }

    [Fact]
    public void NaryMessagesIgnoreCurrentCulture()
    {
        UnderCulture(() =>
        {
            var bad = Assert.ThrowsAny<InvalidBitException>(() => LogicGates.AndN(1, -9));
            Assert.Equal("inputs[1]", bad.ParamName);
            Assert.DoesNotContain("NEG", bad.Message);

            var tooMany = Assert.Throws<ArgumentException>(() => LogicGates.OrN(new int[LogicGates.MaxInputs + 1]));
            Assert.Contains("65536", tooMany.Message);
            Assert.Contains("65537", tooMany.Message);

            var few = Assert.Throws<ArgumentException>(() => LogicGates.AndN(1));
            Assert.Contains("at least 2", few.Message);
        });
    }
}
