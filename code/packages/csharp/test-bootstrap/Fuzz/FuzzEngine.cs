using System.Diagnostics;
using System.Runtime.CompilerServices;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Fuzz;

/// <summary>The code under test. It receives its own copy of the input and a <see cref="Handle"/>.</summary>
/// <remarks>
/// Any exception that escapes is a crash (except the engine's own budget sentinel and
/// <see cref="OutOfMemoryException"/>, which is an OOM outcome).
/// </remarks>
public delegate void Target(byte[] data, Handle h);

/// <summary>The code under test, as outcomes: runs <paramref name="data"/>, reports edges into <paramref name="cov"/>.</summary>
public delegate Outcome ExecFunc(byte[] data, Bitmap cov);

/// <summary>
/// The deterministic, budgeted coverage-guided campaign (VER06 sections 5 and 9.1, VER06h sections 5
/// and 6).
/// </summary>
/// <remarks>
/// A campaign is a pure function of (corpus, options, target): triage every seed, then repeatedly
/// pick a seed, give it an energy-scaled number of mutated children, run each child and either admit
/// it (new coverage), record it (crash, timeout, OOM) or drop it (redundant). Nothing here reads a
/// clock, a file or the environment, and nothing starts a thread.
/// </remarks>
public static class FuzzEngine
{
    /// <summary>Distinct crash signatures a report holds; further novel ones only increment <c>CrashesDropped</c>.</summary>
    public const int MaxCrashes = 1000;

    /// <summary>Seeds accepted by one campaign.</summary>
    public const int MaxInitialCorpus = 10_000;

    /// <summary>Bytes of corpus seeds plus crash inputs one campaign retains (VER06h section 6).</summary>
    public const long MaxRetainedBytes = 256L << 20;

    /// <summary>Stack frames inspected per crash, however deep the recursion (CWE-400).</summary>
    internal const int MaxWalk = 4096;

    // ---------------------------------------------------------------------------
    // The executor
    // ---------------------------------------------------------------------------

    /// <summary>
    /// Runs a bounded coverage-guided campaign against <paramref name="target"/>. See
    /// <see cref="RunOutcomes(IReadOnlyList{byte[]}, FuzzOptions, ExecFunc)"/> for the contracts. The
    /// coverage tier is <c>caller-reported</c> when the target called <c>Edge</c> at least once
    /// during the campaign, <c>black-box-uninstrumented</c> otherwise.
    /// </summary>
    public static FuzzReport Run(IReadOnlyList<byte[]> initialCorpus, FuzzOptions opts, Target target)
    {
        ArgumentNullException.ThrowIfNull(target);
        ArgumentNullException.ThrowIfNull(opts);
        opts.Validate();
        object campaign = new();
        long epoch = 0;
        return RunOutcomes(
            initialCorpus,
            opts,
            (data, cov) =>
            {
                // A FRESH handle per execution, retired by RunTarget: a stale continuation or task can
                // never touch a later run's budgets or coverage map.
                Handle h = new(opts.StepBudget, opts.MemBudget, campaign, ++epoch);
                h.Cov = cov;
                return RunTarget(target, data, h);
            });
    }

