namespace CodingAdventures.TestBootstrap.Pbt;

/// <summary>
/// The budget of a shrink, as a tree of scopes (VER05g section 3.1, ported from the Java
/// engine's VER05e section 4.1, itself redesigned over several review rounds).
/// </summary>
/// <remarks>
/// <para>
/// Every public entry point that runs user code (a property, a shrinker) opens a
/// <em>child</em> node of the ambient scope with a fresh local cap; internal combinators join
/// their enclosing node. A charge walks from the node up to the outermost root and is
/// <b>all or nothing</b>: every node on the path must have room, then every counter is
/// incremented. Whatever a property or a custom shrinker does, however it nests, the total
/// work of one top-level <c>Runner.Check</c> is therefore capped by the outermost root. The
/// bound does not depend on classifying a call as "inside a property" or "inside a shrinker"
/// (an earlier join/isolate classification had a new hole in each review round).
/// </para>
/// <para>
/// Two things are charged: <b>attempts</b> (one per property evaluation) and <b>work</b>
/// (elements copied, at least 1 per charge). One lock, owned by the outermost root, guards
/// every counter in its tree; user code never runs while it is held.
/// </para>
/// <para>
/// The current node is carried in an <see cref="AsyncLocal{T}"/>, so it flows into child
/// threads, <c>Task.Run</c> and async continuations, which then charge the same tree. A
/// node's parent is the nearest <em>open</em> ancestor; a flow whose whole chain has closed
/// gets a fresh root, so a dead root is never charged. Threads that do <em>not</em> flow the
/// execution context (see the spec) have no ambient scope.
/// </para>
/// </remarks>
internal sealed class ShrinkBudget : IDisposable
{
    /// <summary>The most nested owning scopes; deeper scopes join, keeping the charge walk short.</summary>
    internal const int MaxDepth = 64;

    private static readonly AsyncLocal<ShrinkBudget?> Ambient = new();

    /// <summary>Test hook, read only when a root is created (<c>Hooks.Reset</c> restores it).</summary>
    internal static long DefaultMaxWork = Limits.MaxShrinkWork;

    private static long lastWork;
    private static long lastAttempts;

    /// <summary>Work spent by the most recent owning scope that ended (for tests).</summary>
    internal static long LastWork => Volatile.Read(ref lastWork);

    /// <summary>Attempts spent by the most recent owning scope that ended (for tests).</summary>
    internal static long LastAttempts => Volatile.Read(ref lastAttempts);

    private readonly object sync;            // the outermost root's lock, shared by the whole tree
    private readonly ShrinkBudget root;      // outermost node of the tree (this for a root)
    private readonly ShrinkBudget? parent;   // nearest open ancestor at creation, or null
    private readonly ShrinkBudget? target;   // non-null for a joined handle: the owner it aliases
    private readonly ShrinkBudget? previous; // ambient value to restore when an owner ends
    private readonly int depth;
    private readonly long maxAttempts;
    private readonly long maxWork;
    private long attempts;                   // guarded by sync
    private long work;                       // guarded by sync
    private volatile bool closed;
    private volatile bool refused;           // set on a ROOT when any charge in its tree was refused

    private ShrinkBudget(ShrinkBudget? parent, long maxAttempts, long maxWork, ShrinkBudget? previous)
    {
        this.parent = parent;
        root = parent == null ? this : parent.root;
        sync = parent == null ? new object() : parent.sync;
        this.previous = previous;
        depth = parent == null ? 0 : parent.depth + 1;
        this.maxAttempts = maxAttempts;
        this.maxWork = maxWork;
    }

    /// <summary>A joined handle: charges and queries go to the owner it aliases.</summary>
    private ShrinkBudget(ShrinkBudget owner)
    {
        root = owner.root;
        sync = owner.sync;
        target = owner;
        depth = owner.depth;
    }

    private ShrinkBudget Owner => target ?? this;

    /// <summary>True when this scope has an owning ancestor (it is not an outermost root).</summary>
    internal bool IsNested => Owner.parent != null;

    /// <summary>True once any charge anywhere in this scope's tree has been refused.</summary>
    internal bool WasRefused => root.refused;

    /// <summary>True for a handle that joined an existing scope instead of owning one.</summary>
    internal bool IsJoined => target != null;

    /// <summary>The nearest open node from <paramref name="a"/> upward whose root is open, or null.</summary>
    private static ShrinkBudget? Usable(ShrinkBudget? a)
    {
        while (a != null && a.closed)
        {
            a = a.parent;
        }

        return a != null && !a.root.closed ? a : null;
    }

    /// <summary>The ambient open budget of this execution context, or null outside any shrink.</summary>
    internal static ShrinkBudget? Current => Usable(Ambient.Value);

    /// <summary>Joins the ambient scope when there is one, else opens a fresh root. Dispose the result.</summary>
    internal static ShrinkBudget Enter(int maxSteps)
    {
        ShrinkBudget? open = Usable(Ambient.Value);
        if (open != null)
        {
            return new ShrinkBudget(open);
        }

        return NewRoot(Limits.AttemptBudget(maxSteps), DefaultMaxWork, null);
    }

