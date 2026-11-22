// ============================================================================
// ConformanceTests.cs -- bit-exact conformance against the TRIG-REF-1 vectors
// ============================================================================
//
// code/specs/fixtures/trig-reference-v1 holds every vector as one row
//
//     op <TAB> x <TAB> y <TAB> cr <TAB> flags
//
// where x and y are IEEE binary64 *bit patterns* (16 lowercase hex digits),
// never decimals.  This test decodes x with BitConverter.Int64BitsToDouble,
// calls the PUBLIC function, encodes the result with
// BitConverter.DoubleToInt64Bits and compares the two 64-bit integers
// (PHY00a 6.5).  Printed doubles are never compared: they hide the last
// digits.  The only relaxation is NaN, which matches any NaN (VER04p "same":
// the sign and payload of a NaN are not portable).
//
// Three kinds of check live here:
//
//   1. fixture integrity -- the files on disk are the ones the manifest hashes;
//   2. bit-for-bit match -- every row, per group, with failing rows counted
//                           per op so a failure says WHICH operation drifted;
//   3. accuracy vs `cr`  -- every row that carries a correctly rounded value
//                           is within the stated number of representable steps.
//                           An op that has a `cr` value but no stated limit
//                           FAILS (fail closed); `tan` is the one op that is
//                           named as having no accuracy claim (PHY00a 3.8).

using System.Security.Cryptography;
using System.Text.Json;
using Xunit.Sdk;

namespace CodingAdventures.Trig.Tests;

public sealed class ConformanceTests
{
    private static readonly string[] Groups = ["edge", "sweep-binade", "sweep-cw", "sweep-prng"];
    private const ulong QuietNaNBits = 0x7FF8000000000000UL;

    /// <summary>One parsed fixture row.</summary>
    internal sealed record Row(string Op, string X, string Y, string Cr, string Flags);

    // ------------------------------------------------------------------
    // Helpers: locating the fixture, bit casts, parsing, dispatch
    // ------------------------------------------------------------------

