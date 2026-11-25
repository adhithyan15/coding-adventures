using System.Numerics;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Pbt;

/// <summary>
/// Integrated shrinking (VER05g section 1, VER05e section 4, VER05 section 4).
/// </summary>
/// <remarks>
/// A property that fails on <c>[913, -4, 77, 0, 2]</c> has told you nothing; one that
/// fails on <c>[0]</c> has told you what is wrong. Shrinking walks from the first
/// failing candidate toward the smallest one that still fails. It is "integrated"
/// because the generator that made the value knows its own bounds, so a shrunk
/// <c>Integer(5, 10)</c> never becomes 0.
/// <para>
/// All public shrinkers take a predicate that returns <c>true</c> when the property
/// <em>holds</em> (the corpus convention) and return the minimal value together with
/// the number of <em>accepted</em> moves.
/// </para>
/// </remarks>
public static class Shrink
{
    private static bool InBounds(BigInteger v, BigInteger lo, BigInteger hi)
    {
        return lo <= v && v <= hi;
    }

    /// <summary>
    /// Bisects then unit-steps <paramref name="current"/> toward <paramref name="target"/>.
    /// </summary>
    /// <remarks>
    /// Each turn tries, in order,
    /// <list type="number">
    /// <item>the bisection candidate <c>current - floor(diff/2)</c>, diff = current - target, when |diff| &gt; 1;</item>
    /// <item>the unit candidate <c>current -/+ 1</c> toward the target.</item>
    /// </list>
    /// A candidate is accepted only if it is inside [lo, hi], differs from current and
    /// still falsifies. Only accepted moves count as steps. No acceptable candidate
    /// ends the walk. Every turn makes at most two property evaluations and either
    /// accepts a step or ends, so the walk is bounded by <c>2 * budget + 1</c>
    /// evaluations.
    /// <para>
    /// Worked example (corpus pbt/shrink_int/54321_le_100, property x &lt;= 100):
    /// 54321 -&gt; 27161 -&gt; 13581 -&gt; ... -&gt; 101, 15 accepted moves. 101 is the
    /// smallest value that still fails; 100 would satisfy the property, so neither
    /// the bisection nor the unit candidate from 101 is accepted.
    /// </para>
    /// </remarks>
    internal static Shrunk<BigInteger> IntWalk(
        BigInteger current,
        BigInteger target,
        BigInteger lo,
        BigInteger hi,
        Func<BigInteger, bool> falsifies,
        int budget)
    {
        int steps = 0;
        while (steps < budget && current != target)
        {
            BigInteger diff = current - target;
            bool moved = false;
            if (BigInteger.Abs(diff) > BigInteger.One)
            {
                BigInteger cand = Hooks.BisectCandidate(current, diff);
                if (cand != current && InBounds(cand, lo, hi) && falsifies(cand))
                {
                    current = cand;
                    steps++;
                    moved = true;
                }
            }

            if (!moved)
            {
                BigInteger cand = Hooks.UnitCandidate(current, diff);
                if (cand != current && InBounds(cand, lo, hi) && falsifies(cand))
                {
                    current = cand;
                    steps++;
                    moved = true;
                }
            }

            if (!moved)
            {
                break;
            }
        }

        return new Shrunk<BigInteger>(current, steps);
    }

    /// <summary>The target probe followed by the walk (VER05 section 4.2).</summary>
    internal static Shrunk<BigInteger> ShrinkIntCore(
        BigInteger initial,
        BigInteger target,
        BigInteger lo,
        BigInteger hi,
        Func<BigInteger, bool> falsifies,
        int maxSteps)
    {
        if (maxSteps == 0)
        {
            return new Shrunk<BigInteger>(initial, 0); // no budget, no shrinking
        }

        if (InBounds(target, lo, hi) && target != initial && falsifies(target))
        {
            return new Shrunk<BigInteger>(target, 1); // the target itself fails: cannot do better
        }

        return IntWalk(initial, target, lo, hi, falsifies, maxSteps);
    }

    private static readonly BigInteger MaxByte = new(255);

