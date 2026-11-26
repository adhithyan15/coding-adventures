using System.Collections;
using System.Collections.Concurrent;
using System.Numerics;
using CodingAdventures.TestBootstrap.CorpusJson;
using CodingAdventures.TestBootstrap.Pbt;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>
/// The hierarchical budget tree (VER05g section 3.1): the bound holds whatever the nesting, with no
/// classification of a call as "inside a property" or "inside a shrinker". Ported from the Java
/// engine's ShrinkBudgetTest (VER05e section 4.1).
/// </summary>
public sealed class ShrinkBudgetTreeTests : HookedTest
{
    private static BigInteger Big(long v) => new(v);

    /// <summary>Runs <paramref name="body"/> with a small root work cap; returns the root's {attempts, work}.</summary>
    private static long[] RootTotals(long cap, Action body)
    {
        ShrinkBudget.DefaultMaxWork = cap;
        body();
        return new[] { ShrinkBudget.LastAttempts, ShrinkBudget.LastWork };
    }

    private static Func<List<byte>, bool> SizeIs(int n, Action tick) => l =>
    {
        tick();
        return l.Count == n;
    };

    private static Gen<List<byte>> Lists(int max) => Gens.ListOf(Gens.U8(), 0, max);

    private static List<byte> Sevens(int n) => Enumerable.Repeat((byte)7, n).ToList();

    // 1. A property calling Runner.Check on every attempt, at nesting depths 1, 2 and 3.
    private static void Nested(int depth, ref long evals, int maxSteps)
    {
        long local = 0;
        Runner.Check(
            1,
            Lists(40),
            l =>
            {
                if (ShrinkBudget.Current != null)
                {
                    local++; // only evaluations inside the outermost shrink
                }

                if (depth > 1)
                {
                    long inner = 0;
                    Nested(depth - 1, ref inner, maxSteps);
                    local += inner;
                }

                return l.Count < 5;
            },
            new Options(100, maxSteps, CiMode.Off));
        evals += local;
    }

    [Fact]
    public void APropertyCallingRunnerCheckKeepsTheRootTotalsWithinTheRootCap()
    {
        for (int depth = 1; depth <= 3; depth++)
        {
            const int maxSteps = 5;
            long evals = 0;
            int d = depth;
            long[] t = RootTotals(1_000_000, () => Nested(d, ref evals, maxSteps));
            long cap = 100 + Limits.AttemptBudget(maxSteps); // numTests + shrink cap
            Assert.True(t[0] <= cap, "depth " + d + " attempts " + t[0]);
            Assert.True(t[1] <= 1_000_000, "depth " + d + " work " + t[1]);
            Assert.True(evals > 0);
            Assert.True(evals <= 3 * cap * d, "depth " + d + " evaluations " + evals);
            Hooks.Reset();
        }
    }

    // A branching property (m = 2 nested checks per evaluation, depth 4) stays inside the root cap.
    private static void Branch(int level, ref long counted)
    {
        long local = 0;
        Runner.Check(
            1,
            Gens.Integer(Big(0), Big(9)),
            x =>
            {
                if (level > 1)
                {
                    local++; // evaluations made by NESTED checks
                }

                if (level < 4)
                {
                    long a = 0;
                    long b = 0;
                    Branch(level + 1, ref a);
                    Branch(level + 1, ref b);
                    local += a + b;
                }

                return x < 3;
            },
            new Options(10, 5, CiMode.Off));
        counted += local;
    }

    [Fact]
    public void ABranchingPropertyKeepsTotalEvaluationsWithinTheRootCap()
    {
        long counted = 0;
        long[] t = RootTotals(1_000_000, () => Branch(1, ref counted));
        long cap = 10 + Limits.AttemptBudget(5);
        Assert.True(counted > 0);
        Assert.True(t[0] <= cap, "root attempts " + t[0]);
        Assert.True(counted <= cap * 2, "nested evaluations " + counted + " (2^depth would be far more)");
    }

