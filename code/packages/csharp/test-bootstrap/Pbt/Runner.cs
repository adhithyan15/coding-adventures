using System.Globalization;
using CodingAdventures.TestBootstrap.CorpusJson;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Pbt;

/// <summary>
/// The honest runner (VER05 section 7.1, VER05e section 5).
/// </summary>
/// <remarks>
/// Verdict honesty:
/// <list type="bullet">
/// <item>zero cases evaluated is <c>property-exhausted</c>, never a pass;</item>
/// <item>a generator exception is <c>property-exhausted</c>;</item>
/// <item>a property exception is <c>property-panic</c>;</item>
/// <item>the reported minimal witness is re-verified: if the shrinker throws, returns
/// something that no longer fails, a value the property throws on, or a nonsensical
/// step count, the original candidate is reported with 0 steps;</item>
/// <item>redacted mode never emits the witness or the seed;</item>
/// <item>the call always returns a verdict (invalid options aside, which throw instead
/// of producing something that could be mistaken for a pass).</item>
/// </list>
/// There is no wall-clock timeout: <c>property-timeout</c> is registered and never
/// produced, so a property that loops forever hangs the run.
/// </remarks>
public static class Runner
{
    /// <summary>Runs <paramref name="prop"/> over <c>options.NumTests</c> candidates from <paramref name="seed"/>.</summary>
    public static Verdict Check<T>(ulong seed, Gen<T> gen, Func<T, bool> prop, Options? options)
    {
        return Check(seed, gen, prop, options, null);
    }

    /// <summary>
    /// <see cref="Check{T}(ulong, Gen{T}, Func{T, bool}, Options?)"/> with an explicit
    /// shrinker that overrides the generator's own; it exists so a defective shrinker can
    /// be tested. <paramref name="prop"/> returns true when the property holds. Invalid
    /// options throw <see cref="ArgumentException"/> (no verdict is produced).
    /// </summary>
    public static Verdict Check<T>(ulong seed, Gen<T> gen, Func<T, bool> prop, Options? options, Shrinker<T>? shrink)
    {
        Options o = options ?? Options.Defaults();
        ArgumentNullException.ThrowIfNull(prop);
        ArgumentNullException.ThrowIfNull(gen);
        if (o.NumTests < 0 || o.NumTests > Limits.MaxCases)
        {
            throw new ArgumentException("NumTests " + o.NumTests + " outside [0, " + Limits.MaxCases + "]");
        }

        if (o.MaxShrinkSteps < 0 || o.MaxShrinkSteps > Limits.MaxShrinkSteps)
        {
            throw new ArgumentException(
                "MaxShrinkSteps " + o.MaxShrinkSteps + " outside [0, " + Limits.MaxShrinkSteps + "]");
        }

        bool redact = o.Ci != CiMode.Off;

        if (o.NumTests == 0)
        {
            return new Verdict(
                false,
                FailureIds.Exhausted,
                "no cases were evaluated (NumTests is 0); a property that never ran is not a pass",
                0,
                0,
                0,
                null,
                null,
                null,
                null);
        }

        // A check nested in a property (an ambient scope exists) pays one attempt at entry, one per
        // case after the first and the size of every sampled collection as work, all charged up
        // the chain: so neither nesting depth, branching nor numTests multiplies the outermost
        // budget. A starved nested check reports property-exhausted without evaluating anything
        // (never a pass). A top-level check has no ambient scope and always evaluates its first case.
        ShrinkBudget? outer = ShrinkBudget.Current;
        bool nested = outer != null;
        if (nested && !outer!.Charge(1))
        {
            return Spent(0);
        }

        // The WHOLE check runs in one scope (case loop included): a root with no enclosing budget,
        // a child otherwise, so a nested check during generation is charged to it too.
        using ShrinkBudget scope = ShrinkBudget.EnterCheck(o.NumTests, o.MaxShrinkSteps);
        SplitMix64 rng = Hooks.NewRng(seed);
        for (int iteration = 1; iteration <= o.NumTests; iteration++)
        {
            if (nested && iteration > 1 && !scope.Charge(1))
            {
                return Spent(iteration);
            }

            T candidate;
            try
            {
                candidate = gen.Sample(rng);
            }
            catch (Exception e) when (Safe.Recoverable(e))
            {
                return Exhausted(e, iteration, redact);
            }

            if (nested && !scope.ChargeCopy(SizeOf(candidate)))
            {
                return Spent(iteration);
            }

            bool holds;
            try
            {
                holds = prop(candidate);
            }
            catch (Exception e) when (Safe.Recoverable(e))
            {
                return Panic(seed, iteration, e, redact);
            }

            if (!holds)
            {
                return Falsified(seed, iteration, candidate, gen, prop, shrink, o, redact, scope);
            }
        }

        if (!nested && scope.WasRefused)
        {
            // A nested check or generator inside this run was starved, so the property was not
            // fully evaluated: never report that as a pass.
            return new Verdict(
                false,
                FailureIds.Exhausted,
                "a nested check was starved by the shrink budget; the property was not fully evaluated",
                o.NumTests,
                o.NumTests,
                0,
                null,
                null,
                null,
                null);
        }

        return new Verdict(true, null, null, o.NumTests, 0, 0, null, null, null, null);
    }

