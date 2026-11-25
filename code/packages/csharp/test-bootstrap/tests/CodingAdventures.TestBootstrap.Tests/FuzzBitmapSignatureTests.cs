using CodingAdventures.TestBootstrap.Fuzz;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>Bitmap, admission, energy, failure ids, signatures and the TAP line.</summary>
public sealed class FuzzBitmapSignatureTests
{
    // ---- bitmap ---------------------------------------------------------------------

    [Fact]
    public void BitmapRecordsMasksAndCounts()
    {
        Bitmap b = new();
        Assert.Equal(0, b.Count);
        b.Record(3);
        b.Record(3);
        b.Record(65_536 + 3); // masked to 3
        b.Record(ulong.MaxValue); // 2^64-1 mod 65536 = 65535
        Assert.True(b.Has(3));
        Assert.True(b.Has(65_535));
        Assert.False(b.Has(4));
        Assert.Equal(2, b.Count);
        Assert.Equal(new[] { 3, 65_535 }, b.Edges());
        byte[] raw = b.Raw();
        Assert.Equal(Bitmap.CoverageBytes, raw.Length);
        Assert.Equal(0b1000, raw[0]);
        Assert.Equal(0x80, raw[8191]);
        raw[0] = 0; // a copy
        Assert.Equal(0b1000, b.Raw()[0]);
        b.Reset();
        Assert.Equal(0, b.Count);
        Assert.False(b.Has(3));
        Assert.Empty(b.Edges());
    }

    [Fact]
    public void BitmapGrowsPastItsInitialEdgeBuffer()
    {
        Bitmap b = new();
        for (ulong i = 0; i < 1000; i += 3)
        {
            b.Record(i);
        }

        Assert.Equal(334, b.Count);
        Assert.Equal(334, b.Trace().Length);
        Bitmap c = new();
        c.Record(5000);
        c.Merge(b);
        Assert.Equal(335, c.Count);
    }

    [Fact]
    public void AdmitCountsUniqueNovelEdges()
    {
        Bitmap b = new();
        Bitmap.Admission a = b.Admit(new[] { 1, 2, 2, 3 });
        Assert.True(a.HasNew);
        Assert.Equal(3, a.CountNew);
        a = b.Admit(new[] { 1, 2 });
        Assert.False(a.HasNew);
        Assert.Equal(0, a.CountNew);
        a = b.Admit(new[] { 3, 9, 9 });
        Assert.Equal(1, a.CountNew);
    }

    [Fact]
    public void CorpusAdmitCountsUniqueNovelEdgesOverFullWords()
    {
        Bitmap.Novelty n = Bitmap.CorpusAdmit(new ulong[] { 1, 2, 3 }, new ulong[] { 2, 4 });
        Assert.True(n.Admitted);
        Assert.Equal(1, n.NewEdges);
        n = Bitmap.CorpusAdmit(new ulong[] { 1, 2, 3 }, new ulong[] { 1, 2 });
        Assert.False(n.Admitted);
        Assert.Equal(0, n.NewEdges);
        n = Bitmap.CorpusAdmit(new ulong[] { 1 }, new ulong[] { 7, 7, 7, 1 });
        Assert.Equal(1, n.NewEdges); // a repeated novel edge counts once
        n = Bitmap.CorpusAdmit(new ulong[] { 1 }, new ulong[] { 1UL + 65_536, ulong.MaxValue });
        Assert.Equal(2, n.NewEdges); // admission compares full words, it does not reduce mod 65536
        Assert.False(Bitmap.CorpusAdmit(Array.Empty<ulong>(), Array.Empty<ulong>()).Admitted);
    }