    /// <summary>
    /// Runs one execution. Its type and name delimit the target's frames: the stack is cut here.
    /// </summary>
    /// <remarks>
    /// Only the target runs inside this <c>try</c>. A real <see cref="OutOfMemoryException"/> is an OOM
    /// outcome, and nothing is rendered from it (the message and the stack are never read on that
    /// path); every other exception goes to <see cref="Classify(Exception, Handle)"/>. The sticky-exhaustion check after
    /// the try is what stops a target that swallowed the budget sentinel (which .NET cannot prevent:
    /// the sentinel is an <see cref="Exception"/>). A <c>StackOverflowException</c> cannot be caught in
    /// .NET and ends the process; targets must bound their recursion.
    /// </remarks>
    [MethodImpl(MethodImplOptions.NoInlining)]
    internal static Outcome RunTarget(Target target, byte[] data, Handle h)
    {
        // The copy is made OUTSIDE the try: an out-of-memory raised by the engine's own copy is the
        // engine's problem, not a verdict about the target.
        byte[] copy = (byte[])data.Clone();
        AsyncVoidWatch watch = new();
        SynchronizationContext? previous = SynchronizationContext.Current;
        SynchronizationContext.SetSynchronizationContext(watch);
        bool outOfMemory = false;
        Exception? thrown = null;
        try
        {
            target(copy, h);
        }
        catch (OutOfMemoryException)
        {
            outOfMemory = true;
        }
        catch (Exception t) // a target may throw anything; every kind is a crash
        {
            thrown = t;
        }
        finally
        {
            SynchronizationContext.SetSynchronizationContext(previous);
            h.Retire(); // nothing may use this handle after the target has returned
        }

        try
        {
            // Read AFTER Retire(): from here no stale thread can change the handle's state.
            Handle.BudgetExceeded? exhausted = h.Exhausted;
            if (outOfMemory)
            {
                // A real OOM; but if a budget had already run out, that broke first.
                return exhausted != null ? ExhaustedOutcome(exhausted) : Outcome.Oom();
            }

            if (thrown != null)
            {
                // ... unless its budget had already run out: a target that swallowed the sentinel and
                // then threw something ordinary is still a timeout or an OOM, not a crash.
                return exhausted != null ? ExhaustedOutcome(exhausted) : Classify(thrown, h);
            }

            // A target that swallowed the sentinel and returned is not a pass either.
            if (exhausted != null)
            {
                return ExhaustedOutcome(exhausted);
            }

            // An async void method returns at its first await: the rest of it runs LATER. Give
            // suspended operations and in-flight posted callbacks a short, bounded grace to finish
            // (it costs nothing when there are none), then judge.
            watch.AwaitQuiescence(AsyncGraceMs);
            if (watch.Faulted)
            {
                return Outcome.Crash(AsyncVoidFaulted, Array.Empty<string>());
            }

            if (watch.Pending > 0 || watch.Outstanding > 0)
            {
                return Outcome.Crash(AsyncVoidPending, Array.Empty<string>());
            }

            return Outcome.Pass();
        }
        finally
        {
            watch.Close();
        }
    }

    /// <summary>
    /// How long the executor waits, after the target returned, for a suspended <c>async void</c>
    /// operation to finish and for posted callbacks to drain, before it calls the run
    /// <c>async-void-pending</c>. Only ever spent when such work exists. An operation that resumes
    /// later than this is reported as pending even if it would have completed quietly.
    /// </summary>
    internal const int AsyncGraceMs = 10;

    /// <summary>Crash kind of a run whose target left an <c>async void</c> operation unfinished.</summary>
    internal const string AsyncVoidPending = "async-void-pending";

    /// <summary>Crash kind of a run during which a posted callback (an <c>async void</c> fault) threw.</summary>
    internal const string AsyncVoidFaulted = "async-void-exception";

    /// <summary>
    /// A counting <see cref="SynchronizationContext"/> installed around the target. The runtime's
    /// <c>AsyncVoidMethodBuilder</c> calls <c>OperationStarted</c> when an <c>async void</c> method
    /// begins and <c>OperationCompleted</c> when it finishes, so <see cref="Pending"/> is non-zero
    /// while such a method is suspended. A fault in an <c>async void</c> method is rethrown by
    /// <see cref="Post"/>ing a callback that throws: every posted callback runs here with its
    /// exceptions swallowed (an <c>async void</c> fault can never terminate the host), and one that
    /// throws while the run is live sets <see cref="Faulted"/>, from WHATEVER thread it ran on. A
    /// callback that merely yields or blocks is never a fault.
    /// </summary>
    private sealed class AsyncVoidWatch : SynchronizationContext
    {
        private int _pending;
        private int _posted;
        private int _done;
        private int _faulted;
        private int _live = 1;

        internal int Pending => Volatile.Read(ref _pending);

        internal int Outstanding => Volatile.Read(ref _posted) - Volatile.Read(ref _done);