    /// <summary>The enclosing shrink budget was spent: exhausted, never a pass.</summary>
    private static Verdict Spent(int iteration)
    {
        return new Verdict(
            false,
            FailureIds.Exhausted,
            "the enclosing shrink budget was spent at iteration " + iteration,
            Math.Max(0, iteration - 1),
            iteration,
            0,
            null,
            null,
            null,
            null);
    }

    /// <summary>The most elements the work estimate of one sampled value will visit.</summary>
    private const int WalkCap = 10_000;

    /// <summary>
    /// An approximate size of a sampled value in elements (work): a collection or array counts its
    /// length plus, one level deeper, the length of each element that is itself a collection,
    /// array or string (visiting at most <see cref="WalkCap"/> elements); a string counts its
    /// length. Generation work is approximate by design; a hostile collection counts 0.
    /// </summary>
    private static long SizeOf(object? v)
    {
        try
        {
            long n = Shallow(v);
            if (n == 0)
            {
                return 0;
            }

            long total = n;
            if (v is System.Collections.IEnumerable items and not string and not byte[])
            {
                int visited = 0;
                foreach (object? e in items)
                {
                    if (visited++ >= WalkCap)
                    {
                        break;
                    }

                    total += Shallow(e);
                }
            }

            return total;
        }
        catch (Exception e) when (Safe.Recoverable(e))
        {
            return 0; // a hostile collection: the estimate is best effort
        }
    }

    private static long Shallow(object? v)
    {
        switch (v)
        {
            case string text:
                return text.Length;
            case Array array:
                return array.LongLength;
            case System.Collections.ICollection c:
                return c.Count;
            case System.Runtime.CompilerServices.ITuple tuple:
                return tuple.Length;
            case System.Collections.IEnumerable items:
                {
                    // ICollection<T>/IReadOnlyCollection<T>-only types (HashSet<T>, ImmutableArray<T>, ...)
                    // and lazy sequences: count by enumerating, capped (the caller's try/catch guards a hostile one).
                    long n = 0;
                    foreach (object? unused in items)
                    {
                        if (++n >= WalkCap)
                        {
                            break;
                        }
                    }

                    return n;
                }

            default:
                return 0;
        }
    }

    private static Verdict Exhausted(Exception e, int iteration, bool redact)
    {
        string text;
        if (e is ExhaustedException)
        {
            text = "filter precondition rejected all candidates at iteration " + iteration;
        }
        else if (redact)
        {
            text = "generator error at iteration " + iteration + " (" + e.GetType().FullName + ")";
        }
        else
        {
            text = "generator error at iteration " + iteration + ": " + Safe.SafeString(e);
        }

        return new Verdict(false, FailureIds.Exhausted, text, iteration - 1, iteration, 0, null, null, null, null);
    }