    // 2. A custom shrinker recursing into Gen.Shrink / ShrinkList, and a filter predicate doing so.
    [Fact]
    public void ARecursingCustomShrinkerStaysInsideTheRootCap()
    {
        Gen<List<byte>> inner = Lists(50);
        List<byte> start = Sevens(50);
        long calls = 0;
        long extra = 0;
        const int maxSteps = 10;
        Shrinker<List<byte>> custom = (value, falsifies, steps) =>
        {
            for (int pass = 0; pass < 100; pass++)
            {
                inner.Shrink(new List<byte>(value), falsifies, steps);
                inner.Shrink(new List<byte>(value), SizeIs(50, () => extra++), steps);
                Shrink.ShrinkList(new List<BigInteger> { 1, 2, 3 }, BigInteger.Zero, l => l.Count == 0, steps);
            }

            return new Shrunk<List<byte>>(value, 0);
        };
        Gen<List<byte>> outer = Gen<List<byte>>.Of(_ => new List<byte>(start), custom);
        long[] t = RootTotals(1_000_000, () => outer.Shrink(new List<byte>(start), SizeIs(50, () => calls++), maxSteps));
        Assert.True(t[0] <= Limits.AttemptBudget(maxSteps), "root attempts " + t[0]);
        Assert.True(t[1] <= 1_000_000, "root work " + t[1]);
        Assert.True(calls + extra <= Limits.AttemptBudget(maxSteps), "all evaluations " + (calls + extra));
        Assert.True(extra > 0);
    }

    [Fact]
    public void AFilterPredicateThatShrinksStaysInsideTheRootCap()
    {
        Gen<List<byte>> g = Lists(200);
        List<byte> start = Enumerable.Repeat((byte)3, 200).ToList();
        long innerEvals = 0;
        const int maxSteps = 5;
        Gen<BigInteger> filtered = Gens.Filter(
            Gens.Integer(Big(0), Big(1000)),
            _ =>
            {
                g.Shrink(new List<byte>(start), SizeIs(200, () => innerEvals++), 1000);
                return true;
            },
            100);
        long[] t = RootTotals(5_000_000, () => filtered.Shrink(Big(900), x => x >= 10, maxSteps));
        Assert.True(innerEvals > 0, "the predicate must have shrunk");
        Assert.True(t[0] <= Limits.AttemptBudget(maxSteps), "root attempts " + t[0]);
        Assert.True(t[1] <= 5_000_000, "root work " + t[1]);
        // Each inner shrink alone could use ~400 attempts; the root allows 144 in all.
        Assert.True(innerEvals <= Limits.AttemptBudget(maxSteps), "inner evaluations " + innerEvals);
    }

    [Fact]
    public void AFilterPredicateIsNotInvokedUnderASpentBudget()
    {
        int calls = 0;
        Gen<BigInteger> filtered = Gens.Filter(Gens.Integer(Big(0), Big(1000)), _ => { calls++; return true; }, 100);
        using (ShrinkBudget root = ShrinkBudget.EnterChild(0))
        {
            while (root.Charge(1))
            {
            }

            Shrunk<BigInteger> r = filtered.Shrink(Big(900), _ => true, 1000);
            Assert.Equal(0, r.Steps);
            Assert.Equal(0, calls);
        }

        Shrunk<BigInteger> ok = filtered.Shrink(Big(900), x => x >= 10, 1000);
        Assert.True(calls > 0);
        Assert.Equal(Big(10), ok.Value);
    }