    /// <summary>
    /// The scope of a public entry point: a CHILD of the ambient scope with a fresh local cap,
    /// or a fresh root when there is no open ambient scope. Deeper than <see cref="MaxDepth"/>
    /// it joins instead. <paramref name="maxWork"/> is also the root cap when none is open.
    /// </summary>
    internal static ShrinkBudget EnterChild(int maxSteps, long maxWork = Limits.MaxShrinkWork)
    {
        ShrinkBudget? ambient = Ambient.Value;
        ShrinkBudget? open = Usable(ambient);
        if (open == null)
        {
            return NewRoot(
                Limits.AttemptBudget(maxSteps),
                maxWork == Limits.MaxShrinkWork ? DefaultMaxWork : maxWork,
                null);
        }

        if (open.depth + 1 >= MaxDepth)
        {
            return new ShrinkBudget(open);
        }

        ShrinkBudget child = new(open, Limits.AttemptBudget(maxSteps), maxWork, ambient);
        Ambient.Value = child;
        return child;
    }

    /// <summary>
    /// The scope of a whole <c>Runner.Check</c> (case loop included): a root when no open
    /// ambient scope exists, a child otherwise. Its attempt cap is <c>numTests</c> plus the
    /// shrink attempt cap, so a nested check's cases and shrink are all charged up the chain.
    /// </summary>
    internal static ShrinkBudget EnterCheck(long numTests, int maxSteps)
    {
        long cap = numTests + Limits.AttemptBudget(maxSteps);
        ShrinkBudget? ambient = Ambient.Value;
        ShrinkBudget? open = Usable(ambient);
        if (open == null)
        {
            return NewRoot(cap, DefaultMaxWork, null);
        }

        if (open.depth + 1 >= MaxDepth)
        {
            return new ShrinkBudget(open);
        }

        ShrinkBudget child = new(open, cap, Limits.MaxShrinkWork, ambient);
        Ambient.Value = child;
        return child;
    }

    private static ShrinkBudget NewRoot(long maxAttempts, long maxWork, ShrinkBudget? previous)
    {
        ShrinkBudget fresh = new(null, maxAttempts, maxWork, previous);
        Ambient.Value = fresh;
        return fresh;
    }

    /// <summary>Attempts charged to this node (and its joined handles) so far.</summary>
    internal long Attempts
    {
        get
        {
            ShrinkBudget o = Owner;
            lock (o.sync)
            {
                return o.attempts;
            }
        }
    }

    /// <summary>Elements charged to this node so far.</summary>
    internal long Work
    {
        get
        {
            ShrinkBudget o = Owner;
            lock (o.sync)
            {
                return o.work;
            }
        }
    }

    /// <summary>
    /// Charges one attempt over a candidate of <paramref name="elements"/> elements (at least 1)
    /// to this node and every ancestor. Returns false, charging nothing, when any of them cannot
    /// afford it: callers must check this BEFORE building the candidate.
    /// </summary>
    internal bool Charge(long elements) => Take(elements, 1);

    /// <summary>
    /// Charges a copy that is not itself a property evaluation (an initial copy, a
    /// list-to-array conversion): work only, so attempt counts are unchanged.
    /// </summary>
    internal bool ChargeCopy(long elements) => Take(elements, 0);

    private bool Take(long elements, int attemptCost)
    {
        ShrinkBudget start = Owner;
        long cost = Math.Max(1, elements);
        lock (start.sync)
        {
            for (ShrinkBudget? n = start; n != null; n = n.parent)
            {
                // Overflow-safe: work <= maxWork always, so the subtraction cannot wrap.
                if (n.attempts >= n.maxAttempts || cost > n.maxWork - n.work)
                {
                    start.root.refused = true;
                    return false;
                }
            }

            for (ShrinkBudget? n = start; n != null; n = n.parent)
            {
                n.attempts += attemptCost;
                n.work += cost;
            }

            return true;
        }
    }

    /// <summary>True while at least one more attempt of one element could be afforded everywhere on the path.</summary>
    internal bool HasRoom
    {
        get
        {
            ShrinkBudget start = Owner;
            lock (start.sync)
            {
                for (ShrinkBudget? n = start; n != null; n = n.parent)
                {
                    if (n.attempts >= n.maxAttempts || n.work >= n.maxWork)
                    {
                        return false;
                    }
                }

                return true;
            }
        }
    }

    /// <summary>Wraps a property so each evaluation is charged one attempt first (false once refused).</summary>
    internal Func<V, bool> Charged<V>(Func<V, bool> property)
    {
        return v => Charge(1) && property(v);
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (target != null)
        {
            return;
        }

        try
        {
            // Restore the ambient budget FIRST, so nothing below can leave a stale one.
            Ambient.Value = previous;
        }
        finally
        {
            closed = true;
            lock (sync)
            {
                Volatile.Write(ref lastWork, work);
                Volatile.Write(ref lastAttempts, attempts);
            }
        }
    }
}
