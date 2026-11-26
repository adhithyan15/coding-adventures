namespace CodingAdventures.TestBootstrap.Pbt;

/// <summary>
/// Thrown by <see cref="Gen{T}.Sample"/> when generation fails on data (a filter
/// that never accepts). Combinators propagate it; the runner reports
/// <c>property-exhausted</c>.
/// </summary>
public sealed class ExhaustedException : Exception
{
    /// <summary>Creates the exception.</summary>
    public ExhaustedException(string message)
        : base(message)
    {
    }
}

/// <summary>
/// Thrown when a shrinker is handed a start value that does not falsify the
/// property: a caller bug, never a verdict.
/// </summary>
public sealed class NotFalsifyingException : InvalidOperationException
{
    /// <summary>Creates the exception.</summary>
    public NotFalsifyingException()
        : base("the start value does not falsify the property")
    {
    }
}

/// <summary>Thrown by <see cref="Verdict.AssertPassed"/> for a failing verdict.</summary>
public sealed class PropertyFailedException : Exception
{
    /// <summary>Creates the exception with the failure id and message.</summary>
    public PropertyFailedException(string failureId, string message)
        : base("[" + failureId + "] " + message)
    {
        FailureId = failureId;
    }

    /// <summary>The closed failure id.</summary>
    public string FailureId { get; }
}

/// <summary>
/// The result of shrinking: the smallest value found that still falsifies, and how
/// many <em>accepted</em> moves it took (the target probe is one move).
/// </summary>
public readonly record struct Shrunk<T>(T Value, int Steps);

/// <summary>
/// Shrinks a failing value. <c>falsifies(v)</c> is true while <c>v</c> still makes
/// the property fail; the shrinker returns a value that still falsifies (or the
/// original) and the number of accepted moves.
/// </summary>
public delegate Shrunk<T> Shrinker<T>(T value, Func<T, bool> falsifies, int maxSteps);

/// <summary>
/// Whether output is public CI output. The default, <see cref="Unset"/>, redacts:
/// the engine fails closed, because a seed plus a property is a working exploit
/// reproducer (VER00 section 7.3) and this package cannot read the environment to
/// find out where it is running.
/// </summary>
public enum CiMode
{
    /// <summary>Treated as CI: witnesses and seeds are redacted.</summary>
    Unset,

    /// <summary>Redacts.</summary>
    On,

    /// <summary>Local development: messages carry the seed and the witness.</summary>
    Off,
}

/// <summary>
/// Configures <see cref="Runner"/>.
/// </summary>
/// <param name="NumTests">How many cases to run. 0 is legal and is a failure: a property that never ran proves nothing.</param>
/// <param name="MaxShrinkSteps">The shrink budget, 0..<see cref="Limits.MaxShrinkSteps"/>.</param>
/// <param name="Ci">Redaction mode.</param>
public sealed record Options(int NumTests, int MaxShrinkSteps, CiMode Ci)
{
    /// <summary>100 cases, 1000 shrink steps and CI redaction.</summary>
    public static Options Defaults()
    {
        return new Options(100, 1000, CiMode.Unset);
    }

    /// <summary>The same options with another CI mode.</summary>
    public Options WithCi(CiMode mode)
    {
        return this with { Ci = mode };
    }

    /// <summary>
    /// Pure CI detection. Pass <c>Environment.GetEnvironmentVariable</c>: it
    /// reports true when <c>CI</c> is set to anything except empty, "0" or "false"
    /// (trimmed, any case). The package calls no environment function itself.
    /// </summary>
    public static bool CiFromLookup(Func<string, string?> lookup)
    {
        string? value = lookup("CI");
        if (value == null)
        {
            return false;
        }

        string v = value.Trim().ToLowerInvariant();
        return !(v.Length == 0 || v == "0" || v == "false");
    }
}

/// <summary>The closed failure vocabulary (VER05 section 5.1).</summary>
public static class FailureIds
{
    /// <summary>The property was falsified; a minimal witness was found.</summary>
    public const string Counterexample = "property-counterexample";

    /// <summary>Nothing (or no valid input) was evaluated.</summary>
    public const string Exhausted = "property-exhausted";

    /// <summary>Registered but never produced by this lane.</summary>
    public const string Timeout = "property-timeout";

    /// <summary>The property itself threw.</summary>
    public const string Panic = "property-panic";

    /// <summary>The closed set of ids, in registration order.</summary>
    public static readonly IReadOnlyList<string> All = new[] { Counterexample, Exhausted, Timeout, Panic };

    /// <summary>
    /// True only for one of the four registered ids.
    /// <code>
    ///   IsPbtFailureId("property-panic")   -> true
    ///   IsPbtFailureId("unregistered-id")  -> false
    /// </code>
    /// </summary>
    public static bool IsPbtFailureId(string? candidate)
    {
        return candidate != null && All.Contains(candidate);
    }
}

/// <summary>
/// The outcome of <see cref="Runner"/>. In redacted mode <c>Seed</c>,
/// <c>Counterexample</c> and <c>Minimal</c> are null; only the message, iteration,
/// step count and digest remain.
/// </summary>
public sealed record Verdict(
    bool Passed,
    string? FailureId,
    string? Message,
    int CasesRun,
    int Iteration,
    int ShrinkSteps,
    ulong? Seed,
    object? Counterexample,
    object? Minimal,
    string? Digest)
{
    /// <summary>The negation of <see cref="Passed"/>.</summary>
    public bool Failed => !Passed;

    /// <summary>Throws <see cref="PropertyFailedException"/> carrying the failure id unless the verdict is a pass.</summary>
    public void AssertPassed()
    {
        if (!Passed)
        {
            throw new PropertyFailedException(FailureId ?? "unknown", Message ?? string.Empty);
        }
    }
}