    // The filter SAMPLER's retries are attempts charged up the chain inside a nested check.
    [Fact]
    public void AFilterSamplerRejectingEverythingStaysInsideTheRootCapInANestedCheck()
    {
        long predCalls = 0;
        Gen<BigInteger> rejecting = Gens.Filter(Gens.Integer(Big(0), Big(1000)), _ => { predCalls++; return false; }, Limits.MaxRetries);
        const int maxSteps = 5;
        RootTotals(
            1_000_000,
            () => Gens.Integer(Big(0), Big(1000)).Shrink(
                Big(900),
                x =>
                {
                    Runner.Check(1, rejecting, _ => true, new Options(3, 10, CiMode.Off));
                    return x >= 10;
                },
                maxSteps));
        Assert.True(predCalls > 0);
        Assert.True(predCalls <= Limits.AttemptBudget(maxSteps), "sampler retries " + predCalls);
    }

    [Fact]
    public void MaxRetriesIsCappedByALimit()
    {
        Gen<BigInteger> g = Gens.Integer(Big(0), Big(9));
        Assert.NotNull(Gens.Filter(g, _ => true, Limits.MaxRetries));
        Assert.Throws<ArgumentException>(() => Gens.Filter(g, _ => true, Limits.MaxRetries + 1));
    }

    // 3. A thread spawned in a scope charges the root, also after its spawning inner scope closed.
    [Fact]
    public void AThreadSpawnedInAScopeChargesTheRootEvenAfterTheInnerScopeClosed()
    {
        using ShrinkBudget root = ShrinkBudget.EnterChild(10);
        Thread t;
        using ManualResetEventSlim go = new(false);
        bool charged = false;
        using (ShrinkBudget inner = ShrinkBudget.EnterChild(10))
        {
            t = new Thread(() =>
            {
                go.Wait();
                ShrinkBudget? cur = ShrinkBudget.Current;
                charged = cur != null && cur.Charge(1);
            });
            t.Start();
        }

        long before = root.Attempts;
        go.Set();
        t.Join();
        Assert.True(charged);
        Assert.Equal(before + 1, root.Attempts);
    }

    // 4. A flow outliving the root gets a fresh working root each time; a reused worker is not starved.
    [Fact]
    public void AThreadOutlivingTheRootGetsAFreshWorkingRootEachTime()
    {
        using ManualResetEventSlim go = new(false);
        Thread t;
        List<int> sizes = new();
        using (ShrinkBudget root = ShrinkBudget.EnterChild(1))
        {
            t = new Thread(() =>
            {
                go.Wait();
                Gen<List<byte>> g = Lists(50);
                for (int round = 0; round < 3; round++) // the same inherited thread, reused
                {
                    sizes.Add(g.Shrink(Sevens(50), l => l.Count >= 1, 1000).Value.Count);
                }
            });
            t.Start();
        }

        go.Set();
        t.Join();
        Assert.Equal(new List<int> { 1, 1, 1 }, sizes);
    }

    // A flow inheriting a budget that outlives its scope must not keep the spent budget.
    [Fact]
    public void AThreadOutlivingItsScopeShrinksNormallyAfterwards()
    {
        Gen<List<byte>> lists = Lists(50);
        using ManualResetEventSlim go = new(false);
        List<Shrunk<List<byte>>> output = new();
        Thread? holder = null;
        Gen<BigInteger> parent = Gen<BigInteger>.Of(
            _ => BigInteger.Zero,
            (value, falsifies, steps) =>
            {
                holder = new Thread(() =>
                {
                    go.Wait();
                    output.Add(lists.ShrinkNested(Sevens(50), l => l.Count >= 1, 1000));
                    output.Add(lists.Shrink(Sevens(50), l => l.Count >= 1, 1000));
                });
                holder.Start();
                return new Shrunk<BigInteger>(value, 0);
            });
        parent.Shrink(BigInteger.Zero, _ => true, 10); // the scope closes here, with the thread alive
        go.Set();
        holder!.Join();
        Assert.Equal(2, output.Count);
        Assert.All(output, r => Assert.Single(r.Value));
    }

