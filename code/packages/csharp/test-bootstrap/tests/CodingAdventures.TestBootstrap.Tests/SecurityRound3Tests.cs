using System.Numerics;
using CodingAdventures.TestBootstrap.Pbt;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>Regressions for the third C# security review (internal ShrinkNested, work estimate, stacked filters).</summary>
public sealed class SecurityRound3Tests : HookedTest
{
    private static BigInteger Big(long v) => new(v);

    // HIGH: ShrinkNested is internal, and even a leaf shrinker that charges nothing is bounded by it.
    [Fact]
    public void ShrinkNestedIsInternalOnTheGenType()
    {
        System.Reflection.MethodInfo? m = typeof(Gen<int>).GetMethod("ShrinkNested");
        Assert.Null(m); // not public: GetMethod only finds public instance methods
        Assert.NotNull(typeof(Gen<int>).GetMethod("ShrinkNested", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic));
    }

    private static (Gen<BigInteger> Top, Func<long> Calls, Func<Func<BigInteger, bool>> Property) RecursiveChain()
    {
        long calls = 0;
        Gen<BigInteger> leaf = Gens.Integer(Big(0), Big(1000));
        Gen<BigInteger> mid = Gen<BigInteger>.Of(
            _ => Big(0),
            (v, f, s) =>
            {
                for (int i = 0; i < 30; i++)
                {
                    leaf.ShrinkNested(Big(900), f, s);
                }

                return new Shrunk<BigInteger>(v, 0);
            });
        Gen<BigInteger> top = Gen<BigInteger>.Of(
            _ => Big(0),
            (v, f, s) =>
            {
                for (int i = 0; i < 30; i++)
                {
                    mid.ShrinkNested(v, f, s);
                }

                return new Shrunk<BigInteger>(v, 0);
            });
        return (top, () => Interlocked.Read(ref calls), () => x => { Interlocked.Increment(ref calls); return x >= 10; });
    }

    [Fact]
    public void ANestedShrinkNestedChainIsBoundedWithAndWithoutAnAmbientScope()
    {
        const int maxSteps = 5;

        // No ambient scope: ShrinkNested opens a fresh root, which caps the whole recursion.
        (Gen<BigInteger> top, Func<long> calls, Func<Func<BigInteger, bool>> prop) = RecursiveChain();
        top.ShrinkNested(Big(900), prop(), maxSteps);
        Assert.True(calls() <= Limits.AttemptBudget(maxSteps), "no ambient: evaluations " + calls());
        Assert.True(calls() > 0);

        // A spent ambient root: every evaluation is refused.
        (Gen<BigInteger> top2, Func<long> calls2, Func<Func<BigInteger, bool>> prop2) = RecursiveChain();
        using (ShrinkBudget root = ShrinkBudget.EnterChild(0))
        {
            while (root.Charge(1))
            {
            }

            top2.ShrinkNested(Big(900), prop2(), maxSteps);
            Assert.Equal(0, calls2());
            Assert.True(root.WasRefused);
        }

        // With room under an ambient root: bounded by that root's cap, not by 30^3 x 20.
        (Gen<BigInteger> top3, Func<long> calls3, Func<Func<BigInteger, bool>> prop3) = RecursiveChain();
        using (ShrinkBudget root = ShrinkBudget.EnterChild(maxSteps))
        {
            top3.ShrinkNested(Big(900), prop3(), maxSteps);
            Assert.True(calls3() <= Limits.AttemptBudget(maxSteps), "ambient: evaluations " + calls3());
            Assert.True(root.Attempts <= Limits.AttemptBudget(maxSteps));
        }
    }

    // LOW: ICollection<T>-only types and tuples are counted as work too.
    [Fact]
    public void ANestedCheckSamplingAHashSetOrATupleIsChargedAsWork()
    {
        HashSet<int> set = new(Enumerable.Range(0, 2000));
        List<Verdict> inner = new();
        ShrinkBudget.DefaultMaxWork = 10_000;
        Gens.Integer(Big(0), Big(1000)).Shrink(
            Big(900),
            x =>
            {
                inner.Add(Runner.Check(1, Gens.Constant(set), _ => true, new Options(50, 10, CiMode.Off)));
                return x >= 10;
            },
            1000);
        Assert.Contains(inner, v => v.FailureId == FailureIds.Exhausted);

        // A tuple counts its items, and a lazily enumerated sequence is counted up to the walk cap.
        List<Verdict> tuples = new();
        ShrinkBudget.DefaultMaxWork = 20;
        Gens.Integer(Big(0), Big(1000)).Shrink(
            Big(900),
            x =>
            {
                tuples.Add(Runner.Check(1, Gens.Constant(new Triple<int, int, int>(1, 2, 3)), _ => true, new Options(50, 10, CiMode.Off)));
                return x >= 10;
            },
            1000);
        Assert.Contains(tuples, v => v.FailureId == FailureIds.Exhausted);
    }

    // LOW (shared with Java): stacked filters multiply retries; every retry is charged even at top level.
    [Fact]
    public void StackedTopLevelFiltersAreBoundedByTheCheckRootWork()
    {
        long predCalls = 0;
        Func<int, Func<BigInteger, bool>> multipleOf = k => x => { predCalls++; return x % k == 0; };
        Gen<BigInteger> source = Gens.Integer(Big(1), Big(10_000_000));
        Gen<BigInteger> stacked = Gens.Filter(
            Gens.Filter(Gens.Filter(source, multipleOf(31), Limits.MaxRetries), multipleOf(37), Limits.MaxRetries),
            multipleOf(41),
            Limits.MaxRetries);

        // About 31 x 37 x 41 = 47,000 predicate calls are expected per sample; the root allows 2,000.
        ShrinkBudget.DefaultMaxWork = 2_000;
        Verdict v = Runner.Check(1, stacked, _ => true, new Options(1, 10, CiMode.Off));
        Assert.Equal(FailureIds.Exhausted, v.FailureId);
        Assert.True(predCalls <= 2_000 + 10, "predicate calls " + predCalls);

        // With the normal allowance an ordinary filter at top level is unaffected.
        Hooks.Reset();
        long ordinary = 0;
        Verdict ok = Runner.Check(
            1,
            Gens.Filter(Gens.Integer(Big(0), Big(999)), x => { ordinary++; return x < 10; }, 10_000),
            _ => true,
            new Options(100, 10, CiMode.Off));
        Assert.True(ok.Passed, ok.Message);
        Assert.True(ordinary > 100);
    }
}
