using System.Numerics;
using System.Runtime.CompilerServices;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Pbt;

/// <summary>The value of <see cref="Gens.Tuple2{A, B}"/>.</summary>
public sealed record Pair<A, B>(A First, B Second) : ITuple
{
    /// <inheritdoc/>
    public int Length => 2;

    /// <inheritdoc/>
    public object? this[int index] => index switch
    {
        0 => First,
        1 => Second,
        _ => throw new IndexOutOfRangeException(),
    };
}

/// <summary>The value of <see cref="Gens.Tuple3{A, B, C}"/>.</summary>
public sealed record Triple<A, B, C>(A First, B Second, C Third) : ITuple
{
    /// <inheritdoc/>
    public int Length => 3;

    /// <inheritdoc/>
    public object? this[int index] => index switch
    {
        0 => First,
        1 => Second,
        2 => Third,
        _ => throw new IndexOutOfRangeException(),
    };
}

/// <summary>One weighted alternative of <see cref="Gens.Frequency{T}"/>.</summary>
public sealed record Arm<T>(long Weight, Gen<T> Gen);

/// <summary>
/// The generator algebra (VER05 section 3, VER05e section 3).
/// </summary>
/// <remarks>
/// Draw order is the compatibility contract between language lanes: the pbt-v1
/// vectors pin exact outputs per seed, so reordering a draw breaks conformance.
/// Nested generators are length-bounded per level, not in total: <c>ListOf(ListOf(g, 0, n), 0, n)</c>
/// can produce n x n elements, so choose bounds with the product in mind. Shrinking, by contrast,
/// shares one work budget across all levels.
/// Integers are <see cref="BigInteger"/> wherever the corpus or the full 128-bit range
/// is involved; <c>U8</c>, <c>U64</c> and <c>Int64</c> use the native types, which in C#
/// include the unsigned ones.
/// </remarks>
public static class Gens
{
    private static readonly BigInteger MinI128 = -(BigInteger.One << 127);
    private static readonly BigInteger MaxI128 = (BigInteger.One << 127) - BigInteger.One;
    private static readonly BigInteger Two127 = BigInteger.One << 127;
    private static readonly BigInteger Two128 = BigInteger.One << 128;

    // ---------------------------------------------------------------------------
    // Primitive generators
    // ---------------------------------------------------------------------------

    /// <summary>Always generates <paramref name="v"/> and draws nothing.</summary>
    public static Gen<T> Constant<T>(T v)
    {
        return Gen<T>.Of(_ => v, null);
    }

    /// <summary><c>Bounded(2) == 1</c>. Shrinks true to false.</summary>
    public static Gen<bool> Bool()
    {
        return Gen<bool>.Of(
            rng => rng.Bounded(2) == 1,
            (value, falsifies, maxSteps) =>
                value && maxSteps > 0 && falsifies(false)
                    ? new Shrunk<bool>(false, 1)
                    : new Shrunk<bool>(value, 0));
    }

    // Adapts the big-integer walk to a native integer type.
    private static Shrinker<T> BigShrinker<T>(
        BigInteger lo, BigInteger hi, Func<T, BigInteger> toBig, Func<BigInteger, T> fromBig)
    {
        BigInteger target = ClampZero(lo, hi);
        return (value, falsifies, maxSteps) =>
        {
            Shrunk<BigInteger> r = Shrink.ShrinkIntCore(
                toBig(value), target, lo, hi, b => falsifies(fromBig(b)), maxSteps);
            return new Shrunk<T>(fromBig(r.Value), r.Steps);
        };
    }

    /// <summary>The in-range point nearest zero: the integer shrink target.</summary>
    internal static BigInteger ClampZero(BigInteger lo, BigInteger hi)
    {
        if (lo.Sign > 0)
        {
            return lo;
        }

        if (hi.Sign < 0)
        {
            return hi;
        }

        return BigInteger.Zero;
    }

