using System.Numerics;
using System.Text;
using CodingAdventures.TestBootstrap.CorpusJson;
using CodingAdventures.TestBootstrap.Pbt;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>
/// One regression test for each item the Java security reviews produced (VER05g section 3).
/// </summary>
public sealed class HardeningTests : HookedTest
{
    private static BigInteger Big(long v) => new(v);

    // Item: every public count is capped, and GenBytes stays capped even when maxLen is 0.
    [Fact]
    public void GenerationCountIsCappedEvenForEmptyStrings()
    {
        Assert.ThrowsAny<ArgumentException>(() => Gens.GenBytes(0, 0, 0, int.MaxValue));
        Assert.ThrowsAny<ArgumentException>(() => Gens.GenBytes(0, 0, 0, Limits.MaxGenCount + 1));
        Assert.ThrowsAny<ArgumentException>(() => Gens.GenPrimitive(0, "bool", int.MaxValue));
        Assert.ThrowsAny<ArgumentException>(() => Gens.GenPrimitive(0, "u8", Limits.MaxGenCount + 1));
        Assert.ThrowsAny<ArgumentException>(() => Gens.GenBounded(0, Big(0), Big(1), int.MaxValue));
        Assert.ThrowsAny<ArgumentException>(() => Gens.GenBounded(0, Big(0), Big(1), Limits.MaxGenCount + 1));
        Assert.Equal(Limits.MaxGenCount, Gens.GenBytes(0, 0, 0, Limits.MaxGenCount).Count);
        Assert.Equal(Limits.MaxGenCount, Gens.GenPrimitive(0, "bool", Limits.MaxGenCount).Count);
    }

    // Item: GenBounded is limited by count x byte width.
    [Fact]
    public void GenBoundedHasACountTimesWidthBudget()
    {
        BigInteger wide = BigInteger.Pow(10, 1000);
        Assert.Throws<ArgumentException>(() => Gens.GenBounded(0, BigInteger.Zero, wide, 100_000));
        Assert.Equal(1000, Gens.GenBounded(0, BigInteger.Zero, wide, 1000).Count);

        // The span, not the bounds, sets the width: huge but narrow bounds are cheap.
        Assert.Equal(1_000_000, Gens.GenBounded(0, wide, wide + 1, 1_000_000).Count);
    }

    // Item: global attempt budget counts rejected attempts (sequence shrinking).
    [Fact]
    public void SequenceShrinkingChargesRejectedAttemptsAgainstABudget()
    {
        const int n = 20_000;
        List<BigInteger> start = Enumerable.Repeat(BigInteger.One, n).ToList();
        int calls = 0;

        // Holds for every list except the full-length one: every smaller candidate is rejected.
        Shrunk<List<BigInteger>> r = Shrink.ShrinkList(start, BigInteger.Zero, l => { calls++; return l.Count != n; }, 10);
        Assert.Equal(n, r.Value.Count);
        Assert.Equal(0, r.Steps);
        long budget = Limits.AttemptBudget(10);
        Assert.Equal(16L * 10 + 64, budget);
        Assert.True(calls <= budget + 2, "evaluations " + calls + " exceed budget " + budget);

        int byteCalls = 0;
        Shrunk<byte[]> b = Gens.Bytes(0, 50_000).Shrink(new byte[50_000], x => { byteCalls++; return x.Length == 50_000; }, 5);
        Assert.Equal(50_000, b.Value.Length);
        Assert.True(byteCalls <= Limits.AttemptBudget(5));
    }

    // Item: one GLOBAL budget over all passes and all components of a tuple.
    [Fact]
    public void TupleShrinkingSharesOneGlobalBudget()
    {
        // Component B makes exactly one accepted step per call, so the loop repeats once
        // per step; component A rejects every candidate but would cost a full local budget
        // each pass if the budget were per pass.
        Gen<BigInteger> oneStep = Gen<BigInteger>.Of(
            _ => Big(1_000_000),
            (value, falsifies, max) =>
                value.Sign > 0 && max > 0 && falsifies(value - 1)
                    ? new Shrunk<BigInteger>(value - 1, 1)
                    : new Shrunk<BigInteger>(value, 0));
        const int n = 5_000;
        Gen<List<byte>> list = Gens.ListOf(Gens.U8(), 0, n);
        List<byte> start = Enumerable.Repeat((byte)1, n).ToList();
        int calls = 0;
        const int maxSteps = 100;
        Shrunk<Pair<List<byte>, BigInteger>> r = Gens.Tuple2(list, oneStep).Shrink(
            new Pair<List<byte>, BigInteger>(start, Big(1_000_000)),
            p => { calls++; return p.First.Count == n; },
            maxSteps);
        long budget = Limits.AttemptBudget(maxSteps);
        Assert.True(calls <= budget, "evaluations " + calls + " exceed the global budget " + budget);
        Assert.Equal(n, r.Value.First.Count);

        calls = 0;
        Gens.Tuple3(list, oneStep, Gens.Bool()).Shrink(
            new Triple<List<byte>, BigInteger, bool>(start, Big(1_000_000), true),
            t => { calls++; return t.First.Count == n; },
            maxSteps);
        Assert.True(calls <= budget, "tuple3 evaluations " + calls);
    }