        internal bool Faulted => Volatile.Read(ref _faulted) != 0;

        /// <summary>Stops recording faults: the run is judged.</summary>
        internal void Close() => Volatile.Write(ref _live, 0);

        /// <summary>Waits up to <paramref name="graceMs"/> for pending operations and posted callbacks to finish.</summary>
        internal void AwaitQuiescence(int graceMs)
        {
            if (Pending == 0 && Outstanding == 0)
            {
                return;
            }

            Stopwatch sw = Stopwatch.StartNew();
            SpinWait spin = default;
            while ((Pending > 0 || Outstanding > 0) && sw.ElapsedMilliseconds < graceMs)
            {
                spin.SpinOnce(sleep1Threshold: -1);
            }
        }

        public override void OperationStarted() => Interlocked.Increment(ref _pending);

        public override void OperationCompleted() => Interlocked.Decrement(ref _pending);

        public override void Post(SendOrPostCallback d, object? state)
        {
            Interlocked.Increment(ref _posted);
            ThreadPool.QueueUserWorkItem(
                _ =>
                {
                    try
                    {
                        d(state);
                    }
                    catch (Exception)
                    {
                        // an async void fault must not take the process down; it fails the run instead
                        if (Volatile.Read(ref _live) != 0)
                        {
                            Volatile.Write(ref _faulted, 1);
                        }
                    }
                    finally
                    {
                        Interlocked.Increment(ref _done);
                    }
                });
        }

        public override void Send(SendOrPostCallback d, object? state)
        {
            d(state);
        }
    }

    private static Outcome ExhaustedOutcome(Handle.BudgetExceeded exhausted)
    {
        return exhausted.Which == Handle.Budget.Steps ? Outcome.Timeout() : Outcome.Oom();
    }

    /// <summary>Turns an exception into an outcome. Budget sentinels of another handle are rethrown to their owner.</summary>
    internal static Outcome Classify(Exception t, Handle h) => Classify(t, h, CaptureFrames);

    /// <summary>Crash kind of a budget or retired-handle sentinel that escaped into a run it does not belong to.</summary>
    internal const string StaleSentinel = "stale-budget-sentinel";

    /// <summary>
    /// The frame capture of <see cref="Classify(Exception, Handle)"/>: the runtime's own record, never a virtual member.</summary>
    /// <remarks>
    /// Cost: capturing a stack is O(depth) per crash, and the minimiser re-executes the target (and
    /// so re-captures a stack) for every candidate it tries, up to its budget. Only the first
    /// <see cref="MaxWalk"/> frames are then inspected.
    /// </remarks>
    private static List<string> CaptureFrames(Exception t)
    {
        StackTrace st = new(t, false);
        return FramesOf(st.FrameCount, i => RawOf(st.GetFrame(i)));
    }

    /// <summary><see cref="Classify(Exception, Handle)"/> with an injectable frame capture (the test seam for a failing walk).</summary>
    internal static Outcome Classify(Exception t, Handle h, Func<Exception, List<string>> capture)
    {
        if (t is Handle.BudgetExceeded b)
        {
            if (!ReferenceEquals(b.Campaign, h.Campaign))
            {
                throw b; // another campaign's budget: let it unwind to its owner
            }

            // Same campaign, different execution (or a retired handle): a sentinel caught in one run
            // and rethrown in a later one must not be read as THIS run's budget breach.
            if (b.Epoch != h.Epoch || b.Which == Handle.Budget.Retired)
            {
                return Outcome.Crash(StaleSentinel, Array.Empty<string>());
            }

            return b.Which == Handle.Budget.Steps ? Outcome.Timeout() : Outcome.Oom();
        }

        // Rendering an untrusted exception. The type name cannot be overridden. The virtual
        // StackTrace, ToString and Message members CAN be hostile, so none of them is ever called:
        // frames are read from the runtime's own capture with "new StackTrace(exception)". Even so
        // the walk is guarded: whatever it raises, the campaign goes on (an out-of-memory is a
        // counted OOM, everything else means "no frames").
        Type type = t.GetType();
        string kind = FuzzSignature.NormaliseKind(type.FullName ?? type.Name);
        List<string> frames;
        try
        {
            frames = capture(t);
        }
        catch (OutOfMemoryException)
        {
            return Outcome.Oom(); // the stack walk itself ran out of memory: a counted OOM
        }
        catch (Exception)
        {
            frames = new List<string>();
        }

        return Outcome.Crash(kind, frames);
    }