    /// <summary>[0, 255]: <c>Bounded(256)</c>.</summary>
    public static Gen<byte> U8()
    {
        return Gen<byte>.Of(
            rng => (byte)rng.Bounded(256),
            BigShrinker<byte>(BigInteger.Zero, new BigInteger(255), b => new BigInteger(b), b => (byte)b));
    }

    /// <summary>A full 64-bit word: <c>NextU64()</c>.</summary>
    public static Gen<ulong> U64()
    {
        return Gen<ulong>.Of(
            rng => rng.NextU64(),
            BigShrinker<ulong>(BigInteger.Zero, new BigInteger(ulong.MaxValue), b => new BigInteger(b), b => (ulong)b));
    }

    internal static BigInteger SampleI128(SplitMix64 rng)
    {
        BigInteger hi = rng.NextU64();
        BigInteger lo = rng.NextU64();
        BigInteger v = (hi << 64) | lo;
        if (v >= Two127)
        {
            v -= Two128;
        }

        return v;
    }

    /// <summary>
    /// A signed 128-bit integer from two stream words, high word first, read as
    /// two's complement:
    /// <code>
    ///   words 0x8000...0 : 0   -&gt; -2^127   (top bit set =&gt; subtract 2^128)
    ///   words 0x7fff...f : f.. -&gt;  2^127 - 1
    /// </code>
    /// </summary>
    public static Gen<BigInteger> I128()
    {
        return Gen<BigInteger>.Of(SampleI128, BigShrinker<BigInteger>(MinI128, MaxI128, b => b, b => b));
    }

    /// <summary>
    /// A uniform integer over the inclusive range [min, max], any width. The width is
    /// span = max - min + 1 and the offset from min is drawn by
    /// <see cref="SplitMix64.BoundedBig"/> (VER05 section 7.2). It shrinks toward the
    /// in-range point nearest zero and never leaves [min, max].
    /// </summary>
    public static Gen<BigInteger> Integer(BigInteger min, BigInteger max)
    {
        if (min > max)
        {
            throw new ArgumentException("Integer needs min <= max");
        }

        return Gen<BigInteger>.Of(rng => BoundedInt(rng, min, max), BigShrinker<BigInteger>(min, max, b => b, b => b));
    }

    internal static BigInteger BoundedInt(SplitMix64 rng, BigInteger lo, BigInteger hi)
    {
        BigInteger span = hi - lo + BigInteger.One;
        return lo + rng.BoundedBig(span);
    }

    /// <summary><see cref="Integer"/> over native long bounds: the same draws, a native result.</summary>
    public static Gen<long> Int64(long min, long max)
    {
        if (min > max)
        {
            throw new ArgumentException("Int64 needs min <= max");
        }

        // ulong arithmetic wraps, so a full-range span is 0 (meaning 2^64).
        ulong span = unchecked((ulong)max - (ulong)min + 1UL);
        return Gen<long>.Of(
            rng =>
            {
                ulong offset = span == 0 ? rng.NextU64() : rng.Bounded(span);
                return unchecked((long)((ulong)min + offset));
            },
            BigShrinker<long>(new BigInteger(min), new BigInteger(max), b => new BigInteger(b), b => (long)b));
    }

    private static void CheckLengths(int minLen, int maxLen, int limit, string what)
    {
        if (minLen < 0 || maxLen < minLen)
        {
            throw new ArgumentException(what + " needs 0 <= minLen <= maxLen");
        }

        if (maxLen > limit)
        {
            throw new ArgumentException(what + " maxLen " + maxLen + " exceeds " + limit);
        }
    }