    // Item: the runner wraps the property once per falsified case, so nested shrinkers
    // together stay inside one budget.
    [Fact]
    public void RunnerSharesOneBudgetAcrossNestedShrinkers()
    {
        Gen<Pair<List<byte>, List<byte>>> g = Gens.Tuple2(Gens.ListOf(Gens.U8(), 40, 60), Gens.ListOf(Gens.U8(), 40, 60));
        int calls = 0;
        const int maxSteps = 5;
        Verdict v = Runner.Check(
            3,
            g,
            p => { calls++; return p.First.Count < 40 || p.Second.Count < 40; }, // fails for every candidate with both lists >= 40
            new Options(3, maxSteps, CiMode.Off));
        Assert.Equal(FailureIds.Counterexample, v.FailureId);
        // 1 original evaluation + shrink budget + 1 re-verification.
        Assert.True(calls <= 1 + Limits.AttemptBudget(maxSteps) + 1, "evaluations " + calls);
    }

    // Item: integer shrinking is bounded under the same attempt budget (and is at most
    // two evaluations per accepted step).
    [Fact]
    public void IntegerWalkIsBoundedByTheBudget()
    {
        int calls = 0;
        Shrunk<BigInteger> r = Shrink.ShrinkInt(
            Big(1_000_000_000), BigInteger.Zero, BigInteger.Zero, Big(2_000_000_000), x => { calls++; return x <= 5; }, 10);
        Assert.Equal(10, r.Steps);
        Assert.True(calls <= Limits.AttemptBudget(10), "evaluations " + calls);
        Gen<BigInteger> g = Gens.Integer(Big(0), Big(2_000_000_000));
        calls = 0;
        Verdict v = Runner.Check(1, g, x => { calls++; return x < 100; }, new Options(100, 10, CiMode.Off));
        Assert.Equal(FailureIds.Counterexample, v.FailureId);
        Assert.True(calls <= 100 + Limits.AttemptBudget(10) + 1, "evaluations " + calls);
    }

    // Item: phase 2 (single deletion) copies each candidate once, not twice.
    [Fact]
    public void DeletionPhaseCopiesEachCandidateOnce()
    {
        const int n = 2000;
        Gen<List<long>> g = Gens.ListOf(Gens.Int64(0, 9), 0, n);
        List<long> start = new(new long[n]); // all zero: the element phase does no work
        long before = GC.GetAllocatedBytesForCurrentThread();

        // Every smaller candidate "holds" (rejected): n deletion attempts, n-1 elements each.
        Shrunk<List<long>> r = g.Shrink(start, l => l.Count == n, 1000);
        long allocated = GC.GetAllocatedBytesForCurrentThread() - before;
        Assert.Equal(n, r.Value.Count);
        long oneCopyEach = (long)n * n * sizeof(long); // n attempts x n x 8 bytes
        Assert.True(allocated < oneCopyEach * 1.4, "allocated " + allocated + " vs one copy each " + oneCopyEach);
        Assert.True(allocated > oneCopyEach * 0.9, "the test must actually exercise the deletion phase: " + allocated);
    }

    // The property may scribble on its candidate without reaching the shrinker's state.
    [Fact]
    public void AMutatingPropertyCannotCorruptTheDeletionPhase()
    {
        Shrunk<List<BigInteger>> r = Shrink.ShrinkList(
            new List<BigInteger> { Big(7), Big(1), Big(1), Big(1), Big(7) },
            BigInteger.Zero,
            l =>
            {
                bool holds = l.Count(v => v == Big(7)) < 2;
                l.Clear();
                return holds;
            },
            1000);
        Assert.Equal(new List<BigInteger> { Big(7), Big(7) }, r.Value);
    }

    // ---- strict parser limits ------------------------------------------------------------

    private static string Repeat(string s, int n) => new StringBuilder(s.Length * n).Insert(0, s, n).ToString();