    /// <summary>One stack frame reduced to what the signature needs.</summary>
    internal readonly record struct RawFrame(IReadOnlyList<string> TypeSegments, string Method, bool Plumbing, bool IsExecutor);

    private static Type TopLevel(Type t)
    {
        Type cur = t;
        while (cur.DeclaringType != null)
        {
            cur = cur.DeclaringType;
        }

        return cur;
    }

    private static RawFrame? RawOf(StackFrame? f)
    {
        System.Reflection.MethodBase? m = f?.GetMethod();
        if (m == null)
        {
            return null;
        }

        List<string> segs = new();
        Type? top = null;
        for (Type? cur = m.DeclaringType; cur != null; cur = cur.DeclaringType)
        {
            segs.Insert(0, cur.Name);
            top = cur;
        }

        bool plumbing = false;
        bool executor = false;
        if (top != null)
        {
            string ns = top.Namespace ?? string.Empty;
            bool engine = top == typeof(FuzzEngine) || top == typeof(Handle);
            executor = top == typeof(FuzzEngine) && m.Name == "RunTarget";
            plumbing = engine
                || top.Assembly == typeof(object).Assembly
                || ns == "System" || ns.StartsWith("System.", StringComparison.Ordinal)
                || ns.StartsWith("Microsoft.", StringComparison.Ordinal);
        }

        return new RawFrame(segs, m.Name, plumbing, executor);
    }

    /// <summary>
    /// Normalises a captured stack: innermost first, engine and BCL frames dropped, cut at the
    /// executor's own frame, consecutive duplicates collapsed, at most 64 kept. At most
    /// <see cref="MaxWalk"/> frames are inspected, so a hostile trace cannot make this loop long.
    /// </summary>
    internal static List<string> FramesOf(int count, Func<int, RawFrame?> at)
    {
        List<string> frames = new();
        string prevRaw = string.Empty;
        int limit = Math.Min(count, MaxWalk);
        for (int i = 0; i < limit; i++)
        {
            RawFrame? maybe = at(i);
            if (maybe is not { } f)
            {
                continue;
            }

            if (f.IsExecutor)
            {
                break;
            }

            if (f.Plumbing)
            {
                continue; // engine / BCL plumbing, not the target's
            }

            string raw = string.Join("+", f.TypeSegments) + "#" + f.Method;
            if (frames.Count == 0 || raw != prevRaw)
            {
                prevRaw = raw;
                frames.Add(FuzzSignature.NormaliseFrame(f.TypeSegments, f.Method));
                if (frames.Count >= FuzzSignature.MaxFrames)
                {
                    break;
                }
            }
        }

        return FuzzSignature.Collapse(frames);
    }

    // ---------------------------------------------------------------------------
    // Minimisation (VER06 section 5.3)
    // ---------------------------------------------------------------------------

    private static bool SameCrash(Outcome o, string sig)
    {
        return o.Kind == OutcomeKind.Crash && FuzzSignature.CrashSignature(o.CrashKind, o.Frames) == sig;
    }

    /// <summary>The result of a minimisation: the bytes, executions spent, and whether it finished.</summary>
    internal readonly record struct MinimizedResult(byte[] Data, int Spent, bool Finished);

