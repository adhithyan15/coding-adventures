using System.Runtime.CompilerServices;
using CodingAdventures.TestBootstrap.Fuzz;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>The executor, minimiser and campaign (VER06h sections 5 and 6).</summary>
public sealed class FuzzEngineTests
{
    private static FuzzOptions Opts(int iterations, ulong seed) => FuzzOptions.Defaults() with { MaxIterations = iterations, Seed = seed };

    private static List<byte[]> Seeds(params byte[][] s) => s.ToList();

    private static FuzzReport Run(List<byte[]> corpus, FuzzOptions o, Target t) => FuzzEngine.Run(corpus, o, t);

    // ---- the executor ----------------------------------------------------------------

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static void Boom(byte[] d, Handle h) => throw new InvalidOperationException("secret " + string.Join(",", d));

    private static Outcome Exec(Target t, byte[]? input = null)
    {
        Handle h = new(1000, 1 << 20);
        h.Cov = new Bitmap();
        return FuzzEngine.RunTarget(t, input ?? new byte[] { 1, 2, 3 }, h);
    }

    [Fact]
    public void AThrownExceptionIsACrashWithKindAndInnermostFirstFrames()
    {
        Outcome o = Exec((d, h) => Boom(d, h));
        Assert.Equal(OutcomeKind.Crash, o.Kind);
        Assert.Equal("System.InvalidOperationException", o.CrashKind);
        Assert.Equal(new[] { "FuzzEngineTests.Boom", "FuzzEngineTests.AThrownExceptionIsACrashWithKindAndInnermostFirstFrames" }, o.Frames);
        foreach (string f in o.Frames)
        {
            Assert.DoesNotContain(':', f);
        }
    }

    [Fact]
    public void EveryKindOfExceptionIsACrashNeverAnEscape()
    {
        Assert.Equal("System.Exception", Exec((d, h) => throw new Exception("x")).CrashKind);
        Assert.Equal("System.IndexOutOfRangeException", Exec((d, h) => { _ = d[99]; }).CrashKind);
        Assert.Equal("System.NullReferenceException", Exec((d, h) => { object? o = null; _ = o!.GetHashCode(); }).CrashKind);
        Assert.Equal("System.InvalidCastException", Exec((d, h) => { object o = "s"; _ = (int)o; }).CrashKind);
        Assert.Equal("System.DivideByZeroException", Exec((d, h) => { int z = d.Length - 3; _ = 1 / z; }).CrashKind);
        Assert.Equal("System.ArgumentException", Exec((d, h) => throw new ArgumentNullException("p")).CrashKind.Replace("ArgumentNullException", "ArgumentException"));
        Assert.Equal(OutcomeKind.Pass, Exec((d, h) => { }).Kind);
    }

