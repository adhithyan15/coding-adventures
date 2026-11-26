using System.Numerics;
using System.Text;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>
/// SplitMix64, bounded draws, FNV and sub-seeds. Vectors come from the independent
/// reference <c>code/scripts/pbt_conformance.py</c> (shared with the Java lane), never from
/// this engine's own output.
/// </summary>
public sealed class SplitMix64Tests : HookedTest
{
    private const ulong Gamma = 0x9e3779b97f4a7c15UL;

    private static ulong U(string decimalText) => ulong.Parse(decimalText, System.Globalization.CultureInfo.InvariantCulture);

    // ---- inverse of the finalizer ---------------------------------------------------

    // The finalizer is a bijection on 64 bits: each xor-shift is its own inverse after
    // enough rounds, each multiplier is odd (invertible mod 2^64). So for every wanted
    // output there is exactly one state whose next output is that value.
    private static ulong UnXorShift(ulong x, int shift)
    {
        ulong result = x;
        for (int i = 0; i < 64 / shift + 1; i++)
        {
            result = x ^ (result >> shift);
        }

        return result;
    }

    private static ulong ModInverse(ulong a)
    {
        ulong x = a; // Newton iteration: each step doubles the correct low bits
        for (int i = 0; i < 6; i++)
        {
            x = unchecked(x * (2 - (a * x)));
        }

        return x;
    }

    /// <summary>The seed whose very first NextU64 output equals <paramref name="output"/>.</summary>
    internal static ulong SeedWhoseNextOutputIs(ulong output)
    {
        ulong z = UnXorShift(output, 31);
        z = unchecked(z * ModInverse(0x94d049bb133111ebUL));
        z = UnXorShift(z, 27);
        z = unchecked(z * ModInverse(0xbf58476d1ce4e5b9UL));
        z = UnXorShift(z, 30);
        ulong seed = unchecked(z - Gamma); // state + gamma == z
        Assert.Equal(output, new SplitMix64(seed).NextU64());
        return seed;
    }

    // ---- basic vectors ----------------------------------------------------------------

    [Fact]
    public void PinnedVector()
    {
        SplitMix64 r = new(0);
        Assert.Equal(0xe220a8397b1dcdafUL, r.NextU64());
        Assert.Equal(0x6e789e6aa1b965f4UL, r.NextU64());
    }

    [Fact]
    public void StateRoundTrip()
    {
        SplitMix64 r = new(77);
        r.NextU64();
        ulong saved = r.State;
        ulong a = r.NextU64();
        r.State = saved;
        Assert.Equal(a, r.NextU64());
    }

    [Fact]
    public void BoundedSmallVector()
    {
        SplitMix64 r = new(5);
        foreach (ulong want in new ulong[] { 3, 5, 2, 2, 3 })
        {
            Assert.Equal(want, r.Bounded(7));
        }
    }

    // A span of 2^63+1 rejects raw values below (2^64 - n) mod n = 2^63 - 1, about half of
    // all draws. A threshold that is always 0 would change these.
    [Fact]
    public void BoundedWideThresholdIsApplied()
    {
        ulong n = (1UL << 63) + 1;
        SplitMix64 r = new(5);
        foreach (string want in new[] { "4654242949169100535", "8957066056171264800", "204786321411665706", "1908141438795372386", "6903704994719435514" })
        {
            Assert.Equal(U(want), r.Bounded(n));
        }
    }

    [Fact]
    public void BoundedOneConsumesOneWord()
    {
        SplitMix64 r = new(5);
        for (int i = 0; i < 3; i++)
        {
            Assert.Equal(0UL, r.Bounded(1));
        }

        Assert.Equal(U("15755400384260043844"), r.State);
    }

