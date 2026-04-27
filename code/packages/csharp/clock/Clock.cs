namespace CodingAdventures.Clock;

/// <summary>
/// Record of one clock transition.
/// </summary>
public sealed record ClockEdge(long Cycle, int Value, bool IsRising, bool IsFalling);

/// <summary>
/// Square-wave clock generator for digital circuit simulations.
/// </summary>
public sealed class Clock
{
    private readonly List<Action<ClockEdge>> _listeners = [];
    private long _totalTicks;

    /// <summary>
    /// Create a clock with the requested frequency in Hz.
    /// </summary>
    public Clock(long frequencyHz = 1_000_000)
    {
        if (frequencyHz <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(frequencyHz), "frequency_hz must be > 0.");
        }

        FrequencyHz = frequencyHz;
    }

    /// <summary>The clock frequency in Hz.</summary>
    public long FrequencyHz { get; }

    /// <summary>The current cycle number, incremented on rising edges.</summary>
    public long Cycle { get; private set; }

    /// <summary>The current signal value, either 0 or 1.</summary>
    public int Value { get; private set; }

    /// <summary>Total half-cycles elapsed since construction or reset.</summary>
    public long TotalTicks => _totalTicks;

    /// <summary>The period of one complete clock cycle in nanoseconds.</summary>
    public double PeriodNs => 1_000_000_000d / FrequencyHz;

    /// <summary>
    /// Advance one half-cycle and notify all registered listeners.
    /// </summary>
    public ClockEdge Tick()
    {
        var oldValue = Value;
        Value = 1 - Value;
        _totalTicks++;

        var isRising = oldValue == 0 && Value == 1;
        var isFalling = oldValue == 1 && Value == 0;
        if (isRising)
        {
            Cycle++;
        }

        var edge = new ClockEdge(Cycle, Value, isRising, isFalling);
        foreach (var listener in _listeners.ToArray())
        {
            listener(edge);
        }

        return edge;
    }

    /// <summary>
    /// Execute a complete cycle, returning the rising and falling edges.
    /// </summary>
    public (ClockEdge Rising, ClockEdge Falling) FullCycle()
    {
        var rising = Tick();
        var falling = Tick();
        return (rising, falling);
    }

    /// <summary>
    /// Run the clock for the requested number of full cycles.
    /// </summary>
    public IReadOnlyList<ClockEdge> Run(long cycles)
    {
        if (cycles < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(cycles), "cycles must be >= 0.");
        }

        var edges = new List<ClockEdge>();
        for (var i = 0L; i < cycles; i++)
        {
            var (rising, falling) = FullCycle();
            edges.Add(rising);
            edges.Add(falling);
        }

        return edges;
    }

    /// <summary>
    /// Register a callback that receives every clock edge.
    /// </summary>
    public void RegisterListener(Action<ClockEdge> callback)
    {
        ArgumentNullException.ThrowIfNull(callback);
        _listeners.Add(callback);
    }

    /// <summary>
    /// Remove a previously registered callback.
    /// </summary>
    public void UnregisterListener(Action<ClockEdge> callback)
    {
        ArgumentNullException.ThrowIfNull(callback);
        if (!_listeners.Remove(callback))
        {
            throw new ArgumentException("Listener was not registered.", nameof(callback));
        }
    }

    /// <summary>
    /// Reset timing state while preserving registered listeners and frequency.
    /// </summary>
    public void Reset()
    {
        Cycle = 0;
        Value = 0;
        _totalTicks = 0;
    }
}

/// <summary>
/// Generates a slower output clock from a faster source clock.
/// </summary>
public sealed class ClockDivider
{
    private long _counter;

    /// <summary>
    /// Create a divider that registers itself on the source clock.
    /// </summary>
    public ClockDivider(Clock source, long divisor)
    {
        Source = source ?? throw new ArgumentNullException(nameof(source));
        if (divisor < 2)
        {
            throw new ArgumentOutOfRangeException(nameof(divisor), $"Divisor must be >= 2, got {divisor}.");
        }

        Divisor = divisor;
        Output = new Clock(source.FrequencyHz / divisor);
        Source.RegisterListener(OnEdge);
    }

    /// <summary>The source clock being divided.</summary>
    public Clock Source { get; }

    /// <summary>The integer division factor.</summary>
    public long Divisor { get; }

    /// <summary>The generated output clock.</summary>
    public Clock Output { get; }

    private void OnEdge(ClockEdge edge)
    {
        if (!edge.IsRising)
        {
            return;
        }

        _counter++;
        if (_counter < Divisor)
        {
            return;
        }

        _counter = 0;
        Output.Tick();
        Output.Tick();
    }
}

/// <summary>
/// Rotates one active phase across a set of non-overlapping clock phases.
/// </summary>
public sealed class MultiPhaseClock
{
    private readonly int[] _phaseValues;

    /// <summary>
    /// Create a multi-phase clock generator registered on the source clock.
    /// </summary>
    public MultiPhaseClock(Clock source, int phases = 4)
    {
        Source = source ?? throw new ArgumentNullException(nameof(source));
        if (phases < 2)
        {
            throw new ArgumentOutOfRangeException(nameof(phases), $"Phases must be >= 2, got {phases}.");
        }

        Phases = phases;
        _phaseValues = new int[phases];
        Source.RegisterListener(OnEdge);
    }

    /// <summary>The source clock driving the phase rotation.</summary>
    public Clock Source { get; }

    /// <summary>The number of generated phases.</summary>
    public int Phases { get; }

    /// <summary>The phase index that will be activated by the next rising edge.</summary>
    public int ActivePhase { get; private set; }

    /// <summary>
    /// Return the current value of a phase, either 0 or 1.
    /// </summary>
    public int GetPhase(int index)
    {
        if (index < 0 || index >= Phases)
        {
            throw new ArgumentOutOfRangeException(nameof(index));
        }

        return _phaseValues[index];
    }

    /// <summary>A snapshot of the current phase values.</summary>
    public IReadOnlyList<int> PhaseValues => Array.AsReadOnly((int[])_phaseValues.Clone());

    private void OnEdge(ClockEdge edge)
    {
        if (!edge.IsRising)
        {
            return;
        }

        Array.Fill(_phaseValues, 0);
        _phaseValues[ActivePhase] = 1;
        ActivePhase = (ActivePhase + 1) % Phases;
    }
}

/// <summary>
/// Package metadata.
/// </summary>
public static class ClockPackage
{
    /// <summary>The package version.</summary>
    public const string Version = "0.1.0";
}