    /// <summary>A byte value (held as an int 0..255) shrunk toward 0 by the integer walk.</summary>
    internal static readonly Shrinker<int> ByteWalk = (value, falsifies, budget) =>
    {
        Shrunk<BigInteger> r = IntWalk(
            new BigInteger(value),
            BigInteger.Zero,
            BigInteger.Zero,
            MaxByte,
            b => falsifies((int)b),
            budget);
        return new Shrunk<int>((int)r.Value, r.Steps);
    };

    private static List<E> Copy<E>(List<E> source)
    {
        return new List<E>(source);
    }

    /// <summary>
    /// Shrinks a falsifying sequence (VER05 section 4.3). <paramref name="falsifies"/>
    /// always receives a fresh list (a new <c>List</c>, which it may keep or mutate), but the
    /// ELEMENTS are not cloned: elements that are mutable reference types are shared with the
    /// caller's list, so a property must not mutate them.
    /// </summary>
    /// <remarks>
    /// <list type="number">
    /// <item><b>probe.</b> minLen == 0: does the empty list fail? Then it is the answer, one step. (minLen &gt; 0: probe the length-minLen prefix and carry on from it.)</item>
    /// <item><b>bisection:</b> keep the left half, else the right half, while one of them still fails.</item>
    /// <item><b>single deletion,</b> right to left.</item>
    /// <item><b>element-wise,</b> left to right, via <paramref name="shrinkElem"/> (null means none).</item>
    /// </list>
    /// Worked example (corpus list_sum_le_20, initial [10 5 8 4]): the empty list
    /// satisfies the property; both halves ([10 5], [8 4]) satisfy it too; deleting
    /// index 3 leaves [10 5 8] (sum 23, still failing); element-wise 10 -&gt; 8 gives
    /// the 1-minimal [8 5 8].
    /// <para>
    /// Every property evaluation, accepted or rejected, is charged against one budget
    /// of <c>16 * maxSteps + 64</c> (VER05e section 4): a rejected attempt copies the
    /// list but is not a step, so without this an n-element list costs O(n^2) work
    /// whatever <c>maxSteps</c> says. When it is spent nothing further is accepted.
    /// </para>
    /// </remarks>
    internal static Shrunk<List<E>> ShrinkSequence<E>(
        List<E> initial,
        Func<List<E>, bool> falsifies,
        int maxSteps,
        int minLen,
        Shrinker<E>? shrinkElem)
    {
        if (maxSteps == 0)
        {
            return new Shrunk<List<E>>(initial, 0); // no shrinking, no copy: may alias the input
        }

        // One budget for the whole shrink, shared with every nested shrinker (a list of lists, a
        // tuple of lists): attempts AND elements copied. The initial copy is charged too; once the
        // budget is spent the input is returned as is, without copying (it may alias `initial`).
        using ShrinkBudget budget = ShrinkBudget.Enter(maxSteps);
        if (!budget.ChargeCopy(initial.Count))
        {
            return new Shrunk<List<E>>(initial, 0);
        }

        List<E> current = Copy(initial);
        int steps = 0;

        // Each candidate is built fresh and handed straight to the property (no second copy).
        // The property may scribble on it, so an ACCEPTED move is applied to `current` in place
        // from the known move, never by reusing the candidate.
        if (current.Count > minLen)
        {
            if (budget.Charge(minLen))
            {
                if (falsifies(current.GetRange(0, minLen)))
                {
                    if (minLen == 0)
                    {
                        return new Shrunk<List<E>>(new List<E>(), 1);
                    }

                    current.RemoveRange(minLen, current.Count - minLen);
                    steps = 1;
                }
            }
        }

        int floor = Math.Max(1, minLen);

        // Phase 1: bisection (left half first, then right half).
        while (current.Count > floor && steps < maxSteps)
        {
            int mid = current.Count / 2;
            if (mid >= minLen && budget.Charge(mid) && falsifies(current.GetRange(0, mid)))
            {
                current.RemoveRange(mid, current.Count - mid);
                steps++;
                continue;
            }

            int rightLen = current.Count - mid;
            if (rightLen >= minLen && budget.Charge(rightLen) && falsifies(current.GetRange(mid, rightLen)))
            {
                current.RemoveRange(0, mid);
                steps++;
                continue;
            }

            break;
        }

        // Phase 2: single-element deletion, right to left.
        if (!Hooks.SkipDeletionPhase)
        {
            int i = current.Count - 1;
            while (i >= 0 && current.Count > floor && steps < maxSteps && budget.HasRoom)
            {
                // Charge BEFORE copying: a spent budget must not cost another copy.
                if (!budget.Charge(current.Count - 1))
                {
                    break;
                }

                List<E> cand = Copy(current);
                cand.RemoveAt(i);
                if (falsifies(cand))
                {
                    current.RemoveAt(i);
                    steps++;
                    i = Math.Min(i, current.Count - 1);
                }
                else
                {
                    i--;
                }
            }
        }

        // Phase 3: element-wise, left to right.
        if (shrinkElem != null)
        {
            int size = current.Count;
            for (int index = 0; index < size; index++)
            {
                int remaining = maxSteps - steps;
                if (remaining <= 0 || !budget.HasRoom)
                {
                    break;
                }

                int at = index;
                bool WithElement(E candidate)
                {
                    if (!budget.Charge(current.Count))
                    {
                        return false;
                    }

                    List<E> probe = Copy(current);
                    probe[at] = candidate;
                    return falsifies(probe);
                }

                Shrunk<E> r = shrinkElem(current[index], WithElement, remaining);
                if (r.Steps > 0)
                {
                    current[index] = r.Value;
                    steps += r.Steps;
                }
            }
        }

        return new Shrunk<List<E>>(current, steps);
    }

