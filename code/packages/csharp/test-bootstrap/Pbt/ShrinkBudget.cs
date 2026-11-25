namespace CodingAdventures.TestBootstrap.Pbt;

/// <summary>
/// The one budget of one whole shrink (VER05g, security review round 1).
/// </summary>
/// <remarks>
/// A shrink of one falsified case may involve many shrinkers: a tuple's components, a list
/// of lists, a custom shrinker. If each kept its own budget the work would multiply, so the
/// budget is ambient: the outermost entry point (<see cref="Gen{T}.Shrink"/>, the public
/// <see cref="Shrink"/> functions, the runner) opens a scope, and every nested shrinker on
/// the same thread charges the same object. Two things are charged:
/// <list type="bullet">
/// <item><b>attempts</b>, one per property evaluation, up to <c>16 * maxSteps + 64</c>;</item>
/// <item><b>work</b>, the number of list elements copied for candidates, up to
/// <see cref="Limits.MaxShrinkWork"/>.</item>
/// </list>
/// Once either is spent every further <see cref="Charge"/> is refused, so no shrinker accepts
/// anything more and every loop ends; the best value found so far is kept.
/// </remarks>
internal sealed class ShrinkBudget : IDisposable
{
    [ThreadStatic]
    private static ShrinkBudget? current;

    private readonly bool owner;
    private readonly long maxAttempts;
    private readonly long maxWork;

    private ShrinkBudget(int maxSteps, long maxWork, bool owner)
    {
        maxAttempts = Limits.AttemptBudget(maxSteps);
        this.maxWork = maxWork;
        this.owner = owner;
    }

    /// <summary>Work spent by the most recent outermost shrink on this thread (for tests).</summary>
    [ThreadStatic]
    internal static long LastWork;

    /// <summary>Attempts spent by the most recent outermost shrink on this thread (for tests).</summary>
    [ThreadStatic]
    internal static long LastAttempts;

    /// <summary>Attempts charged so far.</summary>
    internal long Attempts { get; private set; }

    /// <summary>Elements charged so far.</summary>
    internal long Work { get; private set; }

    /// <summary>The ambient budget of this thread, or null outside any shrink.</summary>
    internal static ShrinkBudget? Current => current;

    /// <summary>
    /// Joins the ambient budget, or opens a new one sized for <paramref name="maxSteps"/>.
    /// Dispose the result (a nested join disposes to nothing).
    /// </summary>
    internal static ShrinkBudget Enter(int maxSteps, long maxWork = Limits.MaxShrinkWork)
    {
        if (current != null)
        {
            return new ShrinkBudget(0, 0, false) { Delegate = current };
        }

        ShrinkBudget fresh = new(maxSteps, maxWork, true);
        current = fresh;
        return fresh;
    }

    private ShrinkBudget? Delegate { get; init; }

    /// <summary>
    /// Charges one attempt over a candidate of <paramref name="elements"/> elements (at
    /// least 1). Returns false, charging nothing, when either budget cannot afford it:
    /// callers must check this BEFORE building the candidate.
    /// </summary>
    internal bool Charge(long elements)
    {
        if (Delegate != null)
        {
            return Delegate.Charge(elements);
        }

        long cost = Math.Max(1, elements);
        if (Attempts >= maxAttempts || Work + cost > maxWork)
        {
            return false;
        }

        Attempts++;
        Work += cost;
        return true;
    }

    /// <summary>True while at least one more attempt of one element could be afforded.</summary>
    internal bool HasRoom => Delegate != null ? Delegate.HasRoom : Attempts < maxAttempts && Work < maxWork;

    /// <inheritdoc/>
    public void Dispose()
    {
        if (owner)
        {
            LastWork = Work;
            LastAttempts = Attempts;
            current = null;
        }
    }
}
