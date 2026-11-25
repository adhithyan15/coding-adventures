using System.Numerics;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Pbt;

/// <summary>
/// Sabotage hooks. These are the only production indirections added for the
/// negative controls of VER05g section 6: a test swaps one, runs the corpus, and the
/// runner must report problems. Production code never reassigns them, and the class
/// is internal (visible to the test assembly only), so no consumer can.
/// </summary>
internal static class Hooks
{
    /// <summary>The bisection move: <c>current - floor(diff / 2)</c>.</summary>
    internal static volatile Func<BigInteger, BigInteger, BigInteger> BisectCandidate = DefaultBisect;

    /// <summary>The unit move: one step toward the target.</summary>
    internal static volatile Func<BigInteger, BigInteger, BigInteger> UnitCandidate = DefaultUnit;

    /// <summary>Disables the single-deletion phase of sequence shrinking.</summary>
    internal static volatile bool SkipDeletionPhase;

    /// <summary>Builds the stream for a seed.</summary>
    internal static volatile Func<ulong, SplitMix64> NewRng = DefaultRng;

    /// <summary>Restores every hook to its production value.</summary>
    internal static void Reset()
    {
        BisectCandidate = DefaultBisect;
        UnitCandidate = DefaultUnit;
        SkipDeletionPhase = false;
        NewRng = DefaultRng;
        ShrinkBudget.DefaultMaxWork = Limits.MaxShrinkWork;
    }

    // BigInteger's >> floors toward negative infinity, as the corpus requires.
    private static BigInteger DefaultBisect(BigInteger current, BigInteger diff) => current - (diff >> 1);

    private static BigInteger DefaultUnit(BigInteger current, BigInteger diff) => current - diff.Sign;

    private static SplitMix64 DefaultRng(ulong seed) => new(seed);
}