    internal static void CheckBudget(int maxSteps)
    {
        if (maxSteps < 0 || maxSteps > Limits.MaxShrinkSteps)
        {
            throw new ArgumentOutOfRangeException(
                nameof(maxSteps), "maxSteps " + maxSteps + " outside [0, " + Limits.MaxShrinkSteps + "]");
        }
    }

    /// <summary>
    /// Shrinks a falsifying integer toward <paramref name="target"/> inside [lo, hi]
    /// (the corpus calls these min and max).
    /// </summary>
    /// <remarks>
    /// <c>prop(x)</c> is true when the property holds. Steps counts accepted moves only
    /// and the target probe is one move. Throws <see cref="NotFalsifyingException"/>
    /// when <paramref name="initial"/> satisfies <paramref name="prop"/>, and
    /// <see cref="ArgumentException"/> for a null argument, lo &gt; hi, an initial
    /// outside the bounds, or a budget outside [0, MaxShrinkSteps]. The result is
    /// never outside [lo, hi].
    /// </remarks>
    public static Shrunk<BigInteger> ShrinkInt(
        BigInteger initial,
        BigInteger target,
        BigInteger lo,
        BigInteger hi,
        Func<BigInteger, bool> prop,
        int maxSteps)
    {
        ArgumentNullException.ThrowIfNull(prop);
        CheckBudget(maxSteps);
        if (lo > hi)
        {
            throw new ArgumentException("min exceeds max");
        }

        if (!InBounds(initial, lo, hi))
        {
            throw new ArgumentException("initial value must lie within [min, max]");
        }

        using ShrinkBudget budget = ShrinkBudget.EnterChild(maxSteps);
        if (!budget.Charge(1))
        {
            return new Shrunk<BigInteger>(initial, 0); // starved by an enclosing shrink
        }

        if (prop(initial))
        {
            throw new NotFalsifyingException();
        }

        return ShrinkIntCore(initial, target, lo, hi, v => budget.Charge(1) && !prop(v), maxSteps);
    }

    /// <summary>
    /// Shrinks a falsifying list of integers: empty probe, halves, single deletion,
    /// then each element toward <paramref name="elemTarget"/> (unbounded).
    /// <paramref name="prop"/> is true when the list satisfies the property. The
    /// property receives fresh lists, so one that scribbles on its argument cannot corrupt
    /// the shrinker's state. When the shared budget is already spent the returned list may
    /// alias <paramref name="initial"/>.
    /// </summary>
    public static Shrunk<List<BigInteger>> ShrinkList(
        List<BigInteger> initial,
        BigInteger elemTarget,
        Func<List<BigInteger>, bool> prop,
        int maxSteps)
    {
        ArgumentNullException.ThrowIfNull(initial);
        ArgumentNullException.ThrowIfNull(prop);
        CheckBudget(maxSteps);

        // Public entry point: a child scope, charged before the first copies (the working copy
        // and the copy the first property call receives) and for the initial property evaluation.
        using ShrinkBudget budget = ShrinkBudget.EnterChild(maxSteps);
        if (!budget.ChargeCopy(2L * initial.Count) || !budget.Charge(1))
        {
            return new Shrunk<List<BigInteger>>(initial, 0);
        }

        List<BigInteger> items = Copy(initial);
        if (prop(Copy(items)))
        {
            throw new NotFalsifyingException();
        }

        Shrinker<BigInteger> walk = (value, falsifies, steps) =>
            IntWalk(value, elemTarget, UnboundedLow, UnboundedHigh, falsifies, steps);

        // A candidate is already a fresh list, so the property may scribble on it.
        return ShrinkSequence(items, c => !prop(c), maxSteps, 0, walk);
    }

