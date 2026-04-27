namespace CodingAdventures.Rng.Tests;

// RngTests.cs — xUnit tests for Lcg, Xorshift64, and Pcg32
// =========================================================
//
// Three categories of tests for each generator:
//
//   1. Known-vector tests — verify the first three outputs for seed=1
//      against the reference Go implementation. These catch any arithmetic
//      error immediately.
//
//   2. Property tests — statistical properties that must hold regardless of
//      exact values: floats in [0,1), range bounds, u64 construction, etc.
//
//   3. Behavioural tests — seed=0 fixup for Xorshift64, determinism
//      (same seed → same sequence), independence (different seeds diverge).

public sealed class LcgTests
{
    // Reference values computed from the Go implementation with seed=1.
    // LCG: [1817669548, 2187888307, 2784682393]
    private const uint LcgSeed1Out0 = 1817669548;
    private const uint LcgSeed1Out1 = 2187888307;
    private const uint LcgSeed1Out2 = 2784682393;

    [Fact]
    public void NextU32_MatchesKnownVectors_Seed1()
    {
        var g = new Lcg(1);
        Assert.Equal(LcgSeed1Out0, g.NextU32());
        Assert.Equal(LcgSeed1Out1, g.NextU32());
        Assert.Equal(LcgSeed1Out2, g.NextU32());
    }

    [Fact]
    public void NextU32_Seed0_IsDeterministic()
    {
        var a = new Lcg(0);
        var b = new Lcg(0);
        for (int i = 0; i < 10; i++)
        {
            Assert.Equal(a.NextU32(), b.NextU32());
        }
    }

    [Fact]
    public void NextU32_DifferentSeeds_ProduceDifferentSequences()
    {
        var a = new Lcg(1);
        var b = new Lcg(2);
        Assert.NotEqual(a.NextU32(), b.NextU32());
    }

    [Fact]
    public void NextU64_EqualsHiLoCombination_Seed1()
    {
        // NextU64 must compose two NextU32 calls as (hi << 32) | lo.
        var g1 = new Lcg(1);
        ulong u64 = g1.NextU64();

        var g2 = new Lcg(1);
        ulong hi = (ulong)g2.NextU32();
        ulong lo = (ulong)g2.NextU32();
        Assert.Equal((hi << 32) | lo, u64);
    }

    [Fact]
    public void NextFloat_IsInUnitInterval()
    {
        var g = new Lcg(42);
        for (int i = 0; i < 1000; i++)
        {
            double f = g.NextFloat();
            Assert.True(f >= 0.0 && f < 1.0, $"Out of [0,1): {f}");
        }
    }

    [Fact]
    public void NextFloat_SpansBothHalves_Seed1()
    {
        // A healthy sequence should contain values both below and above 0.5.
        var g = new Lcg(1);
        bool sawLow = false, sawHigh = false;
        for (int i = 0; i < 100; i++)
        {
            double f = g.NextFloat();
            if (f < 0.5) sawLow = true;
            if (f >= 0.5) sawHigh = true;
        }
        Assert.True(sawLow && sawHigh);
    }

    [Fact]
    public void NextIntInRange_StaysWithinBounds()
    {
        var g = new Lcg(7);
        for (int i = 0; i < 1000; i++)
        {
            long v = g.NextIntInRange(1, 6);
            Assert.InRange(v, 1L, 6L);
        }
    }

    [Fact]
    public void NextIntInRange_CoversAllValues_1to6()
    {
        var g = new Lcg(0);
        var seen = new System.Collections.Generic.HashSet<long>();
        for (int i = 0; i < 10000; i++)
        {
            seen.Add(g.NextIntInRange(1, 6));
        }
        for (long v = 1; v <= 6; v++)
        {
            Assert.Contains(v, seen);
        }
    }

