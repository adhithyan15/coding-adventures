using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Fuzz;

/// <summary>
/// What a <see cref="Target"/> sees: coverage reporting and two deterministic budgets
/// (VER06h sections 4.2 and 5). One handle is reused across executions; a target must not retain it.
/// </summary>
public sealed class Handle
{
    /// <summary>Which budget a sentinel stands for.</summary>
    internal enum Budget
    {
        Steps,
        Memory,

        /// <summary>The handle was retired: its execution is over and nothing may use it any more.</summary>
        Retired,
    }

    /// <summary>
    /// Thrown by <see cref="Tick"/> and <see cref="Reserve"/>. Unlike Java, .NET has no throwable
    /// that <c>catch (Exception)</c> misses, so a target CAN swallow this: the guarantee comes from
    /// stickiness (every later call rethrows) and the executor's post-run check of
    /// <see cref="Exhausted"/>, never from the exception type. It carries the handle that raised it
    /// so an inner campaign never claims an outer campaign's budget.
    /// </summary>
    internal sealed class BudgetExceeded : Exception
    {
        internal BudgetExceeded(Handle owner, Budget budget)
            : base("fuzz budget exhausted")
        {
            Owner = owner;
            Campaign = owner.Campaign;
            Epoch = owner.Epoch;
            Which = budget;
        }

        internal Handle Owner { get; }

        /// <summary>The campaign that raised it: an inner campaign never claims an outer one's sentinel.</summary>
        internal object Campaign { get; }

        /// <summary>The execution that raised it: a sentinel caught and rethrown in a LATER run is stale.</summary>
        internal long Epoch { get; }

        internal Budget Which { get; }
    }

    private readonly object _gate = new();
    private Bitmap? _cov;
    private bool _retired;
    private int _steps;
    private long _reserved;
    private BudgetExceeded? _exhausted;

    internal Handle(int stepBudget, int memBudget)
        : this(stepBudget, memBudget, new object(), 0)
    {
    }

    /// <summary>
    /// A handle for ONE execution. The campaign allocates a fresh one per run, so a continuation or
    /// task that outlives the run (an <c>async void</c> target, a spawned <c>Task</c>) can only ever
    /// touch its own, retired handle, never the next run's counters or coverage map.
    /// </summary>
    internal Handle(int stepBudget, int memBudget, object campaign, long epoch)
    {
        StepBudget = stepBudget;
        MemBudget = memBudget;
        Campaign = campaign;
        Epoch = epoch;
    }

    /// <summary>The campaign this handle belongs to (an identity token).</summary>
    internal object Campaign { get; }

    /// <summary>Which execution of the campaign this handle served.</summary>
    internal long Epoch { get; }

    internal Bitmap? Cov
    {
        get
        {
            lock (_gate)
            {
                return _cov;
            }
        }

        set
        {
            lock (_gate)
            {
                _cov = value;
            }
        }
    }

    /// <summary>Whether <see cref="Retire"/> has run.</summary>
    internal bool Retired
    {
        get
        {
            lock (_gate)
            {
                return _retired;
            }
        }
    }

    /// <summary>
    /// Ends the handle's life (the executor calls it in a <c>finally</c> after the target). Afterwards
    /// <see cref="Edge"/> does nothing, and <see cref="Tick"/> and <see cref="Reserve"/> throw a
    /// retired sentinel. Taking the same lock as <see cref="Edge"/> means no write to the coverage
    /// map can be in flight once this returns.
    /// </summary>
    internal void Retire()
    {
        lock (_gate)
        {
            _retired = true;
            _cov = null;
        }
    }

    internal int Steps
    {
        get
        {
            lock (_gate)
            {
                return _steps;
            }
        }

        set
        {
            lock (_gate)
            {
                _steps = value;
            }
        }
    }

    internal int StepBudget { get; }

    internal long Reserved
    {
        get
        {
            lock (_gate)
            {
                return _reserved;
            }
        }

        set
        {
            lock (_gate)
            {
                _reserved = value;
            }
        }
    }

    internal int MemBudget { get; }

    /// <summary>
    /// The sentinel of the FIRST breach of either budget. Exhaustion is sticky and shared: once set,
    /// <see cref="Tick"/> and <see cref="Reserve"/> both rethrow it at once, so a target that
    /// swallows one breach and calls the other budget's hook is still stopped and reported as the
    /// budget that ran out first.
    /// </summary>
    internal BudgetExceeded? Exhausted
    {
        get
        {
            lock (_gate)
            {
                return _exhausted;
            }
        }

        set
        {
            lock (_gate)
            {
                _exhausted = value;
            }
        }
    }

    /// <summary>
    /// A stable edge id derived from a name (FNV-1a), so a campaign is deterministic across runs and
    /// machines (no identity hash codes, no hash randomisation).
    /// </summary>
    public static ulong EdgeId(string label) => SplitMix64.Fnv1a64(label);

    /// <summary>Reports that the target executed the program point with this id (reduced mod 65,536).</summary>
    public void Edge(ulong id)
    {
        lock (_gate)
        {
            _cov?.Record(id);
        }
    }

    /// <summary>
    /// Counts one unit of work. After <c>StepBudget</c> ticks the execution is a timeout:
    /// <c>Tick</c> throws a private sentinel, and keeps throwing on every later call. A target that
    /// loops without ever ticking cannot be stopped (the engine starts no thread), so CI needs an
    /// external wall-clock limit.
    /// </summary>
    public void Tick()
    {
        // The whole body runs under the lock, retired check included: a stale thread racing
        // Retire() either finishes before it or is refused after it, and cannot update the counters
        // of a handle the executor has already read.
        lock (_gate)
        {
            if (_retired)
            {
                throw new BudgetExceeded(this, Budget.Retired);
            }

            if (_exhausted != null)
            {
                throw _exhausted;
            }

            if (_steps >= StepBudget)
            {
                _exhausted = new BudgetExceeded(this, Budget.Steps);
                throw _exhausted;
            }

            _steps++;
        }
    }

    /// <summary>
    /// Declares an allocation of <paramref name="n"/> bytes about to be made on behalf of the input.
    /// When the running total of one execution exceeds <c>MemBudget</c> the execution is an
    /// out-of-memory outcome. A non-positive <paramref name="n"/> reserves nothing. Call it before
    /// <c>new byte[n]</c> sized by input data.
    /// </summary>
    public void Reserve(long n)
    {
        lock (_gate)
        {
            if (_retired)
            {
                throw new BudgetExceeded(this, Budget.Retired);
            }

            if (_exhausted != null)
            {
                throw _exhausted;
            }

            if (n <= 0)
            {
                return;
            }

            if (n > MemBudget - _reserved)
            {
                _exhausted = new BudgetExceeded(this, Budget.Memory);
                throw _exhausted;
            }

            _reserved += n;
        }
    }
}