    [Fact]
    public void JsonDepthIsCappedAt64()
    {
        string ok = Repeat("[", 64) + Repeat("]", 64);
        Assert.NotNull(StrictJson.Parse(Encoding.UTF8.GetBytes(ok)));
        string deep = Repeat("[", 65) + Repeat("]", 65);
        JsonException e = Assert.Throws<JsonException>(() => StrictJson.Parse(Encoding.UTF8.GetBytes(deep)));
        Assert.Contains("nesting deeper than 64", e.Message);

        // A hostile million-deep document must be a JsonException, never a stack overflow.
        string huge = Repeat("[", 1_000_000);
        Assert.Throws<JsonException>(() => StrictJson.Parse(Encoding.UTF8.GetBytes(huge)));
        string objects = Repeat("{\"a\":", 100_000);
        Assert.Throws<JsonException>(() => StrictJson.Parse(Encoding.UTF8.GetBytes(objects)));
    }

    [Fact]
    public void JsonSizeIsCappedAt8MiB()
    {
        byte[] atLimit = Encoding.UTF8.GetBytes("\"" + new string('a', StrictJson.MaxBytes - 2) + "\"");
        Assert.Equal(StrictJson.MaxBytes, atLimit.Length);
        Assert.IsType<string>(StrictJson.Parse(atLimit));
        byte[] over = new byte[StrictJson.MaxBytes + 1];
        Array.Fill(over, (byte)' ');
        JsonException e = Assert.Throws<JsonException>(() => StrictJson.Parse(over));
        Assert.Contains("exceeds", e.Message);
    }

    [Fact]
    public void JsonNodeCountIsCappedAt200000()
    {
        string ok = "[" + Repeat("0,", StrictJson.MaxNodes - 2) + "0]"; // array + (MaxNodes-1) numbers = MaxNodes values
        Assert.NotNull(StrictJson.Parse(Encoding.UTF8.GetBytes(ok)));
        string over = "[" + Repeat("0,", StrictJson.MaxNodes) + "0]";
        JsonException e = Assert.Throws<JsonException>(() => StrictJson.Parse(Encoding.UTF8.GetBytes(over)));
        Assert.Contains("more than " + StrictJson.MaxNodes, e.Message);
    }

    [Fact]
    public void JsonDuplicateKeysAreRefusedAndNamed()
    {
        JsonException e = Assert.Throws<JsonException>(() => StrictJson.Parse(Encoding.UTF8.GetBytes("{\"a\":1,\"a\":2}")));
        Assert.Contains("duplicate JSON key \"a\"", e.Message);
        string longKey = new('k', 5000);
        JsonException clipped = Assert.Throws<JsonException>(
            () => StrictJson.Parse(Encoding.UTF8.GetBytes("{\"" + longKey + "\":1,\"" + longKey + "\":2}")));
        Assert.True(clipped.Message.Length < 200, "the quoted key must be clipped");
        Assert.Throws<JsonException>(() => StrictJson.Parse(Encoding.UTF8.GetBytes("{\"a\":{\"b\":1,\"b\":1}}")));
    }

    [Fact]
    public void JsonUtf8IsStrict()
    {
        Assert.Throws<JsonException>(() => StrictJson.Parse(new byte[] { (byte)'"', 0xff, (byte)'"' }));
        Assert.Throws<JsonException>(() => StrictJson.Parse(new byte[] { (byte)'"', 0xc3, (byte)'"' })); // truncated sequence
        Assert.Throws<JsonException>(() => StrictJson.Parse(new byte[] { (byte)'"', 0xed, 0xa0, 0x80, (byte)'"' })); // encoded surrogate
        Assert.Throws<JsonException>(() => StrictJson.Parse(new byte[] { (byte)'"', 0xc0, 0x80, (byte)'"' })); // overlong
        Assert.Throws<JsonException>(() => StrictJson.Parse(new byte[] { 0xef, 0xbb, 0xbf, (byte)'1' })); // BOM
        Assert.Equal("é", StrictJson.Parse(new byte[] { (byte)'"', 0xc3, 0xa9, (byte)'"' }));
    }

    [Fact]
    public void JsonNumbersRejectLeadingZerosAndKeepTheirDigits()
    {
        foreach (string bad in new[] { "01", "-01", "00", "+1", "1.", ".5", "1e", "1e+", "-", "--1", "0x10", "NaN", "Infinity" })
        {
            Assert.True(Record.Exception(() => StrictJson.Parse(Encoding.UTF8.GetBytes(bad))) is JsonException, bad);
        }

        Assert.Equal(new JsonNumber("0"), StrictJson.Parse("0"u8.ToArray()));
        Assert.Equal(new JsonNumber("-0"), StrictJson.Parse("-0"u8.ToArray()));
        Assert.Equal(new JsonNumber("123456789012345678901234567890"), StrictJson.Parse("123456789012345678901234567890"u8.ToArray()));
        Assert.Equal(new JsonNumber("1.5e-3"), StrictJson.Parse("1.5e-3"u8.ToArray()));
    }