    [Fact]
    public void NextIntInRange_SingleValueRange_AlwaysReturnsThatValue()
    {
        var g = new Lcg(123);
        for (int i = 0; i < 20; i++)
        {
            Assert.Equal(42L, g.NextIntInRange(42, 42));
        }
    }

    [Fact]
    public void NextIntInRange_NegativeRange_WorksCorrectly()
    {
        var g = new Lcg(5);
        for (int i = 0; i < 500; i++)
        {
            long v = g.NextIntInRange(-10, -1);
            Assert.InRange(v, -10L, -1L);
        }
    }
}

public sealed class Xorshift64Tests
{
    // Reference values from Go implementation with seed=1.
    // Xorshift64: [1082269761, 201397313, 1854285353]
    private const uint XorSeed1Out0 = 1082269761;
    private const uint XorSeed1Out1 = 201397313;
    private const uint XorSeed1Out2 = 1854285353;

    [Fact]
    public void NextU32_MatchesKnownVectors_Seed1()
    {
        var g = new Xorshift64(1);
        Assert.Equal(XorSeed1Out0, g.NextU32());
        Assert.Equal(XorSeed1Out1, g.NextU32());
        Assert.Equal(XorSeed1Out2, g.NextU32());
    }

    [Fact]
    public void Seed0_IsReplacedWith1_ProducingSameSequenceAsSeed1()
    {
        // Seed 0 is a fixed point; we normalise it to 1.
        var g0 = new Xorshift64(0);
        var g1 = new Xorshift64(1);
        Assert.Equal(g1.NextU32(), g0.NextU32());
        Assert.Equal(g1.NextU32(), g0.NextU32());
    }

    [Fact]
    public void NextU32_Deterministic_SameSeed()
    {
        var a = new Xorshift64(99);
        var b = new Xorshift64(99);
        for (int i = 0; i < 20; i++)
        {
            Assert.Equal(a.NextU32(), b.NextU32());
        }
    }

    [Fact]
    public void NextU64_EqualsHiLoCombination()
    {
        var g1 = new Xorshift64(17);
        ulong u64 = g1.NextU64();

        var g2 = new Xorshift64(17);
        ulong hi = (ulong)g2.NextU32();
        ulong lo = (ulong)g2.NextU32();
        Assert.Equal((hi << 32) | lo, u64);
    }

    [Fact]
    public void NextFloat_IsInUnitInterval()
    {
        var g = new Xorshift64(42);
        for (int i = 0; i < 1000; i++)
        {
            double f = g.NextFloat();
            Assert.True(f >= 0.0 && f < 1.0, $"Out of [0,1): {f}");
        }
    }

    [Fact]
    public void NextIntInRange_StaysWithinBounds_LargeRange()
    {
        var g = new Xorshift64(3);
        for (int i = 0; i < 1000; i++)
        {
            long v = g.NextIntInRange(0, 99);
            Assert.InRange(v, 0L, 99L);
        }
    }

    [Fact]
    public void NextIntInRange_CoversAllValues_1to6()
    {
        var g = new Xorshift64(0);
        var seen = new System.Collections.Generic.HashSet<long>();
        for (int i = 0; i < 10000; i++)
        {
            seen.Add(g.NextIntInRange(1, 6));
        }
        for (long v = 1; v <= 6; v++)
        {
            Assert.Contains(v, seen);
        }
    }

    [Fact]
    public void NextIntInRange_SingleValueRange()
    {
        var g = new Xorshift64(7);
        for (int i = 0; i < 20; i++)
        {
            Assert.Equal(-5L, g.NextIntInRange(-5, -5));
        }
    }
}

public sealed class Pcg32Tests
{
    // Reference values from Go implementation with seed=1.
    // PCG32: [1412771199, 1791099446, 124312908]
    private const uint PcgSeed1Out0 = 1412771199;
    private const uint PcgSeed1Out1 = 1791099446;
    private const uint PcgSeed1Out2 = 124312908;