    /// <summary>Walk up from the test binary until the repository's code/specs is found.</summary>
    private static string FixtureDir()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir != null && !Directory.Exists(Path.Combine(dir.FullName, "code", "specs")))
        {
            dir = dir.Parent;
        }

        Assert.NotNull(dir);
        return Path.Combine(dir!.FullName, "code", "specs", "fixtures", "trig-reference-v1");
    }

    private static ulong Parse(string hex) => Convert.ToUInt64(hex, 16);

    private static double FromHex(string hex) => BitConverter.UInt64BitsToDouble(Parse(hex));

    private static ulong BitsOf(double v) => BitConverter.DoubleToUInt64Bits(v);

    private static IEnumerable<string> ReadLines(string file)
    {
        var text = File.ReadAllText(Path.Combine(FixtureDir(), file));
        Assert.EndsWith("\n", text);
        return text.Split('\n').Where(l => l.Length > 0);
    }

    internal static List<Row> Load(string group)
    {
        var rows = new List<Row>();
        foreach (var line in ReadLines(group + ".tsv"))
        {
            var f = line.Split('\t');
            Assert.Equal(5, f.Length);
            rows.Add(new Row(f[0], f[1], f[2], f[3], f[4]));
        }

        return rows;
    }

    /// <summary>Call the public function named by <paramref name="op"/>; an unknown op fails.</summary>
    internal static double Call(string op, string x)
    {
        if (op == "atan2")
        {
            var comma = x.IndexOf(',');
            return Trig.Atan2(FromHex(x[..comma]), FromHex(x[(comma + 1)..]));
        }

        var v = FromHex(x);
        switch (op)
        {
            case "sin": return Trig.Sin(v);
            case "cos": return Trig.Cos(v);
            case "tan": return Trig.Tan(v);
            case "radians": return Trig.Radians(v);
            case "degrees": return Trig.Degrees(v);
            case "sqrt": return Trig.Sqrt(v);
            case "atan": return Trig.Atan(v);
            default:
                throw new XunitException("unknown op in fixture: " + op);
        }
    }

    /// <summary>True when the row's result matches the reference under VER04p "same".</summary>
    private static bool Matches(Row r)
    {
        if (r.Y == "error")
        {
            // sqrt of a negative: this port's idiomatic error is ArgumentOutOfRangeException.
            try
            {
                Call(r.Op, r.X);
            }
            catch (ArgumentOutOfRangeException)
            {
                return true;
            }

            return false;
        }

        var got = Call(r.Op, r.X);
        var want = Parse(r.Y);
        return want == QuietNaNBits ? double.IsNaN(got) : BitsOf(got) == want;
    }

    // ------------------------------------------------------------------
    // 1. Fixture integrity
    // ------------------------------------------------------------------

    internal static JsonDocument Manifest() =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(FixtureDir(), "manifest.json")));

    [Fact]
    public void ManifestHashesAndCountsMatchTheFiles()
    {
        using var m = Manifest();
        Assert.Equal("TRIG-REF-1", m.RootElement.GetProperty("algorithm").GetString());
        var seen = 0;
        foreach (var g in m.RootElement.GetProperty("groups").EnumerateArray())
        {
            var file = g.GetProperty("file").GetString()!;
            var data = File.ReadAllBytes(Path.Combine(FixtureDir(), file));
            Assert.Equal(g.GetProperty("sha256").GetString(), Convert.ToHexString(SHA256.HashData(data)).ToLowerInvariant());
            Assert.Equal(g.GetProperty("rows").GetInt32(), data.Count(b => b == (byte)'\n'));
            seen++;
        }

        Assert.Equal(Groups.Length, seen);
    }

    [Fact]
    public void RowSyntaxIsClosed()
    {
        var ops = new[] { "sin", "cos", "tan", "radians", "degrees", "sqrt", "atan", "atan2" };
        var hex = new System.Text.RegularExpressions.Regex("^[0-9a-f]{16}$");
        foreach (var group in Groups)
        {
            foreach (var r in Load(group))
            {
                Assert.Contains(r.Op, ops);
                var inputs = r.Op == "atan2" ? r.X.Split(',') : new[] { r.X };
                Assert.Equal(r.Op == "atan2" ? 2 : 1, inputs.Length);
                foreach (var input in inputs)
                {
                    Assert.Matches(hex, input);
                }

                Assert.True(r.Y == "error" || hex.IsMatch(r.Y), "y " + r.Y);
                Assert.True(r.Cr == "-" || hex.IsMatch(r.Cr), "cr " + r.Cr);
            }
        }
    }

    // ------------------------------------------------------------------
    // 2. Bit-for-bit match, per group, failing rows counted per op
    // ------------------------------------------------------------------

    private static void CheckGroup(string group)
    {
        var failing = new SortedDictionary<string, int>();
        var first = new List<string>();
        var checkedRows = 0;
        foreach (var r in Load(group))
        {
            checkedRows++;
            if (!Matches(r))
            {
                failing[r.Op] = failing.GetValueOrDefault(r.Op) + 1;
                if (first.Count < 8)
                {
                    first.Add($"{r.Op} {r.X} want {r.Y}");
                }
            }
        }

        Assert.True(checkedRows > 0, group + " has rows");
        Assert.True(
            failing.Count == 0,
            $"{group}: failing rows per op {{{string.Join(", ", failing.Select(p => p.Key + "=" + p.Value))}}}; first {string.Join(" | ", first)}");
    }

    [Fact] public void EdgeVectorsMatchBitForBit() => CheckGroup("edge");

    [Fact] public void SweepBinadeVectorsMatchBitForBit() => CheckGroup("sweep-binade");

    [Fact] public void SweepCwVectorsMatchBitForBit() => CheckGroup("sweep-cw");

    [Fact] public void SweepPrngVectorsMatchBitForBit() => CheckGroup("sweep-prng");

    // ------------------------------------------------------------------
    // 3. Accuracy against the correctly rounded column
    // ------------------------------------------------------------------

    /// <summary>Map a double's bits onto a monotone integer line so that steps can be counted.</summary>
    private static long Ordered(ulong bits)
    {
        var mag = (long)(bits & 0x7FFFFFFFFFFFFFFFUL);
        return (bits >> 63) != 0 ? -mag : mag;
    }

    /// <summary>The stated limits, in representable steps (PHY00a 4.1 gate).</summary>
    private static readonly Dictionary<string, long> Limits = new()
    {
        ["sin"] = 3,
        ["cos"] = 3,
        ["radians"] = 1,
        ["degrees"] = 1,
    };

    /// <summary>
    /// PHY00a 3.8: NO accuracy claim for tan (sin / cos plus a division and a
    /// pinned pole sentinel), although tan rows carry a cr.
    /// </summary>
    private const string NoClaim = "tan";

    /// <summary>
    /// Worst distance, in steps, per op, over <paramref name="rows"/>.  Fails closed:
    /// an op with a cr value but no limit (other than the named no-claim op) fails.
    /// </summary>
    internal static Dictionary<string, long> WorstSteps(IEnumerable<Row> rows, Func<Row, double> call)
    {
        var worst = new Dictionary<string, long>();
        foreach (var r in rows)
        {
            if (r.Cr == "-" || r.Op == NoClaim)
            {
                continue;
            }

            Assert.True(Limits.ContainsKey(r.Op), $"op {r.Op} has a cr value but no accuracy limit");
            var steps = Math.Abs(Ordered(BitsOf(call(r))) - Ordered(Parse(r.Cr)));
            worst[r.Op] = Math.Max(worst.GetValueOrDefault(r.Op), steps);
        }

        return worst;
    }

    [Fact]
    public void AccuracyAgainstTheCorrectlyRoundedColumn()
    {
        var worst = WorstSteps(Groups.SelectMany(Load), r => Call(r.Op, r.X));
        foreach (var (op, limit) in Limits)
        {
            var w = worst.GetValueOrDefault(op);
            Assert.True(w <= limit, $"{op}: worst {w} steps > {limit}");
        }
    }

    [Fact]
    public void AnOpWithACrValueButNoLimitFails()
    {
        // sqrt rows carry no cr today; if one ever does, the gate must refuse it.
        var row = new Row("sqrt", "4000000000000000", "3ff6a09e667f3bcd", "3ff6a09e667f3bcd", "");
        Assert.ThrowsAny<XunitException>(() => WorstSteps([row], r => Call(r.Op, r.X)));
    }

    [Fact]
    public void TanIsTheOneOpWithNoAccuracyClaim()
    {
        // A tan row with a deliberately wrong cr is skipped, not failed.
        var row = new Row("tan", "3ff0000000000000", "3ff8eb245cbee3a6", "0000000000000001", "");
        Assert.Empty(WorstSteps([row], r => Call(r.Op, r.X)));
        Assert.DoesNotContain("tan", Limits.Keys);
    }

    [Fact]
    public void AnUnknownOpFails()
    {
        Assert.ThrowsAny<XunitException>(() => Call("sinh", "0000000000000000"));
    }

    // ------------------------------------------------------------------
    // Contract points of PHY00a 3.3 asserted directly
    // ------------------------------------------------------------------

    [Fact]
    public void SinOfNegativeZeroIsNegativeZero()
    {
        Assert.Equal(0x8000000000000000UL, BitsOf(Trig.Sin(-0.0)));
        Assert.Equal(0UL, BitsOf(Trig.Sin(0.0)));
    }

    [Fact]
    public void CosOfZeroIsOne()
    {
        Assert.Equal(1.0, Trig.Cos(0.0));
        Assert.Equal(1.0, Trig.Cos(-0.0));
    }

    [Fact]
    public void TinyArgumentsAreReturnedUnchanged()
    {
        var tiny = FromHex("3e3fffffffffffff");
        Assert.Equal(BitsOf(tiny), BitsOf(Trig.Sin(tiny)));
        Assert.Equal(BitsOf(double.Epsilon), BitsOf(Trig.Sin(double.Epsilon)));
        Assert.Equal(1.0, Trig.Cos(tiny));
    }

    [Fact]
    public void OutsideTheDomainIsNaN()
    {
        double[] xs =
        [
            double.NaN, double.PositiveInfinity, double.NegativeInfinity, 1e20,
            BitConverter.UInt64BitsToDouble(0x41C0000000000001UL), double.MaxValue,
        ];
        foreach (var x in xs)
        {
            Assert.True(double.IsNaN(Trig.Sin(x)), "sin " + x);
            Assert.True(double.IsNaN(Trig.Cos(x)), "cos " + x);
            Assert.True(double.IsNaN(Trig.Tan(x)), "tan " + x);
        }
    }

    [Fact]
    public void TheLastInDomainValueIsFinite()
    {
        Assert.Equal(0x3FD4E67C0E2622DFUL, BitsOf(Trig.Sin(536870912.0)));
        Assert.Equal(0xBFEE3EDD2DFEEF90UL, BitsOf(Trig.Cos(536870912.0)));
    }

    [Fact]
    public void RadiansAndDegreesAreSingleMultiplies()
    {
        Assert.Equal(Trig.PI, Trig.Radians(180.0));
        Assert.Equal(180.0, Trig.Degrees(Trig.PI));
        Assert.Equal(double.PositiveInfinity, Trig.Degrees(1.7e308));
        Assert.True(double.IsNaN(Trig.Radians(double.NaN)));
        Assert.Equal(0x8000000000000000UL, BitsOf(Trig.Radians(-0.0)));
        Assert.Equal(0x8000000000000000UL, BitsOf(Trig.Degrees(-0.0)));
    }

    [Fact]
    public void TanPoleSentinel()
    {
        Assert.Equal(1e308, Trig.Tan(Trig.PI / 2.0));
        Assert.Equal(-1e308, Trig.Tan(-Trig.PI / 2.0));
    }

    [Fact]
    public void Atan2OfNaNIsNaN()
    {
        var nan = double.NaN;
        Assert.True(double.IsNaN(Trig.Atan2(nan, 1.0)));
        Assert.True(double.IsNaN(Trig.Atan2(1.0, nan)));
        Assert.True(double.IsNaN(Trig.Atan2(-1.0, nan)));
        Assert.True(double.IsNaN(Trig.Atan2(nan, nan)));
        Assert.True(double.IsNaN(Trig.Atan2(nan, -1.0)));
        Assert.True(double.IsNaN(Trig.Atan2(nan, 0.0)));
    }

    [Fact]
    public void Atan2PinnedConventions()
    {
        Assert.Equal(Trig.PI, Trig.Atan2(-0.0, -1.0));
        Assert.Equal(0.0, Trig.Atan2(0.0, 0.0));
        Assert.True(double.IsNaN(Trig.Atan2(double.PositiveInfinity, double.PositiveInfinity)));
        Assert.Equal(Trig.PI / 2.0, Trig.Atan2(1e300, 1e-300));
    }

    [Fact]
    public void AtanKeepsTheSignOfZero()
    {
        Assert.Equal(0x8000000000000000UL, BitsOf(Trig.Atan(-0.0)));
    }

    [Fact]
    public void NegativeQuadrantArithmetic()
    {
        // k = -1 and k = -5 map to quadrant 3: "& 3" is the floor modulus.
        Assert.Equal(-Trig.Sin(Trig.PI / 2.0), Trig.Sin(-Trig.PI / 2.0));
        Assert.False(double.IsNaN(Trig.Sin(-5 * (Trig.PI / 2.0))));
        Assert.True(Math.Abs(Trig.Cos(-5 * (Trig.PI / 2.0))) < 1e-15);
    }

    [Fact]
    public void SqrtOfNegativeIsAnError()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => Trig.Sqrt(-1.0));
    }
}
