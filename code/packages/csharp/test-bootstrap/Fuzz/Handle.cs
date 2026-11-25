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
            Which = budget;
        }

        internal Handle Owner { get; }

        internal Budget Which { get; }
    }

    internal Handle(int stepBudget, int memBudget)
    {
        StepBudget = stepBudget;
        MemBudget = memBudget;
    }

    internal Bitmap? Cov { get; set; }

    internal int Steps { get; set; }

    internal int StepBudget { get; }

    internal long Reserved { get; set; }

    internal int MemBudget { get; }

    /// <summary>
    /// The sentinel of the FIRST breach of either budget. Exhaustion is sticky and shared: once set,
    /// <see cref="Tick"/> and <see cref="Reserve"/> both rethrow it at once, so a target that
    /// swallows one breach and calls the other budget's hook is still stopped and reported as the
    /// budget that ran out first.
    /// </summary>
    internal BudgetExceeded? Exhausted { get; set; }

    /// <summary>Clears the per-execution state (the campaign calls this before every run).</summary>
    internal void Begin(Bitmap cov)
    {
        Cov = cov;
        Steps = 0;
        Reserved = 0;
        Exhausted = null;
    }

    /// <summary>
    /// A stable edge id derived from a name (FNV-1a), so a campaign is deterministic across runs and
    /// machines (no identity hash codes, no hash randomisation).
    /// </summary>
    public static ulong EdgeId(string label) => SplitMix64.Fnv1a64(label);

    /// <summary>Reports that the target executed the program point with this id (reduced mod 65,536).</summary>
    public void Edge(ulong id)
    {
        Cov?.Record(id);
    }

    /// <summary>
    /// Counts one unit of work. After <c>StepBudget</c> ticks the execution is a timeout:
    /// <c>Tick</c> throws a private sentinel, and keeps throwing on every later call. A target that
    /// loops without ever ticking cannot be stopped (the engine starts no thread), so CI needs an
    /// external wall-clock limit.
    /// </summary>
    public void Tick()
    {
        if (Exhausted != null)
        {
            throw Exhausted;
        }

        if (Steps >= StepBudget)
        {
            Exhausted = new BudgetExceeded(this, Budget.Steps);
            throw Exhausted;
        }

        Steps++;
    }

    /// <summary>
    /// Declares an allocation of <paramref name="n"/> bytes about to be made on behalf of the input.
    /// When the running total of one execution exceeds <c>MemBudget</c> the execution is an
    /// out-of-memory outcome. A non-positive <paramref name="n"/> reserves nothing. Call it before
    /// <c>new byte[n]</c> sized by input data.
    /// </summary>
    public void Reserve(long n)
    {
        if (Exhausted != null)
        {
            throw Exhausted;
        }

        if (n <= 0)
        {
            return;
        }

        if (n > MemBudget - Reserved)
        {
            Exhausted = new BudgetExceeded(this, Budget.Memory);
            throw Exhausted;
        }

        Reserved += n;
    }
}