    private static Verdict Panic(ulong seed, int iteration, Exception e, bool redact)
    {
        if (redact)
        {
            return new Verdict(
                false,
                FailureIds.Panic,
                "property evaluation threw at iteration " + iteration + " (" + e.GetType().FullName + ")",
                iteration,
                iteration,
                0,
                null,
                null,
                null,
                null);
        }

        return new Verdict(
            false,
            FailureIds.Panic,
            "property evaluation threw at iteration " + iteration + ": " + Safe.SafeString(e),
            iteration,
            iteration,
            0,
            seed,
            null,
            null,
            null);
    }

    private static Verdict Falsified<T>(
        ulong seed,
        int iteration,
        T candidate,
        Gen<T> gen,
        Func<T, bool> prop,
        Shrinker<T>? shrink,
        Options o,
        bool redact,
        ShrinkBudget checkScope)
    {
        // A candidate the property throws on is not a counterexample to the claim; it is
        // a different failure, so it is never accepted as "smaller".
        bool Falsifies(T v)
        {
            try
            {
                return !prop(v);
            }
            catch (Exception e) when (Safe.Recoverable(e))
            {
                return false;
            }
        }

        T minimal = candidate;
        int steps = 0;

        // Untrusted shrinker: fall back to the original. The shrink scope is a child of the check
        // scope (itself a child of any enclosing shrink): every evaluation is charged to the whole
        // chain, so nothing a property or shrinker nests can exceed the outermost budget.
        try
        {
            Shrunk<T> shrunk;
            using (ShrinkBudget scope = ShrinkBudget.EnterChild(o.MaxShrinkSteps))
            {
                Func<T, bool> budgeted = scope.Charged<T>(Falsifies);
                shrunk = shrink != null
                    ? shrink(candidate, budgeted, o.MaxShrinkSteps)
                    : gen.ShrinkNested(candidate, budgeted, o.MaxShrinkSteps);
            }

            // Re-verify: the witness we report must really fail the property. One attempt is
            // charged to the check scope (not the spent shrink scope); if the chain refuses it the
            // shrunk value is not adopted: the original witness is reported with 0 steps.
            if (checkScope.Charge(1)
                && shrunk.Steps >= 0
                && shrunk.Steps <= o.MaxShrinkSteps
                && Falsifies(shrunk.Value))
            {
                minimal = shrunk.Value;
                steps = shrunk.Steps;
            }
        }
        catch (Exception e) when (Safe.Recoverable(e))
        {
            // keep the original
        }

        if (redact)
        {
            string rendered = Canon.Render(minimal); // total: never throws
            string preimage = "iter:" + iteration.ToString(CultureInfo.InvariantCulture)
                + ";steps:" + steps.ToString(CultureInfo.InvariantCulture) + ";shrunk:" + rendered;
            string digest = SplitMix64.Fnv1a64(preimage).ToString("x16", CultureInfo.InvariantCulture);
            return new Verdict(
                false,
                FailureIds.Counterexample,
                "property falsified after " + iteration + " iterations (shrunk in " + steps
                + " steps, repro-fnv: " + digest + ")",
                iteration,
                iteration,
                steps,
                null,
                null,
                null,
                digest);
        }

        return new Verdict(
            false,
            FailureIds.Counterexample,
            "property falsified at iteration " + iteration + " (seed=" + seed.ToString(CultureInfo.InvariantCulture)
            + "): initial=" + Canon.Render(candidate) + ", minimal=" + Canon.Render(minimal)
            + " after " + steps + " shrink steps",
            iteration,
            iteration,
            steps,
            seed,
            candidate,
            minimal,
            null);
    }
}