    /// <summary>
    /// The power schedule (PT-F7): fuzz-v1 pins no energy value. Expected values come from an
    /// independent Python evaluation of floor(base*(1+new/total)/sqrt(max(1,len)) + 0.5).
    /// </summary>
    [Theory]
    [InlineData(10, 4, 3, 6, 8)]
    [InlineData(5, 4, 0, 0, 3)] // 2.5 rounds up (half away from zero), not to even
    [InlineData(10, 1, 5, 0, 10)] // no edges: bonus 1.0
    [InlineData(0, 1_000_000, 1, 1, 1)]
    [InlineData(10, 0, 1, 1, 20)] // zero length counts as 1
    [InlineData(10, 100, 1, 2, 2)] // 1.5 rounds to 2
    [InlineData(10, 9, 2, 4, 5)]
    [InlineData(10, 16, 1, 3, 3)]
    [InlineData(7, 2, 1, 2, 7)]
    [InlineData(100, 25, 5, 10, 30)]
    [InlineData(10, 1, 6, 6, 20)]
    [InlineData(3, 1, 1, 2, 5)] // 4.5 rounds to 5
    [InlineData(1, 1, 0, 5, 1)]
    [InlineData(10, 2, 1, 1, 14)]
    public void EnergySchedule(int baseEnergy, int len, int newEdges, int total, int expected)
    {
        Assert.Equal(expected, Bitmap.CalculateEnergy(baseEnergy, len, newEdges, total));
    }

    [Fact]
    public void EnergyClampsAndIsMonotone()
    {
        Assert.Equal(Mutate.MaxIterations, Bitmap.CalculateEnergy(int.MaxValue, 1, 1, 1));
        Assert.True(Bitmap.CalculateEnergy(10, 4, 6, 6) > Bitmap.CalculateEnergy(10, 4, 0, 6));
        Assert.True(Bitmap.CalculateEnergy(10, 100, 1, 2) < Bitmap.CalculateEnergy(10, 1, 1, 2));
    }

    // ---- signatures -------------------------------------------------------------------

    [Fact]
    public void SignaturesArePinnedAgainstTheReference()
    {
        // Values computed by the independent oracle scripts/fuzz_conformance.py crash_signature.
        Assert.Equal("4eca7ae8", FuzzSignature.CrashSignature("panic", new[] { "eval", "step" }));
        Assert.Equal("4e61ea76", FuzzSignature.CrashSignature("timeout", Array.Empty<string>()));
        Assert.Equal("7146a410", FuzzSignature.CrashSignature("oom", Array.Empty<string>()));
        Assert.Equal("43a76e9e", FuzzSignature.CrashSignature("invalid", Array.Empty<string>()));
        Assert.Equal("35a55e82", FuzzSignature.CrashSignature("k", new[] { "f" }));
        Assert.Equal("258cf594", FuzzSignature.CrashSignature("System.InvalidOperationException", new[] { "Parser.Read", "Parser.Run" }));
        Assert.Equal("557f548e", FuzzSignature.CrashSignature("x", new[] { "a:b" }));
        Assert.Equal("557f548e", FuzzSignature.CrashSignature("x", new[] { "a", "b" })); // the documented, unescaped ambiguity
        Assert.Equal(
            (SplitMix64.Fnv1a64("panic:eval:step") & 0xFFFFFFFFUL).ToString("x8"),
            FuzzSignature.CrashSignature("panic", new[] { "eval", "step" }));
    }

    [Fact]
    public void DedupSeparatesKindsFramesAndDepth()
    {
        Assert.True(FuzzSignature.IsDuplicateCrash("panic", new[] { "eval", "step" }, "panic", new[] { "eval", "step" }));
        Assert.False(FuzzSignature.IsDuplicateCrash("panic", new[] { "eval", "step" }, "segv", new[] { "alloc", "write" }));
        Assert.False(FuzzSignature.IsDuplicateCrash("panic", new[] { "a" }, "panic", new[] { "a", "b" }));
        Assert.False(FuzzSignature.IsDuplicateCrash("panic", new[] { "a", "b" }, "panic", new[] { "b", "a" }));
    }

    [Fact]
    public void FailureVocabularyIsClosed()
    {
        Assert.Equal(new[] { "fuzz-crash", "fuzz-timeout", "fuzz-oom", "fuzz-leak", "fuzz-panic" }, FuzzFailureIds.All);
        foreach (string id in FuzzFailureIds.All)
        {
            Assert.True(FuzzFailureIds.IsFuzzFailureId(id));
        }

        foreach (string bad in new[] { "fuzz-arbitrary-error", string.Empty, "FUZZ-CRASH", "fuzz-crash ", "property-panic", "fuzz-crash\n" })
        {
            Assert.False(FuzzFailureIds.IsFuzzFailureId(bad), bad);
        }

        Assert.False(FuzzFailureIds.IsFuzzFailureId(null));
    }