    /// <summary>
    /// Shrinks <paramref name="data"/> while it keeps the exact signature. Every candidate execution,
    /// accepted or rejected, is counted in <c>Spent</c>, and the budget is checked before each one, so
    /// no stream of rejected candidates can run past it.
    /// </summary>
    internal static MinimizedResult Minimize(byte[] data, string sig, Func<byte[], Outcome> test, int budget)
    {
        byte[] cur = (byte[])data.Clone();
        int spent = 0;
        if (cur.Length == 0)
        {
            return new MinimizedResult(cur, 0, true);
        }

        // Pass 1: delete blocks, halving the block size. Starting at max(1, len/2) (not len/2) is
        // what makes the empty input reachable from a 1-byte crasher.
        for (int chunk = Math.Max(1, cur.Length / 2); chunk > 0; chunk /= 2)
        {
            int offset = 0;
            while (offset < cur.Length)
            {
                if (spent >= budget)
                {
                    return new MinimizedResult(cur, spent, false);
                }

                int end = offset + Math.Min(chunk, cur.Length - offset);
                byte[] candidate = new byte[cur.Length - (end - offset)];
                Array.Copy(cur, 0, candidate, 0, offset);
                Array.Copy(cur, end, candidate, offset, cur.Length - end);
                spent++;
                if (SameCrash(test(candidate), sig))
                {
                    cur = candidate; // same offset again: the tail slid left
                }
                else
                {
                    offset += chunk;
                }
            }
        }

        // Pass 2: zero individual non-essential bytes.
        for (int i = 0; i < cur.Length; i++)
        {
            if (cur[i] == 0)
            {
                continue;
            }

            if (spent >= budget)
            {
                return new MinimizedResult(cur, spent, false);
            }

            byte original = cur[i];
            cur[i] = 0;
            spent++;
            if (!SameCrash(test((byte[])cur.Clone()), sig))
            {
                cur[i] = original;
            }
        }

        return new MinimizedResult(cur, spent, true);
    }

    /// <summary>
    /// Shrinks <paramref name="data"/> while it keeps the EXACT crash signature: pass 1 deletes blocks,
    /// pass 2 zeroes bytes. A candidate that crashes differently is rejected, so minimisation never
    /// trades one bug for another. <paramref name="budget"/> caps the executions of
    /// <paramref name="test"/>; it must be in [0, <see cref="FuzzOptions.MaxMinimizeBudget"/>] and
    /// <paramref name="data"/> at most <see cref="Mutate.MaxInputBytes"/>.
    /// </summary>
    public static byte[] MinimizeCrashingInput(byte[] data, string sig, Func<byte[], Outcome> test, int budget)
    {
        if (budget < 0 || budget > FuzzOptions.MaxMinimizeBudget)
        {
            throw new ArgumentException("budget must be in [0, " + FuzzOptions.MaxMinimizeBudget + "]", nameof(budget));
        }

        ArgumentNullException.ThrowIfNull(test);
        if (data.Length > Mutate.MaxInputBytes)
        {
            throw new ArgumentException("data exceeds " + Mutate.MaxInputBytes + " bytes", nameof(data));
        }

        return Minimize(data, sig, test, budget).Data;
    }

    // ---------------------------------------------------------------------------
    // The campaign
    // ---------------------------------------------------------------------------

    private sealed record SeedEntry(byte[] Data, int NewEdges, int Generation);

    private sealed class Campaign
    {
        private readonly ExecFunc _exec;
        private readonly FuzzOptions _opts;
        private readonly long _retainedCap;
        private readonly Dictionary<string, int> _bySig = new();
        private int _minimizeLeft;
        private long _retained;

        internal Campaign(ExecFunc exec, FuzzOptions opts, long retainedCap)
        {
            _exec = exec;
            _opts = opts;
            _retainedCap = retainedCap;
            _minimizeLeft = opts.MaxIterations;
            Rng = new SplitMix64(opts.Seed);
        }

        internal SplitMix64 Rng { get; }

        internal Bitmap Step { get; } = new();

        internal Bitmap Cumulative { get; } = new();

        internal List<SeedEntry> Corpus { get; } = new();

        internal List<CrashRecord> Crashes { get; } = new();

        internal int Timeouts { get; private set; }

        internal int Ooms { get; private set; }

        internal int Dropped { get; private set; }

        internal int AdmissionsDropped { get; private set; }