    // Cross-thread: a thread created inside a scope shares the budget (also Task.Run: the context flows).
    [Fact]
    public void ThreadsAndTasksCreatedInsideAScopeShareOneGlobalBudget()
    {
        Gen<List<byte>> lists = Lists(50);
        List<byte> start = Sevens(50);
        long calls = 0;
        const int maxSteps = 10;
        Gen<BigInteger> parent = Gen<BigInteger>.Of(
            _ => BigInteger.Zero,
            (value, falsifies, steps) =>
            {
                List<Thread> threads = new();
                for (int i = 0; i < 6; i++)
                {
                    Thread th = new(() => lists.ShrinkNested(new List<byte>(start), l => { Interlocked.Increment(ref calls); return l.Count == 50; }, maxSteps));
                    threads.Add(th);
                    th.Start();
                }

                Task[] tasks = Enumerable.Range(0, 6).Select(_ => Task.Run(() =>
                    lists.ShrinkNested(new List<byte>(start), l => { Interlocked.Increment(ref calls); return l.Count == 50; }, maxSteps))).ToArray();
                threads.ForEach(th => th.Join());
                Task.WaitAll(tasks);
                return new Shrunk<BigInteger>(value, 0);
            });
        parent.Shrink(BigInteger.Zero, _ => true, maxSteps);
        Assert.True(Interlocked.Read(ref calls) <= Limits.AttemptBudget(maxSteps), "evaluations " + calls);
        Assert.True(calls > 50, "the workers must really have shrunk");
    }

    // 12. The pool-thread escape, pinned as documented behaviour: a thread that does not flow the
    // execution context (here one that already existed, as a long-lived consumer thread does) has
    // no ambient budget, so a shrinker running there opens a fresh root.
    [Fact]
    public void AThreadThatDoesNotFlowTheContextHasNoAmbientBudget()
    {
        using BlockingCollection<Func<object?>> queue = new();
        object? seen = "unset";
        using ManualResetEventSlim done = new(false);
        Thread worker = new(() =>
        {
            foreach (Func<object?> job in queue.GetConsumingEnumerable())
            {
                seen = job();
                done.Set();
            }
        });
        worker.Start(); // started BEFORE any scope: it holds an empty context

        using (ShrinkBudget root = ShrinkBudget.EnterChild(10))
        {
            queue.Add(() => ShrinkBudget.Current);
            done.Wait();
            Assert.Null(seen); // documented limitation
            Assert.NotNull(ShrinkBudget.Current);

            // A work item that SUPPRESSES flow is the same, even on a fresh thread.
            object? suppressed = "unset";
            using (ExecutionContext.SuppressFlow())
            {
                Thread t = new(() => suppressed = ShrinkBudget.Current);
                t.Start();
                t.Join();
            }

            Assert.Null(suppressed);

            // Whereas Task.Run flows the context even on a pool thread that already existed.
            Assert.Same(root, Task.Run(() => ShrinkBudget.Current).Result);
        }

        queue.CompleteAdding();
        worker.Join();
    }

    // 5. A passing inner check with a huge NumTests under an outer shrink is bounded by case charging.
    [Fact]
    public void APassingInnerCheckWithHugeNumTestsIsBoundedUnderAnOuterShrink()
    {
        long innerCases = 0;
        const int maxSteps = 5;
        long[] t = RootTotals(
            1_000_000,
            () => Gens.Integer(Big(0), Big(1000)).Shrink(
                Big(900),
                x =>
                {
                    Runner.Check(1, Gens.Integer(Big(0), Big(9)), _ => { innerCases++; return true; }, new Options(Limits.MaxCases, 10, CiMode.Off));
                    return x >= 10;
                },
                maxSteps));
        Assert.True(innerCases <= Limits.AttemptBudget(maxSteps) + 64, "inner cases " + innerCases);
        Assert.True(t[0] <= Limits.AttemptBudget(maxSteps), "root attempts " + t[0]);
    }