    // ---- TAP ----------------------------------------------------------------------------

    [Fact]
    public void TapLineIsOpaqueAndExact()
    {
        Assert.Equal("not ok 1 - ct_compare [fuzz-crash:3a8f19b2]", FuzzSignature.FormatTapFailure(1, "ct_compare", "fuzz-crash", "3a8f19b2"));
        Assert.Equal("not ok 42 - a/b.c-d_e [fuzz-oom:00000000]", FuzzSignature.FormatTapFailure(42, "a/b.c-d_e", "fuzz-oom", "00000000"));
    }

    [Fact]
    public void TapLineRefusesUnregisteredIdsAndBadHashes()
    {
        Assert.Throws<ArgumentException>(() => FuzzSignature.FormatTapFailure(1, "t", "fuzz-bogus", "3a8f19b2"));
        Assert.Throws<ArgumentException>(() => FuzzSignature.FormatTapFailure(1, "t", null!, "3a8f19b2"));
        foreach (string bad in new[] { "3A8F19B2", "3a8f19b", "3a8f19b22", string.Empty, "3a8f19b\n", "zzzzzzzz", "3a8f19b2\n" })
        {
            Assert.Throws<ArgumentException>(() => FuzzSignature.FormatTapFailure(1, "t", "fuzz-crash", bad));
        }

        Assert.Throws<ArgumentException>(() => FuzzSignature.FormatTapFailure(1, "t", "fuzz-crash", null!));
    }

    [Fact]
    public void TapTargetCannotForgeALine()
    {
        string line = FuzzSignature.FormatTapFailure(1, "evil\nok 2 - forged [x]\r\n", "fuzz-crash", "3a8f19b2");
        Assert.DoesNotContain('\n', line);
        Assert.DoesNotContain('\r', line);
        Assert.StartsWith("not ok 1 - evil_ok_2_-_forged__x___", line);
        Assert.Equal("not ok 1 - a__b [fuzz-crash:3a8f19b2]", FuzzSignature.FormatTapFailure(1, "aéb", "fuzz-crash", "3a8f19b2"));
        Assert.Equal("not ok 1 - " + new string('x', 128) + " [fuzz-crash:3a8f19b2]", FuzzSignature.FormatTapFailure(1, new string('x', 500), "fuzz-crash", "3a8f19b2"));
        Assert.Equal("not ok 1 - " + new string('_', 128) + " [fuzz-crash:3a8f19b2]", FuzzSignature.FormatTapFailure(1, new string('é', 100), "fuzz-crash", "3a8f19b2"));
        Assert.Equal("not ok 1 -  [fuzz-crash:3a8f19b2]", FuzzSignature.FormatTapFailure(1, string.Empty, "fuzz-crash", "3a8f19b2"));
        Assert.Equal("not ok 1 -  [fuzz-crash:3a8f19b2]", FuzzSignature.FormatTapFailure(1, null!, "fuzz-crash", "3a8f19b2"));
        // A lone surrogate and a hostile megabyte-long id cost O(128), not O(length).
        Assert.Equal("not ok 1 - ___ [fuzz-crash:3a8f19b2]", FuzzSignature.FormatTapFailure(1, "\ud800", "fuzz-crash", "3a8f19b2"));
        string huge = FuzzSignature.FormatTapFailure(1, new string('y', 4_000_000), "fuzz-crash", "3a8f19b2");
        Assert.Equal("not ok 1 - ".Length + 128 + " [fuzz-crash:3a8f19b2]".Length, huge.Length);
    }

