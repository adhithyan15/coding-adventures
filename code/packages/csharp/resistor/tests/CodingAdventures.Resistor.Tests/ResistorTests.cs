using CodingAdventures.Resistor;

namespace CodingAdventures.Resistor.Tests;

public sealed class ResistorTests
{
    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", ResistorPackage.Version);
    }

    [Fact]
    public void StoresNominalProperties()
    {
        var resistor = new Resistor(1_000.0, tolerance: 0.01, tempcoPpmPerC: 100.0, powerRatingWatts: 0.25);

        Assert.Equal(1_000.0, resistor.ResistanceOhms);
        Assert.Equal(0.01, resistor.Tolerance);
        Assert.Equal(100.0, resistor.TempcoPpmPerC);
        Assert.Equal(0.25, resistor.PowerRatingWatts);
    }

    [Fact]
    public void RejectsInvalidConstruction()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new Resistor(0.0));
        Assert.Throws<ArgumentOutOfRangeException>(() => new Resistor(100.0, tolerance: -0.01));
        Assert.Throws<ArgumentOutOfRangeException>(() => new Resistor(100.0, powerRatingWatts: 0.0));
    }

    [Fact]
    public void OhmsLawHelpersUseResistance()
    {
        var resistor = new Resistor(1_000.0);

        Assert.Equal(0.001, resistor.Conductance(), 12);
        Assert.Equal(0.005, resistor.CurrentForVoltage(5.0), 12);
        Assert.Equal(5.0, resistor.VoltageForCurrent(0.005), 12);
        Assert.Equal(-0.002, resistor.CurrentForVoltage(-2.0), 12);
    }

    [Fact]
    public void PowerAndEnergyHelpersAgree()
    {
        var resistor = new Resistor(1_000.0);

        Assert.Equal(0.025, resistor.PowerForVoltage(5.0), 12);
        Assert.Equal(0.025, resistor.PowerForCurrent(0.005), 12);
        Assert.Equal(0.05, resistor.EnergyForVoltage(5.0, 2.0), 12);
        Assert.Equal(0.05, resistor.EnergyForCurrent(0.005, 2.0), 12);
    }

    [Fact]
    public void EnergyRejectsNegativeDurations()
    {
        var resistor = new Resistor(100.0);

        Assert.Throws<ArgumentOutOfRangeException>(() => resistor.EnergyForVoltage(10.0, -1.0));
        Assert.Throws<ArgumentOutOfRangeException>(() => resistor.EnergyForCurrent(0.1, -1.0));
    }

    [Fact]
    public void ToleranceAndTemperatureHelpersWork()
    {
        var resistor = new Resistor(1_000.0, tolerance: 0.01, tempcoPpmPerC: 100.0);

        Assert.Equal(990.0, resistor.MinResistance(), 12);
        Assert.Equal(1_010.0, resistor.MaxResistance(), 12);
        Assert.Equal(1_005.0, resistor.ResistanceAtTemperature(75.0), 12);
        Assert.Equal(995.0, resistor.ResistanceAtTemperature(-25.0), 12);
    }

    [Fact]
    public void PowerRatingChecksRespectOptionalRating()
    {
        var unrated = new Resistor(100.0);
        var rated = new Resistor(100.0, powerRatingWatts: 0.25);

        Assert.True(unrated.IsWithinPowerRatingForVoltage(1_000.0));
        Assert.True(unrated.IsWithinPowerRatingForCurrent(100.0));
        Assert.True(rated.IsWithinPowerRatingForVoltage(5.0));
        Assert.False(rated.IsWithinPowerRatingForVoltage(10.0));
        Assert.True(rated.IsWithinPowerRatingForCurrent(0.04));
        Assert.False(rated.IsWithinPowerRatingForCurrent(0.1));
    }

    [Fact]
    public void NetworkHelpersComputeEquivalentResistance()
    {
        var resistors = new[] { new Resistor(100.0), new Resistor(200.0), new Resistor(300.0) };

        Assert.Equal(600.0, ResistorNetwork.SeriesEquivalent(resistors), 12);
        Assert.Equal(500.0, ResistorNetwork.ParallelEquivalent([new Resistor(1_000.0), new Resistor(1_000.0)]), 12);
    }

    [Fact]
    public void NetworkHelpersValidateResistorLists()
    {
        Assert.Throws<ArgumentNullException>(() => ResistorNetwork.SeriesEquivalent(null!));
        Assert.Throws<ArgumentException>(() => ResistorNetwork.SeriesEquivalent([]));
        Assert.Throws<ArgumentNullException>(() => ResistorNetwork.SeriesEquivalent([null!]));
        Assert.Throws<ArgumentException>(() => ResistorNetwork.ParallelEquivalent([]));
    }

    [Fact]
    public void VoltageDividerUsesBottomResistanceRatio()
    {
        var top = new Resistor(1_000.0);
        var bottom = new Resistor(1_000.0);

        Assert.Equal(2.5, ResistorNetwork.VoltageDivider(5.0, top, bottom), 12);
        Assert.Throws<ArgumentNullException>(() => ResistorNetwork.VoltageDivider(5.0, null!, bottom));
        Assert.Throws<ArgumentNullException>(() => ResistorNetwork.VoltageDivider(5.0, top, null!));
    }
}
