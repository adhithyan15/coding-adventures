using System.Numerics;
using CodingAdventures.TestBootstrap.Pbt;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>Integer, list and byte shrinking: exact minimals, step counts and error paths.</summary>
public sealed class ShrinkTests : HookedTest
{
    private static BigInteger Big(long v) => new(v);

    [Fact]
    public void MonotoneThresholdsLandExactlyOnT()
    {
        // VER05 section 7.3: for "fails iff x >= T" the shrunk witness is exactly T.
        for (long t = 1; t <= 120; t++)
        {
            long threshold = t;
            for (long start = t; start <= 400; start += 7)
            {
                Shrunk<BigInteger> r = Shrink.ShrinkInt(Big(start), BigInteger.Zero, Big(-1000), Big(1000), x => x < Big(threshold), 1000);
                Assert.Equal(Big(t), r.Value);
            }

            for (long start = -t; start >= -400; start -= 7)
            {
                Shrunk<BigInteger> r = Shrink.ShrinkInt(Big(start), BigInteger.Zero, Big(-1000), Big(1000), x => x > Big(-threshold), 1000);
                Assert.Equal(Big(-t), r.Value);
            }
        }
    }

    [Fact]
    public void ShrinkIntExtremesDoNotOverflow()
    {
        BigInteger min = -(BigInteger.One << 127);
        BigInteger max = (BigInteger.One << 127) - 1;

        // Everything fails: an opposite-extreme target wins the probe in one step.
        Assert.Equal(new Shrunk<BigInteger>(max, 1), Shrink.ShrinkInt(min, max, min, max, _ => false, 1000));
        Assert.Equal(new Shrunk<BigInteger>(min, 1), Shrink.ShrinkInt(max, min, min, max, _ => false, 1000));

        // Extreme start, in-range target: the walk crosses 2^127 without overflow.
        Assert.Equal(Big(-100), Shrink.ShrinkInt(min, BigInteger.Zero, min, max, x => x > Big(-100), 1000).Value);
        Assert.Equal(Big(100), Shrink.ShrinkInt(max, BigInteger.Zero, min, max, x => x < Big(100), 1000).Value);

        // Walking away from an out-of-range target never leaves [min, max].
        Shrunk<BigInteger> r = Shrink.ShrinkInt(min, max, min, max, x => x > Big(5), 1000);
        Assert.True(r.Value >= min && r.Value <= max);
    }

    [Fact]
    public void ShrinkIntBudgetAndErrors()
    {
        Func<BigInteger, bool> le100 = x => x <= Big(100);
        Assert.Equal(new Shrunk<BigInteger>(Big(54321), 0), Shrink.ShrinkInt(Big(54321), BigInteger.Zero, BigInteger.Zero, Big(100000), le100, 0));
        Assert.Equal(2, Shrink.ShrinkInt(Big(54321), BigInteger.Zero, BigInteger.Zero, Big(100000), le100, 2).Steps);
        Assert.Equal(
            new Shrunk<BigInteger>(BigInteger.Zero, 1),
            Shrink.ShrinkInt(Big(500), BigInteger.Zero, BigInteger.Zero, Big(1000), _ => false, 10));

        Assert.Throws<NotFalsifyingException>(() => Shrink.ShrinkInt(Big(5), BigInteger.Zero, BigInteger.Zero, Big(10), le100, 10));
        Assert.Throws<ArgumentException>(() => Shrink.ShrinkInt(Big(500), BigInteger.Zero, BigInteger.Zero, Big(10), le100, 10));
        Assert.Throws<ArgumentException>(() => Shrink.ShrinkInt(Big(500), BigInteger.Zero, Big(10), Big(0), le100, 10));
        Assert.Throws<ArgumentNullException>(() => Shrink.ShrinkInt(Big(500), BigInteger.Zero, BigInteger.Zero, Big(1000), null!, 10));
        Assert.Throws<ArgumentOutOfRangeException>(() => Shrink.ShrinkInt(Big(500), BigInteger.Zero, BigInteger.Zero, Big(1000), le100, -1));
        Assert.Throws<ArgumentOutOfRangeException>(() => Shrink.ShrinkInt(Big(500), BigInteger.Zero, BigInteger.Zero, Big(1000), le100, Limits.MaxShrinkSteps + 1));
    }

    [Fact]
    public void ShrinkIntCorpusExampleTakesFifteenSteps()
    {
        Shrunk<BigInteger> r = Shrink.ShrinkInt(Big(54321), BigInteger.Zero, BigInteger.Zero, Big(100000), x => x <= Big(100), 1000);
        Assert.Equal(Big(101), r.Value);
        Assert.Equal(15, r.Steps);
    }

