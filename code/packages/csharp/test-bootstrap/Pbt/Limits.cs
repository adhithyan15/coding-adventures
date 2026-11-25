namespace CodingAdventures.TestBootstrap.Pbt;

/// <summary>
/// Bounds that keep every loop and allocation finite (CWE-400). A caller or a
/// fixture can ask for more only by being refused.
/// </summary>
public static class Limits
{
    /// <summary>The most cases <see cref="Runner.Check{T}(ulong, Gen{T}, Func{T, bool}, Options?, Shrinker{T}?)"/> will run.</summary>
    public const int MaxCases = 100_000;

    /// <summary>The largest shrink budget.</summary>
    public const int MaxShrinkSteps = 10_000;

    /// <summary>The longest <see cref="Gens.Bytes"/> generator.</summary>
    public const int MaxInputBytes = 1 << 20;

    /// <summary>The longest <see cref="Gens.ListOf{T}"/> generator.</summary>
    public const int MaxListLen = 100_000;

    /// <summary>The most values any corpus entry point (GenPrimitive, GenBounded, GenBytes) will generate.</summary>
    public const int MaxGenCount = 1_000_000;

    /// <summary>
    /// The total attempt budget of shrinking is <c>AttemptFactor * maxSteps + AttemptSlack</c>:
    /// rejected attempts cost a list copy each but are not accepted moves, so they
    /// need their own bound (VER05e section 4).
    /// </summary>
    public const int AttemptFactor = 16;

    /// <summary>See <see cref="AttemptFactor"/>.</summary>
    public const int AttemptSlack = 64;

    /// <summary>Bounds count x maxLen for <see cref="Gens.GenBytes"/> and count x width for GenBounded.</summary>
    public const int MaxGenBytesTotal = 1 << 20;

    /// <summary>The largest total weight of <see cref="Gens.Frequency{T}"/> (VER05: 2^32).</summary>
    public const long MaxFrequencyTotal = 1L << 32;

    /// <summary>
    /// The most list elements one whole shrink may copy, summed over every attempt of every
    /// nested shrinker. An attempt copies its candidate, so counting attempts alone would
    /// let a 1 MiB witness cost ~16,000 x 1M element copies.
    /// </summary>
    public const long MaxShrinkWork = 64L * 1024 * 1024;

    /// <summary>The largest <c>maxRetries</c> a <see cref="Gens.Filter{T}(Gen{T}, Func{T, bool}, int)"/> accepts.</summary>
    public const int MaxRetries = 10_000;

    /// <summary>How many samples a filter tries before giving up.</summary>
    public const int DefaultMaxRetries = 100;

    /// <summary>The attempt budget for a shrink with <paramref name="maxSteps"/> accepted moves.</summary>
    public static long AttemptBudget(int maxSteps)
    {
        return ((long)AttemptFactor * maxSteps) + AttemptSlack;
    }
}
