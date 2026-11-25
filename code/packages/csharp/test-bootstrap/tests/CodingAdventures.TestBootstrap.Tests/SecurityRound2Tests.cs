using System.Numerics;
using CodingAdventures.TestBootstrap.Pbt;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>Regressions for the second C# security review (re-entrancy, thread hops, uncharged copies).</summary>
public sealed class SecurityRound2Tests : HookedTest
{
    private static BigInteger Big(long v) => new(v);

    private static Verdict InnerRun() =>
        Runner.Check(5, Gens.ListOf(Gens.U8(), 0, 50), l => l.Sum(b => b) <= 20, new Options(200, 1000, CiMode.On));

    // LOW-1: a property that runs its own Runner.Check gets its own isolated budget.
    [Fact]
    public void AReentrantRunnerCheckIsNotStarvedByTheOuterShrink()
    {
        Verdict standalone = InnerRun();
        Assert.Equal(FailureIds.Counterexample, standalone.FailureId);
        Assert.True(standalone.ShrinkSteps > 3, "the inner shrink must need a real budget");

        List<Verdict> inner = new();
        Func<BigInteger, bool> plain = x => x < 500;
        Func<BigInteger, bool> reentrant = x =>
        {
            inner.Add(InnerRun());
            return x < 500;
        };
        Options outer = new(100, 10_000, CiMode.On); // room: an inner check is standalone-like
        Verdict expected = Runner.Check(9, Gens.Integer(Big(0), Big(1000)), plain, outer);
        Verdict actual = Runner.Check(9, Gens.Integer(Big(0), Big(1000)), reentrant, outer);

        Assert.NotEmpty(inner);
        Assert.All(inner, v => Assert.Equal(standalone, v)); // same digest, steps and message
        Assert.Equal(expected, actual);                      // the outer verdict is unaffected
        Assert.Null(ShrinkBudget.Current);                   // and the ambient budget was restored
    }

    [Fact]
    public void AReentrantGenShrinkAndShrinkListGetFreshBudgetsAndRestoreTheOuterOne()
    {
        Gen<List<byte>> g = Gens.ListOf(Gens.U8(), 0, 40);
        List<byte> start = Enumerable.Repeat((byte)3, 40).ToList();
        int innerCalls = 0;
        Gen<BigInteger> outerGen = Gens.Integer(Big(0), Big(1000));
        Shrunk<BigInteger> r = outerGen.Shrink(
            Big(900),
            x =>
            {
                // Each evaluation runs a full inner shrink; it must always find the same minimum.
                Shrunk<List<byte>> a = g.Shrink(start, l => l.Count >= 1, 1000);
                Shrunk<List<BigInteger>> b = Shrink.ShrinkList(new List<BigInteger> { 5, 6, 7 }, BigInteger.Zero, l => l.Count == 0, 1000);
                innerCalls++;
                Assert.Single(a.Value);
                Assert.Equal(new List<BigInteger> { 0 }, b.Value);
                return x >= 10;
            },
            1000);
        Assert.True(innerCalls > 3);
        Assert.True(r.Value >= 10);
        Assert.Null(ShrinkBudget.Current);
    }

    // LOW-2: the budget flows into child tasks and is safe for concurrent charging.
    [Fact]
    public async Task ShrinkingInsideTasksStaysWithinOneGlobalBudget()
    {
        Gen<List<byte>> lists = Gens.ListOf(Gens.U8(), 0, 50);
        List<byte> start = Enumerable.Repeat((byte)7, 50).ToList();
        long calls = 0;
        const int maxSteps = 10;
        Gen<BigInteger> parent = Gen<BigInteger>.Of(
            _ => Big(0),
            (value, falsifies, steps) =>
            {
                Task[] tasks = Enumerable.Range(0, 12).Select(_ => Task.Run(() =>
                    lists.ShrinkNested(start, l => { Interlocked.Increment(ref calls); return l.Count == 50; }, maxSteps))).ToArray();
                Task.WaitAll(tasks);
                return new Shrunk<BigInteger>(value, 0);
            });
        parent.Shrink(Big(0), _ => true, maxSteps);
        await Task.Yield();
        Assert.True(Interlocked.Read(ref calls) <= Limits.AttemptBudget(maxSteps), "evaluations " + calls + " across tasks");
        Assert.True(calls > 50, "the tasks must really have shrunk");
        Assert.Null(ShrinkBudget.Current);
    }

    // LOW-3: the initial copies are charged and skipped once the budget is spent.
    [Fact]
    public void TuplePassesOverALargeListDoNotCopyAfterTheBudgetIsSpent()
    {
        ShrinkBudget.DefaultMaxWork = 2_000_000; // small work allowance so the test is quick
        const int n = 10_000;
        Gen<BigInteger> oneStep = Gen<BigInteger>.Of(
            _ => Big(1_000_000),
            (value, falsifies, max) =>
                value.Sign > 0 && max > 0 && falsifies(value - 1)
                    ? new Shrunk<BigInteger>(value - 1, 1)
                    : new Shrunk<BigInteger>(value, 0));
        Gen<Pair<List<long>, BigInteger>> g = Gens.Tuple2(Gens.ListOf(Gens.Int64(0, 9), 0, n), oneStep);
        Pair<List<long>, BigInteger> start = new(new List<long>(new long[n]), Big(1_000_000));

        long before = GC.GetAllocatedBytesForCurrentThread();
        Shrunk<Pair<List<long>, BigInteger>> r = g.Shrink(start, p => p.First.Count == n, 1000);
        long allocated = GC.GetAllocatedBytesForCurrentThread() - before;

        Assert.True(r.Steps >= 900, "the stateful component must force many passes: " + r.Steps);
        Assert.True(ShrinkBudget.LastWork <= 2_000_000, "charged work " + ShrinkBudget.LastWork);
        // Charged copies: <= 2M elements x 8 bytes = 16 MB. Uncharged initial copies on ~1000 passes
        // would add 1000 x 80 KB = 80 MB more.
        Assert.True(allocated < 40_000_000, "allocated " + allocated);
    }

    [Fact]
    public void BytesAndListEntryPointsChargeTheirInitialCopyAndReturnTheInputWhenSpent()
    {
        ShrinkBudget.DefaultMaxWork = 5;
        byte[] data = new byte[100];
        int calls = 0;
        Shrunk<byte[]> b = Gens.Bytes(0, 200).Shrink(data, x => { calls++; return true; }, 1000);
        Assert.Same(data, b.Value);
        Assert.Equal(0, b.Steps);
        Shrunk<List<BigInteger>> l = Shrink.ShrinkList(Enumerable.Repeat(Big(1), 100).ToList(), BigInteger.Zero, x => false, 1000);
        Assert.Equal(100, l.Value.Count);
        Assert.Equal(0, l.Steps);
        Shrunk<byte[]> pb = Shrink.ShrinkBytes(data, x => false, 1000);
        Assert.Same(data, pb.Value);
        Assert.Equal(0, calls);
    }
}
