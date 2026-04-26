namespace CodingAdventures.Resistor;

public static class ResistorPackage
{
    public const string Version = "0.1.0";
}

public sealed record Resistor
{
    public Resistor(
        double resistanceOhms,
        double tolerance = 0.0,
        double tempcoPpmPerC = 0.0,
        double? powerRatingWatts = null)
    {
        if (resistanceOhms <= 0.0)
        {
            throw new ArgumentOutOfRangeException(nameof(resistanceOhms), "Resistance must be > 0 ohms.");
        }

        if (tolerance < 0.0)
        {
            throw new ArgumentOutOfRangeException(nameof(tolerance), "Tolerance must be >= 0.");
        }

        if (powerRatingWatts is <= 0.0)
        {
            throw new ArgumentOutOfRangeException(nameof(powerRatingWatts), "Power rating must be > 0 watts when provided.");
        }

        ResistanceOhms = resistanceOhms;
        Tolerance = tolerance;
        TempcoPpmPerC = tempcoPpmPerC;
        PowerRatingWatts = powerRatingWatts;
    }

    public double ResistanceOhms { get; }
    public double Tolerance { get; }
    public double TempcoPpmPerC { get; }
    public double? PowerRatingWatts { get; }

    public double Conductance()
    {
        return 1.0 / ResistanceOhms;
    }

    public double CurrentForVoltage(double voltage)
    {
        return voltage / ResistanceOhms;
    }

    public double VoltageForCurrent(double current)
    {
        return current * ResistanceOhms;
    }

    public double PowerForVoltage(double voltage)
    {
        return voltage * voltage / ResistanceOhms;
    }

    public double PowerForCurrent(double current)
    {
        return current * current * ResistanceOhms;
    }

    public double EnergyForVoltage(double voltage, double durationSeconds)
    {
        ValidateDuration(durationSeconds);
        return PowerForVoltage(voltage) * durationSeconds;
    }

    public double EnergyForCurrent(double current, double durationSeconds)
    {
        ValidateDuration(durationSeconds);
        return PowerForCurrent(current) * durationSeconds;
    }

    public double MinResistance()
    {
        return ResistanceOhms * (1.0 - Tolerance);
    }

    public double MaxResistance()
    {
        return ResistanceOhms * (1.0 + Tolerance);
    }

    public double ResistanceAtTemperature(double celsius, double referenceCelsius = 25.0)
    {
        var alpha = TempcoPpmPerC * 1e-6;
        var deltaT = celsius - referenceCelsius;
        return ResistanceOhms * (1.0 + alpha * deltaT);
    }

    public bool IsWithinPowerRatingForVoltage(double voltage)
    {
        return PowerRatingWatts is null || PowerForVoltage(voltage) <= PowerRatingWatts;
    }

    public bool IsWithinPowerRatingForCurrent(double current)
    {
        return PowerRatingWatts is null || PowerForCurrent(current) <= PowerRatingWatts;
    }

    private static void ValidateDuration(double durationSeconds)
    {
        if (durationSeconds < 0.0)
        {
            throw new ArgumentOutOfRangeException(nameof(durationSeconds), "Duration must be >= 0 seconds.");
        }
    }
}

public static class ResistorNetwork
{
    public static double SeriesEquivalent(IEnumerable<Resistor> resistors)
    {
        var (items, count) = Materialize(resistors);
        if (count == 0)
        {
            throw new ArgumentException("At least one resistor is required.", nameof(resistors));
        }

        return items.Sum(resistor => resistor.ResistanceOhms);
    }

    public static double ParallelEquivalent(IEnumerable<Resistor> resistors)
    {
        var (items, count) = Materialize(resistors);
        if (count == 0)
        {
            throw new ArgumentException("At least one resistor is required.", nameof(resistors));
        }

        return 1.0 / items.Sum(resistor => 1.0 / resistor.ResistanceOhms);
    }

    public static double VoltageDivider(double vin, Resistor rTop, Resistor rBottom)
    {
        ArgumentNullException.ThrowIfNull(rTop);
        ArgumentNullException.ThrowIfNull(rBottom);

        var total = rTop.ResistanceOhms + rBottom.ResistanceOhms;
        return vin * (rBottom.ResistanceOhms / total);
    }

    private static (IReadOnlyList<Resistor> Items, int Count) Materialize(IEnumerable<Resistor> resistors)
    {
        ArgumentNullException.ThrowIfNull(resistors);
        var items = resistors.ToList();
        foreach (var resistor in items)
        {
            ArgumentNullException.ThrowIfNull(resistor);
        }

        return (items, items.Count);
    }
}
