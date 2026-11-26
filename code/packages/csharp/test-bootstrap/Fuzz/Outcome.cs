namespace CodingAdventures.TestBootstrap.Fuzz;

/// <summary>The four outcomes of one execution.</summary>
public enum OutcomeKind
{
    /// <summary>The target returned normally.</summary>
    Pass,

    /// <summary>The target threw.</summary>
    Crash,

    /// <summary>The target exhausted its step budget.</summary>
    Timeout,

    /// <summary>The target exhausted its memory budget (or raised a real OutOfMemoryException).</summary>
    Oom,
}

/// <summary>
/// How one execution ended. <see cref="CrashKind"/> and <see cref="Frames"/> are meaningful only for
/// <see cref="OutcomeKind.Crash"/>; they never carry a message or input bytes.
/// </summary>
public sealed record Outcome(OutcomeKind Kind, string CrashKind, IReadOnlyList<string> Frames)
{
    /// <summary>The target returned.</summary>
    public static Outcome Pass() => new(OutcomeKind.Pass, string.Empty, Array.Empty<string>());

    /// <summary>The step budget ran out.</summary>
    public static Outcome Timeout() => new(OutcomeKind.Timeout, string.Empty, Array.Empty<string>());

    /// <summary>The memory budget ran out.</summary>
    public static Outcome Oom() => new(OutcomeKind.Oom, string.Empty, Array.Empty<string>());

    /// <summary>A crash with an already-normalised kind and frames (the list is copied).</summary>
    public static Outcome Crash(string kind, IEnumerable<string> frames)
    {
        return new Outcome(OutcomeKind.Crash, kind, Array.AsReadOnly(frames.ToArray()));
    }
}