    [Fact]
    public void BoundedZeroIsRejected()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new SplitMix64(1).Bounded(0));
    }

    [Fact]
    public void BoundedStaysInRangeAcrossSeeds()
    {
        foreach (ulong n in new ulong[] { 1, 2, 3, 7, 255, 256, 1000, 1UL << 32, (1UL << 63) + 1, ulong.MaxValue })
        {
            for (ulong seed = 0; seed < 50; seed++)
            {
                SplitMix64 r = new(seed);
                for (int i = 0; i < 20; i++)
                {
                    Assert.True(r.Bounded(n) < n);
                }
            }
        }
    }

    [Fact]
    public void BoundedIsRoughlyUniform()
    {
        SplitMix64 r = new(1234);
        int[] counts = new int[6];
        const int draws = 60000;
        for (int i = 0; i < draws; i++)
        {
            counts[r.Bounded(6)]++;
        }

        foreach (int c in counts)
        {
            Assert.InRange(c, (draws / 6) - 500, (draws / 6) + 500);
        }
    }

    // ---- the rejection branch ------------------------------------------------------------

    [Fact]
    public void ThresholdFormulaIsTheUnsignedTwoToThe64MinusNModN()
    {
        Assert.Equal(1UL, SplitMix64.Threshold(3));
        Assert.Equal((1UL << 63) - 1, SplitMix64.Threshold((1UL << 63) + 1));
        Assert.Equal(0UL, SplitMix64.Threshold(1));
        Assert.Equal(0UL, SplitMix64.Threshold(256));
        Assert.Equal(1UL, SplitMix64.Threshold(ulong.MaxValue));
        for (ulong n = 1; n < 300; n++)
        {
            BigInteger want = ((BigInteger.One << 64) - n) % n;
            Assert.Equal(want, new BigInteger(SplitMix64.Threshold(n)));
        }
    }

    [Fact]
    public void RejectionBranchFiresExactlyBelowTheThreshold()
    {
        // n = 3: threshold (2^64 - 3) mod 3 = 1, so only the raw word 0 is rejected.
        ulong seed = SeedWhoseNextOutputIs(0);
        Assert.Equal(unchecked(0UL - Gamma), seed); // output 0 needs state 0 before the add
        SplitMix64 r = new(seed);
        SplitMix64 reference = new(seed);
        reference.NextU64(); // the rejected word
        ulong second = reference.NextU64();
        Assert.Equal(second % 3, r.Bounded(3));
        Assert.Equal(reference.State, r.State); // a rejected word is consumed, then exactly one more

        // The raw word 1 is the first accepted one: no second draw.
        ulong seedOne = SeedWhoseNextOutputIs(1);
        SplitMix64 accepted = new(seedOne);
        Assert.Equal(1UL, accepted.Bounded(3));
        Assert.Equal(unchecked(seedOne + Gamma), accepted.State);
    }

    [Fact]
    public void WideThresholdBoundaryIsExact()
    {
        ulong n = (1UL << 63) + 1;
        ulong threshold = (1UL << 63) - 1; // (2^64 - n) mod n

        // word == threshold is accepted and reduced mod n.
        SplitMix64 on = new(SeedWhoseNextOutputIs(threshold));
        Assert.Equal(threshold, on.Bounded(n));

        // word == threshold - 1 is rejected: a second word is consumed.
        ulong seed = SeedWhoseNextOutputIs(threshold - 1);
        SplitMix64 below = new(seed);
        ulong got = below.Bounded(n);
        Assert.NotEqual(unchecked(seed + Gamma), below.State);
        SplitMix64 reference = new(seed);
        reference.NextU64();
        Assert.Equal(reference.NextU64() % n, got);

        // A top-bit-set word is far above the threshold (unsigned compare, not signed).
        const ulong high = 0xfedcba9876543210UL;
        SplitMix64 top = new(SeedWhoseNextOutputIs(high));
        Assert.Equal(high % n, top.Bounded(n));
    }

    // ---- wide spans -------------------------------------------------------------------------

    [Fact]
    public void BoundedBigSmallSpansMatchBounded()
    {
        SplitMix64 a = new(11);
        SplitMix64 b = new(11);
        for (int i = 0; i < 50; i++)
        {
            Assert.Equal(new BigInteger(b.Bounded(1000)), a.BoundedBig(new BigInteger(1000)));
        }
    }

    [Fact]
    public void BoundedBigFullWordSpanIsRawWord()
    {
        SplitMix64 a = new(3);
        SplitMix64 b = new(3);
        Assert.Equal(new BigInteger(b.NextU64()), a.BoundedBig(BigInteger.One << 64));
    }

    [Fact]
    public void BoundedBigWideVectors()
    {
        SplitMix64 r = new(9);
        BigInteger span = (BigInteger.One << 130) + 7;
        foreach (string want in new[]
        {
            "255448235011303640280899557894340534535",
            "89342299821301620937131352229074699094",
            "334684220971821213071345962061252959748",
        })
        {
            Assert.Equal(Fx.Big(want), r.BoundedBig(span));
        }

        r = new SplitMix64(9);
        span = new BigInteger(3) << 126;
        foreach (string want in new[]
        {
            "232196006557297741543928818731175635554",
            "90284548084330202170444353266735210592",
            "89342299821301620962466552557356346366",
        })
        {
            Assert.Equal(Fx.Big(want), r.BoundedBig(span));
        }
    }

    // VER05 section 7.2: raw % span over-weights the low third of a 3*2^126 span.
    [Fact]
    public void BoundedBigThirdsAreUnbiased()
    {
        BigInteger span = new BigInteger(3) << 126;
        BigInteger third = BigInteger.One << 126;
        SplitMix64 r = new(2024);
        int[] counts = new int[3];
        const int draws = 30000;
        for (int i = 0; i < draws; i++)
        {
            BigInteger v = r.BoundedBig(span);
            Assert.True(v.Sign >= 0 && v < span);
            counts[(int)(v / third)]++;
        }

        foreach (int c in counts)
        {
            Assert.True(Math.Abs(c - (draws / 3)) < 700, "third count " + c);
        }
    }

    [Fact]
    public void BoundedBigRejectsBadSpan()
    {
        foreach (BigInteger span in new[] { BigInteger.Zero, new BigInteger(-5) })
        {
            Assert.Throws<ArgumentOutOfRangeException>(() => new SplitMix64(1).BoundedBig(span));
        }
    }

    // ---- fnv and sub-seeds ---------------------------------------------------------------------

    [Fact]
    public void FnvVectors()
    {
        Assert.Equal(0xcbf29ce484222325UL, SplitMix64.Fnv1a64(string.Empty));
        Assert.Equal(0xaf63dc4c8601ec8cUL, SplitMix64.Fnv1a64("a"));
        Assert.Equal(0x85944171f73967e8UL, SplitMix64.Fnv1a64("foobar"));
        Assert.Equal(0x85944171f73967e8UL, SplitMix64.Fnv1a64(Encoding.UTF8.GetBytes("foobar")));

        // Bytes above 0x7f must be hashed as unsigned values (a sign-extended byte is a classic bug).
        Assert.Equal(SplitMix64.Fnv1a64(new byte[] { 0xc3, 0xa9 }), SplitMix64.Fnv1a64("é"));
        Assert.NotEqual(SplitMix64.Fnv1a64(new byte[] { 0xff }), SplitMix64.Fnv1a64(new byte[] { 0x7f }));
    }

    [Fact]
    public void SubSeedVectors()
    {
        Assert.Equal(0x338262d8f096398fUL, SplitMix64.SubSeed(0, "x"));
        Assert.Equal(0xcca3a0330363d462UL, SplitMix64.SubSeed(42, "prop/ct_compare_eq_reflexive"));
        Assert.Equal(0x30ef66c3e79dda4eUL, SplitMix64.SubSeed(ulong.MaxValue, string.Empty));

        // Without the final scramble the answer would be run XOR hash (TS/Dart lanes).
        Assert.NotEqual(SplitMix64.Fnv1a64("x"), SplitMix64.SubSeed(0, "x"));
    }
}