    [Fact]
    public void TapFailuresRenderEveryBucketWithoutPayload()
    {
        CrashRecord a = new("aaaaaaaa", "k", new[] { "f" }, new byte[] { 0xde, 0xad }, true, false);
        CrashRecord b = new("bbbbbbbb", "k", new[] { "g" }, new byte[] { 1 }, true, false);
        FuzzReport r = new(5, 1, 1, new[] { a, b }, 2, 3, 0, 0, FuzzReport.TierReported, 7, new List<byte[]> { new byte[] { 1 } });
        IReadOnlyList<string> lines = FuzzSignature.TapFailures(r, "tgt", 10);
        Assert.Equal(
            new[]
            {
                "not ok 10 - tgt [fuzz-crash:aaaaaaaa]",
                "not ok 11 - tgt [fuzz-crash:bbbbbbbb]",
                "not ok 12 - tgt [fuzz-timeout:4e61ea76]",
                "not ok 13 - tgt [fuzz-oom:7146a410]",
            },
            lines);
        Assert.All(lines, l => Assert.DoesNotContain("dead", l));
        FuzzReport clean = new(5, 1, 1, Array.Empty<CrashRecord>(), 0, 0, 0, 0, FuzzReport.TierReported, 7, new List<byte[]>());
        Assert.Empty(FuzzSignature.TapFailures(clean, "tgt", 1));
    }

    [Fact]
    public void TapFailuresAlwaysEmitANotOkLineEvenForAMalformedSignature()
    {
        CrashRecord bad = new("NOT-HEX!", "k", new[] { "f" }, Array.Empty<byte>(), true, false);
        FuzzReport r = new(1, 1, 1, new[] { bad }, 0, 0, 0, 0, FuzzReport.TierBlackBox, 1, new List<byte[]>());
        IReadOnlyList<string> lines = FuzzSignature.TapFailures(r, "t", 1);
        Assert.Single(lines);
        Assert.Equal("not ok 1 - t [fuzz-crash:43a76e9e]", lines[0]);
    }

    // ---- frame normalisation ---------------------------------------------------------------

    [Theory]
    [InlineData("Parser$Inner.Read", new[] { "Parser", "Inner" }, "Read")]
    [InlineData("App.Run", new[] { "App", "<>c__DisplayClass1_0" }, "<Run>b__0")]
    [InlineData("App.Run", new[] { "App", "<>c" }, "<Run>b__3_0")]
    [InlineData("App.Run", new[] { "App", "<Run>d__5" }, "MoveNext")]
    [InlineData("App.Outer.Inner", new[] { "App", "<>c__DisplayClass2_0" }, "<Outer>g__Inner|0_0")]
    [InlineData("Box_1.Get", new[] { "Box`1" }, "Get")]
    [InlineData("Main..ctor", new[] { "Main" }, ".ctor")]
    [InlineData("Odd_Class.m_x", new[] { "Odd:Class" }, "m:x")]
    [InlineData("A__.m", new[] { "Aé" }, "m")]
    [InlineData("_.F", new string[0], "F")]
    [InlineData("App.<>n__0", new[] { "App" }, "<>n__0")]
    public void NormaliseFrameKeepsTypeAndMethodOnly(string expected, string[] segments, string method)
    {
        string got = FuzzSignature.NormaliseFrame(segments, method);
        Assert.Equal(expected, got);
        Assert.DoesNotContain(':', got);
    }

    [Fact]
    public void NormaliseKindAndCollapse()
    {
        Assert.Equal("System.InvalidOperationException", FuzzSignature.NormaliseKind("System.InvalidOperationException"));
        Assert.Equal("a_b", FuzzSignature.NormaliseKind("a b"));
        Assert.Equal("Foo_1_", FuzzSignature.NormaliseKind("Foo`1+"));
        Assert.Equal(64, FuzzSignature.NormaliseKind(new string('x', 200)).Length);
        Assert.Equal(new[] { "a", "b", "a" }, FuzzSignature.Collapse(new[] { "a", "a", "b", "b", "b", "a" }));
        List<string> many = Enumerable.Range(0, 100).Select(i => "f" + i).ToList();
        List<string> kept = FuzzSignature.Collapse(many);
        Assert.Equal(64, kept.Count);
        Assert.Equal("f0", kept[0]);
        Assert.Equal("f63", kept[63]);
    }
}
