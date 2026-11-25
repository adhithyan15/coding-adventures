using System.Runtime.CompilerServices;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Pbt;

/// <summary>
/// A generator of values of type <typeparamref name="T"/>: it draws from a PRNG
/// stream and knows how to shrink what it drew (VER05g section 1, VER05e section 3).
/// </summary>
/// <remarks>
/// Sampling throws <see cref="ExhaustedException"/> because generation can fail on
/// data (a filter that never accepts); every combinator propagates it. The shrinker
/// is optional: <c>null</c> means "values are reported unshrunk".
/// <para>
/// Both <see cref="Sample"/> and <see cref="Shrink"/> call
/// <see cref="RuntimeHelpers.EnsureSufficientExecutionStack"/>. .NET cannot catch a
/// stack overflow, so runaway generator nesting (a recursive generator with no base
/// case, a 100,000-deep <c>Map</c> chain) is turned into a catchable
/// <see cref="InsufficientExecutionStackException"/> that the runner reports as
/// <c>property-exhausted</c>.
/// </para>
/// </remarks>
public sealed class Gen<T>
{
    private readonly Func<SplitMix64, T> sampler;
    private readonly Shrinker<T>? shrinker;

    private Gen(Func<SplitMix64, T> sampler, Shrinker<T>? shrinker)
    {
        this.sampler = sampler;
        this.shrinker = shrinker;
    }

    /// <summary>Builds a generator from a sampler and an optional shrinker.</summary>
    public static Gen<T> Of(Func<SplitMix64, T> sampler, Shrinker<T>? shrinker)
    {
        ArgumentNullException.ThrowIfNull(sampler);
        return new Gen<T>(sampler, shrinker);
    }

    /// <summary>Draws one value.</summary>
    public T Sample(SplitMix64 rng)
    {
        RuntimeHelpers.EnsureSufficientExecutionStack();
        return sampler(rng);
    }

    /// <summary>
    /// Shrinks <paramref name="value"/> while <paramref name="falsifies"/> holds; <c>(value, 0)</c>
    /// if there is no shrinker. A public entry point (VER05g section 3.1): it opens a child of the
    /// ambient shrink budget (a fresh root outside any shrink) with its own local cap, and charges
    /// every evaluation of <paramref name="falsifies"/>, so however a property or a custom
    /// <see cref="Shrinker{T}"/> nests calls, the total work stays inside the outermost budget.
    /// </summary>
    public Shrunk<T> Shrink(T value, Func<T, bool> falsifies, int maxSteps)
    {
        CodingAdventures.TestBootstrap.Pbt.Shrink.CheckBudget(maxSteps);
        using ShrinkBudget budget = ShrinkBudget.EnterChild(maxSteps);
        return Run(value, budget.Charged(falsifies), maxSteps);
    }

    /// <summary>
    /// For combinators shrinking their components: joins the enclosing shrink budget instead of
    /// opening a new one (a fresh root only when none is open). INTERNAL on purpose: it is not a
    /// safe entry point for user code. As defence in depth it still charges every evaluation of
    /// <paramref name="falsifies"/> to the budget it joined (over-charging a combinator that also
    /// charges is harmless), so even a leaf shrinker that charges nothing is bounded.
    /// </summary>
    internal Shrunk<T> ShrinkNested(T value, Func<T, bool> falsifies, int maxSteps)
    {
        CodingAdventures.TestBootstrap.Pbt.Shrink.CheckBudget(maxSteps);
        using ShrinkBudget budget = ShrinkBudget.Enter(maxSteps);
        return Run(value, budget.Charged(falsifies), maxSteps);
    }

    private Shrunk<T> Run(T value, Func<T, bool> falsifies, int maxSteps)
    {
        RuntimeHelpers.EnsureSufficientExecutionStack();
        if (shrinker == null)
        {
            return new Shrunk<T>(value, 0);
        }

        return shrinker(value, falsifies, maxSteps);
    }
}
