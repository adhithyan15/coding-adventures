using CodingAdventures.TestBootstrap.Fuzz;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>
/// A Handle serves exactly one execution (security review: a handle reused across runs could be
/// touched by an async-void continuation or a spawned task while the NEXT run was in progress).
/// </summary>
public sealed class FuzzHandleLifetimeTests
{
    private static FuzzOptions Opts(int iterations) => FuzzOptions.Defaults() with { MaxIterations = iterations, Seed = 1 };

    private static Handle Fresh(int steps = 5, int mem = 100) => new(steps, mem) { Cov = new Bitmap() };

    [Fact]
    public void AnAsyncVoidTargetThatTicksPastTheBudgetAfterAnAwaitIsNotAPass()
    {
        using ManualResetEventSlim release = new(false);
        using ManualResetEventSlim finished = new(false);
        Target target = (d, h) => Go(h, release, finished);
        Handle handle = Fresh(1);
        Outcome o = FuzzEngine.RunTarget(target, new byte[] { 1 }, handle);
        Assert.Equal(OutcomeKind.Crash, o.Kind);
        Assert.Equal("async-void-pending", o.CrashKind);
        release.Set(); // let the stale continuation run: it must be inert, not fatal
        Assert.True(finished.Wait(10_000));
        Assert.True(handle.Retired);
        Assert.Equal(0, handle.Steps); // the stale ticks were refused, not counted
    }

    private static async void Go(Handle h, ManualResetEventSlim release, ManualResetEventSlim finished)
    {
        try
        {
            await Task.Run(() => release.Wait());
            for (int i = 0; i < 100; i++)
            {
                h.Tick(); // past the budget, after the await
            }
        }
        catch (Exception)
        {
            // the retired sentinel: inert
        }
        finally
        {
            finished.Set();
        }
    }

    [Fact]
    public void AnAsyncVoidContinuationNeverAltersTheNextRunsCountersOrCoverage()
    {
        using ManualResetEventSlim release = new(false);
        using ManualResetEventSlim finished = new(false);
        int run = 0;
        Handle? first = null;
        Bitmap cov = new();
        ExecFunc exec = (data, c) =>
        {
            Handle h = new(1000, 1000, new object(), ++run) { Cov = c };
            first ??= h;
            return FuzzEngine.RunTarget(
                (d, hh) =>
                {
                    if (run == 1)
                    {
                        Go2(hh, release, finished);
                    }
                    else
                    {
                        hh.Edge(7);
                        hh.Tick();
                    }
                },
                data,
                h);
        };
        Outcome a = exec(new byte[] { 1 }, cov);
        Assert.Equal("async-void-pending", a.CrashKind);
        Outcome b = exec(new byte[] { 1 }, cov); // the second run starts while the first is still suspended
        Assert.Equal(OutcomeKind.Pass, b.Kind);
        Assert.Equal(new[] { 7 }, cov.Edges());
        release.Set();
        Assert.True(finished.Wait(10_000));
        Assert.Equal(new[] { 7 }, cov.Edges()); // the stale Edge(99) after the await recorded nothing
        Assert.Equal(0, first!.Steps);
    }

    private static async void Go2(Handle h, ManualResetEventSlim release, ManualResetEventSlim finished)
    {
        try
        {
            await Task.Run(() => release.Wait());
            h.Edge(99);
            h.Tick();
        }
        catch (Exception)
        {
            // inert
        }
        finally
        {
            finished.Set();
        }
    }

    [Fact]
    public void ASpawnedTaskThatLaterCallsEdgeOrTickIsInert()
    {
        using ManualResetEventSlim release = new(false);
        Task? spawned = null;
        Bitmap cov = new();
        Handle h = new(10, 10) { Cov = cov };
        Outcome o = FuzzEngine.RunTarget(
            (d, hh) =>
            {
                spawned = Task.Run(() =>
                {
                    release.Wait();
                    hh.Edge(5);
                    hh.Tick();
                });
            },
            Array.Empty<byte>(),
            h);
        Assert.Equal(OutcomeKind.Pass, o.Kind); // a plain Task is not async void: the run itself is fine
        release.Set();
        Assert.Throws<AggregateException>(() => spawned!.Wait(10_000)); // the retired Tick refused
        Assert.Empty(cov.Edges());
        Assert.Equal(0, h.Steps);
    }

    [Fact]
    public void ARetiredHandleRefusesTickAndReserveAndIgnoresEdge()
    {
        Handle h = Fresh();
        h.Edge(1);
        Assert.Equal(new[] { 1 }, h.Cov!.Edges());
        Bitmap cov = h.Cov!;
        h.Retire();
        h.Edge(2);
        Assert.Equal(new[] { 1 }, cov.Edges());
        Assert.Throws<Handle.BudgetExceeded>(h.Tick);
        Assert.Throws<Handle.BudgetExceeded>(() => h.Reserve(1));
        Assert.Throws<Handle.BudgetExceeded>(() => h.Reserve(0));
    }

    [Fact]
    public void EveryExecutionOfACampaignGetsItsOwnHandle()
    {
        List<Handle> seen = new();
        FuzzReport r = FuzzEngine.Run(new List<byte[]> { new byte[] { 0 } }, Opts(20), (d, h) => seen.Add(h));
        Assert.True(r.Iterations == 20);
        Assert.Equal(seen.Count, seen.Distinct().Count());
        Assert.All(seen, h => Assert.True(h.Retired));
    }

    [Fact]
    public void AFaultingAsyncVoidTargetIsACrashAndNeverTerminatesTheHost()
    {
        Outcome o = FuzzEngine.RunTarget((d, h) => Boom(), Array.Empty<byte>(), Fresh());
        Assert.Equal(OutcomeKind.Crash, o.Kind);
        Assert.Equal("async-void-exception", o.CrashKind);
    }