    [Fact]
    public void NextU32_MatchesKnownVectors_Seed1()
    {
        var g = new Pcg32(1);
        Assert.Equal(PcgSeed1Out0, g.NextU32());
        Assert.Equal(PcgSeed1Out1, g.NextU32());
        Assert.Equal(PcgSeed1Out2, g.NextU32());
    }

    [Fact]
    public void NextU32_Seed0_IsDeterministic()
    {
        var a = new Pcg32(0);
        var b = new Pcg32(0);
        for (int i = 0; i < 10; i++)
        {
            Assert.Equal(a.NextU32(), b.NextU32());
        }
    }

    [Fact]
    public void NextU32_DifferentSeeds_DifferentFirstOutput()
    {
        var a = new Pcg32(1);
        var b = new Pcg32(2);
        Assert.NotEqual(a.NextU32(), b.NextU32());
    }

    [Fact]
    public void NextU64_EqualsHiLoCombination()
    {
        var g1 = new Pcg32(55);
        ulong u64 = g1.NextU64();

        var g2 = new Pcg32(55);
        ulong hi = (ulong)g2.NextU32();
        ulong lo = (ulong)g2.NextU32();
        Assert.Equal((hi << 32) | lo, u64);
    }

    [Fact]
    public void NextFloat_IsInUnitInterval()
    {
        var g = new Pcg32(42);
        for (int i = 0; i < 1000; i++)
        {
            double f = g.NextFloat();
            Assert.True(f >= 0.0 && f < 1.0, $"Out of [0,1): {f}");
        }
    }

    [Fact]
    public void NextFloat_SpansBothHalves_LongRun()
    {
        var g = new Pcg32(1);
        bool sawLow = false, sawHigh = false;
        for (int i = 0; i < 100; i++)
        {
            double f = g.NextFloat();
            if (f < 0.5) sawLow = true;
            if (f >= 0.5) sawHigh = true;
        }
        Assert.True(sawLow && sawHigh);
    }

    [Fact]
    public void NextIntInRange_StaysWithinBounds()
    {
        var g = new Pcg32(9);
        for (int i = 0; i < 1000; i++)
        {
            long v = g.NextIntInRange(1, 6);
            Assert.InRange(v, 1L, 6L);
        }
    }

    [Fact]
    public void NextIntInRange_CoversAllValues_1to6()
    {
        var g = new Pcg32(0);
        var seen = new System.Collections.Generic.HashSet<long>();
        for (int i = 0; i < 10000; i++)
        {
            seen.Add(g.NextIntInRange(1, 6));
        }
        for (long v = 1; v <= 6; v++)
        {
            Assert.Contains(v, seen);
        }
    }

    [Fact]
    public void NextIntInRange_SingleValueRange()
    {
        var g = new Pcg32(111);
        for (int i = 0; i < 20; i++)
        {
            Assert.Equal(100L, g.NextIntInRange(100, 100));
        }
    }

    [Fact]
    public void NextIntInRange_NegativeRange()
    {
        var g = new Pcg32(22);
        for (int i = 0; i < 500; i++)
        {
            long v = g.NextIntInRange(-20, -10);
            Assert.InRange(v, -20L, -10L);
        }
    }

    // Cross-generator sanity: all three should produce different sequences
    // from the same seed (they use different algorithms).
    [Fact]
    public void AllThreeGenerators_ProduceDifferentOutputsForSameSeed()
    {
        var lcg  = new Lcg(1);
        var xor  = new Xorshift64(1);
        var pcg  = new Pcg32(1);

        uint lcgVal = lcg.NextU32();
        uint xorVal = xor.NextU32();
        uint pcgVal = pcg.NextU32();

        // They should all be different (this is essentially guaranteed by design).
        Assert.NotEqual(lcgVal, xorVal);
        Assert.NotEqual(xorVal, pcgVal);
        Assert.NotEqual(lcgVal, pcgVal);
    }
}