        /// <summary>Runs one input; the edges of the run are left in <see cref="Step"/>.</summary>
        internal Outcome Execute(byte[] data)
        {
            Step.Reset();
            Outcome? output = _exec(data, Step);
            if (output == null || !Enum.IsDefined(output.Kind))
            {
                throw new ArgumentException("exec returned no outcome");
            }

            return output;
        }

        /// <summary>Puts one non-passing execution in its bucket.</summary>
        internal void File(Outcome output, byte[] data)
        {
            switch (output.Kind)
            {
                case OutcomeKind.Timeout:
                    Timeouts++;
                    break;
                case OutcomeKind.Oom:
                    Ooms++;
                    break;
                case OutcomeKind.Crash:
                    RecordCrash(output, data);
                    break;
            }
        }

        /// <summary>
        /// Puts a passing input in the corpus iff it covered a new edge. The first passing seed is
        /// always admitted, even with no coverage, so there is something to mutate in black-box mode.
        /// </summary>
        internal void Admit(byte[] data, int generation)
        {
            Bitmap.Admission a = Cumulative.Admit(Step.Trace());
            if (a.HasNew || (generation == 0 && Corpus.Count == 0))
            {
                if (_retained + data.Length > _retainedCap)
                {
                    AdmissionsDropped++;
                    return;
                }

                _retained += data.Length;
                Corpus.Add(new SeedEntry((byte[])data.Clone(), a.CountNew, generation));
            }
        }

        private void RecordCrash(Outcome output, byte[] data)
        {
            string sig = FuzzSignature.CrashSignature(output.CrashKind, output.Frames);
            if (_bySig.TryGetValue(sig, out int at))
            {
                CrashRecord rec = Crashes[at];
                rec.Hit();
                if (!rec.InputDropped && data.Length < rec.MinimalLength)
                {
                    _retained -= rec.MinimalLength - data.Length;
                    rec.ReplaceMinimal((byte[])data.Clone());
                }

                return;
            }

            if (Crashes.Count >= MaxCrashes)
            {
                Dropped++;
                return;
            }

            int allowance = Math.Min(_opts.MinimizeBudget, _minimizeLeft);
            MinimizedResult m = Minimize(data, sig, Execute, allowance);
            _minimizeLeft -= m.Spent;
            byte[] kept = m.Data;
            bool droppedInput = false;
            if (_retained + kept.Length > _retainedCap)
            {
                kept = Array.Empty<byte>();
                droppedInput = true;
            }
            else
            {
                _retained += kept.Length;
            }

            _bySig[sig] = Crashes.Count;
            Crashes.Add(new CrashRecord(sig, output.CrashKind, output.Frames, kept, m.Finished && !droppedInput, droppedInput));
        }
    }

    /// <summary>
    /// Runs a bounded coverage-guided campaign over <paramref name="exec"/>.
    /// </summary>
    /// <remarks>
    /// Contracts (VER06 section 9.1), each pinned by a test:
    /// <list type="bullet">
    /// <item>a pure function of (corpus, options, exec): same inputs, same report;</item>
    /// <item>with a live corpus <c>Iterations == opts.MaxIterations</c> exactly, and no mutated
    /// candidate exceeds <c>opts.MaxInputLen</c> (initial seeds run as given, up to
    /// <see cref="Mutate.MaxInputBytes"/> each);</item>
    /// <item>an empty corpus is replaced by one <c>00</c> seed; seeds that crash, time out or exhaust
    /// memory are never mutated, so a corpus of crashers fuzzes nothing (iterations == 0);</item>
    /// <item>every execution lands in crashes, timeouts, ooms or the corpus; <c>IsClean</c> needs at
    /// least one iteration and no failure at all;</item>
    /// <item>one <see cref="CrashRecord"/> per signature; a novel crash is minimised.</item>
    /// </list>
    /// Per iteration the stream draws: <c>Bounded(len(corpus))</c> for the seed; per energy slot
    /// <c>1 + Bounded(4)</c> steps, one <c>NextU64</c> sub-seed, then, if <c>len(corpus) &gt; 1</c>,
    /// <c>Bounded(4)</c> and on 0 a donor index and two split draws (as the Go, Rust and Java lanes).
    /// </remarks>
    public static FuzzReport RunOutcomes(IReadOnlyList<byte[]> initialCorpus, FuzzOptions opts, ExecFunc exec)
    {
        return RunOutcomes(initialCorpus, opts, exec, MaxRetainedBytes);
    }