    /// <summary>
    /// A byte string: <c>L = minLen + Bounded(span)</c>, then L draws of <c>Bounded(256)</c>.
    /// Shrinks by sequence shrinking, respecting minLen.
    /// </summary>
    public static Gen<byte[]> Bytes(int minLen, int maxLen)
    {
        CheckLengths(minLen, maxLen, Limits.MaxInputBytes, "Bytes");
        ulong span = (ulong)((long)(maxLen - minLen) + 1);
        return Gen<byte[]>.Of(
            rng =>
            {
                int n = minLen + (int)rng.Bounded(span);
                byte[] output = new byte[n];
                for (int i = 0; i < n; i++)
                {
                    output[i] = (byte)rng.Bounded(256);
                }

                return output;
            },
            (value, falsifies, maxSteps) =>
            {
                // The list conversion (and the array one on the way out) are copies too: charge
                // them first, and do nothing once the budget is spent.
                using ShrinkBudget budget = ShrinkBudget.Enter(maxSteps);
                if (maxSteps == 0 || !budget.ChargeCopy(2L * value.Length))
                {
                    return new Shrunk<byte[]>(value, 0);
                }

                Shrunk<List<int>> r = Shrink.ShrinkSequence(
                    Shrink.ToList(value),
                    c => budget.ChargeCopy(c.Count) && falsifies(Shrink.ToArray(c)),
                    maxSteps,
                    minLen,
                    Shrink.ByteWalk);
                return new Shrunk<byte[]>(Shrink.ToArray(r.Value), r.Steps);
            });
    }

    // ---------------------------------------------------------------------------
    // Combinators
    // ---------------------------------------------------------------------------

    /// <summary>Transforms generated values. Drops shrinking: <paramref name="f"/> has no inverse.</summary>
    public static Gen<U> Map<T, U>(Gen<T> g, Func<T, U> f)
    {
        return Gen<U>.Of(rng => f(g.Sample(rng)), null);
    }

    /// <summary><see cref="Filter{T}(Gen{T}, Func{T, bool}, int)"/> with the default retry count (100).</summary>
    public static Gen<T> Filter<T>(Gen<T> g, Func<T, bool> pred)
    {
        return Filter(g, pred, Limits.DefaultMaxRetries);
    }

    /// <summary>
    /// Keeps only values satisfying <paramref name="pred"/>. After exactly
    /// <paramref name="maxRetries"/> rejected samples sampling throws
    /// <see cref="ExhaustedException"/>. Shrinking is preserved: a shrink candidate that
    /// fails <paramref name="pred"/> is simply not a valid input, so it is treated as
    /// "does not falsify".
    /// </summary>
    public static Gen<T> Filter<T>(Gen<T> g, Func<T, bool> pred, int maxRetries)
    {
        if (maxRetries < 1 || maxRetries > Limits.MaxRetries)
        {
            throw new ArgumentException("Filter needs 1 <= maxRetries <= " + Limits.MaxRetries);
        }

        return Gen<T>.Of(
            rng =>
            {
                // Every retry inside a check is charged, so stacked filters (each retrying 10,000
                // times) cannot multiply uncharged work: inside a NESTED check a retry is an
                // attempt charged up the chain; in a top-level check it is one element of work
                // against the check root's 64M allowance (legitimate use needs thousands, not
                // millions). When refused the sample is exhausted.
                ShrinkBudget? budget = ShrinkBudget.Current;
                bool charged = budget != null;
                bool nested = budget != null && budget.IsNested;
                for (int i = 0; i < maxRetries; i++)
                {
                    if (charged && !(nested ? budget!.Charge(1) : budget!.ChargeCopy(1)))
                    {
                        throw new ExhaustedException("the enclosing shrink budget was spent while filtering");
                    }

                    T v = g.Sample(rng);
                    if (pred(v))
                    {
                        return v;
                    }
                }

                throw new ExhaustedException("filter rejected " + maxRetries + " samples");
            },
            (value, falsifies, maxSteps) =>
            {
                // Charge before the user predicate runs: it is property code and may be costly.
                ShrinkBudget? budget = ShrinkBudget.Current;
                return g.ShrinkNested(value, c => (budget == null || budget.Charge(1)) && pred(c) && falsifies(c), maxSteps);
            });
    }

