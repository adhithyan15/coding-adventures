namespace CodingAdventures.Wave;

/// <summary>
/// Immutable sinusoidal wave: y(t) = A * sin(2*pi*f*t + phase).
/// </summary>
public sealed class Wave
{
    /// <summary>Create a wave with optional phase in radians.</summary>
    public Wave(double amplitude, double frequency, double phase = 0.0)
    {
        if (amplitude < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(amplitude), "Amplitude must be non-negative");
        }

        if (frequency <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(frequency), "Frequency must be positive");
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
    public double AngularFrequency => 2.0 * Math.PI * Frequency;

    /// <summary>Evaluate the wave at time <paramref name="time"/> in seconds.</summary>
    public double Evaluate(double time) => Amplitude * Math.Sin(AngularFrequency * time + Phase);
}