    [Fact]
    public void JsonTrailingDataIsRefused()
    {
        foreach (string bad in new[] { "1 2", "{} {}", "[] x", "null,", "true false", "[1,]", "{\"a\":1,}", "[1 2]", "{\"a\" 1}", "{1:2}" })
        {
            Assert.True(Record.Exception(() => StrictJson.Parse(Encoding.UTF8.GetBytes(bad))) is JsonException, bad);
        }
    }

    // ---- other bounds ---------------------------------------------------------------------

    [Fact]
    public void CanonIsBoundedAndTotal()
    {
        // Depth > 16 renders as "...", node count truncates, nothing throws.
        object nested = 1;
        for (int i = 0; i < 40; i++)
        {
            nested = new List<object> { nested };
        }

        string rendered = Canon.Render(nested);
        Assert.Contains("...", rendered);
        Assert.True(rendered.Length < 100);

        List<int> wide = Enumerable.Range(0, 100_000).ToList();
        string wideRendered = Canon.Render(wide);
        Assert.EndsWith("...]", wideRendered);
        Assert.True(wideRendered.Length < 100_000);

        // An unbounded enumerable is cut off by the node budget.
        IEnumerable<int> infinite = InfiniteInts();
        Assert.EndsWith("...]", Canon.Render(infinite));
    }

    private static IEnumerable<int> InfiniteInts()
    {
        int i = 0;
        while (true)
        {
            yield return i++;
        }
    }

    [Fact]
    public void ValuesDecodeIsDepthAndSizeCapped()
    {
        object? node = "x";
        for (int i = 0; i < 70; i++)
        {
            node = new List<object?> { node };
        }

        Assert.Throws<CaseException>(() => Values.Decode(node));
        Assert.Throws<CaseException>(() => Values.Decode(Enumerable.Repeat((object?)true, Values.MaxSize + 1).ToList()));
        Assert.Equal(Values.MaxSize, ((List<object?>)Values.Decode(Enumerable.Repeat((object?)true, Values.MaxSize).ToList())!).Count);
        Dictionary<string, object?> bigInt = new() { ["kind"] = "int", ["value"] = new string('9', Values.MaxIntDigits + 1) };
        Assert.Throws<CaseException>(() => Values.Decode(bigInt));
        bigInt["value"] = new string('9', Values.MaxIntDigits);
        Assert.IsType<BigInteger>(Values.Decode(bigInt));
        Dictionary<string, object?> bigBytes = new() { ["kind"] = "bytes", ["hex"] = new string('a', (2 * Values.MaxSize) + 2) };
        Assert.Throws<CaseException>(() => Values.Decode(bigBytes));
    }

    [Fact]
    public void ReportedProblemTextIsBounded()
    {
        List<object?> cases = Fx.CopyCases(Fx.LoadCases());
        Dictionary<string, object?> first = Fx.Obj(cases[0]);
        first["id"] = new string('x', 5000);
        first["type"] = new string('t', 5000);
        List<string> problems = Corpus.Evaluate(cases).Problems.ToList();
        Assert.NotEmpty(problems);
        Assert.All(problems, p => Assert.True(p.Length <= Values.MaxProblemLen + 3, "problem of " + p.Length + " chars"));
    }

    [Fact]
    public void CorpusRunnerRefusesOversizeGenerationBeforeGenerating()
    {
        List<object?> cases = Fx.CopyCases(Fx.LoadCases());
        int index = cases.FindIndex(c => (string)Fx.Obj(c)["op"]! == "gen_bytes");
        Dictionary<string, object?> c = Fx.Obj(cases[index]);
        string id = (string)c["id"]!;
        c["count"] = new Dictionary<string, object?> { ["kind"] = "int", ["value"] = "10000" };
        c["max_len"] = new Dictionary<string, object?> { ["kind"] = "int", ["value"] = "10000" };
        c["outputs"] = Enumerable.Repeat((object?)new Dictionary<string, object?> { ["kind"] = "bytes", ["hex"] = "" }, 10_000).ToList();
        List<string> problems = Corpus.Evaluate(cases).Problems.Where(p => p.StartsWith(id + ":", StringComparison.Ordinal)).ToList();
        Assert.Contains(problems, p => p.Contains("generation budget"));
    }
}