    /// <summary>Dependent generation: sample, then sample from <c>f(value)</c>. No shrinking.</summary>
    public static Gen<U> FlatMap<T, U>(Gen<T> g, Func<T, Gen<U>> f)
    {
        return Gen<U>.Of(rng => f(g.Sample(rng)).Sample(rng), null);
    }

    /// <summary>Picks one generator uniformly (<c>Bounded(len)</c>, then that arm's draws). No shrinking.</summary>
    public static Gen<T> OneOf<T>(params Gen<T>[] gens)
    {
        if (gens.Length == 0)
        {
            throw new ArgumentException("OneOf needs at least one generator");
        }

        Gen<T>[] arms = (Gen<T>[])gens.Clone();
        return Gen<T>.Of(rng => arms[(int)rng.Bounded((ulong)arms.Length)].Sample(rng), null);
    }

    /// <summary>
    /// A weighted choice: <c>pick = Bounded(total)</c>; arms are walked in order,
    /// subtracting each weight, so <c>Frequency(Arm(1, a), Arm(3, b))</c> picks b three
    /// times in four. A zero-weight arm can never be selected. An empty list, a negative
    /// weight, a zero total or a total above 2^32 is rejected.
    /// </summary>
    public static Gen<T> Frequency<T>(params Arm<T>[] arms)
    {
        if (arms.Length == 0)
        {
            throw new ArgumentException("Frequency needs at least one arm");
        }

        long total = 0;
        foreach (Arm<T> a in arms)
        {
            if (a.Weight < 0 || a.Weight > Limits.MaxFrequencyTotal)
            {
                throw new ArgumentException("Frequency weight outside [0, 2^32]");
            }

            total += a.Weight; // at most len * 2^32: cannot wrap a long
            if (total > Limits.MaxFrequencyTotal)
            {
                throw new ArgumentException("Frequency total weight exceeds 2^32");
            }
        }

        if (total == 0)
        {
            throw new ArgumentException("Frequency total weight must be > 0");
        }

        Arm<T>[] owned = (Arm<T>[])arms.Clone();
        long sum = total;
        return Gen<T>.Of(
            rng =>
            {
                long pick = (long)rng.Bounded((ulong)sum);
                foreach (Arm<T> a in owned)
                {
                    if (pick < a.Weight)
                    {
                        return a.Gen.Sample(rng);
                    }

                    pick -= a.Weight;
                }

                throw new InvalidOperationException("unreachable: pick < total");
            },
            null);
    }

    /// <summary>
    /// A list: <c>L = minLen + Bounded(span)</c>, then L elements in order. Shrinks by
    /// sequence shrinking, respecting minLen, with the element generator's shrinker for
    /// the element-wise phase.
    /// </summary>
    public static Gen<List<T>> ListOf<T>(Gen<T> elem, int minLen, int maxLen)
    {
        CheckLengths(minLen, maxLen, Limits.MaxListLen, "ListOf");
        ulong span = (ulong)((long)(maxLen - minLen) + 1);
        return Gen<List<T>>.Of(
            rng =>
            {
                int n = minLen + (int)rng.Bounded(span);
                List<T> output = new(Math.Min(n, 1024));
                for (int i = 0; i < n; i++)
                {
                    output.Add(elem.Sample(rng));
                }

                return output;
            },
            (value, falsifies, maxSteps) =>
                Shrink.ShrinkSequence(value, falsifies, maxSteps, minLen, elem.ShrinkNested));
    }

    // One shrink-wide evaluation budget for a tuple: the repeat-until-no-progress loop
    // calls each component shrinker once per pass, and each of those has its own local
    // budget, so without a shared one the work is quadratic in maxSteps. Every
    // evaluation of the property, in any component or pass, is charged here; when it
    // is spent nothing further is accepted.
    private static Func<V, bool> GlobalBudget<V>(Func<V, bool> falsifies, int maxSteps)
    {
        long budget = Limits.AttemptBudget(maxSteps);
        long spent = 0;
        return v =>
        {
            if (Interlocked.Increment(ref spent) - 1 >= budget)
            {
                return false;
            }

            return falsifies(v);
        };
    }