    // 6. ShrinkInt and SimulateReplay inside a property are charged.
    [Fact]
    public void ShrinkIntAndSimulateReplayInsideAPropertyAreCharged()
    {
        long evals = 0;
        const int maxSteps = 5;
        long[] t = RootTotals(
            1_000_000,
            () => Gens.Integer(Big(0), Big(1000)).Shrink(
                Big(900),
                x =>
                {
                    Shrink.ShrinkInt(Big(500), BigInteger.Zero, BigInteger.Zero, Big(1000), v => { evals++; return v < 100; }, 1000);
                    Shrink.SimulateReplay(3, v => { evals++; return v < 100; });
                    return x >= 10;
                },
                maxSteps));
        Assert.True(evals > 0);
        Assert.True(t[0] <= Limits.AttemptBudget(maxSteps), "root attempts " + t[0]);
        Assert.True(evals <= Limits.AttemptBudget(maxSteps), "inner evaluations " + evals);
    }

    // 8. A starved nested check reports property-exhausted without evaluating anything; the
    // TOP-LEVEL check, which has no ambient scope, always evaluates its first case.
    [Fact]
    public void AStarvedNestedCheckIsExhaustedButATopLevelCheckStillRuns()
    {
        int evaluated = 0;
        using (ShrinkBudget root = ShrinkBudget.EnterChild(0)) // 64 attempts
        {
            while (root.Charge(1))
            {
            }

            long before = root.Attempts;
            Verdict nested = Runner.Check(1, Gens.ListOf(Gens.U8(), 5, 5), _ => { evaluated++; return false; }, new Options(10, 1000, CiMode.Off));
            Assert.Equal(FailureIds.Exhausted, nested.FailureId);
            Assert.False(nested.Passed);
            Assert.Equal(0, evaluated);
            Assert.Equal(before, root.Attempts); // all or nothing: unchanged

            // A size-0 sample charges no work, so ONLY the entry charge can stop this one.
            int zeroSized = 0;
            Verdict tiny = Runner.Check(1, Gens.Integer(Big(0), Big(9)), _ => { zeroSized++; return true; }, new Options(1, 10, CiMode.Off));
            Assert.Equal(FailureIds.Exhausted, tiny.FailureId);
            Assert.Equal(0, zeroSized);
        }

        Verdict top = Runner.Check(1, Gens.ListOf(Gens.U8(), 5, 5), _ => false, new Options(10, 1000, CiMode.Off));
        Assert.Equal(FailureIds.Counterexample, top.FailureId);
        Assert.NotNull(top.Minimal);
    }

    // A nested check pays exactly one attempt at entry (its first case is free, a size-0 sample is
    // work only): without the entry charge a branching property would multiply checks for free.
    [Fact]
    public void ANestedCheckPaysOneAttemptAtEntry()
    {
        using ShrinkBudget root = ShrinkBudget.EnterChild(1000);
        long before = root.Attempts;
        Verdict v = Runner.Check(1, Gens.Integer(Big(0), Big(9)), _ => true, new Options(1, 10, CiMode.Off));
        Assert.True(v.Passed);
        Assert.Equal(before + 1, root.Attempts);
    }

    // A nested Runner.Check during the outer case loop (before any shrink) is charged to the outer root.
    [Fact]
    public void ANestedCheckInsideTheOuterCaseLoopChargesTheOuterRoot()
    {
        long innerCases = 0;
        long[] t = RootTotals(
            1_000_000,
            () => Runner.Check(
                1,
                Gens.Integer(Big(0), Big(9)),
                _ =>
                {
                    Runner.Check(2, Gens.Integer(Big(0), Big(9)), _ => { innerCases++; return true; }, new Options(200, 10, CiMode.Off));
                    return true; // the outer property never fails: no shrink at all
                },
                new Options(5, 5, CiMode.Off)));
        long cap = 5 + Limits.AttemptBudget(5);
        Assert.True(t[0] > 0 && t[0] <= cap, "root attempts " + t[0]);
        Assert.True(innerCases <= cap, "inner cases " + innerCases + " (uncharged: 5 x 200)");
    }

