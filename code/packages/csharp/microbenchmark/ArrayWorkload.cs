using System.Diagnostics;
using System.Runtime.InteropServices;

namespace CodingAdventures.Microbenchmark;

/// <summary>The fixed indexed-array reference workload for VER07.</summary>
public static class ArrayWorkload
{
    public const int Length = 4096;
    public const long Checksum = 511_533;
    public const string WorkloadId = "indexed-array-sum-4096";

    /// <summary>Creates the deterministic input before any timing begins.</summary>
    public static int[] CreateArray() => Enumerable.Range(0, Length).Select(i => (17 * i + 3) % 251).ToArray();

    /// <summary>Sums the array through indexed access for the complete batch.</summary>
    public static long IndexedBatch(int[] values, long operations)
    {
        ArgumentNullException.ThrowIfNull(values);
        if (operations < 0) throw new ArgumentOutOfRangeException(nameof(operations), "operations must be nonnegative");
        long total = 0;
        for (long operation = 0; operation < operations; operation++)
            for (var index = 0; index < values.Length; index++) total = checked(total + values[index]);
        return total;
    }

    /// <summary>Sums the same array through native element iteration.</summary>
    public static long IterationBatch(int[] values, long operations)
    {
        ArgumentNullException.ThrowIfNull(values);
        if (operations < 0) throw new ArgumentOutOfRangeException(nameof(operations), "operations must be nonnegative");
        long total = 0;
        for (long operation = 0; operation < operations; operation++)
            foreach (var value in values) total = checked(total + value);
        return total;
    }

    /// <summary>Runs the paired array comparison with caller-supplied clock and metadata.</summary>
    public static Report Run(Config config, Func<long> clock, string[] invocation, string revision, string profile)
    {
        var values = CreateArray();
        if (values.Length != Length || values.Sum(value => (long)value) != Checksum)
            throw new InvalidOperationException("indexed-array checksum definition changed");
        var report = Microbenchmark.Collect(
            config,
            Checksum,
            count => IndexedBatch(values, count),
            count => IterationBatch(values, count),
            clock);
        return new Report
        {
            Config = report.Config,
            BatchSize = report.BatchSize,
            Pairs = report.Pairs,
            Analysis = report.Analysis,
            Workload = WorkloadId,
            Invocation = invocation,
            Revision = revision,
            Runtime = RuntimeInformation.FrameworkDescription,
            Os = RuntimeInformation.OSDescription,
            Arch = RuntimeInformation.ProcessArchitecture.ToString().ToLowerInvariant(),
            Profile = profile,
        };
    }

    /// <summary>Provides a monotonic nanosecond clock based on Stopwatch.</summary>
    public static Func<long> CreateClock()
    {
        var start = Stopwatch.GetTimestamp();
        return () =>
        {
            try { return checked(Stopwatch.GetElapsedTime(start).Ticks * 100L); }
            catch (OverflowException) { return -1; }
        };
    }
}