    // "Unbounded" for the walk: wide enough that no candidate is ever out of range.
    // (Fixed, not derived from the input, so it does not allocate per call.)
    private static readonly BigInteger UnboundedLow = -(BigInteger.One << 4096);
    private static readonly BigInteger UnboundedHigh = BigInteger.One << 4096;

    /// <summary>
    /// Shrinks falsifying bytes: sequence shrinking over byte values toward 0x00.
    /// <paramref name="prop"/> is true when the bytes satisfy the property.
    /// </summary>
    public static Shrunk<byte[]> ShrinkBytes(byte[] initial, Func<byte[], bool> prop, int maxSteps)
    {
        ArgumentNullException.ThrowIfNull(initial);
        ArgumentNullException.ThrowIfNull(prop);
        CheckBudget(maxSteps);
        using ShrinkBudget budget = ShrinkBudget.EnterChild(maxSteps);

        // The clone, the list conversion and the array conversion back.
        if (!budget.ChargeCopy(3L * initial.Length) || !budget.Charge(1))
        {
            return new Shrunk<byte[]>(initial, 0);
        }

        if (prop((byte[])initial.Clone()))
        {
            throw new NotFalsifyingException();
        }

        // Each attempt also converts its candidate back to an array: charge that copy too.
        Shrunk<List<int>> r = ShrinkSequence(
            ToList(initial), c => budget.ChargeCopy(c.Count) && !prop(ToArray(c)), maxSteps, 0, ByteWalk);
        return new Shrunk<byte[]>(ToArray(r.Value), r.Steps);
    }

    internal static List<int> ToList(byte[] input)
    {
        List<int> output = new(input.Length);
        foreach (byte b in input)
        {
            output.Add(b);
        }

        return output;
    }

    internal static byte[] ToArray(List<int> input)
    {
        byte[] output = new byte[input.Count];
        for (int i = 0; i < output.Length; i++)
        {
            output[i] = (byte)input[i];
        }

        return output;
    }

    /// <summary>The first failing value and its shrunk value from <see cref="SimulateReplay"/>.</summary>
    public readonly record struct Replay(BigInteger InitialFail, BigInteger Minimal);

    /// <summary>
    /// Replays a failure from nothing but <paramref name="seed"/>.
    /// </summary>
    /// <remarks>
    /// It draws up to 1000 values <c>Bounded(1000)</c>; the first that falsifies
    /// <paramref name="prop"/> is the initial failure; that value is then shrunk toward 0
    /// over [0, 1000]. The same seed gives the same pair forever. <paramref name="prop"/>
    /// is true when the value is fine. Throws <see cref="ArgumentException"/> if no draw
    /// falsified <paramref name="prop"/>.
    /// </remarks>
    public static Replay SimulateReplay(ulong seed, Func<BigInteger, bool> prop)
    {
        ArgumentNullException.ThrowIfNull(prop);
        using ShrinkBudget budget = ShrinkBudget.EnterChild(1000);
        SplitMix64 rng = Hooks.NewRng(seed);
        BigInteger? initialFail = null;
        for (int i = 0; i < 1000 && budget.Charge(1); i++)
        {
            BigInteger v = rng.Bounded(1000);
            if (!prop(v))
            {
                initialFail = v;
                break;
            }
        }

        if (initialFail is not BigInteger first)
        {
            throw new ArgumentException("property was not falsified within 1000 draws");
        }

        Shrunk<BigInteger> r = ShrinkInt(first, BigInteger.Zero, BigInteger.Zero, 1000, prop, 1000);
        return new Replay(first, r.Value);
    }
}