    [Fact]
    public void ARefusedTakeLeavesEveryCounterUnchanged()
    {
        using ShrinkBudget root = ShrinkBudget.EnterChild(1000, 100);
        using ShrinkBudget child = ShrinkBudget.EnterChild(1000, 1_000_000);
        Assert.True(child.Charge(60));
        Assert.False(child.Charge(60)); // the root has only 40 work left
        Assert.Equal(1, child.Attempts);
        Assert.Equal(60, child.Work);
        Assert.Equal(1, root.Attempts);
        Assert.Equal(60, root.Work);
    }

    // 9. A long.MaxValue cost is refused without wrapping.
    [Fact]
    public void AHugeCostIsRefusedWithoutWrapping()
    {
        using ShrinkBudget b = ShrinkBudget.EnterChild(1000, 100);
        Assert.True(b.Charge(60));
        Assert.False(b.Charge(long.MaxValue));
        Assert.False(b.ChargeCopy(long.MaxValue));
        Assert.Equal(60, b.Work);
    }

    // 10. Concurrent charging from N threads never exceeds the cap.
    [Fact]
    public void ConcurrentChargingNeverExceedsTheCap()
    {
        using ShrinkBudget b = ShrinkBudget.EnterChild(1000, 5000);
        long ok = 0;
        List<Thread> threads = new();
        for (int i = 0; i < 16; i++)
        {
            Thread t = new(() =>
            {
                for (int k = 0; k < 1000; k++)
                {
                    if (b.Charge(1))
                    {
                        Interlocked.Increment(ref ok);
                    }
                }
            });
            threads.Add(t);
            t.Start();
        }

        threads.ForEach(t => t.Join());
        Assert.Equal(5000, ok);
        Assert.Equal(5000, b.Work);
        Assert.Equal(5000, b.Attempts);
    }

    // 11. The ambient budget is restored after a property or shrinker throws; nesting beyond the cap joins.
    [Fact]
    public void TheAmbientBudgetIsRestoredAfterThrowsAndDeepNestingJoins()
    {
        Gen<BigInteger> thrower = Gen<BigInteger>.Of(_ => BigInteger.Zero, (v, f, s) => throw new InvalidOperationException("shrinker boom"));
        Assert.Throws<InvalidOperationException>(() => thrower.Shrink(BigInteger.Zero, _ => true, 5));
        Assert.Null(ShrinkBudget.Current);
        Assert.Throws<InvalidOperationException>(
            () => Gens.Integer(Big(0), Big(10)).Shrink(Big(5), _ => throw new InvalidOperationException("property boom"), 5));
        Assert.Null(ShrinkBudget.Current);

        List<ShrinkBudget> opened = new();
        for (int i = 0; i < ShrinkBudget.MaxDepth + 6; i++)
        {
            opened.Add(ShrinkBudget.EnterChild(10));
        }

        for (int i = 0; i < opened.Count; i++)
        {
            Assert.Equal(i >= ShrinkBudget.MaxDepth, opened[i].IsJoined);
        }

        for (int i = opened.Count - 1; i >= 0; i--)
        {
            opened[i].Dispose();
        }

        Assert.Null(ShrinkBudget.Current);
    }

    [Fact]
    public void EnterCheckAlsoJoinsBeyondTheDepthCap()
    {
        List<ShrinkBudget> opened = new();
        for (int i = 0; i < ShrinkBudget.MaxDepth + 3; i++)
        {
            opened.Add(ShrinkBudget.EnterCheck(5, 5));
        }

        Assert.False(opened[0].IsNested);
        Assert.True(opened[1].IsNested);
        Assert.True(opened[ShrinkBudget.MaxDepth + 2].IsJoined);
        for (int i = opened.Count - 1; i >= 0; i--)
        {
            opened[i].Dispose();
        }

        Assert.Null(ShrinkBudget.Current);
    }