    /// <summary>Samples a then b, and shrinks one coordinate at a time.</summary>
    public static Gen<Pair<A, B>> Tuple2<A, B>(Gen<A> ga, Gen<B> gb)
    {
        return Gen<Pair<A, B>>.Of(
            rng =>
            {
                A a = ga.Sample(rng);
                B b = gb.Sample(rng);
                return new Pair<A, B>(a, b);
            },
            (value, rawFalsifies, maxSteps) =>
            {
                Func<Pair<A, B>, bool> falsifies = GlobalBudget(rawFalsifies, maxSteps);
                A first = value.First;
                B second = value.Second;
                int steps = 0;

                // Repeat passes until a whole pass makes no progress: shrinking one
                // coordinate can unlock more shrinking of the other. Every pass that
                // continues accepted at least one move, so the budget bounds it.
                bool progress = true;
                while (progress && steps < maxSteps)
                {
                    progress = false;
                    if (maxSteps - steps > 0)
                    {
                        B fixedB = second;
                        Shrunk<A> r = ga.ShrinkNested(first, a => falsifies(new Pair<A, B>(a, fixedB)), maxSteps - steps);
                        if (r.Steps > 0)
                        {
                            first = r.Value;
                            steps += r.Steps;
                            progress = true;
                        }
                    }

                    if (maxSteps - steps > 0)
                    {
                        A fixedA = first;
                        Shrunk<B> r = gb.ShrinkNested(second, b => falsifies(new Pair<A, B>(fixedA, b)), maxSteps - steps);
                        if (r.Steps > 0)
                        {
                            second = r.Value;
                            steps += r.Steps;
                            progress = true;
                        }
                    }
                }

                return new Shrunk<Pair<A, B>>(new Pair<A, B>(first, second), steps);
            });
    }

    /// <summary>Samples a, b, c in order and shrinks one coordinate at a time.</summary>
    public static Gen<Triple<A, B, C>> Tuple3<A, B, C>(Gen<A> ga, Gen<B> gb, Gen<C> gc)
    {
        return Gen<Triple<A, B, C>>.Of(
            rng =>
            {
                A a = ga.Sample(rng);
                B b = gb.Sample(rng);
                C c = gc.Sample(rng);
                return new Triple<A, B, C>(a, b, c);
            },
            (value, rawFalsifies, maxSteps) =>
            {
                Func<Triple<A, B, C>, bool> falsifies = GlobalBudget(rawFalsifies, maxSteps);
                A first = value.First;
                B second = value.Second;
                C third = value.Third;
                int steps = 0;
                bool progress = true;
                while (progress && steps < maxSteps)
                {
                    progress = false;
                    if (maxSteps - steps > 0)
                    {
                        B fb = second;
                        C fc = third;
                        Shrunk<A> r = ga.ShrinkNested(first, a => falsifies(new Triple<A, B, C>(a, fb, fc)), maxSteps - steps);
                        if (r.Steps > 0)
                        {
                            first = r.Value;
                            steps += r.Steps;
                            progress = true;
                        }
                    }

                    if (maxSteps - steps > 0)
                    {
                        A fa = first;
                        C fc = third;
                        Shrunk<B> r = gb.ShrinkNested(second, b => falsifies(new Triple<A, B, C>(fa, b, fc)), maxSteps - steps);
                        if (r.Steps > 0)
                        {
                            second = r.Value;
                            steps += r.Steps;
                            progress = true;
                        }
                    }

                    if (maxSteps - steps > 0)
                    {
                        A fa = first;
                        B fb = second;
                        Shrunk<C> r = gc.ShrinkNested(third, c => falsifies(new Triple<A, B, C>(fa, fb, c)), maxSteps - steps);
                        if (r.Steps > 0)
                        {
                            third = r.Value;
                            steps += r.Steps;
                            progress = true;
                        }
                    }
                }

                return new Shrunk<Triple<A, B, C>>(new Triple<A, B, C>(first, second, third), steps);
            });
    }