    [Fact]
    public void FloorDivisionOnNegativeDiffMatchesPythonAndGo()
    {
        // diff = -7: floor(-7/2) = -4, so current - (-4) = current + 4, not current + 3.
        Assert.Equal(Big(-6), Hooks.BisectCandidate(Big(-10), Big(-7)));
        Assert.Equal(Big(-9), Hooks.BisectCandidate(Big(-10), Big(-1))); // floor(-1/2) = -1
        Assert.Equal(Big(7), Hooks.BisectCandidate(Big(10), Big(7)));   // floor(7/2) = 3
    }

    [Fact]
    public void ShrinkersDoNotAliasTheCaller()
    {
        List<BigInteger> input = new() { Big(10), Big(5), Big(8), Big(4) };
        List<BigInteger> snapshot = new(input);
        Shrunk<List<BigInteger>> r = Shrink.ShrinkList(input, BigInteger.Zero, l => { l.Clear(); return l.Count == 0 && false; }, 1000);
        Assert.Equal(snapshot, input);
        Assert.NotSame(input, r.Value);

        byte[] bytes = { 1, 2, 3, 4, 5, 6 };
        byte[] copy = (byte[])bytes.Clone();
        Shrink.ShrinkBytes(bytes, b => { Array.Clear(b); return b.Length < 5; }, 1000);
        Assert.Equal(copy, bytes);
    }

    [Fact]
    public void ListAndBytesShrinkErrors()
    {
        Assert.Throws<NotFalsifyingException>(() => Shrink.ShrinkList(new List<BigInteger> { Big(1) }, BigInteger.Zero, _ => true, 10));
        Assert.Throws<NotFalsifyingException>(() => Shrink.ShrinkBytes(new byte[] { 1 }, _ => true, 10));
        Assert.Throws<ArgumentNullException>(() => Shrink.ShrinkList(null!, BigInteger.Zero, _ => false, 10));
        Assert.Throws<ArgumentNullException>(() => Shrink.ShrinkList(new List<BigInteger>(), BigInteger.Zero, null!, 10));
        Assert.Throws<ArgumentNullException>(() => Shrink.ShrinkBytes(null!, _ => false, 10));
        Assert.Throws<ArgumentNullException>(() => Shrink.ShrinkBytes(new byte[1], null!, 10));
        Assert.Throws<ArgumentOutOfRangeException>(() => Shrink.ShrinkBytes(new byte[] { 1 }, _ => false, -1));
        Assert.Throws<ArgumentOutOfRangeException>(() => Shrink.ShrinkList(new List<BigInteger> { Big(1) }, BigInteger.Zero, _ => false, Limits.MaxShrinkSteps + 1));
    }

    [Fact]
    public void ZeroBudgetReturnsTheStartUnchanged()
    {
        Shrunk<List<BigInteger>> l = Shrink.ShrinkList(new List<BigInteger> { Big(5), Big(6) }, BigInteger.Zero, _ => false, 0);
        Assert.Equal(new List<BigInteger> { Big(5), Big(6) }, l.Value);
        Assert.Equal(0, l.Steps);
        Shrunk<byte[]> b = Shrink.ShrinkBytes(new byte[] { 5, 6 }, _ => false, 0);
        Assert.Equal(new byte[] { 5, 6 }, b.Value);
        Assert.Equal(0, b.Steps);
    }

    [Fact]
    public void SequenceShrinkingIsOneMinimalByDeletion()
    {
        // corpus list_sum_le_20 shape: the minimum cannot lose any element and stay failing.
        Func<List<BigInteger>, bool> prop = l => l.Aggregate(BigInteger.Zero, (a, b) => a + b) <= Big(20);
        Shrunk<List<BigInteger>> r = Shrink.ShrinkList(new List<BigInteger> { Big(10), Big(5), Big(8), Big(4) }, BigInteger.Zero, prop, 1000);
        Assert.False(prop(r.Value));
        for (int i = 0; i < r.Value.Count; i++)
        {
            List<BigInteger> smaller = new(r.Value);
            smaller.RemoveAt(i);
            Assert.True(prop(smaller), "deleting index " + i + " must make the property hold");
        }
    }

    [Fact]
    public void SimulateReplayIsDeterministicAndRejectsUnfalsifiable()
    {
        Shrink.Replay a = Shrink.SimulateReplay(42, x => x <= Big(100));
        Shrink.Replay b = Shrink.SimulateReplay(42, x => x <= Big(100));
        Assert.Equal(a, b);
        Assert.Equal(Big(101), a.Minimal);
        Assert.Throws<ArgumentException>(() => Shrink.SimulateReplay(1, _ => true));
        Assert.Throws<ArgumentNullException>(() => Shrink.SimulateReplay(1, null!));
    }

