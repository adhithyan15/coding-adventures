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

    /// <summary>Shrinks <paramref name="value"/> while <paramref name="falsifies"/> holds; <c>(value, 0)</c> if there is no shrinker.</summary>
    public Shrunk<T> Shrink(T value, Func<T, bool> falsifies, int maxSteps)
    {
        RuntimeHelpers.EnsureSufficientExecutionStack();
        if (shrinker == null)
        {
            return new Shrunk<T>(value, 0);
        }

        // Join (or open) the one budget of this whole shrink, so nested generators share it.
        using ShrinkBudget budget = ShrinkBudget.Enter(maxSteps);
        return shrinker(value, falsifies, maxSteps);
    }
}