    // Sampling a large collection inside a nested check is work.
    [Fact]
    public void ANestedCheckSamplingLargeListsIsChargedAsWork()
    {
        List<Verdict> inner = new();
        RootTotals(
            10_000,
            () => Gens.Integer(Big(0), Big(1000)).Shrink(
                Big(900),
                x =>
                {
                    inner.Add(Runner.Check(1, Gens.ListOf(Gens.U8(), 2000, 2000), _ => true, new Options(50, 10, CiMode.Off)));
                    return x >= 10;
                },
                1000));
        Assert.NotEmpty(inner);
        Assert.Contains(inner, v => v.FailureId == FailureIds.Exhausted);
    }

    // Generation work counts one level deeper (a list of lists).
    [Fact]
    public void SampledListsOfListsAreChargedByTheirContents()
    {
        List<Verdict> inner = new();
        RootTotals(
            20_000,
            () => Gens.Integer(Big(0), Big(1000)).Shrink(
                Big(900),
                x =>
                {
                    inner.Add(Runner.Check(1, Gens.ListOf(Gens.ListOf(Gens.U8(), 100, 100), 100, 100), _ => true, new Options(50, 10, CiMode.Off)));
                    return x >= 10;
                },
                1000));

        // 100 + 100 x 100 = 10,100 per case: the second case no longer fits in 20,000.
        Assert.Equal(FailureIds.Exhausted, inner[0].FailureId);
    }

    [Fact]
    public void MaxStepsIsValidatedAndZeroStepsReturnsTheInputUncopied()
    {
        Gen<List<byte>> g = Lists(5);
        Assert.Throws<ArgumentOutOfRangeException>(() => g.Shrink(new List<byte>(), _ => true, -1));
        Assert.Throws<ArgumentOutOfRangeException>(() => g.ShrinkNested(new List<byte>(), _ => true, -1));
        List<int> input = new() { 1, 2, 3 };
        Assert.Same(input, Shrink.ShrinkSequence(input, _ => true, 0, 0, null).Value);
    }

    // When the chain refuses the re-verification charge, the shrunk value is not adopted.
    [Fact]
    public void ARefusedReVerificationReportsTheOriginalWitnessWithZeroSteps()
    {
        using (ShrinkBudget root = ShrinkBudget.EnterChild(0)) // 64 attempts
        {
            // A shrinker that finds a smaller failing value but spends the whole chain first.
            Shrinker<int> spender = (value, falsifies, steps) =>
            {
                while (ShrinkBudget.Current!.Charge(1))
                {
                }

                return new Shrunk<int>(3, 1);
            };
            Verdict v = Runner.Check(1, Gens.Constant(100), _ => false, new Options(1, 5, CiMode.Off), spender);
            Assert.Equal(FailureIds.Counterexample, v.FailureId);
            Assert.Equal(100, v.Minimal);
            Assert.Equal(0, v.ShrinkSteps);
        }

        // With room, the same shrinker's result IS adopted (3 still fails the property).
        Shrinker<int> plain = (value, falsifies, steps) => new Shrunk<int>(3, 1);
        Verdict ok = Runner.Check(1, Gens.Constant(100), _ => false, new Options(1, 5, CiMode.Off), plain);
        Assert.Equal(3, ok.Minimal);
        Assert.Equal(1, ok.ShrinkSteps);
    }

    // A top-level check whose nested check was starved is exhausted, never passed.
    [Fact]
    public void AStarvedNestedCheckNeverLetsTheTopLevelCheckPass()
    {
        Verdict v = Runner.Check(
            1,
            Gens.Integer(Big(0), Big(9)),
            _ =>
            {
                Runner.Check(2, Gens.Integer(Big(0), Big(9)), _ => true, new Options(5000, 0, CiMode.Off));
                return true; // the property itself never fails
            },
            new Options(3, 0, CiMode.Off)); // root cap: 3 + 64 attempts
        Assert.False(v.Passed);
        Assert.Equal(FailureIds.Exhausted, v.FailureId);

        // Without starvation the same shape passes.
        Verdict ok = Runner.Check(
            1,
            Gens.Integer(Big(0), Big(9)),
            _ =>
            {
                Runner.Check(2, Gens.Integer(Big(0), Big(9)), _ => true, new Options(5, 0, CiMode.Off));
                return true;
            },
            new Options(3, 0, CiMode.Off));
        Assert.True(ok.Passed);
    }

