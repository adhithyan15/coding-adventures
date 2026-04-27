namespace CodingAdventures.Clock.Tests;

public sealed class ClockTests
{
    [Fact]
    public void ClockStartsInKnownState()
    {
        var clock = new Clock();

        Assert.Equal(1_000_000, clock.FrequencyHz);
        Assert.Equal(0, clock.Cycle);
        Assert.Equal(0, clock.Value);
        Assert.Equal(0, clock.TotalTicks);
    }

    [Fact]
    public void TickAlternatesEdgesAndCycles()
    {
        var clock = new Clock();

        var first = clock.Tick();
        var second = clock.Tick();
        var third = clock.Tick();

        Assert.Equal(new ClockEdge(1, 1, true, false), first);
        Assert.Equal(new ClockEdge(1, 0, false, true), second);
        Assert.Equal(new ClockEdge(2, 1, true, false), third);
        Assert.Equal(3, clock.TotalTicks);
    }

    [Fact]
    public void FullCycleReturnsRisingThenFalling()
    {
        var clock = new Clock();

        var (rising, falling) = clock.FullCycle();

        Assert.True(rising.IsRising);
        Assert.True(falling.IsFalling);
        Assert.Equal(0, clock.Value);
        Assert.Equal(1, clock.Cycle);
        Assert.Equal(2, clock.TotalTicks);
    }

    [Fact]
    public void RunProducesTwoEdgesPerCycle()
    {
        var clock = new Clock();

        var edges = clock.Run(5);

        Assert.Equal(10, edges.Count);
        Assert.Equal(5, clock.Cycle);
        Assert.Empty(clock.Run(0));
    }

    [Fact]
    public void ListenersReceiveEdgesAndCanBeRemoved()
    {
        var clock = new Clock();
        var received = new List<ClockEdge>();
        void Listener(ClockEdge edge) => received.Add(edge);

        clock.RegisterListener(Listener);
        clock.Run(2);
        clock.UnregisterListener(Listener);
        clock.Tick();

        Assert.Equal(4, received.Count);
        Assert.Throws<ArgumentException>(() => clock.UnregisterListener(Listener));
    }

    [Fact]
    public void ResetPreservesListenersAndFrequency()
    {
        var clock = new Clock(5_000_000);
        var received = new List<ClockEdge>();
        clock.RegisterListener(received.Add);

        clock.Run(3);
        clock.Reset();
        clock.Tick();

        Assert.Equal(5_000_000, clock.FrequencyHz);
        Assert.Equal(1, received.Last().Cycle);
        Assert.Equal(1, clock.Cycle);
        Assert.Equal(1, clock.TotalTicks);
    }

    [Theory]
    [InlineData(1_000_000, 1000.0)]
    [InlineData(1_000_000_000, 1.0)]
    public void PeriodUsesFrequency(long frequency, double expectedPeriodNs)
    {
        var clock = new Clock(frequency);

        Assert.Equal(expectedPeriodNs, clock.PeriodNs, precision: 10);
    }

    [Fact]
    public void ClockRejectsInvalidFrequency()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new Clock(0));
        Assert.Throws<ArgumentOutOfRangeException>(() => new Clock(-1));
    }

    [Fact]
    public void DividerProducesOutputCyclesFromRisingEdges()
    {
        var master = new Clock(1_000_000_000);
        var divider = new ClockDivider(master, 4);

        master.Run(8);

        Assert.Equal(250_000_000, divider.Output.FrequencyHz);
        Assert.Equal(2, divider.Output.Cycle);
        Assert.Equal(0, divider.Output.Value);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(-1)]
    public void DividerRejectsInvalidDivisor(long divisor)
    {
        var master = new Clock();

        Assert.Throws<ArgumentOutOfRangeException>(() => new ClockDivider(master, divisor));
    }

    [Fact]
    public void MultiPhaseClockRotatesOneActivePhase()
    {
        var master = new Clock();
        var phases = new MultiPhaseClock(master, 4);

        for (var expected = 0; expected < 4; expected++)
        {
            master.Tick();

            for (var phase = 0; phase < 4; phase++)
            {
                Assert.Equal(phase == expected ? 1 : 0, phases.GetPhase(phase));
            }

            master.Tick();
        }

        master.Tick();
        Assert.Equal(1, phases.GetPhase(0));
    }

    [Fact]
    public void MultiPhaseClockRejectsInvalidPhaseCounts()
    {
        var master = new Clock();

        Assert.Throws<ArgumentOutOfRangeException>(() => new MultiPhaseClock(master, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => new MultiPhaseClock(master, 0));
    }
}