    // ---------------------------------------------------------------------------
    // Corpus entry points (pbt-v1)
    // ---------------------------------------------------------------------------

    private static void CheckCount(int count)
    {
        if (count < 0 || count > Limits.MaxGenCount)
        {
            throw new ArgumentOutOfRangeException(
                nameof(count), "count " + count + " outside [0, " + Limits.MaxGenCount + "]");
        }
    }

    /// <summary>
    /// <paramref name="count"/> primitives of <paramref name="typeName"/> ("bool", "u8",
    /// "u64", "i128") drawn from <paramref name="seed"/>. Booleans are <c>bool</c>; every
    /// integer is a <see cref="BigInteger"/> (the corpus compares by value).
    /// </summary>
    public static List<object> GenPrimitive(ulong seed, string typeName, int count)
    {
        CheckCount(count);
        SplitMix64 rng = Hooks.NewRng(seed);
        List<object> output = new(Math.Min(count, 1024));
        for (int i = 0; i < count; i++)
        {
            switch (typeName)
            {
                case "bool":
                    output.Add(rng.Bounded(2) == 1);
                    break;
                case "u8":
                    output.Add(new BigInteger(rng.Bounded(256)));
                    break;
                case "u64":
                    output.Add(new BigInteger(rng.NextU64()));
                    break;
                case "i128":
                    output.Add(SampleI128(rng));
                    break;
                default:
                    throw new ArgumentException("unknown primitive type \"" + typeName + "\"");
            }
        }

        return output;
    }

    /// <summary><paramref name="count"/> uniform integers over [lo, hi] drawn from <paramref name="seed"/>.</summary>
    public static List<BigInteger> GenBounded(ulong seed, BigInteger lo, BigInteger hi, int count)
    {
        if (lo > hi)
        {
            throw new ArgumentException("need lo <= hi");
        }

        CheckCount(count);

        // Each draw is an integer about as wide as the span; count x width bounds what
        // the returned list can hold (a 1,000-digit span x 10^6 draws would not).
        long width = Math.Max(1, ((long)(hi - lo).GetBitLength() + 7) / 8);
        if ((long)count * width > Limits.MaxGenBytesTotal)
        {
            throw new ArgumentException(
                "count " + count + " x " + width + "-byte span exceeds the generation budget of "
                + Limits.MaxGenBytesTotal + " bytes");
        }

        SplitMix64 rng = Hooks.NewRng(seed);
        List<BigInteger> output = new(Math.Min(count, 1024));
        for (int i = 0; i < count; i++)
        {
            output.Add(BoundedInt(rng, lo, hi));
        }

        return output;
    }

    /// <summary><paramref name="count"/> byte strings of length in [minLen, maxLen] from <paramref name="seed"/>.</summary>
    public static List<byte[]> GenBytes(ulong seed, int minLen, int maxLen, int count)
    {
        if (minLen < 0 || maxLen < minLen || maxLen > Limits.MaxInputBytes)
        {
            throw new ArgumentException("need 0 <= minLen <= maxLen <= " + Limits.MaxInputBytes);
        }

        CheckCount(count); // applies even when maxLen == 0: empty strings still cost one slot each
        if (maxLen > 0 && count > Limits.MaxGenBytesTotal / maxLen)
        {
            throw new ArgumentException(
                "count " + count + " x maxLen " + maxLen + " exceeds the total of " + Limits.MaxGenBytesTotal + " bytes");
        }

        Gen<byte[]> gen = Bytes(minLen, maxLen);
        SplitMix64 rng = Hooks.NewRng(seed);
        List<byte[]> output = new(Math.Min(count, 1024));
        for (int i = 0; i < count; i++)
        {
            output.Add(gen.Sample(rng));
        }

        return output;
    }
}