    private static async void Boom()
    {
        await Task.CompletedTask;
        throw new InvalidOperationException("faulted");
    }

    [Fact]
    public void ACompletedAsyncVoidTargetIsAPass()
    {
        Assert.Equal(OutcomeKind.Pass, FuzzEngine.RunTarget((d, h) => Quiet(h), Array.Empty<byte>(), Fresh()).Kind);
    }

    private static async void Quiet(Handle h)
    {
        await Task.CompletedTask;
        h.Tick();
    }

    [Fact]
    public void ASentinelRethrownInALaterRunIsStaleNotThatRunsBudgetBreach()
    {
        object campaign = new();
        Handle old = new(1, 1, campaign, 1);
        Handle.BudgetExceeded caught = new(old, Handle.Budget.Steps);
        Handle later = new(1, 1, campaign, 2);
        Outcome o = FuzzEngine.Classify(caught, later);
        Assert.Equal(OutcomeKind.Crash, o.Kind);
        Assert.Equal(FuzzEngine.StaleSentinel, o.CrashKind);
        Assert.Equal(OutcomeKind.Timeout, FuzzEngine.Classify(new Handle.BudgetExceeded(later, Handle.Budget.Steps), later).Kind);
        // A retired-handle sentinel is never a budget verdict either.
        old.Retire();
        Assert.Equal(FuzzEngine.StaleSentinel, FuzzEngine.Classify(new Handle.BudgetExceeded(old, Handle.Budget.Retired), old).CrashKind);
        // End to end: a target that stashes a sentinel and rethrows it in the next run is a crash, not a timeout.
        Handle.BudgetExceeded? stash = null;
        FuzzReport r = FuzzEngine.Run(
            new List<byte[]> { new byte[] { 1 } },
            Opts(5) with { StepBudget = 0, MinimizeBudget = 0 },
            (d, h) =>
            {
                if (stash != null)
                {
                    throw stash;
                }

                try
                {
                    h.Tick();
                }
                catch (Handle.BudgetExceeded b)
                {
                    stash = b;
                    throw;
                }
            });
        Assert.True(r.Timeouts >= 1);
    }

    [Fact]
    public void OptionsToStringDoesNotPrintTheSeedAndCollectionsAreReadOnly()
    {
        string s = (FuzzOptions.Defaults() with { Seed = 123456789 }).ToString();
        Assert.DoesNotContain("123456789", s);
        Assert.DoesNotContain("Seed", s);
        FuzzReport r = FuzzEngine.Run(new List<byte[]> { new byte[] { 1 } }, Opts(3) with { MinimizeBudget = 0 }, (d, h) => throw new InvalidOperationException());
        Assert.IsNotType<CrashRecord[]>(r.Crashes);
        Assert.IsNotType<string[]>(r.Crashes[0].Frames);
        Assert.Throws<NotSupportedException>(() => ((IList<string>)r.Crashes[0].Frames).Add("x"));
        Assert.IsNotType<string[]>(Outcome.Crash("k", new[] { "f" }).Frames);
    }

    [Fact]
    public void AnAsyncVoidFaultOnAPoolThreadWhileTheTargetBlocksIsACrash()
    {
        Outcome o = FuzzEngine.RunTarget(
            (d, h) =>
            {
                DelayThenThrow();
                Thread.Sleep(100); // the fault happens on a pool thread while the target is still running
            },
            Array.Empty<byte>(),
            Fresh());
        Assert.Equal(OutcomeKind.Crash, o.Kind);
        Assert.Equal("async-void-exception", o.CrashKind);
    }

    [Fact]
    public void AFaultThatLandsAfterTheTargetReturnedIsStillCaughtWithinTheGrace()
    {
        Outcome o = FuzzEngine.RunTarget((d, h) => DelayThenThrow(), Array.Empty<byte>(), Fresh());
        Assert.Equal(OutcomeKind.Crash, o.Kind);
        Assert.Contains(o.CrashKind, new[] { "async-void-exception", "async-void-pending" });
    }

    private static async void DelayThenThrow()
    {
        await Task.Delay(1);
        throw new InvalidOperationException("late fault");
    }

    [Fact]
    public void AnAsyncTaskTargetThatYieldsAndCompletesIsAPass()
    {
        for (int i = 0; i < 50; i++)
        {
            Outcome o = FuzzEngine.RunTarget((d, h) => YieldingWork(h).GetAwaiter().GetResult(), Array.Empty<byte>(), Fresh(100));
            Assert.Equal(OutcomeKind.Pass, o.Kind);
        }
    }

    private static async Task YieldingWork(Handle h)
    {
        await Task.Yield();
        h.Tick();
        await Task.Delay(1);
        h.Tick();
    }

    [Fact]
    public void ARetiredHandleCannotBeChangedByARacingStaleThread()
    {
        for (int round = 0; round < 50; round++)
        {
            Handle h = Fresh(int.MaxValue, int.MaxValue);
            using ManualResetEventSlim go = new(false);
            Task stale = Task.Run(() =>
            {
                go.Wait();
                try
                {
                    for (int i = 0; i < 100_000; i++)
                    {
                        h.Tick();
                        h.Reserve(1);
                        h.Edge(3);
                    }
                }
                catch (Handle.BudgetExceeded)
                {
                    // refused after Retire
                }
            });
            go.Set();
            h.Retire();
            int steps = h.Steps;
            long reserved = h.Reserved;
            Assert.Null(h.Exhausted);
            Assert.True(stale.Wait(10_000)); // finished, its later calls refused
            Assert.Equal(steps, h.Steps);
            Assert.Equal(reserved, h.Reserved);
        }
    }
}
