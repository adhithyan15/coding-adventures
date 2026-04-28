namespace CodingAdventures.Wave.Tests;

public sealed class WaveTests
{
    private const double Epsilon = 1e-10;

    [Fact]
    public void Constructor_StoresAmplitudeFrequencyAndDefaultPhase()
    {
        var wave = new Wave(1.0, 440.0);

        Assert.Equal(1.0, wave.Amplitude);
        Assert.Equal(440.0, wave.Frequency);
        Assert.Equal(0.0, wave.Phase);
    }

    [Fact]
    public void Constructor_StoresExplicitPhase()
    {
        var wave = new Wave(2.0, 100.0, Math.PI / 2.0);

        Assert.Equal(Math.PI / 2.0, wave.Phase, Epsilon);
    }

    [Fact]
    public void Period_IsInverseFrequency()
    {
        Assert.Equal(0.25, new Wave(1.0, 4.0).Period, Epsilon);
    }

    [Fact]
    public void AngularFrequency_IsTwoPiTimesFrequency()
    {
        Assert.Equal(2.0 * Math.PI, new Wave(1.0, 1.0).AngularFrequency, Epsilon);
    }

    [Fact]
    public void Evaluate_HandlesZeroCrossingPeakAndTrough()
    {
        Assert.Equal(0.0, new Wave(1.0, 1.0).Evaluate(0.0), Epsilon);
        Assert.Equal(3.0, new Wave(3.0, 1.0).Evaluate(0.25), 1e-9);
        Assert.Equal(-2.0, new Wave(2.0, 1.0).Evaluate(0.75), 1e-9);
    }

    [Fact]
    public void Evaluate_IsPeriodic()
    {
        var wave = new Wave(2.0, 5.0);
        const double time = 0.123;

        Assert.Equal(wave.Evaluate(time), wave.Evaluate(time + wave.Period), 1e-9);
    }

    [Fact]
    public void Phase_ShiftsWave()
    {
        Assert.Equal(1.0, new Wave(1.0, 1.0, Math.PI / 2.0).Evaluate(0.0), 1e-9);

        var wave = new Wave(1.0, 1.0, 0.0);
        var opposite = new Wave(1.0, 1.0, Math.PI);
        Assert.Equal(0.0, wave.Evaluate(0.3) + opposite.Evaluate(0.3), 1e-9);
    }

    [Fact]
    public void ZeroAmplitude_AlwaysEvaluatesToZero()
    {
        Assert.Equal(0.0, new Wave(0.0, 1.0).Evaluate(0.5), Epsilon);
    }

    [Fact]
    public void InvalidParameters_Throw()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new Wave(-1.0, 1.0));
        Assert.Throws<ArgumentOutOfRangeException>(() => new Wave(1.0, 0.0));
        Assert.Throws<ArgumentOutOfRangeException>(() => new Wave(1.0, -1.0));
    }

    [Fact]
    public void HighFrequency_HasExpectedPeriodAndQuarterCyclePeak()
    {
        var wave = new Wave(1.0, 1000.0);

        Assert.Equal(0.001, wave.Period, Epsilon);
        Assert.Equal(1.0, wave.Evaluate(0.00025), 1e-8);
    }
}
