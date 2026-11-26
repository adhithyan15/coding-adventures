namespace CodingAdventures.TestBootstrap.Fuzz;

/// <summary>
/// The closed vocabulary of fuzz failure ids (VER06 section 6). Only these five strings may appear
/// in a public failure line.
/// </summary>
/// <remarks>
/// <c>fuzz-leak</c> and <c>fuzz-panic</c> are registered but this engine cannot produce them
/// (VER06h section 9, item 7).
/// </remarks>
public static class FuzzFailureIds
{
    /// <summary>A target crashed.</summary>
    public const string Crash = "fuzz-crash";

    /// <summary>A target exhausted its step budget.</summary>
    public const string Timeout = "fuzz-timeout";

    /// <summary>A target exhausted its memory budget.</summary>
    public const string Oom = "fuzz-oom";

    /// <summary>Registered; unproducible here.</summary>
    public const string Leak = "fuzz-leak";

    /// <summary>Registered; unproducible here.</summary>
    public const string Panic = "fuzz-panic";

    private static readonly string[] Registered = { Crash, Timeout, Oom, Leak, Panic };

    /// <summary>Every registered id, in registration order (a copy).</summary>
    public static IReadOnlyList<string> All => (string[])Registered.Clone();

    /// <summary>
    /// Whether <paramref name="candidate"/> is one of the five registered ids.
    /// <code>
    ///   IsFuzzFailureId("fuzz-crash")           -> true
    ///   IsFuzzFailureId("fuzz-arbitrary-error") -> false
    /// </code>
    /// </summary>
    public static bool IsFuzzFailureId(string? candidate)
    {
        return candidate != null && Array.IndexOf(Registered, candidate) >= 0;
    }
}
