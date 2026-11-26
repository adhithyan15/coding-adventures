namespace CodingAdventures.TestBootstrap.Fuzz;

/// <summary>
/// Bounds a campaign; every field is validated against its ceiling (CWE-400: nothing a caller
/// controls is unbounded). Use <c>with</c> to derive variants.
/// </summary>
/// <remarks>
/// <c>MinimizeBudget</c> is the most target executions spent minimising ONE novel crash. On top of
/// it the whole campaign spends at most <c>MaxIterations</c> minimisation executions in total, so
/// minimisation can at most double a campaign's work however many distinct crashes it finds.
/// </remarks>
/// <param name="MaxIterations">mutation executions, in [0, <see cref="Mutate.MaxIterations"/>]</param>
/// <param name="Seed">the campaign seed</param>
/// <param name="MaxInputLen">the longest mutated candidate, in [0, <see cref="Mutate.MaxInputBytes"/>]</param>
/// <param name="BaseEnergy">the energy base, in [0, <see cref="Mutate.MaxIterations"/>]</param>
/// <param name="MinimizeBudget">executions per crash minimisation, in [0, <see cref="MaxMinimizeBudget"/>]</param>
/// <param name="StepBudget"><c>Tick()</c> calls per execution, in [0, <see cref="MaxStepBudget"/>]</param>
/// <param name="MemBudget"><c>Reserve()</c> bytes per execution, in [0, <see cref="MaxMemBudget"/>]</param>
public sealed record FuzzOptions(
    int MaxIterations,
    ulong Seed,
    int MaxInputLen,
    int BaseEnergy,
    int MinimizeBudget,
    int StepBudget,
    int MemBudget)
{
    /// <summary>The limits only: the seed is deliberately not printed (VER06 section 6).</summary>
    public override string ToString()
    {
        return "FuzzOptions{maxIterations=" + MaxIterations + " maxInputLen=" + MaxInputLen + " baseEnergy=" + BaseEnergy
            + " minimizeBudget=" + MinimizeBudget + " stepBudget=" + StepBudget + " memBudget=" + MemBudget + "}";
    }

    /// <summary>Ceiling of the step budget a target can observe.</summary>
    public const int MaxStepBudget = 100_000_000;

    /// <summary>Ceiling of the memory budget a target can observe (1 GiB).</summary>
    public const int MaxMemBudget = 1 << 30;

    /// <summary>Ceiling of the executions spent minimising one crash.</summary>
    public const int MaxMinimizeBudget = 100_000;

    /// <summary>Default per-crash minimisation budget.</summary>
    public const int DefaultMinimizeBudget = 2000;

    /// <summary>Default step budget.</summary>
    public const int DefaultStepBudget = 200_000;

    /// <summary>Default memory budget (64 MiB).</summary>
    public const int DefaultMemBudget = 64 << 20;

    /// <summary>The standard campaign: 1000 iterations, seed 42, inputs up to 4096 bytes.</summary>
    public static FuzzOptions Defaults() =>
        new(1000, 42, 4096, 10, DefaultMinimizeBudget, DefaultStepBudget, DefaultMemBudget);

    /// <summary>Checks every field against its range; throws <see cref="ArgumentException"/>.</summary>
    public void Validate()
    {
        Check(nameof(MaxIterations), MaxIterations, Mutate.MaxIterations);
        Check(nameof(MaxInputLen), MaxInputLen, Mutate.MaxInputBytes);
        Check(nameof(BaseEnergy), BaseEnergy, Mutate.MaxIterations);
        Check(nameof(MinimizeBudget), MinimizeBudget, MaxMinimizeBudget);
        Check(nameof(StepBudget), StepBudget, MaxStepBudget);
        Check(nameof(MemBudget), MemBudget, MaxMemBudget);
    }

    private static void Check(string name, int v, int hi)
    {
        if (v < 0 || v > hi)
        {
            throw new ArgumentException(name + " must be in [0, " + hi + "]");
        }
    }
}
