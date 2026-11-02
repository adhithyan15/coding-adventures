namespace CodingAdventures.Rng.Tests;

// ConformanceTests.cs — the shared rng corpus (spec rng.md, "Conformance corpus")
// ==============================================================================
//
// code/specs/fixtures/rng-v1/cases.txt is the flat form of the corpus every
// language's rng package must reproduce; its results come from the independent
// reference code/scripts/rng_conformance.py. Each line is whitespace-separated:
//
//     case <id> <generator> <seed>
//     next_u32 <result>
//     next_u64 <result>
//     next_float <16 hex digits of the IEEE-754 bits>
//     next_int_in_range <min> <max> <result | error>
//     end
//
// The file is found by walking up from the test assembly to the directory that
// contains code/specs, because dotnet test runs from bin/<config>/<tfm>/.

using System.Globalization;
using Xunit;

public class ConformanceTests
{
    /// <summary>The three generators behind one interface.</summary>
    private sealed class AnyRng
    {
        private readonly Lcg? _lcg;
        private readonly Xorshift64? _xs;
        private readonly Pcg32? _pcg;

        public AnyRng(string name, ulong seed)
        {
            switch (name)
            {
                case "lcg": _lcg = new Lcg(seed); break;
                case "xorshift64": _xs = new Xorshift64(seed); break;
                case "pcg32": _pcg = new Pcg32(seed); break;
                default: throw new ArgumentException($"unknown generator {name}");
            }
        }

        public uint U32() => _lcg?.NextU32() ?? _xs?.NextU32() ?? _pcg!.NextU32();
        public ulong U64() => _lcg?.NextU64() ?? _xs?.NextU64() ?? _pcg!.NextU64();
        public double F64() => _lcg?.NextFloat() ?? _xs?.NextFloat() ?? _pcg!.NextFloat();
        public long InRange(long min, long max) =>
            _lcg?.NextIntInRange(min, max) ?? _xs?.NextIntInRange(min, max) ?? _pcg!.NextIntInRange(min, max);
    }

    private static string CorpusPath()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir != null && !Directory.Exists(Path.Combine(dir.FullName, "code", "specs")))
        {
            dir = dir.Parent;
        }
        Assert.NotNull(dir);
        return Path.Combine(dir!.FullName, "code", "specs", "fixtures", "rng-v1", "cases.txt");
    }

    [Fact]
    public void EveryCorpusCaseMatches()
    {
        int cases = 0, calls = 0;
        string id = "";
        AnyRng? g = null;
        var inv = CultureInfo.InvariantCulture;
        foreach (var line in File.ReadAllLines(CorpusPath()))
        {
            if (line.Length == 0 || line.StartsWith('#')) continue;
            var t = line.Split(' ');
            switch (t[0])
            {
                case "case":
                    id = t[1];
                    g = new AnyRng(t[2], ulong.Parse(t[3], inv));
                    cases++;
                    break;
                case "end":
                    g = null;
                    break;
                case "next_u32":
                    calls++;
                    Assert.True(uint.Parse(t[1], inv) == g!.U32(), id);
                    break;
                case "next_u64":
                    calls++;
                    Assert.True(ulong.Parse(t[1], inv) == g!.U64(), id);
                    break;
                case "next_float":
                    calls++;
                    Assert.True(
                        ulong.Parse(t[1], NumberStyles.HexNumber, inv) == (ulong)BitConverter.DoubleToInt64Bits(g!.F64()),
                        id);
                    break;
                case "next_int_in_range":
                {
                    calls++;
                    long min = long.Parse(t[1], inv), max = long.Parse(t[2], inv);
                    var gen = g!;
                    if (t[3] == "error")
                    {
                        // Thrown before any draw, so the sequence carries on.
                        Assert.Throws<ArgumentOutOfRangeException>(() => gen.InRange(min, max));
                    }
                    else
                    {
                        Assert.True(long.Parse(t[3], inv) == gen.InRange(min, max), id);
                    }
                    break;
                }
                default:
                    Assert.Fail($"malformed corpus line: {line}");
                    break;
            }
        }
        // Guard against a truncated or empty file passing vacuously.
        Assert.True(cases >= 27, $"cases: {cases}");
        Assert.True(calls >= 150, $"calls: {calls}");
    }

    // ── Regressions for the v1 range bug ─────────────────────────────────────

    [Fact]
    public void WholeLongDomainNoLongerDividesByZero()
    {
        var g = new Lcg(3);
        var values = Enumerable.Range(0, 64).Select(_ => g.NextIntInRange(long.MinValue, long.MaxValue)).ToList();
        Assert.True(values.Any(v => v < 0) && values.Any(v => v > 0));
    }

    [Fact]
    public void WideRangesReachBeyondTwoToThe32()
    {
        var g = new Pcg32(3);
        Assert.Contains(Enumerable.Range(0, 64).Select(_ => g.NextIntInRange(0, 1L << 40)), v => v >= 1L << 32);
    }

    [Fact]
    public void HalfDomainPlusOneTerminates()
    {
        var g = new Xorshift64(3);
        for (int i = 0; i < 100; i++)
        {
            Assert.True(g.NextIntInRange(long.MinValue, 0) <= 0);
        }
    }
}