    [Fact]
    public void TheMessageIsNeverPartOfTheSignatureOrAnyOutput()
    {
        Target t = (d, h) => Boom(d, h);
        Outcome a = Exec(t, new byte[] { 1 });
        Outcome b = Exec(t, new byte[] { 9, 9, 9 });
        Assert.Equal(FuzzSignature.CrashSignature(a.CrashKind, a.Frames), FuzzSignature.CrashSignature(b.CrashKind, b.Frames));
        Assert.DoesNotContain("secret", a.CrashKind + string.Join("|", a.Frames));
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static int Recurse(int depth) => depth == 0 ? throw new InvalidOperationException() : Recurse(depth - 1) + 1;

    [Fact]
    public void ADeepRecursionCrashCollapsesItsFramesToOne()
    {
        Outcome a = Exec((d, h) => Recurse(2000));
        Outcome b = Exec((d, h) => Recurse(1500));
        Assert.Equal("FuzzEngineTests.Recurse", a.Frames[0]);
        Assert.Equal(2, a.Frames.Count); // one collapsed Recurse frame, then the target lambda
        Assert.True(a.Frames.Count <= 64);
        Assert.Equal(FuzzSignature.CrashSignature(a.CrashKind, a.Frames), FuzzSignature.CrashSignature(b.CrashKind, b.Frames));
    }

    private static int s_virtualCalls;

    private sealed class HostileException : Exception
    {
        public override string? StackTrace
        {
            get
            {
                s_virtualCalls++;
                throw new InvalidOperationException("hostile StackTrace");
            }
        }

        public override string Message
        {
            get
            {
                s_virtualCalls++;
                throw new InvalidOperationException("hostile Message");
            }
        }

        public override string ToString()
        {
            s_virtualCalls++;
            throw new InvalidOperationException("hostile ToString");
        }

        public override System.Collections.IDictionary Data
        {
            get
            {
                s_virtualCalls++;
                throw new InvalidOperationException("hostile Data");
            }
        }
    }

    [Fact]
    public void AHostileExceptionTypeIsACrashAndNoVirtualMemberIsEverCalled()
    {
        s_virtualCalls = 0;
        Outcome o = Exec((d, h) => throw new HostileException());
        Assert.Equal(OutcomeKind.Crash, o.Kind);
        Assert.StartsWith("CodingAdventures.TestBootstrap.Tests.FuzzEngineTests_", o.CrashKind);
        Assert.Equal(64, o.CrashKind.Length); // the kind is capped
        Assert.Equal(new[] { "FuzzEngineTests.AHostileExceptionTypeIsACrashAndNoVirtualMemberIsEverCalled" }, o.Frames);
        Assert.Equal(0, s_virtualCalls);
        FuzzReport report = Run(Seeds(new byte[] { 1 }), Opts(30, 1) with { MinimizeBudget = 10 }, (d, h) => throw new HostileException());
        Assert.Single(report.Crashes);
        Assert.Equal(0, s_virtualCalls);
        Assert.DoesNotContain("hostile", report.ToString() + report.Crashes[0]);
    }

    [Fact]
    public void AFailingFrameWalkNeverEndsTheCampaign()
    {
        Handle h = new(10, 10);
        Outcome o = FuzzEngine.Classify(new InvalidOperationException(), h, _ => throw new InvalidProgramException("walk failed"));
        Assert.Equal(OutcomeKind.Crash, o.Kind);
        Assert.Empty(o.Frames);
        // An out-of-memory while walking the stack is a counted OOM, not an escape.
        Assert.Equal(OutcomeKind.Oom, FuzzEngine.Classify(new InvalidOperationException(), h, _ => throw new OutOfMemoryException()).Kind);
    }

    [Fact]
    public void ARealOutOfMemoryExceptionIsAnOomOutcomeAndNothingIsRendered()
    {
        Outcome o = Exec((d, h) => throw new OutOfMemoryException("heap " + string.Join(",", d)));
        Assert.Equal(OutcomeKind.Oom, o.Kind);
        Assert.Equal(string.Empty, o.CrashKind);
        Assert.Empty(o.Frames);
        FuzzReport r = Run(Seeds(new byte[] { 1 }), Opts(30, 1), (d, h) => throw new OutOfMemoryException());
        Assert.Equal(1, r.Ooms);
        // ... unless a budget broke first.
        Handle h2 = new(0, 10);
        Outcome t = FuzzEngine.RunTarget(
            (d, hh) =>
            {
                try
                {
                    hh.Tick();
                }
                catch (Exception)
                {
                    // swallowed
                }

                throw new OutOfMemoryException();
            },
            Array.Empty<byte>(),
            h2);
        Assert.Equal(OutcomeKind.Timeout, t.Kind);
    }

    // ---- sticky exhaustion ---------------------------------------------------------------

    private static Outcome RunWith(int steps, int mem, Target t)
    {
        Handle h = new(steps, mem);
        h.Cov = new Bitmap();
        return FuzzEngine.RunTarget(t, Array.Empty<byte>(), h);
    }

    private static void Swallow(Action a)
    {
        try
        {
            a();
        }
        catch (Exception)
        {
            // keeps going: .NET cannot make the sentinel un-catchable
        }
    }

    [Fact]
    public void ExhaustionIsStickyAndSharedBetweenTickAndReserve()
    {
        // swallow a MEMORY breach, then call reserve again: stopped, reported OOM.
        Assert.Equal(OutcomeKind.Oom, RunWith(1000, 100, (d, h) => { Swallow(() => h.Reserve(1000)); h.Reserve(1); }).Kind);
        // swallow a MEMORY breach, then tick: stopped as the memory breach.
        Assert.Equal(OutcomeKind.Oom, RunWith(1000, 100, (d, h) => { Swallow(() => h.Reserve(1000)); h.Tick(); }).Kind);
        // swallow a STEPS breach, then reserve: a timeout.
        Assert.Equal(OutcomeKind.Timeout, RunWith(1, 100, (d, h) => { h.Tick(); Swallow(h.Tick); h.Reserve(1); }).Kind);
        // The first breach wins: a steps breach then a memory breach stays a timeout.
        Assert.Equal(OutcomeKind.Timeout, RunWith(0, 100, (d, h) => { Swallow(h.Tick); h.Reserve(1000); }).Kind);
        Assert.Equal(OutcomeKind.Oom, RunWith(0, 100, (d, h) => { Swallow(() => h.Reserve(1000)); Swallow(h.Tick); }).Kind);
    }

    [Fact]
    public void SwallowThenReturnAndSwallowThenThrowAreStillTimeoutOrOom()
    {
        Target swallowSteps = (d, h) => { Swallow(() => { h.Tick(); h.Tick(); }); };
        Target swallowMem = (d, h) => { Swallow(() => h.Reserve(1000)); };
        Target stepsThenThrow = (d, h) =>
        {
            try
            {
                h.Tick();
                h.Tick();
            }
            catch (Exception)
            {
                throw new InvalidOperationException("ordinary");
            }
        };
        Target memThenThrow = (d, h) =>
        {
            try
            {
                h.Reserve(1000);
            }
            catch (Exception)
            {
                throw new InvalidOperationException("ordinary");
            }
        };
        foreach (Target t in new[] { swallowSteps, stepsThenThrow })
        {
            Assert.Equal(OutcomeKind.Timeout, RunWith(1, 100, t).Kind);
        }

        foreach (Target t in new[] { swallowMem, memThenThrow })
        {
            Assert.Equal(OutcomeKind.Oom, RunWith(10, 100, t).Kind);
        }

        FuzzReport a = Run(Seeds(new byte[] { 1 }), Opts(5, 1) with { StepBudget = 1 }, stepsThenThrow);
        Assert.True(a.Timeouts > 0);
        Assert.Empty(a.Crashes);
        FuzzReport b = Run(Seeds(new byte[] { 1 }), Opts(5, 1) with { MemBudget = 10 }, memThenThrow);
        Assert.True(b.Ooms > 0);
        Assert.Empty(b.Crashes);
        Assert.Equal(OutcomeKind.Crash, RunWith(10, 100, (d, h) => throw new InvalidOperationException()).Kind);
    }

    [Fact]
    public void ASwallowingLoopOnlyEverGetsSentinelsBack()
    {
        int swallowed = 0;
        Outcome o = Exec((d, h) =>
        {
            for (int i = 0; i < 5000; i++)
            {
                try
                {
                    h.Tick();
                }
                catch (Exception)
                {
                    swallowed++;
                }
            }
        });
        Assert.Equal(OutcomeKind.Timeout, o.Kind);
        Assert.Equal(4000, swallowed); // 1000 ticks pass, every later one is refused
        Handle strict = new(10, 100);
        strict.Cov = new Bitmap();
        Assert.Equal(OutcomeKind.Timeout, FuzzEngine.RunTarget((d, h) => { for (; ; ) { h.Tick(); } }, Array.Empty<byte>(), strict).Kind);
        Assert.Equal(10, strict.Steps);
        Assert.Equal(OutcomeKind.Timeout, RunWith(0, 0, (d, h) => h.Tick()).Kind);
    }

    [Fact]
    public void EachExecutionOfACampaignStartsWithFreshBudgets()
    {
        FuzzReport r = Run(
            Seeds(new byte[] { 0 }),
            Opts(50, 1) with { MemBudget = 10 },
            (d, h) =>
            {
                if (d.Length > 0 && d[0] != 0)
                {
                    h.Reserve(100);
                }
                else
                {
                    h.Reserve(5);
                    h.Tick();
                }
            });
        Assert.True(r.Ooms > 0);
        Assert.True(r.AdmittedSeeds >= 1, "the clean seed ran with a fresh budget");
    }

    [Fact]
    public void ReserveAccountsBytesWithoutOverflow()
    {
        Handle h = new(10, 100);
        h.Reserve(60);
        h.Reserve(40);
        Assert.Equal(100, h.Reserved);
        Assert.Throws<Handle.BudgetExceeded>(() => h.Reserve(1));
        Assert.Equal(100, h.Reserved);
        Handle z = new(10, 100);
        z.Reserve(0);
        z.Reserve(-5);
        Assert.Equal(0, z.Reserved);
        Handle g = new(10, 100);
        Assert.Throws<Handle.BudgetExceeded>(() => g.Reserve(long.MaxValue));
        Assert.Equal(OutcomeKind.Oom, RunWith(10, 100, (d, hh) => hh.Reserve(101)).Kind);
        Assert.Equal(OutcomeKind.Pass, RunWith(10, 100, (d, hh) => hh.Reserve(100)).Kind);
    }

    [Fact]
    public void TickIsADeterministicStepBudget()
    {
        Handle h = new(3, 100);
        h.Tick();
        h.Tick();
        h.Tick(); // the third is allowed
        Assert.Throws<Handle.BudgetExceeded>(h.Tick);
        Assert.Throws<Handle.BudgetExceeded>(h.Tick); // and every later one
        Assert.Throws<Handle.BudgetExceeded>(() => h.Reserve(1)); // shared
    }

    [Fact]
    public void AForeignBudgetSentinelIsRethrownToItsOwner()
    {
        Handle mine = new(1, 1);
        Handle other = new(1, 1);
        Handle.BudgetExceeded foreign = new(other, Handle.Budget.Steps);
        Assert.Same(foreign, Assert.Throws<Handle.BudgetExceeded>(() => FuzzEngine.Classify(foreign, mine)));
        Assert.Equal(OutcomeKind.Timeout, FuzzEngine.Classify(new Handle.BudgetExceeded(mine, Handle.Budget.Steps), mine).Kind);
        Assert.Equal(OutcomeKind.Oom, FuzzEngine.Classify(new Handle.BudgetExceeded(mine, Handle.Budget.Memory), mine).Kind);
    }

    [Fact]
    public void NestedCampaignsDoNotClaimEachOthersTimeouts()
    {
        FuzzOptions inner = Opts(5, 1) with { StepBudget = 1_000_000 };
        FuzzOptions outer = Opts(5, 1) with { StepBudget = 5 };
        FuzzReport r = Run(
            Seeds(new byte[] { 1 }),
            outer,
            (d, oh) =>
            {
                Run(Seeds(new byte[] { 1 }), inner, (dd, ih) => { });
                for (int i = 0; i < 100; i++)
                {
                    oh.Tick(); // the OUTER budget runs out here
                }
            });
        Assert.True(r.Timeouts > 0);
        Assert.Empty(r.Crashes);
    }

    [Fact]
    public void TheTargetReceivesItsOwnCopyOfTheInput()
    {
        byte[] original = { 1, 2, 3 };
        FuzzReport r = Run(Seeds(original), Opts(20, 1), (d, h) => Array.Fill(d, (byte)9));
        Assert.Equal(new byte[] { 1, 2, 3 }, original);
        Assert.Equal(new byte[] { 1, 2, 3 }, r.GetCorpus()[0]);
    }

    [Fact]
    public void EdgeIdIsAStableFnvOfTheLabel()
    {
        Assert.Equal(Handle.EdgeId("a"), Handle.EdgeId("a"));
        Assert.NotEqual(Handle.EdgeId("a"), Handle.EdgeId("b"));
        Assert.Equal(0xaf63dc4c8601ec8cUL, Handle.EdgeId("a"));
        Handle h = new(1, 1); // an edge before the campaign wired a map is ignored, not a crash
        h.Edge(5);
    }

    // ---- frames ----------------------------------------------------------------------------

    [Fact]
    public void FramesOfInspectsAtMostMaxWalkFramesAndKeepsSixtyFour()
    {
        int calls = 0;
        List<string> frames = FuzzEngine.FramesOf(
            10_000,
            i =>
            {
                calls++;
                return new FuzzEngine.RawFrame(new[] { "C" + i }, "M", false, false);
            });
        Assert.Equal(64, frames.Count);
        Assert.Equal("C0.M", frames[0]);
        calls = 0;
        List<string> same = FuzzEngine.FramesOf(
            1_000_000,
            _ =>
            {
                calls++;
                return new FuzzEngine.RawFrame(new[] { "R" }, "M", false, false);
            });
        Assert.Equal(new[] { "R.M" }, same);
        Assert.Equal(FuzzEngine.MaxWalk, calls);
    }

    [Fact]
    public void PlumbingIsDroppedAndTheCutIsAtTheExecutor()
    {
        FuzzEngine.RawFrame?[] mixed =
        {
            new FuzzEngine.RawFrame(new[] { "List`1" }, "get_Item", true, false),
            null,
            new FuzzEngine.RawFrame(new[] { "Handle" }, "Tick", true, false),
            new FuzzEngine.RawFrame(new[] { "Target" }, "Go", false, false),
            new FuzzEngine.RawFrame(new[] { "FuzzEngine" }, "Classify", true, false),
            new FuzzEngine.RawFrame(new[] { "Other" }, "Go", false, false),
            new FuzzEngine.RawFrame(new[] { "FuzzEngine" }, "RunTarget", true, true),
            new FuzzEngine.RawFrame(new[] { "Above" }, "Never", false, false),
        };
        Assert.Equal(new[] { "Target.Go", "Other.Go" }, FuzzEngine.FramesOf(mixed.Length, i => mixed[i]));
    }

    [Fact]
    public void RealFramesDropTheBclAndTheEngine()
    {
        Outcome o = Exec((d, h) => ((Action)(() => throw new InvalidOperationException())).Invoke());
        Assert.DoesNotContain(o.Frames, f => f.StartsWith("Handle.", StringComparison.Ordinal) || f.StartsWith("FuzzEngine.", StringComparison.Ordinal));
        Assert.DoesNotContain(o.Frames, f => f.StartsWith("Action.", StringComparison.Ordinal));
        Outcome tick = RunWith(0, 0, (d, h) => throw new IndexOutOfRangeException());
        Assert.Equal(OutcomeKind.Crash, tick.Kind);
        Outcome list = Exec((d, h) => _ = new List<int>()[3]);
        Assert.Equal("FuzzEngineTests.RealFramesDropTheBclAndTheEngine", list.Frames[0]);
    }

    [Fact]
    public void DifferentFailuresGetDifferentSignatures()
    {
        Outcome a = Exec((d, h) => throw new InvalidOperationException());
        Outcome b = Exec((d, h) => throw new ArgumentException());
        Outcome c = Exec((d, h) => Boom(d, h));
        Assert.NotEqual(FuzzSignature.CrashSignature(a.CrashKind, a.Frames), FuzzSignature.CrashSignature(b.CrashKind, b.Frames));
        Assert.NotEqual(FuzzSignature.CrashSignature(a.CrashKind, a.Frames), FuzzSignature.CrashSignature(c.CrashKind, c.Frames));
    }

    private static async Task<int> AsyncBoom()
    {
        await Task.Yield();
        throw new InvalidOperationException();
    }

    [Fact]
    public void AnAsyncStateMachineFrameReportsTheUserMethod()
    {
        Outcome o = Exec((d, h) => AsyncBoom().GetAwaiter().GetResult());
        Assert.Contains("FuzzEngineTests.AsyncBoom", o.Frames);
        Assert.DoesNotContain(o.Frames, f => f.Contains("MoveNext", StringComparison.Ordinal));
    }

    // ---- minimisation --------------------------------------------------------------------

    private static Outcome CrashOn(bool crash, string kind) => crash ? Outcome.Crash(kind, new[] { "f" }) : Outcome.Pass();

    [Fact]
    public void TheMinimiserReachesTheEmptyInput()
    {
        string sig = FuzzSignature.CrashSignature("k", new[] { "f" });
        Assert.Empty(FuzzEngine.MinimizeCrashingInput(new byte[] { 7 }, sig, d => CrashOn(true, "k"), 100));
        Assert.Empty(FuzzEngine.MinimizeCrashingInput(Array.Empty<byte>(), sig, d => CrashOn(true, "k"), 0));
        Assert.Empty(FuzzEngine.MinimizeCrashingInput(new byte[40], sig, d => CrashOn(true, "k"), 1000));
    }

    [Fact]
    public void TheMinimiserKeepsTheExactSignatureAndZeroesBytes()
    {
        string sig = FuzzSignature.CrashSignature("k", new[] { "f" });
        Func<byte[], Outcome> test = d => CrashOn(d.Length >= 4 && d[3] == 0x42, "k");
        Assert.Equal(new byte[] { 0, 0, 0, 0x42 }, FuzzEngine.MinimizeCrashingInput(new byte[] { 1, 2, 3, 0x42, 5, 6, 7, 8 }, sig, test, 1000));
    }

    [Fact]
    public void TheMinimiserNeverTradesOneCrashForAnother()
    {
        string sig = FuzzSignature.CrashSignature("k", new[] { "f" });
        Func<byte[], Outcome> test = d => d.Length >= 6 ? CrashOn(true, "k") : (d.Length > 0 ? CrashOn(true, "other") : Outcome.Pass());
        Assert.Equal(6, FuzzEngine.MinimizeCrashingInput(new byte[] { 1, 2, 3, 4, 5, 6 }, sig, test, 1000).Length);
        Assert.Equal(6, FuzzEngine.MinimizeCrashingInput(new byte[6], sig, d => d.Length == 6 ? CrashOn(true, "k") : Outcome.Pass(), 1000).Length);
    }

    [Fact]
    public void EveryCandidateExecutionCountsAgainstTheBudgetRejectedOnesIncluded()
    {
        string sig = FuzzSignature.CrashSignature("k", new[] { "f" });
        int runs = 0;
        Func<byte[], Outcome> never = d =>
        {
            runs++;
            return Outcome.Pass();
        };
        Assert.Equal(500, FuzzEngine.MinimizeCrashingInput(new byte[500], sig, never, 25).Length);
        Assert.Equal(25, runs); // exactly the budget, rejected attempts included
        runs = 0;
        byte[] nonzero = Enumerable.Repeat((byte)1, 500).ToArray();
        FuzzEngine.MinimizedResult m = FuzzEngine.Minimize(nonzero, sig, never, 600);
        Assert.True(runs <= 600);
        Assert.Equal(runs, m.Spent);
        runs = 0;
        Assert.False(FuzzEngine.Minimize(nonzero, sig, never, 30).Finished);
        Assert.Equal(30, runs);
        runs = 0;
        FuzzEngine.MinimizedResult pass2 = FuzzEngine.Minimize(new byte[] { 1, 1 }, sig, never, 3); // runs out during pass 2
        Assert.False(pass2.Finished);
        Assert.Equal(3, runs);
    }

    [Fact]
    public void TheMinimiserSkipsZeroBytesAndValidatesItsArguments()
    {
        string sig = FuzzSignature.CrashSignature("k", new[] { "f" });
        int runs = 0;
        FuzzEngine.Minimize(new byte[3], sig, d => { runs++; return Outcome.Pass(); }, 100);
        Assert.Equal(3, runs); // only pass 1 runs for an all-zero input
        Assert.Throws<ArgumentException>(() => FuzzEngine.MinimizeCrashingInput(new byte[1], sig, d => Outcome.Pass(), -1));
        Assert.Throws<ArgumentException>(() => FuzzEngine.MinimizeCrashingInput(new byte[1], sig, d => Outcome.Pass(), FuzzOptions.MaxMinimizeBudget + 1));
        Assert.Throws<ArgumentNullException>(() => FuzzEngine.MinimizeCrashingInput(new byte[1], sig, null!, 1));
        Assert.Throws<ArgumentException>(() => FuzzEngine.MinimizeCrashingInput(new byte[Mutate.MaxInputBytes + 1], sig, d => Outcome.Pass(), 1));
    }

    // ---- campaigns -------------------------------------------------------------------------

    private static Target Staircase(bool instrumented)
    {
        byte[] want = { 128, 255, 127, 1 }; // boundary values: one step each, but all four at once is hopeless
        return (d, h) =>
        {
            for (int i = 0; i < want.Length; i++)
            {
                if (d.Length <= i || d[i] != want[i])
                {
                    return;
                }

                if (instrumented)
                {
                    h.Edge(Handle.EdgeId("stair" + i));
                }
            }

            throw new InvalidOperationException("cracked");
        };
    }

    [Fact]
    public void CoverageFeedbackCracksAStaircaseBlindMutationDoesNot()
    {
        int[] cracked = new int[2];
        foreach (bool instrumented in new[] { true, false })
        {
            for (ulong seed = 0; seed < 8; seed++)
            {
                FuzzReport r = Run(Seeds(new byte[4]), Opts(8000, seed) with { MinimizeBudget = 0 }, Staircase(instrumented));
                if (r.Crashes.Count > 0)
                {
                    cracked[instrumented ? 0 : 1]++;
                }
            }
        }

        Assert.True(cracked[0] >= 4, "coverage guidance cracked only " + cracked[0] + " of 8 seeds");
        Assert.Equal(0, cracked[1]);
    }

    [Fact]
    public void ACampaignIsDeterministicAndSeedSensitive()
    {
        FuzzReport a = Run(Seeds(new byte[4]), Opts(2000, 3), Staircase(true));
        FuzzReport b = Run(Seeds(new byte[4]), Opts(2000, 3), Staircase(true));
        Assert.Equal(a.ToString(), b.ToString());
        Assert.Equal(a.GetCorpus().Count, b.GetCorpus().Count);
        for (int i = 0; i < a.GetCorpus().Count; i++)
        {
            Assert.Equal(a.GetCorpus()[i], b.GetCorpus()[i]);
        }

        FuzzReport c = Run(Seeds(new byte[4]), Opts(2000, 4), Staircase(true));
        bool differs = c.GetCorpus().Count != a.GetCorpus().Count;
        for (int i = 0; !differs && i < a.GetCorpus().Count; i++)
        {
            differs = !a.GetCorpus()[i].AsSpan().SequenceEqual(c.GetCorpus()[i]);
        }

        Assert.True(differs, "a different seed must explore differently");
    }

    [Fact]
    public void BudgetLengthCapAndEmptyCorpus()
    {
        int longest = 0;
        bool first = true;
        FuzzReport r = Run(
            new List<byte[]>(),
            Opts(500, 9) with { MaxInputLen = 6 },
            (d, h) =>
            {
                if (first)
                {
                    first = false;
                    Assert.Equal(new byte[] { 0 }, d);
                }

                longest = Math.Max(longest, d.Length);
                h.Edge(d.Length > 0 ? d[0] : 1000UL);
            });
        Assert.Equal(500, r.Iterations);
        Assert.True(longest <= 6, "mutated candidates are capped: " + longest);
        Assert.True(r.IsClean);
        int seen = 0; // initial seeds run as given, even above the cap
        Run(Seeds(new byte[100]), Opts(3, 1) with { MaxInputLen = 4 }, (d, h) => seen = Math.Max(seen, d.Length));
        Assert.Equal(100, seen);
        Assert.Equal(20, Run(Seeds(new byte[3]), Opts(20, 1) with { MaxInputLen = 0 }, (d, h) => { }).Iterations);
    }

    [Fact]
    public void CrashingSeedsAreNotMutatedAndNeverClean()
    {
        FuzzReport r = Run(Seeds(new byte[] { 1 }, new byte[] { 2 }), Opts(100, 1), (d, h) => throw new InvalidOperationException());
        Assert.Equal(0, r.Iterations);
        Assert.False(r.IsClean);
        Assert.Single(r.Crashes);
        Assert.Equal(2, r.Crashes[0].Hits);
        foreach (Target t in new Target[] { (d, h) => { for (; ; ) { h.Tick(); } }, (d, h) => h.Reserve(long.MaxValue) })
        {
            FuzzReport f = Run(Seeds(new byte[1]), Opts(100, 1) with { StepBudget = 5 }, t);
            Assert.Equal(0, f.Iterations);
            Assert.False(f.IsClean);
        }
    }

    [Fact]
    public void EveryExecutionLandsInABucketAndIsCleanIsHonest()
    {
        FuzzReport clean = Run(Seeds(new byte[2]), Opts(50, 1), (d, h) => h.Edge(1));
        Assert.True(clean.IsClean);
        Assert.Equal(FuzzReport.TierReported, clean.CoverageTier);
        Assert.False(Run(Seeds(new byte[2]), Opts(0, 1), (d, h) => { }).IsClean);
        Target slow = (d, h) => { if (d.Length != 1 || d[0] != 0) { for (; ; ) { h.Tick(); } } };
        FuzzReport t = Run(Seeds(new byte[] { 0 }), Opts(200, 1) with { StepBudget = 10 }, slow);
        Assert.True(t.Timeouts > 0);
        Assert.False(t.IsClean);
        Target greedy = (d, h) => { if (d.Length != 1 || d[0] != 0) { h.Reserve(1 << 20); } };
        FuzzReport m = Run(Seeds(new byte[] { 0 }), Opts(200, 1) with { MemBudget = 10 }, greedy);
        Assert.True(m.Ooms > 0);
        Assert.False(m.IsClean);
        Target bad = (d, h) => { if (d.Length != 1 || d[0] != 0) { throw new InvalidOperationException(); } };
        FuzzReport c = Run(Seeds(new byte[] { 0 }), Opts(200, 1), bad);
        Assert.NotEmpty(c.Crashes);
        Assert.False(c.IsClean);
        Assert.Equal(c.Crashes.Count, c.UniqueCrashes);
    }

    [Fact]
    public void DedupKeepsOneRecordPerSignatureWithCorrectHitsAndTheShorterInput()
    {
        ExecFunc exec = (d, cov) => d.Length == 0 ? Outcome.Pass() : Outcome.Crash("k", new[] { "f" });
        FuzzReport r = FuzzEngine.RunOutcomes(Seeds(new byte[] { 5, 5, 5, 5 }, new byte[] { 6, 6 }), Opts(50, 1) with { MinimizeBudget = 0 }, exec);
        Assert.Single(r.Crashes);
        Assert.Equal(2, r.Crashes[0].Hits);
        Assert.Equal(2, r.Crashes[0].MinimalInput().Length); // the shorter input is kept on a re-hit
        Assert.False(r.Crashes[0].Minimized);
        FuzzReport m = FuzzEngine.RunOutcomes(Seeds(new byte[] { 5, 5, 5, 5 }), Opts(50, 1), exec);
        Assert.Single(m.Crashes[0].MinimalInput());
        Assert.True(m.Crashes[0].Minimized);
    }

    [Fact]
    public void ALongerReHitNeverReplacesTheShorterInput()
    {
        ExecFunc exec = (d, cov) => Outcome.Crash("k", new[] { "f" });
        FuzzReport r = FuzzEngine.RunOutcomes(Seeds(new byte[] { 1 }, new byte[] { 2, 2, 2 }), Opts(0, 1), exec);
        Assert.Single(r.Crashes[0].MinimalInput());
        Assert.Equal(2, r.Crashes[0].Hits);
    }

    [Fact]
    public void RealCrashesDedupByTypeAndFramesNotByMessage()
    {
        FuzzReport r = Run(
            Seeds(new byte[] { 1 }, new byte[] { 2 }, new byte[] { 3 }),
            Opts(0, 1) with { MinimizeBudget = 0 },
            (d, h) =>
            {
                if (d[0] == 3)
                {
                    throw new ArgumentException("other kind");
                }

                Boom(d, h);
            });
        Assert.Equal(2, r.Crashes.Count);
        Assert.Equal(2, r.Crashes.Single(c => c.Kind == "System.InvalidOperationException").Hits);
    }

    [Fact]
    public void TheCrashTableIsCappedAndCountsDroppedSignatures()
    {
        List<byte[]> many = Enumerable.Range(0, FuzzEngine.MaxCrashes + 25).Select(i => new[] { (byte)(i >> 8), (byte)i }).ToList();
        ExecFunc exec = (d, cov) => Outcome.Crash("k" + ((d[0] << 8) | d[1]), new[] { "f" });
        FuzzReport r = FuzzEngine.RunOutcomes(many, Opts(10, 1) with { MinimizeBudget = 0 }, exec);
        Assert.Equal(FuzzEngine.MaxCrashes, r.Crashes.Count);
        Assert.Equal(25, r.CrashesDropped);
        Assert.Equal(FuzzEngine.MaxCrashes + 25, r.UniqueCrashes);
        Assert.False(r.IsClean);
        Assert.Throws<NotSupportedException>(() => ((IList<CrashRecord>)r.Crashes).Add(null!));
    }

    [Fact]
    public void CoverageAdmissionGrowsTheCorpusAndTiersAreHonest()
    {
        FuzzReport r = Run(Seeds(new byte[] { 0 }), Opts(500, 1), (d, h) => { if (d.Length > 0) { h.Edge(d[0]); } });
        Assert.True(r.AdmittedSeeds > 1);
        Assert.True(r.TotalEdges > 1);
        Assert.Equal(FuzzReport.TierReported, r.CoverageTier);
        FuzzReport blind = Run(Seeds(new byte[] { 0 }), Opts(100, 1), (d, h) => { });
        Assert.Equal(FuzzReport.TierBlackBox, blind.CoverageTier);
        Assert.Equal(1, blind.AdmittedSeeds); // the first passing seed is admitted even with no coverage
        Assert.Equal(0, blind.TotalEdges);
        Assert.True(blind.IsClean);
    }

    [Fact]
    public void MinimisationWorkIsBoundedByMaxIterationsAcrossTheCampaign()
    {
        int execs = 0;
        List<byte[]> many = Enumerable.Range(0, 100).Select(i => Enumerable.Repeat((byte)(i + 1), 100).ToArray()).ToList();
        ExecFunc safe = (d, cov) =>
        {
            execs++;
            return Outcome.Crash("k" + (d.Length == 0 ? 0 : d[0]), new[] { "f" });
        };
        FuzzReport r = FuzzEngine.RunOutcomes(many, Opts(5, 1), safe);
        int probes = execs - many.Count;
        Assert.True(probes >= 1 && probes <= 5, probes + " minimisation probes for MaxIterations 5");
        Assert.Contains(r.Crashes, c => !c.Minimized);
        Assert.Equal(100, r.Crashes.Count);
        execs = 0;
        FuzzReport z = FuzzEngine.RunOutcomes(many, Opts(0, 1), safe);
        Assert.Equal(many.Count, execs); // MaxIterations 0: nothing is minimised at all
        Assert.All(z.Crashes, c => Assert.False(c.Minimized));
        execs = 0;
        FuzzEngine.RunOutcomes(Seeds(new byte[500]), Opts(1000, 1) with { MinimizeBudget = 7 }, (d, cov) =>
        {
            execs++;
            return Outcome.Crash("k", new[] { "f" });
        });
        Assert.True(execs - 1 <= 7, execs + " probes for MinimizeBudget 7");
    }

    [Fact]
    public void OptionsAndArgumentsAreValidated()
    {
        Target ok = (d, h) => { };
        List<byte[]> one = Seeds(new byte[1]);
        FuzzOptions d = FuzzOptions.Defaults();
        FuzzOptions[] bad =
        {
            d with { MaxIterations = -1 }, d with { MaxIterations = Mutate.MaxIterations + 1 },
            d with { MaxInputLen = -1 }, d with { MaxInputLen = Mutate.MaxInputBytes + 1 },
            d with { BaseEnergy = -1 }, d with { BaseEnergy = Mutate.MaxIterations + 1 },
            d with { MinimizeBudget = -1 }, d with { MinimizeBudget = FuzzOptions.MaxMinimizeBudget + 1 },
            d with { StepBudget = -1 }, d with { StepBudget = FuzzOptions.MaxStepBudget + 1 },
            d with { MemBudget = -1 }, d with { MemBudget = FuzzOptions.MaxMemBudget + 1 },
        };
        foreach (FuzzOptions o in bad)
        {
            Assert.Throws<ArgumentException>(() => Run(one, o, ok));
            Assert.Throws<ArgumentException>(() => FuzzEngine.RunOutcomes(one, o, (x, y) => Outcome.Pass()));
        }

        (d with { MaxIterations = Mutate.MaxIterations, MaxInputLen = Mutate.MaxInputBytes }).Validate();
        Assert.Throws<ArgumentNullException>(() => Run(one, d, null!));
        Assert.Throws<ArgumentNullException>(() => FuzzEngine.RunOutcomes(one, d, null!));
        Assert.Throws<ArgumentNullException>(() => Run(one, null!, ok));
        Assert.Throws<ArgumentNullException>(() => FuzzEngine.RunOutcomes(null!, d, (x, y) => Outcome.Pass()));
        List<byte[]> tooMany = Enumerable.Range(0, FuzzEngine.MaxInitialCorpus + 1).Select(_ => Array.Empty<byte>()).ToList();
        Assert.Throws<ArgumentException>(() => Run(tooMany, d, ok));
        Assert.Throws<ArgumentException>(() => Run(Seeds(new byte[Mutate.MaxInputBytes + 1]), d, ok));
        Assert.Throws<ArgumentException>(() => FuzzEngine.RunOutcomes(one, d, (x, y) => null!));
        Assert.Throws<ArgumentException>(() => FuzzEngine.RunOutcomes(one, d, (x, y) => new Outcome((OutcomeKind)99, string.Empty, Array.Empty<string>())));
        Assert.Equal(1000, d.MaxIterations);
        Assert.Equal(42UL, d.Seed);
        Assert.Equal(4096, d.MaxInputLen);
        Assert.Equal(10, d.BaseEnergy);
        Assert.Equal(2000, d.MinimizeBudget);
        Assert.Equal(200_000, d.StepBudget);
        Assert.Equal(64 << 20, d.MemBudget);
    }

    // ---- retained bytes cap --------------------------------------------------------------------

    [Fact]
    public void TheRetainedBytesCapRefusesAdmissionsAndCrashInputsVisibly()
    {
        ExecFunc cover = (d, cov) =>
        {
            cov.Record(d.Length == 0 ? 0 : d[0] + 1UL);
            return Outcome.Pass();
        };
        FuzzReport r = FuzzEngine.RunOutcomes(Seeds(new byte[] { 1, 2, 3, 4 }), Opts(300, 1), cover, 6);
        Assert.True(r.AdmissionsDropped > 0);
        Assert.True(r.GetCorpus().Sum(c => (long)c.Length) <= 6);
        Assert.Equal(0, FuzzEngine.RunOutcomes(Seeds(new byte[] { 1, 2, 3, 4 }), Opts(300, 1), cover, 1L << 20).AdmissionsDropped);
        ExecFunc crash = (d, cov) => Outcome.Crash("k", new[] { "f" });
        FuzzReport c = FuzzEngine.RunOutcomes(Seeds(new byte[] { 1, 2, 3, 4 }), Opts(0, 1), crash, 2);
        Assert.True(c.Crashes[0].InputDropped);
        Assert.Empty(c.Crashes[0].MinimalInput());
        Assert.False(c.Crashes[0].Minimized);
        FuzzReport d2 = FuzzEngine.RunOutcomes(Seeds(new byte[] { 1, 2, 3, 4 }, new byte[] { 9 }), Opts(0, 1), crash, 2);
        Assert.Equal(2, d2.Crashes[0].Hits);
        Assert.True(d2.Crashes[0].InputDropped); // a dropped record is not replaced by a later shorter input
    }

    // ---- privacy of every rendering -------------------------------------------------------------

    [Fact]
    public void NoRenderingOfAReportOrARecordShowsSeedsOrBytes()
    {
        byte[] secret = { 0xde, 0xad, 0xbe, 0xef, 0x13, 0x37 };
        FuzzReport r = Run(
            Seeds(secret),
            Opts(50, 0x5eedUL) with { MinimizeBudget = 0 },
            (d, h) =>
            {
                h.Edge((ulong)d.Length);
                if (d.Length >= 6)
                {
                    throw new InvalidOperationException("payload " + string.Join(",", d));
                }
            });
        Assert.NotEmpty(r.Crashes);
        List<string> renderings = new() { r.ToString(), string.Format("{0}|{1}", r, string.Join(";", r.Crashes)), $"{r}", r.Crashes[0].ToString() };
        foreach (CrashRecord c in r.Crashes)
        {
            renderings.Add(string.Join(",", c.Frames));
        }

        renderings.AddRange(FuzzSignature.TapFailures(r, "tgt", 1));
        foreach (string s in renderings)
        {
            Assert.DoesNotContain("deadbeef", s, StringComparison.OrdinalIgnoreCase);
            Assert.DoesNotContain("222", s);
            Assert.DoesNotContain("payload", s);
            Assert.DoesNotContain(0x5eedUL.ToString(), s);
            Assert.DoesNotContain("5eed", s, StringComparison.OrdinalIgnoreCase);
            Assert.DoesNotContain("Byte[]", s);
        }
    }

    [Fact]
    public void AccessorsReturnCopies()
    {
        FuzzReport r = Run(Seeds(new byte[] { 1, 2, 3 }), Opts(5, 1) with { MinimizeBudget = 0 }, (d, h) => { if (d.Length == 3 && d[0] == 1) { throw new InvalidOperationException(); } });
        CrashRecord c = r.Crashes[0];
        byte[] m = c.MinimalInput();
        Array.Fill(m, (byte)99);
        Assert.Equal(new byte[] { 1, 2, 3 }, c.MinimalInput());
        foreach (byte[] x in r.GetCorpus())
        {
            Array.Fill(x, (byte)7);
        }

        Assert.Equal(1UL, r.Seed);
        Assert.Equal("System.InvalidOperationException", c.Kind);
        Assert.Equal(8, c.Signature.Length);
    }
}
