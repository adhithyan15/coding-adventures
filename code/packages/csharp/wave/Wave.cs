namespace CodingAdventures.Wave;

using TrigFunctions = CodingAdventures.Trig.Trig;

/// <summary>
/// Immutable sinusoidal wave: y(t) = A * sin(2*pi*f*t + phase).
/// </summary>
public sealed class Wave
{
    /// <summary>Create a wave with optional phase in radians.</summary>
    public Wave(double amplitude, double frequency, double phase = 0.0)
    {
        if (!double.IsFinite(amplitude) || amplitude < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(amplitude), "Amplitude must be non-negative");
        }

        if (!double.IsFinite(frequency) || frequency <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(frequency), "Frequency must be positive");
        }

        if (frequency > double.MaxValue / (2.0 * TrigFunctions.PI))
        {
            throw new ArgumentOutOfRangeException(nameof(frequency), "Angular frequency must remain finite");
        }

        if (!double.IsFinite(phase))
        {
            throw new ArgumentOutOfRangeException(nameof(phase), "Phase must be finite");
        }

        Amplitude = amplitude;
        Frequency = frequency;
        Phase = phase;
    }

    /// <summary>Peak displacement.</summary>
    public double Amplitude { get; }

    /// <summary>Cycles per second in hertz.</summary>
    public double Frequency { get; }

    /// <summary>Starting offset in radians.</summary>
    public double Phase { get; }

    /// <summary>Time for one complete cycle, in seconds.</summary>
    public double Period => 1.0 / Frequency;

    /// <summary>Angular frequency in radians per second.</summary>
    public double AngularFrequency => 2.0 * TrigFunctions.PI * Frequency;

    /// <summary>Evaluate the wave at time <paramref name="time"/> in seconds.</summary>
    public double Evaluate(double time)
    {
        if (!double.IsFinite(time))
        {
            throw new ArgumentOutOfRangeException(nameof(time), "Time must be finite");
        }

        if (Amplitude == 0.0)
        {
            return 0.0;
        }

        var reducedTime = time % Period;
        var reducedPhase = Phase % (2.0 * TrigFunctions.PI);
        var angle = 2.0 * TrigFunctions.PI * (Frequency * reducedTime) + reducedPhase;
        var unitValue = TrigFunctions.Sin(angle);
        if (unitValue >= 1.0) return Amplitude;
        if (unitValue <= -1.0) return -Amplitude;
        return Amplitude * unitValue;
    }
}
