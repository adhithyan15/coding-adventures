namespace CodingAdventures.TestBootstrap.Fuzz;

/// <summary>
/// One deduplicated crash. The minimal input is PRIVATE (VER06 section 6.3): it is reachable only
/// through <see cref="MinimalInput"/>, never through <see cref="ToString"/>, and public output goes
/// through <see cref="FuzzSignature.FormatTapFailure"/> only.
/// </summary>
/// <remarks>
/// This is a class, not a record, precisely so the generated <c>ToString</c>, <c>Equals</c> and
/// property of a record cannot print the bytes.
/// </remarks>
public sealed class CrashRecord
{
    private byte[] _minimal;

    internal CrashRecord(string signature, string kind, IReadOnlyList<string> frames, byte[] minimal, bool minimized, bool dropped)
    {
        Signature = signature;
        Kind = kind;
        Frames = Array.AsReadOnly(frames.ToArray()); // a ReadOnlyCollection: not castable back to a writable array
        Hits = 1;
        Minimized = minimized;
        _minimal = minimal;
        InputDropped = dropped;
    }

    /// <summary>The 8-hex-digit signature.</summary>
    public string Signature { get; }

    /// <summary>The normalised crash kind (a type name).</summary>
    public string Kind { get; }

    /// <summary>The normalised frames, innermost first.</summary>
    public IReadOnlyList<string> Frames { get; }

    /// <summary>How many campaign executions hit this signature (triage included).</summary>
    public int Hits { get; private set; }

    /// <summary>
    /// False when minimisation was skipped or cut short by a budget, so the minimal input is only the
    /// shortest input seen, not a fixed point.
    /// </summary>
    public bool Minimized { get; }

    /// <summary>True when the retained-bytes cap forced the input to be dropped.</summary>
    public bool InputDropped { get; }

    /// <summary>A copy of the shortest input known to reproduce the signature. PRIVATE: never log it.</summary>
    public byte[] MinimalInput() => (byte[])_minimal.Clone();

    internal int MinimalLength => _minimal.Length;

    internal void Hit() => Hits++;

    internal void ReplaceMinimal(byte[] shorter) => _minimal = shorter;

    /// <summary>Signature, kind, frames and counts; never input bytes.</summary>
    public override string ToString()
    {
        return "CrashRecord{sig=" + Signature + " kind=" + Kind + " frames=[" + string.Join(", ", Frames)
            + "] hits=" + Hits + " minimized=" + Minimized + "}";
    }
}

/// <summary>
/// What a campaign found. Every execution lands in exactly one bucket. <see cref="Iterations"/>
/// counts MUTATION executions (triage is not counted); crashes, timeouts and OOMs count triage and
/// mutation executions; anything that passed is in the corpus or was redundant.
/// </summary>
/// <remarks>
/// The seed and the corpus are PRIVATE (VER06 section 6): reachable only through the explicit
/// <see cref="Seed"/> and <see cref="GetCorpus"/> accessors, never through <see cref="ToString"/>.
/// </remarks>
public sealed class FuzzReport
{
    /// <summary>Edges were reported by the target.</summary>
    public const string TierReported = "caller-reported";

    /// <summary>No edge was collected, so there was no feedback.</summary>
    public const string TierBlackBox = "black-box-uninstrumented";

    private readonly List<byte[]> _corpus;

    internal FuzzReport(
        int iterations,
        int admittedSeeds,
        int totalEdges,
        IReadOnlyList<CrashRecord> crashes,
        int timeouts,
        int ooms,
        int crashesDropped,
        int admissionsDropped,
        string coverageTier,
        ulong seed,
        List<byte[]> corpus)
    {
        Iterations = iterations;
        AdmittedSeeds = admittedSeeds;
        TotalEdges = totalEdges;
        Crashes = Array.AsReadOnly(crashes.ToArray());
        Timeouts = timeouts;
        Ooms = ooms;
        CrashesDropped = crashesDropped;
        AdmissionsDropped = admissionsDropped;
        CoverageTier = coverageTier;
        Seed = seed;
        _corpus = corpus;
    }

    /// <summary>Mutation executions run.</summary>
    public int Iterations { get; }

    /// <summary>Seeds in the final corpus.</summary>
    public int AdmittedSeeds { get; }

    /// <summary>Distinct covered edges.</summary>
    public int TotalEdges { get; }

    /// <summary>One record per distinct signature (at most <see cref="FuzzEngine.MaxCrashes"/>); read-only.</summary>
    public IReadOnlyList<CrashRecord> Crashes { get; }

    /// <summary>Executions that exhausted their step budget.</summary>
    public int Timeouts { get; }

    /// <summary>Executions that exhausted their memory budget.</summary>
    public int Ooms { get; }

    /// <summary>Novel signatures beyond <see cref="FuzzEngine.MaxCrashes"/>, so a cap never silently loses a failure.</summary>
    public int CrashesDropped { get; }

    /// <summary>Corpus admissions refused by the retained-bytes cap (VER06h section 6).</summary>
    public int AdmissionsDropped { get; }

    /// <summary><see cref="TierReported"/> or <see cref="TierBlackBox"/>: what was actually collected.</summary>
    public string CoverageTier { get; }

    /// <summary>
    /// Whether the campaign can be read as "found nothing": at least one mutation iteration ran and no
    /// execution, triage included, crashed, timed out or ran out of memory. A campaign that never got
    /// past triage, or ran zero iterations, fuzzed nothing.
    /// </summary>
    public bool IsClean => Iterations > 0 && Crashes.Count == 0 && Timeouts == 0 && Ooms == 0 && CrashesDropped == 0;

    /// <summary>The number of distinct crash signatures seen.</summary>
    public int UniqueCrashes => Crashes.Count + CrashesDropped;

    /// <summary>The campaign seed. PRIVATE.</summary>
    public ulong Seed { get; }

    /// <summary>Copies of the admitted seeds. PRIVATE, like <see cref="CrashRecord.MinimalInput"/>.</summary>
    public IReadOnlyList<byte[]> GetCorpus() => _corpus.Select(c => (byte[])c.Clone()).ToArray();

    /// <summary>Counts, never seeds or bytes.</summary>
    public override string ToString()
    {
        return "FuzzReport{iterations=" + Iterations + " admitted=" + AdmittedSeeds + " edges=" + TotalEdges
            + " crashes=" + Crashes.Count + " timeouts=" + Timeouts + " ooms=" + Ooms
            + " dropped=" + CrashesDropped + " admissionsDropped=" + AdmissionsDropped
            + " tier=" + CoverageTier + " clean=" + IsClean + "}";
    }
}