    [Fact]
    public void SequenceShrinkMinLenPrefixProbe()
    {
        // minLen 2: the length-2 prefix is probed and, when it fails, carried on from.
        Gen<List<byte>> g = Gens.ListOf(Gens.U8(), 2, 8);
        Shrunk<List<byte>> r = g.Shrink(new List<byte> { 9, 9, 9, 9, 9, 9 }, l => l.Count >= 2, 1000);
        Assert.Equal(new List<byte> { 0, 0 }, r.Value);
        Assert.Equal(3, r.Steps); // prefix probe, then each element's target probe
        Shrunk<List<byte>> kept = g.Shrink(new List<byte> { 9, 9, 9, 9 }, l => l.Count >= 2 && l[0] == 9, 1000);
        Assert.Equal(new List<byte> { 9, 0 }, kept.Value);
    }

    // ---- bisection: left half, then right half, with exact steps ------------------------

    /// <summary>Records every candidate the shrinker evaluates, in order.</summary>
    private static (Shrunk<List<BigInteger>> Result, List<string> Seen) Trace(List<BigInteger> start, Func<List<BigInteger>, bool> falsifies)
    {
        List<string> seen = new();
        Shrunk<List<BigInteger>> r = Shrink.ShrinkSequence<BigInteger>(
            start,
            c =>
            {
                seen.Add(string.Join(",", c));
                return falsifies(c);
            },
            1000,
            0,
            null);
        return (r, seen);
    }

    [Fact]
    public void BisectionAcceptsTheLeftHalfThenTheRightHalf()
    {
        // [0,7,0,0,0,0,0,0], failing iff it contains a 7.
        List<BigInteger> start = new() { 0, 7, 0, 0, 0, 0, 0, 0 };
        (Shrunk<List<BigInteger>> r, List<string> seen) = Trace(start, l => l.Contains(7));
        Assert.Equal(new List<BigInteger> { 7 }, r.Value);
        Assert.Equal(3, r.Steps);
        Assert.Equal(
            new[]
            {
                string.Empty,     // empty probe: does not fail
                "0,7,0,0",        // left half of 8: fails, accepted (step 1)
                "0,7",            // left half of 4: fails, accepted (step 2)
                "0",              // left half of 2: holds, rejected
                "7",              // right half of 2: fails, accepted (step 3)
            },
            seen);
    }

    [Fact]
    public void BisectionAcceptsTheRightHalfThenTheLeftHalf()
    {
        List<BigInteger> start = new() { 0, 0, 0, 0, 7, 0, 0, 0 };
        (Shrunk<List<BigInteger>> r, List<string> seen) = Trace(start, l => l.Contains(7));
        Assert.Equal(new List<BigInteger> { 7 }, r.Value);
        Assert.Equal(3, r.Steps);
        Assert.Equal(new[] { string.Empty, "0,0,0,0", "7,0,0,0", "7,0", "7" }, seen);
    }

    [Fact]
    public void BisectionThroughTheListAndBytesApisKeepsTheExactStepCounts()
    {
        List<BigInteger> ones = Enumerable.Repeat(BigInteger.One, 7).ToList();
        List<BigInteger> leftHeavy = new() { Big(9) };
        leftHeavy.AddRange(ones);
        Shrunk<List<BigInteger>> left = Shrink.ShrinkList(leftHeavy, BigInteger.Zero, l => l.Count == 0 || l[0] != Big(9), 1000);
        Assert.Equal(new List<BigInteger> { Big(9) }, left.Value);
        Assert.Equal(3, left.Steps);

        List<BigInteger> rightHeavy = new(ones) { Big(9) };
        Shrunk<List<BigInteger>> right = Shrink.ShrinkList(rightHeavy, BigInteger.Zero, l => l.Count == 0 || l[^1] != Big(9), 1000);
        Assert.Equal(new List<BigInteger> { Big(9) }, right.Value);
        Assert.Equal(3, right.Steps);

        Shrunk<byte[]> bytes = Gens.Bytes(0, 8).Shrink(new byte[] { 9, 1, 1, 1, 1, 1, 1, 1 }, b => b.Length > 0 && b[0] == 9, 1000);
        Assert.Equal(new byte[] { 9 }, bytes.Value);
        Assert.Equal(3, bytes.Steps);
    }

    [Fact]
    public void DeletionPhaseSurvivesAPropertyThatMutatesItsArgument()
    {
        Shrunk<List<BigInteger>> r = Shrink.ShrinkList(
            new List<BigInteger> { Big(7), Big(1), Big(1), Big(1), Big(7) },
            BigInteger.Zero,
            l =>
            {
                bool holds = l.Count(v => v == Big(7)) < 2;
                for (int i = 0; i < l.Count; i++)
                {
                    l[i] = Big(99);
                }

                return holds;
            },
            1000);
        Assert.Equal(new List<BigInteger> { Big(7), Big(7) }, r.Value);
    }
}
