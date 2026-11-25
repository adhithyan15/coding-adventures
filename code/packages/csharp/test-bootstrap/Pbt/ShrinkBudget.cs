namespace CodingAdventures.TestBootstrap.Pbt;

/// <summary>
/// The one budget of one whole shrink (VER05g section 3.1, security review rounds 1 and 2).
/// </summary>
/// <remarks>
/// A shrink of one falsified case may involve many shrinkers: a tuple's components, a list of
/// lists, a custom shrinker. If each kept its own budget the work would multiply, so the budget
/// is ambient (<see cref="AsyncLocal{T}"/>, so it also flows into child tasks and async
/// continuations): the outermost entry point opens a scope, and every nested shrinker joins it.
/// Two things are charged, thread-safely:
/// <list type="bullet">
/// <item><b>attempts</b>, one per property evaluation, up to <c>16 * maxSteps + 64</c>;</item>
/// <item><b>work</b>, the number of list elements copied, up to <see cref="Limits.MaxShrinkWork"/>.</item>
/// </list>
/// Once either is spent every further charge is refused, so nothing more is accepted and every
/// loop ends; the best value found so far is kept.
/// <para>
/// Public top-level entry points (<c>Runner.Check</c>, <c>Gen.Shrink</c>, <c>Shrink.ShrinkList</c>,
/// <c>Shrink.ShrinkBytes</c>) open an <em>isolated</em> scope: a property that itself runs a
/// verdict or a shrink gets its own fresh budget instead of eating (and being starved by) the
/// outer one, and the outer budget is restored when the inner scope ends. Combinators that shrink
/// their components use <c>Gen.ShrinkNested</c>, which joins.
/// </para>
/// </remarks>
internal sealed class ShrinkBudget : IDisposable
{
    private static readonly AsyncLocal<ShrinkBudget?> Ambient = new();

    private readonly object sync = new();
    private readonly ShrinkBudget? joined;     // non-null for a joining scope: charges go there
    private readonly ShrinkBudget? previous;   // ambient value to restore when an owner ends
    private readonly long maxAttempts;
    private readonly long maxWork;
    private long attempts;
    private long work;

    private ShrinkBudget(int maxSteps, long maxWork, ShrinkBudget? joined, ShrinkBudget? previous)
    {
        maxAttempts = Limits.AttemptBudget(maxSteps);
        this.maxWork = maxWork;
        this.joined = joined;
        this.previous = previous;
    }

    /// <summary>Overridable by tests (see <c>Hooks.Reset</c>): the work allowance of a new budget.</summary>
    internal static long DefaultMaxWork = Limits.MaxShrinkWork;

    /// <summary>Work spent by the most recent owning scope that ended (for tests).</summary>
    internal static long LastWork;

    /// <summary>Attempts spent by the most recent owning scope that ended (for tests).</summary>
    internal static long LastAttempts;

    /// <summary>The ambient budget of this execution context, or null outside any shrink.</summary>
    internal static ShrinkBudget? Current => Ambient.Value;

    /// <summary>Attempts charged so far.</summary>
    internal long Attempts => Root.attempts;

    /// <summary>Elements charged so far.</summary>
    internal long Work => Root.work;

    private ShrinkBudget Root => joined ?? this;

    /// <summary>
    /// Opens a scope. With <paramref name="isolated"/> false and an ambient budget present it
    /// joins that budget; otherwise it opens a fresh one sized for <paramref name="maxSteps"/>,
    /// remembering the ambient one to restore. Dispose the result.
    /// </summary>
    internal static ShrinkBudget Enter(int maxSteps, bool isolated = false, long? maxWork = null)
    {
        ShrinkBudget? ambient = Ambient.Value;
        if (!isolated && ambient != null)
        {
            return new ShrinkBudget(0, 0, ambient.Root, null);
        }

        ShrinkBudget fresh = new(maxSteps, maxWork ?? DefaultMaxWork, null, ambient);
        Ambient.Value = fresh;
        return fresh;
    }

    /// <summary>
    /// Charges one attempt over a candidate of <paramref name="elements"/> elements (at least 1).
    /// Returns false, charging nothing, when either budget cannot afford it: callers must check
    /// this BEFORE building the candidate.
    /// </summary>
    internal bool Charge(long elements) => Root.Take(elements, 1);

    /// <summary>
    /// Charges the copy of <paramref name="elements"/> elements that is not itself a property
    /// evaluation (an initial copy, a list-to-array conversion). Refused once spent.
    /// </summary>
    internal bool ChargeCopy(long elements) => Root.Take(elements, 0);

    private bool Take(long elements, int attemptCost)
    {
        long cost = Math.Max(1, elements);
        lock (sync)
        {
            if (attempts >= maxAttempts || work + cost > maxWork)
            {
                return false;
            }

            attempts += attemptCost;
            work += cost;
            return true;
        }
    }

    /// <summary>True while at least one more attempt of one element could be afforded.</summary>
    internal bool HasRoom
    {
        get
        {
            ShrinkBudget r = Root;
            lock (r.sync)
            {
                return r.attempts < r.maxAttempts && r.work < r.maxWork;
            }
        }
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (joined == null)
        {
            LastWork = work;
            LastAttempts = attempts;
            Ambient.Value = previous;
        }
    }
}