    // Re-entrancy: the inner verdict equals the standalone verdict while the outer budget has room.
    [Fact]
    public void AReentrantCheckMatchesAStandaloneRunWhileTheOuterBudgetHasRoom()
    {
        Func<Verdict> innerRun = () => Runner.Check(
            5, Gens.ListOf(Gens.U8(), 0, 50), l => l.Sum(b => b) <= 20, new Options(200, 1000, CiMode.On));
        Verdict standalone = innerRun();
        List<Verdict> inner = new();
        Options outer = new(100, 10_000, CiMode.On);
        Verdict expected = Runner.Check(9, Gens.Integer(Big(0), Big(1000)), x => x < 500, outer);
        Verdict actual = Runner.Check(9, Gens.Integer(Big(0), Big(1000)), x => { inner.Add(innerRun()); return x < 500; }, outer);
        Assert.NotEmpty(inner);
        Assert.All(inner, v => Assert.Equal(standalone, v));
        Assert.Equal(expected, actual);
    }

    // A hostile collection cannot break the nested work estimate, and an unrenderable witness still gets a verdict.
    private sealed class HostileList : ICollection<int>, ICollection
    {
        public int Count => throw new InvalidOperationException("hostile count");

        public bool IsReadOnly => true;

        public bool IsSynchronized => false;

        public object SyncRoot => this;

        public void Add(int item) { }

        public void Clear() { }

        public bool Contains(int item) => false;

        public void CopyTo(int[] array, int arrayIndex) { }

        public void CopyTo(Array array, int index) { }

        public IEnumerator<int> GetEnumerator() => throw new InvalidOperationException("hostile enumerator");

        public bool Remove(int item) => false;

        IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
    }

    [Fact]
    public void AHostileCollectionStillYieldsAVerdictTopLevelAndNested()
    {
        HostileList bad = new();
        Verdict v = Runner.Check(1, Gens.Constant(bad), _ => false, new Options(1, 10, CiMode.Off));
        Assert.Equal(FailureIds.Counterexample, v.FailureId);
        Assert.Contains(Canon.Unrenderable, v.Message);

        Verdict? nested = null;
        RootTotals(
            1_000_000,
            () => Gens.Integer(Big(0), Big(1000)).Shrink(
                Big(900),
                x =>
                {
                    nested = Runner.Check(1, Gens.Constant(bad), _ => false, new Options(1, 10, CiMode.Off));
                    return x >= 10;
                },
                5));
        Assert.Equal(FailureIds.Counterexample, nested!.FailureId);
    }

    // Per-pass copies of a large list are skipped once the budget is spent (tuple passes).
    [Fact]
    public void ABudgetSpentInsideTheRunnerStopsEveryNestedShrinkerFromCopying()
    {
        long touched = 0;
        Gen<List<byte>> g = Gens.ListOf(Gens.U8(), 0, 100_000);
        ShrinkBudget.DefaultMaxWork = 1_000_000;
        Verdict v = Runner.Check(
            1,
            Gens.Constant(new List<byte>(new byte[100_000])),
            l => { touched += l.Count; return l.Count != 100_000; },
            new Options(1, 1000, CiMode.Off),
            (value, falsifies, steps) => g.ShrinkNested(value, falsifies, steps));
        Assert.Equal(FailureIds.Counterexample, v.FailureId);
        Assert.True(touched <= 1_000_000 + 300_000, "elements handed to the property " + touched);
    }
}