    /// <summary><see cref="RunOutcomes(IReadOnlyList{byte[]}, FuzzOptions, ExecFunc)"/> with an explicit retained-bytes cap (the test seam).</summary>
    internal static FuzzReport RunOutcomes(IReadOnlyList<byte[]> initialCorpus, FuzzOptions opts, ExecFunc exec, long retainedCap)
    {
        ArgumentNullException.ThrowIfNull(opts);
        opts.Validate();
        ArgumentNullException.ThrowIfNull(exec);
        ArgumentNullException.ThrowIfNull(initialCorpus);
        if (initialCorpus.Count > MaxInitialCorpus)
        {
            throw new ArgumentException("initial corpus exceeds " + MaxInitialCorpus + " seeds", nameof(initialCorpus));
        }

        for (int i = 0; i < initialCorpus.Count; i++)
        {
            if (initialCorpus[i].Length > Mutate.MaxInputBytes)
            {
                throw new ArgumentException("initial corpus seed " + i + " exceeds " + Mutate.MaxInputBytes + " bytes", nameof(initialCorpus));
            }
        }

        IReadOnlyList<byte[]> seeds = initialCorpus.Count == 0 ? new[] { new byte[] { 0 } } : initialCorpus;
        Campaign c = new(exec, opts, retainedCap);

        // 1. Triage: run every seed once.
        foreach (byte[] data in seeds)
        {
            Outcome output = c.Execute(data);
            if (output.Kind == OutcomeKind.Pass)
            {
                c.Admit(data, 0);
            }
            else
            {
                c.File(output, data);
            }
        }

        // 2. The mutation loop.
        int iterations = 0;
        while (iterations < opts.MaxIterations && c.Corpus.Count > 0)
        {
            SeedEntry seed = c.Corpus[(int)c.Rng.Bounded((ulong)c.Corpus.Count)];
            int energy = Bitmap.CalculateEnergy(opts.BaseEnergy, seed.Data.Length, seed.NewEdges, c.Cumulative.Count);
            for (int slot = 0; slot < energy && iterations < opts.MaxIterations; slot++)
            {
                iterations++;
                int steps = 1 + (int)c.Rng.Bounded(4);
                ulong sub = c.Rng.NextU64();
                byte[] candidate = Mutate.MutateSequence(seed.Data, sub, steps, Mutate.MaxInputBytes); // steps <= 4
                if (c.Corpus.Count > 1 && c.Rng.Bounded(4) == 0)
                {
                    byte[] donor = c.Corpus[(int)c.Rng.Bounded((ulong)c.Corpus.Count)].Data;
                    ulong ts = c.Rng.Bounded((ulong)candidate.Length + 1UL);
                    ulong ds = c.Rng.Bounded((ulong)donor.Length + 1UL);
                    candidate = Mutate.Splice(candidate, donor, ts, ds);
                }

                if (candidate.Length > opts.MaxInputLen)
                {
                    Array.Resize(ref candidate, opts.MaxInputLen);
                }

                Outcome output = c.Execute(candidate);
                if (output.Kind == OutcomeKind.Pass)
                {
                    c.Admit(candidate, seed.Generation + 1);
                }
                else
                {
                    c.File(output, candidate);
                }
            }
        }

        int total = c.Cumulative.Count;
        List<byte[]> corpus = c.Corpus.Select(s => s.Data).ToList();
        return new FuzzReport(
            iterations,
            c.Corpus.Count,
            total,
            c.Crashes,
            c.Timeouts,
            c.Ooms,
            c.Dropped,
            c.AdmissionsDropped,
            total > 0 ? FuzzReport.TierReported : FuzzReport.TierBlackBox,
            opts.Seed,
            corpus);
    }
}
